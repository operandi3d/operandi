//! Menus as forms (menu_forms.rs): live lists, run-time titles, submenus,
//! the in-window bar, and each menu's snapshot.
//!
//! The snapshots in `menu_snapshots/` were written from the hand-written
//! menus before each was converted, so a converted menu is checked
//! identical to its old code: titles with their shortcuts, greying and
//! why, ticks, native keys, order, separators and submenus, in several
//! app states. `OPERANDI_BLESS_MENUS=1` rewrites them (only for a change
//! that is meant, listed in docs/SPECIFICATION_QUESTIONS.md).
use super::*;
use crate::app_menu::{Entry, Menu, same_shape};
use crate::form::command_form::{Control, Form, FormRef, Style};

fn find_menu(a: &Operandi, ctx: &Context, id: &str) -> Menu {
    a.menu_model(ctx)
        .into_iter()
        .find(|m| m.id == id)
        .unwrap_or_else(|| panic!("no {id} menu"))
}

fn submenu<'a>(m: &'a Menu, title: &str) -> &'a Menu {
    m.entries
        .iter()
        .find_map(|e| match e {
            Entry::Submenu(s) if s.title == title => Some(s),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no {title} submenu in {}", m.title))
}

fn command(line: &str, label: &str) -> Control {
    Control {
        command: Some(line.into()),
        label: label.into(),
        ..Control::default()
    }
}

/// A user form for the tests, saved over (or beside) the shipped ones.
fn user_form(a: &mut Operandi, id: &str, label: &str, controls: Vec<Control>) {
    a.forms
        .save(Form {
            version: 1,
            id: id.into(),
            label: label.into(),
            style: Style::Menu,
            icons: Default::default(),
            icon_size: Default::default(),
            exported: false,
            show_when: None,
            slot: None,
            columns: None,
            workbench: false,
            controls,
        })
        .unwrap();
}

/// A live list fills a menu at run time: the layouts list follows a new
/// layout, ticks the current one, and each entry runs its own line.
#[test]
fn live_list_fills_at_run_time_and_runs_its_lines() {
    let ctx = Context::default();
    let mut a = Operandi::empty();
    let names = a.layouts.names();
    user_form(
        &mut a,
        "menu.system",
        "System",
        vec![
            command("layout.window hash:formEditor", "Form Editor"),
            Control {
                divider: Some(String::new()),
                ..Control::default()
            },
            Control {
                list: Some("layouts".into()),
                ..Control::default()
            },
        ],
    );
    let menu = find_menu(&a, &ctx, "menu.system");
    let titles: Vec<&str> = menu.leaves().iter().map(|i| i.title.as_str()).collect();
    assert_eq!(titles[0], "Form Editor");
    assert_eq!(
        &titles[1..],
        names.iter().map(String::as_str).collect::<Vec<_>>()
    );
    let current = menu
        .leaves()
        .into_iter()
        .find(|i| i.checked)
        .expect("the current layout is ticked")
        .clone();
    assert_eq!(current.title, a.layouts.current);
    // An unknown source is one greyed entry saying so.
    user_form(
        &mut a,
        "menu.system",
        "System",
        vec![Control {
            list: Some("nowhere".into()),
            ..Control::default()
        }],
    );
    let menu = find_menu(&a, &ctx, "menu.system");
    let leaf = menu.leaves()[0].clone();
    assert!(!leaf.enabled);
    assert_eq!(leaf.why.as_deref(), Some("No live list called nowhere"));
}

/// `{title}` is the entry's own line's answered title; `{title:<line>}`
/// another line's, in an entry or a submenu's label. A line that answers
/// no title shows the line.
#[test]
fn run_time_titles_fill_from_command_answers() {
    let ctx = Context::default();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    user_form(
        &mut a,
        "menu.test.sub",
        "Sub",
        vec![command("app.redo", "{title}")],
    );
    user_form(
        &mut a,
        "menu.system",
        "System",
        vec![
            command("app.undo", "{title}"),
            command(
                "layout.window hash:formEditor",
                "Open {title:app.undo} · {title}",
            ),
            Control {
                form: Some(FormRef::Id("menu.test.sub".into())),
                label: "More: {title:app.redo}".into(),
                ..Control::default()
            },
        ],
    );
    let undo = a.line_state("app.undo").unwrap().title.unwrap();
    assert!(undo.starts_with("Undo"), "{undo}");
    let redo = a.line_state("app.redo").unwrap().title.unwrap();
    let menu = find_menu(&a, &ctx, "menu.system");
    let titles: Vec<String> = menu.leaves().iter().map(|i| i.title.clone()).collect();
    assert_eq!(titles[0], undo);
    assert_eq!(
        titles[1],
        format!("Open {undo} · layout.window hash:formEditor")
    );
    let sub = submenu(&menu, &format!("More: {redo}"));
    assert_eq!(sub.leaves()[0].title, redo);
}

/// A sub-form is a submenu with its own separators; a form's head slot is
/// placed before its controls (LAY-84).
#[test]
fn sub_forms_are_submenus_and_slots_are_placed() {
    let ctx = Context::default();
    let mut a = Operandi::empty();
    user_form(
        &mut a,
        "menu.test.inner",
        "Inner",
        vec![
            command("layout.window hash:formEditor", "One"),
            Control {
                divider: Some(String::new()),
                ..Control::default()
            },
            command("layout.window hash:formEditor", "Two"),
        ],
    );
    user_form(
        &mut a,
        "menu.system",
        "System",
        vec![Control {
            form: Some(FormRef::Id("menu.test.inner".into())),
            ..Control::default()
        }],
    );
    let mut head = a.forms.get("menu.test.inner").unwrap().clone();
    head.id = "menu.test.head".into();
    head.controls = vec![command("layout.window hash:formEditor", "Zero")];
    head.slot = Some(crate::form::command_form::Slot {
        form: "menu.test.inner".into(),
        at: crate::form::command_form::End::Head,
        sort: 0,
    });
    a.forms.save(head).unwrap();
    let menu = find_menu(&a, &ctx, "menu.system");
    let inner = submenu(&menu, "Inner");
    assert_eq!(inner.id, "menu.test.inner");
    assert!(matches!(inner.entries[2], Entry::Separator));
    let titles: Vec<&str> = inner.leaves().iter().map(|i| i.title.as_str()).collect();
    assert_eq!(titles, ["Zero", "One", "Two"]);
}

/// The native menu updates in place while the shape holds, and rebuilds
/// when a live list or an edited form changes it.
#[test]
fn shape_changes_with_lists_and_submenu_titles_only() {
    let ctx = Context::default();
    let mut a = Operandi::empty();
    let before = a.menu_model(&ctx);
    a.run(Command::NewMesh);
    assert!(
        same_shape(&before, &a.menu_model(&ctx)),
        "ticks and greying only"
    );
    a.execute("operandi.layout.create Another copy:true");
    let grown = a.menu_model(&ctx);
    assert!(!same_shape(&before, &grown), "the layouts list grew");
    let mut renamed = grown.clone();
    renamed[0].title.push('!');
    assert!(!same_shape(&grown, &renamed));
}

/// The Form Editor shows a live list as one item, named by its source.
#[test]
fn form_editor_names_a_live_list_as_one_item() {
    let list = Control {
        list: Some("recentProjects".into()),
        ..Control::default()
    };
    assert_eq!(
        crate::workbench::control_name(&list),
        "Recent Projects (live)"
    );
    assert_eq!(
        crate::menu_forms::list_title("palettes"),
        "Windows/Palettes"
    );
    // A control is exactly one kind.
    let both = Control {
        list: Some("layouts".into()),
        command: Some("app.undo".into()),
        ..Control::default()
    };
    let form = Form {
        version: 1,
        id: "t".into(),
        label: String::new(),
        style: Style::Menu,
        icons: Default::default(),
        icon_size: Default::default(),
        exported: false,
        show_when: None,
        slot: None,
        columns: None,
        workbench: false,
        controls: vec![both],
    };
    assert!(form.check().is_err());
}

// ------------------------------------------------------------ snapshots

/// The states each menu is snapshotted in.
pub(crate) fn snapshot_states() -> Vec<(&'static str, Operandi)> {
    let mesh = || {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::new(2., 3., 4.), DVec3::ONE, 0)
                .unwrap()
                .into();
        a.editor.synchronize_identities().unwrap();
        a.run(Command::Mode(Mode::Polygon));
        a
    };
    let empty = Operandi::empty();
    let mut scenes = mesh();
    scenes.execute("scene.new");
    scenes.switch_scene(0);
    let mut tool = mesh();
    tool.activate(Tool::Move);
    let mut item = mesh();
    item.run(Command::Mode(Mode::Item));
    let mut session = mesh();
    session.execute("Ellipsoid");
    let mut modal = mesh();
    modal.show_bindings = true;
    let mut bound = mesh();
    for (action, chord) in [
        ("Edges", "Ctrl+2"),
        ("Move", "Ctrl+M"),
        ("Frame All", "Shift+A"),
        ("Undo", "Ctrl+U"),
        ("Toggle Palettes", "Ctrl+9"),
    ] {
        bound.bindings.insert(action.into(), chord.into());
    }
    vec![
        ("empty", empty),
        ("mesh", mesh()),
        ("scenes", scenes),
        ("tool", tool),
        ("item", item),
        ("session", session),
        ("modal", modal),
        ("bound", bound),
    ]
}

