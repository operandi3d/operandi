//! Viewport shading (VS-1..VS-3, v1.1): the CPU side of every style,
//! rendered on a unit sphere and a cube in orthographic and perspective
//! views and sampled as screen bytes, plus checks that the shader carries
//! the same numbers.
use super::*;

fn defaults() -> StyleColors {
    crate::theme::set_overrides(&Default::default());
    StyleColors::live()
}

const FRONT: ViewBasis = ViewBasis {
    right: Vec3::X,
    up: Vec3::Y,
    toward: Vec3::Z,
};
const TOP: ViewBasis = ViewBasis {
    right: Vec3::X,
    up: Vec3::NEG_Z,
    toward: Vec3::Y,
};
const RIGHT: ViewBasis = ViewBasis {
    right: Vec3::NEG_Z,
    up: Vec3::Y,
    toward: Vec3::X,
};
/// An orbited view: turned `yaw` round world Y and tilted `pitch` down.
fn orbit(yaw: f32, pitch: f32) -> ViewBasis {
    let toward = Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    );
    let right = Vec3::Y.cross(toward).normalize();
    ViewBasis {
        right,
        up: toward.cross(right),
        toward,
    }
}

/// The default grey fill (the theme's Solid colour).
fn grey() -> [f32; 3] {
    unit(crate::theme::viewport::SOLID)
}
fn bytes(c: [f32; 3]) -> [u8; 3] {
    c.map(|v| (v * 255. + 0.5) as u8)
}
fn colour(style: Style, n: Vec3, to_eye: Vec3, view: &ViewBasis) -> [u8; 3] {
    let c = grey();
    bytes(shade(style, c, c, n, to_eye, view, &defaults()))
}

/// A unit sphere in an orthographic view at screen point (x, y) in -1..1
/// (y up), or None off the sphere.
fn sphere_rgb(style: Style, view: ViewBasis, x: f32, y: f32) -> Option<[u8; 3]> {
    let z2 = 1. - x * x - y * y;
    if z2 <= 0. {
        return None;
    }
    let n = view.right * x + view.up * y + view.toward * z2.sqrt();
    Some(colour(style, n, view.toward, &view))
}
fn sphere(style: Style, view: ViewBasis, x: f32, y: f32) -> u8 {
    sphere_rgb(style, view, x, y).expect("on the sphere")[0]
}

/// A perspective camera 4 units from the origin along `view.toward`, 40°
/// across: the ray through screen point (x, y) in -1..1.
fn ray(view: &ViewBasis, x: f32, y: f32) -> (Vec3, Vec3) {
    let eye = view.toward * 4.;
    let half = 20f32.to_radians().tan();
    let d = (-view.toward + (view.right * x + view.up * y) * half).normalize();
    (eye, d)
}
/// The unit sphere seen in perspective: the colour at screen (x, y).
fn persp_sphere(style: Style, view: &ViewBasis, x: f32, y: f32) -> Option<[u8; 3]> {
    let (eye, d) = ray(view, x, y);
    let b = eye.dot(d);
    let disc = b * b - (eye.length_squared() - 1.);
    if disc < 0. {
        return None;
    }
    let p = eye + d * (-b - disc.sqrt());
    Some(colour(style, p.normalize(), (eye - p).normalize(), view))
}
/// The cube from -0.6 to 0.6 seen in perspective: the face normal hit and
/// the colour at screen (x, y).
fn persp_cube(style: Style, view: &ViewBasis, x: f32, y: f32) -> Option<(Vec3, [u8; 3])> {
    let (eye, d) = ray(view, x, y);
    let (mut near, mut far, mut normal) = (f32::MIN, f32::MAX, Vec3::ZERO);
    for axis in 0..3 {
        let (o, v) = (eye[axis], d[axis]);
        if v.abs() < 1e-9 {
            if o.abs() > 0.6 {
                return None;
            }
            continue;
        }
        let (a, b) = ((-0.6 - o) / v, (0.6 - o) / v);
        let (t0, t1) = (a.min(b), a.max(b));
        if t0 > near {
            near = t0;
            normal = Vec3::ZERO;
            normal[axis] = -v.signum();
        }
        far = far.min(t1);
    }
    if near > far || near < 0. {
        return None;
    }
    let p = eye + d * near;
    Some((normal, colour(style, normal, (eye - p).normalize(), view)))
}
/// Screen points on a grid, -1..1 both ways.
fn grid(steps: usize) -> impl Iterator<Item = (f32, f32)> {
    let s = steps as f32;
    (0..=steps).flat_map(move |j| {
        (0..=steps).map(move |i| (i as f32 / s * 2. - 1., j as f32 / s * 2. - 1.))
    })
}

