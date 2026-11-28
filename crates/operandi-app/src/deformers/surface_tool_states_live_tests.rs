//! SS-U05 / SS-U03: Tool States Save, popup recall and named Reset while a
//! Thicken-on stage is live, then explicit Cancel. Every step under test is
//! driven through synthetic egui pointer, key and text events on the real
//! panel widgets: the Tool States header, State name field, Save button,
//! States popup, Reset and Cancel buttons, the Offset field and the Both
//! Sides checkbox. That first test does not exercise Rename or Delete; the
//! appended tests below cover panel Rename/Delete with default Reset, and
//! armed Thicken/Smooth Shift switches through the menu routing with separate
//! retained values and Tool States namespaces.
//!
//! Fixture: an original 2 m cube built with the Box tool, face 0 selected,
//! identity transform, no assigned maps, neutral contexts. Initial parameters
//! are set explicitly through the existing command route: Offset 0.2 m,
//! Scale 100 %, Max. Smoothing Angle 89.5°, Thicken on, the other options off.
//!
//! Expected scenes come from a separate `Editor` started from the same source
//! scene with explicit parameters. That is an integration comparison between
//! two routes into the same project kernel, not independent reference
//! geometry. Widget rectangles and displayed values are read from egui's own
//! AccessKit output; no production instrumentation is added. The Tool States
//! store is session-only in the current core, so nothing is written to user or
//! global preset storage. No native input, focus, display-scale, restart
//! persistence or name-collision claim is made in this module.
use super::tests::{draw, key, pointer};
use super::*;
use egui::accesskit::{self, Role, Toggled};
use operandi_core::SmoothShiftParameters;

// Tall enough to show the whole Polygon tab without scrolling; scrolling
// itself is covered by layout_tests::toolbox_scrolls_to_its_last_row.
const SIZE: Vec2 = vec2(1440., 1100.);
const OPTIONS: [&str; 4] = ["Thicken", "Sharp", "Polygon Angle Weighting", "Both Sides"];
const NUMBERS: [(&str, &str); 3] = [
    ("sst_offset", " m"),
    ("sst_scale", " %"),
    ("sst_angle", " °"),
];

/// Saved state A: the explicit Thicken-on fixture parameters.
fn state_a() -> SmoothShiftParameters {
    SmoothShiftParameters {
        offset: 0.2,
        scale: 1.,
        angle: 89.5,
        thicken: true,
        sharp: false,
        angle_weight: false,
        both_sides: false,
    }
}

fn with(offset: f64, both_sides: bool) -> SmoothShiftParameters {
    SmoothShiftParameters {
        offset,
        both_sides,
        ..state_a()
    }
}

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    ctx
}

/// Original 2 m cube, idle, face 0 selected.
fn cube() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Box);
    a.run(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..Default::default()
    }));
    a.run(Command::Apply);
    a.run(Command::Drop);
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    a.category = toolbox::Category::Polygon;
    a.camera.view = View::Perspective;
    a.frame(false);
    a
}

/// Programmatic fixture setup: every parameter explicit, so the stage never
/// depends on retained Thicken values.
fn arm(a: &mut Operandi) {
    a.activate(Tool::Thicken);
    for (name, value) in [
        ("offset", "0.2m"),
        ("scale", "100%"),
        ("angle", "89.5 deg"),
        ("thicken", "on"),
        ("sharp", "off"),
        ("angle_weight", "off"),
        ("both_sides", "off"),
    ] {
        a.execute(&format!("Thicken {name} {value}"));
        assert_eq!(a.status, "Ready", "{name}");
    }
    assert_eq!(params(a), state_a());
}

fn params(a: &Operandi) -> SmoothShiftParameters {
    let l = a.editor.session.live.as_ref().expect("Thicken live");
    assert_eq!(l.tool, Tool::Thicken);
    l.params.smooth_shift
}

/// Separate Editor: source scene → Thicken → explicit parameters → stage.
fn editor_expected(source: &Scene, p: SmoothShiftParameters) -> Scene {
    let mut e = Editor {
        scene: source.clone(),
        ..Default::default()
    };
    e.dispatch(Command::Activate(Tool::Thicken)).unwrap();
    e.dispatch(Command::ConfigureSmoothShift(p)).unwrap();
    let staged = e.session.live.as_ref().unwrap().params.clone();
    e.dispatch(Command::Preview(staged)).unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().params.smooth_shift, p);
    e.scene
}

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) -> Vec<accesskit::Node> {
    let out = draw(a, ctx, SIZE, events);
    out.platform_output
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

/// Hover, press and release at the widget centre over three passes.
fn click_at(a: &mut Operandi, ctx: &Context, at: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(at)]);
    frame(a, ctx, vec![pointer(at, true, Modifiers::NONE)]);
    frame(a, ctx, vec![pointer(at, false, Modifiers::NONE)]);
}

/// The surface panel's checkbox for each option row, below the angle field.
fn checkboxes(a: &Operandi, nodes: &[accesskit::Node]) -> Vec<(Rect, bool)> {
    let below = a.field_rects["sst_angle"].bottom();
    OPTIONS
        .iter()
        .map(|label| {
            // UIC-115: a checkbox row names itself (no left label).
            let hits: Vec<_> = nodes
                .iter()
                .filter(|n| n.role() == Role::CheckBox)
                .filter(|n| n.label().or(n.value()) == Some(*label))
                .filter(|n| rect(n).top() >= below)
                .collect();
            assert_eq!(hits.len(), 1, "{label}: one checkbox on its row");
            let checked = match hits[0].toggled() {
                Some(Toggled::True) => true,
                Some(Toggled::False) => false,
                t => panic!("{label}: unexpected toggled state {t:?}"),
            };
            (rect(hits[0]), checked)
        })
        .collect()
}

