//! The snapping layer of the tool pipe (architecture review 2026-11-14,
//! pick B; Jason: each tool reads one shared stack of layers, action
//! centre, falloff, symmetry, snapping, then the tool itself, instead of
//! rolling its own). Every snap asks here: tool handles and Tweak
//! (SN-08), draw-tool, Pen and Tube points (DSN-08..13), Draw's
//! join-a-point with snapping off, the Ruler's ends (RUL-02), Weld's drag
//! (WD-02) and the symmetry seam (SYM3-23). A caller names its [`Role`],
//! which fixes its radius in one table, the target types it wants
//! (`snapping::Targets`, each a [`Kind`]), and its own rules for which
//! elements don't count; the service finds the nearest target left. So a
//! new target type works in every tool at once. A tool card can name a
//! tool's role when that work lands.
//!
//! The targets come from every shown mesh, gathered once per scene
//! ([`WorldTargets`]; edges, centres and boxes only once a type needs
//! them) and projected once per view into grids of [`CELL`]-point screen
//! squares ([`Grid`] for points, [`SegmentGrid`] for edges). They are
//! reused until the scene's items or the view change, so a pointer move
//! reads only the squares around the pointer, and there is no size cap:
//! a million-point mesh snaps like a small one.
//!
//! Hidden targets never snap: mesh points, edges and centres behind a
//! surface don't count. Snaps onto the scene as it is now use the picking
//! surface, so a point snaps exactly when it could be picked (as Weld
//! always did); tool handles use the cage of the scene before the drag.
//! Targets that aren't geometry (a Pen draft's own points, guide lines,
//! pivots, boxes) always count. What each type means is in
//! `snap_kinds.rs`; the grid-crossing target is `snapping::grid_crossing`
//! and angle snaps share `operandi_core::angle_snap`.
use crate::camera::{Camera, Projector};
use crate::picking::Surface;
use crate::snap_kinds::{self, Kind};
use crate::snapping::{Hit, Targets};
use eframe::egui::{Pos2, Rect, Vec2};
use glam::DVec3;
use operandi_core::{Item, Scene};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// Who is snapping. The radii, in screen points, all live here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A tool handle or Tweak's grab point (SN-08): the Outer Range of
    /// Snapping Options, targets from the scene before the drag.
    Handle,
    /// A Curve, B-Spline, Bezier, Pen or Tube point with snapping on
    /// (DSN-08): the Outer Range, targets pinned for the drag.
    DrawPoint,
    /// A Draw point joining another mesh's point with snapping off.
    JoinPoint,
    /// A Ruler end landing on a mesh point (RUL-02).
    RulerEnd,
    /// A Weld drag onto another point of the same mesh (WD-02).
    Weld,
    /// A placed point landing on the symmetry plane (SYM3-23): its own
    /// Symmetry Seam Snap setting.
    SymmetrySeam,
}

impl Role {
    /// The radius when it doesn't come from Snapping Options.
    pub const fn fixed_radius(self) -> Option<f32> {
        match self {
            Role::Handle | Role::DrawPoint => None,
            Role::JoinPoint => Some(8.),
            Role::RulerEnd => Some(10.),
            // Logical (egui) points: a Retina display's 2× pixels don't
            // halve it.
            Role::Weld => Some(15.),
            Role::SymmetrySeam => None,
        }
    }
    /// This role's radius with the user's Snapping Options: the Outer
    /// Range, or for the seam its own Symmetry Seam Snap setting.
    pub fn radius(self, options: &crate::snapping::SnappingPreferences) -> f32 {
        match self {
            Role::SymmetrySeam => options.symmetry_seam,
            _ => self.fixed_radius().unwrap_or_else(|| options.outer()),
        }
    }
}

// The card-to-role mapping for the tool-card pipe. `snap_move` already
// reads the card directly; tools that move onto the card's snap layer next
// look their role up here.
#[cfg_attr(not(test), allow(dead_code))]
impl Role {
    /// The role a tool card's snap layer names (pipe layer 4): what of the
    /// tool snaps decides how it searches. None: the tool doesn't snap.
    pub fn of_card(snap: operandi_core::tool_card::Snap) -> Option<Self> {
        use operandi_core::tool_card::Snap;
        match snap {
            Snap::None => None,
            // Its move handle, its relocated centre, or Tweak's grabbed
            // element: a dragged handle against the scene before the drag.
            Snap::Handle | Snap::Centre | Snap::Grab => Some(Role::Handle),
            // Points it places on the work plane.
            Snap::Placement => Some(Role::DrawPoint),
        }
    }
    /// The role of `tool`'s handle snap, from its tool card.
    pub fn of_tool(tool: operandi_core::Tool) -> Option<Self> {
        Self::of_card(operandi_core::tool_card::card(tool).layers.snap)
    }
}

