//! Painting stage 2's strokes (PNT-45..72), the acceptance list 6–13:
//! strokes laid with the core stroke on a test sphere or plane, pixels
//! read back. Only with `--features painting`.
#![cfg(feature = "painting")]
use glam::{DVec2, DVec3};
use operandi_core::paint::*;
use operandi_core::paint_stroke::*;
use operandi_core::*;
use std::sync::Arc;

const VIEW: DVec2 = DVec2::new(400., 400.);

/// An editor with a mesh and a 256 px colour texture on it, black.
fn painted(mesh: Option<Mesh>, tool: Tool) -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    match mesh {
        Some(m) => e.scene.active_mut().unwrap().mesh = m.into(),
        None => e
            .dispatch(Command::InsertUnit {
                tool,
                destination: UnitDestination::ExistingMesh,
            })
            .unwrap(),
    }
    e.dispatch(Command::Paint(PaintEdit::AddTexture {
        kind: TextureKind::Color,
        image: NewImage {
            resolution: 256,
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

fn sphere() -> (Editor, u64) {
    painted(None, Tool::Sphere)
}

fn stroke(
    e: &Editor,
    image: u64,
    view: PaintView,
    tool: PaintTool,
    settings: PaintSettings,
) -> PaintStroke {
    PaintStroke::begin(StrokeSetup {
        scene: &e.scene,
        item: e.scene.selection.item.unwrap(),
        image,
        pixels: e.scene.paint.buffers[&image].clone(),
        uv_map: None,
        view,
        tool,
        settings,
        symmetry: Symmetry::default(),
        falloff: None,
        lasso: None,
    })
    .unwrap()
}

fn front() -> PaintView {
    PaintView::looking(DVec3::new(0., 0., 4.), DVec3::ZERO, VIEW)
}

fn settings(size: f64) -> PaintSettings {
    let mut s = PaintSettings::default();
    s.tip.size = size;
    s
}

/// The brightest red value of the image.
fn max_red(px: &operandi_paint::PaintImage) -> f32 {
    (0..px.pixel_count())
        .map(|i| px.get(i)[0])
        .fold(0., f32::max)
}

fn painted_count(px: &operandi_paint::PaintImage) -> usize {
    (0..px.pixel_count())
        .filter(|&i| px.get(i)[0] > 0.01)
        .count()
}

/// A short horizontal stroke through the view's centre.
fn drag(s: &mut PaintStroke, radius: f64) {
    for k in 0..20 {
        s.dab(DVec2::new(180. + f64::from(k) * 2., 200.), radius, None, 1.);
    }
}

#[test]
fn airbrush_paints_white_and_one_undo_removes_it_exactly() {
    let (mut e, image) = sphere();
    let before = e.scene.paint.buffers[&image].clone();
    let mut s = stroke(&e, image, front(), PaintTool::Airbrush, settings(30.));
    assert!(s.reachable() > 0);
    drag(&mut s, 30.);
    assert!(max_red(s.pixels()) > 0.99, "white at the stroke's centre");
    assert!(
        s.take_dirty().is_some(),
        "changed pixels are reported for upload"
    );
    let steps = e.undo.len();
    e.dispatch(Command::Paint(PaintEdit::Stroke {
        image,
        via: None,
        pixels: Arc::new(s.pixels().clone()),
        label: s.label(),
    }))
    .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(e.undo.last().unwrap().label, "Airbrush Stroke");
    assert!(e.image_changed(image));
    e.dispatch(Command::Undo).unwrap();
    assert!(Arc::ptr_eq(&e.scene.paint.buffers[&image], &before));
}

#[test]
fn the_back_of_the_sphere_is_untouched() {
    let (e, image) = sphere();
    let mut s = stroke(&e, image, front(), PaintTool::Airbrush, settings(150.));
    drag(&mut s, 150.);
    // Paint the whole visible half, then check every texel facing away.
    assert!(painted_count(s.pixels()) > 100);
    let base = s.base().clone();
    let item = e.scene.active().unwrap();
    let mesh = &item.mesh;
    // Texels whose UV-mapped point is on the back (z < -0.1) kept their
    // colour: find them through each polygon's centre UV.
    let map = mesh.uv_maps.values().next().unwrap();
    let mut checked = 0;
    for (f, face) in mesh.faces.iter().enumerate() {
        let centre: DVec3 = face
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .sum::<DVec3>()
            / face.vertices.len() as f64;
        if centre.z > -0.3 {
            continue;
        }
        let uvs: Vec<DVec2> = map[f].iter().flatten().copied().collect();
        if uvs.len() != face.vertices.len() {
            continue;
        }
        let uv = uvs.iter().sum::<DVec2>() / uvs.len() as f64;
        let (x, y) = ((uv.x * 256.) as u32, ((1. - uv.y) * 256.) as u32);
        let i = (y.min(255) * 256 + x.min(255)) as usize;
        assert_eq!(s.pixels().get(i), base.get(i), "back face {f}");
        checked += 1;
    }
    assert!(checked > 10);
}

#[test]
fn a_smaller_view_gives_a_wider_band_on_the_model() {
    let (e, image) = sphere();
    let near = PaintView::looking(DVec3::new(0., 0., 3.), DVec3::ZERO, VIEW);
    let far = PaintView::looking(DVec3::new(0., 0., 8.), DVec3::ZERO, VIEW);
    let mut a = stroke(&e, image, near, PaintTool::Paintbrush, settings(20.));
    a.dab(DVec2::new(200., 200.), 20., None, 1.);
    let mut b = stroke(&e, image, far, PaintTool::Paintbrush, settings(20.));
    b.dab(DVec2::new(200., 200.), 20., None, 1.);
    // The same screen size covers more of the model from further away.
    assert!(painted_count(b.pixels()) > painted_count(a.pixels()) * 3);
}

#[test]
fn opacity_is_a_ceiling_for_one_stroke() {
    let (e, image) = sphere();
    let mut s50 = settings(30.);
    s50.opacity = 0.5;
    let mut s = stroke(&e, image, front(), PaintTool::Airbrush, s50);
    for _ in 0..30 {
        drag(&mut s, 30.);
    }
    let first = max_red(s.pixels());
    assert!((first - 0.5).abs() < 0.01, "{first}");
    // A second stroke over the first goes further.
    let mut e2 = e.clone();
    e2.scene
        .paint
        .buffers
        .insert(image, Arc::new(s.pixels().clone()));
    let mut again = stroke(&e2, image, front(), PaintTool::Airbrush, s50);
    drag(&mut again, 30.);
    assert!(max_red(again.pixels()) > 0.7);
}

#[test]
fn paintbrush_overlaps_dont_darken_and_low_density_airbrush_builds_up() {
    let (e, image) = sphere();
    let mut soft = settings(30.);
    soft.density = 0.2;
    let mut once = stroke(&e, image, front(), PaintTool::Paintbrush, soft);
    once.dab(DVec2::new(200., 200.), 30., None, 1.);
    let mut twice = stroke(&e, image, front(), PaintTool::Paintbrush, soft);
    twice.dab(DVec2::new(200., 200.), 30., None, 1.);
    twice.dab(DVec2::new(200., 200.), 30., None, 1.);
    assert_eq!(
        once.pixels(),
        twice.pixels(),
        "Paintbrush: no build-up within a stroke"
    );
    let mut air = stroke(&e, image, front(), PaintTool::Airbrush, soft);
    air.dab(DVec2::new(200., 200.), 30., None, 1.);
    let one = max_red(air.pixels());
    for _ in 0..10 {
        air.dab(DVec2::new(200., 200.), 30., None, 1.);
    }
    let many = max_red(air.pixels());
    assert!(
        one < 0.25 && many > 0.8,
        "Airbrush builds up: {one} -> {many}"
    );
}

#[test]
fn eraser_takes_alpha_or_paints_the_bg_colour() {
    let (e, image) = sphere();
    // RGBA (the texture's format): alpha goes.
    let mut s = stroke(&e, image, front(), PaintTool::Eraser, settings(30.));
    s.dab(DVec2::new(200., 200.), 30., None, 1.);
    let min_alpha = (0..s.pixels().pixel_count())
        .map(|i| s.pixels().get(i)[3])
        .fold(1., f32::min);
    assert!(min_alpha < 0.01);
    // RGB: the BG colour (here green).
    let mut rgb = e.clone();
    let mut px = operandi_paint::PaintImage::new(operandi_paint::Layout {
        width: 256,
        height: 256,
        format: operandi_paint::Format::Rgb,
        depth: operandi_paint::Depth::U8,
    })
    .unwrap();
    px.fill([1., 0., 0., 1.]);
    rgb.scene.paint.buffers.insert(image, Arc::new(px));
    let mut set = settings(30.);
    set.bg.color = [0., 1., 0.];
    let mut s = stroke(&rgb, image, front(), PaintTool::Eraser, set);
    s.dab(DVec2::new(200., 200.), 30., None, 1.);
    let greenest = (0..s.pixels().pixel_count())
        .map(|i| s.pixels().get(i)[1])
        .fold(0., f32::max);
    assert!(greenest > 0.99);
}

#[test]
fn attenuation_steps_fade_the_stroke_out() {
    let (e, image) = sphere();
    let mut set = settings(10.);
    set.attenuation = 20;
    let mut s = stroke(&e, image, front(), PaintTool::Paintbrush, set);
    for k in 0..20 {
        s.dab(DVec2::new(120. + f64::from(k) * 4., 200.), 10., None, 1.);
    }
    let after_twenty = s.pixels().clone();
    // Dabs past the 20th lay nothing.
    for k in 0..10 {
        s.dab(DVec2::new(240. + f64::from(k) * 4., 230.), 10., None, 1.);
    }
    assert_eq!(s.pixels(), &after_twenty);
    // And the ones near the end were weaker than the first.
    let mut first = stroke(&e, image, front(), PaintTool::Paintbrush, set);
    first.dab(DVec2::new(200., 200.), 10., None, 1.);
    let mut late = stroke(&e, image, front(), PaintTool::Paintbrush, set);
    for _ in 0..18 {
        late.dab(DVec2::new(-500., -500.), 10., None, 1.);
    }
    late.dab(DVec2::new(200., 200.), 10., None, 1.);
    assert!(max_red(late.pixels()) < max_red(first.pixels()) * 0.2);
}

#[test]
fn every_blend_mode_matches_the_shader_tree_formula() {
    use operandi_paint::Blend;
    let pairs = [
        (PaintBlend::Normal, Some(Blend::Normal)),
        (PaintBlend::Add, Some(Blend::Add)),
        (PaintBlend::Subtract, Some(Blend::Subtract)),
        (PaintBlend::Difference, Some(Blend::Difference)),
        (PaintBlend::Multiply, Some(Blend::Multiply)),
        (PaintBlend::Screen, Some(Blend::Screen)),
        (PaintBlend::Overlay, Some(Blend::Overlay)),
        (PaintBlend::SoftLight, Some(Blend::SoftLight)),
        (PaintBlend::HardLight, Some(Blend::HardLight)),
        (PaintBlend::Darken, Some(Blend::Darken)),
        (PaintBlend::Lighten, Some(Blend::Lighten)),
        (PaintBlend::ColorDodge, Some(Blend::ColorDodge)),
        (PaintBlend::ColorBurn, Some(Blend::ColorBurn)),
        (PaintBlend::Erase, None),
    ];
    assert_eq!(pairs.len(), PaintBlend::ALL.len());
    // Red paint over a blue-green gradient, opaque, at 60 %.
    for (mode, blend) in pairs {
        let mut s = PaintSettings::default();
        s.fg.color = [1., 0., 0.];
        s.blend = mode;
        for g in 0..=10 {
            let below = [0., g as f32 / 10., 1. - g as f32 / 10., 1.];
            let out = mix(PaintTool::Paintbrush, &s, below, 0.6, true);
            match blend {
                Some(b) => {
                    for k in 0..3 {
                        let f = operandi_paint::blend::full(b, below[k], [1., 0., 0.][k]);
                        let want = below[k] + (f - below[k]) * 0.6;
                        assert!((out[k] - want).abs() < 1e-5, "{mode:?} channel {k}");
                    }
                }
                None => assert!((out[3] - 0.4).abs() < 1e-6, "Erase takes alpha"),
            }
        }
    }
}

/// Two quads side by side sharing an edge in 3D, each its own UV island.
fn two_islands() -> Mesh {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(-1., -1., 0.),
            DVec3::new(0., -1., 0.),
            DVec3::new(0., 1., 0.),
            DVec3::new(-1., 1., 0.),
            DVec3::new(1., -1., 0.),
            DVec3::new(1., 1., 0.),
        ],
        ..Mesh::default()
    };
    let faces: Vec<Face> = [vec![0, 1, 2, 3], vec![1, 4, 5, 2]]
        .into_iter()
        .map(|vertices| Face {
            vertices,
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        })
        .collect();
    m.faces = faces.into();
    let island = |u0: f64| {
        vec![
            Some(DVec2::new(u0, 0.05)),
            Some(DVec2::new(u0 + 0.35, 0.05)),
            Some(DVec2::new(u0 + 0.35, 0.95)),
            Some(DVec2::new(u0, 0.95)),
        ]
    };
    m.uv_maps
        .insert("Texture".into(), vec![island(0.05), island(0.6)]);
    m
}

#[test]
fn a_stroke_across_a_uv_seam_paints_both_sides_alike() {
    let (e, image) = painted(Some(two_islands()), Tool::Box);
    let view = PaintView::looking(DVec3::new(0., 0., 3.), DVec3::ZERO, VIEW);
    let mut s = stroke(&e, image, view, PaintTool::Paintbrush, settings(40.));
    // A dab right on the shared edge (x = 0 is the screen centre).
    s.dab(DVec2::new(200., 200.), 40., None, 1.);
    let px = s.pixels();
    let at = |u: f64, v: f64| {
        px.get(((((1. - v) * 256.) as usize).min(255)) * 256 + ((u * 256.) as usize).min(255))[0]
    };
    // The last column of the left island and the first of the right one,
    // both at the edge: painted, and alike.
    let left = at(0.398, 0.5);
    let right = at(0.602, 0.5);
    assert!(
        left > 0.9 && right > 0.9,
        "both sides painted: {left} {right}"
    );
    assert!((left - right).abs() < 0.05);
    // Outside the islands nothing is painted.
    assert_eq!(at(0.5, 0.5), 0.);
}