/// The one panel node with this role and text below the option rows.
fn panel_node(a: &Operandi, nodes: &[accesskit::Node], role: Role, label: &str) -> Rect {
    let below = checkboxes(a, nodes)[3].0.bottom();
    let hits: Vec<Rect> = nodes
        .iter()
        .filter(|n| n.role() == role && text(n).as_deref() == Some(label))
        .map(rect)
        .filter(|r| r.top() >= below && r.left() >= a.field_rects["sst_angle"].left() - 400.)
        .collect();
    assert_eq!(hits.len(), 1, "{label}: one panel {role:?}");
    hits[0]
}

fn click_button(a: &mut Operandi, ctx: &Context, label: &str) {
    let nodes = frame(a, ctx, vec![]);
    let at = panel_node(a, &nodes, Role::Button, label).center();
    click_at(a, ctx, at);
}

fn click_option(a: &mut Operandi, ctx: &Context, label: &str) {
    let nodes = frame(a, ctx, vec![]);
    let i = OPTIONS.iter().position(|o| *o == label).unwrap();
    let at = checkboxes(a, &nodes)[i].0.center();
    click_at(a, ctx, at);
}

/// Replace a focused single-line field's text with End, Backspace and Text
/// events, then confirm it with Return.
fn type_into(a: &mut Operandi, ctx: &Context, at: Pos2, old: &str, new: &str) {
    click_at(a, ctx, at);
    let mut events = vec![key(Key::End)];
    events.extend((0..old.chars().count()).map(|_| key(Key::Backspace)));
    events.push(Event::Text(new.into()));
    frame(a, ctx, events);
    frame(a, ctx, vec![key(Key::Enter)]);
    frame(a, ctx, vec![]);
}

fn text_input_in(nodes: &[accesskit::Node], area: Rect) -> accesskit::Node {
    let mut hits: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == Role::TextInput && area.contains(rect(n).center()))
        .cloned()
        .collect();
    assert_eq!(hits.len(), 1, "one text input in {area:?}");
    hits.remove(0)
}

fn set_offset(a: &mut Operandi, ctx: &Context, value: &str) {
    let nodes = frame(a, ctx, vec![]);
    let field = text_input_in(&nodes, a.field_rects["sst_offset"]);
    let old = field.value().unwrap_or_default().to_string();
    type_into(a, ctx, rect(&field).center(), &old, value);
}

/// The Tool States body is collapsed by default; open it through its header.
fn expand_tool_states(a: &mut Operandi, ctx: &Context) {
    let nodes = frame(a, ctx, vec![]);
    let headers: Vec<Rect> = nodes
        .iter()
        .filter(|n| n.role() != Role::Label && text(n).as_deref() == Some("Tool States"))
        .map(rect)
        .collect();
    assert_eq!(headers.len(), 1, "one Tool States header");
    click_at(a, ctx, headers[0].center());
}

/// Displayed controls: three numeric field texts and four checkbox states,
/// plus the States popup's selected text.
#[derive(Debug, PartialEq)]
struct Displayed {
    numbers: Vec<String>,
    options: [bool; 4],
    state: String,
}

