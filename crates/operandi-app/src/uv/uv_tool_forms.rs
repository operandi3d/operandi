//! Tool Properties of the UV tab's live tools (BEHAVIOR-UV-TOOLS §2–4,
//! UVS-1 §1, §1.5, §3): Project, Project from View, UV Unwrap, UV Relax and
//! Unwrap & Relax, in the specs' field order. Fields that don't apply to the
//! current choice are greyed with the reason (UIC-117); options not built
//! yet are left out and listed as unbuilt rows.
//!
//! Field keys are the reference's attribute names where the name map has
//! them (`TOOL uv.unwrap`, `uv.relax`, `uv.pelt` in
//! specifications/command-names/MODO-NAMES-SUBSET.txt), so `tool.attr`
//! scripts port.
use super::*;
use crate::form::Unit;
use crate::live_fields::{check, pick, plain, whole};
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::uv_project::ProjectionKind;
use operandi_core::uv_relax::{RelaxBoundary, RelaxMode, RelaxOptions};
use operandi_core::uv_tools::{
    Axis3, InitialProjection, SeamSource, UnwrapOptions, UnwrapSolver, UvAxis,
};

type Form = ToolForm<Parameters>;

const AXES: [(Axis3, &str); 3] = [(Axis3::X, "X"), (Axis3::Y, "Y"), (Axis3::Z, "Z")];
const UV_AXES: [(UvAxis, &str); 2] = [(UvAxis::U, "U"), (UvAxis::V, "V")];

/// Pointer pixels per Iteration for a drag anywhere (UV-Q6 provisional:
/// right is more).
pub(crate) const PIXELS_PER_ITERATION: f64 = 2.;

/// The UV Map field shared by the UV tools: the mesh's maps, and a new
/// map's name (written when the tool first runs, UVT-01).
fn map_field(
    app: &Operandi,
    key: &str,
    get: fn(&Parameters) -> Option<String>,
    set: fn(&mut Parameters, Option<String>),
) -> tf::Field<Parameters> {
    let mesh = app.editor.scene.active().map(|i| i.mesh.clone());
    let mut names: Vec<String> = mesh
        .as_ref()
        .map(|m| m.uv_maps.keys().cloned().collect())
        .unwrap_or_default();
    let fresh = mesh
        .as_ref()
        .map(|m| operandi_core::uv_maps::free_name(m, "Texture"))
        .unwrap_or_else(|| "Texture".into());
    let current = mesh
        .as_ref()
        .map(|m| operandi_core::uv_tools::target_uv_map(m, None));
    let mut options: Vec<(Option<String>, String)> = vec![(
        None,
        match &current {
            Some(c) if names.contains(c) => format!("{c} (selected)"),
            _ => format!("{fresh} (new)"),
        },
    )];
    names.sort();
    options.extend(names.into_iter().map(|n| (Some(n.clone()), n)));
    if options.iter().all(|(v, _)| v.as_deref() != Some(&fresh)) {
        options.push((Some(fresh.clone()), format!("{fresh} (new)")));
    }
    let refs: Vec<(Option<String>, &str)> = options
        .iter()
        .map(|(v, l)| (v.clone(), l.as_str()))
        .collect();
    pick(key, "UV Map", &refs, get, set).tip("The UV map written; a new one is made when missing")
}

