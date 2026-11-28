//! Tool Properties of the Editor's live tools (everything `ToolSession::live`
//! runs except the primitives and Smooth Shift / Thicken, which have their
//! own forms), declared as field lists for the shared form
//! (`tool_form.rs`).
//!
//! Each tool's rows edit its `Parameters`; the rows every live tool shares
//! sit around them: the title, the selected camera or light's channels (CL-14), Negative Scale (IT-04/05) and the
//! Drop / Cancel buttons. One publish hands all of it to the editor.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolFields, ToolForm};
use operandi_core::camera_light::{CameraEdit, ItemChannelEdit, LightEdit, LightKind};

/// The live tool's panel.
pub(super) struct LiveFields;

/// What the live panel edits: the tool's parameters plus the rows that
/// belong to the scene rather than the tool.
#[derive(Clone, PartialEq)]
pub(super) struct LiveP {
    pub(super) params: Parameters,
    /// The selected camera or light's channels, in the units shown.
    pub(super) channels: Vec<f64>,
}

/// The live tool this panel serves.
pub(super) fn live_tool(app: &Operandi) -> Option<&Live> {
    app.editor
        .session
        .live
        .as_ref()
        .filter(|l| !l.tool.is_primitive() && !l.tool.is_smooth_shift())
}

fn params(p: &LiveP) -> &Parameters {
    &p.params
}
fn params_mut(p: &mut LiveP) -> &mut Parameters {
    &mut p.params
}

/// The message for a refused field, naming it.
pub(super) fn refusal(label: &str, error: impl std::fmt::Display) -> String {
    format!("{label}: {error}. Correct the field or Escape to revert.")
}

impl ToolFields for LiveFields {
    type P = LiveP;
    fn params(app: &Operandi) -> Option<LiveP> {
        let live = live_tool(app)?;
        Some(LiveP {
            params: live.params.clone(),
            channels: app
                .channel_rows()
                .iter()
                .map(|r| r.value * r.scale)
                .collect(),
        })
    }
    fn form(app: &Operandi, p: &LiveP) -> ToolForm<LiveP> {
        let mut f = ToolForm::new();
        let Some(live) = live_tool(app) else {
            return f;
        };
        let tool = live.tool;
        f.push(live_title(app, tool));
        f.items
            .extend(app.live_body(live, &p.params).lift((params, params_mut)));
        // The falloff's own sections follow the tool's (FalloffFields,
        // FO-10), for every tool that takes falloff.
        if operandi_core::camera_light::tool_shows_item_channels(tool) {
            app.channel_fields(&mut f);
        }
        // IT-04/05: Negative Scale lets an item's scale go below zero.
        if matches!(tool, Tool::Scale | Tool::Transform)
            && app.editor.scene.selection.mode == Mode::Item
        {
            f.field(
                tf::check(
                    "negative_scale",
                    "Negative Scale",
                    |p: &LiveP| p.params.negative_scale,
                    |p, on| p.params.negative_scale = on,
                )
                .tip("Allow negative item scale (mirroring the item)"),
            );
        }
        f.separator();
        if matches!(tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude) {
            f.note(app.center_axis_note())
                .note("Ordinary handles edit this stage. Shift-click the viewport away from handles to continue on selected caps.")
                .buttons(vec![tf::Button::command("Continue on Caps", Command::Continue)]);
        }
        live_footer(app, &mut f);
        f
    }
    fn publish(app: &mut Operandi, p: LiveP) -> Result<()> {
        let Some(live) = live_tool(app).cloned() else {
            return Err("The tool isn't active".into());
        };
        for (row, shown) in app.channel_rows().into_iter().zip(&p.channels) {
            let value = shown / row.scale;
            if (value - row.value).abs() > 1e-12 {
                app.editor.dispatch((row.make)(value))?;
            }
        }
        if p.params != live.params {
            app.editor.dispatch(configure(live.tool, p.params))?;
        }
        Ok(())
    }
    fn published(app: &mut Operandi) {
        app.status = "Ready".into();
    }
    fn settled(app: &mut Operandi) -> bool {
        // A refused handle move retains confirmed params/geometry and
        // clears its field buffers. With no pending invalid text, that
        // rejected attempt must not trap Drop behind a stale stage error:
        // revalidate the retained values rather than bypassing the core
        // guard.
        if app.field_errors.is_empty()
            && let Some(live) = live_tool(app)
            && live.preview_error.is_some()
        {
            let c = configure(live.tool, live.params.clone());
            return match app.editor.dispatch(c) {
                Ok(()) => true,
                Err(error) => {
                    app.status = error;
                    false
                }
            };
        }
        app.field_errors.is_empty()
    }
    fn parse_error(key: &str, label: &str, error: String) -> String {
        refusal(&Operandi::field_name(key, label), error)
    }
    fn publish_status(key: &str, error: &str) -> String {
        if key.is_empty() {
            return error.into();
        }
        format!("{}: {error}", Operandi::field_name(key, key))
    }
}

/// The command that hands `p` to the live `tool`.
fn configure(tool: Tool, p: Parameters) -> Command {
    match tool {
        Tool::Bevel => Command::ConfigureBevel(p),
        Tool::Extrude => Command::ConfigureExtrude(p),
        _ => Command::Preview(p),
    }
}

/// The live panel's title: the tool and the selection mode.
pub(super) fn live_title<P>(app: &Operandi, tool: Tool) -> tf::Item<P> {
    tf::Item::Title(format!(
        "{} · {:?}",
        app.editor.live_tool_label().unwrap_or(tool.label()),
        app.editor.scene.selection.mode
    ))
}

/// Drop Tool and Cancel, and the typing hint, under every live tool.
pub(super) fn live_footer<P>(app: &Operandi, f: &mut ToolForm<P>) {
    // CS-01/21: an armed tool says how to start it and offers Apply, which
    // runs it once with these values and leaves it armed (Extrude has its
    // own Apply row).
    if let Some(status) = app.editor.armed_status() {
        f.note(status);
        if app.editor.armed_tool() != Some(Tool::Extrude) {
            f.buttons(vec![tf::Button::command("Apply", Command::Apply).tip(
                "Run the tool once with these values; it stays ready for a click",
            )]);
        }
    }
    let drop = app.shortcut("Drop");
    // UIC-04: the hint describes the rollback performed by Cancel.
    let cancel = match live_tool(app).map(|live| live.tool) {
        Some(Tool::Bevel | Tool::TwistBevel | Tool::EdgeExtend) => {
            "Restore this stage; keep previously continued stages"
        }
        _ => "Discard the current tool's changes",
    };
    f.buttons(vec![
        tf::Button::command(format!("Drop Tool{drop}"), Command::Drop)
            .tip(format!("Commit current task · Drop{drop}")),
        tf::Button::command("Cancel", Command::Cancel).tip(cancel),
    ])
    .note("Enter accepts expressions, e.g. 250 mm or 1 m/4.");
}

/// One of the selected camera or light's channels (CL-14).
pub(super) struct ChannelRow {
    key: &'static str,
    label: &'static str,
    /// The value as stored.
    value: f64,
    /// Shown = stored × scale (mm, degrees).
    scale: f64,
    range: std::ops::RangeInclusive<f64>,
    unit: Unit,
    make: Box<dyn Fn(f64) -> Command>,
}

// ---- small field builders over `Parameters` ----

type Form = ToolForm<Parameters>;

/// A number written straight into the parameters.
fn num(
    key: &str,
    label: &str,
    unit: Unit,
    get: impl Fn(&Parameters) -> f64 + 'static,
    set: impl Fn(&mut Parameters, f64) + 'static,
) -> tf::Field<Parameters> {
    tf::number(key, label, unit, get, move |p, v| {
        set(p, v);
        Ok(())
    })
}

/// A distance in metres.
pub(super) fn dist(
    key: &str,
    label: &str,
    get: impl Fn(&Parameters) -> f64 + 'static,
    set: impl Fn(&mut Parameters, f64) + 'static,
) -> tf::Field<Parameters> {
    num(key, label, Unit::Distance, get, set)
}

/// A plain number shown with no unit.
pub(super) fn plain(
    key: &str,
    label: &str,
    get: impl Fn(&Parameters) -> f64 + 'static,
    set: impl Fn(&mut Parameters, f64) + 'static,
) -> tf::Field<Parameters> {
    num(key, label, Unit::Scalar, get, set).suffix("")
}

/// Three fields for a vector in `unit` (suffix `suffix`).
pub(super) fn vec3(
    f: &mut Form,
    key: &str,
    label: &str,
    unit: Unit,
    suffix: &'static str,
    get: impl Fn(&Parameters) -> DVec3 + 'static,
    set: impl Fn(&mut Parameters, usize, f64) + 'static,
) {
    let fields = tf::vector(key, label, unit, get, move |p, a, v| {
        set(p, a, v);
        Ok(())
    });
    f.fields_from(fields.into_iter().map(|x| x.suffix(suffix)).collect());
}

