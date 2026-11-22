//! Painting stage 7 in the app: sculpt mask brushes, the mask's controls,
//! and painting tips on sculpt brushes, through the Sculpt tab's buttons
//! and real pointer drags (PNT-151, 246, 260).
use super::toolbox_form_tests::button_by_line;
use super::*;
use operandi_core::SculptBrush;
use std::sync::Arc;

const SIZE: Vec2 = Vec2::new(1200., 2600.);
const N: u32 = 40;

fn grid() -> operandi_core::Mesh {
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=N {
        for i in 0..=N {
            mesh.vertices.push(DVec3::new(
                2. * i as f64 / N as f64 - 1.,
                2. * j as f64 / N as f64 - 1.,
                0.,
            ));
        }
    }
    for j in 0..N {
        for i in 0..N {
            let a = j * (N + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + N + 2, a + N + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    mesh
}

fn at(i: u32, j: u32) -> usize {
    (j * (N + 1) + i) as usize
}

fn app() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = grid().into();
    a.editor.undo.clear();
    a.camera.view = View::Front;
    a.camera.target = DVec3::ZERO;
    a.camera.distance = 3.;
    a.category = toolbox::Category::Sculpt;
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![]);
    (a, ctx)
}

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    super::tests::draw(a, ctx, SIZE, events);
    a.keep_sculpt_exclusive();
}

fn press(p: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn stroke(a: &mut Operandi, ctx: &Context, from: Pos2, by: Vec2, steps: usize) {
    frame(a, ctx, vec![Event::PointerMoved(from)]);
    frame(a, ctx, vec![press(from, true)]);
    for k in 1..=steps {
        frame(
            a,
            ctx,
            vec![Event::PointerMoved(from + by * (k as f32 / steps as f32))],
        );
    }
    frame(a, ctx, vec![press(from + by, false)]);
    frame(a, ctx, vec![]);
}

/// Clicks the Sculpt tab button that runs `line` (laid out at `SIZE`).
fn click_line(a: &mut Operandi, ctx: &Context, line: &str, _mods: Modifiers) {
    frame(a, ctx, vec![]);
    let at = button_by_line(a, line).center();
    frame(a, ctx, vec![Event::PointerMoved(at)]);
    frame(a, ctx, vec![press(at, true), press(at, false)]);
}

fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera
        .project(p, a.viewport_rect)
        .map(|(q, _)| q)
        .unwrap()
}

fn points(a: &Operandi) -> Vec<DVec3> {
    a.editor.scene.active().unwrap().mesh.vertices.clone()
}

#[test]
fn a3_mask_paint_holds_points_from_the_push_brush_and_undoes_in_one_step() {
    let (mut a, ctx) = app();
    let id = a.editor.scene.active().unwrap().id;
    // The Sculpt tab's Mask Paint button, then a drag over the left half.
    click_line(
        &mut a,
        &ctx,
        "operandi.paint.maskBrush kind:paint",
        Modifiers::NONE,
    );
    assert_eq!(
        a.mask_brush_active(),
        Some(SculptBrush::VmPaint),
        "{}",
        a.status
    );
    a.sculpt.settings.brush_mut(SculptBrush::VmPaint).tip.size = 60.;
    // N32 (Jason, 2026-11-19): one stroke adds its strength, so a full
    // mask in one stroke needs full strength.
    a.sculpt
        .settings
        .brush_mut(SculptBrush::VmPaint)
        .offset_amount = 1.;
    let steps = a.editor.undo.len();
    let left = screen(&a, DVec3::new(-0.25, -0.5, 0.));
    let dy = screen(&a, DVec3::new(0., 0.5, 0.)).y - left.y;
    stroke(&mut a, &ctx, left, vec2(0., dy), 12);
    assert_eq!(
        a.editor.undo.len(),
        steps + 1,
        "one undo step: {}",
        a.status
    );
    assert_eq!(a.editor.undo.last().unwrap().label, "Mask Paint");
    let mask = a.editor.scene.paint.masks.get(&id).expect("a mask").clone();
    assert!(
        mask[at(15, 20)] > 0.9,
        "left side masked: {}",
        mask[at(15, 20)]
    );
    assert_eq!(mask[at(32, 20)], 0., "right side free");
    // The mesh itself is unchanged: no hidden map left behind.
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert!(mesh.attributes.point_weights.is_empty());
    assert!(mesh.vertices.iter().all(|p| p.z == 0.));
    // Push across the middle: the masked left doesn't move, the right does.
    a.activate_sculpt(SculptBrush::Push);
    a.sculpt.settings.brush_mut(SculptBrush::Push).tip.size = 400.;
    let c = screen(&a, DVec3::ZERO);
    stroke(&mut a, &ctx, c, vec2(4., 0.), 3);
    let v = points(&a);
    assert!(v[at(15, 20)].z.abs() < 1e-9, "masked: {}", v[at(15, 20)].z);
    assert!(v[at(30, 20)].z.abs() > 1e-4, "free: {}", v[at(30, 20)].z);
    // Undo the push, invert the mask, push again: now the left moves.
    a.execute("app.undo");
    a.execute("operandi.paint.maskSet invert:true");
    stroke(&mut a, &ctx, c, vec2(4., 0.), 3);
    let v = points(&a);
    assert!(v[at(15, 20)].z.abs() > 1e-4);
    assert!(v[at(30, 20)].z.abs() < 1e-9);
    a.execute("app.undo");
    // Clear Mask: one step, gone.
    a.execute("operandi.paint.maskClear");
    assert!(!a.editor.scene.paint.masks.contains_key(&id));
    a.execute("app.undo");
    assert!(
        a.editor.scene.paint.masks.contains_key(&id),
        "Clear Mask undoes"
    );
}

