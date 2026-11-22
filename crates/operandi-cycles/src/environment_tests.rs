//! The render world from environments (specifications:
//! environments-2026-11-09, "ENV"): the studio default, the §4 formulas in
//! Cycles, "newest visible wins" per ray kind (§6, §10), and scenes.
use super::*;
use operandi_core::environment::{EnvironmentMaterial, RayKind};

const RED: [f32; 3] = [1.0, 0.0, 0.0];
const GREEN: [f32; 3] = [0.0, 1.0, 0.0];

fn close(a: [f32; 3], b: [f32; 3], tolerance: f32) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() <= tolerance)
}

fn constant(color: [f32; 3]) -> EnvironmentLayer {
    EnvironmentLayer::flat(color, 1.0)
}

/// The default 4 Color Gradient environment (ENV §3), Intensity 1.
fn four_color() -> EnvironmentLayer {
    EnvironmentLayer {
        material: EnvironmentMaterial::default(),
        intensity: 1.0,
        up: DVec3::Y,
        visible: [true; 4],
        material_opacity: 1.0,
        material_invert: false,
        layers: Vec::new(),
        item_inverse: glam::DMat3::IDENTITY,
        sky: None,
    }
}

fn up(degrees: f64) -> DVec3 {
    let r = degrees.to_radians();
    DVec3::new(0.0, r.sin(), -r.cos())
}

/// Studio settings (no scene) keep the old flat sky: [0.55, 0.6, 0.7] ×
/// 0.6, seen by every ray.
#[test]
fn studio_world_is_the_old_flat_sky() {
    let s = Settings::studio(
        4,
        4,
        View::perspective(DVec3::Z, DVec3::X, DVec3::Y, -DVec3::Z, 0.5),
    );
    assert_eq!(s.world, vec![EnvironmentLayer::flat([0.55, 0.6, 0.7], 0.6)]);
    for direction in [DVec3::Y, -DVec3::Y, DVec3::X] {
        let c = s.world[0].radiance(direction);
        assert!(close(c, [0.33, 0.36, 0.42], 1e-6), "{c:?}");
    }
    assert!(close(
        environment_fog_color(&s.world),
        [0.33, 0.36, 0.42],
        1e-6
    ));
    assert_eq!(environment_fog_color(&[]), [0.0; 3]);
}

/// ENV §4's worked check on the Rust side: 30° up is 0.865.
#[test]
fn worked_check_in_the_reference() {
    let e = four_color();
    assert!((e.radiance(up(30.0))[0] - 0.865).abs() < 1e-6);
    assert!(close(e.radiance(DVec3::Y), [0.9; 3], 1e-6));
    assert!(close(e.radiance(-DVec3::Y), [0.03; 3], 1e-6));
    // The up axis turns the gradient.
    let tipped = EnvironmentLayer {
        up: -DVec3::Y,
        ..four_color()
    };
    assert!(close(tipped.radiance(DVec3::Y), [0.03; 3], 1e-6));
}

/// ENV §6 on the Rust side: the first (newest) visible one per ray kind.
#[test]
fn newest_visible_per_ray_kind() {
    let mut newer = constant(GREEN);
    newer.visible = [false, true, true, true];
    let world = vec![newer, constant(RED)];
    assert_eq!(world_radiance(&world, RayKind::Camera, DVec3::Y), RED);
    assert_eq!(world_radiance(&world, RayKind::Reflection, DVec3::Y), GREEN);
    assert_eq!(world_radiance(&[], RayKind::Camera, DVec3::Y), [0.0; 3]);
}

#[cfg(feature = "cycles")]
mod renders {
    use super::*;
    use operandi_core::environment::EnvironmentType;
    use operandi_core::mesh::Mesh;

    fn mean(img: &Image) -> [f32; 3] {
        let n = (img.rgba.len() / 4) as f32;
        let mut sum = [0.0f32; 3];
        for p in img.rgba.chunks(4) {
            for i in 0..3 {
                sum[i] += p[i];
            }
        }
        sum.map(|v| v / n)
    }

    /// Mean colour of a small empty-scene render looking along `forward`.
    fn sky(world: Vec<EnvironmentLayer>, forward: DVec3) -> [f32; 3] {
        let helper = if forward.y.abs() > 0.9 {
            DVec3::Z
        } else {
            DVec3::Y
        };
        let right = forward.cross(helper).normalize();
        let up = right.cross(forward);
        let mut settings = Settings::studio(
            6,
            6,
            View::perspective(DVec3::ZERO, right, up, forward, 0.01),
        );
        settings.world = world;
        settings.lights = Some(Vec::new());
        settings.sun_strength = 0.0;
        settings.samples = 4;
        settings.gpu = false;
        settings.denoise = false;
        mean(&render(&[], &settings).unwrap())
    }

