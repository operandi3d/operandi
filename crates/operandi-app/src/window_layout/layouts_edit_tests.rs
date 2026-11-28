//! Layouts stage 2 (BEHAVIOR-LAYOUTS-STAGE2.md, layouts_edit.rs).
use super::*;
use crate::layouts_edit::Dialog;
use crate::layouts_tests::{SIZE, data_root, frames, launch};
use crate::panel_dot::{Dot, MenuAction};
use crate::tests::{draw, draw_held, key, pointer};
use operandi_layout::{EMPTY, Pane, WindowLayout};
use std::path::Path;

fn view_slots(a: &Operandi) -> Vec<Pane> {
    a.views
        .panes()
        .into_iter()
        .filter(|p| p.key().is_some())
        .collect()
}

fn app(root: &Path) -> (Operandi, Context) {
    let ctx = Context::default();
    let mut a = launch(root, &ctx);
    frames(&mut a, &ctx, 2);
    (a, ctx)
}

fn user_file(root: &Path, name: &str) -> PathBuf {
    root.join("Layouts").join(format!("{name}.json"))
}

fn state(root: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(root.join("layouts.json")).unwrap()).unwrap()
}

#[test]
fn lay43_new_layout_is_blank_or_a_copy_at_the_right_hand_end() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    // Layout > New opens the dialog with the copy box off (UIC-200).
    a.menu_action(&ctx, app_menu::line_action("operandi.layout.create"));
    assert_eq!(
        a.layouts.dialog,
        Some(Dialog::New {
            name: String::new(),
            copy: false
        })
    );
    a.layouts.dialog = Some(Dialog::New {
        name: "Sketch".into(),
        copy: false,
    });
    a.answer_layout_dialog();
    frames(&mut a, &ctx, 2);
    assert_eq!(a.layouts.current, "Sketch", "{}", a.status);
    assert_eq!(a.layouts.names(), ["Model", "UV", "Animate", "Sketch"]);
    // One empty panel to build from (LAY-74); the scene is untouched.
    let panes = view_slots(&a);
    assert!(matches!(panes.as_slice(), [Pane::Missing(_)]), "{panes:?}");
    assert_eq!(a.views.title_of(panes[0]), EMPTY);
    assert!(user_file(&root, "Sketch").is_file());
    // A copy of the current one: Model split in two.
    a.execute("layout.restore Model");
    a.split_view();
    frames(&mut a, &ctx, 1);
    a.execute("operandi.layout.create Twin copy:true");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.layouts.current, "Twin", "{}", a.status);
    assert_eq!(a.views.viewports().len(), 2);
    assert_eq!(a.layouts.names().last().map(String::as_str), Some("Twin"));
    // A name in use is refused.
    a.execute("operandi.layout.create model");
    assert_eq!(a.layouts.current, "Twin");
    assert!(a.status.contains("already"), "{}", a.status);
}

#[test]
fn lay44_save_as_saves_the_arrangement_under_a_new_name() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.split_view();
    frames(&mut a, &ctx, 1);
    a.menu_action(&ctx, app_menu::line_action("operandi.layout.saveAs"));
    a.layouts.dialog = Some(Dialog::SaveAs {
        name: "Two Views".into(),
    });
    a.answer_layout_dialog();
    frames(&mut a, &ctx, 1);
    assert_eq!(a.layouts.current, "Two Views");
    let saved =
        WindowLayout::from_json(&std::fs::read_to_string(user_file(&root, "Two Views")).unwrap())
            .unwrap();
    assert_eq!(
        saved
            .frame
            .slots()
            .iter()
            .filter(|s| s.kind == "3D View")
            .count(),
        2
    );
    // Save Layout writes now; a shipped one as shipped has nothing to save.
    a.execute("operandi.layout.save");
    assert!(a.status.contains("saved"), "{}", a.status);
}

