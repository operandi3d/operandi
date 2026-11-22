//! Symmetry phase 2 helpers (symmetry-2026-11-13 README items 12–15): the
//! tools here do their work on the primary side as entered, then once more
//! on the other side with everything that has a direction mirrored
//! (SYM3-11/12). These helpers split a selection by side, mirror world-space
//! settings, and find the untouched other side again in the first pass's
//! result by position (so a pass that renumbers points or polygons doesn't
//! lose it).
use crate::Mesh;
use crate::symmetry::{Side, Symmetry, match_tolerance};
use glam::{DMat3, DMat4, DVec3};
use std::collections::HashMap;

/// A grid of points for "which point is at this position" lookups.
struct Grid {
    cell: f64,
    cells: HashMap<(i64, i64, i64), Vec<u32>>,
}
impl Grid {
    fn new(points: impl Iterator<Item = DVec3>, tolerance: f64) -> Self {
        let cell = (tolerance * 4.).max(1e-12);
        let mut cells: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
        for (i, p) in points.enumerate() {
            cells.entry(Self::key(cell, p)).or_default().push(i as u32);
        }
        Self { cell, cells }
    }
    fn key(cell: f64, p: DVec3) -> (i64, i64, i64) {
        let q = (p / cell).floor();
        (q.x as i64, q.y as i64, q.z as i64)
    }
    /// The nearest entry to `target` within `tolerance` (lowest index on a
    /// tie), `at(i)` giving entry `i`'s position.
    fn nearest(&self, target: DVec3, tolerance: f64, at: impl Fn(u32) -> DVec3) -> Option<u32> {
        let (x, y, z) = Self::key(self.cell, target);
        let mut best: Option<(f64, u32)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    for &j in self
                        .cells
                        .get(&(x + dx, y + dy, z + dz))
                        .into_iter()
                        .flatten()
                    {
                        let d = at(j).distance(target);
                        if d <= tolerance && best.is_none_or(|(bd, bj)| d < bd || d == bd && j < bj)
                        {
                            best = Some((d, j));
                        }
                    }
                }
            }
        }
        best.map(|(_, j)| j)
    }
}

/// The points of `mesh` at `positions`, each within `tolerance`.
pub fn points_at(mesh: &Mesh, positions: &[DVec3], tolerance: f64) -> Vec<Option<u32>> {
    let grid = Grid::new(mesh.vertices.iter().copied(), tolerance);
    positions
        .iter()
        .map(|&p| grid.nearest(p, tolerance, |j| mesh.vertices[j as usize]))
        .collect()
}

/// The polygons of `mesh` whose centres are at `centres` and that have
/// `sizes` corners, each within `tolerance`.
pub fn faces_at(mesh: &Mesh, centres: &[(DVec3, usize)], tolerance: f64) -> Vec<Option<u32>> {
    let at: Vec<DVec3> = (0..mesh.faces.len()).map(|f| mesh.center(f)).collect();
    let grid = Grid::new(at.iter().copied(), tolerance);
    centres
        .iter()
        .map(|&(c, n)| {
            grid.nearest(c, tolerance, |j| {
                if mesh.faces[j as usize].vertices.len() == n {
                    at[j as usize]
                } else {
                    DVec3::splat(f64::INFINITY)
                }
            })
        })
        .collect()
}

/// A selection split by side: what the primary pass edits (the primary side
/// and anything on or across the plane) and the other side.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Split<T> {
    pub primary: Vec<T>,
    pub other: Vec<T>,
}
impl<T> Split<T> {
    /// Whether the other pass has anything to do.
    pub fn is_two_sided(&self) -> bool {
        !self.other.is_empty()
    }
}

/// Split `points` of `mesh` by side.
pub fn split_points(mesh: &Mesh, symmetry: Symmetry, points: &[u32], primary: Side) -> Split<u32> {
    let tolerance = match_tolerance(mesh);
    let other = crate::pivot_service::other(primary);
    let mut out = Split::default();
    for &v in points {
        let side = mesh
            .vertices
            .get(v as usize)
            .and_then(|&p| symmetry.side(p, tolerance));
        if side == Some(other) {
            out.other.push(v);
        } else {
            out.primary.push(v);
        }
    }
    out
}

