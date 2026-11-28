//! Replicators (G8 RP-4): renders get every copy as its prototype's mesh
//! placed by the copy's matrix, built once and shared (same points and
//! triangles, so the Cycles session instances one mesh).
use super::*;
use operandi_core::evaluate::Purpose;
use operandi_core::{Command, Editor, Mesh, Mode, Tool};

#[test]
fn replicator_copies_render_as_shared_meshes() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh =
        Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::splat(0.5), 0)
            .unwrap()
            .into();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::Activate(Tool::Array)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.duplicate.copier = Some(operandi_core::duplicate_copier::Copier::Replica);
    p.duplicate.array.count = [3, 1, 2];
    p.duplicate.placed = true;
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(
        scene_meshes(&e.scene).unwrap().len(),
        1,
        "the document: one mesh"
    );
    let meshes = scene_meshes(&e.evaluated(Purpose::Final)).unwrap();
    assert_eq!(meshes.len(), 6, "the original and five copies");
    let first = &meshes[1];
    for m in &meshes[2..] {
        assert_eq!(m.positions, first.positions);
        assert_eq!(m.triangles, first.triangles);
        assert_ne!(m.transform, first.transform);
    }
    assert_eq!(
        meshes[0].positions, first.positions,
        "the prototype's own mesh"
    );
}
