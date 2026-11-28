//! EX-001/005/012 and SA-001/002/004/005/010/011/012: cross-tool handover from
//! inactive numeric Polygon Extrude to inactive numeric Spike through the
//! actual tool panels. The covered question is the handover itself: the moved
//! cap selected by Extrude is the only Spike target, the four Extrude walls
//! survive the Spike stage, Spike leaves its partial-target fan descendants
//! selected, and both history stages keep their exact before/after scenes.
//!
//! Every tested step uses a production route: menu activation, synthetic egui
//! pointer/key/text events on the real Extent, Sides, Mode, Strength and Apply
//! widgets, the ordinary non-text Q key for Drop and Edit-menu Undo/Redo. No
//! tested field, confirmed field, live parameter or selection is written
//! directly.
//!
//! Fixture: an original 1 m planar quad (0,0,0),(1,0,0),(1,1,0),(0,1,0), face
//! [0,1,2,3] explicitly selected in Polygon mode, identity transform, neutral
//! contexts and no UV map definitions, so the Make UVs setters are unavailable
//! and the retained U applies (EX-002).
//!
//! Independent contract derivation: 4V/1F → Shift, Sides 1, +0.6 m Z Extrude →
//! 8V/5F with the moved cap at z 0.6 (EX-005/012) → Spike Strength 0.25 on
//! that cap only → 9V/8F (SA-004: V+1, F−1+4). The cap's corner mean is
//! (0.5,0.5,0.6), its mean edge length 1 and its first-corner normal +Z, so the
//! apex is (0.5,0.5,0.85) (SA-002/003). A separate `Editor` driven from the
//! same source supplies whole-scene integration expectations; that comparison
//! is integration evidence between two routes into one kernel, not independent
//! geometry parity. No native input, display-scale, persistence or native
//! history payload claim is made here.
use super::tests::{draw, key, pointer};
use super::*;
use egui::accesskit::{self, Role};
use operandi_core::{ExtrudeMode, ExtrudeUvs, Mesh};

const SIZE: Vec2 = vec2(1440., 900.);
const EXTENT: DVec3 = DVec3::new(0., 0., 0.6);
const STRENGTH: f64 = 0.25;
const APEX: DVec3 = DVec3::new(0.5, 0.5, 0.85);

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

fn configured(mut p: Parameters) -> Parameters {
    p.extent = EXTENT;
    p.extrude_sides = 1;
    p.extrude_mode = ExtrudeMode::Shift;
    p.extrude_uvs = ExtrudeUvs::U;
    p
}