fn snapshot_entries(entries: &[Entry], depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    for e in entries {
        match e {
            Entry::Separator => out.push_str(&format!("{pad}---\n")),
            Entry::Submenu(m) => {
                out.push_str(&format!("{pad}> {}\n", m.title));
                snapshot_entries(&m.entries, depth + 1, out);
            }
            Entry::Item(i) => {
                let tick = if i.checked { "[x] " } else { "" };
                let grey = match &i.why {
                    Some(why) => format!(" (greyed: {why})"),
                    None if !i.enabled => " (greyed)".into(),
                    None => String::new(),
                };
                let key = if i.command_key.is_empty() {
                    String::new()
                } else {
                    format!(" {{key {}}}", i.command_key)
                };
                // Linux shows Cmd as Ctrl: the snapshot does on every
                // platform, so CI on Linux checks the same text.
                let title = i
                    .title
                    .replace("Cmd+", "Ctrl+")
                    .replace("Ctrl+Ctrl+", "Ctrl+");
                out.push_str(&format!("{pad}{tick}{title}{grey}{key}\n"));
            }
        }
    }
}

/// Menu `id`'s snapshot in every state.
pub(crate) fn snapshot(id: &str) -> String {
    let ctx = Context::default();
    let mut out = String::new();
    for (state, a) in snapshot_states() {
        // The snapshots are of default builds: a painting build's own
        // entries are checked in painting_tests.rs.
        #[cfg(feature = "painting")]
        let a = {
            let mut a = a;
            a.painting.hide_menus = true;
            a
        };
        let menu = find_menu(&a, &ctx, id);
        out.push_str(&format!("## {state}: {}\n", menu.title));
        snapshot_entries(&menu.entries, 0, &mut out);
    }
    out
}

