//! Workbench edit mode, layouts stage 4 (LAY-82, LAY-86, LAY-87, UIC-212;
//! workbench.rs): Edit Workbench, Pick to Add from a toolbox button, a
//! Tool Properties row, a preset and a Command History line, reorder,
//! every right-click property, dividers, the tool-pipe preset,
//! Duplicate/Delete, undo, save/reopen and Revert.
use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::click_line;
use super::*;
use crate::form::Proficiency;
use crate::form::command_form::{self as cf, Control, Form, PopupFace, ToggleStyle, Width};
use crate::workbench::{TOOLBOX_WORKBENCH, TOP_WORKBENCH};
use std::path::{Path, PathBuf};

const SIZE: Vec2 = vec2(1440., 1400.);

/// An app on a fresh data folder (so form files are written), with a
/// cube in Polygon mode and one polygon selected.
fn app(ctx: &Context) -> (PathBuf, Operandi) {
    let root = crate::layouts_tests::data_root();
    let mut a = crate::layouts_tests::launch(&root, ctx);
    a.activate(Tool::Box);
    a.run(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..Default::default()
    }));
    a.run(Command::Apply);
    a.run(Command::Drop);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![5];
    draw(&mut a, ctx, SIZE, vec![]);
    (root, a)
}

fn bench(a: &Operandi) -> Form {
    a.forms.get(TOOLBOX_WORKBENCH).unwrap().clone()
}

fn lines(a: &Operandi) -> Vec<String> {
    bench(a)
        .controls
        .iter()
        .map(|c| {
            c.command
                .clone()
                .or_else(|| c.property.clone())
                .or_else(|| c.divider.clone().map(|d| format!("--{d}")))
                .unwrap_or_default()
        })
        .collect()
}

fn user_file(root: &Path, id: &str) -> PathBuf {
    root.join("Layouts/Forms").join(cf::file_name(id))
}

fn click_at(a: &mut Operandi, ctx: &Context, at: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(
        a,
        ctx,
        SIZE,
        vec![
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    );
}

fn rect(a: &Operandi, key: &str) -> Rect {
    *a.workbench
        .rects
        .get(key)
        .unwrap_or_else(|| panic!("{key} not drawn; drawn: {:?}", a.workbench.rects.keys()))
}

#[test]
fn the_shipped_layouts_have_two_empty_workbenches_and_nothing_else_changes() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    for id in [TOOLBOX_WORKBENCH, TOP_WORKBENCH] {
        let form = a.forms.shipped(id).unwrap();
        assert!(form.workbench && form.controls.is_empty(), "{id}");
    }
    // No other shipped form is a workbench, and the top row is hidden.
    for id in a.forms.ids() {
        if id != TOOLBOX_WORKBENCH && id != TOP_WORKBENCH {
            assert!(!a.forms.get(id).unwrap().workbench, "{id}");
        }
    }
    assert!(!a.workbench.top_bar);
    // The Workbench tab is the Workbench set's only form (Jason,
    // 2026-11-23): it is never in the Model strip, and its own set draws
    // no strip.
    assert!(!a.toolbox_rects.contains_key("Category Workbench"));
    // Opening it switches to that set, with the Edit Workbench button and
    // an empty hint.
    a.execute("operandi.toolbox.tab workbench");
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.layouts.tool_set, toolbox::ToolSet::Workbench);
    assert!(!a.toolbox_rects.contains_key("Category Workbench"));
    assert!(!a.toolbox_rects.contains_key("Category Create"));
    a.category = toolbox::Category::Create;
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.workbench.tab_shown);
    a.category = toolbox::Category::Workbench;
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.toolbox_rects.contains_key("Edit Workbench"));
    assert!(a.workbench.editing.is_none());
    // A form that isn't a workbench can't be edited.
    a.execute("operandi.workbench.edit form:toolbox.polygon");
    assert!(a.status.contains("isn't a workbench"), "{}", a.status);
    assert!(a.workbench.editing.is_none());
    assert!(
        a.line_state("operandi.workbench.edit form:toolbox.polygon")
            .unwrap()
            .unavailable
            .is_some()
    );
}

#[test]
fn pick_to_add_takes_a_toolbox_button_instead_of_running_it() {
    let ctx = Context::default();
    let (root, mut a) = app(&ctx);
    // Pick needs Edit Workbench first.
    assert!(
        a.line_state("operandi.workbench.pick")
            .unwrap()
            .unavailable
            .is_some()
    );
    a.execute("operandi.workbench.edit");
    assert_eq!(a.workbench.editing.as_deref(), Some(TOOLBOX_WORKBENCH));
    assert_eq!(a.category, toolbox::Category::Workbench);
    a.execute("operandi.workbench.pick");
    assert!(a.workbench.picking, "{}", a.status);
    assert!(a.line_state("operandi.workbench.pick").unwrap().on);
    // Go to the Polygon tab and click Bevel: added, not started.
    a.category = toolbox::Category::Polygon;
    draw(&mut a, &ctx, SIZE, vec![]);
    let steps = a.editor.history.undo.len();
    click_line(&mut a, &ctx, "Bevel", Modifiers::NONE);
    assert!(
        a.editor.session.live.is_none(),
        "a picked button doesn't run"
    );
    assert_eq!(a.editor.history.undo.len(), steps);
    assert_eq!(lines(&a), ["Bevel"]);
    assert_eq!(bench(&a).controls[0].width, Width::Full);
    assert!(
        user_file(&root, TOOLBOX_WORKBENCH).exists(),
        "LAY-87: the user's copy"
    );
    // Esc stops picking (and doesn't cancel anything else).
    draw(&mut a, &ctx, SIZE, vec![key(Key::Escape)]);
    assert!(!a.workbench.picking);
    // Done; the Workbench tab's Bevel button starts Bevel.
    a.execute("operandi.workbench.edit");
    assert!(a.workbench.editing.is_none());
    a.category = toolbox::Category::Workbench;
    draw(&mut a, &ctx, SIZE, vec![]);
    click_line(&mut a, &ctx, "Bevel", Modifiers::NONE);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Bevel)
    );
}

