//! Render fix list 2026-11-13 (F1, F3, F6, F7): acceptance renders. F4 is
//! checked in `material_extras_tests.rs` (`absorption_follows_beers_law`).
use super::*;
use operandi_core::camera_light::LightSettings;
use operandi_core::mesh::Mesh;
use operandi_core::scene::Transform;

/// The default material (shader node 4) of a one-mesh scene.
fn material(scene: &mut Scene) -> &mut operandi_core::shader_tree::MaterialChannels {
    match scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        Some(NodeKind::Material(m)) => m,
        _ => panic!("no default material"),
    }
}

/// A ball of radius 0.5 with its centre 1 m up, over a white 6 m floor.
/// `glass`: (Transparent Color, Absorption Distance), fully see-through
/// at index 1.5; `None` leaves the default grey clay.
fn ball_and_floor(glass: Option<([f64; 3], f64)>) -> Vec<RenderMesh> {
    ball_and_floor_at(glass, 1.0)
}

/// `ball_and_floor` with the glass at Transparent Amount `amount`.
fn ball_and_floor_at(glass: Option<([f64; 3], f64)>, amount: f64) -> Vec<RenderMesh> {
    let mut ball = Scene::default();
    ball.new_mesh().unwrap();
    let item = ball.items.last_mut().unwrap();
    item.mesh = operandi_core::preset::matcap::sphere(48, 24, "Default")
        .unwrap()
        .into();
    item.transform.translation = DVec3::new(0.0, 1.0, 0.0);
    if let Some((tint, distance)) = glass {
        let m = material(&mut ball);
        m.transparent_amount = amount;
        m.refractive_index = 1.5;
        m.transparent_color = tint;
        m.absorption_distance = distance;
    }
    let mut floor = Scene::default();
    floor.new_mesh().unwrap();
    floor.items.last_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::new(0.0, -0.01, 0.0), DVec3::new(6.0, 0.02, 6.0), 1)
            .unwrap()
            .into();
    material(&mut floor).diffuse_color = [1.0; 3];
    let mut meshes = scene_meshes(&ball).unwrap();
    meshes.extend(scene_meshes(&floor).unwrap());
    meshes
}

/// Straight down onto the floor, orthographic, 5 m across `size` pixels:
/// columns run along +X, rows along +Z.
fn top_down(size: u32) -> Settings {
    let mut view = View::perspective(
        DVec3::new(0.0, 5.0, 0.0),
        DVec3::X,
        -DVec3::Z,
        -DVec3::Y,
        0.1,
    );
    view.orthographic = Some(2.5);
    let mut settings = Settings::studio(size, size, view);
    settings.gpu = false;
    settings.denoise = false;
    settings
}

/// The pixel at floor point (x, z) for a `top_down(size)` image.
fn pixel(size: u32, x: f64, z: f64) -> (usize, usize) {
    let per_metre = size as f64 / 5.0;
    (
        ((x + 2.5) * per_metre) as usize,
        ((z + 2.5) * per_metre) as usize,
    )
}

/// Every pixel's colour in the square of half-size `half` metres around
/// floor point (x, z).
fn patch(img: &Image, x: f64, z: f64, half: f64) -> Vec<[f32; 3]> {
    let (c0, r0) = pixel(img.width, x - half, z - half);
    let (c1, r1) = pixel(img.width, x + half, z + half);
    let mut out = Vec::new();
    for r in r0..r1 {
        for c in c0..c1 {
            let p = &img.rgba[(r * img.width as usize + c) * 4..];
            out.push([p[0], p[1], p[2]]);
        }
    }
    out
}

fn mean(px: &[[f32; 3]]) -> [f32; 3] {
    let n = px.len().max(1) as f32;
    let mut sum = [0.0f32; 3];
    for p in px {
        for i in 0..3 {
            sum[i] += p[i];
        }
    }
    sum.map(|v| v / n)
}

