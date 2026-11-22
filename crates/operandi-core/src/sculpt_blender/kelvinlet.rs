// SPDX-FileCopyrightText: 2019 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/blenkernel/intern/kelvinlet.cc` at
// commit b4eed6fd498e76a7e93404bac2326e9f84deba22.

//! Regularized Kelvinlets (de Goes and James, 2017): smooth, volume-aware
//! deformations for the elastic brushes. Displacements fade with distance
//! on their own, so they need no tip falloff.

use glam::DVec3;

/// Material and brush constants of one Kelvinlet deformation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kelvinlet {
    a: f64,
    b: f64,
    c: f64,
    /// Scale and twist strength.
    force: f64,
    radius_scaled: [f64; 3],
}

impl Kelvinlet {
    /// `poisson_ratio` is Blender's Volume Preservation (0..<0.5).
    pub fn new(radius: f64, force: f64, shear_modulus: f64, poisson_ratio: f64) -> Self {
        let a = 1. / (4. * std::f64::consts::PI * shear_modulus);
        let b = a / (4. * (1. - poisson_ratio));
        let c = 2. * (3. * a - 2. * b);
        let r0 = radius;
        Self {
            a,
            b,
            c,
            force,
            radius_scaled: [r0, r0 * 2., r0 * 4.],
        }
    }

    /// Formula (6) of the paper for the first `n` regularization scales.
    fn grab_terms(&self, r: f64, n: usize) -> [f64; 3] {
        let mut k = [0.; 3];
        for (i, k) in k.iter_mut().enumerate().take(n) {
            let eps = self.radius_scaled[i];
            let re = (r * r + eps * eps).sqrt();
            *k = (self.a - self.b) / re
                + (self.b * r * r) / re.powi(3)
                + (self.a * eps * eps) / (2. * re.powi(3));
        }
        k
    }

    /// Grab: `delta` carried along, fading with distance from `location`.
    pub fn grab(&self, p: DVec3, location: DVec3, delta: DVec3) -> DVec3 {
        let k = self.grab_terms(p.distance(location), 1);
        delta * (k[0] * self.c)
    }

    /// Bi-scale grab: falls off faster (less far-field pull).
    pub fn grab_biscale(&self, p: DVec3, location: DVec3, delta: DVec3) -> DVec3 {
        let rs = self.radius_scaled;
        let k = self.grab_terms(p.distance(location), 2);
        let u = k[0] - k[1];
        delta * (u * self.c / (1. / rs[0] - 1. / rs[1]))
    }

    /// Tri-scale grab: compact support, the most local of the three.
    pub fn grab_triscale(&self, p: DVec3, location: DVec3, delta: DVec3) -> DVec3 {
        let rs = self.radius_scaled;
        let k = self.grab_terms(p.distance(location), 3);
        let (r0, r1, r2) = (rs[0] * rs[0], rs[1] * rs[1], rs[2] * rs[2]);
        let w = [1., -((r2 - r0) / (r2 - r1)), (r1 - r0) / (r2 - r1)];
        let u = w[0] * k[0] + w[1] * k[1] + w[2] * k[2];
        let fade = u * self.c / (w[0] / rs[0] + w[1] / rs[1] + w[2] / rs[2]);
        delta * fade
    }

    /// Formula (16): uniform scaling about `location`.
    fn scale_step(&self, p: DVec3, location: DVec3, _normal: DVec3) -> DVec3 {
        let rv = p - location;
        let eps = self.radius_scaled[0];
        let re = (rv.length_squared() + eps * eps).sqrt();
        let u = (2. * self.b - self.a) / re.powi(3) + (3. * eps * eps) / (2. * re.powi(5));
        rv * (u * self.c * self.force)
    }

    /// Formula (15): twisting about `normal` through `location`.
    fn twist_step(&self, p: DVec3, location: DVec3, normal: DVec3) -> DVec3 {
        let rv = p - location;
        let eps = self.radius_scaled[0];
        let re = (rv.length_squared() + eps * eps).sqrt();
        let u = -self.a / re.powi(3) + (3. * eps * eps) / (2. * re.powi(5));
        normal.cross(rv) * (u * self.c * self.force)
    }

    /// Scale, integrated over the step with Runge–Kutta 4.
    pub fn scale(&self, p: DVec3, location: DVec3, normal: DVec3) -> DVec3 {
        self.integrate(Self::scale_step, p, location, normal)
    }

    /// Twist, integrated over the step with Runge–Kutta 4.
    pub fn twist(&self, p: DVec3, location: DVec3, normal: DVec3) -> DVec3 {
        self.integrate(Self::twist_step, p, location, normal)
    }

    fn integrate(
        &self,
        f: fn(&Self, DVec3, DVec3, DVec3) -> DVec3,
        p: DVec3,
        location: DVec3,
        normal: DVec3,
    ) -> DVec3 {
        let k0 = f(self, p, location, normal);
        let k1 = f(self, p + k0 * 0.5, location, normal);
        let k2 = f(self, p + k1 * 0.5, location, normal);
        // Blender evaluates the last stage at (p + k2) − location.
        let k3 = f(self, p + k2 - location, location, normal);
        (k0 + k1 * 2. + k2 * 2. + k3) / 6.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grabs_follow_the_delta_and_fade_with_distance() {
        let k = Kelvinlet::new(1., 0., 1., 0.4);
        let d = DVec3::X;
        for f in [
            Kelvinlet::grab,
            Kelvinlet::grab_biscale,
            Kelvinlet::grab_triscale,
        ] {
            let near = f(&k, DVec3::ZERO, DVec3::ZERO, d);
            let far = f(&k, DVec3::Y * 3., DVec3::ZERO, d);
            assert!(near.x > 0. && near.y == 0. && near.z == 0.);
            assert!(far.x.abs() < near.x);
        }
        // Tri-scale is the most local.
        let p = DVec3::Y * 3.;
        assert!(k.grab_triscale(p, DVec3::ZERO, d).x.abs() < k.grab(p, DVec3::ZERO, d).x.abs());
    }

    #[test]
    fn scale_pushes_out_and_twist_turns_around_the_normal() {
        let k = Kelvinlet::new(1., 1., 1., 0.4);
        let s = k.scale(DVec3::X * 0.5, DVec3::ZERO, DVec3::Z);
        assert!(s.x != 0. && s.y.abs() < 1e-12);
        let t = k.twist(DVec3::X * 0.5, DVec3::ZERO, DVec3::Z);
        assert!(t.y.abs() > 0. && t.z.abs() < 1e-12);
        // No force, no move.
        let none = Kelvinlet::new(1., 0., 1., 0.4);
        assert_eq!(none.scale(DVec3::X, DVec3::ZERO, DVec3::Z), DVec3::ZERO);
    }
}