/// Project (UVT §2.3).
pub(crate) fn project_form(app: &Operandi, p: &Parameters, f: &mut Form) {
    let o = &p.uv_project;
    let kinds: Vec<(ProjectionKind, &str)> = ProjectionKind::ALL
        .iter()
        .map(|&k| (k, k.label()))
        .collect();
    f.field(map_field(
        app,
        "uvmap",
        |p| p.uv_project.uv_map.clone(),
        |p, m| p.uv_project.uv_map = m,
    ))
    .field(
        pick(
            "proj",
            "Projection Type",
            &kinds,
            |p| p.uv_project.kind,
            |p, k| p.uv_project.kind = k,
        )
        .tip("Planar, Cylindrical, Spherical, Barycentric or Cubic"),
    )
    .field(
        pick(
            "axis",
            "Axis",
            &AXES,
            |p| p.uv_project.axis,
            |p, a| p.uv_project.axis = a,
        )
        .enabled_if(o.kind.uses_axis(), "Barycentric has no direction"),
    )
    .field(
        pick(
            "mode",
            "Settings",
            &[(false, "Automatic"), (true, "Manual")],
            |p| p.uv_project.manual,
            |p, m| p.uv_project.manual = m,
        )
        .enabled_if(o.kind.uses_frame(), "Barycentric has no shape to size")
        .tip("Automatic fits the selection; Manual uses Size, Center and Rotation"),
    );
    let manual = o.manual && o.kind.uses_frame();
    let why = "Settings: Manual sizes the shape by hand";
    for (key, label, get, set) in [
        (
            "size",
            "Size",
            (|p: &Parameters| p.uv_project.size) as fn(&Parameters) -> glam::DVec3,
            (|p: &mut Parameters, a: usize, v: f64| p.uv_project.size[a] = v.max(1e-6))
                as fn(&mut Parameters, usize, f64),
        ),
        (
            "cen",
            "Center",
            |p: &Parameters| p.uv_project.center,
            |p: &mut Parameters, a: usize, v: f64| p.uv_project.center[a] = v,
        ),
    ] {
        let fields = tf::vector(key, label, Unit::Distance, get, move |p, a, v| {
            set(p, a, v);
            Ok(())
        });
        f.fields_from(
            fields
                .into_iter()
                .map(|x| x.enabled_if(manual, why))
                .collect(),
        );
    }
    let fields = tf::vector(
        "rot",
        "Rotation",
        Unit::Angle,
        |p: &Parameters| p.uv_project.rotation,
        |p, a, v| {
            p.uv_project.rotation[a] = v;
            Ok(())
        },
    );
    f.fields_from(
        fields
            .into_iter()
            .map(|x| x.enabled_if(manual, why))
            .collect(),
    );
    f.field(
        tf::number(
            "seam",
            "Seam Angle",
            Unit::Angle,
            |p: &Parameters| p.uv_project.seam_angle,
            |p, v| {
                p.uv_project.seam_angle = v;
                Ok(())
            },
        )
        .enabled_if(
            o.kind.uses_seam(),
            "Only Cylindrical and Spherical wrap round",
        ),
    );
    // Atlas types (UVT §2.3).
    let atlas = o.kind.is_atlas();
    let atlas2 = o.kind == ProjectionKind::Atlas2;
    let why = "Only the Atlas types make charts";
    f.field(
        plain(
            "gaps",
            "Relative Gap Size",
            |p| p.uv_project.atlas.relative_gap,
            |p, v| p.uv_project.atlas.relative_gap = if v.is_finite() { v.max(0.) } else { 0. },
        )
        .enabled_if(atlas, why),
    )
    .field(
        check(
            "matr",
            "Segment by Material",
            |p| p.uv_project.atlas.segment_by_material,
            |p, on| p.uv_project.atlas.segment_by_material = on,
        )
        .enabled_if(atlas, why),
    )
    .field(
        check(
            "part",
            "Segment by Part",
            |p| p.uv_project.atlas.segment_by_part,
            |p, on| p.uv_project.atlas.segment_by_part = on,
        )
        .enabled_if(atlas, why),
    )
    .field(
        check(
            "orient",
            "Orientation",
            |p| p.uv_project.atlas.orientation || p.uv_project.kind == ProjectionKind::Atlas2,
            |p, on| p.uv_project.atlas.orientation = on,
        )
        .enabled_if(
            o.kind == ProjectionKind::Atlas,
            "Always on for Atlas 2; only the Atlas types make charts",
        )
        .tip("Turn each chart so up in the model runs up V"),
    )
    .field(
        whole(
            "segments",
            "Segments",
            (1., 10_000.),
            "Segments must be a whole number of one or more",
            |p| p.uv_project.atlas.segments as f64,
            |p, n| p.uv_project.atlas.segments = n.max(1),
        )
        .enabled_if(atlas2, "Atlas 2 only: roughly how many charts"),
    )
    .field(
        whole(
            "iterations",
            "Iterations",
            (1., 1000.),
            "Iterations must be a whole number of one or more",
            |p| p.uv_project.atlas.iterations as f64,
            |p, n| p.uv_project.atlas.iterations = n.max(1),
        )
        .enabled_if(atlas2, "Atlas 2 only: more gives less stretch and more charts"),
    )
    .note("Projects the selected polygons (all visible ones with none selected) straight away; changing a field projects again. Drop the tool to keep it.");
}

