//! Edge panel (Milestone 2): click-to-add tools Add Loop and Add Point.
//!
//! Both tools are activated from the Edges panel and then
//! act on the edge under the pointer: a click on a highlighted edge adds a
//! loop (or a point) where the pointer meets the edge; holding Ctrl places it
//! at the middle ("Split at the Middle"). Each click is one undoable edit.
//! The same geometry is callable with `edge.addLoop` / `edge.addPoint`, which
//! act on the selected edges at `position:<0–1 or %>` (default 0.5).
//! Holding the button after the click slides the new loop/point along the
//! edge with a live percent; releasing keeps it (Jason-specified, 2026-11-02:
//! the reference's pointer behaviour wasn't measured). One undo step.
//! Tool audit Tier 1 adds the placement options (Split at the Middle, Both
//! Sides, Absolute Distance, Preserve Curvature, Show Position/Distance) as
//! Tool Properties while a tool is armed. Not built yet: repositioning the
//! active one with later clicks, Keep Quads, and Split/Cap Sections/Gap
//! (greyed). The tab is drawn from its form, `assets/forms/toolbox.edge.json`
//! (layouts stage 3 batch 4); its Add Loop / Add Point rows run the
//! `Add Loop` / `Add Point` commands.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolFields, ToolForm};
use operandi_core::EdgeInsertOptions;

/// Tool properties of Add Loop or Add Point. `position` is the percent of
/// the last placement (AL-06: a click replaces it) and the default for
/// `edge.addLoop` / `edge.addPoint`.
/// The armed Add Loop / Add Point's Tool Properties.
pub(super) struct EdgeInsertFields;

