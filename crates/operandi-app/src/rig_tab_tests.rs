//! The Rig tab (UIC-190..201): every button names a command, the tab shows
//! in the Setup and Game tool sets, and it draws.
use super::*;

fn lines(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(m) => {
            for key in ["command", "line"] {
                if let Some(serde_json::Value::String(s)) = m.get(key) {
                    out.push(s.clone());
                }
            }
            for v in m.values() {
                lines(v, out);
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|v| lines(v, out)),
        _ => {}
    }
}

#[test]
fn uic221_every_rig_tab_button_names_a_command() {
    crate::anim_switch::set_for_test(true);
    let a = crate::tests::box_app();
    let form: serde_json::Value =
        serde_json::from_str(include_str!("../assets/forms/toolbox.rig.json")).unwrap();
    let mut all = Vec::new();
    lines(&form, &mut all);
    assert!(all.len() > 30, "{}", all.len());
    for l in all {
        assert!(
            a.line_state(&l).is_some(),
            "the Rig tab's {l:?} names no command"
        );
    }
}

#[test]
fn the_rig_tab_shows_in_the_setup_and_game_sets_and_draws() {
    use toolbox::{Category, ToolSet};
    crate::anim_switch::set_for_test(true);
    let mut a = crate::tests::box_app();
    assert!(
        !ToolSet::Model.tabs().contains(&Category::Rig),
        "not in Model"
    );
    assert_eq!(ToolSet::Setup.tabs().last(), Some(&Category::Rig));
    assert_eq!(ToolSet::Game.tabs().last(), Some(&Category::Rig));
    a.toolbox_tab_line("rig");
    assert_eq!(a.category, Category::Rig);
    assert_eq!(a.layouts.tool_set, ToolSet::Setup, "the first set with Rig");
    let ctx = Context::default();
    let _ = crate::tests::draw(&mut a, &ctx, egui::vec2(1400., 900.), vec![]);
    assert!(a.form_rects.contains_key("operandi.skel.tool"));
    crate::anim_switch::set_for_test(false);
    assert!(
        !ToolSet::Setup.tabs().contains(&Category::Rig),
        "hidden with the animation switch off"
    );
    crate::anim_switch::set_for_test(true);
}
