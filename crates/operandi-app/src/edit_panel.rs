//! The Edit tab is a form (`assets/forms/toolbox.edit.json`, layouts
//! stage 3 batch 6): the Slide button (SUB-51), the Slice and Axis Slice
//! sections (PS-02, AX-01), the Center row, the Subdivide group and
//! Freeze... (edit wave 1; behaviour in `subdivide_ui.rs`), Booleans &
//! Drill, Remesh and the UV operations.
//!
//! Here: the Slice tools' Tool Properties (`slice_fields`); their viewport
//! gestures and drawing are in `slice_view.rs`.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::{LoopSliceEdit, LoopSliceMode, OffsetSide, SliceShow};

/// Why a Loop Slice option is greyed out (spec loop-slice-2026-11-15,
/// greying table in section 4).
const PICK_PROFILE: &str = "Pick a profile first";
const USED_WITH_SPLIT: &str = "Used with Split on";
const PROFILE_SETS_LOOPS: &str = "The profile sets the loops; pick None to place slices";

/// The Profile list: none, then the shared Profile layer's shapes.
pub(crate) fn profile_options() -> Vec<(Option<operandi_core::ProfileShape>, &'static str)> {
    let mut options = vec![(None, "None")];
    options.extend(
        operandi_core::ProfileShape::ALL
            .into_iter()
            .map(|s| (Some(s), s.label())),
    );
    options
}

impl Operandi {
    /// Tool Properties of a Slice tool.
    pub(super) fn slice_form(&self, tool: Tool, p: &Parameters, f: &mut ToolForm<Parameters>) {
        match tool {
            Tool::LoopSlice => loop_slice_form(p, f),
            Tool::EdgeSlice => edge_slice_form(p, f),
            Tool::PolygonSlice => polygon_slice_form(p, f, self.slice_workplane_normal()),
            _ => self.path_slice_form(tool, p, f),
        }
    }
}

/// A slice number whose refusal names the field.
pub(super) fn slice_num(
    key: &str,
    label: &str,
    unit: Unit,
    get: impl Fn(&Parameters) -> f64 + 'static,
    set: impl Fn(&mut Parameters, f64) -> std::result::Result<(), String> + 'static,
) -> tf::Field<Parameters> {
    let name = label.to_owned();
    tf::number(key, label, unit, get, move |p, v| {
        set(p, v).map_err(|e| crate::live_fields::refusal(&name, e))
    })
}

