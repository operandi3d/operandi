//! Layouts stage 5: per-layout key maps (BEHAVIOR-LAYOUTS-STAGE5.md,
//! layout_keys.rs). LAY-20, LAY-71, UIC-201.
use super::*;
use crate::input_map_editor::Scope;
use crate::layout_keys::{Mark, hidden, layered, mark};
use crate::layouts::SHIPPED;
use crate::layouts_tests::{SIZE, data_root, frames, launch};
use crate::tests::{draw, draw_held, key};
use operandi_layout::WindowLayout;
use std::path::Path;

fn app(root: &Path) -> (Operandi, Context) {
    let ctx = Context::default();
    let mut a = launch(root, &ctx);
    frames(&mut a, &ctx, 2);
    (a, ctx)
}

/// Press and release `k` (a key left down makes egui call the next press
/// a repeat, which shortcuts ignore).
fn press(a: &mut Operandi, ctx: &Context, k: Key) {
    draw(a, ctx, SIZE, vec![key(k)]);
    let up = Event::Key {
        key: k,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    draw(a, ctx, SIZE, vec![up]);
    a.persist_layout(ctx);
}

fn user_file(root: &Path, name: &str) -> PathBuf {
    root.join("Layouts").join(format!("{name}.json"))
}

fn keys(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(a, c)| ((*a).to_owned(), (*c).to_owned()))
        .collect()
}

#[test]
fn lay20_keys_a_layout_does_not_set_fall_back_to_global() {
    let global = input::default_bindings();
    let layout = keys(&[("Move", "Ctrl+M")]);
    let map = layered(&global, &layout);
    assert_eq!(map["Move"], "Ctrl+M");
    // Everything else is the global map.
    for (action, chord) in &global {
        if action != "Move" {
            assert_eq!(&map[action], chord, "{action}");
        }
    }
    // No layout keys: the global map exactly.
    assert_eq!(layered(&global, &BTreeMap::new()), global);
    // An unknown action or a chord that doesn't read does nothing.
    let junk = keys(&[("No Such Action", "Q"), ("Rotate", "Ctrl+Nonsense")]);
    assert_eq!(layered(&global, &junk), global);
}

#[test]
fn lay20_a_layout_key_wins_only_where_contexts_overlap() {
    let global = input::default_bindings();
    // R is Scale's global key: in this layout it is Move's, and Scale's R
    // is hidden there.
    let layout = keys(&[("Move", "R")]);
    let map = layered(&global, &layout);
    assert_eq!(map["Move"], "R");
    assert_eq!(map["Scale"], "", "hidden in this layout");
    assert_eq!(hidden(&global, &layout)["Scale"], "Move");
    assert!(input::bindings_valid(&map));
    assert_eq!(
        mark(&global, &layout, "Move"),
        Some(Mark::Set { global: "W".into() })
    );
    assert_eq!(
        mark(&global, &layout, "Scale"),
        Some(Mark::Hidden { by: "Move".into() })
    );
    assert_eq!(mark(&global, &layout, "Rotate"), None);
    // B in Edge mode: a layout key for Polygon Bevel leaves Edge Bevel and
    // Vertex Bevel, which never apply at the same time, alone.
    let layout = keys(&[("Bevel", "B")]);
    let map = layered(&global, &layout);
    assert_eq!(map["Edge Bevel"], "B");
    assert_eq!(map["Vertex Bevel"], "B");
    // The Render window's keys never meet main-window ones.
    let layout = keys(&[("Move", "P")]);
    assert_eq!(
        layered(&global, &layout)[crate::image_viewer_ui::PAUSE],
        "P"
    );
}

#[test]
fn lay20_switching_layouts_swaps_the_key_that_fires() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    // In Model, H selects Edges mode; Hide Selected's H is hidden there.
    a.execute("operandi.layout.setKey action:Edges key:H");
    assert_eq!(a.status, "Edges: H in Model", "{}", a.status);
    assert_eq!(a.bindings["Edges"], "H");
    assert_eq!(a.bindings["Hide Selected"], "");
    a.execute("Vertices");
    assert_eq!(a.editor.scene.selection.mode, operandi_core::Mode::Vertex);
    press(&mut a, &ctx, Key::H);
    assert_eq!(a.editor.scene.selection.mode, operandi_core::Mode::Edge);
    a.execute("Vertices");
    // UV has no keys of its own: H is Hide Selected again, 2 is Edges.
    a.execute("layout.restore UV");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.bindings, input::default_bindings());
    press(&mut a, &ctx, Key::H);
    assert_eq!(a.editor.scene.selection.mode, operandi_core::Mode::Vertex);
    // Back to Model: H is Edges again.
    a.execute("layout.restore Model");
    frames(&mut a, &ctx, 1);
    press(&mut a, &ctx, Key::H);
    assert_eq!(a.editor.scene.selection.mode, operandi_core::Mode::Edge);
}

