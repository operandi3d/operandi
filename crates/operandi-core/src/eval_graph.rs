//! The evaluation graph (master plan F2 / AR-03; design in
//! docs/design/EVAL-GRAPH-PROPOSAL.md, step 1): who reads whom, which
//! results are out of date, and recomputing only those.
//!
//! A node is one value someone can ask for (a channel's value now, an
//! item's shown mesh). An edge says a node reads another. Editing a source
//! marks it dirty and everything downstream of it; asking for a node
//! ([`EvalGraph::pull`]) recomputes the dirty nodes it depends on, inputs
//! first, and nothing else. A loop is refused when an edge would make one.
//!
//! The graph is a cache built from the scene: never undo data, never saved
//! (AR-04, AR-15). This module is the bare data structure and knows
//! nothing about scenes; `eval_build` (step 2) makes the nodes and edges
//! for a scene.
use std::collections::HashMap;

/// A node's index in its graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u32);

impl NodeId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// What a node stands for. The same key always names the same node of a
/// graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeKey {
    /// The document's current time.
    Time,
    /// An animated channel's value at the current time (`item`, `pos.Y`).
    Channel(u64, String),
    /// An item field as posed (what a channel writes).
    Field(u64, String),
    /// An item's transform (later: its world transform through parents).
    Transform(u64),
    /// An item's shown mesh: Base Mesh, MeshOps, later deformers (AR-05).
    ShownMesh(u64),
}

impl NodeKey {
    /// The item the node belongs to, if any.
    pub fn item(&self) -> Option<u64> {
        match self {
            NodeKey::Time => None,
            NodeKey::Channel(i, _)
            | NodeKey::Field(i, _)
            | NodeKey::Transform(i)
            | NodeKey::ShownMesh(i) => Some(*i),
        }
    }
    /// A short label, e.g. `Channel 3 pos.Y` (Schematic, debugging).
    pub fn label(&self) -> String {
        match self {
            NodeKey::Time => "Time".into(),
            NodeKey::Channel(i, n) => format!("Channel {i} {n}"),
            NodeKey::Field(i, n) => format!("Field {i} {n}"),
            NodeKey::Transform(i) => format!("Transform {i}"),
            NodeKey::ShownMesh(i) => format!("Shown Mesh {i}"),
        }
    }
}

/// Nodes, edges, dirty bits and cached values of type `V`.
#[derive(Clone, Debug)]
pub struct EvalGraph<V> {
    keys: Vec<NodeKey>,
    index: HashMap<NodeKey, NodeId>,
    /// `inputs[n]`: the nodes `n` reads, in the order they were connected
    /// (its compute sees their values in this order).
    inputs: Vec<Vec<NodeId>>,
    /// `outputs[n]`: the nodes that read `n`.
    outputs: Vec<Vec<NodeId>>,
    dirty: Vec<bool>,
    values: Vec<Option<V>>,
    /// Nodes computed so far (tests count work with it).
    computes: u64,
}

impl<V> Default for EvalGraph<V> {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            index: HashMap::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            dirty: Vec::new(),
            values: Vec::new(),
            computes: 0,
        }
    }
}

