//! Painting stage 3's tools (PNT-90..120), acceptance 14–19, on a flat
//! plane whose UVs match its shape: Smudge, Blur, Sharpen, Clone, Line,
//! Fill, the gradients and the Lasso falloff. Painting builds only.
#![cfg(feature = "painting")]
use glam::{DVec2, DVec3};
use operandi_core::paint::*;
use operandi_core::paint_lasso::{Lasso, LassoStyle};
use operandi_core::paint_stroke::*;
use operandi_core::*;
use std::sync::Arc;

const SIDE: u32 = 128;

/// A 2 × 2 m plane at z = 0, UV u = (x + 1) / 2, v = (y + 1) / 2, with a
/// black 128 px texture.
fn plane() -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(-1., -1., 0.),
            DVec3::new(1., -1., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(-1., 1., 0.),
        ],
        ..Mesh::default()
    };
    m.faces = vec![Face {
        vertices: vec![0, 1, 2, 3],
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }]
    .into();
    let uv = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)].map(|(u, v)| Some(DVec2::new(u, v)));
    m.uv_maps.insert("Texture".into(), vec![uv.to_vec()]);
    e.scene.active_mut().unwrap().mesh = m.into();
    e.dispatch(Command::Paint(PaintEdit::AddTexture {
        kind: TextureKind::Color,
        image: NewImage {
            resolution: SIDE,
            color: Some([0., 0., 0.]),
            ..NewImage::default()
        },
        selected: None,
        item: None,
    }))
    .unwrap();
    let image = e.paint.last_image.unwrap();
    (e, image)
}

fn view() -> PaintView {
    PaintView::looking(DVec3::new(0., 0., 3.), DVec3::ZERO, DVec2::new(400., 400.))
}

/// Screen point of plane point (x, y).
fn screen(x: f64, y: f64) -> DVec2 {
    view().project(DVec3::new(x, y, 0.)).unwrap().0
}

/// The pixel at plane point (x, y).
fn px_at(x: f64, y: f64) -> usize {
    let (u, v) = ((x + 1.) / 2., (y + 1.) / 2.);
    let col = ((u * f64::from(SIDE)) as u32).min(SIDE - 1);
    let row = (((1. - v) * f64::from(SIDE)) as u32).min(SIDE - 1);
    (row * SIDE + col) as usize
}

fn set_image(e: &mut Editor, image: u64, f: impl Fn(f64, f64) -> [f32; 4]) {
    let mut px = (*e.scene.paint.buffers[&image]).clone();
    for row in 0..SIDE {
        for col in 0..SIDE {
            let x = (f64::from(col) + 0.5) / f64::from(SIDE) * 2. - 1.;
            let y = 1. - (f64::from(row) + 0.5) / f64::from(SIDE) * 2.;
            px.set((row * SIDE + col) as usize, f(x, y));
        }
    }
    e.scene.paint.buffers.insert(image, Arc::new(px));
}

fn stroke(
    e: &Editor,
    image: u64,
    tool: PaintTool,
    s: PaintSettings,
    lasso: Option<&Lasso>,
) -> PaintStroke {
    PaintStroke::begin(StrokeSetup {
        scene: &e.scene,
        item: e.scene.selection.item.unwrap(),
        image,
        pixels: e.scene.paint.buffers[&image].clone(),
        uv_map: None,
        view: view(),
        tool,
        settings: s,
        symmetry: Symmetry::default(),
        falloff: None,
        lasso,
    })
    .unwrap()
}

fn sized(size: f64) -> PaintSettings {
    let mut s = PaintSettings::default();
    s.tip.size = size;
    s
}

fn red(p: &operandi_paint::PaintImage, x: f64, y: f64) -> f32 {
    p.get(px_at(x, y))[0]
}

#[test]
fn smudge_drags_a_painted_edge_along_the_stroke() {
    let (mut e, image) = plane();
    set_image(&mut e, image, |x, _| {
        if x < 0. {
            [1., 0., 0., 1.]
        } else {
            [0., 0., 0., 1.]
        }
    });
    let mut s = stroke(&e, image, PaintTool::Smudge, sized(30.), None);
    assert_eq!(red(s.pixels(), 0.1, 0.), 0.);
    for k in 0..30 {
        let x = -0.3 + f64::from(k) * 0.02;
        let p = screen(x, 0.);
        s.dab(p, 30., None, 1.);
    }
    // Red carried past the edge, toward the stroke's end.
    assert!(
        red(s.pixels(), 0.1, 0.) > 0.2,
        "{}",
        red(s.pixels(), 0.1, 0.)
    );
    // Far from the stroke nothing moved.
    assert_eq!(red(s.pixels(), 0.1, 0.8), 0.);
    assert_eq!(s.label(), "Smudge Stroke");
}