#[test]
fn uic201_key_hints_show_the_key_that_fires_and_follow_a_switch() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.layout.setKey action:Move key:Ctrl+M");
    frames(&mut a, &ctx, 1);
    let chord = input::display_chord("Ctrl+M");
    // Button and menu hints, and toolbox {binding} / {shortcut} placeholders.
    assert_eq!(a.shortcut("Move"), format!("  [{chord}] "));
    assert_eq!(
        a.form_tooltip_for_test("Move{binding}", "Move"),
        format!("Move  [{chord}] ")
    );
    assert_eq!(
        a.form_tooltip_for_test("Key: {shortcut}", "Move"),
        "Key: Ctrl+M"
    );
    a.execute("layout.restore UV");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.shortcut("Move"), "  [W] ");
    assert_eq!(a.form_tooltip_for_test("Key: {shortcut}", "Move"), "Key: W");
    // A pie key set only in UV opens that pie only in UV.
    a.execute(&format!(
        "operandi.layout.setKey action:{{{}}} key:Ctrl+Alt+K",
        crate::pie_menu::TOGGLES_LINE
    ));
    assert_eq!(
        a.bindings[crate::pie_menu::TOGGLES_LINE],
        "Ctrl+Alt+K",
        "{}",
        a.status
    );
    a.execute("layout.restore Model");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.bindings[crate::pie_menu::TOGGLES_LINE], "Ctrl+1");
}

#[test]
fn lay71_layout_keys_save_with_the_layout_file_and_reopen() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    assert!(!user_file(&root, "Model").exists());
    a.execute("operandi.layout.setKey action:Move key:Ctrl+M");
    a.execute("operandi.layout.setKey action:Rotate key:none");
    frames(&mut a, &ctx, 1);
    // The user's copy of Model holds them (LAY-71); the global map file is
    // untouched.
    let text = std::fs::read_to_string(user_file(&root, "Model")).unwrap();
    let doc = WindowLayout::from_json(&text).unwrap();
    assert_eq!(doc.keys, keys(&[("Move", "Ctrl+M"), ("Rotate", "")]));
    assert!(!a.environment.paths().input_map().exists());
    // Keys set for a layout that isn't showing go to its file too.
    a.execute("operandi.layout.setKey action:Scale key:Ctrl+Alt+S layout:UV");
    let uv =
        WindowLayout::from_json(&std::fs::read_to_string(user_file(&root, "UV")).unwrap()).unwrap();
    assert_eq!(uv.keys, keys(&[("Scale", "Ctrl+Alt+S")]));
    assert_eq!(a.bindings["Scale"], "R", "UV's key doesn't fire in Model");
    // Reopen: Model's keys are live again.
    let (b, _) = app(&root);
    assert_eq!(b.layouts.current, "Model");
    assert_eq!(b.bindings["Move"], "Ctrl+M");
    assert_eq!(b.bindings["Rotate"], "");
    assert_eq!(b.bindings["Scale"], "R");
    // Giving the key back to the global map, then clearing, drops the copy
    // once it matches the shipped file again.
    a.execute("operandi.layout.setKey action:Move");
    assert_eq!(a.bindings["Move"], "W");
    a.execute("operandi.layout.clearKeys");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.bindings, input::default_bindings());
    assert!(!user_file(&root, "Model").exists());
}

