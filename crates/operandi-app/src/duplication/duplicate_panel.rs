//! The Duplicate tools' Tool Properties (specifications/duplicate-tools);
//! the Duplicate tab (DS-60) is a form since layouts stage 3 batch 3,
//! `assets/forms/toolbox.duplicate.json`. Each tool's placer section
//! then its copier section (DS-03), as field lists for the shared form
//! (UIC-110..117). Viewport gestures and handles are in
//! `duplicate_view.rs`; geometry in `operandi_core::duplicate`.
use super::*;
use crate::form::Unit;
use crate::live_fields::{check, pick};
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::{Circle, Copies, MirrorMode, UvsAround};

type Form = ToolForm<Parameters>;

impl Operandi {
    /// Tool Properties of the Duplicate tools.
    pub(super) fn duplicate_form(&self, tool: Tool, p: &Parameters, f: &mut Form) {
        let item_mode = self.editor.scene.selection.mode == Mode::Item;
        let d = &p.duplicate;
        match tool {
            Tool::Mirror => {
                f.section("Mirror Generator", false, mirror_rows(p));
                f.section("Mirror Generator Snap", false, mirror_snap_rows());
                f.section(
                    "Clone Effector",
                    false,
                    copies_rows(
                        tool,
                        &d.mirror.copies,
                        item_mode,
                        operandi_core::duplicate_items::instancing(d),
                    ),
                );
            }
            Tool::Clone => {
                f.section("Linear Generator", false, clone_rows());
                f.section(
                    "Clone Effector",
                    false,
                    copies_rows(
                        tool,
                        &d.clone.copies,
                        item_mode,
                        operandi_core::duplicate_items::instancing(d),
                    ),
                );
            }
            Tool::Array => {
                f.section("Array Generator", false, array_rows());
                f.section(
                    "Clone Effector",
                    false,
                    copies_rows(
                        tool,
                        &d.array.copies,
                        item_mode,
                        operandi_core::duplicate_items::instancing(d),
                    ),
                );
            }
            Tool::RadialArray => {
                f.section("Helix Generator", false, circle_rows(tool));
                f.section(
                    "Clone Effector",
                    false,
                    copies_rows(
                        tool,
                        &d.radial.copies,
                        item_mode,
                        operandi_core::duplicate_items::instancing(d),
                    ),
                );
            }
            Tool::Lathe => {
                f.section("Helix Generator", false, circle_rows(tool));
                f.section("Sweep Effector", false, sweep_rows(&d.lathe.circle));
            }
            Tool::ScatterClone => {
                f.section("Scatter Generator", false, scatter_rows());
                f.section(
                    "Clone Effector",
                    false,
                    copies_rows(
                        tool,
                        &d.scatter.copies,
                        item_mode,
                        operandi_core::duplicate_items::instancing(d),
                    ),
                );
            }
            Tool::Symmetrize => return self.symmetrize_form(p, f),
            _ => return,
        }
        if d.copies(tool).is_some() && operandi_core::duplicate_items::replicating(d) {
            f.section("Instancer", false, crate::replica_rows::replica_rows());
        } else if d.copies(tool).is_some() {
            let (get, get_mut) = copies_lens(tool);
            let instancing = operandi_core::duplicate_items::instancing(d);
            let rows = crate::duplicate_item_rows::item_rows(get, get_mut, instancing, item_mode);
            f.section("Item", false, rows);
        }
        // DS-31: Apply makes the copies without a click in the view.
        f.buttons(vec![
            tf::Button::change("Apply", |p: &mut Parameters| p.duplicate.placed = true)
                .tip("Make the copies with these values (or click in the view)")
                .enabled_if(
                    !d.placed,
                    "The copies are made; values now update them live",
                ),
        ]);
        let live = self.editor.session.live.as_ref();
        if let Some(error) = live.and_then(|l| l.preview_error.as_ref()) {
            f.error(error.clone());
        } else if operandi_core::duplicate::nothing_to_copy(&self.editor.scene) {
            // DS-50.
            f.note(operandi_core::duplicate::NOTHING_TO_COPY);
        }
        f.note(duplicate_hint(tool, d.placed));
    }
}

