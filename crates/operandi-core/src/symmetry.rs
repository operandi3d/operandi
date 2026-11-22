//! Symmetry (specifications/symmetry-2026-11-01): one shared setting that
//! mirrors transforms, adds counterparts to a selection, doubles primitives
//! and makes a mesh exactly symmetric. Tool context like the action center,
//! not scene data: it is not saved with the scene and not undoable.
use crate::{Mesh, Mode, Result, Scene};
use glam::DVec3;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Symmetry {
    /// `None` is Off; otherwise 0, 1, 2 for X, Y, Z, or 3 for Arbitrary
    /// (SYM-ST-01, SYM-B2-09).
    pub axis: Option<usize>,
    /// The plane is `n · p = offset` in the mesh item's own coordinates
    /// (SYM-ST-02, SYM-ST-03), `n` the axis or the Arbitrary vector.
    pub offset: f64,
    /// The Arbitrary axis vector (SYM-B2-09); used when `axis` is 3.
    pub vector: DVec3,
    /// Work Plane: the plane passes through the work plane centre instead of
    /// using Offset (SYM-B2-08).
    pub work_plane: bool,
    /// Topology: Select Symmetrical finds counterparts through connectivity
    /// (SYM-SEL-06, SYM-B2-07).
    pub topology: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Negative,
    Plane,
    Positive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymmetrizeSide {
    /// Copy the positive side onto the negative side.
    Positive,
    /// Copy the negative side onto the positive side.
    Negative,
}

impl Default for Symmetry {
    fn default() -> Self {
        Self::OFF
    }
}
impl Symmetry {
    pub const OFF: Self = Self {
        axis: None,
        offset: 0.,
        vector: DVec3::X,
        work_plane: false,
        topology: false,
    };
    pub fn x() -> Self {
        Self {
            axis: Some(0),
            ..Self::OFF
        }
    }
    pub fn label(self) -> &'static str {
        match self.axis {
            None => "Off",
            Some(0) => "X",
            Some(1) => "Y",
            Some(2) => "Z",
            _ => "Arbitrary",
        }
    }
    pub fn validate(self) -> Result<()> {
        if self.axis.is_some_and(|a| a > 3) {
            return Err("Symmetry axis must be X, Y, Z or Arbitrary".into());
        }
        if !self.offset.is_finite() || self.offset.abs() > 1e8 {
            return Err("Symmetry offset must be finite".into());
        }
        if self.axis == Some(3) && !(self.vector.is_finite() && self.vector.length() > 1e-12) {
            return Err("The Arbitrary axis needs a nonzero finite vector".into());
        }
        Ok(())
    }
    /// The plane's unit normal; `None` when Off.
    pub fn normal(self) -> Option<DVec3> {
        Some(match self.axis? {
            a @ 0..=2 => {
                let mut n = DVec3::ZERO;
                n[a] = 1.;
                n
            }
            _ => self.vector.normalize_or(DVec3::X),
        })
    }
    /// With Work Plane on, the same setting with its plane through `center`.
    pub fn through(self, center: DVec3) -> Self {
        match (self.work_plane, self.normal()) {
            (true, Some(n)) => Self {
                offset: n.dot(center),
                ..self
            },
            _ => self,
        }
    }
    /// Mirror of a local point across the plane; unchanged when Off.
    pub fn mirror(self, p: DVec3) -> DVec3 {
        match self.normal() {
            Some(n) => p - n * (2. * (n.dot(p) - self.offset)),
            None => p,
        }
    }
    /// Mirror of a direction (no offset).
    pub fn mirror_vector(self, d: DVec3) -> DVec3 {
        match self.normal() {
            Some(n) => d - n * (2. * n.dot(d)),
            None => d,
        }
    }
    /// SYM-ST-05. `tolerance` decides "on the plane"; points closer than the
    /// mesh's matching tolerance count as on it (provisional, see
    /// `match_tolerance`).
    pub fn side(self, p: DVec3, tolerance: f64) -> Option<Side> {
        let d = self.normal()?.dot(p) - self.offset;
        Some(if d.abs() <= tolerance {
            Side::Plane
        } else if d > 0. {
            Side::Positive
        } else {
            Side::Negative
        })
    }
    /// Snap a point onto the plane (SYM-TR-04: a point on the plane keeps
    /// only the components along the plane).
    pub fn onto_plane(self, p: DVec3) -> DVec3 {
        match self.normal() {
            Some(n) => p - n * (n.dot(p) - self.offset),
            None => p,
        }
    }

    /// SYM-PR-01/02: the generated primitive followed by its mirror image.
    /// Point `n + i` mirrors point `i`; polygon `m + k` uses the mirrored
    /// points of polygon `k` with the winding reversed but the first corner
    /// kept (`[a, b, c, d]` → `[a', d', c', b']`), so normals stay outward.
    /// Per-corner data follows its corner. Off returns the mesh unchanged.
    pub fn doubled(self, mesh: Mesh) -> Result<Mesh> {
        if self.axis.is_none() || mesh.faces.is_empty() && mesh.vertices.is_empty() {
            return Ok(mesh);
        }
        // SYM-B2-05: a primitive centred on the plane is made once (measured
        // for the Cube; the bounding-box centre stands in for "its centre").
        let n = self.normal().unwrap_or(DVec3::X);
        let (lo, hi) = mesh
            .vertices
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(n.dot(*v)), hi.max(n.dot(*v)))
            });
        if ((lo + hi) / 2. - self.offset).abs() <= match_tolerance(&mesh) {
            return Ok(mesh);
        }
        let mut copy = mesh.clone();
        for v in &mut copy.vertices {
            *v = self.mirror(*v);
        }
        // Corner `k` of the mirrored polygon is corner `(n - k) % n` of the
        // original.
        fn flip<T: Clone>(corners: &mut [T]) {
            if corners.len() > 1 {
                corners[1..].reverse();
            }
        }
        for f in &mut copy.faces {
            flip(&mut f.vertices);
            if let Some(uv) = &mut f.uv {
                flip(uv);
            }
        }
        for map in copy.uv_maps.values_mut() {
            map.iter_mut().for_each(|c| flip(c));
        }
        let a = &mut copy.attributes;
        for map in a.uv_records.values_mut() {
            map.iter_mut().for_each(|c| flip(c));
        }
        for map in a.corner_weights.values_mut() {
            map.iter_mut().for_each(|c| flip(c));
        }
        for map in a.corner_colors.values_mut() {
            map.iter_mut().for_each(|c| flip(c));
        }
        for map in a.corner_normals.values_mut() {
            for corners in map.iter_mut() {
                flip(corners);
                for n in corners.iter_mut().flatten() {
                    *n = self.mirror_vector(*n);
                }
            }
        }
        if !a.extrude_edge_uses.is_empty() || !a.typed_maps.is_empty() {
            return Err("Symmetry can't mirror this primitive's extra mesh data yet".into());
        }
        let mut out = mesh;
        crate::append_mesh(&mut out, copy)?;
        Ok(out)
    }
}

