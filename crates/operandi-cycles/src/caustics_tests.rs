//! Direct Caustics (Direct Caustics spec v1, DC-01..03): caustics cast by
//! the scene's own lights through glass, rendered with Cycles' manifold
//! next event estimation and Operandi's caustics-only lights.
use super::*;
use operandi_core::camera_light::LightSettings;
#[cfg(feature = "cycles")]
use operandi_core::mesh::Mesh;
use operandi_core::scene::Transform;

/// A spot light 3 m up aiming straight down, cone 10°, that does only
/// caustics (Affect Diffuse and Specular 0, Affect Caustics 100 %), and
/// optionally a second wide spot with Affect Caustics 0 (DC-03).
fn lights(direct: bool, caustics: f64, fill: bool) -> Scene {
    let mut scene = Scene::default();
    scene.render.direct_caustics.enabled = direct;
    let down = Transform {
        translation: DVec3::new(0.0, 3.0, 0.0),
        rotation: DVec3::new(-std::f64::consts::FRAC_PI_2, 0.0, 0.0),
        ..Default::default()
    };
    let mut spot = LightSettings::new(LightKind::Spot);
    spot.cone_angle = 10f64.to_radians();
    spot.radius = 0.0;
    spot.affect_diffuse = 0.0;
    spot.affect_specular = 0.0;
    spot.stored.affect_caustics = caustics;
    scene.add_light(spot, down.clone()).unwrap();
    if fill {
        let mut wide = LightSettings::new(LightKind::Spot);
        wide.cone_angle = 120f64.to_radians();
        wide.stored.affect_caustics = 0.0;
        scene.add_light(wide, down).unwrap();
    }
    scene
}

/// DC-01's glass sphere (radius 0.5, index 1.5, fully see-through) with
/// its centre 1 m up, and the white diffuse floor under it.
#[cfg(feature = "cycles")]
fn sphere_and_floor() -> Vec<RenderMesh> {
    let mut glass = Scene::default();
    glass.new_mesh().unwrap();
    let item = glass.items.last_mut().unwrap();
    item.mesh = operandi_core::preset::matcap::sphere(48, 24, "Default")
        .unwrap()
        .into();
    item.transform.translation = DVec3::new(0.0, 1.0, 0.0);
    if let Some(NodeKind::Material(m)) = glass
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.transparent_amount = 1.;
        m.refractive_index = 1.5;
        m.transparent_color = [1.0; 3];
    }
    let mut floor = Scene::default();
    floor.new_mesh().unwrap();
    floor.items.last_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::new(0.0, -0.01, 0.0), DVec3::new(6.0, 0.02, 6.0), 1)
            .unwrap()
            .into();
    let mut meshes = scene_meshes(&glass).unwrap();
    meshes.extend(scene_meshes(&floor).unwrap());
    meshes
}

#[test]
fn caustics_only_copies_follow_affect_caustics() {
    // Off: the caustic light lights nothing, so it is left out.
    assert!(scene_lights(&lights(false, 1.0, false)).is_empty());
    // On: one caustics-only copy at Affect Caustics' share.
    let full = scene_lights(&lights(true, 1.0, false));
    assert_eq!(full.len(), 1);
    assert!(full[0].caustics_only && !full[0].visible_to_camera);
    assert!((full[0].direction - -DVec3::Y).length() < 1e-9);
    let half = scene_lights(&lights(true, 0.5, false));
    assert!((half[0].strength[0] - 0.5 * full[0].strength[0]).abs() < 1e-4);
    // DC-02: Affect Caustics 0 leaves nothing.
    assert!(scene_lights(&lights(true, 0.0, false)).is_empty());
    // DC-03: the fill light stays an ordinary light with no copy.
    let both = scene_lights(&lights(true, 1.0, true));
    assert_eq!(both.iter().filter(|l| l.caustics_only).count(), 1);
    assert_eq!(both.len(), 2);
    assert!(Rays::from_render(&lights(true, 1.0, false).render).direct_caustics);
}

#[cfg(feature = "cycles")]
mod renders {
    use super::*;

    /// The brightest floor pixel right under the sphere and the mean
    /// brightness of the floor 2 m to the side, seen from straight above
    /// (orthographic).
    fn floor(direct: bool, caustics: f64, fill: bool) -> (f32, f32) {
        let scene = lights(direct, caustics, fill);
        let mut view = View::perspective(
            DVec3::new(0.0, 5.0, 0.0),
            DVec3::X,
            -DVec3::Z,
            -DVec3::Y,
            0.1,
        );
        view.orthographic = Some(2.5);
        let mut settings = Settings::studio(40, 40, view);
        settings.world = Vec::new();
        settings.lights = Some(scene_lights(&scene));
        settings.sun_strength = 0.0;
        settings.rays = Rays::from_render(&scene.render);
        settings.samples = 64;
        settings.gpu = false;
        settings.denoise = false;
        let img = render(&sphere_and_floor(), &settings).unwrap();
        // 40 px span 5 m: x 4..10 is about 2 m to the side.
        let mean = |x0: usize, y0: usize| {
            let mut sum = 0.0;
            for y in y0..y0 + 6 {
                for x in x0..x0 + 6 {
                    let p = &img.rgba[(y * 40 + x) * 4..];
                    sum += (p[0] + p[1] + p[2]) / 3.0;
                }
            }
            sum / 36.0
        };
        if let Ok(dir) = std::env::var("OPERANDI_CAUSTIC_SHOTS") {
            let name = format!("{dir}/dc-{direct}-{caustics}-{fill}.png");
            let px: Vec<u8> = img
                .rgba
                .chunks(4)
                .flat_map(|p| {
                    [p[0], p[1], p[2], 1.0]
                        .map(|v| (v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8)
                })
                .collect();
            image::save_buffer(&name, &px, 40, 40, image::ColorType::Rgba8).unwrap();
        }
        // The caustic is small and sharp: take the brightest pixel under
        // the sphere.
        let peak = (15..25)
            .flat_map(|y| (15..25).map(move |x| (y, x)))
            .map(|(y, x)| {
                let p = &img.rgba[(y * 40 + x) * 4..];
                (p[0] + p[1] + p[2]) / 3.0
            })
            .fold(0.0f32, f32::max);
        let _ = mean(17, 17);
        (peak, mean(4, 17))
    }

    #[test]
    fn spot_through_glass_focuses_on_the_floor() {
        // DC-01: off, the floor is black; on, a bright focused spot.
        let (under, side) = floor(false, 1.0, false);
        assert!(under < 1e-3 && side < 1e-3, "off: {under} {side}");
        let (under, side) = floor(true, 1.0, false);
        assert!(under > 0.05, "on: {under}");
        assert!(side < 1e-3, "the light does only caustics: {side}");
        // DC-02: Affect Caustics 0 is black again.
        let (under, _) = floor(true, 0.0, false);
        assert!(under < 1e-3, "affect 0: {under}");
        // DC-03: a wide fill light lights the floor and adds no caustic.
        let (_, lit) = floor(false, 1.0, true);
        let (under_fill, side_fill) = floor(true, 1.0, true);
        let (under_alone, _) = floor(true, 1.0, false);
        assert!(
            lit > 1e-3 && (side_fill - lit).abs() < 0.2 * lit,
            "fill: {lit} {side_fill}"
        );
        assert!(
            under_fill > 0.5 * under_alone,
            "{under_fill} vs {under_alone}"
        );
    }
}