/// What to do next, shown under the rows.
fn duplicate_hint(tool: Tool, placed: bool) -> &'static str {
    match (tool, placed) {
        (Tool::Mirror, false) => "Click in the view (or Apply) to mirror across the plane.",
        (Tool::Mirror, true) => {
            "Drag the centre grip to slide the plane, the edge cube to turn it about its hinge. Drop (Space) keeps it."
        }
        (Tool::Lathe | Tool::RadialArray, false) => {
            "Click in the view to put the axis there (or Apply to use Center)."
        }
        (Tool::Lathe | Tool::RadialArray, true) => {
            "Drag the arc's end cubes for the angles, an axis cube to tilt it, the axis to move it. Right-drag changes Count (Radial Array). Click away to move the axis there."
        }
        (Tool::ScatterClone, false) => "Click in the view (or Apply) to scatter the copies.",
        (Tool::ScatterClone, true) => {
            "Change Count, Range or Seed: the copies follow. Drop (Space) keeps them."
        }
        (Tool::Clone, false) => "Press and drag in the view: the first copy follows the pointer.",
        (Tool::Clone, true) => {
            "Drag the arrow's grip to change the step; right-drag left/right for more or fewer copies. Click away to clone again."
        }
        (Tool::Array, false) => "Click in the view: the copies sit side by side.",
        (Tool::Array, true) => {
            "Drag to change the spacing, right-drag to add or remove copies, or drag an axis grip."
        }
        _ => "",
    }
}

fn to_items(f: Form) -> Vec<tf::Item<Parameters>> {
    f.items
}

/// A number written straight into the parameters, refused with a message.
fn num(
    key: &str,
    label: &str,
    unit: Unit,
    get: impl Fn(&Parameters) -> f64 + 'static,
    set: impl Fn(&mut Parameters, f64) -> std::result::Result<(), String> + 'static,
) -> tf::Field<Parameters> {
    crate::edit_panel::slice_num(key, label, unit, get, set)
}

fn finite(v: f64) -> std::result::Result<f64, String> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err("must be a finite number".into())
    }
}

#[allow(clippy::too_many_arguments)]
fn vector(
    f: &mut Form,
    key: &str,
    label: &str,
    unit: Unit,
    get: impl Fn(&Parameters) -> DVec3 + 'static,
    set: impl Fn(&mut Parameters, usize, f64) + 'static,
    enabled: bool,
    why: &str,
) {
    let why = why.to_owned();
    let fields = tf::vector(key, label, unit, get, move |p, a, v| {
        set(p, a, finite(v)?);
        Ok(())
    });
    f.fields_from(
        fields
            .into_iter()
            .map(|x| x.enabled_if(enabled, why.clone()))
            .collect(),
    );
}

fn mirror_rows(p: &Parameters) -> Vec<tf::Item<Parameters>> {
    let m = &p.duplicate.mirror;
    let free = m.mode == MirrorMode::FreeRotation;
    let mut f = Form::new();
    f.field(
        tf::axis(
            |p: &Parameters| p.duplicate.mirror.axis,
            |p, a| {
                let m = &mut p.duplicate.mirror;
                m.axis = a;
                // MR-22: Left and Up follow Axis.
                let (left, up) = operandi_core::Mirror::axis_frame(a);
                m.left = left;
                m.up = up;
            },
        )
        .tip("The plane's facing direction: X mirrors across the Y-Z plane"),
    )
    .field(
        num(
            "mirror_angle",
            "Angle",
            Unit::Angle,
            |p| p.duplicate.mirror.angle,
            |p, v| {
                if !(-360. ..=360.).contains(&v) {
                    return Err("Angle must be from −360° to 360°".into());
                }
                p.duplicate.mirror.angle = v;
                Ok(())
            },
        )
        .tip("Turns the mirror plane about its hinge (the Left line); 0° is a plain mirror"),
    )
    .field(pick(
        "mirror-mode",
        "Mode",
        &[
            (MirrorMode::Axis, "Axis"),
            (MirrorMode::FreeRotation, "Free Rotation"),
        ],
        |p| p.duplicate.mirror.mode,
        |p, m| p.duplicate.mirror.mode = m,
    ));
    vector(
        &mut f,
        "mirror_center",
        "Center",
        Unit::Distance,
        |p| p.duplicate.mirror.center,
        |p, a, v| p.duplicate.mirror.center[a] = v,
        true,
        "",
    );
    let why = "Set by Axis; choose Free Rotation to type it";
    vector(
        &mut f,
        "mirror_left",
        "Left",
        Unit::Scalar,
        |p| p.duplicate.mirror.left,
        |p, a, v| p.duplicate.mirror.left[a] = v,
        free,
        why,
    );
    vector(
        &mut f,
        "mirror_up",
        "Up",
        Unit::Scalar,
        |p| p.duplicate.mirror.up,
        |p, a, v| p.duplicate.mirror.up[a] = v,
        free,
        why,
    );
    to_items(f)
}