/// Whether `target` is within `role`'s radius of `point` on screen: the
/// check for targets that aren't points (the symmetry plane).
pub fn within(
    projector: &Projector,
    options: &crate::snapping::SnappingPreferences,
    role: Role,
    point: DVec3,
    target: DVec3,
) -> bool {
    match (projector.project(point), projector.project(target)) {
        (Some((a, _)), Some((b, _))) => a.distance(b) <= role.radius(options),
        _ => false,
    }
}

/// The owner id of the box around every shown mesh (no item has id 0).
pub const ALL_ITEMS: u64 = 0;

/// Side of one grid square, in screen points.
pub const CELL: f32 = 16.;
/// The grid reaches this far past the view (the largest snap range), so a
/// handle dragged just off the view still finds the points inside it.
/// Points farther out sit in one overflow list, read only by a query that
/// reaches past the grid.
const MARGIN: f32 = 200.;
/// An edge spanning more grid squares than this on screen goes in the
/// overflow list instead of every square it covers.
const LONG_EDGE_CELLS: usize = 64;
/// Intersection tests this many of the edges nearest the handle.
const CROSSING_EDGES: usize = 48;

/// What a target belongs to, for the caller's "doesn't count" rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// A guide line or one of the caller's own extra points.
    None,
    /// Point `v` of mesh item `item`.
    Point { item: u64, v: u32 },
    /// The edge between points `a` and `b` of `item`.
    Edge { item: u64, a: u32, b: u32 },
    /// Polygon `face` of `item`.
    Polygon { item: u64, face: u32 },
    /// The item as a whole (its pivot or bounding box).
    Item(u64),
}

/// A snap found: where, which type, and what it belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapHit {
    pub hit: Hit,
    pub kind: Kind,
    pub owner: Owner,
}

/// Polygon centres, and the (span, face) each belongs to.
type PolygonCenters = (Vec<DVec3>, Vec<(u32, u32)>);
/// A cached point grid: the targets and view it was built for.
type GridEntry = (Rc<WorldTargets>, PointList, Projector, Rc<Grid>);
/// The cached edge grid: the targets and view it was built for.
type SegmentEntry = (Rc<WorldTargets>, Projector, Rc<SegmentGrid>);

/// One shown mesh in [`WorldTargets`].
struct Span {
    id: u64,
    /// Index of its first point in `WorldTargets::points`.
    start: usize,
    /// Its index in `WorldTargets::items`.
    item: usize,
    matrix: glam::DMat4,
}

/// The targets of one scene in world space, in scene order (items in
/// order, then each mesh's points in order). Points and pivots are
/// gathered at once; edges, centres and boxes on first need.
pub struct WorldTargets {
    /// The items these were gathered from (the reuse key, and the meshes
    /// the lazy lists read).
    items: Vec<Item>,
    spans: Vec<Span>,
    points: Vec<DVec3>,
    /// Every shown item's pivot (its world position).
    pivots: Vec<(u64, DVec3)>,
    /// (span, first point, second point), point indices into `points`.
    edges: OnceCell<Vec<(u32, u32, u32)>>,
    edge_centers: OnceCell<Vec<DVec3>>,
    /// Polygon centres and their (span, face).
    polygon_centers: OnceCell<PolygonCenters>,
    /// The 27 box points of each mesh's world bounding box.
    boxes: OnceCell<Vec<(u64, DVec3)>>,
    /// G9 GD-2: the scene's guides with Snap to Guide on (world segments;
    /// part of the reuse key).
    guides: Vec<(DVec3, DVec3)>,
}

