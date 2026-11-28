//! The Tool Properties header names the active tool (Jason, 2026-11-06),
//! and reads "Tool Properties" when no tool is on.
use super::tests::{box_app, draw};
use super::*;

/// The text painted inside the Tool Properties header.
fn header_text(a: &mut Operandi) -> String {
    let ctx = Context::default();
    theme::install(&ctx);
    a.tool_properties_open = true;
    // Two passes: the first lays the toolbox out, the second records it.
    draw(a, &ctx, vec2(1100., 1600.), vec![]);
    let output = draw(a, &ctx, vec2(1100., 1600.), vec![]);
    let rect = a.toolbox_rects["Tool Properties"];
    fn find(s: &egui::Shape, rect: Rect, out: &mut Vec<String>) {
        match s {
            egui::Shape::Text(t) if rect.contains(t.pos) => out.push(t.galley.text().into()),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| find(s, rect, out)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    // Scrolled-away rows behind the header are clipped out; skip them.
    for s in output
        .shapes
        .iter()
        .filter(|s| s.clip_rect.contains_rect(rect))
    {
        find(&s.shape, rect, &mut found);
    }
    assert_eq!(found.len(), 1, "one header label: {found:?}");
    found.remove(0)
}

#[test]
fn header_names_the_active_tool_and_falls_back_with_none() {
    let mut a = box_app();
    a.category = toolbox::Category::Polygon;
    assert_eq!(header_text(&mut a), "Tool Properties");

    a.run(Command::ArmShape(ShapeKind::Capsule));
    assert!(a.editor.session.shape.is_some(), "{}", a.status);
    assert_eq!(header_text(&mut a), "Capsule");
    a.run(Command::Cancel);
    assert!(a.editor.session.shape.is_none());
    assert_eq!(header_text(&mut a), "Tool Properties");

    a.run(Command::ArmCone);
    assert_eq!(header_text(&mut a), "Cone");
    a.run(Command::Cancel);

    a.activate(Tool::Box);
    assert_eq!(header_text(&mut a), "Cube");
    a.run(Command::Drop);
    assert_eq!(header_text(&mut a), "Tool Properties");
}

#[test]
fn title_follows_what_the_panel_shows() {
    let mut a = box_app();
    assert_eq!(a.tool_properties_title(), "Tool Properties");
    a.activate(Tool::Move);
    assert_eq!(a.tool_properties_title(), "Move");
    // A utility tool's panel wins over the live tool's, so its name does too.
    a.utility = Some(create_utility::Utility::Dimensions { orient: false });
    assert_eq!(a.tool_properties_title(), "Dimensions");
    a.utility = None;
    a.run(Command::Drop);
    a.edge_click = Some(edge_panel::EdgeClick::AddLoop);
    assert_eq!(a.tool_properties_title(), "Add Loop");
}
