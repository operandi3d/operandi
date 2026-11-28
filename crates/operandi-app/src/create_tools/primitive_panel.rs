//! Box, Sphere and Cylinder Tool Properties (SP-P01 / CYL-P01 and Cube),
//! declared as data for the shared Tool Properties form.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, Button, ToolFields, ToolForm};
use operandi_core::{FalloffSample, FalloffSpacing};

/// The live Box, Sphere or Cylinder.
pub(super) struct PrimitiveFields;

const BUDGET: &str = "Sides and Segments require positive integers within the preview budget";
const SEGMENTS: &str = "Segments require a positive integer within the preview budget";

fn live_primitive(app: &Operandi) -> Option<&Live> {
    app.editor
        .session
        .live
        .as_ref()
        .filter(|l| l.tool.is_primitive())
}

impl ToolFields for PrimitiveFields {
    type P = Parameters;
    fn params(app: &Operandi) -> Option<Parameters> {
        live_primitive(app).map(|l| l.params.clone())
    }
    fn form(app: &Operandi, p: &Parameters) -> ToolForm<Parameters> {
        let tool = live_primitive(app).map_or(Tool::Box, |l| l.tool);
        let mut f = ToolForm::new();
        f.push(crate::live_fields::live_title(app, tool));
        if tool == Tool::Box {
            box_form(&mut f);
        } else {
            radial_form(&mut f, tool, p);
        }
        falloff_section(&mut f, app, tool);
        // Primitives keep their segment settings between uses; this puts
        // them back to the starting values.
        f.buttons(vec![
            Button::command("Reset Tool Attributes", Command::ResetPrimitive)
                .tip("Restore the starting Sides/Segments; later uses start from them too"),
        ]);
        f.separator();
        crate::live_fields::live_footer(app, &mut f);
        f
    }
    fn publish(app: &mut Operandi, p: Parameters) -> Result<()> {
        app.editor.dispatch(Command::Preview(p))
    }
    fn published(app: &mut Operandi) {
        app.status = "Ready".into();
    }
    fn settled(app: &mut Operandi) -> bool {
        // A refused handle move keeps the confirmed values; with nothing
        // pending, revalidate them so a stale stage error can't trap Drop.
        if app.field_errors.is_empty()
            && let Some(live) = live_primitive(app)
            && live.preview_error.is_some()
        {
            let p = live.params.clone();
            return match app.editor.dispatch(Command::Preview(p)) {
                Ok(()) => true,
                Err(error) => {
                    app.status = error;
                    false
                }
            };
        }
        app.field_errors.is_empty()
    }
    fn parse_error(key: &str, _label: &str, error: String) -> String {
        format!(
            "{}: {error}. Correct the field or Escape to revert.",
            Operandi::field_label(key)
        )
    }
    fn publish_status(key: &str, error: &str) -> String {
        format!(
            "{}: {error}",
            Operandi::field_label(if key.is_empty() { "value" } else { key })
        )
    }
}

fn position(f: &mut ToolForm<Parameters>) {
    f.fields_from(tf::vector(
        "position",
        "Position",
        Unit::Distance,
        |p: &Parameters| p.position,
        |p, a, v| {
            p.position[a] = v;
            Ok(())
        },
    ));
}

fn make_uvs() -> tf::Field<Parameters> {
    tf::check(
        "make_uvs",
        "Make UVs",
        |p: &Parameters| p.make_uvs,
        |p, on| p.make_uvs = on,
    )
    .tip("Add a basic UV layout to the \"Texture\" map (provisional layout)")
    .command(|app, p| app.run(Command::PrimitiveUvs(p.make_uvs)))
}

