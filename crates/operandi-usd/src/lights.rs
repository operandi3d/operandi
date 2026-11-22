//! Cameras, lights and the environment as USD prims (USD stage 3).
//!
//! The quantities follow Blender's USD exporter, fed with the same physical
//! quantities Operandi hands Cycles (operandi-cycles `light_power`):
//! - cameras: USD's lens and film values are in tenths of a scene unit
//!   (metres here), so millimetres-in-metres × 10;
//! - point, spot and photometric lights: power P = 4πI, written as P / π
//!   with `normalize` on, as Blender writes a light of P watts;
//! - area lights: radiance L over area A, written as L × A with
//!   `normalize` on (power π L A, what Cycles gets);
//! - directional lights: the Cycles sun strength S, written as S / 4 (the
//!   factor Blender uses to match other renderers);
//! - cylinder and dome lights: their radiance, unnormalised.
//!
//! USD lights shine down their local −Z, as Operandi's do, and an area
//! light lies in its local XY plane, so the item's own transform places
//! them unchanged. The Environment becomes a DomeLight whose lat-long
//! picture is baked from the environment's own colours.
use glam::DVec3;
use operandi_core::Scene;
use operandi_core::camera_light::{
    AreaShape, CameraSettings, LightKind, LightSettings, Projection,
};

