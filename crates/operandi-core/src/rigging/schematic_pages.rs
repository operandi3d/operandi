//! Schematic workspaces (BEHAVIOR-SCHEMATIC SC-01..17, SC-60, SC-73..75,
//! SC-91): named pages that show only the nodes placed on them, at the
//! places they were put, saved with the scene (`Rig::workspaces`, native
//! v67). Removing a node from a page never touches the item or piece.
//!
//! Pages the user hasn't made yet are shown without being saved:
//! - a scene with rig pieces and no workspace shows one page,
//!   "Workspace", holding every rig node, arranged (SC-74);
//! - when workspaces exist, rig pieces on none of them show on a page
//!   called "Unplaced", arranged (SC-73);
//! - a piece on no page that touches an item on a saved page shows on
//!   that page beside the item (SC-16, provisional: the reference's rule
//!   is still to be measured), until it is moved or the page arranged.
//!
//! Moving a node on an unsaved page saves the page first (the app sends
//! one `operandi.rig.workspaceAdd` with every node's place), so a page
//! is never re-arranged under the user once they touch it.
use crate::rig_schematic::{self, Node, Schematic};
use crate::schematic_layout::{self, Metrics};
use crate::{Command, Editor, Scene};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// A named page of nodes (SC-01).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub id: u64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<Placement>,
}

/// A node on a page (SC-91): which item or piece, and where.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    /// `item:<id>` or `piece:<id>`, as the Schematic's node keys.
    pub key: String,
    pub x: f32,
    pub y: f32,
    /// Moved by hand since the last Arrange: Arrange leaves it alone
    /// unless asked for it by name (SC-60).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
    /// An item node's channels shown as ports even when nothing is wired
    /// to them yet: channels dragged onto the page (#54).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<String>,
}

/// Which page a view shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PageId {
    Saved(u64),
    /// No workspace yet: every rig node (SC-74).
    All,
    /// Pieces on no workspace (SC-73).
    Unplaced,
}

/// A wire whose other end isn't on the page (SC-14): drawn as a marked
/// port, not a wire across the view.
#[derive(Clone, Debug, PartialEq)]
pub struct Stub {
    pub node: String,
    pub port: String,
    pub output: bool,
    /// The node at the other end.
    pub other: String,
}

/// What one page shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageView {
    /// The page's nodes and the wires between them.
    pub schematic: Schematic,
    /// Every node's top-left corner.
    pub positions: BTreeMap<String, [f32; 2]>,
    /// Nodes saved on the page (the rest are shown, not saved).
    pub saved: BTreeSet<String>,
    pub stubs: Vec<Stub>,
}

/// The pages to offer, in order, with their names (SC-02, SC-75).
pub fn pages(scene: &Scene) -> Vec<(PageId, String)> {
    let saved = &scene.animation.rig.workspaces;
    let mut out: Vec<(PageId, String)> = saved
        .iter()
        .map(|w| (PageId::Saved(w.id), w.name.clone()))
        .collect();
    let rig = !scene.animation.rig.pieces.is_empty();
    if saved.is_empty() {
        if rig {
            out.push((PageId::All, "Workspace".into()));
        }
    } else if !unplaced_pieces(scene).is_empty() {
        out.push((PageId::Unplaced, "Unplaced".into()));
    }
    out
}

/// Piece keys on no saved workspace.
fn unplaced_pieces(scene: &Scene) -> Vec<String> {
    let placed: BTreeSet<&str> = scene
        .animation
        .rig
        .workspaces
        .iter()
        .flat_map(|w| w.nodes.iter().map(|p| p.key.as_str()))
        .collect();
    scene
        .animation
        .rig
        .pieces
        .iter()
        .map(|p| format!("piece:{}", p.id))
        .filter(|k| !placed.contains(k.as_str()))
        .collect()
}

