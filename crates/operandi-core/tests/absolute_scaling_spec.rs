//! Absolute Scaling against `specifications/snaps-precision-2026-11-10`
//! (`cases.json`, AS-01..38): points and the fields afterwards.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

/// Cases whose expected points are the reference's, not Operandi's.
const OWN: &[(&str, &str)] = &[
    ("AS-11", "Q2 decided: flat axes are skipped, not collapsed"),
    ("AS-13", "Q2 decided: flat axes are skipped, not collapsed"),
    ("AS-17", "Q2 decided: flat axes are skipped, not collapsed"),
    ("AS-25", "Q2 decided: flat axes are skipped, not collapsed"),
    ("AS-29", "Q2 decided: flat axes are skipped, not collapsed"),
    ("AS-34", "Q2 decided: flat axes are skipped, not collapsed"),
    (
        "AS-18",
        "Q4 provisional: Orient fits flat selections too (the reference did nothing)",
    ),
];

fn all() -> Vec<Value> {
    let path = format!(
        "{}/../../specifications/snaps-precision-2026-11-10/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn fields(f: &Value) -> AbsoluteFields {
    let num = |k: &str| f[k].as_f64().unwrap_or(0.);
    let center = |k: &str| match f[k].as_str().unwrap_or("Center") {
        "High" => ScaleCenter::High,
        "Low" => ScaleCenter::Low,
        "Origin" => ScaleCenter::Origin,
        _ => ScaleCenter::Center,
    };
    AbsoluteFields {
        uniform: num("Uniform"),
        axis: match f["Axis"].as_str().unwrap_or("Longest Axis") {
            "X Axis" => UniformAxis::X,
            "Y Axis" => UniformAxis::Y,
            "Z Axis" => UniformAxis::Z,
            _ => UniformAxis::Longest,
        },
        orient: f["Orient To Selection"].as_bool().unwrap_or(false),
        size: DVec3::new(num("X"), num("Y"), num("Z")),
        center: [center("X Center"), center("Y Center"), center("Z Center")],
    }
}

fn mode(button: &str) -> AbsoluteMode {
    match button {
        "Grab Size" => AbsoluteMode::Grab,
        "Uniform Scale" => AbsoluteMode::Uniform,
        "Explicit Scale" => AbsoluteMode::Explicit,
        "Reference Scale" => AbsoluteMode::Reference,
        other => panic!("button {other}"),
    }
}

/// The case's scene with its selection, and the mesh items by case name.
fn setup(c: &Value) -> (Editor, Vec<(String, u64)>) {
    let input = &c["input"];
    let mut e = Editor::default();
    let mut names = Vec::new();
    if let Some(meshes) = input["meshes"].as_object() {
        for (name, m) in meshes {
            e.scene.new_mesh().unwrap();
            let item = e.scene.active_mut().unwrap();
            item.mesh = mesh(m).into();
            names.push((name.clone(), item.id));
        }
    } else {
        e.scene.new_mesh().unwrap();
        let item = e.scene.active_mut().unwrap();
        item.mesh = mesh(input).into();
        if let Some(t) = input.get("item_transform") {
            item.transform.translation = point(&t["position"]);
            item.transform.scale = point(&t["scale"]);
        }
        names.push(("mesh".into(), item.id));
    }
    e.synchronize_identities().unwrap();
    let sel = &input["selection"];
    let m = match sel["mode"].as_str().unwrap() {
        "vertex" => Mode::Vertex,
        "edge" => Mode::Edge,
        "item" => Mode::Item,
        _ => Mode::Polygon,
    };
    e.dispatch(Command::Mode(m)).unwrap();
    if m == Mode::Item {
        let ids: Vec<u64> = sel["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| {
                names
                    .iter()
                    .find(|(k, _)| k == n.as_str().unwrap())
                    .unwrap()
                    .1
            })
            .collect();
        e.scene.selection.item = Some(ids[0]);
        e.scene.selection.also = ids[1..].to_vec();
    } else {
        e.scene.selection.faces = indices(&sel["polygons"]);
        e.scene.selection.vertices = indices(&sel["vertices"]);
        e.scene.selection.edges = sel["edges"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|x| {
                let x = indices(x);
                (x[0], x[1])
            })
            .collect();
    }
    (e, names)
}