/// Saves `img` under OPERANDI_RENDER_FIX_SHOTS when it is set (for
/// looking at a failure).
fn shot(img: &Image, name: &str) {
    if let Ok(dir) = std::env::var("OPERANDI_RENDER_FIX_SHOTS") {
        let px: Vec<u8> = img
            .rgba
            .chunks(4)
            .flat_map(|p| {
                [p[0], p[1], p[2], 1.0].map(|v| (v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8)
            })
            .collect();
        image::save_buffer(
            format!("{dir}/{name}.png"),
            &px,
            img.width,
            img.height,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}

/// F1: a narrow caustics-only spot straight down through a glass ball,
/// Direct Caustics on. Returns the caustic's summed colour on the floor.
fn spot_caustic(tint: [f64; 3], distance: f64, name: &str) -> [f32; 3] {
    spot_caustic_at(tint, distance, 1.0, name)
}

/// `spot_caustic` with the glass at Transparent Amount `amount`.
fn spot_caustic_at(tint: [f64; 3], distance: f64, amount: f64, name: &str) -> [f32; 3] {
    let mut scene = Scene::default();
    scene.render.direct_caustics.enabled = true;
    let mut spot = LightSettings::new(LightKind::Spot);
    // A narrow cone so every ray crosses the ball near its axis: the
    // path inside is close to the diameter (1 m).
    spot.cone_angle = 4f64.to_radians();
    spot.radius = 0.0;
    spot.affect_diffuse = 0.0;
    spot.affect_specular = 0.0;
    spot.stored.affect_caustics = 1.0;
    let down = Transform {
        translation: DVec3::new(0.0, 3.0, 0.0),
        rotation: DVec3::new(-std::f64::consts::FRAC_PI_2, 0.0, 0.0),
        ..Default::default()
    };
    scene.add_light(spot, down).unwrap();
    let mut settings = top_down(40);
    settings.world = Vec::new();
    settings.lights = Some(scene_lights(&scene));
    settings.rays = Rays::from_render(&scene.render);
    settings.samples = 64;
    let img = render(
        &ball_and_floor_at(Some((tint, distance)), amount),
        &settings,
    )
    .unwrap();
    shot(&img, name);
    // The light does only caustics, so everything lit is the caustic.
    let all = patch(&img, 0.0, 0.0, 2.4);
    let mut sum = [0.0f32; 3];
    for p in &all {
        for i in 0..3 {
            sum[i] += p[i];
        }
    }
    sum
}

#[test]
fn shadow_caustic_takes_the_inside_colour() {
    // F1: amber glass, Absorption Distance 0.5 m. Cycles' manifold walk
    // (MNEE) already carries the inside's absorption into the caustic, so
    // no Cycles change is needed for this (an extra Beer's-law step tinted
    // it twice: 1 : 0.08 : 0.0004). Beer's law keeps
    // colour^(path / distance) per channel; near the axis the path is the
    // 1 m diameter, so R:G:B = 1 : (0.45/0.85)^2 : (0.12/0.85)^2
    // = 1 : 0.28 : 0.02. (The fix list's 1 : 0.40 : 0.16 was for its own
    // geometry.)
    let tint = [0.85, 0.45, 0.12];
    // At 1 m the path is one Absorption Distance: 1 : 0.53 : 0.14.
    for (distance, power) in [(0.5, 2), (1.0, 1)] {
        let c = spot_caustic(tint, distance, &format!("f1-amber-{distance}"));
        assert!(c[0] > 0.0, "no caustic: {c:?}");
        let want = [
            1.0,
            (0.45f32 / 0.85).powi(power),
            (0.12f32 / 0.85).powi(power),
        ];
        let got = [1.0, c[1] / c[0], c[2] / c[0]];
        eprintln!("F1 amber caustic, distance {distance}: R:G:B = {got:?}, Beer's law {want:?}");
        for i in 1..3 {
            assert!((got[i] - want[i]).abs() < 0.05, "{got:?} vs {want:?}");
        }
    }
    // A clear ball throws a neutral caustic.
    let c = spot_caustic([1.0; 3], 0.5, "f1-clear");
    let got = [1.0, c[1] / c[0], c[2] / c[0]];
    eprintln!("F1 clear caustic R:G:B = {got:?}");
    assert!(
        (got[1] - 1.0).abs() < 0.05 && (got[2] - 1.0).abs() < 0.05,
        "{got:?}"
    );
}

#[test]
fn glass_below_full_transparency_still_casts_its_caustic() {
    // Cycles' MNEE evaluates a caster's shader with only 4 closures and
    // then looks for the refractive one. The see-through mix used to emit
    // the principled surface first, so at 98 % Transparent Amount its
    // closures filled the slots, the glass was dropped and the caster
    // gave no caustic at all (a solid shadow). The glass now comes first.
    // MNEE weighs each of the two crossings by the glass closure's weight,
    // so 98 % keeps about 0.98^2 = 0.96 of the caustic. Lower amounts fall
    // off faster than amount^2 in Cycles (about 0.09 at 50 %); that part
    // is Cycles' and only checked to stay a caustic, not a solid shadow.
    let full = spot_caustic_at([1.0; 3], 0.5, 1.0, "caster-mix-100");
    let most = spot_caustic_at([1.0; 3], 0.5, 0.98, "caster-mix-98");
    let half = spot_caustic_at([1.0; 3], 0.5, 0.5, "caster-mix-50");
    let ratio = |c: [f32; 3]| (c[0] + c[1] + c[2]) / (full[0] + full[1] + full[2]);
    eprintln!(
        "caustic vs 100 %: 98 % = {:.3}, 50 % = {:.3}",
        ratio(most),
        ratio(half)
    );
    assert!(full[0] > 0.0, "no caustic at 100 %: {full:?}");
    assert!((ratio(most) - 0.96).abs() < 0.05, "98 %: {}", ratio(most));
    assert!(ratio(half) > 0.02, "50 % gave no caustic: {}", ratio(half));
}

#[test]
fn final_render_adds_no_sun_without_lights() {
    // F3: a constant white world and no light items. The final render
    // and the Preview light with the scene's lights only, so the floor
    // around a grey ball is lit evenly from every side. The old studio
    // setup adds a sun over the camera's shoulder, which throws a hard
    // shadow toward +X +Z (the check below proves it would be caught).
    let lit = |settings: Settings, name: &str| {
        let mut settings = settings;
        settings.world = vec![EnvironmentLayer::flat([1.0; 3], 1.0)];
        settings.samples = 32;
        let img = render(&ball_and_floor(None), &settings).unwrap();
        shot(&img, name);
        // The studio sun travels along (0.5, -1, 0.8): from the ball's
        // centre 1 m up, its shadow lands around (0.5, 0.8).
        let shadow_side = mean(&patch(&img, 0.5, 0.8, 0.15))[0];
        let other_side = mean(&patch(&img, -0.5, -0.8, 0.15))[0];
        (shadow_side, other_side)
    };
    let empty = Scene::default();
    let (a, b) = lit(top_down(80).with_only_scene_lights(&empty), "f3-only");
    eprintln!("F3 scene lights only: {a} vs {b}");
    assert!((a - b).abs() < 0.1 * b, "uneven without a sun: {a} vs {b}");
    let (a, b) = lit(top_down(80).with_scene_lights(&empty), "f3-studio");
    eprintln!("F3 studio sun: {a} vs {b}");
    assert!(a < 0.7 * b, "the studio sun's shadow: {a} vs {b}");
}

#[test]
fn a_scene_without_polygons_renders_its_environment() {
    // F6: the final render and the Preview no longer refuse a scene with
    // no polygons; it renders the environment. Same settings path as the
    // final render (render_ui.rs).
    let mut scene = Scene::default();
    scene.add_default_camera_and_light().unwrap();
    let meshes = scene_meshes(&scene).unwrap();
    assert!(meshes.is_empty());
    let up = View::perspective(DVec3::ZERO, DVec3::X, DVec3::Z, DVec3::Y, 0.01);
    let mut settings = Settings::studio(6, 6, up)
        .with_only_scene_lights(&scene)
        .with_scene_world(&scene);
    assert!(!settings.world.is_empty(), "a new scene has an environment");
    settings.samples = 4;
    settings.gpu = false;
    settings.denoise = false;
    let want = world_radiance(
        &settings.world,
        operandi_core::environment::RayKind::Camera,
        DVec3::Y,
    );
    let got = mean(&patch_all(&render(&meshes, &settings).unwrap()));
    eprintln!("F6 empty scene: {got:?}, environment {want:?}");
    for i in 0..3 {
        assert!((got[i] - want[i]).abs() < 0.02, "{got:?} vs {want:?}");
    }
}

fn patch_all(img: &Image) -> Vec<[f32; 3]> {
    img.rgba.chunks(4).map(|p| [p[0], p[1], p[2]]).collect()
}

#[test]
fn glass_shadow_is_even_and_tinted_without_direct_caustics() {
    // F7: Direct Caustics off, a sun at 45° through a glass ball. The
    // shadow is see-through: even (no bright focus), tinted by the
    // Transparent Color, and not black. Absorption Distance 0 passes the
    // colour once through the solid (F4), so the shadow is about the lit
    // floor times the colour.
    let tint = [1.0, 0.5, 0.25];
    let mut scene = Scene::default();
    // A new scene's settings: refraction caustics on (F5), which must not
    // focus light under the glass either.
    scene.render = operandi_core::render_settings::RenderSettings::new_scene();
    scene.render.direct_caustics.enabled = false;
    scene
        .add_light(
            LightSettings::new(LightKind::Directional),
            Transform::default(),
        )
        .unwrap();
    let mut lights = scene_lights(&scene);
    assert_eq!(lights.len(), 1);
    lights[0].direction = DVec3::new(1.0, -1.0, 0.0).normalize();
    let mut settings = top_down(80);
    settings.world = Vec::new();
    settings.lights = Some(lights);
    settings.rays = Rays::from_render(&scene.render);
    settings.samples = 64;
    let img = render(&ball_and_floor(Some((tint, 0.0))), &settings).unwrap();
    shot(&img, "f7-glass-shadow");
    // From the ball's centre 1 m up, the shadow is centred on x = 1, and
    // spans x 0.3..1.7 (the ball hides x up to 0.5 from the camera).
    let shadow = patch(&img, 1.15, 0.0, 0.2);
    let lit = mean(&patch(&img, -1.8, 0.0, 0.2));
    let m = mean(&shadow);
    let ratio = [m[0] / lit[0], m[1] / lit[1], m[2] / lit[2]];
    eprintln!("F7 shadow / lit floor = {ratio:?} (lit {lit:?})");
    assert!(lit[0] > 0.05, "the floor is lit: {lit:?}");
    // Not black and not focused: red (which the colour passes whole)
    // matches the lit floor; G and B fall with the colour.
    assert!((ratio[0] - 1.0).abs() < 0.15, "{ratio:?}");
    assert!(
        (ratio[1] / ratio[0] - 0.5).abs() < 0.1 && (ratio[2] / ratio[0] - 0.25).abs() < 0.1,
        "{ratio:?}"
    );
    // Even: no bright focus (with one, the peak was about 3x the lit
    // floor; sampling noise alone stays near 1.5x).
    let peak = shadow.iter().map(|p| p[0]).fold(0.0f32, f32::max);
    eprintln!("F7 shadow peak / lit floor (R) = {}", peak / lit[0]);
    assert!(peak < 2.0 * lit[0], "focus: peak {peak}, lit {}", lit[0]);
}
