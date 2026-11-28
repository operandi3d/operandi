//! BRI-R16/R18/R20 Bridge S1 recovery regression through the application's
//! confirmation, menu-dispatcher and Save routes (not direct core Save).
//!
//! Fixture: an original 2 m cube built with the Box tool, explicit ordered
//! opposed faces [0, 1], identity transform, no assigned maps, neutral
//! contexts and four unselected faces. Explicit S1 parameters: Segments 1,
//! Twist 0, Curve, Remove on, Flip/Reverse off, Tension 1 (100 %).
//!
//! The expected applied scene comes from a separate `Editor` dispatched with
//! identical inputs and parameters. It is an integration comparison between
//! two routes into the same project code, not independent reference geometry.
//! Field Escape uses synthetic egui events; no native focus or physical-input
//! claim is made here.
use super::tests::{draw, key, pointer};
use super::*;
use operandi_core::{BridgeMode, BridgeParameters};

// Tall enough to show the whole Polygon tab without scrolling; scrolling
// itself is covered by layout_tests::toolbox_scrolls_to_its_last_row.
const SIZE: Vec2 = vec2(1440., 1100.);

fn s1_parameters() -> BridgeParameters {
    BridgeParameters {
        segments: 1,
        twist: 0,
        mode: BridgeMode::Curve,
        remove: true,
        flip: false,
        reverse: false,
        tension: 1.,
    }
}

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx
}

/// Original 2 m cube, idle, with ordered polygon selection [0, 1].
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
    a.editor.scene.selection.faces = vec![0, 1];
    a.category = toolbox::Category::Polygon;
    a.camera.view = View::Perspective;
    a.frame(false);
    a
}

/// Arms Bridge through the menu dispatcher and sets every S1 parameter
/// explicitly through the canonical command route.
fn arm(a: &mut Operandi, ctx: &Context) {
    a.menu_action(ctx, app_menu::line_action("Bridge"));
    assert!(a.editor.session.bridge.is_some(), "{}", a.status);
    for (attr, value) in [
        ("segments", "1"),
        ("twist", "0"),
        ("mode", "curve"),
        ("remove", "on"),
        ("flip", "off"),
        ("orient", "off"),
        ("tension", "1"),
    ] {
        a.execute(&format!("tool.attr edge.bridge {attr} {value}"));
        assert!(
            a.status.starts_with("Bridge configured"),
            "{attr}: {}",
            a.status
        );
    }
    assert_eq!(
        a.editor.session.bridge.as_ref().unwrap().requested,
        s1_parameters()
    );
    draw(a, ctx, SIZE, vec![]);
    assert!(
        a.field_rects.contains_key("bri_twist"),
        "Bridge panel drawn"
    );
}

/// Separate Editor dispatch with the same source scene and parameters.
fn editor_expected(source: &Scene) -> Scene {
    let mut e = Editor {
        scene: source.clone(),
        ..Default::default()
    };
    e.dispatch(Command::ArmBridge).unwrap();
    e.dispatch(Command::ConfigureBridge(s1_parameters()))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.scene
}

/// A physical tap: press then release (see smooth_shift_escape_tests).
fn tap(k: Key) -> Vec<Event> {
    let Event::Key {
        physical_key,
        modifiers,
        ..
    } = key(k)
    else {
        unreachable!()
    };
    vec![
        key(k),
        Event::Key {
            key: k,
            physical_key,
            pressed: false,
            repeat: false,
            modifiers,
        },
    ]
}

