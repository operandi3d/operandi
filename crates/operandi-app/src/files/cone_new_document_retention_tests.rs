//! CA-CON-003/015 with NSM-001/002: a successful File > New replaces the
//! document but keeps the pure numeric Cone attributes stored by Drop.
//!
//! Fixture: the production New baseline plus a second, ordinary empty mesh
//! that owns an empty "Old Map" UV definition, so the old Cone target (item 5, after the new scene's camera, light and environment)
//! differs from the new document's first mesh (item 1). Cone is armed by the
//! command line, its choices are set through the production ConfigureCone
//! command and its numbers through the panel field-confirmation route, then
//! Apply and Drop run through the shared commands. New is requested through
//! the File action guard and accepted with the actual modal Discard button.
//!
//! Retained: Position, Size, Axis, Sides, Segments, Polygon Type, Caps, Edge
//! Weight, Make UVs, Distortion Compensation and Gaps. The selected UV map
//! names a map of the old document; it keeps its existing New default (None).
//! Cross-document map retention is unresolved and is not tested as policy.
//! Expected geometry comes from a separate Editor given the same parameters:
//! an integration comparison of two routes into one kernel, not a reference.
use super::tests::draw;
use super::*;
use egui::accesskit::{self, Role, Toggled};
use operandi_core::{ConeCap, ConeParameters, Mesh, PolygonRepresentation, scene::Transform};

const SIZE: Vec2 = vec2(1440., 900.);
const OLD_MAP: &str = "Old Map";

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

/// The nondefault, admitted positive-domain settings configured before New.
fn requested() -> ConeParameters {
    ConeParameters {
        position: DVec3::new(1., 2., 3.),
        size: DVec3::new(0.6, 0.8, 1.1),
        axis: 2,
        sides: 8,
        segments: 3,
        polygon: PolygonRepresentation::CatmullClark,
        top_cap: ConeCap::Single,
        bottom_cap: ConeCap::None,
        edge_weight: 37.,
        make_uvs: false,
        compensation: true,
        gaps: 7.,
        selected_uv_map: Some(OLD_MAP.into()),
        falloff: Default::default(),
    }
}

/// Production New baseline, then a second mesh holding an empty old UV map.
fn old_document() -> Operandi {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a.run(Command::NewMesh);
    let item = a.editor.scene.active_mut().unwrap();
    let mut mesh = (*item.mesh).clone();
    mesh.uv_maps.insert(OLD_MAP.into(), Vec::new());
    item.mesh = mesh.into();
    a.editor.scene.validate().unwrap();
    a.saved = a.editor.scene.clone();
    a
}

/// Arm, configure, Apply and Drop numeric Cone through the app routes.
fn apply_and_drop_cone(a: &mut Operandi) {
    a.execute("Cone");
    assert_eq!(
        a.editor.session.cone.as_ref().unwrap().target,
        5,
        "the second mesh, after the camera, light and environment"
    );
    let want = requested();
    let mut choices = a.editor.session.cone.as_ref().unwrap().requested.clone();
    choices.axis = want.axis;
    choices.polygon = want.polygon;
    choices.bottom_cap = want.bottom_cap;
    choices.make_uvs = want.make_uvs;
    choices.compensation = want.compensation;
    choices.selected_uv_map = want.selected_uv_map.clone();
    a.run(Command::ConfigureCone(choices));
    for (key, value) in [
        ("cone_position0", "1 m"),
        ("cone_position1", "2 m"),
        ("cone_position2", "3 m"),
        ("cone_size0", "0.6 m"),
        ("cone_size1", "0.8 m"),
        ("cone_size2", "1.1 m"),
        ("cone_sides", "8"),
        ("cone_segments", "3"),
        ("cone_weight", "37"),
        ("cone_gaps", "7"),
    ] {
        a.fields.insert(key.into(), value.into());
    }
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(a.editor.session.cone.as_ref().unwrap().requested, want);
    a.execute("Apply");
    assert!(!a.editor.scene.items[4].mesh.faces.is_empty());
    a.execute("Drop");
    assert!(a.editor.session.cone.is_none());
    assert_eq!(
        *a.editor.presets.get::<operandi_core::ConePreset>(),
        want,
        "Drop stores the requested values"
    );
    assert!(a.dirty());
}

