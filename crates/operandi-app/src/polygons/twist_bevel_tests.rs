//! Twist Bevel in the app (twist-bevel-2026-11-15): its names, the
//! toolbox and Tools menu, the panel, the handles and the session rules it
//! shares with Bevel (TA-01..07).
use super::tests::box_app;
use super::*;
use viewport::twist_bevel_handles::*;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

fn live(a: &Operandi) -> &operandi_core::Live {
    a.editor.session.live.as_ref().expect("live tool")
}

/// The box with its top polygon (+Y) selected in Polygon mode, and that
/// polygon's index.
fn top_selected() -> (Operandi, u32) {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    let mesh = &a.editor.scene.active().unwrap().mesh;
    let top = (0..mesh.faces.len())
        .find(|&f| mesh.normal(f).dot(DVec3::Y) > 0.99)
        .unwrap() as u32;
    a.editor.scene.selection.faces = vec![top];
    (a, top)
}

fn faces(a: &Operandi) -> usize {
    a.editor.scene.active().unwrap().mesh.faces.len()
}

fn set(a: &mut Operandi, edit: impl FnOnce(&mut operandi_core::TwistBevelOptions)) {
    let mut p = live(a).params.clone();
    edit(&mut p.twist_bevel);
    a.run(Command::Preview(p));
}

#[test]
fn starts_from_its_name_tool_set_and_the_tools_menu() {
    let (mut a, _) = top_selected();
    a.execute("twist bevel");
    assert_eq!(live(&a).tool, Tool::TwistBevel, "{}", a.status);
    a.run(Command::Drop);
    a.execute("tool.set poly.twistBevel on");
    assert_eq!(live(&a).tool, Tool::TwistBevel, "{}", a.status);
    a.run(Command::Drop);
    let model = a.menu_model(&Context::default());
    let tools = model.iter().find(|m| m.title == "Tools").expect("Tools");
    let actions: Vec<_> = tools.leaves().iter().map(|i| i.action).collect();
    assert!(actions.contains(&app_menu::line_action(Tool::TwistBevel.label())));
    assert!(input::available("Twist Bevel", Mode::Polygon, false));
    assert!(!input::available("Twist Bevel", Mode::Edge, false));
}

/// Vertex and Edge mode refuse it (a polygon tool, SI-03).
#[test]
fn refuses_outside_polygon_mode() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Edge));
    a.activate(Tool::TwistBevel);
    assert!(a.editor.session.live.is_none(), "{}", a.status);
}

