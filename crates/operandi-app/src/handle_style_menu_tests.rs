//! MN-01: View > Tool Handles > Draw Style. The menu runs
//! `tool.handleStyle`, ticks the current style, saves the preference, and
//! Invisible hides the handles while hauling still drives the tool.
use super::tests::{draw, pointer};
use super::*;
use crate::handle_preferences::{HandleDetail, HandlePreferences};

/// 3×3 quads in z = 0 facing +z; point `r * 4 + c` at (c, r, 0).
fn grid() -> operandi_core::Mesh {
    let mut m = operandi_core::Mesh {
        vertices: (0..16)
            .map(|i| DVec3::new((i % 4) as f64, (i / 4) as f64, 0.))
            .collect(),
        ..Default::default()
    };
    for r in 0..3u32 {
        for c in 0..3u32 {
            let i = r * 4 + c;
            m.faces.push(operandi_core::Face {
                vertices: vec![i, i + 1, i + 5, i + 4],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    m
}

const LINES: [&str; 4] = [
    "tool.handleStyle invisible",
    "tool.handleStyle basic",
    "tool.handleStyle advanced",
    "tool.handleStyle default",
];

fn ticked(a: &Operandi) -> Vec<&'static str> {
    LINES
        .into_iter()
        .filter(|l| a.line_state(l).is_some_and(|s| s.on))
        .collect()
}

#[test]
fn view_menu_has_tool_handles_draw_style() {
    use crate::form::command_form::FormRef;
    let a = Operandi::empty();
    let sub = |c: &crate::form::command_form::Control| match &c.form {
        Some(FormRef::Id(id)) => Some(id.clone()),
        _ => None,
    };
    let view = a.forms.shipped("menu.view").unwrap();
    assert!(
        view.controls
            .iter()
            .any(|c| sub(c).as_deref() == Some("menu.view.toolHandles")),
        "View holds Tool Handles"
    );
    let handles = a.forms.shipped("menu.view.toolHandles").unwrap();
    assert_eq!(handles.label, "Tool Handles");
    assert_eq!(
        sub(&handles.controls[0]).as_deref(),
        Some("menu.view.toolHandles.drawStyle")
    );
    let style = a.forms.shipped("menu.view.toolHandles.drawStyle").unwrap();
    assert_eq!(style.label, "Draw Style");
    let got: Vec<_> = style
        .controls
        .iter()
        .map(|c| (c.label.as_str(), c.command.as_deref().unwrap()))
        .collect();
    assert_eq!(
        got,
        vec![
            ("Invisible", LINES[0]),
            ("Basic", LINES[1]),
            ("Advanced", LINES[2]),
            ("Default", LINES[3]),
        ]
    );
}

#[test]
fn the_current_style_is_ticked_and_default_puts_back_basic() {
    let mut a = Operandi::empty();
    assert_eq!(ticked(&a), vec![LINES[1]], "factory style is Basic");
    a.execute(LINES[0]);
    assert_eq!(a.handle_preferences.selected, HandleDetail::Invisible);
    assert_eq!(ticked(&a), vec![LINES[0]]);
    a.execute(LINES[2]);
    assert_eq!(ticked(&a), vec![LINES[2]]);
    a.execute(LINES[3]);
    assert_eq!(a.handle_preferences.selected, HandleDetail::Basic);
    assert_eq!(ticked(&a), vec![LINES[1]], "Default is never ticked itself");
}

#[test]
fn picking_a_style_saves_the_preference() {
    let mut a = Operandi::empty();
    let path = a.environment.paths().handle_preferences();
    a.execute(LINES[2]);
    assert_eq!(
        HandlePreferences::load(&path).selected,
        HandleDetail::Advanced
    );
    a.execute(LINES[0]);
    assert_eq!(
        HandlePreferences::load(&path).selected,
        HandleDetail::Invisible
    );
}

#[test]
fn invisible_hides_handles_but_hauling_still_moves() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = grid().into();
    a.camera.view = View::Front;
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![4];
    a.editor.undo.clear();
    draw(&mut a, &ctx, size, vec![]);
    a.execute(LINES[0]);
    a.activate(Tool::Move);
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    assert!(a.handles(r).is_empty(), "Invisible draws no handles");
    let before = a.editor.scene.active().unwrap().mesh.vertices[5];
    // A haul: a drag well away from where the handles would be.
    let from = r.left_top() + vec2(80., 80.);
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(from, true, Modifiers::NONE)],
    );
    for step in 1..=4 {
        let at = from + vec2(0., -15. * step as f32);
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(at)]);
    }
    let to = from + vec2(0., -60.);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(to, false, Modifiers::NONE)],
    );
    let after = a.editor.scene.active().unwrap().mesh.vertices[5];
    assert_ne!(before, after, "hauling still drives Move: {}", a.status);
}