impl WorldTargets {
    fn build(scene: &Scene) -> Self {
        let mut spans = Vec::new();
        let mut points = Vec::new();
        let mut pivots = Vec::new();
        for (index, item) in scene.items.iter().enumerate() {
            if !scene.shows(item) {
                continue;
            }
            let Ok(matrix) = scene.world_matrix(item.id) else {
                continue;
            };
            pivots.push((item.id, matrix.w_axis.truncate()));
            if !item.is_mesh() {
                continue;
            }
            spans.push(Span {
                id: item.id,
                start: points.len(),
                item: index,
                matrix,
            });
            points.extend(
                item.mesh
                    .vertices
                    .iter()
                    .map(|p| matrix.transform_point3(*p)),
            );
        }
        Self {
            items: scene.items.clone(),
            spans,
            points,
            pivots,
            edges: OnceCell::new(),
            edge_centers: OnceCell::new(),
            polygon_centers: OnceCell::new(),
            boxes: OnceCell::new(),
            guides: scene.guides.snapping(),
        }
    }
    fn span_of(&self, i: usize) -> usize {
        self.spans.partition_point(|span| span.start <= i) - 1
    }
    /// The item and point number of point `i`.
    pub fn owner(&self, i: usize) -> (u64, u32) {
        let span = &self.spans[self.span_of(i)];
        (span.id, (i - span.start) as u32)
    }
    /// The world position of point `v` of `item`.
    pub fn point_of(&self, item: u64, v: u32) -> Option<DVec3> {
        let span = self.spans.iter().find(|s| s.id == item)?;
        let end = self
            .spans
            .iter()
            .find(|s| s.start > span.start)
            .map_or(self.points.len(), |s| s.start);
        self.points[span.start..end].get(v as usize).copied()
    }
    /// The points (numbers in their mesh) a target is made from: one for
    /// a point, two for an edge, a polygon's corners; none otherwise.
    pub fn owner_points(&self, owner: &Owner) -> Vec<u32> {
        match *owner {
            Owner::Point { v, .. } => vec![v],
            Owner::Edge { a, b, .. } => vec![a, b],
            Owner::Polygon { item, face } => self.face_points(item, face).to_vec(),
            Owner::None | Owner::Item(_) => Vec::new(),
        }
    }
    fn edges(&self) -> &[(u32, u32, u32)] {
        self.edges.get_or_init(|| {
            let mut out = Vec::new();
            for (k, span) in self.spans.iter().enumerate() {
                let start = span.start as u32;
                for (a, b) in self.items[span.item].mesh.edges() {
                    out.push((k as u32, start + a, start + b));
                }
            }
            out
        })
    }
    fn edge_owner(&self, e: usize) -> Owner {
        let (span, a, b) = self.edges()[e];
        let span = &self.spans[span as usize];
        let start = span.start as u32;
        Owner::Edge {
            item: span.id,
            a: a - start,
            b: b - start,
        }
    }
    fn edge_ends(&self, e: usize) -> (DVec3, DVec3) {
        let (_, a, b) = self.edges()[e];
        (self.points[a as usize], self.points[b as usize])
    }
    fn edge_centers(&self) -> &[DVec3] {
        self.edge_centers.get_or_init(|| {
            self.edges()
                .iter()
                .map(|&(_, a, b)| (self.points[a as usize] + self.points[b as usize]) / 2.)
                .collect()
        })
    }
    fn polygon_centers(&self) -> &PolygonCenters {
        self.polygon_centers.get_or_init(|| {
            let mut centers = Vec::new();
            let mut owners = Vec::new();
            for (k, span) in self.spans.iter().enumerate() {
                let mesh = &self.items[span.item].mesh;
                for (f, face) in mesh.faces.iter().enumerate() {
                    if face.vertices.len() < 3 {
                        continue;
                    }
                    let sum: DVec3 = face
                        .vertices
                        .iter()
                        .map(|&v| self.points[span.start + v as usize])
                        .sum();
                    centers.push(sum / face.vertices.len() as f64);
                    owners.push((k as u32, f as u32));
                }
            }
            (centers, owners)
        })
    }
    /// SN2-32: the 8 corners of each mesh's own box (it turns with the
    /// item, Operandi choice Q-08), then of one world box around every
    /// shown mesh (owned by [`ALL_ITEMS`]).
    fn boxes(&self) -> &[(u64, DVec3)] {
        self.boxes.get_or_init(|| {
            let mut out = Vec::new();
            for span in &self.spans {
                let local = &self.items[span.item].mesh.vertices;
                let Some(&first) = local.first() else {
                    continue;
                };
                let (lo, hi) = local
                    .iter()
                    .fold((first, first), |(lo, hi), &p| (lo.min(p), hi.max(p)));
                out.extend(
                    snap_kinds::box_corners(lo, hi)
                        .map(|p| (span.id, span.matrix.transform_point3(p))),
                );
            }
            if self.spans.len() > 1
                && let Some(&first) = self.points.first()
            {
                let (lo, hi) = self
                    .points
                    .iter()
                    .fold((first, first), |(lo, hi), &p| (lo.min(p), hi.max(p)));
                out.extend(snap_kinds::box_corners(lo, hi).map(|p| (ALL_ITEMS, p)));
            }
            out
        })
    }
    /// The corner points (numbers in its mesh) of `face` of `item`.
    fn face_points(&self, item: u64, face: u32) -> &[u32] {
        self.spans
            .iter()
            .find(|s| s.id == item)
            .and_then(|s| self.items[s.item].mesh.faces.get(face as usize))
            .map_or(&[], |f| f.vertices.as_slice())
    }
}