#[test]
fn ctrl_mask_paint_erases_and_the_mask_shows_red() {
    let (mut a, ctx) = app();
    let id = a.editor.scene.active().unwrap().id;
    let n = a.editor.scene.active().unwrap().mesh.vertices.len();
    a.run_mask_edit(id, Some(Arc::new(vec![1.; n])), "Mask Paint");
    a.execute("operandi.paint.maskBrush kind:paint");
    a.sculpt.settings.brush_mut(SculptBrush::VmPaint).tip.size = 120.;
    // N32: one stroke removes its strength; full strength erases fully.
    a.sculpt
        .settings
        .brush_mut(SculptBrush::VmPaint)
        .offset_amount = 1.;
    let c = screen(&a, DVec3::ZERO);
    frame(&mut a, &ctx, vec![Event::PointerMoved(c)]);
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::NONE
    };
    let ev = |p: Pos2, pressed: bool| Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: ctrl,
    };
    super::tests::draw_held(&mut a, &ctx, SIZE, vec![ev(c, true)], ctrl);
    for k in 1..6 {
        super::tests::draw_held(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(c + vec2(k as f32 * 2., 0.))],
            ctrl,
        );
    }
    super::tests::draw_held(&mut a, &ctx, SIZE, vec![ev(c + vec2(10., 0.), false)], ctrl);
    let mask = a.editor.scene.paint.masks.get(&id).unwrap();
    assert!(
        mask[at(20, 20)] < 0.5,
        "Ctrl erased under the brush: {}",
        mask[at(20, 20)]
    );
    assert!(mask[at(2, 2)] > 0.99, "elsewhere still masked");
}

