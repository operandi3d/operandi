//! Pies as forms (pie_forms.rs, UIC-308..313): shipped pies read from
//! their files, user copies and Revert, `operandi.pie.open`, any pie bound
//! in the Input Map (old maps migrated), sub-pies opened in place and
//! refused loops, icons, unknown lines, the centre name, the Form Editor's
//! slot edits, and Export Pie / Import Pie.
use super::*;
use crate::form::command_form::{Control, FormRef, IconMode, Style};
use crate::pie_menu::{self, Outcome};
use crate::tests::{box_app, draw, pointer};

const SIZE: Vec2 = vec2(1200., 800.);
const AT: Pos2 = pos2(600., 400.);

fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    draw(a, ctx, SIZE, events);
}

fn labels(a: &Operandi, id: &str) -> Vec<Option<String>> {
    let pie = a.pie_named(id).unwrap();
    (0..pie_menu::SLOTS)
        .map(|i| pie.slot(i).map(|e| e.label.clone()))
        .collect()
}

fn some(names: &[&str]) -> Vec<Option<String>> {
    names
        .iter()
        .map(|n| (!n.is_empty()).then(|| (*n).to_owned()))
        .collect()
}

/// A user pie `id` called `title` with these slots (line, label); an
/// empty line is an empty slot.
fn user_pie(a: &mut Operandi, id: &str, title: &str, slots: &[(&str, &str)]) {
    a.execute(&format!(
        "operandi.form.new id:{id} label:{{{title}}} style:pie"
    ));
    assert!(a.forms.get(id).is_some(), "{}", a.status);
    for (i, (line, label)) in slots.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        a.execute(&format!(
            "operandi.form.slot index:{i} line:{{{line}}} label:{{{label}}} form:{id}"
        ));
        assert!(!a.status.starts_with("No "), "{}", a.status);
    }
}

/// P1: the seven shipped pies are form files with today's slots, empty
/// slots kept in place; no pie is hard-coded (UIC-308).
#[test]
fn shipped_pies_are_forms_with_their_slots() {
    let a = Operandi::empty();
    for id in pie_menu::ALL {
        let form = a.forms.shipped(id).unwrap_or_else(|| panic!("{id}"));
        assert_eq!(form.style, Style::Pie, "{id}");
        assert_eq!(form.controls.len(), pie_menu::SLOTS, "{id}");
        assert_eq!(form.icons, IconMode::Text, "{id}: labels only, as before");
    }
    assert_eq!(
        labels(&a, pie_menu::VIEW),
        some(&[
            "Top",
            "Perspective",
            "Right",
            "Front",
            "Bottom",
            "Back",
            "Left",
            "Camera"
        ])
    );
    assert_eq!(
        labels(&a, pie_menu::TOGGLES),
        some(&[
            "Lights",
            "Cameras",
            "Wireframe",
            "Vertices",
            "Grid",
            "Background",
            "",
            ""
        ])
    );
    assert_eq!(
        labels(&a, pie_menu::ACTION_CENTERS),
        some(&[
            "Automatic",
            "Selection",
            "Selection Auto Axis",
            "Element",
            "Screen",
            "Origin",
            "Local",
            ""
        ])
    );
    assert_eq!(
        labels(&a, pie_menu::MODELING),
        some(&[
            "Extrude",
            "Bevel",
            "Edge Extend",
            "",
            "Loop Slice",
            "Smooth Shift",
            "Smooth",
            ""
        ])
    );
    // UIC-313: the centre shows the pie's name.
    assert_eq!(a.pie_named(pie_menu::VIEW).unwrap().title, "View Direction");
    assert_eq!(
        a.pie_named(pie_menu::SELECTION).unwrap().title,
        "Selection & Transforms"
    );
    // The Layouts pie stays generated, through the same engine.
    assert_eq!(a.pie_named(pie_menu::LAYOUTS).unwrap().title, "Layouts");
    // Old action names still find their pies.
    assert_eq!(
        a.pie_named("Pie: Shading Style").unwrap().name,
        pie_menu::SHADING
    );
}