/// What the creation path did with new geometry (symmetry-2026-11-13
/// SYM3-20..25), so the tool can say so in the status bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Creation {
    /// Symmetry is off (or nothing was made): the geometry is as drawn.
    Off,
    /// A mirror copy was added; points on the seam are shared.
    Mirrored,
    /// The geometry lies on the plane, so it is its own mirror (SYM3-21).
    Itself,
    /// The geometry has points on both sides: made once, no copy (SYM3-25).
    Crosses,
}
impl Creation {
    /// SYM3-25: the status message for geometry that crosses the plane.
    pub const CROSSES_MESSAGE: &'static str = "Crosses the symmetry plane: not mirrored";
    pub fn message(self) -> Option<&'static str> {
        (self == Self::Crosses).then_some(Self::CROSSES_MESSAGE)
    }
}

/// SYM3-40: the status line for a tool that doesn't support Symmetry yet.
pub fn ignores_message(tool: &str) -> String {
    format!("{tool} ignores Symmetry for now")
}

impl Symmetry {
    /// The creation path with the **Weld seam** policy (SYM3-20..25): the
    /// new geometry `mesh` (in the target's local space) followed by its
    /// mirror image, where points on the plane are single points shared by
    /// both halves, so the result is one connected surface with no crack and
    /// no doubled points. Polygons whose corners all lie on the plane are
    /// made once. Mirrored polygons reverse winding keeping their first
    /// corner (as `doubled`), so normals face outward; polylines keep their
    /// order. Seam points are put exactly on the plane.
    ///
    /// Geometry with points on both sides is returned unchanged
    /// ([`Creation::Crosses`], SYM3-25); geometry entirely on the plane is
    /// its own mirror ([`Creation::Itself`]).
    pub fn welded(self, mesh: Mesh) -> Result<(Mesh, Creation)> {
        self.mirror_append(mesh, true)
    }

