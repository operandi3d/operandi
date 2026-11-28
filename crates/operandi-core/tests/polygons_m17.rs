use glam::DVec3;
use operandi_core::*;
#[path = "support/m17.rs"]
mod support;
use support::*;

#[test]
fn t17_interior_vertices_remain_editable_without_orphan_slots() {
    let f = serde_json::json!({"vertices":[[0,0,0],[1,0,0],[2,0,0],[0,1,0],[1,1,0],[2,1,0],[0,2,0],[1,2,0],[2,2,0]],"faces":[[0,1,4,3],[1,2,5,4],[3,4,7,6],[4,5,8,7]],"selected_face_indices":[0,1,2,3]});
    for tool in [Tool::Bevel, Tool::Extrude] {
        let mut e = editor_from(&f);
        let base = e.scene.clone();
        set(&mut e, tool, 0.2, 0.1, DVec3::Z * 0.2, 3, true);
        let m = &e.scene.active().unwrap().mesh;
        assert_eq!(m.vertices.len(), counts(m)[0]);
        assert_eq!(m.vertices[4], DVec3::new(1., 1., 0.2));
        let first = e.scene.clone();
        e.dispatch(Command::Continue).unwrap();
        set(&mut e, tool, 0.1, 0.05, DVec3::Z * 0.1, 2, true);
        let m = &e.scene.active().unwrap().mesh;
        assert_eq!(m.vertices.len(), counts(m)[0]);
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.scene, first);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, base);
    }
}
#[test]
fn t17_real_24_corner_cylinder_cap_and_successful_staged_save() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.history.undo.clear();
    let mesh = cylinder_geometry(
        DVec3::new(0., -0.5, 0.),
        RadialParameters {
            radius: DVec3::new(2., 0.5, 2.),
            sides: 24,
            segments: 1,
        },
    )
    .unwrap();
    let top = mesh
        .faces
        .iter()
        .enumerate()
        .find(|(fi, f)| f.vertices.len() == 24 && mesh.normal(*fi).y > 0.)
        .unwrap()
        .0 as u32;
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.faces = vec![top];
    set(&mut e, Tool::Bevel, 0.2, 0.1, DVec3::ZERO, 1, true);
    assert_eq!(counts(&e.scene.active().unwrap().mesh), [72, 120, 50]);
    e.dispatch(Command::Continue).unwrap();
    set(&mut e, Tool::Bevel, 0.15, 0.05, DVec3::ZERO, 1, true);
    let scene = e.scene.clone();
    let folder = std::env::temp_dir().join(format!(
        "operandi-m17-save-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&folder).unwrap();
    let path = folder.join("cylinder.opscene");
    e.save_committed(&path).unwrap();
    assert!(e.session.live.is_none());
    assert_eq!(e.history.undo.len(), 2);
    assert_eq!(persistence::load(&path).unwrap(), scene);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(counts(&e.scene.active().unwrap().mesh), [72, 120, 50]);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(folder).unwrap();
}
