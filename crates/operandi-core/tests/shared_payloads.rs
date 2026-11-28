//! Structural regression checks for copy-on-write mesh sharing. Drag samples,
//! staged dispatch copies, live bases, I2 stage images and history checkpoints
//! must keep sharing the allocation of a mesh they did not edit, instead of
//! deep-copying it. Timings live in `examples/drag_cost.rs`.
use glam::DVec3;
use operandi_core::*;
use std::path::PathBuf;

fn editor_with_untouched_item() -> Editor {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/i2/expectations/graphs/G0.json");
    let mut scene: Scene = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut other = scene.items[0].clone();
    other.id = scene.next_id;
    other.name = "Untouched".into();
    other.transform.translation = DVec3::new(5., 0., 0.);
    scene.next_id += 1;
    scene.items.push(other);
    let mut e = Editor {
        scene,
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    e
}
fn p(z: f64) -> Parameters {
    Parameters {
        extent: DVec3::Z * z,
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }
}

#[test]
fn untouched_mesh_and_identities_stay_shared_through_i2_and_history() {
    let mut e = editor_with_untouched_item();
    let mesh = e.scene.items[1].mesh.clone();
    let ids = e.identities.current.meshes[&2].clone();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(p(0.))).unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    e.dispatch(Command::Preview(p(0.5))).unwrap();
    let stage = e.i2.selected.as_ref().unwrap();
    for (label, scene) in [
        ("current", &e.scene),
        ("live base", &e.session.live.as_ref().unwrap().base),
        ("stage base", &stage.base.scene),
        ("stage begun", &stage.begun.scene),
        ("stage movement", &stage.movement.as_ref().unwrap().scene),
    ] {
        assert!(
            Shared::same(scene.items[1].mesh.shared(), mesh.shared()),
            "{label}"
        );
    }
    assert!(Shared::same(&e.identities.current.meshes[&2], &ids));
    assert!(Shared::same(
        &stage.movement.as_ref().unwrap().identities.meshes[&2],
        &ids
    ));
    // The edited mesh did change, through a private copy.
    assert!(!Shared::same(
        e.scene.items[0].mesh.shared(),
        stage.base.scene.items[0].mesh.shared()
    ));
    assert_ne!(e.scene.items[0].mesh, stage.base.scene.items[0].mesh);

    e.dispatch(Command::Drop).unwrap();
    let checkpoint = e.history.undo.last().unwrap();
    assert!(Shared::same(
        checkpoint.before.items[1].mesh.shared(),
        mesh.shared()
    ));
    assert!(Shared::same(
        checkpoint.after.items[1].mesh.shared(),
        mesh.shared()
    ));
    e.dispatch(Command::Undo).unwrap();
    assert!(Shared::same(e.scene.items[1].mesh.shared(), mesh.shared()));
    e.dispatch(Command::Redo).unwrap();
    assert!(Shared::same(e.scene.items[1].mesh.shared(), mesh.shared()));
}

#[test]
fn editing_one_snapshot_never_changes_another() {
    let mut e = editor_with_untouched_item();
    let before = e.clone();
    e.scene.items[1].mesh.vertices[0].x += 1.;
    assert_ne!(e.scene.items[1].mesh, before.scene.items[1].mesh);
    assert_eq!(
        before.scene.items[1].mesh.vertices[0].x + 1.,
        e.scene.items[1].mesh.vertices[0].x
    );
    // Byte-identical persistence: sharing is invisible in the file format.
    let shared = persistence::encode(&before.scene).unwrap();
    let detached = persistence::decode(&shared).unwrap();
    assert_eq!(persistence::encode(&detached).unwrap(), shared);
    assert!(!Shared::same(
        detached.items[1].mesh.shared(),
        before.scene.items[1].mesh.shared()
    ));
    assert_eq!(detached, before.scene);
}
