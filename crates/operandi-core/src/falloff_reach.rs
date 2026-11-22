//! Falloffs whose weights are worked out per point from the selection or a
//! picked element (falloff spec 2026-11-15 batch 2): Soft Selection (FO-47),
//! Coplanar (FO-48) and Element (FO-45). Unlike the other falloffs, Soft
//! Selection, Coplanar and Element reach points outside the selection, so a
//! transform moves those too (`ActiveFalloff::reach`).
//!
//! Each weight list is computed once per drag: the result is remembered
//! while the mesh, its world matrix, the selection and the falloff's
//! options stay the same (Fast drags).
use crate::falloff::FalloffShape;
use crate::{Mesh, Mode, Result, Selection, Symmetry, TweakConnect, TweakGrab, TweakMode};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::sync::Arc;

/// FO-47 Soft Selection: the selection at full weight, fading out over
/// Radius beyond it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoftSelectionFalloff {
    /// How far beyond the selection the effect reaches (scene units).
    pub radius: f64,
    /// Only points connected to the selection count, and distance runs
    /// along the surface (FO-Q05, provisional); off: straight-line.
    pub connectivity: bool,
    pub shape: FalloffShape,
    /// Highlight the affected points / edges while the falloff is shown.
    pub show_vertex: bool,
    pub show_edge: bool,
}
impl Default for SoftSelectionFalloff {
    /// FO-47 (measured): Radius 0.1 m, Use Connectivity on, Linear, Show
    /// Vertex on, Show Edge off.
    fn default() -> Self {
        Self {
            radius: 0.1,
            connectivity: true,
            shape: FalloffShape::Linear,
            show_vertex: true,
            show_edge: false,
        }
    }
}
impl SoftSelectionFalloff {
    pub fn validate(self) -> Result<()> {
        if !(self.radius.is_finite() && self.radius >= 0.) {
            return Err("Soft Selection Radius must be zero or more".into());
        }
        self.shape.validate()
    }
}

/// FO-48 Coplanar: the selection plus neighbouring polygons facing within
/// Angle of it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CoplanarFalloff {
    /// Degrees; 0 takes only polygons facing exactly the same way.
    pub angle: f64,
    pub connectivity: bool,
    pub shape: FalloffShape,
    pub show_vertex: bool,
    pub show_edge: bool,
}
impl Default for CoplanarFalloff {
    /// FO-48 (measured): Angle 0°, Use Connectivity on, Linear, Show Vertex
    /// and Show Edge on.
    fn default() -> Self {
        Self {
            angle: 0.,
            connectivity: true,
            shape: FalloffShape::Linear,
            show_vertex: true,
            show_edge: true,
        }
    }
}
impl CoplanarFalloff {
    pub fn validate(self) -> Result<()> {
        if !(self.angle.is_finite() && (0. ..=180.).contains(&self.angle)) {
            return Err("Coplanar Angle must be from 0° to 180°".into());
        }
        self.shape.validate()
    }
}

/// FO-45 Element: the element under the pointer at full weight, with Range
/// around it. The pick itself is tool context (`Editor::falloff_element`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ElementFalloff {
    pub mode: TweakMode,
    /// World distance; 0 moves only the picked element.
    pub range: f64,
    pub connect: TweakConnect,
    /// Pick on the surface even in wireframe views. Stored; Operandi always
    /// picks on the surface it draws (provisional).
    pub always_raycast: bool,
    pub shape: FalloffShape,
}
impl Default for ElementFalloff {
    /// FO-45: Mode Auto (proposed), Range 0, Ignore, Always Raycast on,
    /// Linear (measured).
    fn default() -> Self {
        Self {
            mode: TweakMode::Auto,
            range: 0.,
            connect: TweakConnect::Ignore,
            always_raycast: true,
            shape: FalloffShape::Linear,
        }
    }
}
impl ElementFalloff {
    pub fn validate(self) -> Result<()> {
        if !(self.range.is_finite() && self.range >= 0.) {
            return Err("Element Range must be zero or more".into());
        }
        self.shape.validate()
    }
}