/// Move / Rotate / Scale / Transform in UV space (UVE-1 section 2): the
/// offsets, angle and scale in U and V, and Transform's Tear Off (UVT-41).
fn uv_tool_form(tool: Tool, p: &Parameters, f: &mut Form) {
    let uv_number = |key: &str,
                     label: &str,
                     unit: Unit,
                     get: fn(&Parameters) -> f64,
                     set: fn(&mut Parameters, f64)| {
        tf::number(key, label, unit, get, move |p, v| {
            if !v.is_finite() {
                return Err("Enter a number".into());
            }
            set(p, v);
            Ok(())
        })
    };
    let moves = matches!(tool, Tool::Move | Tool::Transform);
    let turns = matches!(tool, Tool::Rotate | Tool::Transform);
    if moves {
        f.field(uv_number(
            "uv_offset_u",
            "Offset U",
            Unit::Scalar,
            |p| p.delta.x,
            |p, v| p.delta.x = v,
        ))
        .field(uv_number(
            "uv_offset_v",
            "Offset V",
            Unit::Scalar,
            |p| p.delta.y,
            |p, v| p.delta.y = v,
        ));
    }
    if turns {
        f.field(if tool == Tool::Rotate {
            uv_number(
                "uv_angle",
                "Angle",
                Unit::Angle,
                |p| p.delta.z,
                |p, v| p.delta.z = v,
            )
        } else {
            uv_number(
                "uv_angle",
                "Angle",
                Unit::Angle,
                |p| p.rotation_delta.z,
                |p, v| p.rotation_delta.z = v,
            )
        });
    }
    match tool {
        Tool::Scale => {
            f.field(uv_number(
                "uv_scale_u",
                "Scale U",
                Unit::Scalar,
                |p| p.delta.x,
                |p, v| p.delta.x = v,
            ))
            .field(uv_number(
                "uv_scale_v",
                "Scale V",
                Unit::Scalar,
                |p| p.delta.y,
                |p, v| p.delta.y = v,
            ));
        }
        Tool::Transform => {
            f.field(uv_number("uv_scale_u", "Scale U", Unit::Scalar, |p| p.scale_delta.x, |p, v| p.scale_delta.x = v))
                .field(uv_number("uv_scale_v", "Scale V", Unit::Scalar, |p| p.scale_delta.y, |p, v| p.scale_delta.y = v))
                .field(
                    check(
                        "uv_tear_off",
                        "Tear Off",
                        |p| p.uv.as_ref().is_some_and(|s| s.tear_off),
                        |p, on| {
                            if let Some(s) = p.uv.as_mut() {
                                s.tear_off = on;
                                s.proxy = None;
                            }
                        },
                    )
                    .tip("Split the selected polygons' UVs from their neighbours first, so the piece comes away; Ctrl/Cmd+Shift-drag does it for one drag"),
                );
        }
        _ => {}
    }
    let map = p.uv.as_ref().map(|s| s.map.as_str()).unwrap_or("");
    f.note(format!("Editing UVs of {map}; 3D stays where it is"));
}

/// A dropdown over the parameters.
pub(super) fn pick<T: PartialEq + Clone + 'static>(
    key: &str,
    label: &str,
    options: &[(T, &str)],
    get: impl Fn(&Parameters) -> T + 'static,
    set: impl Fn(&mut Parameters, T) + 'static,
) -> tf::Field<Parameters> {
    tf::choice(key, label, options, get, set)
}

/// A whole number from `lo` to `hi`, refused with `refusal`.
pub(super) fn whole(
    key: &str,
    label: &str,
    range: (f64, f64),
    refusal: &'static str,
    get: impl Fn(&Parameters) -> f64 + 'static,
    set: impl Fn(&mut Parameters, u32) + 'static,
) -> tf::Field<Parameters> {
    tf::count(key, label, range, refusal, get, set)
}

/// A checkbox over the parameters.
pub(super) fn check(
    key: &str,
    label: &str,
    get: impl Fn(&Parameters) -> bool + 'static,
    set: impl Fn(&mut Parameters, bool) + 'static,
) -> tf::Field<Parameters> {
    tf::check(key, label, get, set)
}

/// `tool.attr edge.extend <attr> <value>`: the reference names (registry
/// thread, 2026-11-13) as this panel's field keys, and Haul's readable
/// values as its option labels. Integers pick the options in the panel's
/// order, which is the reference order (spec v1.1): planeHandle none,
/// move, scale; hauling global, local; uvs none, u, v.
pub(super) fn edge_extend_attr(attr: &str, value: &str) -> (String, String) {
    let key = match attr.to_ascii_lowercase().as_str() {
        "offx" => "delta0",
        "offy" => "delta1",
        "offz" => "delta2",
        "rotx" => "rotation_delta0",
        "roty" => "rotation_delta1",
        "rotz" => "rotation_delta2",
        "sclx" => "scale_delta0",
        "scly" => "scale_delta1",
        "sclz" => "scale_delta2",
        "segs" => "segments0",
        "movehandle" => "extend_handles_move",
        "rotatehandle" => "extend_handles_rotate",
        "scalehandle" => "extend_handles_scale",
        "localhandle" => "extend_handles_local",
        "planehandle" => "extend_plane",
        "hauling" => "extend_haul",
        "uvs" => "extend_make_uvs",
        _ => attr,
    };
    let value = match (key, value.to_ascii_lowercase().as_str()) {
        ("extend_haul", "global") => "Global Offsets",
        ("extend_haul", "local") => "Local Offsets",
        _ => value,
    };
    (key.to_owned(), value.to_owned())
}

/// Edge Extend's Handles, Plane, Haul and Make UVs rows.
fn extend_options(f: &mut Form) {
    use operandi_core::{ExtendHaul, ExtendPlane, ExtendUvs};
    f.separator()
        .field(
            check(
                "extend_handles_move",
                "Move Handles",
                |p| p.edge_extend.handles.moves,
                |p, on| p.edge_extend.handles.moves = on,
            )
            .tip("Show the Offset arrows"),
        )
        .field(
            check(
                "extend_handles_rotate",
                "Rotate Handles",
                |p| p.edge_extend.handles.rotate,
                |p, on| p.edge_extend.handles.rotate = on,
            )
            .tip("Show the Rotate rings"),
        )
        .field(
            check(
                "extend_handles_scale",
                "Scale Handles",
                |p| p.edge_extend.handles.scale,
                |p, on| p.edge_extend.handles.scale = on,
            )
            .tip("Show the Scale squares"),
        )
        .field(
            check(
                "extend_handles_local",
                "Local Handles",
                |p| p.edge_extend.handles.local,
                |p, on| p.edge_extend.handles.local = on,
            )
            .tip("Show the Inset and Shift arrows"),
        )
        .field(pick(
            "extend_plane",
            "Plane",
            &[
                (ExtendPlane::None, "None"),
                (ExtendPlane::Move, "Move"),
                (ExtendPlane::Scale, "Scale"),
            ],
            |p| p.edge_extend.plane,
            |p, v| p.edge_extend.plane = v,
        ))
        .field(pick(
            "extend_haul",
            "Haul",
            &[
                (ExtendHaul::Global, "Global Offsets"),
                (ExtendHaul::Local, "Local Offsets"),
            ],
            |p| p.edge_extend.haul,
            |p, v| p.edge_extend.haul = v,
        ))
        .field(pick(
            "extend_make_uvs",
            "Make UVs",
            &[
                (ExtendUvs::Off, "None"),
                (ExtendUvs::U, "U"),
                (ExtendUvs::V, "V"),
            ],
            |p| p.edge_extend.make_uvs,
            |p, v| p.edge_extend.make_uvs = v,
        ))
        .note("Shift-click extends again from the new edges with the same values.");
}

/// A greyed checkbox at a fixed value (shown, not built).
fn greyed_check(key: &str, label: &str, on: bool, why: &str) -> tf::Field<Parameters> {
    tf::check(key, label, move |_| on, |_, _| {}).enabled_if(false, why)
}

/// Use Material and Material Name for Vertex Bevel, Vertex Extrude and
/// Edge Extrude (VTO-T02/T03, extrude-2026-11-05; defaults off and
/// empty). The name is applied as typed, like Polygon Bevel's (BV-R16).
pub(super) fn vertex_material_fields(p: &Parameters, f: &mut Form) {
    f.field(
        check(
            "vertex_use_material",
            "Use Material",
            |p| p.vertex_use_material,
            |p, on| p.vertex_use_material = on,
        )
        .tip("Tag the new polygons with Material Name"),
    )
    .field(
        tf::text(
            "vertex_material",
            "Material Name",
            |p: &Parameters| p.vertex_material.clone(),
            |p, t| p.vertex_material = t,
        )
        .enabled_if(p.vertex_use_material, "Turn on Use Material first"),
    );
}

