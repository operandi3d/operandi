//! The Action Center × Symmetry test matrix
//! (specifications/ac-symmetry-rulebook-2026-11-19, cases.json, 311 cases),
//! run against the pivot service through the real tools (Move, Rotate,
//! Scale, Bevel, Extrude, Smooth Shift, Thicken).
//!
//! Not oracles (ACS-20, ACS-21; the rulebook marks them
//! `operandi_oracle: false`), replaced here by Operandi-rule cases with the
//! same inputs:
//! - the 24 Smooth Shift / Thicken cases recorded the reference app scaling
//!   about the item origin. Operandi scales about the selected Action
//!   Center (Jason's ruling, 2026-11-18). The expected points are derived
//!   from the recorded ones: a point the reference scaled to `r = s·q`
//!   (about the origin) is, scaled about centre `c`, `c + s·(q − c) = r +
//!   (1 − s)·c`, with `c` the typed centre on the primary side, its mirror
//!   on the other side and its foot on the plane for seam points (ACS-03..05).
//! - the 6 scripted Extrude cases with Symmetry X recorded the shift
//!   unmirrored; in the UI it is mirrored (ACS-21). Expected points: the
//!   recorded ones with the X shift of every new point on the other side
//!   of the plane reversed (−0.3 instead of +0.3).
//!
//! Known gaps (open questions, not fudged in the code) are listed in
//! `KNOWN_GAPS` with their question; every other case must pass.
#![cfg_attr(not(operandi_specs), allow(dead_code, unused_imports))]
use glam::{DVec3, dvec3};
use operandi_core::*;
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(operandi_specs)]
fn cases() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("OPERANDI_SPECS_DIR"),
        "/specifications/ac-symmetry-rulebook-2026-11-19/cases.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["cases"].as_array().unwrap().clone()
}

const Q4: &str = "ACS-Q4: moved item, the reference picked the primary side from the centre's world X; Operandi tests it against the item's plane (ACS-08)";
const Q5: &str = "ACR-Q09: AC-RULES v6 has every tool's typed values in the one A-SEL frame (the + side's, S-02), which the nine Selection / Selection Border Scale cases now match; this Rotate case was recorded turning about world Y instead";
const Q7: &str = "ACR-Q29: AC-RULES v6 A-LOCAL puts Z on the cluster's normal for every tool (the Local Scale case ACS-H-local-x-pair-scale was recorded that way); this typed Rotate Y 30 was recorded turning each island about its normal, which is Rotate Z under the sheet";
const Q6: &str = "ACR-Q10: A-SEL under Symmetry with a selected polygon lying across the plane: which of its points and whether its normal count (AC-RULES v6 S-02 doesn't say)";
/// Cases that depend on an open question (case id, question). A listed
/// case that passes still counts as a pass.
///
/// AC-RULES v6 (2026-11-27) took nine Selection and Selection Border Scale
/// cases off this list: A-SEL from the + side's points gives the frame
/// they were recorded with. Step 2 took the two Element Scale cases off:
/// A-ELEM's frame (Z the normal, X world X laid square) is theirs.
///
/// A-LOCAL (2026-11-28, Z the normal) took the Local Scale case off (it
/// was recorded with Z along the normal) and put the typed Local Rotate
/// case on (recorded with Y along it).
const KNOWN_GAPS: &[(&str, &str)] = &[
    ("ACS-HL-rotate-local-typedRY30", Q7),
    ("ACS-H-select-x-pair-rotate", Q5),
    ("ACS-H-select-x-PC-scale", Q6),
    ("ACS-H-select-x-pairstraddle-scale", Q6),
    ("ACS-P5-move-c0.7_0.25_0.5-x-pair", Q4),
    ("ACS-P5-rotate-c0.7_0.25_0.5-x-pair", Q4),
    ("ACS-P5-move-c0.2_0.25_0.5-x-pair", Q4),
    ("ACS-P5-rotate-c0.2_0.25_0.5-x-pair", Q4),
    ("ACS-P5-scale-c0.2_0.25_0.5-x-pair", Q4),
    ("ACS-P5-move-c0.7_0.25_0.5-x-oneside", Q4),
    ("ACS-P5-rotate-c0.7_0.25_0.5-x-oneside", Q4),
    ("ACS-P5-move-c0.2_0.25_0.5-x-oneside", Q4),
    ("ACS-P5-rotate-c0.2_0.25_0.5-x-oneside", Q4),
    ("ACS-P5-scale-c0.2_0.25_0.5-x-oneside", Q4),
];