/// The brightest value of the orthographic sphere and the centre of the
/// points that reach it.
fn brightest(style: Style, view: ViewBasis) -> ((f32, f32), u8) {
    let points: Vec<(f32, f32, u8)> = grid(100)
        .filter_map(|(x, y)| Some((x, y, sphere_rgb(style, view, x, y)?[0])))
        .collect();
    let peak = points.iter().map(|p| p.2).max().unwrap();
    let top: Vec<_> = points.iter().filter(|p| p.2 == peak).collect();
    let n = top.len() as f32;
    let x = top.iter().map(|p| p.0).sum::<f32>() / n;
    let y = top.iter().map(|p| p.1).sum::<f32>() / n;
    ((x, y), peak)
}
fn near(a: u8, b: u8, by: u8) {
    assert!(a.abs_diff(b) <= by, "{a} vs {b}");
}

/// VS-3 Solid: flat #999999 for the default grey, unlit.
#[test]
fn solid_is_flat_999999() {
    for (x, y) in [(0., 0.), (0.6, 0.6), (-0.5, -0.7)] {
        assert_eq!(sphere_rgb(Style::Solid, FRONT, x, y), Some([0x99; 3]));
    }
}

/// VS-2/VS-3 Shaded: lit by the view light straight above the line of
/// sight; ambient 29 + diffuse 184, the far side flat 29, no highlight.
#[test]
fn shaded_peaks_above_centre_and_is_flat_dark_below() {
    let ((x, y), peak) = brightest(Style::Shaded, FRONT);
    near(peak, 213, 1);
    assert!(x.abs() < 0.03, "centred left/right: {x}");
    assert!((y - 0.707).abs() < 0.05, "45° up: {y}");
    for (x, y) in [(0.3, 0.2), (0.6, -0.1), (0.2, 0.7)] {
        assert_eq!(
            sphere(Style::Shaded, FRONT, x, y),
            sphere(Style::Shaded, FRONT, -x, y)
        );
    }
    near(
        sphere(Style::Shaded, FRONT, 0., 0.),
        (29. + 184. * std::f32::consts::FRAC_1_SQRT_2).round() as u8,
        1,
    );
    for (x, y) in [(0., -0.8), (0.3, -0.9), (-0.2, -0.75)] {
        near(sphere(Style::Shaded, FRONT, x, y), 29, 1);
    }
}

/// VS-3: Default and Advanced use the same view light with their own
/// ambient and diffuse; Advanced adds a highlight where the normal is
/// halfway between light and eye. Since viewport materials (VM-1) the
/// highlight is the default material's principled gloss (Roughness 0.4),
/// broader than VS-3's tight one (VM-Q03), and the levels are matched in
/// linear light.
#[test]
fn default_and_advanced_levels() {
    let ((_, y), peak) = brightest(Style::Default, FRONT);
    near(peak, 215, 1);
    assert!((y - 0.707).abs() < 0.05);
    near(sphere(Style::Default, FRONT, 0., -0.9), 98, 1);
    let ((x, y), peak) = brightest(Style::Advanced, FRONT);
    assert!((245..=255).contains(&peak), "highlight peak {peak}");
    assert!(x.abs() < 0.03);
    assert!((y - 0.383).abs() < 0.05, "halfway: {y}");
    near(sphere(Style::Advanced, FRONT, 0., -0.9), 90, 3);
    // Facing the light: ambient + diffuse (225 for the default grey), with
    // a little of the gloss.
    near(sphere(Style::Advanced, FRONT, 0., 0.707), 225, 3);
    // The gloss spreads: brighter than the diffuse alone a way off its
    // centre.
    let lobe = sphere(Style::Advanced, FRONT, 0., 0.15) as i32;
    let n = Vec3::new(0., 0.15, (1. - 0.0225_f32).sqrt());
    let diffuse = (0.44 * 204. + 135. * n.dot(FRONT.headlight())) as i32;
    assert!(lobe > diffuse + 5, "{lobe} vs {diffuse}");
}

