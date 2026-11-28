//! Tool Properties of the path tools (Curve Extrude, Curve Clone and their
//! presets; CX-xx, `operandi_core::path_tools`): the path's own options,
//! Path Steps, then the Sweep or Clone effector (CX-01: path + effector),
//! as field lists for the shared form (UIC-110..117). The viewport side is
//! `path_tool_view.rs`.
use super::*;
use crate::form::Unit;
use crate::live_fields::{check, pick};
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::path_tools::{PathKind, PathTool, ScalePivot};

type Form = ToolForm<Parameters>;
type Lens = (
    fn(&Parameters) -> &PathTool,
    fn(&mut Parameters) -> &mut PathTool,
);

/// What a press in the view does to the path (DPM-01's modes, plus Sketch
/// for a freehand stroke fitted to a smooth curve, curve_fit.rs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PathMode {
    /// Click empty space adds a point after the last; drag a point to move it.
    #[default]
    Add,
    /// Clicks never add points; drag points to tidy up.
    Edit,
    /// Click a point to remove it.
    Delete,
    /// Drag a freehand stroke: it replaces the path, fitted to a few smooth
    /// spans.
    Sketch,
}

pub(crate) fn lens(tool: Tool) -> Lens {
    match tool {
        Tool::CurveExtrude => (
            |p| &p.duplicate.curve_extrude,
            |p| &mut p.duplicate.curve_extrude,
        ),
        _ => (
            |p| &p.duplicate.curve_clone,
            |p| &mut p.duplicate.curve_clone,
        ),
    }
}

/// Is `tool` one of the path tools?
pub(crate) fn is_path_tool(tool: Tool) -> bool {
    matches!(tool, Tool::CurveExtrude | Tool::CurveClone)
}

fn percent(
    key: &str,
    label: &str,
    get: impl Fn(&Parameters) -> f64 + 'static,
    set: impl Fn(&mut Parameters, f64) + 'static,
) -> tf::Field<Parameters> {
    crate::edit_panel::slice_num(key, label, Unit::Percent, get, move |p, v| {
        if !(v.is_finite() && v > 0.) {
            return Err("Size must be more than 0 %".into());
        }
        set(p, v);
        Ok(())
    })
}

fn angle(
    key: &str,
    label: &str,
    get: impl Fn(&Parameters) -> f64 + 'static,
    set: impl Fn(&mut Parameters, f64) + 'static,
) -> tf::Field<Parameters> {
    crate::edit_panel::slice_num(key, label, Unit::Angle, get, move |p, v| {
        if !v.is_finite() {
            return Err("must be a finite number".into());
        }
        set(p, v);
        Ok(())
    })
}

impl Operandi {
    pub(super) fn path_tool_form(&self, tool: Tool, p: &Parameters, f: &mut Form) {
        let (get, get_mut) = lens(tool);
        let t = get(p);
        let kind = t.curve.kind;
        let title = match kind {
            PathKind::Curve => "Curve Path",
            PathKind::Bezier => "Bezier Path",
            PathKind::BSpline => "B-Spline Path",
            PathKind::Pen => "Pen Path",
        };
        f.section(title, false, path_rows(get, get_mut, t));
        if kind.uses_steps() {
            f.section("Path Steps", false, steps_rows(get, get_mut, t));
        }
        if tool == Tool::CurveExtrude {
            f.section(
                "Sweep Effector",
                false,
                sweep_rows(get, get_mut, kind == PathKind::Pen),
            );
        } else {
            let instancing = operandi_core::duplicate_items::instancing(&p.duplicate);
            f.section(
                "Clone Effector",
                false,
                clone_rows(get, get_mut, t, kind, instancing),
            );
            f.section(
                "Item",
                false,
                crate::duplicate_item_rows::item_rows(
                    |p| &p.duplicate.curve_clone.clone.copies,
                    |p| &mut p.duplicate.curve_clone.clone.copies,
                    instancing,
                    false,
                ),
            );
        }
        f.buttons(vec![
            tf::Button::change("Clear Path", move |p: &mut Parameters| {
                get_mut(p).curve.points.clear()
            })
            .tip("Remove every path point and start again")
            .enabled_if(!t.curve.points.is_empty(), "The path has no points yet"),
        ]);
        let live = self.editor.session.live.as_ref();
        if let Some(error) = live.and_then(|l| l.preview_error.as_ref()) {
            f.error(error.clone());
        }
        f.note(match (tool, t.has_path()) {
            (_, false) if kind == PathKind::Bezier => {
                "Click in the view to place the path's anchors; each click adds smooth handles you can drag. Sketch mode: drag a freehand path."
            }
            (_, false) => {
                "Click in the view to place path points (on the surface or the work plane). Sketch mode: drag a freehand path."
            }
            (Tool::CurveExtrude, true) => {
                "Drag points to reshape the path; click empty space to add more. Drop (Space) keeps the result."
            }
            _ => {
                "Drag points to reshape the path; click empty space to add more. Drop (Space) keeps the copies."
            }
        });
    }
}

