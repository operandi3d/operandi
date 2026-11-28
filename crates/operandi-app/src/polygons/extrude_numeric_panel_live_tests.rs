//! EX-001/002/012/013 and EX-T11/T13: inactive numeric Polygon Extrude through
//! the actual tool panel. Every step under test is driven through synthetic
//! egui pointer, key and text events on the real widgets: the Extent X/Y/Z and
//! Sides fields, the Mode popup and the Apply button. Drop is the ordinary
//! non-text Q key through the keyboard input map, and Undo/Redo use the
//! production Edit menu routing.
//!
//! Fixture: an original 1 m planar quad, four vertices and one face, explicit
//! polygon selection of that face, identity transform, no UV map definitions,
//! neutral contexts. Explicit parameters: Extent (0, 0, 0.6 m), Sides 1, Mode
//! Shift and the retained Make UVs choice U. With no UV map defined the Make
//! UVs setters are unavailable (EX-002); U is the retained value, not typed.
//! Sides 1 equals the reset value, so its entry does not discriminate a
//! dropped Sides edit on its own.
//!
//! Expected scenes come from a separate `Editor` started from the same source
//! with the same explicit parameters. That is an integration comparison
//! between two routes into the same project kernel, not independent reference
//! geometry. The 4/1 → 8/5 → 12/9 cardinalities and the one-stage-per-Apply
//! boundaries are derived independently from EX-005/EX-012: Shift keeps the
//! four boundary points, adds one row of four per Apply with S = 1, replaces
//! the cap and adds four walls. Widget rectangles, displayed values and
//! enabled states are read from egui's own AccessKit output; no production
//! instrumentation is added. No native input, focus, display-scale or
//! native history payload claim is made here.
//!
//! The fixture above describes the original two-stage test. The appended
//! `ex_002_actual_panel_uv_map_selector_retains_v_after_deselection` test uses
//! the same quad plus an empty Texture UV definition; its own comment states
//! that fixture and what it does and does not establish.
use super::tests::{draw, key, pointer};
use super::*;
use egui::accesskit::{self, Role};
use operandi_core::{ExtrudeMode, ExtrudeUvs, Mesh};

const SIZE: Vec2 = vec2(1440., 900.);
const FIRST: DVec3 = DVec3::new(0., 0., 0.6);
const SECOND: DVec3 = DVec3::new(0., 0., -0.25);

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    ctx
}

/// Idle editor holding the original quad with face 0 explicitly selected.
fn quad_editor() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(0., 1., 0.),
        ],
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.scene.validate().unwrap();
    e
}

fn quad_app(fixture: &Editor) -> Operandi {
    let mut a = Operandi::empty();
    a.editor = fixture.clone();
    a.saved = a.editor.scene.clone();
    a.category = toolbox::Category::Polygon;
    a.camera.view = View::Perspective;
    a.frame(false);
    a
}

/// The explicit configuration for one stage.
fn configured(mut p: Parameters, extent: DVec3) -> Parameters {
    p.extent = extent;
    p.extrude_sides = 1;
    p.extrude_mode = ExtrudeMode::Shift;
    p.extrude_uvs = ExtrudeUvs::U;
    p
}

/// Separate Editor: same source → Extrude → explicit parameters → Apply for
/// each extent in turn. Returns the editor after every Apply.
fn editor_expected(fixture: &Editor, extents: &[DVec3]) -> Vec<Editor> {
    let mut e = fixture.clone();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    extents
        .iter()
        .map(|&d| {
            let p = configured(e.session.live.as_ref().unwrap().params.clone(), d);
            e.dispatch(Command::ConfigureExtrude(p)).unwrap();
            e.dispatch(Command::Apply).unwrap();
            e.clone()
        })
        .collect()
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

fn text_input_in(nodes: &[accesskit::Node], area: Rect) -> accesskit::Node {
    let mut hits: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == Role::TextInput && area.contains(rect(n).center()))
        .cloned()
        .collect();
    assert_eq!(hits.len(), 1, "one text input in {area:?}");
    hits.remove(0)
}