/// P1 (UIC-216): a changed copy of a shipped pie wins; Revert brings the
/// shipped one back.
#[test]
fn a_changed_shipped_pie_wins_and_reverts() {
    let mut a = Operandi::empty();
    a.execute("operandi.form.slot index:0 line:{View Left} label:West form:pie.view");
    assert_eq!(labels(&a, pie_menu::VIEW)[0].as_deref(), Some("West"));
    assert!(a.forms.is_changed(pie_menu::VIEW));
    a.execute("operandi.form.revert pie.view");
    assert_eq!(labels(&a, pie_menu::VIEW)[0].as_deref(), Some("Top"));
    assert!(!a.forms.is_changed(pie_menu::VIEW));
}

/// P1: `operandi.pie.open <id>` opens any pie, tapped, waiting for a
/// click; an unknown id says so and opens nothing.
#[test]
fn pie_open_line_opens_a_pie_by_id() {
    let mut a = box_app();
    let ctx = Context::default();
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    a.execute("operandi.pie.open pie.shading");
    frame(&mut a, &ctx, vec![]);
    let open = a.pie.as_ref().expect("the shading pie opens");
    assert_eq!(open.pie.name, pie_menu::SHADING);
    assert!(open.key.is_none(), "typed: tapped, not held");
    a.pie = None;
    a.execute("operandi.pie.open no.such.pie");
    assert_eq!(a.status, "No pie called no.such.pie");
    frame(&mut a, &ctx, vec![]);
    assert!(a.pie.is_none());
    // The old name still works.
    a.execute("Pie: Modeling");
    frame(&mut a, &ctx, vec![]);
    assert_eq!(a.pie.as_ref().unwrap().pie.name, pie_menu::MODELING);
}

/// P1 (UIC-311): a user pie gets an Input Map row named after it, binds
/// to a key with the clash check, and its key opens it held.
#[test]
fn any_pie_binds_in_the_input_map() {
    let mut a = box_app();
    let ctx = Context::default();
    user_pie(
        &mut a,
        "my.views",
        "My Views",
        &[("View Top", "Up"), ("View Front", "Ahead")],
    );
    let line = pie_menu::line("my.views");
    assert!(a.input_map_actions().contains(&line));
    assert_eq!(a.action_label(&line), "Pie: My Views");
    assert_eq!(a.action_label(pie_menu::VIEW_LINE), "Pie: View Direction");
    // Assign Key with a key used elsewhere: the usual clash warning.
    a.execute("operandi.pie.assignKey my.views key:W");
    assert!(a.show_bindings, "Assign Key opens the Input Map");
    let clash = a.input_map_ui.clash.clone().expect("W clashes with Move");
    assert_eq!(clash.others, vec!["Move".to_owned()]);
    // A free key, saved, opens the pie while held.
    a.execute("operandi.pie.assignKey my.views key:Ctrl+Alt+P");
    assert!(a.input_map_ui.clash.is_none(), "{}", a.status);
    a.save_input_map();
    assert_eq!(a.bindings[&line], "Ctrl+Alt+P");
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    let mods = Modifiers::CTRL | Modifiers::ALT;
    frame(&mut a, &ctx, vec![key_event(Key::P, true, mods)]);
    let open = a.pie.as_ref().expect("the key opens the user pie");
    assert_eq!(open.pie.name, "my.views");
    assert!(open.key.is_some(), "held");
    // Hold, point up, release: runs View Top.
    a.camera.view = View::Front;
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT + vec2(0., -80.))]);
    frame(&mut a, &ctx, vec![key_event(Key::P, false, mods)]);
    assert!(a.pie.is_none());
    assert_eq!(a.camera.view, View::Top);
    // Bare Assign Key waits for the keys on the pie's row.
    a.execute("operandi.pie.assignKey pie.view");
    assert_eq!(a.recording_keys_for(), Some(pie_menu::VIEW_LINE));
}

