use crate::Result;
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Face {
    pub vertices: Vec<u32>,
    pub uv: Option<Vec<DVec2>>,
    pub material: String,
    pub smooth: bool,
    #[serde(default)]
    pub part: Option<String>,
    #[serde(default)]
    pub material_label: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mesh {
    /// Shared copy-on-write like `faces` (corner maps hold a list per
    /// polygon).
    #[serde(default)]
    pub attributes: crate::Shared<crate::MeshAttributes>,
    // Big arrays are written packed (mesh_pack.rs, native version 34).
    #[serde(with = "crate::mesh_pack::points")]
    pub vertices: Vec<DVec3>,
    /// Shared copy-on-write, so a copy of a mesh whose points alone change
    /// (every drag frame, every undo checkpoint of a move) shares its
    /// polygons instead of copying a million of them.
    #[serde(with = "crate::mesh_pack::polygons")]
    pub faces: crate::Shared<Vec<Face>>,
    /// Shared copy-on-write like `faces` (a list per polygon per map).
    #[serde(default, with = "crate::mesh_pack::uv_maps")]
    pub uv_maps: crate::Shared<BTreeMap<String, Vec<Vec<Option<DVec2>>>>>,
}
/// The points whose coordinates differ bit for bit between two equally long
/// point lists, in order. Blocks of points are compared branch-free first, so
/// a drag frame's diff of a million points runs at memory speed and only the
/// blocks that changed are looked at point by point.
pub fn moved_points(old: &[DVec3], new: &[DVec3]) -> Vec<u32> {
    const BLOCK: usize = 64;
    let bits = |a: &DVec3, b: &DVec3| {
        (a.x.to_bits() ^ b.x.to_bits())
            | (a.y.to_bits() ^ b.y.to_bits())
            | (a.z.to_bits() ^ b.z.to_bits())
    };
    let mut out = Vec::new();
    for (block, (a, b)) in old.chunks(BLOCK).zip(new.chunks(BLOCK)).enumerate() {
        if a.iter().zip(b).fold(0, |acc, (p, q)| acc | bits(p, q)) == 0 {
            continue;
        }
        for (i, (p, q)) in a.iter().zip(b).enumerate() {
            if bits(p, q) != 0 {
                out.push((block * BLOCK + i) as u32);
            }
        }
    }
    out
}

/// Whether a polygon lists a point twice (without allocating for the
/// usual few corners).
fn repeats(v: &[u32]) -> bool {
    if v.len() <= 16 {
        (1..v.len()).any(|i| v[..i].contains(&v[i]))
    } else {
        v.iter().collect::<BTreeSet<_>>().len() != v.len()
    }
}

/// What a mesh's polygon list says on its own, remembered per polygon list
/// allocation (`Mesh::face_summary`), so per-frame readouts don't read a
/// million polygons while a drag moves points.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FaceSummary {
    /// Polygons with smooth shading on (`Face::smooth`).
    pub smooth: usize,
    /// Corners of those polygons.
    pub smooth_corners: u64,
    /// Every distinct `Face::material`, sorted.
    pub materials: Vec<String>,
    /// Triangles the polygons fill drawn as they are (an n-gon is n - 2):
    /// the drawn count of a mesh with no listed polygon kinds.
    pub plain_triangles: u64,
}
impl Mesh {
    pub fn face_summary(&self) -> std::sync::Arc<FaceSummary> {
        self.faces.derived(|faces| {
            let mut out = FaceSummary::default();
            let mut materials = BTreeSet::new();
            for f in faces {
                out.plain_triangles += f.vertices.len().saturating_sub(2) as u64;
                if f.smooth {
                    out.smooth += 1;
                    out.smooth_corners += f.vertices.len() as u64;
                }
                if !materials.contains(&f.material) {
                    materials.insert(f.material.clone());
                }
            }
            out.materials = materials.into_iter().collect();
            out
        })
    }
}
/// A mesh's extent (`Shared<Mesh>::extent`).
#[derive(Default)]
struct Extent {
    reach: f64,
    bounds: Option<(DVec3, DVec3)>,
}
impl crate::Shared<Mesh> {
    /// The largest point coordinate, in absolute value (0 for no points),
    /// remembered for this allocation.
    pub fn reach(&self) -> f64 {
        self.extent().reach
    }
    /// The points' bounds (`None` for no points), remembered for this
    /// allocation.
    pub fn bounds(&self) -> Option<(DVec3, DVec3)> {
        self.extent().bounds
    }
    /// One pass over the points for `reach` and `bounds` (one remembered
    /// result per allocation, so both share it).
    fn extent(&self) -> std::sync::Arc<Extent> {
        self.derived(|m| {
            let mut out = Extent::default();
            for p in &m.vertices {
                out.reach = out.reach.max(p.abs().max_element());
                out.bounds = Some(match out.bounds {
                    Some((lo, hi)) => (lo.min(*p), hi.max(*p)),
                    None => (*p, *p),
                });
            }
            out
        })
    }
    /// `Mesh::validate`, skipped when this exact shared allocation already
    /// passed it (see `shared.rs`). Shadows the plain method for `item.mesh`.
    pub fn validate(&self) -> Result<()> {
        self.checked(Mesh::validate)
    }
    /// Records that this mesh passes `Mesh::validate` without reading every
    /// polygon, when it is `base` (which passed) with only some points
    /// moved: the same polygon list allocation, the same UV maps, attributes
    /// and point count, every moved point within the coordinate budget, and
    /// no polygon using one of them now degenerate. That is exactly what
    /// `validate` would check again, so a drag frame costs what its moved
    /// points cost. Returns false (recording nothing) when it can't tell;
    /// the caller's `validate` then runs in full and reports any error.
    pub(crate) fn prove_points_moved(&self, base: &Self) -> bool {
        if crate::Shared::same(self, base) {
            return self.passed();
        }
        let (old, new) = (&**base, &**self);
        if !base.passed()
            || !crate::Shared::same(&old.faces, &new.faces)
            || old.vertices.len() != new.vertices.len()
            || old.uv_maps != new.uv_maps
            || old.attributes != new.attributes
        {
            return false;
        }
        let moved = moved_points(&old.vertices, &new.vertices);
        if moved.iter().any(|&v| {
            let p = new.vertices[v as usize];
            !p.is_finite() || p.abs().max_element() > 1e9
        }) {
            return false;
        }
        let around = new.point_faces();
        let degenerate = moved.iter().flat_map(|&v| around.of(v)).any(|&f| {
            let f = f as usize;
            !new.is_polyline(f)
                && new.normal(f) == DVec3::ZERO
                && !new.attributes.degenerate_faces.contains(&(f as u32))
        });
        if degenerate {
            return false;
        }
        self.record_checked();
        true
    }
}
impl crate::Shared<Mesh> {
    /// Records that this mesh passes `Mesh::validate` without reading every
    /// polygon, when it is `base` (which passed) with only the values of UV
    /// map `map` changed: the same polygons, points, attributes and other
    /// maps, and the edited map the same shape with finite values. A UV
    /// drag frame then costs what its map costs. Returns false (recording
    /// nothing) when it can't tell.
    pub(crate) fn prove_uv_values_moved(&self, base: &Self, map: &str) -> bool {
        if crate::Shared::same(self, base) {
            return self.passed();
        }
        let (old, new) = (&**base, &**self);
        if !base.passed()
            || !crate::Shared::same(&old.faces, &new.faces)
            || old.vertices != new.vertices
            || old.attributes != new.attributes
            || old.uv_maps.len() != new.uv_maps.len()
        {
            return false;
        }
        for (name, rows) in new.uv_maps.iter() {
            let Some(before) = old.uv_maps.get(name) else {
                return false;
            };
            if name != map {
                if rows != before {
                    return false;
                }
                continue;
            }
            if rows.len() != before.len()
                || rows
                    .iter()
                    .zip(before)
                    .any(|(r, b)| r.len() != b.len() || r.iter().flatten().any(|v| !v.is_finite()))
            {
                return false;
            }
        }
        self.record_checked();
        true
    }
}
impl Mesh {
    pub fn box_mesh(center: DVec3, size: DVec3, plane: u8) -> Result<Self> {
        if plane > 2
            || !center.is_finite()
            || !size.is_finite()
            || size.min_element() <= 1e-8
            || size.max_element() > 1e8
        {
            return Err(
                "Box dimensions must be finite, positive and between 10 nm and 100,000 km".into(),
            );
        }
        let vertices = [
            (-1., -1., -1.),
            (1., -1., -1.),
            (1., 1., -1.),
            (-1., 1., -1.),
            (-1., -1., 1.),
            (1., -1., 1.),
            (1., 1., 1.),
            (-1., 1., 1.),
        ]
        .map(|(x, y, z)| {
            let p = DVec3::new(x, y, z) * size * 0.5;
            center + p
        })
        .to_vec();
        let faces = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 4, 7, 3],
            [1, 2, 6, 5],
            [0, 1, 5, 4],
            [3, 7, 6, 2],
        ]
        .into_iter()
        .map(|v| Face {
            vertices: v.to_vec(),
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        })
        .collect();
        let mesh = Self {
            vertices,
            faces,
            uv_maps: BTreeMap::new().into(),
            attributes: Default::default(),
        };
        mesh.validate()?;
        Ok(mesh)
    }
    pub fn is_polyline(&self, element: usize) -> bool {
        matches!(
            self.attributes.representations.get(element),
            Some(crate::PolygonRepresentation::Polyline { .. })
        )
    }
    pub fn has_polylines(&self) -> bool {
        (0..self.faces.len()).any(|f| self.is_polyline(f))
    }
    /// Directed spans of an element. Face corners form a cycle; open lines do not.
    pub fn element_spans(&self, element: usize) -> impl Iterator<Item = (u32, u32)> + '_ {
        let f = &self.faces[element];
        let open = matches!(
            self.attributes.representations.get(element),
            Some(crate::PolygonRepresentation::Polyline { closed: false, .. })
        );
        let count = f.vertices.len().saturating_sub(usize::from(open));
        (0..count).map(move |i| (f.vertices[i], f.vertices[(i + 1) % f.vertices.len()]))
    }
    pub fn require_surface_elements(&self) -> Result<()> {
        if self.has_polylines() {
            return Err("This tool can't edit polylines yet; nothing changed".into());
        }
        Ok(())
    }
    /// One flag per point: true when at least one polygon uses it. Linear in
    /// the mesh size; tools use it instead of scanning every polygon per point.
    pub fn used_points(&self) -> Vec<bool> {
        let mut used = vec![false; self.vertices.len()];
        for f in &self.faces {
            for &v in &f.vertices {
                if let Some(u) = used.get_mut(v as usize) {
                    *u = true;
                }
            }
        }
        used
    }
    /// How many polygon sides use each edge (1 on an open border), keyed
    /// (low point, high point). Remembered per topology like `edges`, so
    /// tools read it every drag frame instead of counting the whole mesh.
    pub fn edge_counts(&self) -> std::rc::Rc<crate::topology_cache::EdgeCounts> {
        crate::topology_cache::edge_counts(self, || {
            let mut counts = crate::topology_cache::EdgeCounts::default();
            for f in &self.faces {
                let n = f.vertices.len();
                for i in 0..n {
                    let (a, b) = (f.vertices[i], f.vertices[(i + 1) % n]);
                    *counts.entry((a.min(b), a.max(b))).or_default() += 1;
                }
            }
            counts
        })
    }
    /// The polygons using each point. Remembered per topology like
    /// `edges`, so a drag's display can look up a point's neighbours
    /// without scanning every polygon.
    /// Each polygon side's opposite side in the polygon across its edge
    /// (`SideMates`), remembered per topology.
    pub fn side_mates(&self) -> std::rc::Rc<crate::topology_cache::SideMates> {
        crate::topology_cache::side_mates(self, || crate::topology_cache::SideMates::new(self))
    }
    /// The surface polygons on each edge (`EdgeFaces`), remembered per
    /// topology like `edges`.
    pub fn edge_faces(&self) -> std::rc::Rc<crate::topology_cache::EdgeFaces> {
        crate::topology_cache::edge_faces(self, || {
            let mut map = crate::topology_cache::EdgeFaces::new();
            for (fi, f) in self.faces.iter().enumerate() {
                if self.is_polyline(fi) {
                    continue;
                }
                let n = f.vertices.len();
                for i in 0..n {
                    let key = crate::topology::edge_key(f.vertices[i], f.vertices[(i + 1) % n]);
                    map.entry(key).or_default().push(fi);
                }
            }
            map
        })
    }
    pub fn point_faces(&self) -> std::rc::Rc<crate::topology_cache::PointFaces> {
        crate::topology_cache::point_faces(self, || crate::topology_cache::PointFaces::new(self))
    }
    pub fn edges(&self) -> Vec<(u32, u32)> {
        // Remembered per topology: a drag that only moves points gets the
        // list without sorting every edge again (see `topology_cache.rs`).
        crate::topology_cache::edges(self, || {
            // A contiguous corner-edge buffer avoids tree nodes/allocations during
            // repeated enumeration. Sorting preserves the canonical public order
            // used by selection indices; coincident positions never merge IDs.
            let mut edges = Vec::with_capacity(self.faces.iter().map(|f| f.vertices.len()).sum());
            for fi in 0..self.faces.len() {
                for (a, b) in self.element_spans(fi) {
                    edges.push((a.min(b), a.max(b)));
                }
            }
            edges.sort_unstable();
            edges.dedup();
            edges
        })
        .as_ref()
        .clone()
    }
    pub fn normal(&self, face: usize) -> DVec3 {
        if self.is_polyline(face) {
            return DVec3::ZERO;
        }
        let f = &self.faces[face];
        let mut n = DVec3::ZERO;
        let origin = self.vertices[f.vertices[0] as usize];
        for i in 0..f.vertices.len() {
            n += (self.vertices[f.vertices[i] as usize] - origin)
                .cross(self.vertices[f.vertices[(i + 1) % f.vertices.len()] as usize] - origin);
        }
        n.normalize_or_zero()
    }
    pub fn center(&self, face: usize) -> DVec3 {
        let f = &self.faces[face];
        f.vertices
            .iter()
            .map(|&i| self.vertices[i as usize])
            .sum::<DVec3>()
            / f.vertices.len() as f64
    }
    pub fn validate(&self) -> Result<()> {
        self.attributes.validate(self)?;
        if self.vertices.len() > 5_000_000 || self.faces.len() > 5_000_000 {
            return Err("Mesh exceeds development safety budget".into());
        }
        if self
            .vertices
            .iter()
            .any(|p| !p.is_finite() || p.abs().max_element() > 1e9)
        {
            return Err("Invalid mesh coordinates".into());
        }
        if self.uv_maps.len() > 64 {
            return Err("UV map count limit".into());
        }
        for (name, faces) in &self.uv_maps {
            if name.len() > 1_048_576
                || faces.len() != self.faces.len()
                || faces.iter().zip(&self.faces).any(|(u, f)| {
                    u.len() != f.vertices.len() || u.iter().flatten().any(|v| !v.is_finite())
                })
            {
                return Err("Invalid named sparse UV map".into());
            }
        }
        for (fi, f) in self.faces.iter().enumerate() {
            // A polygon needs three points, a line two (copied edges paste
            // as two-point lines, CLP-04), except a one-point polygon:
            // Make Polygon from one point (make-split-2026-11-05 rule 2,
            // MP06), kept as an open straight line of one point, so it has
            // no spans, no area and no edges.
            let min = match self.attributes.representations.get(fi) {
                Some(crate::PolygonRepresentation::Polyline {
                    closed: false,
                    curve: crate::curves::CurveKind::Line,
                }) => 1,
                Some(crate::PolygonRepresentation::Polyline { .. }) => 2,
                _ => 3,
            };
            if f.vertices.len() < min
                || f.vertices.len() > 4096
                || f.vertices
                    .iter()
                    .any(|&i| i as usize >= self.vertices.len())
                || (repeats(&f.vertices)
                    && !self.attributes.degenerate_faces.contains(&(fi as u32)))
            {
                return Err(format!("Invalid face {fi}"));
            }
            if let Some(uv) = &f.uv
                && (uv.len() != f.vertices.len() || uv.iter().any(|v| !v.is_finite()))
            {
                return Err("Invalid corner UVs".into());
            }
            if !self.is_polyline(fi)
                && self.normal(fi) == DVec3::ZERO
                && !self.attributes.degenerate_faces.contains(&(fi as u32))
            {
                return Err(format!("Degenerate face {fi}"));
            }
        }
        // Topology only, so remembered per topology (see `topology_cache.rs`):
        // a drag that moves points checks edges without sorting them again.
        if *self.exceptional_edge_uses() != self.attributes.extrude_edge_uses {
            return Err("Nonmanifold edge or inconsistent winding".into());
        }
        Ok(())
    }
    /// `edge_uses`, remembered per topology.
    pub(crate) fn exceptional_edge_uses(&self) -> std::rc::Rc<crate::topology_cache::EdgeUses> {
        crate::topology_cache::edge_uses(self, || self.edge_uses())
    }
    /// Edges used by more than two polygons, or twice in the same direction,
    /// keyed "a:b" with each use as [polygon, from, to] in polygon order.
    pub(crate) fn edge_uses(&self) -> crate::topology_cache::EdgeUses {
        // Edge uses as a flat list, sorted below: the same groups (in face
        // order within each edge) as a sorted map, much faster on big meshes.
        let corners: usize = self.faces.iter().map(|f| f.vertices.len()).sum();
        let mut edges: Vec<((u32, u32), [u32; 3])> = Vec::with_capacity(corners);
        for (fi, f) in self.faces.iter().enumerate() {
            if self.is_polyline(fi) {
                continue;
            }
            for i in 0..f.vertices.len() {
                let a = f.vertices[i];
                let b = f.vertices[(i + 1) % f.vertices.len()];
                edges.push(((a.min(b), a.max(b)), [fi as u32, a, b]));
            }
        }
        // Stable, so uses of one edge stay in face order.
        edges.sort_by_key(|e| e.0);
        let mut exceptions = BTreeMap::new();
        for group in edges.chunk_by(|x, y| x.0 == y.0) {
            let ((a, b), _) = group[0];
            if group.len() > 2 || (group.len() == 2 && group[0].1[1..] == group[1].1[1..]) {
                exceptions.insert(
                    format!("{a}:{b}"),
                    group.iter().map(|g| g.1).collect::<Vec<_>>(),
                );
            }
        }
        exceptions
    }
    /// Conservative isolation proof for tools that read adjacent faces. An
    /// empty selection means all faces for such tools and cannot isolate an
    /// existing exceptional boundary.
    pub(crate) fn require_disjoint_exception_component(&self, selected: &[u32]) -> Result<()> {
        if self.attributes.extrude_edge_uses.is_empty() {
            return Ok(());
        }
        if selected.is_empty() {
            return Err(
                "Exceptional winding boundary is outside this operation's supported component"
                    .into(),
            );
        }
        let mut incident: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (fi, face) in self.faces.iter().enumerate() {
            for &vertex in &face.vertices {
                incident.entry(vertex).or_default().push(fi);
            }
        }
        let mut vertices = BTreeSet::new();
        let mut pending = Vec::new();
        for &f in selected {
            let face = self
                .faces
                .get(f as usize)
                .ok_or("Invalid selected polygon")?;
            for &vertex in &face.vertices {
                if vertices.insert(vertex) {
                    pending.push(vertex);
                }
            }
        }
        let mut visited_faces = BTreeSet::new();
        while let Some(vertex) = pending.pop() {
            for &fi in incident.get(&vertex).into_iter().flatten() {
                if visited_faces.insert(fi) {
                    for &v in &self.faces[fi].vertices {
                        if vertices.insert(v) {
                            pending.push(v);
                        }
                    }
                }
            }
        }
        if self
            .attributes
            .extrude_edge_uses
            .values()
            .flatten()
            .any(|r| vertices.contains(&r[1]) || vertices.contains(&r[2]))
        {
            return Err(
                "Exceptional winding boundary is outside this operation's supported component"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn is_closed(&self) -> bool {
        !self.has_polylines()
            && self.edges().len() * 2 == self.faces.iter().map(|f| f.vertices.len()).sum::<usize>()
    }
    /// Compatibility entry point; source replacement now preserves arbitrary
    /// valid simple planar polygon boundaries, including concave n-gons.
    pub fn cap_edit(&self, face: usize, inset: f64, offset: DVec3) -> Result<Self> {
        // Legacy callers supply an explicit vector as well as a planar inset.
        let shifted = self.edit_polygons(
            &[u32::try_from(face).map_err(|_| "Face limit")?],
            crate::PolygonEdit::Bevel {
                shift: 0.,
                inset,
                grouped: true,
            },
        )?;
        if offset == DVec3::ZERO {
            return Ok(shifted);
        }
        if !offset.is_finite() {
            return Err("Offset must be finite".into());
        }
        if inset == 0. {
            return self.edit_polygons(
                &[face as u32],
                crate::PolygonEdit::Extrude {
                    extent: offset,
                    sides: 1,
                },
            );
        }
        let mut out = shifted;
        for &v in &out.faces[face].vertices {
            out.vertices[v as usize] += offset;
        }
        out.validate()?;
        Ok(out)
    }
}
/// Major-axis plane orientation, preserving right-handed winding: XZ / YZ / XY.
pub fn orient(p: DVec3, plane: u8) -> DVec3 {
    match plane {
        1 => DVec3::new(p.y, -p.x, p.z),
        2 => DVec3::new(p.x, -p.z, p.y),
        _ => p,
    }
}

impl Mesh {
    /// Remove the faces in `remove`, keeping the others in order with every
    /// per-face record; face references to removed faces are dropped.
    pub fn remove_faces(&mut self, remove: &BTreeSet<usize>) -> Result<()> {
        if remove.is_empty() {
            return Ok(());
        }
        let n = self.faces.len();
        if remove.iter().any(|&f| f >= n) {
            return Err("Invalid face removal".into());
        }
        // Move the removed faces to the end, then truncate.
        let kept = n - remove.len();
        let mut next_kept = 0;
        let mut next_removed = kept;
        let new_of_old: Vec<usize> = (0..n)
            .map(|f| {
                let slot = if remove.contains(&f) {
                    &mut next_removed
                } else {
                    &mut next_kept
                };
                *slot += 1;
                *slot - 1
            })
            .collect();
        self.permute_faces(&new_of_old)?;
        self.faces.truncate(kept);
        for corners in self.uv_maps.values_mut() {
            corners.truncate(kept);
        }
        let a = &mut self.attributes;
        for m in a.typed_maps.values_mut() {
            m.corners.truncate(kept);
        }
        for set in a.polygon_sets.values_mut() {
            set.retain(|&f| (f as usize) < kept);
        }
        for m in a.corner_weights.values_mut() {
            m.truncate(kept);
        }
        for m in a.corner_colors.values_mut() {
            m.truncate(kept);
        }
        for m in a.corner_normals.values_mut() {
            m.truncate(kept);
        }
        for m in a.uv_records.values_mut() {
            m.truncate(kept);
        }
        for uses in a.extrude_edge_uses.values_mut() {
            uses.retain(|u| (u[0] as usize) < kept);
        }
        a.extrude_edge_uses.retain(|_, uses| !uses.is_empty());
        if a.smoothing_labels.len() == n {
            a.smoothing_labels.truncate(kept);
        }
        if a.representations.len() == n {
            a.representations.truncate(kept);
        }
        a.degenerate_faces.retain(|&f| (f as usize) < kept);
        Ok(())
    }
    /// Move face `old` to index `new_of_old[old]`, carrying every per-face
    /// and per-corner record and every face reference with it.
    /// `new_of_old` must be a permutation of `0..faces.len()`.
    pub fn permute_faces(&mut self, new_of_old: &[usize]) -> Result<()> {
        let n = self.faces.len();
        let mut check = vec![false; n];
        if new_of_old.len() != n
            || new_of_old
                .iter()
                .any(|&j| j >= n || std::mem::replace(&mut check[j], true))
        {
            return Err("Invalid face permutation".into());
        }
        fn apply<T>(items: &mut Vec<T>, new_of_old: &[usize]) {
            if items.len() != new_of_old.len() {
                return;
            }
            let mut slots: Vec<Option<T>> = (0..items.len()).map(|_| None).collect();
            for (old, item) in items.drain(..).enumerate() {
                slots[new_of_old[old]] = Some(item);
            }
            items.extend(slots.into_iter().flatten());
        }
        let map = |f: u32| new_of_old.get(f as usize).map_or(f, |&j| j as u32);
        apply(&mut self.faces, new_of_old);
        for corners in self.uv_maps.values_mut() {
            apply(corners, new_of_old);
        }
        let a = &mut self.attributes;
        for m in a.typed_maps.values_mut() {
            apply(&mut m.corners, new_of_old);
        }
        for set in a.polygon_sets.values_mut() {
            *set = set.iter().map(|&f| map(f)).collect();
        }
        for m in a.corner_weights.values_mut() {
            apply(m, new_of_old);
        }
        for m in a.corner_colors.values_mut() {
            apply(m, new_of_old);
        }
        for m in a.corner_normals.values_mut() {
            apply(m, new_of_old);
        }
        for m in a.uv_records.values_mut() {
            apply(m, new_of_old);
        }
        for uses in a.extrude_edge_uses.values_mut() {
            for u in uses.iter_mut() {
                u[0] = map(u[0]);
            }
        }
        apply(&mut a.smoothing_labels, new_of_old);
        apply(&mut a.representations, new_of_old);
        a.degenerate_faces = a.degenerate_faces.iter().map(|&f| map(f)).collect();
        Ok(())
    }
}
impl Mesh {
    /// Ear clipping in a dominant-axis projection. Returns corner indices without
    /// altering polygon connectivity or corner attributes. Rejects weak boundaries.
    pub fn triangles(&self, face: usize) -> Result<Vec<[usize; 3]>> {
        self.triangulate(face, true)
    }
    /// Drawing/picking share this projection, including existing nonplanar
    /// primitive quads and depth-preserving grouped polygon edits (IC-B04).
    /// A simple dominant-axis projection with nonzero edges/area and no
    /// crossings is required. Triangles remain transient corner-index views;
    /// rendering, picking and editing use the same surface, never stored edges.
    pub fn surface_triangles(&self, face: usize) -> Result<Vec<[usize; 3]>> {
        self.triangulate(face, false)
    }
    fn triangulate(&self, face: usize, planar: bool) -> Result<Vec<[usize; 3]>> {
        if self.is_polyline(face) {
            return Err("Polyline has no filled surface".into());
        }
        let f = self.faces.get(face).ok_or("Invalid face")?;
        // A keyhole: a merged region with holes, each hole joined to the
        // border by a doubled bridge, so the bridge's points repeat
        // (CMD-P-05). Admitted as a degenerate face; each of its loops was
        // checked simple when it was made.
        let keyhole = f.vertices.len() > 4
            && self.attributes.degenerate_faces.contains(&(face as u32))
            && repeats(&f.vertices);
        if f.vertices.len() < 3
            || f.vertices.len() > 4096
            || f.vertices
                .iter()
                .any(|&v| v as usize >= self.vertices.len())
            || (repeats(&f.vertices) && !keyhole)
        {
            return Err("Invalid/repeated polygon indices or corner budget (4096)".into());
        }
        // Explicitly admitted zero-weight contour endpoints retain their cage
        // quad/corner records. Draw/pick the surviving triangle using original
        // corner indices; this does not alter editable topology or sparse maps.
        if !planar
            && f.vertices.len() == 4
            && self.attributes.degenerate_faces.contains(&(face as u32))
        {
            let mut cs = Vec::new();
            for c in 0..4 {
                if cs.last().is_none_or(|&last| {
                    self.vertices[f.vertices[last] as usize]
                        != self.vertices[f.vertices[c] as usize]
                }) {
                    cs.push(c);
                }
            }
            if cs.len() > 1
                && self.vertices[f.vertices[cs[0]] as usize]
                    == self.vertices[f.vertices[*cs.last().unwrap()] as usize]
            {
                cs.pop();
            }
            if cs.len() == 3 {
                let [a, b, c] =
                    [cs[0], cs[1], cs[2]].map(|i| self.vertices[f.vertices[i] as usize]);
                if (b - a).cross(c - a).length_squared() > 1e-24 {
                    return Ok(vec![[cs[0], cs[1], cs[2]]]);
                }
            }
        }
        let n = self.normal(face);
        if n == DVec3::ZERO {
            return Err("Zero-area polygon".into());
        }
        let origin = self.vertices[f.vertices[0] as usize];
        let scale = f
            .vertices
            .iter()
            .map(|v| (self.vertices[*v as usize] - origin).length())
            .fold(1., f64::max);
        if planar
            && f.vertices
                .iter()
                .any(|v| (self.vertices[*v as usize] - origin).dot(n).abs() > 1e-9 * scale)
        {
            return Err("Nonplanar polygons aren't supported here".into());
        }
        let axis = n.abs().max_element();
        let mut points: Vec<DVec2> = f
            .vertices
            .iter()
            .map(|v| {
                let p = self.vertices[*v as usize] - origin;
                if n.x.abs() == axis {
                    DVec2::new(p.y, p.z)
                } else if n.y.abs() == axis {
                    DVec2::new(p.z, p.x)
                } else {
                    DVec2::new(p.x, p.y)
                }
            })
            .collect();
        // A valid folded corner quad can have an edge parallel to the dropped
        // coordinate axis. Use its normal-plane chart in that case; all consumers
        // share these transient triangles and the same boundary validation.
        if !planar
            && (0..points.len()).any(|i| {
                points[i].distance_squared(points[(i + 1) % points.len()]) <= scale * scale * 1e-12
            })
        {
            let seed = if n.x.abs() < 0.8 { DVec3::X } else { DVec3::Y };
            let u = n.cross(seed).normalize();
            let v = n.cross(u);
            points = f
                .vertices
                .iter()
                .map(|&id| {
                    let d = self.vertices[id as usize] - origin;
                    DVec2::new(d.dot(u), d.dot(v))
                })
                .collect();
        }
        let cross = |a: DVec2, b: DVec2, c: DVec2| (b - a).perp_dot(c - a);
        let eps = scale * scale * 1e-12;
        for i in (0..points.len()).filter(|_| !keyhole) {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            if (a - b).length_squared() <= eps {
                return Err("Zero-length polygon edge".into());
            }
            for j in i + 1..points.len() {
                if j == i + 1 || (i == 0 && j == points.len() - 1) {
                    continue;
                }
                let c = points[j];
                let d = points[(j + 1) % points.len()];
                let ab_c = cross(a, b, c);
                let ab_d = cross(a, b, d);
                let cd_a = cross(c, d, a);
                let cd_b = cross(c, d, b);
                if ab_c * ab_d <= eps * eps
                    && cd_a * cd_b <= eps * eps
                    && a.min(b).cmple(c.max(d) + DVec2::splat(eps)).all()
                    && c.min(d).cmple(a.max(b) + DVec2::splat(eps)).all()
                {
                    return Err("Self-intersecting or weak polygon boundary".into());
                }
            }
        }
        let area = (0..points.len())
            .map(|i| points[i].perp_dot(points[(i + 1) % points.len()]))
            .sum::<f64>();
        if area.abs() <= eps {
            return Err("Zero-area polygon".into());
        }
        let sign = area.signum();
        let mut remaining: Vec<_> = (0..points.len()).collect();
        let mut out = Vec::new();
        let same = |a: usize, b: usize| points[a].distance_squared(points[b]) <= eps;
        while remaining.len() > 3 {
            // A keyhole's bridge leaves zero-width spikes and repeated
            // corners as the ears around it go: drop them, drawing nothing.
            if keyhole {
                let n = remaining.len();
                if let Some(i) = (0..n).find(|&i| {
                    let (a, b, c) = (
                        remaining[(i + n - 1) % n],
                        remaining[i],
                        remaining[(i + 1) % n],
                    );
                    same(a, b) || same(a, c)
                }) {
                    remaining.remove(i);
                    continue;
                }
            }
            let mut found = false;
            for i in 0..remaining.len() {
                let a = remaining[(i + remaining.len() - 1) % remaining.len()];
                let b = remaining[i];
                let c = remaining[(i + 1) % remaining.len()];
                if sign * cross(points[a], points[b], points[c]) <= eps {
                    continue;
                }
                if remaining.iter().any(|&p| {
                    p != a
                        && p != b
                        && p != c
                        // A keyhole's repeated point on the ear's corner.
                        && !(keyhole && (same(p, a) || same(p, b) || same(p, c)))
                        && [
                            cross(points[a], points[b], points[p]),
                            cross(points[b], points[c], points[p]),
                            cross(points[c], points[a], points[p]),
                        ]
                        .iter()
                        .all(|x| sign * x >= -eps)
                }) {
                    continue;
                }
                out.push([a, b, c]);
                remaining.remove(i);
                found = true;
                break;
            }
            if !found {
                return Err("Polygon cannot be triangulated without repair".into());
            }
        }
        out.push([remaining[0], remaining[1], remaining[2]]);
        Ok(out)
    }
}

#[cfg(test)]
mod points_moved_tests {
    use super::*;
    use crate::Shared;

    fn grid(n: u32) -> Mesh {
        let mut mesh = Mesh::default();
        for j in 0..=n {
            for i in 0..=n {
                mesh.vertices.push(DVec3::new(i as f64, j as f64, 0.));
            }
        }
        for j in 0..n {
            for i in 0..n {
                let a = j * (n + 1) + i;
                mesh.faces.push(Face {
                    vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                });
            }
        }
        mesh
    }

    /// Bit-for-bit, across block boundaries: a sign of zero counts, and an
    /// unequal tail block is read.
    #[test]
    fn moved_points_are_found_bit_for_bit() {
        let old: Vec<DVec3> = (0..200).map(|i| DVec3::splat(i as f64)).collect();
        let mut new = old.clone();
        assert!(moved_points(&old, &new).is_empty());
        new[0].x = -0.;
        new[63].y += 1e-12;
        new[64].z = 7.;
        new[199].x = f64::NAN;
        assert_eq!(moved_points(&old, &new), [0, 63, 64, 199]);
    }

    /// A copy whose points alone moved shares the polygons, and the proof
    /// stands in for `validate` only when `validate` would pass: a point
    /// out of range or a polygon made degenerate still fails in full.
    #[test]
    fn moved_points_skip_validation_only_when_it_would_pass() {
        let base = Shared::new(grid(4));
        base.validate().unwrap();
        let mut moved = base.clone();
        moved.vertices[6].z = 0.5;
        assert!(Shared::same(&moved.faces, &base.faces), "faces were copied");
        assert!(moved.prove_points_moved(&base));
        assert!(moved.passed());

        // Out of the coordinate budget, or not a number.
        for bad in [2e9, f64::NAN, f64::INFINITY] {
            let mut m = base.clone();
            m.vertices[6].x = bad;
            assert!(!m.prove_points_moved(&base));
            assert!(m.validate().is_err());
        }
        // Folding a quad flat onto a line makes it degenerate.
        let mut m = base.clone();
        m.vertices[0] = m.vertices[1];
        m.vertices[5] = m.vertices[6];
        assert!(!m.prove_points_moved(&base));
        assert!(m.validate().is_err());

        // Any other change goes through the full check.
        let mut m = base.clone();
        m.faces[0].material = "Other".into();
        assert!(!Shared::same(&m.faces, &base.faces));
        assert!(!m.prove_points_moved(&base));
        let mut m = base.clone();
        m.vertices.push(DVec3::ZERO);
        assert!(!m.prove_points_moved(&base));
        // An unchecked base proves nothing.
        let fresh = Shared::new(grid(4));
        let mut m = fresh.clone();
        m.vertices[6].z = 0.5;
        assert!(!m.prove_points_moved(&fresh));
    }
}