#[test]
fn pick_paints_green_under_the_pointer() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    a.execute("operandi.workbench.edit");
    a.category = toolbox::Category::Polygon;
    draw(&mut a, &ctx, SIZE, vec![]);
    let at = crate::toolbox_form_tests::button_by_line(&a, "Bevel").center();
    let green = |out: &egui::FullOutput| {
        out.shapes.iter().any(|s| {
            matches!(&s.shape, egui::epaint::Shape::Rect(r)
                if r.fill.g() > r.fill.r() && r.fill.g() > r.fill.b() && r.fill.a() < 255 && r.rect.contains(at))
        })
    };
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    let off = draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    assert!(!green(&off), "no green before Pick to Add");
    a.execute("operandi.workbench.pick on");
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    let on = draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    assert!(green(&on), "the button under the pointer lights green");
}

#[test]
fn pick_to_add_takes_a_tool_properties_row_and_shows_it_while_the_tool_is_open() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    a.activate(Tool::Bevel);
    a.tool_properties_open = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![]);
    let (id, row) = a
        .workbench
        .property_rects
        .iter()
        .find(|(k, _)| k.starts_with("Bevel/"))
        .map(|(k, r)| (k.clone(), *r))
        .unwrap_or_else(|| panic!("no Bevel rows: {:?}", a.workbench.property_rects.keys()));
    let before = a.editor.session.live.as_ref().unwrap().params.clone();
    a.execute("operandi.workbench.edit");
    a.execute("operandi.workbench.pick on");
    a.category = toolbox::Category::Polygon;
    draw(&mut a, &ctx, SIZE, vec![]);
    click_at(&mut a, &ctx, row.center());
    assert_eq!(lines(&a), std::slice::from_ref(&id), "{}", a.status);
    assert!(bench(&a).controls[0].property.is_some());
    assert!(
        a.editor.session.live.as_ref().unwrap().params == before,
        "the field didn't change"
    );
    // In use, the Workbench tab draws that row while Bevel is open...
    a.execute("operandi.workbench.edit off");
    a.category = toolbox::Category::Workbench;
    draw(&mut a, &ctx, SIZE, vec![]);
    let shown = rect(&a, &format!("property:{id}"));
    assert!(shown.height() >= 16., "{shown:?}");
    // Its alignment sticks too: Normal draws it a third wide.
    a.execute(&format!(
        "operandi.workbench.set index:0 attr:width value:normal form:{TOOLBOX_WORKBENCH}"
    ));
    draw(&mut a, &ctx, SIZE, vec![]);
    let normal = rect(&a, &format!("property:{id}"));
    assert!(
        normal.width() < shown.width() / 2.,
        "{normal:?} vs {shown:?}"
    );
    // ...and its name, greyed, once Bevel is dropped.
    a.run(Command::Drop);
    draw(&mut a, &ctx, SIZE, vec![]);
    rect(&a, &format!("property:{id}"));
    // The command form of it.
    a.execute("operandi.workbench.addProperty Bevel/shift label:Shift");
    assert_eq!(
        bench(&a).controls[1].property.as_deref(),
        Some("Bevel/shift")
    );
    a.execute("operandi.workbench.addProperty nonsense");
    assert!(a.status.contains("<tool>/<key>"), "{}", a.status);
}

#[test]
fn pick_to_add_takes_a_command_history_line_and_a_preset() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    a.execute("poly.flip");
    let recorded = a.history.log.records.back().unwrap().line.clone();
    a.history.open_height = 300.;
    a.history.tab = command_history::Tab::History;
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![]);
    a.execute("operandi.workbench.edit");
    a.execute("operandi.workbench.pick on");
    draw(&mut a, &ctx, SIZE, vec![]);
    let flip = a
        .history
        .log
        .records
        .iter()
        .position(|r| r.line == recorded)
        .unwrap();
    let row = a
        .history_trees
        .history
        .rects
        .iter()
        .find(|(k, _)| *k == flip)
        .map(|(_, r)| *r)
        .expect("the flip's history row");
    click_at(&mut a, &ctx, row.center());
    assert_eq!(lines(&a), [recorded], "{}", a.status);
    // A Preset Browser entry: a button loading it.
    assert!(a.workbench_pick_preset("Brick", Path::new("/presets/brick.lxp")));
    let preset = bench(&a).controls.last().unwrap().clone();
    assert_eq!(preset.label, "Brick");
    assert_eq!(
        preset.command.as_deref(),
        Some("preset.do preset:\"/presets/brick.lxp\"")
    );
    // Not while Pick to Add is off.
    a.execute("operandi.workbench.pick off");
    assert!(!a.workbench_pick_preset("Brick", Path::new("/presets/brick.lxp")));
    assert!(!a.workbench_pick_history("poly.flip"));
    // The workbench's own lines aren't picked.
    a.execute("operandi.workbench.pick on");
    assert!(!a.workbench_pick_history("operandi.workbench.pick on:on"));
}

#[test]
fn dragging_an_item_reorders_the_workbench() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    for line in ["Select All", "Grow", "Shrink"] {
        a.execute(&format!("operandi.workbench.add {{{line}}}"));
    }
    assert_eq!(lines(&a), ["Select All", "Grow", "Shrink"], "{}", a.status);
    a.execute("operandi.workbench.edit");
    draw(&mut a, &ctx, SIZE, vec![]);
    let first = rect(&a, "toolbox.workbench#0").center();
    let last = rect(&a, "toolbox.workbench#2");
    let target = pos2(first.x, last.bottom() - 2.);
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(first)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(first, true, Modifiers::NONE)],
    );
    for k in 1..=8 {
        let at = first.lerp(target, k as f32 / 8.);
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    }
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(target, false, Modifiers::NONE)],
    );
    assert_eq!(lines(&a), ["Grow", "Shrink", "Select All"], "{}", a.status);
    // The command form, and its bounds.
    a.execute("operandi.workbench.move 2 0");
    assert_eq!(lines(&a), ["Select All", "Grow", "Shrink"]);
    a.execute("operandi.workbench.move 0 9");
    assert!(a.status.contains("has 3 controls"), "{}", a.status);
}

