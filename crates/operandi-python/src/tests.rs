//! Python scripts run headless on an editor (the Python test scripts of
//! the plan): keys from Python, bulk mesh round trip, undo groups, errors.
use super::*;
use std::sync::Mutex;

/// One Python thread serves one script at a time; tests take turns.
static TURN: Mutex<()> = Mutex::new(());

fn run(e: &mut Editor, code: &str) -> Outcome {
    let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
    run_blocking(&mut EditorHost(e), Source::file_text(code, "<test>"))
}

fn ok(e: &mut Editor, code: &str) -> String {
    let out = run(e, code);
    assert!(out.ok, "script failed:\n{}\n{:?}", out.output, out.error);
    out.output
}

/// An editor with a one-triangle mesh item, active.
fn triangle() -> (Editor, u64) {
    let mut e = Editor::default();
    operandi_core::command_line::run_line(&mut e, "mesh.new")
        .expect("mesh.new is headless")
        .expect("mesh.new");
    let id = e.scene.selection.item.expect("the new mesh is active");
    e.set_mesh_data(
        None,
        vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        Some(vec![vec![0, 1, 2]]),
    )
    .unwrap();
    (e, id)
}

#[test]
fn print_and_python_values_come_back() {
    let mut e = Editor::default();
    let out = ok(
        &mut e,
        "print('hello', 6 * 7)\nimport sys\nprint('to stderr', file=sys.stderr)",
    );
    assert_eq!(out, "hello 42\nto stderr\n");
    assert!(version().is_some_and(|v| v.starts_with('3')));
}

#[test]
fn a_python_error_ends_the_script_with_its_traceback_and_python_keeps_working() {
    let mut e = Editor::default();
    let out = run(&mut e, "print('before')\n1 / 0\nprint('after')");
    assert!(!out.ok);
    assert_eq!(out.output, "before\n");
    let tb = out.error.expect("a traceback");
    assert!(tb.contains("ZeroDivisionError"), "{tb}");
    assert!(tb.contains("<test>"), "{tb}");
    // A syntax error too, and the next script runs as usual.
    let out = run(&mut e, "def broken(:\n");
    assert!(out.error.is_some_and(|t| t.contains("SyntaxError")));
    assert_eq!(ok(&mut e, "print('fine')"), "fine\n");
}

#[test]
fn run_reports_the_commands_own_result() {
    let (mut e, _) = triangle();
    let out = ok(
        &mut e,
        "import operandi
print(operandi.run('operandi.time.set frame:12'))
print(operandi.run('operandi.time.set', frame=6))
print(operandi.run('no.such.command', check=False))
try:
    operandi.run('operandi.time.set frame:nonsense')
except operandi.CommandError as err:
    print('refused:', err)
",
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "True");
    assert_eq!(lines[1], "True");
    assert_eq!(lines[2], "False");
    assert!(lines[3].starts_with("refused: "), "{out}");
    assert_eq!(
        operandi_core::animation::frames(e.scene.time.rate, e.scene.time.current),
        6.
    );
}

#[test]
fn key_a_bouncing_ball_from_python_and_read_it_back() {
    let (mut e, id) = triangle();
    let undo_before = e.undo.len();
    let out = ok(
        &mut e,
        &format!(
            "import operandi
ball = {id}
for frame, y in [(0, 2.0), (12, 0.0), (24, 2.0)]:
    operandi.set_time(frame)
    operandi.set_channel(ball, 'pos.Y', y)
    operandi.set_key(ball, 'pos.Y')
q = operandi.query('operandi.anim.query', frames='0,6,12,24')
channel = [c for c in q['channels'] if c['channel'] == f'{{ball}}:pos.Y'][0]
print([k['frame'] for k in channel['keys']])
print([round(channel['values'][f], 4) for f in ('0', '6', '12', '24')])
"
        ),
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "[0.0, 12.0, 24.0]", "{out}");
    // At 0, 12 and 24 the keys; 6 lies between 2 and 0.
    let values = lines[1];
    assert!(
        values.starts_with("[2.0, ") && values.ends_with(", 0.0, 2.0]"),
        "{values}"
    );
    // The whole script is one undo step (UIC-227).
    assert_eq!(e.undo.len(), undo_before + 1);
    assert_eq!(e.undo.last().unwrap().label, "Script <test>");
}

