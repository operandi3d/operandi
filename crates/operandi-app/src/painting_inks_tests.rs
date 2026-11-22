//! Painting stage 5 in the app (painting_inks.rs): the ink buttons and
//! rows, Image Ink's widget, handles and keys, Stamp, Projection Ink's
//! greying and the Curve Constraint through the viewport's press and drag.
use super::*;
use operandi_core::paint::{NewImage, PaintEdit};
use operandi_core::paint_inks::{Ink, InkKind};
use operandi_core::paint_stroke::PaintTool;
use std::sync::Arc;

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.execute("operandi.paint.addTexture kind:color resolution:256 color:0,0,0");
    assert!(a.paint_target().is_some(), "{}", a.status);
    a.viewport_rect = rect();
    a
}

fn rect() -> Rect {
    Rect::from_min_size(pos2(0., 0.), vec2(800., 600.))
}

/// A 64 px "photo": left half red, right half green.
fn photo(a: &mut Operandi) -> u64 {
    a.run(Command::Paint(PaintEdit::NewImage(NewImage {
        name: Some("Photo".into()),
        resolution: 64,
        color: Some([0., 0., 0.]),
        ..NewImage::default()
    })));
    let id = a.editor.paint.last_image.unwrap();
    let mut px = (*a.editor.scene.paint.buffers[&id]).clone();
    for y in 0..64 {
        for x in 0..64 {
            let c = if x < 32 {
                [1., 0., 0., 1.]
            } else {
                [0., 1., 0., 1.]
            };
            px.set(y * 64 + x, c);
        }
    }
    a.editor.scene.paint.buffers.insert(id, Arc::new(px));
    id
}

fn target(a: &Operandi) -> Arc<operandi_core::paint::pixels::PaintImage> {
    let id = a.current_image().unwrap();
    a.editor.scene.paint.buffers[&id].clone()
}

/// (red, green) pixel counts of the paint target.
fn reds_greens(a: &Operandi) -> (usize, usize) {
    let px = target(a);
    let mut out = (0, 0);
    for i in 0..px.pixel_count() {
        let c = px.get(i);
        if c[0] > 0.9 && c[1] < 0.1 {
            out.0 += 1;
        }
        if c[1] > 0.9 && c[0] < 0.1 {
            out.1 += 1;
        }
    }
    out
}

fn dab(a: &mut Operandi, p: Pos2) {
    let r = rect();
    let kind = a.paint_press(p, r, Modifiers::NONE);
    assert!(matches!(kind, input::GestureKind::Paint), "{}", a.status);
    a.paint_drag(p + vec2(1., 0.), r);
    a.finish_paint_stroke();
}

#[test]
fn ink_buttons_stack_in_order_and_turn_off_again() {
    let mut a = app();
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.ink kind:random");
    a.execute("operandi.paint.ink kind:parametric");
    let kinds: Vec<InkKind> = a.painting.brush.ink.inks.0.iter().map(Ink::kind).collect();
    assert_eq!(kinds, [InkKind::Random, InkKind::Parametric]);
    assert!(a.line_state("operandi.paint.ink kind:random").unwrap().on);
    assert_eq!(
        a.painting.brush.tool,
        Some(PaintTool::Paintbrush),
        "the tool stays"
    );
    a.execute("operandi.paint.ink kind:random");
    assert!(!a.line_state("operandi.paint.ink kind:random").unwrap().on);
    assert_eq!(a.painting.brush.ink.inks.0.len(), 1);
    // Parametric's starting rows (PNT-181).
    let Ink::Parametric(p) = a.painting.brush.ink.inks.0[0] else {
        panic!()
    };
    assert_eq!(
        p.parameter,
        operandi_core::paint_inks::Parameter::IncidenceAngle
    );
    assert!(p.use_as_mask);
    a.execute("operandi.paint.inkSet kind:parametric parameter:Slope useAsMask:false");
    let Ink::Parametric(p) = a.painting.brush.ink.inks.0[0] else {
        panic!()
    };
    assert_eq!(p.parameter, operandi_core::paint_inks::Parameter::Slope);
    assert!(!p.use_as_mask);
}