#[test]
fn every_right_click_property_is_set_saved_and_drawn() {
    let ctx = Context::default();
    let (root, mut a) = app(&ctx);
    a.execute("operandi.workbench.add {Select All}");
    a.execute("operandi.workbench.add {Grow}");
    a.execute("operandi.workbench.add {operandi.workbench.topBar} label:{Top Row}");
    for line in [
        "operandi.workbench.set 0 label {Everything}",
        "operandi.workbench.set 0 tooltip {All of it · {shortcut}}",
        "operandi.workbench.set 0 popup popup",
        "operandi.workbench.set 0 width normal",
        "operandi.workbench.set 1 width normal",
        "operandi.workbench.set 1 level advanced",
        "operandi.workbench.set 1 command {Shrink}",
        "operandi.workbench.set 2 toggle checkbox",
    ] {
        a.execute(line);
        assert!(a.last_failure.is_none(), "{line}: {}", a.status);
    }
    let f = bench(&a);
    assert_eq!(f.controls[0].label, "Everything");
    assert_eq!(f.controls[0].tooltip, "All of it · {shortcut}");
    assert_eq!(f.controls[0].popup, PopupFace::Popup);
    assert_eq!(f.controls[1].level, Proficiency::Advanced);
    assert_eq!(f.controls[1].command.as_deref(), Some("Shrink"));
    assert_eq!(f.controls[2].toggle, ToggleStyle::Checkbox);
    // Bad values are refused and change nothing.
    a.execute("operandi.workbench.set 0 width huge");
    assert!(a.status.contains("isn't one of"), "{}", a.status);
    assert_eq!(bench(&a), f);
    a.execute("operandi.workbench.set 9 label x");
    assert!(a.status.contains("no control 9"), "{}", a.status);
    // Drawn: two Normal buttons share a row, a third each; the Full one
    // takes the next row whole.
    a.category = toolbox::Category::Workbench;
    draw(&mut a, &ctx, SIZE, vec![]);
    let all = rect(&a, "toolbox.workbench:Select All");
    let shrink = rect(&a, "toolbox.workbench:Shrink");
    assert_eq!(all.top(), shrink.top(), "Normal buttons share a row");
    assert!((shrink.left() - all.right() - 2.).abs() < 0.5);
    let top_row = rect(&a, "toolbox.workbench:operandi.workbench.topBar");
    assert!(top_row.top() > all.bottom());
    a.execute("operandi.workbench.add {Invert}");
    draw(&mut a, &ctx, SIZE, vec![]);
    let full = rect(&a, "toolbox.workbench:Invert");
    assert!(
        (full.width() - (3. * all.width() + 4.)).abs() < 0.5,
        "{full:?} {all:?}"
    );
    // Wide takes two thirds.
    a.execute("operandi.workbench.set 0 width wide");
    draw(&mut a, &ctx, SIZE, vec![]);
    let wide = rect(&a, "toolbox.workbench:Select All");
    assert!(
        (wide.width() - (2. * all.width() + 2.)).abs() < 0.5,
        "{wide:?}"
    );
    // Proficiency filters (LAY-86): a Basic panel leaves Shrink out.
    let basic = cf::plan(&a.forms, TOOLBOX_WORKBENCH, Proficiency::Basic, &|_| true);
    let advanced = cf::plan(&a.forms, TOOLBOX_WORKBENCH, Proficiency::Advanced, &|_| {
        true
    });
    assert_eq!(advanced.len(), basic.len() + 1);
    // The checkbox style ticks from the line's answer, and runs it.
    assert!(!a.workbench.top_bar);
    let check = a.toolbox_rects["Top Row"];
    click_at(&mut a, &ctx, check.center());
    assert!(a.workbench.top_bar, "{}", a.status);
    // Toggle Button: framed, lit while on.
    a.execute("operandi.workbench.set 2 toggle toggleButton");
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.toolbox_rects.contains_key("Top Row"));
    // The tooltip names the binding as forms do.
    let tip = a
        .form_button_tooltip(&bench(&a).controls[0], "Select All")
        .unwrap();
    assert!(tip.starts_with("All of it · "), "{tip}");
    // Custom icon from disk: a file panel picks a PNG; the button wears it.
    fn stand_in(_: file_chooser::Choose) -> Option<PathBuf> {
        let path = std::env::temp_dir().join("operandi-workbench-icon.png");
        image::RgbaImage::from_pixel(32, 32, image::Rgba([200, 40, 40, 255]))
            .save(&path)
            .unwrap();
        Some(path)
    }
    a.chooser = Some(stand_in);
    a.execute("operandi.workbench.chooseIcon 0");
    let icon = bench(&a).controls[0]
        .icon_file
        .clone()
        .unwrap_or_else(|| panic!("an icon file: {}", a.status));
    assert!(icon.ends_with("operandi-workbench-icon.png"), "{icon}");
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.workbench_icon_loaded(&icon));
    a.execute("operandi.workbench.set 0 iconFile \"\"");
    assert!(bench(&a).controls[0].icon_file.is_none(), "{}", a.status);
    // All of it was saved in the user's copy and reads back.
    let saved = std::fs::read_to_string(user_file(&root, TOOLBOX_WORKBENCH)).unwrap();
    let back = Form::parse(&saved).unwrap();
    assert_eq!(back, bench(&a));
    assert!(
        saved.contains("\"width\": \"wide\"") && saved.contains("\"toggle\": \"toggleButton\"")
    );
}

