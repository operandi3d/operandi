//! Deform toolbox tab, Tier 1B (specifications/deform-presets-2026-11-06):
//! Element Move, the transform-plus-falloff presets, Push, Smooth and the
//! shared Align row. Since layouts stage 3 batch 3 the tab is a form,
//! `assets/forms/toolbox.deform.json` (toolbox_form.rs), with its flyouts
//! as `flyout` sub-forms; this file starts the presets and holds the
//! Deform tools' Tool Properties. Behaviour lives in operandi-core's
//! `deform.rs`.
use super::*;
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::{PushNormals, tool_preset};

impl Operandi {
    /// Start a Deform preset, or drop it when it is the open one.
    #[cfg(test)]
    pub(super) fn activate_deform(&mut self, preset: operandi_core::DeformPreset) {
        self.activate_preset(preset.id());
    }

    /// Start a tool preset row (TP-10), or drop it when it is the open one.
    pub(super) fn activate_preset(&mut self, id: &str) {
        let Some(row) = tool_preset::row(id) else {
            self.status = format!("No tool preset called {id}");
            return;
        };
        if let Some(phrase) = &row.actor.app {
            // A mode the app runs itself (Weld, the Align tools).
            self.execute(phrase);
            return;
        }
        let before = self.editor.live_tool_label();
        self.run(Command::ActivatePreset(row.id.clone()));
        if self.editor.live_tool_label() != before {
            self.clear_fields();
        }
        if self
            .editor
            .session
            .preset
            .as_ref()
            .is_some_and(|p| p.id == row.id)
        {
            use tool_preset::FalloffKind as K;
            self.status = if row.falloff_kind() == Some(K::Selection) {
                format!(
                    "{} · drag the handle; right-drag changes Steps; Selection Border pivot",
                    row.label
                )
            } else if row.falloff_kind() == Some(K::Screen) {
                format!(
                    "{} · drag in the view; the circle follows each press; right-drag sizes it",
                    row.label
                )
            } else if row.falloff_kind() == Some(K::SoftSelection) {
                format!("{} · drag the handle; right-drag changes Radius", row.label)
            } else if row.falloff_kind() == Some(K::Element) {
                format!(
                    "{} · drag an element up or down to push it along its normal",
                    row.label
                )
            } else {
                format!(
                    "{} · drag the handle; the falloff handles reshape the effect",
                    row.label
                )
            };
        }
    }
}