#[test]
fn acceptance_25_image_ink_in_the_view_reveals_the_photo_and_moves_with_the_widget() {
    let mut a = app();
    let id = photo(&mut a);
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.set size:6");
    a.execute("operandi.paint.ink kind:image");
    a.execute(&format!("operandi.paint.inkSet kind:image image:{id}"));
    let r = rect();
    // The first click shows the widget and paints nothing (PNT-171).
    let before = target(&a);
    let kind = a.paint_press(r.center(), r, Modifiers::NONE);
    assert!(matches!(kind, input::GestureKind::Blocked));
    assert!(a.painting.brush.ink.widget_shown);
    assert!(Arc::ptr_eq(&before, &target(&a)));
    // Fitted to the view's height, centred: the left of centre is red.
    dab(&mut a, r.center() - vec2(20., 0.));
    let (red, green) = reds_greens(&a);
    assert!(red > 0 && green == 0, "{red} {green}: {}", a.status);
    // Drag the centre handle 300 px left: the green half is over the
    // centre now.
    let kind = a.paint_press(r.center(), r, Modifiers::NONE);
    assert!(matches!(kind, input::GestureKind::Paint));
    a.paint_drag(r.center() - vec2(300., 0.), r);
    a.finish_paint_stroke();
    let i = *a.painting.brush.ink.inks.image().unwrap();
    assert_eq!(
        i.pos,
        Some(glam::DVec2::new(100., 300.)),
        "recorded on release"
    );
    dab(&mut a, r.center() - vec2(20., 0.));
    assert!(reds_greens(&a).1 > 0, "green revealed after the move");
    // Turning it off and on puts it back in the view's centre.
    a.execute("operandi.paint.ink kind:image");
    a.execute("operandi.paint.ink kind:image");
    assert_eq!(a.painting.brush.ink.inks.image().unwrap().pos, None);
}