#[test]
fn right_click_opens_the_item_menu_while_editing() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    a.execute("operandi.workbench.add {Grow}");
    a.execute("operandi.workbench.edit");
    draw(&mut a, &ctx, SIZE, vec![]);
    let at = rect(&a, "toolbox.workbench#0").center();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    let secondary = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(&mut a, &ctx, SIZE, vec![secondary(true), secondary(false)]);
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    assert!(
        egui::Popup::is_any_open(&ctx),
        "the right-click menu opened"
    );
    // It lists the properties, Duplicate, Delete and Show in Form Editor
    // (stage 7).
    let text: String = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("|");
    for entry in [
        "Label",
        "Tooltip",
        "Command",
        "Popup Face",
        "Alignment",
        "Proficiency",
        "Toggle Style",
        "Custom Icon…",
        "Duplicate",
        "Delete",
        "Show in Form Editor",
    ] {
        assert!(text.contains(entry), "{entry} missing from {text}");
    }
    // A click on an item while editing doesn't run it.
    let steps = a.editor.history.undo.len();
    draw(&mut a, &ctx, SIZE, vec![key(Key::Escape)]);
    click_at(&mut a, &ctx, at);
    assert_eq!(a.editor.history.undo.len(), steps);
}

#[test]
fn labelled_dividers_fold_in_use_but_not_while_editing() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    a.execute("operandi.workbench.add {Select All}");
    a.execute("operandi.workbench.addDivider label:Bevels");
    a.execute("operandi.workbench.add {Grow}");
    a.execute("operandi.workbench.addDivider");
    a.execute("operandi.workbench.add {Shrink}");
    assert_eq!(
        lines(&a),
        ["Select All", "--Bevels", "Grow", "--", "Shrink"]
    );
    a.category = toolbox::Category::Workbench;
    draw(&mut a, &ctx, SIZE, vec![]);
    rect(&a, "toolbox.workbench:Grow");
    // Fold it: the section's buttons go; Select All above stays.
    let header = a.toolbox_rects["Bevels"];
    click_at(&mut a, &ctx, header.center());
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(!a.workbench.rects.contains_key("toolbox.workbench:Grow"));
    assert!(!a.workbench.rects.contains_key("toolbox.workbench:Shrink"));
    rect(&a, "toolbox.workbench:Select All");
    // Editing: every item shows, the section stays open and a click on its
    // header doesn't fold anything.
    a.execute("operandi.workbench.edit");
    draw(&mut a, &ctx, SIZE, vec![]);
    for i in 0..5 {
        rect(&a, &format!("toolbox.workbench#{i}"));
    }
    let folded = a.create_folded.clone();
    let header = rect(&a, "toolbox.workbench#1").center();
    click_at(&mut a, &ctx, header);
    assert_eq!(a.create_folded, folded);
    // Add Divider with Label from its button names it Section N.
    draw(&mut a, &ctx, SIZE, vec![]);
    let add = a.toolbox_rects["Add Divider with Label"];
    click_at(&mut a, &ctx, add.center());
    assert_eq!(lines(&a).last().unwrap(), "--Section 6", "{}", a.status);
}

#[test]
fn the_tool_pipe_preset_re_runs_the_same_pipe() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    // No tool: greyed with why. Macro and script aren't built.
    let why = |a: &Operandi, line: &str| a.line_state(line).unwrap().unavailable;
    assert!(
        why(&a, "operandi.workbench.addChain")
            .unwrap()
            .contains("Open a tool")
    );
    for kind in ["macro", "script"] {
        let line = format!("operandi.workbench.addChain as:{kind}");
        assert!(why(&a, &line).unwrap().contains("aren't in Operandi yet"));
        a.execute(&line);
        assert!(a.status.contains("aren't in Operandi yet"), "{}", a.status);
    }
    // A pipe: Selection centre, linear falloff, X symmetry, Bevel.
    a.execute("action center selection");
    a.execute("Falloff linear");
    a.execute("Symmetry x");
    a.execute("Bevel");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Bevel)
    );
    let pipe = (a.editor.action_context, a.editor.symmetry.axis);
    assert!(why(&a, "operandi.workbench.addChain").is_none());
    a.execute("operandi.workbench.addChain");
    assert!(a.last_failure.is_none(), "{}", a.status);
    let preset = bench(&a).controls[0].clone();
    assert_eq!(preset.label, "Bevel Preset");
    let line = preset.command.clone().unwrap();
    assert!(line.starts_with("cmds.batch "), "{line}");
    assert!(
        preset.tooltip.contains("action center selection"),
        "{}",
        preset.tooltip
    );
    // Everything back to plain.
    a.run(Command::Drop);
    a.execute("action center automatic");
    a.execute("Falloff none");
    a.execute("Symmetry off");
    assert!(a.editor.session.live.is_none() && a.editor.spatial_falloff.is_none());
    assert_ne!((a.editor.action_context, a.editor.symmetry.axis), pipe);
    // One click on the preset button puts the whole pipe back.
    a.category = toolbox::Category::Workbench;
    draw(&mut a, &ctx, SIZE, vec![]);
    click_line(&mut a, &ctx, &line, Modifiers::NONE);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Bevel),
        "{}",
        a.status
    );
    assert_eq!((a.editor.action_context, a.editor.symmetry.axis), pipe);
    assert!(matches!(a.editor.spatial_falloff, Some(Falloff::Linear(_))));
    // With a name of its own.
    a.execute("operandi.workbench.addChain label:{Soft Bevel}");
    assert_eq!(bench(&a).controls[1].label, "Soft Bevel");
    // addPipe, its name before 2026-11-23, still adds one.
    a.execute("operandi.workbench.addPipe label:{Old Name}");
    assert_eq!(bench(&a).controls[2].label, "Old Name", "{}", a.status);
}

