//! Tool Properties of the Edit tab's wave 2 path slicing tools (Pen Slice,
//! Curve Slice, Primitive Slice; spec WAVE2-PATH-SLICE) and wave 3's
//! Polygon Axis Slice (WAVE3-AXIS-SLICE). Viewport gestures and drawing are in `path_slice_view.rs`.
//!
//! Their rows are field lists for the shared Tool Properties form
//! (`live_fields.rs`).
use super::*;
use operandi_core::{AxisSliceMode, PathCutter, PrimitiveShape, SliceAxis, SliceOperation};

/// Why the boolean options are greyed.
const BOOLEAN_LATER: &str = "Boolean operation types come with the Booleans work (wave 4)";

impl Operandi {
    /// Tool Properties of the wave 2 and 3 tools.
    pub(super) fn path_slice_form(&self, tool: Tool, p: &Parameters, f: &mut ToolForm<Parameters>) {
        match tool {
            Tool::PenSlice => {
                let o = &p.slice.pen_slice;
                f.field(pick(
                    "pen-slice-type",
                    "Type",
                    &[(true, "Closed"), (false, "Open")],
                    |p| p.slice.pen_slice.closed,
                    |p, c| p.slice.pen_slice.closed = c,
                ))
                .separator();
                cutter_fields(
                    f,
                    "pen_slice",
                    &o.cutter,
                    |p| &p.slice.pen_slice.cutter,
                    |p| &mut p.slice.pen_slice.cutter,
                );
                f.note(if o.points.is_empty() {
                    "Click on the work plane to place points; the path cuts along the Axis. Drag a point to move it."
                } else {
                    "Click to add points, drag one to move it. Drop (Q) keeps the cut."
                });
                remove_last(f, tool, !o.points.is_empty());
            }
            Tool::CurveSlice => {
                let o = &p.slice.curve_slice;
                f.field(
                    check(
                        "curve_slice_closed",
                        "Closed",
                        |p| p.slice.curve_slice.closed,
                        |p, on| p.slice.curve_slice.closed = on,
                    )
                    .tip("Join the curve's end back to its start"),
                )
                .field(
                    slice_num(
                        "curve_slice_steps",
                        "Steps",
                        Unit::Count,
                        |p| p.slice.curve_slice.steps as f64,
                        |p, v| {
                            p.slice.curve_slice.steps = whole(v, 1., 4096., "Steps")? as u32;
                            Ok(())
                        },
                    )
                    .enabled_if(!o.auto_steps, "Turn off Auto first"),
                )
                .field(
                    check(
                        "curve_slice_auto",
                        "Auto",
                        |p| p.slice.curve_slice.auto_steps,
                        |p, on| p.slice.curve_slice.auto_steps = on,
                    )
                    .tip("Pick the number of steps by how sharply the curve turns"),
                )
                .field(
                    slice_num(
                        "curve_slice_smooth",
                        "Smooth Angle",
                        Unit::Angle,
                        |p| p.slice.curve_slice.smooth_angle,
                        |p, v| {
                            if !(0.1..=90.).contains(&v) {
                                return Err("Smooth Angle must be from 0.1° to 90°".into());
                            }
                            p.slice.curve_slice.smooth_angle = v;
                            Ok(())
                        },
                    )
                    .enabled_if(o.auto_steps, "Turn on Auto first"),
                )
                .separator();
                cutter_fields(
                    f,
                    "curve_slice",
                    &o.cutter,
                    |p| &p.slice.curve_slice.cutter,
                    |p| &mut p.slice.curve_slice.cutter,
                );
                f.note(if o.points.is_empty() {
                    "Click on the work plane to draw a curve through the points; it cuts along the Axis."
                } else {
                    "Click to add points, drag one to move it. Drop (Q) keeps the cut."
                });
                remove_last(f, tool, !o.points.is_empty());
            }
            Tool::PrimitiveSlice => primitive_slice_form(p, f),
            Tool::AxisSlice => axis_slice_form(p, f),
            _ => {}
        }
    }
}

use crate::edit_panel::slice_num;
use crate::form::Unit;
use crate::live_fields::{check, pick, vec3};
use crate::tool_form::{self as tf, ToolForm};

type Cutter = (
    fn(&Parameters) -> &PathCutter,
    fn(&mut Parameters) -> &mut PathCutter,
);

/// Remove Last Point under a path tool.
fn remove_last(f: &mut ToolForm<Parameters>, tool: Tool, any: bool) {
    f.buttons(vec![
        tf::Button::run("Remove Last Point", move |app| {
            app.path_slice_remove_last(tool)
        })
        .enabled_if(any, "No points to remove"),
    ]);
}

