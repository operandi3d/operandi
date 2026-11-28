//! The Mesh Stack tab (Mesh Stack v1, MO-10..28): the selected mesh's
//! operation stack, top = last evaluated, with its Base Mesh at the bottom,
//! and the selected op's Properties under it. Every change goes through a
//! command (mesh_stack_editor.rs), so buttons, menus and scripts agree.
//!
//! Provisional for v1: the list shows only the selected mesh (Single item
//! view, MO-28); Add Operator is a menu, not the preset browser (MO-15); no
//! Ghosted/Hide/Filter header buttons yet (MO-25), no Defer/Pause (MO-29).
use super::*;
use crate::tree_list::{
    Cell, Column, DropAt, Heading, RowIcon, Selected, TreeEvent, TreeHooks, TreeRow, TreeSpec,
};
use crate::unavailable::Gate;
use operandi_core::mesh_stack::{OpKind, OpStatus, op_defs};
use operandi_core::mesh_stack_editor::{IndexEdit, OpRef, OpRow, SelOpEdit};
use operandi_core::meshop_selection::{Blend, SelKind, SelOp};
use operandi_core::meshop_types::FieldKind;

/// The Properties panel's one category while an op is selected.
pub(crate) const MESH_OP_TAB: &str = "Mesh Op";

/// A row of the Mesh Operations list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum OpKey {
    Op(u64, u64),
    Base(u64),
}

/// The ghost of the full result in Edit Base Mesh (MO-26/27). Provisional
/// fixed tint; the spec makes it a preference.
pub(crate) fn ghost_color() -> Color32 {
    Color32::from_rgba_unmultiplied(110, 190, 255, 110)
}

fn spec() -> TreeSpec {
    TreeSpec {
        id: "mesh-stack-tree",
        columns: vec![
            Column::state(
                Heading::Eye,
                "On or off · off passes its input straight through",
            ),
            Column::name(),
        ],
        own_search: false,
        last_fills: false,
    }
}

/// Choices for an item link: "(none)", the other meshes, and the linked
/// id itself when it names no mesh any more (SAFE-08 "missing input").
fn item_options(meshes: &[(u64, String)], current: u64) -> Vec<(u64, &str)> {
    let mut out: Vec<(u64, &str)> = vec![(0, "(none)")];
    out.extend(meshes.iter().map(|(id, n)| (*id, n.as_str())));
    if current != 0 && !meshes.iter().any(|(id, _)| *id == current) {
        out.push((current, "(missing item)"));
    }
    out
}

/// A Minimum and Maximum pair; a Maximum below the Minimum is no limit.
fn min_max(
    ui: &mut Ui,
    min: &mut f64,
    max: &mut f64,
    dmin: f64,
    dmax: f64,
    unit: crate::form::Unit,
    what: &str,
) {
    use crate::form::rows;
    rows::number_d(
        ui,
        "Minimum",
        min,
        dmin,
        unit,
        -1e9..=1e9,
        &format!("Smallest {what}"),
    );
    rows::number_d(
        ui,
        "Maximum",
        max,
        dmax,
        unit,
        -1e9..=1e9,
        &format!("Largest {what}; below Minimum means no limit"),
    );
}
/// Three rows for a direction, "Direction X" then "Y" and "Z" (UIC-114).
fn direction(ui: &mut Ui, label: &str, x: &mut f64, y: &mut f64, z: &mut f64, d: (f64, f64, f64)) {
    use crate::form::{Unit, rows};
    let first = format!("{label} X");
    for (l, v, dv) in [(first.as_str(), x, d.0), ("Y", y, d.1), ("Z", z, d.2)] {
        // #21: "Y" and "Z" repeat under Position and Normal; each row
        // keys on its whole name so their fields stay apart.
        let key = format!("{label} {}", l.trim_start_matches(label).trim());
        rows::number_keyed_d(
            ui,
            &key,
            l,
            v,
            dv,
            Unit::Scalar,
            -1e9..=1e9,
            "A direction; only its way matters",
        );
    }
}

/// Select by Falloff's rows (TB-01..05): Threshold and the op's own
/// falloff copy, with Use Active Falloff and Clear Falloff (UIC-14 greyed
/// with a reason when they can't act).
fn selop_falloff_rows(
    ui: &mut Ui,
    threshold: &mut f64,
    falloff: &mut Option<operandi_core::ShapeWeights>,
    active: Option<&operandi_core::ShapeWeights>,
) {
    use crate::form::{Unit, rows};
    use crate::unavailable::Gate as _;
    let mut percent = *threshold * 100.;
    rows::number_d(
        ui,
        "Threshold",
        &mut percent,
        50.,
        Unit::Percent,
        0.0..=100.0,
        "Picks what the falloff weighs at least this much (at each element's centre)",
    );
    *threshold = percent / 100.;
    let shown = falloff.as_ref().map_or_else(
        || "None".to_owned(),
        |w| {
            let mut names = vec![w.first.label()];
            names.extend(w.added.iter().map(|a| a.0.label()));
            names.join(" + ")
        },
    );
    rows::value(ui, "Falloff", &shown);
    ui.gated_ui(
        crate::unavailable::unless(
            active.is_some(),
            "Turn on a Linear, Radial, Cylinder or Noise falloff from the Falloff menu first",
        ),
        |ui| {
            if rows::button(ui, "Use Active Falloff")
                .on_hover_text("Copy the falloff from the Falloff menu into this selection")
                .clicked()
            {
                *falloff = active.cloned();
            }
        },
    );
    ui.gated_ui(
        crate::unavailable::unless(falloff.is_some(), "This selection has no falloff"),
        |ui| {
            if rows::button(ui, "Clear Falloff")
                .on_hover_text("Remove the falloff; the selection then picks nothing")
                .clicked()
            {
                *falloff = None;
            }
        },
    );
}

/// Select Conditional Loop's rows (TB-10..20), in the reference's order.
fn conditional_loop_rows(ui: &mut Ui, r: &mut operandi_core::meshop_conditional_loop::LoopRules) {
    use crate::form::{Unit, rows};
    use operandi_core::meshop_conditional_loop::LoopMethod;
    rows::check_d(
        ui,
        "Loop Both Directions",
        &mut r.loop_both_dirs,
        true,
        "Grow from both ends of each edge",
    );
    rows::check_d(
        ui,
        "Start Direction",
        &mut r.direction_ab,
        true,
        "With one direction: on grows from the edge's first point, off from its second",
    );
    rows::choice_d(
        ui,
        "Loop Method",
        &mut r.method,
        &LoopMethod::Angle,
        &LoopMethod::ALL,
        "Middle: straight through 4-edge points only. Angle, Dot Product: the straightest edge",
    );
    rows::number_d(
        ui,
        "Max Loop Count",
        &mut r.cap_loop_count,
        0,
        Unit::Count,
        0..=u32::MAX,
        "Edges each way, counting the start edge; 0 = no limit",
    );
    rows::check_d(
        ui,
        "Stop At Selection",
        &mut r.stop_at_selection,
        true,
        "Stop where selected edges meet",
    );
    rows::number_d(
        ui,
        "Selection Count",
        &mut r.stop_at_selection_count,
        2,
        Unit::Count,
        1..=u32::MAX,
        "How many selected edges must meet",
    );
    rows::check_d(
        ui,
        "Shared Polygon",
        &mut r.stop_at_shared_polygon,
        true,
        "Stop before an edge of the same polygon",
    );
    rows::check_d(
        ui,
        "Sharp Angles",
        &mut r.stop_at_sharp_angles,
        true,
        "Stop before a turn of 90° or more",
    );
    rows::check_d(
        ui,
        "Stop At Selection Set",
        &mut r.stop_at_selection_set,
        true,
        "Stop where edges of a selection set meet",
    );
    rows::text_d(
        ui,
        "Set Name",
        &mut r.stop_at_set_name,
        "",
        "The edge selection set; empty never stops",
    );
    rows::number_d(
        ui,
        "Set Count",
        &mut r.stop_at_set_count,
        2,
        Unit::Count,
        1..=u32::MAX,
        "How many set edges must meet",
    );
    rows::check_d(
        ui,
        "Stop At Cut Map",
        &mut r.stop_at_seam_map,
        true,
        "Stop where seam edges meet",
    );
    rows::text_d(
        ui,
        "Cut Map Name",
        &mut r.stop_at_seam_name,
        "UV Seam",
        "The UV seam map",
    );
    rows::number_d(
        ui,
        "Seam Count",
        &mut r.stop_at_seam_count,
        2,
        Unit::Count,
        1..=u32::MAX,
        "How many seam edges must meet",
    );
    rows::note(ui, "Grows each edge selected so far into an edge loop.");
}

