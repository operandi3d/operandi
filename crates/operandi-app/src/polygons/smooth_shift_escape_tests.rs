//! SS-U02/SS-U03 synthetic egui routing checks for Smooth Shift and Thicken:
//! a field draft Escape restores the confirmed value and keeps the stage live;
//! a later viewport Escape on the factory input map drops exactly like the
//! viewport Q Drop binding. Custom input maps keep their pre-existing binding
//! resolution: an action assigned to Escape, a remapped Drop, a multi-action
//! conflict, an unbound or rebound default Escape assignment, and Escape
//! assigned to an action that is unavailable or context-gated in that state
//! are never captured by the Smooth Shift fallback. No physical-input or native-focus
//! claim.
use super::tests::{draw, key, pointer};
use super::*;
use operandi_core::SmoothShiftParameters;

const SIZE: Vec2 = vec2(1440., 900.);
const CONFLICT: &str = "Input map conflict: assign unique keys before invoking this shortcut";

/// Each body runs as its own test per identity so a failure on one identity
/// does not hide the other.
macro_rules! per_identity {
    ($($name:ident => $body:ident;)*) => {$(
        mod $name {
            use super::*;
            #[test]
            fn smooth_shift() {
                $body(Tool::SmoothShift)
            }
            #[test]
            fn thicken() {
                $body(Tool::Thicken)
            }
        }
    )*};
}

per_identity! {
    default_map_field_escape_then_viewport_escape_drops_like_q => field_then_viewport_escape;
    default_map_armed_and_numeric_applied_escape_drops_like_q => armed_and_applied_escape;
    action_assigned_to_escape_keeps_its_invocation => action_on_escape;
    drop_remapped_to_escape_drops_without_conflict => drop_on_escape;
    multi_action_escape_conflict_is_unchanged => escape_conflict;
    unbound_or_rebound_default_escape_is_not_captured => default_escape_moved;
    unrelated_remap_keeps_default_escape_drop => unrelated_remap;
    unavailable_action_on_escape_is_not_captured => unavailable_action_on_escape;
}

/// Original 2 m cube with face 0 selected and the tool armed at Offset 0.2 m,
/// Scale 100 %, Max. Smoothing Angle 89.5° and all four options off.
fn armed_with(tool: Tool, ctx: &Context, bindings: &[(&str, &str)]) -> Operandi {
    let mut a = Operandi::empty();
    for &(action, chord) in bindings {
        a.bindings.insert(action.into(), chord.into());
    }
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
    a.activate(tool);
    for (name, value) in [
        ("offset", "0.2m"),
        ("scale", "100%"),
        ("angle", "89.5 deg"),
        ("thicken", "off"),
        ("sharp", "off"),
        ("angle_weight", "off"),
        ("both_sides", "off"),
    ] {
        a.execute(&format!("{} {name} {value}", tool.label()));
        assert_eq!(a.status, "Ready", "{tool:?} {name}");
    }
    draw(&mut a, ctx, SIZE, vec![]);
    a
}

fn armed(tool: Tool, ctx: &Context) -> Operandi {
    armed_with(tool, ctx, &[])
}

/// Armed, then the first ordinary viewport click establishes the live stage.
fn live_with(tool: Tool, ctx: &Context, bindings: &[(&str, &str)]) -> Operandi {
    let mut a = armed_with(tool, ctx, bindings);
    let source = a.editor.scene.clone();
    let at = empty_viewport(&a);
    click(&mut a, ctx, at);
    assert!(a.editor.session.live.as_ref().unwrap().applied);
    assert_ne!(a.editor.scene, source);
    a
}

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx
}