#[test]
fn blur_softens_an_edge_and_sharp_crisps_it() {
    let (mut e, image) = plane();
    set_image(&mut e, image, |x, _| {
        if x < 0. {
            [1., 1., 1., 1.]
        } else {
            [0., 0., 0., 1.]
        }
    });
    let mut blur = stroke(&e, image, PaintTool::Blur, sized(40.), None);
    for _ in 0..5 {
        blur.dab(screen(0., 0.), 40., None, 1.);
    }
    let left = red(blur.pixels(), -0.01, 0.);
    let right = red(blur.pixels(), 0.01, 0.);
    assert!(
        left < 1. && right > 0.,
        "the edge is softer: {left} {right}"
    );
    // Sharp on a soft ramp: values move away from the local average.
    set_image(&mut e, image, |x, _| {
        let v = (0.5 + x * 2.).clamp(0., 1.) as f32;
        [v, v, v, 1.]
    });
    let before = (*e.scene.paint.buffers[&image]).clone();
    let mut sharp = stroke(&e, image, PaintTool::Sharpen, sized(40.), None);
    for _ in 0..3 {
        sharp.dab(screen(0.2, 0.), 40., None, 1.);
    }
    // Where the ramp bends (x ≈ 0.25), the bright side gets brighter.
    let b = before.get(px_at(0.24, 0.))[0];
    let a = sharp.pixels().get(px_at(0.24, 0.))[0];
    assert!(a > b, "sharper: {b} -> {a}");
    assert_eq!(sharp.label(), "Sharpen Stroke");
    assert_eq!(PaintTool::Sharpen.button_label(), "Sharp");
}

#[test]
fn clone_copies_from_the_source_offset_and_keeps_it() {
    let (mut e, image) = plane();
    // A red patch on the left.
    set_image(&mut e, image, |x, y| {
        if (x + 0.5).abs() < 0.2 && y.abs() < 0.2 {
            [1., 0., 0., 1.]
        } else {
            [0., 0., 0., 1.]
        }
    });
    let mut s = stroke(&e, image, PaintTool::Clone, sized(20.), None);
    // No source: nothing painted (PNT-95).
    s.dab(screen(0.5, 0.), 20., None, 1.);
    assert!(!s.changed);
    // Source at the patch, brush on the right: the offset is fixed.
    let offset = screen(-0.5, 0.) - screen(0.5, 0.);
    s.clone_offset = Some(offset);
    s.dab(screen(0.5, 0.), 20., None, 1.);
    assert!(red(s.pixels(), 0.5, 0.) > 0.9, "the patch is copied");
    // A next stroke with the same offset continues from it.
    let mut next = stroke(&e, image, PaintTool::Clone, sized(20.), None);
    next.clone_offset = Some(offset);
    next.dab(screen(0.55, 0.05), 20., None, 1.);
    assert!(red(next.pixels(), 0.55, 0.05) > 0.9);
    assert_eq!(red(next.pixels(), 0.5, 0.7), 0.);
}

#[test]
fn line_circle_and_rectangle_stroke_outlines_with_the_tip() {
    let (e, image) = plane();
    let mut s = sized(4.);
    s.tip.kind = operandi_core::brush_engine::TipKind::Hard;
    let mut line = stroke(&e, image, PaintTool::Line, s, None);
    line.shape(screen(-0.6, 0.), screen(0.6, 0.), 4.);
    for x in [-0.5, -0.2, 0.0, 0.3, 0.5] {
        assert!(red(line.pixels(), x, 0.) > 0.9, "on the line at {x}");
    }
    assert_eq!(red(line.pixels(), 0., 0.5), 0.);
    let mut circle_s = s;
    circle_s.shape = LineShape::Circle;
    let mut circle = stroke(&e, image, PaintTool::Line, circle_s, None);
    circle.shape(screen(0., 0.), screen(0.5, 0.), 4.);
    assert_eq!(red(circle.pixels(), 0., 0.), 0., "not filled");
    for (x, y) in [(0.5, 0.), (-0.5, 0.), (0., 0.5), (0., -0.5)] {
        assert!(red(circle.pixels(), x, y) > 0.9, "rim at {x},{y}");
    }
    let mut rect_s = s;
    rect_s.shape = LineShape::Rectangle;
    let mut rect = stroke(&e, image, PaintTool::Line, rect_s, None);
    rect.shape(screen(-0.4, -0.3), screen(0.4, 0.3), 4.);
    assert!(red(rect.pixels(), 0.4, 0.) > 0.9 && red(rect.pixels(), 0., 0.3) > 0.9);
    assert_eq!(red(rect.pixels(), 0., 0.), 0.);
}