/// Three rows for a position, "<label> X" then "Y" and "Z", each keyed on
/// its whole name (#21), defaulting to the origin.
fn position_rows(
    ui: &mut Ui,
    label: &str,
    x: &mut f64,
    y: &mut f64,
    z: &mut f64,
    unit: crate::form::Unit,
    tip: &str,
) {
    let first = format!("{label} X");
    for (axis, l, v) in [("X", first.as_str(), x), ("Y", "Y", y), ("Z", "Z", z)] {
        let key = format!("{label} {axis}");
        crate::form::rows::number_keyed_d(ui, &key, l, v, 0., unit, -1e9..=1e9, tip);
    }
}

/// The op's selection ops (MO-50..53) under its Select by Index: each
/// with its eye, Invert, Blend Mode and own values, a Remove button, and
/// an Add Selection menu. Returns the one change made this frame.
fn selection_ops(
    ui: &mut Ui,
    op: &operandi_core::mesh_stack::MeshOp,
    below: &[operandi_core::mesh_stack::MeshOp],
    active_falloff: Option<&operandi_core::ShapeWeights>,
) -> (Option<SelOpEdit>, Option<egui::Rect>) {
    use crate::form::{Unit, rows};
    // #19: what this op's selection numbers (polygons, edges or points).
    let what = op
        .kind
        .def()
        .map_or("polygons", |d| d.selection_element().plural());
    let count = op.selops.len();
    let mut edit = None;
    for (n, s) in op.selops.iter().enumerate() {
        let mut next = s.clone();
        let mut remove = false;
        let mut moved = None;
        ui.push_id(("selop", n), |ui| {
            rows::value(ui, &format!("{}. {}", n + 1, s.kind.label()), "");
            rows::check_d(
                ui,
                "Enable",
                &mut next.enable,
                true,
                "Off: as if it weren't there",
            );
            rows::check_d(
                ui,
                "Invert",
                &mut next.invert,
                false,
                "Pick what it doesn't",
            );
            let blends: Vec<(Blend, &str)> = Blend::ALL.iter().map(|b| (*b, b.label())).collect();
            rows::choice_d(
                ui,
                "Blend Mode",
                &mut next.blend,
                &Blend::Override,
                &blends,
                "How it combines with the selection ops below it",
            );
            match &mut next.kind {
                SelKind::Index { index } => {
                    let tip = format!("Numbers of the {what} it picks, like 0-9,12");
                    rows::text_d(ui, "Index", index, "", &tip);
                }
                SelKind::Range { start, end, step } => {
                    rows::number_d(
                        ui,
                        "Start",
                        start,
                        0,
                        Unit::Count,
                        0..=u32::MAX,
                        "The first number",
                    );
                    rows::number_d(
                        ui,
                        "End",
                        end,
                        0,
                        Unit::Count,
                        0..=u32::MAX,
                        "The last number; 0 = the last one",
                    );
                    rows::number_d(
                        ui,
                        "Step",
                        step,
                        1,
                        Unit::Count,
                        1..=u32::MAX,
                        "Every nth one",
                    );
                }
                SelKind::Pattern { pattern } => {
                    rows::text_d(
                        ui,
                        "Pattern",
                        pattern,
                        "1,0",
                        "Repeating 1/0 list: 1,0 = every other",
                    );
                }
                SelKind::Previous { source, name } => {
                    let mut sources: Vec<(u64, String)> = vec![(0, "(none)".into())];
                    // Numbered like the rows, so two of a kind differ.
                    let labels = operandi_core::mesh_stack::op_labels(below);
                    sources.extend(below.iter().zip(labels).map(|(o, l)| (o.id, l)));
                    let options: Vec<(u64, &str)> =
                        sources.iter().map(|(id, l)| (*id, l.as_str())).collect();
                    rows::choice_d(
                        ui,
                        "Source Item",
                        source,
                        &0,
                        &options,
                        "An operation lower in this stack",
                    );
                    // PG-03: show labels, store the short ids.
                    let groups = below
                        .iter()
                        .find(|o| o.id == *source)
                        .and_then(|o| o.kind.def())
                        .map(|d| d.groups())
                        .unwrap_or_default();
                    let mut names: Vec<(String, String)> = vec![(String::new(), "(none)".into())];
                    names.extend(groups);
                    if !names.iter().any(|(id, _)| id == name) {
                        names.push((name.clone(), name.clone()));
                    }
                    let options: Vec<(String, &str)> = names
                        .iter()
                        .map(|(id, l)| (id.clone(), l.as_str()))
                        .collect();
                    rows::choice_d(
                        ui,
                        "Name",
                        name,
                        &String::new(),
                        &options,
                        "The group it picks",
                    );
                }
                SelKind::SelectionSet { name } => {
                    rows::text_d(
                        ui,
                        "Selection Set",
                        name,
                        "",
                        "A selection set on the mesh below, of this kind or of polygons",
                    );
                }
                SelKind::Random { seed, probability } => {
                    rows::number_d(
                        ui,
                        "Seed",
                        seed,
                        0,
                        Unit::Count,
                        0..=u32::MAX,
                        "Random number generator seed",
                    );
                    rows::number_d(
                        ui,
                        "Probability",
                        probability,
                        50.0,
                        Unit::Percent,
                        0.0..=100.0,
                        "The chance each one is picked",
                    );
                }
                // #19: rule ops, answered again on every evaluation.
                SelKind::EdgeAngle {
                    min,
                    max,
                    convex,
                    concave,
                } => {
                    min_max(ui, min, max, 30., 180., Unit::Angle, "angle between the two polygons (0° = flat)");
                    rows::check_d(ui, "Convex", convex, true, "Outward bends");
                    rows::check_d(ui, "Concave", concave, true, "Inward bends");
                }
                SelKind::Border {} => {
                    rows::note(ui, "Edges with a polygon on one side only.");
                }
                SelKind::VertexCount { min, max } => {
                    rows::number_d(ui, "Minimum", min, 3, Unit::Count, 0..=u32::MAX, "Fewest corners");
                    rows::number_d(
                        ui,
                        "Maximum",
                        max,
                        3,
                        Unit::Count,
                        0..=u32::MAX,
                        "Most corners; below Minimum means no limit",
                    );
                }
                SelKind::Area { min, max } => {
                    min_max(ui, min, max, 0., 1., Unit::Distance, "side of a square of the polygon's area");
                }
                SelKind::EdgeLength { min, max } => {
                    min_max(ui, min, max, 0., 1., Unit::Distance, "edge length");
                }
                SelKind::Normal { x, y, z, min, max } => {
                    direction(ui, "Direction", x, y, z, (0., 1., 0.));
                    min_max(ui, min, max, 0., 45., Unit::Angle, "angle from the direction");
                }
                SelKind::Plane {
                    x,
                    y,
                    z,
                    nx,
                    ny,
                    nz,
                    negative,
                    include_zero,
                } => {
                    position_rows(ui, "Position", x, y, z, Unit::Distance, "A point on the plane");
                    direction(ui, "Normal", nx, ny, nz, (0., 1., 0.));
                    rows::check_d(ui, "Negative Side", negative, false, "Pick the side the normal points away from");
                    rows::check_d(ui, "Include Zero", include_zero, true, "Elements on the plane count too");
                }
                SelKind::Distance { x, y, z, min, max } => {
                    position_rows(ui, "From", x, y, z, Unit::Distance, "Where distance is measured from");
                    min_max(ui, min, max, 0., 1., Unit::Distance, "distance of each element's centre");
                }
                SelKind::RandomCount {
                    seed,
                    number,
                    percent,
                } => {
                    rows::number_d(ui, "Seed", seed, 0, Unit::Count, 0..=u32::MAX, "Random number generator seed");
                    rows::number_d(ui, "Number", number, 10, Unit::Count, 0..=u32::MAX, "How many; 0 uses Percentage");
                    rows::number_d(ui, "Percentage", percent, 50., Unit::Percent, 0.0..=100.0, "The share picked when Number is 0");
                }
                SelKind::RandomParts { seed, chance } => {
                    rows::number_d(ui, "Seed", seed, 0, Unit::Count, 0..=u32::MAX, "Random number generator seed");
                    rows::number_d(ui, "Chance", chance, 50., Unit::Percent, 0.0..=100.0, "The chance each connected part is picked");
                }
                SelKind::VertexEdges { min, max } => {
                    rows::number_d(ui, "Minimum Edges", min, 5, Unit::Count, 0..=u32::MAX, "Fewest edges at the point");
                    rows::number_d(
                        ui,
                        "Maximum Edges",
                        max,
                        0,
                        Unit::Count,
                        0..=u32::MAX,
                        "Most edges; below Minimum means no limit",
                    );
                }
                SelKind::Symmetric { x, y, z, tolerance } => {
                    rows::check_d(ui, "Reflect X", x, true, "Mirror across X");
                    rows::check_d(ui, "Reflect Y", y, false, "Mirror across Y");
                    rows::check_d(ui, "Reflect Z", z, false, "Mirror across Z");
                    rows::number_d(ui, "Tolerance", tolerance, 0.0001, Unit::Distance, 0.0..=1e9, "How close a mirrored element must be");
                    rows::note(ui, "Picks the mirror image of the selection below; set Blend Mode to Add to keep both.");
                }
                SelKind::UvSeam { map } => {
                    rows::text_d(ui, "UV Map", map, "Texture", "The UV map whose seams it picks");
                }
                SelKind::Connected {} => {
                    rows::note(ui, "Every element of the parts the selection below touches.");
                }
                SelKind::PolygonTag { tag, name } => {
                    rows::choice_d(
                        ui,
                        "Tag",
                        tag,
                        &0,
                        &[(0, "Material"), (1, "Part")],
                        "Which polygon tag it reads",
                    );
                    rows::text_d(ui, "Name", name, "Default", "The tag's name, as typed");
                }
                SelKind::Coplanar { angle } => {
                    rows::number_d(
                        ui,
                        "Angle",
                        angle,
                        1.,
                        Unit::Angle,
                        0.0..=180.0,
                        "How far a neighbor may turn from the polygon it grows from",
                    );
                    rows::note(ui, "Grows the selection below across flat neighbors.");
                }
                SelKind::HardEdges {} => {
                    rows::note(ui, "Edges marked hard by the vertex normal tools.");
                }
                SelKind::VertexMapValue { map, min, max } => {
                    rows::text_d(ui, "Weight Map", map, "Weight", "The weight map it reads");
                    min_max(ui, min, max, 0.5, 1., Unit::Scalar, "weight value (1 = 100%)");
                }
                SelKind::Falloff { threshold, falloff } => {
                    selop_falloff_rows(ui, threshold, falloff, active_falloff);
                }
                SelKind::ConditionalLoop(rules) => conditional_loop_rows(ui, rules),
            }
            // #19: the order matters for Subtract and Multiply. The
            // steps are listed first to last, top to bottom, so the arrows
            // point the way the row moves (Steve Hill, Jason's call
            // 2026-11-25); the glyphs match the Add browser's ⏶/⏷.
            ui.horizontal(|ui| {
                ui.gated_ui(crate::unavailable::unless(n > 0, "Already the first"), |ui| {
                    if ui
                        .small_button("⏶")
                        .on_hover_text("Move earlier: applied before the step above it")
                        .clicked()
                    {
                        moved = Some(n - 1);
                    }
                });
                ui.gated_ui(
                    crate::unavailable::unless(n + 1 < count, "Already the last"),
                    |ui| {
                        if ui
                            .small_button("⏷")
                            .on_hover_text("Move later: applied after the step below it")
                            .clicked()
                        {
                            moved = Some(n + 1);
                        }
                    },
                );
            });
            remove = rows::button(ui, "Remove").clicked();
        });
        if remove {
            edit = Some(SelOpEdit::Remove(n));
        } else if let Some(to) = moved {
            edit = Some(SelOpEdit::Move(n, to));
        } else if next != *s {
            edit = Some(SelOpEdit::Set(n, next));
        }
    }
    let b = ui.button("Add Selection…");
    (edit, b.clicked().then_some(b.rect))
}

