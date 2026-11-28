//! GitHub #35: the global falloff shapes the Cone, Capsule, Torus and
//! Ellipsoid as they're made, as #15 does for the Cube, Sphere and
//! Cylinder. Turning the falloff on, moving it and off keeps the tool open
//! and rebuilds its preview; Apply keeps the shaped mesh; each primitive
//! MeshOp shapes itself with its own copy of the falloff.
use crate::mesh_stack_tools::{add, ok, with_base};
use glam::DVec3;
use operandi_core::*;

fn ramp(top: f64) -> Falloff {
    Falloff::Linear(LinearFalloff {
        start: DVec3::new(0., top, 0.),
        end: DVec3::new(0., -0.75, 0.),
        ..Default::default()
    })
}

/// Arms `kind`'s tool with Falloff Scale 0% and returns its preview.
fn armed(kind: &str) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let falloff = ShapeFalloff {
        scale: 0.,
        ..Default::default()
    };
    match kind {
        "cone" => {
            e.dispatch(Command::ArmCone).unwrap();
            let p = ConeParameters {
                segments: 4,
                falloff,
                ..Default::default()
            };
            e.dispatch(Command::ConfigureCone(p)).unwrap();
        }
        "ellipsoid" => {
            e.dispatch(Command::ArmEllipsoid).unwrap();
            let p = EllipsoidParameters {
                falloff,
                ..Default::default()
            };
            e.dispatch(Command::ConfigureEllipsoid(p)).unwrap();
        }
        "torus" => {
            e.dispatch(Command::ArmShape(ShapeKind::Torus)).unwrap();
            let p = TorusParameters {
                falloff,
                ..Default::default()
            };
            e.dispatch(Command::ConfigureShape(ShapeParameters::Torus(p)))
                .unwrap();
        }
        "capsule" => {
            e.dispatch(Command::ArmShape(ShapeKind::Capsule)).unwrap();
            let p = CapsuleParameters {
                segments: 4,
                falloff,
                ..Default::default()
            };
            e.dispatch(Command::ConfigureShape(ShapeParameters::Capsule(p)))
                .unwrap();
        }
        _ => unreachable!(),
    }
    e
}
fn preview(e: &Editor) -> Mesh {
    e.session
        .cone
        .as_ref()
        .and_then(|s| s.preview.clone())
        .or_else(|| e.session.ellipsoid.as_ref().and_then(|s| s.preview.clone()))
        .or_else(|| e.session.shape.as_ref().and_then(|s| s.preview.clone()))
        .expect("the tool shows a preview")
}
/// The widest point's distance from the Y axis, among points above y.
fn widest_above(m: &Mesh, y: f64) -> f64 {
    m.vertices
        .iter()
        .filter(|v| v.y > y)
        .map(|v| v.x.hypot(v.z))
        .fold(0., f64::max)
}

const KINDS: [&str; 4] = ["cone", "capsule", "torus", "ellipsoid"];

#[test]
fn the_falloff_shapes_each_preview_and_can_be_moved_and_turned_off() {
    for kind in KINDS {
        let mut e = armed(kind);
        let plain = preview(&e);
        e.dispatch(Command::SpatialFalloff(Some(ramp(0.75))))
            .unwrap();
        let shaped = preview(&e);
        assert_ne!(
            shaped.vertices, plain.vertices,
            "{kind}: the falloff shapes it"
        );
        assert_eq!(
            shaped.faces.len(),
            plain.faces.len(),
            "{kind}: same polygons"
        );
        // Scale 0% toward the full (top) end: the upper part narrows.
        assert!(
            widest_above(&shaped, 0.05) < widest_above(&plain, 0.05) - 1e-6,
            "{kind}: the top narrows"
        );
        // Moving the falloff rebuilds the preview again.
        e.dispatch(Command::SpatialFalloff(Some(ramp(3.)))).unwrap();
        assert_ne!(
            preview(&e).vertices,
            shaped.vertices,
            "{kind}: follows a move"
        );
        // Off: the plain shape, and the tool is still open.
        e.dispatch(Command::SpatialFalloff(None)).unwrap();
        assert_eq!(preview(&e).vertices, plain.vertices, "{kind}: off is plain");
    }
}

#[test]
fn apply_keeps_the_shaped_mesh_in_one_undo_step() {
    let mut e = armed("cone");
    e.dispatch(Command::SpatialFalloff(Some(ramp(0.75))))
        .unwrap();
    let shaped = preview(&e);
    let undo = e.history.undo.len();
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(e.history.undo.len(), undo + 1);
    assert_eq!(e.scene.active().unwrap().mesh.vertices, shaped.vertices);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh.vertices.is_empty());
}

#[test]
fn each_primitive_op_follows_the_falloff_step_below_and_matches_its_tool() {
    for (kind, type_id) in [
        ("cone", "prim.cone.item"),
        ("capsule", "prim.capsule.item"),
        ("torus", "prim.toroid.item"),
        ("ellipsoid", "prim.ellipsoid.item"),
    ] {
        let (mut e, id) = with_base(Mesh::default());
        e.dispatch(Command::SpatialFalloff(Some(ramp(0.75))))
            .unwrap();
        let mut values = serde_json::json!({"falloffScale": 0});
        if matches!(kind, "cone" | "capsule") {
            values["segments"] = 4.into();
        }
        // The Falloff step takes a copy of the active falloff.
        let step = add(&mut e, id, "falloff.meshop.item", serde_json::json!({}));
        add(&mut e, id, type_id, values);
        let op_mesh = ok(&e, id);

        let mut t = armed(kind);
        t.dispatch(Command::SpatialFalloff(Some(ramp(0.75))))
            .unwrap();
        assert_eq!(op_mesh.vertices, preview(&t).vertices, "{kind}: op = tool");

        // Clear the step's copy: the plain shape again.
        let mut clear = serde_json::Map::new();
        clear.insert("falloff".into(), serde_json::Value::Null);
        e.dispatch(Command::MeshOpSet {
            op: step,
            values: clear,
        })
        .unwrap();
        assert_ne!(ok(&e, id).vertices, op_mesh.vertices, "{kind}: cleared");
    }
}
