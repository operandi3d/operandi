//! The USD rows that were done but untested (docs/USD_STATUS.md):
//! orthographic and depth-of-field cameras, Disk, Cylinder and Portal
//! lights both ways, and IES files, through OpenUSD.
use crate::plan_tests::box_scene;
use glam::DVec3;
use operandi_core::camera_light::{
    AreaShape, CameraSettings, LightKind, LightSettings, Projection,
};
use operandi_core::interchange::{ExportFormat, Imported};
use operandi_core::scene::Transform;
use operandi_core::{Item, Scene};
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("operandi-usd-rows")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Exports `scene` as .usda; returns its text and the scene read back.
fn round_trip(scene: &Scene, name: &str) -> (String, Imported) {
    let path = scratch(name).join(format!("{name}.usda"));
    crate::export(&path, scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let back = crate::import(&path, &Default::default()).unwrap();
    (text, back)
}

fn place() -> Transform {
    Transform {
        translation: DVec3::new(1.0, 2.0, 3.0),
        ..Default::default()
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-5 * b.abs().max(1.0)
}

fn the_camera(scene: &Scene) -> &Item {
    scene
        .items
        .iter()
        .find(|i| i.camera.is_some())
        .expect("a camera")
}

fn the_light(scene: &Scene) -> &Item {
    scene
        .items
        .iter()
        .find(|i| i.light.is_some())
        .expect("a light")
}

#[test]
fn an_orthographic_camera_keeps_its_view_size() {
    let mut scene = box_scene("Box");
    let mut camera = CameraSettings {
        projection: Projection::Orthographic,
        ..CameraSettings::default()
    };
    camera.target_distance = 7.5;
    let aspect = camera.film_width / camera.film_height;
    let (w, h) = camera.orthographic_half_size(aspect);
    scene.add_camera(camera, place()).unwrap();
    let (text, back) = round_trip(&scene, "ortho");
    assert!(
        text.contains("token projection = \"orthographic\""),
        "{text}"
    );
    // USD's apertures are the view's size, in tenths of a unit.
    assert!(
        text.contains(&format!("float horizontalAperture = {}", (20.0 * w) as f32)),
        "{text}"
    );
    let c = the_camera(&back.scene).camera.as_ref().unwrap();
    assert_eq!(c.projection, Projection::Orthographic);
    let (bw, bh) = c.orthographic_half_size(c.film_width / c.film_height);
    assert!(near(bw, w) && near(bh, h), "{bw} × {bh} vs {w} × {h}");
    let at = the_camera(&back.scene).transform.translation;
    assert!(at.abs_diff_eq(place().translation, 1e-9), "{at}");
}

#[test]
fn depth_of_field_keeps_its_f_stop_and_focus_distance() {
    let mut scene = box_scene("Box");
    let mut camera = CameraSettings::default();
    camera.dof.enable = true;
    camera.dof.f_stop = 2.8;
    camera.focus_distance = 4.25;
    camera.focal_length = 0.085;
    scene.add_camera(camera, place()).unwrap();
    let (text, back) = round_trip(&scene, "dof");
    for want in [
        "float fStop = 2.8",
        "float focusDistance = 4.25",
        "float focalLength = 0.85",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
    let c = the_camera(&back.scene).camera.as_ref().unwrap();
    assert!(c.dof.enable);
    assert!(near(c.dof.f_stop, 2.8), "{}", c.dof.f_stop);
    assert!(near(c.focus_distance, 4.25), "{}", c.focus_distance);
    assert!(near(c.focal_length, 0.085), "{}", c.focal_length);
    // Without depth of field: fStop 0, USD's "no depth of field".
    let mut plain = box_scene("Box");
    plain
        .add_camera(CameraSettings::default(), place())
        .unwrap();
    let (text, back) = round_trip(&plain, "no-dof");
    assert!(text.contains("float fStop = 0\n"), "{text}");
    assert!(!the_camera(&back.scene).camera.as_ref().unwrap().dof.enable);
}

/// A light of `kind` exported and read back: its text and settings.
fn light_round_trip(light: LightSettings, name: &str) -> (String, LightSettings) {
    let mut scene = box_scene("Box");
    scene.add_light(light, place()).unwrap();
    let (text, back) = round_trip(&scene, name);
    let item = the_light(&back.scene);
    assert!(
        item.transform
            .translation
            .abs_diff_eq(place().translation, 1e-9)
    );
    (text, item.light.clone().unwrap())
}

#[test]
fn a_round_area_light_is_a_disk_light_both_ways() {
    let mut disk = LightSettings::new(LightKind::Area);
    disk.shape = AreaShape::Elliptical;
    disk.width = 0.6;
    disk.height = 0.6;
    disk.intensity = 3.0;
    disk.dissolve = 0.0;
    disk.color = [1.0, 0.5, 0.25];
    let (text, back) = light_round_trip(disk.clone(), "disk");
    for want in [
        "def DiskLight ",
        "float inputs:radius = 0.3",
        "bool inputs:normalize = 1",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
    assert_eq!(
        (back.kind, back.shape),
        (LightKind::Area, AreaShape::Elliptical)
    );
    assert!(near(back.width, 0.6) && near(back.height, 0.6));
    // The same radiance, so Cycles gets the same power.
    assert!(near(back.intensity, 3.0), "{}", back.intensity);
    assert!(back.color.iter().zip(disk.color).all(|(a, b)| near(*a, b)));
}

#[test]
fn a_cylinder_light_keeps_its_size_and_radiance() {
    let mut cylinder = LightSettings::new(LightKind::Cylinder);
    cylinder.radius = 0.05;
    cylinder.length = 1.5;
    cylinder.intensity = 12.0;
    cylinder.dissolve = 0.25;
    let (text, back) = light_round_trip(cylinder, "cylinder");
    for want in [
        "def CylinderLight ",
        "float inputs:length = 1.5",
        "float inputs:radius = 0.05",
        "float inputs:intensity = 9",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
    assert_eq!(back.kind, LightKind::Cylinder);
    assert!(near(back.radius, 0.05) && near(back.length, 1.5));
    // Dissolve is baked into the intensity: 12 × 0.75.
    assert!(near(back.intensity, 9.0), "{}", back.intensity);
}

#[test]
fn a_portal_light_keeps_its_size() {
    let mut portal = LightSettings::new(LightKind::Portal);
    portal.width = 2.0;
    portal.height = 1.25;
    let (text, back) = light_round_trip(portal, "portal");
    for want in [
        "def PortalLight ",
        "float inputs:width = 2",
        "float inputs:height = 1.25",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
    assert_eq!(back.kind, LightKind::Portal);
    assert!(
        near(back.width, 2.0) && near(back.height, 1.25),
        "{} × {}",
        back.width,
        back.height
    );
}

#[test]
fn a_photometric_light_writes_and_reads_its_ies_file() {
    let dir = scratch("ies-file");
    let ies = dir.join("downlight.ies");
    std::fs::write(&ies, "IESNA:LM-63-2002\nTILT=NONE\n").unwrap();
    let mut light = LightSettings::new(LightKind::Photometric);
    light.photometry_file = ies.display().to_string();
    light.intensity = 2.0;
    light.dissolve = 0.0;
    let (text, back) = light_round_trip(light, "ies");
    for want in [
        "def SphereLight ",
        "prepend apiSchemas = [\"ShapingAPI\"]",
        "asset inputs:shaping:ies:file = @",
        "downlight.ies@",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
    assert_eq!(back.kind, LightKind::Photometric);
    assert_eq!(
        std::path::Path::new(&back.photometry_file).file_name(),
        ies.file_name()
    );
    assert!(std::path::Path::new(&back.photometry_file).is_file());
    assert!(near(
        4.0 * std::f64::consts::PI * back.intensity,
        8.0 * std::f64::consts::PI
    ));
}

/// OpenUSD crashes reading the animation of a stage whose
/// timeCodesPerSecond is 0 (found by the usd-wg assets sweep); the import
/// refuses such a file instead of taking the app down.
#[test]
fn a_zero_time_codes_per_second_file_is_refused_not_a_crash() {
    let path = scratch("tcps").join("tcps0.usda");
    std::fs::write(
        &path,
        r#"#usda 1.0
(
    timeCodesPerSecond = 0
    upAxis = "Y"
)

def Xform "Cube"
{
    double3 xformOp:translate.timeSamples = {
        0: (0, 0, 0),
        10: (1, 0, 0),
    }
    uniform token[] xformOpOrder = ["xformOp:translate"]

    def Mesh "Mesh"
    {
        int[] faceVertexCounts = [3]
        int[] faceVertexIndices = [0, 1, 2]
        point3f[] points = [(0, 0, 0), (1, 0, 0), (0, 1, 0)]
    }
}
"#,
    )
    .unwrap();
    let e = crate::import(&path, &Default::default()).unwrap_err();
    assert!(e.contains("timeCodesPerSecond"), "{e}");
}
