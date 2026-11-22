//! OpenEXR multilayer render saves (render outputs, 2026-11-20).
//!
//! One file, one part, every output as channels: the first Final Color is
//! the main R, G, B, A; every other output is "<Output name>.R/.G/.B/.A";
//! each Cryptomatte type is "<CryptoName>00.R/G/B/A", "...01" and so on,
//! with the Cryptomatte metadata (`cryptomatte/<key>/name`, `hash`,
//! `conversion`, `manifest`) in the header. Everything is 32-bit float:
//! Cryptomatte must be, and the outputs keep their exact numbers (RO-Q17).
use exr::prelude::*;
use operandi_core::render_outputs::{CryptomatteType, cryptomatte_metadata_prefix};
use operandi_cycles::CryptoLayers;
use std::path::Path;

/// One output picture for the file: linear (or sRGB-encoded) RGBA floats,
/// top row first. An empty name is the main layer.
pub struct OutputLayer<'a> {
    pub name: &'a str,
    pub rgba: &'a [f32],
    /// Shaded AA Samples: the heat map also keeps its raw share of samples
    /// as "<name>.Y", full float (RO-Q30).
    pub heat_map: bool,
}

fn text(s: &str) -> Text {
    // UTF-8 bytes as they are: names and manifests may hold any letters.
    Text::from_slice_unchecked(s.as_bytes())
}

/// Channel prefixes for the outputs: dots become underscores (a dot
/// separates layer and channel), a name that could pass for a Cryptomatte
/// layer gets "Output " in front, and repeated names get " 2", " 3".
fn layer_names(outputs: &[OutputLayer]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    outputs
        .iter()
        .map(|o| {
            let mut base = o.name.replace('.', "_");
            if CryptomatteType::ALL
                .iter()
                .any(|k| base.starts_with(k.layer_name()))
            {
                base = format!("Output {base}");
            }
            let mut name = base.clone();
            let mut n = 2;
            while seen.contains(&name) {
                name = format!("{base} {n}");
                n += 1;
            }
            seen.push(name.clone());
            name
        })
        .collect()
}

/// Writes `outputs` and the Cryptomatte layers `crypto` into one OpenEXR
/// file at `path`.
pub fn write(
    path: &Path,
    width: u32,
    height: u32,
    outputs: &[OutputLayer],
    crypto: &[CryptoLayers],
) -> std::result::Result<(), String> {
    let n = width as usize * height as usize;
    if outputs.iter().any(|o| o.rgba.len() != n * 4)
        || crypto
            .iter()
            .flat_map(|c| &c.passes)
            .any(|p| p.len() != n * 4)
    {
        return Err("The picture's size doesn't match its pixels".into());
    }
    let plane = |rgba: &[f32], k: usize| -> FlatSamples {
        FlatSamples::F32(rgba.as_chunks::<4>().0.iter().map(|p| p[k]).collect())
    };
    let mut channels: SmallVec<[AnyChannel<FlatSamples>; 4]> = SmallVec::new();
    for (o, name) in outputs.iter().zip(layer_names(outputs)) {
        for (k, c) in ["R", "G", "B", "A"].into_iter().enumerate() {
            let full = if name.is_empty() {
                c.to_owned()
            } else {
                format!("{name}.{c}")
            };
            channels.push(AnyChannel::new(text(&full), plane(o.rgba, k)));
        }
        if o.heat_map {
            let t: Vec<f32> = o
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| operandi_core::render_outputs::heat_value([p[0], p[1], p[2]]))
                .collect();
            channels.push(AnyChannel::new(
                text(&format!("{name}.Y")),
                FlatSamples::F32(t),
            ));
        }
    }
    let mut attributes = LayerAttributes::default();
    for c in crypto {
        for (k, pass) in c.passes.iter().enumerate() {
            for (j, ch) in ["R", "G", "B", "A"].into_iter().enumerate() {
                let mut channel =
                    AnyChannel::new(text(&format!("{}{k:02}.{ch}", c.name)), plane(pass, j));
                // Ids are bit patterns: never quantised (lossless anyway).
                channel.quantize_linearly = true;
                channels.push(channel);
            }
        }
        let key = cryptomatte_metadata_prefix(c.name);
        for (field, value) in [
            ("name", c.name),
            ("hash", "MurmurHash3_32"),
            ("conversion", "uint32_to_float32"),
            ("manifest", c.manifest.as_str()),
        ] {
            attributes.other.insert(
                text(&format!("{key}{field}")),
                AttributeValue::Text(text(value)),
            );
        }
    }
    if channels.is_empty() {
        return Err("Nothing to save".into());
    }
    let layer = Layer::new(
        (width as usize, height as usize),
        attributes,
        // Scanlines, ZIP: what compositors' Cryptomatte readers expect.
        Encoding {
            compression: Compression::ZIP16,
            blocks: Blocks::ScanLines,
            line_order: LineOrder::Increasing,
        },
        AnyChannels::sort(channels),
    );
    Image::from_layer(layer)
        .write()
        .to_file(path)
        .map_err(|e| format!("Can't save {}: {e}", path.display()))
}