fn v3(v: &Value) -> DVec3 {
    let a: Vec<f64> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    dvec3(a[0], a[1], a[2])
}
fn list(v: &Value) -> Vec<DVec3> {
    v.as_array().unwrap().iter().map(v3).collect()
}
fn num(values: &Value, key: &str, or: f64) -> f64 {
    values.get(key).and_then(Value::as_f64).unwrap_or(or)
}

fn symmetry_of(case: &Value) -> Symmetry {
    let s = &case["symmetry"];
    if s.is_string() {
        return Symmetry::OFF;
    }
    Symmetry {
        axis: Some(match s["axis"].as_str().unwrap() {
            "X" => 0,
            "Y" => 1,
            _ => 2,
        }),
        offset: s.get("offset").and_then(Value::as_f64).unwrap_or(0.),
        topology: s.get("topology").and_then(Value::as_bool).unwrap_or(false),
        ..Symmetry::OFF
    }
}

/// An editor holding the case's fixture (one mesh item, at its position;
/// the Parent case parents it under a locator), Polygon mode, the case's
/// selection and Symmetry.
fn editor(case: &Value) -> Editor {
    let input = &case["input"];
    let mut e = Editor::default();
    let parent = input
        .get("parent")
        .and_then(Value::as_str)
        .map(|_| dvec3(0.6, 0.3, 0.));
    let parent_id = parent.map(|at| {
        e.dispatch(Command::NewMesh).unwrap();
        let item = e.scene.active_mut().unwrap();
        item.name = "Locator".into();
        item.transform.translation = at;
        item.id
    });
    e.dispatch(Command::NewMesh).unwrap();
    let faces: Vec<Face> = input["polygons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| Face {
            vertices: p
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u32)
                .collect(),
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        })
        .collect();
    let item = e.scene.active_mut().unwrap();
    item.mesh = Mesh {
        vertices: list(&input["points"]),
        faces: faces.into(),
        ..Default::default()
    }
    .into();
    item.parent = parent_id;
    if let Some(at) = input.get("item_position").filter(|v| v.is_array()) {
        item.transform.translation = v3(at);
    }
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = match &case["selected_polygons"] {
        Value::Array(a) => a.iter().map(|x| x.as_u64().unwrap() as u32).collect(),
        _ => (0..e.scene.active().unwrap().mesh.faces.len() as u32).collect(),
    };
    e.dispatch(Command::Symmetry(symmetry_of(case))).unwrap();
    e
}

fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.to_vec()
}

/// Same points in the same order within `tolerance`.
fn same_order(got: &[DVec3], want: &[DVec3], tolerance: f64) -> std::result::Result<(), String> {
    if got.len() != want.len() {
        return Err(format!("{} points, want {}", got.len(), want.len()));
    }
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        if g.distance(*w) > tolerance {
            return Err(format!("point {i}: {g} want {w}"));
        }
    }
    Ok(())
}

/// Same points as a set within `tolerance` (tools that add points may
/// number them differently from the reference).
fn same_set(got: &[DVec3], want: &[DVec3], tolerance: f64) -> std::result::Result<(), String> {
    if got.len() != want.len() {
        return Err(format!("{} points, want {}", got.len(), want.len()));
    }
    let mut used = vec![false; got.len()];
    for w in want {
        match (0..got.len()).find(|&i| !used[i] && got[i].distance(*w) <= tolerance) {
            Some(i) => used[i] = true,
            None => return Err(format!("no point near {w}")),
        }
    }
    Ok(())
}

