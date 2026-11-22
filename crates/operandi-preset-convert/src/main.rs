//! `operandi-preset-convert`: a texture-set folder, optionally with the
//! node graph JSON the Blender exporter wrote (tools/blender-export), becomes
//! a material `.oppreset` (operandi_core::preset::convert).
//!
//! ```text
//! operandi-preset-convert <folder> --out <file.oppreset>
//!     [--graph <graph.json>] [--material <name>] [--set <name>]
//!     [--licence <name>] [--url <url>] [--credit <text>]
//!     [--copy-maps] [--no-thumbnail]
//! ```
//!
//! - `--copy-maps` copies the pictures the preset uses next to it and names
//!   them relative to the preset, so the folder can be moved as a whole.
//! - The thumbnail is the base colour map scaled to 128 px (the preset
//!   browser's rendered ball isn't on main yet).
//! - MX-08: a `.mtlx` in the folder that reads the set's pictures (as
//!   ambientCG ships) is preferred over the graph and the pictures' names;
//!   `--no-mtlx` turns that off.
//! - Exit codes: 0 converted, 2 refused (the graph or the .mtlx needs a
//!   Node Material), 1 any other error. Nothing is written unless the
//!   conversion succeeds.
//! - `pack-manifest` and `pack-sizes` are the library pack's steps
//!   (pack.rs); `pack-manifest` exits 3 when the licence rule refuses.
mod pack;

use base64::Engine;
use operandi_core::node_graph_reader::GraphFile;
use operandi_core::preset::PresetCredit;
use operandi_core::preset::convert::{ConvertError, convert, pick_material};
use operandi_core::preset::texture_set::{MapKind, TextureSet, texture_sets};
use operandi_core::shader_tree::{NodeKind, TextureEffect, TextureSource};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const IMAGE_EXTENSIONS: [&str; 9] = [
    "png", "jpg", "jpeg", "tif", "tiff", "bmp", "tga", "exr", "hdr",
];

#[derive(Default)]
struct Args {
    folder: PathBuf,
    out: Option<PathBuf>,
    graph: Option<PathBuf>,
    material: Option<String>,
    set: Option<String>,
    licence: Option<String>,
    url: Option<String>,
    credit: Option<String>,
    copy_maps: bool,
    thumbnail: bool,
    no_mtlx: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        thumbnail: true,
        ..Args::default()
    };
    let mut it = std::env::args().skip(1);
    let mut folder = None;
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "--out" => args.out = Some(value("--out")?.into()),
            "--graph" => args.graph = Some(value("--graph")?.into()),
            "--material" => args.material = Some(value("--material")?),
            "--set" => args.set = Some(value("--set")?),
            "--licence" | "--license" => args.licence = Some(value("--licence")?),
            "--url" => args.url = Some(value("--url")?),
            "--credit" => args.credit = Some(value("--credit")?),
            "--copy-maps" => args.copy_maps = true,
            "--no-thumbnail" => args.thumbnail = false,
            "--no-mtlx" => args.no_mtlx = true,
            "-h" | "--help" => return Err(usage()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n{}", usage())),
            s => folder = Some(PathBuf::from(s)),
        }
    }
    args.folder = folder.ok_or_else(usage)?;
    Ok(args)
}

fn usage() -> String {
    "usage: operandi-preset-convert <folder> --out <file.oppreset> [--graph <graph.json>] \
     [--material <name>] [--set <name>] [--licence <name>] [--url <url>] [--credit <text>] \
     [--copy-maps] [--no-thumbnail] [--no-mtlx]"
        .into()
}

fn size_of(path: &Path) -> Option<(u32, u32)> {
    image::image_dimensions(path).ok()
}

/// MX-08: the folder's `.mtlx` that reads one of `set`'s pictures.
fn mtlx_for(folder: &Path, set: &TextureSet) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(operandi_core::materialx::EXTENSION))
        })
        .collect();
    found.sort();
    found.into_iter().find(|p| {
        operandi_core::materialx::read(p).is_ok_and(|doc| {
            let reads = operandi_core::materialx::pictures(&doc, folder);
            let names: Vec<_> = reads.iter().filter_map(|r| r.file_name()).collect();
            set.files()
                .any(|f| f.file_name().is_some_and(|n| names.contains(&n)))
        })
    })
}

/// The folder's texture sets, by the shared loader's rules (CD-20).
fn sets_in(folder: &Path) -> Result<Vec<TextureSet>, String> {
    let mut pictures = Vec::new();
    for entry in std::fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || !path.is_file() {
            continue;
        }
        if path.extension().is_some_and(|e| {
            IMAGE_EXTENSIONS.contains(&e.to_string_lossy().to_ascii_lowercase().as_str())
        }) {
            pictures.push(path);
        }
    }
    let refs: Vec<&Path> = pictures.iter().map(PathBuf::as_path).collect();
    Ok(texture_sets(&refs))
}