fn box_form(f: &mut ToolForm<Parameters>) {
    position(f);
    f.fields_from(tf::vector(
        "size",
        "Size",
        Unit::Distance,
        |p: &Parameters| p.size,
        |p, a, v| {
            p.size[a] = v;
            Ok(())
        },
    ));
    f.fields_from(
        tf::vector(
            "segments",
            "Segments",
            Unit::Count,
            |p: &Parameters| {
                DVec3::new(
                    p.segments[0] as f64,
                    p.segments[1] as f64,
                    p.segments[2] as f64,
                )
            },
            |p, a, v| {
                if v < 1. || v.fract() != 0. || v > u32::MAX as f64 {
                    return Err(SEGMENTS.into());
                }
                p.segments[a] = v as u32;
                Ok(())
            },
        )
        .into_iter()
        .map(|f| f.suffix(""))
        .collect(),
    );
    f.separator();
    let rounding = "Rounding is unavailable in this build";
    f.unbuilt("Radius", "0 m", rounding)
        .unbuilt("Radius Segments", "3", rounding)
        .unbuilt("Sharp", "Off", "Sharp rounding support is unavailable")
        .unbuilt("Bulge", "0", "Bulge is unavailable")
        .unbuilt("Axis", "X / Y / Z", "Rounding axis choices are unavailable")
        .field(make_uvs())
        .unbuilt(
            "Distortion Compensation",
            "Off",
            "UV compensation is unavailable",
        )
        .unbuilt("Gaps", "0%", "UV gaps are unavailable")
        .separator()
        .field(tf::choice(
            "box_handle_mode",
            "Handle Mode",
            &[
                (BoxHandleMode::FromCenter, "Resize from Center"),
                (BoxHandleMode::Resize, "Resize"),
                (BoxHandleMode::MoveAll, "Move All"),
            ],
            |p: &Parameters| p.box_handle_mode,
            |p, m| p.box_handle_mode = m,
        ))
        .unbuilt(
            "Show Corner",
            "Off",
            "Corner handles and constraints are unavailable",
        )
        .separator();
    for (key, label, max) in [("min", "Min", false), ("max", "Max", true)] {
        f.fields_from(tf::vector(
            key,
            label,
            Unit::Distance,
            move |p: &Parameters| {
                if max {
                    p.position + p.size * 0.5
                } else {
                    p.position - p.size * 0.5
                }
            },
            move |p, a, v| {
                let bounds = BoxBounds::new(p.position, p.size)?;
                let bounds = bounds.edit_side(a, max, v, BoxHandleMode::Resize)?;
                p.position = bounds.position();
                p.size = bounds.size();
                Ok(())
            },
        ));
    }
    f.wide(Button::command("Apply", Command::Apply).enabled_if(
        false,
        "Apply is unavailable while Cube is interactive; edits replace the live preview",
    ));
}

