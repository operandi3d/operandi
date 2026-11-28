//! Scoped interactive Polygon Pen controls; values address captured points only.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolFields, ToolForm};
impl Operandi {
    pub(super) fn start_interactive_pen(&mut self) {
        let already = self.editor.session.pen_draft.is_some();
        self.run(Command::PenStart);
        if !already && self.editor.session.pen_draft.is_some() {
            self.clear_fields();
            self.tool_properties_open = true;
        }
    }
    /// The Pen draft's drawing-plane normal: the plane fixed by its first
    /// point, else the plane the view would draw on now.
    pub(super) fn pen_plane_normal(&self) -> DVec3 {
        let plane = match (&self.editor.session.pen_draft, self.pen_plane) {
            (Some(d), Some((plane, _))) if !d.points().is_empty() => plane,
            _ => self.actual_plane(),
        };
        match plane {
            0 => DVec3::Y,
            1 => DVec3::X,
            _ => DVec3::Z,
        }
    }
    /// The view Make UVs maps from (BD-62): the axis view being drawn in,
    /// when it shows Backdrops. Perspective, the free Orthographic view and
    /// camera views keep world X and Y (provisional, BD-Q22).
    pub(super) fn pen_uv_view(&self) -> Option<operandi_core::backdrop::ViewKind> {
        use operandi_core::backdrop::ViewKind;
        if !self.display.settings.drawing.backdrops || self.camera.through {
            return None;
        }
        match crate::backdrop_ui::view_kind(self.camera.view) {
            kind @ ViewKind::Axis(_) => Some(kind),
            ViewKind::Free => None,
        }
    }
    /// Note the view a Pen point is drawn in, for Make UVs (BD-62).
    pub(super) fn sync_pen_uv_view(&mut self) {
        let Some(draft) = &self.editor.session.pen_draft else {
            return;
        };
        let mut options = draft.options();
        let view = self.pen_uv_view();
        if options.uv_view != view {
            options.uv_view = view;
            self.run(Command::PenOptions(options));
        }
    }
    /// Keep the draft's wall plane in step with the drawing plane.
    pub(super) fn sync_pen_plane(&mut self) {
        let Some(draft) = &self.editor.session.pen_draft else {
            return;
        };
        let mut options = draft.options();
        let normal = self.pen_plane_normal();
        if options.plane_normal != normal {
            options.plane_normal = normal;
            self.run(Command::PenOptions(options));
        }
    }
    /// Right-drag with Pen walls: Offset follows the pointer's sideways
    /// travel (one undo-free option change per move, like the panel field).
    pub(super) fn pen_offset_drag(&mut self, start_offset: f64, metres_per_pixel: f64, dx: f32) {
        let Some(draft) = &self.editor.session.pen_draft else {
            return;
        };
        let mut options = draft.options();
        options.offset = (start_offset + dx as f64 * metres_per_pixel).max(0.);
        options.plane_normal = self.pen_plane_normal();
        self.run(Command::PenOptions(options));
        for map in [&mut self.fields, &mut self.confirmed_fields] {
            map.remove("pen_offset");
        }
        self.status = format!(
            "Pen · Offset {}",
            workspace::number_text(options.offset, "m")
        );
    }
}

/// The Pen's Tool Properties: its options plus the current point.
pub(super) struct PenFields;

#[derive(Clone, PartialEq)]
pub(super) struct PenP {
    options: PenOptions,
    current: Option<usize>,
    point: Option<DVec3>,
}