    /// `welded`, or with `weld` false the seam points are copied too (the
    /// two halves meet but aren't joined: Symmetrize with Merge off).
    pub fn mirror_append(self, mut mesh: Mesh, weld: bool) -> Result<(Mesh, Creation)> {
        if self.axis.is_none() || mesh.vertices.is_empty() {
            return Ok((mesh, Creation::Off));
        }
        let tolerance = match_tolerance(&mesh);
        let sides: Vec<Side> = mesh
            .vertices
            .iter()
            .map(|&p| self.side(p, tolerance).unwrap_or(Side::Plane))
            .collect();
        let negative = sides.contains(&Side::Negative);
        let positive = sides.contains(&Side::Positive);
        if negative && positive {
            return Ok((mesh, Creation::Crosses));
        }
        for (p, side) in mesh.vertices.iter_mut().zip(&sides) {
            if *side == Side::Plane {
                *p = self.onto_plane(*p);
            }
        }
        if !negative && !positive {
            return Ok((mesh, Creation::Itself));
        }
        {
            let a = &mesh.attributes;
            let simple = a.typed_maps.is_empty()
                && a.vertex_sets.is_empty()
                && a.polygon_sets.is_empty()
                && a.edge_sets.is_empty()
                && a.point_uvs.is_empty()
                && a.corner_weights.is_empty()
                && a.corner_colors.is_empty()
                && a.extrude_edge_uses.is_empty()
                && a.relative_morphs.is_empty()
                && a.creases.is_empty()
                && a.uv_records.is_empty()
                && a.corner_normals.is_empty()
                && a.point_weights.is_empty()
                && a.point_colors.is_empty()
                && a.smoothing_labels.is_empty();
            if !simple {
                return Err("Symmetry can't mirror this tool's extra mesh data yet".into());
            }
        }
        let n = mesh.vertices.len();
        // Seam points map to themselves; the others get a mirrored copy.
        let mut mirrored_of = vec![0u32; n];
        let mut copies = Vec::new();
        for (v, side) in sides.iter().enumerate() {
            mirrored_of[v] = if weld && *side == Side::Plane {
                v as u32
            } else {
                copies.push(self.mirror(mesh.vertices[v]));
                (n + copies.len() - 1) as u32
            };
        }
        mesh.vertices.extend(copies);
        let has_representations = mesh.attributes.representations.len() == mesh.faces.len();
        let mut faces = mesh.faces.to_vec();
        let mut representations = mesh.attributes.representations.clone();
        let mut uv_maps = mesh.uv_maps.to_owned_value();
        fn flip<T: Clone>(corners: &mut [T]) {
            if corners.len() > 1 {
                corners[1..].reverse();
            }
        }
        for (k, face) in mesh.faces.iter().enumerate() {
            if face
                .vertices
                .iter()
                .all(|&v| sides[v as usize] == Side::Plane)
            {
                continue;
            }
            let polyline = mesh.is_polyline(k);
            let mut copy = face.clone();
            for v in &mut copy.vertices {
                *v = mirrored_of[*v as usize];
            }
            if !polyline {
                flip(&mut copy.vertices);
                if let Some(uv) = &mut copy.uv {
                    flip(uv);
                }
            }
            faces.push(copy);
            if has_representations {
                representations.push(mesh.attributes.representations[k]);
            }
            for rows in uv_maps.values_mut() {
                let mut row = rows.get(k).cloned().unwrap_or_default();
                if !polyline {
                    flip(&mut row);
                }
                rows.push(row);
            }
        }
        if !representations.is_empty() && representations.len() != faces.len() {
            // Representations were partial (only some faces typed): pad.
            representations.resize(faces.len(), crate::PolygonRepresentation::Face);
        }
        mesh.faces = faces.into();
        mesh.uv_maps = uv_maps.into();
        mesh.attributes.representations = representations;
        mesh.validate()?;
        Ok((mesh, Creation::Mirrored))
    }
}