/// Split `faces` of `mesh` by side: a polygon is on the other side when all
/// its points are (one that touches the plane or crosses it goes with the
/// primary pass).
pub fn split_faces(mesh: &Mesh, symmetry: Symmetry, faces: &[u32], primary: Side) -> Split<u32> {
    let tolerance = match_tolerance(mesh);
    let other = crate::pivot_service::other(primary);
    let mut out = Split::default();
    for &f in faces {
        let all_other = mesh.faces.get(f as usize).is_some_and(|face| {
            face.vertices.iter().all(|&v| {
                mesh.vertices
                    .get(v as usize)
                    .and_then(|&p| symmetry.side(p, tolerance))
                    == Some(other)
            })
        });
        if all_other {
            out.other.push(f);
        } else {
            out.primary.push(f);
        }
    }
    out
}

/// The side that gets the edit as entered: the first pick's side, else
/// Negative (SYM3-11's numeric-entry rule).
pub fn primary_or_negative(side: Option<Side>) -> Side {
    side.unwrap_or(Side::Negative)
}

/// The plane as a reflection of world space for an item at `world`, so
/// world-space settings (drawn paths, cut planes, copy transforms) can be
/// mirrored. `None` when Off or the transform can't be inverted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldMirror {
    pub matrix: DMat4,
}
impl WorldMirror {
    pub fn new(symmetry: Symmetry, world: DMat4) -> Option<Self> {
        let n = symmetry.normal()?;
        if world.determinant().abs() < 1e-300 {
            return None;
        }
        let reflect = DMat3::IDENTITY - DMat3::from_cols(n * n.x, n * n.y, n * n.z) * 2.;
        let local = DMat4::from_mat3_translation(reflect, n * (2. * symmetry.offset));
        Some(Self {
            matrix: world * local * world.inverse(),
        })
    }
    pub fn point(&self, p: DVec3) -> DVec3 {
        self.matrix.transform_point3(p)
    }
    pub fn vector(&self, d: DVec3) -> DVec3 {
        self.matrix.transform_vector3(d)
    }
    /// A world transform `t` as seen from the mirror side: mirror, `t`,
    /// mirror back. Applied to a mirrored source it gives the mirror image of
    /// `t` applied to the source.
    pub fn conjugate(&self, t: DMat4) -> DMat4 {
        self.matrix * t * self.matrix
    }
}

/// Whether two point lists are the same set within `tolerance` (a mirrored
/// path or cut that lands on the original is done only once, SYM3-21).
pub fn same_points(a: &[DVec3], b: &[DVec3], tolerance: f64) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|p| b.iter().any(|q| p.distance(*q) <= tolerance))
}

/// The world-space tolerance for comparing mirrored settings of `mesh` at
/// `world`.
pub fn world_tolerance(mesh: &Mesh, world: DMat4) -> f64 {
    let scale = crate::scene::uniform_scale(world);
    match_tolerance(mesh) * scale
}

/// `scene` with its selection cut down to the primary side's points (in
/// Vertex mode), for fitting a tool's handles to the side the user works on
/// (Bend's spine). `None` with Symmetry off or a one-sided selection.
pub fn primary_side_scene(scene: &crate::Scene, symmetry: Symmetry) -> Option<crate::Scene> {
    symmetry.axis?;
    let item = scene.active()?;
    let points: Vec<u32> = crate::transform_vertices(scene).ok()?.into_iter().collect();
    let primary = primary_or_negative(scene.symmetry_primary_side(symmetry));
    let split = split_points(&item.mesh, symmetry, &points, primary);
    if !split.is_two_sided() || split.primary.is_empty() {
        return None;
    }
    let mut out = scene.clone();
    out.selection.mode = crate::Mode::Vertex;
    out.selection.vertices = split.primary;
    out.selection.edges.clear();
    out.selection.faces.clear();
    Some(out)
}