/// The typed transform of a Move/Rotate/Scale case.
fn transform_params(e: &Editor, tool: Tool, values: &Value) -> Parameters {
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = match tool {
        Tool::Move => dvec3(
            num(values, "Move X", 0.),
            num(values, "Move Y", 0.),
            num(values, "Move Z", 0.),
        ),
        Tool::Rotate => dvec3(
            num(values, "Rotate X (deg)", 0.),
            num(values, "Rotate Y (deg)", 0.),
            num(values, "Rotate Z (deg)", 0.),
        ),
        _ => dvec3(
            num(values, "Scale X", 1.),
            num(values, "Scale Y", 1.),
            num(values, "Scale Z", 1.),
        ),
    };
    p
}

fn tool_of(name: &str) -> Tool {
    match name {
        "Move" => Tool::Move,
        "Rotate" => Tool::Rotate,
        "Scale" => Tool::Scale,
        "Smooth Shift" => Tool::SmoothShift,
        "Thicken" => Tool::Thicken,
        "Bevel (polygon)" => Tool::Bevel,
        _ => Tool::Extrude,
    }
}

/// The Operandi-rule expected points of a non-oracle case (see the module
/// comment).
fn operandi_rule(case: &Value) -> Vec<DVec3> {
    let recorded = list(&case["expected_points_item_space"]);
    let input = list(&case["input"]["points"]);
    let symmetry = symmetry_of(case);
    let centre = v3(&case["centre_world"]);
    // The fixtures sit at the origin, so world and item space agree; the
    // matching tolerance is far below the fixture's spacing.
    let tolerance = 1e-9;
    let primary = match symmetry.side(centre, tolerance) {
        Some(Side::Positive) => Side::Positive,
        _ => Side::Negative,
    };
    let other = |p: DVec3| matches!(symmetry.side(p, tolerance), Some(s) if s != Side::Plane && s != primary);
    let tool = case["tool"].as_str().unwrap();
    recorded
        .iter()
        .enumerate()
        .map(|(i, &r)| {
            let moved = i >= input.len() || input[i].distance(r) > 1e-12;
            if !moved {
                return r;
            }
            if tool == "Extrude (polygon)" {
                let shift = num(&case["values"], "Shift X", 0.);
                return if other(r) {
                    r - dvec3(2. * shift, 0., 0.)
                } else {
                    r
                };
            }
            let s = num(&case["values"], "Scale", 1.);
            let c = match symmetry.side(r, tolerance) {
                None => centre,
                Some(Side::Plane) => symmetry.onto_plane(centre),
                Some(side) if side == primary => centre,
                Some(_) => symmetry.mirror(centre),
            };
            r + (1. - s) * c
        })
        .collect()
}

/// Runs one scripted case (typed centre); `Ok` when it matches.
fn run_typed(case: &Value) -> std::result::Result<(), String> {
    let tool = tool_of(case["tool"].as_str().unwrap());
    let mut e = editor(case);
    let values = &case["values"];
    let tolerance = case["tolerance"].as_f64().unwrap();
    let oracle = case.get("operandi_oracle").and_then(Value::as_bool) != Some(false);
    let want = if oracle {
        list(&case["expected_points_item_space"])
    } else {
        operandi_rule(case)
    };
    e.dispatch(Command::Activate(tool))
        .map_err(|x| x.to_string())?;
    // "Automatic, centre typed in Tool Properties": a placed centre (ACS-07).
    e.dispatch(Command::SetActionCenter(v3(&case["centre_world"])))
        .map_err(|x| x.to_string())?;
    match tool {
        Tool::Move | Tool::Rotate | Tool::Scale => {
            let p = transform_params(&e, tool, values);
            e.dispatch(Command::Preview(p)).map_err(|x| x.to_string())?;
        }
        Tool::Bevel => {
            let mut p = e.session.live.as_ref().unwrap().params.clone();
            p.shift = num(values, "Shift", 0.);
            p.inset = num(values, "Inset", 0.);
            e.dispatch(Command::Preview(p)).map_err(|x| x.to_string())?;
        }
        Tool::Extrude => {
            let mut p = e.session.live.as_ref().unwrap().params.clone();
            p.extent = dvec3(num(values, "Shift X", 0.), num(values, "Shift Y", 0.), 0.);
            e.dispatch(Command::Preview(p)).map_err(|x| x.to_string())?;
        }
        _ => {
            let mut p = e.session.live.as_ref().unwrap().params.clone();
            p.smooth_shift.offset = num(values, "Shift", 0.);
            p.smooth_shift.scale = num(values, "Scale", 1.);
            e.dispatch(Command::Preview(p)).map_err(|x| x.to_string())?;
        }
    }
    e.dispatch(Command::Drop).map_err(|x| x.to_string())?;
    let got = points(&e);
    let polygons = e.scene.active().unwrap().mesh.faces.len() as u64;
    if let Some(n) = case.get("expected_polygon_count").and_then(Value::as_u64)
        && polygons != n
    {
        return Err(format!("{polygons} polygons, want {n}"));
    }
    if matches!(tool, Tool::Move | Tool::Rotate | Tool::Scale) {
        same_order(&got, &want, tolerance)
    } else {
        same_set(&got, &want, tolerance)
    }
}

