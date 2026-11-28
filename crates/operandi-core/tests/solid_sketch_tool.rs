//! Solid Sketch tool session (spec `solid-sketch` v0.3 sections 5, 7, 8 and
//! tests 12, 13, 14, 23, 25): live rebuild, steps inside the tool, one undo
//! step per session, Cancel, no-mesh refusal, keep the sketch.
use glam::{DQuat, DVec3};
use operandi_core::solid_sketch::{SketchOptions, SubdivMode};
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.synchronize_identities().unwrap();
    e
}
fn start(e: &mut Editor) {
    e.dispatch(Command::SolidSketchStart {
        options: SketchOptions::ss_default(),
        resume: true,
    })
    .unwrap();
}
fn edit(e: &mut Editor, edit: SketchEdit) {
    e.dispatch(Command::SolidSketch {
        edit,
        continuing: false,
    })
    .unwrap();
}
fn add(e: &mut Editor, x: f64) {
    edit(
        e,
        SketchEdit::Add {
            at: DVec3::new(x, 0., 0.),
            size: None,
            up: None,
        },
    );
}
fn faces(e: &Editor) -> usize {
    e.scene.active().map_or(0, |i| i.mesh.faces.len())
}
fn session(e: &Editor) -> &SketchSession {
    e.session.solid_sketch.as_ref().unwrap()
}

#[test]
fn no_mesh_layer_refuses_with_the_message() {
    let mut e = Editor::default();
    let err = e
        .dispatch(Command::SolidSketchStart {
            options: SketchOptions::default(),
            resume: false,
        })
        .unwrap_err();
    assert_eq!(
        err,
        "Solid Sketch needs a mesh layer. Add or select a mesh first."
    );
    assert!(e.session.solid_sketch.is_none());
}

#[test]
fn clicks_build_a_chain_live_and_drop_is_one_step() {
    let mut e = editor();
    let before = e.scene.clone();
    let steps = e.history.undo.len();
    start(&mut e);
    assert_eq!(e.session.active_task(), Some(ActiveTask::SolidSketch));
    assert_eq!(session(&e).options().subdiv, SubdivMode::Level1);
    for x in [0., 1., 2.] {
        add(&mut e, x);
    }
    assert_eq!(faces(&e), 22);
    assert_eq!(session(&e).current(), Some(2));
    assert_eq!(session(&e).sketch().nodes[2].parent, Some(1));
    // Subdiv Mode Level 1: subdivision polygons.
    let mesh = &e.scene.active().unwrap().mesh;
    assert!(
        mesh.attributes
            .representations
            .iter()
            .all(|r| *r == PolygonRepresentation::Subdivs)
    );
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.solid_sketch.is_none());
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Solid Sketch");
    assert_eq!(faces(&e), 22);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}

#[test]
fn undo_inside_the_tool_steps_back_one_edit_and_a_drag_is_one_step() {
    let mut e = editor();
    start(&mut e);
    add(&mut e, 0.);
    add(&mut e, 1.);
    // A drag: first frame a step, later frames join it.
    for (k, continuing) in [(0, false), (1, true), (2, true)] {
        let _ = k;
        e.dispatch(Command::SolidSketch {
            edit: SketchEdit::Translate {
                index: 1,
                delta: DVec3::Z * 0.1,
                branch: false,
            },
            continuing,
        })
        .unwrap();
    }
    assert!((session(&e).sketch().nodes[1].center.z - 0.3).abs() < 1e-12);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.session.solid_sketch.is_some());
    assert_eq!(session(&e).sketch().nodes[1].center, DVec3::X);
    e.dispatch(Command::Redo).unwrap();
    assert!((session(&e).sketch().nodes[1].center.z - 0.3).abs() < 1e-12);
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(session(&e).sketch().len(), 1);
    assert_eq!(faces(&e), 6);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(faces(&e), 0);
    // Nothing of its own left: Undo closes the tool, changing nothing.
    e.dispatch(Command::Undo).unwrap();
    assert!(e.session.solid_sketch.is_none());
}

#[test]
fn cancel_restores_the_layer() {
    let mut e = editor();
    let before = e.scene.clone();
    start(&mut e);
    add(&mut e, 0.);
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.session.solid_sketch.is_none());
    assert_eq!(e.scene, before);
}

#[test]
fn bare_tool_defaults_to_level_2_and_preset_to_level_1() {
    assert_eq!(SketchOptions::default().subdiv, SubdivMode::Level2);
    let p = SketchOptions::ss_default();
    assert_eq!(p.subdiv, SubdivMode::Level1);
    assert_eq!(p.edit_mode, solid_sketch::EditMode::CreateMoveScale);
}