/// Slide options shared by Edge Slide and Vertex Slide (tool audit Tier
/// 1; provisional readings TA1-Q04..Q06).
pub(super) fn slide_extras(p: &Parameters, f: &mut Form, edges: bool) {
    f.separator().field(
        check(
            "slide_stop_negative",
            "Stop Negative Slide",
            |p| p.slide_extras.stop_negative,
            |p, on| p.slide_extras.stop_negative = on,
        )
        .tip("Keep the slide between 0 and 100%: negative amounts do nothing"),
    );
    if edges {
        f.unbuilt(
            "Loop Slide",
            "on",
            "On is the measured slide (ES-05: it may pass the next loop); what Off changes isn't measured yet",
        )
        .unbuilt(
            "Merge Vertices",
            "on",
            "ES-04: an edge slide always merges points that reach their neighbour",
        );
    } else {
        f.field(
            check(
                "slide_loop",
                "Loop Slide",
                |p| p.slide_extras.loop_slide,
                |p, on| p.slide_extras.loop_slide = on,
            )
            .tip("Keep each point on its edge; off lets it carry on past the end"),
        );
    }
    f.unbuilt(
        "Duplicate",
        "off",
        "Not built yet: sliding a copy joined by new polygons is a bigger change",
    )
    .field(
        check(
            "slide_preserve_curvature",
            "Preserve Curvature",
            |p| p.slide_extras.preserve_curvature,
            |p, on| p.slide_extras.preserve_curvature = on,
        )
        .tip("Points follow the surface's curve instead of the straight edge"),
    )
    .field(
        tf::percent(
            "slide_tension",
            "Tension",
            "%",
            |p: &Parameters| p.slide_extras.tension,
            |p, v| {
                if !(0. ..=1000.).contains(&v) {
                    return Err("Tension must be between 0% and 1000%".into());
                }
                p.slide_extras.tension = v / 100.;
                Ok(())
            },
        )
        .enabled_if(
            p.slide_extras.preserve_curvature,
            "Turn on Preserve Curvature first",
        ),
    )
    .field(
        check(
            "slide_slip_uvs",
            "Slip UVs",
            |p| p.slide_extras.slip_uvs,
            |p, on| p.slide_extras.slip_uvs = on,
        )
        .tip("Slid points take new UVs so the texture stays still (every UV map)"),
    );
}

impl Operandi {
    /// A field's name in messages: the long name where the row label is
    /// short ("+/−", an axis letter), else the label.
    pub(super) fn field_name(key: &str, label: &str) -> String {
        let known = matches!(
            key,
            "bevel_segments"
                | "bevel_shift_range"
                | "bevel_inset_range"
                | "bevel_angle"
                | "bevel_weld_range"
                | "edge_slide_distance"
                | "tweak_range"
                | "vertex_slide_percent"
                | "slide_tension"
                | "edge_weight"
                | "extrude_sides"
        ) || key.starts_with("extent")
            || key.starts_with("delta")
            || key.starts_with("position")
            || key.starts_with("size")
            || key.starts_with("radius")
            || key.starts_with("min")
            || key.starts_with("max");
        if known || label.len() <= 1 || label == key {
            Self::field_label(key)
        } else {
            label.into()
        }
    }

    fn center_axis_note(&self) -> String {
        format!(
            "Center: {} · Axis: {}",
            self.editor.action_context.center.label(),
            self.editor.action_context.axis.label()
        )
    }