/// A node for an item no rig piece touches (placed by hand, SC-10).
fn item_node(scene: &Scene, id: u64) -> Option<Node> {
    let item = scene.item(id)?;
    Some(Node {
        key: format!("item:{id}"),
        label: item.name.clone(),
        id,
        piece: false,
        kind: None,
        detail: None,
        inputs: Vec::new(),
        outputs: Vec::new(),
        broken: false,
        column: 0,
    })
}

/// Whether `key` names an item or piece in the scene.
fn exists(scene: &Scene, key: &str) -> bool {
    match key.split_once(':') {
        Some(("item", id)) => id.parse().ok().and_then(|i| scene.item(i)).is_some(),
        Some(("piece", id)) => id
            .parse()
            .ok()
            .and_then(|i| scene.animation.rig.piece(i))
            .is_some(),
        _ => false,
    }
}

/// The nodes `keys` from the whole rig, the wires between them, and the
/// stubs of wires leaving them.
fn subset(full: &Schematic, scene: &Scene, keys: &BTreeSet<String>) -> (Schematic, Vec<Stub>) {
    let mut s = Schematic::default();
    for k in keys {
        if let Some(n) = full.nodes.iter().find(|n| &n.key == k) {
            s.nodes.push(n.clone());
        } else if let Some(id) = k.strip_prefix("item:").and_then(|i| i.parse().ok())
            && let Some(n) = item_node(scene, id)
        {
            s.nodes.push(n);
        }
    }
    let shown: BTreeSet<&str> = s.nodes.iter().map(|n| n.key.as_str()).collect();
    let mut stubs = Vec::new();
    for w in &full.wires {
        match (
            shown.contains(w.from.as_str()),
            shown.contains(w.to.as_str()),
        ) {
            (true, true) => s.wires.push(w.clone()),
            (true, false) => stubs.push(Stub {
                node: w.from.clone(),
                port: w.from_port.clone(),
                output: true,
                other: w.to.clone(),
            }),
            (false, true) => stubs.push(Stub {
                node: w.to.clone(),
                port: w.to_port.clone(),
                output: false,
                other: w.from.clone(),
            }),
            (false, false) => {}
        }
    }
    (s, stubs)
}

/// Pieces and the items they touch, for these piece keys.
fn with_their_items(full: &Schematic, pieces: &[String]) -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = pieces.iter().cloned().collect();
    for w in &full.wires {
        if keys.contains(&w.from) {
            keys.insert(w.to.clone());
        } else if keys.contains(&w.to) {
            keys.insert(w.from.clone());
        }
    }
    keys
}

