//! MP-C13 SO-U01 peer amendment 1: the read-only Scale Offset offer check.
//!
//! `Editor::scale_offset_polygon_eligibility` must refuse every statically
//! unsupported target (context, selection, ownership, geometry, data and
//! transform) with the same reason the edit itself gives, must accept every
//! target the reviewed edit supports (including scaled, reflected and sheared
//! transforms), and must leave distance-dependent refusals to the edit.
use glam::{DMat3, DVec3};
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn square() -> Vec<DVec3> {
    vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(2., 2., 0.),
        DVec3::new(0., 2., 0.),
    ]
}

fn editor(mesh: Mesh) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    *e.scene.active_mut().unwrap().mesh = mesh;
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.scene.validate().unwrap();
    e.synchronize_identities().unwrap();
    e
}

fn one(points: Vec<DVec3>) -> Editor {
    let corners = (0..points.len() as u32).collect();
    editor(Mesh {
        vertices: points,
        faces: vec![face(corners)].into(),
        ..Default::default()
    })
}

/// Eligible, and checking it changes nothing.
fn assert_eligible(e: &Editor) {
    let before = e.clone();
    let target = e.scale_offset_polygon_eligibility();
    assert_eq!(target, Ok((e.scene.selection.item.unwrap(), vec![0])));
    assert!(*e == before);
}

/// Refused by the offer check with exactly the edit's own reason at both a
/// zero and a nonzero distance, and nothing changes.
fn assert_static_refusal(e: Editor, reason: &str) {
    let before = e.clone();
    let error = e.scale_offset_polygon_eligibility().unwrap_err();
    assert!(error.contains(reason), "expected '{reason}', got '{error}'");
    assert!(e == before);
    for d in [0., 0.1, -0.1] {
        let mut edit = e.clone();
        assert_eq!(
            edit.dispatch(Command::ScaleOffsetPolygon(d)).unwrap_err(),
            error
        );
        assert!(edit == before);
    }
}

#[test]
fn supported_targets_are_offered_without_mutation() {
    assert_eligible(&one(square()));
    assert_eligible(&one(vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(4., 0., 0.),
        DVec3::new(0., 3., 0.),
    ]));
    // Unrelated shared geometry and far unused points elsewhere in the mesh.
    let mut m = Mesh {
        vertices: square(),
        faces: vec![face(vec![0, 1, 2, 3])].into(),
        ..Default::default()
    };
    m.vertices.extend([
        DVec3::new(500., 0., 0.),
        DVec3::new(501., 0., 0.),
        DVec3::new(500., 1., 0.),
        DVec3::new(501., 1., 0.),
        DVec3::new(4e8, -4e8, 2e8),
    ]);
    m.faces.push(face(vec![4, 5, 6]));
    m.faces.push(face(vec![5, 7, 6]));
    assert_eligible(&editor(m));
    // Scaled/reflected item and a reflected parent over a rotated child.
    let mut e = one(square());
    e.scene.active_mut().unwrap().transform.scale = DVec3::new(-2., 3., 1.);
    assert_eligible(&e);
    let mut e = one(square());
    let child = e.scene.selection.item.unwrap();
    e.scene.active_mut().unwrap().transform.rotation = DVec3::new(0., 0., 0.785);
    let selection = e.scene.selection.clone();
    e.dispatch(Command::NewMesh).unwrap();
    let parent = e.scene.active_mut().unwrap();
    parent.transform.scale = DVec3::new(-2., 1., 1.);
    let parent_id = parent.id;
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == child)
        .unwrap()
        .parent = Some(parent_id);
    e.scene.selection = selection;
    e.scene.validate().unwrap();
    assert_eligible(&e);
}

#[test]
fn distance_dependent_refusals_stay_with_the_edit() {
    let e = one(square());
    assert_eligible(&e);
    for d in [1., 1.5, -1e9 + 1.] {
        let mut edit = e.clone();
        assert!(
            edit.dispatch(Command::ScaleOffsetPolygon(d)).is_err(),
            "{d}"
        );
    }
    let mut edit = e.clone();
    edit.dispatch(Command::ScaleOffsetPolygon(0.5)).unwrap();
}