#[test]
fn lay45_lay46_rename_and_delete_user_layouts_only() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.layout.create Mine copy:true");
    frames(&mut a, &ctx, 1);
    a.execute("operandi.layout.star");
    a.layouts.dialog = Some(Dialog::Rename {
        layout: "Mine".into(),
        name: "Ours".into(),
        opening: true,
    });
    a.answer_layout_dialog();
    frames(&mut a, &ctx, 1);
    assert_eq!(a.layouts.current, "Ours", "{}", a.status);
    assert!(!user_file(&root, "Mine").exists());
    assert!(user_file(&root, "Ours").is_file());
    assert!(a.layouts.stars.contains("Ours"), "the star follows");
    // Shipped layouts keep their names and can't be deleted.
    a.execute("operandi.layout.rename Mine layout:Model");
    assert_eq!(a.layouts.names(), ["Model", "UV", "Animate", "Ours"]);
    assert!(a.status.contains("shipped"), "{}", a.status);
    a.execute("operandi.layout.delete layout:UV");
    assert!(a.layouts.find("UV").is_some());
    assert!(a.status.contains("can't be deleted") || a.status.contains("not deleted"));
    // Delete asks first: the menu only opens the question.
    a.menu_action(
        &ctx,
        app_menu::line_action("operandi.layout.delete ask:true"),
    );
    assert_eq!(
        a.layouts.dialog,
        Some(Dialog::Delete {
            layout: "Ours".into()
        })
    );
    assert!(a.layouts.find("Ours").is_some(), "nothing yet");
    a.answer_layout_dialog();
    frames(&mut a, &ctx, 1);
    assert!(a.layouts.find("Ours").is_none());
    assert!(!user_file(&root, "Ours").exists());
    // The window went to the layout shown before it.
    assert_eq!(a.layouts.current, "Model");
}

#[test]
fn uic15_revert_asks_first_runs_a_command_and_drops_the_copy() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("layout.restore UV");
    frames(&mut a, &ctx, 1);
    a.split_view();
    frames(&mut a, &ctx, 1);
    assert!(user_file(&root, "UV").is_file());
    // Layout > Reset Layout and the tab's double-click only ask.
    a.menu_action(
        &ctx,
        app_menu::line_action("operandi.layout.revert ask:true"),
    );
    assert_eq!(a.layouts.confirm_reset.as_deref(), Some("UV"));
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.panes().len(), 3, "nothing reset without a yes");
    a.answer_layout_reset(false);
    assert_eq!(a.views.panes().len(), 3);
    a.layouts.confirm_reset = Some("UV".into());
    a.answer_layout_reset(true);
    frames(&mut a, &ctx, 1);
    assert_eq!(a.views.panes().len(), 2);
    assert!(!user_file(&root, "UV").exists());
    assert_eq!(a.status, "UV layout reset");
    // The command alone (scripts) reverts without asking; a layout you
    // made has nothing to go back to.
    a.execute("operandi.layout.create Mine copy:true");
    a.execute("operandi.layout.revert");
    assert!(a.status.contains("layout you made"), "{}", a.status);
}

#[test]
fn lay47_clear_empties_the_view_area() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.split_view();
    frames(&mut a, &ctx, 1);
    let scene = a.editor.scene.clone();
    a.menu_action(&ctx, app_menu::line_action("operandi.layout.clear"));
    frames(&mut a, &ctx, 2);
    assert!(matches!(view_slots(&a).as_slice(), [Pane::Missing(_)]));
    assert_eq!(a.editor.scene, scene);
    // Revert brings Model back.
    a.execute("operandi.layout.revert");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.views.viewports().len(), 1);
}

#[test]
fn uic13_reorder_by_command_persists_and_updates_the_layouts_pie() {
    let root = data_root();
    let (mut a, _) = app(&root);
    a.execute("operandi.layout.move UV 0");
    assert_eq!(a.layouts.names(), ["UV", "Model", "Animate"]);
    assert_eq!(
        a.layouts.current, "Model",
        "reordering keeps the current layout"
    );
    let pie = a.layouts_pie();
    assert_eq!(
        pie.entries[0].as_ref().unwrap().command,
        "layout.restore UV"
    );
    assert_eq!(
        pie.entries[1].as_ref().unwrap().command,
        "layout.restore Model"
    );
    let (b, _) = app(&root);
    assert_eq!(
        b.layouts.names(),
        ["UV", "Model", "Animate"],
        "the order survives a restart"
    );
}