pub(crate) const ADD_OPERATOR: &str = "add-operator";
pub(crate) const ADD_SELECTION: &str = "add-selection";

/// Add Operator's contents (AB §5): every op type in its folder under
/// Mesh Operations. Unbuilt ops aren't in the table, and folders without
/// ops don't show (AB-41).
pub(crate) fn operator_catalog() -> crate::add_browser::Catalog {
    let mut entries = Vec::new();
    for d in op_defs() {
        let mut folder = vec!["Mesh Operations".to_string()];
        folder.extend(d.folder().split('/').map(str::to_owned));
        entries.push(crate::add_browser::Entry {
            id: d.id().into(),
            label: d.label().into(),
            folder,
            icon: op_icon(d.id()),
        });
    }
    crate::add_browser::Catalog {
        roots: vec![
            "Mesh Operations".into(),
            "Deformers".into(),
            "Mesh Operation Assemblies".into(),
        ],
        entries,
        plain: false,
    }
}
/// An op type's icon in the Add browser: its direct tool's icon from the
/// generator where there is one; `None` = the default cube.
pub(crate) fn op_icon(id: &str) -> Option<crate::icons::Icon> {
    let slug = match id {
        "prim.cube.item" => "cube",
        "operandi.skin.item" => "skin-mesh",
        "prim.sphere.item" => "sphere",
        "prim.cylinder.item" => "cylinder",
        "prim.cone.item" => "cone",
        "prim.capsule.item" => "capsule",
        "prim.ellipsoid.item" => "ellipsoid",
        "prim.toroid.item" => "torus",
        "prim.text.item" => "text",
        "prim.arc.item" => "arc",
        "poly.bevel.item" => "bevel",
        "poly.smshift.item" => "smooth-shift",
        "subdivide.tool.item" => "subdivide-smooth",
        "pmodel.triangulate.item" => "triple",
        "mirror.clone.item" => "mirror",
        "array.clone.item" => "array",
        "radial.array.item" => "radial-array",
        "radial.sweep.item" => "radial-sweep",
        "linear.clone.item" => "clone",
        "boolean.meshop.item" | "solidDrill.meshop.item" | "axisDrill.meshop.item" => "boolean",
        // Its own studio icon comes with the full MeshBlend Item (B1b).
        "meshblend.item" => "boolean",
        "pmodel.meshmerge" | "pmodel.mergePolygons.item" => "merge",
        "poly.extrude.item" => "extrude",
        "symmetrize.meshop.item" => "symmetry",
        "detriangulate.meshop.item" => "detriangulate",
        "poly.unsubdiv.item" => "unsubdivide",
        "pmodel.materialTag.item" => "tag-material",
        "poly.spikey.item" => "spike",
        "edge.bevel.item" => "edge-bevel",
        "edge.chamfer.item" => "edge-chamfer",
        "edge.extrude.item" => "edge-extrude",
        "edge.relax.item" => "edge-relax",
        "pmodel.edge.subdivide" => "edge-add-point",
        "pmodel.splitEdges.item" => "edge",
        // Batch 1 tool steps: their direct tools' icons.
        "edge.bridge.item" => "bridge",
        "poly.loopSlice.item" => "loop-slice",
        "prim.slice.item" => "primitive-slice",
        "curve.slice.item" => "curve-slice",
        "pen.slice.item" => "pen-slice",
        "edge.slide.item" => "edge-slide",
        "chamfer.edit.item" => "edge-chamfer",
        "poly.thicken.item" => "thicken",
        "poly.setType.meshop.item" => "sds-subdivide",
        "pmodel.partTag.item" => "tag-part",
        "setvmap.subd.item" => "edge-crease",
        "hardedge.meshop.item" | "hardedgeconvert.meshop.item" => "harden-normals",
        "assignselectionset.meshop.item" => "tag-selection-set",
        "symmetry.tool.item" => "symmetry",
        "pmodel.workplane.item" => "workplane",
        "falloff.meshop.item" => "falloff",
        "deform.bend.item" => "bend",
        "deform.twist.item" => "twist",
        "deform.transform.item" => "transform",
        "deform.rotate.item" => "rotate",
        "deform.magnet.item" => "soft-move",
        "deform.weightInfluence.item" => "vmap-paint",
        "deform.pushInfluence.item" => "push",
        "deform.morphInfluence.item" => "vmap-set-value",
        "deform.vortex.item" => "vortex",
        "deform.wrap.item" => "uv-unwrap",
        "deform.lattice.item" => "cube",
        "deform.spline.item" => "curve",
        "deform.bezier.item" => "bezier-curve",
        "deform.curveConstraint.item" => "curve-extrude",
        "uv.create.item" | "uvtransform" => "uv-project",
        "uv.relax.item" => "uv-relax",
        "uv.pelt.item" => "uv-pelt",
        "uvflip.meshop.item" => "uv-flip-u",
        "uvconvert.meshop.item" | "setvmap.create.item" => "tree-uv-map",
        "setvmap.weight.item" => "paint-paintbrush",
        "setvmap.flood.item" => "paint-fill",
        "setvmap.rgb.item" => "add-color-texture",
        "setvmap.seam.item" => "uv-split",
        "setvmap.vnorm.item" => "harden-normals",
        "pmodel.geodesic.mOp" => "paint-target",
        "cRebuild.mOp" | "curve.rebuildSpan" | "curvefill" | "curveWinding.meshop.item" => "curve",
        "pmodel.edgeToCurve.item" => "edges-to-curves",
        "curveBoolean.meshop.item" => "curve-boolean",
        "curveconvert.meshop.item" => "bezier-curve",
        // Curve tools and batch 5 (provisional: borrowed icons until their own).
        "curve.circle.item"
        | "curve.ellipse.item"
        | "curve.rectangle.item"
        | "curve.diamond.item"
        | "curve.nsided.item"
        | "curve.star.item" => "bezier-curve",
        "curve.extrude.item" => "sketch-extrude",
        "curve.sweep" | "laceGeom" => "curve-extrude",
        "curve.clone.item" | "pen.clone.item" | "scatter.clone.item" => "clone",
        "surfaceConstraint.meshop.item" => "pen",
        "uvrect.meshop.item" | "uvfit.meshop.item" | "vertexmaptransfer" => "tree-uv-map",
        "vert.bevel.item" => "vertex-bevel",
        "vert.extrude.item" => "vertex-extrude",
        "pmodel.createVertex" => "vertex-create",
        "vert.merge.item" => "vertex-merge",
        "vert.split.meshop.item" => "vertex-tear",
        "vert.setpos.meshop.item" => "vertex",
        "poly.knife.item" => "polygon-slice",
        "poly.julienne.item" => "julienne",
        "poly.reduct.item" => "poly-reduct",
        "xfrm.jitter.item" => "jitter",
        "xfrm.quantize.item" => "quantize",
        "xfrm.radialAlign.item" => "radial-align",
        "xfrm.linearAlign.item" => "linear-align",
        "xfrm.push.item" => "push",
        "smooth.meshop.item" => "smooth",
        "uv.unwrap.item" | "uvpack.meshop.item" | "pmodel.transformMap" => "tree-uv-map",
        _ => return None,
    };
    crate::icons::Icon::from_slug(slug)
}
/// Add Selection Operation's contents: the selection ops, a plain list.
fn selection_catalog() -> crate::add_browser::Catalog {
    crate::add_browser::Catalog {
        roots: vec!["Selection".into()],
        entries: SelKind::menu()
            .into_iter()
            .map(|(label, _)| crate::add_browser::Entry {
                id: label.into(),
                label: label.into(),
                folder: vec!["Selection".into()],
                icon: Some(crate::icons::Icon::TagSelectionSet),
            })
            .collect(),
        plain: true,
    }
}

