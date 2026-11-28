//! Constrain to Background in the app (SPEC-BACKGROUND-CONSTRAINT,
//! BEHAVIOR-MESH-CONSTRAINT): the Mesh Constraint button and popover, its
//! Tool Properties rows, Esc, saved settings, and the view a press notes.
use super::tests::{draw, key, pointer};
use super::*;
use crate::background_ui::{BackgroundFields, PopoverFields};
use crate::prefs_file::PrefsFile;
use crate::tool_form::ToolFields;
use operandi_core::background_constraint::{BackgroundConstraint, GeometryConstraint};

const SIZE: Vec2 = vec2(1440., 900.);

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 1)
            .unwrap()
            .into();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::new(5., 0., 0.), DVec3::ONE, 1)
            .unwrap()
            .into();
    a
}

#[test]
fn the_menu_toggle_turns_it_on_and_off_without_history() {
    let mut a = app();
    let history = a.editor.history.undo.len();
    a.execute("constrain to background");
    assert_eq!(a.editor.background_constraint, Some(Default::default()));
    assert!(a.tool_properties_open);
    a.execute("constrain to background");
    assert_eq!(a.editor.background_constraint, None);
    assert_eq!(a.editor.history.undo.len(), history);
    assert_eq!(
        a.bindings.get(background_ui::ACTION).map(String::as_str),
        Some("")
    );
}

#[test]
fn turning_it_on_keeps_the_open_tool() {
    let mut a = app();
    a.run(Command::Mode(Mode::Vertex));
    a.activate(Tool::Move);
    a.execute("constrain to background");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
    assert!(a.editor.background_constraint.is_some());
}

#[test]
fn tool_properties_rows_follow_the_options() {
    let mut a = app();
    a.execute("constrain to background");
    let p = BackgroundFields::params(&a).unwrap();
    let form = BackgroundFields::form(&a, &p);
    let labels: Vec<&str> = form.fields().iter().map(|f| f.row.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Geometry Constraint",
            "Offset",
            "Negative Offset",
            "Handle Constraint",
            "Double Sided",
            "Target Item"
        ]
    );
    // Handle Constraint is greyed with its reason: no curve tool has
    // draggable handles yet.
    let greyed: Vec<&str> = form
        .fields()
        .iter()
        .filter(|f| f.disabled_tip.is_some())
        .map(|f| f.row.label.as_str())
        .collect();
    assert_eq!(greyed, ["Handle Constraint"]);
    // Off greys the rest in place (UIC-117).
    let off = BackgroundConstraint {
        mode: GeometryConstraint::Off,
        ..p
    };
    let form = BackgroundFields::form(&a, &off);
    assert!(form.fields()[1..].iter().all(|f| f.disabled_tip.is_some()));
    // A panel edit goes to the editor.
    BackgroundFields::publish(&mut a, BackgroundConstraint { offset: 0.05, ..p }).unwrap();
    assert_eq!(a.editor.background_constraint.unwrap().offset, 0.05);
    assert_eq!(a.editor.background_constraint_preset.offset, 0.05);
}

#[test]
fn escape_over_a_view_clears_it_and_a_press_notes_the_view() {
    let ctx = Context::default();
    let mut a = app();
    a.camera.view = View::Front;
    draw(&mut a, &ctx, SIZE, vec![]);
    a.execute("constrain to background");
    let at = a.viewport_rect.center();
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    );
    let view = a.editor.constraint_view();
    assert!((view.forward - -DVec3::Z).length() < 1e-9, "{:?}", view);
    assert_eq!(view.eye, None);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(at), key(Key::Escape)],
    );
    assert_eq!(a.editor.background_constraint, None);
}