#[test]
fn uic14_uic16_stars_starred_only_and_collapse() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.layout.create Third copy:true");
    a.execute("layout.restore Model");
    // Shipped layouts start starred; a new one doesn't.
    assert!(a.layouts.stars.contains("UV") && !a.layouts.stars.contains("Third"));
    a.execute("operandi.layout.star Model");
    assert!(!a.layouts.stars.contains("Model"), "a bare star toggles");
    a.execute("operandi.layout.starredOnly on:true");
    frames(&mut a, &ctx, 2);
    let shown = a.layouts.shown_names();
    assert_eq!(
        shown,
        ["Model", "UV", "Animate"],
        "starred, and the current one"
    );
    a.execute("operandi.layout.starredOnly");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.layouts.shown_names().len(), 4);
    // The top bar collapsed: no tool sets, a thin line.
    a.execute("operandi.bar.collapse");
    frames(&mut a, &ctx, 1);
    assert!(a.layouts.tool_set_rects.is_empty());
    a.execute("operandi.layout.starredOnly on");
    // A restart keeps the stars, the starred-only and collapsed states.
    let (b, _) = app(&root);
    assert!(b.layouts.stars.contains("UV"));
    assert!(!b.layouts.stars.contains("Model"));
    assert!(b.layouts.starred_only);
    assert!(b.layouts.top_collapsed);
    a.execute("operandi.bar.collapse on:false");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.layouts.shown_names().len(), 3);
    assert!(!a.layouts.tool_set_rects.is_empty());
}

#[test]
fn d08_tool_set_buttons_change_tools_and_keep_the_layout() {
    use crate::toolbox::{Category, ToolSet};

    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.layout.create Third copy:true");
    a.split_view();
    frames(&mut a, &ctx, 3);
    let before = a.capture_layout().unwrap();
    let scene = a.editor.scene.clone();
    let active_view = a.active_view;
    let viewports = a.views.viewports();
    let tree = a.views.main.clone();
    let drawn = a.layouts.tool_set_rects.clone();
    assert_eq!(
        drawn.iter().map(|(set, _)| *set).collect::<Vec<_>>(),
        ToolSet::ALL
    );
    let logo = a.layouts.logo_rect.unwrap();
    for (set, rect) in drawn {
        assert!(rect.left() >= logo.right() && rect.right() < SIZE.x);
        assert_eq!(rect.center().y, logo.center().y);
        click_at(&mut a, &ctx, rect.center());
        frames(&mut a, &ctx, 2);
        assert_eq!(a.layouts.tool_set, set);
        let (category, tool) = match set {
            ToolSet::Model => (Some(Category::Create), Some("Cube")),
            ToolSet::Uv => (Some(Category::Uv), Some("Unwrap Tool")),
            ToolSet::Brush => (Some(Category::Sculpt), Some("Vertex Map Erase")),
            ToolSet::Setup => (Some(Category::Setup), Some("operandi.rig.parent")),
            ToolSet::Animation => (Some(Category::Animation), Some("Key Transforms")),
            ToolSet::Workbench => (Some(Category::Workbench), None),
            ToolSet::Game => (Some(Category::GameVertexMap), Some("Convert to Hard Edges")),
        };
        assert_eq!(a.shown_category(), category, "{set:?}");
        if let Some(tool) = tool {
            assert!(a.form_rects.contains_key(tool), "missing {tool}");
        }
        let after = a.capture_layout().unwrap();
        assert_eq!(after.name, before.name);
        // Jason, 2026-11-23: a set button never changes the layout; no
        // view opens, closes or moves.
        assert_eq!(after.frame, before.frame, "{set:?}");
        assert_eq!(a.views.viewports(), viewports, "{set:?}");
        assert_eq!(a.views.main, tree, "{set:?}");
        assert_eq!(after.windows, before.windows);
        assert_eq!(a.active_view, active_view);
        assert_eq!(a.editor.scene, scene);
    }
    a.execute("operandi.bar.collapse bar:top on:true");
    frames(&mut a, &ctx, 1);
    assert!(a.layouts.tool_set_rects.is_empty());
}

