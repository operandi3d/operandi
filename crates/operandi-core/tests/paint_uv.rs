//! Painting stage 6: strokes in the UV view (PNT-215), image space with no
//! projection. Painting builds only.
#![cfg(feature = "painting")]
use glam::DVec2;
use operandi_core::paint_stroke::*;
use operandi_paint::{Depth, Format, Layout, PaintImage};
use std::sync::Arc;

fn black(side: u32) -> Arc<PaintImage> {
    Arc::new(
        PaintImage::filled(
            Layout {
                width: side,
                height: side,
                format: Format::Rgba,
                depth: Depth::U8,
            },
            Some([0., 0., 0.]),
        )
        .unwrap(),
    )
}

/// A 512-point UV view of the 0..1 tile, its centre at (300, 300).
fn view() -> UvScreen {
    UvScreen {
        screen_center: DVec2::new(300., 300.),
        uv_center: DVec2::splat(0.5),
        zoom: 512.,
        size: DVec2::new(600., 600.),
    }
}

fn screen(u: f64, v: f64) -> DVec2 {
    DVec2::new(300. + (u - 0.5) * 512., 300. - (v - 0.5) * 512.)
}

fn stroke(tool: PaintTool, size: f64) -> PaintStroke {
    let mut s = PaintSettings::default();
    s.tip.size = size;
    PaintStroke::begin_uv(UvStrokeSetup {
        image: 1,
        pixels: black(256),
        uv: view(),
        tool,
        settings: s,
    })
    .unwrap()
}

fn red_at(st: &PaintStroke, u: f64, v: f64) -> f32 {
    let (x, y) = ((u * 256.) as usize, ((1. - v) * 256.) as usize);
    st.pixels().get(y * 256 + x)[0]
}

#[test]
fn a_dab_in_the_uv_view_paints_the_pixels_under_it_at_screen_size() {
    let mut st = stroke(PaintTool::Paintbrush, 20.);
    st.dab(screen(0.25, 0.75), 20., None, 1.);
    assert!(red_at(&st, 0.25, 0.75) > 0.9);
    // 20 screen points at 2 points per pixel: about 10 pixels' radius.
    assert!(red_at(&st, 0.25 + 3. / 256., 0.75) > 0.5);
    assert_eq!(red_at(&st, 0.25 + 14. / 256., 0.75), 0.);
    assert_eq!(red_at(&st, 0.75, 0.25), 0.);
    // Off the tile: nothing, and no panic.
    st.dab(DVec2::new(-500., -500.), 20., None, 1.);
    assert!(st.changed);
}

#[test]
fn fill_and_gradients_work_in_the_uv_view() {
    let mut st = stroke(PaintTool::Fill, 10.);
    let seed = st.pixel_at(screen(0.5, 0.5)).unwrap();
    st.fill(seed);
    assert!(red_at(&st, 0.1, 0.9) > 0.99, "the whole black image fills");
    let mut g = stroke(PaintTool::LinearGradient, 10.);
    let (a, b) = (
        g.uv_at(screen(0., 0.5)).unwrap(),
        g.uv_at(screen(1., 0.5)).unwrap(),
    );
    g.gradient(a, b, false);
    assert!(red_at(&g, 0.02, 0.5) > 0.9, "FG (white) at the start");
    assert!(red_at(&g, 0.98, 0.5) < 0.1, "BG (black) at the end");
}

#[test]
fn blur_runs_on_uv_pixels() {
    // Left half white, right half black: a hard edge at u = 0.5.
    let mut px = (*black(256)).clone();
    for y in 0..256 {
        for x in 0..128 {
            px.set(y * 256 + x, [1., 1., 1., 1.]);
        }
    }
    let mut b = PaintStroke::begin_uv(UvStrokeSetup {
        image: 1,
        pixels: Arc::new(px),
        uv: view(),
        tool: PaintTool::Blur,
        settings: PaintSettings::default(),
    })
    .unwrap();
    let edge = 128.5 / 256.;
    assert_eq!(red_at(&b, edge, 0.5), 0.);
    b.dab(screen(0.5, 0.5), 20., None, 1.);
    assert!(red_at(&b, edge, 0.5) > 0.05, "blur softened the edge");
}

/// Timing probe for the 2048 px auto-created texture on a sphere
/// (`--release -- --ignored paint_uv::probe`).
#[test]
#[ignore]
fn probe_big_texture_stroke_cost() {
    use operandi_core::paint::*;
    use operandi_core::*;
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: UnitDestination::ExistingMesh,
    })
    .unwrap();
    e.dispatch(Command::Paint(PaintEdit::AddTexture {
        kind: TextureKind::Color,
        image: NewImage {
            resolution: 2048,
            ..NewImage::default()
        },
        selected: None,
        item: None,
    }))
    .unwrap();
    let image = e.paint.last_image.unwrap();
    let t = std::time::Instant::now();
    let mut st = PaintStroke::begin(StrokeSetup {
        scene: &e.scene,
        item: e.scene.selection.item.unwrap(),
        image,
        pixels: e.scene.paint.buffers[&image].clone(),
        uv_map: None,
        view: PaintView::looking(
            glam::DVec3::new(0., 0., 4.),
            glam::DVec3::ZERO,
            DVec2::new(1000., 800.),
        ),
        tool: PaintTool::Paintbrush,
        settings: PaintSettings::default(),
        symmetry: Symmetry::default(),
        falloff: None,
        lasso: None,
    })
    .unwrap();
    let begin = t.elapsed();
    let t = std::time::Instant::now();
    for k in 0..50 {
        st.dab(DVec2::new(400. + k as f64 * 4., 400.), 30., None, 1.);
    }
    let dabs = t.elapsed();
    let t = std::time::Instant::now();
    let px = std::sync::Arc::new(st.pixels().clone());
    let copy = t.elapsed();
    eprintln!(
        "begin {begin:?} 50 dabs {dabs:?} copy {copy:?} texels {} {}",
        st.reachable(),
        px.width
    );
}
