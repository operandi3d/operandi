//! HT-5 Show Weightmaps and HT-6 Show UV Distortion in the 3D view
//! (viewport-header-toggles spec v1.1, HT-12..HT-19). The geometry carries
//! both tints (gpu_scene.rs `Tint`); the pictures come from a real
//! (headless) graphics device and skip without one. Pixels are compared
//! with the same view drawn with both toggles off, so the axis gizmo and
//! other overlays don't count.
use super::*;
use crate::gpu_scene::{Tint, Tints};
use glam::DVec2;

const PICTURE: Vec2 = vec2(900., 600.);

/// The 2-unit box (box_app), `W` its selected weight map with every point
/// at `w`, "Texture" its only UV map (each polygon the unit square), and
/// the view at `view`.
fn cube(view: View, w: f64) -> Operandi {
    let mut a = super::tests::box_app();
    a.run(Command::NewWeightMap {
        name: "W".into(),
        value: Some(w),
    });
    assert_eq!(a.editor.selected_weight_map.as_deref(), Some("W"));
    let mesh = &mut *a.editor.scene.active_mut().unwrap().mesh;
    let square = [
        DVec2::new(0., 0.),
        DVec2::new(1., 0.),
        DVec2::new(1., 1.),
        DVec2::new(0., 1.),
    ];
    let rows: Vec<Vec<Option<DVec2>>> = mesh
        .faces
        .iter()
        .map(|f| (0..f.vertices.len()).map(|c| Some(square[c % 4])).collect())
        .collect();
    let maps = &mut *mesh.uv_maps;
    maps.clear();
    maps.insert("Texture".into(), rows);
    a.camera.view = view;
    a.frame(false);
    if view == View::Perspective {
        a.camera.yaw += 0.5;
        a.camera.pitch -= 0.4;
    }
    a
}

/// Set point weights of `W` (one weight-paint stroke, an undo step).
fn paint(a: &mut Operandi, points: impl IntoIterator<Item = (u32, f64)>) {
    let id = a.editor.scene.active().unwrap().id;
    let edit = operandi_core::sculpt::WeightEdit {
        label: "Weight".into(),
        items: vec![operandi_core::sculpt::WeightItemEdit {
            item: id,
            maps: vec![(
                "W".into(),
                points.into_iter().map(|(v, w)| (v, Some(w))).collect(),
            )],
        }],
    };
    a.run(Command::WeightStroke(Box::new(edit)));
}

/// The box's points with y above 0 (its top four).
fn top_points(a: &Operandi) -> Vec<u32> {
    let mesh = &a.editor.scene.active().unwrap().mesh;
    (0..mesh.vertices.len() as u32)
        .filter(|&v| mesh.vertices[v as usize].y > 0.)
        .collect()
}

/// Scale the UVs of the polygon facing the camera most by `k` about their
/// middle; its index.
fn scale_uvs(a: &mut Operandi, k: f64) -> usize {
    let (_, _, _, forward) = a.camera.basis();
    let mesh = &mut *a.editor.scene.active_mut().unwrap().mesh;
    let f = (0..mesh.faces.len())
        .max_by(|&x, &y| {
            let d = |f: usize| mesh.normal(f).dot(-forward);
            d(x).total_cmp(&d(y))
        })
        .unwrap();
    let rows = mesh.uv_maps.get_mut("Texture").unwrap();
    for uv in rows[f].iter_mut().flatten() {
        *uv = DVec2::splat(0.5) + (*uv - DVec2::splat(0.5)) * k;
    }
    f
}