    /// The rows of the live `tool` that edit its parameters.
    fn live_body(&self, live: &Live, p: &Parameters) -> Form {
        let mut f = Form::new();
        match live.tool {
            Tool::LoopSlice
            | Tool::EdgeSlice
            | Tool::PolygonSlice
            | Tool::CurveSlice
            | Tool::PenSlice
            | Tool::PrimitiveSlice
            | Tool::AxisSlice => self.slice_form(live.tool, p, &mut f),
            Tool::Contour => crate::contour_ui::contour_form(p, &mut f),
            // DX-24: a Transform button shows only its placer.
            t if t.is_duplicate() && crate::duplicate_transform_panel::is_transform(p) => {
                self.duplicate_transform_form(t, p, &mut f)
            }
            Tool::Mirror
            | Tool::Lathe
            | Tool::Clone
            | Tool::Array
            | Tool::RadialArray
            | Tool::ScatterClone
            | Tool::Symmetrize => self.duplicate_form(live.tool, p, &mut f),
            Tool::CurveExtrude | Tool::CurveClone => self.path_tool_form(live.tool, p, &mut f),
            Tool::UvProject => uv_tool_forms::project_form(self, p, &mut f),
            Tool::UvViewProject => uv_tool_forms::view_project_form(self, p, &mut f),
            Tool::UvUnwrap => uv_tool_forms::unwrap_form(self, p, &mut f),
            Tool::UvRelax => uv_tool_forms::relax_form(self, p, &mut f),
            Tool::UvPelt => uv_tool_forms::pelt_form(self, p, &mut f),
            Tool::UvPeeler => uv_tool_forms::peel_form(p, &mut f),
            Tool::UvBoxTransform => uv_tool_forms::box_form(p, &mut f),
            Tool::SketchExtrude => {
                // Spec: SKE-P03 builds Sweep with these fixed; the rest of
                // PT-SKE-01..13 is unmeasured, so it's left out (UIC-117).
                let later = "Not measured yet: Sketch Extrude sweeps a fixed section for now";
                f.unbuilt(
                    "Mode",
                    "Sweep",
                    "Edit Path, Delete Knot and Uniform Spans aren't measured yet",
                )
                .unbuilt("Move by Path", "off", later)
                .unbuilt("Uniform", "off", later)
                .unbuilt("Align to Path", "off", later)
                .field(tf::number(
                    "sketch_precision",
                    "Precision (px)",
                    Unit::Scalar,
                    |p: &Parameters| p.sketch_precision,
                    |p, v| {
                        if !v.is_finite() || !(1. ..=1000.).contains(&v) {
                            return Err(
                                "Sketch Extrude Precision requires 1–1000 logical pixels".into()
                            );
                        }
                        p.sketch_precision = v;
                        Ok(())
                    },
                ))
                .unbuilt("Scale", "100 %", later)
                .unbuilt("Spin", "0°", later)
                .separator()
                .unbuilt("Make UVs", "off", later)
                .unbuilt("U Repeat", "1", later)
                .unbuilt("V Repeat", "1", later)
                .unbuilt("U Rotate", "0°", later)
                .unbuilt("Profiles", "(none)", later)
                .note("Drag sensitivity and sampling may change.");
            }
            Tool::EdgeExtend => {
                // SPEC-EDGE-EXTEND-ROTATE-SCALE: Offset, then Rotate and
                // Scale about the action centre, then Local offsets.
                vec3(
                    &mut f,
                    "delta",
                    "Offset",
                    Unit::Distance,
                    " m",
                    |p| p.delta,
                    |p, a, v| p.delta[a] = v,
                );
                vec3(
                    &mut f,
                    "rotation_delta",
                    "Rotate",
                    Unit::Angle,
                    "",
                    |p| p.rotation_delta,
                    |p, a, v| p.rotation_delta[a] = v,
                );
                vec3(
                    &mut f,
                    "scale_delta",
                    "Scale",
                    Unit::Percent,
                    " %",
                    |p| p.scale_delta * 100.,
                    |p, a, v| p.scale_delta[a] = v / 100.,
                );
                f.field(dist("inset", "Inset", |p| p.inset, |p, v| p.inset = v))
                    .field(dist("shift", "Shift", |p| p.shift, |p, v| p.shift = v));
                f.field(whole(
                    "segments0",
                    "Segments",
                    (1., u32::MAX as f64),
                    "Segments require a positive integer within the preview budget",
                    |p| p.segments[0] as f64,
                    |p, n| p.segments[0] = n,
                ))
                .note("Selected boundary edges extend by Offset; the new outer edges stay selected for another Extend.");
                extend_options(&mut f);
            }
            Tool::EdgeCrease => {
                f.field(
                    tf::percent(
                        "edge_weight",
                        "Weight",
                        "%",
                        |p: &Parameters| p.shift,
                        // EW2-01: any weight is kept; the surface clamps its effect.
                        |p, v| {
                            p.shift = v / 100.;
                            Ok(())
                        },
                    ),
                )
                .field(
                    check(
                        "edge_weight_additive",
                        "Additive",
                        |p| p.edge_weight_additive,
                        |p, on| {
                            p.edge_weight_additive = on;
                            // The field becomes the amount added, from 0.
                            if on {
                                p.shift = 0.;
                            }
                        },
                    )
                    .tip("Add Weight to each edge's current weight instead of replacing it"),
                )
                .note("Positive pulls the surface toward the edge, negative pushes it away (Tab surfaces only; Catmull-Clark ignores negatives). The effect stops at ±100%.");
            }
            Tool::EdgeSlide => {
                f.field(interpolation("edge-slide-interpolation"));
                if p.edge_slide_distance {
                    f.field(dist(
                        "edge_slide_distance",
                        "Distance",
                        |p| p.shift,
                        |p, v| p.shift = v,
                    ));
                } else {
                    f.field(plain("shift", "Percent", |p| p.shift, |p, v| p.shift = v));
                }
                f.note("Along each connected edge (1 = all the way, where points merge); negative slides to the other side.");
                slide_extras(p, &mut f, true);
                // UVT-137 / UVT-Q16: (none) slides points in 3D; a map
                // slides that map's UVs along their UV edges, from any view.
                let maps: Vec<String> = self
                    .editor
                    .scene
                    .active()
                    .map(|i| i.mesh.uv_maps.keys().cloned().collect())
                    .unwrap_or_default();
                let mut options: Vec<(Option<String>, &str)> = vec![(None, "(none)")];
                options.extend(maps.iter().map(|m| (Some(m.clone()), m.as_str())));
                let untouched = self.editor.scene == live.base;
                f.field(
                    pick(
                        "slide_uv_map",
                        "UV Map",
                        &options,
                        |p| p.uv.as_ref().map(|s| s.map.clone()),
                        |p, m| {
                            p.uv = m.map(|map| operandi_core::uv_edit::UvToolSpace {
                                map,
                                ..Default::default()
                            })
                        },
                    )
                    .command(|app, p| {
                        let map = p.uv.as_ref().map(|s| s.map.clone());
                        app.run(Command::UvToolSpace(map.map(Some)));
                    })
                    .enabled_if(
                        untouched,
                        "Drop the tool before switching between UV and 3D",
                    )
                    .tip("Slide this UV map's UVs along their UV edges instead of the points"),
                );
            }
            Tool::EdgeBevel | Tool::EdgeChamfer => {
                edge_bevel_form(p, &mut f, live.tool == Tool::EdgeBevel)
            }
            Tool::Inset => {
                f.field(dist("inset", "Inset", |p| p.inset, |p, v| p.inset = v))
                    .note("Each polygon gets a smaller copy inside it; negative grows. Nothing selected insets every polygon.");
            }
            Tool::Shift => {
                f.field(dist("shift", "Shift", |p| p.shift, |p, v| p.shift = v))
                    .note("Each polygon moves along its own normal on new points, joined by side polygons. Nothing selected shifts every polygon.");
            }
            Tool::EdgeRelax => {
                f.field(dist("shift", "Offset", |p| p.shift, |p, v| p.shift = v))
                    .note("Select a bevel's edges and one edge on either side.");
            }
            Tool::VertexCreate
            | Tool::VertexBevel
            | Tool::VertexExtrude
            | Tool::VertexMerge
            | Tool::VertexTear
            | Tool::VertexSlide => vertex_panel::vertex_tool_form(live.tool, p, &mut f),
            Tool::EdgeExtrude => {
                f.field(dist("shift", "Extrude", |p| p.shift, |p, v| p.shift = v))
                    .field(dist("inset", "Width", |p| p.inset, |p, v| p.inset = v));
                vertex_material_fields(p, &mut f);
                f.note("Nothing selected extrudes every edge; Width 0 changes nothing.");
            }
            Tool::PolyHaul => {
                // Add Loop is open (Q-PH-05), so it isn't listed.
                let modes: Vec<_> = operandi_core::PolyHaulMode::ALL
                    .iter()
                    .map(|&m| (m, m.label()))
                    .collect();
                f.field(pick(
                    "poly-haul-mode",
                    "Mode",
                    &modes,
                    |p| p.poly_haul_mode,
                    |p, m| {
                        if p.poly_haul_mode != m {
                            // A new mode starts from the scene as it is.
                            p.poly_haul_target = Default::default();
                        }
                        p.poly_haul_mode = m;
                    },
                ))
                .field(dist(
                    "poly_haul_offset",
                    "Offset",
                    |p| p.poly_haul_offset,
                    |p, v| p.poly_haul_offset = v,
                ))
                .note(if p.poly_haul_mode.uses_selection() {
                    "Drag anywhere: acts on the selected polygons."
                } else {
                    "Start the drag on the edge, point or polygon to work on."
                });
            }
            Tool::Chamfer => chamfer_edit_form(&mut f),
            Tool::Box | Tool::Sphere | Tool::Cylinder => {
                unreachable!("primitives have their own form")
            }
            Tool::SmoothShift | Tool::Thicken => unreachable!("Smooth Shift has its own form"),
            Tool::Bevel => self.bevel_form(live, p, &mut f),
            Tool::TwistBevel => twist_bevel_form(p, &mut f),
            Tool::Extrude => self.extrude_form(live, p, &mut f),
            // UVE-1 section 2: in a UV view the transform tools show U, V
            // offsets, one angle and U / V scale (uv_edit.rs).
            Tool::Move | Tool::Rotate | Tool::Scale | Tool::Transform if p.uv.is_some() => {
                uv_tool_form(live.tool, p, &mut f)
            }
            Tool::Transform => {
                vec3(
                    &mut f,
                    "delta",
                    "Position",
                    Unit::Distance,
                    " m",
                    |p| p.delta,
                    |p, a, v| p.delta[a] = v,
                );
                vec3(
                    &mut f,
                    "rotation_delta",
                    "Rotation °",
                    Unit::Angle,
                    "",
                    |p| p.rotation_delta,
                    |p, a, v| p.rotation_delta[a] = v,
                );
                vec3(
                    &mut f,
                    "scale_delta",
                    "Scale Factor",
                    Unit::Scalar,
                    "",
                    |p| p.scale_delta,
                    |p, a, v| p.scale_delta[a] = v,
                );
                f.note(self.center_axis_note())
                    .note("Scale, Rotate, then Move");
            }
            // Size Absolute (snaps-precision C): Absolute is the base size,
            // Relative the size wanted; the factor is Relative ÷ Absolute.
            Tool::Scale if p.absolute_handles.is_some_and(|a| a.size) => {
                vec3(
                    &mut f,
                    "absolute",
                    "Absolute",
                    Unit::Distance,
                    " m",
                    |p| p.absolute_handles.map_or(DVec3::ONE, |a| a.base),
                    |p, a, v| {
                        if let Some(h) = p.absolute_handles.as_mut()
                            && v.is_finite()
                            && v.abs() > 1e-9
                        {
                            // The size wanted stays; the factor follows.
                            let relative = h.base[a] * p.delta[a];
                            h.base[a] = v;
                            p.delta[a] = relative / v;
                        }
                    },
                );
                vec3(
                    &mut f,
                    "relative",
                    "Relative",
                    Unit::Distance,
                    " m",
                    |p| p.absolute_handles.map_or(DVec3::ONE, |a| a.base) * p.delta,
                    |p, a, v| {
                        if let Some(h) = p.absolute_handles
                            && v.is_finite()
                        {
                            p.delta[a] = v / h.base[a];
                        }
                    },
                );
                f.note(self.center_axis_note()).note(
                    "Drag a circle onto a point for the base size, then its square for the size",
                );
            }
            // Axis Rotate (TC-33): one Angle, about its ring's axis.
            Tool::Rotate if self.axis_rotate_axis().is_some() => {
                let axis = self.axis_rotate_axis().unwrap_or(1);
                f.field(
                    tf::number(
                        "delta",
                        "Angle",
                        Unit::Angle,
                        move |p: &Parameters| p.delta[axis],
                        move |p, v| {
                            p.delta[axis] = v;
                            Ok(())
                        },
                    )
                    .tip(format!(
                        "The turn about world {}, the axis the view looks along",
                        ["X", "Y", "Z"][axis]
                    )),
                )
                .note(format!(
                    "Ctrl+drag the ring to snap to Snap Angle ({}°, Preferences ▸ Handles).",
                    self.handle_preferences.rotation_snap
                ))
                .buttons(vec![
                    tf::Button::command("Apply", Command::Apply)
                        .tip("Turn the selection by Angle about the ring's centre"),
                ]);
            }
            Tool::Move | Tool::Rotate | Tool::Scale => {
                let (label, unit, suffix) = match live.tool {
                    Tool::Move => ("Position (offset)", Unit::Distance, " m"),
                    Tool::Rotate => ("Rotation (degrees)", Unit::Angle, ""),
                    _ => ("Scale Factor", Unit::Scalar, ""),
                };
                vec3(
                    &mut f,
                    "delta",
                    label,
                    unit,
                    suffix,
                    |p| p.delta,
                    |p, a, v| p.delta[a] = v,
                );
                // Scale Absolute (snaps-precision B): the base handles'
                // lengths; they change how a drag maps, not typed factors.
                if live.tool == Tool::Scale && p.absolute_handles.is_some() {
                    vec3(
                        &mut f,
                        "absolute_size",
                        "Absolute Size",
                        Unit::Distance,
                        " m",
                        |p| p.absolute_handles.map_or(DVec3::ONE, |a| a.base),
                        |p, a, v| {
                            if let Some(h) = p.absolute_handles.as_mut()
                                && v.is_finite()
                                && v.abs() > 1e-9
                            {
                                h.base[a] = v;
                            }
                        },
                    );
                }
                f.note(self.center_axis_note()).note(
                    if self.editor.scene.selection.mode == Mode::Item {
                        "Editing item transform"
                    } else {
                        "Editing component positions"
                    },
                );
            }
            Tool::Tweak if self.topology_pen_open() => topology_pen_ui::pen_form(p, &mut f),
            Tool::Tweak => tweak_form(p, &mut f),
            Tool::Push
            | Tool::Smooth
            | Tool::Bend
            | Tool::Jitter
            | Tool::Quantize
            | Tool::RadialAlign
            | Tool::LinearAlign => deform_panel::deform_tool_form(live.tool, p, &mut f),
            Tool::SymmetryTool => {
                f.field(dist(
                    "symmetry_threshold",
                    "Threshold",
                    |p| p.symmetry_fix.threshold,
                    |p, v| p.symmetry_fix.threshold = v.max(0.),
                ))
                .field(
                    check(
                        "symmetry_topology",
                        "Use Topology",
                        |p| p.symmetry_fix.topology,
                        |p, on| p.symmetry_fix.topology = on,
                    )
                    .tip("Pair points through the mesh's connections, so a point pulled well off its mirror position still finds its partner"),
                )
                .field(
                    check(
                        "symmetry_reverse",
                        "Reverse",
                        |p| p.symmetry_fix.reverse,
                        |p, on| p.symmetry_fix.reverse = on,
                    )
                    .tip("Off: the negative side moves to match the positive side. On: the other way"),
                );
                let unpaired = self.editor.symmetry_unpaired.len();
                f.note(if unpaired == 0 {
                    "Drag right to raise Threshold. Every point has a mirror partner.".to_string()
                } else {
                    format!(
                        "Drag right to raise Threshold. {unpaired} point{} still without a mirror partner (yellow squares).",
                        if unpaired == 1 { "" } else { "s" }
                    )
                });
            }
        }
        f
    }