/// What page `id` shows (SC-01, SC-14, SC-16, SC-73, SC-74).
pub fn page(scene: &Scene, id: PageId, m: &Metrics) -> PageView {
    let full = rig_schematic::build(scene);
    let mut view = PageView::default();
    match id {
        PageId::All | PageId::Unplaced => {
            let keys = if id == PageId::All {
                full.nodes.iter().map(|n| n.key.clone()).collect()
            } else {
                with_their_items(&full, &unplaced_pieces(scene))
            };
            let (s, stubs) = subset(&full, scene, &keys);
            view.positions = schematic_layout::layout(&s, m).positions;
            view.schematic = s;
            view.stubs = stubs;
        }
        PageId::Saved(ws) => {
            let Some(w) = scene.animation.rig.workspaces.iter().find(|w| w.id == ws) else {
                return view;
            };
            let mut keys: BTreeSet<String> = BTreeSet::new();
            for p in w.nodes.iter().filter(|p| exists(scene, &p.key)) {
                keys.insert(p.key.clone());
                view.positions.insert(p.key.clone(), [p.x, p.y]);
                view.saved.insert(p.key.clone());
            }
            // SC-16: a piece on no page beside an item of this one.
            let unplaced = unplaced_pieces(scene);
            let mut suggested: Vec<(String, String, bool)> = Vec::new();
            for piece in &unplaced {
                for wire in &full.wires {
                    let (item, left) = if &wire.from == piece {
                        (&wire.to, true)
                    } else if &wire.to == piece {
                        (&wire.from, false)
                    } else {
                        continue;
                    };
                    if view.saved.contains(item) {
                        suggested.push((piece.clone(), item.clone(), left));
                        break;
                    }
                }
            }
            for (piece, _, _) in &suggested {
                keys.insert(piece.clone());
            }
            let (mut s, stubs) = subset(&full, scene, &keys);
            // #54: channels dragged onto the page show as ports, both sides,
            // wired or not.
            for p in w.nodes.iter().filter(|p| !p.channels.is_empty()) {
                if let Some(n) = s.nodes.iter_mut().find(|n| n.key == p.key) {
                    for c in &p.channels {
                        for list in [&mut n.inputs, &mut n.outputs] {
                            if !list.contains(c) {
                                list.push(c.clone());
                            }
                        }
                    }
                }
            }
            let height = |k: &str| {
                s.nodes
                    .iter()
                    .find(|n| n.key == k)
                    .map_or(m.row_h, |n| m.height(n))
            };
            for (piece, item, left) in suggested {
                let at = view.positions[&item];
                let x = if left {
                    at[0] - m.node_w - m.gap_x * 0.5
                } else {
                    at[0] + m.node_w + m.gap_x * 0.5
                };
                let h = height(&piece);
                let mut y = at[1];
                // Step down past anything already there.
                while view.positions.iter().any(|(k, p)| {
                    (p[0] - x).abs() < m.node_w && y < p[1] + height(k) && p[1] < y + h
                }) {
                    y += m.row_h;
                }
                view.positions.insert(piece, [x, y]);
            }
            view.schematic = s;
            view.stubs = stubs;
        }
    }
    view
}

/// Places for `keys` laid out together as a block (SC-60): below the
/// nodes in `keep`, lined up with their left edge, or at the origin.
fn block(
    scene: &Scene,
    keys: &BTreeSet<String>,
    keep: &BTreeMap<String, [f32; 2]>,
    m: &Metrics,
) -> Vec<(String, [f32; 2])> {
    let full = rig_schematic::build(scene);
    let (s, _) = subset(&full, scene, keys);
    let l = schematic_layout::layout(&s, m);
    let height = |k: &str| {
        full.nodes
            .iter()
            .find(|n| n.key == k)
            .map_or(m.head_h + m.row_h + 6., |n| m.height(n))
    };
    let (left, top) = if keep.is_empty() {
        (0., 0.)
    } else {
        let left = keep.values().map(|p| p[0]).fold(f32::MAX, f32::min);
        let bottom = keep
            .iter()
            .map(|(k, p)| p[1] + height(k))
            .fold(f32::MIN, f32::max);
        (left, bottom + m.group_gap)
    };
    l.positions
        .into_iter()
        .map(|(k, p)| (k, [p[0] + left, p[1] + top]))
        .collect()
}

/// `keys` laid out together by data flow, the block's top-left at the
/// origin: places for nodes arriving where the user is looking (SC-13).
pub fn arranged(scene: &Scene, keys: &[String], m: &Metrics) -> Vec<(String, [f32; 2])> {
    let keys: BTreeSet<String> = keys.iter().cloned().collect();
    block(scene, &keys, &BTreeMap::new(), m)
}

type Args = Map<String, Value>;

fn text<'a>(a: &'a Args, name: &str) -> Result<Option<&'a str>, String> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.trim())),
        Some(_) => Err(format!("{name} is text")),
    }
}

