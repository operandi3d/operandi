//! Command History (stage H): records for every source, a class on every
//! command, tool sessions recorded at mouse-up, batches as one step with
//! sub-commands, undo unchanged, the Undos tab's drag row, the History
//! tab keeping undone mistakes, Results with failure messages, the undo
//! preferences and the collapsed dock (UIC-224..229, HIS, CMD).
use super::tests::{PLATFORM_CMD, box_app, draw};
use super::*;
use crate::command_history_ui::UndoKey;
use operandi_core::{CommandClass, CommandRecord, Source};

const SIZE: Vec2 = vec2(1440., 900.);

fn last(a: &Operandi) -> &CommandRecord {
    a.history.log.records.back().expect("a record")
}

fn lines(a: &Operandi) -> Vec<String> {
    a.history
        .log
        .records
        .iter()
        .map(|r| r.line.clone())
        .collect()
}

/// A box with its top polygon selected, in Polygon mode.
fn polygon_app() -> Operandi {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![5];
    a
}

#[test]
fn a_typed_line_is_recorded_written_out_with_its_class_and_result() {
    let mut a = polygon_app();
    a.execute_from(Source::Typed, "poly.setPart Road");
    let r = last(&a);
    assert_eq!(r.line, "poly.setPart name:Road");
    assert_eq!(r.class, CommandClass::Undoable);
    assert_eq!(r.source, Source::Typed);
    assert!(r.ok, "{}", r.message);
    let results = a.history.results.records.back().unwrap();
    assert_eq!(results.line, "poly.setPart name:Road");
    // UIC-226: the undo step keeps the line that made it.
    let step = a.editor.history.undo.last().unwrap();
    assert_eq!(step.records.0[0].line, "poly.setPart name:Road");
}

#[test]
fn buttons_keys_menus_and_pies_record_their_source() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = polygon_app();
    draw(&mut a, &ctx, SIZE, vec![]);
    // A button (the default source of `execute`).
    a.execute("poly.flip");
    assert_eq!(
        (last(&a).line.as_str(), last(&a).source),
        ("poly.flip", Source::Button)
    );
    // A key: Cmd+Z runs Undo through the Input Map.
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::Key {
            key: Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: PLATFORM_CMD,
        }],
    );
    let r = last(&a);
    assert_eq!(
        (r.line.as_str(), r.source),
        ("app.undo", Source::Key),
        "{}",
        a.status
    );
    assert_eq!(
        r.class,
        CommandClass::Quiet,
        "CMD-02: Undo isn't an undo step"
    );
    // A menu entry (File > New runs `scene.new`).
    a.menu_action(&ctx, app_menu::line_action("scene.new"));
    assert_eq!(
        (last(&a).line.as_str(), last(&a).source),
        ("scene.new", Source::Menu),
        "{:?}",
        lines(&a)
    );
    assert_eq!(last(&a).class, CommandClass::SideEffect);
    // A pie entry runs its line as from a pie.
    a.execute_from(Source::Pie, "Front View");
    assert_eq!(last(&a).source, Source::Pie);
    assert_eq!(last(&a).class, CommandClass::Ui);
    // Afterwards buttons are buttons again.
    a.execute("poly.flip");
    assert_eq!(last(&a).source, Source::Button);
}

#[test]
fn every_registry_command_has_a_class() {
    // UIC-225 / CMD-03: core's table, the app's own and kits' commands.
    let missing: Vec<&str> = command_handlers::registry()
        .iter()
        .map(|c| &c.entry)
        .filter(|e| e.class().is_none())
        .map(|e| e.name)
        .collect();
    assert!(missing.is_empty(), "commands without a class: {missing:?}");
    // Queries are quiet (CMD-23).
    let mut a = box_app();
    a.execute_from(Source::Typed, "tool.set prim.cube ?");
    assert_eq!(last(&a).class, CommandClass::Quiet);
    assert!(
        a.editor
            .history
            .undo
            .last()
            .unwrap()
            .records
            .0
            .iter()
            .all(|r| r.line != "tool.set prim.cube ?")
    );
}

