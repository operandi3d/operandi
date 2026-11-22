//! MP-C13 SO-C01-A1: Scale Offset distance is physical scene distance under
//! the composed item/parent transform, including finite invertible scale and
//! reflection.
//!
//! Local expectations come from the admitted original fixture `SO-AFF-F01`
//! (`AFFINE_ORIGINAL_FIXTURES.json`) or are derived by hand here. The sheared
//! case uses the parallelogram relation: lines `d` apart in the scene are
//! `d * local_width / scene_width` apart locally. Separately,
//! `assert_scene_distance` maps source and result through
//! `Scene::world_matrix` and measures the scene-space perpendicular distance of
//! every boundary line. No expectation is produced by the operation itself.
use glam::{DMat3, DMat4, DVec3};
use operandi_core::*;

fn square() -> Vec<DVec3> {
    vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(2., 2., 0.),
        DVec3::new(0., 2., 0.),
    ]
}

/// Square face 0 on item `child`, optionally parented to a second item.
/// Returns the editor with the child active and its face selected.
fn scene(child: Transform, parent: Option<Transform>) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = Mesh {
        vertices: square(),
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: "Tile".into(),
            smooth: true,
            part: Some("Front".into()),
            material_label: None,
        }]
        .into(),
        ..Default::default()
    };
    *e.scene.active_mut().unwrap().mesh = mesh;
    e.scene.active_mut().unwrap().transform = child;
    let child_id = e.scene.active().unwrap().id;
    if let Some(parent) = parent {
        e.dispatch(Command::NewMesh).unwrap();
        let p = e.scene.active_mut().unwrap();
        p.name = "Parent".into();
        p.transform = parent;
        let parent_id = p.id;
        e.scene
            .items
            .iter_mut()
            .find(|i| i.id == child_id)
            .unwrap()
            .parent = Some(parent_id);
    }
    e.scene.selection.item = Some(child_id);
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.scene.selection.vertices.clear();
    e.scene.selection.edges.clear();
    e.scene.validate().unwrap();
    e.synchronize_identities().unwrap();
    e
}

fn scaled(scale: [f64; 3]) -> Transform {
    Transform {
        scale: DVec3::from(scale),
        ..Default::default()
    }
}

fn offset(e: &mut Editor, d: f64) -> Result<()> {
    e.dispatch(Command::ScaleOffsetPolygon(d))
}

fn child_local(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}

/// Full scene equality with `tolerance` allowed on the child's coordinates.
fn assert_scene(actual: &Scene, expected: &Scene, tolerance: f64) {
    let mut normalized = actual.clone();
    for (a, b) in normalized.items.iter_mut().zip(&expected.items) {
        assert_eq!(a.mesh.vertices.len(), b.mesh.vertices.len());
        for (p, q) in a.mesh.vertices.iter().zip(&b.mesh.vertices) {
            assert!(p.distance(*q) <= tolerance, "{p} != {q}");
        }
        a.mesh.vertices = b.mesh.vertices.clone();
    }
    assert_eq!(&normalized, expected);
}

fn expected_with(e: &Editor, local: Vec<DVec3>) -> Scene {
    let mut expected = e.scene.clone();
    expected.active_mut().unwrap().mesh.vertices = local;
    expected
}

/// Scene-space oracle: every result boundary line lies at signed inward
/// distance `d` from its source line, measured after the composed transform.
/// Inward follows the scene-space winding, so reflections are covered.
fn assert_scene_distance(world: DMat4, source: &[DVec3], result: &[DVec3], d: f64, tol: f64) {
    let p: Vec<DVec3> = source.iter().map(|&v| world.transform_point3(v)).collect();
    let q: Vec<DVec3> = result.iter().map(|&v| world.transform_point3(v)).collect();
    let n = p.len();
    let normal = (0..n)
        .map(|i| p[i].cross(p[(i + 1) % n]))
        .sum::<DVec3>()
        .normalize();
    for i in 0..n {
        let (p0, p1) = (p[i], p[(i + 1) % n]);
        let inward = normal.cross(p1 - p0).normalize();
        for corner in [q[i], q[(i + 1) % n]] {
            assert!(((corner - p0).dot(inward) - d).abs() <= tol, "edge {i}");
            assert!((corner - p0).dot(normal).abs() <= tol, "plane {i}");
        }
        assert!(
            (q[(i + 1) % n] - q[i]).dot(p1 - p0) > 0.,
            "edge {i} reversed"
        );
    }
}

/// Zero/no-change contract as in `polygon_scale_offset_core.rs`: only the
/// shared runtime ticket counters advance.
fn assert_no_change(e: &Editor, before: &Editor) {
    assert_eq!(e.identities.publication, before.identities.publication + 1);
    assert_eq!(e.identities.request, before.identities.request + 1);
    let mut normalized = e.clone();
    normalized.identities.publication = before.identities.publication;
    normalized.identities.request = before.identities.request;
    assert!(normalized == *before);
}

/// Full Scene, identities, Undo/Redo and native encoding round trip for a
/// changed invocation already compared against `expected`.
fn assert_history(e: &mut Editor, before: &Scene, ids: &IdentityScene) {
    let after = e.scene.clone();
    assert_eq!(e.identities.current, *ids);
    assert_eq!(
        persistence::decode(&persistence::encode(&after).unwrap()).unwrap(),
        after
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, *before);
    assert_eq!(e.identities.current, *ids);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
    assert_eq!(e.identities.current, *ids);
}