/// Click a panel field, replace its text with End, Backspace and Text events,
/// then press Return.
fn type_field(a: &mut Operandi, ctx: &Context, field: &str, new: &str) {
    let nodes = frame(a, ctx, vec![]);
    let input = text_input_in(&nodes, a.field_rects[field]);
    let old = input.value().unwrap_or_default().to_string();
    click_at(a, ctx, rect(&input).center());
    let mut events = vec![key(Key::End)];
    events.extend((0..old.chars().count()).map(|_| key(Key::Backspace)));
    events.push(Event::Text(new.into()));
    frame(a, ctx, events);
    frame(a, ctx, vec![key(Key::Enter)]);
    frame(a, ctx, vec![]);
}

/// The one node with this role whose text satisfies `want`, below the Sides
/// field and within the tool panel column.
fn panel_node(
    a: &Operandi,
    nodes: &[accesskit::Node],
    role: Role,
    want: impl Fn(&str) -> bool,
) -> accesskit::Node {
    let sides = a.field_rects["extrude_sides"];
    let hits: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == role && text(n).is_some_and(|t| want(&t)))
        .filter(|n| {
            let r = rect(n);
            r.top() >= sides.bottom() && r.left() >= sides.left() - 400.
        })
        .cloned()
        .collect();
    assert_eq!(hits.len(), 1, "one panel {role:?}: {:?}", hits);
    hits[0].clone()
}

/// Displayed panel state, read back from the widgets themselves.
#[derive(Debug, PartialEq)]
struct Shown {
    extent: [String; 3],
    sides: String,
    mode: String,
    /// Make UVs None/U/V disabled states. egui 0.33.3 `Button` publishes no
    /// selected flag to AccessKit, so the retained choice is checked from
    /// the tool parameters instead.
    uvs_disabled: [bool; 3],
    apply_disabled: bool,
}

fn shown(a: &mut Operandi, ctx: &Context) -> Shown {
    let nodes = frame(a, ctx, vec![]);
    let value = |key: &str| {
        text_input_in(&nodes, a.field_rects[key])
            .value()
            .unwrap_or_default()
            .to_string()
    };
    // Make UVs is a segments row (UIC-89): one radio button per choice.
    let uv = |label: &str| panel_node(a, &nodes, Role::RadioButton, |t| t == label).is_disabled();
    Shown {
        extent: [value("extent0"), value("extent1"), value("extent2")],
        sides: value("extrude_sides"),
        mode: text(&panel_node(a, &nodes, Role::ComboBox, |t| {
            matches!(t, "Auto" | "Shift" | "Thick")
        }))
        .unwrap(),
        uvs_disabled: [uv("None"), uv("U"), uv("V")],
        apply_disabled: panel_node(a, &nodes, Role::Button, |t| t == "Apply").is_disabled(),
    }
}

/// Typed field text is kept as entered, so compare its parsed distance.
fn assert_extent_shown(s: &Shown, want: DVec3, step: &str) {
    for (axis, shown_text) in s.extent.iter().enumerate() {
        let parsed = numeric::parse_quantity(shown_text, numeric::Quantity::Distance)
            .unwrap_or_else(|e| panic!("{step}: axis {axis} shows {shown_text:?}: {e}"));
        assert!(
            (parsed - want[axis]).abs() < 1e-12,
            "{step}: axis {axis} shows {shown_text:?}, want {}",
            want[axis]
        );
    }
}

/// All three Make UVs setters are unavailable because no UV map is defined
/// on the quad (EX-002); the retained U choice still applies.
const UVS_UNAVAILABLE: [bool; 3] = [true; 3];

