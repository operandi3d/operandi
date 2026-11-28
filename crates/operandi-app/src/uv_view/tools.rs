//! Element Move (Tweak), Smooth and Slide in a UV view (UVT-130/131/137): the tools
//! run on the map's UV proxy in the core (`operandi_core::uv_pipe`); here
//! the view picks the proxy's UV points, edges and polygons under the
//! pointer and turns drags into the tool's values in U and V.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md UVT-Q11, UVT-Q13): Range
//! follows `uv_pipe::RANGE_UNITS`; a right-drag grows it by
//! `TWEAK_RANGE_RATE` screen pixels of radius per pixel moved, as in 3D. A
//! click with no drag on an element puts Move handles on it (U and V
//! arrows, a centre square); dragging them moves that element.
use super::*;
use operandi_core::uv_pipe::{RANGE_UNITS, RangeUnits, UvProxy};
use operandi_core::{TweakElement, TweakGrab, TweakMode};

/// How close (screen points) the pointer must be to pick a UV point or
/// edge, as the UV view's selection picking.
const POINT_PICK: f32 = 10.;
const EDGE_PICK: f32 = 8.;

/// Which element kinds a Tweak mode picks: points, edges, polygons.
fn picks(mode: TweakMode) -> (bool, bool, bool) {
    match mode {
        TweakMode::Auto | TweakMode::AutoCenter => (true, true, true),
        TweakMode::Vertex => (true, false, false),
        TweakMode::Edge | TweakMode::EdgeCenter => (false, true, false),
        TweakMode::Polygon | TweakMode::PolygonCenter => (false, false, true),
    }
}

/// The proxy element under `p` (screen): in Automatic mode a point near the
/// pointer, else an edge near it, else the polygon it is inside; the
/// fixed modes pick only their kind (UVT-131).
pub(super) fn pick(
    proxy: &UvProxy,
    mode: TweakMode,
    r: Rect,
    uv: &UvView,
    p: Pos2,
) -> Option<TweakElement> {
    let mesh = proxy.mesh();
    let screen = |v: u32| {
        let q = mesh.vertices[v as usize];
        uv.to_screen(r, DVec2::new(q.x, q.y))
    };
    let (points, edges, polygons) = picks(mode);
    if points
        && let Some((v, _)) = (0..mesh.vertices.len() as u32)
            .map(|v| (v, screen(v).distance(p)))
            .filter(|(_, d)| *d <= POINT_PICK)
            .min_by(|a, b| a.1.total_cmp(&b.1))
    {
        return Some(TweakElement::Vertex(v));
    }
    if edges
        && let Some(((a, b), _)) = mesh
            .faces
            .iter()
            .flat_map(|f| {
                let v = &f.vertices;
                (0..v.len()).map(move |i| (v[i], v[(i + 1) % v.len()]))
            })
            .map(|(a, b)| ((a, b), segment_distance(p, screen(a), screen(b))))
            .filter(|(_, d)| *d <= EDGE_PICK)
            .min_by(|a, b| a.1.total_cmp(&b.1))
    {
        return Some(TweakElement::Edge(a.min(b), a.max(b)));
    }
    if polygons {
        let at = uv.to_uv(r, p);
        return mesh
            .faces
            .iter()
            .enumerate()
            .rev()
            .find(|(_, f)| {
                let poly: Vec<DVec2> = f
                    .vertices
                    .iter()
                    .map(|&v| mesh.vertices[v as usize].truncate())
                    .collect();
                point_in_polygon(at, &poly)
            })
            .map(|(f, _)| TweakElement::Polygon(f as u32));
    }
    None
}

impl Operandi {
    /// The live tool when it is `tool` in UV space.
    pub(crate) fn uv_tool_is(&self, tool: Tool) -> bool {
        self.editor
            .live
            .as_ref()
            .is_some_and(|l| l.tool == tool && l.params.uv.is_some())
    }

    /// UVT-131: what a press with Element Move would grab under `p`.
    pub(super) fn uv_tweak_pick(&mut self, r: Rect, uv: &UvView, p: Pos2) -> Option<TweakGrab> {
        let mode = self.editor.live.as_ref()?.params.tweak_options.mode;
        let proxy = self.editor.uv_tool_proxy()?;
        let element = pick(&proxy, mode, r, uv, p)?;
        Some(TweakGrab {
            item: proxy.item,
            element,
        })
    }