/// VS-T3: Default, Shaded and Advanced look different.
#[test]
fn default_shaded_and_advanced_differ() {
    let at = |s| [sphere(s, FRONT, 0., 0.), sphere(s, FRONT, 0., -0.9)];
    let (d, s, a) = (at(Style::Default), at(Style::Shaded), at(Style::Advanced));
    assert!(d != s && s != a && a != d, "{d:?} {s:?} {a:?}");
}

/// VS-3 Texture: lit like Shaded, times the material colour.
#[test]
fn texture_is_lit() {
    let ((_, y), peak) = brightest(Style::Texture, FRONT);
    near(peak, 161, 1);
    assert!((y - 0.707).abs() < 0.05);
    near(sphere(Style::Texture, FRONT, 0., -0.9), 22, 1);
    let head = FRONT.headlight();
    let red = shade(
        Style::Texture,
        [1., 0., 0.],
        [0.; 3],
        head,
        Vec3::Z,
        &FRONT,
        &defaults(),
    );
    assert_eq!(red[1], 0.);
    assert!((red[0] - (TEXTURE.0 + TEXTURE.1)).abs() < 1e-5);
}

/// VS-T6 Gooch: the light fixed to the view, toward its upper right; warm
/// along it and straight against it (|N·L|), a cool band between, no
/// highlight. The same in every view.
#[test]
fn gooch_warm_cool_warm_without_highlight() {
    let warm = [0xCC, 0xCC, 0x5C];
    let cool = [0x1F, 0x1F, 0x78];
    let d = 1. / 3f32.sqrt();
    let perpendicular = Vec3::new(1., -1., 0.).normalize();
    for view in [FRONT, TOP, RIGHT, orbit(0.7, 0.5)] {
        assert_eq!(sphere_rgb(Style::Gooch, view, d, d), Some(warm));
        assert_eq!(
            sphere_rgb(Style::Gooch, view, perpendicular.x, perpendicular.y),
            Some(cool)
        );
        let rim = sphere_rgb(Style::Gooch, view, -0.7, -0.7).unwrap();
        assert!(rim[0] > 0x90 && rim[2] < 0x70, "{rim:?}");
        assert_eq!(brightest(Style::Gooch, view).1, 0xCC);
    }
}

/// VS-T7 Cel: an outline ring, a black spot above the centre, two greys.
#[test]
fn cel_outline_spot_and_two_greys() {
    let g = |x, y| sphere(Style::Cel, FRONT, x, y);
    for a in 0..12 {
        let t = a as f32 / 12. * std::f32::consts::TAU;
        assert_eq!(g(0.95 * t.cos(), 0.95 * t.sin()), CEL_OUTLINE, "{t}");
    }
    let h = (Vec3::from(CEL_LIGHT).normalize() + Vec3::Z).normalize();
    assert_eq!(g(h.x, h.y), CEL_SPOT);
    assert!(h.y > 0.2 && h.x == 0.);
    assert_eq!(g(0., 0.85), CEL_LIT);
    assert_eq!(g(0., -0.5), CEL_SHADE);
    let seen: std::collections::BTreeSet<u8> = grid(40)
        .filter_map(|(x, y)| Some(sphere_rgb(Style::Cel, FRONT, x, y)?[0]))
        .collect();
    assert_eq!(
        seen,
        [CEL_OUTLINE, CEL_SPOT, CEL_LIT, CEL_SHADE]
            .into_iter()
            .collect()
    );
}

