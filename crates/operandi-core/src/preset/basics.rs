//! Basics: six simple material presets made from constants that ship
//! inside Operandi, so a fresh install's preset browser isn't empty (F1,
//! Jason 2026-11-11). Our own work, GPL-3.0-or-later like the app. The app
//! writes them to a folder of its own and shows it as a read-only root;
//! thumbnails render on demand on the cutaway ball like any preset without
//! a picture.
use super::{Preset, PresetCredit, PresetHeader, PresetKind};
use crate::Scene;
use crate::shader_tree::{
    GroupMask, Layer, MaterialChannels, NewLayer, Node, NodeKind, ShaderTree,
};

/// The Basics root's display name.
pub const ROOT_NAME: &str = "Basics";
/// Their licence (the app's own).
pub const LICENCE: &str = "GPL-3.0-or-later";

/// One basic material: its name and how it differs from a new Material.
struct Basic {
    name: &'static str,
    set: fn(&mut MaterialChannels),
}

const BASICS: [Basic; 6] = [
    Basic {
        name: "Plastic",
        set: |m| {
            m.diffuse_color = [0.75, 0.12, 0.1];
            m.roughness = 0.3;
        },
    },
    Basic {
        name: "Metal",
        set: |m| {
            m.diffuse_color = [0.9, 0.9, 0.92];
            m.metallic = 1.;
            m.roughness = 0.25;
        },
    },
    Basic {
        name: "Glass",
        set: |m| {
            m.diffuse_color = [1.; 3];
            m.diffuse_amount = 0.;
            m.roughness = 0.;
            m.transparent_amount = 1.;
            m.refractive_index = 1.5;
        },
    },
    Basic {
        name: "Clay",
        set: |m| {
            m.diffuse_color = [0.72, 0.55, 0.45];
            m.roughness = 0.9;
            m.specular_amount *= 0.5;
        },
    },
    Basic {
        name: "Rubber",
        set: |m| {
            m.diffuse_color = [0.04, 0.04, 0.045];
            m.roughness = 0.65;
        },
    },
    Basic {
        name: "Emissive",
        set: |m| {
            m.diffuse_color = [0.05; 3];
            m.luminous_color = [1., 0.85, 0.6];
            m.luminous_intensity = 3.;
        },
    },
];

/// Every basic material as a preset, in shelf order, with its file name
/// (no extension).
pub fn presets() -> Vec<(&'static str, Preset)> {
    BASICS.iter().map(|b| (b.name, preset(b))).collect()
}

fn preset(b: &Basic) -> Preset {
    let mut channels = match NewLayer::Material.kind() {
        NodeKind::Material(m) => m,
        _ => unreachable!("NewLayer::Material makes a Material"),
    };
    (b.set)(&mut channels);
    let node = |id, parent, name: String, kind| Node {
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
                node(
                    1,
                    None,
                    b.name.into(),
                    NodeKind::Group(GroupMask::default()),
                ),
                node(
                    2,
                    Some(1),
                    format!("{} Material", b.name),
                    NodeKind::Material(channels),
                ),
            ],
            next_id: 3,
            outputs: Default::default(),
            node_graphs: Default::default(),
        },
        ..Scene::default()
    };
    Preset {
        header: PresetHeader {
            kind: PresetKind::Material,
            thumbnail: None,
            source: Some(PresetCredit {
                licence: LICENCE.into(),
                url: String::new(),
                credit: "Operandi".into(),
            }),
        },
        scene,
    }
}

/// Write the Basics into `folder` (made if missing), leaving files that
/// already hold the same bytes alone so their thumbnails stay cached, and
/// removing `.oppreset` files that aren't Basics any more. Returns the
/// preset paths.
pub fn write_to(folder: &std::path::Path) -> Result<Vec<std::path::PathBuf>, String> {
    std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let mut out = Vec::new();
    for (name, preset) in presets() {
        let path = folder.join(format!("{name}.{}", super::PRESET_EXTENSION));
        let bytes = crate::persistence::encode_preset(&preset)?;
        if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
            crate::persistence::save_preset(&path, &preset)?;
        }
        out.push(path);
    }
    if let Ok(read) = std::fs::read_dir(folder) {
        for entry in read.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(super::PRESET_EXTENSION))
                && !out.contains(&path)
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    Ok(out)
}
