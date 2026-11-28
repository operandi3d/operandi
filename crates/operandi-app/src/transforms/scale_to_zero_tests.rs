//! GitHub #24: the Scale tool's handles reach 0% (they stopped at 0.1%),
//! and a drag that starts at 0 can grow again.
use super::tests::box_app;
use super::*;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

fn delta(a: &Operandi) -> DVec3 {
    a.editor.session.live.as_ref().unwrap().params.delta
}

#[test]
fn scale_handles_reach_zero_and_grow_back() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    let original = a.editor.scene.active().unwrap().mesh.clone();
    a.activate(Tool::Scale);
    let start = pos2(400., 300.);
    // The center handle: far left/down goes to exactly 0, not 0.1%.
    let base = a.editor.session.live.as_ref().unwrap().params.clone();
    a.update_handle(
        viewport::SCALE_UNIFORM_HANDLE,
        base,
        start,
        start + vec2(-400., 400.),
        Tool::Scale,
        rect(),
    );
    assert_eq!(delta(&a), DVec3::ZERO, "{}", a.status);
    // From 0 a drag right grows all three axes together.
    let base = a.editor.session.live.as_ref().unwrap().params.clone();
    a.update_handle(
        viewport::SCALE_UNIFORM_HANDLE,
        base,
        start,
        start + vec2(50., 0.),
        Tool::Scale,
        rect(),
    );
    assert!(
        (delta(&a) - DVec3::splat(0.5)).abs().max_element() < 1e-9,
        "{}",
        delta(&a)
    );
    // An axis handle dragged far back past its centre stops at 0.
    let handles = a.handles(rect());
    let (axis, origin, end, _) = handles
        .iter()
        .find(|h| h.0 < 3)
        .cloned()
        .expect("an axis handle");
    let base = a.editor.session.live.as_ref().unwrap().params.clone();
    a.update_handle(
        axis,
        base,
        end,
        origin - (end - origin) * 40.,
        Tool::Scale,
        rect(),
    );
    let d = delta(&a);
    assert_eq!(d[axis], 0., "{d}");
    assert!(d.is_finite());
    // The preview is finite and the drop is one undo step that undoes.
    let steps = a.editor.history.undo.len();
    a.run(Command::Drop);
    let after = a.editor.scene.active().unwrap().mesh.clone();
    assert!(after.vertices.iter().all(|v| v.is_finite()));
    assert_eq!(a.editor.history.undo.len(), steps + 1);
    a.run(Command::Undo);
    assert_eq!(*a.editor.scene.active().unwrap().mesh, *original);
}
