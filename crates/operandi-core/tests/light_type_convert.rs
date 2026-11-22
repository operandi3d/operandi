//! Switching a light's type converts its Intensity so it lights a
//! reference point about as before (LT-Q01). Irradiance here is worked out
//! independently of the conversion: point sources by inverse square, area
//! and cylinder lights by integrating their glowing surface.
use operandi_core::camera_light::{AreaShape, LightKind, LightSettings};

/// Irradiance on a surface facing the light, `d` metres along its axis
/// (side-on for a cylinder, overhead for a dome).
fn irradiance(l: &LightSettings, d: f64) -> f64 {
    let n = 400;
    match l.kind {
        LightKind::Directional => l.intensity,
        LightKind::Point | LightKind::Spot | LightKind::Photometric => l.intensity / (d * d),
        LightKind::Area => {
            // ∫ L cosθ cosθ' / r² dA over the outline (both cosines d / r).
            let (w, h) = (l.width, l.height);
            let mut e = 0.0;
            for i in 0..n {
                for j in 0..n {
                    let x = (i as f64 + 0.5) / n as f64 - 0.5;
                    let y = (j as f64 + 0.5) / n as f64 - 0.5;
                    if l.shape == AreaShape::Elliptical && x * x + y * y > 0.25 {
                        continue;
                    }
                    let (x, y) = (x * w, y * h);
                    let r2 = x * x + y * y + d * d;
                    e += d * d / (r2 * r2) * (w * h / (n * n) as f64);
                }
            }
            l.intensity * e
        }
        LightKind::Cylinder => {
            // Tube along X of radius r, seen from distance d along Y.
            let (r, len) = (l.radius, l.length);
            let mut e = 0.0;
            for i in 0..n {
                for j in 0..n {
                    let x = ((i as f64 + 0.5) / n as f64 - 0.5) * len;
                    let a = (j as f64 + 0.5) / n as f64 * std::f64::consts::TAU;
                    let (py, pz) = (r * a.cos(), r * a.sin());
                    let normal = [0.0, a.cos(), a.sin()];
                    let to = [-x, d - py, -pz];
                    let dist2 = to[0] * to[0] + to[1] * to[1] + to[2] * to[2];
                    let dist = dist2.sqrt();
                    let cos_s = (normal[1] * to[1] + normal[2] * to[2]) / dist;
                    let cos_r = to[1] / dist;
                    if cos_s > 0.0 && cos_r > 0.0 {
                        e += cos_s * cos_r / dist2
                            * (len / n as f64)
                            * (r * std::f64::consts::TAU / n as f64);
                    }
                }
            }
            l.intensity * e
        }
        LightKind::Dome => std::f64::consts::PI * l.intensity,
        LightKind::Mesh | LightKind::Portal => f64::NAN,
    }
}

fn light(kind: LightKind, intensity: f64) -> LightSettings {
    LightSettings {
        intensity,
        ..LightSettings::new(kind)
    }
}

fn close(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance * a.abs().max(b.abs())
}

#[test]
fn point_like_lights_match_at_the_reference_metre() {
    for (from, to) in [
        (LightKind::Directional, LightKind::Point),
        (LightKind::Point, LightKind::Directional),
        (LightKind::Spot, LightKind::Directional),
        (LightKind::Point, LightKind::Spot),
        (LightKind::Photometric, LightKind::Point),
        (LightKind::Dome, LightKind::Directional),
        (LightKind::Directional, LightKind::Dome),
    ] {
        let a = light(from, 7.0);
        let b = a.with_kind(to);
        assert_eq!(b.kind, to);
        assert!(
            close(irradiance(&a, 1.0), irradiance(&b, 1.0), 1e-9),
            "{from:?} -> {to:?}: {} vs {}",
            irradiance(&a, 1.0),
            irradiance(&b, 1.0)
        );
    }
}

#[test]
fn surface_lights_match_where_they_act_as_points() {
    // Far from the light (10 m) the integrated irradiance of an area or
    // cylinder light equals the converted point light's within 1 %.
    let mut area = light(LightKind::Area, 2.0);
    area.width = 0.5;
    area.height = 1.5;
    let mut ellipse = area.clone();
    ellipse.shape = AreaShape::Elliptical;
    let mut tube = light(LightKind::Cylinder, 3.0);
    tube.radius = 0.05;
    tube.length = 1.2;
    for source in [area, ellipse, tube] {
        let point = source.with_kind(LightKind::Point);
        let d = 10.0;
        assert!(
            close(irradiance(&source, d), irradiance(&point, d), 0.01),
            "{:?}: {} vs {}",
            source.kind,
            irradiance(&source, d),
            irradiance(&point, d)
        );
        // And back: the new type's defaults set the size, the brightness
        // still matches.
        let back = point.with_kind(source.kind);
        assert!(
            close(irradiance(&back, d), irradiance(&point, d), 0.01),
            "back to {:?}",
            source.kind
        );
    }
}

#[test]
fn switching_and_back_restores_the_number() {
    let spot = light(LightKind::Spot, 5.0);
    let back = spot
        .with_kind(LightKind::Area)
        .with_kind(LightKind::Directional)
        .with_kind(LightKind::Spot);
    assert!(close(back.intensity, 5.0, 1e-12), "{}", back.intensity);
}

#[test]
fn lights_without_a_comparable_brightness_keep_the_number() {
    for (from, to) in [
        (LightKind::Point, LightKind::Mesh),
        (LightKind::Mesh, LightKind::Area),
        (LightKind::Area, LightKind::Portal),
    ] {
        assert_eq!(
            light(from, 4.0).with_kind(to).intensity,
            4.0,
            "{from:?} -> {to:?}"
        );
    }
}
