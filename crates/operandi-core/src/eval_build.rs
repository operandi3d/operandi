//! A scene's evaluation graph (EVAL-GRAPH-PROPOSAL step 2): the nodes and
//! edges [`eval_graph`](crate::eval_graph) needs, read from the scene.
//!
//! - `Time` is the one source everything time-dependent reads.
//! - Each animated channel is a `Channel` node; it reads `Time` when its
//!   curve can change over time (two or more keys that differ, or a
//!   cycling or extrapolating curve), and its item's posed `Field` reads it.
//! - An item with animated transform channels has a `Transform` node
//!   reading those fields.
//! - An item with a Mesh Stack has a `ShownMesh` node reading its own
//!   transform, `Time` when any op uses the time, and each linked item's
//!   `ShownMesh` and `Transform` (Merge Meshes, Boolean: their results are
//!   placed in this item's space).
//!
//! [`structure`] fingerprints exactly what this reads, so a caller rebuilds
//! only when items, channels or links change, not on every edit.
use crate::Scene;
use crate::eval_graph::{EvalGraph, NodeKey};
use std::hash::{Hash, Hasher};

/// What a node holds: a channel's or field's number, or a change counter
/// for nodes whose value lives elsewhere (a shown mesh in the Mesh Stack
/// cache).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Changed(u64),
}

pub type SceneGraph = EvalGraph<Value>;

/// Whether a channel's value can change with time.
fn time_dependent(curve: &crate::anim_curve::Curve) -> bool {
    let Some(first) = curve.keys.first() else {
        return false;
    };
    curve.keys.iter().any(|k| k.value != first.value)
}

/// Whether a channel drives the item's transform (`pos.`, `rot.`, `scl.`).
fn transform_channel(name: &str) -> bool {
    ["pos.", "rot.", "scl."].iter().any(|p| name.starts_with(p))
}

/// The scene's graph. Fails only on a dependency loop, which Mesh Stack
/// already refuses when made (SAFE-07); a loaded loop is left out.
pub fn build(scene: &Scene) -> SceneGraph {
    let mut g = SceneGraph::new();
    let time = g.node(NodeKey::Time);
    for (key, channel) in &scene.animation.channels {
        if channel.curve.is_empty() {
            continue;
        }
        let ch = g.node(NodeKey::Channel(key.item, key.channel.clone()));
        if time_dependent(&channel.curve) {
            let _ = g.connect(time, ch);
        }
        let field = g.node(NodeKey::Field(key.item, key.channel.clone()));
        let _ = g.connect(ch, field);
        if transform_channel(&key.channel) {
            let transform = g.node(NodeKey::Transform(key.item));
            let _ = g.connect(field, transform);
        }
    }
    // Parents and rig pieces (RG-01): a transform reads its parent's and
    // its pieces' targets'; a piece's animated channels feed it too.
    for item in &scene.items {
        if let Some(parent) = item.parent {
            let theirs = g.node(NodeKey::Transform(parent));
            let mine = g.node(NodeKey::Transform(item.id));
            let _ = g.connect(theirs, mine);
        }
    }
    for p in &scene.animation.rig.pieces {
        if p.is_missing() {
            continue;
        }
        let mine = g.node(NodeKey::Transform(p.item));
        for &t in p.targets.iter().chain(p.refs.values()) {
            let theirs = g.node(NodeKey::Transform(t));
            // A loop (a loaded one; commands refuse them) is left out.
            let _ = g.connect(theirs, mine);
        }
        for name in crate::rig::channel_names(p) {
            if let Some(field) = g.id(&NodeKey::Field(p.id, name)) {
                let _ = g.connect(field, mine);
            }
        }
    }
    for item in &scene.items {
        let Some(stack) = &item.mesh_stack else {
            continue;
        };
        let shown = g.node(NodeKey::ShownMesh(item.id));
        let own = g.node(NodeKey::Transform(item.id));
        let _ = g.connect(own, shown);
        for op in stack.ops.iter().filter(|op| op.enabled) {
            if op.kind.def().is_some_and(|d| d.uses_time()) {
                let _ = g.connect(time, shown);
            }
            for link in op.kind.links(scene) {
                if link == item.id {
                    continue;
                }
                let theirs = g.node(NodeKey::ShownMesh(link));
                let _ = g.connect(theirs, shown);
                let placed = g.node(NodeKey::Transform(link));
                let _ = g.connect(placed, shown);
            }
        }
    }
    g
}

/// A fingerprint of everything [`build`] reads: items with stacks and
/// their links, animated channels and whether each depends on time. Equal
/// fingerprints build equal graphs.
pub fn structure(scene: &Scene) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for (key, channel) in &scene.animation.channels {
        key.item.hash(&mut h);
        key.channel.hash(&mut h);
        channel.curve.is_empty().hash(&mut h);
        time_dependent(&channel.curve).hash(&mut h);
    }
    for item in &scene.items {
        item.parent.hash(&mut h);
    }
    for p in &scene.animation.rig.pieces {
        p.id.hash(&mut h);
        p.kind.hash(&mut h);
        p.item.hash(&mut h);
        p.targets.hash(&mut h);
        p.refs.hash(&mut h);
    }
    for item in &scene.items {
        if let Some(stack) = &item.mesh_stack {
            item.id.hash(&mut h);
            for op in stack.ops.iter().filter(|op| op.enabled) {
                op.kind.def().is_some_and(|d| d.uses_time()).hash(&mut h);
                op.kind.links(scene).hash(&mut h);
            }
        }
    }
    h.finish()
}