/// `OU_LIGHT_*`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum LightType {
    Distant = 0,
    Sphere = 1,
    Rect = 2,
    Disk = 3,
    Cylinder = 4,
    Portal = 5,
    Dome = 6,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraPrim {
    pub orthographic: bool,
    /// Tenths of a metre, as USD's camera schema wants.
    pub focal_length: f32,
    pub horizontal_aperture: f32,
    pub vertical_aperture: f32,
    pub horizontal_offset: f32,
    pub vertical_offset: f32,
    /// Metres.
    pub clipping: [f32; 2],
    /// 0: no depth of field.
    pub f_stop: f32,
    pub focus_distance: f32,
    /// The scene's render camera: the render settings name it.
    pub render: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightPrim {
    pub kind: LightType,
    pub intensity: f32,
    pub color: [f32; 3],
    pub diffuse: f32,
    pub specular: f32,
    pub normalize: bool,
    pub radius: f32,
    pub width: f32,
    pub height: f32,
    pub length: f32,
    /// Distant: the angular size, degrees.
    pub angle: f32,
    /// Spot: shaping cone half-angle (degrees) and softness; 0 = no cone.
    pub cone_angle: f32,
    pub cone_softness: f32,
    /// Photometric: the IES file.
    pub ies_file: String,
    /// Dome: the baked environment picture (written as a file).
    pub dome: Option<DomePicture>,
}

/// A lat-long picture, OpenEXR's convention (the one UsdLux names):
/// rows top (+Y) to bottom, x from longitude +π to −π, longitude 0 at +Z.
#[derive(Clone, Debug, PartialEq)]
pub struct DomePicture {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[f32; 3]>,
}

/// The direction of lat-long pixel (x, y)'s centre.
pub fn latlong_direction(x: usize, y: usize, width: usize, height: usize) -> DVec3 {
    use std::f64::consts::{FRAC_PI_2, PI};
    let longitude = PI - 2.0 * PI * (x as f64 + 0.5) / width as f64;
    let latitude = FRAC_PI_2 - PI * (y as f64 + 0.5) / height as f64;
    DVec3::new(
        latitude.cos() * longitude.sin(),
        latitude.sin(),
        latitude.cos() * longitude.cos(),
    )
}

pub fn camera_prim(
    scene: &Scene,
    id: u64,
    camera: &CameraSettings,
    notes: &mut Vec<String>,
) -> CameraPrim {
    let tenths = |m: f64| (m * 10.0) as f32;
    let orthographic = camera.projection == Projection::Orthographic;
    if matches!(
        camera.projection,
        Projection::Cylindrical | Projection::CylindricalVr
    ) {
        notes.push("Cylindrical cameras are written as perspective".into());
    }
    let (width, height) = if orthographic {
        // USD's orthographic apertures are the view's size, in tenths.
        let aspect = camera.film_width / camera.film_height.max(1e-12);
        let (w, h) = camera.orthographic_half_size(aspect);
        (2.0 * w, 2.0 * h)
    } else {
        (camera.film_width, camera.film_height)
    };
    let clip = if camera.clipping {
        [camera.clip_near as f32, camera.clip_far as f32]
    } else {
        // USD's own default.
        [1.0, 1_000_000.0]
    };
    CameraPrim {
        orthographic,
        focal_length: tenths(camera.focal_length),
        horizontal_aperture: tenths(width),
        vertical_aperture: tenths(height),
        horizontal_offset: tenths(camera.film_offset[0]),
        vertical_offset: tenths(camera.film_offset[1]),
        clipping: clip,
        f_stop: if camera.dof.enable {
            camera.dof.f_stop as f32
        } else {
            0.0
        },
        focus_distance: scene
            .camera_focus_distance(id)
            .unwrap_or(camera.focus_distance) as f32,
        render: false,
    }
}

/// A light as USD, or `None` for one USD can't hold (mesh lights).
pub fn light_prim(light: &LightSettings, notes: &mut Vec<String>) -> Option<LightPrim> {
    use std::f64::consts::PI;
    let keep = 1.0 - light.dissolve.clamp(0.0, 1.0);
    let mut out = LightPrim {
        kind: LightType::Sphere,
        intensity: 0.0,
        color: light.color.map(|c| c as f32),
        diffuse: light.affect_diffuse as f32,
        specular: light.affect_specular as f32,
        normalize: true,
        radius: light.radius as f32,
        width: light.width as f32,
        height: light.height as f32,
        length: light.length as f32,
        angle: 0.0,
        cone_angle: 0.0,
        cone_softness: 0.0,
        ies_file: String::new(),
        dome: None,
    };
    match light.kind {
        LightKind::Directional => {
            let strength = if light.sun.enabled {
                light.sun.strength(light.intensity)
            } else {
                light.intensity
                    * operandi_core::camera_light_defaults::CYCLES_SUN_STRENGTH_PER_INTENSITY
            };
            out.kind = LightType::Distant;
            out.intensity = (strength * keep / 4.0) as f32;
            out.angle = (light.spread_angle.to_degrees() / 2.0) as f32;
        }
        LightKind::Point | LightKind::Spot | LightKind::Photometric => {
            out.kind = LightType::Sphere;
            out.intensity = (4.0 * PI * light.intensity * keep / PI) as f32;
            let spot = light.kind == LightKind::Spot
                || (light.kind == LightKind::Photometric && light.cone_angle < PI - 1e-6);
            if spot {
                out.cone_angle = (light.cone_angle.to_degrees() / 2.0) as f32;
                out.cone_softness = light.spot_blend() as f32;
            }
            if light.kind == LightKind::Photometric {
                out.ies_file = light.photometry_file.clone();
                out.radius = (0.5 * light.width.max(light.height)) as f32;
            }
        }
        LightKind::Area => {
            // Cycles gets radiance × area × π; a normalized USD light's
            // power is π × intensity, so the intensity is radiance × area
            // (Blender writes P / π). Fixed 2026-11-18: this used to write
            // the radiance alone, too dim by the area.
            let area = match light.shape {
                AreaShape::Elliptical => PI / 4.0 * light.width * light.height,
                AreaShape::Rectangular => light.width * light.height,
            };
            out.intensity = (light.intensity * area * keep) as f32;
            if light.shape == AreaShape::Elliptical {
                // An ellipse light deteriorates into a disk light (Blender).
                out.kind = LightType::Disk;
                out.radius = ((light.width + light.height) / 4.0) as f32;
                if (light.width - light.height).abs() > 1e-9 {
                    notes.push("Elliptical area lights are written as round disks".into());
                }
            } else {
                out.kind = LightType::Rect;
            }
        }
        LightKind::Cylinder => {
            out.kind = LightType::Cylinder;
            out.intensity = (light.intensity * keep) as f32;
            out.normalize = false;
        }
        LightKind::Portal => {
            out.kind = LightType::Portal;
            out.intensity = 1.0;
        }
        LightKind::Dome => {
            out.kind = LightType::Dome;
            out.intensity = (light.intensity * keep) as f32;
            out.normalize = false;
        }
        // On their mesh's prim (mesh_lights.rs), which reports the rest.
        LightKind::Mesh => return None,
    }
    Some(out)
}

/// The environment the scene lights with, baked into a DomeLight.
/// `turn` takes the dome's own frame (pole +Y) to the scene's.
pub fn environment_dome(scene: &Scene, turn: glam::DMat3) -> Option<LightPrim> {
    let kind = operandi_core::environment::RayKind::Indirect;
    scene.environment_for(kind)?;
    // Small: a lat-long sky has no detail at this level, and the picture
    // only lights the scene elsewhere. 256 × 128 like Blender's world bake
    // previews; image layers on the environment lose their detail (noted).
    let (width, height) = (256, 128);
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let d = turn * latlong_direction(x, y, width, height);
            pixels.push(scene.environment_radiance(kind, d).map(|c| c as f32));
        }
    }
    Some(LightPrim {
        kind: LightType::Dome,
        intensity: 1.0,
        color: [1.0; 3],
        diffuse: 1.0,
        specular: 1.0,
        normalize: false,
        radius: 0.0,
        width: 0.0,
        height: 0.0,
        length: 0.0,
        angle: 0.0,
        cone_angle: 0.0,
        cone_softness: 0.0,
        ies_file: String::new(),
        dome: Some(DomePicture {
            width,
            height,
            pixels,
        }),
    })
}

