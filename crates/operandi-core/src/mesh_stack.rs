//! Mesh Stack v1: a procedural stack of operations on a mesh item
//! (specifications/meshops-2026-11-11, MO-nn behaviour, SAFE-nn safety).
//!
//! The data model. `Item::mesh` stays the **Base Mesh**; `Item::mesh_stack`
//! holds the stack (MO-01). What the viewport, render and export show is
//! the stack's result, computed here and held by `Runtime`, outside the
//! scene: never in undo checkpoints, `PartialEq` or files (SAFE-03, MO-05).
//!
//! Evaluation. `evaluate_stack` runs Base Mesh first, then each op on the
//! result of the one below (MO-02). Each op runs inside `guarded_op`: a
//! panic, a bad value or an over-limit result fails only that op, which
//! then passes its input through (SAFE-04, Q2). Results are cached per op;
//! changing an op re-runs only it and the ops above (MO-03). Every op is a
//! pure function of its input mesh, its own values and its inputs (linked
//! items' results, and the time for ops that use it), so the same recipe
//! always gives the same mesh (SAFE-01, SAFE-11).
//!
//! Op types. Each type is an entry in the op table (`meshop_types.rs`):
//! Operandi's own in `mesh_stack_builtin.rs`, kits' added at run time. This
//! module never names a particular op type.
//!
//! Links. An op can read other mesh items (Merge Meshes, Boolean): its
//! type lists their ids ([`OpDef::links`]). The runtime evaluates those
//! items first and hands their results, placed in this item's space, to
//! the op. A loop is refused when made and broken on load (SAFE-07); a
//! link to a deleted item fails only that op (SAFE-08); hidden items
//! evaluate as usual (SAFE-09).
use crate::meshop_selection::SelOp;
use crate::meshop_types::{Linked, OpContext, OpDef, Source, op_def};
pub use crate::meshop_types::{op_defs, register};
use crate::time_tick::Tick;
use crate::{Mesh, Result, Shared};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// One stack (MO-01, MO-11). `ops[0]` runs first, right above the Base
/// Mesh; the list shows them the other way up (top = last evaluated).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshStack {
    pub ops: Vec<MeshOp>,
    /// Next op id. Ids name rows for commands and the list; they are never
    /// reused within a stack.
    pub next_id: u64,
}

/// One operation in a stack.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshOp {
    pub id: u64,
    /// The row's eye (MO-12). Off passes the input straight through.
    pub enabled: bool,
    pub kind: OpKind,
    /// The op's Selection input (MO-13, MO-50): v1 has Select by Index of
    /// polygons only. `None` works on the whole mesh (MO-20).
    pub selection: Option<SelectByIndex>,
    /// Selection ops after the Select by Index, bottom to top (MO-50).
    pub selops: Vec<SelOp>,
}

/// MO-53 Select by Index: stored polygon numbers, copied into the op when
/// captured (SAFE-06). They can point at other polygons once something
/// below changes the topology; the Properties warn about it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectByIndex {
    pub polygons: Vec<u32>,
}
impl SelectByIndex {
    /// `0-9,12,18-30` form (MO-53).
    pub fn text(&self) -> String {
        let mut sorted = self.polygons.clone();
        sorted.sort_unstable();
        sorted.dedup();
        let mut out = Vec::new();
        let mut i = 0;
        while i < sorted.len() {
            let start = sorted[i];
            let mut end = start;
            while i + 1 < sorted.len() && sorted[i + 1] == end + 1 {
                i += 1;
                end = sorted[i];
            }
            out.push(if start == end {
                start.to_string()
            } else {
                format!("{start}-{end}")
            });
            i += 1;
        }
        out.join(",")
    }
    /// Parses the `0-9,12` form. Spaces are allowed; an empty text is an
    /// empty selection.
    pub fn parse(text: &str) -> Result<Self> {
        const LIMIT: u64 = 10_000_000;
        let mut polygons = Vec::new();
        for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let bad = || format!("“{part}” isn't a polygon number or a range like 0-9");
            let (a, b) = match part.split_once('-') {
                Some((a, b)) => (a.trim(), b.trim()),
                None => (part, part),
            };
            let a: u32 = a.parse().map_err(|_| bad())?;
            let b: u32 = b.parse().map_err(|_| bad())?;
            if b < a {
                return Err(bad());
            }
            if polygons.len() as u64 + u64::from(b - a) + 1 > LIMIT {
                return Err("Select by Index holds at most 10 million polygons".into());
            }
            polygons.extend(a..=b);
        }
        polygons.sort_unstable();
        polygons.dedup();
        Ok(Self { polygons })
    }
}

/// What an op is: a type from the op table with its values, or a
/// placeholder for a type this build doesn't know (yet) or whose values
/// didn't load (SAFE-12).
#[derive(Clone, Debug, PartialEq)]
pub enum OpKind {
    /// A known type. `values` were read by its [`OpDef::read`]: every
    /// value present, with the right names and types.
    Op {
        type_id: String,
        values: serde_json::Value,
    },
    /// An op type this build doesn't know, or one whose values didn't load.
    /// It keeps its data, passes through and saves back unchanged. If a kit
    /// later registers the type, it runs (K7).
    Placeholder {
        type_id: String,
        version: u32,
        values: serde_json::Value,
        reason: String,
    },
}