/// P1: a saved input map and a layout's keys that name the old pie
/// actions keep their keys under the new lines.
#[test]
fn old_pie_actions_in_saved_maps_migrate() {
    let dir = std::env::temp_dir().join(format!("operandi-pie-migrate-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("input_map.json");
    std::fs::write(
        &path,
        r#"{"Pie: View Direction":"Ctrl+Alt+V","Pie: Modeling":"Ctrl+Alt+M"}"#,
    )
    .unwrap();
    let loaded = input::load_bindings(&path);
    assert_eq!(loaded[pie_menu::VIEW_LINE], "Ctrl+Alt+V");
    assert_eq!(loaded[pie_menu::MODELING_LINE], "Ctrl+Alt+M");
    assert!(!loaded.contains_key("Pie: View Direction"));
    // Untouched pies keep their factory keys.
    assert_eq!(loaded[pie_menu::TOGGLES_LINE], "Ctrl+1");
    let _ = std::fs::remove_dir_all(&dir);
    let mut keys: BTreeMap<String, String> =
        [("Pie: Viewport Toggles".to_owned(), "Ctrl+Alt+T".to_owned())].into();
    pie_menu::migrate_keys(&mut keys);
    assert_eq!(keys[pie_menu::TOGGLES_LINE], "Ctrl+Alt+T");
}

/// Two user pies: `outer` with View Top at N and a sub-pie slot at E
/// opening `inner` (View Left at N).
fn nested(a: &mut Operandi) {
    user_pie(a, "inner", "Inner", &[("View Left", "Left")]);
    user_pie(a, "outer", "Outer", &[("View Top", "Top")]);
    a.execute("operandi.form.slot index:2 sub:inner form:outer");
    assert!(a.status.contains("slot 2"), "{}", a.status);
}

/// P2 (UIC-310): releasing over a sub-pie slot opens it in place at the
/// same centre with its own name; a slot in it runs; Esc goes back one
/// level, and a click in the centre does too.
#[test]
fn sub_pies_open_in_place_and_go_back() {
    let mut a = box_app();
    let ctx = Context::default();
    nested(&mut a);
    let outer = a.pie_named("outer").unwrap();
    let sub = outer.slot(2).unwrap();
    assert_eq!(sub.sub.as_deref(), Some("inner"));
    assert_eq!(sub.label, "Inner", "a sub-pie slot is named after its pie");
    a.bindings
        .insert(pie_menu::line("outer"), "Ctrl+Alt+O".into());
    let mods = Modifiers::CTRL | Modifiers::ALT;
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    frame(&mut a, &ctx, vec![key_event(Key::O, true, mods)]);
    let center = a.pie.as_ref().unwrap().center;
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT + vec2(90., 0.))]);
    frame(&mut a, &ctx, vec![key_event(Key::O, false, mods)]);
    let open = a
        .pie
        .as_ref()
        .expect("the sub-pie opened instead of closing");
    assert_eq!(open.pie.name, "inner");
    assert_eq!(open.pie.title, "Inner", "UIC-313: its own name");
    assert_eq!(open.center, center, "in place, at the same centre");
    assert_eq!(open.stack.len(), 1);
    // Esc goes back one level, not out.
    frame(
        &mut a,
        &ctx,
        vec![key_event(Key::Escape, true, Modifiers::NONE)],
    );
    assert_eq!(a.pie.as_ref().unwrap().pie.name, "outer");
    // Into the sub-pie again by clicking its slot, then a click in the
    // centre goes back.
    let east = a.pie.as_ref().unwrap().rect(2).center();
    frame(&mut a, &ctx, vec![pointer(east, true, Modifiers::NONE)]);
    frame(&mut a, &ctx, vec![pointer(east, false, Modifiers::NONE)]);
    assert_eq!(a.pie.as_ref().unwrap().pie.name, "inner");
    frame(&mut a, &ctx, vec![pointer(center, true, Modifiers::NONE)]);
    frame(&mut a, &ctx, vec![pointer(center, false, Modifiers::NONE)]);
    assert_eq!(a.pie.as_ref().unwrap().pie.name, "outer");
    // In the sub-pie, a slot runs and everything closes.
    frame(&mut a, &ctx, vec![pointer(east, true, Modifiers::NONE)]);
    frame(&mut a, &ctx, vec![pointer(east, false, Modifiers::NONE)]);
    a.camera.view = View::Front;
    let north = a.pie.as_ref().unwrap().rect(0).center();
    frame(&mut a, &ctx, vec![pointer(north, true, Modifiers::NONE)]);
    frame(&mut a, &ctx, vec![pointer(north, false, Modifiers::NONE)]);
    assert!(a.pie.is_none());
    assert_eq!(a.camera.view, View::Left);
    // Esc in the outermost pie closes it, as always.
    a.execute("operandi.pie.open outer");
    frame(&mut a, &ctx, vec![]);
    frame(
        &mut a,
        &ctx,
        vec![key_event(Key::Escape, true, Modifiers::NONE)],
    );
    assert!(a.pie.is_none());
}