/// Radiance RGBE (.hdr) bytes of a picture, uncompressed scanlines.
pub fn hdr_bytes(picture: &DomePicture) -> Vec<u8> {
    let mut out = format!(
        "#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {} +X {}\n",
        picture.height, picture.width
    )
    .into_bytes();
    for p in &picture.pixels {
        let v = p[0].max(p[1]).max(p[2]);
        if v.is_nan() || v <= 1e-32 || !v.is_finite() {
            out.extend([0, 0, 0, 0]);
            continue;
        }
        // frexp: v = m × 2^e with m in [0.5, 1).
        let e = v.log2().floor() as i32 + 1;
        let scale = 256.0 / 2f32.powi(e);
        out.extend([
            (p[0].max(0.0) * scale).min(255.0) as u8,
            (p[1].max(0.0) * scale).min(255.0) as u8,
            (p[2].max(0.0) * scale).min(255.0) as u8,
            (e + 128).clamp(0, 255) as u8,
        ]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latlong_matches_openexr_and_usdlux() {
        // Centre column, middle row: longitude 0 looks down +Z.
        let d = latlong_direction(128, 64, 257, 129);
        assert!(d.abs_diff_eq(DVec3::Z, 1e-9), "{d}");
        // A quarter from the left: longitude +π/2 is +X.
        let d = latlong_direction(64, 64, 257, 129);
        assert!((d - DVec3::X).length() < 0.03, "{d}");
        // Top row is the +Y pole.
        assert!(latlong_direction(10, 0, 256, 128).y > 0.99);
    }

    #[test]
    fn hdr_pixels_round_trip_within_rgbe_precision() {
        let picture = DomePicture {
            width: 2,
            height: 1,
            pixels: vec![[0.5, 0.25, 1.0], [0.0; 3]],
        };
        let bytes = hdr_bytes(&picture);
        assert!(bytes.starts_with(b"#?RADIANCE\n"));
        let data = &bytes[bytes.len() - 8..];
        let e = data[3] as i32 - 128;
        let back = |m: u8| (m as f32 + 0.5) * 2f32.powi(e - 8);
        assert!((back(data[0]) - 0.5).abs() < 0.01);
        assert!((back(data[2]) - 1.0).abs() < 0.01);
        assert_eq!(&data[4..], [0, 0, 0, 0]);
    }

    #[test]
    fn light_power_follows_blenders_usd_units() {
        let mut point = LightSettings::new(LightKind::Point);
        point.intensity = 2.0;
        point.dissolve = 0.5;
        let p = light_prim(&point, &mut Vec::new()).unwrap();
        // 4πI × (1 − Dissolve) / π.
        assert!((p.intensity - 4.0).abs() < 1e-6);
        assert!(p.normalize);
        let mut sun = LightSettings::new(LightKind::Directional);
        sun.intensity = 3.0;
        sun.sun.enabled = false;
        let s = light_prim(&sun, &mut Vec::new()).unwrap();
        assert_eq!(s.kind, LightType::Distant);
        assert!((s.intensity - 0.75).abs() < 1e-6);
        // A 2 m × 2 m area light of radiance 5: 20, normalized.
        let mut area = LightSettings::new(LightKind::Area);
        area.intensity = 5.0;
        area.width = 2.0;
        area.height = 2.0;
        area.dissolve = 0.0;
        let a = light_prim(&area, &mut Vec::new()).unwrap();
        assert!((a.intensity - 20.0).abs() < 1e-5 && a.normalize);
        // Reading it back gives the same radiance.
        let back = crate::import_lights::light_settings(
            "RectLight",
            &crate::stage::LightData {
                intensity: Some(a.intensity as f64),
                normalize: Some(true),
                width: Some(2.0),
                height: Some(2.0),
                ..Default::default()
            },
            1.0,
        );
        let crate::import_lights::Light::Item(back) = back else {
            panic!()
        };
        assert!((back.intensity - 5.0).abs() < 1e-5);
    }
}