/// Project from View (UVT-19).
pub(crate) fn view_project_form(app: &Operandi, _p: &Parameters, f: &mut Form) {
    f.field(map_field(
        app,
        "uvmap",
        |p| p.uv_view_project.uv_map.clone(),
        |p, m| p.uv_view_project.uv_map = m,
    ))
    .field(
        check(
            "autoScale",
            "Auto Scale",
            |p| p.uv_view_project.auto_scale,
            |p, on| p.uv_view_project.auto_scale = on,
        )
        .tip("Fit the result into the 0-1 tile"),
    )
    .field(
        check(
            "hidden",
            "Hidden Face",
            |p| p.uv_view_project.hidden_face,
            |p, on| p.uv_view_project.hidden_face = on,
        )
        .tip("Leave out polygons facing away from the view"),
    )
    .note("Click in a 3D view: the selected polygons get UVs as they look from there. Move the view and click again to redo it.");
}

/// UV Unwrap's fields (UVS-1 §1 table), for the Unwrap tool and Unwrap &
/// Relax's first half. `get` / `put` reach the options inside the
/// parameters.
fn unwrap_fields(
    o: &UnwrapOptions,
    f: &mut Form,
    get: fn(&Parameters) -> &UnwrapOptions,
    put: fn(&mut Parameters) -> &mut UnwrapOptions,
    sources: &[(SeamSource, String)],
) {
    let projections = [
        (InitialProjection::Planar, "Planar"),
        (InitialProjection::Cylindrical, "Cylindrical"),
        (InitialProjection::Spherical, "Spherical"),
        (InitialProjection::UseCurrentMap, "Use Current Map"),
        (InitialProjection::GroupNormal, "Group Normal"),
    ];
    f.field(
        whole(
            "iter",
            "Iterations",
            (0., 100_000.),
            "Iterations must be a whole number of zero or more",
            move |p| get(p).iterations as f64,
            move |p, n| put(p).iterations = n,
        )
        .tip("How far the solver runs; 0 shows only the Initial Projection. Drag right for more"),
    )
    .field(
        plain(
            "gaps",
            "Relative Gap Size",
            move |p| get(p).relative_gap,
            move |p, v| put(p).relative_gap = if v.is_finite() { v.max(0.) } else { 0. },
        )
        .tip("Space left between islands and at the tile edge"),
    )
    .field(check(
        "matr",
        "Segment by Material",
        move |p| get(p).segment_by_material,
        move |p, on| put(p).segment_by_material = on,
    ))
    .field(check(
        "part",
        "Segment by Part",
        move |p| get(p).segment_by_part,
        move |p, on| put(p).segment_by_part = on,
    ))
    .field(pick(
        "project",
        "Initial Projection",
        &projections,
        move |p| get(p).initial_projection,
        move |p, v| put(p).initial_projection = v,
    ))
    .field(pick(
        "pinning",
        "Pinning Axis",
        &UV_AXES,
        move |p| get(p).pinning_axis,
        move |p, v| put(p).pinning_axis = v,
    ))
    .field(
        pick(
            "axis",
            "Projection Axis",
            &AXES,
            move |p| get(p).projection_axis,
            move |p, v| put(p).projection_axis = v,
        )
        .enabled_if(
            o.initial_projection.uses_axis(),
            "Applies only to Planar, Cylindrical or Spherical",
        ),
    )
    .field(tf::number(
        "angle",
        "Offset Angle",
        Unit::Angle,
        move |p: &Parameters| get(p).offset_angle,
        move |p, v| {
            if !v.is_finite() {
                return Err("Enter a number".into());
            }
            put(p).offset_angle = v;
            Ok(())
        },
    ))
    .field(check(
        "seal",
        "Seal Holes",
        move |p| get(p).seal_holes,
        move |p, on| put(p).seal_holes = on,
    ))
    .field(check(
        "layout",
        "Update Layout",
        move |p| get(p).update_layout,
        move |p, on| put(p).update_layout = on,
    ))
    .field(pick(
        "mode",
        "Method",
        &[
            (UnwrapSolver::Conformal, "Conformal"),
            (UnwrapSolver::AngleBased, "Angle Based"),
        ],
        move |p| get(p).method,
        move |p, v| put(p).method = v,
    ))
    .unbuilt("Symmetrize", "off", "Comes with UV Symmetry (UV stage 4)");
    let mut seam_options: Vec<(SeamSource, String)> =
        vec![(SeamSource::SelectedEdges, "Selected Edges".into())];
    seam_options.extend(sources.iter().cloned());
    if !seam_options.iter().any(|(v, _)| *v == o.seams) {
        let missing = match &o.seams {
            SeamSource::SeamMap(n) => format!("UV Seam Map: {n}"),
            SeamSource::SelectionSet(n) => format!("Selection Set: {n}"),
            SeamSource::SelectedEdges => "Selected Edges".into(),
        };
        seam_options.push((o.seams.clone(), missing));
    }
    let refs: Vec<(SeamSource, &str)> = seam_options
        .iter()
        .map(|(v, l)| (v.clone(), l.as_str()))
        .collect();
    f.field(
        pick(
            "seam",
            "UV Seam Type",
            &refs,
            move |p| get(p).seams.clone(),
            move |p, v| put(p).seams = v,
        )
        .tip(
            "Where the surface is cut: the selected edges, a UV seam map or an edge selection set",
        ),
    );
}