/// The hands-on centre placement cases: the Action Center the case names,
/// the click it describes, then the typed values.
fn run_hands_on(case: &Value) -> std::result::Result<(), String> {
    let mode = case["action_center"].as_str().unwrap();
    let id = case["id"].as_str().unwrap();
    let tool = tool_of(case["tool"].as_str().unwrap());
    let mut e = editor(case);
    let shown = case
        .get("centre_shown_world")
        .filter(|v| v.is_array())
        .map(v3);
    // A click on empty space at about (-3.94, *, 2.10) or (4.48, *, 2.10):
    // the click's X and Z, the selection centre's height (rulebook table).
    let click = if id.contains("clickNeg") {
        Some(dvec3(-3.94, 0.25, 2.1))
    } else if id.contains("clickPos") {
        Some(dvec3(4.48, 0.25, 2.1))
    } else {
        None
    };
    let mesh = e.scene.active().unwrap().mesh.clone();
    let world = |c: CenterMode| ActionContext {
        center: c,
        axis: AxisOrientation::World,
    };
    let context = match mode {
        "Automatic" => world(CenterMode::Automatic),
        "None" => world(CenterMode::None),
        "Selection" => ActionContext {
            center: CenterMode::Selection,
            axis: AxisOrientation::Selection,
        },
        "Selection Center Auto Axis" => world(CenterMode::SelectionAutoAxis),
        "Selection Border" => ActionContext {
            center: CenterMode::SelectionBorder,
            axis: AxisOrientation::Selection,
        },
        "Origin" => world(CenterMode::Origin),
        "Local" => ActionContext::LOCAL,
        "Parent" => ActionContext {
            center: CenterMode::Parent,
            axis: AxisOrientation::Parent,
        },
        "Pivot" => ActionContext {
            center: CenterMode::Pivot,
            axis: AxisOrientation::Item,
        },
        "Screen" => world(CenterMode::Screen(PickedPoint::new(shown.unwrap()))),
        "Element" => {
            // The clicked polygon: its centre and its normal.
            let at = shown.unwrap();
            let f = (0..mesh.faces.len())
                .find(|&f| mesh.center(f).distance(at) < 1e-6)
                .ok_or("no polygon at the shown centre")?;
            let item = e.scene.active().unwrap().id;
            let pick = element_frame(&e.scene, item, TweakElement::Polygon(f as u32))
                .map_err(|x| x.to_string())?;
            ActionContext::ELEMENT_UNPICKED.picked(pick)
        }
        other => return Err(format!("unknown mode {other}")),
    };
    e.dispatch(Command::ActionContext(context))
        .map_err(|x| x.to_string())?;
    e.dispatch(Command::Activate(tool))
        .map_err(|x| x.to_string())?;
    if let Some(at) = click.filter(|_| matches!(mode, "Automatic" | "None")) {
        e.dispatch(Command::SetActionCenter(at))
            .map_err(|x| x.to_string())?;
    }
    if let Some(want) = shown {
        let got = e
            .session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center;
        if got.distance(want) > 1e-3 {
            return Err(format!("centre {got} want {want}"));
        }
    }
    let p = transform_params(&e, tool, &case["values_typed_after_activation"]);
    e.dispatch(Command::Preview(p)).map_err(|x| x.to_string())?;
    e.dispatch(Command::Drop).map_err(|x| x.to_string())?;
    same_order(&points(&e), &list(&case["points_after_item_space"]), 1e-3)
}