/// Checks (or, with OPERANDI_BLESS_MENUS=1, writes) menu `id`'s snapshot.
fn check_snapshot(id: &str, golden: &str) {
    let now = snapshot(id);
    // Snapshots are taken on macOS; where the MCP bridge has no transport
    // yet (Windows, mcp.rs) its menu entry is greyed with this reason.
    #[cfg(not(unix))]
    let now = now.replace(" (greyed: The MCP bridge needs macOS or Linux)", "");
    if std::env::var_os("OPERANDI_BLESS_MENUS").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/menu_snapshots")
            .join(format!("{id}.txt"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, &now).unwrap();
        return;
    }
    // Snapshots are taken without Cycles; a build with it (merge-check,
    // the nightly run) has the render lines lit instead of greyed.
    let without_cycles = golden.replace(" (greyed: This build has no Cycles renderer)", "");
    let golden = if cfg!(feature = "cycles") {
        without_cycles.as_str()
    } else {
        golden
    };
    if now != golden {
        let diff: Vec<String> = now
            .lines()
            .zip(golden.lines())
            .filter(|(a, b)| a != b)
            .take(8)
            .map(|(a, b)| format!("  now: {a}\n  was: {b}"))
            .collect();
        panic!(
            "{id} differs from its snapshot ({} lines now, {} before):\n{}",
            now.lines().count(),
            golden.lines().count(),
            diff.join("\n")
        );
    }
}

macro_rules! snapshot_test {
    ($name:ident, $id:literal) => {
        #[test]
        fn $name() {
            check_snapshot($id, include_str!(concat!("menu_snapshots/", $id, ".txt")));
        }
    };
}

snapshot_test!(snapshot_file, "menu.file");
snapshot_test!(snapshot_edit, "menu.edit");
snapshot_test!(snapshot_select, "menu.select");
snapshot_test!(snapshot_item, "menu.item");
snapshot_test!(snapshot_geometry, "menu.geometry");
snapshot_test!(snapshot_texture, "menu.texture");
snapshot_test!(snapshot_tools, "menu.tools");
snapshot_test!(snapshot_view, "menu.view");
snapshot_test!(snapshot_layout, "menu.layout");
snapshot_test!(snapshot_animate, "menu.animate");
snapshot_test!(snapshot_render, "menu.render");
snapshot_test!(snapshot_window, "menu.window");
snapshot_test!(snapshot_system, "menu.system");
snapshot_test!(snapshot_help, "menu.help");

/// Every menu form's line names its command exactly: a phrase line ("Frame
/// All") is a whole command name, not a shorter one with words after it
/// ("Split View" would run Split), and every line is known (UIC-221).
#[test]
fn every_menu_line_names_its_command() {
    let a = Operandi::empty();
    let mut bad = Vec::new();
    for id in a.forms.ids() {
        if !id.starts_with("menu.") {
            continue;
        }
        for c in &a.forms.get(id).unwrap().controls {
            let Some(line) = c.command.as_deref() else {
                continue;
            };
            match a.commands.find_line(line) {
                None => bad.push(format!("{id}: {line} isn't a command")),
                Some((cmd, rest)) => {
                    let phrase = !cmd.entry.name.contains('.');
                    if phrase && !rest.is_empty() {
                        bad.push(format!(
                            "{id}: {line} runs {} with \"{rest}\"",
                            cmd.entry.name
                        ));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// File > Export lists every export format in order, each running the
/// scene.saveAs line the old menu ran.
#[test]
fn file_export_lists_every_format() {
    let a = Operandi::empty();
    let lines: Vec<String> = a
        .forms
        .get("menu.file.export")
        .unwrap()
        .controls
        .iter()
        .filter_map(|c| c.command.clone())
        .collect();
    let want: Vec<String> = operandi_core::interchange::ExportFormat::ALL
        .into_iter()
        .map(crate::file_commands::export_line)
        .collect();
    assert_eq!(lines, want);
}