#[test]
fn node_fields_need_a_node_and_clamp() {
    let mut e = editor();
    start(&mut e);
    let err = e
        .dispatch(Command::SolidSketch {
            edit: SketchEdit::SetNode {
                center: DVec3::ZERO,
                size: DVec3::ONE,
                twist: 0.,
            },
            continuing: false,
        })
        .unwrap_err();
    assert!(err.contains("no nodes"), "{err}");
    add(&mut e, 0.);
    add(&mut e, 1.);
    edit(&mut e, SketchEdit::Select(Some(99)));
    assert_eq!(session(&e).current(), Some(1));
    edit(
        &mut e,
        SketchEdit::SetNode {
            center: DVec3::new(1., 2., 0.),
            size: DVec3::new(0., 0.2, 0.3),
            twist: 0.5,
        },
    );
    let n = &session(&e).sketch().nodes[1];
    assert_eq!(n.center, DVec3::new(1., 2., 0.));
    assert!(n.size.x > 0. && n.size.x < 1e-3);
    assert_eq!(n.twist, 0.5);
}

#[test]
fn branching_from_a_picked_node() {
    let mut e = editor();
    start(&mut e);
    add(&mut e, 0.);
    add(&mut e, 1.);
    add(&mut e, 2.);
    edit(&mut e, SketchEdit::Select(Some(1)));
    edit(
        &mut e,
        SketchEdit::Add {
            at: DVec3::new(1., 0., 1.),
            size: Some(0.2),
            up: None,
        },
    );
    let s = session(&e).sketch();
    assert_eq!(s.nodes[3].parent, Some(1));
    assert_eq!(s.nodes[3].size, DVec3::splat(0.2));
    assert_eq!(faces(&e), 30);
}

#[test]
fn delete_rotate_and_scale_through_commands() {
    let mut e = editor();
    start(&mut e);
    for x in [0., 1., 2.] {
        add(&mut e, x);
    }
    edit(
        &mut e,
        SketchEdit::Rotate {
            index: 1,
            turn: DQuat::from_rotation_y(std::f64::consts::FRAC_PI_2),
        },
    );
    assert!(
        session(&e).sketch().nodes[2]
            .center
            .distance(DVec3::new(1., 0., -1.))
            < 1e-9
    );
    edit(
        &mut e,
        SketchEdit::Scale {
            index: 0,
            factor: 2.,
            branch: false,
        },
    );
    assert_eq!(session(&e).sketch().nodes[0].size, DVec3::splat(0.2));
    edit(
        &mut e,
        SketchEdit::Delete {
            index: 1,
            branch: false,
        },
    );
    assert_eq!(session(&e).sketch().len(), 2);
    assert_eq!(session(&e).current(), Some(0));
    assert_eq!(faces(&e), 14);
    edit(
        &mut e,
        SketchEdit::Delete {
            index: 0,
            branch: true,
        },
    );
    assert_eq!(faces(&e), 0);
    assert_eq!(session(&e).current(), None);
}

#[test]
fn existing_geometry_stays_and_the_sketch_owns_only_its_polygons() {
    let mut e = editor();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::new(5., 0., 0.), DVec3::ONE, 0)
        .unwrap()
        .into();
    e.synchronize_identities().unwrap();
    start(&mut e);
    add(&mut e, 0.);
    assert_eq!(faces(&e), 12);
    e.dispatch(Command::Drop).unwrap();
    // Picking it up again rebuilds the layer from the nodes: the other 6
    // polygons go, and the question says so.
    assert_eq!(solid_sketch_store::resume_question(&e), Some(6));
    start(&mut e);
    assert_eq!(faces(&e), 6);
    assert_eq!(session(&e).current(), Some(0));
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(faces(&e), 12);
}

#[test]
fn keep_the_sketch_round_trips_save_and_reactivation() {
    let mut e = editor();
    start(&mut e);
    for x in [0., 1., 2.] {
        add(&mut e, x);
    }
    edit(
        &mut e,
        SketchEdit::SetNode {
            center: DVec3::new(2., 0.5, 0.),
            size: DVec3::new(0.1, 0.2, 0.3),
            twist: 0.3,
        },
    );
    e.dispatch(Command::Drop).unwrap();
    let saved = persistence::encode(&e.scene).unwrap();
    let scene = persistence::decode(&saved).unwrap();
    let mesh_before = e.scene.active().unwrap().mesh.clone();
    let mut reopened = Editor {
        scene,
        ..Default::default()
    };
    reopened.synchronize_identities().unwrap();
    let mesh = &reopened.scene.active().unwrap().mesh;
    assert_eq!(
        mesh.attributes.solid_sketch,
        mesh_before.attributes.solid_sketch
    );
    assert_eq!(solid_sketch_store::resume_question(&reopened), Some(0));
    start(&mut reopened);
    let rebuilt = &reopened.scene.active().unwrap().mesh;
    assert_eq!(rebuilt.faces.len(), mesh_before.faces.len());
    for (a, b) in rebuilt.vertices.iter().zip(&mesh_before.vertices) {
        assert!(a.distance(*b) < 1e-6);
    }
    assert_eq!(session(&reopened).sketch().len(), 3);
    assert_eq!(session(&reopened).current(), Some(2));
}

