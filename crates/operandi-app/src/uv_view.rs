//! UV view (spec `uv-unwrap-pack-relax` v1.1 §4): any view can show UV
//! space instead of 3D (View > UV View). U runs right and V up, the 0–1 tile
//! being the texture's home. It draws the active mesh's UV map with its
//! selection, seams (discontinuous edges) and the Overlap and Distortion
//! overlays; clicks select in UV space and share the selection with 3D.
//!
//! UV editing stage 1 (uv-editing-2026-11-16) adds: the map button follows
//! and sets the mesh's selected map (UVE-1 section 1); Move, Rotate, Scale
//! and Transform started here edit UVs with flat 2D handles (UVE-1 section
//! 2, UVT-40..42); paint (left-drag) and lasso (right-drag) selection
//! (UVS-1 section 4); and the Checker overlay, an approved Operandi extra
//! (UVT-114: Jason OK'd it on 2026-11-16, a deviation from the reference).
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md UV-Q24..Q27): a UV view is
//! a mode of an ordinary view rather than a separate layout; only the active
//! mesh's UVs are drawn; G (centre on the cursor) is left to Edge Slice; the
//! overlap tint marks whole overlapping triangles.
use super::*;
use crate::unavailable::Gate;
use glam::DVec2;
use operandi_core::uv_maps::UvMapEdit;
use operandi_core::uv_tools::target_uv_map;
use std::collections::HashMap;

mod tools;

/// Discontinuous (seam) edges: pale lavender (spec §4).
const SEAM: Color32 = Color32::from_rgb(153, 153, 255);
/// Overlap tint (spec §4: red).
const OVERLAP: Color32 = Color32::from_rgba_premultiplied(140, 20, 20, 110);
/// Flipped tint (UVT-110: yellow).
const FLIPPED: Color32 = Color32::from_rgba_premultiplied(150, 130, 20, 110);

/// UVT-111 (UVV-P01, provisional): the share of the 0-1 tile under UV
/// polygons, sampled on a 128 x 128 grid of cell centres (overlaps count
/// once).
pub(super) fn coverage(polygons: &[(u32, Vec<u32>, Vec<DVec2>)]) -> f64 {
    const N: usize = 128;
    let mut covered = vec![false; N * N];
    for (_, _, q) in polygons {
        let (lo, hi) = q
            .iter()
            .fold((DVec2::MAX, DVec2::MIN), |(a, b), p| (a.min(*p), b.max(*p)));
        let cell = |x: f64| ((x * N as f64 - 0.5).ceil().max(0.) as usize).min(N);
        let (x0, x1) = (cell(lo.x), cell(hi.x).min(N));
        let (y0, y1) = (cell(lo.y), cell(hi.y).min(N));
        for y in y0..y1.max(y0) {
            for x in x0..x1.max(x0) {
                if covered[y * N + x] {
                    continue;
                }
                let p = DVec2::new((x as f64 + 0.5) / N as f64, (y as f64 + 0.5) / N as f64);
                if point_in_polygon(p, q) {
                    covered[y * N + x] = true;
                }
            }
        }
    }
    covered.iter().filter(|&&c| c).count() as f64 / (N * N) as f64
}
/// Distortion: neutral gray-green, too small toward red, too large toward
/// blue (spec §4).
const NEUTRAL: [f32; 3] = [0.45, 0.52, 0.45];
const SMALL: [f32; 3] = [0.85, 0.15, 0.12];
const LARGE: [f32; 3] = [0.15, 0.30, 0.90];
/// Spec §4 starting zoom: 300 screen points per UV unit (UV-Q14).
pub(super) const START_ZOOM: f64 = 300.;

/// One view's UV state.
#[derive(Clone, Debug)]
pub(super) struct UvView {
    /// The UV point at the view's centre.
    pub center: DVec2,
    /// Screen points per UV unit.
    pub zoom: f64,
    pub show_overlap: bool,
    pub show_distortion: bool,
    /// UVT-114 Checker (approved Operandi extra, 2026-11-16): off by
    /// default.
    pub show_checker: bool,
    /// UVT-110: tint flipped UV polygons yellow (off by default, build
    /// guess).
    pub show_flipped: bool,
    /// UVT-111: the last coverage worked out (it stays while a drag runs,
    /// and is worked out again when the edit ends).
    pub(super) coverage: std::cell::Cell<Option<f64>>,
    drag: Option<Drag>,
    /// The view's rect last frame.
    pub rect: Rect,
    /// Element Move: the UV element a press would grab (UVT-131).
    pub(super) tweak_hover: Option<operandi_core::TweakElement>,
    /// Element Move: the UV element a click with no drag put handles on
    /// (UVT-131, UVT-Q13); its U / V arrows and centre then drag it.
    pub(super) tweak_clicked: Option<operandi_core::TweakGrab>,
    /// Box Transform's box as last worked out (corners, pivot), for
    /// painting.
    pub(super) box_now: Option<([DVec2; 4], DVec2)>,
}

#[derive(Clone, Debug)]
enum Drag {
    Pan {
        last: Pos2,
    },
    Zoom {
        last: Pos2,
        anchor: DVec2,
    },
    /// A transform tool drag: which handle, where it started, the tool's
    /// values then, and the action centre in UV.
    Tool {
        handle: Handle,
        start: Pos2,
        params: Box<operandi_core::Parameters>,
        pivot: DVec2,
        constrain: bool,
        step: bool,
    },
    /// Paint selection (UVS-1 section 4): what's passed over is added
    /// (Ctrl: removed).
    Paint {
        remove: bool,
    },
    /// Lasso (right-drag): the outline so far.
    Lasso {
        path: Vec<Pos2>,
    },
    /// Element Move dragging a UV element (UVT-131).
    Tweak {
        grab: operandi_core::TweakGrab,
        start: Pos2,
        base: Box<operandi_core::Parameters>,
        /// Dragging a clicked element's U (0) or V (1) arrow.
        axis: Option<usize>,
    },
    /// Element Move's right-drag sizing Range, from where it started.
    TweakRange {
        last: Pos2,
        center: Pos2,
    },
    /// Smooth's and Slide's drag anywhere: Strength, Slide amount.
    Haul {
        start: Pos2,
        base: Box<operandi_core::Parameters>,
    },
    /// Box Transform (UVT-43): what the press grabbed, where, the tool's
    /// values then and the box at the session start.
    Boxed {
        grab: tools::BoxGrab,
        start: Pos2,
        base: Box<operandi_core::Parameters>,
        bounds: (DVec2, DVec2),
    },
    /// A Split and Sew brush stroke (UVT-63): the mesh points under it so
    /// far; `flip` (Shift at the press) swaps Split and Sew.
    Brush {
        points: std::collections::BTreeSet<u32>,
        flip: bool,
    },
    /// Dragging Relax pin `index` (UVS-1 §3 Interactive).
    Pin {
        index: usize,
        base: Box<operandi_core::Parameters>,
        start: Pos2,
    },
    /// The brush's right-drag sizing it.
    BrushSize {
        start: f32,
        from: Pos2,
    },
    /// Soft Drag's right-drag sizing its screen circle.
    SoftSize {
        start: f64,
        from: Pos2,
    },
    /// A press that does nothing (Element Move over empty space, TWK-23).
    Nothing,
}

/// A UV transform handle (UVE-1 section 2: U axis, V axis, a centre handle
/// for both, a ring for Rotate).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Handle {
    /// Off every handle: the tool's drag-anywhere gesture.
    Free,
    Center,
    MoveU,
    MoveV,
    Ring,
    ScaleU,
    ScaleV,
    /// Scale's centre handle: both axes evenly.
    ScaleAll,
}

/// Handle lengths in screen points.
const ARROW: f32 = 80.;
const RING: f32 = 58.;
const SCALE_BOX: f32 = 44.;
const GRAB: f32 = 7.;

impl Default for UvView {
    fn default() -> Self {
        Self {
            center: DVec2::splat(0.5),
            zoom: START_ZOOM,
            show_overlap: false,
            show_distortion: false,
            show_checker: false,
            show_flipped: false,
            coverage: std::cell::Cell::new(None),
            drag: None,
            rect: Rect::NOTHING,
            tweak_hover: None,
            tweak_clicked: None,
            box_now: None,
        }
    }
}

impl UvView {
    pub(super) fn to_screen(&self, r: Rect, uv: DVec2) -> Pos2 {
        let d = (uv - self.center) * self.zoom;
        pos2(r.center().x + d.x as f32, r.center().y - d.y as f32)
    }

    pub(super) fn to_uv(&self, r: Rect, p: Pos2) -> DVec2 {
        let d = p - r.center();
        self.center + DVec2::new(d.x as f64, -d.y as f64) / self.zoom
    }

