//! Painting builds: while a paint tool is on, no element highlighting
//! draws (the selected tint and the hover pre-highlight), so it doesn't
//! hide the paint. The selection stays and comes back as soon as another
//! tool is on (Jason, 2026-11-18). A sculpt brush keeps the selection
//! drawn, without the pre-highlight (Jason, 2026-11-23). Read back from
//! the graphics card.
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
    assert!(a.editor.session.has_active_task(), "{}", a.status);
    let back = picture(&mut a, &ctx);
    assert!(differing(&selected, &back) < 50, "the tint comes back");
    assert!(a.rollover_at(centre, r).is_some(), "hover comes back");

    // A sculpt brush keeps it drawn: the selection limits where it
    // reaches, so the area stays visible (Jason, 2026-11-23). No
    // pre-highlight under the brush.
    a.execute("tool.set xfrm.shear off");
    a.activate_sculpt(operandi_core::SculptBrush::Clay);
    assert!(a.sculpt.brush.is_some(), "{}", a.status);
    let sculpting = picture(&mut a, &ctx);
    assert!(
        differing(&selected, &sculpting) < 50,
        "the tint stays while sculpting: {} pixels differ",
        differing(&selected, &sculpting)
    );
    assert!(a.rollover_at(centre, r).is_none(), "no hover highlight");
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

/// The selected tint and outline drawn under a sculpt brush follow the
/// mesh as a stroke moves it, every frame, not the press-time positions.
/// A Move stroke drags a selected band up the Front view; the last frame
/// of the stroke matches the frame after release.
#[test]
fn the_selection_drawn_under_a_sculpt_stroke_follows_the_mesh() {
    let Some(state) = super::performance_tests::headless_gpu() else {
        println!("no graphics device");
        return;
    };
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    // A 20 × 20 grid, 2 m wide, facing the Front view.
    let n = 20u32;
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(glam::DVec3::new(
                2. * i as f64 / n as f64 - 1.,
                2. * j as f64 / n as f64 - 1.,
                0.,
            ));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let v = j * (n + 1) + i;
            mesh.faces.push(operandi_core::Face {
                vertices: vec![v, v + 1, v + n + 2, v + n + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = crate::camera::View::Front;
    a.camera.target = glam::DVec3::ZERO;
    a.camera.distance = 3.;
    a.run(Command::Mode(Mode::Polygon));
    // The middle four rows.
    a.editor.scene.selection.faces = (8 * n..12 * n).collect();
    let id = a.editor.scene.active().unwrap().id;
    a.editor.history.undo.clear();
    a.activate_sculpt(operandi_core::SculptBrush::Move);
    a.sculpt
        .settings
        .brush_mut(operandi_core::SculptBrush::Move)
        .tip
        .size = 150.;
    let before = picture(&mut a, &ctx);
    let c = a
        .camera
        .project(glam::DVec3::ZERO, a.viewport_rect)
        .unwrap()
        .0;
    let button = |p, pressed| egui::Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let _ = draw(&mut a, &ctx, SIZE, vec![egui::Event::PointerMoved(c)]);
    let _ = draw(&mut a, &ctx, SIZE, vec![button(c, true)]);
    let end = c + egui::vec2(0., -60.);
    for k in 1..=4 {
        let p = c + (end - c) * (k as f32 / 4.);
        let _ = draw(&mut a, &ctx, SIZE, vec![egui::Event::PointerMoved(p)]);
    }
    let during = picture(&mut a, &ctx);
    let _ = draw(&mut a, &ctx, SIZE, vec![button(end, false)]);
    let after = picture(&mut a, &ctx);
    assert_eq!(a.editor.history.undo.len(), 1, "{}", a.status);
    assert!(
        differing(&before, &after) > 500,
        "the band moved: {} pixels differ",
        differing(&before, &after)
    );
    assert!(
        differing(&during, &after) < 50,
        "the stroke's last frame draws the selection where the mesh is: {} pixels differ",
        differing(&during, &after)
    );
    assert_eq!(
        a.editor.scene.selection.faces_of(id).len(),
        (4 * n) as usize,
        "the stroke leaves the selection as it was"
    );
}
