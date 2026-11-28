//! Astra P9-001: a multi-mesh command records one step per mesh and joins
//! them into one. Undo memory trimming in between used to shorten the
//! history under it, and joining the steps then panicked.
use glam::DVec3;
use operandi_core::*;

fn add_cube(e: &mut Editor, x: f64) -> u64 {
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        position: DVec3::new(x, 0., 0.),
        size: DVec3::splat(2.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.scene.active().unwrap().id
}

#[test]
fn multi_mesh_commands_survive_undo_memory_trimming() {
    let mut e = Editor::default();
    let a = add_cube(&mut e, 0.);
    let b = add_cube(&mut e, 6.);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::SelectItems(vec![a, b])).unwrap();
    e.dispatch(Command::ClearSelection).unwrap();
    for item in [a, b] {
        e.dispatch(Command::Select {
            item,
            hit: Component::Polygon(0),
            replace: false,
            remove: false,
        })
        .unwrap();
    }
    for _ in 0..4 {
        e.dispatch(Command::FlipPolygons).unwrap();
    }
    assert!(e.history.undo.len() > 2);
    let before = e.scene.clone();
    // A budget so small that every recorded step trims all older ones, as
    // a scene near the real budget does.
    e.history.undo_memory_budget = Some(1);
    e.dispatch(Command::FlipPolygons).unwrap();
    assert_eq!(
        e.history.undo.len(),
        1,
        "trimmed once the steps were joined"
    );
    assert!(!e.history.undo_trim_paused);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before, "the joined step undoes both meshes");
}
