//! LXO import of the Schematic's pages (BEHAVIOR-SCHEMATIC SC-72, SC-90;
//! LXO-SCHEMATIC-FORMAT): each workspace becomes an Operandi workspace of
//! the same name, in the same order, and each node record a place on it.
//!
//! - A workspace is an `assembly` item tagged `GTYP=assembly`,
//!   `SGID=workspace`; an `assembly` without `SGID` is a sub-assembly and
//!   flattens into the workspace above it.
//! - A node record is a `schmNode` item. On graph `schmItem` the shown item
//!   links to the record and the record links to its workspace; its `NPOS`
//!   tag (`x=.. y=..`) is the node's top-left corner, y down.
//! - A child record (graph `parent` to another record) shows part of its
//!   parent's node (a locator's transform item): it merges into the parent.
//! - Records whose item didn't import are dropped. Rig pieces with no
//!   record show on the Schematic's "Unplaced" page (SC-73), arranged.
//! - Backdrops, notes and elbows aren't shown yet: counted in the report.
//!
//! The reference's units are larger than Operandi's points and its nodes
//! smaller than Operandi's (title and linked rows, 170 points wide), so
//! places are scaled by [`SCALE`] and then nodes that would overlap are
//! moved down, keeping each column's order (provisional, SC-90 [probe]).
use super::Report;
use super::phase1::RawItem;
use super::rig_import::Pieces;
use crate::Scene;
use crate::schematic_layout::Metrics;
use crate::schematic_pages::{Placement, Workspace};
use std::collections::BTreeMap;

/// Points per reference unit. The format note suggests 4; Operandi's nodes
/// are wider (170 points) than a reference column (under 30 units), so 6
/// keeps columns from overlapping, and an 8-unit step (one title bar)
/// becomes 48 points, one Operandi title and row.
pub const SCALE: f32 = 6.;

fn oid(id: u32) -> u64 {
    u64::from(id) + 1
}

fn is_workspace(i: &RawItem) -> bool {
    i.kind == "assembly"
        && i.tags.get("GTYP").map(String::as_str) == Some("assembly")
        && i.tags.get("SGID").map(String::as_str) == Some("workspace")
}

/// `x=<f> y=<f>` (or `w=.. h=..`): the two numbers, by name.
pub(super) fn pair(text: &str, a: &str, b: &str) -> Option<(f32, f32)> {
    let mut x = None;
    let mut y = None;
    for part in text.split_whitespace() {
        let (k, v) = part.split_once('=')?;
        let v: f32 = v.trim().parse().ok().filter(|v: &f32| v.is_finite())?;
        if k == a {
            x = Some(v);
        } else if k == b {
            y = Some(v);
        }
    }
    Some((x?, y?))
}

/// The workspace an assembly belongs to: itself, or the nearest one above
/// it through `parent` links.
fn workspace_of(items: &BTreeMap<u32, RawItem>, mut id: u32) -> Option<u32> {
    for _ in 0..64 {
        let i = items.get(&id)?;
        if is_workspace(i) {
            return Some(id);
        }
        if i.kind != "assembly" {
            return None;
        }
        id = i.link("parent")?;
    }
    None
}