impl std::fmt::Debug for WorldTargets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldTargets")
            .field("meshes", &self.spans.len())
            .field("points", &self.points.len())
            .finish()
    }
}

/// The squares of the screen a grid covers: the view plus [`MARGIN`].
#[derive(Clone, Copy)]
struct Cells {
    origin: Pos2,
    cols: usize,
    rows: usize,
}

impl Cells {
    fn new(projector: &Projector) -> Self {
        let area = projector.rect().expand(MARGIN);
        Self {
            origin: area.min,
            cols: ((area.width() / CELL).ceil() as usize).max(1),
            rows: ((area.height() / CELL).ceil() as usize).max(1),
        }
    }
    fn count(&self) -> usize {
        self.cols * self.rows
    }
    fn rect(&self) -> Rect {
        Rect::from_min_size(
            self.origin,
            Vec2::new(self.cols as f32 * CELL, self.rows as f32 * CELL),
        )
    }
    /// The (column, row) square holding `q`, unclamped.
    fn square(&self, q: Pos2) -> (f32, f32) {
        (
            ((q.x - self.origin.x) / CELL).floor(),
            ((q.y - self.origin.y) / CELL).floor(),
        )
    }
    /// The square holding `q`, or [`Cells::count`] when it is outside.
    fn of(&self, q: Pos2) -> usize {
        let (x, y) = self.square(q);
        if x >= 0. && y >= 0. && (x as usize) < self.cols && (y as usize) < self.rows {
            y as usize * self.cols + x as usize
        } else {
            self.count()
        }
    }
    /// The squares (column range, row range) a screen rect touches inside
    /// the grid, or None when it misses the grid.
    fn span(
        &self,
        r: Rect,
    ) -> Option<(
        std::ops::RangeInclusive<usize>,
        std::ops::RangeInclusive<usize>,
    )> {
        if !self.rect().intersects(r) {
            return None;
        }
        let (lx, ly) = self.square(r.min);
        let (hx, hy) = self.square(r.max);
        let clamp = |v: f32, n: usize| (v.max(0.) as usize).min(n - 1);
        Some((
            clamp(lx, self.cols)..=clamp(hx, self.cols),
            clamp(ly, self.rows)..=clamp(hy, self.rows),
        ))
    }
}

/// Counting sort of `(entry, square)` pairs into `starts`/`entries` form:
/// `entries[starts[c]..starts[c + 1]]` lie in square `c`, and the last
/// range (square `count`) is the overflow.
fn bucket<T: Copy + Default>(pairs: &[(T, usize)], count: usize) -> (Vec<u32>, Vec<T>) {
    let mut starts = vec![0u32; count + 2];
    for &(_, c) in pairs {
        starts[c + 1] += 1;
    }
    for c in 0..=count {
        starts[c + 1] += starts[c];
    }
    let mut fill = starts.clone();
    let mut entries = vec![T::default(); pairs.len()];
    for &(e, c) in pairs {
        entries[fill[c] as usize] = e;
        fill[c] += 1;
    }
    starts.pop();
    (starts, entries)
}

/// Points projected through one view and sorted into screen squares (a
/// counting sort, so building is linear in the point count).
pub struct Grid {
    cells: Cells,
    starts: Vec<u32>,
    /// (index into the source points, screen position).
    entries: Vec<(u32, Pos2)>,
}