/// Mirror partners of chosen points of `mesh`, found once with a grid
/// (SYM3-04: nearest point to the mirror position within the matching
/// tolerance, lowest index on a tie; a point on the plane is its own).
/// With Topology on, partners come from connectivity.
pub fn partners_of(mesh: &Mesh, symmetry: Symmetry, points: &[u32]) -> Vec<Option<u32>> {
    if symmetry.axis.is_none() {
        return vec![None; points.len()];
    }
    if symmetry.topology {
        let map = topology_counterparts(mesh, symmetry);
        return points
            .iter()
            .map(|&v| map.get(v as usize).copied().flatten())
            .collect();
    }
    let tolerance = match_tolerance(mesh);
    let cell = tolerance.max(1e-12);
    let key = |p: DVec3| {
        let q = (p / cell).floor();
        (q.x as i64, q.y as i64, q.z as i64)
    };
    let mut grid: std::collections::HashMap<(i64, i64, i64), Vec<u32>> =
        std::collections::HashMap::new();
    for (i, &p) in mesh.vertices.iter().enumerate() {
        grid.entry(key(p)).or_default().push(i as u32);
    }
    points
        .iter()
        .map(|&v| {
            let target = symmetry.mirror(*mesh.vertices.get(v as usize)?);
            let (x, y, z) = key(target);
            let mut best: Option<(f64, u32)> = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        for &j in grid.get(&(x + dx, y + dy, z + dz)).into_iter().flatten() {
                            let d = mesh.vertices[j as usize].distance(target);
                            if d < tolerance
                                && best.is_none_or(|(bd, bj)| d < bd || d == bd && j < bj)
                            {
                                best = Some((d, j));
                            }
                        }
                    }
                }
            }
            best.map(|(_, j)| j)
        })
        .collect()
}

impl Symmetry {
    /// The movement path for a tool that keeps the topology (Edge Slide,
    /// Vertex Slide…): `out` is `base` after the tool moved some points the
    /// same way on both sides. Each moved point on the side opposite
    /// `primary` is put at the mirror image of its partner's new position,
    /// and moved seam points stay on the plane (SYM3-11/12/13), so a
    /// direction the tool chose for one side is mirrored for the other.
    /// Points whose partner didn't move (a one-sided selection) keep the
    /// tool's result (SYM3-10).
    pub fn mirror_moved(self, base: &Mesh, out: &mut Mesh, primary: Side) {
        if self.axis.is_none() || base.vertices.len() != out.vertices.len() {
            return;
        }
        let moved = crate::mesh::moved_points(&base.vertices, &out.vertices);
        if moved.is_empty() {
            return;
        }
        let tolerance = match_tolerance(base);
        let partners = partners_of(base, self, &moved);
        let mut is_moved = vec![false; base.vertices.len()];
        for &v in &moved {
            is_moved[v as usize] = true;
        }
        let other = match primary {
            Side::Positive => Side::Negative,
            _ => Side::Positive,
        };
        let new = out.vertices.clone();
        for (&v, partner) in moved.iter().zip(partners) {
            match self.side(base.vertices[v as usize], tolerance) {
                Some(Side::Plane) => out.vertices[v as usize] = self.onto_plane(new[v as usize]),
                Some(side) if side == other => {
                    if let Some(q) = partner.filter(|&q| q != v && is_moved[q as usize]) {
                        out.vertices[v as usize] = self.mirror(new[q as usize]);
                    }
                }
                _ => {}
            }
        }
    }

    /// The movement path for a tool that moves points by one vector `v`
    /// (local space; Extrude's offset, Extend's offset): of the `moved`
    /// points, each got `v`. Those that started on the side opposite
    /// `primary` get the mirrored vector instead, and those that started on
    /// the plane keep only the part of `v` along it (SYM3-11/12/13).
    pub fn mirror_offset(
        self,
        vertices: &mut [DVec3],
        moved: &[u32],
        v: DVec3,
        primary: Side,
        tolerance: f64,
    ) {
        let Some(n) = self.normal() else {
            return;
        };
        let other = match primary {
            Side::Positive => Side::Negative,
            _ => Side::Positive,
        };
        let fix = self.mirror_vector(v) - v;
        let mut done = std::collections::BTreeSet::new();
        for &p in moved {
            if !done.insert(p) {
                continue;
            }
            let Some(q) = vertices.get_mut(p as usize) else {
                continue;
            };
            match self.side(*q - v, tolerance) {
                Some(Side::Plane) => *q -= n * n.dot(v),
                Some(side) if side == other => *q += fix,
                _ => {}
            }
        }
    }
}

