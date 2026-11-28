//! UV Transform (`uvtransform`, MESHOPS-TIDY TB-40..43): points move onto
//! a Target's surface by the Target's UV map. The spec's worked example.
use crate::mesh_stack_tools::{add, with_base};
use glam::{DVec2, DVec3};
use operandi_core::*;
use serde_json::json;

/// The Target quad: UV (u, v) ↔ place (2u, 0, −2v), normal −Y.
fn target_quad() -> Mesh {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(0., 0., -2.),
            DVec3::new(2., 0., -2.),
            DVec3::new(2., 0., 0.),
        ],
        ..Default::default()
    };
    m.faces.push(Face {
        vertices: vec![0, 1, 2, 3],
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    });
    let uv = [(0., 0.), (0., 1.), (1., 1.), (1., 0.)].map(|(u, v)| Some(DVec2::new(u, v)));
    let mut maps = (*m.uv_maps).clone();
    maps.insert("Texture".into(), vec![uv.to_vec()]);
    m.uv_maps = maps.into();
    m
}
/// P0..P4; two triangles that no setting below folds flat (an op with a
/// Selection skips a mesh with no polygons), P0 loose.
fn source_points() -> Mesh {
    let tri = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    };
    Mesh {
        faces: vec![tri(vec![1, 2, 3]), tri(vec![3, 2, 4])].into(),
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(0.25, 0., 0.),
            DVec3::new(0., 0.25, 0.),
            DVec3::new(0.25, 0.25, 0.1),
            DVec3::new(0.7, 0., 0.),
        ],
        ..Default::default()
    }
}
/// The source mesh with a UV Transform step onto the Target.
fn setup() -> (Editor, u64, u64, mesh_stack_editor::OpRef) {
    let (mut e, id) = with_base(source_points());
    e.scene.new_mesh().unwrap();
    let target = e.scene.items.last().unwrap().id;
    e.scene.item_mut(target).unwrap().mesh = target_quad().into();
    let op = add(&mut e, id, "uvtransform", json!({ "target": target }));
    (e, id, target, op)
}
fn set(e: &mut Editor, op: mesh_stack_editor::OpRef, values: serde_json::Value) {
    let values = values.as_object().unwrap().clone();
    e.dispatch(Command::MeshOpSet { op, values }).unwrap();
}
fn points(e: &Editor, id: u64) -> Vec<DVec3> {
    let r = e
        .mesh_stack
        .current(&e.scene, e.scene.item(id).unwrap())
        .unwrap();
    r.mesh.vertices.clone()
}
fn same(got: &[DVec3], want: &[(f64, f64, f64)]) {
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(want) {
        let w = DVec3::new(w.0, w.1, w.2);
        assert!(g.distance(w) < 1e-9, "got {got:?}, want {want:?}");
    }
}
const DEFAULT: [(f64, f64, f64); 5] = [
    (0., 0., 0.),
    (0.5, 0., 0.),
    (0., 0., -0.5),
    (0.5, 0., -0.5),
    (1.4, 0., 0.),
];

#[test]
fn uv_transform_without_a_map_leaves_the_mesh() {
    let (e, id, _, _) = setup();
    assert_eq!(points(&e, id), source_points().vertices);
}

#[test]
fn uv_transform_places_points_by_the_target_uvs() {
    let (mut e, id, _, op) = setup();
    set(&mut e, op, json!({ "targetuvmap": "Texture" }));
    same(&points(&e, id), &DEFAULT);
    // Wrap off keeps the depth along the Target's normal (−Y).
    set(&mut e, op, json!({ "wrap": false }));
    let got = points(&e, id);
    assert!(
        got[3].distance(DVec3::new(0.5, -0.1, -0.5)) < 1e-9,
        "{got:?}"
    );
    // Push adds along the normal too.
    set(&mut e, op, json!({ "wrap": true, "push": 0.1 }));
    for (g, w) in points(&e, id).iter().zip(DEFAULT) {
        assert!((g.y + 0.1).abs() < 1e-9 && (g.x - w.0).abs() < 1e-9, "{g}");
    }
}

#[test]
fn uv_transform_scale_offset_and_axis() {
    let (mut e, id, _, op) = setup();
    set(
        &mut e,
        op,
        json!({ "targetuvmap": "Texture", "scale": 2.0 }),
    );
    same(
        &points(&e, id),
        &[
            (0.5, 0., -0.5),
            (0.75, 0., -0.5),
            (0.5, 0., -0.75),
            (0.75, 0., -0.75),
            (1.2, 0., -0.5),
        ],
    );
    set(
        &mut e,
        op,
        json!({ "scale": 1.0, "offset.U": 0.0, "offset.V": 0.0 }),
    );
    same(
        &points(&e, id),
        &[
            (1., 0., -1.),
            (1.5, 0., -1.),
            (1., 0., -1.5),
            (1.5, 0., -1.5),
            (0.7, 0., 0.), // a miss stays
        ],
    );
    set(
        &mut e,
        op,
        json!({ "offset.U": 0.5, "offset.V": 0.5, "sourceworldaxis": 0 }),
    );
    same(
        &points(&e, id),
        &[
            (0., 0., 0.),
            (0., 0., 0.),
            (0., 0., -0.5),
            (0.2, 0., -0.5),
            (0., 0., 0.),
        ],
    );
    set(&mut e, op, json!({ "sourceworldaxis": 1 }));
    same(
        &points(&e, id),
        &[
            (0., 0., 0.),
            (0., 0., -0.5),
            (0., 0., 0.),
            (0.2, 0., -0.5),
            (0., 0., -1.4),
        ],
    );
}

#[test]
fn uv_transform_item_placement() {
    let (mut e, id, target, op) = setup();
    set(&mut e, op, json!({ "targetuvmap": "Texture" }));
    // The source moved: off changes nothing, on looks up in world space.
    e.scene.item_mut(id).unwrap().transform.translation = DVec3::new(0.5, 0., 0.);
    same(&points(&e, id), &DEFAULT);
    set(&mut e, op, json!({ "usesourcexfrm": true }));
    same(
        &points(&e, id),
        &[
            (0.5, 0., 0.),
            (1., 0., 0.),
            (0.5, 0., -0.5),
            (1., 0., -0.5),
            (0.7, 0., 0.),
        ],
    );
    // The target moved: the result follows it.
    set(&mut e, op, json!({ "usesourcexfrm": false }));
    e.scene.item_mut(target).unwrap().transform.translation = DVec3::Y;
    let want: Vec<(f64, f64, f64)> = DEFAULT.iter().map(|p| (p.0, p.1 + 1., p.2)).collect();
    same(&points(&e, id), &want);
}

#[test]
fn uv_transform_saves_and_undoes() {
    let (mut e, id, _, op) = setup();
    set(&mut e, op, json!({ "targetuvmap": "Texture" }));
    let stack = e.scene.item(id).unwrap().mesh_stack.clone().unwrap();
    let text = serde_json::to_string(&stack).unwrap();
    assert!(
        text.contains("\"offset.U\"") && text.contains("targetuvmap"),
        "{text}"
    );
    let reopened: mesh_stack::MeshStack = serde_json::from_str(&text).unwrap();
    assert_eq!(reopened, stack);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points(&e, id), source_points().vertices);
}
