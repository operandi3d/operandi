//! New scene against `specifications/new-scene-2026-11-05`: one empty
//! mesh "Mesh" (selected), the camera and the directional light at their
//! default places with their default settings. The app's new document
//! builds the same scene and switches to Item mode (checked in the app's
//! new-scene tests).
use operandi_core::camera_light::*;
use operandi_core::*;

#[test]
fn new_scene_contents_transforms_and_settings() {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.add_default_camera_and_light().unwrap();
    let names: Vec<_> = scene.items.iter().map(|i| i.name.as_str()).collect();
    // ENV §1: and the default Environment.
    assert_eq!(
        names,
        ["Mesh", "Camera", "Directional Light", "Environment"]
    );
    let mesh = &scene.items[0];
    assert!(mesh.mesh.vertices.is_empty() && mesh.mesh.faces.is_empty());
    assert_eq!(mesh.transform, scene::Transform::default());
    assert_eq!(scene.selection.item, Some(mesh.id));
    assert!(scene.items.iter().all(|i| i.parent.is_none()));

    let near = |a: glam::DVec3, b: [f64; 3]| (a - glam::DVec3::from_array(b)).length() < 1e-12;
    let camera = &scene.items[1];
    assert!(near(camera.transform.translation, [0., 0.75, 4.]));
    assert!(near(
        camera.transform.rotation,
        [-0.08726646259971647, 0., 0.]
    ));
    let c = camera.camera.as_ref().unwrap();
    assert_eq!(
        (c.focal_length, c.film_width, c.film_height),
        (0.05, 0.036, 0.024)
    );
    assert_eq!(c.projection, Projection::Perspective);

    let light = &scene.items[2];
    assert!(near(light.transform.translation, [-2., 2., 2.]));
    assert!(near(
        light.transform.rotation,
        [-0.5235987755982988, -std::f64::consts::FRAC_PI_4, 0.]
    ));
    assert!(light.transform.zxy, "rotation order ZXY");
    let l = light.light.as_ref().unwrap();
    assert_eq!(l.kind, LightKind::Directional);
    assert_eq!(l.intensity, 3.);
    assert_eq!(l.shadow, ShadowType::RayTraced);
    assert_eq!(
        operandi_core::camera_light_defaults::RENDER_FRAME,
        [1920, 1080]
    );
}
