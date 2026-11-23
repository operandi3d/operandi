//! The Items list's rig rows (RG-01, IK-02, SK-36) and `operandi.rig.rename`.
use operandi_core::rig_list::{self, RigRow};
use operandi_core::*;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

#[test]
fn rows_list_pieces_then_the_skin_and_rename_is_one_step() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let a = e.scene.items[0].id;
    e.scene.new_mesh().unwrap();
    let b = e.scene.items[1].id;
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{a} targets:{b}"),
    );
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:rotation items:{a} targets:{b}"),
    );
    let rows = rig_list::rows(&e.scene, a);
    let names: Vec<String> = rows
        .iter()
        .map(|r| match r {
            RigRow::Piece { name, .. } => name.clone(),
            RigRow::Skin { .. } => "skin".into(),
        })
        .collect();
    assert_eq!(names, ["Position Constraint", "Rotation Constraint"]);
    assert!(rig_list::rows(&e.scene, b).is_empty());
    let piece = e.scene.animation.rig.pieces[0].id;
    let steps = e.undo.len();
    ok(
        &mut e,
        &format!("operandi.rig.rename piece:{piece} name:\"Follow B\""),
    );
    assert_eq!(e.undo.len(), steps + 1);
    assert!(
        matches!(&rig_list::rows(&e.scene, a)[0], RigRow::Piece { name, .. } if name == "Follow B")
    );
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(
        matches!(&rig_list::rows(&back, a)[0], RigRow::Piece { name, .. } if name == "Follow B")
    );
    // An empty name goes back to the label.
    ok(
        &mut e,
        &format!("operandi.rig.rename piece:{piece} name:\"\""),
    );
    assert!(
        matches!(&rig_list::rows(&e.scene, a)[0], RigRow::Piece { name, .. } if name == "Position Constraint")
    );
    e.dispatch(Command::Undo).unwrap();
    assert!(
        matches!(&rig_list::rows(&e.scene, a)[0], RigRow::Piece { name, .. } if name == "Follow B")
    );
}