type Get = fn(&Parameters) -> &PathTool;
type GetMut = fn(&mut Parameters) -> &mut PathTool;

fn mode_row() -> tf::Item<Parameters> {
    tf::Item::Custom(std::rc::Rc::new(|app: &mut Operandi, ui: &mut Ui| {
        let options = [
            (PathMode::Add, "Add"),
            (PathMode::Edit, "Edit"),
            (PathMode::Delete, "Delete"),
            (PathMode::Sketch, "Sketch"),
        ];
        let names: Vec<&str> = options.iter().map(|(_, n)| *n).collect();
        let now = app.point_modes.path;
        let mut index = options.iter().position(|(m, _)| *m == now).unwrap_or(0);
        let row = ui.scope(|ui| crate::form::rows::segments(ui, "Mode", &mut index, &names));
        // UIC-04: the tooltip names each mode's gesture.
        row.response.on_hover_text(
            "Add: click empty space adds a point after the last; drag a point to move it\n\
             Edit: clicks never add points; drag points\n\
             Delete: click a point to remove it (a Bezier anchor goes with its handles)\n\
             Sketch: drag a freehand stroke; it becomes a smooth path",
        );
        if row.inner
            && let Some((mode, _)) = options.get(index)
        {
            app.point_modes.path = *mode;
        }
    }))
}

fn path_rows(get: Get, get_mut: GetMut, t: &PathTool) -> Vec<tf::Item<Parameters>> {
    let kind = t.curve.kind;
    let mut f = Form::new();
    f.items.push(mode_row());
    f.field(
        check(
            "path_closed",
            "Closed",
            move |p| get(p).curve.closed,
            move |p, on| get_mut(p).curve.closed = on,
        )
        .tip("Join the path's end back to its start"),
    );
    if kind == PathKind::Curve {
        f.field(
            check(
                "path_start_control",
                "Start Control",
                move |p| get(p).curve.start_control,
                move |p, on| get_mut(p).curve.start_control = on,
            )
            .tip("The first point shapes the curve without being on it")
            .enabled_if(!t.curve.closed, "A closed path has no ends"),
        )
        .field(
            check(
                "path_end_control",
                "End Control",
                move |p| get(p).curve.end_control,
                move |p, on| get_mut(p).curve.end_control = on,
            )
            .tip("The last point shapes the curve without being on it")
            .enabled_if(!t.curve.closed, "A closed path has no ends"),
        );
    }
    if kind.uses_steps() {
        f.field(
            check(
                "path_by_length",
                "By Length",
                move |p| get(p).curve.by_length,
                move |p, on| get_mut(p).curve.by_length = on,
            )
            .tip("Space the steps evenly along the path's length, not by its points")
            .enabled_if(!t.steps.automatic, "Automatic places the steps by bend"),
        );
    }
    f.field(
        check(
            "path_curve_polygon",
            "Create Curve Polygon",
            move |p| get(p).curve.create_curve_polygon,
            move |p, on| get_mut(p).curve.create_curve_polygon = on,
        )
        .tip("Leave the path behind in the mesh as a curve"),
    )
    .field(
        angle(
            "path_bank",
            "Bank",
            move |p| get(p).curve.bank,
            move |p, v| get_mut(p).curve.bank = v,
        )
        .tip("Rolls every section about the path"),
    )
    .field(
        angle(
            "path_twist",
            "Twist",
            move |p| get(p).curve.twist,
            move |p, v| get_mut(p).curve.twist = v,
        )
        .tip("Roll spread along the path: none at the start, all of it at the end"),
    );
    if kind == PathKind::Pen {
        f.field(
            check(
                "pen_align",
                "Align",
                move |p| get(p).pen_align,
                move |p, on| get_mut(p).pen_align = on,
            )
            .tip("Turn each copy to follow the path; at a bend, halfway between the two segments"),
        );
    }
    f.items
}