#[test]
fn undo_group_makes_one_step() {
    let (mut e, id) = triangle();
    let before = e.undo.len();
    let y0 = e.scene.item(id).unwrap().transform.translation.y;
    ok(
        &mut e,
        &format!(
            "import operandi
with operandi.undo_group('Raise three times'):
    for y in (1, 2, 3):
        operandi.set_channel({id}, 'pos.Y', y)
"
        ),
    );
    assert_eq!(e.undo.len(), before + 1, "one step");
    assert_eq!(e.undo.last().unwrap().label, "Raise three times");
    assert_eq!(e.scene.item(id).unwrap().transform.translation.y, 3.);
    e.dispatch(operandi_core::Command::Undo).unwrap();
    assert_eq!(e.scene.item(id).unwrap().transform.translation.y, y0);
}

#[test]
fn an_error_inside_an_undo_group_still_closes_it() {
    let (mut e, id) = triangle();
    let before = e.undo.len();
    let out = run(
        &mut e,
        &format!(
            "import operandi
with operandi.undo_group('Half'):
    operandi.set_channel({id}, 'pos.X', 1)
    operandi.set_channel({id}, 'pos.X', 2)
    raise ValueError('stop here')
"
        ),
    );
    assert!(out.error.is_some_and(|t| t.contains("stop here")));
    assert_eq!(e.undo.len(), before + 1);
}

#[test]
fn mesh_bulk_round_trip_is_one_undo_step() {
    let (mut e, id) = triangle();
    let before = e.undo.len();
    let out = ok(
        &mut e,
        "import operandi, array
m = operandi.mesh_get()
print(len(m['points']), m['polygons'])
# A quad, as lists.
operandi.mesh_set([(0, 0, 0), (2, 0, 0), (2, 2, 0), (0, 2, 0)], [[0, 1, 2, 3]])
# Move the points through a flat buffer of doubles.
flat = operandi.mesh_get(flat=True)['points']
moved = array.array('d', [v + 1 for v in flat])
operandi.mesh_set(moved)
m = operandi.mesh_get()
print(m['points'][2], m['polygons'])
try:
    operandi.mesh_set([(0, 0, 0)], [[0, 1, 2]])
except operandi.CommandError as err:
    print('refused')
",
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "3 [[0, 1, 2]]");
    assert_eq!(lines[1], "(3.0, 3.0, 1.0) [[0, 1, 2, 3]]");
    assert_eq!(lines[2], "refused");
    // Two writes, folded with the rest of the script (UIC-227).
    assert_eq!(e.undo.len(), before + 1);
    let mesh = e.scene.item(id).unwrap().mesh.open();
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.faces.len(), 1);
}

#[test]
fn stop_interrupts_a_long_script() {
    let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
    let mut e = Editor::default();
    let mut script = Script::start(Source::file_text(
        "print('start')\nwhile True:\n    pass\n",
        "<loop>",
    ))
    .unwrap();
    let mut host = EditorHost(&mut e);
    assert!(script.pump(&mut host, Duration::from_millis(200)).is_none());
    script.stop();
    let outcome = loop {
        if let Some(o) = script.pump(&mut host, Duration::from_millis(100)) {
            break o;
        }
    };
    assert!(!outcome.ok);
    assert!(
        outcome
            .error
            .is_some_and(|t| t.contains("KeyboardInterrupt"))
    );
    // The next script isn't hit by a stale Stop.
    drop(_turn);
    assert_eq!(ok(&mut e, "print('next')"), "next\n");
}

#[test]
fn one_script_at_a_time() {
    let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
    let mut e = Editor::default();
    let mut first =
        Script::start(Source::file_text("import time\ntime.sleep(0.3)", "<a>")).unwrap();
    let second = Script::start(Source::file_text("pass", "<b>"));
    assert_eq!(second.err().as_deref(), Some("A script is already running"));
    let mut host = EditorHost(&mut e);
    while first.pump(&mut host, Duration::from_millis(100)).is_none() {}
}

#[test]
fn console_lines_echo_values_and_keep_names() {
    let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
    let mut e = Editor::default();
    let mut host = EditorHost(&mut e);
    let a = run_blocking(&mut host, Source::console("x = 20"));
    assert!(a.ok && a.output.is_empty());
    let b = run_blocking(&mut host, Source::console("x + 1"));
    assert_eq!(b.output, "21\n");
}

#[test]
fn query_refuses_a_line_that_is_not_a_query() {
    let (mut e, _) = triangle();
    let out = ok(
        &mut e,
        "import operandi
try:
    operandi.query('operandi.time.set frame:3')
except operandi.CommandError as err:
    print(err)
",
    );
    assert!(out.contains("isn't a query"), "{out}");
    assert_eq!(e.scene.time.current, operandi_core::time_tick::Tick::ZERO);
}
