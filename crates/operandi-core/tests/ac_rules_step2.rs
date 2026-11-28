//! Action Center rule sheet v6, step 2: the Element frame of a picked
//! polygon, edge or point (C-ELEM, A-ELEM) and what a pick does to the
//! Action Center, through the editor's own transform path. The presses
//! themselves are tested in the app (`ac_press_tests.rs`).
use glam::{DMat3, DQuat, DVec3, dvec3};
use operandi_core::*;

fn quad(v: [u32; 4]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A roof: two quads meeting along the ridge (points 1 and 4), quad A
/// (0 1 4 3) flat on the ground plane, quad B (1 2 5 4) standing up from
/// the ridge, plus a loose point (6).
///
/// ```text
///   3 --- 4 --- 5
///   |  A  |  B  |
///   0 --- 1 --- 2
/// ```
fn roof() -> Mesh {
    Mesh {
        vertices: vec![
            dvec3(0., 0., 0.),
            dvec3(2., 0., 0.),
            dvec3(2., 3., 0.),
            dvec3(0., 0., -4.),
            dvec3(2., 0., -4.),
            dvec3(2., 3., -4.),
            dvec3(9., 9., 9.),
        ],
        faces: vec![quad([0, 1, 4, 3]), quad([1, 2, 5, 4])].into(),
        ..Default::default()
    }
}

fn editor(mesh: Mesh) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0];
    e
}
fn id(e: &Editor) -> u64 {
    e.scene.active().unwrap().id
}
fn close(a: DVec3, b: DVec3, what: &str) {
    assert!(a.distance(b) < 1e-9, "{what}: {a} != {b}");
}
fn right_handed(b: DMat3) {
    close(b.x_axis.cross(b.y_axis), b.z_axis, "Y = Z × X");
    assert!((b.x_axis.length() - 1.).abs() < 1e-9 && b.x_axis.dot(b.z_axis).abs() < 1e-9);
}

/// C-ELEM and A-ELEM on a plain item: a polygon's middle and normal; an
/// edge's midpoint, its own direction as X and the normals of the
/// polygons that use it as Z; a point and the normals round it.
#[test]
fn the_element_frame_of_a_polygon_an_edge_and_a_point() {
    let e = editor(roof());
    let mesh = e.scene.active().unwrap().mesh.clone();
    let (flat, wall) = (mesh.normal(0), mesh.normal(1));
    assert!(flat.dot(wall).abs() < 1e-9, "the two quads are square");

    let f = element_frame(&e.scene, id(&e), TweakElement::Polygon(0)).unwrap();
    close(f.center, dvec3(1., 0., -2.), "the polygon's middle");
    close(f.basis.z_axis, flat, "its own normal");
    close(f.basis.x_axis, DVec3::X, "world X laid square to it");
    right_handed(f.basis);

    // The wall faces along world X, so world Z stands in for X.
    let f = element_frame(&e.scene, id(&e), TweakElement::Polygon(1)).unwrap();
    close(f.basis.z_axis, wall, "the wall's normal");
    close(
        f.basis.x_axis,
        DVec3::Z,
        "world Z when Z runs along world X",
    );

    // The ridge, picked either way round.
    for edge in [TweakElement::Edge(1, 4), TweakElement::Edge(4, 1)] {
        let f = element_frame(&e.scene, id(&e), edge).unwrap();
        close(f.center, dvec3(2., 0., -2.), "the edge's midpoint");
        close(f.basis.z_axis, (flat + wall).normalize(), "both normals");
        close(f.basis.x_axis, -DVec3::Z, "along it, point 1 to point 4");
        right_handed(f.basis);
    }
    // A rim edge has one polygon.
    let f = element_frame(&e.scene, id(&e), TweakElement::Edge(0, 3)).unwrap();
    close(f.basis.z_axis, flat, "its one polygon's normal");

    let f = element_frame(&e.scene, id(&e), TweakElement::Vertex(1)).unwrap();
    close(f.center, dvec3(2., 0., 0.), "the point");
    close(
        f.basis.z_axis,
        (flat + wall).normalize(),
        "the normals round it",
    );
    right_handed(f.basis);
}

/// A-ELEM's "no polygons" cases are World: a loose point, and two points
/// of one polygon that aren't an edge of it (its diagonal).
#[test]
fn an_element_with_no_polygons_gets_world_axes() {
    let e = editor(roof());
    let f = element_frame(&e.scene, id(&e), TweakElement::Vertex(6)).unwrap();
    close(f.center, dvec3(9., 9., 9.), "the loose point");
    assert_eq!(f.basis, DMat3::IDENTITY);
    let f = element_frame(&e.scene, id(&e), TweakElement::Edge(0, 4)).unwrap();
    close(f.center, dvec3(1., 0., -2.), "the diagonal's midpoint");
    assert_eq!(f.basis, DMat3::IDENTITY, "a diagonal is no edge");
    assert!(element_frame(&e.scene, id(&e), TweakElement::Polygon(7)).is_err());
}

