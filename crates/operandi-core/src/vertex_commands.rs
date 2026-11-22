//! Milestone 3 Vertex panel geometry (MV-T01..T07, MV-C01..C16).
//!
//! Each operation acts on the ordered point selection of the active mesh, in a
//! basic Operandi profile
//! (see `docs/m3-vertex-tools/ACCEPTANCE_CRITERIA.md`):
//!
//! * **Split** — a point used by n polygons becomes n coincident points, one
//!   per polygon (the first polygon keeps the original).
//! * **Split Edge** — every edge whose two endpoints are selected gets a point
//!   at its middle (provisional reading of the panel row).
//! * **Join** / **Join Averaged** — the selected points weld into one, at the
//!   last selected point or at their average; polygons heal.
//! * **Merge** — selected points within a distance (0 = exactly coincident)
//!   weld in single-linkage groups, each at its most recently selected point.
//!   With nothing selected every point is considered and each group lands at
//!   its average (CMD-V-01).
//! * **Collapse** — the point and its edge neighbours merge at their mean;
//!   the polygons around it collapse away (CMD-B3-02).
//! * **Remove** — the points are dropped from their polygons, which keep
//!   their other corners; polygons left with < 3 corners go (CMD-V-04).
//! * **Remove Edge** — edges between two selected points are dissolved.
//! * **Align** — interior points of the ordered row move onto the line from
//!   the first to the last point (keeping their place along the line's
//!   dominant axis, CMD-V-06, or even spacing), per axis.
//! * **Reorder Index** — Swap / Top / Bottom point renumbering.
//!
//! The live tools (Bevel, Extrude, Tear, Slide, Create) and Weld, New Vertex,
//! Make Polygon and Sync Index Order have their own functions below.
//!
//! Corner data follows the source corner a new corner came from; corners with
//! no source (bevel/extrude cuts, new polygons) are refused while a map holds
//! values there, exactly as for the M2 Edge commands. New points made from a
//! selected point copy its point records (weights, morphs, point UVs).
use crate::edge_commands::{CornerBlend, OutFace, finish, rebuild, weld, weld_keeping, weld_with};
use crate::{EdgeCommand, Face, Mesh, PolygonRepresentation, Result};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

/// Reorder Index modes: Swap, Top, Bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReorderMode {
    Swap,
    Top,
    Bottom,
}

/// Options of Merge (tool and command) and Join (VX-M05, VX-C01). The
/// starting values are the dialogs': both checks off, discontinuous UVs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MergeOptions {
    /// Keep 1-Vertex Polygons: polygons squashed to two corners stay as
    /// two-point lines (one-corner remnants still go, VXM-Q2).
    pub keep_small: bool,
    /// Test Morph Map Positions (Merge): points must also lie within the
    /// Distance of each other in every morph map.
    pub test_morphs: bool,
    /// Merge (Join) Discontinuous UV Values: at a merged point, corners
    /// whose UVs now match share one UV (provisional, VXM-Q1).
    pub merge_uvs: bool,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            keep_small: false,
            test_morphs: false,
            merge_uvs: true,
        }
    }
}

/// The one-shot Vertex commands with their options.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VertexCommand {
    Split,
    SplitEdge,
    Join {
        average: bool,
        options: MergeOptions,
    },
    /// `distance` 0 is the Automatic range (exactly coincident).
    Merge {
        distance: f64,
        options: MergeOptions,
    },
    Collapse,
    Remove,
    RemoveEdge,
    Align {
        axes: [bool; 3],
        uniform: bool,
    },
    Reorder(ReorderMode),
}
impl VertexCommand {
    /// Panel and history label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Split => "Split",
            Self::SplitEdge => "Split Edge",
            Self::Join { average: false, .. } => "Join",
            Self::Join { average: true, .. } => "Join Averaged",
            Self::Merge { .. } => "Merge",
            Self::Collapse => "Collapse",
            Self::Remove => "Remove",
            Self::RemoveEdge => "Remove Edge",
            Self::Align { .. } => "Align",
            Self::Reorder(_) => "Reorder Index",
        }
    }
    /// Command-line verb (`vert.<verb>`).
    pub fn verb(self) -> &'static str {
        match self {
            Self::Split => "vert.split",
            Self::SplitEdge => "vert.splitEdge",
            Self::Join { .. } => "vert.join",
            Self::Merge { .. } => "vert.merge",
            Self::Collapse => "vert.collapse",
            Self::Remove => "vert.remove",
            Self::RemoveEdge => "vert.removeEdge",
            Self::Align { .. } => "vert.align",
            Self::Reorder(_) => "vert.reorder",
        }
    }
}