impl Operandi {
    /// Draws the Add browser `key` while it's open; the id of what was
    /// added, if anything.
    pub(crate) fn show_add_browser(
        &mut self,
        ctx: &egui::Context,
        key: &'static str,
        title: &str,
        catalog: crate::add_browser::Catalog,
    ) -> Option<String> {
        let (state, anchor) = self.add_browsers.get_mut(key)?;
        let out = crate::add_browser::show(ctx, key, title, *anchor, &catalog, state);
        if state.open {
            self.menu_owned_input = true;
        }
        out.added
    }
}

/// A value's conversion between stored and shown.
type Convert = fn(f64) -> f64;

/// Right-click menus of the list.
struct Hooks {
    picked: Option<MenuPick>,
}
#[derive(Clone, Copy)]
enum MenuPick {
    Freeze(OpRef, bool),
    Delete(OpRef),
    EditBase(u64, bool),
}
impl TreeHooks<OpKey> for Hooks {
    fn has_row_menu(&self, _key: OpKey) -> bool {
        true
    }
    fn row_menu(&mut self, ui: &mut Ui, key: OpKey, _rename: &mut dyn FnMut()) {
        match key {
            OpKey::Op(item, op) => {
                let r = OpRef { item, op };
                if ui
                    .button("Freeze")
                    .on_hover_text("Bake this op and those below it into the Base Mesh")
                    .clicked()
                {
                    self.picked = Some(MenuPick::Freeze(r, false));
                    ui.close();
                }
                if ui
                    .button("Duplicate and Freeze")
                    .on_hover_text("A new mesh item with this op and those below it baked in")
                    .clicked()
                {
                    self.picked = Some(MenuPick::Freeze(r, true));
                    ui.close();
                }
                ui.separator();
                if ui.button("Delete").clicked() {
                    self.picked = Some(MenuPick::Delete(r));
                    ui.close();
                }
            }
            OpKey::Base(item) => {
                if ui
                    .button("Edit Base Mesh")
                    .on_hover_text("Direct tools edit the Base Mesh; the result shows as a ghost")
                    .clicked()
                {
                    self.picked = Some(MenuPick::EditBase(item, true));
                    ui.close();
                }
            }
        }
    }
}

