//! Primitives on a moved mesh (Jason, 2026-11-07): the user draws in world
//! space; the new geometry is mapped into the item's local space so it lands
//! exactly where drawn, the item keeps its transform, and the whole thing is
//! one undo step. Only a transform with no inverse (zero scale) is refused.
use glam::DVec3;
use operandi_core::*;

fn moved() -> Transform {
    Transform {
        translation: DVec3::new(1.5, -2., 3.),
        rotation: DVec3::new(0.3, -0.7, 1.1),
        scale: DVec3::new(2., 0.5, 1.5),
        zxy: false,
        order: None,
    }
}

fn editor(transform: Transform) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().transform = transform;
    e
}

/// Every point of the active mesh in world space.
fn world_points(e: &Editor) -> Vec<DVec3> {
    let item = e.scene.active().unwrap();
    let m = e.scene.world_matrix(item.id).unwrap();
    item.mesh
        .vertices
        .iter()
        .map(|&v| m.transform_point3(v))
        .collect()
}

fn assert_same_points(a: &[DVec3], b: &[DVec3], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}: point count");
    assert!(!a.is_empty(), "{what}: made nothing");
    for (i, (p, q)) in a.iter().zip(b).enumerate() {
        assert!(p.distance(*q) < 1e-9, "{what}: point {i} {p} vs {q}");
    }
}

/// Runs `op` on an untransformed mesh and on a moved one and checks the
/// moved mesh's new points sit at the same world positions, the item kept
/// its transform, and one Undo takes the shape away.
fn check(what: &str, op: impl Fn(&mut Editor)) {
    let mut plain = editor(Transform::default());
    op(&mut plain);
    let mut e = editor(moved());
    let history = e.history.undo.len();
    op(&mut e);
    assert_same_points(&world_points(&plain), &world_points(&e), what);
    assert_eq!(e.scene.active().unwrap().transform, moved(), "{what}");
    assert_eq!(e.history.undo.len(), history + 1, "{what}: one undo step");
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh.vertices.is_empty(), "{what}");
    assert_eq!(e.scene.active().unwrap().transform, moved(), "{what}");
    e.dispatch(Command::Redo).unwrap();
    assert_same_points(&world_points(&plain), &world_points(&e), what);
}

fn drawn(tool: Tool) -> impl Fn(&mut Editor) {
    move |e: &mut Editor| {
        e.dispatch(Command::Activate(tool)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.position = DVec3::new(0.4, 1.2, -0.7);
        p.size = DVec3::new(1., 2., 0.5);
        if let Some(r) = p.radial.as_mut() {
            r.radius = DVec3::new(0.6, 0.9, 0.3);
        }
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Drop).unwrap();
    }
}

#[test]
fn drawn_cube_sphere_cylinder_land_where_drawn() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        check(tool.label(), drawn(tool));
    }
}

#[test]
fn moved_cube_keeps_world_space_uvs() {
    // UVs come from the drawn position and size, before the mapping.
    let mut plain = editor(Transform::default());
    drawn(Tool::Box)(&mut plain);
    let mut e = editor(moved());
    drawn(Tool::Box)(&mut e);
    let a = &plain.scene.active().unwrap().mesh.uv_maps;
    let b = &e.scene.active().unwrap().mesh.uv_maps;
    assert_eq!(a, b);
}

#[test]
fn numeric_cone_ellipsoid_torus_land_where_drawn() {
    check("Cone", |e| {
        e.dispatch(Command::ArmCone).unwrap();
        e.dispatch(Command::ConfigureCone(ConeParameters {
            position: DVec3::new(-1., 0.5, 2.),
            ..Default::default()
        }))
        .unwrap();
        e.dispatch(Command::Drop).unwrap();
    });
    check("Ellipsoid", |e| {
        e.dispatch(Command::ArmEllipsoid).unwrap();
        e.dispatch(Command::ConfigureEllipsoid(EllipsoidParameters {
            position: DVec3::new(1., -2., 3.),
            radius: DVec3::new(2., 3., 4.),
            ..Default::default()
        }))
        .unwrap();
        e.dispatch(Command::Drop).unwrap();
    });
    check("Torus", |e| {
        e.dispatch(Command::ArmShape(ShapeKind::Torus)).unwrap();
        e.dispatch(Command::ConfigureShape(ShapeParameters::Torus(
            TorusParameters {
                position: DVec3::new(0.25, 0.5, -1.),
                ..Default::default()
            },
        )))
        .unwrap();
        e.dispatch(Command::Drop).unwrap();
    });
}

#[test]
fn shape_preview_shows_where_drawn() {
    let mut e = editor(moved());
    e.dispatch(Command::ArmShape(ShapeKind::Capsule)).unwrap();
    let p = e.session.shape.as_ref().unwrap().requested.clone();
    e.dispatch(Command::ConfigureShape(p)).unwrap();
    let preview = e.shape_preview_scene().unwrap().unwrap();
    let mut plain = editor(Transform::default());
    plain
        .dispatch(Command::ArmShape(ShapeKind::Capsule))
        .unwrap();
    let p = plain.session.shape.as_ref().unwrap().requested.clone();
    plain.dispatch(Command::ConfigureShape(p)).unwrap();
    let reference = plain.shape_preview_scene().unwrap().unwrap();
    let world = |s: &Scene| {
        let item = s.active().unwrap();
        let m = s.world_matrix(item.id).unwrap();
        item.mesh
            .vertices
            .iter()
            .map(|&v| m.transform_point3(v))
            .collect::<Vec<_>>()
    };
    assert_same_points(&world(&reference), &world(&preview), "Capsule preview");
}

