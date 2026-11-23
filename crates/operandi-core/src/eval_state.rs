//! The evaluation engine's per-document state (EVAL-GRAPH-PROPOSAL step
//! 5): the scene's graph, rebuilt when the scene's structure changes.
//!
//! An `Editor` field listed under **Document** in document_state.rs, so
//! each scene tab keeps its own graph. It is a cache: never undo data,
//! never saved, and two editors compare equal whatever it holds (AR-04,
//! AR-15). Clones (an editor's staged copy) share it, as they share the
//! MeshOps cache; it is keyed by the structure fingerprint, so a stale
//! entry is never read for another structure.
//!
//! The graph holds structure only so far (who reads whom). Values still
//! come from posing and evaluation as before; the proposal's later steps
//! move them in.
use crate::Scene;
use crate::eval_build::{SceneGraph, build, structure};
use std::sync::{Arc, Mutex};

/// A graph with the structure fingerprint it was built for.
type Cached = Option<(u64, Arc<SceneGraph>)>;

#[derive(Clone, Default)]
pub struct EvalState {
    cache: Arc<Mutex<Cached>>,
}

impl PartialEq for EvalState {
    /// A cache, not document state.
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for EvalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EvalState")
    }
}

impl EvalState {
    /// `scene`'s graph: the cached one while its structure is unchanged,
    /// else built afresh.
    pub fn graph(&self, scene: &Scene) -> Arc<SceneGraph> {
        let key = structure(scene);
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((k, g)) = cache.as_ref()
            && *k == key
        {
            return g.clone();
        }
        let g = Arc::new(build(scene));
        *cache = Some((key, g.clone()));
        g
    }

    /// Whether a graph for `scene`'s structure is cached (tests).
    pub fn cached_for(&self, scene: &Scene) -> bool {
        let key = structure(scene);
        self.cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|(k, _)| *k == key)
    }
}

/// The handler behind the registry entry: a query changes nothing.
pub fn run(
    _: &mut crate::Editor,
    _: &str,
    _: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<crate::Command>, String> {
    Ok(None)
}

/// `operandi.eval.graph`: the graph as JSON, nodes and edges (UIC-163, for
/// tests now and the Schematic later, AR-21).
pub fn answer(
    e: &crate::Editor,
    name: &str,
    a: &serde_json::Map<String, serde_json::Value>,
) -> Option<Result<String, String>> {
    if name != "operandi.eval.graph" {
        return None;
    }
    let item = match a.get("item") {
        None | Some(serde_json::Value::Null) => None,
        Some(v) => match v.as_u64() {
            Some(id) => Some(id),
            None => return Some(Err("item is an item id".into())),
        },
    };
    let g = e.eval.graph(&e.scene);
    let keep = |k: &crate::eval_graph::NodeKey| item.is_none() || k.item() == item;
    let nodes: Vec<serde_json::Value> = g
        .describe_nodes()
        .into_iter()
        .filter(|(_, k, _)| keep(k))
        .map(|(id, k, dirty)| {
            let (kind, name) = match k {
                crate::eval_graph::NodeKey::Time => ("time", None),
                crate::eval_graph::NodeKey::Channel(_, n) => ("channel", Some(n.as_str())),
                crate::eval_graph::NodeKey::Field(_, n) => ("field", Some(n.as_str())),
                crate::eval_graph::NodeKey::Transform(_) => ("transform", None),
                crate::eval_graph::NodeKey::ShownMesh(_) => ("shownMesh", None),
            };
            serde_json::json!({
                "id": id.index(),
                "kind": kind,
                "item": k.item(),
                "name": name,
                "label": k.label(),
                "dirty": dirty,
            })
        })
        .collect();
    let edges: Vec<[usize; 2]> = g
        .describe_edges()
        .into_iter()
        .filter(|(from, to)| keep(g.key(*from)) || keep(g.key(*to)))
        .map(|(from, to)| [from.index(), to.index()])
        .collect();
    Some(Ok(
        serde_json::json!({"nodes": nodes, "edges": edges}).to_string()
    ))
}
