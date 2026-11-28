//! Replicators (G8 RP-4): the USD export writes a Replicator's copies as a
//! PointInstancer, however few there are.
use operandi_core::{Command, Editor, Mesh, Mode, Tool};

#[test]
fn replicator_copies_export_as_a_point_instancer() {
    operandi_core::interchange::set_usd_exporter(crate::export);
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().name = "Rock".into();
    e.scene.active_mut().unwrap().mesh =
        Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::splat(0.5), 0)
            .unwrap()
            .into();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::Activate(Tool::Array)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.duplicate.copier = Some(operandi_core::duplicate_copier::Copier::Replica);
    p.duplicate.array.count = [3, 1, 1];
    p.duplicate.placed = true;
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
    let dir = std::env::temp_dir().join(format!("operandi-usd-replica-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("replica.usda");
    let mut args = serde_json::Map::new();
    args.insert(
        "filename".into(),
        path.to_string_lossy().into_owned().into(),
    );
    operandi_core::command_exec::run(&mut e, "scene.saveAs", &args).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("def PointInstancer"), "{text}");
    let back = crate::import(&path, &Default::default()).unwrap();
    let instances = back
        .scene
        .items
        .iter()
        .filter(|i| i.instance_of.is_some())
        .count();
    assert_eq!(instances, 2, "the two copies come back as instances");
}