    /// Polygon Bevel: offsets, profile, grouping and weld settings.
    fn bevel_form(&self, live: &Live, p: &Parameters, f: &mut Form) {
        let profile = self.editor.bevel_profile.is_some();
        f.field(dist("shift", "Shift", |p| p.shift, |p, v| p.shift = v))
            .field(dist(
                "bevel_shift_range",
                "+/−",
                |p| p.bevel.shift_range,
                |p, v| p.bevel.shift_range = v,
            ))
            .field(
                dist("inset", "Inset", |p| p.inset, |p, v| p.inset = v).enabled_if(
                    !(profile && p.bevel.keep_aspect),
                    "Keep Aspect ties Shift to Inset for this profile",
                ),
            )
            .field(dist(
                "bevel_inset_range",
                "+/−",
                |p| p.bevel.inset_range,
                |p, v| p.bevel.inset_range = v,
            ))
            .field(tf::segments(
                "bevel-edges",
                "Edges",
                &[(BevelEdges::Inner, "Inner"), (BevelEdges::Outer, "Outer")],
                |p: &Parameters| p.bevel.edges,
                |p, e| p.bevel.edges = e,
            ))
            .field(check(
                "bevel_use_material",
                "Use Material",
                |p| p.bevel.use_material,
                |p, on| p.bevel.use_material = on,
            ))
            .field(
                tf::text(
                    "bevel_material",
                    "Material Name",
                    |p: &Parameters| p.bevel.material.clone(),
                    |p, t| p.bevel.material = t,
                )
                .enabled_if(p.bevel.use_material, "Turn on Use Material first"),
            )
            .field(check(
                "bevel_group",
                "Group Polygons",
                |p| p.group_polygons,
                |p, on| p.group_polygons = on,
            ))
            .field(tf::number(
                "bevel_segments",
                "Segments",
                Unit::Count,
                |p: &Parameters| p.bevel.segments as f64,
                |p, v| {
                    if v.fract() != 0. || v > 4096. {
                        return Err("Bevel Segments requires an integer up to 4096".into());
                    }
                    p.bevel.segments = v.max(1.) as u32;
                    Ok(())
                },
            ))
            .field(check(
                "bevel_square",
                "Square Corner",
                |p| p.bevel.square,
                |p, on| p.bevel.square = on,
            ))
            .field(
                check(
                    "bevel_edge_rail",
                    "Edge Rail",
                    |p| p.bevel.edge_rail,
                    |p, on| p.bevel.edge_rail = on,
                )
                .enabled_if(p.group_polygons, "Turn on Group Polygons first"),
            )
            .field(check(
                "bevel_offset_even",
                "Offset Even",
                |p| p.bevel.offset_even,
                |p, on| p.bevel.offset_even = on,
            ))
            .field(check(
                "bevel_thicken",
                "Thicken Polygon",
                |p| p.bevel.thicken,
                |p, on| p.bevel.thicken = on,
            ))
            .field(num(
                "bevel_angle",
                "Max. Smoothing Angle",
                Unit::Angle,
                |p| p.bevel.max_angle,
                |p, v| p.bevel.max_angle = v.max(0.),
            ))
            .field(check(
                "bevel_normal_map",
                "Use Vertex Normal Map",
                |p| p.bevel.use_normal_map,
                |p, on| p.bevel.use_normal_map = on,
            ))
            .field(check(
                "bevel_open_boundary",
                "Open Boundary",
                |p| p.bevel.open_boundary,
                |p, on| p.bevel.open_boundary = on,
            ))
            .field(
                check(
                    "bevel_auto_weld",
                    "Auto Weld",
                    |p| p.bevel.auto_weld,
                    |p, on| p.bevel.auto_weld = on,
                )
                .tip(
                    "Repair planar contour contacts; unsupported nonplanar caps refuse atomically",
                ),
            )
            .field(
                // BV-R26: a negative Weld Range entry clamps to zero.
                dist(
                    "bevel_weld_range",
                    "Weld Range",
                    |p| p.bevel.weld_range,
                    |p, v| p.bevel.weld_range = v.max(0.),
                )
                .enabled_if(p.bevel.auto_weld, "Turn on Auto Weld first"),
            )
            .field(
                // PT-BEV-21: Falloff Weight is a popup.
                pick(
                    "bevel_weight",
                    "Falloff Weight",
                    &[
                        (BevelFalloff::Vertex, "Vertex"),
                        (BevelFalloff::Averaged, "Averaged"),
                        (BevelFalloff::Polygon, "Polygon"),
                    ],
                    |p| p.bevel.falloff,
                    |p, w| p.bevel.falloff = w,
                )
                .enabled_if(!p.group_polygons, "Turn off Group Polygons first"),
            )
            .field(check(
                "bevel_falloff_shift",
                "Falloff Shift",
                |p| p.bevel.falloff_shift,
                |p, on| p.bevel.falloff_shift = on,
            ))
            .field(check(
                "bevel_falloff_inset",
                "Falloff Inset",
                |p| p.bevel.falloff_inset,
                |p, on| p.bevel.falloff_inset = on,
            ));
        for (key, label, get, set) in [
            (
                "bevel_reverse_direction",
                "Reverse Direction",
                (|p: &Parameters| p.bevel.reverse_direction) as fn(&Parameters) -> bool,
                (|p: &mut Parameters, on| p.bevel.reverse_direction = on)
                    as fn(&mut Parameters, bool),
            ),
            (
                "bevel_reverse_inset",
                "Reverse Inset",
                |p| p.bevel.reverse_inset,
                |p, on| p.bevel.reverse_inset = on,
            ),
            (
                "bevel_keep_aspect",
                "Keep Aspect",
                |p| p.bevel.keep_aspect,
                |p, on| p.bevel.keep_aspect = on,
            ),
        ] {
            f.field(
                check(key, label, get, set).enabled_if(profile, "Choose a bevel profile first"),
            );
        }
        // PT-BEV-27 lists the choices but not the control kind (asked).
        f.field(tf::segments(
            "bevel-uvs",
            "Make UVs",
            &[
                (BevelUvs::Connected, "Connected"),
                (BevelUvs::U, "U"),
                (BevelUvs::V, "V"),
            ],
            |p: &Parameters| p.bevel.uvs,
            |p, u| p.bevel.uvs = u,
        ));
        self.bevel_extras(live, f);
    }