fn press(e: &mut Editor, c: &Value) -> AbsoluteFields {
    let scale = AbsoluteScale {
        mode: mode(c["settings"]["button"].as_str().unwrap()),
        fields: fields(&c["settings"]["fields"]),
    };
    e.dispatch(Command::AbsoluteScale(scale)).unwrap();
    e.absolute_fields_after(scale.fields).unwrap()
}

fn points_of(e: &Editor, id: u64) -> Vec<DVec3> {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .mesh
        .vertices
        .clone()
}

fn assert_points(id: &str, got: &[DVec3], want: &[DVec3], tol: f64) {
    assert_eq!(got.len(), want.len(), "{id}: point count");
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!(
            (*g - *w).abs().max_element() <= tol,
            "{id}: point {i} is {g}, expected {w}"
        );
    }
}

fn assert_fields(id: &str, got: AbsoluteFields, want: AbsoluteFields) {
    let tol = 1e-4;
    assert!(
        (got.uniform - want.uniform).abs() <= tol,
        "{id}: Uniform {} expected {}",
        got.uniform,
        want.uniform
    );
    assert!(
        (got.size - want.size).abs().max_element() <= tol,
        "{id}: X Y Z {} expected {}",
        got.size,
        want.size
    );
}

#[test]
fn spec_cases_match() {
    let mut checked = 0;
    for c in all() {
        let id = c["id"].as_str().unwrap();
        if !id.starts_with("AS-") || OWN.iter().any(|(p, _)| id.starts_with(p)) {
            continue;
        }
        let tol = c["tolerance"].as_f64().unwrap_or(1e-5);
        let (mut e, names) = setup(&c);
        let polygons: Vec<_> = names
            .iter()
            .map(|(_, id)| {
                let item = e.scene.items.iter().find(|i| i.id == *id).unwrap();
                item.mesh.faces.clone()
            })
            .collect();
        let after = press(&mut e, &c);
        let expected = &c["expected"];
        if let Some(meshes) = expected["meshes"].as_object() {
            for (name, m) in meshes {
                let item = names.iter().find(|(k, _)| k == name).unwrap().1;
                assert_points(id, &points_of(&e, item), &points(&m["points"]), tol);
            }
        } else {
            assert_points(
                id,
                &points_of(&e, names[0].1),
                &points(&expected["points"]),
                tol,
            );
        }
        // Topology never changes (A7).
        for ((_, item), faces) in names.iter().zip(&polygons) {
            let now = &e.scene.items.iter().find(|i| i.id == *item).unwrap().mesh;
            assert_eq!(&now.faces, faces, "{id}: polygons changed");
        }
        assert_fields(id, after, fields(&expected["fields_after"]));
        checked += 1;
    }
    assert_eq!(checked, 38 - OWN.len());
}

fn case(id: &str) -> Value {
    all()
        .into_iter()
        .find(|c| c["id"].as_str().unwrap().starts_with(id))
        .unwrap()
}