#[test]
fn duplicate_delete_and_undo() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    a.execute("operandi.workbench.add {Grow} label:More");
    a.execute("operandi.workbench.add {Shrink}");
    assert!(
        a.line_state("operandi.workbench.undo")
            .unwrap()
            .unavailable
            .is_none()
    );
    a.execute("operandi.workbench.duplicate 0");
    assert_eq!(lines(&a), ["Grow", "Grow", "Shrink"]);
    assert_eq!(bench(&a).controls[1].label, "More");
    a.execute("operandi.workbench.delete 2");
    assert_eq!(lines(&a), ["Grow", "Grow"]);
    a.execute("operandi.workbench.delete 5");
    assert!(a.status.contains("no control 5"), "{}", a.status);
    // Workbench edits aren't scene undo steps; they have their own undo.
    let steps = a.editor.history.undo.len();
    a.execute("operandi.workbench.undo");
    assert_eq!(lines(&a), ["Grow", "Grow", "Shrink"]);
    a.execute("operandi.workbench.undo");
    assert_eq!(lines(&a), ["Grow", "Shrink"]);
    assert_eq!(a.editor.history.undo.len(), steps);
    a.execute("operandi.workbench.undo");
    a.execute("operandi.workbench.undo");
    assert!(lines(&a).is_empty());
    assert!(
        a.line_state("operandi.workbench.undo")
            .unwrap()
            .unavailable
            .is_some()
    );
    a.execute("operandi.workbench.undo");
    assert!(a.status.contains("No workbench edit"), "{}", a.status);
}

#[test]
fn workbench_edits_survive_a_restart_and_revert_drops_them() {
    let ctx = Context::default();
    let (root, mut a) = app(&ctx);
    a.execute("operandi.workbench.add {Grow}");
    a.execute("operandi.workbench.addDivider label:Mine");
    a.execute("operandi.workbench.set 0 width wide");
    a.execute("operandi.workbench.add {Select All} form:toolbar.workbench");
    let mine = bench(&a);
    let top = a.forms.get(TOP_WORKBENCH).unwrap().clone();
    // The next start reads the user's copies; the shipped ones are untouched.
    let mut b = crate::layouts_tests::launch(&root, &ctx);
    assert_eq!(b.forms.get(TOOLBOX_WORKBENCH), Some(&mine));
    assert_eq!(b.forms.get(TOP_WORKBENCH), Some(&top));
    assert!(
        b.forms
            .shipped(TOOLBOX_WORKBENCH)
            .unwrap()
            .controls
            .is_empty()
    );
    // The top row draws its button once shown.
    b.execute("operandi.workbench.topBar");
    draw(&mut b, &ctx, SIZE, vec![]);
    let at = b.workbench.rects["toolbar.workbench:Select All"];
    let tool_controls = b.toolbox_rects["Category Create"];
    assert!(at.bottom() < tool_controls.top(), "{at:?}");
    // Revert: the file goes, the shipped (empty) workbench shows, and the
    // workbench undo for it is forgotten.
    b.execute("operandi.form.revert toolbox.workbench");
    assert!(
        !user_file(&root, TOOLBOX_WORKBENCH).exists(),
        "{}",
        b.status
    );
    assert!(bench(&b).controls.is_empty());
    b.execute("operandi.workbench.add {Grow}");
    b.execute("operandi.form.revert toolbox.workbench");
    b.execute("operandi.workbench.undo");
    assert!(b.status.contains("No workbench edit"), "{}", b.status);
    // The Revert button in the edit tools runs the same line.
    b.execute("operandi.workbench.add {Grow}");
    b.execute("operandi.workbench.edit");
    draw(&mut b, &ctx, SIZE, vec![]);
    let revert = b.toolbox_rects["Revert"];
    click_at(&mut b, &ctx, revert.center());
    assert!(bench(&b).controls.is_empty(), "{}", b.status);
}

#[test]
fn the_top_workbench_is_a_slim_hideable_row() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    let toolbox_top = |a: &Operandi| a.toolbox_rects["Category Create"].top();
    let before = toolbox_top(&a);
    a.execute("operandi.workbench.topBar");
    assert!(a.line_state("operandi.workbench.topBar").unwrap().on);
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![]);
    let moved = toolbox_top(&a) - before;
    assert!(moved > 0. && moved <= 30., "slim: {moved}");
    // Edit it in place: Pick to Add puts a button on it.
    a.execute("operandi.workbench.edit form:toolbar.workbench");
    a.execute("operandi.workbench.pick on");
    a.category = toolbox::Category::Polygon;
    draw(&mut a, &ctx, SIZE, vec![]);
    click_line(&mut a, &ctx, "Bevel", Modifiers::NONE);
    let top = a.forms.get(TOP_WORKBENCH).unwrap();
    assert_eq!(top.controls[0].command.as_deref(), Some("Bevel"));
    assert!(
        bench(&a).controls.is_empty(),
        "the toolbox's workbench is untouched"
    );
    // Hide it: editing it ends, the layout is as before.
    a.execute("operandi.workbench.topBar off");
    assert!(a.workbench.editing.is_none() && !a.workbench.picking);
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(toolbox_top(&a), before);
}