/// The admitted nonuniform 2x3 literals: x moves by d/2, y by d/3.
fn nonuniform_literal(d: f64) -> Vec<DVec3> {
    let (x, y) = (d / 2., d / 3.);
    vec![
        DVec3::new(x, y, 0.),
        DVec3::new(2. - x, y, 0.),
        DVec3::new(2. - x, 2. - y, 0.),
        DVec3::new(x, 2. - y, 0.),
    ]
}

#[test]
fn rotation_translation_and_parent_compose_with_nonuniform_scale() {
    // Rotation and translation are applied after the item's own scale and
    // the parent is rigid, so the local result equals the 2x3 literal.
    let child = Transform {
        translation: DVec3::new(1., -4., 2.),
        rotation: DVec3::new(0., 0., 0.4),
        scale: DVec3::new(2., 3., 1.),
        zxy: false,
        order: None,
    };
    let parent = Transform {
        translation: DVec3::new(5., -2., 3.),
        rotation: DVec3::new(0.3, 0.2, 1.1),
        ..Default::default()
    };
    for d in [0.25, -0.25, 0.6] {
        let mut e = scene(child.clone(), Some(parent.clone()));
        let before = e.scene.clone();
        let ids = e.identities.current.clone();
        let expected = expected_with(&e, nonuniform_literal(d));
        offset(&mut e, d).unwrap();
        assert_scene(&e.scene, &expected, 1e-12);
        let world = e.scene.world_matrix(e.scene.active().unwrap().id).unwrap();
        assert_scene_distance(world, &square(), &child_local(&e), d, 1e-12);
        assert_history(&mut e, &before, &ids);
    }
}

#[test]
fn reflected_parent_with_rotated_child_is_sheared_and_keeps_order() {
    // Parent scale (-2, 1, 1) after a child rotation of 45 degrees maps the
    // local square to a rhombus: edge vectors (-2*sqrt2, sqrt2) and
    // (2*sqrt2, sqrt2) up to sign, side sqrt10, area |det| * 4 = 8. Opposite
    // scene edges are 8/sqrt10 apart, local edges 2 apart, so a scene
    // distance d is a local inset t = d * 2 / (8 / sqrt10) = d * sqrt10 / 4.
    let child = Transform {
        rotation: DVec3::new(0., 0., std::f64::consts::FRAC_PI_4),
        ..Default::default()
    };
    let parent = scaled([-2., 1., 1.]);
    for d in [0.3, -0.5] {
        let mut e = scene(child.clone(), Some(parent.clone()));
        let before = e.scene.clone();
        let ids = e.identities.current.clone();
        let t = d * 10f64.sqrt() / 4.;
        let expected = expected_with(
            &e,
            vec![
                DVec3::new(t, t, 0.),
                DVec3::new(2. - t, t, 0.),
                DVec3::new(2. - t, 2. - t, 0.),
                DVec3::new(t, 2. - t, 0.),
            ],
        );
        offset(&mut e, d).unwrap();
        assert_scene(&e.scene, &expected, 1e-12);
        let world = e.scene.world_matrix(e.scene.active().unwrap().id).unwrap();
        assert!(DMat3::from_mat4(world).determinant() < 0.);
        assert_scene_distance(world, &square(), &child_local(&e), d, 1e-12);
        assert_history(&mut e, &before, &ids);
    }
}

#[test]
fn zero_on_scaled_item_preserves_parked_redo() {
    let mut e = scene(scaled([2., 3., 1.]), Some(scaled([-1., 1., 1.])));
    offset(&mut e, 0.25).unwrap();
    e.dispatch(Command::Undo).unwrap();
    let before = e.clone();
    offset(&mut e, 0.).unwrap();
    assert_no_change(&e, &before);
    e.dispatch(Command::Redo).unwrap();
    assert_scene(
        &e.scene,
        &expected_with(&before, nonuniform_literal(0.25)),
        1e-12,
    );
}

fn assert_refused(mut e: Editor, d: f64, reason: &str) {
    let before = e.clone();
    let error = offset(&mut e, d).unwrap_err();
    assert!(error.contains(reason), "expected '{reason}', got '{error}'");
    assert!(e == before, "refusal mutated the editor: {error}");
}

#[test]
fn singular_unsafe_and_unrepresentable_transforms_refuse_atomically() {
    // Scene validation keeps each item's scale within (1e-8, 1e8], but 50
    // nested 1e-7 parent scales underflow the composed x scale to zero.
    let mut e = scene(Transform::default(), None);
    let child = e.scene.active().unwrap().id;
    let mut below = child;
    for _ in 0..50 {
        e.dispatch(Command::NewMesh).unwrap();
        let p = e.scene.active_mut().unwrap();
        p.transform = scaled([1e-7, 1., 1.]);
        let id = p.id;
        e.scene
            .items
            .iter_mut()
            .find(|i| i.id == below)
            .unwrap()
            .parent = Some(id);
        below = id;
    }
    e.scene.selection.item = Some(child);
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.scene.validate().unwrap();
    e.synchronize_identities().unwrap();
    assert_eq!(
        DMat3::from_mat4(e.scene.world_matrix(child).unwrap()).determinant(),
        0.
    );
    assert_refused(e, 0.25, "singular");
    // A face only 2e-6 wide in the scene collapses under a 0.25 inset.
    assert_refused(
        scene(scaled([1e-6, 1., 1.]), None),
        0.25,
        "collapses or inverts",
    );
    // Outward 1e5 in the scene needs local x near 1e11: beyond the mesh range.
    assert_refused(
        scene(scaled([1e-6, 1., 1.]), None),
        -1e5,
        "Invalid mesh coordinates",
    );
    // The admitted local refusals still apply on a scaled item.
    assert_refused(
        scene(scaled([2., 3., 1.]), None),
        f64::NAN,
        "finite distance",
    );
}
