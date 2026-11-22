//! USD import, stage U3: UsdGeomCamera prims become camera items, UsdLux
//! lights become lights, and a DomeLight's picture becomes the scene's
//! environment image.
//!
//! The units are the export's (`lights.rs`), which follow Blender's USD
//! exporter, read backwards, so a light keeps the power Cycles gets:
//! - camera lens and film values are tenths of a scene unit;
//! - a normalized light's power is P = π × intensity (whatever its size);
//!   an unnormalized one's is π × intensity × its surface area;
//! - Operandi hands Cycles 4πI for a point or spot light of intensity I,
//!   L × area × π for an area light of radiance L, and the sun strength;
//! - a DistantLight's intensity is the Cycles sun strength ÷ 4 and its
//!   angle half the Spread Angle;
//! - cylinder lights are unnormalized radiance, as the export writes them;
//! - a SphereLight with a ShapingAPI IES file is a photometric light
//!   reading that file (its resolved path, else the written one made
//!   absolute against its layer);
//! - a PortalLight keeps its width and height.
//!
//! Every intensity is × 2^exposure, and a colour temperature that's on
//! tints the colour (`camera_light::kelvin_to_rgb`).
use crate::stage::{CameraData, LightData};
use operandi_core::camera_light::{
    AreaShape, CameraSettings, LightKind, LightSettings, Projection, kelvin_to_rgb,
    new_scene_camera_settings,
};
use std::f64::consts::PI;

/// A UsdGeomCamera as camera settings, lengths scaled by `scale` (metres
/// per scene unit).
pub fn camera_settings(c: &CameraData, scale: f64) -> CameraSettings {
    let mut out = new_scene_camera_settings();
    // Tenths of a scene unit to metres.
    let tenths = |v: f64| v / 10.0 * scale;
    let positive = |v: Option<f64>| v.filter(|v| v.is_finite() && *v > 0.0);
    if let Some(f) = positive(c.focal_length) {
        out.focal_length = tenths(f);
    }
    if let Some(w) = positive(c.horizontal_aperture) {
        out.film_width = tenths(w);
    }
    if let Some(h) = positive(c.vertical_aperture) {
        out.film_height = tenths(h);
    }
    out.film_offset = [
        tenths(c.horizontal_aperture_offset.unwrap_or(0.0)),
        tenths(c.vertical_aperture_offset.unwrap_or(0.0)),
    ];
    // The film's width spans the picture, as USD's apertures do.
    out.film_fit = operandi_core::camera_light::FilmFit::Horizontal;
    if c.projection.as_deref() == Some("orthographic") {
        out.projection = Projection::Orthographic;
        // USD's orthographic apertures are the view's size: with the
        // target at the focal length the film is the view.
        out.target_distance = out.focal_length;
    }
    // USD's own fallback (1 to a million units) means "not set".
    if let Some([near, far]) = c.clipping_range
        && (near, far) != (1.0, 1_000_000.0)
        && near > 0.0
        && far > near
    {
        out.clipping = true;
        // Operandi's clipping stays under a million metres (Houdini writes
        // a far plane of exactly 1,000,000).
        out.clip_far = (far * scale).min(CLIP_LIMIT);
        out.clip_near = (near * scale).min(out.clip_far * 0.5);
    }
    if let Some(f) = positive(c.f_stop) {
        out.dof.enable = true;
        out.dof.f_stop = f.max(1.0);
    }
    if let Some(d) = positive(c.focus_distance) {
        out.focus_distance = d * scale;
    }
    // A picture of the film's shape, when that isn't the default frame.
    let aspect = out.film_width / out.film_height.max(1e-12);
    let [w, h] = operandi_core::camera_light_defaults::RENDER_FRAME;
    if ((w as f64 / h as f64) - aspect).abs() > 0.005 * aspect {
        out.resolution.enable = true;
        out.resolution.width = w;
        out.resolution.height = ((w as f64 / aspect).round() as u32).clamp(1, 32_768);
    }
    // Lens and film values out of Operandi's range (a camera in a file of
    // tiny units): the default camera's, rather than refusing the file.
    if out.validate().is_err() {
        let fallback = new_scene_camera_settings();
        out.focal_length = fallback.focal_length;
        out.film_width = fallback.film_width;
        out.film_height = fallback.film_height;
        out.film_offset = fallback.film_offset;
        out.target_distance = fallback.target_distance;
        out.focus_distance = fallback.focus_distance;
        out.resolution = fallback.resolution.clone();
        if out.validate().is_err() {
            return fallback;
        }
    }
    out
}

/// The farthest clipping plane Operandi takes, metres.
const CLIP_LIMIT: f64 = 999_999.0;