    /// The middle of a proxy element in UV (where its handles sit).
    pub(super) fn uv_element_middle(
        proxy: &operandi_core::uv_pipe::UvProxy,
        element: TweakElement,
    ) -> Option<glam::DVec2> {
        let mesh = proxy.mesh();
        let at = |v: u32| mesh.vertices.get(v as usize).map(|q| q.truncate());
        match element {
            TweakElement::Vertex(v) => at(v),
            TweakElement::Edge(a, b) => Some((at(a)? + at(b)?) * 0.5),
            TweakElement::Polygon(f) => {
                let v = &mesh.faces.get(f as usize)?.vertices;
                let sum = v.iter().map(|&c| at(c)).sum::<Option<glam::DVec2>>()?;
                Some(sum / v.len().max(1) as f64)
            }
        }
    }

    /// UVT-Q13: the clicked element's handle under `p`: its U arrow
    /// (axis 0), V arrow (axis 1) or centre square (both), drawn as the
    /// Move handles are (UVT-134).
    pub(super) fn uv_tweak_handle_at(
        &mut self,
        r: Rect,
        uv: &UvView,
        p: Pos2,
    ) -> Option<(TweakGrab, Option<usize>)> {
        let grab = uv.tweak_clicked?;
        let proxy = self.editor.uv_tool_proxy()?;
        let c = uv.to_screen(r, Self::uv_element_middle(&proxy, grab.element)?);
        let on = |a: Pos2, b: Pos2| super::segment_distance(p, a, b) <= super::GRAB;
        if c.distance(p) <= super::GRAB + 2. {
            Some((grab, None))
        } else if on(c, c + egui::vec2(super::ARROW, 0.)) {
            Some((grab, Some(0)))
        } else if on(c, c + egui::vec2(0., -super::ARROW)) {
            Some((grab, Some(1)))
        } else {
            None
        }
    }