/// P2 (UIC-310): a pie can never contain itself: setting a sub-pie that
/// leads back is refused, and a slot naming a pie already open is greyed.
#[test]
fn a_pie_never_contains_itself() {
    let mut a = Operandi::empty();
    nested(&mut a);
    a.execute("operandi.form.slot index:0 sub:outer form:inner");
    assert!(a.status.contains("can't contain itself"), "{}", a.status);
    a.execute("operandi.form.slot index:0 sub:outer form:outer");
    assert!(a.status.contains("can't contain itself"), "{}", a.status);
    assert!(pie_forms_reach(&a, "outer").contains(&"inner".to_owned()));
    // A loop that came in another way (a hand-edited file): the slot is
    // greyed while its pie is open.
    let mut inner = a.forms.get("inner").unwrap().clone();
    inner.controls[1] = Control {
        form: Some(FormRef::Id("outer".into())),
        ..Default::default()
    };
    a.forms.save(inner).unwrap();
    let inner = a.pie_named("inner").unwrap();
    let back = inner.slot(1).unwrap();
    let status = a.slot_status(back, ["outer", "inner"].into_iter());
    assert!(!status.enabled);
    let mut open = pie_menu::Open::new(inner, AT, Rect::from_min_size(Pos2::ZERO, SIZE), None);
    open.stack.push(a.pie_named("outer").unwrap());
    assert_eq!(open.open_ids().collect::<Vec<_>>(), ["outer", "inner"]);
    // A sub-pie that isn't a pie, or doesn't exist, is refused.
    a.execute("operandi.form.slot index:3 sub:toolbox.uv form:outer");
    assert!(a.status.contains("isn't a pie"), "{}", a.status);
    a.execute("operandi.form.slot index:3 sub:nothing form:outer");
    assert_eq!(a.status, "No pie called nothing");
}

fn pie_forms_reach(a: &Operandi, id: &str) -> Vec<String> {
    crate::pie_forms::reachable(&a.forms, id)
}

/// P2 (UIC-309): a slot shows text, icon or both per the pie's icon
/// mode; one without an icon shows its label whatever the mode; widths
/// make room for the icon and the sub-pie marker.
#[test]
fn slots_show_text_icon_or_both() {
    let mut a = Operandi::empty();
    user_pie(
        &mut a,
        "icons",
        "Icons",
        &[("Move", "Move"), ("Rotate", "Rotate")],
    );
    a.execute("operandi.form.set attr:icon value:action-center index:0 form:icons");
    let both = a.pie_named("icons").unwrap();
    assert_eq!(both.icons, IconMode::Both, "a new pie shows both");
    assert!(matches!(both.shows(0), (Some(_), true)));
    assert_eq!(both.shows(1), (None, true), "no icon: the label");
    a.execute("operandi.form.set attr:icons value:icon form:icons");
    let icon = a.pie_named("icons").unwrap();
    assert!(matches!(icon.shows(0), (Some(_), false)));
    assert_eq!(icon.shows(1), (None, true));
    a.execute("operandi.form.set attr:icons value:text form:icons");
    assert_eq!(a.pie_named("icons").unwrap().shows(0), (None, true));
    assert!(
        pie_menu::slot_width(Some(100.), true, false, false)
            > pie_menu::slot_width(Some(100.), false, false, false)
    );
    assert!(
        pie_menu::slot_width(Some(100.), false, false, true)
            > pie_menu::slot_width(Some(100.), false, false, false)
    );
}