impl OpKind {
    /// A new op of type `id` with the direct tool's defaults (MO-40).
    pub fn new(id: &str) -> Result<Self> {
        let def = op_def(id).ok_or_else(|| format!("“{id}” isn't a mesh operation"))?;
        Ok(Self::Op {
            type_id: id.into(),
            values: def.defaults(),
        })
    }
    /// The op's table entry; `None` for a placeholder.
    pub fn def(&self) -> Option<Arc<dyn OpDef>> {
        match self {
            Self::Op { type_id, .. } => op_def(type_id),
            Self::Placeholder { .. } => None,
        }
    }
    pub fn type_id(&self) -> &str {
        match self {
            Self::Op { type_id, .. } | Self::Placeholder { type_id, .. } => type_id,
        }
    }
    pub fn label(&self) -> String {
        match self.def() {
            Some(d) => d.label().into(),
            None => format!("Unknown ({})", self.type_id()),
        }
    }
    pub fn takes_selection(&self) -> bool {
        self.def().is_some_and(|d| d.takes_selection())
    }
    fn version(&self) -> u32 {
        match self {
            Self::Placeholder { version, .. } => *version,
            Self::Op { .. } => self.def().map_or(1, |d| d.version()),
        }
    }
    /// Every value, never "default" (MO-90, SAFE-12).
    pub fn values(&self) -> serde_json::Value {
        match self {
            Self::Op { values, .. } | Self::Placeholder { values, .. } => values.clone(),
        }
    }
    /// The items this op reads in `scene` (SAFE-07); none for a
    /// placeholder.
    pub fn links(&self, scene: &crate::Scene) -> Vec<u64> {
        match (self, self.def()) {
            (Self::Op { values, .. }, Some(d)) => d.links(values, scene),
            _ => Vec::new(),
        }
    }
    /// Rebuilds an op from its saved form. Never fails: an unknown type, a
    /// newer version or values that don't fit load as a placeholder.
    pub fn from_raw(type_id: &str, version: u32, values: serde_json::Value) -> Self {
        let placeholder = |reason: String| Self::Placeholder {
            type_id: type_id.into(),
            version,
            values: values.clone(),
            reason,
        };
        let Some(def) = op_def(type_id) else {
            return placeholder(format!(
                "Operandi doesn't know the mesh operation “{type_id}”"
            ));
        };
        match def.read(version, &values) {
            Ok(values) => Self::Op {
                type_id: type_id.into(),
                values,
            },
            Err(e) if e.starts_with("Saved by") => placeholder(e),
            Err(e) => placeholder(format!("Its values didn't load: {e}")),
        }
    }
    /// A placeholder whose type a kit has registered since it loaded, read
    /// again (K7). Anything else comes back as it is.
    fn revived(&self) -> Cow<'_, Self> {
        match self {
            Self::Placeholder {
                type_id,
                version,
                values,
                ..
            } if op_def(type_id).is_some() => {
                Cow::Owned(Self::from_raw(type_id, *version, values.clone()))
            }
            _ => Cow::Borrowed(self),
        }
    }
    /// Sets some values by name (`meshop.set`): merges `changes` into the
    /// op's values and re-reads them, so an unknown name or a wrong type is
    /// refused and nothing changes.
    pub fn with_values(
        &self,
        changes: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<Self> {
        if let Self::Placeholder { reason, .. } = self {
            return Err(format!("This operation can't be edited here: {reason}"));
        }
        let mut values = self.values();
        let map = values
            .as_object_mut()
            .ok_or("This operation has no values")?;
        for (k, v) in changes {
            if !map.contains_key(k) {
                let names: Vec<_> = map.keys().cloned().collect();
                return Err(format!(
                    "{} has no value “{k}” (it has {})",
                    self.label(),
                    if names.is_empty() {
                        "none".into()
                    } else {
                        names.join(", ")
                    }
                ));
            }
            map.insert(k.clone(), v.clone());
        }
        match Self::from_raw(self.type_id(), self.version(), values) {
            Self::Placeholder { reason, .. } => Err(reason),
            next => Ok(next),
        }
    }
}

// ---- saving (MO-90, SAFE-12) ----

/// The saved form of one op: type, its own version, every value.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOp {
    #[serde(rename = "type")]
    type_id: String,
    version: u32,
    id: u64,
    enabled: bool,
    values: serde_json::Value,
    /// Select by Index, `0-9,12` form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    select_by_index: Option<String>,
    /// Selection ops (native version 50).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    selops: Vec<SelOp>,
}
/// Mirror and Array are saved as two items (registry rev 3, confirmed): a
/// Clone Effector op carrying the copy channels (merge, dist) with a
/// `pmodel.mirror` / `pmodel.array` generator in its Tool Pipe carrying the
/// rest. Operandi keeps them as one op with flat values; these two turn
/// one form into the other.
const CLONE_EFFECTOR: &str = "effector.clone.item";
const EFFECTOR_CHANNELS: &[&str] = &["merge", "dist", "replace", "flip"];
fn clone_generator(menu_id: &str) -> Option<&'static str> {
    match menu_id {
        "mirror.clone.item" => Some("pmodel.mirror"),
        "array.clone.item" => Some("pmodel.array"),
        "radial.array.item" => Some("pmodel.helix"),
        "linear.clone.item" => Some("pmodel.linear"),
        _ => None,
    }
}
/// (saved type, saved values) for an op.
fn saved_form(kind: &OpKind) -> (String, serde_json::Value) {
    let values = kind.values();
    let Some(generator) =
        clone_generator(kind.type_id()).filter(|_| !matches!(kind, OpKind::Placeholder { .. }))
    else {
        return (kind.type_id().into(), values);
    };
    let mut effector = serde_json::Map::new();
    let mut tool = serde_json::Map::new();
    for (k, v) in values.as_object().into_iter().flatten() {
        if EFFECTOR_CHANNELS.contains(&k.as_str()) {
            effector.insert(k.clone(), v.clone());
        } else {
            tool.insert(k.clone(), v.clone());
        }
    }
    effector.insert(
        "toolPipe".into(),
        serde_json::json!({ "type": generator, "values": tool }),
    );
    (CLONE_EFFECTOR.into(), serde_json::Value::Object(effector))
}
/// The op a saved form names. A Clone Effector becomes Mirror or Array by
/// its generator; anything it can't read stays a placeholder that saves
/// back unchanged (SAFE-12).
fn read_saved(type_id: &str, version: u32, values: serde_json::Value) -> OpKind {
    if type_id != CLONE_EFFECTOR {
        return OpKind::from_raw(type_id, version, values);
    }
    let unread = |reason: &str| OpKind::Placeholder {
        type_id: type_id.into(),
        version,
        values: values.clone(),
        reason: reason.into(),
    };
    let Some(map) = values.as_object() else {
        return unread("A Clone Effector without values");
    };
    let generator = map
        .get("toolPipe")
        .and_then(|t| t.get("type"))
        .and_then(|t| t.as_str());
    let menu_id = match generator {
        Some("pmodel.mirror") => "mirror.clone.item",
        Some("pmodel.array") => "array.clone.item",
        Some("pmodel.helix") => "radial.array.item",
        Some("pmodel.linear") => "linear.clone.item",
        Some(other) => {
            return unread(&format!(
                "Operandi doesn't know the Clone Effector generator “{other}”"
            ));
        }
        None => return unread("A Clone Effector with no generator in its Tool Pipe"),
    };
    let mut flat = serde_json::Map::new();
    for (k, v) in map {
        if k != "toolPipe" {
            flat.insert(k.clone(), v.clone());
        }
    }
    let tool = map["toolPipe"].get("values").and_then(|v| v.as_object());
    for (k, v) in tool.into_iter().flatten() {
        flat.insert(k.clone(), v.clone());
    }
    match OpKind::from_raw(menu_id, version, serde_json::Value::Object(flat)) {
        OpKind::Placeholder { reason, .. } => unread(&reason),
        kind => kind,
    }
}

