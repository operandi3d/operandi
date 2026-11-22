//! MaterialX (Stage B of docs/specs/openpbr-materialx, MX-01..08):
//! Operandi's own `.mtlx` reader and writer (no MaterialX C++), with the
//! node definitions it needs bundled from MaterialX and OpenPBR
//! (Apache-2.0; crates/operandi-core/materialx, licences in
//! docs/licenses).
//!
//! - [`material_preset`] reads a file's material into a Material preset
//!   (a material group with a Material and Image Map layers), which loads
//!   through the same `LoadPreset` command as a preset file (one undo
//!   step). A material that needs more than that is refused whole
//!   (MX-04).
//! - [`export`] writes a Material Group as an `open_pbr_surface`
//!   material, listing what it couldn't write.
use crate::Result;
use crate::preset::Preset;
use crate::shader_tree::NodeId;
use std::path::{Path, PathBuf};

mod colour;
pub mod cycles;
pub mod doc;
pub mod export;
pub mod graph;
pub mod import;
#[cfg(test)]
mod library_probe;
pub mod stdlib;
pub mod tree;
pub mod tree_library;
pub mod tree_standard;
pub mod view;
pub mod write;

pub use import::NEEDS_NODES;

/// The file extension.
pub const EXTENSION: &str = "mtlx";

/// MaterialX files bigger than this are refused (a sanity budget).
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Read and parse a `.mtlx` file.
pub fn read(path: &Path) -> Result<doc::Document> {
    let size = std::fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if size > MAX_FILE_BYTES {
        return Err(format!(
            "{}: too large for a MaterialX material",
            path.display()
        ));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    doc::parse(&text)
}

/// The materials a file holds, by name.
pub fn material_names(doc: &doc::Document) -> Vec<String> {
    import::materials(doc)
        .iter()
        .map(|m| m.name.clone())
        .collect()
}

/// The file's material (`material`, or its first) as a Material preset,
/// with notes on anything approximated or left out. `size_of` gives a
/// picture's pixel size (the core can't read pictures).
pub fn material_preset(
    path: &Path,
    material: Option<&str>,
    size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
) -> Result<(Preset, Vec<String>)> {
    let doc = read(path)?;
    let folder = path.parent().unwrap_or(Path::new(""));
    import::preset(&doc, material, folder, size_of)
}

/// Every picture file the document reads, as paths from its folder.
pub fn pictures(doc: &doc::Document, folder: &Path) -> Vec<PathBuf> {
    fn walk(els: &[doc::Element], folder: &Path, out: &mut Vec<PathBuf>) {
        for e in els {
            for p in &e.inputs {
                if p.ty == "filename"
                    && let Some(v) = &p.value
                    && !v.is_empty()
                {
                    out.push(folder.join(format!("{}{v}", p.fileprefix)));
                }
            }
            walk(&e.children, folder, out);
        }
    }
    let mut out = Vec::new();
    walk(&doc.elements, folder, &mut out);
    out
}

/// Write the Material Group holding `node` as MaterialX at `path`.
/// Returns what couldn't be written (MX-05's warning).
///
/// A group the Material and Image Map writer (MX-05) can't write whole is
/// written as its surface's standard MaterialX form instead
/// (tree_standard), when every node it uses has a standard version; the
/// notes then say what that form leaves out.
pub fn export(scene: &crate::Scene, node: NodeId, path: &Path) -> Result<Vec<String>> {
    let folder = path.parent().unwrap_or(Path::new(""));
    let (simple, losses) = export::material_x(scene, node, folder)?;
    if losses.is_empty() {
        crate::persistence::write_atomic(path, simple.as_bytes())?;
        return Ok(losses);
    }
    let group = scene.shader_tree.get(node).and_then(|n| match n.kind {
        crate::shader_tree::NodeKind::Group(_) => Some(n.id),
        _ => n.parent,
    });
    let standard = group.and_then(|g| {
        let surface = tree_standard::group_surface(scene, g)?;
        let name = scene.shader_tree.get(g)?.name.clone();
        // Picture paths relative to the file's folder where they're under it.
        let pictures = crate::Scene {
            images: scene
                .images
                .iter()
                .map(|i| {
                    let mut i = i.clone();
                    if let Ok(rel) = Path::new(&i.path).strip_prefix(folder) {
                        i.path = rel.to_string_lossy().into_owned();
                    }
                    i
                })
                .collect(),
            ..Default::default()
        };
        Some(tree_standard::document_with(
            &surface,
            &pictures,
            &name,
            scene.shader_tree.group_nodes(g),
        ))
    });
    match standard {
        Some(Ok((text, notes))) => {
            crate::persistence::write_atomic(path, text.as_bytes())?;
            Ok(notes)
        }
        Some(Err(why)) => {
            crate::persistence::write_atomic(path, simple.as_bytes())?;
            let mut out = losses;
            out.push(format!("kept to the Material and Image Maps because {why}"));
            Ok(out)
        }
        None => {
            crate::persistence::write_atomic(path, simple.as_bytes())?;
            Ok(losses)
        }
    }
}

#[cfg(test)]
mod tests;