    /// One frame of an Element Move drag in a UV view: the grabbed element
    /// moves by the pointer's offset in U and V since the press (only U or
    /// only V when an arrow of a clicked element is dragged).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn uv_tweak_drag(
        &mut self,
        r: Rect,
        uv: &UvView,
        grab: TweakGrab,
        base: &operandi_core::Parameters,
        start: Pos2,
        p: Pos2,
        axis: Option<usize>,
    ) {
        let mut params = base.clone();
        let mut d = uv.to_uv(r, p) - uv.to_uv(r, start);
        match axis {
            Some(0) => d.y = 0.,
            Some(1) => d.x = 0.,
            _ => {}
        }
        params.tweak = Some(grab);
        params.delta = glam::DVec3::new(d.x, d.y, 0.);
        // What moves was worked out at the first frame; keep it.
        let live = self.editor.live.as_ref();
        params.tweak_plan = live.and_then(|l| l.params.tweak_plan.clone());
        if let (Some(space), Some(now)) =
            (params.uv.as_mut(), live.and_then(|l| l.params.uv.as_ref()))
        {
            space.proxy = now.proxy.clone();
            space.pixels_per_uv = uv.zoom;
        }
        if live.is_some_and(|l| l.params == params) {
            return;
        }
        self.run(Command::Preview(params));
        self.clear_fields();
    }

    /// TWR-06..08 in a UV view: a right-drag grows (right) or shrinks
    /// (left) Element Move's Range; in UV units or pixels by
    /// `uv_pipe::RANGE_UNITS` (UVT-Q11).
    pub(super) fn uv_tweak_range_drag(&mut self, uv: &UvView, from: Pos2, to: Pos2) {
        let dx = (to.x - from.x) as f64;
        let Some(mut params) = self
            .editor
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::Tweak)
            .map(|l| l.params.clone())
        else {
            return;
        };
        if dx == 0. || !params.tweak_options.connect.uses_range() {
            return;
        }
        let per_pixel = match RANGE_UNITS {
            RangeUnits::Uv => 1. / uv.zoom.max(1e-9),
            RangeUnits::Pixels => 1.,
        };
        let rate = crate::viewport::tweak_view::TWEAK_RANGE_RATE;
        let range = params.tweak_options.range;
        let next = (range + dx * rate * per_pixel).max(0.);
        if next == range {
            return;
        }
        params.tweak_options.range = next;
        if let Some(space) = params.uv.as_mut() {
            space.pixels_per_uv = uv.zoom;
        }
        self.run(Command::Preview(params));
        self.clear_fields();
        self.status = format!(
            "Element Move Range: {} · drag right to grow, left to shrink",
            workspace::number_text(
                next,
                if RANGE_UNITS == RangeUnits::Uv {
                    ""
                } else {
                    " px"
                }
            )
        );
    }

    /// Whether the live UV tool takes a drag anywhere (`uv_haul_drag`).
    pub(super) fn uv_haul_tool(&self) -> bool {
        // The UV tab's solvers take Iterations from a drag in any view.
        if self
            .editor
            .live
            .as_ref()
            .is_some_and(|l| matches!(l.tool, Tool::UvUnwrap | Tool::UvRelax | Tool::UvPelt))
        {
            return true;
        }
        [
            Tool::Smooth,
            Tool::EdgeSlide,
            Tool::Jitter,
            Tool::Quantize,
            Tool::RadialAlign,
            Tool::LinearAlign,
        ]
        .into_iter()
        .any(|t| self.uv_tool_is(t))
    }

    /// The drag anywhere of the field-driven tools in a UV view, as in 3D:
    /// right raises Smooth's Strength or Radial / Linear Align's Weight
    /// (200 px = 1), slides further (200 px = the whole rail), widens
    /// Jitter's Range or Quantize's Step (in UV units: the pointer's travel
    /// in the view); left lowers them.
    pub(super) fn uv_haul_drag(
        &mut self,
        uv: &UvView,
        base: &operandi_core::Parameters,
        start: Pos2,
        p: Pos2,
    ) {
        let mut params = base.clone();
        let dx = (p.x - start.x) as f64 / 200.;
        let du = (p.x - start.x) as f64 / uv.zoom.max(1e-9);
        let tool = self.editor.live.as_ref().map(|l| l.tool);
        if let Some(t) = tool
            && let Some(next) = self.uv_iterations_drag(base, t, (p.x - start.x) as f64)
        {
            if self.editor.live.as_ref().is_some_and(|l| l.params != next) {
                self.run(Command::Preview(next));
                self.clear_fields();
            }
            return;
        }
        match tool {
            Some(Tool::Smooth) => {
                params.smooth.strength = (base.smooth.strength + dx).clamp(0., 100.)
            }
            Some(Tool::EdgeSlide) => params.shift = (base.shift + dx).clamp(-1., 1.),
            // UVT-135: only U and V; Range Z stays as it was.
            Some(Tool::Jitter) => {
                let r = (base.jitter.range.truncate().max_element() + du).max(0.);
                let on = base.jitter.enable;
                params.jitter.range.x = if on[0] { r } else { base.jitter.range.x };
                params.jitter.range.y = if on[1] { r } else { base.jitter.range.y };
            }
            Some(Tool::Quantize) => {
                let s = (base.quantize.step.truncate().max_element() + du).max(0.);
                params.quantize.step.x = s;
                params.quantize.step.y = s;
            }
            Some(Tool::RadialAlign) => {
                params.radial_align.weight = (base.radial_align.weight + dx).clamp(0., 1.)
            }
            Some(Tool::LinearAlign) => {
                params.linear_align.weight = (base.linear_align.weight + dx).clamp(0., 1.)
            }
            _ => return,
        }
        if let (Some(space), Some(now)) = (
            params.uv.as_mut(),
            self.editor.live.as_ref().and_then(|l| l.params.uv.as_ref()),
        ) {
            space.proxy = now.proxy.clone();
        }
        if self
            .editor
            .live
            .as_ref()
            .is_some_and(|l| l.params == params)
        {
            return;
        }
        self.run(Command::Preview(params));
        self.clear_fields();
    }

    /// Outline the UV element a press would grab (TWK-02), and the Range
    /// circle while a right-drag sizes it (TWR-11/12).
    pub(super) fn paint_uv_tweak(&self, painter: &egui::Painter, r: Rect, uv: &UvView) {
        use theme::viewport as palette;
        let Some(proxy) = self
            .editor
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::Tweak)
            .and_then(|l| l.params.uv.as_ref())
            .and_then(|s| s.proxy.clone())
        else {
            return;
        };
        let mesh = proxy.mesh();
        let screen = |v: u32| {
            let q = mesh.vertices[v as usize];
            uv.to_screen(r, DVec2::new(q.x, q.y))
        };
        let line = |a: u32, b: u32| {
            let (qa, qb) = (screen(a), screen(b));
            painter.line_segment([qa, qb], Stroke::new(4_f32, palette::edge_backing()));
            painter.line_segment([qa, qb], Stroke::new(2_f32, palette::rollover()));
        };
        if let Some(element) = uv.tweak_hover
            && !matches!(uv.drag, Some(Drag::Tweak { .. }))
        {
            match element {
                TweakElement::Vertex(v) if (v as usize) < mesh.vertices.len() => {
                    let q = screen(v);
                    painter.circle_filled(q, 4.5, palette::edge_backing());
                    painter.circle_filled(q, 3.5, palette::rollover());
                }
                TweakElement::Edge(a, b) if (a.max(b) as usize) < mesh.vertices.len() => line(a, b),
                TweakElement::Polygon(f) => {
                    if let Some(face) = mesh.faces.get(f as usize) {
                        let v = &face.vertices;
                        for i in 0..v.len() {
                            line(v[i], v[(i + 1) % v.len()]);
                        }
                    }
                }
                _ => {}
            }
        }
        // UVT-Q13: the clicked element's handles, U teal and V yellow
        // (UVT-134), following the element as it moves.
        if let Some(grab) = uv.tweak_clicked
            && let Some(middle) = Self::uv_element_middle(&proxy, grab.element)
        {
            let c = uv.to_screen(r, middle);
            let (u_colour, v_colour) = (
                egui::Color32::from_rgb(40, 190, 190),
                egui::Color32::from_rgb(235, 205, 60),
            );
            let lit = match &uv.drag {
                Some(Drag::Tweak { axis, .. }) => Some(*axis),
                _ => None,
            };
            let colour = |axis: Option<usize>, base: egui::Color32| {
                if lit == Some(axis) {
                    palette::selection()
                } else {
                    base
                }
            };
            for (dir, axis, base) in [
                (egui::vec2(1., 0.), 0, u_colour),
                (egui::vec2(0., -1.), 1, v_colour),
            ] {
                let tip = c + dir * super::ARROW;
                let col = colour(Some(axis), base);
                painter.line_segment([c, tip], Stroke::new(2_f32, col));
                let side = egui::vec2(-dir.y, dir.x) * 5.;
                painter.add(egui::Shape::convex_polygon(
                    vec![tip + dir * 9., tip + side, tip - side],
                    col,
                    Stroke::NONE,
                ));
            }
            painter.rect_filled(
                Rect::from_center_size(c, egui::vec2(9., 9.)),
                1.,
                colour(None, egui::Color32::from_rgb(220, 220, 220)),
            );
        }
        if let Some(Drag::TweakRange { center, .. }) = uv.drag {
            let options = self
                .editor
                .live
                .as_ref()
                .map(|l| l.params.tweak_options)
                .unwrap_or_default();
            let radius = match RANGE_UNITS {
                RangeUnits::Uv => options.range * uv.zoom,
                RangeUnits::Pixels => options.range,
            } as f32;
            if radius > 0. && options.connect.uses_range() {
                painter.circle_stroke(center, radius, Stroke::new(1.5_f32, palette::tweak_range()));
            }
        }
    }
}

