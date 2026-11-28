//! Scene tabs and the running tool session (untangle E3 guard): switching
//! tabs takes the open tool, its arming, tweaks and gesture state with the
//! scene, while the bookkeeping of the command running now and the last
//! transform drag stay with the app.
use glam::DVec3;
use operandi_core::*;

fn moving_cube() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::X,
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::CommitTransformStep).unwrap();
    e
}

#[test]
fn a_tab_switch_takes_the_tool_session_with_the_scene() {
    let mut a = moving_cube();
    assert!(a.session.live.is_some());
    assert!(a.session.last_transform_step.is_some());
    a.session.armed = Some(Tool::Scale);
    a.session.extend_from_click = true;
    a.session.gesture_partners = vec![(1, Component::Vertex(0))];
    a.session.tweak_held = true;
    a.session.dispatch_depth = 2;
    let before = a.clone();

    let mut b = Editor::default();
    a.swap_document_state(&mut b);
    // The scene's own session went with it.
    assert!(a.session.live.is_none());
    assert_eq!(a.session.armed, None);
    assert!(!a.session.extend_from_click);
    assert!(a.session.gesture_partners.is_empty());
    assert_eq!(b.session.live.as_ref().map(|l| l.tool), Some(Tool::Move));
    assert_eq!(b.session.armed, Some(Tool::Scale));
    assert!(b.session.extend_from_click);
    assert_eq!(b.session.gesture_partners, vec![(1, Component::Vertex(0))]);
    // Shared bookkeeping stayed.
    assert!(a.session.tweak_held);
    assert_eq!(a.session.dispatch_depth, 2);
    assert!(a.session.last_transform_step.is_some());
    assert!(!b.session.tweak_held);
    assert_eq!(b.session.dispatch_depth, 0);
    assert!(b.session.last_transform_step.is_none());

    a.swap_document_state(&mut b);
    assert!(a == before);
    assert!(b.session.live.is_none() && b.session.armed.is_none() && !b.session.extend_from_click);
}