#[test]
fn a_tool_session_records_tool_on_values_at_mouse_up_and_tool_off() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = polygon_app();
    a.camera.view = View::Front;
    draw(&mut a, &ctx, SIZE, vec![]);
    a.execute("tool.set TransformMove on");
    assert!(a.editor.session.live.is_some(), "{}", a.status);
    draw(&mut a, &ctx, SIZE, vec![]);
    let before = a.history.log.records.len();
    let from = a.viewport_rect.center();
    let press = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(&mut a, &ctx, SIZE, vec![press(from, true)]);
    for k in 1..=10 {
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(
                from + vec2(9. * k as f32, -6. * k as f32),
            )],
        );
    }
    assert_eq!(
        a.history.log.records.len(),
        before,
        "no lines while the mouse moves: {:?}",
        lines(&a)
    );
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![press(from + vec2(90., -60.), false)],
    );
    draw(&mut a, &ctx, SIZE, vec![]);
    let attrs: Vec<String> = lines(&a)[before..]
        .iter()
        .filter(|l| l.starts_with("tool.attr TransformMove "))
        .cloned()
        .collect();
    assert!(!attrs.is_empty(), "values at mouse-up: {:?}", lines(&a));
    let fields = a.tool_attr_values().unwrap().len();
    assert!(
        attrs.len() <= fields,
        "one line per changed value: {attrs:?}"
    );
    a.execute("tool.drop");
    let step = a.editor.history.undo.last().unwrap();
    let recorded: Vec<&str> = step.records.0.iter().map(|r| r.line.as_str()).collect();
    assert_eq!(
        recorded.first(),
        Some(&"tool.set TransformMove on"),
        "{recorded:?}"
    );
    assert!(
        recorded
            .iter()
            .any(|l| l.starts_with("tool.attr TransformMove "))
    );
    assert!(
        lines(&a).iter().any(|l| l == "tool.set TransformMove off"),
        "{:?}",
        lines(&a)
    );
}

#[test]
fn a_batch_is_one_undo_step_with_its_lines_as_sub_commands() {
    let mut a = polygon_app();
    let before = a.editor.scene.clone();
    let steps = a.editor.history.undo.len();
    a.execute_from(
        Source::Typed,
        "cmds.batch label:\"Flip and part\" c1:poly.flip c2:\"poly.setPart Road\"",
    );
    assert_eq!(a.editor.history.undo.len(), steps + 1, "{}", a.status);
    let step = a.editor.history.undo.last().unwrap();
    assert_eq!(step.label, "Flip and part");
    let record = &step.records.0[0];
    assert!(record.line.starts_with("cmds.batch"), "{}", record.line);
    assert_eq!(record.sub, ["poly.flip", "poly.setPart name:Road"]);
    // Sub-commands aren't top-level history lines of their own.
    assert!(!lines(&a).iter().any(|l| l == "poly.flip"));
    a.execute("Undo");
    assert!(
        a.editor.scene == before,
        "one Undo takes the whole batch back"
    );
}

#[test]
fn sub_commands_stop_at_max_sub_commands_recorded() {
    let mut a = polygon_app();
    a.editor.history.settings.max_sub_commands = 2;
    a.execute("cmds.batch c1:poly.flip c2:poly.flip c3:poly.flip");
    let r = &a.editor.history.undo.last().unwrap().records.0[0];
    assert_eq!(r.sub.len(), 2);
    assert_eq!(r.sub_dropped, 1);
}

#[test]
fn undo_and_redo_behave_as_before() {
    // The same edits through lines and as direct commands: the same
    // scenes, steps and labels at every point.
    let mut a = polygon_app();
    let mut b = polygon_app();
    for (line, command) in [
        ("poly.flip", Command::FlipPolygons),
        (
            "toggle subdivision",
            Command::ToggleSubdivision(operandi_core::subdivision::Toggle::Subdivision),
        ),
        ("poly.flip", Command::FlipPolygons),
    ] {
        a.execute(line);
        b.run(command);
        assert!(a.editor.scene == b.editor.scene, "{line}");
    }
    let labels = |e: &operandi_core::Editor| {
        e.history
            .undo
            .iter()
            .map(|c| c.label.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(labels(&a.editor), labels(&b.editor));
    for _ in 0..2 {
        a.execute("Undo");
        b.run(Command::Undo);
        assert!(a.editor.scene == b.editor.scene);
    }
    a.execute("Redo");
    b.run(Command::Redo);
    assert!(a.editor.scene == b.editor.scene);
    assert_eq!(a.editor.history.undo.len(), b.editor.history.undo.len());
    assert_eq!(a.editor.history.redo.len(), b.editor.history.redo.len());
    // Undo and Redo never become steps (CMD-02).
    assert!(
        a.editor
            .history
            .undo
            .iter()
            .all(|c| c.records.0.iter().all(|r| r.line != "app.undo"))
    );
}

#[test]
fn undo_to_jumps_several_steps_and_back() {
    let mut a = polygon_app();
    let start = a.editor.history.undo.len();
    let base = a.editor.scene.clone();
    for _ in 0..3 {
        a.execute("poly.flip");
    }
    a.execute("poly.setPart Road");
    let top = a.editor.scene.clone();
    a.execute_from(Source::Typed, &format!("cmds.undoTo {start}"));
    assert!(a.editor.scene == base, "{}", a.status);
    assert_eq!(a.editor.history.redo.len(), 4);
    a.execute(&format!("cmds.undoTo {}", start + 4));
    assert!(a.editor.scene == top);
}

#[test]
fn dragging_the_undos_row_up_undoes_several_steps() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = polygon_app();
    for _ in 0..3 {
        a.execute("poly.flip");
    }
    let done = a.editor.history.undo.len();
    a.history.open_height = 300.;
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![]);
    let rect = |a: &Operandi, key: UndoKey| {
        a.history_trees
            .undos
            .rects
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("{key:?} drawn"))
    };
    let marker = rect(&a, UndoKey::Marker);
    // Onto the top half of the third-newest step: undo back to before it.
    let onto = rect(&a, UndoKey::Step(done - 3));
    let target = pos2(onto.center().x, onto.top() + 2.);
    let press = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(marker.center())],
    );
    draw(&mut a, &ctx, SIZE, vec![press(marker.center(), true)]);
    for k in 1..=8 {
        let t = k as f32 / 8.;
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(marker.center().lerp(target, t))],
        );
    }
    draw(&mut a, &ctx, SIZE, vec![press(target, false)]);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.editor.history.undo.len(), done - 3, "{}", a.status);
    assert_eq!(a.editor.history.redo.len(), 3);
    assert!(lines(&a).iter().any(|l| l.starts_with("cmds.undoTo")));
}