fn choose_mode(a: &mut Operandi, ctx: &Context, entry: &str) {
    let nodes = frame(a, ctx, vec![]);
    let combo = panel_node(a, &nodes, Role::ComboBox, |t| {
        matches!(t, "Auto" | "Shift" | "Thick")
    });
    click_at(a, ctx, rect(&combo).center());
    let nodes = frame(a, ctx, vec![]);
    assert!(egui::Popup::is_any_open(ctx), "Mode popup opened");
    let entries: Vec<Rect> = nodes
        .iter()
        .filter(|n| n.role() == Role::Button && text(n).as_deref() == Some(entry))
        .map(rect)
        // In line with the combo: not the in-window menu bar's Texture
        // menu. The popup opens above the combo when it doesn't fit below
        // (with Linux's fallback font the panel sits lower), so don't require it to be below.
        .filter(|r| r.left() < rect(&combo).right() && r.right() > rect(&combo).left())
        // The menu bar has its own row now (2026-11-22), so its Texture
        // menu can line up with the combo too.
        .filter(|r| !a.layouts.menu_bar_rect.is_some_and(|m| m.intersects(*r)))
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "{entry}: one popup entry {entries:?} combo {:?}",
        rect(&combo)
    );
    click_at(a, ctx, entries[0].center());
    frame(a, ctx, vec![]);
    assert!(!egui::Popup::is_any_open(ctx), "{entry}: popup closed");
}

fn click_apply(a: &mut Operandi, ctx: &Context) {
    let nodes = frame(a, ctx, vec![]);
    let apply = panel_node(a, &nodes, Role::Button, |t| t == "Apply");
    assert!(!apply.is_disabled(), "Apply enabled before click");
    click_at(a, ctx, rect(&apply).center());
}

fn menu_enabled(a: &Operandi, ctx: &Context, action: app_menu::Action) -> bool {
    a.menu_model(ctx)
        .iter()
        .flat_map(app_menu::Menu::leaves)
        .any(|item| item.action == action && item.enabled)
}

fn labels(h: &[std::sync::Arc<Checkpoint>]) -> Vec<String> {
    h.iter().map(|c| c.label.clone()).collect()
}

fn counts(s: &Scene) -> (usize, usize) {
    let m = &s.active().unwrap().mesh;
    (m.vertices.len(), m.faces.len())
}

/// Exactly one face is selected and it is the moved cap: the source quad's
/// corners, in source order, displaced by `offset` (identity transform).
fn assert_selected_cap_at(s: &Scene, source: &Scene, offset: DVec3, step: &str) {
    assert_eq!(s.selection.mode, Mode::Polygon, "{step}");
    assert_eq!(s.selection.faces.len(), 1, "{step}: one moved cap selected");
    let m = &s.active().unwrap().mesh;
    let src = &source.active().unwrap().mesh;
    let cap = &m.faces[s.selection.faces[0] as usize];
    let corners: Vec<DVec3> = cap
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect();
    let want: Vec<DVec3> = src.faces[0]
        .vertices
        .iter()
        .map(|&v| src.vertices[v as usize] + offset)
        .collect();
    assert_eq!(corners, want, "{step}: selected cap corners");
}

fn armed(a: &Operandi, step: &str) -> Parameters {
    let l = a.editor.session.live.as_ref().expect("Extrude armed");
    assert_eq!(l.tool, Tool::Extrude, "{step}: active tool ({})", a.status);
    assert!(!l.applied, "{step}: armed, no live stage");
    assert!(l.preview_error.is_none(), "{step}: {:?}", l.preview_error);
    l.params.clone()
}