#[test]
fn const_bg_tool_set_and_attrs() {
    let mut a = app();
    let bg = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| i.is_mesh())
        .unwrap()
        .clone();
    a.execute("tool.set const.bg ?");
    assert_eq!(a.status, "0");
    // Options set while off are kept for the next turn-on.
    a.execute("tool.attr const.bg bkgOffset 5cm");
    a.execute("tool.set const.bg on");
    assert_eq!(a.editor.background_constraint.unwrap().offset, 0.05);
    a.execute("tool.set const.bg on");
    assert!(
        a.editor.background_constraint.is_some(),
        "on twice stays on"
    );
    a.execute("tool.attr const.bg geometry 3");
    assert_eq!(
        a.editor.background_constraint.unwrap().mode,
        GeometryConstraint::Point
    );
    a.execute("tool.attr const.bg geometry screen");
    a.execute("tool.attr const.bg geometry ?");
    assert_eq!(a.status, "screen");
    a.execute("tool.attr const.bg dblSided true");
    assert!(a.editor.background_constraint.unwrap().double_sided);
    a.execute("tool.attr const.bg negative ?");
    assert_eq!(a.status, "1");
    a.execute(&format!("tool.attr const.bg target {}", bg.id));
    assert_eq!(a.editor.background_constraint.unwrap().target, Some(bg.id));
    a.execute("tool.attr const.bg target none");
    assert_eq!(a.editor.background_constraint.unwrap().target, None);
    a.execute("tool.attr const.bg handle ?");
    assert_eq!(a.status, "1");
    a.execute("tool.attr const.bg handle false");
    assert!(!a.editor.background_constraint.unwrap().handle_constraint);
    a.execute("tool.attr const.bg geometry 9");
    assert!(a.status.contains("0–3"), "{}", a.status);
    a.execute("Constrain to Background");
    assert!(a.editor.background_constraint.is_none());
}

/// MC-06: the defaults.
#[test]
fn mc06_defaults() {
    let a = app();
    assert_eq!(a.editor.background_constraint, None, "off");
    let d = a.editor.background_constraint_preset;
    assert_eq!(d.mode, GeometryConstraint::ScreenAxis);
    assert_eq!(d.offset, 0.);
    assert!(d.negative_offset && d.handle_constraint && !d.double_sided);
    assert_eq!(d.target, None);
}

/// Where a piece of text was drawn this frame (its centre), to click it.
fn find_text(out: &egui::FullOutput, want: &str) -> Option<Pos2> {
    fn walk(shape: &egui::Shape, want: &str, found: &mut Option<Pos2>) {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == want => {
                *found = Some(t.pos + t.galley.rect.center().to_vec2())
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, want, found)),
            _ => {}
        }
    }
    let mut found = None;
    for c in &out.shapes {
        walk(&c.shape, want, &mut found);
    }
    found
}

/// Wide enough that the whole toolbar row is drawn.
const WIDE: Vec2 = vec2(2600., 900.);

/// Click at `at` with `held` down, then draw a settled frame.
fn click(a: &mut Operandi, ctx: &Context, at: Pos2, held: Modifiers) -> egui::FullOutput {
    super::tests::draw_held(a, ctx, WIDE, vec![Event::PointerMoved(at)], held);
    super::tests::draw_held(a, ctx, WIDE, vec![pointer(at, true, held)], held);
    super::tests::draw_held(a, ctx, WIDE, vec![pointer(at, false, held)], held);
    draw(a, ctx, WIDE, vec![])
}

/// MC-03: the button's label follows the state; Option shows Options.
#[test]
fn mc03_button_label_and_toggle() {
    assert_eq!(background_ui::button_label(false, false), "Mesh Constraint");
    assert_eq!(background_ui::button_label(true, false), "Background");
    assert_eq!(background_ui::button_label(true, true), "Options");
    let ctx = Context::default();
    let mut a = app();
    draw(&mut a, &ctx, WIDE, vec![]);
    let out = draw(&mut a, &ctx, WIDE, vec![]);
    let at = find_text(&out, "Mesh Constraint").expect("the button is in the toolbar");
    let out = click(&mut a, &ctx, at, Modifiers::NONE);
    assert_eq!(a.editor.background_constraint, Some(Default::default()));
    let at = find_text(&out, "Background").expect("on, it reads Background");
    let out = click(&mut a, &ctx, at, Modifiers::NONE);
    assert_eq!(a.editor.background_constraint, None);
    // Option-click opens the popover instead.
    let at = find_text(&out, "Mesh Constraint").unwrap();
    click(&mut a, &ctx, at, Modifiers::ALT);
    assert!(a.mesh_constraint_popover.is_some());
    assert_eq!(a.editor.background_constraint, None);
}