/// The Slice cutter's options (PS spec section 2), shared by the three
/// path tools. `key` prefixes the field keys.
fn cutter_fields(
    f: &mut ToolForm<Parameters>,
    key: &str,
    o: &PathCutter,
    get: fn(&Parameters) -> &PathCutter,
    get_mut: fn(&mut Parameters) -> &mut PathCutter,
) {
    let c: Cutter = (get, get_mut);
    let operations: Vec<(SliceOperation, &str)> = SliceOperation::ALL
        .iter()
        .map(|&op| (op, op.label()))
        .collect();
    let mut operation = pick(
        &format!("{key}-operation"),
        "Operation",
        &operations,
        move |p| c.0(p).operation,
        move |p, op| c.1(p).operation = op,
    );
    for (i, op) in SliceOperation::ALL.iter().enumerate() {
        if !op.is_built() {
            operation = operation.unbuilt_option(
                i,
                "Not built yet: Union, Intersect, Subtract and Add come with the Booleans work (wave 4)",
            );
        }
    }
    let axes: Vec<(SliceAxis, &str)> = SliceAxis::ALL.iter().map(|&a| (a, a.label())).collect();
    f.field(operation).field(pick(
        &format!("{key}-axis"),
        "Axis",
        &axes,
        move |p| c.0(p).axis,
        move |p, a| c.1(p).axis = a,
    ));
    let custom = o.axis == SliceAxis::Custom;
    let vector = tf::vector(
        &format!("{key}_vector"),
        "Vector",
        Unit::Scalar,
        move |p: &Parameters| c.0(p).vector,
        move |p, a, v| {
            c.1(p).vector[a] = v;
            Ok(())
        },
    );
    f.fields_from(
        vector
            .into_iter()
            .map(|x| x.suffix("").enabled_if(custom, "Used with Axis Custom"))
            .collect(),
    )
    .field(
        check(
            &format!("{key}_front"),
            "Front Face Only",
            move |p| c.0(p).front_face_only,
            move |p, on| c.1(p).front_face_only = on,
        )
        .tip("Cut only polygons facing the axis's positive side; off cuts every surface the path passes through"),
    )
    .field(
        tf::text(
            &format!("{key}-material"),
            "Material",
            move |p: &Parameters| c.0(p).material.clone(),
            move |p, t| c.1(p).material = t,
        )
        .tip("Stencil: polygons inside the outline get this material")
        .enabled_if(o.operation == SliceOperation::Stencil, "Set Operation to Stencil first"),
    )
    // Preview Geometry and the depths wait for Boolean (UIC-117).
    .unbuilt(
        "Negative Depth",
        workspace::number_text(o.negative_depth, " m"),
        BOOLEAN_LATER,
    )
    .unbuilt(
        "Positive Depth",
        workspace::number_text(o.positive_depth, " m"),
        BOOLEAN_LATER,
    )
    .field(
        check(
            &format!("{key}_select_edges"),
            "Select Sliced Edges",
            move |p| c.0(p).select_sliced_edges,
            move |p, on| c.1(p).select_sliced_edges = on,
        )
        .tip("Add the new cut edges to the edge selection set named below"),
    )
    .field(
        tf::text(
            &format!("{key}-edge-set"),
            "Edge Selection Set",
            move |p: &Parameters| c.0(p).edge_set.clone(),
            move |p, t| c.1(p).edge_set = t,
        )
        .enabled_if(o.select_sliced_edges, "Turn on Select Sliced Edges first"),
    );
}

