//! OpenPBR in Cycles (OPM-03): the new Principled inputs, the interior
//! scatter, Thin Wall and the principled glass reach the compiled surface
//! and change the render; untouched materials compile as before.
use super::*;
use operandi_core::mesh::Mesh;
use operandi_core::shader_tree::MaterialChannels;

fn box_scene(f: impl FnOnce(&mut MaterialChannels)) -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    if let Some(NodeKind::Material(m)) = scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        f(m);
    }
    scene
}

fn shader(scene: &Scene) -> RenderShader {
    scene_meshes(scene).unwrap()[0].shader.clone().unwrap()
}

#[test]
fn untouched_materials_give_no_openpbr_inputs() {
    let sh = shader(&box_scene(|m| {
        // A material from before the OpenPBR channels.
        m.openpbr = operandi_core::shader_tree::OpenPbrChannels::legacy();
        m.clearcoat_amount = 0.5;
        m.sheen = 0.5;
        m.transparent_amount = 0.5;
        m.refractive_index = 1.5;
    }));
    for input in [
        Input::CoatTint,
        Input::CoatIor,
        Input::SheenTint,
        Input::SheenRoughness,
        Input::ThinFilmThickness,
        Input::ThinFilmIor,
        Input::DispersionScale,
        Input::DispersionAbbe,
    ] {
        assert_eq!(sh.inputs[input as usize], -1, "{input:?}");
    }
    assert!(!sh.thin_wall && !sh.glass_principled && sh.scatter.1 == 0.0);
}

#[test]
fn openpbr_channels_compile_into_the_surface() {
    let sh = shader(&box_scene(|m| {
        m.clearcoat_amount = 1.;
        m.openpbr.coat_color = [1., 0.2, 0.2];
        m.openpbr.coat_ior = 1.8;
        m.sheen = 1.;
        m.openpbr.fuzz_color = [0.2, 0.2, 1.];
        m.openpbr.thin_film_weight = 1.;
        m.openpbr.thin_film_thickness = 0.4;
        m.transparent_amount = 1.;
        m.refractive_index = 1.5;
        m.absorption_distance = 0.5;
        m.transparent_color = [0.5, 0.8, 0.9];
        m.openpbr.transmission_scatter = [0.3, 0.3, 0.3];
        m.more.dispersion = 0.5;
    }));
    assert!(sh.is_valid());
    for input in [
        Input::CoatTint,
        Input::CoatIor,
        Input::SheenTint,
        Input::ThinFilmThickness,
        Input::ThinFilmIor,
        Input::DispersionScale,
    ] {
        assert!(sh.inputs[input as usize] >= 0, "{input:?}");
    }
    assert!(sh.glass_principled && sh.scatter.1 > 0.0 && sh.absorption.1 > 0.0);
}

#[cfg(feature = "cycles")]
fn render_box(scene: &Scene) -> Vec<f32> {
    let mut settings = Settings::studio(
        24,
        24,
        View::perspective(
            DVec3::new(2.0, 1.5, 3.0),
            DVec3::new(0.83, 0.0, -0.55),
            DVec3::Y,
            DVec3::new(-0.5, -0.38, -0.78).normalize(),
            0.6,
        ),
    );
    settings.samples = 8;
    settings.gpu = false;
    settings.denoise = false;
    let image = render(&scene_meshes(scene).unwrap(), &settings).unwrap();
    assert!(image.rgba.iter().all(|v| v.is_finite()));
    image.rgba
}

#[cfg(feature = "cycles")]
#[test]
fn openpbr_channels_change_the_render() {
    let plain = render_box(&box_scene(|m| m.clearcoat_amount = 1.));
    let differs = |other: &[f32]| {
        plain
            .iter()
            .zip(other)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max)
            > 0.01
    };
    let tinted = render_box(&box_scene(|m| {
        m.clearcoat_amount = 1.;
        m.openpbr.coat_color = [1., 0.1, 0.1];
    }));
    assert!(differs(&tinted), "coat colour");
    let film = render_box(&box_scene(|m| {
        m.clearcoat_amount = 1.;
        m.openpbr.thin_film_weight = 1.;
        m.openpbr.thin_film_thickness = 0.5;
    }));
    assert!(differs(&film), "thin film");
    // Thin walled glass with dispersion through the principled glass.
    let glass = render_box(&box_scene(|m| {
        m.transparent_amount = 1.;
        m.refractive_index = 1.5;
        m.more.dispersion = 1.;
        m.openpbr.thin_walled = true;
    }));
    assert!(glass.iter().all(|v| v.is_finite()));
}