/// Seam sources besides Selected Edges: the mesh's seam maps, then its
/// edge selection sets.
fn seam_sources(app: &Operandi) -> Vec<(SeamSource, String)> {
    let Some(item) = app.editor.scene.active() else {
        return Vec::new();
    };
    let mesh = &item.mesh;
    operandi_core::uv_seams::seam_maps(mesh)
        .into_iter()
        .map(|n| (SeamSource::SeamMap(n.clone()), format!("UV Seam Map: {n}")))
        .chain(
            operandi_core::selection_sets::set_names(mesh, operandi_core::SetKind::Edge)
                .into_iter()
                .map(|n| {
                    (
                        SeamSource::SelectionSet(n.clone()),
                        format!("Selection Set: {n}"),
                    )
                }),
        )
        .collect()
}

/// UV Unwrap as a live tool (UVS-1 §1).
pub(crate) fn unwrap_form(app: &Operandi, p: &Parameters, f: &mut Form) {
    f.field(map_field(
        app,
        "name",
        |p| p.uv_unwrap.uv_map.clone(),
        |p, m| p.uv_unwrap.uv_map = m,
    ));
    unwrap_fields(
        &p.uv_unwrap,
        f,
        |p| &p.uv_unwrap,
        |p| &mut p.uv_unwrap,
        &seam_sources(app),
    );
    f.note("Unwraps every visible polygon (hide polygons to keep their UVs); the selected edges are the seams. Drag left or right in a view for fewer or more Iterations.");
}