/// Focuses a Bridge field with a synthetic click, places `draft` in it and
/// delivers a synthetic Escape.
fn field_escape(a: &mut Operandi, ctx: &Context, field: &str, draft: &str) {
    draw(a, ctx, SIZE, vec![]);
    let at = a.field_rects[field].center();
    draw(
        a,
        ctx,
        SIZE,
        vec![
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    );
    a.fields.insert(field.into(), draft.into());
    draw(a, ctx, SIZE, tap(Key::Escape));
    assert!(!a.text_editor_focused(ctx), "{field}: Escape keeps focus");
}

fn labels(h: &[std::sync::Arc<Checkpoint>]) -> Vec<String> {
    h.iter().map(|c| c.label.clone()).collect()
}

/// Application state that a refused route must leave untouched. The Editor
/// comparison covers scene, ordered selection, Bridge session/parameters,
/// undo, redo, revision and every remembered preset.
struct Frozen {
    editor: Editor,
    fields: BTreeMap<String, String>,
    confirmed: BTreeMap<String, String>,
    saved: Scene,
    path: Option<PathBuf>,
}
impl Frozen {
    fn of(a: &Operandi) -> Self {
        Self {
            editor: a.editor.clone(),
            fields: a.fields.clone(),
            confirmed: a.confirmed_fields.clone(),
            saved: a.saved.clone(),
            path: a.path.clone(),
        }
    }
    fn assert_unchanged(&self, a: &Operandi, what: &str) {
        assert_eq!(a.editor.scene, self.editor.scene, "{what}: scene");
        assert_eq!(
            labels(&a.editor.history.undo),
            labels(&self.editor.history.undo),
            "{what}: undo"
        );
        assert_eq!(
            labels(&a.editor.history.redo),
            labels(&self.editor.history.redo),
            "{what}: redo"
        );
        assert_eq!(
            a.editor.session.bridge, self.editor.session.bridge,
            "{what}: Bridge session"
        );
        assert!(
            a.editor == self.editor,
            "{what}: Editor fields changed: {:?}",
            editor_differences(&a.editor, &self.editor)
        );
        assert_eq!(a.fields, self.fields, "{what}: field drafts");
        assert_eq!(
            a.confirmed_fields, self.confirmed,
            "{what}: confirmed fields"
        );
        assert_eq!(a.saved, self.saved, "{what}: saved baseline");
        assert_eq!(a.path, self.path, "{what}: document path");
    }
    /// Field Escape is an explicit correction: `panels.rs` revalidates the
    /// restored text, which re-dispatches `ConfigureBridge` with identical
    /// parameters. That successful dispatch advances only the revision counter
    /// and the identity runtime's publication/request bookkeeping; authored
    /// identities, scene, history, Bridge session and fields must be equal.
    fn assert_unchanged_after_revalidation(&self, a: &Operandi, what: &str) {
        let changed = editor_differences(&a.editor, &self.editor);
        assert!(
            changed
                .iter()
                .all(|f| ["identities", "revision"].contains(f)),
            "{what}: Editor fields changed: {changed:?}"
        );
        assert_eq!(a.editor.identities.current, self.editor.identities.current);
        assert_eq!(
            a.editor.identities.epoch, self.editor.identities.epoch,
            "{what}: identity epoch"
        );
        assert!(
            a.editor.revision >= self.editor.revision,
            "{what}: revision"
        );
        // Normalize only the permitted counters. The remaining IdentityRuntime
        // (current, epoch, allocation owner, observed scene, record_before) is
        // compared through its own equality. Allocation-owner equality is
        // `Arc` identity: it proves the same owner, not unchanged mutable
        // allocation marks, which this test cannot observe.
        let mut normalized = a.editor.clone();
        normalized.revision = self.editor.revision;
        normalized.identities.publication = self.editor.identities.publication;
        normalized.identities.request = self.editor.identities.request;
        assert!(
            normalized.identities == self.editor.identities,
            "{what}: identity runtime beyond publication/request changed"
        );
        assert!(normalized == self.editor, "{what}: normalized Editor");
        assert_eq!(a.fields, self.fields, "{what}: field drafts");
        assert_eq!(
            a.confirmed_fields, self.confirmed,
            "{what}: confirmed fields"
        );
        assert_eq!(a.saved, self.saved, "{what}: saved baseline");
        assert_eq!(a.path, self.path, "{what}: document path");
    }
}

/// Names of the Editor fields that differ, so a failure says what changed.
fn editor_differences(a: &Editor, b: &Editor) -> Vec<&'static str> {
    let mut out = Vec::new();
    macro_rules! compare {
        ($($($field:ident).+),*) => {$(
            if a.$($field).+ != b.$($field).+ {
                out.push(stringify!($($field).+));
            }
        )*};
    }
    compare!(
        presets,
        i2,
        identities,
        bevel_states,
        bevel_state,
        bevel_profile,
        spatial_falloff,
        surface_states,
        surface_state,
        session.bridge,
        session.spike,
        spike_strength,
        session.ellipsoid,
        session.cone,
        scene,
        action_context,
        session.live,
        history.undo,
        history.redo,
        revision,
        next_item_id,
        selection_start,
        extrude_moved_caps,
        sphere_unit_resolution,
        cylinder_unit_resolution
    );
    out
}

/// Private, task-owned destination inside this test's exclusive data root.
fn private_destination(a: &Operandi) -> PathBuf {
    let root = a.environment.paths().data_root().to_path_buf();
    let dir = root.join("bridge-recovery-save");
    std::fs::create_dir(&dir).expect("create private destination directory");
    let path = dir.join("applied.opscene");
    assert!(path.starts_with(&root) && !path.exists());
    path
}

