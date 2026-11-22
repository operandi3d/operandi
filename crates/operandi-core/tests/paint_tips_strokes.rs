//! Painting stage 4's tips and nozzle on strokes (PNT-130..141), acceptance
//! 20–23, on the flat UV-matched plane; brush presets round trip (24).
//! Painting builds only.
#![cfg(feature = "painting")]
use glam::{DVec2, DVec3};
use operandi_core::paint::*;
use operandi_core::paint_stroke::*;
use operandi_core::paint_tips::*;
use operandi_core::*;
use std::sync::Arc;

const SIDE: u32 = 128;

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

fn screen(x: f64, y: f64) -> DVec2 {
    view().project(DVec3::new(x, y, 0.)).unwrap().0
}

fn stroke(
    e: &Editor,
    image: u64,
    brush: &Brush,
    extra_images: Option<Arc<operandi_paint::PaintImage>>,
) -> PaintStroke {
    let s = PaintSettings {
        tip: brush.tip(),
        ..PaintSettings::default()
    };
    let mut st = PaintStroke::begin(StrokeSetup {
        scene: &e.scene,
        item: e.scene.selection.item.unwrap(),
        image,
        pixels: e.scene.paint.buffers[&image].clone(),
        uv_map: None,
        view: view(),
        tool: PaintTool::Paintbrush,
        settings: s,
        symmetry: Symmetry::default(),
        falloff: None,
        lasso: None,
    })
    .unwrap();
    st.extra = brush.extra(|_| extra_images.clone(), [1., 1., 1.]).unwrap();
    st
}

/// The painted (red) values inside the plane's square.
fn values(p: &operandi_paint::PaintImage) -> Vec<f32> {
    (0..p.pixel_count()).map(|i| p.get(i)[0]).collect()
}

fn std_dev(v: &[f32]) -> f32 {
    let lit: Vec<f32> = v.iter().copied().filter(|&x| x > 0.01).collect();
    let mean = lit.iter().sum::<f32>() / lit.len().max(1) as f32;
    (lit.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / lit.len().max(1) as f32).sqrt()
}

#[test]
fn procedural_tip_is_patchy_cellular_differs_and_invert_flips() {
    let (e, image) = plane();
    let mut b = Brush {
        choice: TipChoice::Procedural,
        ..Brush::default()
    };
    b.procedural_tip.size = 150.;
    let mut noise = stroke(&e, image, &b, None);
    noise.dab(screen(0., 0.), 150., None, 1.);
    let n = values(noise.pixels());
    assert!(std_dev(&n) > 0.05, "patchy: {}", std_dev(&n));
    b.procedural.pattern = PatternType::Cellular;
    let mut cell = stroke(&e, image, &b, None);
    cell.dab(screen(0., 0.), 150., None, 1.);
    assert_ne!(values(cell.pixels()), n, "Cellular is another pattern");
    b.procedural.pattern = PatternType::Noise;
    b.procedural.invert = true;
    let mut inv = stroke(&e, image, &b, None);
    inv.dab(screen(0., 0.), 150., None, 1.);
    let i = values(inv.pixels());
    // Where Noise is strongest Invert is weak, and the other way round.
    let (hi, _) = n.iter().enumerate().fold(
        (0, 0f32),
        |(bi, bv), (k, &v)| if v > bv { (k, v) } else { (bi, bv) },
    );
    assert!(i[hi] < n[hi]);
}

/// A picture whose left half is white, right half black.
fn halves() -> Arc<operandi_paint::PaintImage> {
    let mut px = operandi_paint::PaintImage::new(operandi_paint::Layout {
        width: 32,
        height: 32,
        format: operandi_paint::Format::Rgba,
        depth: operandi_paint::Depth::U8,
    })
    .unwrap();
    for y in 0..32 {
        for x in 0..32 {
            let v = if x < 16 { 1. } else { 0. };
            px.set(y * 32 + x, [v, v, v, 1.]);
        }
    }
    Arc::new(px)
}

/// Switches between paint and none along y = 0 across the plane.
fn transitions(p: &operandi_paint::PaintImage) -> usize {
    let row = SIDE / 2;
    let lit: Vec<bool> = (8..SIDE - 8)
        .map(|x| p.get((row * SIDE + x) as usize)[0] > 0.5)
        .collect();
    lit.windows(2).filter(|w| w[0] != w[1]).count()
}