fn loop_slice_form(p: &Parameters, f: &mut ToolForm<Parameters>) {
    use crate::live_fields::{check, pick};
    use path_slice_panel::whole;
    let o = &p.slice.loop_slice;
    let profile = o.profile.active();
    f.field(pick(
        "loop-slice-edit",
        "Edit",
        &[
            (LoopSliceEdit::Move, "Move"),
            (LoopSliceEdit::Add, "Add"),
            (LoopSliceEdit::Remove, "Remove"),
        ],
        |p| p.slice.loop_slice.edit,
        |p, e| p.slice.loop_slice.edit = e,
    ))
    .field(pick(
        "loop-slice-mode",
        "Mode",
        &[
            (LoopSliceMode::Free, "Free"),
            (LoopSliceMode::Uniform, "Uniform"),
            (LoopSliceMode::Symmetry, "Symmetry"),
        ],
        |p| p.slice.loop_slice.mode,
        |p, m| {
            let o = &mut p.slice.loop_slice;
            if o.mode != m {
                o.mode = m;
                let current = o.current;
                o.normalize(current);
            }
        },
    ))
    .field(
        slice_num(
            "loop_slice_current",
            "Current",
            Unit::Count,
            |p| p.slice.loop_slice.current as f64,
            |p, v| {
                let o = &mut p.slice.loop_slice;
                // A drag or step past the last slice stops there instead
                // of refusing (with one slice every nudge showed "Current
                // must be a whole number from 0 to 0", Jason 2026-11-20).
                let last = (o.count() - 1) as f64;
                o.current = whole(v.round().clamp(0., last), 0., last, "Current")? as usize;
                Ok(())
            },
        )
        // LS-Q13 (Jason, 2026-11-16): with a profile the profile sets the
        // loops, so Current, Count, Uniform and Position grey like Inset.
        .enabled_if(!profile, PROFILE_SETS_LOOPS),
    )
    .field(
        slice_num(
            "loop_slice_count",
            "Count",
            Unit::Count,
            |p| p.slice.loop_slice.count() as f64,
            |p, v| {
                p.slice
                    .loop_slice
                    .set_count(whole(v, 1., 100., "Count")? as usize);
                Ok(())
            },
        )
        .enabled_if(!profile, PROFILE_SETS_LOOPS),
    )
    .buttons(vec![
        tf::Button::change("Uniform", |p: &mut Parameters| p.slice.loop_slice.respace())
            .tip("Space the slices evenly across the ring")
            .enabled_if(!profile, PROFILE_SETS_LOOPS),
    ])
    .field(
        slice_num(
            "loop_slice_position",
            "Position",
            Unit::Percent,
            |p| {
                let o = &p.slice.loop_slice;
                o.positions[o.current.min(o.count() - 1)] * 100.
            },
            |p, v| {
                let o = &mut p.slice.loop_slice;
                let index = o.current;
                o.set_position(index, v / 100.);
                Ok(())
            },
        )
        .suffix(" %")
        .enabled_if(!profile, PROFILE_SETS_LOOPS),
    );
    for (key, label, lo, hi, get, set) in [
        (
            "loop_slice_length",
            "Length",
            40.,
            2000.,
            (|p: &Parameters| p.slice.loop_slice.length) as fn(&Parameters) -> f64,
            (|p: &mut Parameters, v| p.slice.loop_slice.length = v) as fn(&mut Parameters, f64),
        ),
        (
            "loop_slice_x",
            "Slider X",
            0.,
            4000.,
            |p| p.slice.loop_slice.slider_x,
            |p, v| p.slice.loop_slice.slider_x = v,
        ),
        (
            "loop_slice_y",
            "Slider Y",
            0.,
            4000.,
            |p| p.slice.loop_slice.slider_y,
            |p, v| p.slice.loop_slice.slider_y = v,
        ),
    ] {
        f.field(slice_num(
            key,
            label,
            Unit::Suffix(" px"),
            get,
            move |p, v| {
                set(p, v.clamp(lo, hi));
                Ok(())
            },
        ));
    }
    f.separator()
        .field(
            check(
                "loop_slice_selected",
                "Slice Selected",
                |p| p.slice.loop_slice.slice_selected,
                |p, on| p.slice.loop_slice.slice_selected = on,
            )
            .tip("Cut only within the selected polygons instead of the whole ring"),
        )
        .field(
            // Spec table "see LS-Q08"; the brief and the measured run (7.)
            // tie it to Slice Selected: greyed in place without it (UIC-92).
            check(
                "loop_slice_keep_quads",
                "Keep Quads",
                |p| p.slice.loop_slice.keep_quads,
                |p, on| p.slice.loop_slice.keep_quads = on,
            )
            .tip("Add edges so the selected polygons the slice ends in stay quads")
            .enabled_if(o.slice_selected, "Turn on Slice Selected first"),
        )
        .field(
            check(
                "loop_slice_ngon",
                "Slice N-gon",
                |p| p.slice.loop_slice.slice_ngon,
                |p, on| p.slice.loop_slice.slice_ngon = on,
            )
            .tip("Continue through polygons with more than four sides (even-sided ones; SL-Q03)"),
        )
        // LS-42/43, spec greying table: these three need a profile
        // (greyed in place with the reason, UIC-92/UIC-117).
        .field(
            check(
                "loop_slice_reverse_direction",
                "Reverse Direction",
                |p| p.slice.loop_slice.profile.reverse_direction,
                |p, on| p.slice.loop_slice.profile.reverse_direction = on,
            )
            .tip("Read the profile from the other side of the ring")
            .enabled_if(profile, PICK_PROFILE),
        )
        .field(
            check(
                "loop_slice_reverse_inset",
                "Reverse Inset",
                |p| p.slice.loop_slice.profile.reverse_inset,
                |p, on| p.slice.loop_slice.profile.reverse_inset = on,
            )
            .tip("Flip the profile in and out: a groove becomes a ridge")
            .enabled_if(profile, PICK_PROFILE),
        )
        .field(
            check(
                "loop_slice_keep_aspect",
                "Keep Aspect",
                |p| p.slice.loop_slice.profile.keep_aspect,
                |p, on| p.slice.loop_slice.profile.keep_aspect = on,
            )
            .tip("Set the depth from the profile's own shape, scaled to the ring's width; off, Inset sets it")
            .enabled_if(profile, PICK_PROFILE),
        )
        .field(
            check(
                "loop_slice_curvature",
                "Preserve Curvature",
                |p| p.slice.loop_slice.preserve_curvature,
                |p, on| p.slice.loop_slice.preserve_curvature = on,
            )
            .tip("New points follow a curve through the surface instead of the straight edge"),
        )
        .field(
            slice_num(
                "loop_slice_tension",
                "Tension",
                Unit::Percent,
                |p| p.slice.loop_slice.tension * 100.,
                |p, v| {
                    p.slice.loop_slice.tension = (v / 100.).clamp(-10., 10.);
                    Ok(())
                },
            )
            .suffix(" %")
            .enabled_if(o.preserve_curvature, "Turn on Preserve Curvature first"),
        )
        .field(
            // LS-41: live with no profile (provisional, LS-Q10: every slice
            // is pushed the same distance along the surface normal) and
            // with a profile when Keep Aspect is off (the profile's deepest
            // point). Greyed while Keep Aspect sets it (spec table).
            slice_num(
                "loop_slice_inset",
                "Inset",
                Unit::Distance,
                |p| p.slice.loop_slice.inset,
                |p, v| {
                    if !v.is_finite() {
                        return Err("must be a number".into());
                    }
                    p.slice.loop_slice.inset = v;
                    Ok(())
                },
            )
            .tip("Push the cut into the surface (a groove); negative pushes it out. Middle-drag up/down in the viewport changes it too")
            .enabled_if(
                o.profile.uses_inset(),
                "Keep Aspect sets the depth from the profile; turn it off to set Inset",
            ),
        )
        .field(
            check(
                "loop_slice_select_new",
                "Select New Polygon",
                |p| p.slice.loop_slice.select_new,
                |p, on| p.slice.loop_slice.select_new = on,
            )
            .tip("Select what the slice made: the new edges in Edge mode, the cut polygons in Polygon mode"),
        )
        .separator()
        .field(
            check(
                "loop_slice_split",
                "Split",
                |p| p.slice.loop_slice.split,
                |p, on| p.slice.loop_slice.split = on,
            )
            .tip("Leave two separate edge loops at each slice: the mesh comes apart there"),
        )
        .field(
            check(
                "loop_slice_cap",
                "Cap Sections",
                |p| p.slice.loop_slice.cap_sections,
                |p, on| p.slice.loop_slice.cap_sections = on,
            )
            .tip("Close each side of a slice that runs all the way round with one polygon")
            .enabled_if(o.split, USED_WITH_SPLIT),
        )
        .field(
            slice_num(
                "loop_slice_gap",
                "Gap",
                Unit::Distance,
                |p| p.slice.loop_slice.gap,
                |p, v| {
                    if !v.is_finite() || v < 0. {
                        return Err("Gap must be zero or more".into());
                    }
                    p.slice.loop_slice.gap = v;
                    Ok(())
                },
            )
            .tip("Open a gap this wide around each slice, centred on it")
            .enabled_if(o.split, USED_WITH_SPLIT),
        )
        .field(
            // LS-40/LS-44 and DRAG-LAYER section 3: Operandi's own profile
            // shapes from the shared Profile layer. Never greyed.
            pick(
                "loop-slice-profile",
                "Profile",
                &profile_options(),
                |p| p.slice.loop_slice.profile.shape,
                |p, shape| p.slice.loop_slice.profile.shape = shape,
            )
            .tip("Cut the ring with a profile shape (a router cut): the profile sets how many loops there are and how deep each goes"),
        )
        .note(if o.rings.is_empty() {
            "No ring yet: click an edge (Shift adds another ring), or select an edge or two neighbouring polygons before starting."
        } else {
            "Drag the triangles on the bar at the top of the view; drag its end dots to resize it. Click an edge for another ring (Shift adds)."
        });
}