#[test]
fn lay_q56_revert_keeps_layout_keys_unless_asked() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.split_view();
    a.execute("operandi.layout.setKey action:Move key:Ctrl+M");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.views.viewports().len(), 2);
    // Revert puts the arrangement back and keeps the keys (provisional).
    a.execute("operandi.layout.revert");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.views.viewports().len(), 1);
    assert!(a.status.contains("key is kept"), "{}", a.status);
    assert_eq!(a.bindings["Move"], "Ctrl+M");
    let doc = WindowLayout::from_json(&std::fs::read_to_string(user_file(&root, "Model")).unwrap())
        .unwrap();
    assert_eq!(doc.keys, keys(&[("Move", "Ctrl+M")]));
    // The reset question offers to clear them too.
    a.layouts.confirm_reset = Some("Model".into());
    a.layouts.confirm_reset_keys = true;
    a.answer_layout_reset(true);
    frames(&mut a, &ctx, 1);
    assert_eq!(a.status, "Model layout reset");
    assert_eq!(a.bindings, input::default_bindings());
    assert!(!user_file(&root, "Model").exists());
    // keys:true from a script.
    a.execute("operandi.layout.setKey action:Move key:Ctrl+M");
    a.execute("operandi.layout.revert keys:true");
    assert_eq!(a.bindings["Move"], "W");
}

#[test]
fn input_map_layout_scope_sets_marks_and_clears_this_layouts_keys() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.open_input_map();
    a.input_map_ui.scope = Scope::Layout;
    // R on Move in this layout: no clash, Scale's R is hidden here.
    a.assign("Move", "R".into());
    assert!(a.input_map_ui.clash.is_none());
    assert!(a.status.contains("instead of Scale"), "{}", a.status);
    assert_eq!(a.input_map_shown()["Scale"], "");
    assert_eq!(
        a.bindings_draft["Move"], "W",
        "the global draft is untouched"
    );
    // Two layout keys on one chord in one context do clash; Undo puts the
    // layout's earlier state back (no key of its own).
    a.assign("Rotate", "R".into());
    let clash = a.input_map_ui.clash.clone().expect("a clash");
    assert_eq!(clash.others, ["Move"]);
    assert_eq!(clash.previous, None);
    a.layout_keys_draft.remove("Rotate");
    a.input_map_ui.clash = None;
    // The editor draws with the scope choice and the markers.
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    a.save_input_map();
    assert_eq!(a.status, "Input map saved");
    assert_eq!(a.bindings["Move"], "R");
    assert_eq!(a.bindings["Scale"], "");
    assert_eq!(a.global_bindings["Move"], "W");
    // The global scope changes the global map for every layout.
    a.open_input_map();
    assert_eq!(a.layout_keys_draft, keys(&[("Move", "R")]));
    a.input_map_ui.scope = Scope::Global;
    a.assign("Rotate", "Ctrl+Alt+R".into());
    a.save_input_map();
    assert_eq!(a.bindings["Rotate"], "Ctrl+Alt+R");
    assert_eq!(
        input::load_bindings(&a.environment.paths().input_map())["Rotate"],
        "Ctrl+Alt+R"
    );
    a.execute("layout.restore UV");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.bindings["Rotate"], "Ctrl+Alt+R");
    assert_eq!(a.bindings["Move"], "W");
    // Clear this layout's keys.
    a.execute("layout.restore Model");
    a.open_input_map();
    a.layout_keys_draft.clear();
    a.save_input_map();
    assert_eq!(a.bindings["Move"], "W");
    assert_eq!(a.bindings["Scale"], "R");
    assert!(a.current_layout_keys().is_empty());
}

#[test]
fn set_key_refuses_unknown_actions_bad_keys_and_layout_clashes() {
    let root = data_root();
    let (mut a, _ctx) = app(&root);
    a.execute("operandi.layout.setKey action:{No Such Thing} key:Q");
    assert!(a.status.contains("no Input Map action"), "{}", a.status);
    a.execute("operandi.layout.setKey action:Move key:Ctrl+Nope");
    assert!(a.status.contains("isn't a key"), "{}", a.status);
    a.execute("operandi.layout.setKey action:Move key:R");
    a.execute("operandi.layout.setKey action:Rotate key:R");
    assert!(a.status.contains("already Move's key"), "{}", a.status);
    assert_eq!(a.bindings["Rotate"], "E");
    a.execute("operandi.layout.setKey action:Move key:R layout:Nowhere");
    assert!(a.status.contains("No layout named Nowhere"), "{}", a.status);
}

