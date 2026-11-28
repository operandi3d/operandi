//! Child of identity: inspect both private allocator marks, not Arc equality.
use crate::*;
#[test]
fn history_queries_do_not_consume_identity_or_stage_marks() {
    let mut editor = Editor {
        scene: serde_json::from_str(include_str!(
            "../../../../docs/i2/expectations/graphs/G0.json"
        ))
        .unwrap(),
        ..Default::default()
    };
    editor.synchronize_identities().unwrap();
    for state in 0..4 {
        match state {
            0 => {}
            1 => {
                editor.dispatch(Command::Activate(Tool::Extrude)).unwrap();
                editor
                    .dispatch(Command::ConfigureExtrude(Parameters {
                        extrude_mode: ExtrudeMode::Shift,
                        extrude_uvs: ExtrudeUvs::None,
                        ..Default::default()
                    }))
                    .unwrap();
                editor.dispatch(Command::BeginExtrude).unwrap();
            }
            2 => {
                let mut p = editor.session.live.as_ref().unwrap().params.clone();
                p.extent = glam::DVec3::Z * 0.5;
                editor.dispatch(Command::Preview(p)).unwrap();
            }
            _ => {
                editor.dispatch(Command::Undo).unwrap();
                editor.dispatch(Command::Undo).unwrap();
            }
        }
        let before = editor.clone();
        let marks = editor.identities.marks().unwrap();
        let serial = editor.i2.serial.next().unwrap();
        for _ in 0..100 {
            let ownership = editor.session_ownership();
            let _ = ownership.local_resume_obstacle();
            for action in [HistoryAction::Undo, HistoryAction::Redo] {
                let _ = ownership.history_route(
                    action,
                    crate::session_ownership::HistoryPosition::of(&editor),
                );
                let _ = editor.history_eligibility(action);
            }
        }
        assert!(editor == before);
        assert_eq!(editor.identities.marks().unwrap(), marks);
        assert_eq!(editor.i2.serial.next().unwrap(), serial);
    }
}