/// The three Local per-island cases (ACS-10/11).
fn run_local(case: &Value) -> std::result::Result<(), String> {
    let id = case["id"].as_str().unwrap();
    let tool = tool_of(case["tool"].as_str().unwrap());
    let mut e = editor(case);
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Local,
        axis: AxisOrientation::Local,
    }))
    .map_err(|x| x.to_string())?;
    e.dispatch(Command::Activate(tool))
        .map_err(|x| x.to_string())?;
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = if id.ends_with("dragX") {
        dvec3(6.08, 1., 1.)
    } else if id.ends_with("typedRY30") {
        dvec3(0., 30., 0.)
    } else {
        // "About 113.8 degrees in all": a free turn whose axis the case
        // doesn't record. Island A (flat, square; its A-LOCAL frame is Z
        // up its normal, X the item's X laid into it, Y = Z × X) shows the
        // turn; it is taken from A's points, read in A's frame, and every
        // other island must take the same turn in its own frame (ACS-11).
        let before = list(&case["input"]["points"]);
        let after = list(&case["expected_points"]);
        let frame = |p: &[DVec3]| {
            let u = (p[1] - p[0]).normalize();
            let w = u.cross(p[3] - p[0]).normalize();
            glam::DMat3::from_cols(u, w, u.cross(w))
        };
        let turn = frame(&after[0..4]) * frame(&before[0..4]).transpose();
        let a_axes = glam::DMat3::from_cols(DVec3::X, -DVec3::Z, DVec3::Y);
        let turn = a_axes.transpose() * turn * a_axes;
        let (x, y, z) = glam::DQuat::from_mat3(&turn).to_euler(glam::EulerRot::XYZ);
        let angle = glam::DQuat::from_mat3(&turn).to_axis_angle().1.to_degrees();
        let angle = angle.min(360. - angle);
        if (angle - 113.8).abs() > 0.5 {
            return Err(format!("island A turned {angle} degrees"));
        }
        dvec3(x.to_degrees(), y.to_degrees(), z.to_degrees())
    };
    e.dispatch(Command::Preview(p)).map_err(|x| x.to_string())?;
    e.dispatch(Command::Drop).map_err(|x| x.to_string())?;
    let tolerance = case["tolerance"].as_f64().unwrap_or(1e-3);
    same_order(&points(&e), &list(&case["expected_points"]), tolerance)
}

fn group(case: &Value) -> &'static str {
    match case.get("kind").and_then(Value::as_str) {
        Some(k) if k.starts_with("Local") => "Local per-island",
        Some(_) => "hands-on centre placement",
        None => match case["tool"].as_str().unwrap() {
            "Move" | "Rotate" | "Scale" => "typed centre transforms",
            "Smooth Shift" | "Thicken" => "Smooth Shift / Thicken (Operandi rule)",
            _ => "Bevel / Extrude",
        },
    }
}

