//! The Skeleton tool in the app (SK-10..19): presses and drags in a view
//! run `operandi.skel.*` lines; a press that drags is one undo step.
use super::*;
use crate::tests::box_app;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, egui::vec2(800., 600.))
}

fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.projector(rect()).project(p).unwrap().0
}

fn joints(a: &Operandi) -> Vec<u64> {
    a.editor
        .scene
        .items
        .iter()
        .filter(|i| i.is_joint())
        .map(|i| i.id)
        .collect()
}

fn world(a: &Operandi, id: u64) -> DVec3 {
    a.editor.scene.world_matrix(id).unwrap().w_axis.truncate()
}

fn click(a: &mut Operandi, at: DVec3, shift: bool) -> Option<input::GestureKind> {
    let at = screen(a, at);
    let m = egui::Modifiers {
        shift,
        ..Default::default()
    };
    let g = a.skeleton_press(at, rect(), m);
    if let Some(input::GestureKind::Skeleton(d)) = &g {
        let d = d.clone();
        a.skeleton_release(&d);
    }
    g
}

fn tool_app() -> Operandi {
    crate::anim_switch::set_for_test(true);
    let mut a = box_app();
    a.execute("operandi.skel.tool");
    assert!(a.skeleton_active());
    a
}

#[test]
fn sk10_sk11_clicks_chain_joints_through_the_middle_of_the_mesh() {
    let mut a = tool_app();
    // Top view over the 2 m box: halfway between its top and bottom.
    click(&mut a, DVec3::ZERO, false);
    let j = joints(&a);
    assert_eq!(j.len(), 1);
    assert!(
        world(&a, j[0]).distance(DVec3::ZERO) < 1e-6,
        "{}",
        world(&a, j[0])
    );
    click(&mut a, DVec3::new(0.5, 0., 0.), false);
    let j = joints(&a);
    assert_eq!(a.editor.scene.item(j[1]).unwrap().parent, Some(j[0]));
    // Shift starts a new chain.
    click(&mut a, DVec3::new(-0.5, 0., 0.5), true);
    let j = joints(&a);
    assert_eq!(a.editor.scene.item(j[2]).unwrap().parent, None);
    // Off the model: the work plane.
    click(&mut a, DVec3::new(3., 0., 0.), false);
    assert_eq!(joints(&a).len(), 4);
}

#[test]
fn sk19_a_dragged_click_is_one_undo_step() {
    let mut a = tool_app();
    let steps = a.editor.undo.len();
    let start = screen(&a, DVec3::ZERO);
    let Some(input::GestureKind::Skeleton(d)) = a.skeleton_press(start, rect(), Default::default())
    else {
        panic!("a joint press");
    };
    for k in 1..=5 {
        let to = screen(&a, DVec3::new(0.1 * k as f64, 0., 0.));
        a.skeleton_drag(&d, start, to, rect());
    }
    a.skeleton_release(&d);
    assert_eq!(a.editor.undo.len(), steps + 1);
    assert!((world(&a, d.joint).x - 0.5).abs() < 1e-6);
    a.run(Command::Undo);
    assert!(joints(&a).is_empty());
}

#[test]
fn sk10_clicking_a_joint_grows_a_branch_and_delete_mode_removes() {
    let mut a = tool_app();
    click(&mut a, DVec3::ZERO, false);
    click(&mut a, DVec3::new(0.5, 0., 0.), false);
    let root = joints(&a)[0];
    click(&mut a, DVec3::ZERO, false);
    assert_eq!(a.skeleton.current, Some(root));
    click(&mut a, DVec3::new(0., 0., 0.5), false);
    let j = joints(&a);
    assert_eq!(j.len(), 3);
    assert_eq!(a.editor.scene.item(j[2]).unwrap().parent, Some(root));
    a.skeleton.mode = skeleton_tool_ui::SkelMode::Delete;
    click(&mut a, DVec3::new(0.5, 0., 0.), false);
    assert_eq!(joints(&a).len(), 2);
}

#[test]
fn the_tool_window_draws() {
    let ctx = Context::default();
    let mut a = tool_app();
    click(&mut a, DVec3::ZERO, false);
    let _ = crate::tests::draw(&mut a, &ctx, egui::vec2(1400., 900.), vec![]);
    a.execute("operandi.skel.tool");
    assert!(!a.skeleton_active());
}