impl<V: Clone> EvalGraph<V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The node for `key`, made (dirty, no value) if it isn't there.
    pub fn node(&mut self, key: NodeKey) -> NodeId {
        if let Some(&id) = self.index.get(&key) {
            return id;
        }
        let id = NodeId(self.keys.len() as u32);
        self.index.insert(key.clone(), id);
        self.keys.push(key);
        self.inputs.push(Vec::new());
        self.outputs.push(Vec::new());
        self.dirty.push(true);
        self.values.push(None);
        id
    }

    /// The node for `key`, if the graph has one.
    pub fn id(&self, key: &NodeKey) -> Option<NodeId> {
        self.index.get(key).copied()
    }

    pub fn key(&self, id: NodeId) -> &NodeKey {
        &self.keys[id.index()]
    }

    /// The nodes `id` reads.
    pub fn inputs(&self, id: NodeId) -> &[NodeId] {
        &self.inputs[id.index()]
    }

    /// The nodes that read `id`.
    pub fn outputs(&self, id: NodeId) -> &[NodeId] {
        &self.outputs[id.index()]
    }

    /// `to` reads `from`. Refused when `from` already reads `to` (directly
    /// or through others), which would make a loop; connecting twice is
    /// one edge. `to` becomes dirty (it has a new input).
    pub fn connect(&mut self, from: NodeId, to: NodeId) -> Result<(), String> {
        if self.inputs[to.index()].contains(&from) {
            return Ok(());
        }
        if from == to || self.reaches(to, from) {
            return Err(format!(
                "Dependency loop: {} would read {}, which reads it",
                self.key(to).label(),
                self.key(from).label()
            ));
        }
        self.inputs[to.index()].push(from);
        self.outputs[from.index()].push(to);
        self.mark_dirty(to);
        Ok(())
    }

    /// Whether `to` is downstream of `from` (reads it, directly or not).
    pub fn reaches(&self, from: NodeId, to: NodeId) -> bool {
        // Visited set sized by the walk, not the graph: connecting a new
        // node (no outputs yet) costs nothing.
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![from];
        while let Some(n) = stack.pop() {
            if n == to {
                return true;
            }
            if !seen.insert(n) {
                continue;
            }
            stack.extend_from_slice(&self.outputs[n.index()]);
        }
        false
    }

    /// Marks `id` and everything downstream dirty. The walk stops at
    /// nodes already dirty (their downstream is dirty already), so a run
    /// of edits costs what it newly marks. Returns how many it marked.
    pub fn mark_dirty(&mut self, id: NodeId) -> usize {
        let mut marked = 0;
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            if std::mem::replace(&mut self.dirty[n.index()], true) {
                continue;
            }
            marked += 1;
            stack.extend_from_slice(&self.outputs[n.index()]);
        }
        marked
    }

    /// Marks the node for `key` dirty, if the graph has one.
    pub fn mark_key_dirty(&mut self, key: &NodeKey) -> usize {
        self.id(key).map_or(0, |id| self.mark_dirty(id))
    }

    /// Every node dirty (after undo restored a whole scene, say).
    pub fn mark_all_dirty(&mut self) {
        self.dirty.iter_mut().for_each(|d| *d = true);
    }

    pub fn is_dirty(&self, id: NodeId) -> bool {
        self.dirty[id.index()]
    }

    /// The dirty nodes, in id order.
    pub fn dirty_nodes(&self) -> Vec<NodeId> {
        (0..self.len())
            .filter(|&n| self.dirty[n])
            .map(|n| NodeId(n as u32))
            .collect()
    }

    /// The cached value of `id`, even if it is dirty (stale).
    pub fn cached(&self, id: NodeId) -> Option<&V> {
        self.values[id.index()].as_ref()
    }

    /// Sets a node's value from outside (a source such as Time) and marks
    /// what reads it dirty; the node itself is clean.
    pub fn set(&mut self, id: NodeId, value: V) {
        self.values[id.index()] = Some(value);
        let downstream = self.outputs[id.index()].clone();
        for n in downstream {
            self.mark_dirty(n);
        }
        self.dirty[id.index()] = false;
    }

    /// The value of `id`, recomputing it and the dirty nodes it reads
    /// (inputs first) with `compute(key, input values)`. Clean nodes keep
    /// their cached values; each dirty node computes once. Iterative, so
    /// long chains (rigs) don't run out of stack.
    pub fn pull(&mut self, id: NodeId, compute: &mut dyn FnMut(&NodeKey, &[&V]) -> V) -> V {
        // Post-order over the dirty part of `id`'s upstream.
        let mut order = Vec::new();
        // Visit state by node, only for nodes the walk touches (a pull of
        // a clean node costs nothing): absent new, false open, true done.
        let mut state: HashMap<NodeId, bool> = HashMap::new();
        let mut stack = vec![(id, false)];
        while let Some((n, children_done)) = stack.pop() {
            let i = n.index();
            if children_done {
                state.insert(n, true);
                order.push(n);
                continue;
            }
            if state.contains_key(&n) {
                continue;
            }
            if !self.dirty[i] && self.values[i].is_some() {
                state.insert(n, true);
                continue;
            }
            state.insert(n, false);
            stack.push((n, true));
            for &input in self.inputs[i].iter().rev() {
                if !state.contains_key(&input) {
                    stack.push((input, false));
                }
            }
        }
        for n in order {
            let i = n.index();
            let value = {
                let inputs: Vec<&V> = self.inputs[i]
                    .iter()
                    .map(|p| {
                        self.values[p.index()]
                            .as_ref()
                            .expect("inputs are computed first")
                    })
                    .collect();
                compute(&self.keys[i], &inputs)
            };
            self.values[i] = Some(value);
            self.dirty[i] = false;
            self.computes += 1;
        }
        self.values[id.index()]
            .clone()
            .expect("the pulled node is computed")
    }

    /// Nodes computed so far.
    pub fn computes(&self) -> u64 {
        self.computes
    }

    /// Every node with its key and whether it is dirty, for the Schematic
    /// and `operandi.eval.graph` (step 8).
    pub fn describe_nodes(&self) -> Vec<(NodeId, &NodeKey, bool)> {
        self.keys
            .iter()
            .enumerate()
            .map(|(n, k)| (NodeId(n as u32), k, self.dirty[n]))
            .collect()
    }

    /// Every edge as (read node, reader).
    pub fn describe_edges(&self) -> Vec<(NodeId, NodeId)> {
        self.inputs
            .iter()
            .enumerate()
            .flat_map(|(to, ins)| ins.iter().map(move |&from| (from, NodeId(to as u32))))
            .collect()
    }
}
