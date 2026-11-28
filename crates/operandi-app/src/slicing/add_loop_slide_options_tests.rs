//! Tool audit Tier 1: Add Loop / Add Point tool properties in the viewport
//! and command line. Geometry rules are tested in operandi-core
//! (tests/add_loop_slide_options.rs).
use super::tests::box_app;
use super::*;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

/// The box's first edge, its screen ends, and the app armed with `tool`.
fn armed(tool: edge_panel::EdgeClick) -> (Operandi, (u32, u32), [Pos2; 2]) {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Edge));
    let mesh = a.editor.scene.active().unwrap().mesh.to_owned_value();
    let (ea, eb) = mesh.edges()[0];
    let project = |v: u32| {
        a.camera
            .project(mesh.vertices[v as usize], rect())
            .unwrap()
            .0
    };
    let ends = [project(ea), project(eb)];
    a.edge_click = Some(tool);
    (a, (ea, eb), ends)
}

fn points(a: &Operandi) -> usize {
    a.editor.scene.active().unwrap().mesh.vertices.len()
}

#[test]
fn split_at_the_middle_checkbox_places_every_click_at_the_middle() {
    let (mut a, _, [pa, pb]) = armed(edge_panel::EdgeClick::AddLoop);
    a.add_loop_settings.middle = true;
    let (taken, slide) = a.edge_click_at(pa.lerp(pb, 0.25), rect(), false);
    assert!(taken);
    assert!(slide.is_none(), "the middle stays put");
    assert!(a.status.ends_with("50%"), "{}", a.status);
    assert_eq!(a.add_loop_settings.position, 50.);
}

#[test]
fn both_sides_click_and_slide_stay_one_undo_step() {
    let (mut a, _, [pa, pb]) = armed(edge_panel::EdgeClick::AddLoop);
    a.add_loop_settings.options.both_sides = true;
    let before = points(&a);
    let undo = a.editor.history.undo.len();
    let (_, slide) = a.edge_click_at(pa.lerp(pb, 0.25), rect(), false);
    // Two closed loops of four points around the box.
    assert_eq!(points(&a), before + 8, "{}", a.status);
    let Some(input::GestureKind::EdgeInsert {
        tool,
        a: x,
        b: y,
        ends,
    }) = slide
    else {
        panic!("no slide gesture: {}", a.status);
    };
    a.edge_insert_slide(tool, x, y, ends, ends[0].0.lerp(ends[1].0, 0.3));
    assert_eq!(points(&a), before + 8);
    assert_eq!(a.editor.history.undo.len(), undo + 1);
    assert!(a.status.contains("30%"), "{}", a.status);
}

#[test]
fn absolute_distance_ignores_the_pointer_and_shows_metres() {
    let (mut a, _, [pa, pb]) = armed(edge_panel::EdgeClick::AddPoint);
    let s = &mut a.add_point_settings;
    s.options.absolute = true;
    s.options.distance1 = 0.5;
    s.show_distance = true;
    a.edge_click_at(pa.lerp(pb, 0.8), rect(), false);
    // The picked edge may be any box edge under the pointer (Top view
    // overlaps them); the new point sits 0.5 m from its nearest corner.
    let m = a.editor.scene.active().unwrap().mesh.to_owned_value();
    let new = *m.vertices.last().unwrap();
    let nearest = m.vertices[..8]
        .iter()
        .map(|p| p.distance(new))
        .fold(f64::MAX, f64::min);
    assert!((nearest - 0.5).abs() < 1e-9, "{new}");
    assert!(
        a.status.contains("25%") && a.status.contains("0.500 m"),
        "{}",
        a.status
    );
    assert_eq!(a.add_point_settings.position, 25.);
}

#[test]
fn absolute_drag_sets_distance_one() {
    let (mut a, _, [pa, pb]) = armed(edge_panel::EdgeClick::AddLoop);
    a.add_loop_settings.options.absolute = true;
    a.add_loop_settings.options.distance1 = 0.5;
    let (_, slide) = a.edge_click_at(pa.lerp(pb, 0.5), rect(), false);
    let Some(input::GestureKind::EdgeInsert {
        tool,
        a: x,
        b: y,
        ends,
    }) = slide
    else {
        panic!("no slide gesture: {}", a.status);
    };
    a.edge_insert_slide(tool, x, y, ends, ends[0].0.lerp(ends[1].0, 0.75));
    // Box edges are 2 m long.
    let d = a.add_loop_settings.options.distance1;
    assert!(x < y);
    assert!((d - 1.5).abs() < 0.02, "{d}");
}

#[test]
fn show_position_off_hides_the_percent() {
    let (mut a, _, [pa, pb]) = armed(edge_panel::EdgeClick::AddLoop);
    a.add_loop_settings.show_position = false;
    a.add_loop_settings.show_distance = true;
    a.edge_click_at(pa.lerp(pb, 0.5), rect(), false);
    let label = a.edge_insert_label.clone().unwrap().1;
    assert_eq!(label, "1.000 m");
}

#[test]
fn add_loop_command_uses_the_tool_properties() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Edge));
    let edge = a.editor.scene.active().unwrap().mesh.edges()[0];
    a.editor.scene.selection.edges = vec![edge];
    a.add_loop_settings.options.both_sides = true;
    a.add_loop_settings.position = 25.;
    let before = points(&a);
    a.execute("edge.addLoop");
    assert_eq!(points(&a), before + 8, "{}", a.status);
    assert!(a.status.contains("25%"), "{}", a.status);
}

#[test]
fn slide_stop_negative_and_loop_slide_reach_the_tool() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![0];
    a.activate(Tool::VertexSlide);
    let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
    p.slide_extras.loop_slide = false;
    p.shift = 1.5;
    assert!(a.run_ok(Command::Preview(p.clone())), "{}", a.status);
    p.slide_extras.loop_slide = true;
    assert!(!a.run_ok(Command::Preview(p)), "Loop Slide keeps ±100%");
}