/// Selected elements by position, so they can be found again after a pass
/// renumbers the mesh.
#[derive(Clone, Debug, Default)]
struct Picked {
    vertices: Vec<DVec3>,
    edges: Vec<(DVec3, DVec3)>,
    faces: Vec<(DVec3, usize)>,
}
impl Picked {
    fn of(mesh: &Mesh, vertices: &[u32], edges: &[(u32, u32)], faces: &[u32]) -> Self {
        let at = |v: u32| mesh.vertices.get(v as usize).copied();
        Self {
            vertices: vertices.iter().filter_map(|&v| at(v)).collect(),
            edges: edges
                .iter()
                .filter_map(|&(a, b)| Some((at(a)?, at(b)?)))
                .collect(),
            faces: faces
                .iter()
                .filter(|&&f| (f as usize) < mesh.faces.len())
                .map(|&f| {
                    (
                        mesh.center(f as usize),
                        mesh.faces[f as usize].vertices.len(),
                    )
                })
                .collect(),
        }
    }
    fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.edges.is_empty() && self.faces.is_empty()
    }
    /// The elements found in `mesh` (missing ones are dropped).
    fn place(&self, mesh: &Mesh) -> (Vec<u32>, Vec<(u32, u32)>, Vec<u32>) {
        let tolerance = match_tolerance(mesh);
        let vertices = points_at(mesh, &self.vertices, tolerance)
            .into_iter()
            .flatten()
            .collect();
        let ends: Vec<DVec3> = self.edges.iter().flat_map(|&(a, b)| [a, b]).collect();
        let found = points_at(mesh, &ends, tolerance);
        let counts = mesh.edge_counts();
        let edges = found
            .chunks(2)
            .filter_map(|pair| match *pair {
                [Some(a), Some(b)] if counts.contains_key(&(a.min(b), a.max(b))) => Some((a, b)),
                _ => None,
            })
            .collect();
        let faces = faces_at(mesh, &self.faces, tolerance)
            .into_iter()
            .flatten()
            .collect();
        (vertices, edges, faces)
    }
}

/// Runs a selection-driven tool `pass` on the active mesh of `scene` with
/// Symmetry (the README's phase 2 tools whose settings carry a direction:
/// Sketch Extrude's path, the Duplicate tools' placements, Lathe's axis).
/// The first pass gets the primary side of the selection (with anything on
/// or across the plane) and `mirrored = false`; the second gets the other
/// side, found again by position, and `mirrored = true`, so the tool can
/// use mirrored settings (SYM3-11/12). The selection afterwards is both
/// passes' selections. With Symmetry off or a one-sided selection there is
/// one plain pass. Returns whether there were two.
pub fn two_pass(
    scene: &mut crate::Scene,
    symmetry: Symmetry,
    mut pass: impl FnMut(&mut crate::Scene, bool) -> crate::Result<()>,
) -> crate::Result<bool> {
    let Some(item) = scene.active().filter(|_| symmetry.axis.is_some()) else {
        pass(scene, false)?;
        return Ok(false);
    };
    let mesh = &item.mesh;
    let primary = primary_or_negative(scene.symmetry_primary_side(symmetry));
    let other = crate::pivot_service::other(primary);
    let s = &scene.selection;
    let points = split_points(mesh, symmetry, &s.vertices, primary);
    let faces = split_faces(mesh, symmetry, &s.faces, primary);
    let tolerance = match_tolerance(mesh);
    let on_other = |v: u32| {
        mesh.vertices
            .get(v as usize)
            .and_then(|&p| symmetry.side(p, tolerance))
            == Some(other)
    };
    type Edges = Vec<(u32, u32)>;
    let (edges_other, edges_primary): (Edges, Edges) = s
        .edges
        .iter()
        .partition(|&&(a, b)| on_other(a) && on_other(b));
    let later = Picked::of(mesh, &points.other, &edges_other, &faces.other);
    let first_empty =
        points.primary.is_empty() && edges_primary.is_empty() && faces.primary.is_empty();
    if later.is_empty() || first_empty {
        pass(scene, false)?;
        return Ok(false);
    }
    scene.selection.vertices = points.primary;
    scene.selection.edges = edges_primary;
    scene.selection.faces = faces.primary;
    pass(scene, false)?;
    let mesh = &scene.active().ok_or("Select a mesh")?.mesh;
    let s = &scene.selection;
    let done = Picked::of(mesh, &s.vertices, &s.edges, &s.faces);
    let (vertices, edges, faces) = later.place(mesh);
    scene.selection.vertices = vertices;
    scene.selection.edges = edges;
    scene.selection.faces = faces;
    pass(scene, true)?;
    let mesh = &scene.active().ok_or("Select a mesh")?.mesh;
    let (vertices, edges, faces) = done.place(mesh);
    let s = &mut scene.selection;
    let join = |first: Vec<u32>, second: &[u32]| {
        let mut out = first;
        for &x in second {
            if !out.contains(&x) {
                out.push(x);
            }
        }
        out
    };
    s.vertices = join(vertices, &s.vertices);
    s.faces = join(faces, &s.faces);
    let mut all = edges;
    for &(a, b) in &s.edges {
        if !all
            .iter()
            .any(|&(x, y)| (x.min(y), x.max(y)) == (a.min(b), a.max(b)))
        {
            all.push((a, b));
        }
    }
    s.edges = all;
    Ok(true)
}