fn assert_clean_new_document(a: &Operandi) {
    let scene = &a.editor.scene;
    // The empty mesh, then the new scene's camera, light (CL-1) and
    // environment (ENV §1).
    assert_eq!(scene.items.len(), 4);
    assert!(scene.items[1].camera.is_some() && scene.items[2].light.is_some());
    let item = scene.active().expect("initial mesh is active");
    assert_eq!((item.id, item.name.as_str()), (1, "Mesh"));
    assert_eq!(item.transform, Transform::default());
    assert_eq!(*item.mesh, Mesh::default(), "no old geometry or maps");
    assert_eq!(scene.selection.item, Some(1));
    assert_eq!(scene, &a.saved);
    assert!(!a.dirty());
    assert!(a.path.is_none());
    assert!(a.editor.history.undo.is_empty() && a.editor.history.redo.is_empty());
    assert!(a.editor.session.live.is_none());
    assert!(
        a.editor.session.cone.is_none(),
        "no old Cone session or preview"
    );
}

#[test]
fn file_new_discard_retains_pure_cone_attributes_only() {
    let mut a = old_document();
    apply_and_drop_cone(&mut a);
    let ctx = context();
    // New opens a scene beside the old one; nothing to discard.
    a.request_file(FileAction::New);
    assert!(a.pending.is_none());
    assert_clean_new_document(&a);
    let retained = ConeParameters {
        selected_uv_map: None,
        ..requested()
    };
    assert_eq!(
        *a.editor.presets.get::<operandi_core::ConePreset>(),
        retained,
        "CA-CON-003/015 retention"
    );

    // Reactivate on the new first mesh and read the panel's own widgets.
    a.execute("Cone");
    let session = a.editor.session.cone.as_ref().expect("Cone armed");
    assert_eq!(session.target, 1);
    assert_eq!(session.requested, retained);
    assert!(session.preview.is_none() && session.error.is_none());
    frame(&mut a, &ctx, vec![]);
    let nodes = frame(&mut a, &ctx, vec![]);
    let shown = |key: &str| {
        let area = a.field_rects[key];
        let hits: Vec<_> = nodes
            .iter()
            .filter(|n| n.role() == Role::TextInput && area.contains(rect(n).center()))
            .collect();
        assert_eq!(hits.len(), 1, "one visible input for {key}");
        hits[0].value().unwrap_or_default().to_string()
    };
    for (key, want) in [
        ("cone_position0", 1.),
        ("cone_position1", 2.),
        ("cone_position2", 3.),
        ("cone_size0", 0.6),
        ("cone_size1", 0.8),
        ("cone_size2", 1.1),
    ] {
        let parsed = numeric::parse_quantity(&shown(key), numeric::Quantity::Distance).unwrap();
        assert!((parsed - want).abs() < 1e-12, "{key} shows {}", shown(key));
    }
    for (key, want) in [
        ("cone_sides", "8"),
        ("cone_segments", "3"),
        ("cone_weight", "37"),
        ("cone_gaps", "7"),
    ] {
        assert_eq!(shown(key), want, "{key}");
    }
    let below_sides = a.field_rects["cone_sides"].bottom();
    let combos: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == Role::ComboBox && rect(n).top() >= below_sides)
        .filter_map(text)
        .collect();
    for want in ["Catmull Clark", "None", "Texture (no map selected)"] {
        assert!(combos.iter().any(|t| t == want), "{want} in {combos:?}");
    }
    let toggled = |label: &str| {
        nodes
            .iter()
            .find(|n| n.role() == Role::CheckBox && text(n).as_deref() == Some(label))
            .unwrap_or_else(|| panic!("{label} checkbox"))
            .toggled()
    };
    assert_eq!(toggled("Make UVs"), Some(Toggled::False));
    assert_eq!(toggled("Distortion Compensation"), Some(Toggled::True));

    // Apply into the new mesh and compare with a separately configured Editor.
    a.execute("Apply");
    let mut expected = Editor::default();
    expected.scene.new_mesh().unwrap();
    expected.dispatch(Command::ArmCone).unwrap();
    expected
        .dispatch(Command::ConfigureCone(retained.clone()))
        .unwrap();
    expected.dispatch(Command::Apply).unwrap();
    assert_eq!(
        a.editor.scene.items.len(),
        4,
        "the mesh, the new scene's camera, light and environment"
    );
    assert_eq!(a.editor.scene.items[0].mesh, expected.scene.items[0].mesh);
    assert!(!a.editor.scene.items[0].mesh.uv_maps.contains_key(OLD_MAP));
}