impl ToolFields for EdgeInsertFields {
    type P = InsertSettings;
    fn params(app: &Operandi) -> Option<InsertSettings> {
        app.edge_click.map(|t| app.insert_settings(t).clone())
    }
    fn form(app: &Operandi, s: &InsertSettings) -> ToolForm<InsertSettings> {
        type P = InsertSettings;
        let mut f = ToolForm::new();
        let Some(tool) = app.edge_click else {
            return f;
        };
        let is_loop = tool == EdgeClick::AddLoop;
        let o = &s.options;
        f.title(tool.label())
            .field(
                tf::percent(
                    "insert_position",
                    "Position",
                    " %",
                    |p: &P| p.position / 100.,
                    |p, v| {
                        p.position = if v.is_finite() { v.clamp(1., 99.) } else { 50. };
                        Ok(())
                    },
                )
                .enabled_if(
                    !o.absolute && !s.middle,
                    if o.absolute {
                        "Absolute Distance is on: use Distance 1"
                    } else {
                        "Split at the Middle is on"
                    },
                ),
            )
            .field(
                tf::check(
                    "insert_middle",
                    "Split at the Middle",
                    |p: &P| p.middle,
                    |p, on| p.middle = on,
                )
                .tip("Every click lands at the middle of the edge (same as holding Ctrl)"),
            );
        if is_loop {
            f.field(
                tf::check(
                    "insert_both",
                    "Both Sides",
                    |p: &P| p.options.both_sides,
                    |p, on| p.options.both_sides = on,
                )
                .tip("Add two loops at once, mirrored about the middle (one undo step)"),
            );
        }
        let distance = |key: &str, label: &str, get: fn(&P) -> f64, set: fn(&mut P, f64)| {
            tf::number(key, label, Unit::Distance, get, move |p, v| {
                set(p, v.max(0.));
                Ok(())
            })
        };
        f.separator()
            .field(
                tf::check(
                    "insert_absolute",
                    "Absolute Distance",
                    |p: &P| p.options.absolute,
                    |p, on| p.options.absolute = on,
                )
                .tip("Place it a fixed distance from an edge end instead of at a percent; dragging sets Distance 1"),
            )
            .field(
                tf::check(
                    "insert_from_end",
                    "Offset from End",
                    |p: &P| p.options.from_end,
                    |p, on| p.options.from_end = on,
                )
                .tip("Measure Distance 1 from the edge's far end")
                .enabled_if(o.absolute, "Turn on Absolute Distance first"),
            )
            .field(
                distance(
                    "insert_distance1",
                    "Distance 1",
                    |p| p.options.distance1,
                    |p, v| p.options.distance1 = v,
                )
                .enabled_if(o.absolute, "Turn on Absolute Distance first"),
            )
            .field(
                distance(
                    "insert_distance2",
                    "Distance 2",
                    |p| p.options.distance2,
                    |p, v| p.options.distance2 = v,
                )
                .enabled_if(
                    o.absolute && o.both_sides && is_loop,
                    "Used by Add Loop with Absolute Distance and Both Sides on",
                ),
            )
            .separator()
            .field(
                tf::check(
                    "insert_curvature",
                    "Preserve Curvature",
                    |p: &P| p.options.preserve_curvature,
                    |p, on| p.options.preserve_curvature = on,
                )
                .tip("New points follow the surface's curve instead of the straight edge"),
            )
            .field(
                tf::percent(
                    "insert_tension",
                    "Tension",
                    " %",
                    |p: &P| p.options.tension,
                    |p, v| {
                        p.options.tension = if v.is_finite() {
                            (v / 100.).clamp(0., 10.)
                        } else {
                            1.
                        };
                        Ok(())
                    },
                )
                .enabled_if(o.preserve_curvature, "Turn on Preserve Curvature first"),
            )
            .separator()
            .field(
                tf::check(
                    "insert_show_position",
                    "Show Position",
                    |p: &P| p.show_position,
                    |p, on| p.show_position = on,
                )
                .tip("Show the percent beside the pointer while placing"),
            )
            .field(
                tf::check(
                    "insert_show_distance",
                    "Show Distance",
                    |p: &P| p.show_distance,
                    |p, on| p.show_distance = on,
                )
                .tip("Show the distance from the edge's start point while placing"),
            );
        if is_loop {
            let later = "Not built yet: its behaviour needs measuring first (tool audit Tier 1)";
            f.separator()
                .unbuilt("Keep Quads", "off", later)
                .unbuilt("Split", "off", later)
                .unbuilt("Cap Sections", "off", later)
                .unbuilt("Gap", "0 m", later);
        }
        f.note("Click an edge to add; hold and drag to slide it. Ctrl: middle.");
        f
    }
    fn publish(app: &mut Operandi, s: InsertSettings) -> Result<()> {
        let tool = app.edge_click.ok_or("The tool isn't active")?;
        *app.insert_settings_mut(tool) = s;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct InsertSettings {
    pub(super) position: f64,
    pub(super) middle: bool,
    pub(super) show_position: bool,
    pub(super) show_distance: bool,
    pub(super) options: EdgeInsertOptions,
}
impl Default for InsertSettings {
    fn default() -> Self {
        Self {
            position: 50.,
            middle: false,
            show_position: true,
            show_distance: false,
            options: EdgeInsertOptions::default(),
        }
    }
}
impl InsertSettings {
    /// Options for one placement: the middle (Ctrl or the checkbox) wins over
    /// Absolute Distance; Both Sides applies to Add Loop only.
    fn options_for(&self, tool: EdgeClick, middle: bool) -> EdgeInsertOptions {
        EdgeInsertOptions {
            absolute: self.options.absolute && !middle,
            both_sides: self.options.both_sides && tool == EdgeClick::AddLoop,
            ..self.options.clone()
        }
    }
}

/// The armed click tool of the Edge panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EdgeClick {
    AddLoop,
    AddPoint,
}
impl EdgeClick {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::AddLoop => "Add Loop",
            Self::AddPoint => "Add Point",
        }
    }
    fn command(self, a: u32, b: u32, t: f64, options: EdgeInsertOptions) -> Command {
        match self {
            Self::AddLoop => Command::AddEdgeLoopWith { a, b, t, options },
            Self::AddPoint => Command::AddEdgePointWith { a, b, t, options },
        }
    }
}