#[test]
fn uic18_quick_switcher_lists_by_recent_use_and_switches_on_release() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.layout.create Third copy:true");
    a.execute("layout.restore UV");
    a.execute("layout.restore Model");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.layouts.mru_order(), ["Model", "UV", "Third", "Animate"]);
    // No default key since 2026-11-25 (Ctrl+Shift+Tab pages the tool
    // tabs); a user who gives it Ctrl+Shift+Tab back gets the held switcher.
    assert_eq!(a.bindings["Layout Switcher"], "");
    a.bindings
        .insert("Layout Switcher".into(), "Ctrl+Shift+Tab".into());
    a.bindings
        .insert(crate::tool_tab_cycle::PREVIOUS.into(), String::new());
    let held = Modifiers {
        ctrl: true,
        shift: true,
        ..Modifiers::NONE
    };
    let tab = Event::Key {
        key: Key::Tab,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: held,
    };
    draw_held(&mut a, &ctx, SIZE, vec![tab.clone()], held);
    let sw = a.layouts.switcher.clone().expect("the switcher opened");
    assert_eq!(sw.list, ["Model", "UV", "Third", "Animate"]);
    assert_eq!(sw.at, 1, "the next one outlined");
    // A further Tab, keys still held, moves the outline on.
    draw_held(&mut a, &ctx, SIZE, vec![tab], held);
    assert_eq!(a.layouts.switcher.as_ref().unwrap().at, 2);
    assert_eq!(a.layouts.current, "Model");
    // Releasing the keys switches.
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.layouts.switcher.is_none());
    assert_eq!(a.layouts.current, "Third", "{}", a.status);
    // From the menu it waits for a pick; Esc closes it.
    a.menu_action(&ctx, app_menu::line_action("operandi.layout.switcher"));
    assert!(a.layouts.switcher.as_ref().is_some_and(|s| !s.held));
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.layouts.switcher.is_some(), "no keys held: it stays");
    draw(&mut a, &ctx, SIZE, vec![key(Key::Escape)]);
    assert!(a.layouts.switcher.is_none());
    assert_eq!(a.layouts.current, "Third");
    // The MRU list survives a restart.
    let (b, _) = app(&root);
    assert_eq!(b.layouts.mru_order(), ["Third", "Model", "UV", "Animate"]);
}

#[test]
fn uic18_the_layouts_pie_runs_layout_restore() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    assert_eq!(a.bindings[crate::pie_menu::LAYOUTS_LINE], "Ctrl+`");
    let pie = a.layouts_pie();
    let commands: Vec<&str> = pie
        .entries
        .iter()
        .flatten()
        .map(|e| e.command.as_str())
        .collect();
    assert_eq!(
        commands,
        [
            "layout.restore Model",
            "layout.restore UV",
            "layout.restore Animate"
        ]
    );
    // Ctrl+` opens it (held), the same as the other pies.
    let held = Modifiers {
        ctrl: true,
        ..Modifiers::NONE
    };
    let backtick = Event::Key {
        key: Key::Backtick,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: held,
    };
    draw_held(&mut a, &ctx, SIZE, vec![backtick], held);
    assert_eq!(
        a.pie.as_ref().map(|p| p.pie.name.as_str()),
        Some(crate::pie_menu::LAYOUTS)
    );
    a.pie = None;
    a.execute(&pie.entries[1].as_ref().unwrap().command);
    assert_eq!(a.layouts.current, "UV");
}

#[test]
fn lay73_the_content_arrow_swaps_what_a_slot_shows() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    let view = a.active_view;
    a.execute("operandi.panel.kind {UV View}");
    frames(&mut a, &ctx, 2);
    let panes = view_slots(&a);
    assert!(matches!(panes.as_slice(), [Pane::Uv(_)]), "{panes:?}");
    assert_ne!(panes[0].view_key(), Some(view), "a fresh key");
    assert_eq!(
        Some(a.active_view),
        panes[0].view_key(),
        "still the active slot"
    );
    // Empty, then back to a 3D view.
    a.execute("operandi.panel.kind Empty");
    assert!(matches!(view_slots(&a).as_slice(), [Pane::Missing(_)]));
    a.execute("operandi.panel.kind {3D View}");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.viewports().len(), 1);
    // A side-column kind is refused with the reason.
    a.execute("operandi.panel.kind Items");
    assert!(a.status.contains("side columns"), "{}", a.status);
    assert_eq!(a.views.viewports().len(), 1);
    // The header draws the arrow (a press on it stays the header's).
    frames(&mut a, &ctx, 1);
    let key = a.active_view;
    let hits = &a.corner.header_hits[&key];
    let area = a.corner.areas[&key];
    assert!(hits.iter().any(|r| (area.right() - r.right()) < 8.));
}