#[test]
fn texture_roller_tiles_along_the_stroke_and_stamp_places_one_copy() {
    let (e, image) = plane();
    let mut b = Brush {
        choice: TipChoice::Image,
        ..Brush::default()
    };
    b.image.image = Some(1);
    b.image.use_as_mask = true;
    b.image_tip.size = 20.;
    let line = |s: &mut PaintStroke| {
        for k in 0..120 {
            s.dab(screen(-0.8 + f64::from(k) * 1.6 / 120., 0.), 20., None, 1.);
        }
    };
    let mut normal = stroke(&e, image, &b, Some(halves()));
    line(&mut normal);
    b.image.mode = ImageMode::Roller;
    let mut roller = stroke(&e, image, &b, Some(halves()));
    line(&mut roller);
    assert!(
        transitions(roller.pixels()) >= 4,
        "the picture repeats along the stroke: {}",
        transitions(roller.pixels())
    );
    assert!(transitions(roller.pixels()) > transitions(normal.pixels()));
    // Stamp: one copy at the dragged size and angle.
    b.choice = TipChoice::Stamp;
    b.stamp = ImageTip {
        image: Some(1),
        use_as_mask: true,
        ..b.stamp
    };
    let mut flat = stroke(&e, image, &b, Some(halves()));
    flat.dab_turned(screen(0., 0.), 60., None, 1., 0.);
    let mut turned = stroke(&e, image, &b, Some(halves()));
    turned.dab_turned(screen(0., 0.), 60., None, 1., std::f64::consts::FRAC_PI_2);
    let lit = |p: &operandi_paint::PaintImage, x: f64, y: f64| {
        let (col, row) = (
            ((x + 1.) / 2. * f64::from(SIDE)) as u32,
            ((1. - (y + 1.) / 2.) * f64::from(SIDE)) as u32,
        );
        p.get((row * SIDE + col) as usize)[0] > 0.5
    };
    // Unturned: the white half is on the left.
    assert!(lit(flat.pixels(), -0.15, 0.) && !lit(flat.pixels(), 0.15, 0.));
    // Turned a quarter: the white half is above or below instead.
    assert_ne!(
        lit(turned.pixels(), 0., 0.15),
        lit(turned.pixels(), 0., -0.15)
    );
    // Nothing outside the stamp's size.
    assert!(!lit(flat.pixels(), -0.7, 0.));
}

/// Width and height of the painted area, in pixels.
fn extent(p: &operandi_paint::PaintImage) -> (u32, u32) {
    let (mut x0, mut y0, mut x1, mut y1) = (SIDE, SIDE, 0, 0);
    for i in 0..p.pixel_count() as u32 {
        if p.get(i as usize)[0] > 0.5 {
            let (x, y) = (i % SIDE, i / SIDE);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    (x1.saturating_sub(x0), y1.saturating_sub(y0))
}

#[test]
fn text_tip_stamps_text_at_the_dragged_size_and_angle() {
    let (e, image) = plane();
    let b = Brush {
        choice: TipChoice::Text,
        ..Brush::default()
    };
    let mut flat = stroke(&e, image, &b, None);
    flat.dab_turned(screen(0., 0.), 100., None, 1., 0.);
    let (w, h) = extent(flat.pixels());
    assert!(w > h * 2, "TEXT reads across: {w} x {h}");
    let mut turned = stroke(&e, image, &b, None);
    turned.dab_turned(screen(0., 0.), 100., None, 1., std::f64::consts::FRAC_PI_2);
    let (tw, th) = extent(turned.pixels());
    assert!(th > tw * 2, "turned a quarter: {tw} x {th}");
    let mut small = stroke(&e, image, &b, None);
    small.dab_turned(screen(0., 0.), 40., None, 1., 0.);
    assert!(
        extent(small.pixels()).0 < w,
        "smaller when dragged less far"
    );
}

#[test]
fn position_jitter_scatters_dabs_off_the_path() {
    let (e, image) = plane();
    let b = Brush::default();
    let run = |nozzle: Nozzle| {
        let mut s = stroke(&e, image, &b, None);
        s.settings.tip.size = 8.;
        let mut tip = s.settings.tip;
        tip.kind = operandi_core::brush_engine::TipKind::Hard;
        s.settings.tip = tip;
        for k in 0..80 {
            let m = nozzle.dab(42, k, 1., 0., 0.);
            let c = screen(-0.7 + f64::from(k) * 1.4 / 80., 0.) + m.offset * 8.;
            s.dab(c, 8. * m.size, None, m.strength);
        }
        extent(s.pixels()).1
    };
    let plain = run(Nozzle::default());
    let jittered = run(Nozzle {
        jitter: true,
        jitter_position: 0.5,
        ..Nozzle::default()
    });
    assert!(
        jittered > plain,
        "scattered sideways: {plain} -> {jittered}"
    );
}

#[test]
fn brush_presets_bring_every_value_back() {
    let mut b = Brush {
        choice: TipChoice::Procedural,
        nozzle_on: true,
        ..Brush::default()
    };
    b.procedural.pattern = PatternType::Dots;
    b.procedural.scale = [2., 0.5, 1.5];
    b.nozzle.jitter = true;
    b.nozzle.jitter_position = 0.7;
    b.text.text = "Hello".into();
    b.basic.size = 77.;
    let dir = std::env::temp_dir().join(format!("operandi-brush-{}", std::process::id()));
    let path = dir.join("Patchy.opbrush");
    BrushPreset::new("Patchy", &b).save(&path).unwrap();
    let back = BrushPreset::load(&path).unwrap();
    assert_eq!(back.brush, b);
    assert_eq!(back.name, "Patchy");
    std::fs::write(dir.join("bad.opbrush"), "{}").unwrap();
    assert!(BrushPreset::load(&dir.join("bad.opbrush")).is_err());
}