/// Input-map action of an Edge command row (distinct from Polygon actions
/// that share a label, such as Remove and Collapse).
pub(super) fn edge_action(command: EdgeCommand) -> &'static str {
    match command {
        EdgeCommand::Subdivide => "Edge Subdivide",
        EdgeCommand::Collapse => "Edge Collapse",
        EdgeCommand::Remove => "Edge Remove",
        EdgeCommand::Spin => "Edge Spin",
        EdgeCommand::Split => "Edge Split",
        other => other.label(),
    }
}

impl Operandi {
    /// `tool.attr edge.bevel|edge.chamfer <attribute> <value|?>`: the same
    /// properties as the panel, on the active Edge Bevel / Edge Chamfer.
    pub(super) fn edge_bevel_property(&mut self, bevel: bool, attr: &str, value: &str) {
        use operandi_core::{EdgeBevelMode, EdgeBevelShape, EdgeEnd};
        let tool = if bevel {
            Tool::EdgeBevel
        } else {
            Tool::EdgeChamfer
        };
        let Some(live) = self.editor.session.live.as_ref().filter(|l| l.tool == tool) else {
            self.status = format!("Activate {} first", tool.label());
            return;
        };
        let mut p = live.params.clone();
        let o = &mut p.edge_bevel;
        if value == "?" {
            self.status = match attr {
                "mode" => format!("{:?}", o.mode).to_ascii_lowercase(),
                "inset" | "offset" | "width" => format!("{} m", p.chamfer.offset),
                "mitering" | "miteringoffset" => format!("{} m", o.support),
                "level" => (p.chamfer.segments / 2).to_string(),
                "segments" => p.chamfer.segments.to_string(),
                "shape" => format!("{:?}", o.shape).to_ascii_lowercase(),
                "depth" => format!("{} %", p.chamfer.depth),
                "parallel" => (o.parallel as u8).to_string(),
                "loopslide" => (o.loop_slide as u8).to_string(),
                "miteringstyle" | "cornerpattern" => match o.corner {
                    operandi_core::CornerPattern::NoMitering => "neutral".into(),
                    operandi_core::CornerPattern::Balanced => "default".into(),
                    c => format!("{c:?}").to_ascii_lowercase(),
                },
                "stopatedge" => (o.stop_at_edge as u8).to_string(),
                "autoweld" => (o.auto_weld as u8).to_string(),
                "edgeend" => match o.edge_end {
                    EdgeEnd::Profile => "profile".into(),
                    EdgeEnd::TriangleFan => "triangleFan".into(),
                },
                "usemat" => (o.use_material as u8).to_string(),
                "matname" => o.material.clone(),
                _ => "Unknown attribute; use help tool.attr".into(),
            };
            return;
        }
        let flag = || match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "on" => Ok(true),
            "0" | "false" | "off" => Ok(false),
            _ => Err("Use true/on/1 or false/off/0".to_string()),
        };
        let parsed: Result<()> = (|| {
            match attr {
                "mode" => {
                    o.mode = match value.to_ascii_lowercase().as_str() {
                        "inset" => EdgeBevelMode::Inset,
                        "width" => EdgeBevelMode::Width,
                        _ => return Err("Mode: inset or width".into()),
                    }
                }
                "inset" | "offset" | "width" => {
                    p.chamfer.offset = numeric::parse_quantity(value, numeric::Quantity::Distance)?
                }
                // Mitering: the second handle's edge on each side (2026-11-23).
                "mitering" | "miteringoffset" if bevel => {
                    let d = numeric::parse_quantity(value, numeric::Quantity::Distance)?;
                    if d.is_nan() || d < 0. {
                        return Err("Mitering Offset is a distance of 0 or more".into());
                    }
                    o.support = d;
                }
                // Round Level, replaced by Segments (2026-11-25), still read
                // from older scripts: level L is 2·L segments.
                "level" if bevel => {
                    let level = numeric::parse_quantity(value, numeric::Quantity::Scalar)?;
                    if level.fract() != 0. || !(0. ..=32.).contains(&level) {
                        return Err("Round Level is a whole number 0–32".into());
                    }
                    p.chamfer.segments = (2 * level as u32).max(1);
                }
                // Segments as the panel shows them: the rows across the
                // strip, 1 being flat (the maintainer, 2026-11-26).
                "segments" => {
                    let n = numeric::parse_quantity(value, numeric::Quantity::Scalar)?;
                    if n.fract() != 0. || !(1. ..=64.).contains(&n) {
                        return Err("Segments is a whole number 1–64".into());
                    }
                    p.chamfer.segments = n as u32;
                }
                "shape" => {
                    o.shape = match value.to_ascii_lowercase().as_str() {
                        "round" => EdgeBevelShape::Round,
                        "square" => EdgeBevelShape::Square,
                        // EBC-Q12 v1 measured Sharp for Edge Chamfer too.
                        "sharp" => EdgeBevelShape::Sharp,
                        _ => return Err("Shape: round, square or sharp".into()),
                    }
                }
                // Script units: 0–1, or a value with % (map percent).
                "depth" => p.chamfer.depth = operandi_core::command_names::script_percent(value)?,
                "parallel" if !bevel => o.parallel = flag()?,
                "loopslide" if bevel => o.loop_slide = flag()?,
                // Mitering Style (Corner Pattern until 2026-11-25; the old
                // attribute and value names still work).
                "miteringstyle" | "cornerpattern" if bevel => {
                    o.corner = match value.to_ascii_lowercase().as_str() {
                        // Default was Balanced, and before that None.
                        "default" | "balanced" | "none" => operandi_core::CornerPattern::Balanced,
                        "uniform" => operandi_core::CornerPattern::Uniform,
                        "pinch" => operandi_core::CornerPattern::Pinch,
                        // Left out until they're finished (2026-11-26).
                        "patch" | "radial" => {
                            return Err(format!(
                                "Mitering Style {value} isn't available yet: default, neutral, uniform or pinch"
                            ));
                        }
                        // Neutral was No Mitering.
                        "neutral" | "nomitering" | "no-mitering" | "no_mitering" => {
                            operandi_core::CornerPattern::NoMitering
                        }
                        _ => {
                            return Err("Mitering Style: default, neutral, uniform or pinch".into());
                        }
                    }
                }
                "stopatedge" if !bevel => o.stop_at_edge = flag()?,
                "autoweld" if !bevel => o.auto_weld = flag()?,
                "edgeend" if !bevel => {
                    o.edge_end = match value.to_ascii_lowercase().as_str() {
                        "profile" => EdgeEnd::Profile,
                        "trianglefan" => EdgeEnd::TriangleFan,
                        _ => return Err("Edge End: profile or triangleFan".into()),
                    }
                }
                "usemat" => o.use_material = flag()?,
                "matname" => o.material = value.to_string(),
                _ => return Err("Unknown attribute; use help tool.attr".into()),
            }
            Ok(())
        })();
        if let Err(e) = parsed {
            self.status = e;
            return;
        }
        let (offset, segments, depth) = (p.chamfer.offset, p.chamfer.segments, p.chamfer.depth);
        let mitering = p.edge_bevel.support;
        if self.run_ok(Command::Preview(p)) {
            self.sync_numeric_value("chamfer_offset", offset, " m");
            self.sync_numeric_value("chamfer_depth", depth, "");
            if bevel {
                self.sync_numeric_value("ebc_mitering", mitering, " m");
            }
            self.sync_numeric_value("chamfer_segments", segments as f64, "");
        }
    }

    /// Run an Edge command row (panel, shortcut and `edge.<verb>`).
    pub(super) fn run_edge_command(&mut self, command: EdgeCommand) {
        let succeeded = self.run_ok(Command::EdgeCommand(command));
        if succeeded {
            self.status = format!("{} done", command.label());
        }
    }

    /// Arm or disarm an Edge click tool (panel row and `tool.set`).
    pub(super) fn toggle_edge_click(&mut self, tool: EdgeClick) {
        if self.edge_click == Some(tool) {
            self.edge_click = None;
            self.status = format!("{} off", tool.label());
            return;
        }
        if self.editor.session.has_active_task() {
            self.status = format!("Drop or Cancel the active tool before {}", tool.label());
            return;
        }
        self.edge_click = Some(tool);
        self.status = format!(
            "{}: click an edge (Ctrl: middle). Click the row again to finish.",
            tool.label()
        );
    }

    /// Nearest visible edge of the active item under `p`, with the pointer's
    /// position along it (0..1 from the first point).
    pub(super) fn pick_edge_parameter(&self, p: Pos2, r: Rect) -> Option<(u32, u32, f64)> {
        let item = self
            .editor
            .scene
            .active()
            .filter(|i| self.editor.scene.shows(i))?;
        let matrix = self.editor.scene.world_matrix(item.id).ok()?;
        let surface = self.pick_surface();
        // Edges pick where they show: on a smooth surface, traced in
        // pieces, each naming its cage edge.
        let mut candidates = crate::picking::PickCandidates::default();
        let reach = Rect::from_center_size(p, Vec2::splat(24.));
        self.item_pick_candidates(
            item,
            matrix,
            Mode::Edge,
            reach,
            &|_| true,
            &self.camera.projector(r),
            &mut candidates,
        );
        let mut best: Option<(f32, u32, u32, f64, [DVec3; 2])> = None;
        for (a, b, [(qa, za), (qb, zb)], [wa, wb]) in candidates.edge_spans() {
            let d = qb - qa;
            let s = ((p - qa).dot(d) / d.length_sq().max(1e-8)).clamp(0., 1.);
            let distance = input::segment_distance(p, qa, qb);
            let t = picking::span_world_parameter(&self.camera, s, za, zb);
            if distance < best.map_or(12., |b| b.0)
                && surface.visible(&self.camera, wa.lerp(wb, t), r)
            {
                best = Some((distance, a, b, t, [wa, wb]));
            }
        }
        let (_, a, b, t, piece) = best?;
        // Along the cage edge: a traced edge's pieces split it evenly.
        let path = crate::subdivision_view::edge_path(
            &self.subdivision_cache,
            item,
            matrix,
            (a.min(b), a.max(b)),
        );
        // From the low point; reported from `a`, the picked first point.
        let t = crate::subdivision_view::path_parameter(&path, piece, t);
        Some((a, b, if a <= b { t } else { 1. - t }))
    }

    /// Viewport click while an Edge click tool is armed. Returns whether the
    /// click was consumed.
    /// Click with an armed Add Loop / Add Point. Returns whether the click
    /// was taken, and the slide gesture to keep while the button is held.
    pub(super) fn edge_click_at(
        &mut self,
        p: Pos2,
        r: Rect,
        ctrl: bool,
    ) -> (bool, Option<input::GestureKind>) {
        let Some(tool) = self.edge_click else {
            return (false, None);
        };
        match self.pick_edge_parameter(p, r) {
            Some((a, b, t)) => {
                let settings = self.insert_settings(tool).clone();
                let middle = ctrl || settings.middle;
                // Keep the new point off the edge ends.
                let t = if middle { 0.5 } else { position_step(t) };
                let options = settings.options_for(tool, middle);
                let ends = self.edge_screen_ends(a, b, r);
                let succeeded = self.run_ok(tool.command(a, b, t, options.clone()));
                if succeeded {
                    let placed = self.placed_t(a, b, t, &options);
                    self.insert_settings_mut(tool).position = placed * 100.;
                    let text = self.insert_readout(tool, a, b, placed);
                    self.status = format!("{}: {text}", tool.label());
                    self.edge_insert_label = Some((p, text));
                    // Split at the Middle keeps the click at the middle; a
                    // Ctrl-click may still slide (as before).
                    if settings.middle {
                        return (true, None);
                    }
                    if let Some(ends) = ends {
                        return (
                            true,
                            Some(input::GestureKind::EdgeInsert { tool, a, b, ends }),
                        );
                    }
                }
            }
            None => self.status = format!("{}: click closer to an edge", tool.label()),
        }
        (true, None)
    }

    fn edge_screen_ends(&self, a: u32, b: u32, r: Rect) -> Option<[(Pos2, f64); 2]> {
        let item = self.editor.scene.active()?;
        let matrix = self.editor.scene.world_matrix(item.id).ok()?;
        let point = |v: u32| -> Option<DVec3> {
            Some(matrix.transform_point3(*item.mesh.vertices.get(v as usize)?))
        };
        let project = |v: u32| -> Option<(Pos2, f64)> { self.camera.project(point(v)?, r) };
        Some([project(a)?, project(b)?])
    }

    pub(super) fn insert_settings(&self, tool: EdgeClick) -> &InsertSettings {
        match tool {
            EdgeClick::AddLoop => &self.add_loop_settings,
            EdgeClick::AddPoint => &self.add_point_settings,
        }
    }

    pub(super) fn insert_settings_mut(&mut self, tool: EdgeClick) -> &mut InsertSettings {
        match tool {
            EdgeClick::AddLoop => &mut self.add_loop_settings,
            EdgeClick::AddPoint => &mut self.add_point_settings,
        }
    }

    /// World length of edge `a`–`b` of the active item.
    fn edge_world_length(&self, a: u32, b: u32) -> f64 {
        let Some(item) = self.editor.scene.active() else {
            return 0.;
        };
        let Ok(matrix) = self.editor.scene.world_matrix(item.id) else {
            return 0.;
        };
        let point = |v: u32| {
            item.mesh
                .vertices
                .get(v as usize)
                .map(|&p| matrix.transform_point3(p))
        };
        match (point(a), point(b)) {
            (Some(p), Some(q)) => p.distance(q),
            _ => 0.,
        }
    }

    /// Where on the clicked edge (0..1 from `a`) the first new point went:
    /// `t`, or the Absolute Distance placement (clamped as the core does).
    fn placed_t(&self, a: u32, b: u32, t: f64, options: &EdgeInsertOptions) -> f64 {
        if !options.absolute {
            return t;
        }
        let share = options.distance1 / self.edge_world_length(a, b).max(f64::MIN_POSITIVE);
        let t = if options.from_end { 1. - share } else { share };
        t.clamp(0.01, 0.99)
    }

    /// Viewport readout for a placement: Show Position (percent) and/or Show
    /// Distance (metres from the edge's start point `a`).
    pub(super) fn insert_readout(&self, tool: EdgeClick, a: u32, b: u32, t: f64) -> String {
        let settings = self.insert_settings(tool);
        let mut parts = Vec::new();
        if settings.show_position {
            let percent = t * 100.;
            parts.push(if (percent - percent.round()).abs() < 1e-6 {
                format!("{percent:.0}%")
            } else {
                format!("{percent:.1}%")
            });
        }
        if settings.show_distance {
            parts.push(format!("{:.3} m", t * self.edge_world_length(a, b)));
        }
        parts.join(" · ")
    }

    /// Slide the loop just added: undo it and add it again at the pointer's
    /// place along the edge, so the whole gesture stays one undo step.
    pub(super) fn edge_insert_slide(
        &mut self,
        tool: EdgeClick,
        a: u32,
        b: u32,
        ends: [(Pos2, f64); 2],
        p: Pos2,
    ) {
        let [(qa, za), (qb, zb)] = ends;
        let d = qb - qa;
        let s = ((p - qa).dot(d) / d.length_sq().max(1e-8)).clamp(0., 1.);
        let t = position_step(picking::span_world_parameter(&self.camera, s, za, zb));
        let mut settings = self.insert_settings(tool).clone();
        // With Absolute Distance the drag sets Distance 1 (from the end it
        // measures from) instead of a percent.
        if settings.options.absolute {
            let length = self.edge_world_length(a, b);
            let from_start = if settings.options.from_end { 1. - t } else { t };
            settings.options.distance1 = from_start * length;
        }
        let options = settings.options_for(tool, false);
        let before = self.editor.scene.clone();
        if !self.run_ok(Command::Undo) {
            return;
        }
        if self.run_ok(tool.command(a, b, t, options.clone())) {
            let placed = self.placed_t(a, b, t, &options);
            settings.position = placed * 100.;
            *self.insert_settings_mut(tool) = settings;
            let text = self.insert_readout(tool, a, b, placed);
            self.status = format!("{}: {text}", tool.label());
            self.edge_insert_label = Some((p, text));
        } else {
            // Put the previous placement back.
            let _ = self.run_ok(Command::Redo);
            if self.editor.scene != before {
                self.status = format!("{}: can't place it there", tool.label());
            }
        }
    }

    /// `edge.addLoop` / `edge.addPoint [position:<0–1 or %>]` on the selected
    /// edges of the active mesh (Edge mode).
    pub(super) fn edge_insert_command(&mut self, tool: EdgeClick, args: &[String]) {
        let settings = self.insert_settings(tool).clone();
        let mut percent = if settings.middle {
            50.
        } else {
            settings.position
        };
        for arg in args {
            match arg.split_once(':') {
                Some((k, v)) if k.eq_ignore_ascii_case("position") => {
                    // Script units: 0–1, or a value with % (map percent).
                    match operandi_core::command_names::script_percent(v) {
                        Ok(x) if x > 0. && x < 100. => percent = x,
                        _ => {
                            self.status =
                                "Position must be between 0 and 1 (or 0% and 100%)".into();
                            return;
                        }
                    }
                }
                _ => {
                    self.status = format!(
                        "Use {} [position:<0–1 or %>]",
                        if tool == EdgeClick::AddLoop {
                            "edge.addLoop"
                        } else {
                            "edge.addPoint"
                        }
                    );
                    return;
                }
            }
        }
        if self.editor.scene.selection.mode != Mode::Edge
            || self.editor.scene.selection.edges.is_empty()
        {
            self.status = format!("Select edges in Edge mode for {}", tool.label());
            return;
        }
        // Each selected edge that still exists gets one edit; one undo step
        // per edge keeps every result individually undoable.
        let edges = self.editor.scene.selection.edges.clone();
        let mut done = 0;
        for (a, b) in edges {
            let exists = self
                .editor
                .scene
                .active()
                .is_some_and(|i| i.mesh.edges().contains(&(a.min(b), a.max(b))));
            if !exists {
                continue;
            }
            let options = settings.options_for(tool, settings.middle);
            let succeeded = self.run_ok(tool.command(a, b, percent / 100., options));
            if !succeeded {
                return;
            }
            done += 1;
        }
        self.status = format!("{}: {done} edge(s) at {percent}%", tool.label());
    }
}

/// AL-02/03, AL-PHY-01/02: the click and slide land on 0.5% steps (74.5%,
/// 43.5%, 25.5% were measured), kept off the edge ends. Provisional: one
/// measured sample at another zoom rounded to 1% instead (AL-Q02).
pub(super) fn position_step(t: f64) -> f64 {
    ((t * 200.).round() / 200.).clamp(0.01, 0.99)
}