/// Validated, de-duplicated selection in selection order.
fn ordered(mesh: &Mesh, selected: &[u32], what: &str) -> Result<Vec<u32>> {
    mesh.validate()?;
    if !mesh.attributes.extrude_edge_uses.is_empty() {
        return Err(format!("{what} needs consistent winding; use Align first"));
    }
    let mut out = Vec::new();
    for &v in selected {
        if v as usize >= mesh.vertices.len() {
            return Err("A selected point no longer exists".into());
        }
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if out.is_empty() {
        return Err(format!("Select points for {what}"));
    }
    Ok(out)
}

/// Run `command` on `selected` points (ordered as selected). Returns the new
/// mesh and the points to select afterwards.
pub fn vertex_command_geometry(
    mesh: &Mesh,
    selected: &[u32],
    command: VertexCommand,
) -> Result<(Mesh, Vec<u32>)> {
    let what = command.label();
    // CMD-V-01: Merge with nothing selected works on every point and puts
    // each group at its average.
    if let VertexCommand::Merge { distance, options } = command
        && selected.is_empty()
    {
        let all: Vec<u32> = (0..mesh.vertices.len() as u32).collect();
        let sel = ordered(mesh, &all, what)?;
        return merge(mesh, &sel, distance, options);
    }
    let sel = ordered(mesh, selected, what)?;
    match command {
        VertexCommand::Split => split(mesh, &sel),
        VertexCommand::SplitEdge => split_edge(mesh, &sel),
        VertexCommand::Join { average, options } => join(mesh, &sel, average, options, what),
        VertexCommand::Merge { distance, options } => merge(mesh, &sel, distance, options),
        VertexCommand::Collapse => collapse(mesh, &sel),
        VertexCommand::Remove => remove(mesh, &sel),
        VertexCommand::RemoveEdge => {
            let set: BTreeSet<u32> = sel.iter().copied().collect();
            let edges: Vec<(u32, u32)> = mesh
                .edges()
                .into_iter()
                .filter(|(a, b)| set.contains(a) && set.contains(b))
                .collect();
            if edges.is_empty() {
                return Err("Remove Edge needs two selected points joined by an edge".into());
            }
            let (out, _) = crate::edge_command_geometry(mesh, &edges, EdgeCommand::Remove)?;
            Ok((out, sel))
        }
        VertexCommand::Align { axes, uniform } => align(mesh, &sel, axes, uniform),
        VertexCommand::Reorder(mode) => reorder(mesh, &sel, mode),
    }
}

fn split(mesh: &Mesh, sel: &[u32]) -> Result<(Mesh, Vec<u32>)> {
    let set: BTreeSet<u32> = sel.iter().copied().collect();
    let mut vertices = mesh.vertices.clone();
    let mut claimed = BTreeSet::new();
    let mut copies = Vec::new();
    let mut faces = Vec::with_capacity(mesh.faces.len());
    for (fi, f) in mesh.faces.iter().enumerate() {
        let mut vs = Vec::with_capacity(f.vertices.len());
        for &v in &f.vertices {
            if set.contains(&v) && !claimed.insert(v) {
                let id = u32::try_from(vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
                vertices.push(mesh.vertices[v as usize]);
                copies.push((id, v));
                vs.push(id);
            } else {
                vs.push(v);
            }
        }
        faces.push(OutFace::copy(mesh, fi, vs));
    }
    if copies.is_empty() {
        return Err("Split needs a selected point shared by two or more polygons".into());
    }
    let out = finish(rebuild(mesh, vertices, faces, &copies, "Split")?, "Split")?;
    let mut selection = sel.to_vec();
    selection.extend(copies.iter().map(|&(new, _)| new));
    Ok((out, selection))
}

fn split_edge(mesh: &Mesh, sel: &[u32]) -> Result<(Mesh, Vec<u32>)> {
    let set: BTreeSet<u32> = sel.iter().copied().collect();
    let edges: Vec<(u32, u32)> = mesh
        .edges()
        .into_iter()
        .filter(|(a, b)| set.contains(a) && set.contains(b))
        .collect();
    if edges.is_empty() {
        return Err("Split Edge needs two selected points joined by an edge".into());
    }
    let mut out = mesh.clone();
    let mut selection = sel.to_vec();
    // Appending keeps earlier point ids, so every listed edge stays valid.
    for (a, b) in edges {
        let (next, mid) = crate::add_point(&out, a, b, 0.5)?;
        out = next;
        selection.push(mid);
    }
    Ok((out, selection))
}

fn join(
    mesh: &Mesh,
    sel: &[u32],
    average: bool,
    options: MergeOptions,
    what: &str,
) -> Result<(Mesh, Vec<u32>)> {
    if sel.len() < 2 {
        return Err(format!("{what} needs two or more selected points"));
    }
    let last = *sel.last().unwrap();
    let position = if average {
        sel.iter()
            .map(|&v| mesh.vertices[v as usize])
            .sum::<DVec3>()
            / sel.len() as f64
    } else {
        mesh.vertices[last as usize]
    };
    let (mut out, remap) = weld_with(mesh, &[(sel.to_vec(), position)], options.keep_small, what)?;
    let kept: Vec<u32> = remap[last as usize].into_iter().collect();
    if options.merge_uvs {
        share_matching_uvs(&mut out, &kept);
    }
    Ok((finish(out, what)?, kept))
}

/// Join / Join Averaged with Symmetry on (Jason, 2026-11-15; provisional,
/// SYM-JOIN-Q1): when two or more selected points sit on each side and
/// none on the plane, each side's points weld by themselves (onto that
/// side's last pick, or that side's average), in one weld. Otherwise the
/// selection joins as without Symmetry: a point and its mirror, or points
/// that include one on the plane, weld into one.
pub fn vertex_join_symmetric_geometry(
    mesh: &Mesh,
    selected: &[u32],
    symmetry: crate::Symmetry,
    average: bool,
    options: MergeOptions,
) -> Result<(Mesh, Vec<u32>)> {
    let command = VertexCommand::Join { average, options };
    let what = command.label();
    if symmetry.axis.is_none() {
        return vertex_command_geometry(mesh, selected, command);
    }
    let sel = ordered(mesh, selected, what)?;
    let sides = crate::symmetry_sides::by_side(mesh, symmetry, &sel, |&v| vec![v]);
    if sides.positive.len() < 2 || sides.negative.len() < 2 || !sides.centre.is_empty() {
        return vertex_command_geometry(mesh, &sel, command);
    }
    let group = |side: &[u32]| {
        let last = *side.last().unwrap();
        let position = if average {
            side.iter()
                .map(|&v| mesh.vertices[v as usize])
                .sum::<DVec3>()
                / side.len() as f64
        } else {
            mesh.vertices[last as usize]
        };
        ((side.to_vec(), position), last)
    };
    let (positive, p_last) = group(&sides.positive);
    let (negative, n_last) = group(&sides.negative);
    let (mut out, remap) = weld_with(mesh, &[positive, negative], options.keep_small, what)?;
    let kept: Vec<u32> = [p_last, n_last]
        .iter()
        .filter_map(|&v| remap[v as usize])
        .collect();
    if options.merge_uvs {
        share_matching_uvs(&mut out, &kept);
    }
    Ok((finish(out, what)?, kept))
}

/// Merge/Join Discontinuous UV Values (provisional reading, VXM-Q1): at each
/// of `points`, corners whose UVs in a map are equal share one UV record, so
/// the UV is continuous there; corners with different UVs keep theirs.
fn share_matching_uvs(mesh: &mut Mesh, points: &[u32]) {
    if points.is_empty() || mesh.attributes.uv_records.is_empty() {
        return;
    }
    let wanted: BTreeSet<u32> = points.iter().copied().collect();
    let mut corners: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
    for (f, face) in mesh.faces.iter().enumerate() {
        for (c, v) in face.vertices.iter().enumerate() {
            if wanted.contains(v) {
                corners.entry(*v).or_default().push((f, c));
            }
        }
    }
    let uv_maps = mesh.uv_maps.clone();
    let attributes = &mut *mesh.attributes;
    for (name, records) in attributes.uv_records.iter_mut() {
        let Some(values) = uv_maps.get(name) else {
            continue;
        };
        for at in corners.values() {
            // Each corner joins the first earlier corner with the same UV.
            for (i, &(f, c)) in at.iter().enumerate() {
                let Some(uv) = values.get(f).and_then(|r| r.get(c)).copied().flatten() else {
                    continue;
                };
                let same = at[..i].iter().find(|&&(g, d)| {
                    values
                        .get(g)
                        .and_then(|r| r.get(d))
                        .copied()
                        .flatten()
                        .is_some_and(|other| other.distance(uv) <= 1e-12)
                });
                if let Some(&(g, d)) = same
                    && let Some(record) = records.get(g).and_then(|r| r.get(d)).copied().flatten()
                    && let Some(slot) = records.get_mut(f).and_then(|r| r.get_mut(c))
                {
                    *slot = Some(record);
                }
            }
        }
    }
}

/// Single-linkage groups of `sel` within `distance` (VX-M02); each group
/// lands at its members' average (VX-M01). With `test_morphs` two points
/// link only if they're also within `distance` in every morph map (VX-M05).
/// Groups keep selection order.
fn merge_groups(
    mesh: &Mesh,
    sel: &[u32],
    distance: f64,
    test_morphs: bool,
) -> Vec<(Vec<u32>, DVec3)> {
    let n = sel.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let p = |i: usize| mesh.vertices[sel[i] as usize];
    let morphs = &mesh.attributes.relative_morphs;
    let delta =
        |map: &BTreeMap<u32, DVec3>, i: usize| map.get(&sel[i]).copied().unwrap_or_default();
    let close = |i: usize, j: usize| {
        p(i).distance(p(j)) <= distance
            && (!test_morphs
                || morphs
                    .values()
                    .all(|m| (p(i) + delta(m, i)).distance(p(j) + delta(m, j)) <= distance))
    };
    for i in 0..n {
        for j in i + 1..n {
            if close(i, j) {
                let (a, b) = (root(&mut parent, i), root(&mut parent, j));
                parent[a] = b;
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let mut out: Vec<(Vec<u32>, DVec3)> = groups
        .into_values()
        .filter(|g| g.len() > 1)
        .map(|g| {
            let centre = g.iter().map(|&i| p(i)).sum::<DVec3>() / g.len() as f64;
            (g.iter().map(|&i| sel[i]).collect(), centre)
        })
        .collect();
    out.sort_by_key(|(m, _)| sel.iter().position(|v| *v == m[0]));
    out
}

/// Merge (tool and command): groups of points within `distance` weld at
/// their average. Nothing is selected afterwards (VX-M06).
fn merge(
    mesh: &Mesh,
    sel: &[u32],
    distance: f64,
    options: MergeOptions,
) -> Result<(Mesh, Vec<u32>)> {
    if !distance.is_finite() || distance < 0. {
        return Err("Merge Distance must be a finite distance of 0 or more".into());
    }
    let groups = merge_groups(mesh, sel, distance, options.test_morphs);
    if groups.is_empty() {
        return Ok((mesh.clone(), Vec::new()));
    }
    let (mut out, remap) = weld_with(mesh, &groups, options.keep_small, "Merge")?;
    if options.merge_uvs {
        let merged: Vec<u32> = groups
            .iter()
            .filter_map(|(members, _)| remap[members[0] as usize])
            .collect();
        share_matching_uvs(&mut out, &merged);
    }
    Ok((finish(out, "Merge")?, Vec::new()))
}

/// The polygons around point `v` in fan order, with each one's corner of `v`.
/// Returns `(face slot, corner)` pairs and whether the fan is closed.
/// Walking face `f` then `g` means `g` is across the edge from `f`'s
/// previous point to `v`, so the fan runs against each polygon's winding.
fn fan(faces: &[OutFace], v: u32, what: &str) -> Result<(Vec<(usize, usize)>, bool)> {
    let incident: Vec<(usize, usize)> = faces
        .iter()
        .enumerate()
        .filter_map(|(fi, f)| f.vertices.iter().position(|&x| x == v).map(|c| (fi, c)))
        .collect();
    let around = |&(fi, c): &(usize, usize)| {
        let vs = &faces[fi].vertices;
        let n = vs.len();
        (vs[(c + n - 1) % n], vs[(c + 1) % n])
    };
    let mut by_next: BTreeMap<u32, (usize, usize)> = BTreeMap::new();
    let mut prevs = BTreeSet::new();
    for slot in &incident {
        let (prev, next) = around(slot);
        prevs.insert(prev);
        if by_next.insert(next, *slot).is_some() {
            return Err(format!(
                "{what} needs a simple fan of polygons at each point"
            ));
        }
    }
    // An open fan starts at the polygon whose next edge is on the boundary.
    let starts: Vec<&(usize, usize)> = incident
        .iter()
        .filter(|s| !prevs.contains(&around(s).1))
        .collect();
    let start = match starts.len() {
        0 => *incident
            .first()
            .ok_or_else(|| format!("{what}: the point has no polygons"))?,
        1 => *starts[0],
        _ => {
            return Err(format!(
                "{what} needs a simple fan of polygons at each point"
            ));
        }
    };
    let mut order = vec![start];
    let mut closed = false;
    loop {
        let (prev, _) = around(order.last().unwrap());
        match by_next.get(&prev) {
            Some(next) if *next == start => {
                closed = true;
                break;
            }
            Some(next) if order.contains(next) => {
                return Err(format!(
                    "{what} needs a simple fan of polygons at each point"
                ));
            }
            Some(next) => order.push(*next),
            None => break,
        }
    }
    if order.len() != incident.len() {
        return Err(format!(
            "{what} needs a simple fan of polygons at each point"
        ));
    }
    Ok((order, closed))
}

fn source_faces(mesh: &Mesh, what: &str) -> Result<Vec<OutFace>> {
    if mesh.has_polylines() {
        return Err(format!("{what} supports surface polygons only"));
    }
    Ok((0..mesh.faces.len())
        .map(|f| OutFace::copy(mesh, f, mesh.faces[f].vertices.clone()))
        .collect())
}

/// CMD-B3-02: Collapse merges each selected point and the points joined to
/// it by an edge into one point at their mean. Polygons around it collapse
/// away; the others keep the merged point. Selections whose neighbourhoods
/// touch merge into one group (provisional: overlaps weren't measured).
fn collapse(mesh: &Mesh, sel: &[u32]) -> Result<(Mesh, Vec<u32>)> {
    let edges = mesh.edges();
    let mut groups: Vec<BTreeSet<u32>> = Vec::new();
    for &v in sel {
        let mut group: BTreeSet<u32> = edges
            .iter()
            .filter_map(|&(a, b)| (a == v).then_some(b).or((b == v).then_some(a)))
            .collect();
        group.insert(v);
        if group.len() < 2 {
            continue;
        }
        // Merge with any earlier group it touches.
        let (touching, rest): (Vec<_>, Vec<_>) =
            groups.into_iter().partition(|g| !g.is_disjoint(&group));
        for g in touching {
            group.extend(g);
        }
        groups = rest;
        groups.push(group);
    }
    if groups.is_empty() {
        return Ok((mesh.clone(), Vec::new()));
    }
    let welds: Vec<(Vec<u32>, DVec3)> = groups
        .iter()
        .map(|g| {
            let center =
                g.iter().map(|&v| mesh.vertices[v as usize]).sum::<DVec3>() / g.len() as f64;
            (g.iter().copied().collect(), center)
        })
        .collect();
    let (out, remap) = weld(mesh, &welds, "Collapse")?;
    let selection = groups
        .iter()
        .filter_map(|g| remap[*g.iter().next()? as usize])
        .collect();
    Ok((finish(out, "Collapse")?, selection))
}

/// CMD-V-04: Remove drops the points from their polygons, which keep the
/// other corners (a quad becomes a triangle); a polygon left with fewer
/// than three corners is removed with them.
pub(crate) fn remove(mesh: &Mesh, sel: &[u32]) -> Result<(Mesh, Vec<u32>)> {
    let set: BTreeSet<u32> = sel.iter().copied().collect();
    let kept = remove_from_polygons(mesh, &set)?;
    let cleaned = crate::mesh_cleanup::remove_unreferenced(&kept, Some(&set))?;
    Ok((finish(cleaned.mesh, "Remove")?, Vec::new()))
}

/// Remove's first half: `set` leaves every polygon, a polygon left under
/// three corners goes, and every point keeps its index (the caller drops
/// what it wants unused). A polyline goes only when it is left under two
/// points, so lines that don't touch `set` are never lost.
pub(crate) fn remove_from_polygons(mesh: &Mesh, set: &BTreeSet<u32>) -> Result<Mesh> {
    let faces: Vec<OutFace> = (0..mesh.faces.len())
        .filter_map(|f| {
            let keep: Vec<usize> = (0..mesh.faces[f].vertices.len())
                .filter(|&c| !set.contains(&mesh.faces[f].vertices[c]))
                .collect();
            let least = if mesh.is_polyline(f) { 2 } else { 3 };
            (keep.len() >= least).then(|| OutFace {
                template: f,
                vertices: keep.iter().map(|&c| mesh.faces[f].vertices[c]).collect(),
                donors: keep.iter().map(|&c| Some((f, c))).collect(),
                blends: Vec::new(),
            })
        })
        .collect();
    rebuild(mesh, mesh.vertices.clone(), faces, &[], "Remove")
}

fn align(mesh: &Mesh, sel: &[u32], axes: [bool; 3], uniform: bool) -> Result<(Mesh, Vec<u32>)> {
    if sel.len() < 3 {
        return Err(
            "Align needs three or more selected points (the first and last set the line)".into(),
        );
    }
    if !axes.iter().any(|&a| a) {
        return Err("Align needs at least one axis".into());
    }
    let n = sel.len();
    let a = mesh.vertices[sel[0] as usize];
    let dir = mesh.vertices[sel[n - 1] as usize] - a;
    if dir.length() < 1e-12 {
        return Err("Align needs distinct first and last points".into());
    }
    let mut out = mesh.clone();
    for (i, &v) in sel.iter().enumerate().take(n - 1).skip(1) {
        let p = mesh.vertices[v as usize];
        // CMD-V-06: a point keeps its place along the line's dominant axis
        // ((1, 0.2) on the line to (3, 0.4) goes to (1, 0.133)).
        let major = (0..3)
            .max_by(|&i, &j| dir[i].abs().total_cmp(&dir[j].abs()))
            .unwrap_or(0);
        let t = if uniform {
            i as f64 / (n - 1) as f64
        } else {
            (p - a)[major] / dir[major]
        };
        let target = a + dir * t;
        for k in 0..3 {
            if axes[k] {
                out.vertices[v as usize][k] = target[k];
            }
        }
    }
    // CMD-V-06 / CMD-O-vert-align-all: a polygon whose points all end up on
    // the line keeps existing with no area, as a recorded degenerate face.
    let moved: BTreeSet<u32> = sel.iter().copied().collect();
    for f in 0..out.faces.len() {
        if out.faces[f].vertices.iter().any(|v| moved.contains(v))
            && !out.is_polyline(f)
            && out.normal(f) == DVec3::ZERO
        {
            out.attributes.degenerate_faces.insert(f as u32);
        }
    }
    Ok((finish(out, "Align")?, sel.to_vec()))
}

fn reorder(mesh: &Mesh, sel: &[u32], mode: ReorderMode) -> Result<(Mesh, Vec<u32>)> {
    let n = mesh.vertices.len();
    let set: BTreeSet<u32> = sel.iter().copied().collect();
    let others = (0..n).filter(|&i| !set.contains(&(i as u32)));
    // order[new] = old.
    let order: Vec<usize> = match mode {
        ReorderMode::Swap => {
            let mut slots: Vec<u32> = sel.to_vec();
            slots.sort_unstable();
            let mut order: Vec<usize> = (0..n).collect();
            for (k, &v) in sel.iter().enumerate() {
                order[slots[k] as usize] = v as usize;
            }
            order
        }
        ReorderMode::Top => sel.iter().map(|&v| v as usize).chain(others).collect(),
        ReorderMode::Bottom => others.chain(sel.iter().map(|&v| v as usize)).collect(),
    };
    let mut new_of_old = vec![0u32; n];
    for (new, &old) in order.iter().enumerate() {
        new_of_old[old] = new as u32;
    }
    let out = crate::mesh_cleanup::remap_points(mesh, &order)?;
    Ok((out, sel.iter().map(|&v| new_of_old[v as usize]).collect()))
}

/// Append one polygon with unassigned corner records, taking its tags from
/// `template` (or plain defaults).
pub(crate) fn append_face(out: &mut Mesh, vertices: Vec<u32>, template: Option<usize>) {
    let corners = vertices.len();
    let source = template.map(|f| out.faces[f].clone());
    out.faces.push(Face {
        vertices,
        uv: source
            .as_ref()
            .and_then(|f| f.uv.as_ref())
            .map(|_| vec![glam::DVec2::ZERO; corners]),
        material: source
            .as_ref()
            .map(|f| f.material.clone())
            .unwrap_or_default(),
        smooth: source.as_ref().is_some_and(|f| f.smooth),
        part: source.as_ref().and_then(|f| f.part.clone()),
        material_label: source.as_ref().and_then(|f| f.material_label.clone()),
    });
    for map in out.uv_maps.values_mut() {
        map.push(vec![None; corners]);
    }
    let a = &mut out.attributes;
    for map in a.uv_records.values_mut() {
        map.push(vec![None; corners]);
    }
    for map in a.corner_weights.values_mut() {
        map.push(vec![None; corners]);
    }
    for map in a.corner_colors.values_mut() {
        map.push(vec![None; corners]);
    }
    for map in a.corner_normals.values_mut() {
        map.push(vec![None; corners]);
    }
    for map in a.typed_maps.values_mut() {
        if !map.corners.is_empty() {
            map.corners.push(vec![None; corners]);
        }
    }
    if !a.smoothing_labels.is_empty() {
        let label = template.and_then(|f| a.smoothing_labels[f].clone());
        a.smoothing_labels.push(label);
    }
    if !a.representations.is_empty() {
        a.representations.push(PolygonRepresentation::Face);
    }
}

/// Make Polygon's Type (make-split-2026-11-05). Auto takes a neighbouring
/// polygon's type (Face for floating points) and makes a line for two points
/// and a one-point polygon for one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MakeType {
    #[default]
    Auto,
    Face,
    Subdivs,
    CatmullClark,
    Curve,
    Polyline,
}

/// Make Polygon (P key: Auto) from the ordered point selection.
pub fn make_polygon_from_points(mesh: &Mesh, selected: &[u32]) -> Result<(Mesh, u32)> {
    make_polygon_typed(mesh, selected, MakeType::Auto, false)
}

/// Make Polygon from points (make-split-2026-11-05 rules 1–6): one polygon
/// through the points in selection order (a crossing order makes a crossing
/// polygon). Where the points border existing polygons the new one is wound
/// to agree with them; floating points keep the selection order. Curve and
/// Polyline make an open curve (Close Curve closes a Curve).
pub fn make_polygon_typed(
    mesh: &Mesh,
    selected: &[u32],
    kind: MakeType,
    close: bool,
) -> Result<(Mesh, u32)> {
    match make_in_order(mesh, selected, kind, close) {
        Ok(made) => Ok(made),
        // Points around an opening selected in another order (a lasso):
        // follow the opening's border instead (Jason's GUI test, 2026-10-31).
        Err(first) => match border_loop(mesh, selected) {
            Some(around) => make_in_order(mesh, &around, kind, close).map_err(|_| first),
            None => Err(first),
        },
    }
}

/// The selected points in the order of the open border they make up, when
/// they are exactly one border loop.
fn border_loop(mesh: &Mesh, selected: &[u32]) -> Option<Vec<u32>> {
    let set: BTreeSet<u32> = selected.iter().copied().collect();
    let counts = mesh.edge_counts();
    let edges: Vec<(u32, u32)> = counts
        .iter()
        .filter(|&(&(a, b), &n)| n == 1 && set.contains(&a) && set.contains(&b))
        .map(|(&e, _)| e)
        .collect();
    if edges.len() != set.len() || edges.len() < 3 {
        return None;
    }
    let mut next: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(a, b) in &edges {
        next.entry(a).or_default().push(b);
        next.entry(b).or_default().push(a);
    }
    if next.len() != set.len() || next.values().any(|n| n.len() != 2) {
        return None;
    }
    let start = edges[0].0;
    let (mut out, mut previous, mut at) = (vec![start], start, edges[0].1);
    while at != start && out.len() <= set.len() {
        out.push(at);
        let n = &next[&at];
        let step = if n[0] == previous { n[1] } else { n[0] };
        previous = at;
        at = step;
    }
    (out.len() == set.len()).then_some(out)
}

fn make_in_order(
    mesh: &Mesh,
    selected: &[u32],
    kind: MakeType,
    close: bool,
) -> Result<(Mesh, u32)> {
    let mut order = ordered(mesh, selected, "Make Polygon")?;
    if order.is_empty() {
        return Err("Make Polygon needs selected points; select geometry first".into());
    }
    if order.len() > 4096 {
        return Err("Make Polygon supports at most 4096 points".into());
    }
    let curve = matches!(kind, MakeType::Curve | MakeType::Polyline);
    let surface = !curve && order.len() >= 3;
    // Neighbour winding: a new edge a→b must run opposite to an existing
    // polygon's a→b. The first such edge decides.
    let mut neighbour = None;
    if surface {
        let n = order.len();
        'search: for i in 0..n {
            let (a, b) = (order[i], order[(i + 1) % n]);
            for (f, face) in mesh.faces.iter().enumerate() {
                if mesh.is_polyline(f) {
                    continue;
                }
                let m = face.vertices.len();
                for k in 0..m {
                    let (x, y) = (face.vertices[k], face.vertices[(k + 1) % m]);
                    if (x, y) == (a, b) || (x, y) == (b, a) {
                        if (x, y) == (a, b) {
                            order.reverse();
                        }
                        neighbour = Some(f);
                        break 'search;
                    }
                }
            }
        }
    }
    let template = neighbour.or_else(|| {
        mesh.faces
            .iter()
            .position(|f| f.vertices.iter().any(|v| order.contains(v)))
    });
    let representation = match kind {
        // Two points make a line, one a one-point polygon (rule 2: MP05,
        // MP06), stored as a one-point line.
        MakeType::Auto if order.len() <= 2 => Some(PolygonRepresentation::Polyline {
            closed: false,
            curve: crate::curves::CurveKind::Line,
        }),
        MakeType::Auto => neighbour
            .and_then(|f| mesh.attributes.representations.get(f).copied())
            .filter(|r| !matches!(r, PolygonRepresentation::Polyline { .. })),
        MakeType::Face => None,
        MakeType::Subdivs => Some(PolygonRepresentation::Subdivs),
        MakeType::CatmullClark => Some(PolygonRepresentation::CatmullClark),
        MakeType::Curve => Some(PolygonRepresentation::Polyline {
            closed: close,
            curve: crate::curves::CurveKind::Curve {
                start_control: false,
                end_control: false,
            },
        }),
        MakeType::Polyline => Some(PolygonRepresentation::Polyline {
            closed: false,
            curve: crate::curves::CurveKind::Line,
        }),
    };
    if matches!(
        kind,
        MakeType::Face | MakeType::Subdivs | MakeType::CatmullClark
    ) && order.len() < 3
    {
        return Err("Make Polygon needs three or more points for this type".into());
    }
    if curve && order.len() < 2 {
        return Err("Make Polygon needs two or more points for a curve".into());
    }
    let mut out = mesh.clone();
    let before = out.faces.len();
    append_face(&mut out, order, template);
    let face = before as u32;
    match representation {
        Some(r) => {
            if out.attributes.representations.is_empty() {
                out.attributes.representations = vec![PolygonRepresentation::Face; before];
            }
            out.attributes.representations.truncate(before);
            out.attributes.representations.push(r);
        }
        None if !out.attributes.representations.is_empty() => {
            out.attributes.representations.truncate(before);
            out.attributes
                .representations
                .push(PolygonRepresentation::Face);
        }
        None => {}
    }
    // A crossing order makes the crossing polygon asked for (rule 1), even
    // when its signed area cancels out.
    if !out.is_polyline(face as usize) && out.normal(face as usize) == glam::DVec3::ZERO {
        out.attributes.degenerate_faces.insert(face);
    }
    out.validate()
        .map_err(|e| format!("Make Polygon couldn't build that polygon ({e}); nothing changed"))?;
    Ok((out, face))
}

/// N04: picks in order, and each off-plane pick paired with its mirror.
type SymmetricPicks = (Vec<u32>, Vec<(u32, u32)>);

/// MV-C01 / MV-T01: append points at local positions. Returns the new ids.
pub fn add_points(mesh: &Mesh, points: &[DVec3]) -> Result<(Mesh, Vec<u32>)> {
    let mut out = mesh.clone();
    let mut ids = Vec::with_capacity(points.len());
    for &p in points {
        if !p.is_finite() || p.abs().max_element() > 1e9 {
            return Err("Point coordinates must be finite and within the scene range".into());
        }
        ids.push(u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?);
        out.vertices.push(p);
    }
    Ok((finish(out, "Create")?, ids))
}

/// MV-T05 Weld: `sources` weld onto `target` at its position.
pub fn weld_points(mesh: &Mesh, sources: &[u32], target: u32) -> Result<(Mesh, Vec<u32>)> {
    let mut members = ordered(mesh, sources, "Weld")?;
    members.retain(|&v| v != target);
    if members.is_empty() {
        return Err("Weld needs a source point other than the target".into());
    }
    if target as usize >= mesh.vertices.len() {
        return Err("The weld target no longer exists".into());
    }
    members.push(target);
    join(mesh, &members, false, MergeOptions::default(), "Weld")
}

/// Where a Weld drag (WD-01..05) was released.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WeldDrop {
    /// Snapped onto this other point of the same mesh: merge into it.
    Point(u32),
    /// Over empty space: the dragged point moves to this world position.
    Position(DVec3),
}

/// WD-03 / WD-05: the geometry of one Weld drag of point `source`.
///
/// `WeldDrop::Point(target)` merges `source` into `target` at the target's
/// position: the target survives (keeping its id and point attributes), the
/// dragged point disappears, an edge between the two collapses and polygons
/// left with fewer than three corners are removed, as Join does.
/// `WeldDrop::Position(local)` only moves `source` there.
///
/// Returns the mesh and the old→new point map (`None` for the removed point).
pub fn weld_drag_geometry(
    mesh: &Mesh,
    source: u32,
    drop: WeldDrop,
) -> Result<(Mesh, Vec<Option<u32>>)> {
    if source as usize >= mesh.vertices.len() {
        return Err("The dragged point no longer exists".into());
    }
    match drop {
        WeldDrop::Position(local) => {
            if !local.is_finite() || local.abs().max_element() > 1e9 {
                return Err("Point coordinates must be finite and within the scene range".into());
            }
            let mut out = mesh.clone();
            out.vertices[source as usize] = local;
            let remap = (0..mesh.vertices.len() as u32).map(Some).collect();
            Ok((finish(out, "Weld")?, remap))
        }
        WeldDrop::Point(target) => {
            if target == source {
                return Err("Weld needs a source point other than the target".into());
            }
            if target as usize >= mesh.vertices.len() {
                return Err("The weld target no longer exists".into());
            }
            let position = mesh.vertices[target as usize];
            let (out, remap) = weld_keeping(
                mesh,
                &[(vec![source, target], position)],
                |_| Some(target),
                "Weld",
            )?;
            let mut remap = remap;
            remap[source as usize] = None;
            Ok((finish(out, "Weld")?, remap))
        }
    }
}

/// Corner-cut shared by Vertex Bevel and Vertex Extrude: every selected
/// interior point with a closed fan gets one new point per incident edge at
/// `distance` along it; each polygon's corner is replaced by the two cut
/// points. `cap` then receives the fan's cut points in order (wound against
/// the polygons) and the current faces/vertices to add its own geometry.
/// Where the UVs of a fan's corners come from, per fan slot: the cut toward
/// the previous and next point, and the original corner at the cut point.
struct FanUvs {
    /// Per fan slot, the polygon's cut point toward its previous corner (its
    /// cut toward the next corner is `ring[slot]`).
    prev_cut: Vec<u32>,
    prev: Vec<Option<CornerBlend>>,
    next: Vec<Option<CornerBlend>>,
    tip: Vec<Option<CornerBlend>>,
}

/// A corner's UV source as a blend: its donor, or the blend it already has.
fn corner_source(f: &OutFace, i: usize) -> Option<CornerBlend> {
    match f.donors[i] {
        Some(donor) => Some(vec![(donor, 1.)]),
        None => f.blends.get(i).cloned().flatten(),
    }
}

/// `(1 - t) * a + t * b` as one blend.
fn lerp_source(a: &Option<CornerBlend>, b: &Option<CornerBlend>, t: f64) -> Option<CornerBlend> {
    let (a, b) = (a.as_ref()?, b.as_ref()?);
    Some(
        a.iter()
            .map(|&(c, w)| (c, w * (1. - t)))
            .chain(b.iter().map(|&(c, w)| (c, w * t)))
            .collect(),
    )
}

/// One cut fan, as the cap callback sees it.
struct FanCut<'a> {
    /// The selected point and its position before the cut.
    point: u32,
    position: DVec3,
    /// The distance asked for along each edge, before any clamp.
    distance: f64,
    /// The polygons close around the point (an interior point).
    closed: bool,
    /// Per fan slot, the polygon's cut toward its next corner, in cap order.
    ring: &'a [u32],
    /// Per fan slot, the output polygon's index and its source polygon.
    face_index: &'a [usize],
    templates: &'a [usize],
    uvs: &'a FanUvs,
}

/// Returns the mesh and the cut points (renumbered) in the order made.
fn corner_cut(
    mesh: &Mesh,
    sel: &[u32],
    distance_at: &dyn Fn(u32) -> f64,
    what: &str,
    min_open: usize,
    cap: impl FnMut(&FanCut, &mut Vec<DVec3>, &mut Vec<OutFace>) -> Result<()>,
) -> Result<(Mesh, Vec<u32>)> {
    corner_cut_full(mesh, sel, distance_at, what, min_open, cap).map(|(m, cuts, _)| (m, cuts))
}

/// [`corner_cut`], also returning the old index of every point kept (a
/// selected point no polygon uses any more is dropped).
fn corner_cut_full(
    mesh: &Mesh,
    sel: &[u32],
    distance_at: &dyn Fn(u32) -> f64,
    what: &str,
    // The smallest open fan (a border point) that is cut: Vertex Extrude
    // needs two polygons (VE03), Vertex Bevel cuts a lone corner (VX-B03).
    min_open: usize,
    mut cap: impl FnMut(&FanCut, &mut Vec<DVec3>, &mut Vec<OutFace>) -> Result<()>,
) -> Result<(Mesh, Vec<u32>, Vec<usize>)> {
    let mut faces = source_faces(mesh, what)?;
    let mut vertices = mesh.vertices.clone();
    let mut copies = Vec::new();
    let mut cuts = Vec::new();
    // Cut point → the selected point it was cut from.
    let mut cut_origin: BTreeMap<u32, u32> = BTreeMap::new();
    for &v in sel {
        let (order, closed) = fan(&faces, v, what)?;
        if closed && order.len() < 3 {
            return Err(format!(
                "{what} needs three or more polygons around an interior point"
            ));
        }
        if !closed && order.len() < min_open {
            return Err(format!(
                "{what} needs a border point with {min_open} or more polygons"
            ));
        }
        let pv = vertices[v as usize];
        let distance = distance_at(v);
        // Cut point on the edge from v toward each neighbour, and how far
        // along that edge it lies (for UVs).
        let mut cut: BTreeMap<u32, u32> = BTreeMap::new();
        let mut along_t: BTreeMap<u32, f64> = BTreeMap::new();
        for &(fi, c) in &order {
            let f = &faces[fi];
            let n = f.vertices.len();
            for w in [f.vertices[(c + n - 1) % n], f.vertices[(c + 1) % n]] {
                if cut.contains_key(&w) {
                    continue;
                }
                // A neighbour that is an earlier point's cut lies on the edge
                // to that point: measure the whole original edge, so big
                // distances let the two cuts pass (VX-B05, VX-B9).
                let far = cut_origin
                    .get(&w)
                    .map_or(vertices[w as usize], |&o| mesh.vertices[o as usize]);
                let along = far - pv;
                let length = along.length();
                if length <= 1e-12 {
                    return Err(format!("{what} needs edges longer than zero"));
                }
                // VX-B05 / VX-E05: a distance longer than the edge stops at
                // its far end (not merged).
                let d = distance.min(length);
                let id = push_point(&mut vertices, pv + along / length * d)?;
                copies.push((id, v));
                cuts.push(id);
                cut_origin.insert(id, v);
                cut.insert(w, id);
                let to_w = (vertices[w as usize] - pv).length();
                along_t.insert(w, if to_w > 1e-12 { d / to_w } else { 1. });
            }
        }
        // Replace v in each polygon by (cut toward prev, cut toward next).
        let mut ring = Vec::with_capacity(order.len());
        let mut fan_faces = Vec::with_capacity(order.len());
        let mut face_index = Vec::with_capacity(order.len());
        let mut uvs = FanUvs {
            prev_cut: Vec::new(),
            prev: Vec::new(),
            next: Vec::new(),
            tip: Vec::new(),
        };
        for &(fi, c) in &order {
            let f = &mut faces[fi];
            let n = f.vertices.len();
            let (prev, next) = (f.vertices[(c + n - 1) % n], f.vertices[(c + 1) % n]);
            // Each cut corner's UV blends this polygon's corner at v with its
            // corner at the neighbour, as far along as the cut point.
            let tip = corner_source(f, c);
            let to_prev = lerp_source(&tip, &corner_source(f, (c + n - 1) % n), along_t[&prev]);
            let to_next = lerp_source(&tip, &corner_source(f, (c + 1) % n), along_t[&next]);
            if f.blends.is_empty() {
                f.blends = vec![None; n];
            }
            f.vertices.splice(c..=c, [cut[&prev], cut[&next]]);
            f.donors.splice(c..=c, [None, None]);
            f.blends.splice(c..=c, [to_prev.clone(), to_next.clone()]);
            ring.push(cut[&next]);
            uvs.prev_cut.push(cut[&prev]);
            fan_faces.push(f.template);
            face_index.push(fi);
            uvs.prev.push(to_prev);
            uvs.next.push(to_next);
            uvs.tip.push(tip);
        }
        let first_new = vertices.len();
        let fan = FanCut {
            point: v,
            position: pv,
            distance,
            closed,
            ring: &ring,
            face_index: &face_index,
            templates: &fan_faces,
            uvs: &uvs,
        };
        cap(&fan, &mut vertices, &mut faces)?;
        // Points the cap made (rounding) take the cut point's records.
        copies.extend((first_new as u32..vertices.len() as u32).map(|id| (id, v)));
    }
    let out = rebuild(mesh, vertices, faces, &copies, what)?;
    let set: BTreeSet<u32> = sel.iter().copied().collect();
    let cleaned = crate::mesh_cleanup::remove_unreferenced(&finish(out, what)?, Some(&set))?;
    let cuts = cuts
        .into_iter()
        .filter_map(|id| {
            cleaned
                .survivors
                .binary_search(&(id as usize))
                .ok()
                .map(|i| i as u32)
        })
        .collect();
    Ok((finish(cleaned.mesh, what)?, cuts, cleaned.survivors))
}

fn push_point(vertices: &mut Vec<DVec3>, at: DVec3) -> Result<u32> {
    let id = u32::try_from(vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
    vertices.push(at);
    Ok(id)
}

/// `Σ weight · blend` as one blend; absent if any part is.
fn mix_sources(parts: &[(&Option<CornerBlend>, f64)]) -> Option<CornerBlend> {
    let mut out = Vec::new();
    for &(blend, w) in parts {
        out.extend(blend.as_ref()?.iter().map(|&(c, x)| (c, x * w)));
    }
    Some(out)
}

/// The fillet from cut offset `a` to cut offset `b` (both from the corner
/// point): the circular arc tangent to both edges at the cut points
/// (VX-B02), at fraction `t` of its turn, as weights `(α, β)` with point =
/// corner + α·a + β·b. When the two cuts lie at different distances (a
/// clamped edge) the same arc is stretched to meet both (provisional).
/// Edges in a straight line give the straight segment.
fn fillet(a: DVec3, b: DVec3, t: f64) -> (f64, f64) {
    let c = a
        .normalize_or_zero()
        .dot(b.normalize_or_zero())
        .clamp(-1., 1.);
    // The arc turns through π minus the corner's angle.
    let turn = std::f64::consts::PI - c.acos();
    let st = turn.sin();
    if st < 1e-6 {
        return (1. - t, t);
    }
    let (s1, s2) = (((1. - t) * turn).sin(), (t * turn).sin());
    (
        (1. + (c * s1 - s2) / st) / (1. + c),
        (1. + (c * s2 - s1) / st) / (1. + c),
    )
}

/// Whether the fan's polygons disagree on the point's UV in some map (a UV
/// seam runs through it).
fn uv_seam_at(source: &Mesh, fan: &FanCut) -> bool {
    source.uv_maps.values().any(|rows| {
        let at: Vec<Option<glam::DVec2>> = fan
            .uvs
            .tip
            .iter()
            .map(|t| {
                t.as_ref()
                    .and_then(|b| crate::edge_commands::blend_uv(rows, b))
            })
            .collect();
        at.windows(2).any(|w| match (w[0], w[1]) {
            (Some(a), Some(b)) => a.distance(b) > 1e-9,
            (a, b) => a.is_some() != b.is_some(),
        })
    })
}

/// UVN-T-04 (Vertex Bevel at a UV seam): each cap corner takes the UV
/// interpolated toward its cut inside a polygon on the cut's edge that
/// carries the point's continuous UV; where neither polygon on that edge
/// does, the corner takes the point's continuous UV. Without a seam this is
/// the plain interpolation; at a fold (polygons not in one plane) the cap
/// keeps it too (provisional). The choice is one for all UV maps (the blends
/// are shared), so a polygon must carry the continuous UV in every map.
fn cap_seam_uvs(
    source: &Mesh,
    fan: &FanCut,
    cuts: &[u32],
    cut_uv: Vec<Option<CornerBlend>>,
) -> Vec<Option<CornerBlend>> {
    if source.uv_maps.is_empty()
        || !uv_seam_at(source, fan)
        || !crate::corner_uv::flat(source, fan.templates.iter().copied())
    {
        return cut_uv;
    }
    let k = fan.ring.len();
    let v = fan.point;
    // Per map, the point's continuous UV and each fan slot's UV at it.
    let carries: Vec<bool> = (0..k)
        .map(|slot| {
            source.uv_maps.iter().all(|(name, rows)| {
                let at = |j: usize| {
                    fan.uvs.tip[j]
                        .as_ref()
                        .and_then(|b| crate::edge_commands::blend_uv(rows, b))
                };
                let continuous = crate::corner_uv::continuous_uv(
                    source.attributes.point_uvs.get(name),
                    v,
                    (0..k).map(|j| (fan.templates[j], at(j))),
                );
                crate::corner_uv::is_continuous(at(slot), continuous)
            })
        })
        .collect();
    // The continuous UV as a blend: the tip of a slot that carries it (the
    // lowest-numbered polygon when none does, as `continuous_uv`).
    let tip = (0..k)
        .filter(|&j| carries[j])
        .chain((0..k).min_by_key(|&j| fan.templates[j]))
        .find_map(|j| fan.uvs.tip[j].clone());
    cuts.iter()
        .zip(cut_uv)
        .map(|(cut, blend)| {
            // The two slots on this cut's edge: one cuts toward it as its
            // next corner, the other as its previous.
            let next = (0..k).find(|&j| fan.ring[j] == *cut);
            let prev = (0..k).find(|&j| fan.uvs.prev_cut[j] == *cut);
            match (next, prev) {
                (Some(j), _) if carries[j] => fan.uvs.next[j].clone(),
                (_, Some(j)) if carries[j] => fan.uvs.prev[j].clone(),
                (None, None) => blend,
                _ => tip.clone().or(blend),
            }
        })
        .collect()
}

/// The UV fan polygon `slot` has at `at` (projected into its corner's
/// plane): an affine blend of its corner at the point and its two cuts.
fn polygon_uv(
    fan: &FanCut,
    slot: usize,
    p: DVec3,
    at: DVec3,
    vertices: &[DVec3],
) -> Option<CornerBlend> {
    let (alpha, beta) = corner_coords(fan, slot, p, at, vertices);
    mix_sources(&[
        (&fan.uvs.tip[slot], 1. - alpha - beta),
        (&fan.uvs.next[slot], alpha),
        (&fan.uvs.prev[slot], beta),
    ])
}

/// Vertex Bevel's cap for one fan (VX-B01..B03, VX-Q02). Level 0 is one
/// polygon through the cut points. Level L rounds it: each side between two
/// cut points becomes a fillet of 2L segments, also inserted into the
/// polygon on that side; the cap is one L×L grid of quads per cut point,
/// meeting at a centre point. An open fan's last side (along the border)
/// stays straight; a lone corner gets its fillet and no cap.
///
/// Fillets come from the Inset and the edge directions only, so a clamped
/// cut kinks the border (VX-B08); new points take the old surface's UVs
/// where they sit (VX-B09).
///
/// Inside rule **[OP]** (the spec leaves it open within 0.01 of its check
/// values): the centre is the mean of the cut points (VX-B08), lifted along
/// the normal on a corner by the side midpoints' mean pushed on by a quarter
/// of their offset from the cuts' mean; each spoke from the centre to a side midpoint is the quadratic
/// `O + s²(m − O) + s(1 − s)·R·t̂` with `t̂` the spoke's direction across
/// the surface and `R` the geometric mean of the cut and midpoint reaches;
/// the grid inside each sector is the Coons patch of its four sides.
fn bevel_cap(
    source: &Mesh,
    fan: &FanCut,
    level: usize,
    vertices: &mut Vec<DVec3>,
    faces: &mut Vec<OutFace>,
) -> Result<()> {
    let k = fan.ring.len();
    // The cut points around the cap, in cap order; an open fan adds its
    // last polygon's cut toward its previous corner.
    let mut cuts = fan.ring.to_vec();
    let mut cut_uv = fan.uvs.next.clone();
    if !fan.closed {
        cuts.push(fan.uvs.prev_cut[k - 1]);
        cut_uv.push(fan.uvs.prev[k - 1].clone());
    }
    let n = cuts.len();
    let template = fan.templates[0];
    if level == 0 {
        if n >= 3 {
            let blends = cap_seam_uvs(source, fan, &cuts, cut_uv);
            faces.push(OutFace {
                template,
                vertices: cuts,
                donors: vec![None; n],
                blends,
            });
        }
        return Ok(());
    }
    let steps = 2 * level;
    let p = fan.position;
    // Side j runs from cuts[j] to cuts[j + 1]. Sides 0..k are fillets inside
    // fan polygon j; an open fan's last side is straight.
    let mut border = Vec::with_capacity(n * steps);
    let mut uv: BTreeMap<u32, Option<CornerBlend>> = BTreeMap::new();
    for j in 0..n {
        if j >= k && n < 3 {
            break;
        }
        border.push(cuts[j]);
        uv.insert(cuts[j], cut_uv[j].clone());
        let (a, b) = (
            vertices[cuts[j] as usize],
            vertices[cuts[(j + 1) % n] as usize],
        );
        // VX-B08: a fillet comes from the Inset and the two edge directions
        // only. A clamped cut point still sits at the edge's end, so the
        // border kinks there; the fillet isn't refitted.
        let (da, db) = (
            (a - p).normalize_or_zero() * fan.distance,
            (b - p).normalize_or_zero() * fan.distance,
        );
        // The same point as a share of each actual cut offset (for UVs).
        let share = |offset: DVec3| {
            let l = offset.length();
            if l > 1e-12 { fan.distance / l } else { 0. }
        };
        let (sa, sb) = (share(a - p), share(b - p));
        let mut side = Vec::with_capacity(steps - 1);
        for s in 1..steps {
            let t = s as f64 / steps as f64;
            let (at, blend) = if j < k {
                let (alpha, beta) = fillet(da, db, t);
                let (wa, wb) = (alpha * sa, beta * sb);
                // VX-B09: the UV the polygon had at that position.
                (
                    p + da * alpha + db * beta,
                    mix_sources(&[
                        (&fan.uvs.tip[j], 1. - wa - wb),
                        (&fan.uvs.next[j], wa),
                        (&fan.uvs.prev[j], wb),
                    ]),
                )
            } else {
                (
                    a.lerp(b, t),
                    lerp_source(&cut_uv[j], &cut_uv[(j + 1) % n], t),
                )
            };
            let id = push_point(vertices, at)?;
            uv.insert(id, blend);
            side.push(id);
            border.push(id);
        }
        if j < k {
            // Fan polygon j runs prev cut → next cut, the side reversed:
            // its fillet points go in after the prev cut (VX-B02).
            let f = &mut faces[fan.face_index[j]];
            let at = f
                .vertices
                .iter()
                .position(|&x| x == cuts[(j + 1) % n])
                .ok_or("Vertex Bevel lost a cut point")?
                + 1;
            let blends: Vec<_> = side.iter().rev().map(|id| uv[id].clone()).collect();
            f.vertices.splice(at..at, side.iter().rev().copied());
            f.donors.splice(at..at, vec![None; side.len()]);
            f.blends.splice(at..at, blends);
        }
    }
    if n < 3 {
        return Ok(());
    }
    let seam = uv_seam_at(source, fan);
    // VX-B11 step 1: the inside is built as if no cut were clamped; each
    // clamped cut goes back to its place afterwards and shifts the inside.
    let mut clamped = Vec::new();
    for &c in &cuts {
        let off = vertices[c as usize] - p;
        let len = off.length();
        if len > 1e-12 && fan.distance - len > 1e-9 {
            let dir = off / len;
            clamped.push((c, vertices[c as usize], fan.distance - len, dir));
            vertices[c as usize] = p + dir * fan.distance;
        }
    }
    let point = |id: u32, vertices: &[DVec3]| vertices[id as usize];
    let mids: Vec<u32> = (0..n).map(|j| border[j * steps + level]).collect();
    let mean = |ids: &[u32], vertices: &[DVec3]| {
        ids.iter().map(|&i| point(i, vertices)).sum::<DVec3>() / ids.len() as f64
    };
    let (mean_mid, mean_cut) = (mean(&mids, vertices), mean(&cuts, vertices));
    let normal = fan
        .templates
        .iter()
        .map(|&f| source.normal(f))
        .sum::<DVec3>()
        .normalize_or_zero();
    // VX-B08: across the surface the centre is the mean of the cut points;
    // a corner lifts it along the normal by the dome rule (the side
    // midpoints' mean pushed on by a quarter of their offset from the cuts').
    let dome = mean_mid + (mean_mid - mean_cut) * 0.25;
    let centre = mean_cut + normal * (dome - mean_cut).dot(normal);
    let reach = |ids: &[u32], vertices: &[DVec3]| {
        ids.iter()
            .map(|&i| point(i, vertices).distance(centre))
            .sum::<f64>()
            / ids.len() as f64
    };
    let radius = (reach(&cuts, vertices) * reach(&mids, vertices)).sqrt();
    let centre_id = push_point(vertices, centre)?;
    // spokes[j][i]: from the centre (i = 0) to side j's midpoint (i = L).
    let mut spokes = Vec::with_capacity(n);
    for &m in &mids {
        let out = point(m, vertices) - centre;
        let across = (out - normal * out.dot(normal)).normalize_or_zero();
        // Kept below twice the spoke's length so it never runs past its end.
        let r = radius.min(2. * out.length());
        let mut spoke = vec![centre_id];
        for i in 1..level {
            let s = i as f64 / level as f64;
            let at = centre + out * (s * s) + across * (r * s * (1. - s));
            spoke.push(push_point(vertices, at)?);
        }
        spoke.push(m);
        spokes.push(spoke);
    }
    // Sector j, around cuts[j]: g(a, b) runs a along side j toward its
    // midpoint and b back along side j - 1 toward its midpoint.
    let ring_len = n * steps;
    // Each quad, and for a UV seam the fan polygon it reads its UVs off.
    let mut quads = Vec::with_capacity(n * level * level);
    for j in 0..n {
        let before = (j + n - 1) % n;
        let side = |a: usize| border[j * steps + a];
        let back = |b: usize| border[(j * steps + ring_len - b) % ring_len];
        let mut g = vec![vec![0u32; level + 1]; level + 1];
        for a in 0..=level {
            g[a][0] = side(a);
            g[a][level] = spokes[before][level - a];
        }
        for b in 0..=level {
            g[0][b] = back(b);
            g[level][b] = spokes[j][level - b];
        }
        for a in 1..level {
            for b in 1..level {
                let (u, v) = (a as f64 / level as f64, b as f64 / level as f64);
                // Coons patch: the two ruled blends less the bilinear one.
                let weights = [
                    (g[a][0], 1. - v),
                    (g[a][level], v),
                    (g[0][b], 1. - u),
                    (g[level][b], u),
                    (g[0][0], -(1. - u) * (1. - v)),
                    (g[level][0], -u * (1. - v)),
                    (g[0][level], -(1. - u) * v),
                    (g[level][level], -u * v),
                ];
                let at = weights
                    .iter()
                    .map(|&(id, w)| point(id, vertices) * w)
                    .sum::<DVec3>();
                g[a][b] = push_point(vertices, at)?;
            }
        }
        for a in 0..level {
            for b in 0..level {
                let quad = [g[a][b], g[a + 1][b], g[a + 1][b + 1], g[a][b + 1]];
                // Across a UV seam (UVN-T-04) each quad reads its corners'
                // UVs off the one polygon it was cut out of: the half of the
                // sector along side j lies in fan polygon j, the other half
                // in polygon j - 1 (provisional: not probed with rounding).
                let slot = if a >= b || before >= k {
                    j.min(k - 1)
                } else {
                    before
                };
                quads.push((quad, slot));
            }
        }
    }
    // VX-B11 steps 2-3: each clamped cut goes back to its place and moves
    // every inside point by s × (a along + c across), s being its shortfall;
    // several clamped cuts add up. Arc points and other cuts stay.
    let flat = fan
        .templates
        .iter()
        .all(|&f| source.normal(f).dot(normal) > 1. - 1e-9);
    let measured = fan.closed && n == 4 && level <= 3 && flat;
    let inside = centre_id..vertices.len() as u32;
    let mut moves = vec![DVec3::ZERO; inside.len()];
    for &(c, actual, short, dir) in &clamped {
        vertices[c as usize] = actual;
        let along = -dir;
        let mut across = normal.cross(dir);
        if across.length_squared() < 1e-24 {
            across = dir.any_orthonormal_vector();
        }
        let across = across.normalize();
        // Table positions are for Inset 0.2; a and c don't scale.
        let scale = 0.2 / fan.distance;
        for (m, id) in moves.iter_mut().zip(inside.clone()) {
            let (a, c) = if id == centre_id {
                // The centre is the mean of the cut points (VX-B08).
                (1. / n as f64, 0.)
            } else {
                let rel = vertices[id as usize] - p;
                clamp_influence(
                    level,
                    rel.dot(dir) * scale,
                    rel.dot(across) * scale,
                    measured,
                )
            };
            *m += (along * a + across * c) * short;
        }
    }
    for (m, id) in moves.iter().zip(inside.clone()) {
        vertices[id as usize] += *m;
    }
    // VX-B09: an inside point takes the UV of the fan polygon it lies in,
    // at its place.
    for id in inside {
        let at = vertices[id as usize];
        uv.insert(
            id,
            polygon_uv(fan, home_slot(fan, p, at, vertices), p, at, vertices),
        );
    }
    for (quad, slot) in quads {
        // Across a UV seam (UVN-T-04) each quad reads its corners' UVs off
        // the one polygon it was cut out of: the half of the sector along
        // side j lies in fan polygon j, the other half in polygon j - 1
        // (provisional: not probed with rounding).
        let blends = if seam {
            quad.iter()
                .map(|&id| polygon_uv(fan, slot, p, vertices[id as usize], vertices))
                .collect()
        } else {
            quad.iter().map(|id| uv[id].clone()).collect()
        };
        faces.push(OutFace {
            template,
            vertices: quad.to_vec(),
            donors: vec![None; 4],
            blends,
        });
    }
    Ok(())
}

/// VX-B11's measured influence of one clamped cut on the inside points of a
/// flat four-polygon cap, per Round Level: (level, along, across, a, c) at
/// Inset 0.2, positions measured from the original point along the clamped
/// cut's edge (outward) and across it. Each row with an across stands for
/// its mirror too (−across, −c). Rows with a = c = 0 mark measured points
/// below 0.002. The centre (a = 1/n) is handled on its own.
const CLAMP_INFLUENCE: &[(usize, f64, f64, f64, f64)] = &[
    (2, 0.0845, 0., 0.718, 0.),
    (2, 0.0388, 0.0388, 0.438, 0.155),
    (2, 0., 0.0845, 0.141, 0.148),
    (2, -0.0388, 0.0388, 0.062, 0.025),
    (2, -0.0845, 0., 0., 0.),
    (3, 0.1144, 0., 0.858, 0.),
    (3, 0.0812, 0.0187, 0.691, 0.086),
    (3, 0.0578, 0., 0.565, 0.),
    (3, 0.0475, 0.0475, 0.473, 0.184),
    (3, 0.0279, 0.0279, 0.388, 0.115),
    (3, 0.0187, 0.0812, 0.249, 0.206),
    (3, 0., 0.0578, 0.202, 0.123),
    (3, 0., 0.1144, 0.072, 0.145),
    (3, -0.0279, 0.0279, 0.112, 0.023),
    (3, -0.0187, 0.0812, 0.060, 0.079),
    (3, -0.0475, 0.0475, 0.027, 0.023),
    (3, -0.0578, 0., 0.031, 0.),
    (3, -0.0812, 0.0187, 0., 0.008),
    (3, -0.1144, 0., -0.003, 0.),
];

/// `(a, c)` for an inside point at (`along`, `across`) (Inset-0.2 units).
/// The measured case (flat, four polygons, Round Level 1–3) takes its table
/// row, matched by position. Otherwise **[OP]** (VX-B11, "close enough"):
/// a smooth blend of the Level 3 rows, fading to 0 away from them.
fn clamp_influence(level: usize, along: f64, across: f64, measured: bool) -> (f64, f64) {
    let side = if across < 0. { -1. } else { 1. };
    let rows = CLAMP_INFLUENCE
        .iter()
        .filter(move |r| r.0 == if measured { level } else { 3 })
        .map(|&(_, al, ac, a, c)| ((al - along).hypot(ac - across.abs()), a, c));
    if measured {
        return rows
            .filter(|r| r.0 < 0.012)
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .map_or((0., 0.), |(_, a, c)| (a, c * side));
    }
    let (mut sum, mut a_sum, mut c_sum) = (0.01, 0., 0.);
    for (d, a, c) in rows {
        let w = (-(d / 0.02).powi(2)).exp();
        sum += w;
        a_sum += w * a;
        c_sum += w * c;
    }
    (a_sum / sum, side * c_sum / sum)
}

/// Where `at` sits in fan polygon `slot`'s corner: `at − p = α·(next cut −
/// p) + β·(prev cut − p)`, in that corner's plane.
fn corner_coords(fan: &FanCut, slot: usize, p: DVec3, at: DVec3, vertices: &[DVec3]) -> (f64, f64) {
    let a = vertices[fan.ring[slot] as usize] - p;
    let b = vertices[fan.uvs.prev_cut[slot] as usize] - p;
    let x = at - p;
    let (aa, ab, bb) = (a.dot(a), a.dot(b), b.dot(b));
    let det = aa * bb - ab * ab;
    if det.abs() > 1e-24 {
        (
            (x.dot(a) * bb - x.dot(b) * ab) / det,
            (x.dot(b) * aa - x.dot(a) * ab) / det,
        )
    } else {
        (0., 0.)
    }
}

/// The fan polygon whose corner wedge holds `at` best.
fn home_slot(fan: &FanCut, p: DVec3, at: DVec3, vertices: &[DVec3]) -> usize {
    (0..fan.ring.len())
        .max_by(|&i, &j| {
            let score = |s| {
                let (a, b) = corner_coords(fan, s, p, at, vertices);
                a.min(b)
            };
            score(i).total_cmp(&score(j))
        })
        .unwrap_or(0)
}

/// The highest Vertex Bevel Round Level (as Edge Bevel's, 0–32).
pub const VERTEX_BEVEL_MAX_ROUND_LEVEL: u32 = 32;

/// MV-T02 Vertex Bevel at Round Level 0: each selected point is replaced by
/// one point per incident edge at `inset` along it, joined by one new
/// polygon. Returns the mesh and the cut points (selected afterwards).
pub fn vertex_bevel_geometry(
    mesh: &Mesh,
    selected: &[u32],
    inset: f64,
) -> Result<(Mesh, Vec<u32>)> {
    vertex_bevel_geometry_weighted(mesh, selected, inset, None)
}

/// Vertex Bevel with a falloff: each point's inset is scaled by its weight
/// (FAL-T-03).
pub fn vertex_bevel_geometry_weighted(
    mesh: &Mesh,
    selected: &[u32],
    inset: f64,
    weights: Option<&[f64]>,
) -> Result<(Mesh, Vec<u32>)> {
    vertex_bevel_round(mesh, selected, inset, 0, weights)
}

/// Vertex Bevel (spec `vertex-tools-2026-11-14`, VX-B01..B06).
///
/// - An interior point becomes a cap through one cut point per edge; a
///   border point's cap leaves its open side along the border; a corner
///   point of one polygon is cut off with no cap (VX-B03).
/// - Round Level above 0 rounds the cap (VX-B02, VX-Q02; see [`bevel_cap`]).
/// - An Inset longer than an edge stops at the edge's far end (VX-B05).
///
/// Returns the mesh and the cut points, one per edge: the rounding points
/// aren't selected (VX-B06). The cap polygons follow the source polygons,
/// so [`new_vertex_tool_polygons`] names all of them (VX-B07).
pub fn vertex_bevel_round(
    mesh: &Mesh,
    selected: &[u32],
    inset: f64,
    round_level: u32,
    weights: Option<&[f64]>,
) -> Result<(Mesh, Vec<u32>)> {
    let sel = ordered(mesh, selected, "Vertex Bevel")?;
    if !inset.is_finite() || inset < 0. {
        return Err("Vertex Bevel Inset must be a finite distance of 0 or more".into());
    }
    if round_level > VERTEX_BEVEL_MAX_ROUND_LEVEL {
        return Err(format!(
            "Vertex Bevel Round Level must be a whole number from 0 to {VERTEX_BEVEL_MAX_ROUND_LEVEL}"
        ));
    }
    if inset == 0. {
        return Ok((mesh.clone(), sel));
    }
    corner_cut(
        mesh,
        &sel,
        &|v| inset * weights.map_or(1., |w| w[v as usize]),
        "Vertex Bevel",
        1,
        |fan, vertices, faces| bevel_cap(mesh, fan, round_level as usize, vertices, faces),
    )
}

/// MV-T03 Vertex Extrude: each selected interior point moves `extrude` along
/// its average normal; a cut point is made `width` along each incident edge
/// and a ring of triangles joins them to the moved point.
pub fn vertex_extrude_geometry(
    mesh: &Mesh,
    selected: &[u32],
    extrude: f64,
    width: f64,
) -> Result<(Mesh, Vec<u32>)> {
    vertex_extrude_weighted(mesh, selected, extrude, width, None)
}

/// Vertex Extrude (spec `extrude-2026-11-05`) with an optional Use Material
/// name for the new triangles. With nothing selected every point is
/// processed (VE08); Width 0 changes nothing, whatever the Extrude (VE06).
pub fn vertex_extrude_with(
    mesh: &Mesh,
    selected: &[u32],
    extrude: f64,
    width: f64,
    material: Option<&str>,
) -> Result<(Mesh, Vec<u32>)> {
    let all: Vec<u32>;
    let selected = if selected.is_empty() {
        all = (0..mesh.vertices.len() as u32).collect();
        &all[..]
    } else {
        selected
    };
    let (mut out, tips) = vertex_extrude_weighted(mesh, selected, extrude, width, None)?;
    if let Some(name) = material {
        let new = new_vertex_tool_polygons(mesh, &out);
        assign_material(&mut out, new, name);
    }
    Ok((out, tips))
}

/// Vertex Extrude with a falloff: each point's Extrude and Width scale by
/// its weight, and a point of weight 0 isn't extruded (FAL-B3-02).
pub fn vertex_extrude_weighted(
    mesh: &Mesh,
    selected: &[u32],
    extrude: f64,
    width: f64,
    weights: Option<&[f64]>,
) -> Result<(Mesh, Vec<u32>)> {
    let w = |v: u32| weights.map_or(1., |w| w[v as usize]);
    let kept: Vec<u32> = selected.iter().copied().filter(|&v| w(v) > 0.).collect();
    if kept.is_empty() && !selected.is_empty() {
        return Ok((mesh.clone(), selected.to_vec()));
    }
    let selected = &kept[..];
    let sel = ordered(mesh, selected, "Vertex Extrude")?;
    if !extrude.is_finite() || !width.is_finite() || width < 0. {
        return Err("Vertex Extrude needs finite Extrude and a Width of 0 or more".into());
    }
    // Width creates the geometry: Width 0 changes nothing (VE01, VE06).
    if width == 0. {
        return Ok((mesh.clone(), sel));
    }
    let normals: BTreeMap<u32, DVec3> = sel
        .iter()
        .map(|&v| {
            let n = (0..mesh.faces.len())
                .filter(|&f| mesh.faces[f].vertices.contains(&v))
                .map(|f| mesh.normal(f))
                .sum::<DVec3>();
            (v, n.normalize_or_zero())
        })
        .collect();
    // A border point with one polygon (a grid corner) is only cut off: no
    // apex and no triangle, so Extrude has no effect there and the point
    // itself goes (VE04, VE08; Q-EX-03's vertex part is open, so this
    // follows the measured result, provisional).
    let (out, _, survivors) = corner_cut_full(
        mesh,
        &sel,
        &|v| width * w(v),
        "Vertex Extrude",
        1,
        |fan, vertices, faces| {
            let (v, ring, fan_faces, uvs) = (fan.point, fan.ring, fan.templates, fan.uvs);
            if !fan.closed && ring.len() < 2 {
                return Ok(());
            }
            let normal = normals[&v];
            if normal == DVec3::ZERO && extrude != 0. {
                return Err("Vertex Extrude needs a point with a defined normal".into());
            }
            vertices[v as usize] += normal * extrude * w(v);
            let n = ring.len();
            for i in 0..n {
                // The polygon in fan slot i runs prev-cut → next-cut; its
                // triangle joins them to the moved point. On a border point
                // the open side stays open (VE03).
                // UVs fill the corner of fan polygon i that the cut removed.
                faces.push(OutFace {
                    template: fan_faces[i],
                    vertices: vec![ring[i], uvs.prev_cut[i], v],
                    donors: vec![None; 3],
                    blends: vec![uvs.next[i].clone(), uvs.prev[i].clone(), uvs.tip[i].clone()],
                });
            }
            Ok(())
        },
    )?;
    // The moved tips are selected afterwards, renumbered past any cut-off
    // corner that was dropped; a cut-off corner isn't (VE04).
    let tips = sel
        .iter()
        .filter_map(|&v| survivors.binary_search(&(v as usize)).ok())
        .map(|i| i as u32)
        .collect();
    Ok((out, tips))
}

/// MV-T06 Tear: each selected point that lies on a selected polygon and is
/// also used by an unselected one is torn off. The selected polygons'
/// corners at that point get one new point, moved by `offset`; the other
/// polygons keep the original (Jason's GUI test, 2026-10-31).
pub fn vertex_tear_geometry(
    mesh: &Mesh,
    selected: &[u32],
    polygons: &[u32],
    offset: DVec3,
) -> Result<(Mesh, Vec<u32>)> {
    let sel = ordered(mesh, selected, "Tear")?;
    if !offset.is_finite() {
        return Err("Tear offset must be finite".into());
    }
    let mut faces = source_faces(mesh, "Tear")?;
    let torn_faces = |faces: &[OutFace], v: u32| -> Option<Vec<usize>> {
        let incident: Vec<usize> = (0..faces.len())
            .filter(|&f| faces[f].vertices.contains(&v))
            .collect();
        let chosen: Vec<usize> = incident
            .iter()
            .copied()
            .filter(|&f| polygons.contains(&(faces[f].template as u32)))
            .collect();
        (!chosen.is_empty() && chosen.len() < incident.len()).then_some(chosen)
    };
    if !sel.iter().any(|&v| torn_faces(&faces, v).is_some()) {
        return Err("Tear needs a selected polygon and a selected point it shares with an unselected polygon".into());
    }
    if offset == DVec3::ZERO {
        return Ok((mesh.clone(), sel));
    }
    let mut vertices = mesh.vertices.clone();
    let mut copies = Vec::new();
    let mut torn = Vec::new();
    for &v in &sel {
        let Some(chosen) = torn_faces(&faces, v) else {
            continue;
        };
        let id = u32::try_from(vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
        vertices.push(mesh.vertices[v as usize] + offset);
        copies.push((id, v));
        torn.push(id);
        for f in chosen {
            for x in &mut faces[f].vertices {
                if *x == v {
                    *x = id;
                }
            }
        }
    }
    let out = finish(rebuild(mesh, vertices, faces, &copies, "Tear")?, "Tear")?;
    Ok((out, torn))
}

/// MV-T07 Slide: each selected point moves along its incident edge best
/// aligned with `direction` (or against it for negative `amount`), by
/// `amount` (−1..1) of that edge's length. Topology is unchanged.
pub fn vertex_slide_geometry(
    mesh: &Mesh,
    selected: &[u32],
    direction: DVec3,
    amount: f64,
) -> Result<Mesh> {
    let options = SlideOptions {
        by_distance: false,
        merge: false,
    };
    vertex_slide_with(mesh, selected, direction, amount, options).map(|(m, _)| m)
}

/// Vertex Slide options built so far (VTO-T07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlideOptions {
    /// Interpolation Distance: `amount` is a length along the edge instead
    /// of a share of it (Percent).
    pub by_distance: bool,
    /// Merge Vertices: a point that reaches its neighbour merges with it,
    /// as Edge Slide's points do (ES-04).
    pub merge: bool,
}

/// MV-T07 Slide with options. Percent runs −1..1 (±100 %, VTO-T07); a
/// Distance may not carry a point past its neighbour (provisional: the
/// Distance range wasn't recorded). Returns the mesh and the selection
/// after, renumbered when points merge.
pub fn vertex_slide_with(
    mesh: &Mesh,
    selected: &[u32],
    direction: DVec3,
    amount: f64,
    options: SlideOptions,
) -> Result<(Mesh, Vec<u32>)> {
    vertex_slide_extra(
        mesh,
        selected,
        direction,
        amount,
        options,
        &crate::SlideExtras::default(),
    )
}

/// Vertex Slide with the tool-audit options (Stop Negative Slide, Loop
/// Slide, Preserve Curvature, Slip UVs; see [`crate::SlideExtras`]).
pub fn vertex_slide_extra(
    mesh: &Mesh,
    selected: &[u32],
    direction: DVec3,
    amount: f64,
    options: SlideOptions,
    extras: &crate::SlideExtras,
) -> Result<(Mesh, Vec<u32>)> {
    let sel = ordered(mesh, selected, "Slide")?;
    extras.check()?;
    if !amount.is_finite()
        || amount.abs() > 1e6
        || (extras.loop_slide && !options.by_distance && !(-1. ..=1.).contains(&amount))
    {
        return Err("Slide Percent must be between -100% and 100%".into());
    }
    let mut out = mesh.clone();
    if amount == 0. || (extras.stop_negative && amount < 0.) {
        return Ok((out, sel));
    }
    let towards = if amount > 0. { direction } else { -direction };
    // Neighbours of the selected points only (Fast drags: no per-point scan
    // of every edge).
    let mut picked = vec![false; mesh.vertices.len()];
    for &v in &sel {
        picked[v as usize] = true;
    }
    let mut neighbours: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (a, b) in mesh.edges() {
        if picked[a as usize] {
            neighbours.entry(a).or_default().push(b);
        }
        if picked[b as usize] {
            neighbours.entry(b).or_default().push(a);
        }
    }
    let mut merges = Vec::new();
    let mut moves = Vec::new();
    for &v in &sel {
        let pv = mesh.vertices[v as usize];
        let rail = neighbours
            .get(&v)
            .into_iter()
            .flatten()
            .copied()
            .max_by(|&a, &b| {
                let score = |w: u32| {
                    (mesh.vertices[w as usize] - pv)
                        .normalize_or_zero()
                        .dot(towards)
                };
                score(a).total_cmp(&score(b))
            })
            .ok_or("Slide needs points on edges")?;
        let target = mesh.vertices[rail as usize];
        let mut fraction = if options.by_distance {
            amount.abs() / pv.distance(target).max(f64::MIN_POSITIVE)
        } else {
            amount.abs()
        };
        if extras.stop_negative {
            fraction = fraction.min(1.);
        } else if extras.loop_slide && fraction > 1. + 1e-9 {
            return Err("Slide Distance is longer than the edge to the next point".into());
        }
        if extras.loop_slide {
            fraction = fraction.min(1.);
        }
        out.vertices[v as usize] = extras.position(mesh, v, rail, fraction);
        moves.push((v, rail, fraction));
        if options.merge && (fraction - 1.).abs() <= 1e-9 && !picked[rail as usize] {
            merges.push((vec![v, rail], target));
        }
    }
    if extras.slip_uvs {
        crate::edge_slide::slip_uvs(&mut out, mesh, &moves);
    }
    if merges.is_empty() {
        return Ok((finish(out, "Slide")?, sel));
    }
    let (merged, remap) = weld(&out, &merges, "Slide")?;
    let mut after = Vec::new();
    for &v in &sel {
        if let Some(w) = remap[v as usize]
            && !after.contains(&w)
        {
            after.push(w);
        }
    }
    Ok((finish(merged, "Slide")?, after))
}

/// The polygons a Vertex Bevel or Vertex Extrude made: both keep the
/// source polygons in order and append their caps or triangles, so the new
/// ones are those past the source's polygon count.
pub fn new_vertex_tool_polygons(source: &Mesh, result: &Mesh) -> std::ops::Range<usize> {
    source.faces.len().min(result.faces.len())..result.faces.len()
}

/// Use Material: tag `faces` with `name` (an empty or unknown name is
/// assigned as typed, as Polygon Bevel does, BV-R16).
pub fn assign_material(mesh: &mut Mesh, faces: std::ops::Range<usize>, name: &str) {
    for f in faces {
        if let Some(face) = mesh.faces.get_mut(f) {
            face.material = name.to_string();
            face.material_label = Some(name.to_string());
        }
    }
}

/// Slide rail direction for the first selected point: toward its first
/// neighbour in edge order. Used for the default handle direction.
pub fn vertex_slide_direction(mesh: &Mesh, selected: &[u32]) -> Option<DVec3> {
    let &v = selected.first()?;
    let w = mesh
        .edges()
        .into_iter()
        .find_map(|(a, b)| match (a == v, b == v) {
            (true, _) => Some(b),
            (_, true) => Some(a),
            _ => None,
        })?;
    Some((mesh.vertices[w as usize] - mesh.vertices[v as usize]).normalize_or_zero())
}

/// MV-T04 Merge Tool preview: the Merge command at `distance`.
pub fn vertex_merge_geometry(
    mesh: &Mesh,
    selected: &[u32],
    distance: f64,
) -> Result<(Mesh, Vec<u32>)> {
    vertex_merge_with(mesh, selected, distance, MergeOptions::default())
}

/// The Merge Tool with its options (Keep 1-Vertex Polygons, Test Morph Map
/// Positions).
pub fn vertex_merge_with(
    mesh: &Mesh,
    selected: &[u32],
    distance: f64,
    options: MergeOptions,
) -> Result<(Mesh, Vec<u32>)> {
    vertex_command_geometry(mesh, selected, VertexCommand::Merge { distance, options })
}

/// MV-C16 Sync Index Order: renumber `target`'s points so that each point
/// gets the index of its corresponding point in `source`. Correspondence
/// spreads across shared edges from the first polygon pair; the second pair
/// fixes the rotation. Both meshes must have the same connected topology.
/// Returns the renumbered target and the number of points that moved.
pub fn sync_index_order(
    source: &Mesh,
    source_faces: [u32; 2],
    target: &Mesh,
    target_faces: [u32; 2],
) -> Result<(Mesh, usize)> {
    source.validate()?;
    target.validate()?;
    if source.vertices.len() != target.vertices.len() || source.faces.len() != target.faces.len() {
        return Err(
            "Sync Index Order needs two meshes with the same point and polygon counts".into(),
        );
    }
    if source.has_polylines() || target.has_polylines() {
        return Err("Sync Index Order supports surface polygons only".into());
    }
    for (mesh, faces) in [(source, source_faces), (target, target_faces)] {
        if faces.iter().any(|&f| f as usize >= mesh.faces.len()) || faces[0] == faces[1] {
            return Err("Select two different polygons on each mesh".into());
        }
    }
    let directed = |mesh: &Mesh| -> BTreeMap<(u32, u32), usize> {
        let mut m = BTreeMap::new();
        for (fi, f) in mesh.faces.iter().enumerate() {
            let n = f.vertices.len();
            for i in 0..n {
                m.insert((f.vertices[i], f.vertices[(i + 1) % n]), fi);
            }
        }
        m
    };
    let (sd, td) = (directed(source), directed(target));
    let n0 = source.faces[source_faces[0] as usize].vertices.len();
    if target.faces[target_faces[0] as usize].vertices.len() != n0 {
        return Err("The selected polygons do not match".into());
    }
    let attempt = |rotation: usize| -> Option<Vec<Option<u32>>> {
        let mut map: Vec<Option<u32>> = vec![None; source.vertices.len()];
        let mut used = vec![false; target.vertices.len()];
        let mut face_map: Vec<Option<usize>> = vec![None; source.faces.len()];
        let mut queue = vec![(source_faces[0] as usize, target_faces[0] as usize, rotation)];
        face_map[source_faces[0] as usize] = Some(target_faces[0] as usize);
        while let Some((sf, tf, r)) = queue.pop() {
            let (s, t) = (&source.faces[sf].vertices, &target.faces[tf].vertices);
            if s.len() != t.len() {
                return None;
            }
            let n = s.len();
            for i in 0..n {
                let (sv, tv) = (s[i], t[(i + r) % n]);
                match map[sv as usize] {
                    Some(x) if x != tv => return None,
                    Some(_) => {}
                    None => {
                        if used[tv as usize] {
                            return None;
                        }
                        used[tv as usize] = true;
                        map[sv as usize] = Some(tv);
                    }
                }
            }
            for i in 0..n {
                let (sa, sb) = (s[i], s[(i + 1) % n]);
                let (ta, tb) = (t[(i + r) % n], t[(i + 1 + r) % n]);
                match (sd.get(&(sb, sa)), td.get(&(tb, ta))) {
                    (None, None) => {}
                    (Some(&snb), Some(&tnb)) => match face_map[snb] {
                        Some(x) if x != tnb => return None,
                        Some(_) => {}
                        None => {
                            let (sv, tv) =
                                (&source.faces[snb].vertices, &target.faces[tnb].vertices);
                            if sv.len() != tv.len() {
                                return None;
                            }
                            let j = sv.iter().position(|&x| x == sb)?;
                            let k = tv.iter().position(|&x| x == tb)?;
                            face_map[snb] = Some(tnb);
                            queue.push((snb, tnb, (k + tv.len() - j) % tv.len()));
                        }
                    },
                    _ => return None,
                }
            }
        }
        (face_map[source_faces[1] as usize] == Some(target_faces[1] as usize)
            && face_map.iter().all(Option::is_some))
        .then_some(map)
    };
    let map = (0..n0)
        .find_map(attempt)
        .ok_or("The two meshes do not have matching topology from the selected polygons")?;
    let order: Vec<usize> = map
        .iter()
        .map(|t| t.map(|t| t as usize))
        .collect::<Option<Vec<usize>>>()
        .ok_or("Sync Index Order does not support floating points")?;
    let moved = order.iter().enumerate().filter(|(i, t)| i != *t).count();
    Ok((crate::mesh_cleanup::remap_points(target, &order)?, moved))
}

impl crate::Editor {
    /// Shared preflight for Vertex panel edits: idle editor, a visible
    /// active mesh in Vertex mode.
    pub(crate) fn vertex_target(&self, what: &str) -> Result<u64> {
        let item = self.edge_insert_target(what)?;
        if self.scene.selection.mode != crate::Mode::Vertex {
            return Err(format!("{what} works in Vertex mode"));
        }
        Ok(item)
    }

    /// Publish `mesh` for the active item as one edit labeled `label`, with
    /// `points` selected. Remembered edge/polygon selections are kept only
    /// while the polygons are unchanged.
    pub(crate) fn publish_vertex_edit(
        &mut self,
        item: u64,
        mesh: Mesh,
        label: &str,
        points: Vec<u32>,
    ) -> Result<()> {
        let before = self.scene.clone();
        let ids_before = self.identities.current.clone();
        let target = self.scene.active_mut().ok_or("Select a mesh")?;
        // A closed line opened in place keeps its points but loses a span.
        let same_polygons = target.mesh.faces == mesh.faces
            && target.mesh.attributes.representations == mesh.attributes.representations;
        target.mesh = mesh.into();
        let selection = &mut self.scene.selection;
        if !same_polygons {
            selection.edges.clear();
            selection.faces.clear();
        }
        let mut unique = Vec::with_capacity(points.len());
        for v in points {
            if !unique.contains(&v) {
                unique.push(v);
            }
        }
        selection.vertices = unique;
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.invalidate_mesh(&self.scene, item))
        {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }

    /// Run a Vertex command row on the selected points as one undoable edit.
    pub(crate) fn vertex_command(&mut self, command: VertexCommand) -> Result<()> {
        let item = self.vertex_target(command.label())?;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let selected = &self.scene.selection.vertices;
        let (out, selection) = match command {
            // With Symmetry, each side's points join (SYM-JOIN-Q1).
            VertexCommand::Join { average, options } => {
                let symmetry = self.active_symmetry();
                vertex_join_symmetric_geometry(mesh, selected, symmetry, average, options)?
            }
            _ => vertex_command_geometry(mesh, selected, command)?,
        };
        if out == **mesh {
            return Ok(());
        }
        // CMD-V-02/03/05, CMD-B3-02: Join, Collapse and Split leave no points
        // selected.
        let selection = match command {
            VertexCommand::Join { .. } | VertexCommand::Collapse | VertexCommand::Split => {
                Vec::new()
            }
            _ => selection,
        };
        self.publish_vertex_edit(item, out, command.label(), selection)
    }

    /// MV-C01 New Vertex at a world position; the new point is selected.
    pub(crate) fn new_vertex(&mut self, world: DVec3) -> Result<()> {
        let item = self.edge_insert_target("New Vertex")?;
        let active = self.scene.active().ok_or("Select a mesh")?;
        let local = self
            .scene
            .world_matrix(active.id)?
            .inverse()
            .transform_point3(world);
        let (out, ids) = add_points(&active.mesh, &[local])?;
        self.publish_vertex_edit(item, out, "New Vertex", ids)
    }

    /// MV-T05 Weld the selected points (or `sources`) onto `target`.
    pub(crate) fn weld_vertices(&mut self, sources: Vec<u32>, target: u32) -> Result<()> {
        let item = self.vertex_target("Weld")?;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, selection) = weld_points(mesh, &sources, target)?;
        self.publish_vertex_edit(item, out, "Weld", selection)
    }

    /// WD-01..05: one Weld drag of point `source` as one undoable edit. A
    /// `WeldDrop::Position` is a world position here (like New Vertex). The
    /// point selection is kept (the merged-away point drops out of it).
    pub(crate) fn weld_drag(&mut self, source: u32, drop: WeldDrop) -> Result<()> {
        let item = self.vertex_target("Weld")?;
        let active = self.scene.active().ok_or("Select a mesh")?;
        let drop = match drop {
            WeldDrop::Position(world) => WeldDrop::Position(
                self.scene
                    .world_matrix(active.id)?
                    .inverse()
                    .transform_point3(world),
            ),
            point => point,
        };
        let (out, remap) = weld_drag_geometry(&active.mesh, source, drop)?;
        let selection = self
            .scene
            .selection
            .vertices
            .iter()
            .filter_map(|&v| remap.get(v as usize).copied().flatten())
            .collect();
        self.publish_vertex_edit(item, out, "Weld", selection)
    }

    /// MV-C02 Make Polygon from the ordered point selection (Vertex mode).
    /// Make Polygon from the ordered selected points (or a closed edge loop's
    /// points). The new polygon is appended and added to the polygon
    /// selection; in Vertex mode the point selection is cleared (rule 9).
    pub(crate) fn make_polygon_from_vertices_as(
        &mut self,
        points: Vec<u32>,
        kind: MakeType,
        close: bool,
    ) -> Result<()> {
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh for Make Polygon")?
            .id;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, made) = match self.symmetric_picks(item, &points) {
            // N04 (provisional): with Symmetry on, each click also selected
            // the mirror point, so the selection alternates pick, mirror.
            // Make builds the picked polygon and its mirror image; picks
            // too few for a polygon of their own make one polygon across
            // the plane (picks, then their mirrors back).
            Some((picks, mirrors)) if picks.len() >= 3 && !mirrors.is_empty() => {
                let (out, first) = make_polygon_typed(mesh, &picks, kind, close)?;
                // Reversed: a mirror image turns the winding over, and this
                // keeps the two facing the same way across the plane (a
                // bordered gap re-winds to its neighbours anyway).
                let mirrored: Vec<u32> = picks
                    .iter()
                    .rev()
                    .map(|v| mirrors.iter().find(|(p, _)| p == v).map_or(*v, |&(_, m)| m))
                    .collect();
                let (out, second) = make_polygon_typed(&out, &mirrored, kind, close)?;
                (out, vec![first, second])
            }
            Some((picks, mirrors)) if !mirrors.is_empty() => {
                let mut order = picks;
                order.extend(mirrors.iter().rev().map(|&(_, m)| m));
                let (out, face) = make_polygon_typed(mesh, &order, kind, close)?;
                (out, vec![face])
            }
            _ => {
                let (out, face) = make_polygon_typed(mesh, &points, kind, close)?;
                (out, vec![face])
            }
        };
        let mut faces = self.scene.selection.faces.clone();
        let edges = self.scene.selection.edges.clone();
        let vertex_mode = self.scene.selection.mode == crate::Mode::Vertex;
        self.publish_vertex_edit(item, out, "Make Polygon", Vec::new())?;
        faces.extend(made);
        let selection = &mut self.scene.selection;
        selection.faces = faces;
        if !vertex_mode {
            selection.edges = edges;
        }
        Ok(())
    }

    /// N04: with Symmetry on, `points` read as clicks that each also
    /// selected their mirror point right after (SYM-B2-10): the picks in
    /// order (points on the plane are their own mirror and come once) and
    /// each off-plane pick's mirror. `None` when Symmetry is off or the
    /// selection doesn't alternate that way.
    fn symmetric_picks(&self, item: u64, points: &[u32]) -> Option<SymmetricPicks> {
        let symmetry = self.active_symmetry();
        symmetry.axis?;
        let mirror = |v: u32| match self.scene.symmetric_counterpart(
            item,
            crate::Component::Vertex(v),
            symmetry,
        ) {
            Some(crate::Component::Vertex(m)) => Some(m),
            _ => None,
        };
        let (mut picks, mut mirrors) = (Vec::new(), Vec::new());
        let mut i = 0;
        while i < points.len() {
            let v = points[i];
            let m = mirror(v)?;
            picks.push(v);
            if m != v {
                if points.get(i + 1) != Some(&m) {
                    return None;
                }
                mirrors.push((v, m));
                i += 1;
            }
            i += 1;
        }
        Some((picks, mirrors))
    }

    /// MV-C16 Sync Index Order: `source` (item, two polygons) drives the
    /// active item's two selected polygons (Polygon mode).
    pub(crate) fn sync_vertex_order(
        &mut self,
        source_item: u64,
        source_faces: [u32; 2],
    ) -> Result<()> {
        let item = self.edge_insert_target("Sync Index Order")?;
        let faces = &self.scene.selection.faces;
        let [a, b] = faces[..] else {
            return Err("Select the same two polygons on the target mesh".into());
        };
        if source_item == item {
            return Err("Select the matching polygons on a second mesh".into());
        }
        let source = &self
            .scene
            .item(source_item)
            .ok_or("The captured source mesh no longer exists")?
            .mesh;
        let target = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, moved) = sync_index_order(source, source_faces, target, [a, b])?;
        if moved == 0 {
            return Ok(());
        }
        self.publish_vertex_edit(item, out, "Sync Index Order", Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_matches_the_guide_example() {
        // Selected 3, 1, 0, 2 → they take slots 0, 1, 2, 3 in that order.
        let mesh = Mesh {
            vertices: (0..5).map(|i| DVec3::new(i as f64, 0., 0.)).collect(),
            ..Default::default()
        };
        let (out, sel) = reorder(&mesh, &[3, 1, 0, 2], ReorderMode::Swap).unwrap();
        assert_eq!(sel, vec![0, 1, 2, 3]);
        assert_eq!(out.vertices[0].x, 3.);
        assert_eq!(out.vertices[4].x, 4.);
        let (out, _) = reorder(&mesh, &[3, 1], ReorderMode::Bottom).unwrap();
        let xs: Vec<f64> = out.vertices.iter().map(|p| p.x).collect();
        assert_eq!(xs, vec![0., 2., 4., 3., 1.]);
    }
}