fn primitive_slice_form(p: &Parameters, f: &mut ToolForm<Parameters>) {
    let o = &p.slice.primitive_slice;
    f.field(
        pick(
            "primitive-slice-type",
            "Type",
            &[
                (PrimitiveShape::Rectangle, "Rectangle"),
                (PrimitiveShape::Circle, "Circle"),
                (PrimitiveShape::Ellipse, "Ellipse"),
                (PrimitiveShape::Preset, "Preset"),
            ],
            |p| p.slice.primitive_slice.shape,
            |p, s| p.slice.primitive_slice.shape = s,
        )
        .unbuilt_option(3, "Not built yet: profile presets need the preset browser"),
    );
    if o.place.is_some() {
        f.fields_from(tf::vector(
            "primitive_slice_center",
            "Center",
            Unit::Distance,
            |p: &Parameters| {
                p.slice
                    .primitive_slice
                    .place
                    .map(|place| place.center)
                    .unwrap_or_default()
            },
            |p, a, v| {
                let place = p
                    .slice
                    .primitive_slice
                    .place
                    .as_mut()
                    .ok_or("Click a face to place the shape first")?;
                place.center[a] = v;
                Ok(())
            },
        ));
    }
    let size =
        |key: &str, label: &str, get: fn(&Parameters) -> f64, set: fn(&mut Parameters, f64)| {
            slice_num(key, label, Unit::Distance, get, move |p, v| {
                if v <= 0. {
                    return Err("The size must be more than 0".into());
                }
                set(p, v);
                Ok(())
            })
        };
    let sides = || {
        slice_num(
            "primitive_slice_sides",
            "Sides",
            Unit::Count,
            |p| p.slice.primitive_slice.sides as f64,
            |p, v| {
                p.slice.primitive_slice.sides = whole(v, 3., 1024., "Sides")? as u32;
                Ok(())
            },
        )
    };
    match o.shape {
        PrimitiveShape::Circle => {
            f.field(size(
                "primitive_slice_radius",
                "Radius",
                |p| p.slice.primitive_slice.radius,
                |p, v| p.slice.primitive_slice.radius = v,
            ))
            .field(sides());
        }
        _ => {
            f.field(size(
                "primitive_slice_half_x",
                "Half Size X",
                |p| p.slice.primitive_slice.half_x,
                |p, v| p.slice.primitive_slice.half_x = v,
            ))
            .field(size(
                "primitive_slice_half_y",
                "Half Size Y",
                |p| p.slice.primitive_slice.half_y,
                |p, v| p.slice.primitive_slice.half_y = v,
            ));
            if o.shape == PrimitiveShape::Ellipse {
                f.field(sides());
            } else {
                f.field(slice_num(
                    "primitive_slice_corner_radius",
                    "Corner Radius",
                    Unit::Distance,
                    |p| p.slice.primitive_slice.corner_radius,
                    |p, v| {
                        if v < 0. {
                            return Err("Corner Radius must be zero or more".into());
                        }
                        p.slice.primitive_slice.corner_radius = v;
                        Ok(())
                    },
                ))
                .field(
                    slice_num(
                        "primitive_slice_corner_segments",
                        "Corner Segments",
                        Unit::Count,
                        |p| p.slice.primitive_slice.corner_segments as f64,
                        |p, v| {
                            p.slice.primitive_slice.corner_segments =
                                whole(v, 1., 256., "Corner Segments")? as u32;
                            Ok(())
                        },
                    )
                    .enabled_if(o.corner_radius > 0., "Set a Corner Radius above 0 first"),
                )
                .unbuilt(
                    "Exponent",
                    workspace::number_text(o.exponent, ""),
                    "Not built yet: how Exponent shapes the rectangle isn't specified",
                );
            }
        }
    }
    f.field(
        check(
            "primitive_slice_from_center",
            "Edit From Center",
            |p| p.slice.primitive_slice.edit_from_center,
            |p, on| p.slice.primitive_slice.edit_from_center = on,
        )
        .tip("The click is the shape's centre instead of a corner (Ctrl for one drag)"),
    )
    .unbuilt(
        "Show Rotate Handles",
        if o.show_rotate_handles { "on" } else { "off" },
        "Not built yet: use Heading, Pitch and Bank below",
    );
    for (key, label, get, set) in [
        (
            "primitive_slice_heading",
            "Heading",
            (|p: &Parameters| p.slice.primitive_slice.heading) as fn(&Parameters) -> f64,
            (|p: &mut Parameters, v| p.slice.primitive_slice.heading = v)
                as fn(&mut Parameters, f64),
        ),
        (
            "primitive_slice_pitch",
            "Pitch",
            |p| p.slice.primitive_slice.pitch,
            |p, v| p.slice.primitive_slice.pitch = v,
        ),
        (
            "primitive_slice_bank",
            "Bank",
            |p| p.slice.primitive_slice.bank,
            |p, v| p.slice.primitive_slice.bank = v,
        ),
    ] {
        f.field(slice_num(key, label, Unit::Angle, get, move |p, v| {
            set(p, v);
            Ok(())
        }));
    }
    f.field(
        check(
            "primitive_slice_keep_aspect",
            "Keep Aspect",
            |p| p.slice.primitive_slice.keep_aspect,
            |p, on| p.slice.primitive_slice.keep_aspect = on,
        )
        .tip("Keep the width-to-height ratio while dragging the size"),
    )
    .field(check(
        "primitive_slice_flip_x",
        "Flip X",
        |p| p.slice.primitive_slice.flip_x,
        |p, on| p.slice.primitive_slice.flip_x = on,
    ))
    .field(check(
        "primitive_slice_flip_y",
        "Flip Y",
        |p| p.slice.primitive_slice.flip_y,
        |p, on| p.slice.primitive_slice.flip_y = on,
    ))
    .separator()
    .field(slice_num(
        "primitive_slice_clone_x",
        "Clone X",
        Unit::Count,
        |p| p.slice.primitive_slice.clone_x as f64,
        |p, v| {
            p.slice.primitive_slice.clone_x = whole(v, 1., 64., "Clone X")? as u32;
            Ok(())
        },
    ))
    .field(slice_num(
        "primitive_slice_clone_y",
        "Clone Y",
        Unit::Count,
        |p| p.slice.primitive_slice.clone_y as f64,
        |p, v| {
            p.slice.primitive_slice.clone_y = whole(v, 1., 64., "Clone Y")? as u32;
            Ok(())
        },
    ));
    for (key, label, x) in [
        ("primitive_slice_gap_x", "Clone Gap X", true),
        ("primitive_slice_gap_y", "Clone Gap Y", false),
    ] {
        f.field(slice_num(
            key,
            label,
            Unit::Distance,
            move |p| {
                let o = &p.slice.primitive_slice;
                if x { o.gap_x } else { o.gap_y }
            },
            move |p, v| {
                if v < 0. {
                    return Err("Clone Gap must be zero or more".into());
                }
                let o = &mut p.slice.primitive_slice;
                if x {
                    o.gap_x = v;
                } else {
                    o.gap_y = v;
                }
                Ok(())
            },
        ));
    }
    f.field(
        check(
            "primitive_slice_center_clones",
            "Center",
            |p| p.slice.primitive_slice.center_clones,
            |p, on| p.slice.primitive_slice.center_clones = on,
        )
        .tip("Centre the group of clones on the shape's place"),
    )
    .field(
        check(
            "primitive_slice_lazy",
            "Lazy Apply",
            |p| p.slice.primitive_slice.lazy_apply,
            |p, on| p.slice.primitive_slice.lazy_apply = on,
        )
        .tip("While dragging, show only the shape; cut when the mouse is released"),
    )
    .unbuilt(
        "Extend Open Edge",
        "0 m",
        "Used by open preset profiles, which aren't built yet",
    )
    .separator();
    cutter_fields(
        f,
        "primitive_slice",
        &o.cutter,
        |p| &p.slice.primitive_slice.cutter,
        |p| &mut p.slice.primitive_slice.cutter,
    );
    f.note(if o.place.is_none() {
        "Click a face and drag to size the shape (Ctrl or Edit From Center: from its centre)."
    } else {
        "Drag on a face to place and size the shape again; click another face to move it there. Drop (Q) keeps the cut."
    });
}