#[test]
#[cfg(operandi_specs)]
fn every_rulebook_case() {
    let all = cases();
    assert_eq!(all.len(), 311);
    let mut tally: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    let mut failures = Vec::new();
    for case in &all {
        let id = case["id"].as_str().unwrap();
        let result = match case.get("kind").and_then(Value::as_str) {
            Some(k) if k.starts_with("Local") => run_local(case),
            Some(_) => run_hands_on(case),
            None => run_typed(case),
        };
        let gap = KNOWN_GAPS.iter().find(|(gap, _)| id == *gap);
        let entry = tally.entry(group(case)).or_default();
        match (&result, gap) {
            (Ok(()), _) => entry.0 += 1,
            (Err(_), Some(_)) => entry.1 += 1,
            (Err(why), None) => {
                entry.2 += 1;
                failures.push(format!("{id}: {why}"));
            }
        }
    }
    for (name, (pass, gap, fail)) in &tally {
        println!("{name}: {pass} pass, {gap} known gap, {fail} fail");
    }
    for (gap, question) in KNOWN_GAPS {
        println!("known gap {gap}: {question}");
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The rulebook's fixture with the pair (P, N) selected, for the focused
/// tests below.
#[cfg(operandi_specs)]
fn pair_editor(symmetry: Symmetry) -> Editor {
    let case = cases()
        .into_iter()
        .find(|c| c["id"] == "ACS-H-select-x-pair-scale")
        .unwrap();
    let mut e = editor(&case);
    e.dispatch(Command::Symmetry(symmetry)).unwrap();
    e
}

/// The service's answer is the handle plus the groups (AC-RULES v6). Off:
/// one group driving every selected point as entered. Symmetry X: still
/// one group, about the primary pivot on the + half (S-01); its targets
/// carry their role, the + quad's points primary and the − quad's the
/// other side, which are edited through the mirror (S-04), so their pivot
/// is the mirror image of the primary one.
#[test]
#[cfg(operandi_specs)]
fn the_service_lists_one_group_with_each_points_role() {
    use operandi_core::pivot_service::{GroupId, Role};
    let e = pair_editor(Symmetry::OFF);
    let off = e.pivot_request(&e.scene, Tool::Scale).resolve().unwrap();
    assert!(off.primary.is_none());
    assert_eq!(off.items.len(), 1);
    let group = &off.items[0].groups[0];
    assert_eq!((off.items[0].groups.len(), group.id), (1, GroupId::Whole));
    assert_eq!(group.targets.len(), 8);
    assert!(
        group
            .targets
            .iter()
            .all(|t| t.role == Role::Primary && t.side.is_none())
    );
    assert!(group.frame.center.distance(dvec3(0., 0.25, 0.5)) < 1e-9);
    assert_eq!(off.handle, group.frame);

    let e = pair_editor(Symmetry::x());
    let on = e.pivot_request(&e.scene, Tool::Scale).resolve().unwrap();
    assert_eq!(on.primary, Some(Side::Positive));
    let item = &on.items[0];
    let group = &item.groups[0];
    assert_eq!((item.groups.len(), group.id), (1, GroupId::Whole));
    assert!(group.frame.center.distance(dvec3(1.5, 0.25, 0.5)) < 1e-9);
    assert_eq!(group.twin, group.frame);
    assert_eq!(on.handle, group.frame);
    let mirrored = item.map.mirror_frame(group.frame);
    assert!(mirrored.center.distance(dvec3(-1.5, 0.25, 0.5)) < 1e-9);
    let with = |role: Role, side: Side| -> Vec<u32> {
        group
            .targets
            .iter()
            .filter(|t| t.role == role && t.side == Some(side))
            .map(|t| t.point)
            .collect()
    };
    assert_eq!(with(Role::Primary, Side::Positive), [0, 1, 2, 3]);
    assert_eq!(with(Role::Other, Side::Negative), [4, 5, 6, 7]);
}

/// ACS-20 with Symmetry and no placed centre: Smooth Shift's Scale works
/// about the + half's centre and its mirror image, so each cap shrinks
/// about its own centre and the result stays symmetric; Cancel leaves the
/// scene as it was; undo and redo agree.
#[test]
#[cfg(operandi_specs)]
fn smooth_shift_scales_each_side_about_its_own_pivot() {
    let mut e = pair_editor(Symmetry::x());
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::SmoothShift)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.smooth_shift.scale = 0.5;
    e.dispatch(Command::Preview(p.clone())).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Activate(Tool::SmoothShift)).unwrap();
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    assert!(symmetry::unpaired_points(mesh, Symmetry::x(), 1e-9).is_empty());
    // The + cap's new corners sit half way to the + quad's centre.
    for corner in [dvec3(1.25, 0.125, 0.25), dvec3(1.75, 0.375, 0.75)] {
        assert!(
            mesh.vertices.iter().any(|p| p.distance(corner) < 1e-9),
            "{corner}"
        );
    }
    let after = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
}