impl Serialize for MeshOp {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let (type_id, values) = saved_form(&self.kind);
        RawOp {
            type_id,
            version: self.kind.version(),
            id: self.id,
            enabled: self.enabled,
            values,
            select_by_index: self.selection.as_ref().map(SelectByIndex::text),
            selops: self.selops.clone(),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for MeshOp {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let raw = RawOp::deserialize(d)?;
        let selection = raw
            .select_by_index
            .as_deref()
            .map(SelectByIndex::parse)
            .transpose()
            .map_err(serde::de::Error::custom)?;
        Ok(Self {
            id: raw.id,
            enabled: raw.enabled,
            kind: read_saved(&raw.type_id, raw.version, raw.values),
            selection,
            selops: raw.selops,
        })
    }
}

impl MeshStack {
    pub fn index_of(&self, op: u64) -> Option<usize> {
        self.ops.iter().position(|o| o.id == op)
    }
    /// Checks a stack read from a file or script (SAFE-07/10/12): unique
    /// ids below `next_id`. Op values are checked when they run.
    pub fn validate(&self) -> Result<()> {
        let mut seen = std::collections::BTreeSet::new();
        for op in &self.ops {
            if op.id >= self.next_id || !seen.insert(op.id) {
                return Err("A mesh operation stack has repeated or out-of-range ids".into());
            }
        }
        if self.ops.len() > 10_000 {
            return Err("A mesh operation stack holds at most 10,000 operations".into());
        }
        Ok(())
    }
}

// ---- running one op ----

/// Output limits (SAFE-10). Provisional fixed values; the spec asks for
/// preferences.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub max_polygons: usize,
    pub max_points: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_polygons: 4_000_000,
            max_points: 4_000_000,
        }
    }
}

/// How one op ended (SAFE-04): ran, was off, or failed and passed its
/// input through with a short reason for its red row.
#[derive(Clone, Debug, PartialEq)]
pub enum OpStatus {
    Ok,
    Off,
    Failed(String),
}

/// The op on `ctx.input`, without the guard. Values are re-checked by the
/// op because a file or script can bypass the panel (SAFE-10).
fn run_op(op: &MeshOp, links: &[Linked], time: Tick, input: &Mesh) -> Result<Mesh> {
    let kind = op.kind.revived();
    let (def, values) = match (&*kind, kind.def()) {
        (OpKind::Op { values, .. }, Some(def)) => (def, values),
        (OpKind::Placeholder { reason, .. }, _) => return Err(reason.clone()),
        (OpKind::Op { type_id, .. }, None) => {
            return Err(format!(
                "Operandi doesn't know the mesh operation “{type_id}”"
            ));
        }
    };
    // Edit ops on a mesh with no polygons have nothing to do.
    if def.takes_selection() && input.faces.is_empty() {
        return Ok(input.clone());
    }
    let selection = if def.takes_selection() {
        crate::meshop_selection::resolve(op.selection.as_ref(), &op.selops, input)?
    } else {
        None
    };
    // A Selection input that picked nothing: nothing to do (PG-04).
    if selection.as_ref().is_some_and(Vec::is_empty) {
        return Ok(input.clone());
    }
    let (clean, groups) = crate::meshop_selection::split_groups(input);
    let ctx = OpContext {
        input: &clean,
        op: op.id,
        selection: selection.as_deref(),
        links,
        time,
    };
    let mut out = def.run(&ctx, values)?;
    crate::meshop_selection::carry_groups(&groups, &clean, &mut out);
    Ok(out)
}

/// SAFE-04: the op inside a guard. A panic, an error, an invalid result or
/// one over the limits is a failure with a short reason; nothing escapes.
/// `links` are the op's resolved item links (see [`OpDef::links`]).
pub fn guarded_op(
    input: &Mesh,
    op: &MeshOp,
    links: &[Linked],
    time: Tick,
    limits: Limits,
) -> std::result::Result<Mesh, String> {
    let out = match catch_unwind(AssertUnwindSafe(|| run_op(op, links, time, input))) {
        Ok(Ok(mesh)) => mesh,
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("It stopped unexpectedly (an internal error)".into()),
    };
    if out.faces.len() > limits.max_polygons || out.vertices.len() > limits.max_points {
        return Err(format!(
            "Its result would have {} polygons and {} points, over the limit of {}",
            out.faces.len(),
            out.vertices.len(),
            limits.max_polygons
        ));
    }
    match catch_unwind(AssertUnwindSafe(|| out.validate())) {
        Ok(Ok(())) => Ok(out),
        Ok(Err(e)) => Err(format!("It made an invalid mesh: {e}")),
        Err(_) => Err("Its result couldn't be checked (an internal error)".into()),
    }
}

// ---- links between items (SAFE-07..09) ----

