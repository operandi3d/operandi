//! USD import, stage U4: a node graph sidecar. `tools/blender-export`
//! writes every material's Blender node tree as `operandi-node-graph`
//! JSON; when such a file sits beside the USD file as
//! `<file name>_graphs.json` (or is picked in the import dialog), each of
//! its materials whose name matches a material of the USD file replaces
//! that material's Material Group contents with a Node Material layer,
//! which renders the whole Blender graph (Noise, Color Ramp...) instead of
//! the flattened Preview Surface or MaterialX approximation. Names that
//! match nothing, and graphs Node Materials can't render, are reported.
use operandi_core::interchange::UsdImportOptions;
use operandi_core::{Result, Scene};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The sidecar to read for USD file `file`, if any: the picked one, else
/// `<file name>_graphs.json` beside it when it exists.
pub fn sidecar(file: &Path, options: &UsdImportOptions) -> Option<PathBuf> {
    if !options.graphs_sidecar {
        return None;
    }
    if let Some(picked) = &options.graphs_file {
        return Some(picked.clone());
    }
    let stem = file.file_stem()?.to_string_lossy().into_owned();
    let beside = file.with_file_name(format!("{stem}_graphs.json"));
    beside.is_file().then_some(beside)
}

/// Puts the sidecar's graphs in the Material Groups of the matching
/// `tags` (the materials the imported polygons use). Returns report lines.
pub fn add(scene: &mut Scene, sidecar: &Path, tags: &BTreeSet<String>) -> Result<Vec<String>> {
    let name = sidecar
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let text = match std::fs::read_to_string(sidecar) {
        Ok(t) => t,
        Err(e) => return Ok(vec![format!("Graphs sidecar {name} not read: {e}")]),
    };
    let (graphs, refused) = match operandi_core::node_graph::import_report(&text, sidecar.parent())
    {
        Ok(r) => r,
        Err(e) => return Ok(vec![format!("Graphs sidecar {name}: {e}")]),
    };
    let mut matched = Vec::new();
    let mut unmatched = Vec::new();
    for graph in graphs {
        let material = graph.material.name.clone();
        if tags.contains(&material) {
            scene.put_node_graph_for_tag(&material, graph)?;
            matched.push(material);
        } else {
            unmatched.push(material);
        }
    }
    let mut out = Vec::new();
    if !matched.is_empty() {
        out.push(format!(
            "Node graphs from {name}: {} (Node Material layers)",
            matched.join(", ")
        ));
    }
    if !unmatched.is_empty() {
        out.push(format!(
            "Node graphs in {name} that match no material of the file: {}",
            unmatched.join(", ")
        ));
    }
    out.extend(refused.into_iter().map(|r| format!("{name}: {r}")));
    Ok(out)
}