impl Operandi {
    /// UVT-133 with the Element action centre on and a tool running in a
    /// UV view: a click picks the UV point (or edge middle) under the
    /// pointer, at (U, V, 0), every time (C-ELEM). Returns false when the
    /// click isn't for that, or lands on nothing.
    pub(super) fn uv_place_element_center(&mut self, r: Rect, uv: &UvView, p: Pos2) -> bool {
        if !matches!(
            self.editor.action_context.center,
            operandi_core::CenterMode::Element(_)
        ) || !self.uv_tool_live()
        {
            return false;
        }
        let Some(proxy) = self.editor.uv_tool_proxy() else {
            return false;
        };
        let mesh = proxy.mesh();
        let point = match pick(&proxy, TweakMode::Vertex, r, uv, p)
            .or_else(|| pick(&proxy, TweakMode::Edge, r, uv, p))
        {
            Some(TweakElement::Vertex(v)) => mesh.vertices[v as usize],
            Some(TweakElement::Edge(a, b)) => {
                (mesh.vertices[a as usize] + mesh.vertices[b as usize]) * 0.5
            }
            _ => return false,
        };
        let context = operandi_core::ActionContext {
            center: operandi_core::CenterMode::Element(operandi_core::PickedPoint::new(point)),
            axis: operandi_core::AxisOrientation::World,
        };
        self.run(Command::ActionContext(context));
        true
    }

    /// A UV tool's falloff, drawn in UV (UVT-132): Radial as its ellipse,
    /// Linear as its line from Start (full weight) to End.
    pub(super) fn paint_uv_falloff(&self, painter: &egui::Painter, r: Rect, uv: &UvView) {
        use operandi_core::Falloff;
        if !self.uv_tool_live() {
            return;
        }
        let colour = theme::viewport::tweak_range();
        let at = |p: glam::DVec3| uv.to_screen(r, DVec2::new(p.x, p.y));
        match self.editor.spatial_falloff {
            Some(Falloff::Radial(f)) => {
                let n = 64;
                let points: Vec<Pos2> = (0..=n)
                    .map(|k| {
                        let t = k as f64 / n as f64 * std::f64::consts::TAU;
                        at(f.center + glam::DVec3::new(f.size.x * t.cos(), f.size.y * t.sin(), 0.))
                    })
                    .collect();
                painter.add(egui::Shape::line(points, Stroke::new(1.5_f32, colour)));
            }
            Some(Falloff::Linear(f)) => {
                let (a, b) = (at(f.start), at(f.end));
                painter.line_segment([a, b], Stroke::new(1.5_f32, colour));
                painter.circle_filled(a, 4., colour);
                painter.circle_stroke(b, 4., Stroke::new(1.5_f32, colour));
            }
            _ => {}
        }
    }
}