    /// Zoom by `factor` keeping `anchor` (a UV point) where it is on screen.
    pub(super) fn zoom_about(&mut self, factor: f64, anchor: DVec2) {
        let zoom = (self.zoom * factor).clamp(1.0, 1.0e7);
        let k = self.zoom / zoom;
        self.center = anchor + (self.center - anchor) * k;
        self.zoom = zoom;
    }

    /// Frame the UV box `lo..hi` with a margin.
    pub(super) fn fit(&mut self, r: Rect, lo: DVec2, hi: DVec2) {
        let size = (hi - lo).max(DVec2::splat(1e-6));
        let room = DVec2::new(r.width() as f64, r.height() as f64) * 0.85;
        self.zoom = (room.x / size.x).min(room.y / size.y).clamp(1.0, 1.0e7);
        self.center = (lo + hi) * 0.5;
    }
}

/// What the UV view shows of the active mesh this frame.
pub(super) struct UvGeometry {
    /// Per polygon with UVs in the map: (polygon index, points, UVs).
    pub polygons: Vec<(u32, Vec<u32>, Vec<DVec2>)>,
    /// Discontinuous edges, as UV segments.
    pub seams: Vec<(DVec2, DVec2)>,
}

/// The polygons of `mesh` with UVs in `map` at every corner: (polygon,
/// its points, their UVs).
pub(super) fn uv_polygons(
    mesh: &operandi_core::Mesh,
    map: &str,
) -> Vec<(u32, Vec<u32>, Vec<DVec2>)> {
    let Some(rows) = mesh.uv_maps.get(map) else {
        return Vec::new();
    };
    let mut polygons = Vec::new();
    for (f, face) in mesh.faces.iter().enumerate() {
        let Some(row) = rows.get(f) else { continue };
        if row.len() != face.vertices.len() || row.iter().any(Option::is_none) {
            continue;
        }
        let uv: Vec<DVec2> = row.iter().map(|p| p.unwrap()).collect();
        polygons.push((f as u32, face.vertices.clone(), uv));
    }
    polygons
}

/// HT-6 Show UV Distortion: each polygon's UV view distortion colour
/// (`distortion_colours`, UVV-03) in `map`, by polygon index; `None` for
/// polygons without UVs there. The 3D view tints with exactly these, so
/// the two views always agree.
pub(crate) fn polygon_distortion(mesh: &operandi_core::Mesh, map: &str) -> Vec<Option<Color32>> {
    let polygons = uv_polygons(mesh, map);
    let mut out = vec![None; mesh.faces.len()];
    for ((f, _, _), c) in polygons.iter().zip(distortion_colours(mesh, &polygons)) {
        out[*f as usize] = Some(c);
    }
    out
}

/// The polygons of `mesh` that have UVs in `map`, and its seam segments.
pub(super) fn uv_geometry(mesh: &operandi_core::Mesh, map: &str) -> UvGeometry {
    let polygons = uv_polygons(mesh, map);
    let mut edges: HashMap<(u32, u32), Vec<(DVec2, DVec2)>> = HashMap::new();
    for (_, vertices, uv) in &polygons {
        let n = uv.len();
        for i in 0..n {
            let (a, b) = (vertices[i], vertices[(i + 1) % n]);
            let (ua, ub) = (uv[i], uv[(i + 1) % n]);
            let (key, pair) = if a < b {
                ((a, b), (ua, ub))
            } else {
                ((b, a), (ub, ua))
            };
            edges.entry(key).or_default().push(pair);
        }
    }
    let close = |a: DVec2, b: DVec2| (a - b).abs().max_element() <= 1e-6;
    let mut seams = Vec::new();
    for uses in edges.values() {
        if uses.len() < 2 {
            continue;
        }
        let first = uses[0];
        if uses
            .iter()
            .any(|u| !close(u.0, first.0) || !close(u.1, first.1))
        {
            seams.extend(uses.iter().copied());
        }
    }
    UvGeometry { polygons, seams }
}

/// Fan triangles of a polygon's UVs.
fn fan(uv: &[DVec2]) -> impl Iterator<Item = [DVec2; 3]> + '_ {
    (1..uv.len().saturating_sub(1)).map(move |i| [uv[0], uv[i], uv[i + 1]])
}

fn tri_overlap(a: &[DVec2; 3], b: &[DVec2; 3]) -> bool {
    fn apart(p: &[DVec2; 3], q: &[DVec2; 3]) -> bool {
        for i in 0..3 {
            let e = p[(i + 1) % 3] - p[i];
            let n = DVec2::new(-e.y, e.x);
            let pr = |pts: &[DVec2; 3]| {
                let (mut lo, mut hi) = (f64::MAX, f64::MIN);
                for v in pts {
                    let d = n.dot(*v - p[i]);
                    lo = lo.min(d);
                    hi = hi.max(d);
                }
                (lo, hi)
            };
            let ((plo, phi), (qlo, qhi)) = (pr(p), pr(q));
            let tol = 1e-9 * (1.0 + n.length_squared());
            if phi <= qlo + tol || qhi <= plo + tol {
                return true;
            }
        }
        false
    }
    !apart(a, b) && !apart(b, a)
}

