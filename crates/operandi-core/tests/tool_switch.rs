use glam::{DVec2, DVec3};
use operandi_core::*;

fn fixture() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters::default())).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    for (i, f) in e
        .scene
        .active_mut()
        .unwrap()
        .mesh
        .faces
        .iter_mut()
        .enumerate()
    {
        f.uv = Some(vec![DVec2::splat(i as f64); f.vertices.len()]);
        f.material = format!("material-{i}");
        f.part = Some(format!("part-{i}"));
        f.material_label = Some(format!("label-{i}"));
        f.smooth = i % 2 == 0;
    }
    e.scene.selection.faces = vec![5];
    e
}

#[test]
fn switching_commits_each_stage_once_and_cancel_restores_new_base() {
    let mut e = fixture();
    let original = e.scene.clone();
    let history = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    for inset in [0.1, 0.05, 0.1] {
        e.dispatch(Command::Preview(Parameters {
            inset,
            ..Default::default()
        }))
        .unwrap();
    }
    e.dispatch(Command::Continue).unwrap();
    let a = e.scene.clone();
    e.dispatch(Command::Preview(Parameters {
        inset: 0.05,
        shift: 0.1,
        ..Default::default()
    }))
    .unwrap();
    let b = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    assert_eq!(e.history.undo.len(), history + 2);
    assert_eq!(e.session.live.as_ref().unwrap().base, b);
    for extent in [0.2, 0.1, 0.2] {
        e.dispatch(Command::Preview(Parameters {
            extent: DVec3::Y * extent,
            ..Default::default()
        }))
        .unwrap();
    }
    let c = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert_eq!(e.history.undo.len(), history + 3);
    assert_eq!(e.session.live.as_ref().unwrap().base, c);
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::X,
        ..Default::default()
    }))
    .unwrap();
    // Component transforms retain every generated polygon's corner attributes.
    assert_eq!(
        e.scene.active().unwrap().mesh.faces,
        c.active().unwrap().mesh.faces
    );
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, c);
    for expected in [&b, &a, &original] {
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(&e.scene, expected);
    }
    for expected in [&a, &b, &c] {
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(&e.scene, expected);
        e.scene.validate().unwrap();
        assert!(e.session.live.is_none());
    }
    let path = std::env::temp_dir().join(format!(
        "operandi-tool-switch-{}.opscene",
        std::process::id()
    ));
    e.save_committed(&path).unwrap();
    assert_eq!(persistence::load(&path).unwrap(), c);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn every_tool_can_switch_to_another_eligible_tool() {
    let tools = [
        Tool::Box,
        Tool::Sphere,
        Tool::Cylinder,
        Tool::Transform,
        Tool::Move,
        Tool::Rotate,
        Tool::Scale,
        Tool::Bevel,
        Tool::Extrude,
    ];
    for from in tools {
        for to in tools {
            if from == to {
                continue;
            }
            let mut e = fixture();
            e.dispatch(Command::Activate(from)).unwrap();
            let mut p = e.session.live.as_ref().unwrap().params.clone();
            p.position = DVec3::X * 2.;
            p.inset = 0.05;
            p.extent = DVec3::Y * 0.1;
            p.delta = if from == Tool::Scale {
                DVec3::splat(1.1)
            } else {
                DVec3::splat(0.1)
            };
            e.dispatch(Command::Preview(p.clone())).unwrap();
            if from == Tool::Box {
                e.dispatch(Command::Apply).unwrap();
            }
            let scene = e.scene.clone();
            let history = e.history.undo.len();
            e.dispatch(Command::Activate(to))
                .unwrap_or_else(|error| panic!("{from:?} -> {to:?}: {error}"));
            assert_eq!(e.scene, scene);
            let live = e.session.live.as_ref().unwrap();
            assert_eq!(live.tool, to);
            assert_eq!(live.base, scene);
            assert_eq!(e.history.undo.len(), history + 1);
            assert_eq!(e.history.undo.last().unwrap().label, from.label());
            if from == Tool::Box {
                assert_eq!(*e.presets.get::<operandi_core::CubePreset>(), p);
            }
            e.dispatch(Command::Cancel).unwrap();
            assert_eq!(e.scene, scene);
        }
    }
}

#[test]
fn invalid_target_preserves_task_history_and_parameters() {
    let mut e = fixture();
    e.scene.selection.faces.clear();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::X,
        ..Default::default()
    }))
    .unwrap();
    let before = e.clone();
    // Edge Bevel needs selected edges (Bevel and Extrude no longer need a
    // selection, PB-O-01 / PE-O-03).
    assert!(e.dispatch(Command::Activate(Tool::EdgeBevel)).is_err());
    assert!(e == before);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, before.session.live.unwrap().base);
}

#[test]
fn neutral_flat_and_same_tool_keep_drop_semantics() {
    let mut e = fixture();
    let scene = e.scene.clone();
    let history = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Activate(Tool::Sphere)).unwrap();
    assert_eq!(e.history.undo.len(), history);
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.radial.as_mut().unwrap().radius.y = 0.;
    p.radial.as_mut().unwrap().sides = 16;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Activate(Tool::Cylinder)).unwrap();
    assert_eq!(e.sphere_unit_resolution, Some([16, 12]));
    // drag-creation-2026-11-06: the flat base is kept as one step.
    assert_ne!(e.scene, scene);
    assert_eq!(e.history.undo.len(), history + 1);
    let history = history + 1;
    e.dispatch(Command::Activate(Tool::Cylinder)).unwrap();
    assert!(e.session.live.is_none());
    assert_eq!(e.history.undo.len(), history);
}
