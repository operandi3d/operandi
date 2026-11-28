//! CA-CON-025/026 and NSM-001: numeric Cone applied twice through the actual
//! tool panel, then Drop, Edit-menu Undo/Redo and native-format Save/Open.
//!
//! Start: the production New baseline (one selected empty mesh). Cone is
//! armed by clicking the toolbox Cone button. Position, Size, Sides and
//! Segments are typed into the panel fields and confirmed with Return; the
//! Axis Y button and the Face and Single Polygon popup entries are clicked.
//! Make UVs on, Distortion Compensation off and Gaps 0 are the asserted
//! defaults, read back from the panel's own widgets. Apply and Drop are the
//! panel buttons. Undo/Redo, Save and Open are read from the production menu
//! model and dispatched through its routing (the platform menu callback
//! target); Save As and Open complete through the egui file dialog's path
//! field and buttons.
//!
//! The expected state after every step comes from a separate `Editor` given
//! the same command sequence: an integration comparison between two routes
//! into one kernel, not independent reference geometry. Only the 25/25 and
//! 56/56 cardinalities and the CA-CON-025 tool-state sequence are taken from
//! the reviewed specification. Synthetic egui events and AccessKit output
//! only: no native input, focus, display-scale or AppKit menu claim.
use super::tests::{draw, key, pointer};
use super::*;
use egui::accesskit::{self, Role, Toggled};
use operandi_core::{ConeCap, ConeParameters, NumericCone, PolygonRepresentation};

// Tall enough that the Cone panel's Apply sits above the status bar with
// three rows of primitive cells (the toolbox scrolls on shorter windows).
// 1100 tall since the Create tab gained its Edit, Merge, Align and Measure
// headers (2026-11-05); at 1000 the Cone panel's Apply needs a scroll.
// Tall enough for the Create groups, all open, plus the Cone panel.
const SIZE: Vec2 = vec2(1440., 1500.);

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    ctx
}

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) -> Vec<accesskit::Node> {
    draw(a, ctx, SIZE, events)
        .platform_output
        .accesskit_update
        .expect("AccessKit enabled")
        .nodes
        .into_iter()
        .map(|(_, n)| n)
        .collect()
}

fn rect(n: &accesskit::Node) -> Rect {
    let b = n.bounds().expect("widget bounds");
    Rect::from_min_max(
        pos2(b.x0 as f32, b.y0 as f32),
        pos2(b.x1 as f32, b.y1 as f32),
    )
}

fn text(n: &accesskit::Node) -> Option<String> {
    n.label().or(n.value()).map(str::to_string)
}

/// Hover, press and release at one point over three passes.
fn click_at(a: &mut Operandi, ctx: &Context, at: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(at)]);
    frame(a, ctx, vec![pointer(at, true, Modifiers::NONE)]);
    frame(a, ctx, vec![pointer(at, false, Modifiers::NONE)]);
}

/// The one node with this role and exact text, optionally below a field.
fn node(
    nodes: &[accesskit::Node],
    role: Role,
    label: &str,
    below: Option<Rect>,
) -> accesskit::Node {
    let hits: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == role && text(n).as_deref() == Some(label))
        .filter(|n| below.is_none_or(|b| rect(n).top() >= b.bottom()))
        .cloned()
        .collect();
    assert_eq!(hits.len(), 1, "one {role:?} {label:?}: {hits:?}");
    hits[0].clone()
}

fn click(a: &mut Operandi, ctx: &Context, role: Role, label: &str, below: Option<Rect>) {
    let nodes = frame(a, ctx, vec![]);
    let n = node(&nodes, role, label, below);
    assert!(!n.is_disabled(), "{label} enabled");
    click_at(a, ctx, rect(&n).center());
}