/// VS-T8 Matcap (was Reflection): fixed to the view, brighter at the top
/// than the bottom, not flat, the same in every view; the soft Clay Studio
/// matcap has no hard step anywhere (the default, Full Metal, has sharp
/// highlights on purpose).
#[test]
fn reflection_is_a_soft_matcap() {
    let clay = StyleColors {
        matcap: crate::matcap::Choice::BuiltIn(0),
        ..defaults()
    };
    let clay_at = |x: f32, y: f32| {
        let n = Vec3::new(x, y, (1. - x * x - y * y).sqrt());
        let c = grey();
        bytes(shade(Style::Reflection, c, c, n, Vec3::Z, &FRONT, &clay))
    };
    let (mut lo, mut hi) = (255u8, 0u8);
    for j in 0..=60 {
        let mut last: Option<[u8; 3]> = None;
        for i in 0..=60 {
            let (x, y) = (i as f32 / 30. - 1., j as f32 / 30. - 1.);
            if x * x + y * y > 0.95 {
                last = None;
                continue;
            }
            let c = clay_at(x, y);
            lo = lo.min(c[1]);
            hi = hi.max(c[1]);
            if let Some(p) = last {
                for ch in 0..3 {
                    assert!(
                        c[ch].abs_diff(p[ch]) <= 16,
                        "step {p:?} -> {c:?} at {x},{y}"
                    );
                }
            }
            last = Some(c);
        }
    }
    assert!(hi - lo > 40, "varied: {lo}..{hi}");
    let top = sphere(Style::Reflection, FRONT, 0., 0.6);
    let bottom = sphere(Style::Reflection, FRONT, 0., -0.6);
    assert!(top > bottom, "{top} vs {bottom}");
    for view in [TOP, RIGHT, orbit(1.1, -0.4)] {
        for (x, y) in [(0., 0.), (0.5, 0.3), (-0.4, -0.6)] {
            assert_eq!(
                sphere_rgb(Style::Reflection, view, x, y),
                sphere_rgb(Style::Reflection, FRONT, x, y)
            );
        }
    }
}

/// The styles whose picture depends on the view.
const CAMERA_FIXED: [Style; 7] = [
    Style::Shaded,
    Style::Texture,
    Style::Default,
    Style::Advanced,
    Style::Gooch,
    Style::Cel,
    Style::Reflection,
];

/// VS-T9: orbiting a perspective view round a sphere gives the same
/// on-screen picture in every style; the orthographic Front, Top and Right
/// views match too.
#[test]
fn orbiting_round_a_sphere_changes_nothing() {
    let views = [
        orbit(0.9, 0.3),
        orbit(-2.2, -0.6),
        orbit(3.0, 1.2),
        TOP,
        RIGHT,
    ];
    for style in CAMERA_FIXED {
        for (x, y) in grid(24) {
            let front = persp_sphere(style, &FRONT, x, y);
            for view in &views {
                match (front, persp_sphere(style, view, x, y)) {
                    (Some(a), Some(b)) => {
                        for ch in 0..3 {
                            assert!(a[ch].abs_diff(b[ch]) <= 1, "{style:?} {x},{y}: {a:?} {b:?}");
                        }
                    }
                    (None, None) => {}
                    _ => panic!("{style:?} silhouette moved at {x},{y}"),
                }
            }
            if let Some(f) = sphere_rgb(style, FRONT, x * 0.9, y * 0.9) {
                for view in [TOP, RIGHT] {
                    let o = sphere_rgb(style, view, x * 0.9, y * 0.9).unwrap();
                    for ch in 0..3 {
                        assert!(f[ch].abs_diff(o[ch]) <= 1, "{style:?} ortho {x},{y}");
                    }
                }
            }
        }
    }
}

/// VS-T10: rotating a scene light changes nothing without Show Scene
/// Lighting.
#[test]
fn scene_lights_are_ignored_by_default() {
    use crate::gpu::KeyLight;
    use operandi_core::camera_light::LightSettings;
    use operandi_core::{Scene, Transform};
    let mut scene = Scene::default();
    let view = KeyLight::Toward([0., 0.6, 0.8]);
    for rx in [0., 0.7, -1.4, 2.5] {
        scene.items.retain(|i| i.light.is_none());
        let turned = Transform {
            rotation: glam::DVec3::new(rx, 0.3, 0.),
            ..Transform::default()
        };
        scene.add_light(LightSettings::default(), turned).unwrap();
        assert_eq!(key_light(&scene, false, view), (view, true));
    }
}