/// Operandi's own answers for the flat cases (Q2: skip flat axes).
#[test]
fn flat_axes_are_left_alone() {
    // AS-34: Uniform 4 on the box's flat top face (2 × 0 × 0.5, y = 1):
    // ×2 in X and Z about the face's middle, Y unchanged.
    let c = case("AS-34");
    let (mut e, names) = setup(&c);
    let before = points_of(&e, names[0].1);
    let face: Vec<u32> = e.scene.active().unwrap().mesh.faces[3].vertices.clone();
    let after = press(&mut e, &c);
    let now = points_of(&e, names[0].1);
    for (v, (b, n)) in before.iter().zip(&now).enumerate() {
        if face.contains(&(v as u32)) {
            let want = DVec3::new(2. + (b.x - 2.) * 2., b.y, 0.25 + (b.z - 0.25) * 2.);
            assert!((*n - want).abs().max_element() < 1e-9, "point {v}: {n}");
        } else {
            assert_eq!(n, b, "point {v} outside the face moved");
        }
    }
    assert_fields(
        "AS-34",
        after,
        AbsoluteFields {
            uniform: 4.,
            size: DVec3::new(4., 0., 1.),
            ..Default::default()
        },
    );

    // AS-13 / AS-29: Reference with A's flat top face: ×2 in X and Z on the
    // whole mesh about its box middle (3, 0.5, 0.5); Y stays put whatever
    // the Y target.
    for id in ["AS-13", "AS-29"] {
        let c = case(id);
        let (mut e, names) = setup(&c);
        let before = points_of(&e, names[0].1);
        press(&mut e, &c);
        for (b, n) in before.iter().zip(points_of(&e, names[0].1)) {
            let want = DVec3::new(3. + (b.x - 3.) * 2., b.y, 0.5 + (b.z - 0.5) * 2.);
            assert!((n - want).abs().max_element() < 1e-9, "{id}: {n} vs {want}");
        }
    }

    // AS-25: one edge along X, Uniform 4 Longest: the edge grows to 4 long
    // about its middle; it stays on y = 0, z = 0.
    let c = case("AS-25");
    let (mut e, names) = setup(&c);
    press(&mut e, &c);
    let p = points_of(&e, names[0].1);
    assert!((p[0] - DVec3::new(0., 0., 0.)).length() < 1e-9, "{}", p[0]);
    assert!((p[1] - DVec3::new(4., 0., 0.)).length() < 1e-9, "{}", p[1]);

    // AS-17: the flat diamond, Uniform 2 without Orient: ×(2 / 1.414) in X
    // and Y, still flat.
    let c = case("AS-17");
    let (mut e, names) = setup(&c);
    let after = press(&mut e, &c);
    assert!((after.size - DVec3::new(2., 2., 0.)).abs().max_element() < 1e-5);
    assert!(points_of(&e, names[0].1).iter().all(|p| p.z == 0.));
}

/// AS-11: the uneven flat strip (Z = 0): Uniform 4 grows it about its box
/// middle in X and Y; the vertex average would give another centre.
#[test]
fn uneven_flat_strip_scales_about_its_box_middle() {
    let c = case("AS-11");
    let (mut e, names) = setup(&c);
    let before = points_of(&e, names[0].1);
    let measured = e.absolute_box(false).unwrap();
    let mid = (measured.min + measured.max) / 2.;
    let f = 4. / measured.size().max_element();
    press(&mut e, &c);
    for (b, n) in before.iter().zip(points_of(&e, names[0].1)) {
        let want = DVec3::new(mid.x + (b.x - mid.x) * f, mid.y + (b.y - mid.y) * f, b.z);
        assert!((n - want).abs().max_element() < 1e-9, "{n} vs {want}");
    }
}

/// Uniform with a flat Uniform Axis does nothing and says why.
#[test]
fn uniform_on_a_flat_axis_refuses() {
    let c = case("AS-34");
    let (mut e, names) = setup(&c);
    let before = points_of(&e, names[0].1);
    let undo = e.undo.len();
    let scale = AbsoluteScale {
        mode: AbsoluteMode::Uniform,
        fields: AbsoluteFields {
            uniform: 3.,
            axis: UniformAxis::Y,
            ..Default::default()
        },
    };
    let error = e.dispatch(Command::AbsoluteScale(scale)).unwrap_err();
    assert!(error.contains("no size along Y Axis"), "{error}");
    assert_eq!(points_of(&e, names[0].1), before);
    assert_eq!(e.undo.len(), undo);
}

/// AS-18 (Q4, provisional): Orient fits the flat diamond's own square, so
/// Grab reads 1 × 1 × 0 and Uniform 2 doubles it in its plane.
#[test]
fn orient_fits_a_flat_turned_square() {
    let c = case("AS-18");
    let (mut e, names) = setup(&c);
    let measured = e.absolute_box(true).unwrap();
    assert!(
        (measured.size() - DVec3::new(1., 1., 0.))
            .abs()
            .max_element()
            < 1e-5
    );
    let before = points_of(&e, names[0].1);
    let after = press(&mut e, &c);
    for (b, n) in before.iter().zip(points_of(&e, names[0].1)) {
        assert!((n - *b * 2.).length() < 1e-5, "{n}");
    }
    assert!((after.size - DVec3::new(2., 2., 0.)).abs().max_element() < 1e-5);
}

