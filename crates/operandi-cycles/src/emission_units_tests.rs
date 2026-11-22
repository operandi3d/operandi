//! LXP-B02: any surface of radiance L renders pixel L seen straight on,
//! with no exposure or tone mapping, whatever made it: a Modo material
//! (Luminous Intensity is render-unit radiance, 1:1, OPM-Q09), a mesh or
//! cylinder light (Emission strength L, no π, LXP-B02a), or an area light
//! (power L·area·π, which Cycles shows as radiance L).
#![cfg(feature = "cycles")]
use super::*;
use operandi_core::camera_light::LightSettings;
use operandi_core::mesh::Mesh;

/// A 1 m square at z = 0.5 facing +Z (the top of a unit box).
fn square(scene: &mut Scene) -> u64 {
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let vertices = item.mesh.vertices.clone();
    item.mesh
        .faces
        .retain(|f| f.vertices.iter().all(|&v| vertices[v as usize].z > 0.4));
    item.id
}

/// The centre pixel seen from `eye` looking along `forward` (Y up), lit
/// only by `lights`: no sun, no world.
fn centre_from(
    meshes: &[RenderMesh],
    eye: DVec3,
    forward: DVec3,
    lights: Vec<SceneLight>,
) -> [f32; 3] {
    let right = forward.cross(DVec3::Y);
    let mut settings =
        Settings::studio(8, 8, View::perspective(eye, right, DVec3::Y, forward, 0.05));
    settings.sun_strength = 0.0;
    settings.world[0].intensity = 0.0;
    settings.lights = Some(lights);
    settings.samples = 16;
    settings.gpu = false;
    settings.denoise = false;
    let img = render(meshes, &settings).unwrap();
    let p = (4 * 8 + 4) * 4;
    [img.rgba[p], img.rgba[p + 1], img.rgba[p + 2]]
}

/// Looking straight down -Z at the square.
fn centre(meshes: &[RenderMesh]) -> [f32; 3] {
    centre_from(meshes, DVec3::new(0.0, 0.0, 5.5), -DVec3::Z, Vec::new())
}

fn within_one_percent(got: [f32; 3], want: f32) -> bool {
    got.iter().all(|v| (v - want).abs() <= 0.01 * want)
}

#[test]
fn a_material_of_radiance_one_renders_pixel_one() {
    use operandi_core::lxo::Value;
    use operandi_core::lxo::preset_items::PresetItem;
    // A Modo material of Luminous Intensity 1, white, nothing else.
    let mut item = PresetItem {
        kind: "advancedMaterial".into(),
        ..Default::default()
    };
    for (k, v) in [
        ("radiance", 1.0),
        ("diffAmt", 0.0),
        ("specAmt", 0.0),
        ("reflAmt", 0.0),
        ("lumiCol.R", 1.0),
        ("lumiCol.G", 1.0),
        ("lumiCol.B", 1.0),
    ] {
        item.channels.insert(k.into(), Value::Float(v));
    }
    let material = operandi_core::preset::lxp::material_channels(&item, &mut Vec::new());
    assert!((material.luminous_intensity - 1.0).abs() < 1e-9);
    let mut scene = Scene::default();
    square(&mut scene);
    if let Some(NodeKind::Material(m)) = scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        **m = material;
    }
    let got = centre(&scene_meshes(&scene).unwrap());
    assert!(within_one_percent(got, 1.0), "{got:?}");
}

#[test]
fn a_mesh_light_of_the_same_surface_matches() {
    let mut scene = Scene::default();
    let proto = square(&mut scene);
    let mut light = LightSettings::new(LightKind::Mesh);
    light.intensity = 1.0;
    light.color = [1.0; 3];
    light.prototype = Some(proto);
    light.visible_to_camera = true;
    scene.add_light(light, Default::default()).unwrap();
    let got = centre(&scene_meshes(&scene).unwrap());
    assert!(within_one_percent(got, 1.0), "{got:?}");
}

#[test]
fn an_area_light_of_radiance_one_renders_pixel_one() {
    // A 1 m area light at the origin shines down local -Z; look up at it.
    let mut scene = Scene::default();
    let mut light = LightSettings::new(LightKind::Area);
    light.intensity = 1.0;
    light.color = [1.0; 3];
    light.width = 1.0;
    light.height = 1.0;
    light.visible_to_camera = true;
    // Cycles only intersects a lamp with MIS on, which Visible to
    // Reflection turns on (see the bridge's light sync).
    light.stored.visible_to_reflection = true;
    scene.add_light(light, Default::default()).unwrap();
    let got = centre_from(
        &scene_meshes(&scene).unwrap(),
        DVec3::new(0.0, 0.0, -5.0),
        DVec3::Z,
        scene_lights(&scene),
    );
    assert!(within_one_percent(got, 1.0), "{got:?}");
}
