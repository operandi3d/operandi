//! The Lasso falloff (painting stage 3, PNT-115..120): a shape drawn on
//! the screen that limits where a tool acts, projected straight into the
//! view. Inside the shape the weight is 1; outside it fades to 0 over Soft
//! Border screen pixels. It mixes with the other falloffs by its Mix Mode.
//!
//! Built so any tool could use it (`Lasso::point_weights` gives per-point
//! weights through a view), but only painting builds compile it for now,
//! and only the paint tools read it (PNT-Q27).
use crate::MixMode;
use crate::paint_stroke::PaintView;
use glam::{DMat4, DVec2};

/// The Style list (PNT-116).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LassoStyle {
    /// Freehand.
    #[default]
    Lasso,
    Rectangle,
    /// From the centre.
    Circle,
    /// From a corner.
    Ellipse,
}

impl LassoStyle {
    pub const ALL: [LassoStyle; 4] = [
        LassoStyle::Lasso,
        LassoStyle::Rectangle,
        LassoStyle::Circle,
        LassoStyle::Ellipse,
    ];
    pub fn label(self) -> &'static str {
        match self {
            LassoStyle::Lasso => "Lasso",
            LassoStyle::Rectangle => "Rectangle",
            LassoStyle::Circle => "Circle",
            LassoStyle::Ellipse => "Ellipse",
        }
    }
    pub fn named(word: &str) -> Option<LassoStyle> {
        LassoStyle::ALL
            .into_iter()
            .find(|s| s.label().eq_ignore_ascii_case(word.trim()))
    }
}

/// A drawn lasso.
#[derive(Clone, Debug, PartialEq)]
pub struct Lasso {
    pub style: LassoStyle,
    /// Screen points as drawn: the freehand path, or the press and release
    /// points of a shape.
    pub points: Vec<DVec2>,
    /// Soft Border, screen pixels (PNT-117).
    pub soft_border: f64,
    pub mix: MixMode,
    /// Use World Transforms (PNT-119): on, the lasso stays fixed to the
    /// model when the view moves (it weighs surface points through `view`,
    /// the view it was drawn in); off, it stays on the screen.
    pub world_transforms: bool,
    /// The view the lasso was drawn in (stage 6).
    pub view: Option<crate::paint_stroke::PaintView>,
}

impl Default for Lasso {
    fn default() -> Self {
        Lasso {
            style: LassoStyle::Lasso,
            points: Vec::new(),
            soft_border: 0.,
            mix: MixMode::default(),
            world_transforms: true,
            view: None,
        }
    }
}

impl Lasso {
    /// The closed outline on screen.
    pub fn polygon(&self) -> Vec<DVec2> {
        let (Some(&a), Some(&b)) = (self.points.first(), self.points.last()) else {
            return Vec::new();
        };
        let ring = |c: DVec2, r: DVec2| -> Vec<DVec2> {
            (0..64)
                .map(|k| {
                    let t = k as f64 / 64. * std::f64::consts::TAU;
                    c + DVec2::new(t.cos() * r.x, t.sin() * r.y)
                })
                .collect()
        };
        match self.style {
            LassoStyle::Lasso => self.points.clone(),
            LassoStyle::Rectangle => vec![a, DVec2::new(b.x, a.y), b, DVec2::new(a.x, b.y)],
            LassoStyle::Circle => ring(a, DVec2::splat(a.distance(b))),
            LassoStyle::Ellipse => ring((a + b) / 2., ((b - a) / 2.).abs()),
        }
    }

    /// The weight at screen point `p`.
    /// The weight at a surface point seen at `screen` now: through the
    /// view it was drawn in when it is fixed to the model (stage 6).
    pub fn weight_for(&self, screen: DVec2, world: glam::DVec3) -> f64 {
        match (&self.view, self.world_transforms) {
            (Some(v), true) => v.project(world).map_or(0., |(q, _)| self.weight(q)),
            _ => self.weight(screen),
        }
    }