/// A view's picture as RGBA rows.
#[derive(PartialEq)]
struct Picture {
    size: [u32; 2],
    rgba: Vec<u8>,
}
impl Picture {
    fn pixel(&self, i: usize) -> [u8; 3] {
        let p = &self.rgba[4 * i..4 * i + 3];
        [p[0], p[1], p[2]]
    }
    /// The pixels (index, colour) passing `test` that differ from `plain`
    /// (the same view untinted).
    fn tinted(&self, plain: &Picture, test: fn([u8; 3]) -> bool) -> Vec<(usize, [u8; 3])> {
        (0..self.rgba.len() / 4)
            .map(|i| (i, self.pixel(i)))
            .filter(|&(i, p)| test(p) && p != plain.pixel(i))
            .collect()
    }
    fn count(&self, plain: &Picture, test: fn([u8; 3]) -> bool) -> usize {
        self.tinted(plain, test).len()
    }
    /// The mean row of the tinted pixels passing `test`.
    fn mean_row(&self, plain: &Picture, test: fn([u8; 3]) -> bool) -> f64 {
        let rows = self.tinted(plain, test);
        let w = self.size[0] as usize;
        rows.iter().map(|(i, _)| (i / w) as f64).sum::<f64>() / rows.len().max(1) as f64
    }
}
fn red(p: [u8; 3]) -> bool {
    p[0] as i32 > p[1] as i32 + 50 && p[0] as i32 > p[2] as i32 + 50
}
fn blue(p: [u8; 3]) -> bool {
    p[2] as i32 > p[0] as i32 + 50 && p[2] as i32 > p[1] as i32 + 20
}
/// VMT-30's grey-green (weight 0) under the light: green above red and
/// blue.
fn grey_green(p: [u8; 3]) -> bool {
    p[1] > p[0] && p[1] > p[2] && p[1] > 40
}

/// `a` on a headless graphics device; `None` without one.
fn on_gpu(mut a: Operandi) -> Option<Operandi> {
    let state = super::performance_tests::headless_gpu()?;
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    Some(a)
}
/// The first view's picture after a few frames.
fn draw(a: &mut Operandi) -> Picture {
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..3 {
        super::tests::draw(a, &ctx, PICTURE, vec![]);
    }
    let (_, size, rgba) = a
        .viewport_gpu
        .as_ref()
        .unwrap()
        .read_views()
        .into_iter()
        .next()
        .expect("a view picture");
    Picture { size, rgba }
}
/// The picture as set up, then the same with both toggles off (put back
/// afterwards).
fn pictures(a: &mut Operandi) -> (Picture, Picture) {
    let on = draw(a);
    let d = a.display.settings.drawing;
    a.display.settings.drawing.weights = false;
    a.display.settings.drawing.distortion = false;
    let plain = draw(a);
    a.display.settings.drawing.weights = d.weights;
    a.display.settings.drawing.distortion = d.distortion;
    (on, plain)
}

/// The tints a plain build of the active item carries, with what `a`'s
/// views show.
fn tints(a: &Operandi) -> Vec<([f32; 3], Tint)> {
    crate::gpu_scene::set_tints(a.view_tints());
    let item = a.editor.scene.active().unwrap();
    let out = crate::gpu_scene::fill_for_tests(item)
        .into_iter()
        .map(|v| (v.position, v.tint))
        .collect();
    crate::gpu_scene::set_tints(Tints::default());
    out
}

