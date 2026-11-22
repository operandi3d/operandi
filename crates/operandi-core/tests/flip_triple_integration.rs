use operandi_core::*;
use std::path::PathBuf;

#[test]
fn flip_and_triple_both_orders_preserve_payload_identity_history_and_save() {
    for flip_first in [true, false] {
        let mut e = Editor::default();
        e.scene.new_mesh().unwrap();
        let m = &mut e.scene.active_mut().unwrap().mesh;
        m.vertices = vec![
            [0., 0., 0.].into(),
            [1., 0., 0.].into(),
            [1., 1., 0.].into(),
            [0., 1., 0.].into(),
        ];
        m.faces = vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: "mat".into(),
            smooth: false,
            part: Some("part".into()),
            material_label: None,
        }]
        .into();
        m.attributes.corner_weights.insert(
            "weights".into(),
            vec![vec![Some(0.1), None, Some(0.3), Some(0.4)]],
        );
        e.scene.selection.mode = Mode::Polygon;
        e.scene.selection.faces = vec![0];
        e.scene.validate().unwrap();
        e.synchronize_identities().unwrap();
        let source = e.scene.clone();
        let initial = e.identities.current.clone();
        let ops = if flip_first {
            [Command::FlipPolygons, Command::Triple]
        } else {
            [Command::Triple, Command::FlipPolygons]
        };
        e.dispatch(ops[0].clone()).unwrap();
        let middle = e.scene.clone();
        let mid_ids = e.identities.current.clone();
        e.dispatch(ops[1].clone()).unwrap();
        let end = e.scene.clone();
        let end_ids = e.identities.current.clone();
        let m = &end.active().unwrap().mesh;
        assert_eq!((m.vertices.len(), m.faces.len()), (4, 2));
        assert_eq!(end.selection.faces.len(), 2);
        for face in &m.faces {
            assert_eq!(face.material, "mat");
            assert_eq!(face.part.as_deref(), Some("part"));
        }
        for (fi, face) in m.faces.iter().enumerate() {
            for (ci, &vertex) in face.vertices.iter().enumerate() {
                assert_eq!(
                    m.attributes.corner_weights["weights"][fi][ci],
                    [Some(0.1), None, Some(0.3), Some(0.4)][vertex as usize]
                );
            }
        }
        let item = end.selection.item.unwrap();
        assert_eq!(
            end_ids.meshes[&item].vertices,
            initial.meshes[&item].vertices
        );
        if flip_first {
            let mut reversed = initial.meshes[&item].corners[0].clone();
            reversed[1..].reverse();
            assert_eq!(mid_ids.meshes[&item].corners[0], reversed);
        } else {
            for &i in &middle.selection.faces {
                let mut reversed = mid_ids.meshes[&item].corners[i as usize].clone();
                reversed[1..].reverse();
                assert_eq!(end_ids.meshes[&item].corners[i as usize], reversed);
            }
        }
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, middle);
        assert_eq!(e.identities.current, mid_ids);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, source);
        assert_eq!(e.identities.current, initial);
        e.dispatch(Command::Redo).unwrap();
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, end);
        assert_eq!(e.identities.current, end_ids);
        let path = PathBuf::from(format!(
            "{}/flip-triple-{}.opscene",
            std::env::temp_dir().display(),
            std::process::id()
        ));
        persistence::save(&path, &end).unwrap();
        assert_eq!(persistence::load(&path).unwrap(), end);
        std::fs::remove_file(path).unwrap();
    }
}
