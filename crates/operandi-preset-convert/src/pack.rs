//! The Operandi Library pack steps `scripts/build-library-pack.sh` runs
//! (operandi_core::preset::pack):
//!
//! ```text
//! operandi-preset-convert pack-manifest <pack folder> --name <name>
//!     --version <version> --checked <YYYY-MM-DD>
//! operandi-preset-convert pack-sizes <file.oppreset>...
//! ```
//!
//! - `pack-manifest` reads every preset's licence header under Starter/ and
//!   Extras/ and the Matcaps/ and Environments/ SOURCES.tsv lists, refuses the pack (exit 3, every problem listed) when an item
//!   breaks the licence rule, else writes manifest.json and CREDITS.txt and
//!   a `<Name>.png` thumbnail next to each preset that has one in its
//!   header and none on disk (not for Meshes/, where it would list as a
//!   picture of its own).
//! - `pack-sizes` puts the pictures' current sizes into presets whose maps
//!   were scaled down (the lite pack), keeping their paths relative.
use base64::Engine;
use operandi_core::preset::pack::{self, Manifest};
use std::path::{Path, PathBuf};

/// Exit code for a pack the licence rule refuses.
pub const REFUSED: u8 = 3;

pub fn run(sub: &str, args: &[String]) -> Result<(), (u8, String)> {
    let fail = |e: String| (1, e);
    match sub {
        "pack-manifest" => {
            let mut folder = None;
            let (mut name, mut version, mut checked) = (None, None, None);
            let mut it = args.iter();
            while let Some(a) = it.next() {
                let mut value = |n: &str| {
                    it.next()
                        .cloned()
                        .ok_or_else(|| fail(format!("{n} needs a value")))
                };
                match a.as_str() {
                    "--name" => name = Some(value("--name")?),
                    "--version" => version = Some(value("--version")?),
                    "--checked" => checked = Some(value("--checked")?),
                    s if s.starts_with('-') => return Err(fail(format!("unknown option {s}"))),
                    s => folder = Some(PathBuf::from(s)),
                }
            }
            let usage = "usage: operandi-preset-convert pack-manifest <pack folder> --name <name> \
                         --version <version> --checked <YYYY-MM-DD>";
            let (Some(folder), Some(name), Some(version), Some(checked)) =
                (folder, name, version, checked)
            else {
                return Err(fail(usage.into()));
            };
            let manifest = write_manifest(&folder, &name, &version, &checked)?;
            println!(
                "manifest: {} with {} items ({})",
                manifest.title(),
                manifest.items.len(),
                manifest.licence_summary()
            );
            Ok(())
        }
        "pack-sizes" => {
            if args.is_empty() {
                return Err(fail(
                    "usage: operandi-preset-convert pack-sizes <file.oppreset>...".into(),
                ));
            }
            for a in args {
                refresh_sizes(Path::new(a)).map_err(fail)?;
            }
            Ok(())
        }
        _ => Err(fail(format!("unknown step {sub}"))),
    }
}

/// Build, check and write the pack's manifest, credits and thumbnails.
pub fn write_manifest(
    folder: &Path,
    name: &str,
    version: &str,
    checked: &str,
) -> Result<Manifest, (u8, String)> {
    let manifest = pack::build_manifest(folder, name, version, checked).map_err(|problems| {
        (
            REFUSED,
            format!(
                "the licence rule refuses this pack:\n  {}",
                problems.join("\n  ")
            ),
        )
    })?;
    for item in &manifest.items {
        let preset = folder.join(&item.path);
        // Matcaps and environments are pictures already. Mesh presets share
        // a category folder, where a picture beside one would list as an
        // image of its own; their thumbnail stays inside the file.
        if item
            .path
            .starts_with(&format!("{}/", operandi_core::preset::pack::MESHES))
        {
            continue;
        }
        let is_preset = preset
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(operandi_core::preset::is_preset_extension);
        if !is_preset {
            continue;
        }
        let png = preset.with_extension("png");
        if png.exists() {
            continue;
        }
        let header = operandi_core::persistence::read_preset_header(&preset).map_err(|e| (1, e))?;
        if let Some(b64) = &header.thumbnail {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| (1, format!("{}: thumbnail: {e}", item.path)))?;
            std::fs::write(&png, bytes).map_err(|e| (1, format!("{}: {e}", png.display())))?;
        }
    }
    manifest.write(folder).map_err(|e| (1, e))?;
    let credits = folder.join("CREDITS.txt");
    std::fs::write(&credits, pack::credits_text(&manifest))
        .map_err(|e| (1, format!("{}: {e}", credits.display())))?;
    Ok(manifest)
}

/// Read each picture's size again and save, keeping paths relative to the
/// preset.
pub fn refresh_sizes(path: &Path) -> Result<(), String> {
    let mut preset = operandi_core::persistence::load_preset(path)?;
    let folder = path.parent().unwrap_or(Path::new("."));
    for image in &mut preset.scene.images {
        let file = PathBuf::from(&image.path);
        if let Ok((w, h)) = image::image_dimensions(&file) {
            image.width = w;
            image.height = h;
        }
        if let Ok(rel) = file.strip_prefix(folder) {
            image.path = rel.to_string_lossy().into_owned();
        }
    }
    operandi_core::persistence::save_preset(path, &preset)
}