/// VS-T11: the cube in the orthographic Front view. Gooch's front face is
/// one flat mid tone, Cel's is #7A7A7A (not the black spot), and a flat
/// face has one Reflection colour, in perspective too.
#[test]
fn cube_front_view() {
    let front = |style| colour(style, Vec3::Z, Vec3::Z, &FRONT);
    let t = 1. / 3f32.sqrt();
    let mid = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
    let g = front(Style::Gooch);
    let expect = [mid(0x1F, 0xCC), mid(0x1F, 0xCC), mid(0x78, 0x5C)];
    for ch in 0..3 {
        near(g[ch], expect[ch], 1);
    }
    assert_eq!(front(Style::Cel), [CEL_LIT; 3]);
    let reflection: std::collections::BTreeSet<[u8; 3]> = grid(20)
        .filter_map(|(x, y)| persp_cube(Style::Reflection, &FRONT, x * 0.3, y * 0.3))
        .map(|(_, c)| c)
        .collect();
    assert_eq!(reflection.len(), 1, "{reflection:?}");
}

/// VS-T12: the cube from an angled perspective view shows three different
/// Gooch tones and no black Cel face; Cel's outline hugs a sphere's
/// silhouette from any angle.
#[test]
fn cube_from_an_angle_and_the_cel_silhouette() {
    let view = orbit(0.7, 0.5);
    let mut gooch = std::collections::BTreeMap::new();
    for (x, y) in grid(40) {
        if let Some((n, c)) = persp_cube(Style::Gooch, &view, x, y) {
            gooch.insert(n.to_array().map(|v| v as i32), c);
        }
        if let Some((n, c)) = persp_cube(Style::Cel, &view, x, y) {
            assert_ne!(c[0], CEL_SPOT, "black Cel face {n:?} at {x},{y}");
        }
    }
    let tones: std::collections::BTreeSet<_> = gooch.values().collect();
    assert_eq!(gooch.len(), 3, "{gooch:?}");
    assert_eq!(tones.len(), 3, "{gooch:?}");
    // Every outline point lies further out on screen than every other
    // point of the sphere: a ring at its edge, whatever the angle.
    for view in [FRONT, orbit(0.7, 0.5), orbit(-2.0, -1.0), TOP] {
        let (mut inner, mut outer) = (0f32, f32::MAX);
        for (x, y) in grid(80) {
            let Some(c) = persp_sphere(Style::Cel, &view, x, y) else {
                continue;
            };
            let r = (x * x + y * y).sqrt();
            if c[0] == CEL_OUTLINE {
                outer = outer.min(r);
            } else {
                inner = inner.max(r);
            }
        }
        assert!(outer < f32::MAX, "no outline");
        assert!(inner <= outer + 0.03, "outline {outer} inside {inner}");
    }
}

/// VS-2: the view light is straight above the line of sight, 45° up, in
/// every view.
#[test]
fn headlight_is_above_the_line_of_sight() {
    let s = std::f32::consts::FRAC_1_SQRT_2;
    for v in [FRONT, TOP, RIGHT, orbit(0.4, -0.8)] {
        let h = v.headlight();
        assert!((h.dot(v.up) - s).abs() < 1e-3);
        assert!((h.dot(v.toward) - s).abs() < 1e-3);
        assert!(h.dot(v.right).abs() < 1e-6);
    }
    assert_eq!(
        crate::gpu::KeyLight::default(),
        crate::gpu::KeyLight::Toward(HEADLIGHT)
    );
}

/// Advanced's light: the view light without Show Scene Lighting or scene
/// lights; else the first rendering light, by its -Z (directional) or
/// position (point), and whether it casts.
#[test]
fn advanced_key_light() {
    use crate::gpu::KeyLight;
    use operandi_core::camera_light::{LightKind, LightRender, LightSettings, ShadowType};
    use operandi_core::{Scene, Transform};
    let view = KeyLight::default();
    let mut scene = Scene::default();
    scene.items.retain(|i| i.light.is_none());
    assert_eq!(key_light(&scene, true, view), (view, true));
    let down = Transform {
        rotation: glam::DVec3::new(-std::f64::consts::FRAC_PI_2, 0., 0.),
        ..Transform::default()
    };
    let id = scene.add_light(LightSettings::default(), down).unwrap();
    let (key, casts) = key_light(&scene, true, view);
    let KeyLight::Toward(t) = key else {
        panic!("{key:?}")
    };
    assert!(casts);
    assert!((Vec3::from(t) - Vec3::Y).length() < 1e-5, "{t:?}");
    fn light(s: &mut Scene, id: u64) -> &mut LightSettings {
        let item = s.item_mut(id).unwrap();
        item.light.as_mut().unwrap()
    }
    light(&mut scene, id).shadow = ShadowType::None;
    assert!(!key_light(&scene, true, view).1);
    light(&mut scene, id).render = LightRender::Off;
    assert_eq!(key_light(&scene, true, view), (view, true));
    light(&mut scene, id).render = LightRender::Default;
    light(&mut scene, id).kind = LightKind::Point;
    scene.item_mut(id).unwrap().transform = Transform {
        translation: glam::DVec3::new(1., 2., 3.),
        ..Transform::default()
    };
    assert_eq!(key_light(&scene, true, view).0, KeyLight::At([1., 2., 3.]));
}

