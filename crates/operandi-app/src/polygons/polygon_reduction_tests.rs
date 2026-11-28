//! MP-T11 RED-001..004 app routes for basic Polygon Reduction. The Polygon
//! panel Reduction row, viewport, Number field and Apply/Drop/Cancel buttons
//! receive synthetic egui pointer/key/text events at their drawn bounds; the
//! command routes are the production `execute` entry; a user remap is a
//! synthetic key event over the viewport. Geometry is judged by the core
//! test oracle (shared by path, independent of the reducer) and whole scenes
//! are compared with a separate `Editor` driven through the same kernel; that
//! comparison is integration evidence, not independent geometry parity.
//! These are worker-local synthetic-event checks: no native input, focus,
//! display-scale or native GUI claim is made.
//!
//! Fixture: the admitted original 25-point/32-triangle grid with its common
//! face values, every polygon explicitly selected, plus the fixture's
//! separate tagged unrelated item.
#[path = "../../../operandi-core/tests/support/reduction_oracle.rs"]
mod oracle;

use super::*;

#[test]
fn reduction_request_does_not_confirm_or_drop_active_pen() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.selection.mode = Mode::Polygon;
    a.camera.view = View::Front;
    a.execute("tool.set prim.pen on");
    assert!(a.editor.session.pen_draft.is_some());
    a.editor.dispatch(Command::PenCapture(DVec3::ZERO)).unwrap();
    let scene = a.editor.scene.clone();
    let draft = a.editor.session.pen_draft.clone();
    let undo = a.editor.history.undo.clone();
    let redo = a.editor.history.redo.clone();
    let ids = a.editor.identities.current.clone();
    a.fields.insert("pen_x".into(), "bad".into());
    let fields = a.fields.clone();
    a.run(Command::ArmReduction);
    assert!(a.editor.session.reduction.is_none());
    assert_eq!(a.editor.session.pen_draft, draft);
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.editor.identities.current, ids);
    assert!(a.editor.history.undo == undo && a.editor.history.redo == redo);
    assert_eq!(a.fields, fields);
}