/// The selection's points, or `None` when nothing is selected (the tool
/// then works on the whole mesh).
fn selected_points(mesh: &Mesh, selection: &Selection) -> Option<Vec<bool>> {
    let n = mesh.vertices.len();
    let mut on = vec![false; n];
    let mut any = false;
    let mut mark = |v: u32| {
        if let Some(f) = on.get_mut(v as usize) {
            *f = true;
            any = true;
        }
    };
    match selection.mode {
        Mode::Item => return None,
        Mode::Vertex => selection.vertices.iter().for_each(|&v| mark(v)),
        Mode::Edge => selection.edges.iter().for_each(|&(a, b)| {
            mark(a);
            mark(b)
        }),
        Mode::Polygon => {
            for &f in &selection.faces {
                if let Some(face) = mesh.faces.get(f as usize) {
                    face.vertices.iter().for_each(|&v| mark(v));
                }
            }
        }
    }
    any.then_some(on)
}

/// The points joined to `v` by a polygon side.
fn neighbours(mesh: &Mesh, faces: &crate::topology_cache::PointFaces, v: u32, out: &mut Vec<u32>) {
    out.clear();
    for &f in faces.of(v) {
        let corners = &mesh.faces[f as usize].vertices;
        let len = corners.len();
        if len < 2 {
            continue;
        }
        let closed = !mesh.is_polyline(f as usize)
            || matches!(
                mesh.attributes.representations.get(f as usize),
                Some(crate::PolygonRepresentation::Polyline { closed: true, .. })
            );
        for (i, &c) in corners.iter().enumerate() {
            if c != v {
                continue;
            }
            if i + 1 < len || closed {
                out.push(corners[(i + 1) % len]);
            }
            if i > 0 || closed {
                out.push(corners[(i + len - 1) % len]);
            }
        }
    }
}

/// FO-47: 1 on the selection; an unselected point at distance d from the
/// nearest selected point weighs Shape(d / Radius), 0 from Radius on. With
/// Use Connectivity d runs along polygon sides from the selection and only
/// connected points count; without, it is the straight distance. Distances
/// are world distances (`world` is the item's matrix).
pub fn soft_selection_weights(
    mesh: &Mesh,
    world: DMat4,
    selection: &Selection,
    f: SoftSelectionFalloff,
) -> Vec<f64> {
    let n = mesh.vertices.len();
    let Some(selected) = selected_points(mesh, selection) else {
        return vec![1.; n];
    };
    let mut weights: Vec<f64> = selected.iter().map(|&s| if s { 1. } else { 0. }).collect();
    let radius = f.radius;
    if radius <= 0. {
        return weights;
    }
    let at = |v: u32| world.transform_point3(mesh.vertices[v as usize]);
    let seeds: Vec<u32> = (0..n as u32).filter(|&v| selected[v as usize]).collect();
    if f.connectivity {
        // Dijkstra along polygon sides from every selected point, stopping
        // at Radius, so only the selection's neighbourhood is read.
        let faces = mesh.point_faces();
        let mut dist = vec![f64::INFINITY; n];
        let mut heap = std::collections::BinaryHeap::new();
        for &v in &seeds {
            dist[v as usize] = 0.;
            heap.push(std::cmp::Reverse((OrdF64(0.), v)));
        }
        let mut around = Vec::new();
        // A max-heap of `Reverse`: nearest first.
        while let Some(std::cmp::Reverse((OrdF64(d), v))) = heap.pop() {
            if d > dist[v as usize] {
                continue;
            }
            let p = at(v);
            neighbours(mesh, &faces, v, &mut around);
            for &u in &around {
                let next = d + p.distance(at(u));
                if next < radius && next < dist[u as usize] {
                    dist[u as usize] = next;
                    heap.push(std::cmp::Reverse((OrdF64(next), u)));
                }
            }
        }
        for (v, &d) in dist.iter().enumerate() {
            if !selected[v] && d < radius {
                weights[v] = f.shape.weight(d / radius);
            }
        }
    } else {
        // A grid of Radius-sized cells holding the selected points: each
        // point looks only at its own and the 26 cells around it.
        let cell = |p: DVec3| (p / radius).floor().as_ivec3();
        let mut grid: crate::fxhash::FxHashMap<glam::IVec3, Vec<DVec3>> = Default::default();
        let (mut lo, mut hi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
        for &v in &seeds {
            let p = at(v);
            lo = lo.min(p);
            hi = hi.max(p);
            grid.entry(cell(p)).or_default().push(p);
        }
        let (lo, hi) = (lo - DVec3::splat(radius), hi + DVec3::splat(radius));
        for v in 0..n {
            if selected[v] {
                continue;
            }
            let p = at(v as u32);
            if p.cmplt(lo).any() || p.cmpgt(hi).any() {
                continue;
            }
            let c = cell(p);
            let mut best = f64::INFINITY;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if let Some(points) = grid.get(&(c + glam::IVec3::new(dx, dy, dz))) {
                            for q in points {
                                best = best.min(q.distance_squared(p));
                            }
                        }
                    }
                }
            }
            let d = best.sqrt();
            if d < radius {
                weights[v] = f.shape.weight(d / radius);
            }
        }
    }
    weights
}