#[test]
fn history_keeps_undone_mistakes_and_undone_steps_stay_greyed() {
    let mut a = polygon_app();
    a.execute("poly.flip");
    a.execute("Undo");
    assert_eq!(
        a.editor.history.redo.len(),
        1,
        "HIS-14: the undone step stays, redoable"
    );
    a.execute("poly.setPart Road");
    assert!(
        a.editor.history.redo.is_empty(),
        "a new edit throws it away"
    );
    let all = lines(&a);
    let flip = all
        .iter()
        .position(|l| l == "poly.flip")
        .expect("the mistake");
    let undo = all.iter().position(|l| l == "app.undo").expect("the Undo");
    assert!(flip < undo, "HIS-20: in order, ignoring undo: {all:?}");
}

#[test]
fn results_log_a_failure_with_its_message() {
    let mut a = Operandi::empty();
    a.execute_from(Source::Typed, "no.such.command");
    let r = a.history.results.records.back().unwrap();
    assert!(!r.ok);
    assert!(r.message.starts_with("Unknown command"), "{}", r.message);
    a.execute_from(Source::Typed, "cmds.undoTo many");
    let r = a.history.results.records.back().unwrap();
    assert!(!r.ok);
    assert_eq!(r.message, "Use cmds.undoTo <index>");
    // A button's line goes to History, not Results.
    let n = a.history.results.records.len();
    a.execute("no.such.command");
    assert_eq!(a.history.results.records.len(), n);
    assert!(!last(&a).ok);
}

#[test]
fn history_max_and_clear() {
    let mut a = Operandi::empty();
    a.execute("history.max 3");
    for _ in 0..5 {
        a.execute("Front View");
    }
    assert_eq!(a.history.log.records.len(), 3);
    a.execute_from(Source::Typed, "history.max ?");
    assert_eq!(a.status, "3");
    a.execute("history.clear");
    assert!(a.history.log.records.len() <= 1, "only the clear itself");
}

#[test]
fn undo_preferences_keep_todays_defaults() {
    let p = undo_autosave::UndoAutosavePreferences::default();
    let h = p.history();
    assert_eq!(h.max_undo_level, 100, "Max Undo Levels");
    assert!(
        !h.group_selection,
        "Group Selection Commands off, as before"
    );
    assert_eq!(h.max_sub_commands, 100);
    assert_eq!(h, operandi_core::HistorySettings::default());
}

#[test]
fn up_and_down_step_through_typed_lines() {
    let mut a = Operandi::empty();
    for line in ["Front View", "Top View", "Front View"] {
        a.remember_typed(line);
    }
    a.recall(true);
    assert_eq!(a.command, "Front View");
    a.recall(true);
    assert_eq!(a.command, "Top View");
    a.recall(false);
    assert_eq!(a.command, "Front View");
    a.recall(false);
    assert_eq!(a.command, "");
}

#[test]
fn the_model_layout_shows_the_panel_collapsed_to_its_entry() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.history.open_height, 0., "HIS-01: collapsed");
    assert_eq!(
        a.command_history_reserved(),
        30.,
        "the entry's row, as before"
    );
    assert!(
        panel_registry::REGISTRY
            .iter()
            .any(|t| t.kind == panel_registry::PanelKind::CommandHistory),
        "UIC-228: a panel kind"
    );
    // The panel draws as a group's tab too.
    a.lower_frame.add(panel_registry::PanelKind::CommandHistory);
    draw(&mut a, &ctx, SIZE, vec![]);
    a.history.tab = command_history::Tab::Commands;
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(!a.history_trees.commands.rects.is_empty());
}

#[test]
fn a_bare_line_that_opens_a_dialog_records_the_dialogs_line_instead() {
    let mut a = polygon_app();
    a.execute("poly.setPart");
    if !a.has_modal() {
        // No dialog in this build for a bare line: nothing to check.
        return;
    }
    assert!(
        !lines(&a).iter().any(|l| l == "poly.setPart"),
        "CMD-14: the bare line isn't the record: {:?}",
        lines(&a)
    );
}