#[test]
fn fill_replaces_only_near_matching_colours_everywhere() {
    let (mut e, image) = plane();
    set_image(&mut e, image, |x, y| {
        let v = if x < -0.3 {
            0.5
        } else if x < 0.3 {
            0.55
        } else {
            0.8
        };
        // A second 0.5 patch, not connected to the first.
        let v = if x > 0.6 && y > 0.6 { 0.5 } else { v };
        [v, v, v, 1.]
    });
    let mut s = PaintSettings::default();
    s.fg.color = [1., 0., 0.];
    let mut fill = stroke(&e, image, PaintTool::Fill, s, None);
    let seed = fill.pixel_at(screen(-0.6, 0.)).unwrap();
    fill.fill(seed);
    let p = fill.pixels();
    assert_eq!(p.get(px_at(-0.6, 0.))[1], 0., "the seed's colour is filled");
    assert_eq!(p.get(px_at(0., 0.))[1], 0., "within 10 %");
    assert!((p.get(px_at(0.5, 0.))[1] - 0.8).abs() < 0.01, "0.8 is kept");
    assert_eq!(
        p.get(px_at(0.8, 0.8))[1],
        0.,
        "not only the connected region"
    );
    assert_eq!(fill.label(), "Fill");
}

#[test]
fn gradients_flood_the_whole_image() {
    let (e, image) = plane();
    let mut s = PaintSettings::default();
    s.fg.color = [1., 1., 1.];
    s.bg.color = [0., 0., 0.];
    let mut lin = stroke(&e, image, PaintTool::LinearGradient, s, None);
    let (a, b) = (DVec2::new(0.25, 0.5), DVec2::new(0.75, 0.5));
    lin.gradient(a, b, false);
    let p = lin.pixels();
    assert!(p.get(px_at(-0.9, 0.))[0] > 0.99, "FG before the start");
    assert!(
        p.get(px_at(0.9, 0.9))[0] < 0.01,
        "BG past the end, anywhere"
    );
    assert!((p.get(px_at(0., -0.7))[0] - 0.5).abs() < 0.05, "half way");
    let mut rad = stroke(&e, image, PaintTool::RadialGradient, s, None);
    rad.gradient(DVec2::new(0.5, 0.5), DVec2::new(0.75, 0.5), true);
    let p = rad.pixels();
    assert!(p.get(px_at(0., 0.))[0] > 0.95, "FG at the press point");
    assert!(p.get(px_at(0.9, 0.9))[0] < 0.01);
    assert_eq!(rad.label(), "Radial Gradient");
}

#[test]
fn lasso_rectangle_limits_paint_with_use_falloff() {
    let (e, image) = plane();
    let lasso = Lasso {
        style: LassoStyle::Rectangle,
        points: vec![screen(-0.3, 0.3), screen(0.3, -0.3)],
        soft_border: 10.,
        ..Lasso::default()
    };
    let mut s = sized(150.);
    s.use_falloff = true;
    let mut st = stroke(&e, image, PaintTool::Paintbrush, s, Some(&lasso));
    st.dab(screen(0., 0.), 150., None, 1.);
    let p = st.pixels();
    assert!(red(p, 0., 0.) > 0.99, "inside");
    assert_eq!(red(p, 0.7, 0.), 0., "outside, past the soft border");
    // Just outside the rectangle the paint fades.
    let edge = red(p, 0.33, 0.);
    assert!(edge > 0. && edge < 1., "{edge}");
    // Use Falloff off: the lasso doesn't limit.
    let mut free = stroke(&e, image, PaintTool::Paintbrush, sized(150.), Some(&lasso));
    free.dab(screen(0., 0.), 150., None, 1.);
    assert!(red(free.pixels(), 0.7, 0.) > 0.);
}