/// P2 (UIC-221/222): a slot holds any line; one no command has is red,
/// greyed with why, and doing nothing; an `@macro` line is accepted.
#[test]
fn unknown_lines_show_red_and_do_nothing() {
    let mut a = box_app();
    let ctx = Context::default();
    user_pie(
        &mut a,
        "odd",
        "Odd",
        &[
            ("no.such.command 1", "Nope"),
            ("@myMacro.py", "Macro"),
            ("tool.set xfrm.smooth on", "Smooth"),
        ],
    );
    let pie = a.pie_named("odd").unwrap();
    let nope = a.pie_status(&pie.slot(0).unwrap().command);
    assert!(nope.unknown && !nope.enabled);
    assert!(
        a.pie_unavailable(&pie.slot(0).unwrap().command)
            .unwrap()
            .starts_with("Unknown command")
    );
    assert!(
        !a.pie_status(&pie.slot(1).unwrap().command).unknown,
        "@macro lines are accepted"
    );
    let tool = a.pie_status(&pie.slot(2).unwrap().command);
    assert!(tool.enabled && !tool.unknown, "any registry line");
    // Releasing over the unknown slot closes with nothing run.
    a.bindings
        .insert(pie_menu::line("odd"), "Ctrl+Alt+D".into());
    let mods = Modifiers::CTRL | Modifiers::ALT;
    let scene = a.editor.scene.clone();
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    frame(&mut a, &ctx, vec![key_event(Key::D, true, mods)]);
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT + vec2(0., -80.))]);
    frame(&mut a, &ctx, vec![key_event(Key::D, false, mods)]);
    assert!(a.pie.is_none());
    assert_eq!(a.editor.scene, scene);
    // The registry line runs from its slot.
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    frame(&mut a, &ctx, vec![key_event(Key::D, true, mods)]);
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT + vec2(120., 0.))]);
    frame(&mut a, &ctx, vec![key_event(Key::D, false, mods)]);
    assert!(a.editor.session.live.is_some(), "{}", a.status);
}

/// The pie widget's own steps for a sub-pie: `enter` keeps the centre,
/// `back` restores, Esc maps to Back only inside a sub-pie.
#[test]
fn open_enter_and_back_keep_the_centre() {
    let a = Operandi::empty();
    let screen = Rect::from_min_size(Pos2::ZERO, SIZE);
    let mut open = pie_menu::Open::new(
        a.pie_named(pie_menu::VIEW).unwrap(),
        AT,
        screen,
        Some((Key::Space, None)),
    );
    let center = open.center;
    open.enter(
        a.pie_named(pie_menu::SHADING).unwrap(),
        [120.; pie_menu::SLOTS],
    );
    assert_eq!(open.center, center);
    assert!(open.key.is_none());
    assert_eq!(open.click(center, |_| true), Outcome::Back);
    assert!(open.back(|_| [120.; pie_menu::SLOTS]));
    assert_eq!(open.pie.name, pie_menu::VIEW);
    assert!(!open.back(|_| [120.; pie_menu::SLOTS]), "already outermost");
    assert_eq!(open.click(center, |_| true), Outcome::Close);
}