#[test]
fn ht12_weights_tint_top_red_fading_to_grey_green_with_shading() {
    let mut a = cube(View::Front, 0.);
    let top = top_points(&a);
    assert_eq!(top.len(), 4);
    paint(&mut a, top.iter().map(|&v| (v, 1.)));
    // Off by default: nothing baked.
    assert!(tints(&a).iter().all(|(_, t)| t.weight == [0; 4]));
    a.execute("view3d.showweights true");
    assert!(a.display.settings.drawing.weights, "{}", a.status);
    // Each corner carries its point's weight (per point, blended across
    // the polygon by the graphics card).
    for (p, t) in tints(&a) {
        let want = if p[1] > 0. { 1. } else { 0. };
        assert_eq!(t.weight, Tint::weight(want), "{p:?}");
    }
    let Some(mut a) = on_gpu(a) else {
        println!("no graphics device; skipped");
        return;
    };
    let (on, plain) = pictures(&mut a);
    assert!(on != plain, "the tint shows");
    assert!(on.count(&plain, red) > 1000 && on.count(&plain, grey_green) > 1000);
    // The top is red, the bottom grey-green.
    assert!(on.mean_row(&plain, red) < on.mean_row(&plain, grey_green));
    // Off is the normal shading.
    a.execute("view3d.showweights false");
    assert!(draw(&mut a) == plain);
    // Shading stays visible: all at 1 in perspective, the faces turned
    // to the light differently are differently bright reds.
    let mut a = on_gpu(cube(View::Perspective, 1.)).unwrap();
    a.execute("view3d.showweights true");
    let (on, plain) = pictures(&mut a);
    let reds: Vec<u8> = on.tinted(&plain, red).iter().map(|(_, p)| p[0]).collect();
    assert!(reds.len() > 1000);
    let (lo, hi) = (reds.iter().min().unwrap(), reds.iter().max().unwrap());
    assert!(hi - lo > 25, "one flat red: {lo}..{hi}");
}

#[test]
fn ht13_a_point_at_minus_one_turns_its_corner_blue() {
    let mut a = cube(View::Perspective, 0.);
    let corner = top_points(&a)[0];
    paint(&mut a, [(corner, -1.)]);
    a.execute("view3d.showweights true");
    let at_corner: Vec<Tint> = {
        let p = a.editor.scene.active().unwrap().mesh.vertices[corner as usize].as_vec3();
        tints(&a)
            .into_iter()
            .filter(|(q, _)| *q == p.to_array())
            .map(|(_, t)| t)
            .collect()
    };
    assert!(!at_corner.is_empty());
    assert!(at_corner.iter().all(|t| t.weight == Tint::weight(-1.)));
    assert_eq!(Tint::weight(-1.)[0], 0);
    assert_eq!(Tint::weight(0.)[0], 127);
    assert_eq!(Tint::weight(7.)[0], 254, "clamped");
    let Some(mut a) = on_gpu(a) else {
        return;
    };
    let (on, plain) = pictures(&mut a);
    assert!(on.count(&plain, blue) > 50, "the corner shows blue");
    assert_eq!(on.count(&plain, red), 0);
}

#[test]
fn ht14_no_weight_map_selected_means_no_tint() {
    let mut a = cube(View::Perspective, 1.);
    a.run(Command::SelectWeightMap(None));
    a.execute("view3d.showweights true");
    // The button still toggles.
    assert!(a.line_state("view3d.showweights").unwrap().on);
    assert!(a.view_tints().weights.is_none());
    assert!(tints(&a).iter().all(|(_, t)| t.weight == [0; 4]));
    let Some(mut a) = on_gpu(a) else {
        return;
    };
    let (on, plain) = pictures(&mut a);
    assert!(on == plain);
}

#[test]
fn ht15_painting_weights_moves_the_tint_live() {
    let Some(mut a) = on_gpu(cube(View::Perspective, 0.)) else {
        return;
    };
    a.execute("view3d.showweights true");
    let (before, plain) = pictures(&mut a);
    assert_eq!(before.count(&plain, red), 0);
    assert!(before.count(&plain, grey_green) > 1000);
    let n = a.editor.scene.active().unwrap().mesh.vertices.len() as u32;
    paint(&mut a, (0..n).map(|v| (v, 1.)));
    let after = draw(&mut a);
    assert!(after.count(&plain, red) > 1000);
    // Undo takes it back.
    a.run(Command::Undo);
    assert!(draw(&mut a) == before);
}