/// The first enabled op whose item links close a loop, as (item, op)
/// (SAFE-07). Disabled ops don't evaluate, so they can't loop; enabling
/// one checks again.
pub fn find_loop(scene: &crate::Scene) -> Option<(u64, u64)> {
    // Item → (op, linked item) for every enabled op with links.
    let mut edges: HashMap<u64, Vec<(u64, u64)>> = HashMap::new();
    for item in &scene.items {
        for op in item.mesh_stack.iter().flat_map(|s| &s.ops) {
            if op.enabled {
                for to in op.kind.links(scene) {
                    edges.entry(item.id).or_default().push((op.id, to));
                }
            }
        }
    }
    // Depth-first, 1 = on the current path, 2 = done.
    let mut state: HashMap<u64, u8> = HashMap::new();
    fn visit(
        at: u64,
        edges: &HashMap<u64, Vec<(u64, u64)>>,
        state: &mut HashMap<u64, u8>,
    ) -> Option<(u64, u64)> {
        state.insert(at, 1);
        for &(op, to) in edges.get(&at).into_iter().flatten() {
            match state.get(&to) {
                Some(1) => return Some((at, op)),
                Some(_) => {}
                None => {
                    if let Some(found) = visit(to, edges, state) {
                        return Some(found);
                    }
                }
            }
        }
        state.insert(at, 2);
        None
    }
    let mut starts: Vec<u64> = edges.keys().copied().collect();
    starts.sort_unstable();
    for start in starts {
        if !state.contains_key(&start)
            && let Some(found) = visit(start, &edges, &mut state)
        {
            return Some(found);
        }
    }
    None
}

/// SAFE-07 on load: turns off the op that closes each loop, so the scene
/// opens. Returns one line per op turned off.
pub fn break_loops(scene: &mut crate::Scene) -> Vec<String> {
    let mut notes = Vec::new();
    while let Some((item, op)) = find_loop(scene) {
        let Some(i) = scene.item_mut(item) else { break };
        let name = i.name.clone();
        let Some(o) = i
            .mesh_stack
            .as_mut()
            .and_then(|s| s.ops.iter_mut().find(|o| o.id == op))
        else {
            break;
        };
        o.enabled = false;
        notes.push(format!(
            "{} on {name} was turned off: its inputs made a dependency loop",
            o.kind.label()
        ));
    }
    notes
}

/// The reason an op shows for a link to an item that isn't a mesh any
/// more (SAFE-08).
pub const MISSING_INPUT: &str = "Missing input: the linked mesh item is gone";
/// The reason for a link back into itself (SAFE-07).
pub const LOOP_INPUT: &str = "Dependency loop: an input reads this mesh's own result";

/// What an op read besides its own values and the mesh below. Equal
/// inputs let its cached result stand (MO-03).
#[derive(Clone, Debug, Default)]
struct OpInputs {
    links: Vec<Linked>,
    /// The scene time, only for ops whose result depends on it.
    time: Option<Tick>,
}
impl OpInputs {
    fn same(&self, other: &Self) -> bool {
        self.time == other.time && Linked::same(&self.links, &other.links)
    }
    fn time(&self) -> Tick {
        self.time.unwrap_or_default()
    }
}

// ---- running a stack, with per-op caching (MO-02, MO-03) ----

#[derive(Clone)]
struct Entry {
    op: MeshOp,
    inputs: OpInputs,
    output: Shared<Mesh>,
    status: OpStatus,
}

/// The cached results of one item's stack: `entries[i]` is the result of
/// `ops[0..=i]` on `base`.
#[derive(Clone)]
struct Chain {
    base: Shared<Mesh>,
    entries: Vec<Entry>,
    /// The top result without group sets, made once so readers (links,
    /// the viewport) keep seeing the same shared mesh while it's current.
    shown: Shared<Mesh>,
}
impl Chain {
    /// How many ops at the bottom still have valid results for this recipe.
    fn valid_prefix(&self, base: &Shared<Mesh>, ops: &[MeshOp], inputs: &[OpInputs]) -> usize {
        if !Shared::same(&self.base, base) {
            return 0;
        }
        self.entries
            .iter()
            .zip(ops.iter().zip(inputs))
            .take_while(|(e, (op, inputs))| &e.op == *op && e.inputs.same(inputs))
            .count()
    }
    fn complete(&self, base: &Shared<Mesh>, ops: &[MeshOp], inputs: &[OpInputs]) -> bool {
        self.entries.len() == ops.len() && self.valid_prefix(base, ops, inputs) == ops.len()
    }
}

/// The stack's result and how each op ended, in `ops` order.
#[derive(Clone, Debug)]
pub struct Evaluated {
    pub mesh: Shared<Mesh>,
    pub statuses: Vec<OpStatus>,
    /// False while a newer recipe is still being evaluated (this is the
    /// last good result, SAFE-04).
    pub current: bool,
}

/// Runs `ops` on `base`, reusing `reuse`'s results for the unchanged ops
/// at the bottom. Returns `None` when cancelled (SAFE-02).
fn run_chain(
    base: &Shared<Mesh>,
    ops: &[MeshOp],
    inputs: &[OpInputs],
    reuse: Option<&Chain>,
    cancel: &AtomicBool,
    limits: Limits,
    counter: &AtomicU64,
) -> Option<Chain> {
    let keep = reuse.map_or(0, |c| c.valid_prefix(base, ops, inputs));
    let mut entries: Vec<Entry> = reuse.map_or_else(Vec::new, |c| c.entries[..keep].to_vec());
    for (op, op_inputs) in ops[keep..].iter().zip(&inputs[keep..]) {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let input = entries
            .last()
            .map_or_else(|| base.clone(), |e| e.output.clone());
        let (output, status) = if !op.enabled {
            (input, OpStatus::Off)
        } else {
            counter.fetch_add(1, Ordering::Relaxed);
            match guarded_op(&input, op, &op_inputs.links, op_inputs.time(), limits) {
                Ok(mesh) => {
                    let mesh = Shared::new(mesh);
                    // Checked above; later readers skip the walk.
                    mesh.record_checked();
                    (mesh, OpStatus::Ok)
                }
                // Pass the input through (SAFE-04, Q2).
                Err(reason) => (input, OpStatus::Failed(reason)),
            }
        };
        entries.push(Entry {
            op: op.clone(),
            inputs: op_inputs.clone(),
            output,
            status,
        });
    }
    let top = entries
        .last()
        .map_or_else(|| base.clone(), |e| e.output.clone());
    Some(Chain {
        base: base.clone(),
        shown: crate::meshop_selection::strip_groups(&top),
        entries,
    })
}