/// Polygons whose UVs overlap another polygon's (UVV-02), by triangles,
/// found through a grid of buckets.
pub(super) fn overlapping(polygons: &[(u32, Vec<u32>, Vec<DVec2>)]) -> Vec<bool> {
    let tris: Vec<(usize, [DVec2; 3])> = polygons
        .iter()
        .enumerate()
        .flat_map(|(k, p)| fan(&p.2).map(move |t| (k, t)))
        .collect();
    let mut out = vec![false; polygons.len()];
    if tris.is_empty() {
        return out;
    }
    let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
    for (_, t) in &tris {
        for p in t {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    let n = ((tris.len() as f64).sqrt().ceil() as usize).clamp(1, 256);
    let size = (hi - lo).max(DVec2::splat(1e-12));
    let cell = |p: DVec2| {
        let c = ((p - lo) / size * n as f64).floor();
        ((c.x as usize).min(n - 1), (c.y as usize).min(n - 1))
    };
    let mut grid: Vec<Vec<usize>> = vec![Vec::new(); n * n];
    for (i, (_, t)) in tris.iter().enumerate() {
        let (mut tl, mut th) = (t[0].min(t[1]).min(t[2]), t[0].max(t[1]).max(t[2]));
        tl = tl.max(lo);
        th = th.min(hi);
        let (a, b) = (cell(tl), cell(th));
        for y in a.1..=b.1 {
            for x in a.0..=b.0 {
                grid[y * n + x].push(i);
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    for bucket in &grid {
        for (j, &a) in bucket.iter().enumerate() {
            for &b in &bucket[j + 1..] {
                let (pa, pb) = (tris[a].0, tris[b].0);
                if pa == pb || (out[pa] && out[pb]) || !seen.insert((a.min(b), a.max(b))) {
                    continue;
                }
                // Polygons sharing a point with the same UV (neighbours)
                // only touch along edges, which the strict test ignores.
                if tri_overlap(&tris[a].1, &tris[b].1) {
                    out[pa] = true;
                    out[pb] = true;
                }
            }
        }
    }
    out
}

/// Each polygon's distortion colour: its UV-to-3D area ratio against the
/// whole map's (UVV-03).
pub(super) fn distortion_colours(
    mesh: &operandi_core::Mesh,
    polygons: &[(u32, Vec<u32>, Vec<DVec2>)],
) -> Vec<Color32> {
    let area3 = |v: &[u32]| {
        let p: Vec<DVec3> = v.iter().map(|&i| mesh.vertices[i as usize]).collect();
        (1..p.len().saturating_sub(1))
            .map(|i| 0.5 * (p[i] - p[0]).cross(p[i + 1] - p[0]).length())
            .sum::<f64>()
    };
    let area_uv = |u: &[DVec2]| {
        fan(u)
            .map(|t| 0.5 * (t[1] - t[0]).perp_dot(t[2] - t[0]).abs())
            .sum::<f64>()
    };
    let pairs: Vec<(f64, f64)> = polygons
        .iter()
        .map(|p| (area3(&p.1), area_uv(&p.2)))
        .collect();
    let (t3, tu) = pairs.iter().fold((0.0, 0.0), |s, p| (s.0 + p.0, s.1 + p.1));
    let total = if t3 > 0.0 { tu / t3 } else { 1.0 };
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| {
        let c = |i: usize| ((a[i] + (b[i] - a[i]) * t) * 255.) as u8;
        Color32::from_rgba_unmultiplied(c(0), c(1), c(2), 150)
    };
    pairs
        .iter()
        .map(|&(a3, au)| {
            if a3 <= 0.0 || total <= 0.0 {
                return mix(NEUTRAL, NEUTRAL, 0.);
            }
            // Doubling or halving the share is full colour.
            let r = ((au / a3) / total).max(1e-12).log2().clamp(-1., 1.) as f32;
            if r < 0. {
                mix(NEUTRAL, SMALL, -r)
            } else {
                mix(NEUTRAL, LARGE, r)
            }
        })
        .collect()
}

fn point_in_polygon(p: DVec2, poly: &[DVec2]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Whether `c` is in the selection.
fn component_selected(sel: &operandi_core::Selection, c: operandi_core::Component) -> bool {
    use operandi_core::Component;
    match c {
        Component::Vertex(v) => sel.vertices.contains(&v),
        Component::Edge(a, b) => sel
            .edges
            .iter()
            .any(|&(x, y)| (x.min(y), x.max(y)) == (a.min(b), a.max(b))),
        Component::Polygon(f) => sel.faces.contains(&f),
        Component::Item => false,
    }
}

fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-12)).clamp(0., 1.);
    (a + ab * t - p).length()
}

impl Operandi {
    /// Whether `view` shows UV space.
    pub(super) fn is_uv_view(&self, view: u32) -> bool {
        self.uv_views.contains_key(&view) || self.is_uv_pane(view)
    }

    /// Whether `view` is a UV panel in the layout (`Pane::Uv`), as opposed
    /// to a 3D view switched to UV space.
    fn is_uv_pane(&self, view: u32) -> bool {
        matches!(
            self.views.view_pane(view),
            Some(operandi_layout::Pane::Uv(_))
        )
    }

    /// View > UV View: the active view switches between 3D and UV.
    pub(super) fn toggle_uv_view(&mut self) {
        let view = self.active_view;
        if self.is_uv_pane(view) {
            self.status = "This is a UV panel; add a 3D view beside it for 3D".into();
            return;
        }
        if self.uv_views.remove(&view).is_none() {
            self.uv_views.insert(view, UvView::default());
            self.status =
                "UV View: U to the right, V up; Alt+drag pans, the wheel zooms, A fits".into();
        } else {
            self.status = "3D View".into();
        }
    }

    /// The map a UV view shows on the active mesh: its selected map (UVE-1
    /// section 1), or the one an open UV tool edits.
    fn uv_view_map(&self, _view: &UvView) -> Option<String> {
        if let Some(space) = self
            .editor
            .session
            .live
            .as_ref()
            .and_then(|l| l.params.uv.as_ref())
        {
            return Some(space.map.clone());
        }
        let mesh = &self.editor.scene.active()?.mesh;
        (!mesh.uv_maps.is_empty()).then(|| target_uv_map(mesh, None))
    }

    /// A/Shift+A in a UV view: frame all UVs or the selection's. Returns
    /// false when the active view isn't a UV view.
    pub(super) fn uv_view_frame(&mut self, selection: bool) -> bool {
        let view = self.active_view;
        if !self.is_uv_view(view) {
            return false;
        }
        let uv = self.uv_views.entry(view).or_default().clone();
        let Some(item) = self.editor.scene.active() else {
            return true;
        };
        let Some(map) = self.uv_view_map(&uv) else {
            return true;
        };
        let geometry = uv_geometry(&item.mesh, &map);
        let sel = &self.editor.scene.selection;
        let faces: std::collections::BTreeSet<u32> = sel.faces.iter().copied().collect();
        let points: std::collections::BTreeSet<u32> = sel.vertices.iter().copied().collect();
        let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
        for (f, verts, uvs) in &geometry.polygons {
            for (p, q) in verts.iter().zip(uvs) {
                let take = !selection
                    || match sel.mode {
                        Mode::Polygon => faces.contains(f),
                        Mode::Vertex => points.contains(p),
                        Mode::Edge => sel.edges.iter().any(|&(a, b)| a == *p || b == *p),
                        Mode::Item => true,
                    };
                if take {
                    lo = lo.min(*q);
                    hi = hi.max(*q);
                }
            }
        }
        if lo.x > hi.x {
            // Nothing to frame: the 0-1 tile.
            lo = DVec2::ZERO;
            hi = DVec2::ONE;
        }
        if let Some(v) = self.uv_views.get_mut(&view) {
            let r = if v.rect.is_positive() {
                v.rect
            } else {
                self.viewport_rect
            };
            v.fit(r, lo, hi);
        }
        true
    }

    /// . and , in a UV view. Returns false when the active view isn't one.
    pub(super) fn uv_view_zoom_step(&mut self, zoom_in: bool) -> bool {
        if !self.is_uv_view(self.active_view) {
            return false;
        }
        let v = self.uv_views.entry(self.active_view).or_default();
        let c = v.center;
        v.zoom_about(if zoom_in { 1.25 } else { 0.8 }, c);
        true
    }

    /// One frame of a UV view; `active` views take input.
    pub(super) fn uv_view_ui(&mut self, ui: &mut Ui, view: u32, active: bool) {
        // The stock UV scheme's colours (colors-2026-11-16 section 3).
        let _scheme = theme::viewport::use_scheme(theme::viewport::ViewScheme::Uv);
        // The UV Editor window keeps a tool strip down its right side
        // (UVT-200, uv_editor.rs).
        let whole = ui.available_rect_before_wrap();
        let strip = self.uv_editor_strip_width(view);
        let size = vec2((whole.width() - strip).max(1.), whole.height());
        let (r, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let mut uv = self.uv_views.get(&view).cloned().unwrap_or_default();
        uv.rect = r;
        if !self.is_uv_editor(view) {
            self.view_dot(ui, view, r, whole);
        }
        if active {
            self.viewport_rect = r;
            self.uv_view_input(ui, &response, r, &mut uv);
        }
        let painter = ui.painter_at(r);
        self.paint_uv_view(&painter, r, &uv);
        self.uv_view_header(ui, r, &mut uv);
        self.uv_views.insert(view, uv);
        if strip > 0. {
            self.uv_editor_strip(ui, whole);
        }
    }

    fn uv_view_input(&mut self, ui: &Ui, response: &Response, r: Rect, uv: &mut UvView) {
        if self.modal_owned_input
            || egui::Popup::is_any_open(ui.ctx())
            || self
                .dots
                .drag
                .as_ref()
                .is_some_and(|drag| drag.dot == panel_dot::Dot::View(self.drawing_view))
        {
            uv.drag = None;
            return;
        }
        self.sync_uv_tool_space(true);
        let (mods, hover) = ui.input(|i| (i.modifiers, i.pointer.hover_pos()));
        // Wheel: zoom about the pointer.
        if let Some(p) = hover.filter(|p| r.contains(*p)) {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0. {
                uv.zoom_about((scroll as f64 / 200.).exp(), uv.to_uv(r, p));
            }
        }
        // PNT-215: a paint tool paints the image in the UV view.
        #[cfg(feature = "painting")]
        if self.uv_paint_input(ui, r, uv) {
            return;
        }
        let press = response.interact_pointer_pos().unwrap_or(r.center());
        if response.drag_started_by(egui::PointerButton::Primary) {
            // Where the press was, not where the drag threshold was passed.
            let at = ui
                .input(|i| i.pointer.press_origin())
                .filter(|p| r.contains(*p))
                .unwrap_or(press);
            uv.drag = Some(if mods.alt && !self.uv_tool_live() {
                if mods.command || mods.ctrl {
                    Drag::Zoom {
                        last: at,
                        anchor: uv.to_uv(r, at),
                    }
                } else {
                    Drag::Pan { last: at }
                }
            } else if self.uv_tool_is(Tool::Tweak) {
                // UVT-131: press on a UV element and drag it; a press on a
                // clicked element's handles drags that one (UVT-Q13).
                let handle = self.uv_tweak_handle_at(r, uv, at);
                match handle.or_else(|| self.uv_tweak_pick(r, uv, at).map(|g| (g, None))) {
                    Some((grab, axis)) => {
                        if handle.is_none() {
                            uv.tweak_clicked = None;
                        }
                        let base = self
                            .editor
                            .session
                            .live
                            .as_ref()
                            .expect("UV tool")
                            .params
                            .clone();
                        self.hold_pointer();
                        self.clear_fields();
                        Drag::Tweak {
                            grab,
                            start: at,
                            base: Box::new(base),
                            axis,
                        }
                    }
                    None => {
                        self.status = "Element Move: press on a UV point, edge or polygon".into();
                        Drag::Nothing
                    }
                }
            } else if let Some(drag) = self.uv_pin_press(r, uv, at, mods) {
                self.hold_pointer();
                drag
            } else if self.uv_brush.is_some() && self.editor.session.live.is_none() {
                let mut points = std::collections::BTreeSet::new();
                self.uv_brush_collect(r, uv, at, &mut points);
                Drag::Brush {
                    points,
                    flip: mods.shift,
                }
            } else if let Some(drag) = self.uv_box_press(r, uv, at, false) {
                self.hold_pointer();
                drag
            } else if self.uv_haul_tool() {
                let base = self
                    .editor
                    .session
                    .live
                    .as_ref()
                    .expect("UV tool")
                    .params
                    .clone();
                self.hold_pointer();
                Drag::Haul {
                    start: at,
                    base: Box::new(base),
                }
            } else if let Some(pivot) = self.uv_tool_pivot() {
                // Soft Drag: the press centres its circle first (FO-43).
                if mods.is_none() {
                    self.uv_soft_drag_press(r, uv, at);
                }
                // UVE-1 section 2: a drag drives the UV tool. Ctrl/Cmd+Shift
                // tears the selection off for this drag (UVT-41); Ctrl alone
                // constrains Move to one axis and steps Rotate.
                let handle = self.uv_handle_at(r, uv, pivot, at);
                let mut params = self
                    .editor
                    .session
                    .live
                    .as_ref()
                    .expect("UV tool")
                    .params
                    .clone();
                let tear = (mods.command || mods.ctrl) && mods.shift;
                if let Some(space) = params.uv.as_mut()
                    && tear
                    && !space.tear_off
                {
                    space.tear_off = true;
                    space.proxy = None;
                }
                self.hold_pointer();
                Drag::Tool {
                    handle,
                    start: at,
                    params: Box::new(params),
                    pivot,
                    constrain: (mods.command || mods.ctrl) && !tear,
                    step: mods.command || mods.ctrl,
                }
            } else if mods.alt {
                Drag::Pan { last: at }
            } else {
                // Paint selection: plain starts over, Shift adds, Ctrl removes.
                self.run(Command::BeginSelection);
                let remove = mods.ctrl || mods.command;
                if !mods.shift && !remove {
                    self.run(Command::ClearSelection);
                }
                self.uv_paint_at(r, uv, at, remove);
                Drag::Paint { remove }
            });
        }
        if response.drag_started_by(egui::PointerButton::Secondary) && !mods.alt {
            let at = ui
                .input(|i| i.pointer.press_origin())
                .filter(|p| r.contains(*p))
                .unwrap_or(press);
            let soft = self
                .screen_layer()
                .filter(|_| self.soft_drag_open() && self.uv_tool_live());
            let boxed = self.uv_box_press(r, uv, at, true);
            if boxed.is_some() {
                self.hold_pointer();
            }
            uv.drag = Some(if let Some(drag) = boxed {
                drag
            } else if let Some(b) = self.uv_brush.filter(|_| self.editor.session.live.is_none()) {
                Drag::BrushSize {
                    start: b.size,
                    from: at,
                }
            } else if self.uv_tool_is(Tool::Tweak) {
                // TWR-06: Element Move's right-drag sizes Range.
                Drag::TweakRange {
                    last: at,
                    center: at,
                }
            } else if let Some((_, Falloff::Screen(s))) = soft {
                Drag::SoftSize {
                    start: s.size,
                    from: at,
                }
            } else {
                Drag::Lasso { path: vec![at] }
            });
        }
        if (response.dragged() || response.drag_stopped())
            && let Some(p) = response.interact_pointer_pos()
        {
            match uv.drag.clone() {
                Some(Drag::Pan { last }) => {
                    let d = p - last;
                    uv.center -= DVec2::new(d.x as f64, -d.y as f64) / uv.zoom;
                    uv.drag = Some(Drag::Pan { last: p });
                }
                Some(Drag::Zoom { last, anchor }) => {
                    // Left to right zooms in (the Zoom Drag Axis default).
                    let d = (p.x - last.x) as f64;
                    uv.zoom_about((d / 150.).exp(), anchor);
                    uv.drag = Some(Drag::Zoom { last: p, anchor });
                }
                Some(Drag::Tool {
                    handle,
                    start,
                    params,
                    pivot,
                    constrain,
                    step,
                }) => {
                    let constrain = constrain
                        || ui.input(|i| i.modifiers.command || i.modifiers.ctrl)
                            && !ui.input(|i| i.modifiers.shift);
                    let p = self
                        .uv_drag_params(r, uv, handle, start, p, &params, pivot, constrain, step);
                    if let Some(p) = p {
                        self.run(Command::Preview(p));
                    }
                }
                Some(Drag::Paint { remove }) => self.uv_paint_at(r, uv, p, remove),
                Some(Drag::Tweak {
                    grab,
                    start,
                    base,
                    axis,
                }) => self.uv_tweak_drag(r, uv, grab, &base, start, p, axis),
                Some(Drag::TweakRange { last, center }) => {
                    self.uv_tweak_range_drag(uv, last, p);
                    uv.drag = Some(Drag::TweakRange { last: p, center });
                }
                Some(Drag::Haul { start, base }) => self.uv_haul_drag(uv, &base, start, p),
                Some(Drag::SoftSize { start, from }) => {
                    self.uv_soft_size_drag(r, uv, start, from, p)
                }
                Some(Drag::Pin { index, base, start }) => {
                    let mut params = (*base).clone();
                    let d = uv.to_uv(r, p) - uv.to_uv(r, start);
                    if let Some(pin) = params.uv_relax.pins.get_mut(index) {
                        pin.to += d;
                    }
                    self.run(Command::Preview(params));
                    self.clear_fields();
                }
                Some(Drag::Brush { mut points, flip }) => {
                    self.uv_brush_collect(r, uv, p, &mut points);
                    uv.drag = Some(Drag::Brush { points, flip });
                }
                Some(Drag::BrushSize { start, from }) => {
                    if let Some(b) = self.uv_brush.as_mut() {
                        b.size = (start + (p.x - from.x)).max(2.);
                        self.status = format!("Brush size: {:.0} px", b.size);
                    }
                }
                Some(Drag::Boxed {
                    grab,
                    start,
                    base,
                    bounds,
                }) => {
                    let mods = ui.input(|i| i.modifiers);
                    self.uv_box_drag(r, uv, grab, start, p, &base, bounds, mods);
                }
                Some(Drag::Lasso { mut path }) => {
                    if path.last().is_none_or(|l| l.distance(p) > 1.) {
                        path.push(p);
                    }
                    uv.drag = Some(Drag::Lasso { path });
                }
                Some(Drag::Nothing) | None => {}
            }
        }
        if response.drag_stopped() {
            match uv.drag.take() {
                Some(Drag::Tool { .. } | Drag::Tweak { .. }) => {
                    // As in 3D, each Move / Rotate / Scale / Transform /
                    // Element Move drag ends a step of the session: the
                    // tool's start scene becomes the result, so the UV
                    // proxy that picks, centres and falloffs read is made
                    // again from the moved UVs, and the next drag starts
                    // from them.
                    if self
                        .editor
                        .session
                        .live
                        .as_ref()
                        .is_some_and(|l| l.tool.is_transform())
                    {
                        self.run(Command::CommitTransformStep);
                    }
                    self.release_pointer()
                }
                Some(Drag::Haul { .. }) | Some(Drag::Pin { .. }) => self.release_pointer(),
                Some(Drag::Brush { points, flip }) => {
                    self.uv_brush_stroke(points.into_iter().collect(), flip)
                }
                Some(Drag::Boxed { .. }) => {
                    self.run(Command::CommitTransformStep);
                    self.release_pointer();
                }
                Some(Drag::Paint { .. }) => self.run(Command::EndSelection { cancel: false }),
                Some(Drag::Lasso { path }) => self.uv_lasso(r, uv, &path, mods),
                _ => {}
            }
        }
        uv.box_now = self.uv_box_now().map(|(c, p, _)| (c, p));
        // TWK-02: what a press would grab, outlined.
        uv.tweak_hover = if uv.drag.is_none() && self.uv_tool_is(Tool::Tweak) {
            hover
                .filter(|p| r.contains(*p))
                .and_then(|p| self.uv_tweak_pick(r, uv, p))
                .map(|g| g.element)
        } else {
            None
        };
        if response.clicked()
            && let Some(p) = response.interact_pointer_pos()
            && self.uv_place_element_center(r, uv, p)
        {
            return;
        }
        // Relax Interactive: a click pins a UV point (Ctrl/Cmd removes).
        if response.clicked()
            && let Some(p) = response.interact_pointer_pos()
            && self.uv_pin_click(r, uv, p, mods.command || mods.ctrl)
        {
            return;
        }
        // UVT-131 / UVT-Q13: a click with no drag puts handles on the
        // element under the pointer (off empty space clears them).
        if response.clicked()
            && self.uv_tool_is(Tool::Tweak)
            && let Some(p) = response.interact_pointer_pos()
        {
            uv.tweak_clicked = self.uv_tweak_pick(r, uv, p);
            if uv.tweak_clicked.is_some() {
                self.status = "Element Move: drag the U or V arrow, or the centre square".into();
            }
        }
        if !self.uv_tool_is(Tool::Tweak) {
            uv.tweak_clicked = None;
        }
        if response.clicked()
            && !mods.alt
            && !self.uv_tool_live()
            && let Some(p) = response.interact_pointer_pos()
        {
            self.uv_view_click(r, uv, p, mods);
        }
    }

    /// Whether a transform tool is open in UV space.
    pub(super) fn uv_tool_live(&self) -> bool {
        self.editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.params.uv.is_some())
    }

    /// Put an open, untouched Move / Rotate / Scale / Transform in UV space
    /// while a UV view is active (`in_uv`), and back in 3D when a 3D view
    /// is; once it has changed something it stays where it started.
    pub(super) fn sync_uv_tool_space(&mut self, in_uv: bool) {
        let Some(live) = self.editor.session.live.as_ref() else {
            return;
        };
        // Smooth shows its result at once; the core redoes it in the new
        // space. Any other change keeps the session where it started.
        if !operandi_core::uv_pipe::runs_on_uvs(live.tool)
            || live.params.uv.as_ref().is_some_and(|s| s.pinned)
            || (self.editor.scene != live.base && !live.tool.shows_result_at_once())
            || self.pointer_held
        {
            return;
        }
        let has_map = self
            .editor
            .scene
            .active()
            .is_some_and(|i| !i.mesh.uv_maps.is_empty());
        match (in_uv && has_map, live.params.uv.is_some()) {
            (true, false) => {
                let _ = self.editor.dispatch(Command::UvToolSpace(Some(None)));
            }
            (false, true) => {
                let _ = self.editor.dispatch(Command::UvToolSpace(None));
            }
            _ => {}
        }
    }

    /// The open UV tool's action centre in UV (UVT-133): the tool's action
    /// frame, which the core works out in UV space. `None` when no tool is
    /// open in UV space.
    pub(super) fn uv_tool_pivot(&self) -> Option<DVec2> {
        let live = self.editor.session.live.as_ref()?;
        live.params.uv.as_ref()?;
        if !matches!(
            live.tool,
            Tool::Move | Tool::Rotate | Tool::Scale | Tool::Transform
        ) {
            return None;
        }
        Some(live.action_frame?.center.truncate())
    }

    /// Where the handles sit now: the pivot moved by the tool's offset.
    fn uv_handle_center(&self, pivot: DVec2) -> DVec2 {
        let Some(live) = self.editor.session.live.as_ref() else {
            return pivot;
        };
        let (offset, _, _) = operandi_core::uv_edit::tool_values(live.tool, &live.params);
        pivot + offset
    }

    /// The handle under `p` (screen), or `Free`.
    fn uv_handle_at(&self, r: Rect, uv: &UvView, pivot: DVec2, p: Pos2) -> Handle {
        let Some(tool) = self.editor.session.live.as_ref().map(|l| l.tool) else {
            return Handle::Free;
        };
        let c = uv.to_screen(r, self.uv_handle_center(pivot));
        let near = |q: Pos2| q.distance(p) <= GRAB + 2.;
        let on = |a: Pos2, b: Pos2| segment_distance(p, a, b) <= GRAB;
        let moves = matches!(tool, Tool::Move | Tool::Transform);
        let scales = matches!(tool, Tool::Scale | Tool::Transform);
        let turns = matches!(tool, Tool::Rotate | Tool::Transform);
        if near(c) {
            return if tool == Tool::Scale {
                Handle::ScaleAll
            } else if tool == Tool::Rotate {
                Handle::Free
            } else {
                Handle::Center
            };
        }
        if scales {
            let len = if tool == Tool::Scale {
                ARROW
            } else {
                SCALE_BOX
            };
            if near(c + vec2(len, 0.)) {
                return Handle::ScaleU;
            }
            if near(c + vec2(0., -len)) {
                return Handle::ScaleV;
            }
        }
        if moves {
            if on(c, c + vec2(ARROW, 0.)) {
                return Handle::MoveU;
            }
            if on(c, c + vec2(0., -ARROW)) {
                return Handle::MoveV;
            }
        }
        if turns && (p.distance(c) - RING).abs() <= GRAB {
            return Handle::Ring;
        }
        if tool == Tool::Scale {
            if on(c, c + vec2(ARROW, 0.)) {
                return Handle::ScaleU;
            }
            if on(c, c + vec2(0., -ARROW)) {
                return Handle::ScaleV;
            }
        }
        Handle::Free
    }

    /// The tool values for a drag from `start` to `p` on `handle`, added to
    /// the values the tool had at the press (`base`).
    #[allow(clippy::too_many_arguments)]
    fn uv_drag_params(
        &self,
        r: Rect,
        uv: &UvView,
        handle: Handle,
        start: Pos2,
        p: Pos2,
        base: &operandi_core::Parameters,
        pivot: DVec2,
        constrain: bool,
        step: bool,
    ) -> Option<operandi_core::Parameters> {
        let tool = self.editor.session.live.as_ref()?.tool;
        let mut out = base.clone();
        let (a, b) = (uv.to_uv(r, start), uv.to_uv(r, p));
        let centre = pivot + operandi_core::uv_edit::tool_values(tool, base).0;
        let mut offset = b - a;
        // UVT-Q05 (provisional): Snapping on snaps UV moves to the 0.1 grid.
        if let Some(space) = out.uv.as_mut() {
            space.grid_snap = self.snapping.enabled.then_some(0.1);
        }
        let turn = |out: &mut operandi_core::Parameters, degrees: f64| {
            let degrees = if step {
                self.handle_preferences.round_to_snap_angle(degrees)
            } else {
                degrees
            };
            match tool {
                Tool::Rotate => out.delta.z = base.delta.z + degrees,
                _ => out.rotation_delta.z = base.rotation_delta.z + degrees,
            }
        };
        let angle = || {
            let (u, v) = (a - centre, b - centre);
            if u.length() < 1e-12 || v.length() < 1e-12 {
                return 0.;
            }
            v.y.atan2(v.x).to_degrees() - u.y.atan2(u.x).to_degrees()
        };
        let scale_by = |out: &mut operandi_core::Parameters, f: DVec2| {
            let f = f.max(DVec2::splat(1e-3));
            match tool {
                Tool::Scale => {
                    out.delta.x = base.delta.x * f.x;
                    out.delta.y = base.delta.y * f.y;
                }
                _ => {
                    out.scale_delta.x = base.scale_delta.x * f.x;
                    out.scale_delta.y = base.scale_delta.y * f.y;
                }
            }
        };
        let ratio = |axis: usize| {
            let (u, v) = (a - centre, b - centre);
            if u[axis].abs() < 1e-9 {
                1.
            } else {
                v[axis] / u[axis]
            }
        };
        match (tool, handle) {
            (Tool::Move | Tool::Transform, Handle::Free | Handle::Center) => {
                if constrain {
                    if offset.x.abs() >= offset.y.abs() {
                        offset.y = 0.;
                    } else {
                        offset.x = 0.;
                    }
                }
                out.delta.x = base.delta.x + offset.x;
                out.delta.y = base.delta.y + offset.y;
            }
            (Tool::Move | Tool::Transform, Handle::MoveU) => out.delta.x = base.delta.x + offset.x,
            (Tool::Move | Tool::Transform, Handle::MoveV) => out.delta.y = base.delta.y + offset.y,
            (Tool::Rotate, _) | (Tool::Transform, Handle::Ring) => turn(&mut out, angle()),
            (Tool::Scale | Tool::Transform, Handle::ScaleU) => {
                scale_by(&mut out, DVec2::new(ratio(0), 1.))
            }
            (Tool::Scale | Tool::Transform, Handle::ScaleV) => {
                scale_by(&mut out, DVec2::new(1., ratio(1)))
            }
            (Tool::Scale | Tool::Transform, Handle::ScaleAll) => {
                let (u, v) = (a - centre, b - centre);
                let k = if u.length() < 1e-9 {
                    1.
                } else {
                    v.length() / u.length()
                };
                scale_by(&mut out, DVec2::splat(k));
            }
            (Tool::Scale, _) => {
                // Drag anywhere: right is bigger, evenly on both axes.
                let k = (((p.x - start.x) as f64) / 150.).exp();
                scale_by(&mut out, DVec2::splat(k));
            }
            _ => return None,
        }
        Some(out)
    }

    /// The component under `p` in the UV view, in the selection mode.
    fn uv_hit(&self, r: Rect, uv: &UvView, p: Pos2) -> Option<operandi_core::Component> {
        let item = self.editor.scene.active()?;
        let map = self.uv_view_map(uv)?;
        let geometry = uv_geometry(&item.mesh, &map);
        let at = uv.to_uv(r, p);
        match self.editor.scene.selection.mode {
            Mode::Polygon | Mode::Item => geometry
                .polygons
                .iter()
                .rev()
                .find(|(_, _, uvs)| point_in_polygon(at, uvs))
                .map(|(f, _, _)| operandi_core::Component::Polygon(*f)),
            Mode::Vertex => geometry
                .polygons
                .iter()
                .flat_map(|(_, v, q)| v.iter().zip(q))
                .map(|(&v, &q)| (v, (uv.to_screen(r, q) - p).length()))
                .filter(|(_, d)| *d <= 10.)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(v, _)| operandi_core::Component::Vertex(v)),
            Mode::Edge => geometry
                .polygons
                .iter()
                .flat_map(|(_, v, q)| {
                    (0..v.len()).map(move |i| {
                        let j = (i + 1) % v.len();
                        (v[i], v[j], q[i], q[j])
                    })
                })
                .map(|(a, b, qa, qb)| {
                    (
                        (a.min(b), a.max(b)),
                        segment_distance(p, uv.to_screen(r, qa), uv.to_screen(r, qb)),
                    )
                })
                .filter(|(_, d)| *d <= 8.)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|((a, b), _)| operandi_core::Component::Edge(a, b)),
        }
    }

    /// A click in a UV view selects the component under it (Shift adds,
    /// Ctrl removes), shared with the 3D views.
    fn uv_view_click(&mut self, r: Rect, uv: &UvView, p: Pos2, mods: egui::Modifiers) {
        let Some(item) = self.editor.scene.active() else {
            return;
        };
        let id = item.id;
        if self.uv_view_map(uv).is_none() {
            self.status = "This mesh has no UV map; UV Unwrap or New Map makes one".into();
            return;
        }
        if self.editor.scene.selection.mode == Mode::Item {
            self.status = "Pick Vertices, Edges or Polygons to select in the UV view".into();
            return;
        }
        let hit = self.uv_hit(r, uv, p);
        let (add, remove) = (mods.shift, mods.ctrl || mods.command);
        self.run(Command::BeginSelection);
        // The one selection-click path (AR-23), as in the 3D views.
        self.click_select(operandi_core::click_select::Click {
            hit: hit.map(|hit| (id, hit)),
            how: operandi_core::click_select::ClickHow::from_keys(add, remove),
            over_skipped_mesh: false,
        });
        self.run(Command::EndSelection { cancel: false });
    }

    /// One step of paint selection: the component under `p` joins the
    /// selection (or leaves it).
    fn uv_paint_at(&mut self, r: Rect, uv: &UvView, p: Pos2, remove: bool) {
        if self.editor.scene.selection.mode == Mode::Item {
            return;
        }
        let Some(id) = self.editor.scene.active().map(|i| i.id) else {
            return;
        };
        if let Some(hit) = self.uv_hit(r, uv, p) {
            let selected = component_selected(&self.editor.scene.selection, hit);
            if selected == remove {
                self.click_select(operandi_core::click_select::Click {
                    hit: Some((id, hit)),
                    how: operandi_core::click_select::ClickHow::from_keys(true, remove),
                    over_skipped_mesh: false,
                });
            }
        }
    }

    /// Right-drag lasso in a UV view (UVS-1 section 4): what lies wholly
    /// inside the outline is selected; Shift adds, Ctrl removes, plain
    /// starts over. The lasso shape is the 3D lasso's style.
    fn uv_lasso(&mut self, r: Rect, uv: &UvView, path: &[Pos2], mods: egui::Modifiers) {
        let region = lasso::simplify(&lasso::outline(self.lasso_style, path), 0.5);
        if !lasso::valid(&region) {
            return;
        }
        let Some(item) = self.editor.scene.active() else {
            return;
        };
        let id = item.id;
        let Some(map) = self.uv_view_map(uv) else {
            return;
        };
        let geometry = uv_geometry(&item.mesh, &map);
        let screen = |q: &[DVec2]| q.iter().map(|&p| uv.to_screen(r, p)).collect::<Vec<Pos2>>();
        let mut hits: Vec<operandi_core::Component> = Vec::new();
        match self.editor.scene.selection.mode {
            Mode::Polygon => {
                for (f, _, q) in &geometry.polygons {
                    if lasso::eligible(&region, &screen(q), false) {
                        hits.push(operandi_core::Component::Polygon(*f));
                    }
                }
            }
            Mode::Vertex => {
                let mut seen = std::collections::BTreeSet::new();
                for (_, v, q) in &geometry.polygons {
                    for (p, w) in v.iter().zip(q) {
                        if !seen.contains(p) && lasso::contains(&region, uv.to_screen(r, *w)) {
                            seen.insert(*p);
                            hits.push(operandi_core::Component::Vertex(*p));
                        }
                    }
                }
            }
            Mode::Edge => {
                let mut seen = std::collections::BTreeSet::new();
                for (_, v, q) in &geometry.polygons {
                    for i in 0..v.len() {
                        let j = (i + 1) % v.len();
                        let key = (v[i].min(v[j]), v[i].max(v[j]));
                        if !seen.contains(&key)
                            && lasso::eligible(&region, &screen(&[q[i], q[j]]), false)
                        {
                            seen.insert(key);
                            hits.push(operandi_core::Component::Edge(key.0, key.1));
                        }
                    }
                }
            }
            Mode::Item => {
                self.status = "Pick Vertices, Edges or Polygons to lasso in the UV view".into();
                return;
            }
        }
        let remove = mods.ctrl || mods.command;
        self.run(Command::BeginSelection);
        if !mods.shift && !remove {
            self.run(Command::ClearSelection);
        }
        for hit in hits {
            self.click_select(operandi_core::click_select::Click {
                hit: Some((id, hit)),
                how: operandi_core::click_select::ClickHow::from_keys(true, remove),
                over_skipped_mesh: false,
            });
        }
        self.run(Command::EndSelection { cancel: false });
    }

    fn paint_uv_view(&self, painter: &egui::Painter, r: Rect, uv: &UvView) {
        use theme::viewport as palette;
        painter.rect_filled(r, 0., palette::background());
        let tile = Rect::from_two_pos(uv.to_screen(r, DVec2::ZERO), uv.to_screen(r, DVec2::ONE));
        painter.rect_filled(tile, 0., palette::background_top());
        if uv.show_checker {
            self.paint_uv_checker(painter, r, uv);
        }
        // PNT-215: the paint target's image, flat behind the UV map.
        #[cfg(feature = "painting")]
        self.paint_uv_backdrop(painter, tile);
        // Grid every 0.1 over the tile (spec §4 Grid, extent 0-1).
        let grid = Stroke::new(1.0_f32, palette::grid().gamma_multiply(0.35));
        for k in 0..=10 {
            let t = k as f64 / 10.;
            let (a, b) = (
                uv.to_screen(r, DVec2::new(t, 0.)),
                uv.to_screen(r, DVec2::new(t, 1.)),
            );
            painter.line_segment([a, b], grid);
            let (a, b) = (
                uv.to_screen(r, DVec2::new(0., t)),
                uv.to_screen(r, DVec2::new(1., t)),
            );
            painter.line_segment([a, b], grid);
        }
        // Axis: U = 0 and V = 0, each in its own colour (UV scheme).
        let [u_axis, v_axis] = palette::uv_axes();
        painter.line_segment(
            [
                uv.to_screen(r, DVec2::new(-1e4, 0.)),
                uv.to_screen(r, DVec2::new(1e4, 0.)),
            ],
            Stroke::new(1.5_f32, u_axis),
        );
        painter.line_segment(
            [
                uv.to_screen(r, DVec2::new(0., -1e4)),
                uv.to_screen(r, DVec2::new(0., 1e4)),
            ],
            Stroke::new(1.5_f32, v_axis),
        );
        // Labels inside, every 0.5; the UDIM number at the tile's centre.
        let font = FontId::proportional(11.);
        let label = palette::label().gamma_multiply(0.8);
        for k in 0..=2 {
            let t = k as f64 * 0.5;
            painter.text(
                uv.to_screen(r, DVec2::new(t, 0.)) + vec2(3., -3.),
                Align2::LEFT_BOTTOM,
                format!("{t}"),
                font.clone(),
                label,
            );
            if k > 0 {
                painter.text(
                    uv.to_screen(r, DVec2::new(0., t)) + vec2(3., 3.),
                    Align2::LEFT_TOP,
                    format!("{t}"),
                    font.clone(),
                    label,
                );
            }
        }
        // UVT-112: the tile numbers, each at its tile's centre: 1001
        // always, the others once more than tile 1001 is in view.
        let (lo, hi) = (uv.to_uv(r, r.left_bottom()), uv.to_uv(r, r.right_top()));
        let shows_more = lo.x < 0. || lo.y < 0. || hi.x > 1. || hi.y > 1.;
        let (u0, u1) = (lo.x.floor().max(0.) as i64, hi.x.ceil().min(10.) as i64);
        let (v0, v1) = (lo.y.floor().max(0.) as i64, hi.y.ceil().min(99.) as i64);
        for tu in u0..u1 {
            for tv in v0..v1 {
                if (tu, tv) != (0, 0) && !shows_more {
                    continue;
                }
                if (u1 - u0) * (v1 - v0) > 60 {
                    break;
                }
                let centre = uv.to_screen(r, DVec2::new(tu as f64 + 0.5, tv as f64 + 0.5));
                painter.text(
                    centre,
                    Align2::CENTER_CENTER,
                    format!("{}", 1001 + tu + 10 * tv),
                    FontId::proportional(28.),
                    label.gamma_multiply(0.25),
                );
            }
        }
        let _ = tile;

        let Some(item) = self.editor.scene.active() else {
            return;
        };
        let Some(map) = self.uv_view_map(uv) else {
            painter.text(
                r.center_top() + vec2(0., 40.),
                Align2::CENTER_TOP,
                "No UV map: UV Unwrap makes one",
                font,
                label,
            );
            return;
        };
        let geometry = uv_geometry(&item.mesh, &map);
        let sel = &self.editor.scene.selection;
        let mode = sel.mode;
        let faces: std::collections::BTreeSet<u32> = sel.faces.iter().copied().collect();
        let screen = |q: &[DVec2]| q.iter().map(|&p| uv.to_screen(r, p)).collect::<Vec<Pos2>>();
        let fill = |pts: Vec<Pos2>, colour: Color32| {
            // Polygons may be concave: fill as a fan of triangles.
            let mut mesh = egui::Mesh::default();
            for &p in &pts {
                mesh.colored_vertex(p, colour);
            }
            for i in 1..pts.len().saturating_sub(1) {
                mesh.add_triangle(0, i as u32, i as u32 + 1);
            }
            painter.add(mesh);
        };
        if uv.show_distortion {
            let colours = distortion_colours(&item.mesh, &geometry.polygons);
            for ((_, _, q), c) in geometry.polygons.iter().zip(colours) {
                fill(screen(q), c);
            }
        }
        if mode == Mode::Polygon {
            for (f, _, q) in &geometry.polygons {
                if faces.contains(f) {
                    fill(screen(q), palette::selection_fill());
                }
            }
        }
        if uv.show_overlap {
            for ((_, _, q), over) in geometry
                .polygons
                .iter()
                .zip(overlapping(&geometry.polygons))
            {
                if over {
                    fill(screen(q), OVERLAP);
                }
            }
        }
        // UVT-110: flipped (clockwise) UV polygons, yellow; it blends with
        // the red overlap tint.
        if uv.show_flipped {
            for (_, _, q) in &geometry.polygons {
                let area: f64 = (0..q.len())
                    .map(|i| q[i].perp_dot(q[(i + 1) % q.len()]))
                    .sum();
                if area < 0. {
                    fill(screen(q), FLIPPED);
                }
            }
        }
        let wire = Stroke::new(1.0_f32, palette::wireframe());
        for (_, _, q) in &geometry.polygons {
            let mut pts = screen(q);
            pts.push(pts[0]);
            painter.add(egui::Shape::line(pts, wire));
        }
        let seam = Stroke::new(1.5_f32, SEAM);
        for (a, b) in &geometry.seams {
            painter.line_segment([uv.to_screen(r, *a), uv.to_screen(r, *b)], seam);
        }
        let selected = palette::selection();
        match mode {
            Mode::Edge => {
                let edges: std::collections::BTreeSet<(u32, u32)> = sel
                    .edges
                    .iter()
                    .map(|&(a, b)| (a.min(b), a.max(b)))
                    .collect();
                for (_, v, q) in &geometry.polygons {
                    for i in 0..v.len() {
                        let j = (i + 1) % v.len();
                        if edges.contains(&(v[i].min(v[j]), v[i].max(v[j]))) {
                            painter.line_segment(
                                [uv.to_screen(r, q[i]), uv.to_screen(r, q[j])],
                                Stroke::new(2.5_f32, selected),
                            );
                        }
                    }
                }
            }
            Mode::Vertex => {
                let points: std::collections::BTreeSet<u32> =
                    sel.vertices.iter().copied().collect();
                for (_, v, q) in &geometry.polygons {
                    for (p, w) in v.iter().zip(q) {
                        let s = uv.to_screen(r, *w);
                        let c = if points.contains(p) {
                            selected
                        } else {
                            palette::wireframe()
                        };
                        painter.circle_filled(s, if points.contains(p) { 3.5 } else { 2. }, c);
                    }
                }
            }
            _ => {}
        }
        if let Some(Drag::Lasso { path }) = &uv.drag {
            let mut outline = lasso::outline(self.lasso_style, path);
            if let Some(&first) = outline.first() {
                outline.push(first);
            }
            painter.add(egui::Shape::dashed_line(
                &outline,
                Stroke::new(1.0_f32, palette::label()),
                5.,
                4.,
            ));
        }
        // UVT-70: the UV Symmetry line.
        if let Some(axis) = self.editor.uv_symmetry.axis {
            let c = self.editor.uv_symmetry.center;
            let (a, b) = match axis {
                operandi_core::uv_edit::UvAxis::U => (DVec2::new(c, -1e4), DVec2::new(c, 1e4)),
                operandi_core::uv_edit::UvAxis::V => (DVec2::new(-1e4, c), DVec2::new(1e4, c)),
            };
            painter.add(egui::Shape::dashed_line(
                &[uv.to_screen(r, a), uv.to_screen(r, b)],
                Stroke::new(1.5_f32, palette::selection()),
                8.,
                5.,
            ));
        }
        // UVT-111: the tile's coverage, lower right, worked out again
        // when no edit is running.
        if uv.drag.is_none() || uv.coverage.get().is_none() {
            uv.coverage.set(Some(coverage(&geometry.polygons)));
        }
        if let Some(c) = uv.coverage.get() {
            painter.text(
                r.right_bottom() + vec2(-8., -8.),
                Align2::RIGHT_BOTTOM,
                format!("Coverage {:.1}%", c * 100.),
                FontId::proportional(12.),
                label,
            );
        }
        self.paint_uv_handles(painter, r, uv);
        self.paint_uv_tweak(painter, r, uv);
        self.paint_uv_falloff(painter, r, uv);
        self.paint_uv_soft_drag(painter, r);
        self.paint_uv_box(painter, r, uv);
        self.paint_uv_pins(painter, r, uv);
        if let (Some(b), Some(h)) = (self.uv_brush, painter.ctx().pointer_hover_pos())
            && r.contains(h)
        {
            painter.circle_stroke(
                h,
                b.size,
                Stroke::new(1.5_f32, if b.sew { palette::selection() } else { SEAM }),
            );
        }
    }

    /// UVT-114 Checker in the UV view: the numbered check in every tile in
    /// sight (up to a few dozen), under the UVs. The image is made once
    /// and kept by egui.
    fn paint_uv_checker(&self, painter: &egui::Painter, r: Rect, uv: &UvView) {
        let ctx = painter.ctx();
        let id = egui::Id::new("uv-checker-texture");
        let texture = ctx
            .data_mut(|d| d.get_temp::<egui::TextureHandle>(id))
            .unwrap_or_else(|| {
                let t = ctx.load_texture(
                    "uv-checker",
                    crate::uv_checker::image(512),
                    egui::TextureOptions::LINEAR,
                );
                ctx.data_mut(|d| d.insert_temp(id, t.clone()));
                t
            });
        let (lo, hi) = (uv.to_uv(r, r.left_bottom()), uv.to_uv(r, r.right_top()));
        let (u0, u1) = (lo.x.floor() as i64, hi.x.ceil() as i64);
        let (v0, v1) = (lo.y.floor() as i64, hi.y.ceil() as i64);
        if (u1 - u0) * (v1 - v0) > 64 {
            // Zoomed far out: only the home tile.
            let tile =
                Rect::from_two_pos(uv.to_screen(r, DVec2::ZERO), uv.to_screen(r, DVec2::ONE));
            painter.image(
                texture.id(),
                tile,
                Rect::from_min_max(pos2(0., 0.), pos2(1., 1.)),
                Color32::WHITE,
            );
            return;
        }
        for tu in u0..u1 {
            for tv in v0..v1 {
                let a = DVec2::new(tu as f64, tv as f64);
                let tile = Rect::from_two_pos(uv.to_screen(r, a), uv.to_screen(r, a + DVec2::ONE));
                // The home tile full strength, its repeats dimmer.
                let tint = if (tu, tv) == (0, 0) {
                    Color32::WHITE
                } else {
                    Color32::from_gray(150)
                };
                painter.image(
                    texture.id(),
                    tile,
                    Rect::from_min_max(pos2(0., 0.), pos2(1., 1.)),
                    tint,
                );
            }
        }
    }

    /// The open UV tool's flat handles (UVE-1 section 2): red U arrow,
    /// green V arrow, a centre square for both; a ring for Rotate; boxes
    /// for Scale.
    fn paint_uv_handles(&self, painter: &egui::Painter, r: Rect, uv: &UvView) {
        let Some(pivot) = self.uv_tool_pivot() else {
            return;
        };
        let Some(tool) = self.editor.session.live.as_ref().map(|l| l.tool) else {
            return;
        };
        let c = uv.to_screen(r, self.uv_handle_center(pivot));
        let hot = match &uv.drag {
            Some(Drag::Tool { handle, .. }) => Some(*handle),
            _ => painter
                .ctx()
                .pointer_hover_pos()
                .filter(|p| r.contains(*p))
                .map(|p| self.uv_handle_at(r, uv, pivot, p)),
        };
        let lit = |h: Handle, base: Color32| {
            if hot == Some(h) {
                theme::viewport::selection()
            } else {
                base
            }
        };
        // UVT-134: U teal, V yellow.
        let (red, green) = (
            Color32::from_rgb(40, 190, 190),
            Color32::from_rgb(235, 205, 60),
        );
        let arrow = |dir: Vec2, colour: Color32| {
            let tip = c + dir * ARROW;
            painter.line_segment([c, tip], Stroke::new(2.0_f32, colour));
            let side = vec2(-dir.y, dir.x) * 5.;
            painter.add(egui::Shape::convex_polygon(
                vec![tip + dir * 9., tip + side, tip - side],
                colour,
                Stroke::NONE,
            ));
        };
        let square = |at: Pos2, colour: Color32| {
            painter.rect_filled(Rect::from_center_size(at, vec2(9., 9.)), 1., colour);
        };
        let moves = matches!(tool, Tool::Move | Tool::Transform);
        let scales = matches!(tool, Tool::Scale | Tool::Transform);
        if moves {
            arrow(vec2(1., 0.), lit(Handle::MoveU, red));
            arrow(vec2(0., -1.), lit(Handle::MoveV, green));
        }
        if matches!(tool, Tool::Rotate | Tool::Transform) {
            painter.circle_stroke(
                c,
                RING,
                Stroke::new(2.0_f32, lit(Handle::Ring, Color32::from_rgb(90, 140, 230))),
            );
        }
        if scales {
            let len = if tool == Tool::Scale {
                ARROW
            } else {
                SCALE_BOX
            };
            if tool == Tool::Scale {
                painter.line_segment(
                    [c, c + vec2(len, 0.)],
                    Stroke::new(2.0_f32, lit(Handle::ScaleU, red)),
                );
                painter.line_segment(
                    [c, c + vec2(0., -len)],
                    Stroke::new(2.0_f32, lit(Handle::ScaleV, green)),
                );
            }
            square(c + vec2(len, 0.), lit(Handle::ScaleU, red));
            square(c + vec2(0., -len), lit(Handle::ScaleV, green));
        }
        let centre = if tool == Tool::Scale {
            Handle::ScaleAll
        } else {
            Handle::Center
        };
        if tool != Tool::Rotate {
            square(c, lit(centre, Color32::from_rgb(220, 220, 220)));
        } else {
            painter.circle_filled(c, 3., Color32::from_rgb(220, 220, 220));
        }
    }

    /// The UV view's header: the map button and the overlay toggles.
    fn uv_view_header(&mut self, ui: &mut Ui, r: Rect, uv: &mut UvView) {
        // The content arrow at the top right (layouts stage 2, LAY-73).
        let arrow = Rect::from_center_size(
            pos2(
                r.right() - 8. - crate::viewport_buttons::ARROW / 2.,
                r.top() + 17.,
            ),
            vec2(crate::viewport_buttons::ARROW, 16.),
        );
        let view = self.drawing_view;
        self.content_arrow(ui, view, arrow);
        let maps: Vec<String> = self
            .editor
            .scene
            .active()
            .map(|i| i.mesh.uv_maps.keys().cloned().collect())
            .unwrap_or_default();
        let shown = self.uv_view_map(uv);
        let dot_slot = if self.is_uv_editor(view) {
            0.
        } else {
            panel_dot::VIEW_DOT_SLOT
        };
        let header = Rect::from_min_size(
            r.min + vec2(8. + dot_slot, 6.),
            vec2(r.width() - 16. - dot_slot, 22.),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(header));
        {
            let ui = &mut child;
            ui.horizontal(|ui| {
                let title = format!("UV: {}", shown.as_deref().unwrap_or("(none)"));
                // UVS-1 section 4: the map button picks the mesh's selected
                // map (the same as the UV Maps list) or makes a new one.
                let idle = self.editor.session.live.is_none();
                let mut pick: Option<UvMapEdit> = None;
                egui::ComboBox::from_id_salt(("uv-map", self.drawing_view))
                    .selected_text(title)
                    .show_ui(ui, |ui| {
                        for m in &maps {
                            if ui
                                .gated(
                                    crate::unavailable::unless(
                                        idle,
                                        crate::unavailable::TOOL_ACTIVE,
                                    ),
                                    egui::Button::selectable(shown.as_deref() == Some(m), m),
                                )
                                .clicked()
                            {
                                pick = Some(UvMapEdit::Select(m.clone()));
                            }
                        }
                        if !maps.is_empty() {
                            ui.separator();
                        }
                        if ui
                            .gated(
                                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                                egui::Button::new("New Map"),
                            )
                            .on_hover_text("A new, empty UV map; it becomes the selected map")
                            .clicked()
                        {
                            pick = Some(UvMapEdit::New(None));
                        }
                    });
                if let Some(edit) = pick {
                    self.run(Command::UvMap(edit));
                }
                ui.toggle_value(&mut uv.show_overlap, "Overlap")
                    .on_hover_text("Tint polygons whose UVs overlap another's red");
                ui.toggle_value(&mut uv.show_flipped, "Flipped")
                    .on_hover_text("Tint UV polygons that face away (wound clockwise) yellow");
                // UVT-70: the UV Symmetry layer.
                let sym = self.editor.uv_symmetry;
                let shown = match sym.axis {
                    None => "Sym: Off",
                    Some(operandi_core::uv_edit::UvAxis::U) => "Sym: U",
                    Some(operandi_core::uv_edit::UvAxis::V) => "Sym: V",
                };
                let mut next = None;
                egui::ComboBox::from_id_salt(("uv_symmetry", self.drawing_view))
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for (axis, label) in [
                            (None, "Off"),
                            (Some(operandi_core::uv_edit::UvAxis::U), "U"),
                            (Some(operandi_core::uv_edit::UvAxis::V), "V"),
                        ] {
                            if ui.selectable_label(sym.axis == axis, label).clicked() {
                                next = Some(operandi_core::uv_symmetry::UvSymmetry { axis, ..sym });
                            }
                        }
                    })
                    .response
                    .on_hover_text("UV Symmetry: edits in the UV view are mirrored across a U or V line at Center (0.5 to start)");
                if let Some(s) = next {
                    self.run(Command::UvSymmetry(s));
                }
                ui.toggle_value(&mut uv.show_distortion, "Distortion")
                    .on_hover_text(
                        "Colour polygons by UV size against 3D size: red too small, blue too large",
                    );
                // UVT-114: approved Operandi extra (Jason, 2026-11-16).
                ui.toggle_value(&mut uv.show_checker, "Checker").on_hover_text(
                    "A numbered grey and white check, 8 × 8 per tile, behind the UVs; the 3D view's Checker shows it on the model",
                );
                // A UV panel stays UV; only a 3D view switched to UV
                // space can switch back.
                if !self.is_uv_pane(self.drawing_view)
                    && ui
                        .button("3D")
                        .on_hover_text("Show this view in 3D again (View > UV View)")
                        .clicked()
                {
                    self.uv_views_to_close.push(self.drawing_view);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_and_uv_round_trip() {
        let v = UvView::default();
        let r = Rect::from_min_size(pos2(0., 0.), vec2(600., 400.));
        let p = v.to_screen(r, DVec2::new(1., 1.));
        assert_eq!(p, pos2(450., 50.));
        assert!((v.to_uv(r, p) - DVec2::ONE).length() < 1e-9);
        let mut v = v;
        let anchor = DVec2::new(0.8, 0.2);
        let before = v.to_screen(r, anchor);
        v.zoom_about(2.0, anchor);
        assert!((v.to_screen(r, anchor) - before).length() < 1e-3);
    }

    #[test]
    fn overlap_and_seams() {
        let sq = |o: f64| {
            vec![
                DVec2::new(o, 0.),
                DVec2::new(o + 1., 0.),
                DVec2::new(o + 1., 1.),
                DVec2::new(o, 1.),
            ]
        };
        let polys = vec![
            (0, vec![0, 1, 2, 3], sq(0.)),
            (1, vec![4, 5, 6, 7], sq(0.5)),
            (2, vec![8, 9, 10, 11], sq(3.)),
        ];
        assert_eq!(overlapping(&polys), vec![true, true, false]);
        // Neighbours sharing an edge don't overlap.
        let touching = vec![(0, vec![0, 1, 2, 3], sq(0.)), (1, vec![1, 4, 5, 2], sq(1.))];
        assert_eq!(overlapping(&touching), vec![false, false]);
    }
}