#[test]
fn ht16_uv_distortion_matches_the_uv_view() {
    for (k, large) in [(2., true), (0.5, false)] {
        let mut a = cube(View::Perspective, 0.);
        let face = scale_uvs(&mut a, k);
        a.execute("view3d.showDistortion true");
        assert!(a.display.settings.drawing.distortion, "{}", a.status);
        // Exactly the UV view's colours, polygon by polygon.
        let mesh = a.editor.scene.active().unwrap().mesh.clone();
        let want = crate::uv_view::polygon_distortion(&mesh, "Texture");
        let corners: usize = mesh
            .faces
            .iter()
            .map(|f| f.vertices.len() - 2)
            .sum::<usize>()
            * 3;
        let got = tints(&a);
        assert_eq!(got.len(), corners);
        let mut at = 0;
        for (f, polygon) in mesh.faces.iter().enumerate() {
            let n = (polygon.vertices.len() - 2) * 3;
            let c = Tint::colour(want[f].unwrap());
            assert!(
                got[at..at + n].iter().all(|(_, t)| t.distortion == c),
                "{f}"
            );
            at += n;
        }
        // The scaled polygon is stretched (blue, LARGE) or squashed (red,
        // SMALL), at the UV view's opacity.
        let c = want[face].unwrap().to_srgba_unmultiplied();
        if large {
            assert!(c[2] > c[0] + 80, "{c:?}");
        } else {
            assert!(c[0] > c[2] + 80, "{c:?}");
        }
        assert_eq!(c[3], 150);
        let Some(mut a) = on_gpu(a) else {
            continue;
        };
        let (on, plain) = pictures(&mut a);
        if large {
            assert!(on.count(&plain, blue) > 1000, "stretched polygon blue");
        } else {
            assert!(on.count(&plain, red) > 1000, "squashed polygon red");
        }
    }
}

#[test]
fn ht17_polygons_without_uvs_are_untinted() {
    let mut a = cube(View::Perspective, 0.);
    scale_uvs(&mut a, 2.);
    a.editor.scene.active_mut().unwrap().mesh.uv_maps.clear();
    a.execute("view3d.showDistortion true");
    assert!(tints(&a).iter().all(|(_, t)| t.distortion == [0; 4]));
    let Some(mut a) = on_gpu(a) else {
        return;
    };
    let (on, plain) = pictures(&mut a);
    assert!(on == plain);
}

#[test]
fn ht19_weights_win_over_distortion() {
    let mut a = cube(View::Perspective, 1.);
    scale_uvs(&mut a, 2.);
    a.execute("view3d.showweights true");
    a.execute("view3d.showDistortion true");
    let t = tints(&a);
    assert!(
        t.iter()
            .all(|(_, t)| t.weight[3] == 255 && t.distortion[3] > 0)
    );
    let Some(mut a) = on_gpu(a) else {
        return;
    };
    let (both, plain) = pictures(&mut a);
    assert!(both.count(&plain, red) > 1000);
    assert_eq!(
        both.count(&plain, blue),
        0,
        "the stretched polygon shows its weight"
    );
    // Distortion alone shows it blue.
    a.execute("view3d.showweights false");
    assert!(draw(&mut a).count(&plain, blue) > 1000);
}

#[test]
fn ht18_wireframe_draws_no_tint() {
    let Some(mut a) = on_gpu(cube(View::Perspective, 1.)) else {
        return;
    };
    a.execute("view3d.showweights true");
    a.execute("view3d.showDistortion true");
    a.display.settings.active.style = view_display::Style::Wireframe;
    let (on, plain) = pictures(&mut a);
    assert!(on == plain);
}

/// HT-Q5: `bgmesh` is accepted on `view3d.showDistortion` and, for now,
/// treated as 0 (the view's one setting).
#[test]
fn ht_q5_show_distortion_accepts_bgmesh() {
    let mut a = cube(View::Perspective, 0.);
    a.execute("view3d.showDistortion true bgmesh:1");
    assert!(a.display.settings.drawing.distortion, "{}", a.status);
    a.execute("view3d.showDistortion enabled:false bgmesh:0");
    assert!(!a.display.settings.drawing.distortion, "{}", a.status);
}