/// Evaluates a stack from scratch, synchronously, with no cache and no
/// scene: item links are missing inputs. Tests, the stress harness and
/// determinism checks use it; [`Runtime::current`] evaluates with links.
pub fn evaluate_stack(base: &Shared<Mesh>, stack: &MeshStack) -> Evaluated {
    evaluate_stack_within(base, stack, Limits::default())
}
/// `evaluate_stack` with other output limits.
pub fn evaluate_stack_within(base: &Shared<Mesh>, stack: &MeshStack, limits: Limits) -> Evaluated {
    let no_scene = crate::Scene::default();
    let inputs: Vec<OpInputs> = stack
        .ops
        .iter()
        .map(|op| OpInputs {
            links: op
                .kind
                .revived()
                .links(&no_scene)
                .into_iter()
                .map(|item| Linked {
                    item,
                    source: Err(MISSING_INPUT.into()),
                })
                .collect(),
            time: None,
        })
        .collect();
    let chain = run_chain(
        base,
        &stack.ops,
        &inputs,
        None,
        &AtomicBool::new(false),
        limits,
        &AtomicU64::new(0),
    )
    .expect("an uncancelled run finishes");
    evaluated(&chain, true)
}
fn evaluated(chain: &Chain, current: bool) -> Evaluated {
    Evaluated {
        // Group sets are for ops above only (meshop_selection.rs).
        mesh: chain.shown.clone(),
        statuses: chain.entries.iter().map(|e| e.status.clone()).collect(),
        current,
    }
}

// ---- the runtime: cache and background evaluation (SAFE-02, SAFE-15) ----

struct Job {
    base: Shared<Mesh>,
    ops: Vec<MeshOp>,
    inputs: Vec<OpInputs>,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct State {
    chains: HashMap<u64, Chain>,
    jobs: HashMap<u64, Job>,
    background: bool,
    waker: Option<Arc<dyn Fn() + Send + Sync>>,
    limits: Limits,
}

/// Which result a link reads: what the viewport shows (background, never
/// waits) or the current one (render, export, Freeze).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Want {
    Display,
    Final,
}

/// Evaluated results, held by the editor but never part of the scene
/// (SAFE-03). Clones share one cache: an editor's staged copy reads and
/// fills the same results, which are pure functions of their recipes.
#[derive(Clone, Default)]
pub struct Runtime {
    state: Arc<Mutex<State>>,
    /// Ops run so far (tests check MO-03 with it).
    runs: Arc<AtomicU64>,
}
impl PartialEq for Runtime {
    /// The cache is not document state.
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("mesh_stack::Runtime")
    }
}
impl Runtime {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
    /// Background mode: `result` never blocks; evaluations run on worker
    /// threads and `waker` asks for a repaint when one lands.
    pub fn set_background(&self, on: bool, waker: Option<Arc<dyn Fn() + Send + Sync>>) {
        let mut s = self.lock();
        s.background = on;
        s.waker = waker;
    }
    /// Turns background mode on once, with the waker `waker` makes.
    pub fn ensure_background(&self, waker: impl FnOnce() -> Arc<dyn Fn() + Send + Sync>) {
        let mut s = self.lock();
        if !s.background {
            s.background = true;
            s.waker = Some(waker());
        }
    }
    /// Output limits for every later evaluation (SAFE-10).
    pub fn set_limits(&self, limits: Limits) {
        let mut s = self.lock();
        s.limits = limits;
        s.chains.clear();
    }
    /// Ops run so far.
    pub fn op_runs(&self) -> u64 {
        self.runs.load(Ordering::Relaxed)
    }
    /// Whether an evaluation is running.
    pub fn busy(&self) -> bool {
        !self.lock().jobs.is_empty()
    }

    /// One link of `reader`'s op, resolved (SAFE-07..09): the linked
    /// item's shown result in `reader`'s space. Hidden items evaluate the
    /// same way (SAFE-09). `None` while that result is still being worked
    /// out in the background: the reader then waits rather than flash a
    /// failure.
    fn resolve(
        &self,
        scene: &crate::Scene,
        reader: u64,
        id: u64,
        want: Want,
        visiting: &mut Vec<u64>,
    ) -> Option<Linked> {
        let fail = |reason: &str| {
            Some(Linked {
                item: id,
                source: Err(reason.into()),
            })
        };
        if id == reader || visiting.contains(&id) {
            return fail(LOOP_INPUT);
        }
        // Joints link too, for the skin (skin.rs): their mesh is empty.
        let Some(source) = scene
            .item(id)
            .filter(|i| (i.is_mesh() || i.is_joint()) && i.kit.is_none())
        else {
            return fail(MISSING_INPUT);
        };
        let mesh = if source.mesh_stack.is_some() {
            visiting.push(reader);
            let result = match want {
                Want::Display => self.result_in(scene, source, visiting),
                Want::Final => self.current_in(scene, source, visiting),
            };
            visiting.pop();
            match result {
                Some(e) if e.current => e.mesh,
                _ => return None,
            }
        } else {
            source.mesh.shared().clone()
        };
        let place = scene.world_matrix(id).and_then(|world| {
            let into = scene.world_to_local(reader)?;
            let rest = crate::skin::setup_world(scene, id).map_or(into * world, |w| into * w);
            Ok((into * world, world, rest))
        });
        let (to_local, world, rest) = match place {
            Ok(m) => m,
            Err(e) => return fail(&e),
        };
        Some(Linked {
            item: id,
            source: Ok(Source {
                item: id,
                name: source.name.clone(),
                mesh,
                to_local,
                world,
                rest,
            }),
        })
    }
    /// Every op's inputs; `None` while a linked result is pending.
    fn inputs_for(
        &self,
        scene: &crate::Scene,
        item: &crate::Item,
        ops: &[MeshOp],
        want: Want,
        visiting: &mut Vec<u64>,
    ) -> Option<Vec<OpInputs>> {
        let mut out = Vec::with_capacity(ops.len());
        for op in ops {
            if !op.enabled {
                out.push(OpInputs::default());
                continue;
            }
            let kind = op.kind.revived();
            let time = kind
                .def()
                .filter(|d| d.uses_time())
                .map(|_| scene.time.current);
            let mut links = Vec::new();
            for id in kind.links(scene) {
                links.push(self.resolve(scene, item.id, id, want, visiting)?);
            }
            out.push(OpInputs { links, time });
        }
        Some(out)
    }

