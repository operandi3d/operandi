//! SS-U03/SS-U05 and PT-THK-04/07: live Thicken checkbox wiring with the
//! Thicken option on (retained-base shell), driven through synthetic egui
//! pointer events on the real panel checkboxes and buttons.
//!
//! Fixture: an original 2 m cube built with the Box tool, face 0 selected,
//! identity transform, no assigned maps, neutral contexts. Explicit
//! parameters: Offset 0.2 m, Scale 100 %, Max. Smoothing Angle 89.5°, Thicken
//! on, Sharp / Polygon Angle Weighting / Both Sides off.
//!
//! Every expected scene comes from a separate `Editor` started from the same
//! source scene and dispatched with explicit parameters. It is an integration
//! comparison between two routes into the same project kernel, not
//! independent reference geometry; no Thicken geometry acceptance is claimed.
//! Widget rectangles and displayed checked states are read from egui's own
//! AccessKit output; no production instrumentation is added. No native
//! input, focus, display-scale or reopen-in-GUI claim is made here.
use super::tests::{draw, pointer};
use super::*;
use egui::accesskit::{self, Role, Toggled};
use operandi_core::SmoothShiftParameters;

const SIZE: Vec2 = vec2(1440., 900.);
const OPTIONS: [&str; 4] = ["Thicken", "Sharp", "Polygon Angle Weighting", "Both Sides"];

fn explicit(thicken: bool, both_sides: bool) -> SmoothShiftParameters {
    SmoothShiftParameters {
        offset: 0.2,
        scale: 1.,
        angle: 89.5,
        thicken,
        sharp: false,
        angle_weight: false,
        both_sides,
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

/// Activates Thicken and sets every parameter explicitly through the
/// existing command route, so the stage never depends on retained values.
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
    assert_eq!(params(a), explicit(true, false));
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

/// The surface panel's option rows: each label paired with the one checkbox
/// that shares its row, below the Max. Smoothing Angle field.
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

fn displayed(a: &mut Operandi, ctx: &Context) -> [bool; 4] {
    let nodes = frame(a, ctx, vec![]);
    let c = checkboxes(a, &nodes);
    [c[0].1, c[1].1, c[2].1, c[3].1]
}

fn shown(p: SmoothShiftParameters) -> [bool; 4] {
    [p.thicken, p.sharp, p.angle_weight, p.both_sides]
}

/// Hover, press and release at the widget centre over three passes.
fn click_at(a: &mut Operandi, ctx: &Context, at: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(at)]);
    frame(a, ctx, vec![pointer(at, true, Modifiers::NONE)]);
    frame(a, ctx, vec![pointer(at, false, Modifiers::NONE)]);
}

fn click_option(a: &mut Operandi, ctx: &Context, label: &str) {
    let nodes = frame(a, ctx, vec![]);
    let i = OPTIONS.iter().position(|o| *o == label).unwrap();
    let at = checkboxes(a, &nodes)[i].0.center();
    click_at(a, ctx, at);
}

/// The one panel button with this label below the option rows.
fn click_button(a: &mut Operandi, ctx: &Context, label: &str) {
    let nodes = frame(a, ctx, vec![]);
    let below = checkboxes(a, &nodes)[3].0.bottom();
    let hits: Vec<Rect> = nodes
        .iter()
        .filter(|n| n.role() == Role::Button && n.label() == Some(label))
        .map(rect)
        .filter(|r| r.top() >= below && r.left() >= a.field_rects["sst_angle"].left() - 400.)
        .collect();
    assert_eq!(hits.len(), 1, "{label}: one panel button");
    click_at(a, ctx, hits[0].center());
}

fn labels(h: &[std::sync::Arc<Checkpoint>]) -> Vec<String> {
    h.iter().map(|c| c.label.clone()).collect()
}

fn faces(s: &Scene) -> usize {
    s.active().unwrap().mesh.faces.len()
}

/// Armed cube, then the first ordinary viewport click starts the live stage.
/// Returns the app and the pre-stage source scene.
fn live(ctx: &Context) -> (Operandi, Scene, Vec<String>) {
    let mut a = cube();
    arm(&mut a);
    frame(&mut a, ctx, vec![]);
    let source = a.editor.scene.clone();
    let history = labels(&a.editor.history.undo);
    assert_eq!(displayed(&mut a, ctx), shown(explicit(true, false)));
    let at = a.viewport_rect.left_top() + vec2(20., 60.);
    click_at(&mut a, ctx, at);
    assert!(
        a.editor.session.live.as_ref().unwrap().applied,
        "{}",
        a.status
    );
    assert_eq!(params(&a), explicit(true, false));
    assert_eq!(
        a.editor.scene,
        editor_expected(&source, explicit(true, false))
    );
    (a, source, history)
}