/// Stage snapshot printed for the task evidence log (`--nocapture`).
fn snapshot(step: &str, s: &Scene) {
    let m = &s.active().unwrap().mesh;
    println!(
        "SNAPSHOT {step}: V{} F{} selection={:?} mode={:?} uv_maps={:?} vertices={:?} faces={:?}",
        m.vertices.len(),
        m.faces.len(),
        s.selection.faces,
        s.selection.mode,
        m.uv_maps,
        m.vertices,
        m.faces
            .iter()
            .map(|f| f.vertices.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn ex_t11_actual_panel_two_apply_stages_invalid_draft_drop_undo_redo() {
    let ctx = context();
    let fixture = quad_editor();
    let expected = editor_expected(&fixture, &[FIRST, SECOND]);
    let mut a = quad_app(&fixture);
    let source = a.editor.scene.clone();
    snapshot("source", &source);
    assert_eq!(counts(&source), (4, 1));
    assert!(a.editor.history.undo.is_empty() && a.editor.history.redo.is_empty());

    // Production Geometry > Polygon menu activation arms Extrude only.
    a.menu_action(&ctx, app_menu::line_action(Tool::Extrude.label()));
    assert_eq!(a.status, "Ready", "menu activation");
    let reset = armed(&a, "activated");
    assert_eq!(reset.extent, DVec3::ZERO);
    assert_eq!(reset.extrude_sides, 1);
    assert_eq!(reset.extrude_mode, ExtrudeMode::Auto);
    assert_eq!(reset.extrude_uvs, ExtrudeUvs::U);
    let s = shown(&mut a, &ctx);
    assert_extent_shown(&s, DVec3::ZERO, "activated");
    assert_eq!(s.mode, "Auto");
    assert_eq!(s.uvs_disabled, UVS_UNAVAILABLE);
    assert!(!s.apply_disabled, "armed Apply available");
    assert_eq!(a.editor.scene, source);

    // Configuration through the real fields and popup stays armed: no stage,
    // no geometry, selection or history change.
    for (field, value) in [
        ("extent0", "0 m"),
        ("extent1", "0 m"),
        ("extent2", "0.6 m"),
        ("extrude_sides", "1"),
    ] {
        type_field(&mut a, &ctx, field, value);
        assert_eq!(a.status, "Ready", "{field} {value}");
        assert!(a.field_errors.is_empty(), "{field}: {:?}", a.field_errors);
        armed(&a, field);
        assert_eq!(a.editor.scene, source, "{field}: source unchanged");
        assert!(a.editor.history.undo.is_empty(), "{field}: no history");
    }
    choose_mode(&mut a, &ctx, "Shift");
    let p = armed(&a, "Mode Shift");
    assert_eq!(p, configured(reset.clone(), FIRST), "{}", a.status);
    assert_eq!(a.editor.scene, source, "Mode: source unchanged");
    assert_eq!(a.editor.scene.selection, source.selection);
    assert!(a.editor.history.undo.is_empty() && a.editor.history.redo.is_empty());
    let s = shown(&mut a, &ctx);
    assert_extent_shown(&s, FIRST, "configured");
    assert_eq!(s.sides, "1");
    assert_eq!(s.mode, "Shift");
    assert_eq!(s.uvs_disabled, UVS_UNAVAILABLE);
    assert!(!s.apply_disabled);

    // First Apply: one stage, tool still armed with its configuration.
    click_apply(&mut a, &ctx);
    assert_eq!(a.status, "Ready", "first Apply");
    let first = a.editor.scene.clone();
    snapshot("first", &first);
    assert_eq!(counts(&first), (8, 5), "EX-012 first stage");
    assert_eq!(
        armed(&a, "after first Apply"),
        configured(reset.clone(), FIRST)
    );
    assert_eq!(labels(&a.editor.history.undo), ["Extrude"]);
    assert!(a.editor.history.redo.is_empty());
    assert_eq!(a.editor.history.undo[0].before, source);
    assert_eq!(a.editor.history.undo[0].after, first);
    assert_eq!(first, expected[0].scene, "first: whole scene");
    assert_eq!(first.selection, expected[0].scene.selection);
    assert_selected_cap_at(&first, &source, FIRST, "first");
    assert!(first.active().unwrap().mesh.uv_maps.is_empty());
    first.validate().unwrap();
    let s = shown(&mut a, &ctx);
    assert_extent_shown(&s, FIRST, "after first Apply");
    assert_eq!(s.mode, "Shift");
    assert!(!s.apply_disabled, "Apply available for the next stage");

    // Invalid second extent: kept as a visible draft, Apply refuses without a
    // stage or partial edit.
    type_field(&mut a, &ctx, "extent2", "1 m/0");
    assert!(a.field_errors.contains_key("extent2"), "{}", a.status);
    assert_eq!(
        shown(&mut a, &ctx).extent[2],
        "1 m/0",
        "invalid draft shown"
    );
    click_apply(&mut a, &ctx);
    assert_eq!(a.editor.scene, first, "invalid draft: first retained");
    assert_eq!(labels(&a.editor.history.undo), ["Extrude"]);
    assert!(a.editor.history.redo.is_empty());
    assert!(a.field_errors.contains_key("extent2"));
    assert_eq!(armed(&a, "refused Apply").extent, FIRST, "last valid kept");
    assert_eq!(shown(&mut a, &ctx).extent[2], "1 m/0");

    // Correction is accepted but does not edit until Apply.
    type_field(&mut a, &ctx, "extent2", "-250 mm");
    assert!(a.field_errors.is_empty(), "{:?}", a.field_errors);
    assert_eq!(a.status, "Ready");
    assert_eq!(armed(&a, "corrected"), configured(reset.clone(), SECOND));
    assert_eq!(a.editor.scene, first, "correction: first unchanged");
    assert_eq!(labels(&a.editor.history.undo), ["Extrude"]);
    let s = shown(&mut a, &ctx);
    assert_extent_shown(&s, SECOND, "corrected");
    assert_eq!(s.mode, "Shift");

    // Second Apply acts on the current moved cap as a separate stage.
    click_apply(&mut a, &ctx);
    assert_eq!(a.status, "Ready", "second Apply");
    let second = a.editor.scene.clone();
    snapshot("second", &second);
    assert_eq!(counts(&second), (12, 9), "EX-012 second stage");
    assert_eq!(armed(&a, "after second Apply"), configured(reset, SECOND));
    assert_eq!(labels(&a.editor.history.undo), ["Extrude", "Extrude"]);
    assert!(a.editor.history.redo.is_empty());
    assert_eq!(a.editor.history.undo[1].before, first);
    assert_eq!(a.editor.history.undo[1].after, second);
    assert_eq!(second, expected[1].scene, "second: whole scene");
    assert_eq!(second.selection, expected[1].scene.selection);
    assert_selected_cap_at(&second, &source, FIRST + SECOND, "second");
    assert!(second.active().unwrap().mesh.uv_maps.is_empty());
    second.validate().unwrap();

    // Q with no text focus drops the tool and keeps the committed stages.
    frame(&mut a, &ctx, vec![]);
    assert!(!a.text_editor_focused(&ctx), "no text field focus before Q");
    frame(&mut a, &ctx, vec![key(Key::Q)]);
    assert!(a.editor.session.live.is_none(), "Q drops: {}", a.status);
    assert_eq!(a.editor.scene, second, "Drop keeps the second stage");
    assert_eq!(
        labels(&a.editor.history.undo),
        ["Extrude", "Extrude"],
        "no third"
    );

    // Undo/Redo through the production Edit menu routing.
    for (action, want, step) in [
        (app_menu::line_action("app.undo"), &first, "Undo → first"),
        (app_menu::line_action("app.undo"), &source, "Undo → source"),
        (app_menu::line_action("app.redo"), &first, "Redo → first"),
        (app_menu::line_action("app.redo"), &second, "Redo → second"),
    ] {
        assert!(menu_enabled(&a, &ctx, action), "{step}: menu enabled");
        a.menu_action(&ctx, action);
        assert_eq!(a.status, "Ready", "{step}");
        assert_eq!(&a.editor.scene, want, "{step}: whole scene");
        assert_eq!(a.editor.scene.selection, want.selection, "{step}");
        assert!(a.editor.session.live.is_none(), "{step}: no tool restored");
        frame(&mut a, &ctx, vec![]);
    }
    assert_eq!(
        a.editor.scene, expected[1].scene,
        "Redo: second matches again"
    );
    assert_eq!(labels(&a.editor.history.undo), ["Extrude", "Extrude"]);
    assert!(a.editor.history.redo.is_empty());
}

const NO_MAP: &str = "No UV map selected";
const TEXTURE: &str = "Texture";

/// The original quad plus an empty Texture UV definition: the map exists but
/// assigns no corner or point values.
fn textured_quad_editor() -> Editor {
    let mut e = quad_editor();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    mesh.uv_maps.insert(TEXTURE.into(), vec![vec![None; 4]]);
    e.scene.validate().unwrap();
    e
}

/// The explicit stage configuration with retained V on an existing but
/// unselected Texture map.
fn configured_v_unselected(p: Parameters) -> Parameters {
    let mut p = configured(p, FIRST);
    p.extrude_uvs = ExtrudeUvs::V;
    p.extrude_uv_map = Some(TEXTURE.into());
    p.extrude_uv_selected = false;
    p
}

/// Separate Editor: select Texture, choose V, deselect the map, then Apply.
fn editor_expected_v_unselected(fixture: &Editor) -> Editor {
    let mut e = fixture.clone();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    let want = configured_v_unselected(e.session.live.as_ref().unwrap().params.clone());
    let mut selected = want.clone();
    selected.extrude_uv_selected = true;
    e.dispatch(Command::ConfigureExtrude(selected)).unwrap();
    e.dispatch(Command::ConfigureExtrude(want)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e
}

/// Caption of the Extrude UV map selector.
fn uv_map_caption(a: &Operandi, nodes: &[accesskit::Node]) -> String {
    text(&panel_node(a, nodes, Role::ComboBox, |t| {
        t == NO_MAP || t == TEXTURE
    }))
    .unwrap()
}

fn choose_uv_map(a: &mut Operandi, ctx: &Context, entry: &str) {
    let nodes = frame(a, ctx, vec![]);
    let combo = panel_node(a, &nodes, Role::ComboBox, |t| t == NO_MAP || t == TEXTURE);
    click_at(a, ctx, rect(&combo).center());
    let nodes = frame(a, ctx, vec![]);
    assert!(egui::Popup::is_any_open(ctx), "UV map popup opened");
    let entries: Vec<Rect> = nodes
        .iter()
        .filter(|n| n.role() == Role::Button && text(n).as_deref() == Some(entry))
        .map(rect)
        // In line with the combo: not the in-window menu bar's Texture
        // menu. The popup opens above the combo when it doesn't fit below
        // (with Linux's fallback font the panel sits lower), so don't require it to be below.
        .filter(|r| r.left() < rect(&combo).right() && r.right() > rect(&combo).left())
        // The menu bar has its own row now (2026-11-22), so its Texture
        // menu can line up with the combo too.
        .filter(|r| !a.layouts.menu_bar_rect.is_some_and(|m| m.intersects(*r)))
        .collect();
    assert_eq!(entries.len(), 1, "{entry}: one popup entry");
    click_at(a, ctx, entries[0].center());
    frame(a, ctx, vec![]);
    assert!(!egui::Popup::is_any_open(ctx), "{entry}: popup closed");
}

fn click_uv_setter(a: &mut Operandi, ctx: &Context, label: &str) {
    let nodes = frame(a, ctx, vec![]);
    let button = panel_node(a, &nodes, Role::RadioButton, |t| t == label);
    click_at(a, ctx, rect(&button).center());
}

/// EX-002 with EX-009/010 retained-choice Apply, through the actual panel.
///
/// Fixture: the original 1 m quad and explicit face selection with an empty
/// Texture UV definition (one face of four absent corners, no point UVs).
/// Extent (0, 0, 0.6 m), Sides 1 and Mode Shift are entered through the real
/// fields and popup. No first-seen map-selector default is asserted: the
/// existing map combo first chooses "No UV map selected" explicitly, then
/// Texture. The None, U and V buttons are clicked while Texture is selected,
/// Texture is deselected through the same combo with V retained, and the
/// Apply button commits one Shift stage. Drop uses the non-text Q key (with
/// its release) and Undo/Redo use the production Edit menu routing.
///
/// The retained Make UVs choice is read from the live tool parameters: egui
/// 0.33.3 `Button` publishes no selected flag to AccessKit, so the rendered
/// selection marker is not observed here. The expected scene comes from a
/// separately driven `Editor` on the same source; this is integration
/// evidence between two routes into one kernel, not independent geometry or
/// per-corner UV parity. No native input, display or reopen claim is made.
#[test]
fn ex_002_actual_panel_uv_map_selector_retains_v_after_deselection() {
    let ctx = context();
    let fixture = textured_quad_editor();
    let expected = editor_expected_v_unselected(&fixture);
    let mut a = quad_app(&fixture);
    let source = a.editor.scene.clone();
    snapshot("uv source", &source);
    let src_mesh = &source.active().unwrap().mesh;
    assert_eq!(counts(&source), (4, 1));
    assert_eq!(src_mesh.uv_maps[TEXTURE], vec![vec![None; 4]]);
    assert!(src_mesh.attributes.point_uvs.is_empty(), "no assigned UVs");
    assert!(a.editor.history.undo.is_empty() && a.editor.history.redo.is_empty());

    // Arm through the production menu route; configure through real controls.
    a.menu_action(&ctx, app_menu::line_action(Tool::Extrude.label()));
    assert_eq!(a.status, "Ready", "menu activation");
    let reset = armed(&a, "activated");
    for (field, value) in [
        ("extent0", "0 m"),
        ("extent1", "0 m"),
        ("extent2", "0.6 m"),
        ("extrude_sides", "1"),
    ] {
        type_field(&mut a, &ctx, field, value);
        assert!(a.field_errors.is_empty(), "{field}: {:?}", a.field_errors);
        armed(&a, field);
    }
    choose_mode(&mut a, &ctx, "Shift");
    let unchanged = |a: &Operandi, step: &str| {
        assert_eq!(a.editor.scene, source, "{step}: source unchanged");
        assert_eq!(a.editor.scene.selection, source.selection, "{step}");
        assert!(a.editor.history.undo.is_empty(), "{step}: no history");
        assert!(a.editor.history.redo.is_empty(), "{step}: no redo");
    };
    unchanged(&a, "configured");
    let retained_uvs = armed(&a, "configured").extrude_uvs;
    assert_eq!(retained_uvs, reset.extrude_uvs, "fields keep Make UVs");

    // 1. Texture defined but explicitly unselected: all setters unavailable.
    choose_uv_map(&mut a, &ctx, NO_MAP);
    assert_eq!(a.status, "Ready", "deselect");
    let p = armed(&a, "No UV map selected");
    assert!(!p.extrude_uv_selected);
    assert_eq!(p.extrude_uvs, retained_uvs, "deselecting keeps the choice");
    let nodes = frame(&mut a, &ctx, vec![]);
    assert_eq!(uv_map_caption(&a, &nodes), NO_MAP);
    let s = shown(&mut a, &ctx);
    assert_eq!(
        s.uvs_disabled, [true; 3],
        "unselected map: setters disabled"
    );
    assert!(!s.apply_disabled);
    unchanged(&a, "No UV map selected");

    // 2. Select Texture: caption and all three setters available. Each click
    // is accepted into the live parameters only; nothing is applied.
    choose_uv_map(&mut a, &ctx, TEXTURE);
    assert_eq!(a.status, "Ready", "select Texture");
    let p = armed(&a, "Texture selected");
    assert!(p.extrude_uv_selected);
    assert_eq!(p.extrude_uv_map.as_deref(), Some(TEXTURE));
    assert_eq!(p.extrude_uvs, retained_uvs);
    unchanged(&a, "Texture selected");
    for (label, want) in [
        ("None", ExtrudeUvs::None),
        ("U", ExtrudeUvs::U),
        ("V", ExtrudeUvs::V),
    ] {
        let nodes = frame(&mut a, &ctx, vec![]);
        assert_eq!(uv_map_caption(&a, &nodes), TEXTURE, "{label}");
        let s = shown(&mut a, &ctx);
        assert_eq!(s.uvs_disabled, [false; 3], "{label}: setters enabled");
        click_uv_setter(&mut a, &ctx, label);
        assert_eq!(a.status, "Ready", "{label} click");
        assert!(a.field_errors.is_empty(), "{label}: {:?}", a.field_errors);
        let p = armed(&a, label);
        assert_eq!(p.extrude_uvs, want, "{label}: accepted choice");
        assert!(p.extrude_uv_selected, "{label}: Texture still selected");
        assert_eq!(p.extrude_uv_map.as_deref(), Some(TEXTURE));
        assert_eq!(p.extent, FIRST, "{label}: extent retained");
        assert_eq!(p.extrude_mode, ExtrudeMode::Shift, "{label}");
        unchanged(&a, label);
        let nodes = frame(&mut a, &ctx, vec![]);
        assert_eq!(uv_map_caption(&a, &nodes), TEXTURE, "{label} after");
        assert_eq!(shown(&mut a, &ctx).uvs_disabled, [false; 3]);
    }

    // 3. Deselect with V chosen: V and the map name are retained, setters
    // disabled, and a click on a disabled setter changes nothing.
    choose_uv_map(&mut a, &ctx, NO_MAP);
    assert_eq!(a.status, "Ready", "deselect after V");
    let want = configured_v_unselected(reset.clone());
    assert_eq!(armed(&a, "V retained"), want);
    let nodes = frame(&mut a, &ctx, vec![]);
    assert_eq!(uv_map_caption(&a, &nodes), NO_MAP);
    let s = shown(&mut a, &ctx);
    assert_eq!(s.uvs_disabled, [true; 3]);
    assert!(!s.apply_disabled, "Apply stays eligible");
    unchanged(&a, "V retained");
    assert!(source.active().unwrap().mesh.uv_maps.contains_key(TEXTURE));
    for label in ["None", "U"] {
        click_uv_setter(&mut a, &ctx, label);
        assert_eq!(armed(&a, label), want, "disabled {label}: V retained");
        unchanged(&a, label);
    }

    // 4. Apply: one Shift stage with retained V on the unselected map.
    click_apply(&mut a, &ctx);
    assert_eq!(a.status, "Ready", "Apply");
    let result = a.editor.scene.clone();
    snapshot("uv result", &result);
    assert_eq!(counts(&result), (8, 5), "EX-012 one stage");
    assert_eq!(armed(&a, "after Apply"), want);
    assert_eq!(labels(&a.editor.history.undo), ["Extrude"]);
    assert!(a.editor.history.redo.is_empty());
    assert_eq!(a.editor.history.undo[0].before, source);
    assert_eq!(a.editor.history.undo[0].after, result);
    assert_eq!(result, expected.scene, "whole scene vs Editor route");
    assert_eq!(result.selection, expected.scene.selection);
    assert_eq!(
        a.editor.session.live.as_ref().unwrap().params,
        expected.session.live.as_ref().unwrap().params
    );
    assert_selected_cap_at(&result, &source, FIRST, "uv result");
    let m = &result.active().unwrap().mesh;
    assert!(m.uv_maps.contains_key(TEXTURE), "Texture definition kept");
    let generated = &m.attributes.point_uvs[TEXTURE];
    assert!(!generated.is_empty(), "retained V generated Texture UVs");
    assert!(generated.values().all(|uv| uv.is_finite()));
    result.validate().unwrap();

    // 5. Drop with Q (press and release), then Undo/Redo via the Edit menu.
    frame(&mut a, &ctx, vec![]);
    assert!(!a.text_editor_focused(&ctx), "no text field focus before Q");
    let mut release = key(Key::Q);
    if let Event::Key { pressed, .. } = &mut release {
        *pressed = false;
    }
    frame(&mut a, &ctx, vec![key(Key::Q)]);
    frame(&mut a, &ctx, vec![release]);
    assert!(a.editor.session.live.is_none(), "Q drops: {}", a.status);
    assert_eq!(a.editor.scene, result, "Drop keeps the stage");
    assert_eq!(labels(&a.editor.history.undo), ["Extrude"]);
    for (action, want, step) in [
        (app_menu::line_action("app.undo"), &source, "Undo → source"),
        (app_menu::line_action("app.redo"), &result, "Redo → result"),
    ] {
        assert!(menu_enabled(&a, &ctx, action), "{step}: menu enabled");
        a.menu_action(&ctx, action);
        assert_eq!(a.status, "Ready", "{step}");
        assert_eq!(&a.editor.scene, want, "{step}: whole scene");
        assert_eq!(a.editor.scene.selection, want.selection, "{step}");
        let m = &a.editor.scene.active().unwrap().mesh;
        assert!(m.uv_maps.contains_key(TEXTURE), "{step}: definition");
        assert_eq!(
            m.attributes.point_uvs.contains_key(TEXTURE),
            std::ptr::eq(want, &result),
            "{step}: generated assignment presence"
        );
        assert!(a.editor.session.live.is_none(), "{step}: no tool restored");
        frame(&mut a, &ctx, vec![]);
    }
    assert_eq!(labels(&a.editor.history.undo), ["Extrude"]);
    assert!(a.editor.history.redo.is_empty());
}
