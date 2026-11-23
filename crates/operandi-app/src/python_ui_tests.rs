//! Python scripting in the app: the switch hides the console and menu
//! entries; scripts run between frames through the typed-line path, as
//! one undo step, with their output and errors in the Script Console.
use super::*;
use crate::python_ui::Line;
use crate::tests::{box_app, draw};

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

fn frames(a: &mut Operandi, ctx: &Context, n: usize) {
    for _ in 0..n {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
}

fn system_lines(a: &Operandi, ctx: &Context) -> Vec<String> {
    let model = a.menu_model(ctx);
    let Some(system) = model.iter().find(|m| m.title == "System") else {
        return Vec::new();
    };
    system
        .entries
        .iter()
        .filter_map(|e| match e {
            crate::app_menu::Entry::Item(crate::app_menu::Item {
                action: crate::app_menu::Action::Line(n),
                ..
            }) => crate::app_menu::action_line(*n),
            _ => None,
        })
        .collect()
}

#[test]
fn the_python_switch_off_hides_the_console_and_menu_entries() {
    crate::python_switch::set_for_test(false);
    let ctx = Context::default();
    let mut a = box_app();
    frames(&mut a, &ctx, 1);
    assert!(
        !system_lines(&a, &ctx)
            .iter()
            .any(|l| l.starts_with("operandi.python."))
    );
    a.execute("operandi.python.console");
    assert!(!a.python.open);
    assert!(a.status.contains("Python scripting is off"), "{}", a.status);
    a.execute("operandi.python.run code:{print(1)}");
    assert!(!a.python.running());
    // The bridge's python_run is refused too.
    let answer = a
        .mcp_answer(
            &serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {"name": "python_run", "arguments": {"code": "print(1)"}}})
            .to_string(),
        )
        .unwrap();
    assert_eq!(answer["result"]["isError"], true);
    crate::python_switch::set_for_test(true);
}

#[test]
fn the_python_switch_on_shows_the_console_and_menu_entries() {
    crate::python_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    let lines = system_lines(&a, &ctx);
    assert!(
        lines.iter().any(|l| l == "operandi.python.console"),
        "{lines:?}"
    );
    assert!(lines.iter().any(|l| l == "operandi.python.runScript"));
    a.execute("operandi.python.console");
    assert!(a.python.open);
    a.python
        .log
        .push(Line::Error("Traceback: a red line".into()));
    a.python.log.push(Line::Output("an output line".into()));
    frames(&mut a, &ctx, 2);
    // The console keeps its place like other floating windows (UIC-183).
    assert!(a.palettes.places.contains_key("scriptConsole"));
    assert!(a.scripts_folder().ends_with("Scripts"));
}

/// One Python thread runs one script at a time; these tests take turns.
#[cfg(feature = "python")]
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Runs a script from the command line and draws frames until it ends.
#[cfg(feature = "python")]
fn run_script(a: &mut Operandi, ctx: &Context, code: &str) -> operandi_python::Outcome {
    a.python.last = None;
    a.execute(&format!("operandi.python.run code:{{{code}}}"));
    for _ in 0..500 {
        frames(a, ctx, 1);
        if let Some(o) = a.python.last.take() {
            return o;
        }
    }
    panic!("the script didn't finish: {} {:?}", a.status, a.python.log);
}

#[cfg(feature = "python")]
#[test]
fn a_script_runs_typed_lines_as_one_undo_step() {
    let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
    crate::python_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    a.run(Command::Mode(Mode::Item));
    let id = a.editor.scene.active().unwrap().id;
    let before = a.editor.undo.len();
    let out = run_script(
        &mut a,
        &ctx,
        &format!(
            "import operandi\nfor y in (1, 2, 3):\n    operandi.set_channel({id}, 'pos.Y', y)\nprint(operandi.run('no.such.thing', check=False))"
        ),
    );
    assert!(out.ok, "{:?}", out.error);
    assert_eq!(out.output, "False\n");
    assert_eq!(a.editor.scene.item(id).unwrap().transform.translation.y, 3.);
    assert_eq!(a.editor.undo.len(), before + 1, "one step (UIC-227)");
    a.execute("app.undo");
    assert_eq!(a.editor.scene.item(id).unwrap().transform.translation.y, 0.);
}

#[cfg(feature = "python")]
#[test]
fn a_script_error_shows_in_the_console_and_the_app_carries_on() {
    let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
    crate::python_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    let scene = a.editor.scene.clone();
    let out = run_script(&mut a, &ctx, "print('hi')\nraise ValueError('nope')");
    assert!(!out.ok);
    assert!(
        a.python
            .log
            .iter()
            .any(|l| matches!(l, Line::Error(t) if t.contains("ValueError: nope")))
    );
    assert!(a.status.starts_with("Script failed"), "{}", a.status);
    assert_eq!(a.editor.scene, scene);
    // A script can't start another one.
    let out = run_script(
        &mut a,
        &ctx,
        "import operandi\noperandi.run('operandi.python.run code:{1}')",
    );
    assert!(
        out.error
            .is_some_and(|t| t.contains("can't start another script"))
    );
    let out = run_script(&mut a, &ctx, "print(6 * 7)");
    assert_eq!(out.output, "42\n");
}