#[test]
fn no_starts_a_separate_sketch_and_keeps_the_old_polygons() {
    let mut e = editor();
    start(&mut e);
    add(&mut e, 0.);
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::SolidSketchStart {
        options: SketchOptions::ss_default(),
        resume: false,
    })
    .unwrap();
    assert!(session(&e).sketch().is_empty());
    assert_eq!(faces(&e), 6);
    add(&mut e, 3.);
    assert_eq!(faces(&e), 12);
    e.dispatch(Command::Drop).unwrap();
    let data = e
        .scene
        .active()
        .unwrap()
        .mesh
        .attributes
        .solid_sketch
        .clone();
    assert_eq!(data.unwrap().sketch.len(), 1);
}

#[test]
fn strip_removes_the_data_as_one_step() {
    let mut e = editor();
    start(&mut e);
    add(&mut e, 0.);
    e.dispatch(Command::Drop).unwrap();
    let steps = e.history.undo.len();
    e.dispatch(Command::SolidSketchStrip).unwrap();
    assert!(
        e.scene
            .active()
            .unwrap()
            .mesh
            .attributes
            .solid_sketch
            .is_none()
    );
    assert_eq!(faces(&e), 6);
    assert_eq!(e.history.undo.len(), steps + 1);
    assert!(e.dispatch(Command::SolidSketchStrip).is_err());
    e.dispatch(Command::Undo).unwrap();
    assert!(
        e.scene
            .active()
            .unwrap()
            .mesh
            .attributes
            .solid_sketch
            .is_some()
    );
}

#[test]
fn hidden_args_and_options_round_trip() {
    let mut e = editor();
    start(&mut e);
    let mut o = session(&e).options().clone();
    o.hidden.tension = 25.;
    o.hidden.material = "Skin".into();
    o.hidden.uvmap = "Texture".into();
    o.hidden.rot_u = 0.5;
    o.auto_merge = true;
    edit(&mut e, SketchEdit::Options(o.clone()));
    add(&mut e, 0.);
    e.dispatch(Command::Drop).unwrap();
    let data = e
        .scene
        .active()
        .unwrap()
        .mesh
        .attributes
        .solid_sketch
        .clone()
        .unwrap();
    assert_eq!(data.options, o);
    // Kept for the next session (Q18).
    assert!(e.solid_sketch_options.auto_merge);
}

#[test]
fn another_tool_drops_the_sketch_first() {
    let mut e = editor();
    start(&mut e);
    add(&mut e, 0.);
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(e.session.solid_sketch.is_none());
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(faces(&e), 6);
}

#[test]
fn export_writes_only_the_polygons_never_sketch_lines() {
    let mut e = editor();
    start(&mut e);
    for x in [0., 1.] {
        add(&mut e, x);
    }
    e.dispatch(Command::Drop).unwrap();
    let dir = std::env::temp_dir().join(format!("operandi-solid-sketch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, format) in [
        ("s.obj", interchange::ExportFormat::Obj),
        ("s.lxo", interchange::ExportFormat::Lxo),
        ("s.glb", interchange::ExportFormat::Glb),
    ] {
        let path = dir.join(name);
        let report = interchange::export(&path, &e.scene, format).unwrap();
        assert_eq!(report.polygons, 14, "{name}");
    }
    let obj = std::fs::read_to_string(dir.join("s.obj")).unwrap();
    assert_eq!(obj.lines().filter(|l| l.starts_with("l ")).count(), 0);
    assert_eq!(obj.lines().filter(|l| l.starts_with("f ")).count(), 14);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn copy_of_the_mesh_item_keeps_the_sketch() {
    let mut e = editor();
    start(&mut e);
    add(&mut e, 0.);
    e.dispatch(Command::Drop).unwrap();
    let item = e.scene.active().unwrap().clone();
    assert!(item.mesh.attributes.solid_sketch.is_some());
}
