//! The Schematic's picture of the rig (RG-70..72, F12): nodes for rig
//! pieces and the items they touch, ports for the channels in use, wires
//! from outputs to inputs. Built from the scene each time it is asked for
//! (a view, not data); `operandi.rig.schematic` returns it as JSON and the
//! app's Schematic window draws it. Wiring, unwiring and deleting go
//! through the `operandi.rig.*` commands (RG-71/72).
use crate::Scene;
use crate::rig::{self, Piece};
use serde::Serialize;
use std::collections::BTreeMap;

/// A node: an item or a rig piece.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Node {
    /// `item:<id>` or `piece:<id>`.
    pub key: String,
    pub label: String,
    /// The item or piece id.
    pub id: u64,
    pub piece: bool,
    /// Pieces: the kind (`position`, `link`, `mod.expression`...).
    pub kind: Option<String>,
    /// Input ports (channels for items; inputs for pieces).
    pub inputs: Vec<String>,
    /// Output ports.
    pub outputs: Vec<String>,
    /// A loop broke it (RG-02: red).
    pub broken: bool,
    /// The column it lays out in: 0 drivers, 1 pieces, 2 driven.
    pub column: u8,
}

/// A wire from an output port to an input port.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Wire {
    pub from: String,
    pub from_port: String,
    pub to: String,
    pub to_port: String,
    /// The piece the wire belongs to (deleting the wire edits it).
    pub piece: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Schematic {
    pub nodes: Vec<Node>,
    pub wires: Vec<Wire>,
}

/// The port name for an item's world (constraints read and write it).
pub const WORLD: &str = "world";

fn item_key(id: u64) -> String {
    format!("item:{id}")
}
fn piece_key(id: u64) -> String {
    format!("piece:{id}")
}

/// What a piece reads: (input port, item, item port).
fn piece_inputs(p: &Piece) -> Vec<(String, u64, String)> {
    match p.kind.as_str() {
        "link" => match (p.targets.first(), p.names.get("driver")) {
            (Some(&i), Some(c)) => vec![("driver".into(), i, c.clone())],
            _ => Vec::new(),
        },
        k if k.starts_with("mod.") => rig::modifier_inputs(p)
            .into_iter()
            .map(|(n, k)| (n, k.item, k.channel))
            .collect(),
        "dynamicParent" => p
            .targets
            .iter()
            .enumerate()
            .map(|(i, t)| (format!("parent {}", i + 1), *t, WORLD.to_owned()))
            .collect(),
        _ => {
            let mut out: Vec<(String, u64, String)> = p
                .targets
                .iter()
                .enumerate()
                .map(|(i, t)| (format!("target {}", i + 1), *t, WORLD.to_owned()))
                .collect();
            for (name, item) in &p.refs {
                out.push((name.clone(), *item, WORLD.to_owned()));
            }
            out
        }
    }
}

/// What a piece writes: (item, item port); `None` for an unwired modifier.
fn piece_output(p: &Piece) -> Option<(u64, String)> {
    if crate::rig_eval::channel_piece(p) {
        return p.names.get("driven").map(|c| (p.item, c.clone()));
    }
    Some((p.item, WORLD.to_owned()))
}

/// The rig as nodes and wires (RG-70): every piece, every item a piece
/// reads or writes, and on items only the ports in use.
pub fn build(scene: &Scene) -> Schematic {
    let broken = crate::rig_eval::broken_pieces(scene);
    let mut item_ports: BTreeMap<u64, (Vec<String>, Vec<String>, bool, bool)> = BTreeMap::new();
    let mut out = Schematic::default();
    let add_port = |list: &mut Vec<String>, port: &str| {
        if !list.iter().any(|p| p == port) {
            list.push(port.to_owned());
        }
    };
    for p in &scene.animation.rig.pieces {
        let inputs = piece_inputs(p);
        let mut node = Node {
            key: piece_key(p.id),
            label: rig::label(&p.kind),
            id: p.id,
            piece: true,
            kind: Some(p.kind.clone()),
            inputs: Vec::new(),
            outputs: vec!["out".into()],
            broken: broken.contains(&p.id) || p.is_missing(),
            column: 1,
        };
        if p.kind.starts_with("mod.") {
            // Unwired inputs a modifier names (A, B...) as ports too.
            let named: &[&str] = match p.kind.as_str() {
                "mod.blend" => &["A", "B", "Blend"],
                "mod.math" => &["A", "B"],
                "mod.clamp" => &["A", "Min", "Max"],
                "mod.switch" => &["Index", "A", "B"],
                _ => &[],
            };
            for n in named {
                add_port(&mut node.inputs, n);
            }
        }
        for (port, item, item_port) in &inputs {
            add_port(&mut node.inputs, port);
            let e = item_ports.entry(*item).or_default();
            add_port(&mut e.1, item_port);
            e.2 = true;
            out.wires.push(Wire {
                from: item_key(*item),
                from_port: item_port.clone(),
                to: piece_key(p.id),
                to_port: port.clone(),
                piece: p.id,
            });
        }
        if let Some((item, port)) = piece_output(p) {
            let e = item_ports.entry(item).or_default();
            add_port(&mut e.0, &port);
            e.3 = true;
            out.wires.push(Wire {
                from: piece_key(p.id),
                from_port: "out".into(),
                to: item_key(item),
                to_port: port,
                piece: p.id,
            });
        } else {
            item_ports.entry(p.item).or_default();
        }
        out.nodes.push(node);
    }
    for (item, (inputs, outputs, drives, driven)) in item_ports {
        let label = scene
            .item(item)
            .map_or_else(|| format!("item {item} (missing)"), |i| i.name.clone());
        out.nodes.push(Node {
            key: item_key(item),
            label,
            id: item,
            piece: false,
            kind: None,
            inputs,
            outputs,
            broken: scene.item(item).is_none(),
            column: match (drives, driven) {
                (true, false) => 0,
                _ => 2,
            },
        });
    }
    out
}

/// `operandi.rig.schematic`: the model as JSON (UIC-163).
pub fn answer(
    e: &crate::Editor,
    name: &str,
    _: &serde_json::Map<String, serde_json::Value>,
) -> Option<Result<String, String>> {
    (name == "operandi.rig.schematic")
        .then(|| serde_json::to_string(&build(&e.scene)).map_err(|e| e.to_string()))
}