    /// Extrude: offset, sides and stage settings.
    fn extrude_form(&self, live: &Live, p: &Parameters, f: &mut Form) {
        vec3(
            f,
            "extent",
            "Extent (offset)",
            Unit::Distance,
            " m",
            |p| p.extent,
            |p, a, v| p.extent[a] = v,
        );
        // Selection handles write the equivalent world displacement; the
        // fields keep their world meaning (Operandi D18-A).
        f.note("Extent XYZ: World coordinates")
            .field(tf::number(
                "extrude_sides",
                "Sides",
                Unit::Count,
                |p: &Parameters| p.extrude_sides as f64,
                |p, v| {
                    p.extrude_sides = extrude_side_count(v)?;
                    Ok(())
                },
            ))
            .separator();
        let names: Vec<String> = self
            .editor
            .scene
            .active()
            .map(|i| i.mesh.uv_maps.keys().cloned().collect())
            .unwrap_or_default();
        let available =
            p.extrude_uv_selected && p.extrude_uv_map.as_ref().is_some_and(|n| names.contains(n));
        // PT-EXT-03 lists the choices but not the control kind (asked).
        f.field(
            tf::segments(
                "extrude-uvs",
                "Make UVs",
                &[
                    (ExtrudeUvs::None, "None"),
                    (ExtrudeUvs::U, "U"),
                    (ExtrudeUvs::V, "V"),
                ],
                |p: &Parameters| p.extrude_uvs,
                |p, u| p.extrude_uvs = u,
            )
            .enabled_if(
                available,
                "Select a UV map to edit this choice; Apply retains the current mapping choice",
            ),
        )
        .field(pick(
            "extrude_mode",
            "Mode",
            &[
                (ExtrudeMode::Auto, "Auto"),
                (ExtrudeMode::Thick, "Thick"),
                (ExtrudeMode::Shift, "Shift"),
            ],
            |p| p.extrude_mode,
            |p, m| p.extrude_mode = m,
        ))
        .separator()
        .buttons(vec![
            tf::Button::command("Apply", Command::Apply)
                .tip("Apply configured values as a separate stage on the current selected caps")
                .enabled_if(!live.applied, "Applied; change a value to apply again"),
        ])
        .section(
            "Tool States",
            true,
            vec![
                // Spec: EX-Q05 / PT-Q13
                tf::Item::Note("Named tool states aren't available yet".into()),
                tf::Item::Buttons(vec![tf::Button::command(
                    "Reset Tool Attributes",
                    Command::ResetExtrude,
                )]),
            ],
        );
        let none = "No UV map selected";
        let mut options: Vec<(Option<String>, &str)> = vec![(None, none)];
        options.extend(names.iter().map(|n| (Some(n.clone()), n.as_str())));
        let shown = p
            .extrude_uv_map
            .clone()
            .filter(|_| p.extrude_uv_selected)
            .unwrap_or_else(|| none.into());
        f.field(
            pick(
                "extrude_uv_map",
                "UV Map",
                &options,
                |p| p.extrude_uv_map.clone().filter(|_| p.extrude_uv_selected),
                |p, map| {
                    p.extrude_uv_selected = map.is_some();
                    if map.is_some() {
                        p.extrude_uv_map = map;
                    }
                },
            )
            .otherwise(shown),
        );
    }

    /// CL-14: with Move, Rotate or Transform active on the selected camera
    /// or light, its own channels (area Width and Height, spot Cone Angle
    /// and Soft Edge, camera Focal Length and Focus Distance, point
    /// Radius; directional none). Each change is a tweak of the tool.
    pub(super) fn channel_rows(&self) -> Vec<ChannelRow> {
        let live = live_tool(self).map(|l| l.tool);
        let scene = &self.editor.scene;
        if !live.is_some_and(operandi_core::camera_light::tool_shows_item_channels)
            || scene.selection.mode != Mode::Item
        {
            return Vec::new();
        }
        let Some(item) = scene.active() else {
            return Vec::new();
        };
        let id = item.id;
        let deg = 180. / std::f64::consts::PI;
        let cam = move |e: fn(f64) -> CameraEdit| -> Box<dyn Fn(f64) -> Command> {
            Box::new(move |v| Command::ToolItemChannel {
                item: id,
                edit: ItemChannelEdit::Camera(e(v)),
            })
        };
        let light = move |e: fn(f64) -> LightEdit| -> Box<dyn Fn(f64) -> Command> {
            Box::new(move |v| Command::ToolItemChannel {
                item: id,
                edit: ItemChannelEdit::Light(e(v)),
            })
        };
        let row = |key, label, value, scale, range, unit, make| ChannelRow {
            key,
            label,
            value,
            scale,
            range,
            unit,
            make,
        };
        if let Some(c) = &item.camera {
            return vec![
                row(
                    "tool-focal",
                    "Focal Length",
                    c.focal_length,
                    1000.,
                    1.0..=1e5,
                    Unit::Suffix(" mm"),
                    cam(CameraEdit::FocalLength),
                ),
                row(
                    "tool-focus",
                    "Focus Distance",
                    c.focus_distance,
                    1.,
                    0.001..=1e6,
                    Unit::Distance,
                    cam(CameraEdit::FocusDistance),
                ),
            ];
        }
        let Some(l) = &item.light else {
            return Vec::new();
        };
        let radius = || {
            row(
                "tool-radius",
                "Radius",
                l.radius,
                1.,
                0.0..=1e6,
                Unit::Distance,
                light(LightEdit::Radius),
            )
        };
        match l.kind {
            LightKind::Area | LightKind::Portal => vec![
                row(
                    "tool-width",
                    "Width",
                    l.width,
                    1.,
                    0.0001..=1e6,
                    Unit::Distance,
                    light(LightEdit::Width),
                ),
                row(
                    "tool-height",
                    "Height",
                    l.height,
                    1.,
                    0.0001..=1e6,
                    Unit::Distance,
                    light(LightEdit::Height),
                ),
            ],
            LightKind::Spot => vec![
                row(
                    "tool-cone",
                    "Cone Angle",
                    l.cone_angle,
                    deg,
                    0.01..=180.0,
                    Unit::Angle,
                    light(LightEdit::ConeAngle),
                ),
                row(
                    "tool-edge",
                    "Soft Edge",
                    l.soft_edge,
                    deg,
                    0.0..=180.0,
                    Unit::Angle,
                    light(LightEdit::SoftEdge),
                ),
            ],
            LightKind::Point => vec![radius()],
            LightKind::Cylinder => vec![
                radius(),
                row(
                    "tool-length",
                    "Length",
                    l.length,
                    1.,
                    0.0001..=1e6,
                    Unit::Distance,
                    light(LightEdit::Length),
                ),
            ],
            LightKind::Directional | LightKind::Mesh | LightKind::Photometric | LightKind::Dome => {
                Vec::new()
            }
        }
    }

    /// The channel rows as fields, under the item's name.
    fn channel_fields(&self, f: &mut ToolForm<LiveP>) {
        let rows = self.channel_rows();
        if rows.is_empty() {
            return;
        }
        let name = self
            .editor
            .scene
            .active()
            .map(|i| i.name.clone())
            .unwrap_or_default();
        f.separator().note(name);
        for (i, row) in rows.into_iter().enumerate() {
            let range = row.range.clone();
            let mut field = tf::number(
                row.key,
                row.label,
                row.unit,
                move |p: &LiveP| p.channels.get(i).copied().unwrap_or_default(),
                // Values outside the channel's range clamp to it.
                move |p, v| {
                    if let Some(c) = p.channels.get_mut(i) {
                        *c = v.clamp(*range.start(), *range.end());
                    }
                    Ok(())
                },
            );
            if row.unit == Unit::Angle {
                field = field.suffix("°");
            }
            f.field(field);
        }
    }
}

/// Edge Slide and Vertex Slide's Interpolation: switching resets the
/// amount to 0.
pub(super) fn interpolation(key: &str) -> tf::Field<Parameters> {
    pick(
        key,
        "Interpolation",
        &[(false, "Percent"), (true, "Distance")],
        |p| p.edge_slide_distance,
        |p, d| {
            if p.edge_slide_distance != d {
                p.shift = 0.;
            }
            p.edge_slide_distance = d;
        },
    )
}