/// What became of a light prim.
pub enum Light {
    Item(Box<LightSettings>),
    /// A DomeLight: the environment.
    Dome,
    /// Not something Operandi has, with the reason.
    Dropped(&'static str),
}

/// The power factor (2^exposure) and colour of a light.
fn colour_and_gain(l: &LightData) -> ([f64; 3], f64) {
    let mut colour = l.color.unwrap_or([1.0; 3]);
    if l.enable_color_temperature == Some(true) {
        let k = kelvin_to_rgb(l.color_temperature.unwrap_or(6500.0));
        for i in 0..3 {
            colour[i] *= k[i];
        }
    }
    let gain = l.intensity.unwrap_or(1.0) * 2f64.powf(l.exposure.unwrap_or(0.0));
    (colour, gain)
}

/// A UsdLux light of schema type `ty` as light settings; sizes scaled by
/// `scale`.
pub fn light_settings(ty: &str, l: &LightData, scale: f64) -> Light {
    let (colour, gain) = colour_and_gain(l);
    let normalize = l.normalize.unwrap_or(false);
    let radius = l.radius.unwrap_or(0.5).max(0.0) * scale;
    let kind = match ty {
        "DistantLight" => LightKind::Directional,
        "SphereLight" => LightKind::Point,
        "RectLight" | "DiskLight" => LightKind::Area,
        "CylinderLight" => LightKind::Cylinder,
        "PortalLight" => LightKind::Portal,
        "DomeLight" | "DomeLight_1" => return Light::Dome,
        "GeometryLight" | "MeshLight" => {
            return Light::Dropped("mesh lights aren't imported yet");
        }
        _ => return Light::Dropped("not a light type Operandi has"),
    };
    let mut out = LightSettings::new(kind);
    out.color = colour;
    out.affect_diffuse = l.diffuse.unwrap_or(1.0).clamp(0.0, 1.0);
    out.affect_specular = l.specular.unwrap_or(1.0).clamp(0.0, 1.0);
    match kind {
        LightKind::Directional => {
            // The Cycles sun strength is 4 × the USD intensity.
            out.sun.enabled = false;
            out.intensity = 4.0 * gain
                / operandi_core::camera_light_defaults::CYCLES_SUN_STRENGTH_PER_INTENSITY;
            out.spread_angle = (2.0 * l.angle.unwrap_or(0.53 / 2.0)).to_radians();
        }
        LightKind::Point => {
            let point = l.treat_as_point == Some(true) || radius == 0.0;
            out.radius = if point { 0.0 } else { radius };
            let power = if normalize || point {
                PI * gain
            } else {
                PI * gain * 4.0 * PI * radius * radius
            };
            out.intensity = power / (4.0 * PI);
            // ShapingAPI's IES profile: a photometric light.
            if let Some(file) = l.ies.as_ref().and_then(|a| {
                [&a.resolved, &a.anchored, &a.value]
                    .into_iter()
                    .flatten()
                    .find(|p| !p.is_empty())
            }) {
                out.kind = LightKind::Photometric;
                out.photometry_file = file.clone();
                out.width = 2.0 * radius;
                out.height = 2.0 * radius;
            }
            // ShapingAPI's cone: a spot light.
            if let Some(half) = l.cone_angle.filter(|a| *a > 0.0 && *a < 90.0) {
                if out.kind != LightKind::Photometric {
                    out.kind = LightKind::Spot;
                }
                out.cone_angle = (2.0 * half).to_radians();
                let h = half.to_radians();
                let b = l.cone_softness.unwrap_or(0.0).clamp(0.0, 1.0);
                // spot_blend = (cos(h − e) − cos h) / (1 − cos h), solved for e.
                let inside = (h.cos() + b * (1.0 - h.cos())).clamp(-1.0, 1.0);
                out.soft_edge = (h - inside.acos()).max(0.0);
            }
        }
        LightKind::Area => {
            let (w, h) = if ty == "DiskLight" {
                out.shape = AreaShape::Elliptical;
                (2.0 * radius, 2.0 * radius)
            } else {
                (
                    l.width.unwrap_or(1.0) * scale,
                    l.height.unwrap_or(1.0) * scale,
                )
            };
            out.width = w;
            out.height = h;
            let area = if ty == "DiskLight" {
                PI * radius * radius
            } else {
                w * h
            };
            // Cycles gets L × area × π; a normalized light's power is
            // π × intensity, an unnormalized one's π × intensity × area.
            out.intensity = if normalize {
                gain / area.max(1e-12)
            } else {
                gain
            };
        }
        LightKind::Cylinder => {
            out.radius = radius;
            let length = l.length.unwrap_or(1.0) * scale;
            out.length = length;
            let surface = 2.0 * PI * radius * length;
            out.intensity = if normalize {
                gain / surface.max(1e-12)
            } else {
                gain
            };
        }
        LightKind::Portal => {
            out.width = l.width.unwrap_or(1.0) * scale;
            out.height = l.height.unwrap_or(1.0) * scale;
        }
        _ => {}
    }
    Light::Item(Box::new(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light(f: impl FnOnce(&mut LightData)) -> LightData {
        let mut l = LightData::default();
        f(&mut l);
        l
    }

    fn item(ty: &str, l: &LightData) -> LightSettings {
        match light_settings(ty, l, 1.0) {
            Light::Item(s) => *s,
            _ => panic!("{ty}"),
        }
    }

    /// Blender's own numbers (fixtures/usd/usd_t1): a 100 W point light
    /// is intensity 100/π normalized; a 200 W 2 m × 2 m area light 200/π;
    /// a strength-3 sun 0.75. Each gives Cycles the same power.
    #[test]
    fn blender_lights_keep_their_cycles_power() {
        let point = item(
            "SphereLight",
            &light(|l| {
                l.intensity = Some(100.0 / PI);
                l.normalize = Some(true);
                l.radius = Some(0.0);
                l.treat_as_point = Some(true);
            }),
        );
        assert_eq!(point.kind, LightKind::Point);
        assert!((4.0 * PI * point.intensity - 100.0).abs() < 1e-9);
        let area = item(
            "RectLight",
            &light(|l| {
                l.intensity = Some(200.0 / PI);
                l.normalize = Some(true);
                l.width = Some(2.0);
                l.height = Some(2.0);
            }),
        );
        assert!((area.intensity * 4.0 * PI - 200.0).abs() < 1e-9);
        let sun = item(
            "DistantLight",
            &light(|l| {
                l.intensity = Some(0.75);
                l.angle = Some(0.263);
            }),
        );
        let strength =
            sun.intensity * operandi_core::camera_light_defaults::CYCLES_SUN_STRENGTH_PER_INTENSITY;
        assert!((strength - 3.0).abs() < 1e-9);
        assert!((sun.spread_angle.to_degrees() - 0.526).abs() < 1e-9);
    }

    #[test]
    fn exposure_temperature_and_spot_cones() {
        let spot = item(
            "SphereLight",
            &light(|l| {
                l.intensity = Some(1.0);
                l.exposure = Some(2.0);
                l.normalize = Some(true);
                l.radius = Some(0.1);
                l.cone_angle = Some(30.0);
                l.cone_softness = Some(0.25);
                l.enable_color_temperature = Some(true);
                l.color_temperature = Some(3000.0);
            }),
        );
        assert_eq!(spot.kind, LightKind::Spot);
        assert!((4.0 * PI * spot.intensity - 4.0 * PI).abs() < 1e-9, "× 2^2");
        assert!((spot.cone_angle.to_degrees() - 60.0).abs() < 1e-9);
        assert!((spot.spot_blend() - 0.25).abs() < 1e-9);
        assert!(spot.color[0] > spot.color[2], "3000 K is warm");
        // Unnormalized: power grows with the area.
        let small = item("RectLight", &light(|l| l.intensity = Some(2.0)));
        assert_eq!(small.intensity, 2.0);
        assert!(matches!(
            light_settings("DomeLight", &LightData::default(), 1.0),
            Light::Dome
        ));
    }

    #[test]
    fn cameras_read_tenths_of_a_unit() {
        // Blender's 35 mm lens on a 36 × 20.25 mm film, metres.
        let c = camera_settings(
            &CameraData {
                focal_length: Some(0.35),
                horizontal_aperture: Some(0.36),
                vertical_aperture: Some(0.2025),
                clipping_range: Some([0.1, 1000.0]),
                ..Default::default()
            },
            1.0,
        );
        assert!((c.focal_length - 0.035).abs() < 1e-12);
        assert!((c.film_width - 0.036).abs() < 1e-12);
        assert!((c.film_height - 0.02025).abs() < 1e-12);
        assert!(c.clipping && (c.clip_near - 0.1).abs() < 1e-12);
        assert!(!c.resolution.enable, "16:9 is the default frame");
        assert!(!c.dof.enable);
        // A square film gets a square picture.
        let square = camera_settings(
            &CameraData {
                horizontal_aperture: Some(0.36),
                vertical_aperture: Some(0.36),
                ..Default::default()
            },
            1.0,
        );
        assert!(square.resolution.enable);
        assert_eq!(square.resolution.width, square.resolution.height);
    }

    /// Houdini's cameras (usd-wg assets' Teapot): a far plane of exactly a
    /// million, which Operandi's camera check refused, failing the import.
    #[test]
    fn a_million_unit_far_plane_is_clamped() {
        let c = camera_settings(
            &CameraData {
                focal_length: Some(0.3),
                horizontal_aperture: Some(0.20955),
                vertical_aperture: Some(0.089807145),
                clipping_range: Some([0.1, 1_000_000.0]),
                ..Default::default()
            },
            1.0,
        );
        assert!(c.validate().is_ok());
        assert!(c.clipping && c.clip_far < 1e6 && (c.clip_near - 0.1).abs() < 1e-12);
        // Lens values far out of range give the default lens, not an error.
        let tiny = camera_settings(
            &CameraData {
                focal_length: Some(1e-12),
                ..Default::default()
            },
            1e-9,
        );
        assert!(tiny.validate().is_ok());
    }
}