impl Operandi {
    /// The Mesh Stack panel body (registered in panel_registry.rs).
    pub(super) fn mesh_stack_section(&mut self, ui: &mut Ui, _area: Rect) {
        let Some(item) = self.editor.scene.active().filter(|i| i.is_mesh()).cloned() else {
            ui.add_space(6.);
            crate::form::rows::note(ui, "Select a mesh item to see its Mesh Stack.");
            return;
        };
        let mut state = std::mem::take(&mut self.mesh_stack_tree);
        let mut add: Option<String> = None;
        let mut add_open: Option<egui::Rect> = None;
        let editing_base = self.editor.base_edit == Some(item.id);
        let mut base_toggle = None;
        tree_list::tool_row(ui, &mut state, false, |ui, side| match side {
            tree_list::Side::Left => {
                ui.gated_ui(
                    None,
                    |ui| {
                        let b = ui.button("Add Operator…").on_hover_text(
                            "Add a mesh operation above the selected row (on top when none is \
                             selected); polygons selected on the Base Mesh become its Select by Index",
                        );
                        if b.clicked() {
                            add_open = Some(b.rect);
                        }
                    },
                );
            }
            tree_list::Side::Right => {
                if item.mesh_stack.is_some() {
                    let r = tree_list::toggle_button(ui, "Base", editing_base).on_hover_text(
                        if editing_base {
                            "Stop editing the Base Mesh"
                        } else {
                            "Edit Base Mesh: direct tools edit the Base Mesh, with the result \
                             drawn as a ghost"
                        },
                    );
                    if r.clicked() {
                        base_toggle = Some(!editing_base);
                    }
                }
            }
        });
        let evaluated = self.editor.mesh_stack.result(&self.editor.scene, &item);
        let ops = item.mesh_stack.as_ref().map_or(&[][..], |s| &s.ops[..]);
        let row_sel = self.editor.meshop_row.filter(|r| r.item == item.id);
        let mut rows = Vec::new();
        let labels = operandi_core::mesh_stack::op_labels(ops);
        for (i, op) in ops.iter().enumerate().rev() {
            let mut row = TreeRow::new(OpKey::Op(item.id, op.id), 0, labels[i].clone(), 2);
            row.icon = RowIcon::Icon(crate::icons::Icon::TreeMesh);
            row.draggable = true;
            row.selected = if row_sel
                == Some(OpRow {
                    item: item.id,
                    op: Some(op.id),
                }) {
                Selected::Primary
            } else {
                Selected::No
            };
            row.style.dimmed = !op.enabled;
            let status = evaluated.as_ref().and_then(|e| e.statuses.get(i));
            if let Some(OpStatus::Failed(reason)) = status {
                row.tint = Some(theme::error());
                row.tip = Some(format!("Failed, passed through: {reason}"));
            } else if let Some(OpStatus::Noted(notes)) = status {
                if notes.iter().any(|n| n.problem) {
                    row.tint = Some(theme::error());
                }
                row.tip = Some(
                    notes
                        .iter()
                        .map(|n| n.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            } else if op.kind.takes_selection() && op.selection.is_some() {
                row.tip = Some("Works on its Select by Index".into());
            }
            row.cells[0] = Cell::Toggle {
                on: op.enabled,
                tip: "On or off".into(),
                dimmed: false,
            };
            rows.push(row);
        }
        let mut base = TreeRow::new(OpKey::Base(item.id), 0, "Base Mesh", 2);
        base.icon = RowIcon::Icon(icons::Icon::Cube);
        base.fixed = true;
        base.style.italic = editing_base;
        base.selected = if row_sel
            == Some(OpRow {
                item: item.id,
                op: None,
            }) {
            Selected::Primary
        } else {
            Selected::No
        };
        base.tip = Some(format!(
            "{} polygons{}",
            item.mesh.faces.len(),
            if editing_base { " · being edited" } else { "" }
        ));
        rows.push(base);
        let mut hooks = Hooks { picked: None };
        let mut editors = std::mem::take(&mut self.text_editors);
        let events = tree_list::show(ui, &spec(), &mut state, &rows, &mut hooks, &mut editors);
        self.text_editors = editors;
        state.retain(|k| rows.iter().any(|r| r.key == k));
        self.mesh_stack_tree = state;

        // AB-01: Add Operator opens the Add browser.
        if let Some(rect) = add_open {
            let entry = self
                .add_browsers
                .entry(ADD_OPERATOR)
                .or_insert_with(|| (Default::default(), egui::Rect::NOTHING));
            entry.0.open = !entry.0.open;
            entry.1 = rect;
        }
        if let Some(id) = self.show_add_browser(
            ui.ctx(),
            ADD_OPERATOR,
            "Add Mesh Operation",
            operator_catalog(),
        ) {
            add = Some(id);
        }
        if let Some(t) = add {
            self.run(Command::MeshOpCreate(t));
        }
        if let Some(on) = base_toggle {
            self.run(Command::MeshOpEditBase { item: item.id, on });
        }
        match hooks.picked {
            Some(MenuPick::Freeze(op, duplicate)) => {
                self.run(Command::MeshOpFreeze { op, duplicate })
            }
            Some(MenuPick::Delete(r)) => self.run(Command::MeshOpDelete(vec![r])),
            Some(MenuPick::EditBase(id, on)) => self.run(Command::MeshOpEditBase { item: id, on }),
            None => {}
        }
        for event in events {
            match event {
                TreeEvent::Click { key, how: _ } | TreeEvent::DoubleClick(key) => {
                    let row = match key {
                        OpKey::Op(item, op) => OpRow { item, op: Some(op) },
                        OpKey::Base(item) => OpRow { item, op: None },
                    };
                    self.run(Command::MeshOpSelect(Some(row)));
                }
                TreeEvent::Toggle {
                    key: OpKey::Op(item, op),
                    ..
                } => {
                    let on = ops.iter().find(|o| o.id == op).is_some_and(|o| o.enabled);
                    self.run(Command::MeshOpEnable {
                        op: OpRef { item, op },
                        on: !on,
                    });
                }
                TreeEvent::Drop { keys, at, .. } => {
                    // Rows show top = last evaluated, so "before" a row in the
                    // list is above it in the stack (MO-21).
                    let index_of = |op: u64| ops.iter().position(|o| o.id == op);
                    let target = match at {
                        DropAt::Before(OpKey::Op(_, o)) | DropAt::On(OpKey::Op(_, o)) => {
                            index_of(o).map(|i| i + 1)
                        }
                        DropAt::After(OpKey::Op(_, o)) => index_of(o),
                        DropAt::Before(OpKey::Base(_)) | DropAt::End => Some(0),
                        _ => None,
                    };
                    if let (Some(mut to), [OpKey::Op(item, op)]) = (target, keys.as_slice())
                        && let Some(from) = index_of(*op)
                    {
                        if to > from {
                            to -= 1;
                        }
                        self.run(Command::MeshOpMove {
                            op: OpRef {
                                item: *item,
                                op: *op,
                            },
                            position: to,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// The active mesh and its selected op row, when an op is selected.
    pub(super) fn selected_mesh_op(&self) -> Option<(operandi_core::scene::Item, u64)> {
        let item = self.editor.scene.active().filter(|i| i.is_mesh())?;
        let row = self.editor.meshop_row.filter(|r| r.item == item.id)?;
        Some((item.clone(), row.op?))
    }

    /// The Properties panel body for the selected op.
    pub(super) fn mesh_op_properties_panel(&mut self, ui: &mut Ui) {
        let Some((item, _)) = self.selected_mesh_op() else {
            return;
        };
        let evaluated = self.editor.mesh_stack.result(&self.editor.scene, &item);
        self.mesh_op_properties(ui, &item, evaluated.as_ref());
    }

    /// The selected op's Properties (MO-19). A drag is one undo step.
    fn mesh_op_properties(
        &mut self,
        ui: &mut Ui,
        item: &operandi_core::scene::Item,
        evaluated: Option<&operandi_core::mesh_stack::Evaluated>,
    ) {
        use crate::form::{Unit, rows};
        let Some(OpRow {
            op: Some(op_id), ..
        }) = self.editor.meshop_row.filter(|r| r.item == item.id)
        else {
            if self.editor.base_edit == Some(item.id) {
                rows::note(
                    ui,
                    "Editing the Base Mesh: direct tools change it and the stack re-runs on top. \
                     The full result is drawn as a ghost.",
                );
            }
            return;
        };
        let Some(stack) = item.mesh_stack.as_ref() else {
            return;
        };
        let Some(index) = stack.index_of(op_id) else {
            return;
        };
        let op = &stack.ops[index];
        let r = OpRef {
            item: item.id,
            op: op_id,
        };
        if let Some(OpStatus::Failed(reason)) = evaluated.and_then(|e| e.statuses.get(index)) {
            ui.colored_label(theme::error(), format!("Failed, passed through: {reason}"));
        }
        if let OpKind::Placeholder { reason, .. } = &op.kind {
            rows::note(ui, reason);
            return;
        }
        let Some(def) = op.kind.def() else {
            return;
        };
        let values = op.kind.values();
        let defaults = def.defaults();
        // Other items a link can name (SAFE-07 refuses loops when set).
        let meshes: Vec<(u64, String)> = self
            .editor
            .scene
            .items
            .iter()
            // Group locators too: a group links all its meshes (Boolean).
            .filter(|i| (i.is_mesh() || i.is_group_locator()) && i.kit.is_none() && i.id != item.id)
            .map(|i| (i.id, i.name.clone()))
            .collect();
        let mut changes = serde_json::Map::new();
        let mut index_edit = None;
        let mut selop_edit = None;
        let mut selop_open: Option<egui::Rect> = None;
        let mut dragged = false;
        let mut blend_commands = Vec::new();
        let mut blend_dragged = false;
        rows::scope(ui, "mesh-op-properties", |ui| {
            let title = operandi_core::mesh_stack::op_labels(&stack.ops)[index].clone();
            rows::section(ui, &title, false, |ui| {
                // The form comes from the op's type (KIT-4: kits describe
                // their forms, the app draws them).
                for f in def.fields() {
                    if f.kind == FieldKind::Hidden {
                        continue;
                    }
                    let key: &str = &f.key;
                    let label: &str = &f.label;
                    let value = values.get(key).cloned().unwrap_or_default();
                    let default = defaults.get(key).cloned().unwrap_or_default();
                    let tip = format!("{label} ({key})");
                    let why = def.field_unavailable(key, &values);
                    let response = ui
                        .gated_ui(why.as_deref(), |ui| match &f.kind {
                            FieldKind::Check => {
                                let mut on = value.as_bool().unwrap_or(false);
                                let d = default.as_bool().unwrap_or(false);
                                let r = rows::check_d(ui, label, &mut on, d, &tip);
                                if r.changed() {
                                    changes.insert(key.into(), on.into());
                                }
                                r
                            }
                            FieldKind::Choice(names) => {
                                let mut i = value.as_u64().unwrap_or(0) as usize;
                                let d = default.as_u64().unwrap_or(0) as usize;
                                let options: Vec<(usize, &str)> = names
                                    .iter()
                                    .enumerate()
                                    .map(|(i, n)| (i, n.as_ref()))
                                    .collect();
                                let r = rows::choice_d(ui, label, &mut i, &d, &options, &tip);
                                if r.changed() {
                                    changes.insert(key.into(), (i as u64).into());
                                }
                                r
                            }
                            FieldKind::Count { min, max } => {
                                let mut n = value.as_u64().unwrap_or(0) as u32;
                                let d = default.as_u64().unwrap_or(0) as u32;
                                let r = rows::number_d(
                                    ui,
                                    label,
                                    &mut n,
                                    d,
                                    Unit::Count,
                                    *min..=*max,
                                    &tip,
                                );
                                if r.changed() {
                                    changes.insert(key.into(), n.into());
                                }
                                r
                            }
                            FieldKind::Text => {
                                let mut t = value.as_str().unwrap_or_default().to_string();
                                let d = default.as_str().unwrap_or_default();
                                let r = rows::text_d(ui, label, &mut t, d, &tip);
                                if r.changed() {
                                    changes.insert(key.into(), t.into());
                                }
                                r
                            }
                            FieldKind::Item => {
                                let mut id = value.as_u64().unwrap_or(0);
                                let options = item_options(&meshes, id);
                                let r = rows::choice_d(ui, label, &mut id, &0, &options, &tip);
                                if r.changed() {
                                    changes.insert(key.into(), id.into());
                                }
                                r
                            }
                            FieldKind::Items => {
                                // One row per linked item ("(none)" removes it)
                                // and an Add row.
                                let mut ids: Vec<u64> = value
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .filter_map(|v| v.as_u64())
                                    .collect();
                                let mut changed = None;
                                for (n, id) in ids.clone().iter().enumerate() {
                                    let mut pick = *id;
                                    let options = item_options(&meshes, pick);
                                    let row = format!("{label} {}", n + 1);
                                    let r = rows::choice_d(ui, &row, &mut pick, id, &options, &tip);
                                    if r.changed() {
                                        ids[n] = pick;
                                    }
                                    changed =
                                        Some(changed.map_or(r.clone(), |c: egui::Response| c | r));
                                }
                                let mut pick = 0u64;
                                let options = item_options(&meshes, 0);
                                let r = rows::choice_d(
                                    ui,
                                    &format!("Add {label}"),
                                    &mut pick,
                                    &0,
                                    &options,
                                    &tip,
                                );
                                if pick != 0 {
                                    ids.push(pick);
                                }
                                let r = changed.map_or(r.clone(), |c| c | r);
                                ids.retain(|&id| id != 0);
                                let before: Vec<u64> = value
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .filter_map(|v| v.as_u64())
                                    .collect();
                                if ids != before {
                                    changes.insert(key.into(), ids.into());
                                }
                                r
                            }
                            FieldKind::Hidden => ui.label(""),
                            FieldKind::Number
                            | FieldKind::Distance
                            | FieldKind::Angle
                            | FieldKind::Percent => {
                                let (unit, shown, back): (Unit, Convert, Convert) = match f.kind {
                                    FieldKind::Angle => {
                                        (Unit::Angle, f64::to_degrees, f64::to_radians)
                                    }
                                    FieldKind::Percent => {
                                        (Unit::Percent, |v| v * 100., |v| v / 100.)
                                    }
                                    FieldKind::Number => (Unit::Scalar, |v| v, |v| v),
                                    _ => (Unit::Distance, |v| v, |v| v),
                                };
                                let mut x = shown(value.as_f64().unwrap_or(0.));
                                let d = shown(default.as_f64().unwrap_or(0.));
                                let r =
                                    rows::number_d(ui, label, &mut x, d, unit, -1e9..=1e9, &tip);
                                if r.changed() {
                                    changes.insert(key.into(), back(x).into());
                                }
                                r
                            }
                        })
                        .inner;
                    dragged |= response.dragged();
                }
            });
            // #15: a primitive op's own copy of a falloff.
            if def.fields().iter().any(|f| f.key == "falloff") {
                op_falloff_section(ui, &self.editor, &values, &mut changes);
            }
            if def.id() == operandi_core::meshop_wrap::WRAP_ID
                && let Some(e) = wrap_section(
                    ui,
                    &self.editor.scene,
                    &values,
                    item.id,
                    op_id,
                    &mut changes,
                )
            {
                self.status = e;
            }
            if def.id() == operandi_core::meshblend::MESHBLEND_OP {
                let status = evaluated.and_then(|e| e.statuses.get(index));
                (blend_commands, blend_dragged) =
                    crate::meshblend_ui::sections(ui, &self.editor.scene, item, status);
            }
            if op.kind.takes_selection() {
                rows::section(ui, "Selection", false, |ui| {
                    let text = op
                        .selection
                        .as_ref()
                        .map_or_else(|| "Whole mesh".to_string(), |s| s.text());
                    rows::value(ui, "Select by Index", &text);
                    for (label, edit, tip) in [
                        (
                            "Set",
                            IndexEdit::Set,
                            "Use the polygons selected on the Base Mesh",
                        ),
                        (
                            "Add",
                            IndexEdit::Add,
                            "Add the polygons selected on the Base Mesh",
                        ),
                        (
                            "Remove",
                            IndexEdit::Remove,
                            "Take the selected polygons out",
                        ),
                        ("Clear", IndexEdit::Clear, "Work on the whole mesh"),
                    ] {
                        if rows::button(ui, label).on_hover_text(tip).clicked() {
                            index_edit = Some(edit);
                        }
                    }
                    rows::note(
                        ui,
                        "Polygon numbers can point at other polygons once an op below changes \
                         the topology. Select polygons in Edit Base Mesh to set them.",
                    );
                    let active = self.editor.primitive_weights();
                    let (edit, open) = selection_ops(ui, op, &stack.ops[..index], active.as_ref());
                    selop_edit = edit;
                    selop_open = open;
                });
            }
        });
        if !blend_commands.is_empty() {
            // One undo step per drag, as for the form's own fields.
            let dragging = blend_dragged || ui.input(|i| i.pointer.any_down());
            let gesture = match self.mesh_stack_gesture {
                Some(g) if dragging => g,
                _ => crate::colour_field::next_gesture(),
            };
            self.mesh_stack_gesture = dragging.then_some(gesture);
            let previous = self.editor.history.undo_fold.replace(gesture);
            for c in blend_commands {
                self.run(c);
            }
            self.editor.history.undo_fold = previous;
        }
        if let Some(edit) = index_edit {
            self.run(Command::MeshOpIndex { op: r, edit });
        }
        if let Some(rect) = selop_open {
            let entry = self
                .add_browsers
                .entry(ADD_SELECTION)
                .or_insert_with(|| (Default::default(), egui::Rect::NOTHING));
            entry.0.open = !entry.0.open;
            entry.1 = rect;
        }
        // AB-01: Add Selection Operation, a plain filtered list.
        if let Some(id) = self.show_add_browser(
            ui.ctx(),
            ADD_SELECTION,
            "Add Selection Operation",
            selection_catalog(),
        ) && let Some((_, kind)) = SelKind::menu().into_iter().find(|(l, _)| *l == id)
        {
            selop_edit = Some(SelOpEdit::Add(SelOp::new(kind)));
        }
        if let Some(edit) = selop_edit {
            self.run(Command::MeshOpSelOp { op: r, edit });
        }
        if changes.is_empty() {
            if !ui.input(|i| i.pointer.any_down()) {
                self.mesh_stack_gesture = None;
            }
            return;
        }
        // One undo step per drag (SAFE-13).
        let dragging = dragged || ui.input(|i| i.pointer.any_down());
        let gesture = match self.mesh_stack_gesture {
            Some(g) if dragging => g,
            _ => crate::colour_field::next_gesture(),
        };
        self.mesh_stack_gesture = dragging.then_some(gesture);
        let previous = self.editor.history.undo_fold.replace(gesture);
        self.run(Command::MeshOpSet {
            op: r,
            values: changes,
        });
        self.editor.history.undo_fold = previous;
    }
}

/// Wrap's binding rows: how many points are stuck to the Target, and
/// Rebind, which measures the binding again from where the mesh and the
/// Target are now. Returns why a Rebind failed.
fn wrap_section(
    ui: &mut egui::Ui,
    scene: &operandi_core::Scene,
    values: &serde_json::Value,
    item: u64,
    op: u64,
    changes: &mut serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    use crate::form::rows;
    use crate::unavailable::Gate as _;
    let bound = values["binding"].as_array().map_or(0, Vec::len);
    let has_target = values["target"].as_u64().is_some_and(|t| t != 0);
    let mut failed = None;
    rows::section(ui, "Binding", false, |ui| {
        let shown = if bound == 0 {
            "Not bound".to_owned()
        } else {
            format!("{bound} points")
        };
        rows::value(ui, "Bound", &shown);
        ui.gated_ui(
            crate::unavailable::unless(has_target, "Pick a Target first"),
            |ui| {
                if rows::button(ui, "Rebind")
                    .on_hover_text(
                        "Stick each point to its nearest spot on the Target, as things are now",
                    )
                    .clicked()
                {
                    match operandi_core::meshop_wrap::bind(scene, item, op) {
                        Ok(v) => {
                            changes.insert("binding".into(), v);
                        }
                        Err(e) => failed = Some(e.to_string()),
                    }
                }
            },
        );
        rows::note(
            ui,
            "The points follow the Target when it moves or changes shape. Press Rebind \
             after changing the mesh below or the Target's polygons.",
        );
    });
    failed
}

/// A primitive op's Falloff section (#15): which falloff layers the op
/// keeps, Use Active Falloff and Clear Falloff. The op can't follow the
/// tool falloff (it isn't scene data), so it keeps a copy.
fn op_falloff_section(
    ui: &mut egui::Ui,
    editor: &operandi_core::Editor,
    values: &serde_json::Value,
    changes: &mut serde_json::Map<String, serde_json::Value>,
) {
    use crate::form::rows;
    use crate::unavailable::Gate as _;
    let kept: Option<operandi_core::ShapeWeights> =
        serde_json::from_value(values["falloff"].clone())
            .ok()
            .flatten();
    rows::section(ui, "Falloff", false, |ui| {
        let shown = kept.as_ref().map_or_else(
            || "None".to_owned(),
            |w| {
                let mut names = vec![w.first.label()];
                names.extend(w.added.iter().map(|a| a.0.label()));
                names.join(" + ")
            },
        );
        rows::value(ui, "Falloff", &shown);
        // Steve Hill via Jason (2026-11-25): the op's falloff is its own
        // and live; every change here re-evaluates the stack at once.
        if let Some(next) = edit_op_falloff(ui, values, kept.as_ref()) {
            match next.map(serde_json::to_value) {
                Some(Ok(v)) => {
                    changes.insert("falloff".into(), v);
                }
                None => {
                    changes.insert("falloff".into(), serde_json::Value::Null);
                }
                Some(Err(_)) => {}
            }
        }
        let active = editor.primitive_weights();
        let why = if editor.spatial_falloff.is_none() {
            "Turn on a Linear, Radial, Cylinder or Noise falloff from the Falloff menu first"
                .to_owned()
        } else {
            editor.primitive_falloff_note().unwrap_or_default()
        };
        ui.gated_ui(crate::unavailable::unless(active.is_some(), &why), |ui| {
            if rows::button(ui, "Use Active Falloff")
                .on_hover_text("Copy the falloff from the Falloff menu into this operation")
                .clicked()
                && let Some(w) = &active
                && let Ok(v) = serde_json::to_value(w)
            {
                changes.insert("falloff".into(), v);
            }
        });
        ui.gated_ui(
            crate::unavailable::unless(kept.is_some(), "This operation has no falloff"),
            |ui| {
                if rows::button(ui, "Clear Falloff")
                    .on_hover_text("Remove this operation's falloff; the shape is plain again")
                    .clicked()
                {
                    changes.insert("falloff".into(), serde_json::Value::Null);
                }
            },
        );
        rows::note(
            ui,
            "Steps above that move points move them only by this falloff; primitives \
             above are shaped by it through their Falloff Scale rows. Drag its points in \
             the view while this step is selected.",
        );
    });
}

/// The op's own falloff rows: its type, then that type's values. Returns
/// the changed falloff (`Some(None)` = turned off), if any.
fn edit_op_falloff(
    ui: &mut egui::Ui,
    values: &serde_json::Value,
    kept: Option<&operandi_core::ShapeWeights>,
) -> Option<Option<operandi_core::ShapeWeights>> {
    use crate::form::{Unit, rows};
    use operandi_core::{CylinderFalloff, Falloff, FalloffShape, LinearFalloff, RadialFalloff};
    let kind = |f: &Falloff| match f {
        Falloff::Linear(_) => 1,
        Falloff::Radial(_) => 2,
        Falloff::Cylinder(_) => 3,
        Falloff::Noise(_) => 4,
        _ => 5,
    };
    let now = kept.map_or(0, |w| kind(&w.first));
    let mut pick = now;
    let mut options = vec![
        (0, "None"),
        (1, "Linear"),
        (2, "Radial"),
        (3, "Cylinder"),
        (4, "Noise"),
    ];
    if now == 5 {
        options.push((5, "(other)"));
    }
    rows::choice_d(
        ui,
        "Falloff Type",
        &mut pick,
        &0,
        &options,
        "This operation's own falloff",
    );
    // A new falloff starts fitted to the shape: along its axis, from its
    // base (none) to its top (full).
    let f64_of = |k: &str| values.get(k).and_then(|v| v.as_f64()).unwrap_or(0.);
    let centre = glam::DVec3::new(f64_of("cenX"), f64_of("cenY"), f64_of("cenZ"));
    let full = values.get("segmentsX").is_some();
    let size = glam::DVec3::new(f64_of("sizeX"), f64_of("sizeY"), f64_of("sizeZ"))
        * if full { 0.5 } else { 1. };
    let size = if size == glam::DVec3::ZERO {
        glam::DVec3::splat(0.5)
    } else {
        size
    };
    let axis = values
        .get("axis")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(2) as usize;
    if pick != now {
        let mut up = glam::DVec3::ZERO;
        up[axis] = size[axis];
        let first = match pick {
            0 => return Some(None),
            1 => Falloff::Linear(LinearFalloff {
                start: centre + up,
                end: centre - up,
                ..Default::default()
            }),
            2 => Falloff::Radial(RadialFalloff {
                center: centre,
                size,
                ..Default::default()
            }),
            3 => Falloff::Cylinder(CylinderFalloff {
                center: centre,
                size,
                axis,
                ..Default::default()
            }),
            4 => Falloff::Noise(Default::default()),
            _ => return None,
        };
        return Some(Some(operandi_core::ShapeWeights {
            first,
            added: kept.map(|w| w.added.clone()).unwrap_or_default(),
        }));
    }
    let w = kept?;
    let mut first = w.first;
    let shapes = [
        (FalloffShape::Linear, "Linear"),
        (FalloffShape::EaseIn, "Ease-In"),
        (FalloffShape::EaseOut, "Ease-Out"),
        (FalloffShape::Smooth, "Smooth"),
    ];
    let vector = |ui: &mut egui::Ui, label: &str, v: &mut glam::DVec3, tip: &str| {
        let first = format!("{label} X");
        for (a, l) in [(0, first.as_str()), (1, "Y"), (2, "Z")] {
            rows::number_d(ui, l, &mut v[a], 0., Unit::Distance, -1e9..=1e9, tip);
        }
    };
    let shape_row = |ui: &mut egui::Ui, s: &mut FalloffShape| {
        rows::choice_d(
            ui,
            "Shape",
            s,
            &FalloffShape::Linear,
            &shapes,
            "How the weight fades",
        );
    };
    match &mut first {
        Falloff::Linear(f) => {
            vector(ui, "Start", &mut f.start, "Where the weight is full");
            vector(ui, "End", &mut f.end, "Where the weight is none");
            shape_row(ui, &mut f.shape);
        }
        Falloff::Radial(f) => {
            vector(ui, "Center", &mut f.center, "Where the weight is full");
            vector(
                ui,
                "Size",
                &mut f.size,
                "How far it reaches along X, Y and Z",
            );
            shape_row(ui, &mut f.shape);
        }
        Falloff::Cylinder(f) => {
            vector(ui, "Center", &mut f.center, "A point on the tube's axis");
            vector(ui, "Size", &mut f.size, "The tube's radii");
            rows::choice_d(
                ui,
                "Axis",
                &mut f.axis,
                &1,
                &[(0, "X"), (1, "Y"), (2, "Z")],
                "The tube's axis",
            );
            shape_row(ui, &mut f.shape);
        }
        Falloff::Noise(f) => {
            rows::number_d(
                ui,
                "Noise Scale",
                &mut f.scale,
                1.,
                Unit::Distance,
                -1e9..=1e9,
                "Size of the noise blobs",
            );
        }
        _ => {}
    }
    (first != w.first && first.validate().is_ok()).then(|| {
        Some(operandi_core::ShapeWeights {
            first,
            added: w.added.clone(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_op_value_has_a_form_field() {
        for d in op_defs() {
            let values = d.defaults();
            for key in values.as_object().unwrap().keys() {
                assert!(
                    d.fields().iter().any(|f| f.key == key.as_str()),
                    "{}: {key} has no form field",
                    d.id()
                );
            }
        }
    }
}