fn workspace_arg(e: &Editor, a: &Args) -> Result<u64, String> {
    let id = match a.get("workspace") {
        Some(Value::Number(n)) => n.as_u64(),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
    .ok_or("Give workspace (its id; operandi.rig.workspaces lists them)")?;
    if e.scene.animation.rig.workspaces.iter().any(|w| w.id == id) {
        Ok(id)
    } else {
        Err(format!("No workspace {id}"))
    }
}

/// `nodes:item:3,piece:7`: node keys that exist.
fn node_keys(e: &Editor, a: &Args) -> Result<Option<Vec<String>>, String> {
    let Some(list) = text(a, "nodes")? else {
        return Ok(None);
    };
    let mut out = Vec::new();
    for k in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if !exists(&e.scene, k) {
            return Err(format!(
                "{k} isn't an item or rig piece (item:<id> or piece:<id>)"
            ));
        }
        out.push(k.to_owned());
    }
    Ok(Some(out))
}

/// `place:item:3@10/20,piece:7@-40/5`: nodes with their places.
fn places(e: &Editor, a: &Args) -> Result<Vec<(String, [f32; 2])>, String> {
    let Some(list) = text(a, "place")? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for part in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let bad = || format!("Places are key@x/y, such as item:3@10/20, not {part}");
        let (key, at) = part.split_once('@').ok_or_else(bad)?;
        let (x, y) = at.split_once('/').ok_or_else(bad)?;
        let (x, y): (f32, f32) = (
            x.trim().parse().map_err(|_| bad())?,
            y.trim().parse().map_err(|_| bad())?,
        );
        if !x.is_finite() || !y.is_finite() {
            return Err(bad());
        }
        let key = key.trim();
        if !exists(&e.scene, key) {
            return Err(format!("{key} isn't an item or rig piece"));
        }
        out.push((key.to_owned(), [x, y]));
    }
    Ok(out)
}

/// Puts `places` on workspace `ws`, moving nodes already there.
fn put(w: &mut Workspace, places: &[(String, [f32; 2])], pinned: bool) -> bool {
    let mut changed = false;
    for (key, [x, y]) in places {
        let mut p = Placement {
            key: key.clone(),
            x: *x,
            y: *y,
            pinned,
            channels: Vec::new(),
        };
        let old = w.nodes.iter_mut().find(|n| &n.key == key);
        // Moving a node keeps the channels shown on it (#54).
        if let Some(old) = &old {
            p.channels = old.channels.clone();
        }
        match old {
            Some(old) if *old == p => {}
            Some(old) => {
                *old = p;
                changed = true;
            }
            None => {
                w.nodes.push(p);
                changed = true;
            }
        }
    }
    changed
}

fn workspace_mut(scene: &mut Scene, id: u64) -> &mut Workspace {
    scene
        .animation
        .rig
        .workspaces
        .iter_mut()
        .find(|w| w.id == id)
        .expect("checked")
}

