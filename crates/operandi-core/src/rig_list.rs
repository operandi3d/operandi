//! What the Items list shows under an item for its rig (RG-01, IK-02,
//! SK-36): each rig piece on it (constraints, dynamic parents, links,
//! modifiers, IK solvers) and, on a skinned mesh, its skin with one row per
//! joint influence. Goals and poles are items, so they are rows already.
//! Plus `operandi.rig.rename`, the name a piece shows (the kind's label
//! until renamed), one undo step.
use crate::rig::{self, Piece};
use crate::{Command, Editor, Result, Scene};
use serde_json::{Map, Value};

type Args = Map<String, Value>;

/// One row under an item.
#[derive(Clone, Debug, PartialEq)]
pub enum RigRow {
    Piece {
        id: u64,
        kind: String,
        name: String,
        enabled: bool,
        /// Not built by this Operandi (a kind it doesn't know).
        missing: bool,
    },
    /// A skin piece in the item's Mesh Stack, expandable to its joints.
    Skin {
        op: u64,
        enabled: bool,
        influences: Vec<Influence>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Influence {
    pub joint: u64,
    pub name: String,
    pub map: String,
    pub enabled: bool,
    /// The joint is gone.
    pub missing: bool,
}

/// The name a piece shows.
pub fn name(p: &Piece) -> String {
    p.names
        .get("label")
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| rig::label(&p.kind))
}

/// The rows under item `item`, in order: its pieces (evaluation order),
/// then its skin.
pub fn rows(scene: &Scene, item: u64) -> Vec<RigRow> {
    let mut out: Vec<RigRow> = scene
        .animation
        .rig
        .on(item)
        .map(|p| RigRow::Piece {
            id: p.id,
            kind: p.kind.clone(),
            name: name(p),
            enabled: p.flag("enable"),
            missing: p.is_missing(),
        })
        .collect();
    if let Some(stack) = scene.item(item).and_then(|i| i.mesh_stack.as_ref()) {
        for op in stack
            .ops
            .iter()
            .filter(|o| o.kind.type_id() == crate::skin::SKIN_OP)
        {
            let Ok(v) = serde_json::from_value::<crate::skin::SkinValues>(op.kind.values()) else {
                continue;
            };
            let influences = v
                .joints
                .iter()
                .enumerate()
                .map(|(k, j)| Influence {
                    joint: *j,
                    name: scene
                        .item(*j)
                        .map(|i| i.name.clone())
                        .unwrap_or_else(|| v.maps[k].clone()),
                    map: v.maps[k].clone(),
                    enabled: v.enabled[k],
                    missing: scene.item(*j).is_none(),
                })
                .collect();
            out.push(RigRow::Skin {
                op: op.id,
                enabled: op.enabled,
                influences,
            });
        }
    }
    out
}

/// `operandi.rig.rename`.
pub fn run(e: &mut Editor, _name: &str, a: &Args) -> Result<Option<Command>> {
    let piece = match a.get("piece") {
        Some(Value::Number(n)) => n.as_u64(),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
    .ok_or("Give piece")?;
    let p = e
        .scene
        .animation
        .rig
        .piece(piece)
        .cloned()
        .ok_or(format!("No rig piece {piece}"))?;
    let new = a
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_owned();
    if new.len() > 1024 {
        return Err("Name must be at most 1024 bytes".into());
    }
    e.anim_step("Rename Rig Piece", |e| {
        let q = e.scene.animation.rig.piece_mut(piece).expect("found");
        // An empty name, or the kind's label, goes back to the label.
        if new.is_empty() || new == rig::label(&p.kind) {
            Ok(q.names.remove("label").is_some())
        } else {
            let changed = q.names.get("label") != Some(&new);
            q.names.insert("label".into(), new.clone());
            Ok(changed)
        }
    })?;
    Ok(None)
}