/// ACS-01, Selection Border with Symmetry (Jason's hand check, 2026-11-19):
/// a whole row of quads across the plane puts the handle on the + half's
/// border, not on the plane.
#[test]
fn selection_border_takes_the_plus_halfs_border() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    // A row of four quads from x = -2 to 2 (point = row × 5 + column).
    let mut m = Mesh::default();
    for z in 0..2 {
        for x in 0..5 {
            m.vertices.push(dvec3(x as f64 - 2., 1., z as f64));
        }
    }
    for x in 0..4u32 {
        m.faces.push(Face {
            vertices: vec![x, x + 5, x + 6, x + 1],
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        });
    }
    e.scene.active_mut().unwrap().mesh = m.into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0, 1, 2, 3];
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::SelectionBorder,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    let c = e
        .session
        .live
        .as_ref()
        .unwrap()
        .action_frame
        .unwrap()
        .center;
    // The + half's border points: x = 1 and 2 on both rows.
    assert!(c.distance(dvec3(1.5, 1., 0.5)) < 1e-9, "{c}");
}

/// ACS-07 / ACS-21: a centre placed on the − side makes − the primary
/// side, so an Extrude's sideways shift applies as entered there and
/// mirrored on the + side; placed on + it is the other way round.
#[test]
#[cfg(operandi_specs)]
fn a_placed_centre_picks_the_extrude_side() {
    for (at, minus_shift) in [(-1.5, 0.3), (1.5, -0.3)] {
        let mut e = pair_editor(Symmetry::x());
        e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        e.dispatch(Command::SetActionCenter(dvec3(at, 0.25, 0.5)))
            .unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.extent = dvec3(0.3, 0.2, 0.);
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Drop).unwrap();
        let mesh = &e.scene.active().unwrap().mesh;
        // The − quad's corner (-1, 0, 0) moved by the shift for its side.
        let want = dvec3(-1. + minus_shift, 0.2, 0.);
        assert!(
            mesh.vertices.iter().any(|p| p.distance(want) < 1e-9),
            "{at}: {want}"
        );
        assert!(symmetry::unpaired_points(mesh, Symmetry::x(), 1e-9).is_empty());
    }
}

/// A strip of three quads along X (x = −1.5..1.5, z = 0..1): point
/// `row * 4 + col` sits at x = −1.5 + col.
fn strip_scene() -> Scene {
    let mut s = Scene::default();
    s.new_mesh().unwrap();
    let vertices = (0..2)
        .flat_map(|row| (0..4).map(move |col| dvec3(-1.5 + col as f64, 0., row as f64)))
        .collect();
    let quad = |c: u32| Face {
        vertices: vec![c, c + 1, c + 5, c + 4],
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    };
    s.items[0].mesh = Mesh {
        vertices,
        faces: vec![quad(0), quad(1), quad(2)].into(),
        ..Default::default()
    }
    .into();
    s
}