/// Switching on changes nothing; a panel edit is live at once; Drop keeps
/// it as one undo step and the cap stays selected.
#[test]
fn panel_edit_drop_and_undo() {
    let (mut a, top) = top_selected();
    let before = a.editor.scene.clone();
    a.activate(Tool::TwistBevel);
    assert_eq!(faces(&a), 6, "switching on changes nothing");
    a.execute("tool.attr poly.twistBevel shift 1");
    // Scripts give angles in radians (tool.attr help): a quarter turn.
    a.execute("tool.attr poly.twistBevel rotation 1.5707963267948966");
    let o = live(&a).params.twist_bevel;
    assert_eq!(o.shift, 1., "{}", a.status);
    assert!((o.rotation - 90.).abs() < 1e-9, "{}", a.status);
    assert_eq!(faces(&a), 6 + 16, "{}", a.status);
    assert_eq!(a.editor.scene.selection.faces, vec![top]);
    a.run(Command::Drop);
    assert!(a.editor.session.live.is_none(), "{}", a.status);
    assert_eq!(faces(&a), 22);
    a.run(Command::Undo);
    assert_eq!(
        a.editor.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    a.run(Command::Redo);
    assert_eq!(faces(&a), 22);
}

/// Cancel puts the scene back.
#[test]
fn cancel_restores() {
    let (mut a, _) = top_selected();
    let before = a.editor.scene.active().unwrap().mesh.clone();
    a.activate(Tool::TwistBevel);
    // The box is 2 m: a quarter turn needs a radius past its half width.
    set(&mut a, |o| {
        o.shift = 3.;
        o.bend = 90.;
    });
    assert_eq!(faces(&a), 22, "{}", a.status);
    a.run(Command::Cancel);
    assert_eq!(a.editor.scene.active().unwrap().mesh, before);
}

/// Shift+click (Continue) keeps this twist bevel and starts a new one on
/// the cap; Drop leaves one undo step per twist bevel (TA-07).
#[test]
fn continue_stacks_twist_bevels() {
    let (mut a, _) = top_selected();
    a.activate(Tool::TwistBevel);
    set(&mut a, |o| {
        o.shift = 0.5;
        o.rotation = 45.;
    });
    a.run(Command::Continue);
    let p = &live(&a).params.twist_bevel;
    assert_eq!((p.shift, p.rotation), (0., 45.), "the size starts again");
    set(&mut a, |o| o.shift = 0.5);
    assert_eq!(faces(&a), 6 + 16 + 16);
    a.run(Command::Drop);
    a.run(Command::Undo);
    assert_eq!(faces(&a), 22, "{}", a.status);
    a.run(Command::Undo);
    assert_eq!(faces(&a), 6);
}

/// Nothing selected works on every polygon (PB-O-01): an open grid row.
#[test]
fn empty_selection_refuses_a_closed_box_with_a_message() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces.clear();
    a.activate(Tool::TwistBevel);
    if a.editor.session.live.is_some() {
        let mut p = live(&a).params.clone();
        p.twist_bevel.shift = 1.;
        a.run(Command::Preview(p));
    }
    assert_eq!(faces(&a), 6, "a closed shell has no boundary: {}", a.status);
}

/// Five handles on the first tube; each drag changes its own value.
#[test]
fn handles_drag_their_values() {
    let (mut a, _) = top_selected();
    // In perspective neither the Shift arrow nor the rings are end-on.
    a.camera.view = View::Perspective;
    a.activate(Tool::TwistBevel);
    // Long enough (the box is 2 m) that a bend and twist don't fold it.
    set(&mut a, |o| o.shift = 3.);
    let handles = a.handles(rect());
    let ids: Vec<usize> = handles.iter().map(|h| h.0).collect();
    for id in [TB_SHIFT, TB_INSET, TB_TWIST, TB_BEND, TB_BEND_DIRECTION] {
        assert!(ids.contains(&id), "{id} missing from {ids:?}");
    }
    let drag = |a: &mut Operandi, id: usize| {
        let h = a.handles(rect()).into_iter().find(|h| h.0 == id).unwrap();
        let base = live(a).params.clone();
        // Along the arrow, or a quarter way round a ring from its end.
        let (from, to) = if matches!(id, TB_TWIST | TB_BEND_DIRECTION) {
            let c = h.1;
            let d = h.2 - c;
            (h.2, c + vec2(-d.y, d.x))
        } else {
            (h.1, h.1 + (h.2 - h.1) * 1.5)
        };
        a.update_handle(id, base, from, to, Tool::TwistBevel, rect());
        live(a).params.twist_bevel
    };
    let shift = drag(&mut a, TB_SHIFT).shift;
    assert!(shift > 3., "{}", a.status);
    assert!(a.status.contains("Shift"), "readout: {}", a.status);
    assert!(drag(&mut a, TB_INSET).inset < 0., "out grows the cap");
    assert!(drag(&mut a, TB_TWIST).rotation.abs() > 10., "{}", a.status);
    assert!(drag(&mut a, TB_BEND).bend > 1., "{}", a.status);
    assert!(
        drag(&mut a, TB_BEND_DIRECTION).bend_direction != 0.,
        "{}",
        a.status
    );
    // Each handle is hit where it's drawn.
    for (id, _, end, _) in a.handles(rect()) {
        if !matches!(id, TB_TWIST | TB_BEND_DIRECTION) {
            assert_eq!(a.handle_hit(end, rect()), Some(id), "{id}");
        }
    }
}