/// MC-05: the popover's rows: the mode strip, then (Background only) the
/// group without Target Item.
#[test]
fn mc05_popover_rows_and_modes() {
    assert_eq!(
        background_ui::MODES,
        ["(none)", "Guide", "Background", "Primitive"]
    );
    let mut a = app();
    assert_eq!(PopoverFields::params(&a), None, "no group while off");
    a.toggle_background_constraint();
    let p = PopoverFields::params(&a).unwrap();
    let form = PopoverFields::form(&a, &p);
    let labels: Vec<&str> = form.fields().iter().map(|f| f.row.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Geometry Constraint",
            "Offset",
            "Negative Offset",
            "Handle Constraint",
            "Double Sided"
        ]
    );
    // Drawn: the strip and the group show.
    let ctx = Context::default();
    draw(&mut a, &ctx, WIDE, vec![]);
    a.open_mesh_constraint_popover();
    draw(&mut a, &ctx, WIDE, vec![]);
    let out = draw(&mut a, &ctx, WIDE, vec![]);
    for label in [
        "(none)",
        "Guide",
        "Primitive",
        "Negative Offset",
        "Double Sided",
    ] {
        assert!(
            find_text(&out, label).is_some(),
            "{label} is in the popover"
        );
    }
    // (none) turns it off; the group goes and the popover stays.
    let none = find_text(&out, "(none)").unwrap();
    let out = click(&mut a, &ctx, none, Modifiers::NONE);
    assert_eq!(a.editor.background_constraint, None);
    assert!(a.mesh_constraint_popover.is_some());
    assert!(find_text(&out, "Double Sided").is_none());
}

/// MC-03: Edit ▸ Snapping ▸ Mesh Constraints opens the popover; the
/// Snapping menu has no constraint toggle (MC-01).
#[test]
fn mc03_menu_opens_the_popover() {
    let mut a = app();
    let ctx = Context::default();
    a.menu_action(&ctx, crate::app_menu::line_action("Mesh Constraints"));
    assert!(a.mesh_constraint_popover.is_some());
    assert_eq!(a.editor.background_constraint, None);
}

/// MC-04: Esc turns it off and keeps its settings for next time.
#[test]
fn mc04_escape_keeps_the_settings() {
    let ctx = Context::default();
    let mut a = app();
    draw(&mut a, &ctx, SIZE, vec![]);
    a.toggle_background_constraint();
    a.execute("tool.attr const.bg bkgOffset 3cm");
    a.execute("tool.attr const.bg dblSided true");
    let at = a.viewport_rect.center();
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(at), key(Key::Escape)],
    );
    assert_eq!(a.editor.background_constraint, None);
    a.toggle_background_constraint();
    let c = a.editor.background_constraint.unwrap();
    assert_eq!(c.offset, 0.03);
    assert!(c.double_sided);
}

/// MC-02: Snapping and Mesh Constraint switch independently.
#[test]
fn mc02_snapping_and_constraint_are_independent() {
    let mut a = app();
    a.toggle_background_constraint();
    assert!(!a.snapping.enabled);
    a.toggle_snapping();
    assert!(a.snapping.enabled && a.editor.background_constraint.is_some());
    a.toggle_background_constraint();
    assert!(a.snapping.enabled && a.editor.background_constraint.is_none());
    a.toggle_snapping();
    a.toggle_background_constraint();
    assert!(!a.snapping.enabled && a.editor.background_constraint.is_some());
}