/// Mirror Generator Snap (spec v1.5: its own section).
fn mirror_snap_rows() -> Vec<tf::Item<Parameters>> {
    let mut f = Form::new();
    f.field(
        check(
            "mirror_snap_axis",
            "Snap to World Axis",
            |p| p.duplicate.mirror.snap_world_axis,
            |p, on| p.duplicate.mirror.snap_world_axis = on,
        )
        .tip("With snapping on (X), the centre grip snaps the plane onto the world origin"),
    );
    to_items(f)
}

type CopiesLens = (
    fn(&Parameters) -> &Copies,
    fn(&mut Parameters) -> &mut Copies,
);

fn copies_lens(tool: Tool) -> CopiesLens {
    match tool {
        Tool::Mirror => (
            |p| &p.duplicate.mirror.copies,
            |p| &mut p.duplicate.mirror.copies,
        ),
        Tool::Clone => (
            |p| &p.duplicate.clone.copies,
            |p| &mut p.duplicate.clone.copies,
        ),
        Tool::Array => (
            |p| &p.duplicate.array.copies,
            |p| &mut p.duplicate.array.copies,
        ),
        Tool::ScatterClone => (
            |p| &p.duplicate.scatter.copies,
            |p| &mut p.duplicate.scatter.copies,
        ),
        _ => (
            |p| &p.duplicate.radial.copies,
            |p| &mut p.duplicate.radial.copies,
        ),
    }
}

/// The Copies section (DS-20..24).
fn copies_rows(
    tool: Tool,
    c: &Copies,
    item_mode: bool,
    instancing: bool,
) -> Vec<tf::Item<Parameters>> {
    // Instance copies are items too (DX-30).
    let item_mode = item_mode || instancing;
    let (get, get_mut) = copies_lens(tool);
    let mut f = Form::new();
    f.field(
        check(
            "copies_replace",
            "Replace Source",
            move |p| get(p).replace_source,
            move |p, on| get_mut(p).replace_source = on,
        )
        .tip("Remove the original; a copy takes its place"),
    )
    .field(
        check(
            "copies_flip",
            "Invert Polygons",
            move |p| get(p).flip,
            move |p, on| get_mut(p).flip = on,
        )
        .tip("Turns the copies' normals the other way"),
    );
    let item_why = "Item copies are separate items, so nothing welds";
    f.field(
        check(
            "copies_merge",
            "Merge Vertices",
            move |p| get(p).merge,
            move |p, on| get_mut(p).merge = on,
        )
        .tip("Weld copy points within Distance of another point")
        .enabled_if(!item_mode, item_why),
    )
    .field(
        num(
            "copies_distance",
            "Distance",
            Unit::Distance,
            move |p| get(p).distance,
            move |p, v| {
                if !(v.is_finite() && v >= 0.) {
                    return Err("Distance must be 0 or more".into());
                }
                get_mut(p).distance = v;
                Ok(())
            },
        )
        .tip("How close points must be to weld; 0 welds only points on top of each other")
        .enabled_if(
            c.merge && !item_mode,
            if item_mode {
                item_why
            } else {
                "Turn on Merge Vertices to weld"
            },
        ),
    );
    to_items(f)
}