/// The points of `mesh` with no mirror partner within `tolerance`
/// (symmetry-2026-11-13 section 10 check 1, and SYM3-05's unpaired points):
/// a point on the plane must lie on it, every other point needs a point at
/// its mirror position. Empty means the mesh is exactly symmetric. Uses a
/// grid, so it is linear in the point count.
pub fn unpaired_points(mesh: &Mesh, symmetry: Symmetry, tolerance: f64) -> Vec<u32> {
    if symmetry.axis.is_none() {
        return Vec::new();
    }
    let cell = (tolerance * 4.).max(1e-12);
    let key = |p: DVec3| {
        let q = (p / cell).floor();
        (q.x as i64, q.y as i64, q.z as i64)
    };
    let mut grid: std::collections::HashMap<(i64, i64, i64), Vec<u32>> =
        std::collections::HashMap::new();
    for (i, &p) in mesh.vertices.iter().enumerate() {
        grid.entry(key(p)).or_default().push(i as u32);
    }
    let near = |target: DVec3| {
        let (x, y, z) = key(target);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(list) = grid.get(&(x + dx, y + dy, z + dz))
                        && list
                            .iter()
                            .any(|&j| mesh.vertices[j as usize].distance(target) <= tolerance)
                    {
                        return true;
                    }
                }
            }
        }
        false
    };
    let plane = match_tolerance(mesh).max(tolerance);
    (0..mesh.vertices.len() as u32)
        .filter(|&i| {
            let p = mesh.vertices[i as usize];
            match symmetry.side(p, plane) {
                Some(Side::Plane) => symmetry.onto_plane(p).distance(p) > tolerance,
                _ => !near(symmetry.mirror(p)),
            }
        })
        .collect()
}

/// SYM-SEL-05 / SYM-B2-02: matching tolerance. It changes when the mesh is
/// moved with its extents unchanged, and fits just under 3×10⁻⁶ of the largest absolute
/// coordinate, with equality not accepted (the formula itself is inferred:
/// SYM-Q04).
pub fn match_tolerance(mesh: &Mesh) -> f64 {
    tolerance_for(
        mesh.vertices
            .iter()
            .fold(0_f64, |m, v| m.max(v.abs().max_element())),
    )
}

/// `match_tolerance`, with the largest coordinate remembered per mesh
/// allocation (a drag asks every frame of the same base mesh).
pub fn match_tolerance_shared(mesh: &crate::Shared<Mesh>) -> f64 {
    tolerance_for(mesh.reach())
}

fn tolerance_for(largest: f64) -> f64 {
    if largest.is_finite() && largest > 0. {
        largest * 2.95e-6
    } else {
        1e-9
    }
}

/// Bounding-box diagonal of the mesh's points (0 when it has none).
fn diagonal(mesh: &Mesh) -> f64 {
    let mut lo = DVec3::splat(f64::INFINITY);
    let mut hi = DVec3::splat(f64::NEG_INFINITY);
    for &v in &mesh.vertices {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    let d = (hi - lo).length();
    if d.is_finite() { d } else { 0. }
}

/// SYM-SEL-06 / SYM-B2-07 Topology: each point's counterpart found through
/// connectivity, so a displaced point still matches on a symmetric mesh.
/// Points on the plane are their own counterparts; from there matched pairs
/// spread to their neighbours (each neighbour taking the nearest unmatched
/// mirrored neighbour), and stop where the two sides' connectivity differs.
/// The walk itself is Operandi's (the reference's algorithm wasn't
/// measured).
pub fn topology_counterparts(mesh: &Mesh, symmetry: Symmetry) -> Vec<Option<u32>> {
    let n = mesh.vertices.len();
    let mut map: Vec<Option<u32>> = vec![None; n];
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (a, b) in mesh.edges() {
        neighbours[a as usize].push(b);
        neighbours[b as usize].push(a);
    }
    let tolerance = match_tolerance(mesh);
    let mut queue = std::collections::VecDeque::new();
    for (v, &p) in mesh.vertices.iter().enumerate() {
        if symmetry.side(p, tolerance) == Some(Side::Plane) {
            map[v] = Some(v as u32);
            queue.push_back((v as u32, v as u32));
        }
    }
    if queue.is_empty() {
        // No points on the plane: start from exact geometric matches.
        for (v, &p) in mesh.vertices.iter().enumerate() {
            if let Some(m) = nearest(mesh, symmetry.mirror(p), tolerance) {
                map[v] = Some(m);
                queue.push_back((v as u32, m));
            }
        }
    }
    while let Some((u, w)) = queue.pop_front() {
        let (nu, nw) = (&neighbours[u as usize], &neighbours[w as usize]);
        if nu.len() != nw.len() {
            continue;
        }
        for &x in nu {
            if map[x as usize].is_some() {
                continue;
            }
            let target = symmetry.mirror(mesh.vertices[x as usize]);
            let best = nw
                .iter()
                .copied()
                .filter(|&y| map[y as usize].is_none() || y == x)
                .filter(|&y| neighbours[y as usize].len() == neighbours[x as usize].len())
                .min_by(|&a, &b| {
                    mesh.vertices[a as usize]
                        .distance(target)
                        .total_cmp(&mesh.vertices[b as usize].distance(target))
                });
            if let Some(y) = best {
                map[x as usize] = Some(y);
                map[y as usize] = Some(x);
                queue.push_back((x, y));
            }
        }
    }
    map
}

/// Nearest point to `target` within `tolerance`, lowest index on a tie.
pub(crate) fn nearest(mesh: &Mesh, target: DVec3, tolerance: f64) -> Option<u32> {
    let mut best: Option<(f64, u32)> = None;
    for (i, &v) in mesh.vertices.iter().enumerate() {
        let d = v.distance(target);
        if d < tolerance && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i as u32));
        }
    }
    best.map(|(_, i)| i)
}