fn radial_form(f: &mut ToolForm<Parameters>, tool: Tool, p: &Parameters) {
    let Some(radial) = p.radial else {
        return;
    };
    let sphere = tool == Tool::Sphere;
    position(f);
    f.fields_from(tf::vector(
        "radius",
        "Radius",
        Unit::Distance,
        |p: &Parameters| p.radial.map_or(DVec3::ZERO, |r| r.radius),
        |p, a, v| {
            if let Some(r) = p.radial.as_mut() {
                r.radius[a] = v;
            }
            Ok(())
        },
    ));
    f.separator()
        .field(tf::count(
            "radial_sides",
            "Sides",
            (1., u32::MAX as f64),
            BUDGET,
            |p: &Parameters| p.radial.map_or(0., |r| r.sides as f64),
            |p, n| {
                if let Some(r) = p.radial.as_mut() {
                    r.sides = n;
                }
            },
        ))
        .field(tf::count(
            "radial_segments",
            "Segments",
            (1., u32::MAX as f64),
            BUDGET,
            |p: &Parameters| p.radial.map_or(0., |r| r.segments as f64),
            |p, n| {
                if let Some(r) = p.radial.as_mut() {
                    r.segments = n;
                }
            },
        ));
    if sphere {
        // PRIM-S-02: Globe, or Tessellated (an icosahedron split Level
        // times). Quad Ball isn't built.
        f.separator()
            .field(
                tf::choice(
                    "sphere_mode",
                    "Sphere Mode",
                    &[(0, "Globe"), (1, "Tessellated"), (2, "Quad Ball")],
                    |p: &Parameters| usize::from(p.sphere_level.is_some()),
                    |p, mode| {
                        p.sphere_level = match mode {
                            0 => None,
                            _ => p.sphere_level.or(Some(0)),
                        }
                    },
                )
                .greyed_option(2, "Quad Ball isn't available yet"),
            )
            .field(
                tf::count(
                    "sphere_level",
                    "Subdivision Level",
                    (0., 6.),
                    "Subdivision Level requires an integer from 0 to 6",
                    |p: &Parameters| p.sphere_level.unwrap_or(0) as f64,
                    |p, n| p.sphere_level = Some(n),
                )
                .enabled_if(p.sphere_level.is_some(), "Unavailable in Globe mode"),
            );
    }
    f.field(tf::fixed(
        "Polygon Type",
        "Face",
        "Subdivision creation is outside this slice",
    ));
    if sphere {
        f.separator();
    }
    if tool == Tool::Cylinder {
        // PRIM-Y-01: each end can be None, Single, Quads or Radial.
        let options = CylinderCap::ALL.map(|cap| (cap, cap.label()));
        // Labelled like the Cone's (GitHub #18); the keys stay as they were.
        for (end, key, label) in [(1, "Cap Top", "Top Cap"), (0, "Cap Bottom", "Bottom Cap")] {
            f.field(tf::choice(
                key,
                label,
                &options,
                move |p: &Parameters| p.cylinder_caps[end],
                move |p, cap| p.cylinder_caps[end] = cap,
            ));
        }
        // No inert Bulge row: absent from both tested runtime panels and
        // explicitly excluded by the requested slice.
        f.unbuilt(
            "Edge Weight",
            "100%",
            "Edge Weight is unavailable for Face polygons",
        )
        .separator();
    }
    // Sphere spec: Axis is a segmented X/Y/Z choice.
    f.field(tf::axis(
        |p: &Parameters| p.radial_axis,
        |p, a| p.radial_axis = a,
    ))
    .field(make_uvs())
    .unbuilt(
        "Distortion Compensation",
        "Off",
        "UV compensation is outside this slice",
    )
    .unbuilt("Gaps", "0%", "Generated UV atlas unavailable")
    .separator()
    .field(tf::fixed(
        "Handle Mode",
        if sphere {
            "Resize from Center"
        } else {
            "Resize"
        },
        "Explicit side handles use the approved bounded policy; other modes are unavailable",
    ))
    .unbuilt("Show Corner", "Off", "Corner handles aren't available yet")
    .separator();
    for (key, label, max) in [("min", "Min", false), ("max", "Max", true)] {
        f.fields_from(tf::vector(
            key,
            label,
            Unit::Distance,
            move |p: &Parameters| {
                let r = p.radial.map_or(DVec3::ZERO, |r| r.radius);
                if max { p.position + r } else { p.position - r }
            },
            move |p, a, v| {
                let Some(r) = p.radial.as_mut() else {
                    return Ok(());
                };
                // Numeric bounds hold the opposite bound fixed, regardless
                // of any interactive Handle Mode.
                let mut min = p.position - r.radius;
                let mut max_b = p.position + r.radius;
                if max {
                    max_b[a] = v;
                } else {
                    min[a] = v;
                }
                if min[a] > max_b[a] {
                    return Err(
                        "Crossed bounds are unavailable; the valid preview is retained".into(),
                    );
                }
                p.position = (min + max_b) * 0.5;
                r.radius = (max_b - min) * 0.5;
                Ok(())
            },
        ));
    }
    f.separator()
        .wide(Button::command("Apply", Command::Apply).enabled_if(
            false,
            "Active Apply is disabled under the preserved Operandi lifecycle; live numeric edits replace this preview",
        ))
        .note(if radial.radius.min_element() == 0. {
            "Flat draft · drag an axis handle for depth, or enter a positive Radius. Drop discards a flat draft."
        } else if tool == Tool::Cylinder {
            "Radius Y = half height. Numeric edits create/update one solid."
        } else {
            "Radius values are semiaxes. Numeric edits create/update one solid."
        });
}