#[test]
fn image_ink_keys_turn_and_size_the_widget_only_while_it_shows() {
    let mut a = app();
    let id = photo(&mut a);
    a.execute("operandi.paint.airbrush");
    a.execute("operandi.paint.ink kind:image");
    a.execute(&format!("operandi.paint.inkSet kind:image image:{id}"));
    let ctx = Context::default();
    let press = |key: Key| egui::RawInput {
        events: vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    // Hidden widget: E is left for the global Rotate.
    let _ = ctx.run(press(Key::E), |ctx| assert!(!a.ink_widget_keys(ctx)));
    a.execute("operandi.paint.inkWidget action:show");
    for (key, check) in [(Key::E, 5f64), (Key::R, 0.), (Key::F, 0.), (Key::D, 0.)] {
        let _ = ctx.run(press(key), |ctx| assert!(a.ink_widget_keys(ctx), "{key:?}"));
        let i = a.painting.brush.ink.inks.image().unwrap();
        assert!((i.rotation.to_degrees() - check).abs() < 1e-9, "{key:?}");
        if key == Key::F {
            assert!((i.scale - crate::painting_inks::KEY_SCALE).abs() < 1e-9);
        }
    }
    assert!((a.painting.brush.ink.inks.image().unwrap().scale - 1.).abs() < 1e-9);
}

#[test]
fn image_ink_stamp_is_one_click_and_one_undo_step() {
    let mut a = app();
    let id = photo(&mut a);
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.ink kind:image");
    a.execute(&format!(
        "operandi.paint.inkSet kind:image image:{id} stamp:true"
    ));
    a.execute("operandi.paint.inkWidget action:show");
    let steps = a.editor.undo.len();
    let r = rect();
    let kind = a.paint_press(pos2(700., 50.), r, Modifiers::NONE);
    assert!(matches!(kind, input::GestureKind::Blocked), "{}", a.status);
    assert_eq!(a.editor.undo.len(), steps + 1, "{}", a.status);
    let (red, green) = reds_greens(&a);
    assert!(
        red > 100 && green > 100,
        "both halves over the sphere: {red} {green}"
    );
}

#[test]
fn projection_ink_needs_a_camera_background_image() {
    let mut a = app();
    a.execute("operandi.paint.paintbrush");
    let state = a.line_state("operandi.paint.ink kind:projection").unwrap();
    assert_eq!(state.unavailable.as_deref(), Some("Needs a camera"));
    let mesh = a.editor.scene.selection.item;
    a.run(Command::NewMesh);
    let cam = a.editor.scene.selection.item.unwrap();
    a.editor.scene.item_mut(cam).unwrap().camera =
        Some(operandi_core::camera_light::CameraSettings::default());
    a.editor.scene.selection.item = mesh;
    let state = a.line_state("operandi.paint.ink kind:projection").unwrap();
    assert_eq!(
        state.unavailable.as_deref(),
        Some(operandi_core::paint_inks::NO_CAMERA_IMAGE)
    );
    let id = photo(&mut a);
    a.editor
        .scene
        .item_mut(cam)
        .unwrap()
        .camera
        .as_mut()
        .unwrap()
        .background
        .image = Some(id);
    assert!(
        a.line_state("operandi.paint.ink kind:projection")
            .unwrap()
            .unavailable
            .is_none()
    );
    a.execute("operandi.paint.ink kind:projection");
    assert!(a.painting.brush.ink.inks.has(InkKind::Projection));
}

#[test]
fn sculpt_brushes_keep_inks_greyed() {
    let a = app();
    let state = a.line_state("operandi.paint.sculptInks").unwrap();
    assert_eq!(
        state.unavailable.as_deref(),
        Some(crate::painting_inks::SCULPT_INKS)
    );
}

#[test]
fn acceptance_28_background_2d_a_drag_near_a_background_curve_follows_it() {
    let mut a = app();
    let painted = a.editor.scene.selection.item.unwrap();
    // A straight curve across the sphere's front, in another mesh.
    a.run(Command::NewMesh);
    let mut m = operandi_core::Mesh {
        vertices: (0..5)
            .map(|k| glam::DVec3::new(-0.6 + f64::from(k) * 0.3, 0.1, 0.6))
            .collect(),
        ..Default::default()
    };
    m.faces = vec![operandi_core::Face {
        vertices: vec![0, 1, 2, 3, 4],
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }]
    .into();
    m.attributes.representations = vec![operandi_core::PolygonRepresentation::Polyline {
        closed: false,
        curve: Default::default(),
    }];
    a.editor.scene.active_mut().unwrap().mesh = m.into();
    a.editor.scene.selection.item = Some(painted);
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.set size:3 curveConstraint:background2D");
    let r = rect();
    let view = a.paint_view(r);
    let y = view.project(glam::DVec3::new(0., 0.1, 0.6)).unwrap().0.y as f32;
    let free = target(&a);
    let kind = a.paint_press(pos2(380., y + 12.), r, Modifiers::NONE);
    assert!(matches!(kind, input::GestureKind::Paint), "{}", a.status);
    for k in 0..20 {
        let wobble = if k % 2 == 0 { 12. } else { -12. };
        a.paint_drag(pos2(380. + k as f32 * 2., y + wobble), r);
    }
    a.finish_paint_stroke();
    let painted_px = target(&a);
    assert!(!Arc::ptr_eq(&free, &painted_px));
    // The wobble is gone: the same drag without the constraint covers a
    // band about 24 px tall, the constrained one a single tip's width.
    let constrained = (0..painted_px.pixel_count())
        .filter(|&i| painted_px.get(i)[0] > 0.5)
        .count();
    a.execute("app.undo");
    a.execute("operandi.paint.set curveConstraint:none");
    a.paint_press(pos2(380., y + 12.), r, Modifiers::NONE);
    for k in 0..20 {
        let wobble = if k % 2 == 0 { 12. } else { -12. };
        a.paint_drag(pos2(380. + k as f32 * 2., y + wobble), r);
    }
    a.finish_paint_stroke();
    let loose = target(&a);
    let unconstrained = (0..loose.pixel_count())
        .filter(|&i| loose.get(i)[0] > 0.5)
        .count();
    assert!(
        constrained > 0 && unconstrained > constrained * 2,
        "{constrained} vs {unconstrained}"
    );
}