impl Operandi {
    /// Soft Drag in a UV view (UVT-40, UVT-132: the Screen falloff stays a
    /// screen circle): every press centres the circle on the pointer and
    /// stamps the UV proxy's points with their weight in this view.
    pub(super) fn uv_soft_drag_press(&mut self, r: Rect, uv: &UvView, at: Pos2) {
        if !self.soft_drag_open() {
            return;
        }
        let Some((layer, operandi_core::Falloff::Screen(s))) = self.screen_layer() else {
            return;
        };
        let center = glam::DVec2::new((at.x - r.min.x) as f64, (at.y - r.min.y) as f64);
        self.replace_falloff_layer(
            layer,
            operandi_core::Falloff::Screen(operandi_core::ScreenFalloff { center, ..s }),
        );
        self.uv_restamp_screen(r, uv);
    }

    /// Work the Screen layer's UV stamps out again in this view: each UV
    /// proxy point's weight at its place on screen (all UV points face the
    /// view). Kept under `uv_pipe::UV_STAMP_LAYER` so 3D tools never read
    /// them.
    pub(super) fn uv_restamp_screen(&mut self, r: Rect, uv: &UvView) {
        let Some((layer, operandi_core::Falloff::Screen(s))) = self.screen_layer() else {
            return;
        };
        let Some(proxy) = self.editor.uv_tool_proxy() else {
            return;
        };
        let weights: Vec<f64> = proxy
            .mesh()
            .vertices
            .iter()
            .map(|q| {
                let p = uv.to_screen(r, q.truncate());
                s.weight_at(glam::DVec2::new(
                    (p.x - r.min.x) as f64,
                    (p.y - r.min.y) as f64,
                ))
            })
            .collect();
        let key = (layer + operandi_core::uv_pipe::UV_STAMP_LAYER, proxy.item);
        let mut all: std::collections::BTreeMap<(usize, u64), Vec<f64>> = self
            .editor
            .falloff_stamps
            .iter()
            .filter(|(k, _)| **k != key)
            .map(|(k, v)| (*k, v.as_ref().clone()))
            .collect();
        all.insert(key, weights);
        self.run(Command::FalloffStamps(all));
    }

    /// Soft Drag's right-drag in a UV view sizes the circle, as in 3D.
    pub(super) fn uv_soft_size_drag(
        &mut self,
        r: Rect,
        uv: &UvView,
        start: f64,
        from: Pos2,
        to: Pos2,
    ) {
        let Some((layer, operandi_core::Falloff::Screen(s))) = self.screen_layer() else {
            return;
        };
        let size = (start + (to.x - from.x) as f64).max(1.);
        if s.size == size {
            return;
        }
        self.replace_falloff_layer(
            layer,
            operandi_core::Falloff::Screen(operandi_core::ScreenFalloff { size, ..s }),
        );
        self.uv_restamp_screen(r, uv);
        self.status =
            format!("Soft Drag circle: {size:.0} px · drag right for more, left for less");
    }

    /// The Soft Drag circle in a UV view.
    pub(super) fn paint_uv_soft_drag(&self, painter: &egui::Painter, r: Rect) {
        if !self.soft_drag_open() || !self.uv_tool_live() {
            return;
        }
        if let Some((_, operandi_core::Falloff::Screen(s))) = self.screen_layer()
            && s.size > 0.
        {
            let c = Pos2::new(r.min.x + s.center.x as f32, r.min.y + s.center.y as f32);
            painter.circle_stroke(
                c,
                s.size as f32,
                Stroke::new(1.5_f32, theme::viewport::selection()),
            );
        }
    }
}

/// What a Box Transform press grabbed (UVT-43).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BoxGrab {
    Move,
    Turn,
    Pivot,
    /// Right-drag on a side or corner: scale about `anchor` (the opposite
    /// side or corner) along the axes marked.
    Scale {
        anchor: DVec2,
        axes: (bool, bool),
    },
}

/// Pixels from the box's top edge to its rotation handle.
const BOX_TURN_GAP: f32 = 22.;