#[test]
fn lay74_a_layout_built_from_one_empty_panel_saves_and_reopens() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.layout.create Built");
    frames(&mut a, &ctx, 2);
    let empty = view_slots(&a)[0];
    let key = empty.key().unwrap();
    // The corner dot splits it; the content arrow fills each slot.
    a.dot_menu_action(Dot::View(key), MenuAction::SplitHorizontal);
    frames(&mut a, &ctx, 2);
    assert_eq!(view_slots(&a).len(), 2, "{}", a.status);
    assert!(view_slots(&a).iter().all(|p| matches!(p, Pane::Missing(_))));
    a.execute(&format!("operandi.panel.kind {{3D View}} panel:{key}"));
    let other = a
        .views
        .panes()
        .into_iter()
        .find_map(|p| match p {
            Pane::Missing(k) => Some(k),
            _ => None,
        })
        .unwrap();
    a.execute(&format!("operandi.panel.kind {{UV View}} panel:{other}"));
    frames(&mut a, &ctx, 3);
    let mut kinds: Vec<&str> = view_slots(&a)
        .iter()
        .map(|p| match p {
            Pane::Viewport(_) => "3D",
            Pane::Uv(_) => "UV",
            _ => "other",
        })
        .collect();
    kinds.sort();
    assert_eq!(kinds, ["3D", "UV"]);
    let rects: Vec<Rect> = a.views.views().iter().map(|v| a.view_rects[v]).collect();
    assert!(
        (rects[0].center().y - rects[1].center().y).abs() < 1.,
        "side by side"
    );
    // Restart: it opens as built.
    let ctx2 = Context::default();
    let mut b = launch(&root, &ctx2);
    frames(&mut b, &ctx2, 2);
    assert_eq!(b.layouts.current, "Built");
    assert_eq!(b.views.viewports().len(), 1);
    assert_eq!(view_slots(&b).len(), 2);
    assert!(b.views.panes().iter().any(|p| matches!(p, Pane::Uv(_))));
}

#[test]
fn lay72_pieces_save_reuse_and_update_every_layout_using_them() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    // A wireframe view saved as a piece.
    a.display.settings.active.style = view_display::Style::Wireframe;
    frames(&mut a, &ctx, 1);
    a.layouts.dialog = Some(Dialog::SavePiece {
        name: "Wire".into(),
        category: "Views".into(),
        group: false,
    });
    a.answer_layout_dialog();
    assert!(root.join("Pieces/Wire.json").is_file(), "{}", a.status);
    // Reused in another layout's empty panel.
    a.execute("operandi.layout.create Other");
    frames(&mut a, &ctx, 1);
    a.execute("operandi.viewport.use Wire");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.viewports().len(), 1);
    assert_eq!(
        a.display.settings.active.style,
        view_display::Style::Wireframe
    );
    // Back in Model, the piece is saved again as Shaded: Other updates.
    a.execute("layout.restore Model");
    frames(&mut a, &ctx, 1);
    a.display.settings.active.style = view_display::Style::Shaded;
    a.execute("operandi.viewport.save");
    let other =
        WindowLayout::from_json(&std::fs::read_to_string(user_file(&root, "Other")).unwrap())
            .unwrap();
    let slot = other
        .frame
        .slots()
        .into_iter()
        .find(|s| s.piece.as_deref() == Some("Wire"))
        .cloned()
        .expect("Other's slot keeps its piece");
    let style = slot.settings["display"]["active"]["style"].clone();
    assert_ne!(style, serde_json::json!("Wireframe"), "{slot:?}");
    a.execute("layout.restore Other");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.display.settings.active.style, view_display::Style::Shaded);
    // A whole view area as a group piece, placed in an empty panel.
    a.execute("layout.restore Model");
    a.split_view();
    frames(&mut a, &ctx, 1);
    a.execute("operandi.viewport.saveAs Pair group:true");
    a.execute("operandi.layout.create Grouped");
    frames(&mut a, &ctx, 1);
    a.execute("operandi.viewport.use Pair");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.viewports().len(), 2, "{}", a.status);
    assert_eq!(a.layouts.group_piece.as_deref(), Some("Pair"));
    // Delete Saved Viewport: the file goes; panels keep what they show.
    a.execute("operandi.viewport.delete Pair");
    assert!(!root.join("Pieces/Pair.json").exists());
    assert_eq!(a.views.viewports().len(), 2);
    // Pieces survive a restart.
    let (b, _) = app(&root);
    let names: Vec<&str> = b.layouts.pieces.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Wire"]);
}

