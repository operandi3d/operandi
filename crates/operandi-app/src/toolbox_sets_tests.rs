//! The seven toolbox sets (Jason, 2026-11-23): their tabs, the empty
//! Animation set, one-form sets without a strip, the old Paint name, and
//! tabs that open their set.
use super::tests::{box_app, draw};
use super::*;
use crate::toolbox::{Category, NO_TOOLS_YET, ToolSet};

const SIZE: Vec2 = vec2(1440., 900.);

fn shows(a: &mut Operandi, ctx: &Context) -> egui::FullOutput {
    draw(a, ctx, SIZE, vec![]);
    draw(a, ctx, SIZE, vec![])
}

fn text_drawn(out: &egui::FullOutput, wanted: &str) -> bool {
    out.shapes.iter().any(|s| match &s.shape {
        egui::Shape::Text(t) => t.galley.text() == wanted,
        _ => false,
    })
}

#[test]
fn the_bar_lists_seven_sets_in_order_and_paint_reads_as_brush() {
    let labels: Vec<_> = ToolSet::ALL.iter().map(|s| s.label()).collect();
    assert_eq!(
        labels,
        [
            "Model",
            "UV",
            "Brush",
            "Setup",
            "Animation",
            "Workbench",
            "Game"
        ]
    );
    assert_eq!(ToolSet::named("paint"), Some(ToolSet::Brush));
    let mut a = box_app();
    a.execute("operandi.toolbox.set paint");
    assert_eq!(a.layouts.tool_set, ToolSet::Brush, "{}", a.status);
    assert_eq!(a.category, Category::Sculpt, "Brush opens at Sculpt");
}

#[test]
fn every_tab_of_every_set_draws_its_form() {
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    for &set in ToolSet::ALL {
        a.execute(&format!("operandi.toolbox.set {}", set.label()));
        assert_eq!(a.layouts.tool_set, set, "{}", a.status);
        let tabs = set.tabs();
        for &tab in &tabs {
            a.category = tab;
            shows(&mut a, &ctx);
            assert_eq!(a.layouts.tool_set, set, "{tab:?} stays in {set:?}");
            assert_eq!(a.shown_category(), Some(tab));
            // A strip only for sets with more than one tab.
            let strip = a.toolbox_rects.keys().any(|k| k.starts_with("Category "));
            assert_eq!(strip, tabs.len() > 1, "{set:?} {tab:?}");
            if tab != Category::Workbench {
                assert!(!a.form_rects.is_empty(), "{set:?} {tab:?} drew no buttons");
            }
        }
    }
}

#[test]
fn the_new_tabs_hold_their_planned_buttons() {
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    for (tab, lines) in [
        (
            Category::Setup,
            &[
                "Move",
                "item.create camera",
                "Duplicate Item",
                "Bezier Curve",
                "operandi.rig.parent",
                "Parent to Group Locator",
                "Quick Align",
                "Mirror",
                "Ruler",
            ][..],
        ),
        (
            Category::Constraints,
            &[
                "operandi.rig.constrain kind:position",
                "operandi.rig.dynamicParentAdd",
            ][..],
        ),
        (
            Category::InverseKinematics,
            &["operandi.ik.applyTwoBone", "operandi.ik.switch"][..],
        ),
        (
            Category::Weighting,
            &[
                "operandi.skin.bindDialog",
                "Vertex Map Paint",
                "operandi.weight.normalize",
            ][..],
        ),
        (
            Category::GameVertexMap,
            &[
                "scene.saveAs format:fbx export:true",
                "Vertex Map Paint",
                "Select Hard Edges",
                "Convert to Hard Edges",
                "Transfer Normals",
                "Morph to Normals",
            ][..],
        ),
        (Category::Baking, &["operandi.bake.curvature"][..]),
    ] {
        assert!(a.show_toolbox_tab(tab), "{tab:?}");
        shows(&mut a, &ctx);
        for line in lines {
            assert!(a.form_rects.contains_key(*line), "{tab:?}: {line}");
        }
    }
}

#[test]
fn the_animation_set_has_no_tabs_and_says_so() {
    let ctx = Context::default();
    let mut a = box_app();
    a.execute("operandi.toolbox.set animation");
    let out = shows(&mut a, &ctx);
    assert_eq!(a.shown_category(), None);
    assert!(text_drawn(&out, NO_TOOLS_YET));
    assert!(a.form_rects.is_empty(), "no form buttons");
    // The transform row at the top stays.
    assert!(a.toolbox_rects.contains_key("Move"));
    assert!(!a.toolbox_rects.keys().any(|k| k.starts_with("Category ")));
}

#[test]
fn toolbox_tab_opens_the_first_set_that_has_it() {
    crate::anim_switch::set_for_test(true);
    let mut a = box_app();
    for (tab, set) in [
        ("uv", ToolSet::Uv),
        ("sculpt", ToolSet::Brush),
        ("weighting", ToolSet::Setup),
        ("rig", ToolSet::Setup),
        ("baking", ToolSet::Game),
        ("workbench", ToolSet::Workbench),
        ("polygon", ToolSet::Model),
    ] {
        a.execute(&format!("operandi.toolbox.tab {tab}"));
        assert_eq!(a.layouts.tool_set, set, "{tab}: {}", a.status);
        assert_eq!(a.category.label().to_lowercase(), tab);
    }
    // Rig is already in Game: from there it stays.
    a.execute("operandi.toolbox.set game");
    a.execute("operandi.toolbox.tab rig");
    assert_eq!(a.layouts.tool_set, ToolSet::Game);
}

#[test]
fn a_saved_tab_outside_its_set_opens_the_sets_first_tab() {
    let root = crate::layouts_tests::data_root();
    let ctx = Context::default();
    let mut a = crate::layouts_tests::launch(&root, &ctx);
    let mut doc = a.capture_layout().unwrap();
    doc.toolbox = Some(operandi_layout::ToolboxState {
        set: "Game".into(),
        tab: "Polygon".into(),
        uv_tab: false,
    });
    a.apply_layout_doc(&doc).unwrap();
    assert_eq!(a.layouts.tool_set, ToolSet::Game);
    assert_eq!(a.category, Category::GameVertexMap);
}