/// The handler for the workspace and placement commands.
pub fn run(e: &mut Editor, name: &str, a: &Args) -> Result<Option<Command>, String> {
    let m = Metrics::default();
    match name {
        "operandi.rig.workspaceAdd" => {
            let label = text(a, "name")?
                .filter(|s| !s.is_empty())
                .unwrap_or("Workspace");
            if label.len() > 1024 {
                return Err("Name must be at most 1024 bytes".into());
            }
            let label = label.to_owned();
            let places = places(e, a)?;
            e.anim_step("New Workspace", |e| {
                if e.scene.next_id >= u64::MAX - 1 {
                    return Err("Scene item/identity budget exhausted".into());
                }
                let id = e.scene.next_id;
                e.scene.next_id += 1;
                let mut w = Workspace {
                    id,
                    name: label,
                    nodes: Vec::new(),
                };
                put(&mut w, &places, false);
                e.scene.animation.rig.workspaces.push(w);
                Ok(true)
            })?;
        }
        "operandi.rig.workspaceRename" => {
            let ws = workspace_arg(e, a)?;
            let label = text(a, "name")?
                .filter(|s| !s.is_empty())
                .ok_or("Give name")?
                .to_owned();
            if label.len() > 1024 {
                return Err("Name must be at most 1024 bytes".into());
            }
            e.anim_step("Rename Workspace", |e| {
                let w = workspace_mut(&mut e.scene, ws);
                let changed = w.name != label;
                w.name = label;
                Ok(changed)
            })?;
        }
        "operandi.rig.workspaceRemove" => {
            let ws = workspace_arg(e, a)?;
            e.anim_step("Delete Workspace", |e| {
                e.scene.animation.rig.workspaces.retain(|w| w.id != ws);
                Ok(true)
            })?;
        }
        "operandi.rig.workspaceClear" => {
            let ws = workspace_arg(e, a)?;
            e.anim_step("Clear Workspace", |e| {
                let w = workspace_mut(&mut e.scene, ws);
                let changed = !w.nodes.is_empty();
                w.nodes.clear();
                Ok(changed)
            })?;
        }
        "operandi.rig.schematicPlace" => {
            let ws = workspace_arg(e, a)?;
            let mut placed = places(e, a)?;
            let pinned = match a.get("pinned") {
                None | Some(Value::Null) => true,
                Some(Value::Bool(b)) => *b,
                Some(Value::String(s)) => crate::command_args::boolean("pinned", s)?,
                Some(_) => return Err("pinned is true or false".into()),
            };
            // Nodes without a place arrive arranged, below the page (SC-60).
            if let Some(keys) = node_keys(e, a)? {
                let w = e
                    .scene
                    .animation
                    .rig
                    .workspaces
                    .iter()
                    .find(|w| w.id == ws)
                    .expect("checked");
                let new: BTreeSet<String> = keys
                    .into_iter()
                    .filter(|k| !w.nodes.iter().any(|n| &n.key == k))
                    .filter(|k| !placed.iter().any(|p| &p.0 == k))
                    .collect();
                let keep: BTreeMap<String, [f32; 2]> = w
                    .nodes
                    .iter()
                    .map(|n| (n.key.clone(), [n.x, n.y]))
                    .chain(placed.iter().cloned())
                    .collect();
                let arranged = block(&e.scene, &new, &keep, &m);
                e.anim_step("Add to Workspace", |e| {
                    let w = workspace_mut(&mut e.scene, ws);
                    let a = put(w, &placed, pinned);
                    let b = put(w, &arranged, false);
                    Ok(a || b)
                })?;
                return Ok(None);
            }
            if placed.is_empty() {
                return Err("Give place (key@x/y,...) or nodes (key,...)".into());
            }
            placed.dedup_by(|a, b| a.0 == b.0);
            e.anim_step("Move Nodes", |e| {
                Ok(put(workspace_mut(&mut e.scene, ws), &placed, pinned))
            })?;
        }
        "operandi.rig.schematicChannels" => {
            let ws = workspace_arg(e, a)?;
            let list = text(a, "channels")?.ok_or("Give channels:item:name,...")?;
            let mut by_item: Vec<(u64, Vec<String>)> = Vec::new();
            for k in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let key = crate::animation::ChannelKey::parse(k)?;
                if crate::animation::read(&e.scene, &key).is_none() {
                    return Err(format!("{k} isn't a channel of an item in the scene"));
                }
                match by_item.iter_mut().find(|(i, _)| *i == key.item) {
                    Some((_, names)) if names.contains(&key.channel) => {}
                    Some((_, names)) => names.push(key.channel),
                    None => by_item.push((key.item, vec![key.channel])),
                }
            }
            if by_item.is_empty() {
                return Err("Give channels:item:name,...".into());
            }
            let at = match text(a, "at")? {
                None => None,
                Some(t) => {
                    let bad = || format!("at is x/y, such as 40/60, not {t}");
                    let (x, y) = t.split_once('/').ok_or_else(bad)?;
                    let (x, y): (f32, f32) = (
                        x.trim().parse().map_err(|_| bad())?,
                        y.trim().parse().map_err(|_| bad())?,
                    );
                    if !x.is_finite() || !y.is_finite() {
                        return Err(bad());
                    }
                    Some([x, y])
                }
            };
            // Items not on the page yet: at the drop (one below another),
            // or arranged below what is there.
            let w = e
                .scene
                .animation
                .rig
                .workspaces
                .iter()
                .find(|w| w.id == ws)
                .expect("checked");
            let new: Vec<String> = by_item
                .iter()
                .map(|(i, _)| format!("item:{i}"))
                .filter(|k| !w.nodes.iter().any(|n| &n.key == k))
                .collect();
            let placed: Vec<(String, [f32; 2])> = match at {
                Some([x, y]) => new
                    .iter()
                    .enumerate()
                    .map(|(k, key)| (key.clone(), [x, y + k as f32 * m.row_h * 4.]))
                    .collect(),
                None => {
                    let keep: BTreeMap<String, [f32; 2]> = w
                        .nodes
                        .iter()
                        .map(|n| (n.key.clone(), [n.x, n.y]))
                        .collect();
                    let set: BTreeSet<String> = new.iter().cloned().collect();
                    block(&e.scene, &set, &keep, &m)
                }
            };
            e.anim_step("Add Channels to Workspace", |e| {
                let w = workspace_mut(&mut e.scene, ws);
                let mut changed = put(w, &placed, at.is_some());
                for (item, names) in &by_item {
                    let key = format!("item:{item}");
                    let Some(node) = w.nodes.iter_mut().find(|n| n.key == key) else {
                        continue;
                    };
                    for n in names {
                        if !node.channels.contains(n) {
                            node.channels.push(n.clone());
                            changed = true;
                        }
                    }
                }
                Ok(changed)
            })?;
        }
        "operandi.rig.schematicRemove" => {
            let ws = workspace_arg(e, a)?;
            let keys: Vec<String> = text(a, "nodes")?
                .ok_or("Give nodes")?
                .split(',')
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect();
            e.anim_step("Remove from Workspace", |e| {
                let w = workspace_mut(&mut e.scene, ws);
                let before = w.nodes.len();
                w.nodes.retain(|n| !keys.contains(&n.key));
                Ok(w.nodes.len() != before)
            })?;
        }
        "operandi.rig.schematicArrange" => {
            let ws = workspace_arg(e, a)?;
            let view = page(&e.scene, PageId::Saved(ws), &m);
            let w = e
                .scene
                .animation
                .rig
                .workspaces
                .iter()
                .find(|w| w.id == ws)
                .expect("checked");
            // Named nodes, or every node not moved by hand (SC-60).
            let chosen: BTreeSet<String> = match node_keys(e, a)? {
                Some(keys) => keys
                    .into_iter()
                    .filter(|k| view.positions.contains_key(k))
                    .collect(),
                None => view
                    .positions
                    .keys()
                    .filter(|k| !w.nodes.iter().any(|n| &n.key == *k && n.pinned))
                    .cloned()
                    .collect(),
            };
            let keep: BTreeMap<String, [f32; 2]> = view
                .positions
                .iter()
                .filter(|(k, _)| !chosen.contains(*k) && view.saved.contains(*k))
                .map(|(k, p)| (k.clone(), *p))
                .collect();
            let arranged = block(&e.scene, &chosen, &keep, &m);
            e.anim_step("Arrange", |e| {
                Ok(put(workspace_mut(&mut e.scene, ws), &arranged, false))
            })?;
        }
        _ => return Err(format!("Unknown command {name}")),
    }
    Ok(None)
}

/// `operandi.rig.workspaces`: the saved workspaces and their nodes as
/// JSON (UIC-163).
pub fn answer(e: &Editor, name: &str, _: &Args) -> Option<Result<String, String>> {
    (name == "operandi.rig.workspaces").then(|| {
        serde_json::to_string(&e.scene.animation.rig.workspaces).map_err(|e| e.to_string())
    })
}