    /// The lasso's box in its own screen space (its handles' frame).
    pub fn bounds(&self) -> Option<(DVec2, DVec2)> {
        let pts = self.polygon();
        let first = *pts.first()?;
        Some(
            pts.iter()
                .fold((first, first), |(lo, hi), &p| (lo.min(p), hi.max(p))),
        )
    }

    /// Its handles' edits: moved by `by`, then scaled by `k` about `about`.
    pub fn moved(&self, by: DVec2, about: DVec2, k: f64) -> Lasso {
        let k = if k.is_finite() && k > 0.01 { k } else { 0.01 };
        Lasso {
            points: self
                .points
                .iter()
                .map(|&p| about + (p + by - about) * k)
                .collect(),
            ..self.clone()
        }
    }

    pub fn weight(&self, p: DVec2) -> f64 {
        let poly = self.polygon();
        if poly.len() < 3 {
            return 0.;
        }
        if inside(&poly, p) {
            return 1.;
        }
        if self.soft_border <= 0. {
            return 0.;
        }
        let d = poly
            .iter()
            .zip(poly.iter().cycle().skip(1))
            .map(|(&a, &b)| segment_distance(p, a, b))
            .fold(f64::INFINITY, f64::min);
        (1. - d / self.soft_border).max(0.)
    }

    /// Per point of a mesh (local positions, placed by `world`) seen through
    /// `view`: what a modelling tool would multiply its moves by.
    pub fn point_weights(
        &self,
        points: &[glam::DVec3],
        world: DMat4,
        view: &PaintView,
    ) -> Vec<f64> {
        points
            .iter()
            .map(|&p| {
                view.project(world.transform_point3(p))
                    .map_or(0., |(s, _)| self.weight(s))
            })
            .collect()
    }
}

/// Even-odd point in polygon.
fn inside(poly: &[DVec2], p: DVec2) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
    }
    c
}

fn segment_distance(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let d = b - a;
    let t = if d.length_squared() > 0. {
        ((p - a).dot(d) / d.length_squared()).clamp(0., 1.)
    } else {
        0.
    };
    p.distance(a + d * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_is_full_inside_and_fades_over_the_soft_border() {
        let l = Lasso {
            style: LassoStyle::Rectangle,
            points: vec![DVec2::new(10., 10.), DVec2::new(50., 40.)],
            soft_border: 10.,
            ..Lasso::default()
        };
        assert_eq!(l.weight(DVec2::new(30., 20.)), 1.);
        assert!((l.weight(DVec2::new(55., 20.)) - 0.5).abs() < 1e-9);
        assert_eq!(l.weight(DVec2::new(70., 20.)), 0.);
        let hard = Lasso {
            soft_border: 0.,
            ..l
        };
        assert_eq!(hard.weight(DVec2::new(51., 20.)), 0.);
    }

    #[test]
    fn circle_ellipse_and_freehand() {
        let c = Lasso {
            style: LassoStyle::Circle,
            points: vec![DVec2::ZERO, DVec2::new(10., 0.)],
            ..Lasso::default()
        };
        assert_eq!(c.weight(DVec2::new(5., 5.)), 1.);
        assert_eq!(c.weight(DVec2::new(9., 9.)), 0.);
        let e = Lasso {
            style: LassoStyle::Ellipse,
            points: vec![DVec2::ZERO, DVec2::new(40., 10.)],
            ..Lasso::default()
        };
        assert_eq!(e.weight(DVec2::new(20., 5.)), 1.);
        assert_eq!(e.weight(DVec2::new(20., 12.)), 0.);
        let f = Lasso {
            points: vec![DVec2::ZERO, DVec2::new(20., 0.), DVec2::new(10., 20.)],
            ..Lasso::default()
        };
        assert_eq!(f.weight(DVec2::new(10., 5.)), 1.);
        assert_eq!(f.weight(DVec2::new(30., 30.)), 0.);
    }
}