#[test]
fn a1_texture_tip_on_push_through_the_sculpt_tab_pushes_the_picture() {
    let (mut a, ctx) = app();
    // A 64 px picture: white disc on mid-grey.
    a.run(Command::Paint(operandi_core::paint::PaintEdit::NewImage(
        operandi_core::paint::NewImage {
            name: Some("Disc".into()),
            resolution: 64,
            float: true,
            color: Some([0.5, 0.5, 0.5]),
            ..Default::default()
        },
    )));
    let img = a.editor.paint.last_image.unwrap();
    let mut px = (*a.editor.scene.paint.buffers[&img]).clone();
    for y in 0..64 {
        for x in 0..64 {
            let d = ((x as f64 - 31.5).powi(2) + (y as f64 - 31.5).powi(2)).sqrt();
            if d < 12. {
                px.set(y * 64 + x, [1., 1., 1., 1.]);
            }
        }
    }
    a.editor.scene.paint.buffers.insert(img, Arc::new(px));
    a.activate_sculpt(SculptBrush::Push);
    a.sculpt.settings.brush_mut(SculptBrush::Push).tip.size = 200.;
    frame(&mut a, &ctx, vec![]);
    click_line(
        &mut a,
        &ctx,
        "operandi.paint.sculptTip kind:image",
        Modifiers::NONE,
    );
    assert!(
        a.line_state("operandi.paint.sculptTip kind:image")
            .unwrap()
            .on
    );
    a.execute(&format!(
        "operandi.paint.sculptTipSet image:{img} useAsMask:true offset:0.5"
    ));
    let c = screen(&a, DVec3::ZERO);
    stroke(&mut a, &ctx, c, vec2(1., 0.), 1);
    let v = points(&a);
    assert!(
        v[at(20, 20)].z.abs() > 1e-4,
        "the disc pushed: {}",
        v[at(20, 20)].z
    );
    let r = 200. / (screen(&a, DVec3::X).x - c.x) as f64;
    // Half way between the disc and the dab's rim: grey, no step.
    let k = (20. + (r * 0.6) * 20.).round() as u32;
    if k < N {
        assert!(
            v[at(k, 20)].z.abs() < 1e-6,
            "grey border: {}",
            v[at(k, 20)].z
        );
    }
    // The paint tools keep their own tip (PNT-42/Q10).
    assert_eq!(
        a.painting.brush.tips.choice,
        operandi_core::paint_tips::TipChoice::Smooth
    );
    // Back to Smooth on the sculpt tab clears the sculpt tip.
    a.execute("brush.smooth");
    assert_eq!(
        a.painting.brush.sculpt_tips.choice,
        operandi_core::paint_tips::TipChoice::Smooth
    );
}

#[test]
fn projection_ink_is_greyed_on_sculpt_brushes() {
    let (mut a, _ctx) = app();
    a.activate_sculpt(SculptBrush::Clay);
    assert_eq!(
        a.line_state("operandi.paint.ink kind:projection")
            .unwrap()
            .unavailable
            .as_deref(),
        Some(crate::painting_sculpt::PAINT_ONLY_INK)
    );
}

#[test]
fn geometry_to_brush_from_the_utilities_tab_makes_the_texture_tip() {
    let (mut a, ctx) = app();
    {
        let mesh = &mut a.editor.scene.active_mut().unwrap().mesh;
        let mut m = mesh.to_owned_value();
        for p in &mut m.vertices {
            p.z = 0.3 * (-(p.x * p.x + p.y * p.y) / 0.1).exp();
        }
        *mesh = m.into();
    }
    // Bare: the dialog opens.
    a.execute("operandi.paint.geometryToBrush");
    assert!(a.painting.geometry_brush.is_some());
    a.painting.geometry_brush = None;
    a.execute("operandi.paint.geometryToBrush mode:height resolution:64 iterations:200");
    let b = &a.painting.brush.sculpt_tips;
    assert_eq!(
        b.choice,
        operandi_core::paint_tips::TipChoice::Image,
        "{}",
        a.status
    );
    let img = b.image.image.expect("the made picture");
    let px = &a.editor.scene.paint.buffers[&img];
    assert!(px.get(32 * 64 + 32)[0] > 0.9);
    // A closed mesh says why on screen.
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Box,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.execute("operandi.paint.geometryToBrush mode:height");
    assert_eq!(
        a.painting.brush.notice.as_ref().map(|(t, _)| t.as_str()),
        Some(operandi_core::paint_geometry_brush::CLOSED_PATCH)
    );
    frame(&mut a, &ctx, vec![]);
}

#[test]
fn the_tablet_nozzle_scatters_sculpt_dabs() {
    let run = |nozzle: bool| {
        let (mut a, ctx) = app();
        a.activate_sculpt(SculptBrush::Push);
        a.sculpt.settings.brush_mut(SculptBrush::Push).tip.size = 30.;
        if nozzle {
            a.execute("operandi.paint.nozzle");
            a.execute("operandi.paint.tipSet jitter:true jitterPosition:4");
        }
        let c = screen(&a, DVec3::ZERO);
        stroke(&mut a, &ctx, c - vec2(60., 0.), vec2(120., 0.), 12);
        points(&a)
    };
    let (plain, scattered) = (run(false), run(true));
    // The plain stroke moves only a band along y = 0; the nozzle's
    // position jitter reaches rows off it.
    let off_row = |v: &[DVec3]| {
        v.iter()
            .filter(|p| p.z.abs() > 1e-6 && p.y.abs() > 0.1)
            .count()
    };
    assert!(
        off_row(&scattered) > off_row(&plain),
        "{} vs {}",
        off_row(&scattered),
        off_row(&plain)
    );
}