fn empty_viewport(a: &Operandi) -> Pos2 {
    a.viewport_rect.left_top() + vec2(20., 60.)
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    draw(
        a,
        ctx,
        SIZE,
        vec![
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    );
}

/// A physical tap: press then release. The shared `key` helper sends only the
/// press, so egui would report a later press of the same key as a repeat,
/// which the binding map deliberately ignores.
fn tap(k: Key) -> [Event; 2] {
    let Event::Key {
        physical_key,
        modifiers,
        ..
    } = key(k)
    else {
        unreachable!()
    };
    let release = Event::Key {
        key: k,
        physical_key,
        pressed: false,
        repeat: false,
        modifiers,
    };
    [key(k), release]
}

/// Tap `k` with the pointer resting over empty viewport space and no field
/// focused, as in the native observation after clicking the viewport.
fn viewport_key(a: &mut Operandi, ctx: &Context, k: Key) {
    let at = empty_viewport(a);
    let mut events = vec![Event::PointerMoved(at)];
    events.extend(tap(k));
    draw(a, ctx, SIZE, events);
}

fn history(a: &Operandi) -> (Vec<String>, Vec<String>) {
    let labels = |h: &[std::sync::Arc<Checkpoint>]| h.iter().map(|c| c.label.clone()).collect();
    (
        labels(&a.editor.history.undo),
        labels(&a.editor.history.redo),
    )
}

/// User-visible outcome. Independently built apps carry distinct session
/// tokens, so compare scene (geometry, selection, maps), history labels and
/// the active tool state rather than whole editors.
type Outcome = (
    Scene,
    (Vec<String>, Vec<String>),
    Option<(Tool, bool, SmoothShiftParameters)>,
);
fn outcome(a: &Operandi) -> Outcome {
    (
        a.editor.scene.clone(),
        history(a),
        a.editor
            .session
            .live
            .as_ref()
            .map(|l| (l.tool, l.applied, l.params.smooth_shift)),
    )
}

fn assert_dropped_like_q(escape: &Operandi, q: &Operandi, tool: Tool, label: &str) {
    assert!(
        q.editor.session.live.is_none(),
        "{tool:?} {label}: Q oracle must drop"
    );
    assert!(
        escape.editor.session.live.is_none(),
        "{tool:?} {label}: viewport Escape left the tool live"
    );
    assert_eq!(
        escape.editor.scene, q.editor.scene,
        "{tool:?} {label}: Escape Drop scene differs from Q Drop"
    );
    assert_eq!(history(escape), history(q), "{tool:?} {label}");
    assert_ne!(escape.status, CONFLICT, "{tool:?} {label}");
    let scene = &escape.editor.scene;
    let faces = scene.active().unwrap().mesh.faces.len();
    assert!(scene.selection.faces.iter().all(|&f| (f as usize) < faces));
}

/// Armed → first viewport click → unconfirmed Scale 50 % → field Escape →
/// empty viewport click, as in the native sequence. Returns the source scene.
fn live_after_field_escape(tool: Tool, ctx: &Context) -> (Operandi, Scene) {
    let mut a = armed(tool, ctx);
    let source = a.editor.scene.clone();
    let at = empty_viewport(&a);
    click(&mut a, ctx, at);
    assert!(a.editor.session.live.as_ref().unwrap().applied);
    assert_ne!(a.editor.scene, source);
    let live = a.editor.clone();
    assert_eq!(a.confirmed_fields["sst_scale"], "100 %");

    // SS-U02: Escape in a field resolves the unconfirmed draft first.
    let field = a.field_rects["sst_scale"].center();
    click(&mut a, ctx, field);
    a.fields.insert("sst_scale".into(), "50 %".into());
    draw(&mut a, ctx, SIZE, tap(Key::Escape).into());
    assert_eq!(a.fields["sst_scale"], "100 %", "{tool:?}");
    assert!(a.status.contains("restored confirmed value"), "{tool:?}");
    assert!(a.editor == live, "{tool:?}: field Escape changed the stage");
    assert!(!a.text_editor_focused(ctx));

    // Clicking empty viewport space keeps the stage live (observed).
    let at = empty_viewport(&a);
    click(&mut a, ctx, at);
    assert!(
        a.editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool == tool)
    );
    (a, source)
}

fn field_then_viewport_escape(tool: Tool) {
    let ctx = context();
    let (mut q, _) = live_after_field_escape(tool, &ctx);
    viewport_key(&mut q, &ctx, Key::Q);
    let ctx = context();
    let (mut a, source) = live_after_field_escape(tool, &ctx);

    // SS-U03: viewport Escape drops the valid preview and deactivates.
    viewport_key(&mut a, &ctx, Key::Escape);
    assert_dropped_like_q(&a, &q, tool, "live stage");
    assert_eq!(a.editor.history.undo.last().unwrap().label, tool.label());
    let committed = a.editor.scene.clone();
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, source, "{tool:?}");
    a.run(Command::Redo);
    assert_eq!(a.editor.scene, committed, "{tool:?}");
}

fn armed_and_applied_escape(tool: Tool) {
    for applied in [false, true] {
        let ctx = context();
        let mut a = armed(tool, &ctx);
        let source = a.editor.scene.clone();
        if applied {
            a.run(Command::Apply);
            assert_ne!(a.editor.scene, source);
        }
        let mut q = armed(tool, &ctx);
        if applied {
            q.run(Command::Apply);
        }
        assert_eq!(outcome(&a), outcome(&q));
        viewport_key(&mut q, &ctx, Key::Q);
        viewport_key(&mut a, &ctx, Key::Escape);
        let label = if applied { "applied" } else { "armed" };
        assert_dropped_like_q(&a, &q, tool, label);
        if !applied {
            assert_eq!(a.editor.scene, source, "{tool:?}: armed Drop edited");
        }
    }
}

