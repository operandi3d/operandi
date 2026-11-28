//! Jason, 2026-11-01 (confirmed 2026-11-03): a position-only change keeps component identities (a
//! moved point is the same point); a topology change still rebinds them.
use glam::DVec3;
use operandi_core::*;

fn cube() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    let p = Parameters {
        size: DVec3::splat(2.),
        ..Default::default()
    };
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e
}

fn ids(e: &mut Editor) -> MeshIdentity {
    e.synchronize_identities().unwrap();
    (*e.identities.current.meshes[&1]).clone()
}

#[test]
fn moving_components_keeps_their_identities() {
    let mut e = cube();
    let before = ids(&mut e);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::new(0.3, -0.2, 0.1);
    e.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(ids(&mut e), before, "preview keeps IDs");
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(ids(&mut e), before, "drop keeps IDs");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(ids(&mut e), before, "undo keeps IDs");
}

#[test]
fn changing_topology_still_rebinds_identities() {
    let mut e = cube();
    let before = ids(&mut e);
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let p = Parameters {
        inset: 0.2,
        shift: 0.1,
        ..Default::default()
    };
    e.dispatch(Command::Preview(p)).unwrap();
    let after = ids(&mut e);
    assert_ne!(after.scope, before.scope, "a new identity generation");
}