#[test]
fn old_scene_and_cold_start_keep_their_cone_state() {
    let cold = {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        a
    };
    assert_eq!(
        *cold.editor.presets.get::<operandi_core::ConePreset>(),
        ConeParameters::default()
    );
    assert_eq!(
        *Operandi::empty()
            .editor
            .presets
            .get::<operandi_core::ConePreset>(),
        ConeParameters::default()
    );

    let mut a = old_document();
    apply_and_drop_cone(&mut a);
    let scene = a.editor.scene.clone();
    a.request_file(FileAction::New);
    assert!(a.switch_scene(0));
    assert_eq!(a.editor.scene, scene, "the old scene is kept as it was");
    // The preset is shared; New dropped its map name (CA-CON-003/015).
    let shared = ConeParameters {
        selected_uv_map: None,
        ..requested()
    };
    assert_eq!(*a.editor.presets.get::<operandi_core::ConePreset>(), shared);
}

// Amendment 1: New while Cone is still active. Its confirmed valid settings
// live in the session, not yet in the Drop-stored preset.

/// Settings distinct from both the defaults and `requested()`.
fn active_settings() -> ConeParameters {
    ConeParameters {
        position: DVec3::new(0.5, -1., 2.),
        size: DVec3::new(0.4, 0.9, 0.7),
        axis: 0,
        sides: 6,
        segments: 2,
        polygon: PolygonRepresentation::Subdivs,
        top_cap: ConeCap::Single,
        bottom_cap: ConeCap::None,
        edge_weight: 55.,
        make_uvs: false,
        compensation: true,
        gaps: 4.,
        selected_uv_map: None,
        falloff: Default::default(),
    }
}

/// Choices through ConfigureCone, numbers through panel field confirmation.
fn configure_active_cone(a: &mut Operandi, want: &ConeParameters) {
    let mut choices = a.editor.session.cone.as_ref().unwrap().requested.clone();
    choices.axis = want.axis;
    choices.polygon = want.polygon;
    choices.bottom_cap = want.bottom_cap;
    choices.make_uvs = want.make_uvs;
    choices.compensation = want.compensation;
    choices.selected_uv_map = want.selected_uv_map.clone();
    a.run(Command::ConfigureCone(choices));
    for a_ in 0..3 {
        a.fields.insert(
            format!("cone_position{a_}"),
            format!("{} m", want.position[a_]),
        );
        a.fields
            .insert(format!("cone_size{a_}"), format!("{} m", want.size[a_]));
    }
    for (key, value) in [
        ("cone_sides", want.sides as f64),
        ("cone_segments", want.segments as f64),
        ("cone_weight", want.edge_weight),
        ("cone_gaps", want.gaps),
    ] {
        a.fields.insert(key.into(), value.to_string());
    }
    assert!(a.confirm_fields(), "{}", a.status);
    let session = a.editor.session.cone.as_ref().unwrap();
    assert_eq!(&session.requested, want);
    assert!(session.preview.is_some() && session.error.is_none());
}

/// New asks nothing: the old scene, its Cone session included, keeps its
/// own tab (scenes.rs).
fn new_beside(a: &mut Operandi) {
    a.request_file(FileAction::New);
    assert!(a.pending.is_none());
}