impl Grid {
    fn build(points: &[DVec3], projector: &Projector) -> Self {
        let cells = Cells::new(projector);
        let pairs: Vec<((u32, Pos2), usize)> = points
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| {
                let (q, _) = projector.project(p)?;
                (q.x.is_finite() && q.y.is_finite()).then(|| ((i as u32, q), cells.of(q)))
            })
            .collect();
        let (starts, entries) = bucket(&pairs, cells.count());
        Self {
            cells,
            starts,
            entries,
        }
    }

    /// Points within `radius` of `at` on screen, nearest first; equal
    /// distances keep their order.
    pub fn near(&self, at: Pos2, radius: f32) -> Vec<Candidate> {
        let mut out = Vec::new();
        if radius.is_nan() || radius < 0. || !at.x.is_finite() || !at.y.is_finite() {
            return out;
        }
        let mut take = |range: std::ops::Range<usize>| {
            for &(i, q) in &self.entries[range] {
                let distance = q.distance(at);
                if distance <= radius {
                    out.push(Candidate {
                        index: i as usize,
                        screen: q,
                        distance,
                    });
                }
            }
        };
        let reach = Rect::from_center_size(at, Vec2::splat(radius * 2.));
        if let Some((xs, ys)) = self.cells.span(reach) {
            for y in ys {
                let row = y * self.cells.cols;
                take(
                    self.starts[row + xs.start()] as usize
                        ..self.starts[row + xs.end() + 1] as usize,
                );
            }
        }
        if !self.cells.rect().contains_rect(reach) {
            take(self.starts[self.cells.count()] as usize..self.entries.len());
        }
        out.sort_by(|a, b| {
            a.distance
                .total_cmp(&b.distance)
                .then(a.index.cmp(&b.index))
        });
        out
    }
}

/// One point near the query position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    /// Index into the points the grid was built from.
    pub index: usize,
    pub screen: Pos2,
    pub distance: f32,
}

/// Edges projected through one view, each listed in every square its
/// screen box touches (long ones in the overflow list).
pub struct SegmentGrid {
    cells: Cells,
    starts: Vec<u32>,
    entries: Vec<u32>,
}

impl SegmentGrid {
    fn build(world: &WorldTargets, projector: &Projector) -> Self {
        let cells = Cells::new(projector);
        let mut pairs: Vec<(u32, usize)> = Vec::new();
        for (e, &(_, a, b)) in world.edges().iter().enumerate() {
            let (Some((qa, _)), Some((qb, _))) = (
                projector.project(world.points[a as usize]),
                projector.project(world.points[b as usize]),
            ) else {
                continue;
            };
            let r = Rect::from_two_pos(qa, qb);
            if !r.min.is_finite() || !r.max.is_finite() {
                continue;
            }
            match cells.span(r) {
                None => {}
                Some((xs, ys)) if xs.clone().count() * ys.clone().count() <= LONG_EDGE_CELLS => {
                    for y in ys {
                        for x in xs.clone() {
                            pairs.push((e as u32, y * cells.cols + x));
                        }
                    }
                    if !cells.rect().contains_rect(r) {
                        pairs.push((e as u32, cells.count()));
                    }
                }
                Some(_) => pairs.push((e as u32, cells.count())),
            }
        }
        let (starts, entries) = bucket(&pairs, cells.count());
        Self {
            cells,
            starts,
            entries,
        }
    }
    /// Edges that may pass within `radius` of `at` (each once).
    fn near(&self, at: Pos2, radius: f32) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        let reach = Rect::from_center_size(at, Vec2::splat(radius * 2.));
        if let Some((xs, ys)) = self.cells.span(reach) {
            for y in ys {
                let row = y * self.cells.cols;
                let range = self.starts[row + xs.start()] as usize
                    ..self.starts[row + xs.end() + 1] as usize;
                out.extend(self.entries[range].iter().map(|&e| e as usize));
            }
        }
        out.extend(
            self.entries[self.starts[self.cells.count()] as usize..]
                .iter()
                .map(|&e| e as usize),
        );
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// One snap.
pub struct Query<'a> {
    /// The pointer or dragged handle, on screen.
    pub at: Pos2,
    /// The caller's outer radius in screen points: targets this close
    /// count (pre-highlight).
    pub radius: f32,
    /// The caller's Inner Range: a target this close beats Polygon, which
    /// lies under the handle (SN2-15/28).
    pub inner: f32,
    /// Pivot options' Snap Size: Inner Range for pivots (SN2-33).
    pub pivot_inner: f32,
    /// The target types to look for. Grid and the direction types are the
    /// caller's own (they aren't points of the scene).
    pub targets: Targets,
    /// Targets that don't count (the caller's own rule: what the drag
    /// moves, Layers, Only Selected).
    pub skip: &'a dyn Fn(&Owner) -> bool,
    /// Targets that aren't scene geometry (a Pen draft's own points),
    /// drawn on top so never hidden. They rank as vertices, after mesh
    /// points.
    pub extra: &'a [DVec3],
    /// Depth Limitation (SN2-20): targets farther from the camera along
    /// the view than this don't count.
    pub max_depth: Option<f64>,
    /// The surface that hides targets behind it, and that Polygon and
    /// Intersection land on.
    pub surface: &'a Surface,
    pub camera: &'a Camera,
    pub rect: Rect,
}