#[test]
fn the_new_control_fields_read_and_write_and_default_to_nothing() {
    let text = r#"{
        "id": "mine",
        "workbench": true,
        "controls": [
            {"command": "Grow", "width": "wide", "toggle": "checkbox", "popup": "buttons", "iconFile": "/x.png"},
            {"property": "Bevel/inset", "label": "Inset"}
        ]
    }"#;
    let form = Form::parse(text).unwrap();
    assert!(form.workbench);
    assert_eq!(form.controls[0].width, Width::Wide);
    assert_eq!(form.controls[0].toggle, ToggleStyle::Checkbox);
    assert_eq!(form.controls[0].popup, PopupFace::Buttons);
    assert_eq!(form.controls[0].icon_file.as_deref(), Some("/x.png"));
    assert_eq!(Form::parse(&form.to_json()).unwrap(), form);
    // A property is a control kind of its own.
    assert!(Form::parse(r#"{"id":"x","controls":[{"property":"a/b","command":"Grow"}]}"#).is_err());
    // Defaults aren't written, so shipped forms read as they did.
    let plain = Control {
        command: Some("Grow".into()),
        ..Default::default()
    };
    let json = serde_json::to_string(&plain).unwrap();
    assert_eq!(json, r#"{"command":"Grow"}"#);
    let not_bench = Form::parse(r#"{"id":"x"}"#).unwrap();
    assert!(!not_bench.to_json().contains("workbench"));
    assert_eq!(Width::named("full width"), Some(Width::Full));
    assert_eq!(Width::named("full"), Some(Width::Full));
    assert_eq!(
        ToggleStyle::named("toggleButton"),
        Some(ToggleStyle::ToggleButton)
    );
    assert_eq!(PopupFace::named("pop-up"), Some(PopupFace::Popup));
}

/// The paint-identical check (opt-in, build rule of stage 4): every shape
/// the Model layout paints with each of its toolbox tabs open, one per
/// line, written to the file `OPERANDI_PAINT_DUMP` names. Run it on the
/// build before stage 4 and on this one and compare: the only change is
/// the Workbench tab added to the strip.
#[test]
#[ignore]
fn paint_dump_model_tabs() {
    let Ok(path) = std::env::var("OPERANDI_PAINT_DUMP") else {
        return;
    };
    // OPERANDI_PAINT_HEIGHT draws a shorter window (default 1400).
    let height = std::env::var("OPERANDI_PAINT_HEIGHT")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or(SIZE.y);
    let size = vec2(SIZE.x, height);
    let mut text = String::new();
    for tab in [
        "Create",
        "Select",
        "Deform",
        "Sculpt",
        "Duplicate",
        "Edit",
        "Vertex",
        "Edge",
        "Polygon",
    ] {
        let root = crate::layouts_tests::data_root();
        let ctx = Context::default();
        let mut a = crate::layouts_tests::launch(&root, &ctx);
        a.category = toolbox::Category::named(tab).unwrap();
        let mut out = None;
        for _ in 0..3 {
            out = Some(draw(&mut a, &ctx, size, vec![]));
        }
        text.push_str(&format!("== {tab}\n"));
        for s in &out.unwrap().shapes {
            text.push_str(&format!("{:?} {:?}\n", s.clip_rect, s.shape));
        }
    }
    std::fs::write(path, text).unwrap();
}

/// Where `text` was drawn in `out` (the first text shape that is exactly it).
fn text_rect(out: &egui::FullOutput, text: &str) -> Option<Rect> {
    out.shapes.iter().find_map(|s| match &s.shape {
        egui::epaint::Shape::Text(t) if t.galley.text() == text => {
            Some(t.galley.rect.translate(t.pos.to_vec2()))
        }
        _ => None,
    })
}

/// Opens item `index`'s right-click menu, opens submenu `title` and
/// clicks `choice` in it, as a user would.
fn pick_in_item_menu(a: &mut Operandi, ctx: &Context, index: usize, title: &str, choice: &str) {
    let at = rect(a, &format!("{TOOLBOX_WORKBENCH}#{index}")).center();
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    let secondary = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(a, ctx, SIZE, vec![secondary(true), secondary(false)]);
    let out = draw(a, ctx, SIZE, vec![]);
    let sub = text_rect(&out, title).unwrap_or_else(|| panic!("{title} not in the menu"));
    draw(a, ctx, SIZE, vec![Event::PointerMoved(sub.center())]);
    let mut out = draw(a, ctx, SIZE, vec![]);
    for _ in 0..3 {
        if text_rect(&out, choice).is_some() {
            break;
        }
        out = draw(a, ctx, SIZE, vec![]);
    }
    let item = text_rect(&out, choice).unwrap_or_else(|| panic!("{choice} not in {title}"));
    // Along the submenu's row first, then onto the choice, so the pointer
    // stays inside the menus on its way.
    draw(
        a,
        ctx,
        SIZE,
        vec![Event::PointerMoved(pos2(item.center().x, sub.center().y))],
    );
    click_at(a, ctx, item.center());
    draw(a, ctx, SIZE, vec![]);
}

#[test]
fn right_click_properties_stick_through_the_menu() {
    let ctx = Context::default();
    let (root, mut a) = app(&ctx);
    a.execute("operandi.workbench.add {Select All}");
    a.execute("operandi.workbench.add {Grow}");
    a.execute("operandi.workbench.edit on:true");
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(bench(&a).controls[0].width, Width::Full);
    pick_in_item_menu(&mut a, &ctx, 0, "Alignment", "Normal");
    assert_eq!(bench(&a).controls[0].width, Width::Normal, "{}", a.status);
    pick_in_item_menu(&mut a, &ctx, 1, "Alignment", "Wide");
    pick_in_item_menu(&mut a, &ctx, 1, "Proficiency", "Advanced");
    pick_in_item_menu(&mut a, &ctx, 1, "Toggle Style", "Checkbox");
    pick_in_item_menu(&mut a, &ctx, 0, "Popup Face", "Buttons");
    let f = bench(&a);
    assert_eq!(f.controls[0].width, Width::Normal);
    assert_eq!(f.controls[0].popup, PopupFace::Buttons);
    assert_eq!(f.controls[1].width, Width::Wide);
    assert_eq!(f.controls[1].level, Proficiency::Advanced);
    assert_eq!(f.controls[1].toggle, ToggleStyle::Checkbox);
    // Leave edit mode: Normal draws a third of the row, still after redraws.
    a.execute("operandi.workbench.edit on:false");
    a.category = toolbox::Category::Workbench;
    for _ in 0..3 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    assert_eq!(bench(&a).controls[0].width, Width::Normal);
    let all = rect(&a, "toolbox.workbench:Select All");
    let panel = a.toolbox_rects.values().fold(all, |r, o| r.union(*o));
    assert!(all.width() < panel.width() / 2., "{all:?} in {panel:?}");
    // The menu's text fields and Clear Custom Icon send the same named
    // form (menu_text): they take too.
    for (attr, value) in [
        ("label", "{All}"),
        ("tooltip", "{Every one}"),
        ("command", "{Select All}"),
        ("iconFile", "\"\""),
    ] {
        a.execute(&format!(
            "operandi.workbench.set index:0 attr:{attr} value:{value} form:{TOOLBOX_WORKBENCH}"
        ));
        assert!(a.status.ends_with("set"), "{attr}: {}", a.status);
    }
    let f = bench(&a);
    assert_eq!(
        (f.controls[0].label.as_str(), f.controls[0].tooltip.as_str()),
        ("All", "Every one")
    );
    // Save/reopen keeps them.
    let saved = std::fs::read_to_string(user_file(&root, TOOLBOX_WORKBENCH)).unwrap();
    assert_eq!(Form::parse(&saved).unwrap(), bench(&a));
    let b = crate::layouts_tests::launch(&root, &Context::default());
    assert_eq!(b.forms.get(TOOLBOX_WORKBENCH).unwrap(), &f);
}

// ---------------------------------------------- menu text fields, labels

/// Opens item `index`'s right-click menu with a real right-click.
fn open_item_menu(a: &mut Operandi, ctx: &Context, index: usize) -> egui::FullOutput {
    let at = rect(a, &format!("{TOOLBOX_WORKBENCH}#{index}")).center();
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    let secondary = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(a, ctx, SIZE, vec![secondary(true), secondary(false)]);
    let out = draw(a, ctx, SIZE, vec![]);
    assert!(egui::Popup::is_any_open(ctx), "the item menu opened");
    out
}

fn field_id(index: usize, attr: &str) -> Id {
    Id::new(("workbench-field", TOOLBOX_WORKBENCH, index, attr))
}

/// Where item `index`'s `attr` text field was drawn in the open menu.
fn field(ctx: &Context, index: usize, attr: &str) -> Rect {
    ctx.read_response(field_id(index, attr))
        .unwrap_or_else(|| panic!("no {attr} field drawn"))
        .rect
}

/// Presses, then on the next frame releases, at `at`: the release is
/// what used to close the menu.
fn press_then_release(a: &mut Operandi, ctx: &Context, at: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(a, ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![pointer(at, false, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![]);
}

/// Replaces the focused field's text, as typed.
fn retype(a: &mut Operandi, ctx: &Context, text: &str) {
    let select_all = Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    };
    draw(a, ctx, SIZE, vec![select_all, Event::Text(text.into())]);
}

#[test]
fn menu_text_fields_stay_open_on_click_release_and_commit_or_cancel() {
    let ctx = Context::default();
    let (_root, mut a) = app(&ctx);
    a.execute("operandi.workbench.add {bevel}");
    a.execute("operandi.workbench.edit on:true");
    draw(&mut a, &ctx, SIZE, vec![]);
    let out = open_item_menu(&mut a, &ctx, 0);
    // The Label field starts from the name the button shows: the tool's
    // ("Bevel"; the line and the edit chip say "bevel").
    let label = field(&ctx, 0, "label");
    let shown = text_rect(&out, "Bevel").expect("the Label field starts filled in");
    assert!(
        label.expand(2.).contains(shown.center()),
        "{shown:?} {label:?}"
    );
    // Click-release into Label: the menu stays open, the field has focus.
    press_then_release(&mut a, &ctx, label.center());
    assert!(
        egui::Popup::is_any_open(&ctx),
        "the click into Label closed the menu"
    );
    assert!(ctx.memory(|m| m.has_focus(field_id(0, "label"))));
    retype(&mut a, &ctx, "Round Off");
    assert!(egui::Popup::is_any_open(&ctx));
    assert!(
        bench(&a).controls[0].label.is_empty(),
        "nothing set while typing"
    );
    // Enter sets it and closes the menu.
    draw(&mut a, &ctx, SIZE, vec![key(Key::Enter)]);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(bench(&a).controls[0].label, "Round Off", "{}", a.status);
    assert!(!egui::Popup::is_any_open(&ctx));
    // Tooltip, then a click away from the menu: set, menu closed.
    open_item_menu(&mut a, &ctx, 0);
    let tooltip = field(&ctx, 0, "tooltip");
    press_then_release(&mut a, &ctx, tooltip.center());
    assert!(
        egui::Popup::is_any_open(&ctx),
        "the click into Tooltip closed the menu"
    );
    retype(&mut a, &ctx, "Rounds the edges");
    let away = pos2(SIZE.x - 30., 60.);
    press_then_release(&mut a, &ctx, away);
    assert!(
        !egui::Popup::is_any_open(&ctx),
        "a click away closes the menu"
    );
    assert_eq!(
        bench(&a).controls[0].tooltip,
        "Rounds the edges",
        "{}",
        a.status
    );
    // Typing into one field, then clicking into another, sets the first.
    open_item_menu(&mut a, &ctx, 0);
    press_then_release(&mut a, &ctx, field(&ctx, 0, "label").center());
    retype(&mut a, &ctx, "Round");
    press_then_release(&mut a, &ctx, field(&ctx, 0, "tooltip").center());
    assert!(egui::Popup::is_any_open(&ctx));
    assert_eq!(bench(&a).controls[0].label, "Round");
    // Esc drops what was typed and closes the menu.
    press_then_release(&mut a, &ctx, field(&ctx, 0, "label").center());
    retype(&mut a, &ctx, "Nope");
    draw(&mut a, &ctx, SIZE, vec![key(Key::Escape)]);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(!egui::Popup::is_any_open(&ctx));
    assert_eq!(bench(&a).controls[0].label, "Round");
    // A submenu choice still closes the whole menu.
    pick_in_item_menu(&mut a, &ctx, 0, "Alignment", "Wide");
    assert_eq!(bench(&a).controls[0].width, Width::Wide);
    assert!(!egui::Popup::is_any_open(&ctx), "a choice closes the menu");
}

#[test]
fn a_cleared_label_makes_icon_only_buttons_six_to_a_row() {
    let ctx = Context::default();
    let (root, mut a) = app(&ctx);
    let tools = [
        "cube", "sphere", "cylinder", "bevel", "inset", "extrude", "shift",
    ];
    for (i, line) in tools.iter().enumerate() {
        a.execute(&format!("operandi.workbench.add {{{line}}}"));
        a.execute(&format!("operandi.workbench.set {i} width normal"));
        a.execute(&format!(
            "operandi.workbench.set index:{i} attr:label value:\"\" form:{TOOLBOX_WORKBENCH}"
        ));
        assert!(a.last_failure.is_none(), "{line}: {}", a.status);
    }
    // A command with no icon: cleared, it still shows its name.
    a.execute("operandi.workbench.add {Select All}");
    a.execute("operandi.workbench.set 7 width normal");
    a.execute("operandi.workbench.set 7 label \"\"");
    let f = bench(&a);
    assert!(f.controls.iter().all(|c| c.no_label && c.label.is_empty()));
    a.category = toolbox::Category::Workbench;
    draw(&mut a, &ctx, SIZE, vec![]);
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    let cells: Vec<Rect> = tools
        .iter()
        .map(|l| rect(&a, &format!("{TOOLBOX_WORKBENCH}:{l}")))
        .collect();
    // Six share the first row, each a sixth; the seventh starts the next.
    for c in &cells[1..6] {
        assert_eq!(c.top(), cells[0].top(), "{cells:?}");
        assert!((c.width() - cells[0].width()).abs() < 0.5);
    }
    for pair in cells[..6].windows(2) {
        assert!(
            (pair[1].left() - pair[0].right() - 2.).abs() < 0.5,
            "{pair:?}"
        );
    }
    assert!(cells[6].top() > cells[0].bottom() - 0.5, "{cells:?}");
    let all = rect(&a, &format!("{TOOLBOX_WORKBENCH}:Select All"));
    assert!(
        (all.width() - (2. * cells[0].width() + 2.)).abs() < 0.5,
        "no icon: a labelled Normal slot, {all:?}"
    );
    assert!(
        text_rect(&out, "Select All").is_some(),
        "never a blank button"
    );
    for tool in ["Cube", "Bevel", "Shift"] {
        assert!(text_rect(&out, tool).is_none(), "{tool} drew its label");
    }
    // The tooltip still names it in full.
    let tip = a.workbench_icon_tip(&f.controls[3], "bevel");
    assert!(tip.starts_with("Bevel"), "{tip}");
    // Wide is two slots.
    a.execute("operandi.workbench.set 0 width wide");
    draw(&mut a, &ctx, SIZE, vec![]);
    let wide = rect(&a, &format!("{TOOLBOX_WORKBENCH}:cube"));
    assert!(
        (wide.width() - (2. * cells[1].width() + 2.)).abs() < 0.5,
        "{wide:?}"
    );
    // Saved in the user's copy and back after a restart.
    let saved = std::fs::read_to_string(user_file(&root, TOOLBOX_WORKBENCH)).unwrap();
    assert!(saved.contains("\"noLabel\": true"), "{saved}");
    let b = crate::layouts_tests::launch(&root, &Context::default());
    assert_eq!(b.forms.get(TOOLBOX_WORKBENCH).unwrap(), &bench(&a));
}

#[test]
fn reset_label_brings_back_the_default_name() {
    let ctx = Context::default();
    let (root, mut a) = app(&ctx);
    a.execute("operandi.workbench.add {bevel}");
    a.execute("operandi.workbench.add {Grow} label:More");
    a.execute("operandi.workbench.set 0 label \"\"");
    assert!(bench(&a).controls[0].no_label);
    a.execute("operandi.workbench.edit on:true");
    draw(&mut a, &ctx, SIZE, vec![]);
    // Icon-only: the Label field starts empty; Reset Label is offered.
    let out = open_item_menu(&mut a, &ctx, 0);
    assert!(
        text_rect(&out, "Bevel").is_none(),
        "an icon-only Label is empty"
    );
    let reset = text_rect(&out, "Reset Label").expect("Reset Label in the menu");
    click_at(&mut a, &ctx, reset.center());
    draw(&mut a, &ctx, SIZE, vec![]);
    let c = &bench(&a).controls[0];
    assert!(!c.no_label && c.label.is_empty(), "{c:?} {}", a.status);
    assert!(!egui::Popup::is_any_open(&ctx));
    // A custom label resets too; a default one offers no Reset.
    let out = open_item_menu(&mut a, &ctx, 1);
    let reset = text_rect(&out, "Reset Label").expect("Reset Label for More");
    click_at(&mut a, &ctx, reset.center());
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(bench(&a).controls[1].label.is_empty(), "{}", a.status);
    let out = open_item_menu(&mut a, &ctx, 1);
    assert!(text_rect(&out, "Reset Label").is_none());
    assert!(
        text_rect(&out, "Grow").is_some(),
        "the field shows the default name"
    );
    // Typing the default name keeps the label following it.
    a.execute("operandi.workbench.set 0 label Bevel");
    assert!(bench(&a).controls[0].label.is_empty());
    let b = crate::layouts_tests::launch(&root, &Context::default());
    assert_eq!(b.forms.get(TOOLBOX_WORKBENCH).unwrap(), &bench(&a));
}