/// UV Relax's fields (UVS-1 §3 table).
fn relax_fields(
    o: &RelaxOptions,
    f: &mut Form,
    get: fn(&Parameters) -> &RelaxOptions,
    put: fn(&mut Parameters) -> &mut RelaxOptions,
    iterations_key: &'static str,
    seal_key: &'static str,
) {
    let modes: Vec<(RelaxMode, &str)> = RelaxMode::ALL.iter().map(|&m| (m, m.label())).collect();
    let bounds: Vec<(RelaxBoundary, &str)> =
        RelaxBoundary::ALL.iter().map(|&b| (b, b.label())).collect();
    let solver = matches!(o.mode, RelaxMode::Conformal | RelaxMode::AngleBased);
    f.field(pick(
        "relaxmode",
        "Mode",
        &modes,
        move |p| get(p).mode,
        move |p, m| put(p).mode = m,
    ))
    .field(
        whole(
            iterations_key,
            "Iterations",
            (0., 100_000.),
            "Iterations must be a whole number of zero or more",
            move |p| get(p).iterations as f64,
            move |p, n| put(p).iterations = n,
        )
        .tip("Amount of relaxing; 0 changes nothing. Drag right for more"),
    )
    .field(
        check(
            "live",
            "Interactive",
            move |p| get(p).interactive,
            move |p, on| put(p).interactive = on,
        )
        .tip("Click UV points in the UV view to pin them (Ctrl/Cmd+click removes one); drag a pin to pull the map"),
    )
    .unbuilt("Pause", "off", "UVR-P06: the map re-solves while pins move")
    .unbuilt("Save Constraints", "off", "UVR-P06: pins last for the tool session")
    .field(
        check(
            seal_key,
            "Seal Holes",
            move |p| get(p).seal_holes,
            move |p, on| put(p).seal_holes = on,
        )
        .enabled_if(solver, "Seal Holes works in Conformal and Angle Based only"),
    )
    .field(
        pick(
            "boundary",
            "Boundary",
            &bounds,
            move |p| get(p).boundary,
            move |p, b| put(p).boundary = b,
        )
        .enabled_if(
            o.mode != RelaxMode::Spring,
            "Spring relaxes borders and inside together",
        )
        .tip("Smooth, Locked, Maintain (slide along the border) or Straighten"),
    )
    .field(
        check(
            "uniform",
            "Uniform Spans",
            move |p| get(p).uniform_spans,
            move |p, on| put(p).uniform_spans = on,
        )
        .enabled_if(
            o.boundary == RelaxBoundary::Straighten && o.mode != RelaxMode::Spring,
            "Only with Boundary Straighten",
        ),
    )
    .field(
        tf::percent(
            "area",
            "Area Weight",
            " %",
            move |p: &Parameters| get(p).area_weight,
            move |p, v| {
                if !(v.is_finite() && (0. ..=100.).contains(&v)) {
                    return Err("Area Weight must be from 0 % to 100 %".into());
                }
                put(p).area_weight = v / 100.;
                Ok(())
            },
        )
        .enabled_if(
            o.mode == RelaxMode::Adaptive,
            "Area Weight works in Adaptive only",
        ),
    )
    .field(
        check(
            "noEdge",
            "Ignore Edge Selection",
            move |p| get(p).ignore_edge_selection,
            move |p, on| put(p).ignore_edge_selection = on,
        )
        .tip("Off: islands holding a selected edge stay still"),
    )
    .unbuilt("UV Symmetry", "Off", "Comes with UV Symmetry (UV stage 4)")
    .unbuilt(
        "Show Transform Handles",
        "off",
        "UV-Q13: what the handle drives is still open",
    );
}

/// UV Relax (UVS-1 §3).
pub(crate) fn relax_form(app: &Operandi, p: &Parameters, f: &mut Form) {
    f.field(map_field(
        app,
        "name",
        |p| p.uv_relax.uv_map.clone(),
        |p, m| p.uv_relax.uv_map = m,
    ));
    relax_fields(
        &p.uv_relax,
        f,
        |p| &p.uv_relax,
        |p| &mut p.uv_relax,
        "iter",
        "seal",
    );
    f.note("Relaxes every visible island of the map. Drag left or right in a view for fewer or more Iterations; a falloff limits where it works.");
}