/// Separate Editor: same source → Extrude Apply → Drop → Spike Apply → Drop.
/// Returns the editor after each Drop.
fn editor_expected(fixture: &Editor) -> (Editor, Editor) {
    let mut e = fixture.clone();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    let p = configured(e.session.live.as_ref().unwrap().params.clone());
    e.dispatch(Command::ConfigureExtrude(p)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let first = e.clone();
    e.dispatch(Command::ArmSpike).unwrap();
    e.dispatch(Command::ConfigureSpike(STRENGTH)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    (first, e)
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

/// Hover, press and release at a point over three passes.
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

fn field_text(a: &mut Operandi, ctx: &Context, field: &str) -> String {
    let nodes = frame(a, ctx, vec![]);
    text_input_in(&nodes, a.field_rects[field])
        .value()
        .unwrap_or_default()
        .to_string()
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

/// The one node with this role and text below `anchor` field in the tool
/// panel column.
fn panel_node(
    a: &Operandi,
    nodes: &[accesskit::Node],
    anchor: &str,
    role: Role,
    want: impl Fn(&str) -> bool,
) -> accesskit::Node {
    let field = a.field_rects[anchor];
    let hits: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == role && text(n).is_some_and(|t| want(&t)))
        .filter(|n| {
            let r = rect(n);
            r.top() >= field.bottom() && r.left() >= field.left() - 400.
        })
        .cloned()
        .collect();
    assert_eq!(hits.len(), 1, "one panel {role:?} below {anchor}: {hits:?}");
    hits[0].clone()
}

fn apply_button(a: &mut Operandi, ctx: &Context, anchor: &str) -> accesskit::Node {
    let nodes = frame(a, ctx, vec![]);
    panel_node(a, &nodes, anchor, Role::Button, |t| t == "Apply")
}

fn choose_mode(a: &mut Operandi, ctx: &Context, entry: &str) {
    let nodes = frame(a, ctx, vec![]);
    let combo = panel_node(a, &nodes, "extrude_sides", Role::ComboBox, |t| {
        matches!(t, "Auto" | "Shift" | "Thick")
    });
    click_at(a, ctx, rect(&combo).center());
    let nodes = frame(a, ctx, vec![]);
    assert!(egui::Popup::is_any_open(ctx), "Mode popup opened");
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

fn menu_enabled(a: &Operandi, ctx: &Context, action: app_menu::Action) -> bool {
    a.menu_model(ctx)
        .iter()
        .flat_map(app_menu::Menu::leaves)
        .any(|item| item.action == action && item.enabled)
}

/// Q with no text focus, through the keyboard input map: a press, then its
/// release. The shared `key` helper sends only the press; without the release
/// egui reports the next Q press as a repeat, which the input map ignores.
fn drop_with_q(a: &mut Operandi, ctx: &Context) {
    frame(a, ctx, vec![]);
    assert!(!a.text_editor_focused(ctx), "no text field focus before Q");
    frame(a, ctx, vec![key(Key::Q)]);
    frame(
        a,
        ctx,
        vec![Event::Key {
            key: Key::Q,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
}

fn labels(h: &[std::sync::Arc<Checkpoint>]) -> Vec<String> {
    h.iter().map(|c| c.label.clone()).collect()
}

fn mesh(s: &Scene) -> &Mesh {
    &s.active().unwrap().mesh
}

fn counts(s: &Scene) -> (usize, usize) {
    let m = mesh(s);
    (m.vertices.len(), m.faces.len())
}

fn loop_of(m: &Mesh, f: &Face) -> Vec<DVec3> {
    f.vertices.iter().map(|&v| m.vertices[v as usize]).collect()
}

/// Oriented-cycle equality allowing any starting corner (SA-004).
fn same_cycle(a: &[DVec3], b: &[DVec3]) -> bool {
    a.len() == b.len() && (0..b.len()).any(|r| (0..a.len()).all(|i| a[i] == b[(i + r) % b.len()]))
}

/// Face payload apart from its vertex references.
fn face_payload(f: &Face) -> Face {
    Face {
        vertices: Vec::new(),
        ..f.clone()
    }
}

/// Stage snapshot printed for the task evidence log (`--nocapture`).
fn snapshot(step: &str, s: &Scene) {
    let m = mesh(s);
    println!(
        "SNAPSHOT {step}: V{} F{} selection={:?} mode={:?} item={:?} uv_maps={:?} vertices={:?} faces={:?}",
        m.vertices.len(),
        m.faces.len(),
        s.selection.faces,
        s.selection.mode,
        s.selection.item,
        m.uv_maps,
        m.vertices,
        m.faces
            .iter()
            .map(|f| f.vertices.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn extrude_moved_cap_hands_over_to_partial_spike_with_walls_and_both_stages() {
    let ctx = context();
    let fixture = quad_editor();
    let (expected_first, expected_second) = editor_expected(&fixture);
    let mut a = quad_app(&fixture);
    let source = a.editor.scene.clone();
    snapshot("source", &source);
    assert_eq!(counts(&source), (4, 1));
    assert!(a.editor.history.undo.is_empty() && a.editor.history.redo.is_empty());

    // 1. Extrude through menu activation and the actual panel widgets.
    a.menu_action(&ctx, app_menu::line_action(Tool::Extrude.label()));
    assert_eq!(a.status, "Ready", "Extrude activation");
    let reset = a
        .editor
        .session
        .live
        .as_ref()
        .expect("Extrude armed")
        .params
        .clone();
    for (field, value) in [
        ("extent0", "0 m"),
        ("extent1", "0 m"),
        ("extent2", "0.6 m"),
        ("extrude_sides", "1"),
    ] {
        type_field(&mut a, &ctx, field, value);
        assert!(a.field_errors.is_empty(), "{field}: {:?}", a.field_errors);
    }
    choose_mode(&mut a, &ctx, "Shift");
    let l = a.editor.session.live.as_ref().expect("Extrude armed");
    assert_eq!(l.tool, Tool::Extrude);
    assert!(!l.applied, "configuration stays armed");
    assert_eq!(l.params, configured(reset), "{}", a.status);
    let nodes = frame(&mut a, &ctx, vec![]);
    for uv in ["None", "U", "V"] {
        assert!(
            panel_node(&a, &nodes, "extrude_sides", Role::RadioButton, |t| t == uv).is_disabled(),
            "Make UVs {uv} unavailable without a UV map"
        );
    }
    assert_eq!(a.editor.scene, source, "configuration: source unchanged");
    assert!(a.editor.history.undo.is_empty() && a.editor.history.redo.is_empty());

    let apply = apply_button(&mut a, &ctx, "extrude_sides");
    assert!(!apply.is_disabled(), "Extrude Apply enabled");
    click_at(&mut a, &ctx, rect(&apply).center());
    assert_eq!(a.status, "Ready", "Extrude Apply");
    let first = a.editor.scene.clone();
    snapshot("first", &first);
    assert_eq!(counts(&first), (8, 5), "EX-012 first stage");
    assert_eq!(labels(&a.editor.history.undo), ["Extrude"]);
    assert!(a.editor.history.redo.is_empty());

    drop_with_q(&mut a, &ctx);
    assert!(
        a.editor.session.live.is_none(),
        "Q drops Extrude: {}",
        a.status
    );
    assert_eq!(a.editor.scene, first, "Drop keeps the first stage");
    assert_eq!(
        labels(&a.editor.history.undo),
        ["Extrude"],
        "Drop adds no stage"
    );
    let first_history = a.editor.history.undo.clone();
    assert!(first_history[0].before == source && first_history[0].after == first);
    assert_eq!(first, expected_first.scene, "first: whole scene");

    // Independent first-stage contract: the only selected face is the moved
    // cap, the source corners in source order at z 0.6; the other four faces
    // are the walls.
    let m1 = mesh(&first);
    assert_eq!(first.selection.mode, Mode::Polygon);
    assert_eq!(first.selection.faces.len(), 1, "one moved cap selected");
    let cap_index = first.selection.faces[0] as usize;
    let cap = loop_of(m1, &m1.faces[cap_index]);
    let want_cap: Vec<DVec3> = loop_of(mesh(&source), &mesh(&source).faces[0])
        .into_iter()
        .map(|p| p + EXTENT)
        .collect();
    assert_eq!(cap, want_cap, "moved cap at z 0.6");
    let walls: Vec<(Vec<DVec3>, Face)> = m1
        .faces
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != cap_index)
        .map(|(_, f)| (loop_of(m1, f), face_payload(f)))
        .collect();
    assert_eq!(walls.len(), 4, "four Extrude walls");
    assert!(walls.iter().all(|(l, _)| l.len() == 4));
    assert!(m1.uv_maps.is_empty());
    first.validate().unwrap();

    // 2. Spike through menu activation. Handover takes the scene as it is:
    // nothing about the selection, editor or live state is set here.
    assert!(menu_enabled(&a, &ctx, app_menu::line_action("Spike")));
    a.menu_action(&ctx, app_menu::line_action("Spike"));
    let spike = a.editor.session.spike.clone().expect("Spike armed");
    assert_eq!(spike.target, first.active().unwrap().id, "{}", a.status);
    assert!(a.editor.session.live.is_none());
    assert_eq!(a.editor.scene, first, "Spike activation: first unchanged");
    assert_eq!(a.editor.scene.selection.faces, [cap_index as u32]);
    let shown = field_text(&mut a, &ctx, "spi_strength");
    let parsed = numeric::parse_quantity(&shown, numeric::Quantity::Scalar)
        .unwrap_or_else(|e| panic!("Strength shows {shown:?}: {e}"));
    assert_eq!(parsed, spike.requested, "displayed retained Strength");

    let unchanged = |a: &Operandi, step: &str| {
        assert_eq!(a.editor.scene, first, "{step}: full first scene");
        assert_eq!(a.editor.scene.selection, first.selection, "{step}");
        assert!(
            a.editor.history.undo == first_history,
            "{step}: first checkpoint"
        );
        assert!(a.editor.history.redo.is_empty(), "{step}");
    };

    type_field(&mut a, &ctx, "spi_strength", "0.25");
    assert!(a.field_errors.is_empty(), "{:?}", a.field_errors);
    assert_eq!(a.editor.session.spike.as_ref().unwrap().requested, STRENGTH);
    unchanged(&a, "Strength 0.25 configured");

    // Invalid dimensioned Strength: kept as a visible draft with an error;
    // Apply is unavailable and a click on it changes nothing.
    type_field(&mut a, &ctx, "spi_strength", "0.25 m");
    assert!(a.field_errors.contains_key("spi_strength"), "{}", a.status);
    assert_eq!(a.status, a.field_errors["spi_strength"], "error shown");
    assert_eq!(field_text(&mut a, &ctx, "spi_strength"), "0.25 m");
    let apply = apply_button(&mut a, &ctx, "spi_strength");
    assert!(apply.is_disabled(), "Spike Apply disabled for invalid text");
    click_at(&mut a, &ctx, rect(&apply).center());
    unchanged(&a, "refused Apply");
    assert_eq!(field_text(&mut a, &ctx, "spi_strength"), "0.25 m");
    let s = a
        .editor
        .session
        .spike
        .as_ref()
        .expect("still armed, no tool switch");
    assert_eq!(s.requested, STRENGTH, "last valid Strength kept");

    type_field(&mut a, &ctx, "spi_strength", "0.25");
    assert!(a.field_errors.is_empty(), "{:?}", a.field_errors);
    assert_eq!(a.editor.session.spike.as_ref().unwrap().requested, STRENGTH);
    unchanged(&a, "corrected");

    // 3. Spike Apply on the selected moved cap only.
    let apply = apply_button(&mut a, &ctx, "spi_strength");
    assert!(!apply.is_disabled(), "Spike Apply enabled");
    click_at(&mut a, &ctx, rect(&apply).center());
    let second = a.editor.scene.clone();
    snapshot("second", &second);
    assert_eq!(counts(&second), (9, 8), "SA-004: V+1, F-1+4");
    assert_eq!(labels(&a.editor.history.undo), ["Extrude", "Spike Apply"]);
    assert!(
        a.editor.history.undo[0] == first_history[0],
        "first stage preserved"
    );
    assert!(a.editor.history.undo[1].before == first && a.editor.history.undo[1].after == second);
    assert!(a.editor.history.redo.is_empty());
    assert!(
        a.editor.session.spike.is_some(),
        "Spike stays armed after Apply"
    );

    let m2 = mesh(&second);
    let tolerance = 2e-6 * 1f64.max(DVec3::new(1., 1., 0.85).length());
    let new_points: Vec<DVec3> = m2
        .vertices
        .iter()
        .copied()
        .filter(|p| !m1.vertices.contains(p))
        .collect();
    assert_eq!(new_points.len(), 1, "exactly one new apex");
    let apex = new_points[0];
    assert!((apex - APEX).length() <= tolerance, "apex {apex:?}");
    for p in &m1.vertices {
        assert!(m2.vertices.contains(p), "first-stage point {p:?} kept");
    }
    let mut faces: Vec<_> = (0..m2.faces.len()).collect();
    for (wall, payload) in &walls {
        let hit = faces
            .iter()
            .position(|&i| same_cycle(&loop_of(m2, &m2.faces[i]), wall))
            .unwrap_or_else(|| panic!("wall {wall:?} retained"));
        assert_eq!(&face_payload(&m2.faces[faces[hit]]), payload, "wall data");
        faces.remove(hit);
    }
    let cap_payload = face_payload(&m1.faces[cap_index]);
    let mut selected = second.selection.faces.clone();
    selected.sort_unstable();
    assert_eq!(
        selected,
        faces.iter().map(|&i| i as u32).collect::<Vec<_>>()
    );
    assert_eq!(second.selection.mode, Mode::Polygon);
    for (i, &f) in faces.iter().enumerate() {
        let tri = loop_of(m2, &m2.faces[f]);
        let want = [cap[i], cap[(i + 1) % 4], apex];
        assert!(
            faces
                .iter()
                .any(|&g| same_cycle(&loop_of(m2, &m2.faces[g]), &want)),
            "fan triangle {want:?}"
        );
        assert_eq!(tri.len(), 3, "fan face is a triangle");
        assert_eq!(face_payload(&m2.faces[f]), cap_payload, "fan from cap");
    }
    assert!(m2.uv_maps.is_empty());
    second.validate().unwrap();

    drop_with_q(&mut a, &ctx);
    assert!(
        a.editor.session.spike.is_none(),
        "Q drops Spike: {}",
        a.status
    );
    assert_eq!(a.editor.scene, second, "Drop keeps the Spike stage");
    assert_eq!(
        labels(&a.editor.history.undo),
        ["Extrude", "Spike Apply"],
        "no third"
    );
    assert_eq!(second, expected_second.scene, "second: whole scene");
    let both_stages = a.editor.history.undo.clone();

    // 4. Undo/Redo through the production Edit menu routing.
    for (action, want, undo, redo, step) in [
        (
            app_menu::line_action("app.undo"),
            &first,
            1,
            1,
            "Undo → first",
        ),
        (
            app_menu::line_action("app.undo"),
            &source,
            0,
            2,
            "Undo → source",
        ),
        (
            app_menu::line_action("app.redo"),
            &first,
            1,
            1,
            "Redo → first",
        ),
        (
            app_menu::line_action("app.redo"),
            &second,
            2,
            0,
            "Redo → second",
        ),
    ] {
        assert!(menu_enabled(&a, &ctx, action), "{step}: menu enabled");
        a.menu_action(&ctx, action);
        assert_eq!(a.status, "Ready", "{step}");
        assert_eq!(&a.editor.scene, want, "{step}: whole scene");
        assert_eq!(a.editor.scene.selection, want.selection, "{step}");
        assert_eq!(
            (a.editor.history.undo.len(), a.editor.history.redo.len()),
            (undo, redo)
        );
        assert!(
            a.editor.session.live.is_none() && a.editor.session.spike.is_none(),
            "{step}"
        );
        frame(&mut a, &ctx, vec![]);
    }
    assert_eq!(labels(&a.editor.history.undo), ["Extrude", "Spike Apply"]);
    for (i, c) in a.editor.history.undo.iter().enumerate() {
        assert!(
            c.before == both_stages[i].before && c.after == both_stages[i].after,
            "stage {i} payloads after Undo/Redo"
        );
    }
    let mut sel = a.editor.scene.selection.faces.clone();
    sel.sort_unstable();
    assert_eq!(sel, selected, "four fan descendants selected again");
}