impl Scene {
    /// The primary side for the selection with no tool centre: the pivot
    /// service's selection-based rule (ACS-01/03): + when anything
    /// selected is on the + side, else −. `None` with Symmetry off or no
    /// component selection. Tools with a live pivot ask
    /// `Editor::symmetry_primary` instead (a placed centre picks the side,
    /// ACS-07). This replaces the first-pick and view-side rules (N27,
    /// superseded by ACS-01, see SPECIFICATION_QUESTIONS ACS-B1).
    pub fn symmetry_primary_side(&self, symmetry: Symmetry) -> Option<Side> {
        symmetry.axis?;
        let mesh = &self.active()?.mesh;
        if self.selection.mode == Mode::Item {
            return None;
        }
        let points: Vec<u32> = crate::transform_vertices(self).ok()?.into_iter().collect();
        let half = crate::pivot_service::half_points(mesh, &points, symmetry);
        let centre = crate::box_centre(half.iter().map(|&v| mesh.vertices[v as usize]));
        Some(crate::pivot_service::primary_side(
            symmetry,
            centre,
            match_tolerance(mesh),
        ))
    }

    /// Selected points and polygons on the mirror side of the first pick, for
    /// a secondary highlight (Jason, 2026-11-02).
    pub fn symmetry_secondary(&self, symmetry: Symmetry) -> (BTreeSet<u32>, BTreeSet<u32>) {
        let mut out = (BTreeSet::new(), BTreeSet::new());
        if self.selection.vertices.is_empty() && self.selection.faces.is_empty() {
            return out;
        }
        let (Some(primary), Some(item)) = (self.symmetry_primary_side(symmetry), self.active())
        else {
            return out;
        };
        let mesh = &item.mesh;
        let tolerance = match_tolerance(mesh);
        let other = |p: DVec3| {
            let side = symmetry.side(p, tolerance);
            side.is_some() && side != Some(primary) && side != Some(Side::Plane)
        };
        for &v in &self.selection.vertices {
            if mesh.vertices.get(v as usize).is_some_and(|&p| other(p)) {
                out.0.insert(v);
            }
        }
        for &f in &self.selection.faces {
            if (f as usize) < mesh.faces.len() && other(mesh.center(f as usize)) {
                out.1.insert(f);
            }
        }
        out
    }

    /// The mirror counterpart of a component of item `item` under
    /// `symmetry`, by the same matching as Select Symmetrical. `None` when Off,
    /// for items, or when no counterpart is found.
    pub fn symmetric_counterpart(
        &self,
        item: u64,
        component: crate::Component,
        symmetry: Symmetry,
    ) -> Option<crate::Component> {
        symmetry.axis?;
        let mesh = &self.item(item)?.mesh;
        let tolerance = match_tolerance(mesh);
        let topology = symmetry
            .topology
            .then(|| topology_counterparts(mesh, symmetry));
        let mirror = |v: u32| match &topology {
            Some(map) => map.get(v as usize).copied().flatten(),
            None => nearest(
                mesh,
                symmetry.mirror(*mesh.vertices.get(v as usize)?),
                tolerance,
            ),
        };
        use crate::Component::*;
        match component {
            Vertex(v) => mirror(v).map(Vertex),
            Edge(a, b) => {
                let (x, y) = (mirror(a)?, mirror(b)?);
                mesh.edges()
                    .contains(&(x.min(y), x.max(y)))
                    .then_some(Edge(x.min(y), x.max(y)))
            }
            Polygon(f) => {
                let face = mesh.faces.get(f as usize)?;
                let set = face
                    .vertices
                    .iter()
                    .map(|&v| mirror(v))
                    .collect::<Option<BTreeSet<u32>>>()?;
                mesh.faces
                    .iter()
                    .position(|g| {
                        g.vertices.len() == face.vertices.len()
                            && g.vertices.iter().copied().collect::<BTreeSet<_>>() == set
                    })
                    .map(|g| Polygon(g as u32))
            }
            Item => None,
        }
    }