#[test]
fn switching_with_the_held_switcher_leaves_no_stuck_modifier() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.layout.setKey action:Edges key:H layout:UV");
    a.execute("layout.restore UV");
    a.execute("layout.restore Model");
    // The switcher's key is the user's choice since 2026-11-25
    // (Ctrl+Shift+Tab pages the tool tabs by default).
    a.bindings
        .insert("Layout Switcher".into(), "Ctrl+Shift+Tab".into());
    a.bindings
        .insert(crate::tool_tab_cycle::PREVIOUS.into(), String::new());
    frames(&mut a, &ctx, 1);
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
    draw_held(&mut a, &ctx, SIZE, vec![tab], held);
    assert!(a.layouts.switcher.is_some());
    // Releasing Ctrl+Shift switches to UV.
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.layouts.current, "UV", "{}", a.status);
    frames(&mut a, &ctx, 1);
    // A plain H right after runs UV's own key, not a Ctrl/Shift chord.
    press(&mut a, &ctx, Key::H);
    assert_eq!(a.editor.scene.selection.mode, operandi_core::Mode::Edge);
    assert!(a.held_tab.is_empty());
    assert!(a.layouts.switcher.is_none() && a.pie.is_none());
    // Mid-drag or mid-tool the switch waits (LAY-02 keeps the tool).
    a.pointer_held = true;
    a.execute("layout.restore Model");
    assert_eq!(a.layouts.current, "UV");
    a.pointer_held = false;
    a.execute("Cube");
    assert!(a.editor.live.is_some(), "{}", a.status);
    a.execute("layout.restore Model");
    assert_eq!(a.layouts.current, "UV", "{}", a.status);
}

#[test]
fn model_and_uv_are_identical_with_an_empty_layout_context() {
    // The shipped files have no keys and write none, but Animate, whose
    // keys make the animation chords live (UIC-201, TL-61).
    for (_, text) in SHIPPED {
        let doc = WindowLayout::from_json(text).unwrap();
        if doc.name == "Animate" {
            assert!(doc.keys.contains_key("Play/Stop"));
            continue;
        }
        assert!(doc.keys.is_empty(), "{}", doc.name);
        assert!(!doc.to_json().contains("\"keys\""), "{}", doc.name);
    }
    let shapes = |a: &mut Operandi, ctx: &Context| {
        let mut out = None;
        for _ in 0..4 {
            out = Some(draw(a, ctx, SIZE, vec![]));
            a.persist_layout(ctx);
        }
        out.unwrap()
            .shapes
            .iter()
            .map(|s| format!("{:?} {:?}", s.clip_rect, s.shape))
            // What paints nothing (a scroll bar fading in its first
            // frames, still fully clear) isn't part of the look.
            .filter(|s| {
                !s.contains(
                    "fill: #00_00_00_00, stroke: Stroke { width: 0.0, color: #00_00_00_00 }",
                )
            })
            .collect::<Vec<_>>()
    };
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    let paint = |a: &mut Operandi| {
        a.execute("layout.restore Model");
        let model = shapes(a, &ctx);
        assert_eq!(a.bindings, input::default_bindings());
        a.execute("layout.restore UV");
        let uv = shapes(a, &ctx);
        assert_eq!(a.bindings, input::default_bindings());
        (model, uv)
    };
    let (model, uv) = paint(&mut a);
    assert!(!user_file(&root, "Model").exists() && !user_file(&root, "UV").exists());
    // Keys set and cleared again leave nothing behind: no file, same paint.
    for layout in ["Model", "UV"] {
        a.execute(&format!(
            "operandi.layout.setKey action:Move key:Ctrl+M layout:{layout}"
        ));
        a.execute(&format!(
            "operandi.layout.setKey action:Edges key:H layout:{layout}"
        ));
        a.execute(&format!("operandi.layout.clearKeys layout:{layout}"));
    }
    let (model_after, uv_after) = paint(&mut a);
    assert!(!user_file(&root, "Model").exists() && !user_file(&root, "UV").exists());
    assert_eq!(model.len(), model_after.len());
    assert!(model == model_after, "Model paints differently");
    assert!(uv == uv_after, "UV paints differently");
}

