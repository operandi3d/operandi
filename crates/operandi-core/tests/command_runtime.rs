//! The run-time command registry (architecture review 2026-11-14, pick E):
//! built from the built-in table, host handlers attach to built-ins, the
//! host adds window-only commands, and kits register under their own
//! namespace without touching built-in names.
use operandi_core::command_registry::{self as table, Area, COMMANDS, jopt, script};
use operandi_core::command_runtime::{Call, Origin, Registry};
use operandi_core::{Command, Editor};
use serde_json::{Map, Value, json};

/// A stand-in host: records what ran.
#[derive(Default)]
struct Host {
    ran: Vec<String>,
}

fn record(h: &mut Host, call: &Call<'_>) {
    h.ran.push(format!("{}|{}", call.entry.name, call.rest));
}
fn other(h: &mut Host, _: &Call<'_>) {
    h.ran.push("other".into());
}

fn args(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn built_in_registry_is_the_table() {
    let r: Registry<Host> = Registry::built_in();
    assert_eq!(r.iter().count(), COMMANDS.len());
    for (row, entry) in r.iter().zip(COMMANDS) {
        assert_eq!(row.entry.name, entry.name);
        assert_eq!(row.origin, Origin::BuiltIn);
        assert!(row.host.is_none());
        assert_eq!(row.entry.json.is_some(), entry.flags.headless);
        // Every name finds its own row, as the static lookup does.
        for name in entry.names() {
            assert_eq!(r.find(name).unwrap().entry.name, entry.name, "{name}");
        }
    }
    // Old spellings resolve the same way.
    assert_eq!(
        r.find("item.rename").map(|c| c.entry.name),
        table::find("item.rename").map(|e| e.name)
    );
    assert_eq!(
        r.find_line("spike reset").unwrap().0.entry.name,
        "Spike Reset"
    );
    assert_eq!(r.matching("").len(), table::matching("").len());
}

#[test]
fn host_handlers_attach_once_and_bare_commands_take_only_bare_lines() {
    let mut r: Registry<Host> = Registry::built_in();
    r.handle("Undo", record).unwrap();
    // The same handler again (another name of the same command) is fine.
    r.handle("undo", record).unwrap();
    // A second, different owner is refused.
    assert!(
        r.handle("Undo", other)
            .unwrap_err()
            .contains("already has a handler")
    );
    assert!(r.handle("no.such.command", record).is_err());

    let mut host = Host::default();
    let (run, entry, rest) = r.host_call("  UNDO ").unwrap();
    run(
        &mut host,
        &Call {
            entry: &entry,
            line: "UNDO",
            rest,
        },
    );
    // `Undo` is app.undo's phrase (UIC-165).
    assert_eq!(host.ran, ["app.undo|"]);
    // Undo takes no arguments: a longer line is another parser's.
    assert!(r.host_call("undo twice").is_none());
    // Commands without a host handler give nothing.
    assert!(r.host_call("Redo").is_none());
}

#[test]
fn host_commands_need_the_window_and_take_free_names() {
    let mut r: Registry<Host> = Registry::built_in();
    static FRAMES: &[table::Arg] = &[table::opt(
        "frames",
        table::ArgType::Int,
        table::Unit::None,
        None,
    )];
    let bench = script("operandi.test.bench", Area::Debug, "A host-only command.").args(FRAMES);
    r.add_host_command(bench, record).unwrap();
    let row = r.find("operandi.test.bench").unwrap();
    assert!(row.entry.flags.ui_only, "needs the window");
    assert_eq!(row.origin, Origin::Host);
    let (_, _, rest) = r.host_call("operandi.test.bench 12").unwrap();
    assert_eq!(rest, "12");
    // A taken name is refused.
    let undo = script("Undo", Area::Debug, "x");
    assert!(r.add_host_command(undo, record).is_err());
}

fn kit_editor_handler(
    e: &mut Editor,
    name: &str,
    a: &Map<String, Value>,
) -> Result<Option<Command>, String> {
    assert_eq!(name, "mykit.rename");
    let to = a
        .get("name")
        .and_then(Value::as_str)
        .ok_or("name is a string")?;
    let _ = e;
    Ok(Some(Command::Rename(to.into())))
}

#[test]
fn kits_register_inside_their_namespace_only() {
    let mut r: Registry<Host> = Registry::built_in();
    let ok = script("mykit.doThing", Area::Modeling, "A kit command.");
    r.register("mykit", ok, Some(record)).unwrap();
    assert_eq!(
        r.find("MYKIT.DOTHING").unwrap().origin,
        Origin::Kit("mykit".into())
    );
    // The same name twice.
    assert!(r.register("mykit", ok, Some(record)).is_err());
    // Built-in namespaces, and ones kept for Operandi.
    for space in ["poly", "tool", "item", "operandi", "app", "Select"] {
        let e = script("x.y", Area::Modeling, "x");
        assert!(
            r.register(space, e, Some(record))
                .unwrap_err()
                .contains("belongs to Operandi"),
            "{space}"
        );
    }
    // Names outside the namespace, aliases included.
    let outside = script("other.thing", Area::Modeling, "x");
    assert!(r.register("mykit", outside, Some(record)).is_err());
    let alias = script("mykit.two", Area::Modeling, "x").aliases(&["poly.flip"]);
    assert!(r.register("mykit", alias, Some(record)).is_err());
    assert!(r.find("mykit.two").is_none(), "a refusal adds nothing");
    // Phrases, bad namespaces, nothing to run.
    let phrase = table::action("mykit.phrase", Area::Modeling, "x");
    assert!(r.register("mykit", phrase, Some(record)).is_err());
    for bad in ["", "9kit", "my-kit", "my.kit"] {
        let e = script("mykit.z", Area::Modeling, "x");
        assert!(r.register(bad, e, Some(record)).is_err(), "{bad:?}");
    }
    let idle = script("mykit.idle", Area::Modeling, "x");
    assert!(r.register("mykit", idle, None).is_err());
    // Built-ins are untouched.
    assert_eq!(r.find("poly.flip").unwrap().origin, Origin::BuiltIn);
}

#[test]
fn a_kit_headless_command_runs_on_the_editor() {
    let mut r: Registry<Host> = Registry::built_in();
    static ARGS: &[table::JsonArg] = &[jopt("name", "string", "New name.")];
    let entry = script("mykit.rename", Area::Items, "Renames the active item.").on_editor(
        "Renames the active item.",
        ARGS,
        kit_editor_handler,
    );
    r.register("mykit", entry, None).unwrap();
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let command = r
        .command_for(&mut e, "mykit.rename", &args(json!({"name": "Kit Mesh"})))
        .unwrap()
        .unwrap();
    e.dispatch(command).unwrap();
    assert_eq!(e.scene.active().unwrap().name, "Kit Mesh");
    // Its arguments are checked like a built-in's.
    let wrong = r.command_for(&mut e, "mykit.rename", &args(json!({"title": "x"})));
    assert!(wrong.err().unwrap().contains("no argument \"title\""));
    // Built-ins run through the same lookup.
    r.command_for(&mut e, "mesh.new", &args(json!({"name": "Second"})))
        .unwrap();
    assert_eq!(e.scene.active().unwrap().name, "Second");
    assert!(r.command_for(&mut e, "mykit.nothing", &Map::new()).is_err());
}

/// A bare-only handler leaves a phrase's arguments to the host's other
/// parsers (`Bridge` starts the tool; `bridge segments 3` is the attribute
/// parser's), even though the entry lists those arguments.
#[test]
fn bare_only_handlers_leave_arguments_to_other_parsers() {
    let mut r: Registry<Host> = Registry::built_in();
    r.handle_bare("Bridge", record).unwrap();
    assert!(r.host_call("bridge").is_some());
    assert!(r.host_call("bridge segments 3").is_none());
    // Attaching it again with words is a different owner.
    assert!(r.handle("Bridge", record).is_err());
    assert!(r.find("Bridge").unwrap().bare_only);
    // A handler that reads words gets them, and a required choice must be
    // one of the choices.
    r.handle("Symmetry", record).unwrap();
    assert_eq!(r.host_call("symmetry x").unwrap().2, "x");
    assert!(r.host_call("symmetry sideways").is_none());
    assert!(r.host_call("symmetry").is_none(), "its axis is required");
}

/// UIC-223: a command answers its own greying and lit state; a line that
/// names no command has no state (UIC-221: the UI shows it red).
#[test]
fn commands_answer_their_own_state() {
    use operandi_core::command_runtime::CommandState;
    fn busy(h: &Host, _: &Call<'_>) -> CommandState {
        CommandState::unless((!h.ran.is_empty()).then_some("Busy"))
    }
    fn lit(_: &Host, call: &Call<'_>) -> CommandState {
        CommandState::lit(call.rest.ends_with(" on"))
    }
    let mut r: Registry<Host> = Registry::built_in();
    r.answer("Undo", busy).unwrap();
    // The same answer again is fine; a different one is refused.
    r.answer("undo", busy).unwrap();
    assert!(r.answer("Undo", lit).is_err());
    assert!(r.answer("no.such.command", lit).is_err());
    r.answer("tool.set", lit).unwrap();
    let mut host = Host::default();
    assert_eq!(r.state(&host, "Undo"), Some(CommandState::default()));
    host.ran.push("x".into());
    assert_eq!(
        r.state(&host, " Undo ").unwrap().unavailable.as_deref(),
        Some("Busy")
    );
    assert!(r.state(&host, "tool.set prim.cube on").unwrap().on);
    assert!(!r.state(&host, "tool.set prim.cube off").unwrap().on);
    // Without an answer: runnable, not on.
    assert_eq!(r.state(&host, "Redo"), Some(CommandState::default()));
    assert_eq!(r.state(&host, "not a command at all"), None);
}

#[test]
fn host_calls_read_choices_by_name_as_well_as_by_place() {
    let mut r: Registry<Host> = Registry::built_in();
    static ARGS: &[table::Arg] = &[
        table::arg("index", table::ArgType::Int, table::Unit::None),
        table::arg(
            "attr",
            table::ArgType::Choice(&["label", "width"]),
            table::Unit::None,
        ),
        table::arg("value", table::ArgType::Text, table::Unit::None),
    ];
    let set = script("operandi.test.set", Area::Debug, "x").args(ARGS);
    r.add_host_command(set, record).unwrap();
    assert!(r.host_call("operandi.test.set 0 width normal").is_some());
    // A right-click menu's named form (workbench Alignment > Normal).
    assert!(
        r.host_call("operandi.test.set index:0 attr:width value:normal")
            .is_some()
    );
    assert!(
        r.host_call("operandi.test.set value:normal attr:WIDTH index:0")
            .is_some()
    );
    // Not one of the choices, by place or by name: another parser's line.
    assert!(r.host_call("operandi.test.set 0 height 2").is_none());
    assert!(
        r.host_call("operandi.test.set index:0 attr:height value:2")
            .is_none()
    );
}