/// Unwrap & Relax (UVS-1 §1.5): both halves' fields, each with its switch.
pub(crate) fn pelt_form(app: &Operandi, p: &Parameters, f: &mut Form) {
    f.field(map_field(
        app,
        "name",
        |p| p.uv_pelt.unwrap.uv_map.clone(),
        |p, m| p.uv_pelt.unwrap.uv_map = m,
    ))
    .field(check(
        "enableU",
        "Unwrap",
        |p| p.uv_pelt.unwrap_on,
        |p, on| p.uv_pelt.unwrap_on = on,
    ));
    unwrap_fields(
        &p.uv_pelt.unwrap,
        f,
        |p| &p.uv_pelt.unwrap,
        |p| &mut p.uv_pelt.unwrap,
        &seam_sources(app),
    );
    f.separator().field(check(
        "enableR",
        "Relax",
        |p| p.uv_pelt.relax_on,
        |p, on| p.uv_pelt.relax_on = on,
    ));
    relax_fields(
        &p.uv_pelt.relax,
        f,
        |p| &p.uv_pelt.relax,
        |p| &mut p.uv_pelt.relax,
        "relaxiter",
        "relaxSeal",
    );
    f.note("Unwraps, then relaxes the result. Drag left or right in a view for fewer or more relax Iterations.");
}

impl Operandi {
    /// The UV tools' drag anywhere (UV-Q6, provisional): right adds
    /// Iterations, left takes them away, one per `PIXELS_PER_ITERATION`.
    /// `None` when the live tool has no such drag.
    pub(crate) fn uv_iterations_drag(
        &self,
        base: &Parameters,
        tool: Tool,
        dx: f64,
    ) -> Option<Parameters> {
        let step = |n: u32| (n as f64 + dx / PIXELS_PER_ITERATION).round().max(0.) as u32;
        let mut p = base.clone();
        match tool {
            Tool::UvUnwrap => p.uv_unwrap.iterations = step(base.uv_unwrap.iterations),
            Tool::UvRelax => p.uv_relax.iterations = step(base.uv_relax.iterations),
            Tool::UvPelt => p.uv_pelt.relax.iterations = step(base.uv_pelt.relax.iterations),
            _ => return None,
        }
        Some(p)
    }

    /// Project from View (UVT-19): a click in 3D view `r` projects the
    /// selected polygons as seen from it. Returns whether the click was
    /// taken.
    pub(crate) fn uv_view_project_click(&mut self, r: Rect) -> bool {
        let Some(live) = self
            .editor
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::UvViewProject)
        else {
            return false;
        };
        let mut p = live.params.clone();
        p.uv_view_project.view = Some(view_frame(&self.camera, r));
        self.run(Command::Preview(p));
        self.clear_fields();
        true
    }
}

/// The camera's view as Project from View needs it: world → screen with
/// the view's height spanning −1..1 (camera.rs `project`'s maths).
pub(crate) fn view_frame(
    camera: &crate::camera::Camera,
    r: Rect,
) -> operandi_core::uv_project::ViewFrame {
    use glam::{DMat4, DVec4};
    let (eye, right, up, forward) = camera.basis();
    let perspective = camera.view == crate::camera::View::Perspective;
    let k = 2. * camera.lens;
    let row = |v: glam::DVec3, s: f64| DVec4::new(v.x * s, v.y * s, v.z * s, -v.dot(eye) * s);
    let w = if perspective {
        row(forward, 1.)
    } else {
        DVec4::new(0., 0., 0., camera.distance)
    };
    let _ = r;
    let matrix = DMat4::from_cols(row(right, k), row(up, k), row(forward, 1.), w).transpose();
    operandi_core::uv_project::ViewFrame {
        matrix,
        eye: perspective.then_some(eye),
        forward,
    }
}

