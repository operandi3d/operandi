// SPDX-FileCopyrightText: 2009 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/blenkernel/intern/brush.cc`
// (`BKE_brush_curve_strength`) and
// `source/blender/editors/sculpt_paint/mesh/sculpt.cc`
// (`apply_hardness_to_distances`, `calc_brush_cube_distances`) at commit
// b4eed6fd498e76a7e93404bac2326e9f84deba22.

//! Brush falloff: the preset curves, hardness and square tips.

/// Blender's falloff presets (`eBrushCurvePreset`, without Custom).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Curve {
    Smooth,
    Smoother,
    Sphere,
    Root,
    Sharp,
    Linear,
    Pow4,
    InverseSquare,
    Constant,
}

impl Curve {
    pub const ALL: [Curve; 9] = [
        Self::Smooth,
        Self::Smoother,
        Self::Sphere,
        Self::Root,
        Self::Sharp,
        Self::Linear,
        Self::Pow4,
        Self::InverseSquare,
        Self::Constant,
    ];

    /// Strength at `distance` from the centre of a brush `radius` wide:
    /// 1 at the centre (0 for distances outside), following the preset.
    pub fn strength(self, distance: f64, radius: f64) -> f64 {
        if !(radius > 0. && distance < radius) {
            return 0.;
        }
        // p runs 1 at the centre to 0 at the rim.
        let p = 1. - distance.max(0.) / radius;
        match self {
            Self::Sharp => p * p,
            Self::Smooth => 3. * p * p - 2. * p * p * p,
            Self::Smoother => p.powi(3) * (p * (p * 6. - 15.) + 10.),
            Self::Root => p.sqrt(),
            Self::Linear => p,
            Self::Constant => 1.,
            Self::Sphere => (2. * p - p * p).max(0.).sqrt(),
            Self::Pow4 => p.powi(4),
            Self::InverseSquare => p * (2. - p),
        }
    }
}

/// Hardness: a hard brush keeps full strength out to `hardness × radius`
/// and squeezes the curve into the rest. Returns the distance to feed the
/// curve.
pub fn apply_hardness(distance: f64, radius: f64, hardness: f64) -> f64 {
    if hardness <= 0. {
        return distance;
    }
    let threshold = hardness * radius;
    if hardness >= 1. {
        return if distance < threshold { 0. } else { radius };
    }
    if distance < threshold {
        0.
    } else {
        (distance / radius - hardness) / (1. - hardness) * radius
    }
}

/// Distance of a point in a square tip's local space (the tip spans −1..1
/// on each axis): 0 inside the flat core, rising through the rounded
/// corners, 1 at and beyond the edge. `roundness` 1 is a circle, 0 a hard
/// square.
pub fn cube_distance(local: &[f64], roundness: f64) -> f64 {
    let hardness = 1. - roundness;
    if local.iter().any(|c| c.abs() > 1.) {
        return 1.;
    }
    let mut excess_sq = 0.;
    let mut any = false;
    for c in local {
        let e = (c.abs() - hardness).max(0.);
        any |= e > 0.;
        excess_sq += e * e;
    }
    if !any {
        return 0.;
    }
    let rcp = if roundness != 0. { 1. / roundness } else { 0. };
    (excess_sq.sqrt() * rcp).min(1.)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_run_from_one_to_zero() {
        for c in Curve::ALL {
            assert!((c.strength(0., 1.) - 1.).abs() < 1e-12, "{c:?} centre");
            assert_eq!(c.strength(1., 1.), 0., "{c:?} rim");
            assert_eq!(c.strength(2., 1.), 0., "{c:?} outside");
            assert_eq!(c.strength(0.5, 0.), 0., "{c:?} no radius");
            assert!(c.strength(0.3, 1.) >= c.strength(0.7, 1.), "{c:?} falls");
        }
    }

    #[test]
    fn presets_match_their_formulas_halfway() {
        let at = |c: Curve| c.strength(0.5, 2.); // p = 0.75
        let p: f64 = 0.75;
        assert!((at(Curve::Sharp) - p * p).abs() < 1e-12);
        assert!((at(Curve::Smooth) - (3. * p * p - 2. * p.powi(3))).abs() < 1e-12);
        assert!((at(Curve::Root) - p.sqrt()).abs() < 1e-12);
        assert!((at(Curve::Sphere) - (2. * p - p * p).sqrt()).abs() < 1e-12);
        assert!((at(Curve::InverseSquare) - p * (2. - p)).abs() < 1e-12);
        assert!((at(Curve::Pow4) - p.powi(4)).abs() < 1e-12);
        assert_eq!(at(Curve::Constant), 1.);
    }

    #[test]
    fn hardness_keeps_the_core_full() {
        assert_eq!(apply_hardness(0.3, 1., 0.), 0.3);
        assert_eq!(apply_hardness(0.3, 1., 0.5), 0.);
        assert!((apply_hardness(0.75, 1., 0.5) - 0.5).abs() < 1e-12);
        assert_eq!(apply_hardness(0.99, 1., 1.), 0.);
        assert_eq!(apply_hardness(1., 1., 1.), 1.);
        assert_eq!(apply_hardness(0.2, 1., 1.), 0.);
    }

    #[test]
    fn square_tip_distances() {
        // Round tip: the plain distance.
        assert!((cube_distance(&[0.6, 0.], 1.) - 0.6).abs() < 1e-12);
        // Hard square: full inside, 1 outside.
        assert_eq!(cube_distance(&[0.9, 0.9], 0.), 0.);
        assert_eq!(cube_distance(&[1.1, 0.], 0.), 1.);
        // Half round: the flat core reaches 0.5.
        assert_eq!(cube_distance(&[0.4, 0.4], 0.5), 0.);
        assert!((cube_distance(&[0.75, 0.5], 0.5) - 0.5).abs() < 1e-12);
    }
}