/// MC-06: settings persist across launches (not the on/off state, nor
/// Target Item).
#[test]
fn mc06_settings_round_trip_through_preferences() {
    let mut a = app();
    a.toggle_background_constraint();
    a.execute("tool.attr const.bg bkgOffset 2cm");
    a.execute("tool.attr const.bg geometry point");
    a.execute("tool.attr const.bg handle false");
    let bg = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| i.is_mesh())
        .unwrap()
        .id;
    a.execute(&format!("tool.attr const.bg target {bg}"));
    a.sync_mesh_constraint();
    let path = a.environment.paths().mesh_constraint_preferences();
    let saved = background_ui::MeshConstraintPreferences::load(&path);
    assert_eq!(saved.background.offset, 0.02);
    assert_eq!(saved.background.mode, GeometryConstraint::Point);
    assert!(!saved.background.handle_constraint);
    // Next launch: off, with the same options and no Target Item.
    let mut b = app();
    b.environment = a.environment.clone();
    b.load_mesh_constraint();
    assert_eq!(b.editor.background_constraint, None);
    b.toggle_background_constraint();
    let c = b.editor.background_constraint.unwrap();
    assert_eq!(
        (c.offset, c.mode, c.target),
        (0.02, GeometryConstraint::Point, None)
    );
    // A bad file is ignored.
    std::fs::write(
        &path,
        br#"{"version":1,"background":{"offset":-1,"negative_offset":false}}"#,
    )
    .unwrap();
    assert_eq!(
        background_ui::MeshConstraintPreferences::load(&path),
        Default::default()
    );
}

/// MC-08: snapping runs first, the constraint after: a point snapped onto
/// another point is then pushed onto the background along the view.
#[test]
fn mc08_a_snapped_point_ends_on_the_background() {
    let ctx = Context::default();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::new(0., 0., -0.5), DVec3::new(20., 20., 1.), 1)
            .unwrap()
            .into();
    a.run(Command::NewMesh);
    let fg = operandi_core::Mesh {
        // Point 0 in one triangle; the snap target, point 3, in another.
        vertices: vec![
            DVec3::new(0., 0., 5.),
            DVec3::new(-1., 0., 5.),
            DVec3::new(0., -1., 5.),
            DVec3::new(1., 0.5, 5.),
            DVec3::new(2., 0.5, 5.),
            DVec3::new(1., 1.5, 5.),
        ],
        faces: [[0, 1, 2], [3, 4, 5]]
            .into_iter()
            .map(|v| operandi_core::Face {
                vertices: v.to_vec(),
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect::<Vec<_>>()
            .into(),
        ..Default::default()
    };
    a.editor.scene.active_mut().unwrap().mesh = fg.into();
    a.camera.view = View::Front;
    a.camera.target = DVec3::new(1., 0.5, 0.);
    a.camera.distance = 6.;
    draw(&mut a, &ctx, SIZE, vec![]);
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![0];
    a.activate(Tool::Move);
    a.toggle_snapping();
    a.toggle_background_constraint();
    let r = a.viewport_rect;
    // Dragged to just short of point 1: the handle snaps onto it...
    let params = operandi_core::Parameters {
        delta: DVec3::new(0.98, 0.49, 0.),
        ..a.editor.session.live.as_ref().unwrap().params.clone()
    };
    let snapped = a.snap_move(Tool::Move, params, None, r);
    assert!(
        (snapped.delta - DVec3::new(1., 0.5, 0.)).length() < 1e-9,
        "{:?}",
        snapped.delta
    );
    a.run(Command::Preview(snapped));
    // ...then the constraint pushes it along the view onto the background.
    let p = a.editor.scene.active().unwrap().mesh.vertices[0];
    assert!(
        (p - DVec3::new(1., 0.5, 0.)).length() < 1e-9,
        "{p} {}",
        a.status
    );
}