/// Click the text input, replace its text with End, Backspace and Text
/// events, then press Return.
fn type_into(a: &mut Operandi, ctx: &Context, input: &accesskit::Node, new: &str) {
    let old = input.value().unwrap_or_default().to_string();
    click_at(a, ctx, rect(input).center());
    let mut events = vec![key(Key::End)];
    events.extend((0..old.chars().count()).map(|_| key(Key::Backspace)));
    events.push(Event::Text(new.into()));
    frame(a, ctx, events);
    frame(a, ctx, vec![key(Key::Enter)]);
    frame(a, ctx, vec![]);
}

fn field_input(a: &Operandi, nodes: &[accesskit::Node], field: &str) -> accesskit::Node {
    let area = a.field_rects[field];
    let hits: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == Role::TextInput && area.contains(rect(n).center()))
        .cloned()
        .collect();
    assert_eq!(hits.len(), 1, "one visible input for {field}");
    hits[0].clone()
}

fn type_field(a: &mut Operandi, ctx: &Context, field: &str, new: &str) {
    let nodes = frame(a, ctx, vec![]);
    let input = field_input(a, &nodes, field);
    type_into(a, ctx, &input, new);
    assert!(a.field_errors.is_empty(), "{field}: {:?}", a.field_errors);
}

/// Open a panel popup by its current text and click one entry.
fn choose(a: &mut Operandi, ctx: &Context, combo: &str, entry: &str) {
    let below = Some(a.field_rects["cone_sides"]);
    choose_below(a, ctx, combo, entry, below);
}

fn choose_below(a: &mut Operandi, ctx: &Context, combo: &str, entry: &str, below: Option<Rect>) {
    click(a, ctx, Role::ComboBox, combo, below);
    let nodes = frame(a, ctx, vec![]);
    assert!(egui::Popup::is_any_open(ctx), "{combo} popup opened");
    let at = rect(&node(&nodes, Role::Button, entry, None)).center();
    click_at(a, ctx, at);
    frame(a, ctx, vec![]);
    assert!(!egui::Popup::is_any_open(ctx), "{entry}: popup closed");
}

/// Reviewed CA-CON-025 values; the rest are the asserted defaults.
fn requested(sides: u32) -> ConeParameters {
    ConeParameters {
        position: DVec3::new(1., 2., 3.),
        size: DVec3::new(0.5, 0.75, 1.),
        sides,
        segments: 3,
        ..Default::default()
    }
}

type History = Vec<(String, Scene, Scene)>;

fn history(h: &[std::sync::Arc<Checkpoint>]) -> History {
    h.iter()
        .map(|c| (c.label.clone(), c.before.clone(), c.after.clone()))
        .collect()
}

/// Editor state compared between the app and the separate Editor route.
#[derive(Debug, PartialEq)]
struct State {
    scene: Scene,
    cone: Option<NumericCone>,
    preset: ConeParameters,
    live: bool,
    undo: Vec<String>,
    redo: Vec<String>,
}

fn state(e: &Editor) -> State {
    let labels = |h: &[std::sync::Arc<Checkpoint>]| h.iter().map(|c| c.label.clone()).collect();
    State {
        scene: e.scene.clone(),
        cone: e.session.cone.clone(),
        preset: e.presets.get::<operandi_core::ConePreset>().clone(),
        live: e.session.live.is_some(),
        undo: labels(&e.history.undo),
        redo: labels(&e.history.redo),
    }
}

/// Full scene, selection, history payload and numeric Cone session/preset.
fn assert_matches(a: &Operandi, oracle: &Editor, step: &str) {
    assert_eq!(state(&a.editor), state(oracle), "{step}: state");
    assert!(
        history(&a.editor.history.undo) == history(&oracle.history.undo),
        "{step}: undo"
    );
    assert!(
        history(&a.editor.history.redo) == history(&oracle.history.redo),
        "{step}: redo"
    );
    a.editor.scene.validate().unwrap();
    snapshot(step, &a.editor);
}

fn counts(s: &Scene) -> (usize, usize) {
    let m = &s.active().unwrap().mesh;
    (m.vertices.len(), m.faces.len())
}