/// Toggles through the real checkboxes and checks each replacement against a
/// fresh Editor stage from the source. Returns the four distinct stages.
fn toggle_sequence(a: &mut Operandi, ctx: &Context, source: &Scene) -> Vec<Scene> {
    let mut seen = vec![a.editor.scene.clone()];
    for (label, thicken, both_sides) in [
        ("Both Sides", true, true),
        ("Both Sides", true, false),
        ("Thicken", false, false),
        ("Thicken", true, false),
    ] {
        click_option(a, ctx, label);
        let p = explicit(thicken, both_sides);
        assert_eq!(
            params(a),
            p,
            "{label} → {thicken}/{both_sides}: {}",
            a.status
        );
        assert_eq!(displayed(a, ctx), shown(p), "{label}: displayed state");
        assert!(a.editor.session.live.as_ref().unwrap().applied);
        assert!(
            a.editor
                .session
                .live
                .as_ref()
                .unwrap()
                .preview_error
                .is_none()
        );
        let expected = editor_expected(source, p);
        assert_eq!(a.editor.scene, expected, "{label} → {thicken}/{both_sides}");
        let sel = &a.editor.scene.selection;
        assert!(!sel.faces.is_empty());
        assert!(
            sel.faces
                .iter()
                .all(|&f| (f as usize) < faces(&a.editor.scene))
        );
        seen.push(a.editor.scene.clone());
    }
    // Discriminants: on/off states differ, and toggling back reproduces the
    // first stage from source instead of stacking another shell.
    let on = &seen[0];
    let both = &seen[1];
    let off = &seen[3];
    assert_ne!(on, source);
    assert_ne!(both, on);
    assert_ne!(off, on);
    assert_ne!(off, both);
    assert!(faces(on) > faces(off), "retained base adds faces");
    assert_eq!(&seen[2], on, "Both Sides off replaces from source");
    assert_eq!(&seen[4], on, "Thicken on replaces from source");
    seen
}

#[test]
fn thicken_on_live_checkboxes_replace_from_source_then_cancel_restores() {
    let ctx = context();
    let (mut a, source, history) = live(&ctx);
    let source_selection = source.selection.clone();
    toggle_sequence(&mut a, &ctx, &source);
    click_button(&mut a, &ctx, "Cancel");
    assert_eq!(
        a.editor.scene, source,
        "Cancel restores source: {}",
        a.status
    );
    assert_eq!(a.editor.scene.selection, source_selection);
    assert_eq!(
        labels(&a.editor.history.undo),
        history,
        "Cancel adds no history"
    );
    // Existing contract: explicit Cancel resolves the stage and deactivates.
    assert!(
        a.editor.session.live.is_none(),
        "Cancel deactivates: {}",
        a.status
    );
    assert!(
        a.editor.history.redo.is_empty(),
        "Cancel leaves nothing to redo"
    );
}

#[test]
fn thicken_on_live_checkboxes_drop_undo_redo_and_save_load() {
    let ctx = context();
    let (mut a, source, history) = live(&ctx);
    let stages = toggle_sequence(&mut a, &ctx, &source);
    let final_params = params(&a);
    assert_eq!(final_params, explicit(true, false));
    click_button(&mut a, &ctx, "Drop");
    assert!(
        a.editor.session.live.is_none(),
        "Drop deactivates: {}",
        a.status
    );
    let committed = a.editor.scene.clone();
    assert_eq!(committed, editor_expected(&source, final_params));
    assert_eq!(&committed, stages.last().unwrap());
    let after_drop = labels(&a.editor.history.undo);
    assert_eq!(after_drop.len(), history.len() + 1, "{after_drop:?}");
    assert_eq!(after_drop.last().unwrap(), "Thicken");

    a.run(Command::Undo);
    assert!(a.editor.session.live.is_none());
    assert_eq!(labels(&a.editor.history.redo), ["Thicken"]);
    assert_eq!(a.editor.scene, source, "Undo restores source: {}", a.status);
    assert_eq!(labels(&a.editor.history.undo), history);
    a.run(Command::Redo);
    assert_eq!(
        a.editor.scene, committed,
        "Redo restores shell: {}",
        a.status
    );
    assert_eq!(labels(&a.editor.history.undo), after_drop);
    assert!(
        a.editor.session.live.is_none(),
        "Redo restores the dropped shell only"
    );
    assert!(a.editor.history.redo.is_empty());
    // Retained Thicken settings are session state, untouched by history.
    assert_eq!(
        *a.editor.presets.get::<operandi_core::ThickenPreset>(),
        Some(final_params)
    );

    // Existing application Save route into this test's private data root.
    let root = a.environment.paths().data_root().to_path_buf();
    let dir = root.join("thicken-shell-live");
    std::fs::create_dir(&dir).expect("create private destination directory");
    let path = dir.join("shell.opscene");
    assert!(path.starts_with(&root) && !path.exists());
    assert!(a.save(path.clone()), "{}", a.status);
    assert_eq!(a.status, "Scene saved");
    let reloaded = persistence::load(&path).unwrap();
    reloaded.validate().unwrap();
    assert_eq!(reloaded, committed);
    assert_eq!(reloaded.selection, committed.selection);
}