#[test]
fn faces_flip_toward_the_eye_and_round_to_bytes() {
    let k = defaults();
    let base = crate::theme::viewport::SOLID;
    assert_eq!(
        face_color(Style::Solid, base, Vec3::Z, Vec3::Z, &FRONT, &k, false),
        Color32::from_gray(0x99)
    );
    for style in Style::ALL {
        assert_eq!(
            face_color(style, base, -Vec3::Z, Vec3::Z * 3., &FRONT, &k, false),
            face_color(style, base, Vec3::Z, Vec3::Z, &FRONT, &k, false),
            "{style:?}"
        );
    }
}

/// The Gooch defaults are the spec's, editable in Display > Colors, and
/// the uniform carries them.
#[test]
fn the_uniform_rows_carry_the_colours() {
    let k = defaults();
    assert_eq!(
        crate::theme::viewport::gooch_cool(),
        Color32::from_rgb(0x1F, 0x1F, 0x78)
    );
    assert_eq!(
        crate::theme::viewport::gooch_warm(),
        Color32::from_rgb(0xCC, 0xCC, 0x5C)
    );
    let rows = k.uniform_rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(&rows[0][..3], &k.gooch_cool);
    assert_eq!(&rows[1][..3], &k.gooch_warm);
    crate::theme::set_override(
        crate::theme::ColorId::GoochCool,
        Some(Color32::from_rgb(255, 0, 0)),
    );
    assert_eq!(StyleColors::live().gooch_cool, [1., 0., 0.]);
    crate::theme::set_overrides(&Default::default());
}

/// The shader carries this module's numbers, every placeholder is filled
/// and the Look struct holds the matcap.
#[test]
fn the_wgsl_carries_the_numbers() {
    let w = wgsl();
    for (name, v) in [
        ("shaded", SHADED.0),
        ("shaded", SHADED.1),
        ("texture", TEXTURE.1),
        ("default", DEFAULT.0),
        ("advanced", advanced_key()),
        ("solid", SOLID),
        ("cel spot", CEL_SPOT_ABOVE),
    ] {
        assert!(w.contains(&format!("{v:?}")), "{name} {v}");
    }
    let body = w
        .split("struct Look {")
        .nth(1)
        .and_then(|s| s.split("};").next())
        .unwrap();
    assert!(!body.contains("array<"), "the matcaps are a texture now");
    assert!(w.contains(&format!(
        "@group(2) @binding({}) var op_matcap: texture_2d_array<f32>;",
        crate::matcap::BINDING
    )));
    assert_eq!(w.matches('{').count(), w.matches('}').count());
    for gap in ["{cel", "{head", "{matcap", "_a}", "{gooch", "{spec"] {
        assert!(!w.contains(gap), "unfilled {gap}");
    }
}

/// Writes CPU spheres of every style (orthographic Front, Top, Right and a
/// perspective orbit) to `OPERANDI_SHADING_PREVIEW` (a PNG path) for a
/// look by eye.
#[test]
#[ignore]
fn preview_spheres() {
    let Ok(path) = std::env::var("OPERANDI_SHADING_PREVIEW") else {
        return;
    };
    let styles = [
        Style::Solid,
        Style::Shaded,
        Style::Texture,
        Style::Default,
        Style::Advanced,
        Style::Gooch,
        Style::Cel,
        Style::Reflection,
    ];
    let s = 128u32;
    let mut img =
        image::RgbImage::from_pixel(s * styles.len() as u32, s * 4, image::Rgb([58, 59, 62]));
    for (vi, view) in [FRONT, TOP, RIGHT, orbit(0.8, 0.4)].into_iter().enumerate() {
        for (si, &style) in styles.iter().enumerate() {
            for py in 0..s {
                for px in 0..s {
                    let x = (px as f32 + 0.5) / s as f32 * 2.2 - 1.1;
                    let y = 1.1 - (py as f32 + 0.5) / s as f32 * 2.2;
                    let c = if vi == 3 {
                        persp_sphere(style, &view, x * 0.4, y * 0.4)
                    } else {
                        sphere_rgb(style, view, x, y)
                    };
                    if let Some(c) = c {
                        img.put_pixel(si as u32 * s + px, vi as u32 * s + py, image::Rgb(c));
                    }
                }
            }
        }
    }
    img.save(path).unwrap();
}