/// The Cryptomatte layers of a file `write` made (the render cache keeps
/// them so a history slot saves them later).
pub fn read_crypto(path: &Path) -> std::result::Result<Vec<CryptoLayers>, String> {
    let image = exr::prelude::read_all_flat_layers_from_file(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out: Vec<CryptoLayers> = Vec::new();
    for layer in &image.layer_data {
        let other = &layer.attributes.other;
        let n = layer.size.area();
        for kind in CryptomatteType::ALL.iter().copied() {
            let name = kind.layer_name();
            let mut passes: Vec<Vec<f32>> = Vec::new();
            for k in 0..8 {
                let find = |c: &str| {
                    let wanted = format!("{name}{k:02}.{c}");
                    layer
                        .channel_data
                        .list
                        .iter()
                        .find(|ch| ch.name.as_slice() == wanted.as_bytes())
                        .map(|ch| match &ch.sample_data {
                            FlatSamples::F32(v) => v.clone(),
                            FlatSamples::F16(v) => v.iter().map(|h| h.to_f32()).collect(),
                            FlatSamples::U32(v) => v.iter().map(|u| *u as f32).collect(),
                        })
                };
                let Some(planes) = ["R", "G", "B", "A"]
                    .into_iter()
                    .map(find)
                    .collect::<Option<Vec<Vec<f32>>>>()
                else {
                    break;
                };
                passes.push(
                    (0..n)
                        .flat_map(|i| [planes[0][i], planes[1][i], planes[2][i], planes[3][i]])
                        .collect(),
                );
            }
            if passes.is_empty() {
                continue;
            }
            let key = text(&format!("{}manifest", cryptomatte_metadata_prefix(name)));
            let manifest = match other.get(&key) {
                Some(AttributeValue::Text(t)) => String::from_utf8_lossy(t.as_slice()).into_owned(),
                _ => "{}".to_owned(),
            };
            out.push(CryptoLayers {
                kind,
                name,
                levels: passes.len() as u32 * 2,
                passes,
                manifest,
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::render_outputs::{hash_to_float, murmur3_32};

    /// A multilayer file read back with the exr crate: layer names, values
    /// and the Cryptomatte metadata.
    #[test]
    fn multilayer_file_reads_back() {
        let dir = std::env::temp_dir().join(format!("operandi-exr-ml-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("render.exr");
        let (w, h) = (2u32, 1u32);
        let main = [0.25, 0.5, 0.75, 1.0, 1.0, 0.0, 0.0, 0.5];
        let depth = [4.0, 4.0, 4.0, 1.0, 9.5, 9.5, 9.5, 1.0];
        let dup = [0.1f32; 8];
        let cube = hash_to_float(murmur3_32(b"Cube"));
        let crypto = CryptoLayers {
            kind: CryptomatteType::Object,
            name: "CryptoObject",
            levels: 2,
            passes: vec![vec![cube, 1.0, 0.0, 0.0, cube, 0.6, 0.0, 0.0]],
            manifest: format!(r#"{{"Cube":"{:08x}"}}"#, murmur3_32(b"Cube")),
        };
        write(
            &path,
            w,
            h,
            &[
                OutputLayer {
                    name: "",
                    rgba: &main,
                    heat_map: false,
                },
                OutputLayer {
                    name: "Depth Output",
                    rgba: &depth,
                    heat_map: false,
                },
                OutputLayer {
                    name: "Depth Output",
                    rgba: &dup,
                    heat_map: false,
                },
            ],
            std::slice::from_ref(&crypto),
        )
        .unwrap();
        let image = exr::prelude::read_all_flat_layers_from_file(&path).unwrap();
        assert_eq!(image.layer_data.len(), 1, "one part");
        let layer = &image.layer_data[0];
        let channel = |name: &str| -> Vec<f32> {
            let ch = layer
                .channel_data
                .list
                .iter()
                .find(|c| c.name.as_slice() == name.as_bytes())
                .unwrap_or_else(|| panic!("no channel {name}"));
            match &ch.sample_data {
                FlatSamples::F32(v) => v.clone(),
                other => panic!("{name} isn't full float: {other:?}"),
            }
        };
        assert_eq!(channel("R"), vec![0.25, 1.0]);
        assert_eq!(channel("A"), vec![1.0, 0.5]);
        assert_eq!(channel("Depth Output.R"), vec![4.0, 9.5]);
        assert_eq!(channel("Depth Output 2.G"), vec![0.1, 0.1]);
        assert_eq!(channel("CryptoObject00.R"), vec![cube, cube]);
        assert_eq!(channel("CryptoObject00.G"), vec![1.0, 0.6]);
        let key = cryptomatte_metadata_prefix("CryptoObject");
        let attr = |field: &str| match layer.attributes.other.get(&text(&format!("{key}{field}"))) {
            Some(AttributeValue::Text(t)) => String::from_utf8_lossy(t.as_slice()).into_owned(),
            other => panic!("{field}: {other:?}"),
        };
        assert_eq!(attr("name"), "CryptoObject");
        assert_eq!(attr("hash"), "MurmurHash3_32");
        assert_eq!(attr("conversion"), "uint32_to_float32");
        let manifest: serde_json::Value = serde_json::from_str(&attr("manifest")).unwrap();
        assert_eq!(
            manifest["Cube"],
            format!("{:08x}", murmur3_32(b"Cube")),
            "the manifest's hash is the id's bits"
        );
        assert_eq!(
            u32::from_str_radix(manifest["Cube"].as_str().unwrap(), 16).unwrap() & 0x807f_ffff,
            cube.to_bits() & 0x807f_ffff
        );
        // The cache reads its Cryptomatte layers back unchanged.
        assert_eq!(read_crypto(&path).unwrap(), vec![crypto]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Group D (RO-Q30): the AA heat map keeps its raw share as .Y.
    #[test]
    fn heat_map_keeps_its_raw_share() {
        use operandi_core::render_outputs::heat_color;
        let dir = std::env::temp_dir().join(format!("operandi-exr-heat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("render.exr");
        let mut heat = Vec::new();
        for t in [0.0f32, 0.25, 1.0] {
            heat.extend(heat_color(t));
            heat.push(1.0);
        }
        let main = [0.5f32; 12];
        write(
            &path,
            3,
            1,
            &[
                OutputLayer {
                    name: "",
                    rgba: &main,
                    heat_map: false,
                },
                OutputLayer {
                    name: "Shaded AA Samples Output",
                    rgba: &heat,
                    heat_map: true,
                },
            ],
            &[],
        )
        .unwrap();
        let image = exr::prelude::read_all_flat_layers_from_file(&path).unwrap();
        let layer = &image.layer_data[0];
        let names: Vec<String> = layer
            .channel_data
            .list
            .iter()
            .map(|c| String::from_utf8_lossy(c.name.as_slice()).into_owned())
            .collect();
        assert!(
            !names.iter().any(|n| n == "Y"),
            "only the heat map: {names:?}"
        );
        let y = layer
            .channel_data
            .list
            .iter()
            .find(|c| c.name.as_slice() == b"Shaded AA Samples Output.Y")
            .expect("a .Y channel");
        let FlatSamples::F32(v) = &y.sample_data else {
            panic!("full float")
        };
        for (got, want) in v.iter().zip([0.0, 0.25, 1.0]) {
            assert!((got - want).abs() < 1e-5, "{v:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wrong_sizes_are_refused() {
        let path = std::env::temp_dir().join("operandi-never-written.exr");
        let err = write(
            &path,
            2,
            2,
            &[OutputLayer {
                name: "",
                rgba: &[0.0; 4],
                heat_map: false,
            }],
            &[],
        );
        assert!(err.is_err());
        assert!(!path.exists());
    }
}