/// Edge Bevel / Edge Chamfer properties (EBC-D-01/02). Options the specs
/// haven't measured are shown greyed out at their Reset values.
fn edge_bevel_form(p: &Parameters, f: &mut Form, bevel: bool) {
    use operandi_core::{CornerPattern, EdgeBevelMode, EdgeBevelShape, EdgeEnd};
    f.field(
        pick(
            "edge-bevel-mode",
            "Mode",
            &[
                (EdgeBevelMode::Inset, "Inset"),
                (EdgeBevelMode::Width, "Width"),
            ],
            |p| p.edge_bevel.mode,
            |p, m| p.edge_bevel.mode = m,
        )
        .tip("Width: width across the corner, so the sides move further on sharp corners"),
    );
    let label = match (p.edge_bevel.mode, bevel) {
        (EdgeBevelMode::Width, _) => "Width",
        (_, true) => "Inset",
        (_, false) => "Offset",
    };
    f.field(dist(
        "chamfer_offset",
        label,
        |p| p.chamfer.offset,
        |p, v| p.chamfer.offset = v,
    ));
    if bevel {
        // Mitering, the second handle (2026-11-23): the drag-layer spec
        // gives Edge Bevel's second handle as Mitering; this is the
        // tester's reading of it (an edge added on each side), live
        // instead of the greyed EBC-Q03 placeholder. Labelled Mitering
        // Offset (the tester, 2026-11-24) so it isn't taken for the
        // Corner Pattern's mitering.
        f.field(
            dist(
                "ebc_mitering",
                "Mitering Offset",
                |p| p.edge_bevel.support,
                |p, v| p.edge_bevel.support = v.max(0.),
            )
            .tip("Adds an edge this far beyond the bevel on each side of the selected edges; with Inset 0 it adds them beside the edge itself. 0 adds none"),
        );
    }
    // UIC-92 / UIC-117: rows that don't apply grey in place. Square and
    // Sharp ignore Level, Segments and Depth (EBS-03); Depth only moves the
    // middle points, which Round Level 0 / Segments 1 doesn't have (EBS-02).
    let round = p.edge_bevel.shape == EdgeBevelShape::Round;
    let corner_shape = "Square and Sharp ignore this; they always run through the original edge";
    // Segments (the tester, 2026-11-25, Round Level replaced by Segments;
    // counting fixed by the maintainer, 2026-11-26): the rows of polygons
    // across the strip, 1 to 64, 1 being one flat strip. Edge Chamfer's
    // Segments always counted this way (EBC-D-02), so the two tools and
    // their scripts agree. Round Level L is 2·L rows (EBC-B3-02); scripts
    // can still set `level`.
    f.field(
        whole(
            "chamfer_segments",
            "Segments",
            (1., 64.),
            "Segments requires an integer from 1 to 64",
            |p| p.chamfer.segments as f64,
            |p, n| p.chamfer.segments = n,
        )
        .enabled_if(round, corner_shape)
        .tip("Rows of polygons across the bevel. 1 is one flat strip"),
    );
    let shape = pick(
        "edge-bevel-shape",
        "Edge Shape",
        &[
            (EdgeBevelShape::Round, "Round"),
            (EdgeBevelShape::Square, "Square"),
            (EdgeBevelShape::Sharp, "Sharp"),
        ],
        |p| p.edge_bevel.shape,
        |p, s| p.edge_bevel.shape = s,
    )
    .tip("Square and Sharp: two flat strips meeting at the original edge, at any Level. Sharp is measured the same as Square so far (EBS-05).");
    let depth_why = if !round {
        corner_shape
    } else {
        "Segments 1 is one flat strip; raise Segments to use Depth"
    };
    f.field(shape).field(
        plain(
            "chamfer_depth",
            "Depth (%)",
            |p| p.chamfer.depth,
            |p, v| p.chamfer.depth = v,
        )
        .enabled_if(round && p.chamfer.segments >= 2, depth_why)
        .tip("100% is the full round profile; lower values flatten it toward a straight chamfer"),
    );
    if bevel {
        // A tester's request (2026-11-24), on by default: the measured
        // result (EBC-Q-LS1, EBC-B3-03).
        f.field(
            check(
                "edge_bevel_loop_slide",
                "Loop Slide",
                |p| p.edge_bevel.loop_slide,
                |p, on| p.edge_bevel.loop_slide = on,
            )
            .tip("On: where an unbevelled edge runs between two bevelled edges, the new point slides along it, keeping the edge flow. Off: the point goes an even width from both edges and that edge bends to reach it"),
        )
        // How corners where rounded strips meet are filled (a tester's
        // request, 2026-11-24; EBC-Q-CP1). Named Mitering Style, with No
        // Mitering as Neutral and Balanced as Default, by the tester
        // (2026-11-25).
        .field(
            pick(
                "edge-bevel-corner",
                "Mitering Style",
                // Default first and the default; Patch and Radial are left
                // out until they're finished (the tester, 2026-11-26).
                &[
                    (CornerPattern::Balanced, "Default"),
                    (CornerPattern::NoMitering, "Neutral"),
                    (CornerPattern::Uniform, "Uniform"),
                    (CornerPattern::Pinch, "Pinch"),
                ],
                |p| p.edge_bevel.corner,
                |p, c| p.edge_bevel.corner = c,
            )
            .enabled_if(round && p.chamfer.segments >= 2, "Mitering Style fills corners of rounded strips: use Round with Segments 2 or more")
            .tip("How bevelled strips meet at a corner. Default: a rounded corner, rows wrapping around in an even grid. Neutral: a rounded corner whose lines run straight up into it. Uniform: rows nest as smooth arches under a pointed arch. Pinch: rows meet in V seams under an arch pinched to a spike"),
        );
    }
    if !bevel {
        f.field(
            check(
                "edge_bevel_parallel",
                "Parallel",
                |p| p.edge_bevel.parallel,
                |p, on| p.edge_bevel.parallel = on,
            )
            .tip("Keep each new edge the Offset away from the original edge; off moves the Offset along the side edges"),
        )
        .field(
            check(
                "edge_bevel_stop",
                "Stop At Edge",
                |p| p.edge_bevel.stop_at_edge,
                |p, on| p.edge_bevel.stop_at_edge = on,
            )
            .tip("Side points stop at the far end of their edge instead of running past it"),
        )
        .field(
            check(
                "edge_bevel_weld",
                "Auto Weld",
                |p| p.edge_bevel.auto_weld,
                |p, on| p.edge_bevel.auto_weld = on,
            )
            .tip("Merge new points that land on another point"),
        )
        .field(
            tf::segments(
                "edge-bevel-end",
                "Edge End",
                &[
                    (EdgeEnd::Profile, "Profile"),
                    (EdgeEnd::TriangleFan, "Triangle Fan"),
                ],
                |p: &Parameters| p.edge_bevel.edge_end,
                |p, e| p.edge_bevel.edge_end = e,
            )
            .tip("Profile: an interior end keeps its point. Triangle Fan: it slides along the edge beyond and closes with triangles"),
        );
    }
    f.field(
        check(
            "edge_bevel_use_material",
            "Use Material",
            |p| p.edge_bevel.use_material,
            |p, on| p.edge_bevel.use_material = on,
        )
        .tip("Give the new polygons a material"),
    )
    .field(
        tf::text(
            "edge_bevel_material",
            "Material Name",
            |p: &Parameters| p.edge_bevel.material.clone(),
            |p, t| p.edge_bevel.material = t,
        )
        .enabled_if(p.edge_bevel.use_material, "Turn on Use Material first"),
    );
    // Jason asked for these greyed (an exception to UIC-117, FORM-Q08).
    // Not measured yet (EBC-Q03): shown at their Reset values so the panel
    // reads like the reference, but they can't be changed.
    let why = "Not measured yet; these stay at their Reset values (EBC-Q03)";
    f.field(if bevel {
        greyed_check("ebc_sharp", "Sharp", false, why)
    } else {
        greyed_check("ebc_sharp", "Sharp Corner", false, why)
    });
    if !bevel {
        // Edge Bevel's Mitering is live (above); Edge Chamfer's isn't yet.
        f.field(tf::value("ebc_mitering", "Mitering", "0").enabled_if(false, why));
    }
    f.field(greyed_check("ebc_coplanar", "Coplanar", false, why));
    if !bevel {
        f.field(tf::value("ebc_flatness", "Flatness", "0.00001").enabled_if(false, why))
            .field(greyed_check("ebc_reverse_x", "Reverse X", false, why))
            .field(greyed_check("ebc_reverse_y", "Reverse Y", false, why))
            .field(greyed_check(
                "ebc_boundary_coplanar",
                "Boundary Coplanar",
                true,
                why,
            ));
    }
    f.field(greyed_check(
        "ebc_boundary_extend",
        "Boundary Extend",
        true,
        why,
    ));
}

/// Chamfer Edit (CE-P02 builds Mode Inset; PT-CHE-02/03/07/08/09/11 are
/// unmeasured, so they're left out).
fn chamfer_edit_form(f: &mut Form) {
    use operandi_core::{EdgeBevelMode, EdgeBevelShape};
    let later = "Not measured yet: Chamfer Edit works in Inset mode for now";
    f.field(dist(
        "chamfer_offset",
        "Offset",
        |p| p.chamfer.offset,
        |p, v| p.chamfer.offset = v,
    ))
    .field(
        pick(
            "chamfer-edit-mode",
            "Mode",
            &[
                (EdgeBevelMode::Inset, "Inset"),
                (EdgeBevelMode::Width, "Width"),
            ],
            |_| EdgeBevelMode::Inset,
            |_, _| {},
        )
        .unbuilt_option(1, "Width mode isn't measured yet"),
    )
    .field(
        pick(
            "chamfer-edit-shape",
            "Edge Shape",
            &[
                (EdgeBevelShape::Round, "Round"),
                (EdgeBevelShape::Square, "Square"),
                (EdgeBevelShape::Sharp, "Sharp"),
            ],
            |_| EdgeBevelShape::Round,
            |_, _| {},
        )
        .unbuilt_option(1, "Not measured yet: Chamfer Edit builds Round for now")
        .unbuilt_option(2, "Not measured yet: Chamfer Edit builds Round for now"),
    )
    .unbuilt("Absolute Size", "Off", later)
    .field(whole(
        "chamfer_segments",
        "Segments",
        (1., 64.),
        "Chamfer segments require an integer from 1 to 64",
        |p| p.chamfer.segments as f64,
        |p, n| p.chamfer.segments = n,
    ))
    .field(whole(
        "chamfer_source_segments",
        "Source Segments",
        (1., 64.),
        "Chamfer segments require an integer from 1 to 64",
        |p| p.chamfer.source_segments as f64,
        |p, n| p.chamfer.source_segments = n,
    ))
    .field(plain(
        "chamfer_depth",
        "Depth (%)",
        |p| p.chamfer.depth,
        |p, v| p.chamfer.depth = v,
    ))
    .unbuilt("Swap Editing Side", "off", later)
    .unbuilt("Reverse Direction", "off", later)
    .unbuilt("Reverse Depth", "off", later)
    .unbuilt("Profiles", "(none)", later)
    .note("One strip at a time; Source Segments must be 1.");
}