/// Jason 2026-11-10: a selected polygon shows the selection colour on top
/// of every style. The style shades first; the highlight is scaled by how
/// bright the style made the default grey. In the lit styles that is the
/// selection lit exactly as before; in Gooch, Cel and Reflection the
/// selection colour now shows (it used to vanish), with their tones, bands
/// and matcap still visible in its brightness.
#[test]
fn selection_goes_on_top_of_every_style() {
    let k = defaults();
    let sel = unit(crate::theme::viewport::selection_fill());
    let views = [FRONT, orbit(0.7, 0.5)];
    for style in Style::ALL {
        for view in views {
            for (x, y) in [(0., 0.), (0.4, 0.5), (-0.5, -0.4), (0., 0.38)] {
                let n = view.right * x + view.up * y + view.toward * (1. - x * x - y * y).sqrt();
                let marked = shade_marked(style, sel, sel, n, view.toward, &view, &k, true);
                match style {
                    // Lit styles: the selection as a lit surface colour.
                    Style::Wireframe
                    | Style::Solid
                    | Style::Shaded
                    | Style::Texture
                    | Style::Default
                    | Style::VertexMap => {
                        let plain = shade(style, sel, sel, n, view.toward, &view, &k);
                        for ch in 0..3 {
                            assert!((marked[ch] - plain[ch]).abs() < 1e-4, "{style:?}");
                        }
                    }
                    _ => {
                        // The selection's hue: as much stronger in red than
                        // blue as the selection colour itself.
                        let (r, b) = (marked[0], marked[2]);
                        // (Skipping Advanced's highlight, which saturates.)
                        if r > 0.05 && marked.iter().all(|&v| v < 0.999) {
                            assert!(
                                (b / r - sel[2] / sel[0]).abs() < 0.02,
                                "{style:?} {marked:?}"
                            );
                        }
                    }
                }
            }
        }
    }
    // Cel's bands still read through the selection: the lit band brighter
    // than the shade band, the spot near black.
    let at = |x: f32, y: f32| {
        let n = Vec3::new(x, y, (1. - x * x - y * y).sqrt());
        shade_marked(Style::Cel, sel, sel, n, Vec3::Z, &FRONT, &k, true)[0]
    };
    assert!(at(0., 0.85) > at(0., -0.5));
    assert!(at(0., 0.38) < at(0., -0.5));
    // Never darker than the floor's share of the selection colour.
    assert!(at(0., 0.38) >= sel[0] * MARKED_FLOOR - 1e-4);
    // Unmarked surfaces are plain `shade`.
    let n = Vec3::Z;
    assert_eq!(
        shade_marked(Style::Gooch, sel, sel, n, n, &FRONT, &k, false),
        shade(Style::Gooch, sel, sel, n, n, &FRONT, &k)
    );
}

/// The view's matcap choice reaches the Reflection style; each built-in
/// gives its own look.
#[test]
fn reflection_uses_the_views_matcap() {
    let n = Vec3::new(0.3, 0.5, 0.81).normalize();
    let looks: std::collections::BTreeSet<[u8; 3]> = (0..crate::matcap::BUILT_IN.len())
        .map(|i| {
            let k = StyleColors {
                matcap: crate::matcap::Choice::BuiltIn(i as u8),
                ..defaults()
            };
            let c = grey();
            bytes(shade(Style::Reflection, c, c, n, Vec3::Z, &FRONT, &k))
        })
        .collect();
    assert_eq!(looks.len(), crate::matcap::BUILT_IN.len(), "{looks:?}");
}
