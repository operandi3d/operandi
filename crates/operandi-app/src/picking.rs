//! Immutable surface shared by display and component queries.
//! Scene-equal snapshots may reuse triangulation and the bounds tree; camera
//! and pointer are supplied to each query and are never stored in the cache.
use crate::{Camera, DVec3, Pos2, Rect, Scene, camera};
use std::collections::BTreeMap;

type Hit = (u64, u32, f64);

#[derive(Clone, Copy)]
struct Bounds {
    min: DVec3,
    max: DVec3,
}
impl Bounds {
    fn empty() -> Self {
        Self {
            min: DVec3::splat(f64::INFINITY),
            max: DVec3::splat(f64::NEG_INFINITY),
        }
    }
    fn include(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
    fn entry(self, origin: DVec3, direction: DVec3, limit: f64) -> Option<f64> {
        let mut near: f64 = 0.;
        let mut far = limit;
        for axis in 0..3 {
            // A conservative expansion keeps rays on shared edges eligible.
            let pad = 1e-10 * self.min[axis].abs().max(self.max[axis].abs()).max(1.);
            let lo = self.min[axis] - pad;
            let hi = self.max[axis] + pad;
            if direction[axis] == 0. {
                if origin[axis] < lo || origin[axis] > hi {
                    return None;
                }
            } else {
                let a = (lo - origin[axis]) / direction[axis];
                let b = (hi - origin[axis]) / direction[axis];
                near = near.max(a.min(b));
                far = far.min(a.max(b));
                if near > far {
                    return None;
                }
            }
        }
        Some(near)
    }
}
struct Triangle {
    points: [DVec3; 3],
    bounds: Bounds,
    item: u64,
    face: u32,
    order: usize,
}
enum Children {
    Leaf(std::ops::Range<usize>),
    Branch(usize, usize),
}
struct Node {
    bounds: Bounds,
    children: Children,
}
#[derive(Default)]
pub(super) struct Surface {
    corners: BTreeMap<(u64, usize), Vec<[usize; 3]>>,
    triangles: Vec<Triangle>,
    lines: Vec<(u64, u32, [DVec3; 2])>,
    nodes: Vec<Node>,
}
/// Reuse while the scene's items are equal: the surface depends only on
/// their meshes, placements and visibility, never on the selection, so
/// picking a point or polygon doesn't rebuild it (that rebuild was a long
/// stall on large meshes). Shared mesh handles make unchanged comparisons
/// cheap; copy-on-write keeps the stored snapshot immutable. Only items
/// with geometry and their parents count (`picked_items`): moving a camera
/// or light with nothing under it (Camera view navigation) keeps it.
#[derive(Default)]
pub(super) struct SurfaceCache {
    // Interior mutability so click handlers (which borrow the app
    // immutably) share it instead of building a fresh surface per click.
    entry: std::cell::RefCell<Option<(Vec<operandi_core::Item>, std::sync::Arc<Surface>)>>,
    /// The items the rollover last saw (`settled`).
    rollover_seen: std::cell::RefCell<Option<Vec<operandi_core::Item>>>,
    /// Surfaces built so far (speed guards, Window > Frame Time).
    builds: std::cell::Cell<u64>,
}
impl SurfaceCache {
    pub fn builds(&self) -> u64 {
        self.builds.get()
    }
    /// Whether the rollover may pick this frame: the surface is current,
    /// or the scene's items are the same as on the rollover's last frame.
    /// While something changes the scene every frame without a pointer
    /// drag (a numeric scrub, typed values, the MCP bridge), the rollover
    /// waits instead of rebuilding the whole surface each frame; it picks
    /// again on the first frame the scene holds still.
    pub fn settled(&self, scene: &Scene) -> bool {
        let current = self
            .entry
            .borrow()
            .as_ref()
            .is_some_and(|(items, _)| same_picked(items, scene));
        let mut seen = self.rollover_seen.borrow_mut();
        let still = seen.as_ref().is_some_and(|items| same_picked(items, scene));
        if !still {
            *seen = Some(picked_items(scene));
        }
        current || still
    }
    /// Smooth polygons pick and hide on their evaluated surface from
    /// `smooth` (the one the viewport draws), not on the cage.
    pub fn get(
        &self,
        scene: &Scene,
        smooth: &crate::subdivision_view::SubdivisionCache,
    ) -> std::sync::Arc<Surface> {
        if let Some((items, surface)) = &*self.entry.borrow()
            && same_picked(items, scene)
        {
            return surface.clone();
        }
        self.builds.set(self.builds.get() + 1);
        let surface = std::sync::Arc::new(Surface::with_smooth(scene, Some(smooth)));
        *self.entry.borrow_mut() = Some((picked_items(scene), surface.clone()));
        surface
    }
}

/// Which of `scene`'s items the picking surface depends on: those with
/// points (meshes, curves) and every parent above them (their placement
/// and visibility pass down). In scene order.
fn picks_from(scene: &Scene) -> Vec<bool> {
    let index: std::collections::HashMap<u64, usize> = scene
        .items
        .iter()
        .enumerate()
        .map(|(k, i)| (i.id, k))
        .collect();
    let mut keep = vec![false; scene.items.len()];
    for (k, item) in scene.items.iter().enumerate() {
        if item.mesh.vertices.is_empty() || keep[k] {
            continue;
        }
        keep[k] = true;
        // Walk up until a parent already kept (or a loop, or the top).
        let mut parent = item.parent;
        while let Some(&p) = parent.and_then(|id| index.get(&id)) {
            if keep[p] {
                break;
            }
            keep[p] = true;
            parent = scene.items[p].parent;
        }
    }
    keep
}
fn picked_items(scene: &Scene) -> Vec<operandi_core::Item> {
    let keep = picks_from(scene);
    scene
        .items
        .iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(i, _)| i.clone())
        .collect()
}
/// Whether `stored` (from `picked_items`) still matches `scene`, without
/// copying its items.
fn same_picked(stored: &[operandi_core::Item], scene: &Scene) -> bool {
    let keep = picks_from(scene);
    let mut now = scene
        .items
        .iter()
        .zip(keep)
        .filter_map(|(i, k)| k.then_some(i));
    stored.iter().all(|s| now.next() == Some(s)) && now.next().is_none()
}