/// Stage record for the task evidence log (`--nocapture`).
fn snapshot(step: &str, e: &Editor) {
    let s = &e.scene;
    let m = &s.active().unwrap().mesh;
    println!(
        "SNAPSHOT {step}: V{} F{} items={} selection={:?} uv_maps={:?} uv_records={:?} cone={:?} preset_sides={} undo={:?} redo={:?}",
        m.vertices.len(),
        m.faces.len(),
        s.items.len(),
        s.selection,
        m.uv_maps.keys().collect::<Vec<_>>(),
        m.attributes
            .uv_records
            .iter()
            .map(|(k, v)| (k, v.len()))
            .collect::<Vec<_>>(),
        e.session.cone.as_ref().map(|c| (
            c.target,
            c.requested.sides,
            c.preview.as_ref().map(|p| p.vertices.len()),
            c.error.clone()
        )),
        e.presets.get::<operandi_core::ConePreset>().sides,
        e.history.undo.iter().map(|c| &c.label).collect::<Vec<_>>(),
        e.history.redo.iter().map(|c| &c.label).collect::<Vec<_>>(),
    );
}

/// The enabled production menu item for `action`; returns its title.
fn menu_item(a: &Operandi, ctx: &Context, action: app_menu::Action) -> String {
    let items: Vec<_> = a
        .menu_model(ctx)
        .iter()
        .flat_map(|m| m.leaves().into_iter().cloned().collect::<Vec<_>>())
        .filter(|i| i.action == action)
        .collect();
    assert_eq!(items.len(), 1, "{action:?} menu item");
    assert!(items[0].enabled, "{action:?} enabled: {:?}", items[0]);
    items[0].title.clone()
}

fn menu(a: &mut Operandi, ctx: &Context, action: app_menu::Action) {
    frame(a, ctx, vec![]);
    let title = menu_item(a, ctx, action);
    println!("MENU {action:?}: {title:?}");
    a.menu_action(ctx, action);
    frame(a, ctx, vec![]);
}

/// Complete the open file dialog through its path field and button.
fn complete_dialog(a: &mut Operandi, ctx: &Context, path: &std::path::Path, button: &str) {
    let shown = a.file_dialog.as_ref().expect("file dialog").path.clone();
    let nodes = frame(a, ctx, vec![]);
    let inputs: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == Role::TextInput && n.value() == Some(shown.as_str()))
        .cloned()
        .collect();
    assert_eq!(inputs.len(), 1, "one dialog path field");
    type_into(a, ctx, &inputs[0], &path.display().to_string());
    let nodes = frame(a, ctx, vec![]);
    let field = nodes
        .iter()
        .find(|n| n.role() == Role::TextInput && n.value() == Some(&*path.display().to_string()))
        .expect("typed path shown");
    // UIC-83: OK sits in the button column right of the field.
    let _ = field;
    let at = rect(&node(&nodes, Role::Button, button, None)).center();
    click_at(a, ctx, at);
    frame(a, ctx, vec![]);
    assert!(
        a.file_dialog.is_none(),
        "{button} closed dialog: {}",
        a.status
    );
}