fn clone_rows() -> Vec<tf::Item<Parameters>> {
    let mut f = Form::new();
    f.field(num(
        "clone_count",
        "Number of Clones",
        Unit::Count,
        |p| p.duplicate.clone.count as f64,
        |p, v| {
            p.duplicate.clone.count =
                crate::path_slice_panel::whole(v, 1., 100_000., "Number of Clones")? as u32;
            Ok(())
        },
    ));
    vector(
        &mut f,
        "clone_offset",
        "Offset",
        Unit::Distance,
        |p| p.duplicate.clone.offset,
        |p, a, v| p.duplicate.clone.offset[a] = v,
        true,
        "",
    );
    let scale = tf::vector(
        "clone_scale",
        "Scale",
        Unit::Percent,
        |p: &Parameters| p.duplicate.clone.scale,
        |p, a, v| {
            if !(v.is_finite() && v > 0.) {
                return Err(crate::live_fields::refusal(
                    "Scale",
                    "Scale must be more than 0 %",
                ));
            }
            p.duplicate.clone.scale[a] = v;
            Ok(())
        },
    );
    f.fields_from(
        scale
            .into_iter()
            .map(|x| x.tip("Size change per copy, compounding: 50 % gives 50, 25, 12.5 %"))
            .collect(),
    );
    vector(
        &mut f,
        "clone_rotate",
        "Rotate",
        Unit::Angle,
        |p| p.duplicate.clone.rotate,
        |p, a, v| p.duplicate.clone.rotate[a] = v,
        true,
        "",
    );
    to_items(f)
}

fn array_rows() -> Vec<tf::Item<Parameters>> {
    let mut f = Form::new();
    for (axis, label) in ["Count X", "Y", "Z"].into_iter().enumerate() {
        f.field(num(
            &format!("array_count{axis}"),
            label,
            Unit::Count,
            move |p| p.duplicate.array.count[axis] as f64,
            move |p, v| {
                p.duplicate.array.count[axis] =
                    crate::path_slice_panel::whole(v, 1., 100_000., "Count")? as u32;
                Ok(())
            },
        ));
    }
    vector(
        &mut f,
        "array_offset",
        "Offset",
        Unit::Distance,
        |p| p.duplicate.array.offset,
        |p, a, v| p.duplicate.array.offset[a] = v,
        true,
        "",
    );
    vector(
        &mut f,
        "array_jitter",
        "Jitter",
        Unit::Distance,
        |p| p.duplicate.array.jitter,
        |p, a, v| p.duplicate.array.jitter[a] = v,
        true,
        "",
    );
    vector(
        &mut f,
        "array_scale",
        "Scale",
        Unit::Percent,
        |p| p.duplicate.array.scale,
        |p, a, v| p.duplicate.array.scale[a] = v,
        true,
        "",
    );
    f.field(
        check(
            "array_between",
            "Between",
            |p| p.duplicate.array.between,
            |p, on| p.duplicate.array.between = on,
        )
        .tip("Offset is the distance from the first copy to the last"),
    );
    to_items(f)
}

type CircleLens = (
    fn(&Parameters) -> &Circle,
    fn(&mut Parameters) -> &mut Circle,
);

/// The Circle section Lathe and Radial Array share (LT section 3).
fn circle_rows(tool: Tool) -> Vec<tf::Item<Parameters>> {
    let (get, get_mut): CircleLens = if tool == Tool::Lathe {
        (
            |p| &p.duplicate.lathe.circle,
            |p| &mut p.duplicate.lathe.circle,
        )
    } else {
        (
            |p| &p.duplicate.radial.circle,
            |p| &mut p.duplicate.radial.circle,
        )
    };
    let mut f = Form::new();
    f.field(
        num(
            "circle_count",
            "Count",
            Unit::Count,
            move |p| get(p).count as f64,
            move |p, v| {
                get_mut(p).count = crate::path_slice_panel::whole(v, 1., 100_000., "Count")? as u32;
                Ok(())
            },
        )
        .tip(if tool == Tool::Lathe {
            "Segments over the whole sweep"
        } else {
            "Pieces in the ring, the original included"
        }),
    )
    .field(tf::axis(
        move |p: &Parameters| get(p).axis,
        move |p, a| {
            let c = get_mut(p);
            c.axis = a;
            // LT-32: the direction follows the dropdown.
            c.direction = DVec3::from_array(std::array::from_fn(|i| f64::from(i == a)));
        },
    ));
    vector(
        &mut f,
        "circle_center",
        "Center",
        Unit::Distance,
        move |p| get(p).center,
        move |p, a, v| get_mut(p).center[a] = v,
        true,
        "",
    );
    vector(
        &mut f,
        "circle_direction",
        "Axis",
        Unit::Scalar,
        move |p| get(p).direction,
        move |p, a, v| get_mut(p).direction[a] = v,
        true,
        "",
    );
    f.field(num(
        "circle_start",
        "Start Angle",
        Unit::Angle,
        move |p| get(p).start,
        move |p, v| {
            get_mut(p).start = finite(v)?;
            Ok(())
        },
    ))
    .field(num(
        "circle_end",
        "End Angle",
        Unit::Angle,
        move |p| get(p).end,
        move |p, v| {
            get_mut(p).end = finite(v)?;
            Ok(())
        },
    ))
    .field(
        num(
            "circle_offset",
            "Offset",
            Unit::Distance,
            move |p| get(p).offset,
            move |p, v| {
                get_mut(p).offset = finite(v)?;
                Ok(())
            },
        )
        .tip(if tool == Tool::Lathe {
            "Rise along the axis per segment (springs, coils)"
        } else {
            "Rise along the axis per copy (spiral stairs)"
        }),
    );
    to_items(f)
}

