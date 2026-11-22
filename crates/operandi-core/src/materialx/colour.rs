//! MaterialX colour spaces for constants (MX-03): every colour becomes
//! linear Rec.709, Operandi's working space. Matrices are the standard
//! ones for these primaries (Bradford-adapted to D65 where the white
//! point differs), as MaterialX's colour-management notes give them.

/// `c` (in `space`) as linear Rec.709, `None` for a space this doesn't
/// know (then the value is used as it is, with a note).
pub fn to_lin_rec709(c: [f64; 3], space: &str) -> Option<[f64; 3]> {
    const AP1: [[f64; 3]; 3] = [
        [1.705_051, -0.621_792, -0.083_259],
        [-0.130_256, 1.140_805, -0.010_548],
        [-0.024_003, -0.128_969, 1.152_972],
    ];
    const AP0: [[f64; 3]; 3] = [
        [2.521_686, -1.134_131, -0.387_555],
        [-0.276_479, 1.372_719, -0.096_240],
        [-0.015_378, -0.152_975, 1.168_353],
    ];
    const P3: [[f64; 3]; 3] = [
        [1.224_940, -0.224_940, 0.0],
        [-0.042_057, 1.042_057, 0.0],
        [-0.019_638, -0.078_636, 1.098_274],
    ];
    let m = |m: &[[f64; 3]; 3], c: [f64; 3]| {
        [0, 1, 2].map(|r| m[r][0] * c[0] + m[r][1] * c[1] + m[r][2] * c[2])
    };
    let g = |p: f64| c.map(|v: f64| v.max(0.).powf(p));
    Some(match space {
        "lin_rec709" | "lin_srgb" | "raw" | "none" | "" => c,
        "srgb_texture" => c.map(srgb_decode),
        "g22_rec709" => g(2.2),
        "g18_rec709" => g(1.8),
        "acescg" | "lin_ap1" => m(&AP1, c),
        "g22_ap1" => m(&AP1, g(2.2)),
        "lin_ap0" | "aces2065-1" => m(&AP0, c),
        "lin_displayp3" => m(&P3, c),
        "srgb_displayp3" => m(&P3, c.map(srgb_decode)),
        _ => return None,
    })
}

/// Whether pictures in `space` hold sRGB-encoded values (what Operandi
/// reads colour maps as).
pub fn is_srgb_encoded(space: &str) -> bool {
    matches!(
        space,
        "srgb_texture" | "srgb_displayp3" | "g22_rec709" | "g22_ap1"
    )
}

fn srgb_decode(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_stays_white_and_spaces_convert() {
        for space in [
            "lin_rec709",
            "srgb_texture",
            "acescg",
            "lin_displayp3",
            "lin_ap0",
        ] {
            let w = to_lin_rec709([1.; 3], space).unwrap();
            assert!(w.iter().all(|v| (v - 1.).abs() < 1e-4), "{space}: {w:?}");
        }
        let half = to_lin_rec709([0.5; 3], "srgb_texture").unwrap();
        assert!((half[0] - 0.214_04).abs() < 1e-4);
        // ACEScg red is more saturated than Rec.709 red.
        let red = to_lin_rec709([1., 0., 0.], "acescg").unwrap();
        assert!(red[0] > 1.6 && red[1] < 0.);
        assert!(to_lin_rec709([1.; 3], "mystery").is_none());
    }
}