fn steps_rows(get: Get, get_mut: GetMut, t: &PathTool) -> Vec<tf::Item<Parameters>> {
    let mut f = Form::new();
    f.field(
        crate::edit_panel::slice_num(
            "path_steps",
            "Steps",
            Unit::Count,
            move |p| get(p).steps.number as f64,
            move |p, v| {
                get_mut(p).steps.number =
                    crate::path_slice_panel::whole(v, 1., 10_000., "Steps")? as u32;
                Ok(())
            },
        )
        .tip("How many steps along the path (Clone: copies, the original included)")
        .enabled_if(!t.steps.automatic, "Automatic chooses the steps"),
    )
    .field(
        check(
            "path_automatic",
            "Automatic",
            move |p| get(p).steps.automatic,
            move |p, on| get_mut(p).steps.automatic = on,
        )
        .tip("Add steps where the path bends more than Smoothing Angle"),
    )
    .field(
        angle(
            "path_smoothing",
            "Smoothing Angle",
            move |p| get(p).steps.smoothing_angle,
            move |p, v| get_mut(p).steps.smoothing_angle = v,
        )
        .tip("With Automatic: the most the path may turn between two steps")
        .enabled_if(t.steps.automatic, "Turn on Automatic to use it"),
    )
    .field(
        check(
            "path_align",
            "Align to Path",
            move |p| get(p).steps.align_to_path,
            move |p, on| get_mut(p).steps.align_to_path = on,
        )
        .tip("Turn each section to follow the path's direction; off only moves them"),
    )
    .field(
        check(
            "path_start_at_source",
            "Start at Source",
            move |p| get(p).steps.start_at_source,
            move |p, on| get_mut(p).steps.start_at_source = on,
        )
        .tip("Move the path so it starts on the selection; off uses it where it was drawn"),
    );
    f.items
}

fn sweep_rows(get: Get, get_mut: GetMut, pen: bool) -> Vec<tf::Item<Parameters>> {
    let mut f = Form::new();
    if pen {
        f.field(
            check(
                "sweep_corner_scale",
                "Corner Scale",
                move |p| get(p).corner_scale,
                move |p, on| get_mut(p).corner_scale = on,
            )
            .tip("At a bend, widen the section across it so the walls keep their thickness"),
        );
    }
    f.field(
        check(
            "sweep_invert",
            "Invert",
            move |p| get(p).sweep.invert,
            move |p, on| get_mut(p).sweep.invert = on,
        )
        .tip("On: the swept shape faces out (the buttons turn this on); off turns it inside out"),
    )
    .field(
        check(
            "sweep_cap_start",
            "Cap Start",
            move |p| get(p).sweep.cap_start,
            move |p, on| get_mut(p).sweep.cap_start = on,
        )
        .tip("Keep the original polygons as the start cap; off leaves the start open"),
    )
    .field(
        check(
            "sweep_cap_end",
            "Cap End",
            move |p| get(p).sweep.cap_end,
            move |p, on| get_mut(p).sweep.cap_end = on,
        )
        .tip("Close the far end with a copy of the polygons"),
    )
    .field(
        percent(
            "sweep_size",
            "Size",
            move |p| get(p).sweep.size,
            move |p, v| get_mut(p).sweep.size = v,
        )
        .tip("The last section's size; the ones between change evenly"),
    );
    f.items
}

fn clone_rows(
    get: Get,
    get_mut: GetMut,
    t: &PathTool,
    _kind: PathKind,
    instancing: bool,
) -> Vec<tf::Item<Parameters>> {
    let mut f = Form::new();
    let c = &t.clone.copies;
    f.field(
        check(
            "path_copies_replace",
            "Replace Source",
            move |p| get(p).clone.copies.replace_source,
            move |p, on| get_mut(p).clone.copies.replace_source = on,
        )
        .tip("Move the original onto the path's first point, placed like the copies"),
    )
    .field(
        check(
            "path_copies_flip",
            "Invert Polygons",
            move |p| get(p).clone.copies.flip,
            move |p, on| get_mut(p).clone.copies.flip = on,
        )
        .tip("Turns the copies' normals the other way"),
    )
    .field(
        check(
            "path_copies_merge",
            "Merge Vertices",
            move |p| get(p).clone.copies.merge,
            move |p, on| get_mut(p).clone.copies.merge = on,
        )
        .tip("Weld copy points within Distance of another point")
        .enabled_if(
            !instancing,
            "Instances are separate items, so nothing welds",
        ),
    )
    .field(
        crate::edit_panel::slice_num(
            "path_copies_distance",
            "Distance",
            Unit::Distance,
            move |p| get(p).clone.copies.distance,
            move |p, v| {
                if !(v.is_finite() && v >= 0.) {
                    return Err("Distance must be 0 or more".into());
                }
                get_mut(p).clone.copies.distance = v;
                Ok(())
            },
        )
        .tip("How close points must be to weld; 0 welds only points on top of each other")
        .enabled_if(c.merge, "Turn on Merge Vertices to weld"),
    )
    .field(
        percent(
            "clone_size",
            "Size",
            move |p| get(p).clone.size,
            move |p, v| get_mut(p).clone.size = v,
        )
        .tip("The last copy's size; the ones between change evenly"),
    )
    .field(pick(
        "clone_pivot",
        "Scale Pivot",
        &[
            (ScalePivot::Bottom, "Bottom"),
            (ScalePivot::Center, "Center"),
        ],
        move |p| get(p).clone.pivot,
        move |p, v| get_mut(p).clone.pivot = v,
    ));
    f.items
}