/// Move = Escape (Drop Cube keeps its factory Escape). The pre-existing map
/// resolution gated Drop Cube to Box, so Escape reached Move alone; it must
/// still invoke Move exactly as the shared "Move" command does.
fn action_on_escape(tool: Tool) {
    let custom = [("Move", "Escape")];
    let ctx = context();
    let mut oracle = live_with(tool, &ctx, &custom);
    oracle.execute("Move");
    assert!(
        oracle
            .editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool == Tool::Move)
    );
    let ctx = context();
    let mut a = live_with(tool, &ctx, &custom);
    viewport_key(&mut a, &ctx, Key::Escape);
    assert_ne!(a.status, CONFLICT, "{tool:?}");
    assert_eq!(
        outcome(&a),
        outcome(&oracle),
        "{tool:?}: Escape did not reach Move"
    );
}

/// Drop = Escape: the explicit Drop action fires once, without a new conflict
/// against the default Drop Cube assignment, and matches Q on the factory map.
fn drop_on_escape(tool: Tool) {
    let ctx = context();
    let mut q = live_with(tool, &ctx, &[]);
    viewport_key(&mut q, &ctx, Key::Q);
    let ctx = context();
    let mut a = live_with(tool, &ctx, &[("Drop", "Escape")]);
    viewport_key(&mut a, &ctx, Key::Escape);
    assert_dropped_like_q(&a, &q, tool, "Drop=Escape");
}

/// Move = Escape and Rotate = Escape: the existing conflict report wins and
/// nothing is invoked; the live stage, scene and history stay as they were.
fn escape_conflict(tool: Tool) {
    let ctx = context();
    let mut a = live_with(tool, &ctx, &[("Move", "Escape"), ("Rotate", "Escape")]);
    let before = outcome(&a);
    viewport_key(&mut a, &ctx, Key::Escape);
    assert_eq!(a.status, CONFLICT, "{tool:?}");
    assert_eq!(outcome(&a), before, "{tool:?}: conflict changed state");
}

/// Removing or moving the factory Drop Cube = Escape assignment opts out of
/// the default route: Escape (and the new key) leave the stage live, as the
/// pre-existing resolution did.
fn default_escape_moved(tool: Tool) {
    for (chord, pressed) in [("", Key::Escape), ("F12", Key::Escape), ("F12", Key::F12)] {
        let ctx = context();
        let mut a = live_with(tool, &ctx, &[("Drop Cube", chord)]);
        let before = outcome(&a);
        viewport_key(&mut a, &ctx, pressed);
        assert_ne!(a.status, CONFLICT, "{tool:?} {chord:?}");
        assert_eq!(
            outcome(&a),
            before,
            "{tool:?}: Drop Cube={chord:?}, {pressed:?} was captured"
        );
    }
}

/// Remapping an unrelated action does not disable the factory Escape Drop.
fn unrelated_remap(tool: Tool) {
    let custom = [("Move", "T")];
    let ctx = context();
    let mut q = live_with(tool, &ctx, &custom);
    viewport_key(&mut q, &ctx, Key::Q);
    let ctx = context();
    let mut a = live_with(tool, &ctx, &custom);
    viewport_key(&mut a, &ctx, Key::Escape);
    assert_dropped_like_q(&a, &q, tool, "Move=T");
}

/// Escape assigned to an action that the existing map filters out while the
/// tool is live (New Mesh and Grow by live availability). Delete Item is no
/// longer one over a viewport: there it removes the selection (Jason,
/// 2026-11-10). Pre-existing resolution invoked nothing, so the
/// default Drop Cube fallback must not turn that no-action into a Drop.
fn unavailable_action_on_escape(tool: Tool) {
    let mut captured = Vec::new();
    for action in ["New Mesh", "Grow"] {
        let ctx = context();
        let mut a = live_with(tool, &ctx, &[(action, "Escape")]);
        let before = (outcome(&a), a.status.clone());
        viewport_key(&mut a, &ctx, Key::Escape);
        if (outcome(&a), a.status.clone()) != before {
            captured.push(action);
        }
    }
    assert!(
        captured.is_empty(),
        "{tool:?}: Escape assigned to {captured:?} was captured by the fallback"
    );
}
