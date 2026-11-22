//! A MaterialX material the shader tree can't show as layers, kept whole
//! as a MaterialX graph (materials architecture, slice 1;
//! docs/MATERIALX_GRAPHS.md).
//!
//! Where MX-04 used to refuse such a material, [`preset`] makes a Material
//! preset holding one Material Group with a Node Material layer whose
//! graph is the material's own MaterialX text. Cycles renders it through
//! `materialx::cycles`; the viewport draws it from the same nodes
//! (`graph_view`, stage 5). A material whose nodes can't be translated is still
//! refused whole, naming the node.
use super::doc::Document;
use super::{NEEDS_NODES, cycles, write};
use crate::node_graph::{GraphMaterial, MaterialXGraph, NodeSection, StoredGraph};
use crate::preset::{Preset, PresetHeader, PresetKind};
use crate::shader_tree::{GroupMask, Layer, Node, NodeKind, NodeMaterialLayer, ShaderTree};
use crate::{Result, Scene};
use std::path::Path;

/// The refusal for a material whose graph can't render.
pub fn refusal(r: &cycles::Refusal) -> String {
    if r.node.is_empty() {
        format!("{NEEDS_NODES}: {}", r.reason)
    } else {
        format!("{NEEDS_NODES}: {} ({}), {}", r.node, r.category, r.reason)
    }
}

/// Material `material` (the first when `None`) as a graph layer preset,
/// with notes on what the render approximates.
pub fn preset(
    doc: &Document,
    material: Option<&str>,
    folder: &Path,
) -> Result<(Preset, Vec<String>)> {
    let translated = cycles::translate(doc, material, folder).map_err(|r| refusal(&r))?;
    let name = translated.name.clone();
    let text = write::material(doc, &name);
    let graph = StoredGraph {
        id: 1,
        material: GraphMaterial {
            name: name.clone(),
            output: "Material Output".into(),
            nodes: Vec::new(),
            links: Vec::new(),
            exposed: Vec::new(),
            groups_used: Vec::new(),
            extra: Default::default(),
        },
        groups: Vec::new(),
        source: Default::default(),
        licence: Default::default(),
        materialx: Some(MaterialXGraph::new(&name, folder, text)),
    };
    // The stored text must read back as the same render.
    let again = graph.flatten(&Default::default())?;
    if again != translated.graph {
        return Err(format!(
            "{name}: the stored MaterialX graph reads back differently"
        ));
    }
    let node = |id: u64, parent: Option<u64>, name: String, kind: NodeKind| Node {
        id,
        parent,
        name,
        layer: Layer::default(),
        kind,
        extras: Default::default(),
    };
    let scene = Scene {
        shader_tree: ShaderTree {
            node_views: Default::default(),
            nodes: vec![
                node(1, None, name.clone(), NodeKind::Group(GroupMask::default())),
                node(
                    2,
                    Some(1),
                    format!("{name} Graph"),
                    NodeKind::NodeMaterial(NodeMaterialLayer {
                        graph: Some(1),
                        overrides: Default::default(),
                    }),
                ),
            ],
            next_id: 3,
            outputs: Default::default(),
            node_graphs: NodeSection {
                graphs: vec![graph],
                ..Default::default()
            },
        },
        ..Scene::default()
    };
    scene.validate()?;
    let mut notes = vec![format!(
        "{name}: kept as a MaterialX graph ({} Cycles nodes)",
        translated.graph.nodes.len()
    )];
    notes.extend(translated.notes);
    Ok((
        Preset {
            header: PresetHeader {
                kind: PresetKind::Material,
                thumbnail: None,
                source: None,
            },
            scene,
        },
        notes,
    ))
}