#[test]
fn uic201_the_input_map_marks_keys_the_layout_sets_or_hides() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.layout.setKey action:Move key:R");
    ctx.enable_accesskit();
    a.open_input_map();
    a.input_map_ui.search = "Move".into();
    let labels = |a: &mut Operandi| -> Vec<String> {
        let mut out = Vec::new();
        for _ in 0..2 {
            out = draw(a, &ctx, vec2(1440., 1000.), vec![])
                .platform_output
                .accesskit_update
                .expect("AccessKit enabled")
                .nodes
                .into_iter()
                .filter_map(|(_, n)| n.label().or(n.value()).map(str::to_owned))
                .collect();
        }
        out
    };
    // Global scope: Move shows its global W, marked as set by the layout.
    let global = labels(&mut a);
    assert!(global.iter().any(|l| l == "W"), "{global:?}");
    assert!(global.iter().any(|l| l == "◆"), "{global:?}");
    assert!(global.iter().any(|l| l == "Clear this layout's keys"));
    // This layout's scope shows the key that fires here.
    a.input_map_ui.scope = Scope::Layout;
    let here = labels(&mut a);
    assert!(here.iter().any(|l| l == "R"), "{here:?}");
    assert!(here.iter().any(|l| l == "Use Global"), "{here:?}");
    assert!(
        here.iter().any(|l| l.contains("This layout (Model)")),
        "{here:?}"
    );
    // Scale's R is hidden here, marked ◇.
    a.input_map_ui.search = "Scale".into();
    let scale = labels(&mut a);
    assert!(scale.iter().any(|l| l == "◇"), "{scale:?}");
}

#[test]
fn lay_q57_a_copy_carries_the_keys_a_blank_layout_starts_without() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.layout.setKey action:Move key:Ctrl+M");
    a.execute("operandi.layout.create Twin copy:true");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.layouts.current, "Twin", "{}", a.status);
    assert_eq!(a.bindings["Move"], "Ctrl+M");
    a.execute("operandi.layout.rename Pair");
    assert_eq!(a.current_layout_keys(), keys(&[("Move", "Ctrl+M")]));
    a.execute("operandi.layout.create Blank");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.layouts.current, "Blank", "{}", a.status);
    assert_eq!(a.bindings["Move"], "W");
    // Deleting the copy takes its keys with it.
    a.execute("operandi.layout.delete layout:Pair");
    assert!(a.layouts.find("Pair").is_none(), "{}", a.status);
    assert_eq!(a.bindings["Move"], "W");
}

#[test]
fn inactive_layout_key_failures_preserve_the_file_and_allow_identical_retries() {
    use std::fs;
    let root = data_root();
    let (mut a, _) = app(&root);
    let folder = root.join("Layouts");
    let path = user_file(&root, "UV");
    let moved = root.join("moved-layouts");
    let set = |a: &mut Operandi, key| a.set_layout_key("Scale", Some(key), "UV");
    let stored = |a: &Operandi| a.layouts.find("UV").unwrap().doc().keys.clone();

    // Folder creation, then the temp write.
    if folder.exists() {
        fs::rename(&folder, &moved).unwrap();
    } else {
        fs::create_dir(&moved).unwrap();
    }
    fs::write(&folder, b"folder obstruction").unwrap();
    assert!(set(&mut a, "Ctrl+Alt+S").is_err());
    assert!(stored(&a).is_empty());
    assert_eq!(fs::read(&folder).unwrap(), b"folder obstruction");
    fs::remove_file(&folder).unwrap();
    fs::rename(&moved, &folder).unwrap();

    let temp = path.with_extension("json.tmp");
    fs::create_dir(&temp).unwrap();
    assert!(set(&mut a, "Ctrl+Alt+S").is_err());
    assert!(stored(&a).is_empty());
    fs::remove_dir(&temp).unwrap();
    set(&mut a, "Ctrl+Alt+S").unwrap();
    let original = fs::read(&path).unwrap();

    // Rename and removal each fail against an occupied directory.
    let saved = path.with_extension("saved");
    fs::rename(&path, &saved).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(set(&mut a, "Ctrl+Alt+K").is_err());
    assert!(a.clear_layout_keys("UV").is_err());
    assert_eq!(stored(&a), keys(&[("Scale", "Ctrl+Alt+S")]));
    assert_eq!(fs::read(&saved).unwrap(), original);
    assert_eq!(a.bindings["Scale"], "R");
    fs::remove_dir(&path).unwrap();
    fs::rename(&saved, &path).unwrap();

    set(&mut a, "Ctrl+Alt+K").unwrap();
    let (mut b, ctx) = app(&root);
    b.execute("layout.restore UV");
    frames(&mut b, &ctx, 1);
    assert_eq!(b.bindings["Scale"], "Ctrl+Alt+K");
    a.clear_layout_keys("UV").unwrap();
    assert!(!path.exists());
    let (mut b, ctx) = app(&root);
    b.execute("layout.restore UV");
    frames(&mut b, &ctx, 1);
    assert_eq!(b.bindings["Scale"], "R");
}