#[test]
fn ca_con_025_026_actual_panel_two_applies_menu_history_and_reopen() {
    let ctx = context();
    let mut a = Operandi::empty();
    a.initialize_new_document();
    let mut oracle = Editor::default();
    oracle.scene.new_mesh().unwrap();
    oracle.scene.add_default_camera_and_light().unwrap();
    // RI-05: new-scene render settings.
    oracle.scene.use_new_scene_render_settings();
    // New scene spec NS01: Item mode.
    oracle.scene.selection.mode = Mode::Item;
    let baseline = a.editor.scene.clone();
    assert_eq!(baseline, oracle.scene, "NSM-001 baseline");
    assert_eq!(counts(&baseline), (0, 0));
    assert!(a.editor.history.undo.is_empty() && a.editor.history.redo.is_empty());
    frame(&mut a, &ctx, vec![]);

    // Arm through the toolbox Cone button.
    click(&mut a, &ctx, Role::Button, "Cone", None);
    oracle.dispatch(Command::ArmCone).unwrap();
    let session = a
        .editor
        .session
        .cone
        .as_ref()
        .expect("Cone armed by its button");
    assert_eq!(session.target, baseline.active().unwrap().id);
    assert_eq!(session.requested, ConeParameters::default());
    assert_eq!(a.editor.scene, baseline, "arming commits nothing");

    // Numbers through the visible fields, each confirmed with Return.
    for (field, value) in [
        ("cone_position0", "1 m"),
        ("cone_position1", "2 m"),
        ("cone_position2", "3 m"),
        ("cone_size0", "0.5 m"),
        ("cone_size1", "0.75 m"),
        ("cone_size2", "1 m"),
        ("cone_sides", "8"),
        ("cone_segments", "3"),
    ] {
        type_field(&mut a, &ctx, field, value);
        assert_eq!(a.editor.scene, baseline, "{field}: scene unchanged");
        assert!(a.editor.history.undo.is_empty(), "{field}: no history");
    }
    // Choices through their controls: Axis Y, Face and Single Polygon.
    let below = Some(a.field_rects["cone_sides"]);
    // Axis is joined segments (UIC-89), one radio button each.
    click(&mut a, &ctx, Role::RadioButton, "Y", below);
    choose(&mut a, &ctx, "Face", "Face");
    // Top Cap, then Bottom Cap below it; both start at Single Polygon.
    let nodes = frame(&mut a, &ctx, vec![]);
    let sides = a.field_rects["cone_sides"];
    let top = nodes
        .iter()
        .filter(|n| n.role() == Role::ComboBox && text(n).as_deref() == Some("Single Polygon"))
        .map(rect)
        .filter(|r| r.top() >= sides.bottom())
        .min_by(|x, y| x.top().total_cmp(&y.top()))
        .unwrap();
    choose_below(&mut a, &ctx, "Single Polygon", "Single Polygon", Some(top));
    let want = requested(8);
    assert_eq!(want.axis, 1);
    assert_eq!(want.polygon, PolygonRepresentation::Face);
    assert_eq!(want.bottom_cap, ConeCap::Single);
    assert!(want.make_uvs && !want.compensation && want.gaps == 0.);
    assert_eq!(a.editor.session.cone.as_ref().unwrap().requested, want);
    let nodes = frame(&mut a, &ctx, vec![]);
    let toggled = |label| node(&nodes, Role::CheckBox, label, None).toggled();
    assert_eq!(toggled("Make UVs"), Some(Toggled::True));
    assert_eq!(toggled("Distortion Compensation"), Some(Toggled::False));
    assert_eq!(field_input(&a, &nodes, "cone_gaps").value(), Some("0"));
    oracle
        .dispatch(Command::ConfigureCone(want.clone()))
        .unwrap();
    assert!(a.editor.session.cone.as_ref().unwrap().preview.is_some());
    assert_eq!(counts(&a.editor.scene), (0, 0), "preview is not committed");
    assert!(!a.dirty());
    assert_matches(&a, &oracle, "configured");

    // First panel Apply: 25/25, Cone stays armed.
    let sides = Some(a.field_rects["cone_sides"]);
    click(&mut a, &ctx, Role::Button, "Apply", sides);
    oracle.dispatch(Command::Apply).unwrap();
    assert_eq!(counts(&a.editor.scene), (25, 25), "CA-CON-025 first Apply");
    assert_eq!(
        a.editor.session.cone.as_ref().unwrap().requested,
        want,
        "still armed"
    );
    assert_matches(&a, &oracle, "first Apply");
    let first = a.editor.scene.clone();

    // Sides 10 through its field: a new preview, the committed scene exact.
    type_field(&mut a, &ctx, "cone_sides", "10");
    oracle
        .dispatch(Command::ConfigureCone(requested(10)))
        .unwrap();
    assert_eq!(a.editor.scene, first, "Sides edit leaves the first scene");
    let preview = a
        .editor
        .session
        .cone
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap();
    assert_eq!(preview.vertices.len(), 31, "distinct Sides 10 preview");
    assert_matches(&a, &oracle, "Sides 10");

    // Second panel Apply: a separate 31/31 cone, 56/56 in all.
    click(&mut a, &ctx, Role::Button, "Apply", sides);
    oracle.dispatch(Command::Apply).unwrap();
    assert_eq!(counts(&a.editor.scene), (56, 56), "CA-CON-025 second Apply");
    assert_matches(&a, &oracle, "second Apply");
    let second = a.editor.scene.clone();
    let armed = a
        .editor
        .session
        .cone
        .clone()
        .expect("armed after second Apply");
    assert_eq!(armed.requested, requested(10));
    assert!(armed.preview.is_none());

    // Drop through the panel button keeps the geometry.
    click(&mut a, &ctx, Role::Button, "Drop", sides);
    oracle.dispatch(Command::Drop).unwrap();
    assert!(a.editor.session.cone.is_none(), "dropped: {}", a.status);
    assert_eq!(a.editor.scene, second);
    assert_eq!(
        *a.editor.presets.get::<operandi_core::ConePreset>(),
        requested(10)
    );
    assert_matches(&a, &oracle, "Drop");
    let dropped = state(&a.editor);

    // Drop after Apply shows no preview, so it records nothing. Edit menu
    // Undo takes back the second Apply; Cone stays closed (like Ellipsoid).
    menu(&mut a, &ctx, app_menu::line_action("app.undo"));
    oracle.dispatch(Command::Undo).unwrap();
    assert_eq!(a.editor.scene, first, "Undo returns 25/25");
    assert!(a.editor.session.cone.is_none(), "Undo doesn't reopen Cone");
    assert_matches(&a, &oracle, "Undo second Apply");

    // Redo puts the second Cone back, still closed: the Drop state.
    menu(&mut a, &ctx, app_menu::line_action("app.redo"));
    oracle.dispatch(Command::Redo).unwrap();
    assert_eq!(a.editor.scene, second);
    assert_eq!(state(&a.editor), dropped, "Redo restores the Drop state");
    assert_matches(&a, &oracle, "Redo second Apply");
    assert!(a.dirty());

    // Save to a task-owned native path, then reopen it.
    let root = a.environment.paths().data_root().to_path_buf();
    let dir = root.join("cone-repeat-history");
    std::fs::create_dir(&dir).expect("create private destination directory");
    let path = dir.join("two-applies.opscene");
    assert!(path.starts_with(&root) && !path.exists());
    menu(&mut a, &ctx, app_menu::line_action("scene.save"));
    complete_dialog(&mut a, &ctx, &path, "OK");
    assert_eq!(a.path.as_deref(), Some(path.as_path()), "{}", a.status);
    assert!(!a.dirty());
    assert_eq!(a.saved, second);
    assert_eq!(state(&a.editor), dropped, "Save changes no editor state");
    println!("SAVED {} bytes", std::fs::metadata(&path).unwrap().len());

    // Close it first: opening a file that is already open only switches
    // to its scene.
    menu(&mut a, &ctx, app_menu::line_action("scene.close"));
    assert!(a.path.is_none(), "{}", a.status);
    menu(&mut a, &ctx, app_menu::line_action("scene.open"));
    complete_dialog(&mut a, &ctx, &path, "OK");
    assert_eq!(a.status, "Scene opened");
    assert_eq!(a.path.as_deref(), Some(path.as_path()));
    assert_eq!(
        a.editor.scene, second,
        "reopened scene equals the saved scene"
    );
    assert_eq!(a.editor.scene.selection, second.selection);
    let (got, want) = (&a.editor.scene.items[0].mesh, &second.items[0].mesh);
    assert_eq!(got.uv_maps, want.uv_maps);
    assert_eq!(got.attributes, want.attributes);
    assert!(!a.dirty() && a.editor.session.cone.is_none());
    a.editor.scene.validate().unwrap();
    snapshot("reopened", &a.editor);
}