impl crate::Operandi {
    /// The surface viewport picking and snapping read: the evaluated
    /// scene's Pick view (AR-01, `operandi_core::evaluate`), which is the
    /// document scene until picking can map results back to Base Mesh
    /// elements (AR-04).
    pub(crate) fn pick_surface(&self) -> std::sync::Arc<Surface> {
        let scene = self
            .editor
            .evaluated(operandi_core::evaluate::Purpose::Pick);
        self.surface_cache.get(&scene, &self.subdivision_cache)
    }
}

/// An item filter for `Surface::pick_in` / `visible_in`: `None` (every
/// item) when `items` is empty.
pub(crate) fn limit(items: &[u64]) -> Option<&[u64]> {
    (!items.is_empty()).then_some(items)
}
/// Convert a screen-space span fraction to its world-space fraction.
pub(super) fn span_world_parameter(camera: &Camera, t: f32, za: f64, zb: f64) -> f64 {
    let t = t as f64;
    if camera.view == crate::camera::View::Perspective {
        (t / zb) / ((1. - t) / za + t / zb)
    } else {
        t
    }
}
impl std::fmt::Debug for Surface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Surface({} triangles)", self.triangles.len())
    }
}
impl Surface {
    #[cfg(test)]
    pub fn new(scene: &Scene) -> Self {
        Self::with_smooth(scene, None)
    }
    pub fn with_smooth(
        scene: &Scene,
        smooth: Option<&crate::subdivision_view::SubdivisionCache>,
    ) -> Self {
        let mut out = Self::default();
        // CP-D01: items Display Visible hides aren't drawn, so they neither
        // pick nor hide what's behind them.
        let display_hidden = scene.display_hidden();
        for item in scene
            .items
            .iter()
            .filter(|item| item.visible && !display_hidden.contains(&item.id))
        {
            let Ok(matrix) = scene.world_matrix(item.id) else {
                continue;
            };
            let world: Vec<_> = item
                .mesh
                .vertices
                .iter()
                .map(|&v| matrix.transform_point3(v))
                .collect();
            // Cage polygons drawn as smooth surface: their triangles come
            // from the evaluated pieces (a fan per piece), so picking and
            // hiding follow what is drawn, not the invisible cage.
            let evaluated = smooth.and_then(|cache| cache.smooth(item));
            // HID-03: hidden polygons aren't drawn, so they don't pick or
            // hide what is behind them. Marking them covered skips both
            // loops below.
            let mut covered = (!item.elements.hidden.is_empty())
                .then(|| operandi_core::Protection::of(&item.mesh, &item.elements))
                .flatten()
                .map(|p| p.face_hidden)
                .unwrap_or_else(|| vec![false; item.mesh.faces.len()]);
            let hidden = covered.clone();
            if let Some(surface) = &evaluated {
                let piece_world: Vec<_> = surface
                    .mesh
                    .vertices
                    .iter()
                    .map(|&v| matrix.transform_point3(v))
                    .collect();
                for (piece, face) in surface.mesh.faces.iter().enumerate() {
                    let Some(&source) = surface.sources.get(piece) else {
                        continue;
                    };
                    if source >= covered.len()
                        || hidden[source]
                        || !operandi_core::subdivision::is_smooth_polygon(&item.mesh, source)
                    {
                        continue;
                    }
                    covered[source] = true;
                    for i in 1..face.vertices.len().saturating_sub(1) {
                        let points = [0, i, i + 1].map(|k| piece_world[face.vertices[k] as usize]);
                        let bounds = points.iter().fold(Bounds::empty(), |b, &p| {
                            b.include(Bounds { min: p, max: p })
                        });
                        out.triangles.push(Triangle {
                            points,
                            bounds,
                            item: item.id,
                            face: source as u32,
                            order: out.triangles.len(),
                        });
                    }
                }
            }
            for (fi, face) in item.mesh.faces.iter().enumerate() {
                if hidden[fi] {
                    continue;
                }
                if item.mesh.is_polyline(fi) {
                    out.lines.extend(item.mesh.element_spans(fi).map(|(a, b)| {
                        (item.id, fi as u32, [world[a as usize], world[b as usize]])
                    }));
                    continue;
                }
                // A polygon the ear clipper refuses (a self-crossing outline
                // kept from an import) still draws and picks, as a fan.
                let corners = item.mesh.surface_triangles(fi).unwrap_or_else(|_| {
                    (1..face.vertices.len().saturating_sub(1))
                        .map(|i| [0, i, i + 1])
                        .collect()
                });
                for triangle in corners.iter().filter(|_| !covered[fi]) {
                    let points = triangle.map(|i| world[face.vertices[i] as usize]);
                    let bounds = points.iter().fold(Bounds::empty(), |b, &p| {
                        b.include(Bounds { min: p, max: p })
                    });
                    out.triangles.push(Triangle {
                        points,
                        bounds,
                        item: item.id,
                        face: fi as u32,
                        order: out.triangles.len(),
                    });
                }
                out.corners.insert((item.id, fi), corners);
            }
        }
        if !out.triangles.is_empty() {
            out.build(0, out.triangles.len());
        }
        out
    }
    pub fn corners(&self, item: u64, face: usize) -> &[[usize; 3]] {
        self.corners.get(&(item, face)).map_or(&[], Vec::as_slice)
    }
    fn build(&mut self, start: usize, end: usize) -> usize {
        let bounds = self.triangles[start..end]
            .iter()
            .fold(Bounds::empty(), |b, t| b.include(t.bounds));
        let node = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            children: Children::Leaf(start..end),
        });
        if end - start > 8 {
            let extent = bounds.max - bounds.min;
            let axis = if extent.x >= extent.y && extent.x >= extent.z {
                0
            } else if extent.y >= extent.z {
                1
            } else {
                2
            };
            let mid = start + (end - start) / 2;
            self.triangles[start..end].select_nth_unstable_by(mid - start, |a, b| {
                (a.bounds.min[axis] + a.bounds.max[axis])
                    .total_cmp(&(b.bounds.min[axis] + b.bounds.max[axis]))
                    .then(a.order.cmp(&b.order))
            });
            let left = self.build(start, mid);
            let right = self.build(mid, end);
            self.nodes[node].children = Children::Branch(left, right);
        }
        node
    }
    #[allow(clippy::too_many_arguments)] // recursive walk state
    fn visit(
        &self,
        node: usize,
        entry: f64,
        o: DVec3,
        d: DVec3,
        hit: &mut Option<(Hit, usize)>,
        tests: &mut usize,
        only: Option<&[u64]>,
    ) {
        let limit = hit.map_or(f64::INFINITY, |((_, _, t), _)| t);
        // The parent already intersected this bound to order its children.
        // Reuse that entry distance; only the nearest-hit limit can shrink
        // while visiting the first sibling. Equality still admits stable ties.
        if entry > limit {
            return;
        }
        match &self.nodes[node].children {
            Children::Leaf(range) => {
                for triangle in &self.triangles[range.clone()] {
                    // An item filter skips other items' triangles: the ray
                    // passes through them. The BVH is shared, so filtering
                    // costs nothing but those skipped tests.
                    if only.is_some_and(|only| !only.contains(&triangle.item)) {
                        continue;
                    }
                    *tests += 1;
                    let [a, b, c] = triangle.points;
                    if let Some(t) = camera::triangle_hit(o, d, a, b, c)
                        && hit.is_none_or(|((_, _, old), order)| {
                            t < old || (t == old && triangle.order < order)
                        })
                    {
                        *hit = Some(((triangle.item, triangle.face, t), triangle.order));
                    }
                }
            }
            Children::Branch(a, b) => {
                let first = self.nodes[*a].bounds.entry(o, d, limit);
                let second = self.nodes[*b].bounds.entry(o, d, limit);
                let children = if first.unwrap_or(f64::INFINITY) <= second.unwrap_or(f64::INFINITY)
                {
                    [(*a, first), (*b, second)]
                } else {
                    [(*b, second), (*a, first)]
                };
                for (child, entry) in children {
                    if let Some(entry) = entry {
                        self.visit(child, entry, o, d, hit, tests, only);
                    }
                }
            }
        }
    }
    #[cfg(test)]
    fn query(&self, o: DVec3, d: DVec3) -> (Option<Hit>, usize) {
        self.query_in(o, d, None)
    }
    /// `query` seeing only `only`'s triangles when it is set.
    fn query_in(&self, o: DVec3, d: DVec3, only: Option<&[u64]>) -> (Option<Hit>, usize) {
        let mut hit = None;
        let mut tests = 0;
        if let Some(root) = self.nodes.first()
            && let Some(entry) = root.bounds.entry(o, d, f64::INFINITY)
        {
            self.visit(0, entry, o, d, &mut hit, &mut tests, only);
        }
        (hit.map(|(h, _)| h), tests)
    }
    pub fn pick(&self, camera: &Camera, p: Pos2, r: Rect) -> Option<Hit> {
        self.pick_in(camera, p, r, None)
    }
    /// Every polygon the segment `a`..`b` passes through, strictly between
    /// its ends: (item, face, fraction along the segment). Intersection
    /// snapping (SN2-31) reads it.
    pub fn segment_hits(&self, a: DVec3, b: DVec3) -> Vec<(u64, u32, f64)> {
        let length = (b - a).length();
        let mut out = Vec::new();
        if length < 1e-12 {
            return out;
        }
        let d = (b - a) / length;
        let mut stack = vec![0usize];
        while let Some(node) = stack.pop() {
            let Some(n) = self.nodes.get(node) else {
                continue;
            };
            if n.bounds.entry(a, d, length).is_none() {
                continue;
            }
            match &n.children {
                Children::Leaf(range) => {
                    for t in &self.triangles[range.clone()] {
                        let [p, q, r] = t.points;
                        // Either side: an edge may pass through from behind.
                        let hit = camera::triangle_hit(a, d, p, q, r)
                            .or_else(|| camera::triangle_hit(a, d, p, r, q));
                        if let Some(at) = hit
                            && at > length * 1e-6
                            && at < length * (1. - 1e-6)
                        {
                            out.push((t.item, t.face, at / length));
                        }
                    }
                }
                Children::Branch(x, y) => {
                    stack.push(*x);
                    stack.push(*y);
                }
            }
        }
        out.sort_by(|x, y| x.2.total_cmp(&y.2));
        out.dedup_by(|x, y| x.0 == y.0 && x.1 == y.1);
        out
    }
    /// The polygon under `p` (item, face, distance along the pointer's
    /// ray), ignoring curves and polylines: what Polygon snapping lands on.
    pub fn polygon_under(&self, camera: &Camera, p: Pos2, r: Rect) -> Option<Hit> {
        let (o, d) = camera.ray(p, r);
        self.query_in(o, d, None).0
    }
    /// `pick` limited to the items in `only` when it is set: other items neither
    /// pick nor hide it (Pick Other Meshes in Component Modes off).
    pub fn pick_in(&self, camera: &Camera, p: Pos2, r: Rect, only: Option<&[u64]>) -> Option<Hit> {
        let (o, d) = camera.ray(p, r);
        let hit = self.query_in(o, d, only).0;
        let mut line: Option<(f32, Hit)> = None;
        for &(item, element, [a, b]) in &self.lines {
            if only.is_some_and(|only| !only.contains(&item)) {
                continue;
            }
            if let (Some((qa, za)), Some((qb, zb))) = (camera.project(a, r), camera.project(b, r)) {
                let delta = qb - qa;
                let t = ((p - qa).dot(delta) / delta.length_sq().max(1e-8)).clamp(0., 1.);
                // Screen interpolation is perspective divided. Recover the
                // geometric span parameter before comparing against surface depth.
                let t = span_world_parameter(camera, t, za, zb);
                let world = a.lerp(b, t);
                let depth = (world - o).dot(d);
                let distance = crate::input::segment_distance(p, qa, qb);
                if distance < 12.
                    && depth > 0.
                    && self.visible_in(camera, world, r, only)
                    && hit.is_none_or(|h| depth <= h.2 + 1e-5)
                    && line
                        .is_none_or(|(old, h)| distance < old || (distance == old && depth < h.2))
                {
                    // Match direct edge picking: nearest screen span first,
                    // then depth. A neighboring span near a corner must not
                    // replace the exact projected hit with a shallower endpoint.
                    line = Some((distance, (item, element, depth)));
                }
            }
        }
        line.map(|(_, hit)| hit).or(hit)
    }
    pub fn visible(&self, camera: &Camera, world: DVec3, r: Rect) -> bool {
        self.visible_in(camera, world, r, None)
    }
    /// `visible` with only `only`'s surfaces able to hide `world` when
    /// it is set.
    pub fn visible_in(&self, camera: &Camera, world: DVec3, r: Rect, only: Option<&[u64]>) -> bool {
        if camera.project(world, r).is_none() {
            return false;
        }
        // Aim the ray at `world` itself. Going through its screen position
        // (f32) moved the ray off the point by the rounding, and on a
        // surface seen at an angle a neighbouring polygon then lay in front
        // by more than the tolerance: points hid behind their own polygons,
        // more often the larger the scene (the lasso missed points on a
        // 10 m sphere). The tolerance also grows with depth for that reason.
        let (eye, _, _, forward) = camera.basis();
        let (o, d) = if camera.view == crate::camera::View::Perspective {
            (eye, (world - eye).normalize())
        } else {
            // From the front of the drawn slab (`Camera::ortho_depth`), as
            // `Camera::ray` does, so surfaces the zoomed-in eye has passed
            // still hide what lies behind them.
            let front = (world - eye).dot(forward) + camera.ortho_depth().0;
            (world - forward * front, forward)
        };
        let depth = (world - o).dot(d);
        let tolerance = 1e-5_f64.max(depth.abs() * 1e-6);
        self.query_in(o, d, only)
            .0
            .is_none_or(|(_, _, t)| t + tolerance >= depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mode, View, pos2, vec2};
    use operandi_core::{Face, Mesh, RadialParameters};

    fn sphere(sides: u32, segments: u32) -> Scene {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh = operandi_core::radial::globe_geometry(
            DVec3::ZERO,
            RadialParameters {
                radius: DVec3::ONE,
                sides,
                segments,
            },
        )
        .unwrap()
        .into();
        scene
    }
    // Independent exhaustive traversal retains the original scene/face/corner
    // ordering and sidedness, without bounds pruning or reordered triangles.
    fn reference(scene: &Scene) -> Vec<(u64, u32, [DVec3; 3])> {
        scene
            .items
            .iter()
            .filter(|i| i.visible)
            .flat_map(|i| {
                i.mesh.faces.iter().enumerate().flat_map(move |(fi, f)| {
                    // Same fan fallback as `Surface::new` for refused shapes.
                    i.mesh
                        .surface_triangles(fi)
                        .unwrap_or_else(|_| {
                            (1..f.vertices.len().saturating_sub(1))
                                .map(|k| [0, k, k + 1])
                                .collect()
                        })
                        .into_iter()
                        .map(move |tri| {
                            (
                                i.id,
                                fi as u32,
                                tri.map(|c| {
                                    i.transform.point(i.mesh.vertices[f.vertices[c] as usize])
                                }),
                            )
                        })
                })
            })
            .collect()
    }
    fn brute(triangles: &[(u64, u32, [DVec3; 3])], o: DVec3, d: DVec3) -> Option<Hit> {
        let mut hit = None;
        for &(item, face, [a, b, c]) in triangles {
            if let Some(t) = camera::triangle_hit(o, d, a, b, c)
                && hit.is_none_or(|(_, _, old)| t < old)
            {
                hit = Some((item, face, t));
            }
        }
        hit
    }
    fn assert_cached_matches_fresh(cache: &mut SurfaceCache, scene: &Scene) {
        let actual = cache.get(scene, &Default::default());
        let fresh = Surface::new(scene);
        assert_eq!(actual.corners, fresh.corners);
        let triangles = |s: &Surface| {
            let mut out: Vec<_> = s
                .triangles
                .iter()
                .map(|t| (t.order, t.item, t.face, t.points))
                .collect();
            out.sort_by_key(|x| x.0);
            out
        };
        assert_eq!(triangles(&actual), triangles(&fresh));
        assert_eq!(actual.lines, fresh.lines);
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(700., 700.));
        for view in [View::Perspective, View::Front, View::Top] {
            let camera = Camera {
                view,
                ..Default::default()
            };
            for point in [pos2(100., 100.), pos2(350., 350.), pos2(500., 320.)] {
                assert_eq!(
                    actual.pick(&camera, point, rect),
                    fresh.pick(&camera, point, rect)
                );
            }
            for point in [DVec3::ZERO, DVec3::new(0.3, 0.2, 0.)] {
                assert_eq!(
                    actual.visible(&camera, point, rect),
                    fresh.visible(&camera, point, rect)
                );
            }
        }
    }

    #[test]
    fn cache_matches_fresh_after_scene_mutations_and_history() {
        use crate::{Command, Editor, Parameters, Tool};
        let mut scene = sphere(8, 4);
        scene.new_mesh().unwrap();
        scene.items[1].parent = Some(1);
        scene.items[0].transform.rotation = DVec3::new(0.2, 0.4, 0.1);
        scene.items[0].transform.scale = DVec3::new(2., 0.5, 1.5);
        scene.items[1].mesh = scene.items[0].mesh.clone();
        scene.items[1].transform.rotation = DVec3::new(-0.3, 0.1, 0.2);
        scene.items[1].transform.scale = DVec3::new(0.8, 1.4, 0.6);
        scene.new_mesh().unwrap();
        scene.items[2].parent = Some(2);
        scene.items[2].mesh = scene.items[0].mesh.clone();
        scene.items[2].transform.rotation = DVec3::new(0.1, -0.2, 0.3);
        scene.items[2].transform.scale = DVec3::new(1.1, 0.7, 1.3);
        let mut cache = SurfaceCache::default();
        assert_cached_matches_fresh(&mut cache, &scene);
        let cached = cache.get(&scene, &Default::default());
        assert!(std::sync::Arc::ptr_eq(
            &cached,
            &cache.get(&scene, &Default::default())
        ));
        scene.items[0].visible = false; // hidden parent, visible child
        assert_cached_matches_fresh(&mut cache, &scene);
        scene.items[1].visible = false;
        assert_cached_matches_fresh(&mut cache, &scene);
        scene.items[1].visible = true;
        scene.items[1].mesh.vertices[0].x += 0.2; // COW mutation
        assert_cached_matches_fresh(&mut cache, &scene);
        scene.items[0].transform.translation.x = 2.;
        assert_cached_matches_fresh(&mut cache, &scene);
        scene.items[1].parent = None;
        assert_cached_matches_fresh(&mut cache, &scene);
        scene.items[1].parent = Some(2); // invalid hierarchy is skipped, as fresh
        assert_cached_matches_fresh(&mut cache, &scene);
        scene = sphere(6, 3); // document replacement
        assert_cached_matches_fresh(&mut cache, &scene);
        let mut editor = Editor {
            scene,
            ..Default::default()
        };
        editor.dispatch(Command::Activate(Tool::Box)).unwrap();
        editor
            .dispatch(Command::Preview(Parameters {
                size: DVec3::splat(1.),
                ..Default::default()
            }))
            .unwrap();
        assert_cached_matches_fresh(&mut cache, &editor.scene);
        editor
            .dispatch(Command::Preview(Parameters {
                size: DVec3::splat(2.),
                ..Default::default()
            }))
            .unwrap();
        assert_cached_matches_fresh(&mut cache, &editor.scene);
        editor.dispatch(Command::Cancel).unwrap();
        assert_cached_matches_fresh(&mut cache, &editor.scene);
        editor.dispatch(Command::Activate(Tool::Box)).unwrap();
        editor
            .dispatch(Command::Preview(Parameters {
                size: DVec3::splat(1.),
                ..Default::default()
            }))
            .unwrap();
        editor.dispatch(Command::Drop).unwrap();
        assert_cached_matches_fresh(&mut cache, &editor.scene);
        editor.dispatch(Command::Undo).unwrap();
        assert_cached_matches_fresh(&mut cache, &editor.scene);
        editor.dispatch(Command::Redo).unwrap();
        assert_cached_matches_fresh(&mut cache, &editor.scene);
    }

    #[test]
    fn equal_document_polyline_and_query_context_match_fresh() {
        use operandi_core::PolygonRepresentation;
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let mesh = &mut scene.active_mut().unwrap().mesh;
        *mesh = Mesh {
            vertices: vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.)],
            faces: vec![Face {
                vertices: vec![0, 1, 2],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            }]
            .into(),
            ..Default::default()
        }
        .into();
        mesh.attributes.representations = vec![PolygonRepresentation::Polyline {
            closed: false,
            curve: Default::default(),
        }];
        scene.validate().unwrap();
        let mut cache = SurfaceCache::default();
        let first = cache.get(&scene, &Default::default());
        let separate: Scene = serde_json::from_slice(&serde_json::to_vec(&scene).unwrap()).unwrap();
        assert!(!operandi_core::Shared::same(
            scene.items[0].mesh.shared(),
            separate.items[0].mesh.shared()
        ));
        assert_eq!(scene, separate);
        assert!(std::sync::Arc::ptr_eq(
            &first,
            &cache.get(&separate, &Default::default())
        ));
        assert_eq!(first.lines.len(), 2);
        let fresh = Surface::new(&separate);
        for view in [View::Front, View::Perspective] {
            let camera = Camera {
                view,
                distance: 5.,
                ..Default::default()
            };
            for rect in [
                Rect::from_min_size(Pos2::ZERO, vec2(700., 700.)),
                Rect::from_min_size(pos2(20., 30.), vec2(1400., 900.)),
            ] {
                for point in [
                    rect.center(),
                    rect.left_top(),
                    rect.center() + vec2(10., -3.),
                ] {
                    assert_eq!(
                        first.pick(&camera, point, rect),
                        fresh.pick(&camera, point, rect)
                    );
                }
            }
        }
        let mut selection = separate.clone();
        selection.selection.mode = Mode::Item;
        // A selection change alone reuses the surface; it depends only on
        // the items.
        let changed = cache.get(&selection, &Default::default());
        assert!(std::sync::Arc::ptr_eq(&first, &changed));
        assert_cached_matches_fresh(&mut cache, &selection);
    }

    #[test]
    fn per_item_world_matrix_matches_authoritative_point_after_hierarchy_changes() {
        let mut scene = sphere(6, 3);
        scene.new_mesh().unwrap();
        scene.new_mesh().unwrap();
        scene.items[1].parent = Some(1);
        scene.items[2].parent = Some(2);
        for (i, item) in scene.items.iter_mut().enumerate() {
            item.transform.rotation = DVec3::new(0.1 + i as f64 * 0.2, -0.3, 0.4);
            item.transform.scale = DVec3::new(1.5, 0.7 + i as f64 * 0.1, 2.);
        }
        for step in 0..3 {
            if step == 1 {
                scene.items[0].transform.translation.x = 3.;
            }
            if step == 2 {
                scene.items[1].parent = Some(2);
            }
            for item in &scene.items {
                let matrix = scene.world_matrix(item.id).ok();
                for point in [DVec3::ZERO, DVec3::new(0.3, -1., 2.)] {
                    let actual = matrix.map_or(DVec3::NAN, |m| m.transform_point3(point));
                    let expected = scene.world_point(item.id, point);
                    if expected.is_nan() {
                        assert!(actual.is_nan());
                    } else {
                        assert_eq!(actual, expected);
                    }
                }
            }
        }
    }

    #[test]
    fn indexed_hits_match_exhaustive_surface_across_views_and_shared_edges() {
        let mut scene = sphere(24, 12);
        scene.items[0].transform.translation = DVec3::new(-0.3, 0.2, 0.);
        scene.items[0].transform.rotation = DVec3::new(0.2, -0.4, 0.1);
        scene.items[0].transform.scale = DVec3::new(1., 0.8, 1.2);
        // Coincident visible instances exercise stable first-hit ties.
        scene.new_mesh().unwrap();
        scene.items[1].mesh = scene.items[0].mesh.clone();
        scene.items[1].transform = scene.items[0].transform.clone();
        scene.new_mesh().unwrap();
        scene.items[2].mesh = Mesh::box_mesh(DVec3::new(0., 0., 1.5), DVec3::splat(0.5), 0)
            .unwrap()
            .into();
        scene.items[2].visible = false;
        let surface = Surface::new(&scene);
        let reference = reference(&scene);
        let r = Rect::from_min_size(Pos2::ZERO, vec2(700., 700.));
        for view in [
            View::Perspective,
            View::Orthographic,
            View::Top,
            View::Bottom,
            View::Front,
            View::Back,
            View::Right,
            View::Left,
        ] {
            let camera = Camera {
                view,
                distance: 4.,
                ..Default::default()
            };
            let mut queries = vec![r.center()];
            for x in 0..15 {
                for y in 0..15 {
                    queries.push(pos2(x as f32 * 50., y as f32 * 50.));
                }
            }
            for &v in &scene.items[0].mesh.vertices {
                if let Some((p, _)) = camera.project(scene.items[0].transform.point(v), r) {
                    queries.push(p);
                }
            }
            for p in queries {
                let (o, d) = camera.ray(p, r);
                assert_eq!(
                    surface.pick(&camera, p, r),
                    brute(&reference, o, d),
                    "{view:?} {p:?}"
                );
            }
        }
    }
    #[test]
    fn reused_bounds_keep_inside_parallel_grazing_and_nearest_sibling_hits() {
        let mut scene = Scene::default();
        // Overlapping bounds and coincident instances exercise a limit that
        // shrinks after one sibling, including stable equal-distance ties.
        for (center, size) in [
            (DVec3::ZERO, DVec3::splat(2.)),
            (DVec3::Z * 3., DVec3::splat(2.)),
            (DVec3::ZERO, DVec3::splat(2.)),
            (DVec3::X * 0.5, DVec3::new(1e-6, 4., 6.)),
        ] {
            scene.new_mesh().unwrap();
            scene.active_mut().unwrap().mesh = Mesh::box_mesh(center, size, 0).unwrap().into();
        }
        let surface = Surface::new(&scene);
        let reference = reference(&scene);
        for z in [-10., -1., 0., 1., 10.] {
            for x in [-1.00000000001, -1., 0., 0.5, 1., 1.00000000001] {
                let origin = DVec3::new(x, 0., z);
                for direction in [
                    DVec3::Z,
                    -DVec3::Z,
                    DVec3::X,
                    DVec3::Y,
                    DVec3::new(1e-12, 0., 1.).normalize(),
                ] {
                    assert_eq!(
                        surface.query(origin, direction).0,
                        brute(&reference, origin, direction),
                        "{origin:?} {direction:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn concave_and_nonplanar_surfaces_keep_display_triangulation() {
        let mut scene = sphere(8, 4);
        scene.new_mesh().unwrap();
        let mesh = &mut scene.active_mut().unwrap().mesh;
        mesh.vertices = vec![
            DVec3::new(-2., -1., 2.),
            DVec3::new(2., -1., 2.),
            DVec3::new(2., 1., 2.1),
            DVec3::new(0., 0., 2.),
            DVec3::new(-2., 1., 2.),
        ];
        mesh.faces.push(Face {
            vertices: vec![0, 1, 2, 3, 4],
            uv: None,
            material: "concave".into(),
            smooth: false,
            part: None,
            material_label: None,
        });
        let surface = Surface::new(&scene);
        let reference = reference(&scene);
        let camera = Camera {
            view: View::Front,
            distance: 5.,
            ..Default::default()
        };
        let r = Rect::from_min_size(Pos2::ZERO, vec2(700., 700.));
        for item in &scene.items {
            for fi in 0..item.mesh.faces.len() {
                assert_eq!(
                    surface.corners(item.id, fi),
                    item.mesh.surface_triangles(fi).unwrap()
                );
            }
        }
        for x in 0..25 {
            for y in 0..25 {
                let p = pos2(x as f32 * 28., y as f32 * 28.);
                let (o, d) = camera.ray(p, r);
                assert_eq!(surface.pick(&camera, p, r), brute(&reference, o, d));
            }
        }
    }
    #[test]
    fn component_visibility_prunes_dense_mesh_instead_of_testing_every_face() {
        let scene = sphere(96, 48);
        let before = scene.clone();
        let surface = Surface::new(&scene);
        let camera = Camera::default();
        let r = Rect::from_min_size(Pos2::ZERO, vec2(1000., 1000.));
        let mut tests = 0;
        let mut queries = 0;
        let mut hits = 0;
        for &v in scene.items[0].mesh.vertices.iter().step_by(5) {
            let p = camera.project(v, r).unwrap().0;
            let (o, d) = camera.ray(p, r);
            let (hit, count) = surface.query(o, d);
            hits += usize::from(hit.is_some());
            tests += count;
            queries += 1;
        }
        assert!(hits > queries / 2);
        assert!(
            tests < queries * surface.triangles.len() / 20,
            "{tests} triangle tests for {queries} rays"
        );
        assert_eq!(scene, before, "picking must not mutate mesh or attributes");
    }
    #[test]
    fn snapshots_follow_geometry_visibility_and_mode_without_stale_hits() {
        let mut scene = sphere(24, 12);
        let r = Rect::from_min_size(Pos2::ZERO, vec2(700., 700.));
        let camera = Camera {
            view: View::Front,
            ..Default::default()
        };
        let original = Surface::new(&scene);
        assert!(original.pick(&camera, r.center(), r).is_some());
        for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item] {
            scene.selection.mode = mode;
            assert_eq!(
                Surface::new(&scene).pick(&camera, r.center(), r),
                original.pick(&camera, r.center(), r)
            );
        }
        scene.items[0].transform.translation.x = 10.;
        assert!(Surface::new(&scene).pick(&camera, r.center(), r).is_none());
        scene.items[0].transform.translation.x = 0.;
        scene.items[0].visible = false;
        assert!(Surface::new(&scene).pick(&camera, r.center(), r).is_none());
        assert!(
            original.pick(&camera, r.center(), r).is_some(),
            "snapshot owns immutable geometry"
        );
        assert!(
            Surface::new(&Scene::default())
                .pick(&camera, r.center(), r)
                .is_none()
        );
        // Orthographic views draw a slab about the orbit centre (bug #2),
        // so "behind the eye" means in front of that slab.
        let reach = crate::camera::ORTHO_REACH.max(camera.distance);
        assert!(
            !original.visible(&camera, DVec3::new(0., 0., 2. * reach), r),
            "point in front of the drawn slab"
        );
    }
}