/// A. Invalid Segments/Twist/Tension drafts block the panel Apply route and
/// typed `tool.apply`; field Escape restores, and a valid correction applies
/// once atomically.
#[test]
fn invalid_draft_blocks_apply_then_escape_and_correction_apply_once() {
    for (field, bad, correction) in [
        ("bri_segments", "1+", "2-1"),
        ("bri_twist", "bad", "0*5"),
        ("bri_tension", "12x%", "50%*2"),
    ] {
        let ctx = context();
        let mut a = cube();
        let source = a.editor.scene.clone();
        let expected = editor_expected(&source);
        arm(&mut a, &ctx);
        assert_eq!(a.editor.scene, source, "arming edited the source");
        let armed = Frozen::of(&a);

        a.fields.insert(field.into(), bad.into());
        let drafted = Frozen::of(&a);
        // `Command::Apply` is the panel Apply button's route.
        a.run(Command::Apply);
        drafted.assert_unchanged(&a, field);
        assert!(a.field_errors.contains_key(field), "{field}: no error");
        assert_eq!(a.status, a.field_errors[field]);
        a.execute("tool.apply");
        drafted.assert_unchanged(&a, field);
        assert_eq!(a.fields[field], bad);

        field_escape(&mut a, &ctx, field, bad);
        assert!(a.field_errors.is_empty(), "{field}: stale error");
        assert_eq!(a.fields[field], armed.confirmed[field]);
        armed.assert_unchanged_after_revalidation(&a, field);

        a.fields.insert(field.into(), correction.into());
        a.run(Command::Apply);
        assert_eq!(a.editor.scene, expected, "{field}: integration comparison");
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 8);
        assert_eq!(
            a.editor.history.undo.len(),
            armed.editor.history.undo.len() + 1
        );
        assert!(
            a.editor
                .history
                .undo
                .last()
                .unwrap()
                .label
                .contains("Bridge")
        );
        assert!(a.editor.history.redo.is_empty());
        assert_eq!(
            a.editor.session.bridge.as_ref().unwrap().requested,
            s1_parameters()
        );
        assert!(a.field_errors.is_empty());
    }
}

/// B. Invalid Bridge drafts block File → Save while memory (undone source)
/// differs from the saved Applied file; the destination bytes, Editor,
/// undo/redo and drafts survive. A later valid Save writes the current scene.
#[test]
fn invalid_draft_save_keeps_destination_bytes_history_and_redo() {
    for (field, bad) in [("bri_tension", "12x%"), ("bri_segments", "3..")] {
        let ctx = context();
        let mut a = cube();
        let source = a.editor.scene.clone();
        let expected = editor_expected(&source);
        arm(&mut a, &ctx);
        a.run(Command::Apply);
        assert_eq!(a.editor.scene, expected, "{field}: integration comparison");
        let applied_undo = labels(&a.editor.history.undo);

        let path = private_destination(&a);
        assert!(a.save(path.clone()), "{field}: {}", a.status);
        let applied_bytes = std::fs::read(&path).unwrap();
        // Existing contract: a successful Save commits by dropping the session.
        assert!(
            a.editor.session.bridge.is_none(),
            "{field}: valid Save kept Bridge"
        );
        assert_eq!(persistence::load(&path).unwrap(), expected);
        assert_eq!(a.saved, expected);

        // Undo the actual Bridge Apply through the menu dispatcher.
        a.menu_action(&ctx, app_menu::line_action("app.undo"));
        assert_eq!(a.editor.scene, source, "{field}: {}", a.status);
        assert_eq!(a.editor.scene.selection.faces, vec![0, 1]);
        assert_eq!(a.editor.scene.selection.mode, Mode::Polygon);
        assert!(
            a.editor
                .history
                .redo
                .last()
                .unwrap()
                .label
                .contains("Bridge")
        );
        assert_eq!(labels(&a.editor.history.undo).len() + 1, applied_undo.len());
        assert!(a.dirty());

        arm(&mut a, &ctx);
        assert_eq!(a.editor.scene, source, "{field}: re-arming edited");
        a.fields.insert(field.into(), bad.into());
        let before = Frozen::of(&a);
        let redo = labels(&a.editor.history.redo);
        a.menu_action(&ctx, app_menu::line_action("scene.save"));
        assert_eq!(
            std::fs::read(&path).unwrap(),
            applied_bytes,
            "{field}: destination bytes changed"
        );
        before.assert_unchanged(&a, field);
        assert_eq!(labels(&a.editor.history.redo), redo, "{field}: redo lost");
        assert!(a.editor.session.bridge.is_some(), "{field}: hidden Drop");
        assert_eq!(a.fields[field], bad);
        assert!(a.field_errors.contains_key(field));
        assert_eq!(a.status, a.field_errors[field]);
        assert!(a.dirty());
        let reloaded = persistence::load(&path).unwrap();
        reloaded.validate().unwrap();
        assert_eq!(reloaded, expected);

        // Correction, then a valid Save writes the current (undone) scene.
        field_escape(&mut a, &ctx, field, bad);
        assert!(a.field_errors.is_empty());
        a.menu_action(&ctx, app_menu::line_action("scene.save"));
        assert_eq!(a.status, "Scene saved", "{field}");
        assert_ne!(std::fs::read(&path).unwrap(), applied_bytes);
        assert_eq!(persistence::load(&path).unwrap(), source);
        assert_eq!(a.saved, source);
        assert!(!a.dirty());

        // Redo still reproduces the Applied scene, which saves to the same bytes.
        a.menu_action(&ctx, app_menu::line_action("app.redo"));
        assert_eq!(a.editor.scene, expected, "{field}: {}", a.status);
        a.menu_action(&ctx, app_menu::line_action("scene.save"));
        assert_eq!(std::fs::read(&path).unwrap(), applied_bytes);
    }
}

