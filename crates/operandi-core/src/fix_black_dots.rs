//! Fix Black Dots (Edit tab, Commands). Spec: missing-tools-g1
//! G1-SMALL-COMMANDS v1, MT2-01..05, cases FB-A..G.
//!
//! A "black dot" is two polygons sharing a run of two or more edges whose
//! inner points no other polygon uses: the inner points pinch the
//! subdivided surface. Each such pair becomes one polygon (its outline with
//! the shared run dropped). The inner points are kept, unused (Q-FB-1,
//! provisional), and a bent pair still merges into one non-flat polygon
//! (Q-FB-2, provisional). Runs on the whole active mesh in any mode; the
//! selection doesn't limit it (MT2-01).
use crate::edge_commands::{OutFace, rebuild};
use crate::{Editor, Mesh, Mode, Result};
use std::collections::{BTreeMap, BTreeSet};

/// Shown when nothing is found (MT2-04); no change and no undo step.
pub const NO_BLACK_DOTS: &str = "No black dots found.";

/// One pair to merge: polygons `f` and `g`, and the shared run in `f`'s
/// corner order from `start` through the inner points to `end`.
struct Pinch {
    f: usize,
    g: usize,
    /// `f`'s corner index of the run's first point.
    f_start: usize,
    /// Number of shared sides in the run (at least two).
    sides: usize,
}

/// The merged mesh and the points of every fixed run (ends and inner
/// points, sorted), or `None` when there is nothing to fix.
pub fn fix_black_dots_geometry(mesh: &Mesh) -> Result<Option<(Mesh, Vec<u32>)>> {
    // One pass over the mesh as it is: a merge that leaves a new pinch
    // behind (a cube corner, FB-C) is not merged again in the same run.
    let pinches = find_pinches(mesh);
    if pinches.is_empty() {
        return Ok(None);
    }
    let (current, points) = merge(mesh, &pinches)?;
    if points.is_empty() {
        return Ok(None);
    }
    let chosen: BTreeSet<u32> = points.into_iter().collect();
    current.validate().map_err(|e| {
        format!("Fix Black Dots would leave an invalid mesh ({e}); nothing changed")
    })?;
    Ok(Some((current, chosen.into_iter().collect())))
}

/// Pairs to merge, each polygon in at most one pair.
fn find_pinches(mesh: &Mesh) -> Vec<Pinch> {
    let edge_faces = mesh.edge_faces();
    let point_faces = mesh.point_faces();
    // Polygon pairs sharing two or more edges.
    let mut shared: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for faces in edge_faces.values() {
        if let [a, b] = faces[..]
            && a != b
        {
            *shared.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    let mut used = BTreeSet::new();
    let mut out = Vec::new();
    for (&(f, g), &count) in &shared {
        if count < 2 || used.contains(&f) || used.contains(&g) {
            continue;
        }
        if let Some(p) = pinch(mesh, f, g, &point_faces) {
            used.insert(f);
            used.insert(g);
            out.push(p);
        }
    }
    out
}

/// The first shared run of `f` and `g` that pinches: two or more sides in
/// a row, `g` running it the opposite way, and inner points used only by
/// `f` and `g` (once each).
fn pinch(
    mesh: &Mesh,
    f: usize,
    g: usize,
    point_faces: &crate::topology_cache::PointFaces,
) -> Option<Pinch> {
    let fv = &mesh.faces[f].vertices;
    let gv = &mesh.faces[g].vertices;
    let n = fv.len();
    let g_sides: BTreeSet<(u32, u32)> = (0..gv.len())
        .map(|i| (gv[i], gv[(i + 1) % gv.len()]))
        .collect();
    // Side i of f (fv[i] -> fv[i+1]) is shared when g has it reversed.
    let side = |i: usize| g_sides.contains(&(fv[(i + 1) % n], fv[i % n]));
    if (0..n).all(side) {
        return None;
    }
    // Start each run just after a side that isn't shared.
    let first = (0..n).find(|&i| !side(i))?;
    let mut i = 1;
    while i <= n {
        let at = (first + i) % n;
        if !side(at) {
            i += 1;
            continue;
        }
        let mut sides = 0;
        while sides < n && side((at + sides) % n) {
            sides += 1;
        }
        let inner_ok = (1..sides).all(|k| {
            let v = fv[(at + k) % n];
            let users = point_faces.of(v);
            users.len() == 2 && users.contains(&(f as u32)) && users.contains(&(g as u32))
        });
        if sides >= 2 && inner_ok {
            return Some(Pinch {
                f,
                g,
                f_start: at,
                sides,
            });
        }
        i += sides;
    }
    None
}

/// Merge every pinch into one polygon: `f` from the run's end round to its
/// start, then `g`'s corners between the run's start and end.
fn merge(mesh: &Mesh, pinches: &[Pinch]) -> Result<(Mesh, Vec<u32>)> {
    let mut replaced: BTreeMap<usize, Option<OutFace>> = BTreeMap::new();
    let mut points = Vec::new();
    for p in pinches {
        let fv = &mesh.faces[p.f].vertices;
        let gv = &mesh.faces[p.g].vertices;
        let n = fv.len();
        let start = fv[p.f_start];
        let end_corner = (p.f_start + p.sides) % n;
        let end = fv[end_corner];
        let mut vertices = Vec::new();
        let mut donors = Vec::new();
        // f: end .. start (inclusive both), its unshared part.
        let mut c = end_corner;
        loop {
            vertices.push(fv[c]);
            donors.push(Some((p.f, c)));
            if fv[c] == start {
                break;
            }
            c = (c + 1) % n;
        }
        // g: strictly between start and end.
        let m = gv.len();
        let g_start = gv.iter().position(|&v| v == start).ok_or("Broken pinch")?;
        let mut c = (g_start + 1) % m;
        while gv[c] != end {
            vertices.push(gv[c]);
            donors.push(Some((p.g, c)));
            c = (c + 1) % m;
        }
        let distinct: BTreeSet<_> = vertices.iter().collect();
        if vertices.len() < 3 || distinct.len() != vertices.len() {
            // Not a clean outline (the pair wraps round itself): leave it.
            continue;
        }
        points.extend((0..=p.sides).map(|k| fv[(p.f_start + k) % n]));
        replaced.insert(
            p.f,
            Some(OutFace {
                template: p.f,
                vertices,
                donors,
                blends: Vec::new(),
            }),
        );
        replaced.insert(p.g, None);
    }
    let mut faces = Vec::with_capacity(mesh.faces.len());
    for (fi, f) in mesh.faces.iter().enumerate() {
        match replaced.remove(&fi) {
            Some(Some(merged)) => faces.push(merged),
            Some(None) => {}
            None => faces.push(OutFace::copy(mesh, fi, f.vertices.clone())),
        }
    }
    let out = rebuild(mesh, mesh.vertices.clone(), faces, &[], "Fix Black Dots")?;
    Ok((out, points))
}

impl Editor {
    /// Fix Black Dots (MT2): one undo step; afterwards Vertex mode with
    /// the fixed runs' points selected (MT2-03).
    pub(crate) fn fix_black_dots(&mut self) -> Result<()> {
        let item = self.edge_insert_target("Fix Black Dots")?;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let Some((out, points)) = fix_black_dots_geometry(mesh)? else {
            return Err(NO_BLACK_DOTS.into());
        };
        let before = self.scene.clone();
        let selection = &mut self.scene.selection;
        selection.mode = Mode::Vertex;
        selection.tag = None;
        selection.edges.clear();
        selection.vertices = points;
        self.publish_polygon_edit_from(item, out, "Fix Black Dots", Vec::new(), before)
    }
}