/// ACS-Q-HANDLE1 (answered): Edge Bevel / Extrude / Chamfer put the handle
/// at the selection's bounding-box centre, whatever the pick order; with
/// Symmetry on, the primary (+) side's box. Edge Slide sits on the
/// first-picked edge. The other edge tools (Edge Weight, unchecked,
/// ACS-Q-HANDLE2) keep the edge nearest the centre.
#[test]
fn edge_handles_follow_each_tools_rule() {
    let at = |edges: Vec<(u32, u32)>, symmetry: Symmetry, pick: HandlePick| {
        let mut s = strip_scene();
        s.selection.mode = Mode::Edge;
        s.selection.edges = edges;
        edge_handle_frame(&s, symmetry, pick).unwrap().0
    };
    let slide = |edges: Vec<(u32, u32)>, symmetry: Symmetry| {
        let mut s = strip_scene();
        s.selection.mode = Mode::Edge;
        s.selection.edges = edges;
        edge_slide_handle_frame(&s, symmetry).unwrap().0
    };
    assert_eq!(handle_pick(Tool::EdgeBevel), HandlePick::BoxCentre);
    assert_eq!(handle_pick(Tool::EdgeSlide), HandlePick::FirstPicked);
    assert_eq!(handle_pick(Tool::EdgeCrease), HandlePick::Nearest);
    // The vertical edges at x = −0.5, 0.5 and 1.5: the box centre is 0.5.
    let one = vec![(1, 5), (2, 6), (3, 7)];
    let other = vec![(7, 3), (1, 5), (2, 6)];
    let a = at(one.clone(), Symmetry::OFF, HandlePick::BoxCentre);
    assert_eq!(a, at(other.clone(), Symmetry::OFF, HandlePick::BoxCentre));
    assert!(a.distance(dvec3(0.5, 0., 0.5)) < 1e-9, "{a}");
    // Edge x = −0.5 and 1.5 only: the box centre sits between them, on no
    // edge.
    let b = at(vec![(3, 7), (1, 5)], Symmetry::OFF, HandlePick::BoxCentre);
    assert!(b.distance(dvec3(0.5, 0., 0.5)) < 1e-9, "{b}");
    // Symmetry X, edges at x = −1.5 and 1.5: the + side, either order.
    for edges in [vec![(0, 4), (3, 7)], vec![(3, 7), (0, 4)]] {
        let p = at(edges, Symmetry::x(), HandlePick::BoxCentre);
        assert!(p.distance(dvec3(1.5, 0., 0.5)) < 1e-9, "{p}");
    }
    // Edge Slide: the first-picked edge.
    assert!(slide(one, Symmetry::OFF).distance(dvec3(-0.5, 0., 0.5)) < 1e-9);
    assert!(slide(other, Symmetry::OFF).distance(dvec3(1.5, 0., 0.5)) < 1e-9);
    // Nearest: the edge nearest the box centre.
    let n = at(
        vec![(3, 7), (1, 5), (2, 6)],
        Symmetry::OFF,
        HandlePick::Nearest,
    );
    assert!(n.distance(dvec3(0.5, 0., 0.5)) < 1e-9, "{n}");
}

/// ACS-Q-HANDLE1 (answered): Vertex Extrude at the box centre, Vertex
/// Slide on the first-picked point; the rest on the point nearest the
/// centre (lowest index on a tie).
#[test]
fn vertex_handles_follow_each_tools_rule() {
    let at = |points: Vec<u32>, symmetry: Symmetry, pick: HandlePick| {
        let mut s = strip_scene();
        s.selection.mode = Mode::Vertex;
        s.selection.vertices = points;
        vertex_handle_frame(&s, symmetry, pick).unwrap().0
    };
    assert_eq!(handle_pick(Tool::VertexExtrude), HandlePick::BoxCentre);
    assert_eq!(handle_pick(Tool::VertexSlide), HandlePick::FirstPicked);
    let a = at(vec![3, 1], Symmetry::OFF, HandlePick::BoxCentre);
    assert_eq!(a, at(vec![1, 3], Symmetry::OFF, HandlePick::BoxCentre));
    assert!(a.distance(dvec3(0.5, 0., 0.)) < 1e-9, "{a}");
    for points in [vec![0, 3], vec![3, 0]] {
        let p = at(points, Symmetry::x(), HandlePick::BoxCentre);
        assert!(p.distance(dvec3(1.5, 0., 0.)) < 1e-9, "{p}");
    }
    // Vertex Slide: the first picked; under Symmetry, of the + side.
    let f = at(vec![3, 1, 2], Symmetry::OFF, HandlePick::FirstPicked);
    assert!(f.distance(dvec3(1.5, 0., 0.)) < 1e-9, "{f}");
    let f = at(vec![1, 2, 3], Symmetry::OFF, HandlePick::FirstPicked);
    assert!(f.distance(dvec3(-0.5, 0., 0.)) < 1e-9, "{f}");
    let f = at(vec![0, 3], Symmetry::x(), HandlePick::FirstPicked);
    assert!(f.distance(dvec3(1.5, 0., 0.)) < 1e-9, "{f}");
    // Nearest, on a tie: the lower index.
    let t = at(vec![2, 1], Symmetry::OFF, HandlePick::Nearest);
    assert!(t.distance(dvec3(-0.5, 0., 0.)) < 1e-9, "{t}");
}