impl ToolFields for PenFields {
    type P = PenP;
    fn params(app: &Operandi) -> Option<PenP> {
        let draft = app.editor.session.pen_draft.as_ref()?;
        let current = draft.current();
        Some(PenP {
            options: draft.options(),
            current,
            point: current.map(|i| draft.points()[i]),
        })
    }
    fn form(_: &Operandi, p: &PenP) -> ToolForm<PenP> {
        let mut f = ToolForm::new();
        let o = p.options;
        let show = "Viewport overlays for Pen aren't specified yet";
        // PT-PEN-01: Type is a popup, in the pen-2026-11-05 order; Spline
        // Patch isn't built yet.
        let types: Vec<(Option<PenType>, &str)> = [
            Some(PenType::Polygon),
            Some(PenType::Vertices),
            None,
            Some(PenType::Subdivs),
            Some(PenType::Polyline),
            Some(PenType::CatmullClark),
            Some(PenType::BSpline),
        ]
        .into_iter()
        .map(|t| (t, t.map_or("Spline Patch", pen_type_label)))
        .collect();
        f.title("Pen");
        // DPM-10 (D1, Jason 2026-11-13): Pen gets the curve tools' Mode.
        f.push(crate::draw_tools::app_mode_row(
            |m| m.pen,
            |m, mode| m.pen = mode,
        ));
        f.field(
            tf::choice(
                "pen_type",
                "Type",
                &types,
                |p: &PenP| Some(p.options.output),
                |p, t| {
                    if let Some(t) = t {
                        p.options.output = t;
                        if !t.surface() {
                            p.options.wall = WallMode::Off;
                        }
                    }
                },
            )
            .greyed_option(2, "Operandi has no spline patch polygons yet"),
        );
        if p.current.is_some() {
            f.field(tf::number(
                "pen_current",
                "Current Point",
                Unit::Count,
                |p: &PenP| p.current.unwrap_or_default() as f64,
                |p, v| {
                    if v < 0. || v.fract() != 0. || v > usize::MAX as f64 {
                        return Err("Current Point must be a captured integer index".into());
                    }
                    p.current = Some(v as usize);
                    Ok(())
                },
            ));
            for (axis, key, label) in [(0, "pen_x", "X"), (1, "pen_y", "Y"), (2, "pen_z", "Z")] {
                f.field(
                    tf::number(
                        key,
                        label,
                        Unit::Distance,
                        move |p: &PenP| p.point.unwrap_or_default()[axis],
                        move |p, v| {
                            let point = p.point.as_mut().ok_or("Capture a point first")?;
                            point[axis] = v;
                            Ok(())
                        },
                    )
                    .suffix("m"),
                );
            }
        } else {
            f.field(tf::value(
                "pen_no_point",
                "Current Point",
                "capture a point first",
            ));
        }
        let walls = o.wall != WallMode::Off;
        f.field(
            tf::check(
                "pen_flip",
                "Flip Polygon",
                |p: &PenP| p.options.flip,
                |p, on| p.options.flip = on,
            )
            .tip("Reverse the new polygons so they face the other way"),
        )
        .field(
            tf::check(
                "pen_close",
                "Close",
                |p: &PenP| p.options.close,
                |p, on| p.options.close = on,
            )
            .tip("Join the last point back to the first (Polyline and walls)"),
        )
        // Spec: PEN-R02 (only the numeric route was measured; no mouse
        // merge distance)
        .unbuilt(
            "Merge",
            "off",
            "Off: merging clicked points into existing ones isn't measured yet",
        )
        // SPEC-PEN-MAKE-QUADS: clicks build a strip of quads. UIC-92: greyed
        // in place, with the reason in the tooltip (UIC-04), when it can't
        // apply or can't change.
        .field(
            tf::check(
                "pen_make_quads",
                "Make Quads",
                |p: &PenP| p.options.make_quads,
                |p, on| p.options.make_quads = on,
            )
            .tip("Click a starting edge, then each click adds a quad; Ctrl+click ends the strip with a triangle, Shift+click starts a new strip")
            // The last reason that applies is the one shown.
            .enabled_if(
                p.current.is_none(),
                "Make Quads can't change mid-draft; Drop or Shift+click first",
            )
            .enabled_if(
                !walls,
                "Make Quads with Wall Mode isn't specified yet; set Wall Mode to Off",
            )
            .enabled_if(o.output.surface(), "Make Quads applies to the polygon types"),
        )
        .separator()
        // Draw-tools Pen walls (Jason QA 2026-11-05): the clicked path
        // becomes a strip of quads. PT-PEN-08: Wall Mode is a popup.
        .field(
            tf::choice(
                "pen_wall",
                "Wall Mode",
                &[
                    WallMode::Off,
                    WallMode::BothSides,
                    WallMode::Inner,
                    WallMode::Outer,
                ]
                .map(|m| (m, wall_label(m))),
                |p: &PenP| p.options.wall,
                |p, m| {
                    if m != WallMode::Off
                        && p.options.wall == WallMode::Off
                        && p.options.offset <= 0.
                    {
                        p.options.offset = 0.1;
                    }
                    p.options.wall = m;
                },
            )
            .tip("Turn the clicked path into a wall strip; right-drag sets Offset")
            .enabled_if(
                !o.make_quads,
                "Wall Mode with Make Quads isn't specified yet; turn Make Quads off",
            )
            .enabled_if(o.output.surface(), "Wall Mode applies to the polygon types"),
        )
        .field(
            tf::number(
                "pen_offset",
                "Offset",
                Unit::Distance,
                |p: &PenP| p.options.offset,
                |p, v| {
                    if v < 0. {
                        return Err("Offset must be 0 or more".into());
                    }
                    p.options.offset = v;
                    Ok(())
                },
            )
            .suffix("m")
            .enabled_if(walls, "Choose a Wall Mode first"),
        )
        .field(
            tf::number(
                "pen_inset",
                "Inset",
                Unit::Distance,
                |p: &PenP| p.options.inset,
                |p, v| {
                    p.options.inset = v;
                    Ok(())
                },
            )
            .suffix("m")
            .enabled_if(walls, "Choose a Wall Mode first"),
        )
        .field(
            tf::count(
                "pen_segments",
                "Segments",
                (1., 256.),
                "Segments must be a whole number 1–256",
                |p: &PenP| p.options.segments as f64,
                |p, n| p.options.segments = n,
            )
            .enabled_if(walls, "Choose a Wall Mode first"),
        )
        .separator()
        // Spec: PEN-R05 (label/handle placement unresolved)
        .unbuilt("Show Angles", "off", show)
        .unbuilt("Show Handles", "off", show)
        .unbuilt("Show Numbers", "off", show)
        .field(
            tf::check(
                "pen_select_new",
                "Select New",
                |p: &PenP| p.options.select_new,
                |p, on| p.options.select_new = on,
            )
            .tip("Select the new element after Drop; off keeps the current selection"),
        )
        .separator()
        // pen-2026-11-05 rule 7: world X and Y (Project To: Backdrop Item,
        // with no backdrop); BD-62 with a Backdrop drawn in the view: the
        // picture's UVs. Other projections aren't measured (Q-PEN-03).
        .field(
            tf::check(
                "pen_make_uvs",
                "Make UVs",
                |p: &PenP| p.options.make_uvs,
                |p, on| p.options.make_uvs = on,
            )
            .tip("UV map Texture: each point's place on the Backdrop picture under it in the view you draw in, else its world X and Y"),
        )
        .unbuilt(
            "Project To",
            "Backdrop Item",
            "Action Axis and U/V Direction aren't measured yet",
        )
        .separator()
        // Spec: PEN-R06 (recall does not restore the point path)
        .unbuilt(
            "Tool States",
            "(none)",
            "Named Pen tool states aren't available yet",
        )
        .buttons(vec![
            tf::Button::command("Drop", Command::Drop),
            tf::Button::run("Cancel", |app| {
                app.run(Command::Cancel);
                app.clear_fields();
            }),
        ]);
        f
    }
    fn publish(app: &mut Operandi, p: PenP) -> Result<()> {
        let draft = app.editor.session.pen_draft.clone().ok_or("Pen ended")?;
        let mut options = p.options;
        if !options.output.surface() {
            options.wall = WallMode::Off;
        }
        if options.wall != WallMode::Off {
            options.plane_normal = app.pen_plane_normal();
        }
        let current = draft.current();
        let moved = p.point != current.map(|i| draft.points()[i]);
        let picked = p.current != current;
        if moved && picked {
            return Err("Confirm point coordinates before changing Current Point".into());
        }
        // Staged on a copy, so a refused step leaves the draft as it was.
        let mut candidate = app.editor.clone();
        if options != draft.options() {
            candidate.dispatch(Command::PenOptions(options))?;
        }
        if let (true, Some(index)) = (picked, p.current) {
            candidate.dispatch(Command::PenSelect(index))?;
        } else if let (true, Some(point)) = (moved, p.point) {
            candidate.dispatch(Command::PenRefine(point))?;
        }
        app.editor = candidate;
        Ok(())
    }
    fn published(app: &mut Operandi) {
        app.status = "Ready".into();
    }
}

fn wall_label(mode: WallMode) -> &'static str {
    match mode {
        WallMode::Off => "Off",
        WallMode::BothSides => "Both Sides",
        WallMode::Inner => "Inner",
        WallMode::Outer => "Outer",
    }
}

fn pen_type_label(output: PenType) -> &'static str {
    match output {
        PenType::Polygon => "Polygon",
        PenType::Vertices => "Vertices",
        PenType::Subdivs => "Subdivision",
        PenType::Polyline => "Polyline",
        PenType::CatmullClark => "Catmull-Clark",
        PenType::BSpline => "B-Spline",
    }
}