    /// The studio flat sky still renders as before.
    #[test]
    fn studio_world_renders_the_flat_sky() {
        let world = Settings::studio(
            1,
            1,
            View::perspective(DVec3::Z, DVec3::X, DVec3::Y, -DVec3::Z, 0.5),
        )
        .world;
        let seen = sky(world, DVec3::new(0.3, 0.4, -1.0).normalize());
        assert!(close(seen, [0.33, 0.36, 0.42], 0.01), "{seen:?}");
    }

    /// ENV §4 in Cycles: the worked check (30° up = 0.865), the four
    /// points, and every type against the Rust reference.
    #[test]
    fn formulas_match_the_spec() {
        let at = |e: EnvironmentLayer, d: DVec3| sky(vec![e], d)[0];
        assert!((at(four_color(), up(30.0)) - 0.865).abs() < 0.005);
        assert!((at(four_color(), DVec3::Y) - 0.9).abs() < 0.005);
        // Just above and below the horizon: near sky / ground (1° is
        // already 0.378 with exponent 4), matching the reference.
        for d in [up(1.0), up(-1.0)] {
            let want = four_color().radiance(d)[0];
            assert!((at(four_color(), d) - want).abs() < 0.01, "{want}");
        }
        assert!((at(four_color(), -DVec3::Y) - 0.03).abs() < 0.005);
        for kind in EnvironmentType::ALL.iter().copied() {
            let mut e = four_color();
            e.material = EnvironmentMaterial {
                kind,
                zenith: [0.8, 0.2, 0.1],
                sky: [0.1, 0.7, 0.2],
                ground: [0.3, 0.2, 0.6],
                nadir: [0.05, 0.1, 0.4],
                sky_exponent: 2.0,
                ground_exponent: 1.0,
                ..Default::default()
            };
            e.intensity = 2.0;
            for degrees in [60.0, 20.0, -20.0, -60.0] {
                let d = up(degrees);
                let seen = sky(vec![e.clone()], d);
                let want = e.radiance(d);
                assert!(
                    close(seen, want, 0.02),
                    "{kind:?} {degrees}°: {seen:?} vs {want:?}"
                );
            }
        }
    }

    /// ENV §6 measured cases: both visible, the newer shows; newer hidden
    /// from the camera, the older; older hidden, the newer; Intensity 0.5
    /// halves the one shown. Not added together.
    #[test]
    fn newest_visible_environment_shows() {
        let older = constant(RED);
        let newer = constant(GREEN);
        let seen = |w: Vec<EnvironmentLayer>| sky(w, DVec3::Y);
        assert!(close(seen(vec![newer.clone(), older.clone()]), GREEN, 0.01));
        let mut hidden = newer.clone();
        hidden.visible[0] = false;
        assert!(close(seen(vec![hidden, older.clone()]), RED, 0.01));
        let mut old_hidden = older.clone();
        old_hidden.visible[0] = false;
        assert!(close(seen(vec![newer.clone(), old_hidden]), GREEN, 0.01));
        let mut half = newer.clone();
        half.intensity = 0.5;
        assert!(close(seen(vec![half, older]), [0.0, 0.5, 0.0], 0.01));
        assert!(close(seen(Vec::new()), [0.0; 3], 1e-4), "none is black");
    }

