//! Painting builds: while a paint tool or sculpt brush is on, no element
//! highlighting draws (the selected tint and the hover pre-highlight), so
//! it doesn't hide the paint. The selection stays and comes back as soon
//! as another tool is on (Jason, 2026-11-18). Read back from the graphics
//! card.
use super::*;
use crate::tests::draw;
use operandi_core::paint_stroke::PaintTool;

const SIZE: egui::Vec2 = egui::vec2(900., 600.);

fn picture(a: &mut Operandi, ctx: &Context) -> Vec<u8> {
    for _ in 0..3 {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
    a.viewport_gpu.as_ref().unwrap().read_views().remove(0).2
}

fn differing(p: &[u8], q: &[u8]) -> usize {
    p.chunks(4)
        .zip(q.chunks(4))
        .filter(|(p, q)| p.iter().zip(*q).any(|(a, b)| a.abs_diff(*b) > 12))
        .count()
}

#[test]
fn a_paint_tool_hides_the_selection_highlight_until_another_tool() {
    let Some(state) = super::performance_tests::headless_gpu() else {
        println!("no graphics device");
        return;
    };
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
            .unwrap()
            .into();
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.run(Command::Mode(Mode::Polygon));
    let plain = picture(&mut a, &ctx);
    let id = a.editor.scene.active().unwrap().id;
    a.editor.scene.selection.faces = (0..6).collect();
    let selected = picture(&mut a, &ctx);
    assert!(
        differing(&plain, &selected) > 1000,
        "the selection tints the cube"
    );
    let centre = a.viewport_rect.center();
    let r = a.viewport_rect;
    assert!(a.rollover_at(centre, r).is_some(), "hover pre-highlights");

    a.execute("operandi.paint.paintbrush");
    assert_eq!(a.painting.brush.tool, Some(PaintTool::Paintbrush));
    let painting = picture(&mut a, &ctx);
    assert!(
        differing(&plain, &painting) < 50,
        "no tint while painting: {} pixels differ",
        differing(&plain, &painting)
    );
    assert!(a.rollover_at(centre, r).is_none(), "no hover highlight");
    assert_eq!(
        a.editor.scene.selection.faces_of(id).len(),
        6,
        "the selection stays"
    );

    // Another tool (Shear): the highlight is back.
    a.execute("tool.set xfrm.shear on");
    assert!(a.editor.has_active_task(), "{}", a.status);
    let back = picture(&mut a, &ctx);
    assert!(differing(&selected, &back) < 50, "the tint comes back");
    assert!(a.rollover_at(centre, r).is_some(), "hover comes back");

    // A sculpt brush hides it too (painting builds).
    a.execute("tool.set xfrm.shear off");
    a.activate_sculpt(operandi_core::SculptBrush::Clay);
    assert!(a.sculpt.brush.is_some(), "{}", a.status);
    let sculpting = picture(&mut a, &ctx);
    assert!(
        differing(&plain, &sculpting) < 50,
        "no tint while sculpting"
    );
}

/// Texture Locators follow the same rule while painting, unless Texture >
/// Paint Tools > Show Texture Locators while Painting is on (off by
/// default).
#[test]
fn texture_locators_hide_while_painting_unless_asked_for() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
            .unwrap()
            .into();
    assert!(!a.brush_hides_locators(), "no paint tool: they draw");
    a.execute("operandi.paint.paintbrush");
    assert!(a.brush_hides_locators(), "hidden while painting");
    a.execute("operandi.paint.showLocators on");
    assert!(!a.brush_hides_locators(), "shown when asked for");
    a.execute("operandi.paint.showLocators off");
    a.execute("tool.set xfrm.shear on");
    assert!(!a.brush_hides_locators(), "another tool: they draw again");
}