#[test]
fn lay48_lay60_locks_are_per_layout_and_everything_survives_a_restart() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    // Out of the Layout menu since panels move only in UI build mode; the
    // command stays for scripts.
    a.execute("layout.lockThumbs");
    assert!(a.dots.locks.thumbs);
    a.execute("layout.restore UV");
    assert!(!a.dots.locks.thumbs, "UV has its own");
    a.execute("layout.lockDividers lock:on");
    a.execute("layout.restore Model");
    assert!(a.dots.locks.thumbs && !a.dots.locks.dividers);
    a.execute("operandi.layout.star Model on:false");
    a.execute("operandi.layout.move Model 1");
    frames(&mut a, &ctx, 1);
    let s = state(&root);
    assert_eq!(s["locks"]["Model"]["thumbs"], true);
    assert_eq!(s["locks"]["UV"]["dividers"], true);
    // Not in scene files: a restart reads it all back.
    let (b, _) = app(&root);
    assert_eq!(b.layouts.current, "Model");
    assert!(b.dots.locks.thumbs && !b.dots.locks.dividers);
    assert_eq!(b.layouts.names(), ["UV", "Model", "Animate"]);
    assert!(!b.layouts.stars.contains("Model") && b.layouts.stars.contains("UV"));
    assert!(b.layouts.locks["UV"].dividers);
}

#[test]
fn every_stage2_command_line_runs_without_a_window() {
    // UIC-220/221: the bar, menus, keys and pie only run these lines.
    let root = data_root();
    let (mut a, _ctx) = app(&root);
    for line in [
        "operandi.layout.create A",
        "operandi.layout.saveAs B",
        "operandi.layout.rename C layout:B",
        "operandi.layout.star C on:true",
        "operandi.layout.starredOnly on:false",
        "operandi.bar.collapse bar:left on:false width:260",
        "operandi.bar.collapse bar:top on:false",
        "operandi.layout.move C 0",
        "operandi.layout.clear",
        "operandi.layout.save",
        "operandi.layout.delete layout:C",
        "operandi.layout.revert layout:Model",
        "layout.lockThumbs lock:false",
        "layout.lockDividers lock:false",
        "operandi.viewport.saveAs P",
        "operandi.viewport.save",
        "operandi.viewport.revert",
        "operandi.viewport.use P",
        "operandi.viewport.delete P",
        "operandi.panel.kind {3D View}",
    ] {
        a.status.clear();
        a.execute(line);
        assert!(
            !a.status.starts_with("Unknown") && !a.status.contains("isn't a layout command"),
            "{line}: {}",
            a.status
        );
    }
    assert_eq!(a.layouts.names(), ["Model", "UV", "Animate", "A"]);
}

/// Press and release at `at`, hovering first, as a hand would.
fn click_at(a: &mut Operandi, ctx: &Context, at: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(a, ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![pointer(at, false, Modifiers::NONE)]);
}

/// Two clicks at `at`, close enough together to be a double-click.
fn double_click_at(a: &mut Operandi, ctx: &Context, at: Pos2) {
    click_at(a, ctx, at);
    draw(a, ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![pointer(at, false, Modifiers::NONE)]);
    frames(a, ctx, 1);
}

/// Press at `from`, move to `to` in steps, let go there.
fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![pointer(from, true, Modifiers::NONE)]);
    for i in 1..=5 {
        let p = from + (to - from) * (i as f32 / 5.);
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(a, ctx, SIZE, vec![pointer(to, false, Modifiers::NONE)]);
    frames(a, ctx, 1);
}