/// Peeler (UVT-30).
pub(crate) fn peel_form(_p: &Parameters, f: &mut Form) {
    let number = |key: &'static str,
                  label: &'static str,
                  get: fn(&Parameters) -> f64,
                  set: fn(&mut Parameters, f64)| {
        tf::number(key, label, Unit::Scalar, get, move |p, v| {
            if !v.is_finite() {
                return Err("Enter a number".into());
            }
            set(p, v);
            Ok(())
        })
        .suffix("")
    };
    f.field(number("minU", "Min U", |p| p.uv_peel.min.x, |p, v| p.uv_peel.min.x = v))
        .field(number("maxU", "Max U", |p| p.uv_peel.max.x, |p, v| p.uv_peel.max.x = v))
        .field(number("minV", "Min V", |p| p.uv_peel.min.y, |p, v| p.uv_peel.min.y = v))
        .field(number("maxV", "Max V", |p| p.uv_peel.max.y, |p, v| p.uv_peel.max.y = v))
        .field(
            tf::percent(
                "uniformity",
                "Uniformity",
                " %",
                |p: &Parameters| p.uv_peel.uniformity,
                |p, v| {
                    if !(v.is_finite() && (0. ..=100.).contains(&v)) {
                        return Err("Uniformity runs from 0 % to 100 %".into());
                    }
                    p.uv_peel.uniformity = v / 100.;
                    Ok(())
                },
            )
            .tip("0 %: each UV polygon follows its 3D size; 100 %: an even grid"),
        )
        .note("Select the edges to cut at (Edge mode). Works on clean quad strips and patches; elsewhere use Unwrap.");
}

/// Box Transform (UVT-43). Box U/V and Gaps aren't built (UVB-P03).
pub(crate) fn box_form(p: &Parameters, f: &mut Form) {
    let number = |key: &'static str,
                  label: &'static str,
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
    let pivot = p.uv_box.pivot.unwrap_or_default();
    f.field(number("offsetU", "Offset U", Unit::Scalar, |p| p.uv_box.offset.x, |p, v| p.uv_box.offset.x = v).suffix(""))
        .field(number("offsetV", "Offset V", Unit::Scalar, |p| p.uv_box.offset.y, |p, v| p.uv_box.offset.y = v).suffix(""))
        .field(
            number("pivotU", "Pivot U", Unit::Scalar, |p| p.uv_box.pivot.unwrap_or_default().x, |p, v| {
                let mut q = p.uv_box.pivot.unwrap_or_default();
                q.x = v;
                p.uv_box.pivot = Some(q);
            })
            .suffix("")
            .tip(if p.uv_box.pivot.is_none() { "The box's centre until set" } else { "The turn's centre" }),
        )
        .field(
            number("pivotV", "Pivot V", Unit::Scalar, |p| p.uv_box.pivot.unwrap_or_default().y, |p, v| {
                let mut q = p.uv_box.pivot.unwrap_or_default();
                q.y = v;
                p.uv_box.pivot = Some(q);
            })
            .suffix(""),
        )
        .field(number("angle", "Angle", Unit::Angle, |p| p.uv_box.angle, |p, v| p.uv_box.angle = v))
        .field(number("width", "Width", Unit::Scalar, |p| p.uv_box.scale.x, |p, v| p.uv_box.scale.x = if v.abs() < 1e-6 { 1e-6 } else { v }).suffix("").tip("Scale across U (1 keeps the box's width)"))
        .field(number("height", "Height", Unit::Scalar, |p| p.uv_box.scale.y, |p, v| p.uv_box.scale.y = if v.abs() < 1e-6 { 1e-6 } else { v }).suffix("").tip("Scale across V (1 keeps the box's height)"))
        .field(check(
            "showRotation",
            "Show Rotation Handle",
            |p| p.uv_box.show_rotation_handle,
            |p, on| p.uv_box.show_rotation_handle = on,
        ))
        .field(check(
            "tearOff",
            "Tear Off",
            |p| p.uv.as_ref().is_some_and(|s| s.tear_off),
            |p, on| {
                if let Some(s) = p.uv.as_mut() {
                    s.tear_off = on;
                    s.proxy = None;
                }
            },
        ))
        .field(number("snapAngle", "Snap Angle", Unit::Angle, |p| p.uv_box.snap_angle, |p, v| p.uv_box.snap_angle = v.abs().max(0.1)))
        .unbuilt("Box U / Box V", "-", "UVB-P03: Offset moves the box")
        .unbuilt("Gaps", "0", "UVB-P03: not built yet")
        .note("Drag inside the box to move (Ctrl: one axis, Shift: corner to the grid). Drag the round handle to turn about the pivot (Ctrl: Snap Angle steps); drag the pivot cross to move it. Right-drag a side or corner to scale from the opposite one.");
    let _ = pivot;
}