/// The cap handles ride on the shape: bending moves the Shift arrow.
#[test]
fn cap_handles_follow_the_bend() {
    let (mut a, _) = top_selected();
    a.camera.view = View::Front;
    a.activate(Tool::TwistBevel);
    set(&mut a, |o| o.shift = 3.);
    let shift_start = |a: &Operandi| {
        a.handles(rect())
            .into_iter()
            .find(|h| h.0 == TB_SHIFT)
            .expect("Shift arrow")
            .1
    };
    let straight = shift_start(&a);
    set(&mut a, |o| o.bend = 90.);
    assert!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .preview_error
            .is_none(),
        "{}",
        a.status
    );
    let bent = shift_start(&a);
    assert!(straight.distance(bent) > 5.);
}

/// A drag away from the handles changes Shift (TB-Q08).
#[test]
fn drag_anywhere_changes_shift() {
    assert!(viewport::off_handle_drag(Tool::TwistBevel));
    let (mut a, _) = top_selected();
    a.activate(Tool::TwistBevel);
    let base = live(&a).params.clone();
    let start = pos2(400., 300.);
    a.update_handle(
        viewport::OFF_HANDLE,
        base,
        start,
        start + vec2(0., -40.),
        Tool::TwistBevel,
        rect(),
    );
    assert!(live(&a).params.twist_bevel.shift > 0., "{}", a.status);
}

/// The panel lists the spec's rows in order, and greys Profile Bias while
/// Profile is 0 (UIC-117).
#[test]
fn panel_rows_in_spec_order() {
    let (mut a, _) = top_selected();
    a.activate(Tool::TwistBevel);
    let labels: Vec<String> = a.tool_attr_fields().into_iter().map(|f| f.1).collect();
    assert_eq!(
        labels,
        [
            "Shift",
            "Inset",
            "Segments",
            "Group Polygons",
            "Rotation",
            "Rotation Axis",
            "Profile",
            "Profile Bias",
            "Segment Spacing",
            "Center Bias",
            "Bend",
            "Bend Direction",
            // The TwistBevel notes' opt-ins (2026-11-25), after the spec's rows.
            "Stop Before Fold",
            "Even Interior",
            "Keep Cap UVs",
        ]
    );
    a.execute("tool.attr poly.twistBevel stopAtFold true");
    assert!(live(&a).params.twist_bevel.stop_at_fold, "{}", a.status);
    a.execute("tool.attr poly.twistBevel profile 5");
    assert_eq!(
        live(&a).params.twist_bevel.profile,
        1.,
        "clamped: {}",
        a.status
    );
    a.execute("tool.attr poly.twistBevel segments 300");
    assert_eq!(
        live(&a).params.twist_bevel.segments,
        4,
        "refused: {}",
        a.status
    );
}

/// Symmetry X on: the mirror polygon's tube is a mirror image.
#[test]
fn symmetry_mirrors_through_the_editor() {
    let (mut a, _) = top_selected();
    let mesh = &a.editor.scene.active().unwrap().mesh;
    let sides: Vec<u32> = (0..mesh.faces.len() as u32)
        .filter(|&f| mesh.normal(f as usize).x.abs() > 0.99)
        .collect();
    a.editor.scene.selection.faces = sides;
    a.editor.symmetry = operandi_core::Symmetry::x();
    a.activate(Tool::TwistBevel);
    set(&mut a, |o| {
        o.shift = 3.;
        o.bend = 40.;
        o.bend_direction = 30.;
        o.rotation = 25.;
    });
    let m = &a.editor.scene.active().unwrap().mesh;
    assert!(m.faces.len() > 6, "{}", a.status);
    for &p in &m.vertices {
        let mirror = DVec3::new(-p.x, p.y, p.z);
        assert!(
            m.vertices.iter().any(|q| q.distance(mirror) < 1e-9),
            "{p} has no mirror"
        );
    }
}

/// The credits name the ideas (spec section 10).
#[test]
fn credits_line_is_in_about() {
    assert!(about::IDEA_CREDITS.iter().any(|c| c.contains("FUSION")));
}