/// UIC-16, Jason 2026-11-18: the logo is the top bar's collapse control.
/// A click leaves a thin line across the window; a single click on the
/// line does nothing, a double-click or a drag down brings the bar back;
/// the state survives a restart; there is no other collapse button.
#[test]
fn logo_click_collapses_the_top_bar_and_the_line_brings_it_back() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    frames(&mut a, &ctx, 1);
    assert!(!a.layouts.top_collapsed);
    assert!(a.layouts.strip_rect.is_none());
    let logo = a.layouts.logo_rect.expect("the logo is drawn");
    assert!(
        logo.left() < 40.,
        "the logo sits at the bar's left end: {logo:?}"
    );
    let tabs = a.layouts.tool_set_rects.len();
    assert!(tabs > 0);
    // A double-click on the logo collapses it once: the second click lands
    // on the new line, which ignores it.
    double_click_at(&mut a, &ctx, logo.center());
    assert!(a.layouts.top_collapsed, "{}", a.status);
    assert!(a.layouts.logo_rect.is_none() && a.layouts.tool_set_rects.is_empty());
    let strip = a.layouts.strip_rect.expect("the collapsed line is drawn");
    assert!(
        strip.width() > SIZE.x * 0.9,
        "the line runs across the window: {strip:?}"
    );
    assert!(
        strip.height() <= crate::collapse_line::TOP_AREA + 0.5,
        "and stays thin: {strip:?}"
    );
    // Collapsed is remembered between launches.
    let (b, _) = app(&root);
    assert!(b.layouts.top_collapsed);
    // Keys and menus don't live in this row: commands still run.
    a.execute("operandi.layout.starredOnly on:true");
    assert!(a.layouts.starred_only);
    a.execute("operandi.layout.starredOnly on:false");
    // A single click on the line does nothing.
    click_at(&mut a, &ctx, strip.center());
    frames(&mut a, &ctx, 20);
    assert!(
        a.layouts.top_collapsed,
        "a single click leaves it collapsed"
    );
    // A double-click brings it back.
    double_click_at(&mut a, &ctx, strip.center());
    assert!(!a.layouts.top_collapsed, "{}", a.status);
    assert!(a.layouts.strip_rect.is_none());
    assert_eq!(a.layouts.tool_set_rects.len(), tabs);
    let (b, _) = app(&root);
    assert!(!b.layouts.top_collapsed);
    // So does a drag down past the line.
    a.execute("operandi.bar.collapse bar:top on:true");
    frames(&mut a, &ctx, 40);
    let strip = a.layouts.strip_rect.unwrap();
    drag(&mut a, &ctx, strip.center(), strip.center() + vec2(0., 2.));
    assert!(
        a.layouts.top_collapsed,
        "a drag that stays on the line doesn't"
    );
    drag(&mut a, &ctx, strip.center(), strip.center() + vec2(0., 40.));
    assert!(!a.layouts.top_collapsed, "{}", a.status);
}

/// Jason 2026-11-18: the layout tabs' own "‹" hide button is gone; a saved
/// state that had them hidden shows them.
#[test]
fn an_old_hidden_tabs_state_shows_the_tabs() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.layout.starredOnly on:true");
    let mut saved = state(&root);
    saved["collapsed"] = serde_json::Value::Bool(true);
    std::fs::write(root.join("layouts.json"), saved.to_string()).unwrap();
    let (mut b, ctx_b) = app(&root);
    assert!(b.layouts.starred_only, "the rest of the file still reads");
    assert!(!b.layouts.top_collapsed);
    frames(&mut b, &ctx_b, 1);
    assert!(!b.layouts.tool_set_rects.is_empty(), "tool sets shown");
    // The old command is gone: it changes nothing.
    a.execute("operandi.layout.collapse");
    frames(&mut a, &ctx, 1);
    assert!(!a.layouts.top_collapsed && !a.layouts.tool_set_rects.is_empty());
}

fn panel_rect(ctx: &Context, id: &str) -> Rect {
    egui::containers::panel::PanelState::load(ctx, Id::new(id))
        .unwrap()
        .rect
}