impl Operandi {
    /// The live Box Transform's box now: its four corners in UV (lower
    /// left, lower right, upper right, upper left), the pivot, and the
    /// session-start bounds.
    pub(super) fn uv_box_now(&mut self) -> Option<([DVec2; 4], DVec2, (DVec2, DVec2))> {
        if !self.uv_tool_is(Tool::UvBoxTransform) {
            return None;
        }
        let (lo, hi) = self.editor.uv_box_base()?;
        let b = self.editor.live.as_ref()?.params.uv_box;
        let corners =
            [lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)].map(|c| b.apply(c, lo, hi));
        let pivot = b.pivot_of(lo, hi) + b.offset;
        Some((corners, pivot, (lo, hi)))
    }

    /// A press with Box Transform in a UV view: which part it grabbed.
    pub(super) fn uv_box_press(
        &mut self,
        r: Rect,
        uv: &UvView,
        at: Pos2,
        secondary: bool,
    ) -> Option<Drag> {
        let (corners, pivot, bounds) = self.uv_box_now()?;
        let base = Box::new(self.editor.live.as_ref()?.params.clone());
        let s = corners.map(|c| uv.to_screen(r, c));
        let near = |p: Pos2, d: f32| p.distance(at) <= d;
        let grab = if secondary {
            // Corners first, then sides; anchored on the opposite one, in
            // the box's own (session-start) frame.
            let (lo, hi) = bounds;
            let base_corners = [lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)];
            if let Some(i) = (0..4).find(|&i| near(s[i], 10.)) {
                BoxGrab::Scale {
                    anchor: base_corners[(i + 2) % 4],
                    axes: (true, true),
                }
            } else {
                let i = (0..4).find(|&i| {
                    super::segment_distance(at, s[i], s[(i + 1) % 4]) <= super::GRAB + 2.
                })?;
                // Sides: 0 bottom, 1 right, 2 top, 3 left.
                let mid = |a: DVec2, b: DVec2| (a + b) * 0.5;
                BoxGrab::Scale {
                    anchor: mid(base_corners[(i + 2) % 4], base_corners[(i + 3) % 4]),
                    axes: (i % 2 == 1, i % 2 == 0),
                }
            }
        } else {
            let top = s[2] + (s[3] - s[2]) * 0.5;
            let turn = top + vec2(0., -BOX_TURN_GAP);
            let show = base.uv_box.show_rotation_handle;
            if show && near(turn, 8.) {
                BoxGrab::Turn
            } else if show && near(uv.to_screen(r, pivot), 7.) {
                BoxGrab::Pivot
            } else if super::point_in_polygon(uv.to_uv(r, at), &corners) {
                BoxGrab::Move
            } else {
                return None;
            }
        };
        Some(Drag::Boxed {
            grab,
            start: at,
            base,
            bounds,
        })
    }

    /// One frame of a Box Transform drag.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn uv_box_drag(
        &mut self,
        r: Rect,
        uv: &UvView,
        grab: BoxGrab,
        start: Pos2,
        p: Pos2,
        base: &operandi_core::Parameters,
        bounds: (DVec2, DVec2),
        mods: egui::Modifiers,
    ) {
        let mut params = base.clone();
        let b0 = base.uv_box;
        let (lo, hi) = bounds;
        let (a, z) = (uv.to_uv(r, start), uv.to_uv(r, p));
        let b = &mut params.uv_box;
        let ctrl = mods.command || mods.ctrl;
        match grab {
            BoxGrab::Move => {
                let mut d = z - a;
                if ctrl {
                    if d.x.abs() >= d.y.abs() {
                        d.y = 0.
                    } else {
                        d.x = 0.
                    }
                }
                b.offset = b0.offset + d;
                // Shift: the box's lower-left corner lands on the 0.1 grid
                // (UVB-P02).
                if mods.shift {
                    let corner = b0.apply(lo, lo, hi) - b0.offset + b.offset;
                    let snapped = (corner / 0.1).round() * 0.1;
                    b.offset += snapped - corner;
                }
            }
            BoxGrab::Turn => {
                let c = b0.pivot_of(lo, hi) + b0.offset;
                let (u, v) = (a - c, z - c);
                if u.length() > 1e-12 && v.length() > 1e-12 {
                    let mut deg = (v.y.atan2(v.x) - u.y.atan2(u.x)).to_degrees();
                    if ctrl {
                        let step = b0.snap_angle.max(0.1);
                        deg = (deg / step).round() * step;
                    }
                    b.angle = b0.angle + deg;
                }
            }
            BoxGrab::Pivot => {
                // Snaps to the box's sides, corners and centre within 8 px.
                let mut to = z - b0.offset;
                let (cx, cy) = ((lo.x + hi.x) * 0.5, (lo.y + hi.y) * 0.5);
                let marks = [
                    lo,
                    DVec2::new(hi.x, lo.y),
                    hi,
                    DVec2::new(lo.x, hi.y),
                    DVec2::new(cx, cy),
                    DVec2::new(cx, lo.y),
                    DVec2::new(cx, hi.y),
                    DVec2::new(lo.x, cy),
                    DVec2::new(hi.x, cy),
                ];
                if let Some(m) = marks
                    .iter()
                    .find(|m| uv.to_screen(r, **m + b0.offset).distance(p) <= 8.)
                {
                    to = *m;
                }
                b.pivot = Some(to);
            }
            BoxGrab::Scale { anchor, axes } => {
                // The pointer in the box's own frame: undo the move and turn.
                let pivot = b0.pivot_of(lo, hi);
                let back = |q: DVec2| {
                    pivot
                        + glam::DMat2::from_angle(-b0.angle.to_radians()) * (q - b0.offset - pivot)
                };
                let (q0, q1) = (back(a), back(z));
                let ratio = |k: usize| {
                    let d = q0[k] - anchor[k];
                    if d.abs() < 1e-9 {
                        1.
                    } else {
                        (q1[k] - anchor[k]) / d
                    }
                };
                let mut f = DVec2::new(
                    if axes.0 { ratio(0) } else { 1. },
                    if axes.1 { ratio(1) } else { 1. },
                );
                if axes.0 && axes.1 {
                    f = DVec2::splat((f.x + f.y) * 0.5);
                }
                b.anchor = Some(anchor);
                b.scale = (b0.scale * f).max(DVec2::splat(1e-3));
            }
        }
        if let (Some(space), Some(now)) = (
            params.uv.as_mut(),
            self.editor.live.as_ref().and_then(|l| l.params.uv.as_ref()),
        ) {
            space.proxy = now.proxy.clone();
        }
        if self
            .editor
            .live
            .as_ref()
            .is_some_and(|l| l.params == params)
        {
            return;
        }
        self.run(Command::Preview(params));
        self.clear_fields();
    }

    /// The box, its rotation handle and pivot (UVT-43).
    pub(super) fn paint_uv_box(&self, painter: &egui::Painter, r: Rect, uv: &UvView) {
        if !self.uv_tool_is(Tool::UvBoxTransform) {
            return;
        }
        let Some((corners, pivot)) = uv.box_now else {
            return;
        };
        let s: Vec<Pos2> = corners.iter().map(|c| uv.to_screen(r, *c)).collect();
        let mut closed = s.clone();
        closed.push(s[0]);
        let ink = theme::viewport::selection();
        painter.add(egui::Shape::line(closed, Stroke::new(1.5_f32, ink)));
        for &c in &s {
            painter.rect_filled(Rect::from_center_size(c, egui::vec2(7., 7.)), 1., ink);
        }
        let show = self
            .editor
            .live
            .as_ref()
            .is_some_and(|l| l.params.uv_box.show_rotation_handle);
        if show {
            let top = s[2] + (s[3] - s[2]) * 0.5;
            let turn = top + egui::vec2(0., -BOX_TURN_GAP);
            painter.line_segment([top, turn], Stroke::new(1.0_f32, ink));
            painter.circle_stroke(
                turn,
                6.,
                Stroke::new(2.0_f32, egui::Color32::from_rgb(90, 140, 230)),
            );
            let c = uv.to_screen(r, pivot);
            painter.line_segment(
                [c - egui::vec2(6., 0.), c + egui::vec2(6., 0.)],
                Stroke::new(1.5_f32, ink),
            );
            painter.line_segment(
                [c - egui::vec2(0., 6.), c + egui::vec2(0., 6.)],
                Stroke::new(1.5_f32, ink),
            );
        }
    }
}