/// One press is one undo step; Undo puts every point back, Redo repeats
/// it; Grab Size records nothing.
#[test]
fn one_press_is_one_undo_step() {
    let c = case("AS-30");
    let (mut e, names) = setup(&c);
    let before = points_of(&e, names[0].1);
    let undo = e.undo.len();
    press(&mut e, &c);
    let after = points_of(&e, names[0].1);
    assert_ne!(after, before);
    assert_eq!(e.undo.len(), undo + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points_of(&e, names[0].1), before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(points_of(&e, names[0].1), after);
    let grab = AbsoluteScale {
        mode: AbsoluteMode::Grab,
        fields: AbsoluteFields::default(),
    };
    let steps = e.undo.len();
    e.dispatch(Command::AbsoluteScale(grab)).unwrap();
    assert_eq!(e.undo.len(), steps);
}

/// Explicit sizes equal to the current ones record no step.
#[test]
fn nothing_to_move_is_no_edit() {
    let c = case("AS-15");
    let (mut e, _) = setup(&c);
    let undo = e.undo.len();
    let scale = AbsoluteScale {
        mode: AbsoluteMode::Explicit,
        fields: AbsoluteFields {
            size: DVec3::new(2., 1., 0.5),
            ..Default::default()
        },
    };
    e.dispatch(Command::AbsoluteScale(scale)).unwrap();
    assert_eq!(e.undo.len(), undo);
}

/// Nothing to measure (no mesh) is an error, not a panic; so are
/// non-finite sizes.
#[test]
fn invalid_requests_are_refused() {
    let mut e = Editor::default();
    let grab = AbsoluteScale {
        mode: AbsoluteMode::Explicit,
        fields: AbsoluteFields {
            size: DVec3::ONE,
            ..Default::default()
        },
    };
    assert!(e.dispatch(Command::AbsoluteScale(grab)).is_err());
    let c = case("AS-08");
    let (mut e, names) = setup(&c);
    let before = points_of(&e, names[0].1);
    let bad = AbsoluteScale {
        mode: AbsoluteMode::Explicit,
        fields: AbsoluteFields {
            size: DVec3::new(f64::NAN, 1., 1.),
            ..Default::default()
        },
    };
    assert!(e.dispatch(Command::AbsoluteScale(bad)).is_err());
    assert_eq!(points_of(&e, names[0].1), before);
}

/// The scaled points survive a save and reopen.
#[test]
fn result_survives_save_and_reopen() {
    let c = case("AS-08");
    let (mut e, names) = setup(&c);
    press(&mut e, &c);
    let after = points_of(&e, names[0].1);
    let text = serde_json::to_string(&e.scene).unwrap();
    let reopened: Scene = serde_json::from_str(&text).unwrap();
    let mesh = &reopened
        .items
        .iter()
        .find(|i| i.id == names[0].1)
        .unwrap()
        .mesh;
    assert_eq!(mesh.vertices, after);
}

/// ABS-Q12 (Jason, 2026-11-11): Origin keeps the geometry in place around
/// the item's origin. A box from x 1..3 scaled ×2 on X about the origin
/// goes to 2..6, and about the origin on every axis nothing drifts off it.
#[test]
fn origin_center_scales_about_the_item_origin() {
    let c = case("AS-08");
    let (mut e, names) = setup(&c);
    let before = points_of(&e, names[0].1);
    let scale = AbsoluteScale {
        mode: AbsoluteMode::Explicit,
        fields: AbsoluteFields {
            size: DVec3::new(4., 2., 1.),
            center: [ScaleCenter::Origin; 3],
            ..Default::default()
        },
    };
    e.dispatch(Command::AbsoluteScale(scale)).unwrap();
    for (b, n) in before.iter().zip(points_of(&e, names[0].1)) {
        assert!((n - *b * 2.).length() < 1e-9, "{n} vs {}", *b * 2.);
    }
    // The item's own position never changes.
    let item = e.scene.items.iter().find(|i| i.id == names[0].1).unwrap();
    assert_eq!(item.transform.translation, DVec3::ZERO);
    // The script name works too.
    let (_, f) = operandi_core::command_line::absolute_scale_arguments(
        &["explicit".into(), "absolute_size_center_y:originY".into()],
        AbsoluteFields::default(),
    )
    .unwrap();
    assert_eq!(f.center[1], ScaleCenter::Origin);
}