/// P3: New Pie makes eight empty slots; slots set, emptied and swapped
/// keep every other slot in place; each edit is one Undo Edit step.
#[test]
fn form_editor_slot_edits_keep_their_places() {
    let mut a = Operandi::empty();
    a.execute("operandi.form.new id:my.pie label:{Mine} style:pie");
    let form = a.forms.get("my.pie").unwrap().clone();
    assert_eq!(form.style, Style::Pie);
    assert_eq!(form.controls.len(), pie_menu::SLOTS);
    assert!(form.controls.iter().all(|c| c.space.is_some()));
    assert_eq!(a.form_group("my.pie"), "Pies");
    a.execute("operandi.form.slot index:4 line:{View Bottom} label:Down form:my.pie");
    a.execute("operandi.form.slot index:0 line:{View Top} form:my.pie");
    assert_eq!(
        labels(&a, "my.pie"),
        some(&["View Top", "", "", "", "Down", "", "", ""])
    );
    a.execute("operandi.form.swap a:0 b:4 form:my.pie");
    assert_eq!(
        labels(&a, "my.pie"),
        some(&["Down", "", "", "", "View Top", "", "", ""])
    );
    a.execute("operandi.form.slot index:0 form:my.pie");
    assert_eq!(
        labels(&a, "my.pie"),
        some(&["", "", "", "", "View Top", "", "", ""])
    );
    // Undo Edit takes the last edit back.
    a.execute("operandi.workbench.undo form:my.pie");
    assert_eq!(
        labels(&a, "my.pie"),
        some(&["Down", "", "", "", "View Top", "", "", ""])
    );
    // Out of range, or not a pie: refused.
    a.execute("operandi.form.slot index:8 line:Move form:my.pie");
    assert!(a.status.contains("8 slots"), "{}", a.status);
    a.execute("operandi.form.slot index:0 line:Move form:toolbox.uv");
    assert!(a.status.contains("isn't a pie"), "{}", a.status);
    // A user pie's Revert deletes it.
    a.execute("operandi.form.revert my.pie");
    assert!(a.pie_named("my.pie").is_none());
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "operandi-pie-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// P4 (UIC-312): Export Pie writes one file with the pie and every
/// sub-pie it reaches; Import Pie reads it back into another session.
#[test]
fn export_and_import_round_trip_with_sub_pies() {
    let dir = temp_dir("round-trip");
    let file = dir.join("outer.oppie");
    let mut a = Operandi::empty();
    nested(&mut a);
    a.execute(&format!(
        "operandi.pie.export outer filename:{{{}}}",
        file.display()
    ));
    assert!(a.status.contains("and its sub-pie"), "{}", a.status);
    let read = crate::pie_forms::read_pie_file(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(read.pie, "outer");
    let ids: Vec<&str> = read.forms.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["outer", "inner"]);
    let mut b = Operandi::empty();
    b.execute(&format!("operandi.pie.import {{{}}}", file.display()));
    assert_eq!(b.status, "Imported pie outer");
    assert_eq!(b.pie_named("outer").unwrap(), a.pie_named("outer").unwrap());
    assert_eq!(b.pie_named("inner").unwrap(), a.pie_named("inner").unwrap());
    // A shipped pie exports on its own.
    a.execute(&format!(
        "operandi.pie.export pie.view filename:{{{}}}",
        dir.join("view.oppie").display()
    ));
    assert!(
        a.status.starts_with("Exported pie pie.view to"),
        "{}",
        a.status
    );
    a.execute("operandi.pie.export pie.layouts");
    assert!(a.status.contains("no file"), "{}", a.status);
    let _ = std::fs::remove_dir_all(&dir);
}

/// P4: importing over ids that are taken asks first; rename keeps both
/// (and the slots follow the new ids); replace overwrites.
#[test]
fn import_clash_asks_then_renames_or_replaces() {
    let dir = temp_dir("clash");
    let file = dir.join("outer.oppie");
    let mut a = Operandi::empty();
    nested(&mut a);
    a.execute(&format!(
        "operandi.pie.export outer filename:{{{}}}",
        file.display()
    ));
    // This session changes its own outer, then imports the old one.
    a.execute("operandi.form.slot index:0 line:{View Back} label:Behind form:outer");
    a.execute(&format!("operandi.pie.import {{{}}}", file.display()));
    let pending = a.pie_edit.import.clone().expect("asks first");
    assert_eq!(pending.clashes, ["outer", "inner"]);
    assert_eq!(
        labels(&a, "outer")[0].as_deref(),
        Some("Behind"),
        "nothing changed yet"
    );
    // Rename: both kept; the imported outer opens the imported inner.
    a.pie_edit.import = None;
    a.execute(&format!(
        "operandi.pie.import {{{}}} onClash:rename",
        file.display()
    ));
    assert_eq!(a.status, "Imported pie outer.2");
    assert_eq!(labels(&a, "outer.2")[0].as_deref(), Some("Top"));
    assert_eq!(labels(&a, "outer")[0].as_deref(), Some("Behind"));
    assert_eq!(
        a.pie_named("outer.2")
            .unwrap()
            .slot(2)
            .unwrap()
            .sub
            .as_deref(),
        Some("inner.2")
    );
    // Replace: the imported one wins.
    a.execute(&format!(
        "operandi.pie.import {{{}}} onClash:replace",
        file.display()
    ));
    assert_eq!(labels(&a, "outer")[0].as_deref(), Some("Top"));
    // A file that isn't a pie file is refused.
    std::fs::write(dir.join("bad.oppie"), "{\"version\":1}").unwrap();
    a.execute(&format!(
        "operandi.pie.import {{{}}}",
        dir.join("bad.oppie").display()
    ));
    assert!(a.status.contains("bad.oppie"), "{}", a.status);
    let _ = std::fs::remove_dir_all(&dir);
}