impl Operandi {
    /// The mesh points with a UV position under the Split and Sew brush at
    /// `at` (UVT-63).
    pub(super) fn uv_brush_collect(
        &self,
        r: Rect,
        uv: &UvView,
        at: Pos2,
        points: &mut std::collections::BTreeSet<u32>,
    ) {
        let (Some(b), Some(item)) = (self.uv_brush, self.editor.scene.active()) else {
            return;
        };
        let Some(map) = operandi_core::uv_maps::current_map(&item.mesh) else {
            return;
        };
        let geometry = super::uv_geometry(&item.mesh, &map);
        for (_, v, q) in &geometry.polygons {
            for (p, w) in v.iter().zip(q) {
                if uv.to_screen(r, *w).distance(at) <= b.size {
                    points.insert(*p);
                }
            }
        }
    }
}

impl Operandi {
    /// Relax with Interactive on (UVS-1 §3).
    fn relax_interactive(&self) -> bool {
        self.editor
            .live
            .as_ref()
            .is_some_and(|l| l.tool == Tool::UvRelax && l.params.uv_relax.interactive)
    }

    /// The UV corner drawn nearest `p` (within 10 px), as (polygon, corner).
    fn uv_corner_at(&self, r: Rect, uv: &UvView, p: Pos2) -> Option<(usize, usize)> {
        let item = self.editor.scene.active()?;
        let map = operandi_core::uv_maps::current_map(&item.mesh)?;
        let rows = item.mesh.uv_maps.get(&map)?;
        let mut best: Option<((usize, usize), f32)> = None;
        for (f, row) in rows.iter().enumerate() {
            for (c, q) in row.iter().enumerate() {
                let Some(q) = q else { continue };
                let d = uv.to_screen(r, *q).distance(p);
                if d <= 10. && best.is_none_or(|(_, b)| d < b) {
                    best = Some(((f, c), d));
                }
            }
        }
        best.map(|(k, _)| k)
    }