fn axis_slice_form(p: &Parameters, f: &mut ToolForm<Parameters>) {
    let o = &p.slice.axis_slice;
    f.field(pick(
        "axis-slice-mode",
        "Slice Mode",
        &[
            (AxisSliceMode::Number, "Number"),
            (AxisSliceMode::Size, "Size"),
        ],
        |p| p.slice.axis_slice.mode,
        |p, m| p.slice.axis_slice.mode = m,
    ));
    let number_mode = o.mode == AxisSliceMode::Number;
    for (axis, label) in ["Number X", "Number Y", "Number Z"].into_iter().enumerate() {
        f.field(
            slice_num(
                &format!("axis_slice_number{axis}"),
                label,
                Unit::Count,
                move |p| p.slice.axis_slice.number[axis] as f64,
                move |p, v| {
                    p.slice.axis_slice.number[axis] = whole(v, 0., 1000., "Number")? as u32;
                    Ok(())
                },
            )
            .enabled_if(number_mode, "Set Mode to Number first"),
        );
    }
    let size = tf::vector(
        "axis_slice_size",
        "Size",
        Unit::Distance,
        |p: &Parameters| p.slice.axis_slice.size,
        |p, a, v| {
            if v <= 0. {
                return Err(crate::live_fields::refusal(
                    "Size",
                    "Size must be more than 0",
                ));
            }
            p.slice.axis_slice.size[a] = v;
            Ok(())
        },
    );
    f.fields_from(
        size.into_iter()
            .map(|x| x.enabled_if(!number_mode, "Not used when Mode is Number"))
            .collect(),
    );
    vec3(
        f,
        "axis_slice_offset",
        "Offset",
        Unit::Distance,
        " m",
        |p| p.slice.axis_slice.offset,
        |p, a, v| p.slice.axis_slice.offset[a] = v,
    );
    f.field(
        check(
            "axis_slice_hold",
            "Hold Slice",
            |p| p.slice.axis_slice.hold,
            |p, on| p.slice.axis_slice.hold = on,
        )
        .tip("Pause cutting while you adjust the values; the planes still show"),
    )
    .note("Numbers start at 0, so nothing is cut yet: raise Number X, Y or Z (click or drag its arrows). Works on the selected polygons (all when none are selected). Drag in the view to slide the cuts (Offset).");
}

pub(super) fn whole(
    value: f64,
    low: f64,
    high: f64,
    what: &str,
) -> std::result::Result<f64, String> {
    if value.fract() != 0. || !(low..=high).contains(&value) {
        return Err(format!(
            "{what} must be a whole number from {low} to {high}"
        ));
    }
    Ok(value)
}