/// Polygon Type for the numeric primitives: Face, Subdivs or Catmull-Clark
/// (`cc` is how the panel spells the last).
pub(super) fn polygon_type<P>(
    key: &str,
    cc: &str,
    get: impl Fn(&P) -> PolygonRepresentation + 'static,
    set: impl Fn(&mut P, PolygonRepresentation) + 'static,
) -> tf::Field<P> {
    tf::choice(
        key,
        "Polygon Type",
        &[
            (PolygonRepresentation::Face, "Face"),
            (PolygonRepresentation::Subdivs, "Subdivs"),
            (PolygonRepresentation::CatmullClark, cc),
        ],
        get,
        set,
    )
    .otherwise("Unsupported Polyline")
}

/// The UV map a numeric primitive writes: none (the "Texture" map) or one
/// of the active mesh's maps.
pub(super) fn uv_map<P>(
    app: &Operandi,
    key: &str,
    get: impl Fn(&P) -> Option<String> + 'static,
    set: impl Fn(&mut P, Option<String>) + 'static,
) -> tf::Field<P> {
    let names: Vec<String> = app
        .editor
        .scene
        .active()
        .map(|item| item.mesh.uv_maps.keys().cloned().collect())
        .unwrap_or_default();
    let mut options: Vec<(Option<String>, &str)> = vec![(None, "Texture (no map selected)")];
    options.extend(names.iter().map(|n| (Some(n.clone()), n.as_str())));
    tf::choice(key, "UV Map", &options, get, set)
}

/// A whole-number field of zero or more.
pub(super) fn whole<P>(
    key: &str,
    label: &str,
    get: impl Fn(&P) -> u32 + 'static,
    set: impl Fn(&mut P, u32) + 'static,
) -> tf::Field<P> {
    tf::count(
        key,
        label,
        (0., u32::MAX as f64),
        "Enter a nonnegative integer",
        move |p| get(p) as f64,
        set,
    )
}

/// A plain number written straight into the parameters.
pub(super) fn plain<P>(
    key: &str,
    label: &str,
    unit: Unit,
    get: impl Fn(&P) -> f64 + 'static,
    set: impl Fn(&mut P, f64) + 'static,
) -> tf::Field<P> {
    tf::number(key, label, unit, get, move |p, v| {
        set(p, v);
        Ok(())
    })
}

/// A vector written straight into the parameters, one axis at a time.
pub(super) fn plain_vector<P>(
    key: &str,
    label: &str,
    get: impl Fn(&P) -> DVec3 + 'static,
    set: impl Fn(&mut P, usize, f64) + 'static,
) -> Vec<tf::Field<P>> {
    tf::vector(key, label, Unit::Distance, get, move |p, a, v| {
        set(p, a, v);
        Ok(())
    })
}

/// The Falloff section (#15, UIC-105/106: folded until opened): what the
/// global falloff's weight does to the shape. The Falloff menu and the
/// falloff's handles stay usable while the tool is live, so the falloff
/// can be turned on, switched and moved without dropping the shape.
fn falloff_section(f: &mut ToolForm<Parameters>, app: &Operandi, tool: Tool) {
    let mut extra = Vec::new();
    if tool == Tool::Box {
        // The Cube has no Axis of its own; the falloff shapes it along this.
        extra.push(
            tf::axis(|p: &Parameters| p.radial_axis, |p, a| p.radial_axis = a)
                .tip("The axis the falloff shapes the cube along"),
        );
    }
    let fit = Button::run("Fit Falloff to Shape", |app: &mut Operandi| {
        let fitted = app.editor.primitive_fit_falloff();
        match fitted.and_then(|f| app.editor.dispatch(Command::SpatialFalloff(Some(f)))) {
            Ok(()) => app.status = "Linear falloff fitted to the shape".into(),
            Err(e) => app.status = e,
        }
    })
    .tip("A Linear falloff from the shape's top (full) to its base (none)");
    shape_falloff_section(
        f,
        app,
        |p: &Parameters| p.shape_falloff,
        |p| Some(&mut p.shape_falloff),
        (tool == Tool::Sphere).then_some("Moving a sphere's rings would change the sphere"),
        extra,
        Some(fit),
    );
}