    /// The result for `item` (an item with a stack, in `scene` or a probe
    /// copy of one). In the background mode a changed recipe starts an
    /// evaluation and this returns the last good result meanwhile, or
    /// `None` before the first one (SAFE-17 placeholder). Synchronous mode
    /// always returns a current result.
    pub fn result(&self, scene: &crate::Scene, item: &crate::Item) -> Option<Evaluated> {
        self.result_in(scene, item, &mut Vec::new())
    }
    fn result_in(
        &self,
        scene: &crate::Scene,
        item: &crate::Item,
        visiting: &mut Vec<u64>,
    ) -> Option<Evaluated> {
        let stack = item.mesh_stack.as_ref()?;
        let last = |s: &State| s.chains.get(&item.id).map(|c| evaluated(c, false));
        let Some(inputs) = self.inputs_for(scene, item, &stack.ops, Want::Display, visiting) else {
            // A linked item is still evaluating: keep the last result.
            return last(&self.lock());
        };
        let mut s = self.lock();
        let limits = s.limits;
        if let Some(chain) = s.chains.get(&item.id)
            && chain.complete(item.mesh.shared(), &stack.ops, &inputs)
        {
            return Some(evaluated(chain, true));
        }
        if !s.background {
            let reuse = s.chains.get(&item.id).cloned();
            drop(s);
            let chain = run_chain(
                item.mesh.shared(),
                &stack.ops,
                &inputs,
                reuse.as_ref(),
                &AtomicBool::new(false),
                limits,
                &self.runs,
            )?;
            let out = evaluated(&chain, true);
            self.lock().chains.insert(item.id, chain);
            return Some(out);
        }
        let running = s.jobs.get(&item.id).is_some_and(|j| {
            Shared::same(&j.base, item.mesh.shared())
                && j.ops == stack.ops
                && j.inputs.len() == inputs.len()
                && j.inputs.iter().zip(&inputs).all(|(a, b)| a.same(b))
        });
        if !running {
            // A newer recipe cancels the older evaluation (SAFE-02).
            if let Some(old) = s.jobs.remove(&item.id) {
                old.cancel.store(true, Ordering::Relaxed);
            }
            let cancel = Arc::new(AtomicBool::new(false));
            s.jobs.insert(
                item.id,
                Job {
                    base: item.mesh.clone().into(),
                    ops: stack.ops.clone(),
                    inputs: inputs.clone(),
                    cancel: cancel.clone(),
                },
            );
            let reuse = s.chains.get(&item.id).cloned();
            let (runtime, id, base, ops) =
                (self.clone(), item.id, item.mesh.clone(), stack.ops.clone());
            let spawned = std::thread::Builder::new()
                .name("mesh_stack".into())
                .spawn(move || {
                    let chain = run_chain(
                        base.shared(),
                        &ops,
                        &inputs,
                        reuse.as_ref(),
                        &cancel,
                        limits,
                        &runtime.runs,
                    );
                    runtime.finish(id, &cancel, chain);
                });
            if spawned.is_err() {
                s.jobs.remove(&item.id);
            }
        }
        last(&s)
    }
    /// A worker's result: kept only if it is still the newest request for
    /// the item; a stale one is dropped (SAFE-02).
    fn finish(&self, id: u64, cancel: &Arc<AtomicBool>, chain: Option<Chain>) {
        let waker = {
            let mut s = self.lock();
            let newest = s
                .jobs
                .get(&id)
                .is_some_and(|j| Arc::ptr_eq(&j.cancel, cancel));
            if !newest {
                return;
            }
            s.jobs.remove(&id);
            if let Some(chain) = chain {
                s.chains.insert(id, chain);
            }
            s.waker.clone()
        };
        if let Some(wake) = waker {
            wake();
        }
    }
    /// Blocks until no evaluation is running (tests, save, Freeze from a
    /// script). The UI never calls it.
    pub fn wait(&self) {
        while self.busy() {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    /// The current result, evaluating on this thread if needed (linked
    /// items too). Freeze and export use it: they need the result of the
    /// recipe as it is now (SAFE-14), never a stale one.
    pub fn current(&self, scene: &crate::Scene, item: &crate::Item) -> Option<Evaluated> {
        self.current_in(scene, item, &mut Vec::new())
    }
    fn current_in(
        &self,
        scene: &crate::Scene,
        item: &crate::Item,
        visiting: &mut Vec<u64>,
    ) -> Option<Evaluated> {
        let stack = item.mesh_stack.as_ref()?;
        let inputs = self.inputs_for(scene, item, &stack.ops, Want::Final, visiting)?;
        let (reuse, limits) = {
            let s = self.lock();
            (s.chains.get(&item.id).cloned(), s.limits)
        };
        if let Some(c) = &reuse
            && c.complete(item.mesh.shared(), &stack.ops, &inputs)
        {
            return Some(evaluated(c, true));
        }
        let chain = run_chain(
            item.mesh.shared(),
            &stack.ops,
            &inputs,
            reuse.as_ref(),
            &AtomicBool::new(false),
            limits,
            &self.runs,
        )?;
        let out = evaluated(&chain, true);
        // Keep it: the next request for the same recipe reuses it. A
        // background job for an older recipe can't overwrite it with a
        // wrong answer, since every reuse re-checks the recipe (MO-03).
        self.lock().chains.insert(item.id, chain);
        Some(out)
    }
    /// Forget results of items that are gone (SAFE-15: deleting an item
    /// frees its cache) and of items that no longer have a stack.
    pub fn retain(&self, scene: &crate::Scene) {
        let mut s = self.lock();
        let live = |id: &u64| {
            scene
                .items
                .iter()
                .any(|i| i.id == *id && i.mesh_stack.is_some())
        };
        s.chains.retain(|id, _| live(id));
        let gone: Vec<u64> = s.jobs.keys().copied().filter(|id| !live(id)).collect();
        for id in gone {
            if let Some(job) = s.jobs.remove(&id) {
                job.cancel.store(true, Ordering::Relaxed);
            }
        }
    }

    /// The scene as it should be drawn, rendered and exported: each item
    /// with a stack shows its result (MO-01), except `base_edit`, which
    /// shows its Base Mesh (MO-27). `None` when nothing differs from
    /// `scene`. Items shown by their result drop their component selection,
    /// lock and hide sets, which number the Base Mesh's elements
    /// (SAFE-06).
    pub fn display_scene(
        &self,
        scene: &crate::Scene,
        base_edit: Option<u64>,
    ) -> Option<crate::Scene> {
        self.substituted(scene, base_edit, false)
    }
    /// The scene with every stack's current result, evaluated now on this
    /// thread if needed: renders and exports never use a stale result.
    pub fn result_scene(&self, scene: &crate::Scene) -> Option<crate::Scene> {
        self.substituted(scene, None, true)
    }
    fn substituted(
        &self,
        scene: &crate::Scene,
        base_edit: Option<u64>,
        current: bool,
    ) -> Option<crate::Scene> {
        if !scene
            .items
            .iter()
            .any(|i| i.mesh_stack.is_some() && Some(i.id) != base_edit)
        {
            return None;
        }
        self.retain(scene);
        let mut out = scene.clone();
        for (index, item) in out.items.iter_mut().enumerate() {
            if item.mesh_stack.is_none() || Some(item.id) == base_edit {
                continue;
            }
            let original = &scene.items[index];
            if original
                .mesh_stack
                .as_ref()
                .is_some_and(crate::skin::only_skin)
            {
                out.animation
                    .rig
                    .skin_bases
                    .insert(item.id, original.mesh.clone());
            }
            let result = if current {
                self.current(scene, original)
            } else {
                self.result(scene, original)
            };
            item.mesh = match result {
                Some(e) => e.mesh.into(),
                None => Default::default(),
            };
            item.elements = Default::default();
            if out.selection.item == Some(item.id) {
                out.selection.vertices.clear();
                out.selection.edges.clear();
                out.selection.faces.clear();
            }
            out.selection.others.remove(&item.id);
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(id: u64, kind: OpKind) -> MeshOp {
        MeshOp {
            id,
            enabled: true,
            kind,
            selection: None,
            selops: Vec::new(),
        }
    }
    fn stack(kinds: Vec<OpKind>) -> MeshStack {
        let n = kinds.len() as u64;
        MeshStack {
            ops: kinds
                .into_iter()
                .enumerate()
                .map(|(i, k)| op(i as u64, k))
                .collect(),
            next_id: n,
        }
    }
    /// Sets one op value through the checked path.
    fn set(kind: &mut OpKind, key: &str, value: f64) {
        let mut change = serde_json::Map::new();
        change.insert(key.into(), value.into());
        *kind = kind.with_values(&change).unwrap();
    }
    fn empty() -> Shared<Mesh> {
        Shared::new(Mesh::default())
    }

    #[test]
    fn index_text_round_trips() {
        let s = SelectByIndex::parse("18-30, 0-9,12,12").unwrap();
        assert_eq!(s.text(), "0-9,12,18-30");
        assert_eq!(SelectByIndex::parse(&s.text()).unwrap(), s);
        assert!(SelectByIndex::parse("3-1").is_err());
        assert!(SelectByIndex::parse("x").is_err());
        assert_eq!(
            SelectByIndex::parse("").unwrap().polygons,
            Vec::<u32>::new()
        );
    }

    #[test]
    fn cube_then_bevel_runs_bottom_to_top() {
        let mut s = stack(vec![
            OpKind::new("prim.cube.item").unwrap(),
            OpKind::new("poly.bevel.item").unwrap(),
        ]);
        s.ops[1].selection = Some(SelectByIndex { polygons: vec![0] });
        let e = evaluate_stack(&empty(), &s);
        assert_eq!(e.statuses, vec![OpStatus::Ok, OpStatus::Ok]);
        // One cap beveled: the cube's 6 plus 4 walls.
        assert_eq!(e.mesh.faces.len(), 10);
    }

    #[test]
    fn bevel_on_a_whole_closed_mesh_bevels_each_polygon() {
        let s = stack(vec![
            OpKind::new("prim.cube.item").unwrap(),
            OpKind::new("poly.bevel.item").unwrap(),
        ]);
        let e = evaluate_stack(&empty(), &s);
        assert_eq!(e.statuses, vec![OpStatus::Ok, OpStatus::Ok]);
        assert_eq!(e.mesh.faces.len(), 6 * 5);
    }

    #[test]
    fn order_matters() {
        let a = stack(vec![
            OpKind::new("prim.cube.item").unwrap(),
            OpKind::new("pmodel.triangulate.item").unwrap(),
            OpKind::new("subdivide.tool.item").unwrap(),
        ]);
        let b = stack(vec![
            OpKind::new("prim.cube.item").unwrap(),
            OpKind::new("subdivide.tool.item").unwrap(),
            OpKind::new("pmodel.triangulate.item").unwrap(),
        ]);
        let (ea, eb) = (evaluate_stack(&empty(), &a), evaluate_stack(&empty(), &b));
        assert_ne!(ea.mesh.faces.len(), eb.mesh.faces.len());
    }

    #[test]
    fn a_failing_op_passes_through_and_the_rest_run() {
        let mut s = stack(vec![
            OpKind::new("prim.cube.item").unwrap(),
            OpKind::new("poly.bevel.item").unwrap(),
            OpKind::new("pmodel.triangulate.item").unwrap(),
        ]);
        // A polygon number past the end fails the bevel (SAFE-06).
        s.ops[1].selection = Some(SelectByIndex {
            polygons: vec![999],
        });
        let e = evaluate_stack(&empty(), &s);
        assert!(matches!(e.statuses[1], OpStatus::Failed(_)));
        assert_eq!(e.statuses[2], OpStatus::Ok);
        assert_eq!(e.mesh.faces.len(), 12, "the cube, triangulated");
    }

    #[test]
    fn a_disabled_op_passes_through() {
        let mut s = stack(vec![
            OpKind::new("prim.cube.item").unwrap(),
            OpKind::new("pmodel.triangulate.item").unwrap(),
        ]);
        s.ops[1].enabled = false;
        let e = evaluate_stack(&empty(), &s);
        assert_eq!(e.statuses[1], OpStatus::Off);
        assert_eq!(e.mesh.faces.len(), 6);
    }

    #[test]
    fn select_by_index_out_of_range_fails_the_op() {
        let mut s = stack(vec![
            OpKind::new("prim.cube.item").unwrap(),
            OpKind::new("pmodel.triangulate.item").unwrap(),
        ]);
        s.ops[1].selection = Some(SelectByIndex {
            polygons: vec![2, 40],
        });
        let e = evaluate_stack(&empty(), &s);
        assert!(matches!(&e.statuses[1], OpStatus::Failed(r) if r.contains("40")));
        s.ops[1].selection = Some(SelectByIndex { polygons: vec![2] });
        assert_eq!(evaluate_stack(&empty(), &s).mesh.faces.len(), 7);
    }

    #[test]
    fn unknown_op_loads_as_placeholder_and_saves_back_unchanged() {
        let json =
            r#"{"type":"future.thing","version":3,"id":0,"enabled":true,"values":{"a":[1,2]}}"#;
        let op: MeshOp = serde_json::from_str(json).unwrap();
        assert!(matches!(op.kind, OpKind::Placeholder { .. }));
        let back = serde_json::to_value(&op).unwrap();
        assert_eq!(
            back,
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
        let s = MeshStack {
            ops: vec![op],
            next_id: 1,
        };
        let e = evaluate_stack(&empty(), &s);
        assert!(matches!(e.statuses[0], OpStatus::Failed(_)));
    }

    #[test]
    fn mirror_and_array_save_as_a_clone_effector_with_a_generator() {
        for (menu, generator, tool_key) in [
            ("mirror.clone.item", "pmodel.mirror", "axis"),
            ("array.clone.item", "pmodel.array", "numX"),
        ] {
            let op = op(3, OpKind::new(menu).unwrap());
            let v = serde_json::to_value(&op).unwrap();
            assert_eq!(v["type"], "effector.clone.item");
            assert_eq!(v["values"]["toolPipe"]["type"], generator);
            assert!(v["values"]["merge"].is_boolean() && v["values"]["dist"].is_number());
            assert!(v["values"]["toolPipe"]["values"][tool_key].is_number());
            assert!(v["values"]["toolPipe"]["values"].get("merge").is_none());
            let back: MeshOp = serde_json::from_value(v).unwrap();
            assert_eq!(back, op);
        }
    }

    #[test]
    fn v1_flat_mirror_and_array_still_load() {
        let json = r#"{"type":"array.clone.item","version":1,"id":0,"enabled":true,
            "values":{"numX":3,"numY":1,"numZ":1,"offX":1.0,"offY":0.0,"offZ":0.0,"merge":false}}"#;
        let op: MeshOp = serde_json::from_str(json).unwrap();
        assert_eq!(op.kind.type_id(), "array.clone.item");
        assert_eq!(op.kind.values()["numX"], 3);
        assert_eq!(op.kind.values()["dist"], 0.);
        let v = serde_json::to_value(&op).unwrap();
        assert_eq!(v["type"], "effector.clone.item", "saved in the new form");
    }

    #[test]
    fn an_unknown_clone_generator_saves_back_unchanged() {
        let json = r#"{"type":"effector.clone.item","version":1,"id":0,"enabled":true,
            "values":{"merge":true,"dist":0.0,"toolPipe":{"type":"pmodel.radial","values":{"n":4}}}}"#;
        let op: MeshOp = serde_json::from_str(json).unwrap();
        assert!(matches!(op.kind, OpKind::Placeholder { .. }));
        let back = serde_json::to_value(&op).unwrap();
        assert_eq!(
            back,
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    #[test]
    fn every_value_is_saved() {
        for t in op_defs() {
            let op = op(0, OpKind::new(t.id()).unwrap());
            let v = serde_json::to_value(&op).unwrap();
            let back: MeshOp = serde_json::from_value(v.clone()).unwrap();
            assert_eq!(back, op, "{}", t.id());
            assert!(v["values"].is_object(), "{}", t.id());
        }
    }

    #[test]
    fn changing_the_top_op_reruns_only_it() {
        let runtime = Runtime::default();
        let mut scene = crate::Scene::default();
        scene.new_mesh().unwrap();
        let mut item = scene.items[0].clone();
        item.mesh_stack = Some(stack(vec![
            OpKind::new("prim.cube.item").unwrap(),
            OpKind::new("subdivide.tool.item").unwrap(),
            OpKind::new("poly.bevel.item").unwrap(),
        ]));
        runtime.result(&scene, &item).unwrap();
        assert_eq!(runtime.op_runs(), 3);
        runtime.result(&scene, &item).unwrap();
        assert_eq!(runtime.op_runs(), 3, "cached");
        set(
            &mut item.mesh_stack.as_mut().unwrap().ops[2].kind,
            "inset",
            0.05,
        );
        runtime.result(&scene, &item).unwrap();
        assert_eq!(runtime.op_runs(), 4, "only the edited op re-ran");
    }

    #[test]
    fn background_results_arrive_and_stale_ones_are_dropped() {
        let runtime = Runtime::default();
        runtime.set_background(true, None);
        let mut scene = crate::Scene::default();
        scene.new_mesh().unwrap();
        let mut item = scene.items[0].clone();
        item.mesh_stack = Some(stack(vec![OpKind::new("prim.cube.item").unwrap()]));
        let first = runtime.result(&scene, &item);
        assert!(first.is_none_or(|e| !e.current));
        // A newer recipe before the first finishes.
        for k in ["sizeX", "sizeY", "sizeZ"] {
            set(&mut item.mesh_stack.as_mut().unwrap().ops[0].kind, k, 2.);
        }
        runtime.result(&scene, &item);
        runtime.wait();
        let e = runtime.result(&scene, &item).unwrap();
        assert!(e.current);
        let x = e.mesh.vertices.iter().map(|p| p.x).fold(0., f64::max);
        assert_eq!(x, 1., "the newer size won");
    }

    #[test]
    fn deterministic() {
        let s = stack(vec![
            OpKind::new("prim.sphere.item").unwrap(),
            OpKind::new("poly.smshift.item").unwrap(),
            OpKind::new("mirror.clone.item").unwrap(),
            OpKind::new("array.clone.item").unwrap(),
            OpKind::new("subdivide.tool.item").unwrap(),
        ]);
        let a = evaluate_stack(&empty(), &s);
        let b = evaluate_stack(&empty(), &s);
        assert_eq!(*a.mesh, *b.mesh);
        assert!(
            a.statuses.iter().all(|s| *s == OpStatus::Ok),
            "{:?}",
            a.statuses
        );
    }
}
