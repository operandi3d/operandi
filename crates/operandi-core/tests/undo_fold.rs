//! `Editor::undo_fold`: every step recorded under one gesture id is one
//! undo step (a colour-field drag over several items and frames).
use operandi_core::{Command, Editor};

fn smoothing(e: &Editor, amount: f64) -> Command {
    let mut settings = e.scene.smoothing.clone();
    settings.amount = amount;
    Command::SetSmoothing {
        settings,
        continuing: false,
    }
}

#[test]
fn one_gesture_is_one_undo_step() {
    let mut e = Editor::default();
    let start = e.scene.smoothing.amount;
    e.undo_fold = Some(7);
    for amount in [10., 20., 30.] {
        e.dispatch(smoothing(&e, amount)).unwrap();
    }
    e.undo_fold = None;
    assert_eq!(e.undo.len(), 1);
    assert_eq!(e.scene.smoothing.amount, 30.);
    // A new gesture (or no gesture) is a new step.
    e.undo_fold = Some(8);
    e.dispatch(smoothing(&e, 40.)).unwrap();
    e.undo_fold = None;
    e.dispatch(smoothing(&e, 50.)).unwrap();
    assert_eq!(e.undo.len(), 3);
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.smoothing.amount, 30.);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.smoothing.amount, start);
}