/// Lathe's Sweep section.
fn sweep_rows(c: &Circle) -> Vec<tf::Item<Parameters>> {
    let full = c.whole_turns() && c.offset == 0.;
    let why = "A full turn closes on itself: there's nothing to cap";
    let mut f = Form::new();
    f.field(
        check(
            "sweep_flip",
            "Invert Polygons",
            |p| p.duplicate.lathe.sweep.flip,
            |p, on| p.duplicate.lathe.sweep.flip = on,
        )
        .tip("Turns the new polygons' normals the other way (inside of a cup)"),
    )
    .field(
        check(
            "sweep_cap_start",
            "Cap Start",
            |p| p.duplicate.lathe.sweep.cap_start,
            |p, on| p.duplicate.lathe.sweep.cap_start = on,
        )
        .enabled_if(!full, why),
    )
    .field(
        check(
            "sweep_cap_end",
            "Cap End",
            |p| p.duplicate.lathe.sweep.cap_end,
            |p, on| p.duplicate.lathe.sweep.cap_end = on,
        )
        .enabled_if(!full, why),
    )
    .field(
        pick(
            "sweep-uvs",
            "Sweep UVs",
            &[(UvsAround::U, "U"), (UvsAround::V, "V")],
            |p| p.duplicate.lathe.sweep.uvs_around,
            |p, u| p.duplicate.lathe.sweep.uvs_around = u,
        )
        .tip("Which UV direction runs round the axis"),
    );
    to_items(f)
}

/// Scatter Clone's generator (GO-02): copies at random in Centre ± Range.
fn scatter_rows() -> Vec<tf::Item<Parameters>> {
    let mut f = Form::new();
    f.field(
        num(
            "scatter_count",
            "Count",
            Unit::Count,
            |p| p.duplicate.scatter.generator.count as f64,
            |p, v| {
                // UIC-165: use the same Count limit as the generator.
                p.duplicate.scatter.generator.count = crate::path_slice_panel::whole(
                    v,
                    1.,
                    operandi_core::generator_ops::MAX_SCATTER as f64,
                    "Count",
                )? as u32;
                Ok(())
            },
        )
        .tip("Copies, the original included: 5 makes the original and 4 copies"),
    );
    vector(
        &mut f,
        "scatter_centre",
        "Center",
        Unit::Distance,
        |p| p.duplicate.scatter.generator.centre,
        |p, a, v| p.duplicate.scatter.generator.centre[a] = v,
        true,
        "",
    );
    vector(
        &mut f,
        "scatter_range",
        "Range",
        Unit::Distance,
        |p| p.duplicate.scatter.generator.range,
        |p, a, v| p.duplicate.scatter.generator.range[a] = v.abs(),
        true,
        "",
    );
    vector(
        &mut f,
        "scatter_scale",
        "Scale",
        Unit::Percent,
        |p| p.duplicate.scatter.generator.scale,
        |p, a, v| p.duplicate.scatter.generator.scale[a] = v.clamp(0., 0.99),
        true,
        "",
    );
    vector(
        &mut f,
        "scatter_rotate",
        "Rotate",
        Unit::Angle,
        |p| p.duplicate.scatter.generator.rotate,
        |p, a, v| p.duplicate.scatter.generator.rotate[a] = v,
        true,
        "",
    );
    f.field(
        num(
            "scatter_seed",
            "Seed",
            Unit::Count,
            |p| p.duplicate.scatter.generator.seed as f64,
            |p, v| {
                p.duplicate.scatter.generator.seed =
                    crate::path_slice_panel::whole(v, 0., 1_000_000., "Seed")? as u32;
                Ok(())
            },
        )
        .tip("Picks the random set: the same seed always gives the same copies"),
    );
    to_items(f)
}