/// The Falloff section for any primitive's form (#15; Cone, Capsule, Torus
/// and Ellipsoid #35). `get` and `put` reach the form's `ShapeFalloff`;
/// `no_spacing` greys Follow Falloff spacing with its reason for shapes
/// whose rings can't move; `extra` rows and the `fit` button go last.
pub(crate) fn shape_falloff_section<P: 'static>(
    f: &mut ToolForm<P>,
    app: &Operandi,
    get: fn(&P) -> operandi_core::ShapeFalloff,
    put: fn(&mut P) -> Option<&mut operandi_core::ShapeFalloff>,
    no_spacing: Option<&'static str>,
    extra: Vec<tf::Field<P>>,
    fit: Option<Button<P>>,
) {
    let e = &app.editor;
    // UIC-117: greyed with the reason, never hidden.
    let why = if e.spatial_falloff.is_none() {
        Some("Turn on a Linear, Radial, Cylinder or Noise falloff from the Falloff menu".to_owned())
    } else {
        e.primitive_falloff_note()
    };
    let on = why.is_none();
    let why = why.unwrap_or_default();
    let mut items: Vec<tf::Field<P>> = vec![
        tf::percent(
            "falloff_scale",
            "Scale",
            "%",
            move |p: &P| get(p).scale,
            move |p, v| {
                if !v.is_finite() || v < 0. {
                    return Err("Scale must be 0% or more".into());
                }
                put(p).ok_or("No falloff values here")?.scale = v / 100.;
                Ok(())
            },
        )
        .tip("The rings' size where the falloff is full (starts at 50%); 100% leaves them as they are"),
    ];
    items.extend(tf::vector(
        "falloff_offset",
        "Offset",
        Unit::Distance,
        move |p: &P| get(p).offset,
        move |p, a, v| {
            put(p).ok_or("No falloff values here")?.offset[a] = v;
            Ok(())
        },
    ));
    items.push(
        tf::number(
            "falloff_twist",
            "Twist",
            Unit::Angle,
            move |p: &P| get(p).twist,
            move |p, v| {
                put(p).ok_or("No falloff values here")?.twist = v;
                Ok(())
            },
        )
        .tip("How far the rings turn about the axis where the falloff is full"),
    );
    items.push(
        tf::choice(
            "falloff_sample",
            "Sample At",
            &FalloffSample::ALL.map(|s| (s, s.label())),
            move |p: &P| get(p).sample,
            move |p, s| {
                if let Some(f) = put(p) {
                    f.sample = s;
                }
            },
        )
        .tip("Spine: one weight per ring, so rings stay round. Points: every point's own weight"),
    );
    let spacing = tf::choice(
        "falloff_spacing",
        "Spacing",
        &FalloffSpacing::ALL.map(|s| (s, s.label())),
        move |p: &P| get(p).spacing,
        move |p, s| {
            if let Some(f) = put(p) {
                f.spacing = s;
            }
        },
    )
    .tip("Follow Falloff packs the Segments where the falloff changes fastest");
    items.push(match no_spacing {
        Some(reason) => spacing.enabled_if(false, reason),
        None => spacing,
    });
    items.extend(extra);
    let mut section: Vec<tf::Item<P>> = items
        .into_iter()
        .map(|field| {
            tf::Item::Field(Box::new(if on {
                field
            } else {
                field.enabled_if(false, why.clone())
            }))
        })
        .collect();
    if let Some(fit) = fit {
        section.push(tf::Item::Wide(fit));
    }
    f.section("Falloff", true, section);
}