/// One candidate: (distance, kind, index for ties in scene order, hit,
/// owner, never hidden).
type Found = (f32, Kind, usize, Hit, Owner, bool);

/// Which list a point grid was built over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointList {
    Vertex,
    EdgeCenter,
    PolygonCenter,
}

/// The reuse caches. One per app; queries borrow it immutably.
#[derive(Default)]
pub struct SnapService {
    world: RefCell<Option<Rc<WorldTargets>>>,
    grids: RefCell<Vec<GridEntry>>,
    segments: RefCell<Option<SegmentEntry>>,
    cage: RefCell<Option<(Vec<Item>, Arc<Surface>)>>,
    grid_builds: Cell<u64>,
}

impl SnapService {
    /// The targets of `scene`, gathered again only when its items
    /// changed.
    pub fn world(&self, scene: &Scene) -> Rc<WorldTargets> {
        if let Some(world) = &*self.world.borrow()
            && world.items == scene.items
            && world.guides == scene.guides.snapping()
        {
            return world.clone();
        }
        let world = Rc::new(WorldTargets::build(scene));
        *self.world.borrow_mut() = Some(world.clone());
        world
    }
    fn point_grid(
        &self,
        world: &Rc<WorldTargets>,
        list: PointList,
        projector: Projector,
    ) -> Rc<Grid> {
        if let Some((_, _, _, grid)) = self
            .grids
            .borrow()
            .iter()
            .find(|(w, l, p, _)| Rc::ptr_eq(w, world) && *l == list && *p == projector)
        {
            return grid.clone();
        }
        self.grid_builds.set(self.grid_builds.get() + 1);
        let points: &[DVec3] = match list {
            PointList::Vertex => &world.points,
            PointList::EdgeCenter => world.edge_centers(),
            PointList::PolygonCenter => &world.polygon_centers().0,
        };
        let grid = Rc::new(Grid::build(points, &projector));
        let mut grids = self.grids.borrow_mut();
        // One view and scene at a time: older grids go.
        grids.retain(|(w, _, p, _)| Rc::ptr_eq(w, world) && *p == projector);
        grids.push((world.clone(), list, projector, grid.clone()));
        grid
    }
    /// The vertex grid of `world` through `projector` (tests).
    #[cfg(test)]
    pub fn vertex_grid(&self, world: &Rc<WorldTargets>, projector: Projector) -> Rc<Grid> {
        self.point_grid(world, PointList::Vertex, projector)
    }
    fn segment_grid(&self, world: &Rc<WorldTargets>, projector: Projector) -> Rc<SegmentGrid> {
        if let Some((w, p, grid)) = &*self.segments.borrow()
            && Rc::ptr_eq(w, world)
            && *p == projector
        {
            return grid.clone();
        }
        let grid = Rc::new(SegmentGrid::build(world, &projector));
        *self.segments.borrow_mut() = Some((world.clone(), projector, grid.clone()));
        grid
    }
    /// `scene`'s cage surface for hiding points behind it. Only tool
    /// handles use it, because their targets are the scene from before the
    /// drag (`live.base`), which the picking surface doesn't cover; they
    /// hide by the cage, not the smooth surface (provisional, unchanged).
    pub fn cage(&self, scene: &Scene) -> Arc<Surface> {
        if let Some((items, surface)) = &*self.cage.borrow()
            && *items == scene.items
        {
            return surface.clone();
        }
        let surface = Arc::new(Surface::with_smooth(scene, None));
        *self.cage.borrow_mut() = Some((scene.items.clone(), surface.clone()));
        surface
    }
    /// How many times a point grid was built (tests: a drag builds one).
    #[cfg(test)]
    pub fn grid_builds(&self) -> u64 {
        self.grid_builds.get()
    }