/// Tweak's options (TWK-03, TWK-12, TWK-15/16). Those the spec lists but
/// that aren't built yet are left out (UIC-117).
fn tweak_form(p: &Parameters, f: &mut Form) {
    use operandi_core::{TweakConnect, TweakMode};
    let modes: Vec<_> = TweakMode::ALL.iter().map(|&m| (m, m.label())).collect();
    // TWK-15: Rigid moves the whole piece, so Range is greyed with it.
    let range_used = p.tweak_options.connect.uses_range();
    let connects: Vec<_> = TweakConnect::ALL.iter().map(|&c| (c, c.label())).collect();
    f.field(pick(
        "tweak-mode",
        "Mode",
        &modes,
        |p| p.tweak_options.mode,
        |p, m| p.tweak_options.mode = m,
    ))
    .field({
        // TWK-12: Range is a distance of zero or more. In a UV view it is
        // in UV units or view pixels (UVT-Q11/Q17), never metres.
        let get = |p: &Parameters| p.tweak_options.range;
        let set = |p: &mut Parameters, v: f64| {
            p.tweak_options.range = if v.is_finite() { v.max(0.) } else { 0. }
        };
        let field = match p.uv.as_ref().map(|_| operandi_core::uv_pipe::RANGE_UNITS) {
            None => dist("tweak_range", "Range", get, set),
            Some(operandi_core::uv_pipe::RangeUnits::Uv) => plain("tweak_range", "Range", get, set),
            Some(operandi_core::uv_pipe::RangeUnits::Pixels) => {
                plain("tweak_range", "Range", get, set).suffix(" px")
            }
        };
        field.enabled_if(range_used, "Rigid Connections moves without a range")
    })
    .field(pick(
        "tweak-connect",
        "Connected Elements",
        &connects,
        |p| p.tweak_options.connect,
        |p, c| p.tweak_options.connect = c,
    ));
    let shapes = falloff_panel::shape_fields(
        "tweak_shape",
        falloff_panel::SHAPE,
        p.tweak_options.shape,
        |p: &Parameters| p.tweak_options.shape,
        |p, s| p.tweak_options.shape = s,
    );
    f.fields_from(
        shapes
            .into_iter()
            .map(|s| s.enabled_if(range_used, "Rigid Connections moves without a range"))
            .collect(),
    );
    let later = "Not built yet (Tweak slice TWK-3)";
    f.unbuilt("Always Raycast", "on", later)
        .unbuilt("Mix Mode", "Multiply", later)
        .unbuilt("Slip UVs", "off", later)
        .note("Hover to see what you'd grab, then drag a vertex, edge or polygon. Range spreads the move to nearby points. Each drag is one undo step; the selection is left alone.");
}

/// Twist Bevel's panel (twist-bevel-2026-11-15 section 2), rows in the
/// spec's order with a half-row between its groups (UIC-107): size, twist,
/// shape, ring spacing, bend. Values are kept in range as they're entered.
/// Settings with no effect grey out in place (UIC-92, UIC-117).
fn twist_bevel_form(p: &Parameters, f: &mut Form) {
    use operandi_core::{TwistAxis, TwistBevelOptions};
    fn tb(p: &mut Parameters, edit: impl FnOnce(&mut TwistBevelOptions)) {
        edit(&mut p.twist_bevel);
        p.twist_bevel = p.twist_bevel.clamped();
    }
    let o = p.twist_bevel;
    f.field(
        dist("shift", "Shift", |p| p.twist_bevel.shift, |p, v| {
            tb(p, |o| o.shift = v)
        })
        .tip("Length of the tube, along its spine; negative pushes inward"),
    )
    .field(
        dist("inset", "Inset", |p| p.twist_bevel.inset, |p, v| {
            tb(p, |o| o.inset = v)
        })
        .tip("How far the cap's outline moves in (taper); negative grows it"),
    )
    .field(
        whole(
            "segments",
            "Segments",
            (1., 256.),
            "Segments must be a whole number from 1 to 256",
            |p| p.twist_bevel.segments as f64,
            |p, n| tb(p, |o| o.segments = n),
        )
        .tip("Rings between the base and the cap; 1 is a plain bevel"),
    )
    .field(
        check("group", "Group Polygons", |p| p.twist_bevel.group_polygons, |p, on| {
            tb(p, |o| o.group_polygons = on)
        })
        .tip("Each connected patch makes one tube; off, each polygon makes its own"),
    )
    .separator()
    .field(
        num("rotation", "Rotation", Unit::Angle, |p| p.twist_bevel.rotation, |p, v| {
            tb(p, |o| o.rotation = v)
        })
        .tip("Total twist at the cap; each ring turns its share"),
    )
    .field(
        pick(
            "rotationAxis",
            "Rotation Axis",
            &TwistAxis::ALL.map(|a| (a, a.label())),
            |p| p.twist_bevel.rotation_axis,
            |p, a| tb(p, |o| o.rotation_axis = a),
        )
        .tip("Normal twists about the tube's own spine; Action Axis about the action axis"),
    )
    .separator()
    .field(
        plain("profile", "Profile", |p| p.twist_bevel.profile, |p, v| {
            tb(p, |o| o.profile = v)
        })
        .enabled_if(o.segments >= 2, "Profile needs 2 or more Segments")
        .tip("Curves the taper along the length: negative pinches a waist, positive bulges (−1 to 1)"),
    )
    .field(
        plain(
            "profileBias",
            "Profile Bias",
            |p| p.twist_bevel.profile_bias,
            |p, v| tb(p, |o| o.profile_bias = v),
        )
        .enabled_if(
            o.profile != 0. && o.segments >= 2,
            "Profile Bias moves the Profile's peak; set a Profile first",
        )
        .tip("Slides the Profile's widest or thinnest point toward the base (negative) or cap (−1 to 1)"),
    )
    .separator()
    .field(
        plain(
            "segmentSpacing",
            "Segment Spacing",
            |p| p.twist_bevel.segment_spacing,
            |p, v| tb(p, |o| o.segment_spacing = v),
        )
        .enabled_if(o.segments >= 2, "Spacing needs 2 or more Segments")
        .tip("Bunches the rings toward the base (negative) or the cap (−2 to 2); the shape stays"),
    )
    .field(
        plain(
            "centerBias",
            "Center Bias",
            |p| p.twist_bevel.center_bias,
            |p, v| tb(p, |o| o.center_bias = v),
        )
        .enabled_if(o.segments >= 2, "Spacing needs 2 or more Segments")
        .tip("Bunches the rings toward the middle (positive) or both ends (−2 to 2)"),
    )
    .separator()
    .field(
        num("bend", "Bend", Unit::Angle, |p| p.twist_bevel.bend, |p, v| {
            tb(p, |o| o.bend = v)
        })
        .tip("How far the tube curls in total (−360° to 360°); 90° leaves the cap facing sideways"),
    )
    .field(
        num(
            "bendDirection",
            "Bend Direction",
            Unit::Angle,
            |p| p.twist_bevel.bend_direction,
            |p, v| tb(p, |o| o.bend_direction = v),
        )
        .tip("Which way the tube curls, as an angle around the normal"),
    )
    // Three opt-ins from the TwistBevel notes (2026-11-25), off by default;
    // UIC-115 checkbox rows, after the spec's own rows.
    .separator()
    .field(
        check(
            "stopAtFold",
            "Stop Before Fold",
            |p| p.twist_bevel.stop_at_fold,
            |p, on| tb(p, |o| o.stop_at_fold = on),
        )
        .tip("A tube whose cap would fold over stops its Inset just short of that, instead of being refused"),
    )
    .field(
        check(
            "evenInterior",
            "Even Interior",
            |p| p.twist_bevel.even_interior,
            |p, on| tb(p, |o| o.even_interior = on),
        )
        .tip("Points inside a grouped cap and along its straight edges move with the Inset, so the cap shrinks evenly"),
    )
    .field(
        check(
            "keepCapUvs",
            "Keep Cap UVs",
            |p| p.twist_bevel.keep_cap_uvs,
            |p, on| tb(p, |o| o.keep_cap_uvs = on),
        )
        .tip("The caps keep their UVs exactly, seams included; only the walls get new UVs"),
    )
    .note("Handles: Shift arrow, Inset and twist ring at the cap; bend arrow and Bend Direction ring at the base. Drag anywhere for Shift; Shift+click starts a new one on the caps. Nothing selected works on every polygon.");
}