/// C. Invalid Twist blocks menu Undo Bridge Apply without losing the Applied
/// scene, history or draft; Escape then Undo restores the ordered source and
/// Redo/Apply work. Empty and one-face selections refuse Apply and recover.
#[test]
fn invalid_twist_blocks_menu_undo_and_selection_refusals_recover() {
    let ctx = context();
    let mut a = cube();
    let source = a.editor.scene.clone();
    let expected = editor_expected(&source);
    arm(&mut a, &ctx);
    a.run(Command::Apply);
    assert_eq!(a.editor.scene, expected, "integration comparison");
    let applied = Frozen::of(&a);

    a.fields.insert("bri_twist".into(), "bad".into());
    let drafted = Frozen::of(&a);
    a.menu_action(&ctx, app_menu::line_action("app.undo"));
    drafted.assert_unchanged(&a, "invalid Twist Undo");
    assert!(a.field_errors.contains_key("bri_twist"));

    field_escape(&mut a, &ctx, "bri_twist", "bad");
    assert_eq!(a.fields["bri_twist"], "0");
    assert!(a.field_errors.is_empty());
    applied.assert_unchanged_after_revalidation(&a, "Twist Escape");

    a.menu_action(&ctx, app_menu::line_action("app.undo"));
    assert_eq!(a.editor.scene, source, "{}", a.status);
    assert_eq!(a.editor.scene.selection.faces, vec![0, 1]);
    assert_eq!(labels(&a.editor.history.redo).len(), 1);
    a.menu_action(&ctx, app_menu::line_action("app.redo"));
    assert_eq!(a.editor.scene, expected, "{}", a.status);
    a.menu_action(&ctx, app_menu::line_action("app.undo"));
    assert_eq!(a.editor.scene, source);
    // Observed contract: menu Undo/Redo keep the armed session and parameters.
    assert_eq!(
        a.editor.session.bridge.as_ref().unwrap().requested,
        s1_parameters()
    );
    a.run(Command::Apply);
    assert_eq!(a.editor.scene, expected, "Apply after Undo");
    assert!(a.editor.history.redo.is_empty());

    // Empty and one-face ordered selections (set with the fixture's direct
    // selection convention before arming): Apply does nothing, with no
    // message (bridge-2026-11-05, BR18/BR19).
    for faces in [vec![], vec![1]] {
        let mut b = cube();
        b.editor.scene.selection.faces = faces.clone();
        let source = b.editor.scene.clone();
        arm(&mut b, &ctx);
        let armed = Frozen::of(&b);
        for route in ["Apply", "tool.apply"] {
            if route == "Apply" {
                b.run(Command::Apply);
            } else {
                b.execute("tool.apply");
            }
            assert_eq!(b.editor.scene, source, "{faces:?} {route}: scene");
            assert_eq!(
                labels(&b.editor.history.undo),
                labels(&armed.editor.history.undo),
                "{faces:?} {route}: undo"
            );
            assert_eq!(
                b.editor.session.bridge, armed.editor.session.bridge,
                "{faces:?} {route}"
            );
        }

        b.execute("tool.drop");
        assert!(b.editor.session.bridge.is_none());
        b.editor.scene.selection.faces = vec![0, 1];
        let eligible = b.editor.scene.clone();
        arm(&mut b, &ctx);
        b.run(Command::Apply);
        assert_eq!(b.editor.scene, editor_expected(&eligible), "{faces:?}");
        assert_eq!(
            b.editor.history.undo.len(),
            armed.editor.history.undo.len() + 1
        );
    }
}