    /// One environment for the camera, another for indirect light: the
    /// background shows the first, a diffuse plane is lit by the second.
    #[test]
    fn camera_and_indirect_see_different_environments() {
        let mut meshes_scene = Scene::default();
        meshes_scene.new_mesh().unwrap();
        meshes_scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::new(0.0, 0.0, -0.05), DVec3::new(4.0, 4.0, 0.1), 1)
                .unwrap()
                .into();
        let meshes = scene_meshes(&meshes_scene).unwrap();
        let mut backdrop = constant(RED);
        backdrop.visible = [true, false, false, false];
        let mut lighting = constant(GREEN);
        lighting.visible = [false, true, true, true];
        let shot = |forward: DVec3| {
            let mut settings = Settings::studio(
                8,
                8,
                View::perspective(DVec3::new(0.0, 0.0, 3.0), DVec3::X, DVec3::Y, forward, 0.1),
            );
            settings.world = vec![lighting.clone(), backdrop.clone()];
            settings.lights = Some(Vec::new());
            settings.base_color = [0.8; 3];
            settings.roughness = 1.0;
            settings.samples = 16;
            settings.gpu = false;
            settings.denoise = false;
            mean(&render(&meshes, &settings).unwrap())
        };
        let background = shot(DVec3::Z);
        assert!(close(background, RED, 0.01), "background {background:?}");
        let plane = shot(-DVec3::Z);
        assert!(
            plane[1] > 0.3 && plane[0] < 0.02,
            "lit green only: {plane:?}"
        );
    }

    /// A new scene renders its default environment; a scene without one is
    /// black.
    #[test]
    fn scene_environment_reaches_the_render() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.add_default_camera_and_light().unwrap();
        let world = scene_world(&scene);
        assert_eq!(world.len(), 1);
        assert!((sky(world, DVec3::Y)[0] - 0.9).abs() < 0.005);
        scene.items.retain(|i| i.environment.is_none());
        let settings = Settings::studio(
            1,
            1,
            View::perspective(DVec3::Z, DVec3::X, DVec3::Y, -DVec3::Z, 0.5),
        )
        .with_scene_world(&scene);
        assert!(settings.world.is_empty());
    }

    /// ENV §5 measured: a green Constant at 50 % Normal over a red
    /// environment gives (0.5, 0.5, 0).
    #[test]
    fn constant_layer_blends_over_the_material() {
        let mut e = constant(RED);
        e.layers.push(EnvironmentTexture {
            source: EnvironmentTextureSource::Constant(GREEN),
            opacity: 0.5,
            mix: 0,
            invert: false,
            to_image: glam::DMat3::IDENTITY,
            light_probe: false,
        });
        assert!(close(e.radiance(DVec3::Y), [0.5, 0.5, 0.0], 1e-6));
        assert!(close(sky(vec![e], DVec3::Y), [0.5, 0.5, 0.0], 0.01));
    }

    /// A 4×4 latitude-longitude test image: the top row red, green, blue,
    /// yellow from left to right; the rest grey.
    fn equirect() -> String {
        let mut img = image::RgbImage::from_pixel(4, 4, image::Rgb([128, 128, 128]));
        for (x, c) in [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0]]
            .into_iter()
            .enumerate()
        {
            img.put_pixel(x as u32, 0, image::Rgb(c));
        }
        let path = std::env::temp_dir().join(format!("operandi-env-{}.png", std::process::id()));
        img.save(&path).unwrap();
        path.to_string_lossy().into_owned()
    }

    /// Looking at column `c`'s centre, 70° up: u = 0.5 - longitude/360°
    /// with longitude from +Z toward +X, so u grows toward -X and the
    /// picture reads left to right looking down -Z (ETL-M1; ENV §5, the
    /// locator's Spherical Y).
    fn toward_column(c: f64, turn: f64) -> DVec3 {
        let u = (c + 0.5) / 4.0;
        let longitude = (0.5 - u) * std::f64::consts::TAU + turn;
        let e = 70f64.to_radians();
        DVec3::new(
            longitude.sin() * e.cos(),
            e.sin(),
            longitude.cos() * e.cos(),
        )
    }

    /// An Image Map under an environment in a scene: Spherical about Y,
    /// read along the ray; the locator's rotation turns it.
    #[test]
    fn image_layer_reads_latitude_longitude() {
        use operandi_core::environment::EnvironmentLayerEdit;
        use operandi_core::shader_tree::{NewLayer, NodeKind, TextureSource};
        let path = equirect();
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.add_default_camera_and_light().unwrap();
        let env = scene.environments().next().unwrap().id;
        let image = scene.image_for_path(&path, 4, 4).unwrap();
        scene
            .edit_environment_layers(
                env,
                EnvironmentLayerEdit::Add {
                    layer: NewLayer::ImageMap,
                    at: operandi_core::environment::LayerPlace::Top,
                },
            )
            .unwrap();
        let mut node = scene
            .items
            .iter()
            .find(|i| i.id == env)
            .unwrap()
            .environment
            .as_ref()
            .unwrap()
            .layers[0]
            .clone();
        let NodeKind::Texture(t) = &mut node.kind else {
            panic!()
        };
        if let TextureSource::ImageMap { image: slot, .. } = &mut t.source {
            *slot = Some(image);
        }
        let locator = t.locator.unwrap();
        scene
            .edit_environment_layers(env, EnvironmentLayerEdit::Replace(Box::new(node)))
            .unwrap();
        let colours = [RED, GREEN, [0.0, 0.0, 1.0], [1.0, 1.0, 0.0]];
        // The picture is sRGB, so the grey row next to the colours (code
        // 128, 0.216 linear) bleeds into a filtered lookup a little more
        // than when Cycles read it undecoded: 0.08, not 0.05.
        let tolerance = 0.08;
        for (c, want) in colours.iter().enumerate() {
            let seen = sky(scene_world(&scene), toward_column(c as f64, 0.0));
            assert!(close(seen, *want, tolerance), "column {c}: {seen:?}");
        }
        // Rotating the locator 90° about Y turns the image with it.
        let item = scene.items.iter_mut().find(|i| i.id == locator).unwrap();
        item.transform.rotation = DVec3::new(0.0, std::f64::consts::FRAC_PI_2, 0.0);
        for (c, want) in colours.iter().enumerate() {
            let seen = sky(
                scene_world(&scene),
                toward_column(c as f64, std::f64::consts::FRAC_PI_2),
            );
            assert!(close(seen, *want, tolerance), "turned column {c}: {seen:?}");
        }
        let _ = std::fs::remove_file(&path);
    }

    /// ENV §4: Physically-based Daylight renders Cycles' sky with the sun
    /// where its Sun Light shines from: the sun is far brighter than the
    /// sky opposite it, the sky overhead is blue. Without a Sun Light it
    /// draws its gradient [chosen].
    #[test]
    fn physical_daylight_follows_its_sun_light() {
        use operandi_core::environment::{EnvironmentEdit, EnvironmentType};
        let mut e = operandi_core::Editor::default();
        e.scene.new_mesh().unwrap();
        e.scene.add_default_camera_and_light().unwrap();
        let env = e.scene.environments().next().unwrap().id;
        let sun = e.scene.items.iter().find(|i| i.light.is_some()).unwrap().id;
        e.dispatch(operandi_core::Command::EditEnvironment {
            item: env,
            edit: EnvironmentEdit::Kind(EnvironmentType::Physical),
        })
        .unwrap();
        let no_sun = scene_world(&e.scene);
        assert!(no_sun[0].sky.is_none());
        assert!(
            (sky(no_sun, DVec3::Y)[0] - 0.9).abs() < 0.005,
            "gradient without a sun"
        );
        e.dispatch(operandi_core::Command::EditEnvironment {
            item: env,
            edit: EnvironmentEdit::SunLight(Some(sun)),
        })
        .unwrap();
        let world = scene_world(&e.scene);
        let toward = world[0].sky.as_ref().unwrap().toward_sun;
        assert!(
            toward.y > 0.05,
            "the default light shines from above: {toward:?}"
        );
        let at_sun = sky(world.clone(), toward);
        let away = sky(
            world.clone(),
            DVec3::new(-toward.x, toward.y, -toward.z).normalize(),
        );
        assert!(
            at_sun[0] > 10.0 * away[0].max(1e-3),
            "{at_sun:?} vs {away:?}"
        );
        let up = sky(world, DVec3::Y);
        assert!(up[2] > up[0], "blue overhead: {up:?}");
    }
}