/// Jason 2026-11-18: a double-click on a side bar's resize line collapses
/// it to a line at the window edge; the line's double-click reopens it at
/// its old width, a drag out reopens it where the drag lets go, and a
/// drag that ends under the bar's minimum takes the old width.
#[test]
fn side_bar_double_click_collapses_and_its_line_reopens_it() {
    let root = data_root();
    crate::layouts_tests::install_legacy_model(&root);
    let (mut a, ctx) = app(&root);
    frames(&mut a, &ctx, 2);
    let tools = panel_rect(&ctx, "tools");
    let old = tools.width();
    let edge = pos2(tools.right(), tools.center().y);
    let viewport_left = a.viewport_rect.left();
    double_click_at(&mut a, &ctx, edge);
    assert!(a.layouts.left_collapsed, "{}", a.status);
    assert!(!a.layouts.right_collapsed);
    frames(&mut a, &ctx, 2);
    assert!(
        a.viewport_rect.left() < viewport_left - 100.,
        "the view takes the room: {:?}",
        a.viewport_rect
    );
    let (b, _) = app(&root);
    assert!(b.layouts.left_collapsed, "kept between launches");
    // The line sits at the window's left edge.
    let line = pos2(3., tools.center().y);
    frames(&mut a, &ctx, 40);
    click_at(&mut a, &ctx, line);
    frames(&mut a, &ctx, 40);
    assert!(a.layouts.left_collapsed, "a single click does nothing");
    double_click_at(&mut a, &ctx, line);
    assert!(!a.layouts.left_collapsed, "{}", a.status);
    frames(&mut a, &ctx, 1);
    assert!((panel_rect(&ctx, "tools").width() - old).abs() < 1.);
    // Collapse again (by command), then drag the line out to 300.
    a.execute("operandi.bar.collapse bar:left on:true");
    frames(&mut a, &ctx, 40);
    drag(&mut a, &ctx, line, pos2(300., line.y));
    assert!(!a.layouts.left_collapsed, "{}", a.status);
    frames(&mut a, &ctx, 1);
    let w = panel_rect(&ctx, "tools").width();
    assert!((w - 300.).abs() < 2., "reopens where the drag let go: {w}");
    // A short drag (under the minimum) snaps back to the width it had.
    a.execute("operandi.bar.collapse bar:left on:true");
    frames(&mut a, &ctx, 40);
    drag(&mut a, &ctx, line, pos2(60., line.y));
    assert!(!a.layouts.left_collapsed);
    frames(&mut a, &ctx, 1);
    let w2 = panel_rect(&ctx, "tools").width();
    assert!((w2 - w).abs() < 2., "the old width: {w2}");
    // The right bar works the same way.
    let scene = panel_rect(&ctx, "scene");
    double_click_at(&mut a, &ctx, pos2(scene.left(), scene.center().y));
    assert!(a.layouts.right_collapsed, "{}", a.status);
    frames(&mut a, &ctx, 40);
    double_click_at(&mut a, &ctx, pos2(SIZE.x - 3., scene.center().y));
    assert!(!a.layouts.right_collapsed, "{}", a.status);
    frames(&mut a, &ctx, 1);
    assert!((panel_rect(&ctx, "scene").width() - scene.width()).abs() < 1.);
}

/// Jason 2026-11-20: a collapsed side bar shows a tab sticking out of its
/// line; one click on it reopens the panel at its old width.
#[test]
fn a_collapsed_side_bars_tab_reopens_it_with_one_click() {
    use crate::collapse_line::{Bar, SIDE_AREA, tab_rect};
    let root = data_root();
    crate::layouts_tests::install_legacy_model(&root);
    let (mut a, ctx) = app(&root);
    frames(&mut a, &ctx, 2);
    for (bar, id, side) in [(Bar::Left, "tools", "left"), (Bar::Right, "scene", "right")] {
        let old = panel_rect(&ctx, id).width();
        a.execute(&format!("operandi.bar.collapse bar:{side} on:true"));
        frames(&mut a, &ctx, 40);
        let content = ctx.content_rect();
        let area = match bar {
            Bar::Left => Rect::from_min_max(
                pos2(content.left(), a.viewport_rect.top()),
                pos2(content.left() + SIDE_AREA, a.viewport_rect.bottom()),
            ),
            _ => Rect::from_min_max(
                pos2(content.right() - SIDE_AREA, a.viewport_rect.top()),
                pos2(content.right(), a.viewport_rect.bottom()),
            ),
        };
        let tab = tab_rect(bar, panel_rect(&ctx, &format!("{id}-collapsed")));
        assert!(tab.width() > 10. && tab.height() > 40.);
        // It sticks out of the line, into the window.
        match bar {
            Bar::Left => assert!(tab.left() >= area.right() - 0.5),
            _ => assert!(tab.right() <= area.left() + 0.5),
        }
        click_at(&mut a, &ctx, tab.center());
        assert!(!a.layouts.is_collapsed(bar), "{side}: {}", a.status);
        frames(&mut a, &ctx, 1);
        assert!((panel_rect(&ctx, id).width() - old).abs() < 1.);
    }
}