/// On a moved, turned and stretched item the frame is in the world, and Z
/// stays square to the surface (a stretched item squashes plain normals).
#[test]
fn the_element_frame_follows_a_turned_stretched_item() {
    let mut e = editor(roof());
    let item = e.scene.active_mut().unwrap();
    item.transform.translation = dvec3(3., -1., 2.);
    item.transform.rotation = dvec3(0.3, -0.7, 0.2);
    item.transform.scale = dvec3(1., 3., 0.5);
    let world = e.scene.world_matrix(id(&e)).unwrap();
    let mesh = e.scene.active().unwrap().mesh.clone();
    for (face, element) in [(0, TweakElement::Polygon(0)), (1, TweakElement::Polygon(1))] {
        let f = element_frame(&e.scene, id(&e), element).unwrap();
        close(
            f.center,
            world.transform_point3(mesh.center(face)),
            "the middle, in the world",
        );
        // Square to two of the polygon's own edges as they lie in the world.
        let v = &mesh.faces[face].vertices;
        let p = |i: usize| world.transform_point3(mesh.vertices[v[i] as usize]);
        for along in [p(1) - p(0), p(2) - p(1)] {
            assert!(f.basis.z_axis.dot(along.normalize()).abs() < 1e-9);
        }
        right_handed(f.basis);
    }
}

/// Through the tools: a pick moves the Element centre and axes, the tool
/// turns about them, and the next pick moves them again (GitHub #32).
#[test]
fn a_pick_moves_the_tool_and_the_next_pick_moves_it_again() {
    let mut e = editor(roof());
    e.dispatch(Command::ActionContext(ActionContext::ELEMENT_UNPICKED))
        .unwrap();
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    let live = |e: &Editor| e.session.live.as_ref().unwrap().action_frame.unwrap();
    assert_eq!(live(&e).center, DVec3::ZERO);
    assert_eq!(live(&e).basis, DMat3::IDENTITY);

    for element in [
        TweakElement::Polygon(1),
        TweakElement::Edge(1, 4),
        TweakElement::Vertex(5),
        TweakElement::Polygon(0),
    ] {
        let pick = element_frame(&e.scene, id(&e), element).unwrap();
        let context = e.action_context.picked(pick);
        e.dispatch(Command::ActionContext(context)).unwrap();
        close(live(&e).center, pick.center, "the centre goes to the pick");
        for axis in 0..3 {
            close(
                live(&e).basis.col(axis),
                pick.basis.col(axis),
                "its axes too",
            );
        }
        assert!(matches!(e.action_context.center, CenterMode::Element(_)));
    }
    // Rotate Z by 90° turns the selected polygon about its own normal
    // through its middle (the last pick).
    let before = e.scene.active().unwrap().mesh.vertices.clone();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = dvec3(0., 0., 90.);
    e.dispatch(Command::Preview(p)).unwrap();
    let after = &e.scene.active().unwrap().mesh.vertices;
    let pick = live(&e);
    let turn = DQuat::from_axis_angle(pick.basis.z_axis, 90f64.to_radians());
    for v in [0, 1, 4, 3] {
        close(
            after[v],
            pick.center + turn * (before[v] - pick.center),
            "turned about the pick",
        );
    }
}

/// The Center and Axis lists' mixing rule: a pick fills only the parts
/// that are Element. Element's centre with World axes keeps World; the
/// Origin with Element's axes stays at the origin.
#[test]
fn a_pick_fills_only_the_element_part_of_a_mixed_action_center() {
    let mut e = editor(roof());
    let pick = element_frame(&e.scene, id(&e), TweakElement::Polygon(1)).unwrap();
    let frame = |e: &mut Editor, context: ActionContext| {
        e.dispatch(Command::ActionContext(context.picked(pick)))
            .unwrap();
        e.dispatch(Command::Activate(Tool::Scale)).unwrap();
        let f = e.session.live.as_ref().unwrap().action_frame.unwrap();
        e.dispatch(Command::Cancel).unwrap();
        f
    };
    let centre_only = frame(
        &mut e,
        ActionContext {
            center: CenterMode::Element(PickedPoint::ORIGIN),
            axis: AxisOrientation::World,
        },
    );
    close(centre_only.center, pick.center, "Element's centre");
    assert_eq!(centre_only.basis, DMat3::IDENTITY, "World axes kept");
    let axis_only = frame(
        &mut e,
        ActionContext {
            center: CenterMode::Origin,
            axis: AxisOrientation::Element(PickedFrame::WORLD),
        },
    );
    assert_eq!(axis_only.center, DVec3::ZERO, "the Origin kept");
    close(axis_only.basis.z_axis, pick.basis.z_axis, "Element's axes");
}