    /// SYM-SEL-01..04. Adds the counterpart of every selected element in the
    /// current mode, keeping the original selection in pick order. Missing
    /// points are appended in descending index whatever the pick order
    /// (SYM-B2-01: 8, 9 → 8, 9, 6, 5 and 9, 6 → 9, 6, 8, 5; provisional,
    /// SYM-Q-ORDER); edges and polygons walk the selection from the most
    /// recent pick back (one addition measured, SYM-O-04). Off does
    /// nothing (SYM-SEL-02).
    /// Topology matching (SYM-SEL-06) and hidden counterparts (SYM-Q08) are
    /// not built.
    pub fn select_symmetrical(&mut self, symmetry: Symmetry) -> Result<()> {
        symmetry.validate()?;
        if symmetry.axis.is_none() {
            return Ok(());
        }
        let mesh = &self
            .active()
            .ok_or("Select a mesh for Select Symmetrical")?
            .mesh;
        let tolerance = match_tolerance(mesh);
        let topology = symmetry
            .topology
            .then(|| topology_counterparts(mesh, symmetry));
        let counterpart = |v: u32| -> Option<u32> {
            if let Some(map) = &topology {
                return map.get(v as usize).copied().flatten();
            }
            let p = *mesh.vertices.get(v as usize)?;
            nearest(mesh, symmetry.mirror(p), tolerance)
        };
        let s = &self.selection;
        match s.mode {
            Mode::Vertex => {
                let mut added: Vec<u32> = s
                    .vertices
                    .iter()
                    .filter_map(|&v| counterpart(v))
                    .filter(|m| !s.vertices.contains(m))
                    .collect();
                added.sort_unstable_by(|a, b| b.cmp(a));
                added.dedup();
                let mut out = s.vertices.clone();
                out.extend(added);
                self.selection.vertices = out;
            }
            Mode::Edge => {
                let edges: BTreeSet<(u32, u32)> = mesh.edges().into_iter().collect();
                let key = |a: u32, b: u32| (a.min(b), a.max(b));
                let mut out = s.edges.clone();
                for &(a, b) in s.edges.iter().rev() {
                    if let (Some(ma), Some(mb)) = (counterpart(a), counterpart(b)) {
                        let e = key(ma, mb);
                        if edges.contains(&e) && !out.iter().any(|&(x, y)| key(x, y) == e) {
                            out.push(e);
                        }
                    }
                }
                self.selection.edges = out;
            }
            Mode::Polygon => {
                let mut out = s.faces.clone();
                for &f in s.faces.iter().rev() {
                    let Some(face) = mesh.faces.get(f as usize) else {
                        continue;
                    };
                    let Some(mirrored) = face
                        .vertices
                        .iter()
                        .map(|&v| counterpart(v))
                        .collect::<Option<BTreeSet<u32>>>()
                    else {
                        continue;
                    };
                    let found = mesh.faces.iter().position(|g| {
                        g.vertices.len() == face.vertices.len()
                            && g.vertices.iter().copied().collect::<BTreeSet<_>>() == mirrored
                    });
                    if let Some(g) = found.map(|g| g as u32)
                        && !out.contains(&g)
                    {
                        out.push(g);
                    }
                }
                self.selection.faces = out;
            }
            Mode::Item => {
                return Err("Select Symmetrical works on vertices, edges or polygons".into());
            }
        }
        Ok(())
    }

    /// SYM-SZ-01/02: move every point on the target side so it mirrors its
    /// partner on the source side; the whole mesh, nothing selected needed.
    /// Points on the plane stay. Partners are mutual nearest mirrored points
    /// within 1% of the bounding-box diagonal: the measured pairs, off by up
    /// to 0.02 m on a 4 m strip, match (SYM-SZ-03). The exact rule is open
    /// (SYM-Q09); this one is provisional.
    pub fn symmetrize(&mut self, symmetry: Symmetry, side: SymmetrizeSide) -> Result<()> {
        symmetry.validate()?;
        if symmetry.axis.is_none() {
            return Err("Turn Symmetry on (X, Y or Z) before Symmetrize".into());
        }
        let item = self.active().ok_or("Select a mesh for Symmetrize")?;
        let mesh = &item.mesh;
        let plane_tolerance = match_tolerance(mesh);
        let loose = diagonal(mesh) * 0.01;
        let (source, target) = match side {
            SymmetrizeSide::Positive => (Side::Positive, Side::Negative),
            SymmetrizeSide::Negative => (Side::Negative, Side::Positive),
        };
        let on = |s: Side| -> Vec<u32> {
            (0..mesh.vertices.len() as u32)
                .filter(|&i| symmetry.side(mesh.vertices[i as usize], plane_tolerance) == Some(s))
                .collect()
        };
        let sources = on(source);
        let targets = on(target);
        let closest = |p: DVec3, among: &[u32]| -> Option<u32> {
            among
                .iter()
                .copied()
                .map(|i| (mesh.vertices[i as usize].distance(p), i))
                .filter(|&(d, _)| d <= loose)
                .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
                .map(|(_, i)| i)
        };
        let mut moves = Vec::new();
        for &t in &targets {
            let mirrored = symmetry.mirror(mesh.vertices[t as usize]);
            if let Some(s) = closest(mirrored, &sources) {
                let back = symmetry.mirror(mesh.vertices[s as usize]);
                if closest(back, &targets) == Some(t) {
                    moves.push((t, back));
                }
            }
        }
        let item = self.active_mut().ok_or("Select a mesh")?;
        for (t, p) in moves {
            item.mesh.vertices[t as usize] = p;
        }
        self.validate()
    }
}