fn edge_slice_form(p: &Parameters, f: &mut ToolForm<Parameters>) {
    use crate::live_fields::{check, pick};
    let o = &p.slice.edge_slice;
    f.field(
        check(
            "edge_slice_middle",
            "Split at Middle",
            |p| p.slice.edge_slice.split_middle,
            |p, on| p.slice.edge_slice.split_middle = on,
        )
        .tip("Every cut lands at the edge's middle (Ctrl does it for one click)"),
    )
    .field(
        check(
            "edge_slice_split",
            "Split Polygons",
            |p| p.slice.edge_slice.split_polygons,
            |p, on| p.slice.edge_slice.split_polygons = on,
        )
        .tip("Connect the cut points with new edges; off only adds points"),
    )
    .field(pick(
        "edge-slice-show",
        "Show",
        &[
            (SliceShow::None, "None"),
            (SliceShow::Position, "Position"),
            (SliceShow::Distance, "Distance"),
        ],
        |p| p.slice.edge_slice.show,
        |p, s| p.slice.edge_slice.show = s,
    ))
    .unbuilt(
        "Offset",
        "0 m",
        "Not built yet: how Offset moves cuts inside a polygon isn't specified (SL-Q08)",
    )
    .field(
        slice_num(
            "edge_slice_snap",
            "Snap Value",
            Unit::Percent,
            |p| p.slice.edge_slice.snap * 100.,
            |p, v| {
                if !(0. ..=50.).contains(&v) {
                    return Err("Snap Value must be from 0% to 50%".into());
                }
                p.slice.edge_slice.snap = v / 100.;
                Ok(())
            },
        )
        .suffix(" %"),
    )
    .field(
        check(
            "edge_slice_inside",
            "Inside Polygons",
            |p| p.slice.edge_slice.inside,
            |p, on| p.slice.edge_slice.inside = on,
        )
        .tip("Click inside a polygon to cut into its interior"),
    )
    .field(
        check(
            "edge_slice_raycast",
            "Always Raycast",
            |p| p.slice.edge_slice.always_raycast,
            |p, on| p.slice.edge_slice.always_raycast = on,
        )
        .tip("Pick only front-facing polygons, also in wireframe views"),
    )
    .field(
        check(
            "edge_slice_multi",
            "Multi Slice",
            |p| p.slice.edge_slice.multi_slice,
            |p, on| p.slice.edge_slice.multi_slice = on,
        )
        .tip("Draw a dashed line from the last cut to the pointer"),
    )
    .note(if o.points.is_empty() {
        "Click an edge to start; click from edge to edge. Drag along an edge to place a point; drag a point to move it."
    } else if o.closed {
        "Closed. Drag a point to move it; Drop (Q) keeps the cut."
    } else {
        "Click the next edge. Click the first point again to close the loop."
    });
}