/// Push and Smooth rows in Tool Properties.
pub(super) fn deform_tool_form(tool: Tool, p: &Parameters, f: &mut ToolForm<Parameters>) {
    use crate::live_fields::{check, dist, pick, plain, whole};
    if tool == Tool::Bend {
        bend_form(f);
        return;
    }
    if matches!(
        tool,
        Tool::Jitter | Tool::Quantize | Tool::RadialAlign | Tool::LinearAlign
    ) {
        deform_finish_form(tool, p, f);
        return;
    }
    if tool == Tool::Push {
        f.field(dist("push_distance", "Distance", |p| p.shift, |p, v| p.shift = v))
            .field(pick(
                "push-normals",
                "Selection Normals",
                &[
                    (PushNormals::Averaged, "Averaged"),
                    (PushNormals::Selected, "Selected"),
                ],
                |p| p.push_normals,
                |p, n| p.push_normals = n,
            ))
            .note("Drag up to push out, down to pull in. Selected uses only the selected polygons' normals, so a patch moves as a slab.");
        return;
    }
    f.field(plain(
        "smooth_strength",
        "Strength",
        |p| p.smooth.strength,
        |p, v| p.smooth.strength = v,
    ))
    .field(whole(
        "smooth_iterations",
        "Iterations",
        (0., 1000.),
        "Iterations must be a whole number from 0 to 1000",
        |p| p.smooth.iterations as f64,
        |p, n| p.smooth.iterations = n,
    ))
    .field(
        check(
            "smooth_lock_boundary",
            "Lock Boundary",
            |p| p.smooth.lock_boundary,
            |p, on| p.smooth.lock_boundary = on,
        )
        .tip("Points on open borders stay put"),
    )
    .field(
        check(
            "smooth_lock_corner",
            "Lock Corner",
            |p| p.smooth.lock_corner,
            |p, on| p.smooth.lock_corner = on,
        )
        .tip("Border points used by only one polygon stay put"),
    )
    .field(
        check(
            "smooth_preserve_volume",
            "Preserve Volume",
            |p| p.smooth.preserve_volume,
            |p, on| p.smooth.preserve_volume = on,
        )
        .tip("Keep the shape from shrinking (provisional, SM-Q1)"),
    )
    .field(
        check(
            "smooth_lock_sharp",
            "Lock Sharp Edges",
            |p| p.smooth.lock_sharp,
            |p, on| p.smooth.lock_sharp = on,
        )
        .tip("Points on edges sharper than Sharp Angle stay put"),
    )
    .field(
        tf::number(
            "smooth_sharp_angle",
            "Sharp Angle",
            crate::form::Unit::Angle,
            |p: &Parameters| p.smooth.sharp_angle,
            |p, v| {
                p.smooth.sharp_angle = v;
                Ok(())
            },
        )
        .enabled_if(p.smooth.lock_sharp, "Turn on Lock Sharp Edges first"),
    )
    .note("Drag left or right to change Strength. Works on the selection, or the whole mesh with nothing selected.");
}

/// Bend's rows, v2 (bend-handles-2026-11-15 §4, in the reference's order;
/// registry names angle, spineX/Y/Z, bothSides). Bend has no Base of its
/// own: the pivot is the action centre, shown under Auto Action Center
/// (BN-02), and the Axis row is the Action Axis Auto layer's (BN-03).
///
/// Not built yet, so left out (UIC-117): Slip UVs and Morph (G9: Operandi
/// has no shared transform Slip UVs / Morph setting yet) and the Action
/// Axis Handle option (G10, shared work for every tool on Automatic).
fn bend_form(f: &mut ToolForm<Parameters>) {
    use crate::form::Unit;
    use crate::live_fields::check;
    f.field(
        tf::number(
            "angle",
            "Angle",
            Unit::Angle,
            |p: &Parameters| p.bend.angle,
            |p, v| {
                p.bend.angle = v;
                Ok(())
            },
        )
        .tip("Bend at the spine's end; points nearer the centre turn less, none at the centre")
        // BN-46: `tool.attr xfrm.bend angle 90` is 90°; `?` answers radians.
        .script_degrees(),
    );
    // UIC-114: one row per component, the first labelled "Spine X".
    for (axis, (key, label)) in [("spineX", "Spine X"), ("spineY", "Y"), ("spineZ", "Z")]
        .into_iter()
        .enumerate()
    {
        f.field(
            tf::number(
                key,
                label,
                Unit::Distance,
                move |p: &Parameters| p.bend.spine[axis],
                move |p, v| {
                    let mut spine = p.bend.spine;
                    spine[axis] = v;
                    if spine.length() < 1e-9 {
                        return Err("The spine needs a length".into());
                    }
                    p.bend.spine = spine;
                    Ok(())
                },
            )
            .tip("The spine as a vector from the action centre to its end (BN-21)"),
        );
    }
    // UIC-115: a checkbox row with its text beside the box.
    f.field(
        check(
            "bothSides",
            "Both Sides",
            |p| p.bend.both_sides,
            |p, on| p.bend.both_sides = on,
        )
        .tip("Also bend the points behind the centre, as a mirror image"),
    );
    // BN-02 / BN-34: the action centre's own rows. Typing moves the ring
    // and the bend's start; the spine keeps its vector.
    let mut centre: Vec<tf::Item<Parameters>> = tf::vector(
        "operandi.center",
        "Center",
        Unit::Distance,
        |p: &Parameters| p.bend.base,
        |p, axis, v| {
            p.bend.base[axis] = v;
            Ok(())
        },
    )
    .into_iter()
    .map(|f| tf::Item::Field(Box::new(f.tip("The action centre: where the bend starts"))))
    .collect();
    centre.push(tf::Item::Field(Box::new(
        check(
            "operandi.deformSpace",
            "Deform Space",
            |p| p.bend.deform_space,
            |p, on| p.bend.deform_space = on,
        )
        .tip("Compute axis from deform space. Only matters for meshes shown deformed; a plain mesh is the same either way (provisional, BH-Q09)"),
    )));
    f.section("Auto Action Center", false, centre);
    // BN-03 / G5: X / Y / Z only, one always lit (UIC-89 joined segments).
    f.section(
        "Action Axis Auto",
        false,
        vec![tf::Item::Field(Box::new(
            tf::segments(
                "operandi.actionAxis",
                "Axis",
                &[(0, "X"), (1, "Y"), (2, "Z")],
                |p: &Parameters| p.bend.axis,
                |p, axis| p.bend.axis = axis,
            )
            .tip("The axis the bend curls around; the ring turns with it"),
        ))],
    );
    // BN-47: Reset gives Angle 0 and Spine (0, 1 m, 0).
    f.buttons(vec![tf::Button::command("Reset", Command::ResetBend)]);
    // G11.
    f.note("Drag the ring to bend. Drag the spine's end to set the bent region. Click elsewhere to move the centre.");
}