fn displayed(a: &mut Operandi, ctx: &Context) -> Displayed {
    let nodes = frame(a, ctx, vec![]);
    let numbers = NUMBERS
        .iter()
        .map(|(k, _)| {
            text_input_in(&nodes, a.field_rects[*k])
                .value()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    let c = checkboxes(a, &nodes);
    let below = c[3].0.bottom();
    let combos: Vec<String> = nodes
        .iter()
        .filter(|n| n.role() == Role::ComboBox && rect(n).top() >= below)
        // Not Item Transform's Reset / Freeze action menus.
        .filter(|n| n.value() != Some(crate::item_transform_ui::MENU_TEXT))
        .filter_map(text)
        .collect();
    assert_eq!(combos.len(), 1, "one States popup button: {combos:?}");
    Displayed {
        numbers,
        options: [c[0].1, c[1].1, c[2].1, c[3].1],
        state: combos[0].clone(),
    }
}

fn shown(p: SmoothShiftParameters, state: &str) -> Displayed {
    Displayed {
        numbers: vec![
            workspace::number_text(p.offset, NUMBERS[0].1),
            workspace::number_text(p.scale * 100., NUMBERS[1].1),
            workspace::number_text(p.angle, NUMBERS[2].1),
        ],
        options: [p.thicken, p.sharp, p.angle_weight, p.both_sides],
        state: state.into(),
    }
}

/// Typed field text is kept as entered, so compare its parsed quantity.
/// After a recall or Reset the fields are rebuilt, so `canonical` also
/// requires the exact formatted text of the stored values.
fn assert_shows(
    a: &mut Operandi,
    ctx: &Context,
    p: SmoothShiftParameters,
    state: &str,
    canonical: bool,
) {
    let d = displayed(a, ctx);
    let want = shown(p, state);
    assert_eq!(d.options, want.options, "checkboxes");
    assert_eq!(d.state, want.state, "States popup text");
    let quantities = [
        numeric::Quantity::Distance,
        numeric::Quantity::Percent,
        numeric::Quantity::Angle,
    ];
    for ((shown_text, quantity), value) in
        d.numbers
            .iter()
            .zip(quantities)
            .zip([p.offset, p.scale * 100., p.angle])
    {
        let parsed = numeric::parse_quantity(shown_text, quantity).unwrap();
        assert!((parsed - value).abs() < 1e-9, "{shown_text} shows {value}");
    }
    if canonical {
        assert_eq!(d.numbers, want.numbers, "fields rebuilt from values");
    }
}

/// Open the States popup through its button, then click one entry in it.
fn choose_state(a: &mut Operandi, ctx: &Context, entry: &str) {
    let nodes = frame(a, ctx, vec![]);
    let below = checkboxes(a, &nodes)[3].0.bottom();
    let combo: Vec<Rect> = nodes
        .iter()
        .filter(|n| n.role() == Role::ComboBox && rect(n).top() >= below)
        // Not Item Transform's Reset / Freeze action menus.
        .filter(|n| n.value() != Some(crate::item_transform_ui::MENU_TEXT))
        .map(rect)
        .collect();
    assert_eq!(combo.len(), 1, "one States popup button");
    click_at(a, ctx, combo[0].center());
    let nodes = frame(a, ctx, vec![]);
    assert!(egui::Popup::is_any_open(ctx), "States popup opened");
    // Popup entries are selectable labels, exposed as buttons.
    let entries: Vec<Rect> = nodes
        .iter()
        .filter(|n| n.role() == Role::Button && text(n).as_deref() == Some(entry))
        .map(rect)
        .collect();
    assert_eq!(entries.len(), 1, "{entry}: one popup entry");
    click_at(a, ctx, entries[0].center());
    frame(a, ctx, vec![]);
    assert!(!egui::Popup::is_any_open(ctx), "{entry}: popup closed");
}

fn labels(h: &[std::sync::Arc<Checkpoint>]) -> Vec<String> {
    h.iter().map(|c| c.label.clone()).collect()
}

fn faces(s: &Scene) -> usize {
    s.active().unwrap().mesh.faces.len()
}

/// The live stage, its scene and selection agree with a fresh Editor stage
/// from the source, with selected faces inside the current mesh.
fn assert_stage(a: &Operandi, source: &Scene, p: SmoothShiftParameters, step: &str) -> Scene {
    assert_eq!(params(a), p, "{step}: parameters ({})", a.status);
    let l = a.editor.session.live.as_ref().unwrap();
    assert!(l.applied, "{step}: stage stays live");
    assert!(l.preview_error.is_none(), "{step}: {:?}", l.preview_error);
    let expected = editor_expected(source, p);
    assert_eq!(a.editor.scene, expected, "{step}: whole scene");
    assert_eq!(a.editor.scene.selection, expected.selection, "{step}");
    let sel = &a.editor.scene.selection;
    assert!(!sel.faces.is_empty(), "{step}: selected descendants");
    assert!(
        sel.faces
            .iter()
            .all(|&f| (f as usize) < faces(&a.editor.scene))
    );
    expected
}

#[test]
fn thicken_on_live_tool_state_save_recall_reset_then_cancel() {
    let ctx = context();
    let mut a = cube();
    arm(&mut a);
    frame(&mut a, &ctx, vec![]);
    let source = a.editor.scene.clone();
    let history = labels(&a.editor.history.undo);
    assert!(a.editor.surface_states.is_empty(), "isolated session store");

    // First ordinary viewport click starts the live stage.
    let at = a.viewport_rect.left_top() + vec2(20., 60.);
    click_at(&mut a, &ctx, at);
    let stage_a = assert_stage(&a, &source, state_a(), "live A");
    assert_ne!(stage_a, source);
    expand_tool_states(&mut a, &ctx);
    assert_shows(&mut a, &ctx, state_a(), "(none)", true);

    // Save A under a unique name through the State name field and Save.
    let name = format!(
        "sst-live-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let nodes = frame(&mut a, &ctx, vec![]);
    // The name field is the panel text input between the options and Save.
    let below = checkboxes(&a, &nodes)[3].0.bottom();
    let save = panel_node(&a, &nodes, Role::Button, "Save");
    let name_fields: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == Role::TextInput)
        .filter(|n| {
            let r = rect(n);
            // The Name row sits between the options and Save, where the
            // panel drew its field.
            r.top() >= below
                && r.bottom() <= save.top()
                && r.contains(a.field_rects["surface_state_name"].center())
        })
        .cloned()
        .collect();
    assert_eq!(name_fields.len(), 1, "one State name field");
    let old = name_fields[0].value().unwrap_or_default().to_string();
    assert_eq!(old, "State 1", "existing default field text");
    type_into(&mut a, &ctx, rect(&name_fields[0]).center(), &old, &name);
    assert_eq!(a.fields["surface_state_name"], name);
    assert!(
        a.editor.surface_states.is_empty(),
        "typing alone saves nothing"
    );
    click_button(&mut a, &ctx, "Save");
    assert_eq!(
        a.editor.surface_states["Thicken"].get(&name),
        Some(&state_a()),
        "{}",
        a.status
    );
    assert_eq!(a.editor.surface_states["Thicken"].len(), 1);
    assert_eq!(a.editor.surface_state.get("Thicken"), Some(&name));
    assert!(!a.editor.surface_states.contains_key("Smooth Shift"));
    assert_eq!(a.editor.scene, stage_a, "Save leaves geometry unchanged");
    assert_eq!(
        labels(&a.editor.history.undo),
        history,
        "Save adds no history"
    );
    assert_shows(&mut a, &ctx, state_a(), &name, true);

    // Change to a distinct valid B through the Offset field and Both Sides.
    let b = with(0.35, true);
    set_offset(&mut a, &ctx, "0.35m");
    assert_stage(&a, &source, with(0.35, false), "Offset field");
    click_option(&mut a, &ctx, "Both Sides");
    let stage_b = assert_stage(&a, &source, b, "B");
    assert_ne!(stage_b, stage_a, "B is geometrically distinct from A");
    assert_eq!(
        a.editor.surface_states["Thicken"][&name],
        state_a(),
        "editing parameters does not rewrite the saved state"
    );

    // Select (none) first so the recall below is a true recall.
    choose_state(&mut a, &ctx, "(none)");
    assert_eq!(a.editor.surface_state.get("Thicken"), None);
    assert_eq!(a.editor.scene, stage_b, "(none) leaves the stage unchanged");
    assert_shows(&mut a, &ctx, b, "(none)", false);

    choose_state(&mut a, &ctx, &name);
    assert_eq!(a.editor.surface_state.get("Thicken"), Some(&name));
    let recalled = assert_stage(&a, &source, state_a(), "recall A");
    assert_eq!(
        recalled, stage_a,
        "recall replaces from source, not stacked"
    );
    assert_eq!(faces(&a.editor.scene), faces(&stage_a));
    assert_shows(&mut a, &ctx, state_a(), &name, true);
    assert_eq!(
        labels(&a.editor.history.undo),
        history,
        "recall adds no history"
    );
    assert!(a.editor.history.redo.is_empty());

    // Modify while the saved state stays selected; named Reset restores A,
    // not the shared all-off reset defaults.
    let c = with(0.05, true);
    set_offset(&mut a, &ctx, "0.05m");
    click_option(&mut a, &ctx, "Both Sides");
    let stage_c = assert_stage(&a, &source, c, "C");
    assert_ne!(stage_c, stage_a);
    assert_eq!(a.editor.surface_state.get("Thicken"), Some(&name));
    assert_shows(&mut a, &ctx, c, &name, false);
    click_button(&mut a, &ctx, "Reset");
    let reset = assert_stage(&a, &source, state_a(), "named Reset");
    assert_eq!(reset, stage_a);
    assert_ne!(params(&a), SmoothShiftParameters::default());
    assert_shows(&mut a, &ctx, state_a(), &name, true);
    assert_eq!(
        labels(&a.editor.history.undo),
        history,
        "Reset adds no history"
    );
    assert!(a.editor.history.redo.is_empty());

    // Explicit Cancel restores the source of the current stage.
    click_button(&mut a, &ctx, "Cancel");
    assert_eq!(a.editor.scene, source, "Cancel: {}", a.status);
    assert_eq!(a.editor.scene.selection, source.selection);
    assert_eq!(
        labels(&a.editor.history.undo),
        history,
        "Cancel adds no history"
    );
    assert!(a.editor.history.redo.is_empty());
    assert!(
        a.editor.session.live.is_none(),
        "Cancel deactivates: {}",
        a.status
    );
    // Session-only recipe bookkeeping is separate from geometry and history:
    // Cancel keeps the saved state and its selection.
    assert_eq!(a.editor.surface_states["Thicken"][&name], state_a());
    assert_eq!(a.editor.surface_states["Thicken"].len(), 1);
    assert_eq!(a.editor.surface_state.get("Thicken"), Some(&name));

    // Test cleanup of the in-memory store (not Delete coverage).
    a.editor
        .surface_states
        .get_mut("Thicken")
        .unwrap()
        .remove(&name);
    a.editor.surface_state.remove("Thicken");
    assert!(a.editor.surface_states["Thicken"].is_empty());
}

/// Unique session-store name for one saved state in this test process.
fn unique(tag: &str) -> String {
    format!(
        "sst-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

/// The State name field: the panel text input between the options and Save.
fn state_name_field(a: &Operandi, nodes: &[accesskit::Node]) -> accesskit::Node {
    let below = checkboxes(a, nodes)[3].0.bottom();
    let save = panel_node(a, nodes, Role::Button, "Save");
    let hits: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == Role::TextInput)
        .filter(|n| {
            let r = rect(n);
            // The Name row sits between the options and Save, where the
            // panel drew its field.
            r.top() >= below
                && r.bottom() <= save.top()
                && r.contains(a.field_rects["surface_state_name"].center())
        })
        .cloned()
        .collect();
    assert_eq!(hits.len(), 1, "one State name field");
    hits[0].clone()
}

/// Replace the State name field text through real typing.
fn type_state_name(a: &mut Operandi, ctx: &Context, name: &str) {
    let nodes = frame(a, ctx, vec![]);
    let field = state_name_field(a, &nodes);
    let old = field.value().unwrap_or_default().to_string();
    type_into(a, ctx, rect(&field).center(), &old, name);
    assert_eq!(a.fields["surface_state_name"], name);
}

/// Whether the panel button with this label is disabled in AccessKit.
fn button_disabled(a: &mut Operandi, ctx: &Context, label: &str) -> bool {
    let nodes = frame(a, ctx, vec![]);
    let at = panel_node(a, &nodes, Role::Button, label);
    let hits: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == Role::Button && rect(n) == at)
        .collect();
    assert_eq!(hits.len(), 1, "{label}: one button node");
    hits[0].is_disabled()
}

/// Open the States popup, require exactly `listed` among `known` entries,
/// then click `entry`.
fn choose_listed(a: &mut Operandi, ctx: &Context, known: &[&str], listed: &[&str], entry: &str) {
    let nodes = frame(a, ctx, vec![]);
    let below = checkboxes(a, &nodes)[3].0.bottom();
    let combo: Vec<Rect> = nodes
        .iter()
        .filter(|n| n.role() == Role::ComboBox && rect(n).top() >= below)
        // Not Item Transform's Reset / Freeze action menus.
        .filter(|n| n.value() != Some(crate::item_transform_ui::MENU_TEXT))
        .map(rect)
        .collect();
    assert_eq!(combo.len(), 1, "one States popup button");
    click_at(a, ctx, combo[0].center());
    let nodes = frame(a, ctx, vec![]);
    assert!(egui::Popup::is_any_open(ctx), "States popup opened");
    let shown: Vec<String> = nodes
        .iter()
        .filter(|n| n.role() == Role::Button)
        .filter_map(text)
        .filter(|t| known.contains(&t.as_str()))
        .collect();
    let mut want: Vec<String> = listed.iter().map(|s| s.to_string()).collect();
    let mut got = shown.clone();
    want.sort();
    got.sort();
    assert_eq!(got, want, "popup inventory");
    let at: Vec<Rect> = nodes
        .iter()
        .filter(|n| n.role() == Role::Button && text(n).as_deref() == Some(entry))
        .map(rect)
        .collect();
    assert_eq!(at.len(), 1, "{entry}: one popup entry");
    click_at(a, ctx, at[0].center());
    frame(a, ctx, vec![]);
    assert!(!egui::Popup::is_any_open(ctx), "{entry}: popup closed");
}

/// SS-U05 maintenance while a Thicken-on stage is live: panel Rename and
/// Delete of the selected state while the current parameters differ from its
/// saved values, then Reset with (none) selected, then explicit Cancel.
///
/// A second saved state C is a sentinel: Rename must leave it alone, and
/// Delete must not silently select it, so the (none)/defaults checks cannot
/// pass vacuously against an empty store. Rename uses a new non-colliding
/// name; collision and invalid-name policy is not exercised here. Expected
/// geometry comes from a separate `Editor` started from the same source, as in
/// the test above; that is an integration oracle, not reference parity.
#[test]
fn thicken_on_live_tool_state_rename_delete_then_default_reset_and_cancel() {
    let ctx = context();
    let mut a = cube();
    arm(&mut a);
    frame(&mut a, &ctx, vec![]);
    let source = a.editor.scene.clone();
    let history = labels(&a.editor.history.undo);
    assert!(a.editor.surface_states.is_empty(), "isolated session store");

    let p_a = state_a();
    let p_b = with(0.35, true);
    let p_c = SmoothShiftParameters {
        sharp: true,
        ..with(0.1, false)
    };
    assert!(p_a != p_b && p_b != p_c && p_a != p_c);
    let defaults = SmoothShiftParameters::default();
    assert!(!defaults.thicken && defaults.offset == 0. && defaults.angle == 89.5);
    assert!(![p_a, p_b, p_c].contains(&defaults));

    let (name_a, name_b, name_c) = (unique("a"), unique("b"), unique("c"));
    assert!(name_a != name_b && name_b != name_c && name_a != name_c);
    let known = ["(none)", name_a.as_str(), name_b.as_str(), name_c.as_str()];
    let states = |a: &Operandi| a.editor.surface_states["Thicken"].clone();
    let selected = |a: &Operandi| a.editor.surface_state.get("Thicken").cloned();

    // Live stage A, then save A and the sentinel C through the panel.
    let at = a.viewport_rect.left_top() + vec2(20., 60.);
    click_at(&mut a, &ctx, at);
    let stage_a = assert_stage(&a, &source, p_a, "live A");
    expand_tool_states(&mut a, &ctx);
    type_state_name(&mut a, &ctx, &name_a);
    click_button(&mut a, &ctx, "Save");
    assert_eq!(a.status, "Ready");
    assert_eq!(states(&a).get(&name_a), Some(&p_a));

    set_offset(&mut a, &ctx, "0.1m");
    click_option(&mut a, &ctx, "Sharp");
    let stage_c = assert_stage(&a, &source, p_c, "C");
    type_state_name(&mut a, &ctx, &name_c);
    click_button(&mut a, &ctx, "Save");
    assert_eq!(a.status, "Ready");
    assert_eq!(states(&a).len(), 2);
    assert_eq!(states(&a)[&name_c], p_c);
    assert_eq!(selected(&a), Some(name_c.clone()));

    // True recall of A through the popup, then edit to B with A selected.
    choose_listed(&mut a, &ctx, &known, &["(none)", &name_a, &name_c], &name_a);
    assert_eq!(selected(&a), Some(name_a.clone()));
    assert_eq!(assert_stage(&a, &source, p_a, "recall A"), stage_a);
    set_offset(&mut a, &ctx, "0.35m");
    click_option(&mut a, &ctx, "Both Sides");
    let stage_b = assert_stage(&a, &source, p_b, "B");
    assert!(stage_b != stage_a && stage_b != stage_c && stage_a != stage_c);
    assert_eq!(selected(&a), Some(name_a.clone()));
    assert_eq!(states(&a)[&name_a], p_a, "editing does not rewrite A");
    assert_eq!(labels(&a.editor.history.undo), history);

    // Rename A → B through the State name field and Rename.
    assert!(!button_disabled(&mut a, &ctx, "Rename"));
    assert!(!button_disabled(&mut a, &ctx, "Delete"));
    type_state_name(&mut a, &ctx, &name_b);
    assert_eq!(selected(&a), Some(name_a.clone()), "typing renames nothing");
    let selection = a.editor.scene.selection.clone();
    click_button(&mut a, &ctx, "Rename");
    assert_eq!(a.status, "Ready");
    let after = states(&a);
    assert_eq!(after.len(), 2);
    assert!(!after.contains_key(&name_a), "old name removed");
    assert_eq!(
        after[&name_b], p_a,
        "B holds A's saved values, not current B"
    );
    assert_eq!(after[&name_c], p_c, "sentinel unchanged");
    assert_eq!(selected(&a), Some(name_b.clone()));
    // Rename is recipe bookkeeping only.
    assert_eq!(params(&a), p_b, "current parameters unchanged");
    assert_eq!(a.editor.scene, stage_b, "Rename leaves geometry unchanged");
    assert_eq!(a.editor.scene.selection, selection);
    assert_eq!(
        labels(&a.editor.history.undo),
        history,
        "Rename adds no history"
    );
    assert!(a.editor.history.redo.is_empty());
    assert_shows(&mut a, &ctx, p_b, &name_b, false);

    // (none) then B: B recalls the renamed recipe, still A's values.
    choose_listed(
        &mut a,
        &ctx,
        &known,
        &["(none)", &name_b, &name_c],
        "(none)",
    );
    assert_eq!(selected(&a), None);
    assert_eq!(a.editor.scene, stage_b);
    choose_listed(&mut a, &ctx, &known, &["(none)", &name_b, &name_c], &name_b);
    assert_eq!(selected(&a), Some(name_b.clone()));
    assert_eq!(assert_stage(&a, &source, p_a, "recall B"), stage_a);
    assert_shows(&mut a, &ctx, p_a, &name_b, true);
    set_offset(&mut a, &ctx, "0.35m");
    click_option(&mut a, &ctx, "Both Sides");
    assert_eq!(assert_stage(&a, &source, p_b, "B again"), stage_b);
    assert_eq!(selected(&a), Some(name_b.clone()));

    // Delete B while current values differ from its saved values.
    let selection = a.editor.scene.selection.clone();
    click_button(&mut a, &ctx, "Delete");
    assert_eq!(a.status, "Ready");
    let after = states(&a);
    assert_eq!(after.len(), 1, "only the sentinel remains");
    assert_eq!(after[&name_c], p_c);
    assert!(!after.contains_key(&name_a) && !after.contains_key(&name_b));
    assert_eq!(selected(&a), None, "C is not silently chosen");
    assert_eq!(params(&a), p_b, "current values stay usable");
    assert_eq!(a.editor.scene, stage_b, "Delete leaves geometry unchanged");
    assert_eq!(a.editor.scene.selection, selection);
    assert!(a.editor.session.live.as_ref().unwrap().applied);
    assert_eq!(
        labels(&a.editor.history.undo),
        history,
        "Delete adds no history"
    );
    assert!(a.editor.history.redo.is_empty());
    assert_shows(&mut a, &ctx, p_b, "(none)", false);
    assert!(button_disabled(&mut a, &ctx, "Rename"));
    assert!(button_disabled(&mut a, &ctx, "Delete"));
    // The popup lists the sentinel, unselected; choosing (none) changes nothing.
    choose_listed(&mut a, &ctx, &known, &["(none)", &name_c], "(none)");
    assert_eq!(selected(&a), None);
    assert_eq!(params(&a), p_b);
    assert_eq!(a.editor.scene, stage_b);

    // Reset with (none): under Thicken, Thicken's own starting values from
    // its preset row (tool presets TP-15, 2026-11-15; before that it gave
    // Smooth Shift's and turned Thicken off). Not the sentinel.
    let defaults = operandi_core::tool_preset::thicken_values();
    click_button(&mut a, &ctx, "Reset");
    assert_eq!(a.status, "Ready");
    let reset = assert_stage(&a, &source, defaults, "default Reset");
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Thicken);
    assert!(params(&a).thicken, "Reset keeps Thicken on (TP-15)");
    assert_ne!(params(&a), p_c);
    assert_ne!(reset, stage_b);
    assert_shows(&mut a, &ctx, defaults, "(none)", true);
    assert_eq!(
        labels(&a.editor.history.undo),
        history,
        "Reset adds no history"
    );
    assert!(a.editor.history.redo.is_empty());
    assert_eq!(states(&a).len(), 1);
    assert_eq!(states(&a)[&name_c], p_c);
    assert_eq!(selected(&a), None);

    // Explicit Cancel restores the current stage's source.
    click_button(&mut a, &ctx, "Cancel");
    assert_eq!(a.editor.scene, source, "Cancel: {}", a.status);
    assert_eq!(a.editor.scene.selection, source.selection);
    assert_eq!(
        labels(&a.editor.history.undo),
        history,
        "Cancel adds no history"
    );
    assert!(a.editor.history.redo.is_empty());
    assert!(
        a.editor.session.live.is_none(),
        "Cancel deactivates: {}",
        a.status
    );
    // Session-only recipe bookkeeping survives Cancel: sentinel kept, unselected.
    assert_eq!(states(&a).len(), 1);
    assert_eq!(states(&a)[&name_c], p_c);
    assert_eq!(selected(&a), None);

    // Test cleanup of the in-memory store only.
    a.editor
        .surface_states
        .get_mut("Thicken")
        .unwrap()
        .remove(&name_c);
    assert!(a.editor.surface_states["Thicken"].is_empty());
}

/// The armed (not yet applied) surface session for `tool`: its parameters.
fn armed(a: &Operandi, tool: Tool, step: &str) -> SmoothShiftParameters {
    let l = a.editor.session.live.as_ref().expect("surface tool armed");
    assert_eq!(l.tool, tool, "{step}: active identity ({})", a.status);
    assert!(!l.applied, "{step}: still armed, no live stage");
    assert!(l.preview_error.is_none(), "{step}: {:?}", l.preview_error);
    l.params.smooth_shift
}

/// Activate through the production Geometry > Polygon menu routing.
fn menu_activate(a: &mut Operandi, ctx: &Context, tool: Tool) {
    a.menu_action(ctx, app_menu::line_action(tool.label()));
    assert_eq!(a.status, "Ready", "{}: menu activation", tool.label());
    frame(a, ctx, vec![]);
}

/// Open the Tool States body only if it is not already open, so a retained
/// open header is not toggled closed.
fn open_tool_states(a: &mut Operandi, ctx: &Context) {
    let nodes = frame(a, ctx, vec![]);
    let below = checkboxes(a, &nodes)[3].0.bottom();
    let open = nodes.iter().any(|n| {
        n.role() == Role::Button && text(n).as_deref() == Some("Save") && rect(n).top() >= below
    });
    if !open {
        expand_tool_states(a, ctx);
    }
}

/// Type into one surface numeric field and confirm it with Return.
fn set_field(a: &mut Operandi, ctx: &Context, key: &str, value: &str) {
    let nodes = frame(a, ctx, vec![]);
    let field = text_input_in(&nodes, a.field_rects[key]);
    let old = field.value().unwrap_or_default().to_string();
    type_into(a, ctx, rect(&field).center(), &old, value);
    assert_eq!(a.status, "Ready", "{key} {value}");
}

/// Click only the checkboxes whose displayed state differs from `want`, so
/// setup does not depend on any identity's initial or retained options.
fn set_options(a: &mut Operandi, ctx: &Context, want: [bool; 4]) {
    let shown = displayed(a, ctx).options;
    for (i, label) in OPTIONS.iter().enumerate() {
        if shown[i] != want[i] {
            click_option(a, ctx, label);
        }
    }
    assert_eq!(displayed(a, ctx).options, want);
}

/// Open the States popup, require exactly `listed` among `known` entries, then
/// dismiss it by clicking its own button again (no entry, no viewport click).
fn inspect_states(a: &mut Operandi, ctx: &Context, known: &[&str], listed: &[&str]) {
    let nodes = frame(a, ctx, vec![]);
    let below = checkboxes(a, &nodes)[3].0.bottom();
    let combo: Vec<Rect> = nodes
        .iter()
        .filter(|n| n.role() == Role::ComboBox && rect(n).top() >= below)
        // Not Item Transform's Reset / Freeze action menus.
        .filter(|n| n.value() != Some(crate::item_transform_ui::MENU_TEXT))
        .map(rect)
        .collect();
    assert_eq!(combo.len(), 1, "one States popup button");
    click_at(a, ctx, combo[0].center());
    let nodes = frame(a, ctx, vec![]);
    assert!(egui::Popup::is_any_open(ctx), "States popup opened");
    let mut got: Vec<String> = nodes
        .iter()
        .filter(|n| n.role() == Role::Button)
        .filter_map(text)
        .filter(|t| known.contains(&t.as_str()))
        .collect();
    let mut want: Vec<String> = listed.iter().map(|s| s.to_string()).collect();
    got.sort();
    want.sort();
    assert_eq!(got, want, "popup inventory");
    click_at(a, ctx, combo[0].center());
    frame(a, ctx, vec![]);
    assert!(!egui::Popup::is_any_open(ctx), "popup dismissed");
}

/// SS-U05 / SS-U03: Thicken and Smooth Shift each keep their own retained
/// parameters and Tool States namespace across armed identity switches.
///
/// Everything stays armed: no viewport click, Preview or Apply. Both
/// identities are activated through the production menu routing and
/// configured only through the panel's numeric fields, checkboxes, State name
/// field and Save, with distinct explicit values, so no first-seen preset is
/// asserted. After each switch the displayed fields, checkboxes, selected
/// state and popup inventory must belong to the active identity. Thicken is
/// then edited without saving; the switch round trip must keep those current
/// values, and a true (none) → saved recall must restore only Thicken's saved
/// values. Scene, selection and history must not change at any step: the
/// retained values and named states are session bookkeeping, not geometry.
#[test]
fn armed_identity_switches_keep_separate_retained_values_and_tool_states() {
    let ctx = context();
    let mut a = cube();
    frame(&mut a, &ctx, vec![]);
    let source = a.editor.scene.clone();
    let history = labels(&a.editor.history.undo);
    assert!(a.editor.session.live.is_none());
    assert!(a.editor.surface_states.is_empty(), "isolated session store");
    assert!(a.editor.surface_state.is_empty());

    let p_t = state_a();
    let p_s = SmoothShiftParameters {
        offset: -0.1,
        scale: 0.85,
        angle: 45.,
        thicken: false,
        sharp: true,
        angle_weight: false,
        both_sides: true,
    };
    let p_t2 = with(0.35, true);
    assert!(p_t != p_s && p_t != p_t2 && p_s != p_t2);
    let (name_t, name_s) = (unique("thicken"), unique("smooth"));
    assert_ne!(name_t, name_s);
    let known = ["(none)", name_t.as_str(), name_s.as_str()];
    let thicken = Tool::Thicken.label();
    let smooth = Tool::SmoothShift.label();
    assert_ne!(thicken, smooth, "distinct namespace keys");
    let unchanged = |a: &Operandi, step: &str| {
        assert_eq!(a.editor.scene, source, "{step}: scene unchanged");
        assert_eq!(a.editor.scene.selection, source.selection, "{step}");
        assert_eq!(
            labels(&a.editor.history.undo),
            history,
            "{step}: no history"
        );
        assert!(a.editor.history.redo.is_empty(), "{step}: no redo");
    };
    // Exact per-identity store contents and selections.
    let stores = |a: &Operandi,
                  t: &[(&str, SmoothShiftParameters)],
                  s: &[(&str, SmoothShiftParameters)],
                  t_sel: Option<&str>,
                  s_sel: Option<&str>,
                  step: &str| {
        let get = |key: &str| {
            a.editor
                .surface_states
                .get(key)
                .map(|m| m.iter().map(|(n, p)| (n.clone(), *p)).collect::<Vec<_>>())
                .unwrap_or_default()
        };
        let own = |v: &[(&str, SmoothShiftParameters)]| {
            let mut v: Vec<_> = v.iter().map(|(n, p)| (n.to_string(), *p)).collect();
            v.sort_by(|x, y| x.0.cmp(&y.0));
            v
        };
        assert_eq!(get(thicken), own(t), "{step}: Thicken states");
        assert_eq!(get(smooth), own(s), "{step}: Smooth Shift states");
        assert_eq!(
            a.editor.surface_state.get(thicken).map(String::as_str),
            t_sel,
            "{step}: Thicken selection"
        );
        assert_eq!(
            a.editor.surface_state.get(smooth).map(String::as_str),
            s_sel,
            "{step}: Smooth Shift selection"
        );
    };

    // 1. Thicken: explicit pT through the panel, saved as T.
    menu_activate(&mut a, &ctx, Tool::Thicken);
    armed(&a, Tool::Thicken, "Thicken activated");
    unchanged(&a, "Thicken activated");
    open_tool_states(&mut a, &ctx);
    set_field(&mut a, &ctx, "sst_offset", "0.2m");
    set_field(&mut a, &ctx, "sst_scale", "100%");
    set_field(&mut a, &ctx, "sst_angle", "89.5 deg");
    set_options(&mut a, &ctx, [true, false, false, false]);
    assert_eq!(armed(&a, Tool::Thicken, "pT"), p_t);
    type_state_name(&mut a, &ctx, &name_t);
    click_button(&mut a, &ctx, "Save");
    assert_eq!(a.status, "Ready");
    assert_eq!(armed(&a, Tool::Thicken, "T saved"), p_t);
    stores(
        &a,
        &[(name_t.as_str(), p_t)],
        &[],
        Some(name_t.as_str()),
        None,
        "T saved",
    );
    assert_shows(&mut a, &ctx, p_t, &name_t, false);
    unchanged(&a, "T saved");

    // 2. Smooth Shift: its own namespace shows nothing of Thicken's.
    menu_activate(&mut a, &ctx, Tool::SmoothShift);
    armed(&a, Tool::SmoothShift, "Smooth Shift activated");
    assert_eq!(
        *a.editor.presets.get::<operandi_core::ThickenPreset>(),
        Some(p_t),
        "Thicken retained pT"
    );
    unchanged(&a, "Smooth Shift activated");
    open_tool_states(&mut a, &ctx);
    assert_eq!(displayed(&mut a, &ctx).state, "(none)");
    inspect_states(&mut a, &ctx, &known, &["(none)"]);
    armed(&a, Tool::SmoothShift, "empty popup dismissed");
    set_field(&mut a, &ctx, "sst_offset", "-0.1m");
    set_field(&mut a, &ctx, "sst_scale", "85%");
    set_field(&mut a, &ctx, "sst_angle", "45 deg");
    set_options(&mut a, &ctx, [false, true, false, true]);
    assert_eq!(armed(&a, Tool::SmoothShift, "pS"), p_s);
    type_state_name(&mut a, &ctx, &name_s);
    click_button(&mut a, &ctx, "Save");
    assert_eq!(a.status, "Ready");
    assert_eq!(armed(&a, Tool::SmoothShift, "S saved"), p_s);
    let both = |a: &Operandi, t_sel: Option<&str>, step: &str| {
        stores(
            a,
            &[(name_t.as_str(), p_t)],
            &[(name_s.as_str(), p_s)],
            t_sel,
            Some(name_s.as_str()),
            step,
        )
    };
    both(&a, Some(name_t.as_str()), "S saved");
    assert_shows(&mut a, &ctx, p_s, &name_s, false);
    unchanged(&a, "S saved");

    // 3. Switch both ways; fields are rebuilt from each identity's values.
    menu_activate(&mut a, &ctx, Tool::Thicken);
    assert_eq!(armed(&a, Tool::Thicken, "back to Thicken"), p_t);
    assert_eq!(
        *a.editor.presets.get::<operandi_core::SmoothShiftPreset>(),
        p_s,
        "Smooth Shift retained pS"
    );
    open_tool_states(&mut a, &ctx);
    assert_shows(&mut a, &ctx, p_t, &name_t, true);
    inspect_states(&mut a, &ctx, &known, &["(none)", &name_t]);
    assert_eq!(armed(&a, Tool::Thicken, "T popup dismissed"), p_t);
    assert_shows(&mut a, &ctx, p_t, &name_t, true);
    both(&a, Some(name_t.as_str()), "Thicken shown");
    unchanged(&a, "Thicken shown");

    menu_activate(&mut a, &ctx, Tool::SmoothShift);
    assert_eq!(armed(&a, Tool::SmoothShift, "back to Smooth Shift"), p_s);
    open_tool_states(&mut a, &ctx);
    assert_shows(&mut a, &ctx, p_s, &name_s, true);
    inspect_states(&mut a, &ctx, &known, &["(none)", &name_s]);
    assert_eq!(armed(&a, Tool::SmoothShift, "S popup dismissed"), p_s);
    assert_shows(&mut a, &ctx, p_s, &name_s, true);
    both(&a, Some(name_t.as_str()), "Smooth Shift shown");
    unchanged(&a, "Smooth Shift shown");

    // 4. Unsaved Thicken edit to pT2 while T stays selected.
    menu_activate(&mut a, &ctx, Tool::Thicken);
    assert_eq!(armed(&a, Tool::Thicken, "Thicken for pT2"), p_t);
    open_tool_states(&mut a, &ctx);
    set_field(&mut a, &ctx, "sst_offset", "0.35m");
    click_option(&mut a, &ctx, "Both Sides");
    assert_eq!(armed(&a, Tool::Thicken, "pT2"), p_t2);
    assert_shows(&mut a, &ctx, p_t2, &name_t, false);
    both(&a, Some(name_t.as_str()), "pT2 unsaved");
    unchanged(&a, "pT2 unsaved");

    menu_activate(&mut a, &ctx, Tool::SmoothShift);
    assert_eq!(armed(&a, Tool::SmoothShift, "Smooth Shift after pT2"), p_s);
    assert_eq!(
        *a.editor.presets.get::<operandi_core::ThickenPreset>(),
        Some(p_t2),
        "current pT2 retained"
    );
    open_tool_states(&mut a, &ctx);
    assert_shows(&mut a, &ctx, p_s, &name_s, true);
    both(&a, Some(name_t.as_str()), "Smooth Shift after pT2");

    menu_activate(&mut a, &ctx, Tool::Thicken);
    assert_eq!(
        armed(&a, Tool::Thicken, "Thicken keeps pT2"),
        p_t2,
        "current values, not the saved recipe"
    );
    open_tool_states(&mut a, &ctx);
    assert_shows(&mut a, &ctx, p_t2, &name_t, true);
    both(&a, Some(name_t.as_str()), "Thicken keeps pT2");
    unchanged(&a, "Thicken keeps pT2");

    // True recall: (none), then T restores the saved pT in Thicken only.
    choose_listed(&mut a, &ctx, &known, &["(none)", &name_t], "(none)");
    assert_eq!(armed(&a, Tool::Thicken, "(none)"), p_t2);
    both(&a, None, "(none)");
    choose_listed(&mut a, &ctx, &known, &["(none)", &name_t], &name_t);
    assert_eq!(armed(&a, Tool::Thicken, "recall T"), p_t);
    assert_shows(&mut a, &ctx, p_t, &name_t, true);
    both(&a, Some(name_t.as_str()), "recall T");
    assert_eq!(
        *a.editor.presets.get::<operandi_core::SmoothShiftPreset>(),
        p_s,
        "Smooth Shift untouched"
    );
    unchanged(&a, "recall T");

    menu_activate(&mut a, &ctx, Tool::SmoothShift);
    assert_eq!(armed(&a, Tool::SmoothShift, "Smooth Shift final"), p_s);
    open_tool_states(&mut a, &ctx);
    assert_shows(&mut a, &ctx, p_s, &name_s, true);
    both(&a, Some(name_t.as_str()), "Smooth Shift final");
    unchanged(&a, "Smooth Shift final");

    // Cleanup: explicit Cancel of the armed tool keeps the scene unchanged.
    click_button(&mut a, &ctx, "Cancel");
    assert!(
        a.editor.session.live.is_none(),
        "Cancel deactivates: {}",
        a.status
    );
    unchanged(&a, "Cancel");
    both(&a, Some(name_t.as_str()), "after Cancel");

    // Test cleanup of the in-memory store only.
    for (key, name) in [(thicken, &name_t), (smooth, &name_s)] {
        a.editor.surface_states.get_mut(key).unwrap().remove(name);
        a.editor.surface_state.remove(key);
    }
    assert!(a.editor.surface_states.values().all(|m| m.is_empty()));
}