fn run(args: Args) -> Result<(), (u8, String)> {
    let fail = |e: String| (1u8, e);
    let out = args.out.clone().ok_or_else(|| fail(usage()))?;
    let sets = sets_in(&args.folder).map_err(fail)?;
    let set = match &args.set {
        Some(name) => sets
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| fail(format!("no texture set named {name}")))?,
        // The set with the most maps.
        None => sets
            .iter()
            .max_by_key(|s| s.maps.len())
            .ok_or_else(|| fail(format!("{} holds no texture set", args.folder.display())))?,
    };
    let graph_file = match &args.graph {
        Some(p) => {
            let text =
                std::fs::read_to_string(p).map_err(|e| fail(format!("{}: {e}", p.display())))?;
            Some(GraphFile::parse(&text).map_err(fail)?)
        }
        None => None,
    };
    let graph_dir = args
        .graph
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let material =
        match (&graph_file, &args.material) {
            (None, _) => None,
            (Some(g), Some(name)) => Some(
                g.materials
                    .iter()
                    .find(|m| &m.name == name)
                    .ok_or_else(|| fail(format!("the graph has no material {name}")))?,
            ),
            (Some(g), None) => Some(pick_material(&g.materials, set).ok_or_else(|| {
                fail("no material in the graph uses this folder's pictures".into())
            })?),
        };
    // Licence: the command line, else the graph's.
    let from_graph = graph_file.as_ref().and_then(|g| g.licence.clone());
    let licence = args
        .licence
        .clone()
        .or_else(|| from_graph.as_ref().and_then(|l| l.name.clone()));
    let credit = licence.map(|licence| PresetCredit {
        licence,
        url: args
            .url
            .clone()
            .or_else(|| from_graph.as_ref().and_then(|l| l.url.clone()))
            .unwrap_or_default(),
        credit: args
            .credit
            .clone()
            .or_else(|| from_graph.as_ref().and_then(|l| l.credit.clone()))
            .unwrap_or_default(),
    });
    let mtlx = if args.no_mtlx {
        None
    } else {
        mtlx_for(&args.folder, set)
    };
    let converted = match &mtlx {
        // MX-08: the .mtlx says what each picture is for.
        Some(path) => {
            let (mut preset, notes) =
                operandi_core::materialx::material_preset(path, None, &size_of).map_err(|e| {
                    let code = if e.starts_with(operandi_core::materialx::NEEDS_NODES) {
                        2
                    } else {
                        1
                    };
                    (code, format!("{}: {e}", path.display()))
                })?;
            preset.header.source = credit;
            // MX-Q03: the set's AO, as its own preset would have it.
            let mut notes = notes;
            notes.extend(
                set.add_ambient_occlusion(
                    &mut preset,
                    operandi_core::preset::texture_set::ResolutionChoice::Highest,
                    &size_of,
                )
                .map_err(|e| (1, e))?,
            );
            operandi_core::preset::convert::Converted { preset, notes }
        }
        None => convert(
            set,
            material.map(|m| (m, graph_dir.as_path())),
            credit,
            &size_of,
        )
        .map_err(|e| match e {
            ConvertError::Refused(r) => (2, r.to_string()),
            ConvertError::Failed(e) => (1, e),
        })?,
    };
    let mut preset = converted.preset;
    let out_dir = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    std::fs::create_dir_all(&out_dir).map_err(|e| fail(format!("{}: {e}", out_dir.display())))?;
    if args.thumbnail {
        preset.header.thumbnail = thumbnail(&preset.scene, set);
    }
    if args.copy_maps {
        for image in &mut preset.scene.images {
            let from = PathBuf::from(&image.path);
            let name = from
                .file_name()
                .ok_or_else(|| fail(format!("{} has no file name", image.path)))?;
            let to = out_dir.join(name);
            if from != to {
                std::fs::copy(&from, &to)
                    .map_err(|e| fail(format!("copy {}: {e}", from.display())))?;
            }
            image.path = name.to_string_lossy().into_owned();
        }
    }
    operandi_core::persistence::save_preset(&out, &preset).map_err(fail)?;
    let from = match (&mtlx, material) {
        (Some(p), _) => format!(
            " (MaterialX {})",
            p.file_name().unwrap_or_default().to_string_lossy()
        ),
        (None, Some(m)) => format!(" (graph material {})", m.name),
        (None, None) => String::new(),
    };
    println!("converted {} -> {}{from}", set.name, out.display());
    for note in &converted.notes {
        println!("  note: {note}");
    }
    Ok(())
}

/// The base colour map at 128 px as a base64 PNG (PB-52's slot), or the
/// set's first map when it has none.
fn thumbnail(scene: &operandi_core::Scene, set: &TextureSet) -> Option<String> {
    let colour = scene.shader_tree.nodes.iter().find_map(|n| match &n.kind {
        NodeKind::Texture(t)
            if t.effect == TextureEffect::DiffuseColor && n.layer.blend == Default::default() =>
        {
            match &t.source {
                TextureSource::ImageMap {
                    image: Some(id), ..
                } => scene
                    .images
                    .iter()
                    .find(|i| i.id == *id)
                    .map(|i| PathBuf::from(&i.path)),
                _ => None,
            }
        }
        _ => None,
    });
    let path = colour.or_else(|| {
        set.maps
            .iter()
            .find(|m| m.kind == MapKind::BaseColor)
            .or(set.maps.first())
            .map(|m| m.path.clone())
    })?;
    let picture = image::open(&path).ok()?.thumbnail(128, 128).to_rgba8();
    let mut png = Vec::new();
    picture
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(base64::engine::general_purpose::STANDARD.encode(png))
}

fn main() -> ExitCode {
    let all: Vec<String> = std::env::args().skip(1).collect();
    if let Some(sub) = all.first().filter(|s| s.starts_with("pack-")) {
        return match pack::run(sub, &all[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err((code, e)) => {
                eprintln!("{e}");
                ExitCode::from(code)
            }
        };
    }
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, e)) => {
            eprintln!("{e}");
            ExitCode::from(code)
        }
    }
}