/// Poly Haul's picked element on the other side (SYM3-10): `target` names
/// an element of `base`; its mirror partner is found in `after` (the result
/// of hauling `target`) by position. `None` with Symmetry off, when the
/// element is its own mirror (on or across the plane), or when the partner
/// isn't there unchanged.
pub fn mirrored_haul_target(
    base: &Mesh,
    after: &Mesh,
    target: crate::HaulTarget,
    symmetry: Symmetry,
) -> Option<crate::HaulTarget> {
    use crate::HaulTarget;
    symmetry.axis?;
    let tolerance = match_tolerance(base);
    let at = |v: u32| base.vertices.get(v as usize).copied();
    let mirrored = |v: u32| Some(symmetry.mirror(at(v)?));
    // The partner in `base` must differ from the element itself.
    let own = |points: &[DVec3]| -> Option<Vec<u32>> {
        points_at(base, points, tolerance).into_iter().collect()
    };
    let find = |points: &[DVec3]| -> Option<Vec<u32>> {
        points_at(after, points, tolerance).into_iter().collect()
    };
    let polygon = |f: u32| -> Option<(u32, u32)> {
        let face = base.faces.get(f as usize)?;
        let c = symmetry.mirror(base.center(f as usize));
        let n = face.vertices.len();
        let mine = faces_at(base, &[(c, n)], tolerance)[0]?;
        let theirs = faces_at(after, &[(c, n)], tolerance)[0]?;
        Some((mine, theirs))
    };
    match target {
        HaulTarget::None => None,
        HaulTarget::Edge(a, b) => {
            let ends = [mirrored(a)?, mirrored(b)?];
            let mine = own(&ends)?;
            let key = |x: u32, y: u32| (x.min(y), x.max(y));
            if key(mine[0], mine[1]) == key(a, b) {
                return None;
            }
            let found = find(&ends)?;
            after
                .edge_counts()
                .contains_key(&key(found[0], found[1]))
                .then_some(HaulTarget::Edge(found[0], found[1]))
        }
        HaulTarget::Point { point, polygon: f } => {
            let end = [mirrored(point)?];
            if own(&end)?[0] == point {
                return None;
            }
            let found = find(&end)?[0];
            let (_, theirs) = polygon(f)?;
            Some(HaulTarget::Point {
                point: found,
                polygon: theirs,
            })
        }
        HaulTarget::Polygon(f) => {
            let (mine, theirs) = polygon(f)?;
            (mine != f).then_some(HaulTarget::Polygon(theirs))
        }
    }
}

/// A selection sorted by side for one-shot commands that pair elements
/// (Join): what lies on the positive side, the negative side, and on or
/// across the plane, each in selection order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BySide<T> {
    pub positive: Vec<T>,
    pub negative: Vec<T>,
    pub centre: Vec<T>,
}

/// The side a group of points (an edge's ends, say) sits on: a side when
/// some point is on it and none on the other side (points on the plane go
/// with either), else `Plane` (all on the plane, or reaching both sides).
pub fn side_of(mesh: &Mesh, symmetry: Symmetry, points: &[u32], tolerance: f64) -> Side {
    let mut positive = false;
    let mut negative = false;
    for &v in points {
        match mesh
            .vertices
            .get(v as usize)
            .and_then(|&p| symmetry.side(p, tolerance))
        {
            Some(Side::Positive) => positive = true,
            Some(Side::Negative) => negative = true,
            _ => {}
        }
    }
    match (positive, negative) {
        (true, false) => Side::Positive,
        (false, true) => Side::Negative,
        _ => Side::Plane,
    }
}

/// Sort `items` of `mesh` by side, `points` giving each item's points.
pub fn by_side<T: Clone>(
    mesh: &Mesh,
    symmetry: Symmetry,
    items: &[T],
    points: impl Fn(&T) -> Vec<u32>,
) -> BySide<T> {
    let tolerance = match_tolerance(mesh);
    let mut out = BySide {
        positive: Vec::new(),
        negative: Vec::new(),
        centre: Vec::new(),
    };
    for item in items {
        match side_of(mesh, symmetry, &points(item), tolerance) {
            Side::Positive => out.positive.push(item.clone()),
            Side::Negative => out.negative.push(item.clone()),
            Side::Plane => out.centre.push(item.clone()),
        }
    }
    out
}
