//! Rig phase 4 in the app: the Animate > Constrain / Hierarchy menus and
//! the Channel Links window run `operandi.rig.*` lines; driven channels
//! show the driven light; Compensation is kept in the preferences.
use super::*;
use crate::tests::{box_app, draw};

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

fn frames(a: &mut Operandi, ctx: &Context, n: usize) {
    for _ in 0..n {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
}

#[test]
fn the_animate_menu_has_the_rig_entries() {
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let a = box_app();
    let menus = a.menu_model(&ctx);
    let animate = menus
        .iter()
        .find(|m| m.title == "Animate")
        .expect("Animate menu");
    let titles: Vec<String> = animate
        .entries
        .iter()
        .filter_map(|e| match e {
            crate::app_menu::Entry::Submenu(m) => Some(m.title.clone()),
            crate::app_menu::Entry::Item(i) => Some(i.title.clone()),
            _ => None,
        })
        .collect();
    for want in ["Constrain", "Hierarchy", "Channel Links"] {
        assert!(titles.iter().any(|t| t == want), "{titles:?}");
    }
}

#[test]
fn channel_links_window_links_the_two_selected_items() {
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.new_mesh().unwrap();
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    let (driven, driver) = (ids[0], ids[1]);
    a.editor.scene.selection.also = vec![driven];
    a.editor.scene.selection.item = Some(driver);
    a.execute("operandi.rig.links");
    assert!(a.links.open);
    frames(&mut a, &ctx, 2);
    assert!(a.palettes.places.contains_key("channelLinks"));
    // The window's Add Link runs this line.
    a.execute(&format!(
        "operandi.rig.linkAdd driven:{driven}:scl.Z driver:{driver}:pos.X mode:direct"
    ));
    assert!(operandi_core::rig::driven_by(&a.editor.scene, driven, "scl.Z").is_some());
    frames(&mut a, &ctx, 1);
}

#[test]
fn compensation_is_kept_in_the_preferences_and_constraints_drive_fields() {
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    a.execute("operandi.rig.compensation on:true");
    frames(&mut a, &ctx, 1);
    assert!(a.undo_autosave.rig_compensation);
    a.editor.scene.new_mesh().unwrap();
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    a.execute(&format!(
        "operandi.rig.constrain kind:position items:{} targets:{}",
        ids[0], ids[1]
    ));
    assert!(operandi_core::rig::driven_by(&a.editor.scene, ids[0], "pos.X").is_some());
    frames(&mut a, &ctx, 2);
}