/// Reads the pages into `scene.animation.rig.workspaces`.
pub fn apply(
    scene: &mut Scene,
    items: &BTreeMap<u32, RawItem>,
    pieces: &Pieces,
    report: &mut Report,
) {
    let order: Vec<u32> = items
        .values()
        .filter(|i| is_workspace(i))
        .map(|i| i.id)
        .collect();
    if order.is_empty() {
        return;
    }
    // Who shows each record: the item whose schmItem link points at it.
    let mut shown_by: BTreeMap<u32, u32> = BTreeMap::new();
    for i in items.values() {
        for (g, target, _) in &i.links {
            if g == "schmItem" && items.get(target).is_some_and(|t| t.kind == "schmNode") {
                shown_by.entry(*target).or_insert(i.id);
            }
        }
    }
    let key_of = |modo: u32| -> Option<String> {
        if let Some((piece, _)) = pieces.get(&modo) {
            return Some(format!("piece:{piece}"));
        }
        scene.item(oid(modo)).map(|_| format!("item:{}", oid(modo)))
    };
    let mut places: BTreeMap<u32, Vec<(String, f32, f32)>> = BTreeMap::new();
    let mut dropped = 0usize;
    let mut extras: BTreeMap<&str, usize> = BTreeMap::new();
    let mut flattened = 0usize;
    for r in items.values().filter(|i| i.kind == "schmNode") {
        // Child records merge into their parent's node.
        if r.link("parent")
            .is_some_and(|p| items.get(&p).is_some_and(|p| p.kind == "schmNode"))
        {
            continue;
        }
        let Some(ws) = r
            .links
            .iter()
            .filter(|(g, _, _)| g == "schmItem")
            .find_map(|(_, t, _)| workspace_of(items, *t))
        else {
            continue;
        };
        if r.links
            .iter()
            .any(|(g, t, _)| g == "schmItem" && items.get(t).is_some_and(|a| !is_workspace(a)))
        {
            flattened += 1;
        }
        let Some(shown) = shown_by.get(&r.id).and_then(|s| items.get(s)) else {
            continue;
        };
        match shown.kind.as_str() {
            "backdropSchm" => {
                *extras.entry("backdrops").or_default() += 1;
                continue;
            }
            "noteSchm" => {
                *extras.entry("notes").or_default() += 1;
                continue;
            }
            "elbow" => {
                *extras.entry("elbows").or_default() += 1;
                continue;
            }
            _ => {}
        }
        let Some(key) = key_of(shown.id) else {
            dropped += 1;
            continue;
        };
        let Some((x, y)) = r.tags.get("NPOS").and_then(|t| pair(t, "x", "y")) else {
            dropped += 1;
            continue;
        };
        places
            .entry(ws)
            .or_default()
            .push((key, x * SCALE, y * SCALE));
    }
    let m = Metrics::default();
    let full = crate::rig_schematic::build(scene);
    let height = |k: &str| {
        full.nodes
            .iter()
            .find(|n| n.key == k)
            .map_or(m.head_h + m.row_h + 6., |n| m.height(n))
    };
    let mut total = 0usize;
    for ws in order {
        let mut nodes: Vec<Placement> = Vec::new();
        for (key, x, y) in places.remove(&ws).unwrap_or_default() {
            if nodes.iter().any(|n| n.key == key) {
                continue;
            }
            nodes.push(Placement {
                key,
                x,
                y,
                pinned: true,
                channels: Vec::new(),
            });
        }
        clear_overlaps(&mut nodes, &height, &m);
        total += nodes.len();
        if scene.next_id >= u64::MAX - 1 {
            break;
        }
        let id = scene.next_id;
        scene.next_id += 1;
        scene.animation.rig.workspaces.push(Workspace {
            id,
            name: items[&ws].label(),
            nodes,
        });
    }
    report.notes.push(format!(
        "Schematic: {} workspaces with {total} node places",
        scene.animation.rig.workspaces.len()
    ));
    if flattened > 0 {
        report.notes.push(format!(
            "Schematic: {flattened} nodes in sub-assemblies moved onto their workspace"
        ));
    }
    if dropped > 0 {
        report.notes.push(format!(
            "Schematic: {dropped} node places for things Operandi didn't import were left out"
        ));
    }
    if !extras.is_empty() {
        let list: Vec<String> = extras.iter().map(|(k, n)| format!("{n} {k}")).collect();
        report
            .notes
            .push(format!("Schematic: {} not shown yet", list.join(", ")));
    }
}

/// Moves nodes down until none overlaps another, top to bottom, so each
/// column keeps its order.
fn clear_overlaps(nodes: &mut [Placement], height: &dyn Fn(&str) -> f32, m: &Metrics) {
    let mut order: Vec<usize> = (0..nodes.len()).collect();
    order.sort_by(|&a, &b| nodes[a].y.total_cmp(&nodes[b].y).then(a.cmp(&b)));
    let mut done: Vec<usize> = Vec::new();
    for i in order {
        let h = height(&nodes[i].key);
        loop {
            let hit = done.iter().find(|&&j| {
                let (a, b) = (&nodes[i], &nodes[j]);
                (a.x - b.x).abs() < m.node_w
                    && a.y < b.y + height(&b.key) + 2.
                    && b.y < a.y + h + 2.
            });
            match hit {
                Some(&j) => nodes[i].y = nodes[j].y + height(&nodes[j].key) + 4.,
                None => break,
            }
        }
        done.push(i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npos_reads_both_numbers_in_either_order() {
        assert_eq!(pair("x=-86 y=-242", "x", "y"), Some((-86., -242.)));
        assert_eq!(pair("y=2.5 x=1", "x", "y"), Some((1., 2.5)));
        assert_eq!(pair("x=1", "x", "y"), None);
        assert_eq!(pair("x=nan y=1", "x", "y"), None);
    }
}