/// Jitter, Quantize, Radial Align and Linear Align rows (deform_finish.rs;
/// starting values DEFORM-PROBES). Left out until built (UIC-117): Slip
/// UVs, Morph, Lock Stacked, and the Align tools' Center and handles
/// (DFF-Q07).
fn deform_finish_form(tool: Tool, p: &Parameters, f: &mut ToolForm<Parameters>) {
    use crate::form::Unit;
    use crate::live_fields::{check, pick, vec3, whole};
    use operandi_core::{LinearMode, RadialMode};
    // UVT-135: on UVs only U and V take part, so the Z rows are greyed.
    let in_uv = p.uv.is_some();
    let flat = |f: &mut ToolForm<Parameters>, fields: Vec<crate::tool_form::Field<Parameters>>| {
        let fields = fields
            .into_iter()
            .enumerate()
            .map(|(a, x)| {
                x.suffix("")
                    .enabled_if(!(in_uv && a == 2), "UV tools edit U and V only (UVT-135)")
            })
            .collect();
        f.fields_from(fields);
    };
    match tool {
        Tool::Jitter => {
            // UIC-114: one row per component, the first named in full.
            flat(
                f,
                tf::vector(
                    "jitter_range",
                    "Range",
                    Unit::Distance,
                    |p: &Parameters| p.jitter.range,
                    |p, a, v| {
                        p.jitter.range[a] = v.max(0.);
                        Ok(())
                    },
                ),
            );
            f.field(whole(
                "jitter_seed",
                "Seed",
                (0., u32::MAX as f64),
                "Seed must be a whole number of zero or more",
                |p| p.jitter.seed as f64,
                |p, n| p.jitter.seed = n,
            ));
            for (axis, key, label) in [
                (0, "jitter_enable_x", "Enable X"),
                (1, "jitter_enable_y", "Enable Y"),
                (2, "jitter_enable_z", "Enable Z"),
            ] {
                f.field(
                    check(
                        key,
                        label,
                        move |p| p.jitter.enable[axis],
                        move |p, on| p.jitter.enable[axis] = on,
                    )
                    .enabled_if(
                        !(in_uv && axis == 2),
                        "UV tools edit U and V only (UVT-135)",
                    ),
                );
            }
            f.field(
                check(
                    "jitter_rigid_translate",
                    "Rigid Translate",
                    |p| p.jitter.rigid_translate,
                    |p, on| p.jitter.rigid_translate = on,
                )
                .tip("Move each connected piece as one block"),
            )
            .field(
                check(
                    "jitter_rigid_rotate",
                    "Rigid Rotate",
                    |p| p.jitter.rigid_rotate,
                    |p, on| p.jitter.rigid_rotate = on,
                )
                .tip("Turn each connected piece about its centre (provisional, DFF-Q02)"),
            )
            .field(
                check(
                    "jitter_rigid_scale",
                    "Rigid Scale",
                    |p| p.jitter.rigid_scale,
                    |p, on| p.jitter.rigid_scale = on,
                )
                .tip("Resize each connected piece about its centre (provisional, DFF-Q02)"),
            )
            .note("Drag right for more Range. The same Seed always gives the same result.");
        }
        Tool::Quantize => {
            flat(
                f,
                tf::vector(
                    "quantize_step",
                    "Step",
                    Unit::Distance,
                    |p: &Parameters| p.quantize.step,
                    |p, a, v| {
                        p.quantize.step[a] = v.max(0.);
                        Ok(())
                    },
                ),
            );
            f.note("Each point snaps to the nearest multiple of Step from the world origin; a Step of 0 leaves that axis alone. Drag right for a bigger step.");
        }
        Tool::RadialAlign => {
            let mode = p.radial_align.mode;
            f.field(pick(
                "radial_mode",
                "Mode",
                &RadialMode::ALL.map(|m| (m, m.label())),
                |p| p.radial_align.mode,
                |p, m| p.radial_align.mode = m,
            ))
            .field(
                whole(
                    "radial_sides",
                    "Sides",
                    (3., 1000.),
                    "Sides must be a whole number from 3 to 1000",
                    |p| p.radial_align.sides as f64,
                    |p, n| p.radial_align.sides = n,
                )
                .enabled_if(mode == RadialMode::NSided, "Only N-Sided has Sides"),
            )
            .field(
                tf::number(
                    "radial_rotate",
                    "Rotate",
                    Unit::Angle,
                    |p: &Parameters| p.radial_align.rotate,
                    |p, v| {
                        p.radial_align.rotate = v;
                        Ok(())
                    },
                )
                .enabled_if(mode == RadialMode::NSided, "Only N-Sided turns its corners"),
            )
            .field(
                tf::number(
                    "radial_radius",
                    "Radius",
                    Unit::Distance,
                    |p: &Parameters| p.radial_align.radius,
                    |p, v| {
                        if v < 0. {
                            return Err("Radius can't be negative (0 measures it)".into());
                        }
                        p.radial_align.radius = v;
                        Ok(())
                    },
                )
                .tip("0 uses the points' average distance from their centre"),
            )
            .field(tf::number(
                "radial_start",
                "Start Angle",
                Unit::Angle,
                |p: &Parameters| p.radial_align.start_angle,
                |p, v| {
                    p.radial_align.start_angle = v;
                    Ok(())
                },
            ))
            .field(
                tf::number(
                    "radial_central",
                    "Central Angle",
                    Unit::Angle,
                    |p: &Parameters| p.radial_align.central_angle,
                    |p, v| {
                        p.radial_align.central_angle = v;
                        Ok(())
                    },
                )
                .enabled_if(
                    mode == RadialMode::Arc,
                    "Only Arc uses Central Angle (DP-10)",
                ),
            )
            .field(tf::percent(
                "radial_weight",
                "Weight",
                " %",
                |p: &Parameters| p.radial_align.weight,
                |p, v| {
                    p.radial_align.weight = v / 100.;
                    Ok(())
                },
            ))
            .field(
                whole(
                    "radial_smoothing",
                    "Smoothing Iterations",
                    (0., 10_000.),
                    "Smoothing Iterations must be a whole number from 0 to 10000",
                    |p| p.radial_align.smoothing_iterations as f64,
                    |p, n| p.radial_align.smoothing_iterations = n,
                )
                .tip("Polygon selections: relax the points inside the border"),
            )
            .field(
                check(
                    "radial_flatten",
                    "Flatten",
                    |p| p.radial_align.flatten,
                    |p, on| p.radial_align.flatten = on,
                )
                .tip("Polygon selections: put the inner points on the circle's plane first"),
            )
            .field(
                tf::number(
                    "radial_exponent",
                    "Exponent",
                    Unit::Scalar,
                    |p: &Parameters| p.radial_align.exponent,
                    |p, v| {
                        if !(-4. ..=4.).contains(&v) {
                            return Err("Exponent must be between -4 and 4".into());
                        }
                        p.radial_align.exponent = v;
                        Ok(())
                    },
                )
                .enabled_if(mode != RadialMode::NSided, "N-Sided has straight sides")
                .tip("Positive squares the circle off, negative pinches it in"),
            )
            .note("Drag left or right to change Weight.");
        }
        _ => {
            let curve = p.linear_align.mode == LinearMode::Curve;
            f.field(pick(
                "linear_mode",
                "Mode",
                &LinearMode::ALL.map(|m| (m, m.label())),
                |p| p.linear_align.mode,
                |p, m| p.linear_align.mode = m,
            ))
            .field(
                check(
                    "linear_uniform",
                    "Uniform",
                    |p| p.linear_align.uniform,
                    |p, on| p.linear_align.uniform = on,
                )
                .tip("Space the points evenly; off, each moves to its nearest spot on the line"),
            )
            .field(
                check(
                    "linear_preserve",
                    "Preserve Original Curve",
                    |p| p.linear_align.preserve_curve,
                    |p, on| p.linear_align.preserve_curve = on,
                )
                .enabled_if(curve, "Only Curve mode has a curve to keep"),
            )
            .field(tf::percent(
                "linear_weight",
                "Weight",
                " %",
                |p: &Parameters| p.linear_align.weight,
                |p, v| {
                    p.linear_align.weight = v / 100.;
                    Ok(())
                },
            ))
            .field(
                whole(
                    "linear_smoothing",
                    "Smoothing Iterations",
                    (0., 10_000.),
                    "Smoothing Iterations must be a whole number from 0 to 10000",
                    |p| p.linear_align.smoothing_iterations as f64,
                    |p, n| p.linear_align.smoothing_iterations = n,
                )
                .enabled_if(curve, "Curve mode smooths the run; Line is straight"),
            )
            .field(check(
                "linear_align_length",
                "Align Length",
                |p| p.linear_align.align_length,
                |p, on| p.linear_align.align_length = on,
            ))
            .field(
                tf::number(
                    "linear_edge_length",
                    "Edge Length",
                    Unit::Distance,
                    |p: &Parameters| p.linear_align.edge_length,
                    |p, v| {
                        if v < 0. {
                            return Err("Edge Length can't be negative".into());
                        }
                        p.linear_align.edge_length = v;
                        Ok(())
                    },
                )
                .enabled_if(p.linear_align.align_length, "Turn on Align Length first")
                .tip("0 uses the last run's length"),
            )
            .field(check(
                "linear_align_first",
                "Align to First Selected",
                |p| p.linear_align.align_first,
                |p, on| p.linear_align.align_first = on,
            ))
            .field(
                check(
                    "linear_screen_align",
                    "Screen Align",
                    |p| p.linear_align.screen_align,
                    |p, on| p.linear_align.screen_align = on,
                )
                .tip("Straighten as seen along Axis Vector, keeping each point's depth"),
            );
            vec3(
                f,
                "linear_axis",
                "Axis Vector",
                Unit::Scalar,
                "",
                |p| p.linear_align.axis,
                |p, a, v| p.linear_align.axis[a] = v,
            );
            f.note("Drag left or right to change Weight. Works on open runs; a closed loop has no ends.");
        }
    }
}