/// Rotation's sign (HDRI stage 1): an environment image turned +90° on its
/// Texture Locator heading reads, in Cycles' Environment Texture, the same
/// pixel the viewport sky reads (`environment_image_uv`), and what was
/// straight ahead (+Z) is then seen along +X.
#[test]
fn environment_image_rotation_matches_the_viewport() {
    let mut editor = operandi_core::Editor::default();
    editor
        .dispatch(operandi_core::Command::SetEnvironmentImage {
            path: "/hdri/sky.hdr".into(),
            width: 64,
            height: 32,
            item: None,
        })
        .unwrap();
    let env = editor.scene.environments().next().unwrap().id;
    let (locator, _) = editor.scene.environment_image_heading(env).unwrap();
    let mut t = editor
        .scene
        .items
        .iter()
        .find(|i| i.id == locator)
        .unwrap()
        .transform
        .clone();
    t.rotation.y = 90f64.to_radians();
    editor
        .dispatch(operandi_core::Command::ItemTransform {
            item: locator,
            transform: t,
        })
        .unwrap();
    let item = editor.scene.items.iter().find(|i| i.id == env).unwrap();
    let layer = EnvironmentLayer::of(&editor.scene, item).unwrap();
    let to_image = layer.layers[0].to_image;
    // Cycles: direction_to_equirectangular, v from the bottom row.
    let cycles_uv = |d: DVec3| {
        let c = image_frame_to_cycles() * to_image * d.normalize();
        let u = 0.5 - c.y.atan2(c.x) / std::f64::consts::TAU;
        let v = 0.5 + c.z.atan2(c.x.hypot(c.y)) / std::f64::consts::PI;
        [u.rem_euclid(1.0), 1.0 - v]
    };
    for d in [
        DVec3::X,
        DVec3::Z,
        -DVec3::X,
        DVec3::new(0.3, 0.5, -0.8),
        DVec3::new(-0.6, -0.2, 0.1),
    ] {
        let a = cycles_uv(d);
        let b = operandi_core::environment::environment_image_uv(to_image, d);
        assert!(
            (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9,
            "{d}: {a:?} {b:?}"
        );
    }
    let ahead = operandi_core::environment::environment_image_uv(glam::DMat3::IDENTITY, DVec3::Z);
    let seen = operandi_core::environment::environment_image_uv(to_image, DVec3::X);
    assert!((ahead[0] - seen[0]).abs() < 1e-9);
}

/// Light Probe (ETL-Q02): Cycles' Mirror Ball Environment Texture reads
/// the pixel `environment_probe_uv` gives, turned by the locator.
#[test]
fn light_probe_matches_the_viewport() {
    let mut editor = operandi_core::Editor::default();
    editor
        .dispatch(operandi_core::Command::SetEnvironmentImage {
            path: "/hdri/ball.hdr".into(),
            width: 64,
            height: 64,
            item: None,
        })
        .unwrap();
    let env = editor.scene.environments().next().unwrap().id;
    let (locator, _) = editor.scene.environment_image_heading(env).unwrap();
    let item = editor.scene.items.iter().find(|i| i.id == locator).unwrap();
    let mut settings = item.texture_locator().unwrap().clone();
    settings.projection = operandi_core::shader_tree::Projection::LightProbe;
    let mut transform = item.transform.clone();
    transform.rotation = DVec3::new(0.2, 0.9, -0.3);
    editor
        .dispatch(operandi_core::Command::ShaderTree(
            operandi_core::shader_tree::ShaderTreeEdit::SetLocator {
                locator,
                settings: Box::new(settings),
                transform,
                continuing: false,
            },
        ))
        .unwrap();
    let item = editor.scene.items.iter().find(|i| i.id == env).unwrap();
    let layer = EnvironmentLayer::of(&editor.scene, item).unwrap();
    assert!(layer.layers[0].light_probe);
    let to_image = layer.layers[0].to_image;
    // Cycles: direction_to_mirrorball, v from the bottom row.
    let cycles_uv = |d: DVec3| {
        let mut c = probe_frame_to_cycles() * to_image * d.normalize();
        c.y -= 1.0;
        c /= 2.0 * (-0.5 * c.y).max(0.0).sqrt();
        [0.5 * (c.x + 1.0), 1.0 - 0.5 * (c.z + 1.0)]
    };
    for d in [
        DVec3::X,
        DVec3::Z,
        -DVec3::X,
        DVec3::new(0.3, 0.5, -0.8),
        DVec3::new(-0.6, -0.2, 0.1),
    ] {
        let a = cycles_uv(d);
        let b = operandi_core::environment::environment_probe_uv(to_image, d);
        assert!(
            (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9,
            "{d}: {a:?} {b:?}"
        );
    }
}

/// HDRI stage 5: a world with an environment picture is importance
/// sampled (the bridge builds Cycles' background light for it); a
/// gradient or constant world isn't, and the lights' rebuild key follows.
#[test]
fn an_environment_picture_turns_on_world_importance_sampling() {
    let mut world = vec![four_color()];
    assert!(!world_samples_image(&world));
    world[0].layers.push(EnvironmentTexture {
        source: EnvironmentTextureSource::Constant(RED),
        opacity: 1.0,
        mix: 0,
        invert: false,
        to_image: glam::DMat3::IDENTITY,
        light_probe: false,
    });
    assert!(!world_samples_image(&world));
    world.push(constant(GREEN));
    world[1].layers.push(EnvironmentTexture {
        source: EnvironmentTextureSource::Image("/hdri/sky.hdr".into()),
        opacity: 1.0,
        mix: 0,
        invert: false,
        to_image: glam::DMat3::IDENTITY,
        light_probe: false,
    });
    assert!(world_samples_image(&world));
    // From a scene: Load Image… on the environment turns it on.
    let mut editor = operandi_core::Editor::default();
    editor.scene.ensure_environment().unwrap();
    assert!(!world_samples_image(&scene_world(&editor.scene)));
    editor
        .dispatch(operandi_core::Command::SetEnvironmentImage {
            path: "/hdri/sky.hdr".into(),
            width: 8,
            height: 4,
            item: None,
        })
        .unwrap();
    assert!(world_samples_image(&scene_world(&editor.scene)));
}