#[test]
fn static_geometry_data_and_transform_refusals_match_the_edit() {
    let xy = |p: &[[f64; 2]]| p.iter().map(|q| DVec3::new(q[0], q[1], 0.)).collect();
    assert_static_refusal(
        one(xy(&[[0., 0.], [4., 0.], [4., 4.], [2., 1.], [0., 4.]])),
        "not strictly convex",
    );
    assert_static_refusal(
        one(xy(&[[0., 0.], [1., 0.], [2., 0.], [2., 2.], [0., 2.]])),
        "not strictly convex",
    );
    let mut bent = square();
    bent[2].z = 0.5;
    assert_static_refusal(one(bent), "nonplanar");
    assert_static_refusal(
        one(xy(&[[0., 0.], [2., 0.], [2., 0.], [2., 2.], [0., 2.]])),
        "repeated or degenerate",
    );
    let star = (0..5)
        .map(|k| {
            let a = (90. + 144. * k as f64).to_radians();
            DVec3::new(a.cos(), a.sin(), 0.)
        })
        .collect();
    assert_static_refusal(one(star), "self-intersecting");
    for second in [vec![1, 4, 5, 2], vec![2, 4, 5]] {
        let mut m = Mesh {
            vertices: square(),
            faces: vec![face(vec![0, 1, 2, 3]), face(second)].into(),
            ..Default::default()
        };
        m.vertices
            .extend([DVec3::new(4., 0., 0.), DVec3::new(4., 2., 0.)]);
        // Connected polygons take the framed-inset path since the fix pass.
        assert!(editor(m).scale_offset_polygon_eligibility().is_ok());
    }
    let mut line = Mesh {
        vertices: square(),
        faces: vec![face(vec![0, 1, 2, 3])].into(),
        ..Default::default()
    };
    line.attributes.representations = vec![PolygonRepresentation::Polyline {
        closed: true,
        curve: Default::default(),
    }];
    assert_static_refusal(editor(line), "polyline");
    let mut derived = Mesh {
        vertices: square(),
        faces: vec![face(vec![0, 1, 2, 3])].into(),
        ..Default::default()
    };
    derived.attributes.typed_maps.insert(
        "tangents".into(),
        TypedMap {
            kind: "tangent_basis6".into(),
            dimension: 6,
            zero_default: false,
            points: [(2, vec![1., 0., 0., 0., 1., 0.])].into(),
            corners: vec![],
        },
    );
    assert_static_refusal(editor(derived), "tangent or position-derivative");
    // 50 nested 1e-7 parent scales underflow the composed transform.
    let mut e = one(square());
    let child = e.scene.selection.item.unwrap();
    let mut below = child;
    for _ in 0..50 {
        e.dispatch(Command::NewMesh).unwrap();
        let p = e.scene.active_mut().unwrap();
        p.transform.scale = DVec3::new(1e-7, 1., 1.);
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
    assert_eq!(
        DMat3::from_mat4(e.scene.world_matrix(child).unwrap()).determinant(),
        0.
    );
    assert_static_refusal(e, "singular");
}

#[test]
fn context_and_owner_refusals_match_the_target_check() {
    let mut e = one(square());
    e.scene.selection.faces.clear();
    assert!(
        e.scale_offset_polygon_eligibility()
            .unwrap_err()
            .contains("Select one polygon or more")
    );
    let mut e = one(square());
    e.scene.selection.mode = Mode::Vertex;
    assert!(
        e.scale_offset_polygon_eligibility()
            .unwrap_err()
            .contains("Polygon selection mode")
    );
    let mut e = one(square());
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let before = e.clone();
    assert!(
        e.scale_offset_polygon_eligibility()
            .unwrap_err()
            .contains("active tool")
    );
    assert!(e == before);
}