/// A distance the heap can order (`total_cmp`).
struct OrdF64(f64);
impl PartialEq for OrdF64 {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl Eq for OrdF64 {}
impl PartialOrd for OrdF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrdF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// The world-space unit normal of polygon `f` (Newell's method), or zero
/// for a polygon with no area.
fn face_normal(mesh: &Mesh, world: DMat4, f: usize) -> DVec3 {
    let corners = &mesh.faces[f].vertices;
    let mut n = DVec3::ZERO;
    for i in 0..corners.len() {
        let a = world.transform_point3(mesh.vertices[corners[i] as usize]);
        let b = world.transform_point3(mesh.vertices[corners[(i + 1) % corners.len()] as usize]);
        n += a.cross(b);
    }
    n.normalize_or_zero()
}

/// Angles at or below this count as facing the same way (Angle 0).
const COPLANAR_TOLERANCE_DEGREES: f64 = 0.01;

/// FO-48 (weight ramp provisional, FO-Q06): the selection's polygons set a
/// facing (their summed normals). Polygons facing within Angle of it take
/// part, spreading from the selection across shared sides with Use
/// Connectivity, or anywhere on the mesh without. A taking-part polygon
/// weighs Shape(its angle / Angle) (1 at Angle 0); a point takes the most
/// of its polygons; selected points weigh 1. The selection's polygons are
/// the selected ones in Polygon mode, otherwise those whose corners are all
/// selected.
pub fn coplanar_weights(
    mesh: &Mesh,
    world: DMat4,
    selection: &Selection,
    f: CoplanarFalloff,
) -> Vec<f64> {
    let n = mesh.vertices.len();
    let Some(selected) = selected_points(mesh, selection) else {
        return vec![1.; n];
    };
    let mut weights: Vec<f64> = selected.iter().map(|&s| if s { 1. } else { 0. }).collect();
    let polygon = |i: usize| mesh.faces[i].vertices.len() >= 3 && !mesh.is_polyline(i);
    let seeds: Vec<usize> = if selection.mode == Mode::Polygon {
        selection
            .faces
            .iter()
            .map(|&f| f as usize)
            .filter(|&f| f < mesh.faces.len() && polygon(f))
            .collect()
    } else {
        // Only polygons next to the selection can qualify.
        let faces = mesh.point_faces();
        let mut out: Vec<usize> = (0..n as u32)
            .filter(|&v| selected[v as usize])
            .flat_map(|v| faces.of(v).iter().map(|&f| f as usize))
            .filter(|&f| polygon(f) && mesh.faces[f].vertices.iter().all(|&v| selected[v as usize]))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    };
    let facing = seeds
        .iter()
        .map(|&f| {
            let corners = &mesh.faces[f].vertices;
            let mut sum = DVec3::ZERO;
            for i in 0..corners.len() {
                let a = world.transform_point3(mesh.vertices[corners[i] as usize]);
                let b = world
                    .transform_point3(mesh.vertices[corners[(i + 1) % corners.len()] as usize]);
                sum += a.cross(b);
            }
            sum
        })
        .sum::<DVec3>()
        .normalize_or_zero();
    if facing == DVec3::ZERO {
        return weights;
    }
    let limit = f.angle.max(COPLANAR_TOLERANCE_DEGREES);
    let face_weight = |i: usize| -> Option<f64> {
        if !polygon(i) {
            return None;
        }
        let normal = face_normal(mesh, world, i);
        if normal == DVec3::ZERO {
            return None;
        }
        let angle = normal.dot(facing).clamp(-1., 1.).acos().to_degrees();
        (angle <= limit).then(|| {
            if f.angle <= 0. {
                1.
            } else {
                f.shape.weight(angle / f.angle)
            }
        })
    };
    let give = |i: usize, w: f64, weights: &mut Vec<f64>| {
        for &v in &mesh.faces[i].vertices {
            let slot = &mut weights[v as usize];
            *slot = slot.max(w);
        }
    };
    if f.connectivity {
        let faces = mesh.point_faces();
        let mut seen = vec![false; mesh.faces.len()];
        let mut queue = std::collections::VecDeque::new();
        for &s in &seeds {
            seen[s] = true;
            queue.push_back(s);
        }
        while let Some(i) = queue.pop_front() {
            let corners = &mesh.faces[i].vertices;
            for k in 0..corners.len() {
                let (a, b) = (corners[k], corners[(k + 1) % corners.len()]);
                for &g in faces.of(a) {
                    let g = g as usize;
                    if seen[g] || !mesh.faces[g].vertices.contains(&b) {
                        continue;
                    }
                    seen[g] = true;
                    if let Some(w) = face_weight(g) {
                        give(g, w, &mut weights);
                        queue.push_back(g);
                    }
                }
            }
        }
    } else {
        for i in 0..mesh.faces.len() {
            if let Some(w) = face_weight(i) {
                give(i, w, &mut weights);
            }
        }
    }
    for (v, &s) in selected.iter().enumerate() {
        if s {
            weights[v] = 1.;
        }
    }
    weights
}

/// FO-45: weight 1 on the picked element's points (with Edge Loops and an
/// edge pick, its loop's; with Symmetry, also their counterparts), Range
/// around it with the Shape, as Tweak works it out (`tweak_plan`). Every
/// other point weighs 0, and so does the whole mesh before anything is
/// picked or when the pick no longer exists.
pub fn element_weights(
    mesh: &Mesh,
    world: DMat4,
    pick: Option<TweakGrab>,
    f: ElementFalloff,
    symmetry: Symmetry,
) -> Vec<f64> {
    let mut weights = vec![0.; mesh.vertices.len()];
    let Some(grab) = pick else {
        return weights;
    };
    let options = crate::TweakOptions {
        mode: f.mode,
        range: f.range,
        connect: f.connect,
        shape: f.shape,
    };
    if let Ok(plan) = crate::tweak_plan(mesh, world, grab, options, symmetry) {
        for &(v, w) in plan.moves.iter() {
            weights[v as usize] = w;
        }
    }
    weights
}

/// What one remembered weight list was worked out from.
#[derive(Clone, PartialEq)]
pub(crate) enum ReachKey {
    Soft(SoftSelectionFalloff),
    Coplanar(CoplanarFalloff),
    Element(ElementFalloff, Option<TweakGrab>, Symmetry),
    Path(crate::falloff_path::PathFalloff, Vec<DVec3>),
    Curvature(crate::falloff_path::CurvatureFalloff),
}

struct ReachMemo {
    faces: crate::shared::WeakShared<Vec<crate::Face>>,
    positions: Vec<DVec3>,
    world: DMat4,
    selection: Selection,
    key: ReachKey,
    weights: Arc<Vec<f64>>,
}

thread_local! {
    /// The last few results: a drag reads the same base mesh every frame,
    /// and a stack may hold more than one of these falloffs.
    static MEMO: RefCell<Vec<ReachMemo>> = const { RefCell::new(Vec::new()) };
}
const MEMO_SIZE: usize = 4;

/// The weights `key` gives `mesh` (an item with matrix `world` and its own
/// `selection`), remembered while nothing they read changes.
pub(crate) fn reach_weights(
    mesh: &Mesh,
    world: DMat4,
    selection: &Selection,
    key: ReachKey,
) -> Arc<Vec<f64>> {
    let same_selection = |s: &Selection| {
        s.mode == selection.mode
            && s.vertices == selection.vertices
            && s.edges == selection.edges
            && s.faces == selection.faces
    };
    let hit = MEMO.with_borrow(|memo| {
        memo.iter()
            .find(|m| {
                m.key == key
                    && m.world == world
                    && m.faces.is(&mesh.faces)
                    && m.positions == mesh.vertices
                    && same_selection(&m.selection)
            })
            .map(|m| m.weights.clone())
    });
    if let Some(w) = hit {
        return w;
    }
    let weights = Arc::new(match key.clone() {
        ReachKey::Soft(f) => soft_selection_weights(mesh, world, selection, f),
        ReachKey::Coplanar(f) => coplanar_weights(mesh, world, selection, f),
        ReachKey::Element(f, pick, symmetry) => element_weights(mesh, world, pick, f, symmetry),
        ReachKey::Path(f, controls) => crate::falloff_path::path_weights(mesh, world, f, &controls),
        ReachKey::Curvature(f) => crate::falloff_path::curvature_weights(mesh, f),
    });
    MEMO.with_borrow_mut(|memo| {
        if memo.len() >= MEMO_SIZE {
            memo.remove(0);
        }
        memo.push(ReachMemo {
            faces: mesh.faces.downgrade(),
            positions: mesh.vertices.clone(),
            world,
            selection: selection.clone(),
            key,
            weights: weights.clone(),
        });
    });
    weights
}