#[test]
fn unit_shapes_go_into_a_moved_mesh() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        check(tool.label(), move |e| {
            e.dispatch(Command::InsertUnit {
                tool,
                destination: UnitDestination::ExistingMesh,
            })
            .unwrap();
        });
    }
    for shape in [UnitShape::Cone, UnitShape::Torus, UnitShape::Capsule] {
        check(shape.label(), move |e| {
            e.dispatch(Command::InsertShapeUnit(
                shape,
                UnitDestination::ExistingMesh,
            ))
            .unwrap();
        });
    }
    check("Unit Ellipsoid", |e| {
        e.dispatch(Command::InsertEllipsoidUnit(UnitDestination::ExistingMesh))
            .unwrap();
    });
}

#[test]
fn parented_mesh_takes_the_shape_where_drawn() {
    let parented = |e: &mut Editor, child: Transform| {
        let child_id = e.scene.active().unwrap().id;
        e.dispatch(Command::NewMesh).unwrap();
        let parent = e.scene.active_mut().unwrap();
        parent.transform = Transform {
            translation: DVec3::new(-3., 1., 0.5),
            rotation: DVec3::new(0., 0.9, 0.),
            scale: DVec3::splat(1.25),
            zxy: false,
            order: None,
        };
        let parent_id = parent.id;
        let item = e.scene.items.iter_mut().find(|i| i.id == child_id).unwrap();
        item.parent = Some(parent_id);
        item.transform = child;
        e.dispatch(Command::SelectItem(Some(child_id))).unwrap();
        e.scene.validate().unwrap();
        child_id
    };
    let mut e = editor(Transform::default());
    let child = parented(&mut e, moved());
    assert_eq!(e.scene.active().unwrap().id, child, "child is active");
    drawn(Tool::Box)(&mut e);
    let mut plain = editor(Transform::default());
    drawn(Tool::Box)(&mut plain);
    assert_same_points(&world_points(&plain), &world_points(&e), "parented Box");
    let item = e.scene.active().unwrap();
    assert_eq!(item.transform, moved());
    assert!(item.parent.is_some());
}

#[test]
fn zero_scale_mesh_is_refused_and_left_alone() {
    let flat = Transform {
        scale: DVec3::new(1., 0., 1.),
        ..moved()
    };
    let mut e = editor(flat);
    let base = e.scene.clone();
    let history = e.history.undo.len();
    for result in [
        e.dispatch(Command::Activate(Tool::Box)),
        e.dispatch(Command::Activate(Tool::Sphere)),
        e.dispatch(Command::ArmEllipsoid),
        e.dispatch(Command::ArmShape(ShapeKind::Torus)),
        e.dispatch(Command::InsertUnit {
            tool: Tool::Cylinder,
            destination: UnitDestination::ExistingMesh,
        }),
        e.dispatch(Command::InsertShapeUnit(
            UnitShape::Cone,
            UnitDestination::ExistingMesh,
        )),
        e.dispatch(Command::InsertEllipsoidUnit(UnitDestination::ExistingMesh)),
    ] {
        // Scene validation (which already forbids a zero scale) or the
        // primitive's own inverse check refuses it, whichever runs first.
        let err = result.unwrap_err();
        assert!(
            err.contains("zero scale") || err == "Invalid item transform",
            "{err}"
        );
    }
    assert!(!e.session.has_active_task());
    assert_eq!(e.scene, base);
    assert_eq!(e.history.undo.len(), history);
}

#[test]
fn moved_primitive_survives_save_and_reopen() {
    let mut e = editor(moved());
    drawn(Tool::Sphere)(&mut e);
    let before = world_points(&e);
    let dir = std::env::temp_dir().join(format!("operandi-prim-moved-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("moved.opscene");
    operandi_core::persistence::save(&path, &e.scene).unwrap();
    let reopened = Editor {
        scene: operandi_core::persistence::load(&path).unwrap(),
        ..Default::default()
    };
    assert_same_points(&before, &world_points(&reopened), "reopened Sphere");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn world_to_local_refuses_a_flat_transform_with_a_clear_message() {
    let mut e = editor(moved());
    let id = e.scene.active().unwrap().id;
    let inverse = e.scene.world_to_local(id).unwrap();
    let world = e.scene.world_matrix(id).unwrap();
    assert!((inverse * world).abs_diff_eq(glam::DMat4::IDENTITY, 1e-12));
    e.scene.active_mut().unwrap().transform.scale.z = 0.;
    let err = e.scene.world_to_local(id).unwrap_err();
    assert!(err.contains("zero scale"), "{err}");
}