    /// The best target of `query` among `world`'s, or None when nothing
    /// counts within the radius (SN2-15): the nearest on screen, ties
    /// within [`snap_kinds::TIE_PX`] going to the more specific type.
    /// Polygon lies under the handle, so it wins only when no other target
    /// is inside the Inner Range.
    pub fn nearest(&self, world: &Rc<WorldTargets>, query: &Query) -> Option<SnapHit> {
        let projector = query.camera.projector(query.rect);
        let (at, radius, targets) = (query.at, query.radius, query.targets);
        if radius.is_nan() || radius < 0. || !at.x.is_finite() || !at.y.is_finite() {
            return None;
        }
        let mut found: Vec<Found> = Vec::new();
        let hit = |point: DVec3, screen: Pos2, distance: f32| Hit {
            point,
            screen,
            distance,
        };
        let mut from_grid = |list: PointList, kind: Kind, owner: &dyn Fn(usize) -> Owner| {
            let points: &[DVec3] = match list {
                PointList::Vertex => &world.points,
                PointList::EdgeCenter => world.edge_centers(),
                PointList::PolygonCenter => &world.polygon_centers().0,
            };
            for c in self.point_grid(world, list, projector).near(at, radius) {
                let h = hit(points[c.index], c.screen, c.distance);
                found.push((c.distance, kind, c.index, h, owner(c.index), false));
            }
        };
        if targets.vertex {
            from_grid(PointList::Vertex, Kind::Vertex, &|i| {
                let (item, v) = world.owner(i);
                Owner::Point { item, v }
            });
        }
        if targets.edge_center {
            from_grid(PointList::EdgeCenter, Kind::EdgeCenter, &|e| {
                world.edge_owner(e)
            });
        }
        if targets.polygon_center {
            from_grid(PointList::PolygonCenter, Kind::PolygonCenter, &|i| {
                let (span, face) = world.polygon_centers().1[i];
                Owner::Polygon {
                    item: world.spans[span as usize].id,
                    face,
                }
            });
        }
        let mut few = |kind: Kind, list: &[(u64, DVec3)]| {
            for (i, &(item, point)) in list.iter().enumerate() {
                if let Some((screen, _)) = projector.project(point) {
                    let distance = screen.distance(at);
                    if distance <= radius {
                        let h = hit(point, screen, distance);
                        found.push((distance, kind, i, h, Owner::Item(item), true));
                    }
                }
            }
        };
        if targets.bounding_box {
            few(Kind::Box, world.boxes());
        }
        if targets.pivot {
            few(Kind::Pivot, &world.pivots);
        }
        if targets.edge || targets.intersection {
            self.edge_candidates(world, query, &projector, &mut found);
        }
        // G9 GD-2: the nearest spot on a snapping guide, one more target
        // (an edge, drawn on top so never hidden).
        for &(a, b) in &world.guides {
            if let (Some((pa, _)), Some((pb, _))) = (projector.project(a), projector.project(b)) {
                let d = pb - pa;
                let t = if d.length_sq() < 1e-6 {
                    0.
                } else {
                    ((at - pa).dot(d) / d.length_sq()).clamp(0., 1.)
                };
                let screen = pa + d * t;
                let distance = screen.distance(at);
                if distance <= radius {
                    let point = a.lerp(b, t as f64);
                    let h = hit(point, screen, distance);
                    found.push((distance, Kind::Edge, usize::MAX, h, Owner::None, true));
                }
            }
        }
        for &point in query.extra {
            if let Some((screen, _)) = projector.project(point) {
                let distance = screen.distance(at);
                if distance <= radius {
                    let h = hit(point, screen, distance);
                    found.push((distance, Kind::Vertex, usize::MAX, h, Owner::None, true));
                }
            }
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let counts = |f: &Found| {
            let (_, kind, _, h, owner, always) = f;
            let deep = query.max_depth.is_some_and(|limit| {
                projector
                    .project(h.point)
                    .is_none_or(|(_, depth)| depth > limit)
            });
            !deep
                && !(query.skip)(owner)
                && (*always
                    || !kind.occluded()
                    || query.surface.visible(query.camera, h.point, query.rect))
        };
        // The nearest that counts, then any within the tie distance of it
        // of a more specific type.
        let mut best: Option<Found> = None;
        let mut anchor = f32::INFINITY;
        for f in found {
            if f.0 > anchor + snap_kinds::TIE_PX {
                break;
            }
            if best
                .as_ref()
                .is_some_and(|b| f.1.priority() >= b.1.priority())
                || !counts(&f)
            {
                continue;
            }
            anchor = anchor.min(f.0);
            best = Some(f);
        }
        let inner_of = |kind: Kind| {
            if kind == Kind::Pivot {
                query.pivot_inner
            } else {
                query.inner
            }
        };
        if let Some((distance, kind, _, hit, owner, _)) = best
            && (distance <= inner_of(kind) || !targets.polygon)
        {
            return Some(SnapHit { hit, kind, owner });
        }
        // SN2-28: Polygon, where the ray through the handle first hits a
        // visible polygon (distance 0: it is under the handle).
        let polygon = targets
            .polygon
            .then(|| self.polygon_candidate(query, &projector))
            .flatten();
        polygon.or_else(|| best.map(|(_, kind, _, hit, owner, _)| SnapHit { hit, kind, owner }))
    }

    /// SN2-26 Edge and SN2-31 Intersection candidates near the handle.
    fn edge_candidates(
        &self,
        world: &Rc<WorldTargets>,
        query: &Query,
        projector: &Projector,
        found: &mut Vec<Found>,
    ) {
        let (at, radius, targets) = (query.at, query.radius, query.targets);
        let segments = self.segment_grid(world, *projector);
        let mut near: Vec<(f32, usize, DVec3, Pos2)> = segments
            .near(at, radius)
            .into_iter()
            .filter_map(|e| {
                let (a, b) = world.edge_ends(e);
                let (point, screen, d) = snap_kinds::nearest_on_segment(projector, a, b, at)?;
                (d <= radius).then_some((d, e, point, screen))
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        if targets.edge {
            for &(d, e, point, screen) in &near {
                let h = Hit {
                    point,
                    screen,
                    distance: d,
                };
                found.push((d, Kind::Edge, e, h, world.edge_owner(e), false));
            }
        }
        if !targets.intersection {
            return;
        }
        // Q-07 (Operandi choice): where an edge near the handle passes
        // through a polygon that doesn't share one of its points, in any
        // mesh. Only edges near the handle are tested, so it stays fast.
        near.truncate(CROSSING_EDGES);
        for &(_, e, ..) in &near {
            let (a, b) = world.edge_ends(e);
            let Owner::Edge { item, a: va, b: vb } = world.edge_owner(e) else {
                continue;
            };
            for (hit_item, face, t) in query.surface.segment_hits(a, b) {
                let shares = hit_item == item
                    && world
                        .face_points(hit_item, face)
                        .iter()
                        .any(|&v| v == va || v == vb);
                if shares {
                    continue;
                }
                let point = a.lerp(b, t);
                if let Some((screen, _)) = projector.project(point) {
                    let d = screen.distance(at);
                    if d <= radius {
                        let h = Hit {
                            point,
                            screen,
                            distance: d,
                        };
                        found.push((d, Kind::Intersection, e, h, world.edge_owner(e), false));
                    }
                }
            }
        }
    }

    /// SN2-28: the polygon under the handle, if it counts.
    fn polygon_candidate(&self, query: &Query, projector: &Projector) -> Option<SnapHit> {
        let (item, face, t) = query
            .surface
            .polygon_under(query.camera, query.at, query.rect)?;
        let owner = Owner::Polygon { item, face };
        let (o, d) = query.camera.ray(query.at, query.rect);
        let point = o + d * t;
        let (screen, depth) = projector.project(point)?;
        let deep = query.max_depth.is_some_and(|limit| depth > limit);
        (!(query.skip)(&owner) && !deep && point.is_finite()).then_some(SnapHit {
            hit: Hit {
                point,
                screen,
                distance: 0.,
            },
            kind: Kind::Polygon,
            owner,
        })
    }
}

impl crate::Operandi {
    /// The nearest mesh point of the scene as it is now within `role`'s
    /// radius of `at` (screen), not left out by `skip` and not hidden
    /// behind the surface: Draw's join-a-point with snapping off, the
    /// Ruler's ends and Weld (they snap to points only).
    pub(crate) fn snap_scene_point(
        &self,
        at: Pos2,
        role: Role,
        r: Rect,
        skip: &dyn Fn(&Owner) -> bool,
    ) -> Option<SnapHit> {
        let scene = &self.editor.scene;
        let world = self.snap_service.world(scene);
        let surface = self.pick_surface();
        self.snap_service.nearest(
            &world,
            &Query {
                at,
                radius: role.radius(&self.snapping),
                inner: role.radius(&self.snapping),
                pivot_inner: role.radius(&self.snapping),
                targets: Targets::vertex_only(),
                skip,
                extra: &[],
                max_depth: None,
                surface: &surface,
                camera: &self.camera,
                rect: r,
            },
        )
    }
}