/// Re-arm on the new first mesh and read the panel's own widgets. Axis
/// buttons and Min/Max are checked through parameters only, not display.
fn assert_reactivated_panel(a: &mut Operandi, ctx: &Context, want: &ConeParameters) {
    a.execute("Cone");
    let session = a.editor.session.cone.as_ref().expect("Cone armed");
    assert_eq!(session.target, 1);
    assert_eq!(&session.requested, want);
    assert!(session.preview.is_none() && session.error.is_none());
    frame(a, ctx, vec![]);
    let nodes = frame(a, ctx, vec![]);
    let shown = |key: &str| {
        let area = a.field_rects[key];
        let hits: Vec<_> = nodes
            .iter()
            .filter(|n| n.role() == Role::TextInput && area.contains(rect(n).center()))
            .collect();
        assert_eq!(hits.len(), 1, "one visible input for {key}");
        hits[0].value().unwrap_or_default().to_string()
    };
    for axis in 0..3 {
        for (prefix, v) in [("cone_position", want.position), ("cone_size", want.size)] {
            let key = format!("{prefix}{axis}");
            let parsed =
                numeric::parse_quantity(&shown(&key), numeric::Quantity::Distance).unwrap();
            assert!(
                (parsed - v[axis]).abs() < 1e-12,
                "{key} shows {}",
                shown(&key)
            );
        }
    }
    for (key, v) in [
        ("cone_sides", want.sides as f64),
        ("cone_segments", want.segments as f64),
        ("cone_weight", want.edge_weight),
        ("cone_gaps", want.gaps),
    ] {
        assert_eq!(shown(key).parse::<f64>().ok(), Some(v), "{key}");
    }
    let below_sides = a.field_rects["cone_sides"].bottom();
    let combos: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == Role::ComboBox && rect(n).top() >= below_sides)
        .filter_map(text)
        .collect();
    let polygon = match want.polygon {
        PolygonRepresentation::Face => "Face",
        PolygonRepresentation::Subdivs => "Subdivs",
        PolygonRepresentation::CatmullClark => "Catmull Clark",
        PolygonRepresentation::Polyline { .. } | PolygonRepresentation::Patch => {
            unreachable!("not configured")
        }
    };
    let caps = match want.bottom_cap {
        ConeCap::None => "None",
        ConeCap::Single => "Single Polygon",
        ConeCap::QuadGrid => "Quad Grid",
        ConeCap::Radial => "Radial",
    };
    for want in [polygon, caps, "Texture (no map selected)"] {
        assert!(combos.iter().any(|t| t == want), "{want} in {combos:?}");
    }
    let toggled = |label: &str| {
        nodes
            .iter()
            .find(|n| n.role() == Role::CheckBox && text(n).as_deref() == Some(label))
            .unwrap_or_else(|| panic!("{label} checkbox"))
            .toggled()
    };
    let flag = |b: bool| Some(if b { Toggled::True } else { Toggled::False });
    assert_eq!(toggled("Make UVs"), flag(want.make_uvs));
    assert_eq!(toggled("Distortion Compensation"), flag(want.compensation));
}

/// Apply in the new mesh equals a separate Editor given the same values.
fn assert_apply_matches_separate_editor(a: &mut Operandi, want: &ConeParameters) {
    a.execute("Apply");
    let mut expected = Editor::default();
    expected.scene.new_mesh().unwrap();
    expected.dispatch(Command::ArmCone).unwrap();
    expected
        .dispatch(Command::ConfigureCone(want.clone()))
        .unwrap();
    expected.dispatch(Command::Apply).unwrap();
    assert_eq!(
        a.editor.scene.items.len(),
        4,
        "the mesh, the new scene's camera, light and environment"
    );
    assert_eq!(a.editor.scene.items[0].mesh, expected.scene.items[0].mesh);
}

#[test]
fn clean_active_cone_preview_new_retains_confirmed_settings() {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a.execute("Cone");
    let want = active_settings();
    configure_active_cone(&mut a, &want);
    assert_eq!(
        *a.editor.presets.get::<operandi_core::ConePreset>(),
        ConeParameters::default()
    );
    assert!(!a.dirty(), "preview is not a committed edit");
    a.request_file(FileAction::New); // current guard: no prompt for this state
    assert!(a.pending.is_none());
    assert_clean_new_document(&a);
    assert_eq!(
        *a.editor.presets.get::<operandi_core::ConePreset>(),
        want,
        "active confirmed values retained"
    );
    let ctx = context();
    assert_reactivated_panel(&mut a, &ctx, &want);
    assert_apply_matches_separate_editor(&mut a, &want);
}

#[test]
fn applied_active_cone_discard_retains_current_not_dropped_settings() {
    let mut a = old_document();
    apply_and_drop_cone(&mut a); // preset now holds requested()
    a.execute("Cone");
    let want = ConeParameters {
        selected_uv_map: Some(OLD_MAP.into()),
        ..active_settings()
    };
    configure_active_cone(&mut a, &want);
    a.execute("Apply");
    assert!(a.editor.session.cone.is_some(), "Apply keeps Cone active");
    assert_eq!(
        *a.editor.presets.get::<operandi_core::ConePreset>(),
        requested()
    );
    assert!(a.dirty());
    let ctx = context();

    let scene = a.editor.scene.clone();
    new_beside(&mut a);
    assert_clean_new_document(&a);
    let retained = ConeParameters {
        selected_uv_map: None,
        ..want
    };
    assert_eq!(
        *a.editor.presets.get::<operandi_core::ConePreset>(),
        retained,
        "current, not dropped, values"
    );
    assert_reactivated_panel(&mut a, &ctx, &retained);
    assert_apply_matches_separate_editor(&mut a, &retained);
    assert!(!a.editor.scene.items[0].mesh.uv_maps.contains_key(OLD_MAP));

    // The old scene still has its applied Cone, session and all.
    a.run(Command::Drop);
    assert!(a.switch_scene(0));
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.editor.session.cone.as_ref().unwrap().requested, want);
}