    /// A click with Relax Interactive: pin the UV point under it, or with
    /// `remove` take the pin under it away. Returns whether it was taken.
    pub(super) fn uv_pin_click(&mut self, r: Rect, uv: &UvView, p: Pos2, remove: bool) -> bool {
        if !self.relax_interactive() {
            return false;
        }
        let live = self.editor.live.as_ref().expect("checked");
        let mut params = live.params.clone();
        let near_pin = params
            .uv_relax
            .pins
            .iter()
            .position(|pin| uv.to_screen(r, pin.to).distance(p) <= 8.);
        if remove {
            if let Some(i) = near_pin {
                params.uv_relax.pins.remove(i);
                self.run(Command::Preview(params));
                self.clear_fields();
            }
            return true;
        }
        if near_pin.is_some() {
            return true;
        }
        let Some((f, c)) = self.uv_corner_at(r, uv, p) else {
            return true;
        };
        let base = &live.base;
        let Some(item) = base.active() else {
            return true;
        };
        let Some(map) = operandi_core::uv_maps::current_map(&item.mesh) else {
            return true;
        };
        let Some(at) = item.mesh.uv_maps[&map]
            .get(f)
            .and_then(|r| r.get(c).copied().flatten())
        else {
            return true;
        };
        let now = self
            .editor
            .scene
            .active()
            .and_then(|i| i.mesh.uv_maps.get(&map)?.get(f)?.get(c).copied().flatten())
            .unwrap_or(at);
        params.uv_relax.pins.push(operandi_core::uv_relax::UvPin {
            point: item.mesh.faces[f].vertices[c],
            at,
            to: now,
        });
        self.run(Command::Preview(params));
        self.clear_fields();
        self.status = format!(
            "{} pins · drag a pin to pull the map",
            self.editor
                .live
                .as_ref()
                .map_or(0, |l| l.params.uv_relax.pins.len())
        );
        true
    }

    /// A press on a Relax pin starts dragging it.
    pub(super) fn uv_pin_press(
        &mut self,
        r: Rect,
        uv: &UvView,
        at: Pos2,
        mods: egui::Modifiers,
    ) -> Option<Drag> {
        if !self.relax_interactive() || mods.command || mods.ctrl {
            return None;
        }
        let params = self.editor.live.as_ref()?.params.clone();
        let index = params
            .uv_relax
            .pins
            .iter()
            .position(|pin| uv.to_screen(r, pin.to).distance(at) <= 8.)?;
        Some(Drag::Pin {
            index,
            base: Box::new(params),
            start: at,
        })
    }

    /// Relax's pins: small blue squares.
    pub(super) fn paint_uv_pins(&self, painter: &egui::Painter, r: Rect, uv: &UvView) {
        if !self.relax_interactive() {
            return;
        }
        for pin in &self
            .editor
            .live
            .as_ref()
            .expect("checked")
            .params
            .uv_relax
            .pins
        {
            let c = uv.to_screen(r, pin.to);
            let square = Rect::from_center_size(c, egui::vec2(8., 8.));
            painter.rect_filled(square, 1., egui::Color32::from_rgb(70, 130, 230));
            painter.rect_stroke(
                square,
                1.,
                Stroke::new(1.0_f32, egui::Color32::BLACK),
                egui::StrokeKind::Outside,
            );
        }
    }
}