/// The fields edit the current (last) cut; before the first one they show
/// the starting line, and typing a value makes that cut on the work plane.
fn polygon_slice_form(p: &Parameters, f: &mut ToolForm<Parameters>, up: DVec3) {
    use crate::live_fields::{check, pick, vec3};
    let o = &p.slice.polygon_slice;
    for (key, label, end) in [
        ("polygon_slice_start", "Start", false),
        ("polygon_slice_end", "End", true),
    ] {
        vec3(
            f,
            key,
            label,
            Unit::Distance,
            " m",
            move |p| {
                let o = &p.slice.polygon_slice;
                let (start, stop) = o.cuts.last().map_or((o.start, o.end), |c| (c.start, c.end));
                if end { stop } else { start }
            },
            move |p, a, v| {
                let o = &mut p.slice.polygon_slice;
                if o.cuts.is_empty() {
                    o.cuts.push(operandi_core::PlaneCut {
                        start: o.start,
                        end: o.end,
                        up,
                    });
                }
                let cut = o.cuts.last_mut().unwrap();
                if end {
                    cut.end[a] = v;
                } else {
                    cut.start[a] = v;
                }
            },
        );
    }
    f.separator()
        .field(
            check(
                "polygon_slice_fast",
                "Fast Slice",
                |p| p.slice.polygon_slice.fast_slice,
                |p, on| p.slice.polygon_slice.fast_slice = on,
            )
            .tip("Cut only when the mouse is released (for dense meshes)"),
        )
        .field(
            check(
                "polygon_slice_angle_snap",
                "Angle Snap",
                |p| p.slice.polygon_slice.angle_snap,
                |p, on| p.slice.polygon_slice.angle_snap = on,
            )
            .tip("With snapping on (or Ctrl held), the line snaps to multiples of Angle"),
        )
        .field(
            slice_num(
                "polygon_slice_angle",
                "Angle",
                Unit::Angle,
                |p| p.slice.polygon_slice.angle,
                |p, v| {
                    if v <= 0. {
                        return Err("Angle must be more than 0°".into());
                    }
                    p.slice.polygon_slice.angle = v;
                    Ok(())
                },
            )
            .enabled_if(o.angle_snap, "Turn on Angle Snap first"),
        )
        .field(
            check(
                "polygon_slice_split",
                "Split",
                |p| p.slice.polygon_slice.split,
                |p, on| p.slice.polygon_slice.split = on,
            )
            .tip("Separate the two sides into disconnected pieces"),
        );
    let split = "Used with Split on";
    f.field(
        check(
            "polygon_slice_cap",
            "Cap Sections",
            |p| p.slice.polygon_slice.cap_sections,
            |p, on| p.slice.polygon_slice.cap_sections = on,
        )
        .tip("Close every enclosed cut with a polygon on each side")
        .enabled_if(o.split, split),
    )
    .field(
        slice_num(
            "polygon_slice_gap",
            "Gap",
            Unit::Distance,
            |p| p.slice.polygon_slice.gap,
            |p, v| {
                if v < 0. {
                    return Err("Gap must be zero or more".into());
                }
                p.slice.polygon_slice.gap = v;
                Ok(())
            },
        )
        .enabled_if(o.split, split),
    )
    .field(
        pick(
            "polygon-slice-offset-side",
            "Offset Side",
            &[
                (OffsetSide::Center, "Center"),
                (OffsetSide::Above, "Above"),
                (OffsetSide::Below, "Below"),
            ],
            |p| p.slice.polygon_slice.offset_side,
            |p, s| p.slice.polygon_slice.offset_side = s,
        )
        .enabled_if(o.split, split),
    )
    .field(
        check(
            "polygon_slice_infinite",
            "Infinite",
            |p| p.slice.polygon_slice.infinite,
            |p, on| p.slice.polygon_slice.infinite = on,
        )
        .tip("On: the plane cuts everything it meets. Off: only polygons along the drawn line"),
    )
    .note("Drag a line to cut. Drag its ends or the line to move it; Shift+drag starts another cut; middle-click repeats it there.");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Slice tools, Inset and Shift each have their own artwork.
    #[test]
    fn slice_inset_and_shift_icons_are_their_own() {
        let tools = [
            Tool::LoopSlice,
            Tool::EdgeSlice,
            Tool::PolygonSlice,
            Tool::PenSlice,
            Tool::CurveSlice,
            Tool::PrimitiveSlice,
            Tool::AxisSlice,
            Tool::Inset,
            Tool::Shift,
        ];
        for tool in tools {
            let icon = tool_info::info(tool).icon;
            let users: Vec<_> = tool_info::TOOLS.iter().filter(|t| t.icon == icon).collect();
            assert_eq!(users.len(), 1, "{tool:?} shares {icon:?}");
            assert_ne!(icon, icons::Icon::ScriptAction);
        }
    }
}