impl Scene {
    /// Symmetrize (polygons), symmetry-2026-11-13 section 9: delete the
    /// half on the target side, mirror the source half across and, with
    /// `merge`, join the two at the seam. Polygons crossing the plane go
    /// with the target half. With Topology on only points move (the points
    /// version, `symmetrize`) and Merge doesn't apply. The selection is
    /// cleared (the components it named are gone).
    pub fn symmetrize_polygons(
        &mut self,
        symmetry: Symmetry,
        side: SymmetrizeSide,
        merge: bool,
    ) -> Result<()> {
        symmetry.validate()?;
        if symmetry.axis.is_none() {
            return Err("Turn Symmetry on (X, Y or Z) before Symmetrize".into());
        }
        if symmetry.topology {
            return self.symmetrize(symmetry, side);
        }
        let item = self.active().ok_or("Select a mesh for Symmetrize")?;
        let mesh = Mesh::clone(&item.mesh);
        let tolerance = match_tolerance(&mesh);
        let target = match side {
            SymmetrizeSide::Positive => Side::Negative,
            SymmetrizeSide::Negative => Side::Positive,
        };
        let on_target: Vec<bool> = mesh
            .vertices
            .iter()
            .map(|&p| symmetry.side(p, tolerance) == Some(target))
            .collect();
        let doomed: BTreeSet<usize> = (0..mesh.faces.len())
            .filter(|&f| {
                mesh.faces[f]
                    .vertices
                    .iter()
                    .any(|&v| on_target[v as usize])
            })
            .collect();
        let mut half = mesh.clone();
        half.remove_faces(&doomed)?;
        // Target-side points the deleted polygons used go too; loose
        // points elsewhere stay.
        crate::smooth_shift::compact(&mesh, &mut half);
        let keep: Vec<u32> = (0..half.vertices.len() as u32)
            .filter(|&v| symmetry.side(half.vertices[v as usize], tolerance) != Some(target))
            .collect();
        if keep.len() != half.vertices.len() {
            return Err(
                "Symmetrize: loose points on the side being replaced; delete them first".into(),
            );
        }
        let (out, _) = symmetry.mirror_append(half, merge)?;
        let item = self.active_mut().ok_or("Select a mesh")?;
        item.mesh = out.into();
        self.clear_components();
        self.validate()
    }
}

impl Scene {
    /// Spec v3.1 section 11: after a lasso or rectangle, the swept
    /// elements come first in the selection, then the mirror partners the
    /// gesture added (in the order they were added). Only the active
    /// mesh's selection is reordered; partners no longer selected are
    /// skipped.
    pub(crate) fn partners_last(&mut self, partners: &[(u64, crate::Component)]) {
        let Some(active) = self.active().map(|i| i.id) else {
            return;
        };
        fn to_end<T: PartialEq + Copy>(list: &mut Vec<T>, x: T) {
            if let Some(i) = list.iter().position(|&y| y == x) {
                list.remove(i);
                list.push(x);
            }
        }
        let s = &mut self.selection;
        for &(_, c) in partners.iter().filter(|(i, _)| *i == active) {
            match c {
                crate::Component::Vertex(v) => to_end(&mut s.vertices, v),
                crate::Component::Edge(a, b) => {
                    let found = s
                        .edges
                        .iter()
                        .copied()
                        .find(|&(x, y)| (x.min(y), x.max(y)) == (a.min(b), a.max(b)));
                    if let Some(e) = found {
                        to_end(&mut s.edges, e);
                    }
                }
                crate::Component::Polygon(f) => to_end(&mut s.faces, f),
                crate::Component::Item => {}
            }
        }
    }
}
