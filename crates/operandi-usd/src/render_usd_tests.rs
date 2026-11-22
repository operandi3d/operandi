//! Render settings through USD (usd-gaps-3): the Render item goes out as
//! UsdRender RenderSettings and a RenderProduct and comes back the same.
use operandi_core::Scene;
use operandi_core::camera_light::CameraSettings;
use operandi_core::interchange::ExportFormat;
use operandi_core::mesh::Mesh;
use operandi_core::render_settings::Caustics;
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-usd-rs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn scene() -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().mesh = Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
        .unwrap()
        .into();
    scene
}

#[test]
fn the_render_item_round_trips_as_render_settings() {
    let mut scene = scene();
    scene
        .add_camera(CameraSettings::default(), Default::default())
        .unwrap();
    let mut wide = CameraSettings::default();
    wide.resolution.enable = true;
    wide.resolution.width = 800;
    wide.resolution.height = 600;
    let render_cam = scene.add_camera(wide, Default::default()).unwrap();
    scene.items.last_mut().unwrap().name = "Wide".into();
    scene.set_render_camera(render_cam).unwrap();
    scene.render.frame = [1280, 720];
    scene.render.samples = 333;
    scene.render.adaptive_threshold = 0.025;
    scene.render.diffuse_depth = 5;
    scene.render.refraction_depth = 11;
    scene.render.max_radiance = 0.0;
    scene.render.caustics = Caustics::ReflectionOnly;
    scene.render.preview_time_limit = 2.5;
    scene.render.direct_caustics.enabled = true;

    let dir = scratch("rt");
    let out = dir.join("render.usda");
    crate::export(&out, &scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    for expected in [
        "renderSettingsPrimPath = \"/Render/Settings\"",
        "def RenderSettings \"Settings\"",
        "int2 resolution = (1280, 720)",
        "float pixelAspectRatio = 1",
        "rel camera = </root/Wide/Wide>",
        "def RenderProduct \"Final\"",
        "int2 resolution = (800, 600)",
        "int samples = 333",
    ] {
        assert!(text.contains(expected), "missing {expected}:\n{text}");
    }

    let back = crate::import(&out, &Default::default()).unwrap();
    assert_eq!(back.scene.render, scene.render, "{:#?}", back.notes);
    let cam = back.scene.render_camera().unwrap();
    let cam = back.scene.items.iter().find(|i| i.id == cam).unwrap();
    assert_eq!(cam.name, "Wide");
    let r = &cam.camera.as_ref().unwrap().resolution;
    assert!(r.enable);
    assert_eq!((r.width, r.height), (800, 600));
    assert!(
        !back.notes.iter().any(|n| n.starts_with("Render settings")),
        "{:?}",
        back.notes
    );
}

/// Another program's settings: the resolution comes in, the rest keeps
/// Operandi's own, and the report says so.
#[test]
fn other_programs_render_settings_give_the_frame() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/usd/render_camera.usda");
    let text = std::fs::read_to_string(&fixture).unwrap();
    let dir = scratch("foreign");
    let file = dir.join("foreign.usda");
    let text = text.replacen(
        "def RenderSettings \"Settings\"\n    {",
        "def RenderSettings \"Settings\"\n    {\n        uniform int2 resolution = (640, 480)\n        uniform float pixelAspectRatio = 2",
        1,
    );
    assert!(text.contains("(640, 480)"), "the fixture changed:\n{text}");
    std::fs::write(&file, text).unwrap();
    let back = crate::import(&file, &Default::default()).unwrap();
    assert_eq!(back.scene.render.frame, [640, 480]);
    let notes = back.notes.join("\n");
    assert!(
        notes.contains("pixel aspect 2 read as square pixels"),
        "{notes}"
    );
    assert!(
        notes.contains("Render settings from /Render/Settings"),
        "{notes}"
    );
}
