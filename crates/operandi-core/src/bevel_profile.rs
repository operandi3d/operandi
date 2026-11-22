//! The two admitted original shapes. This is a bounded shape library, not an
//! arbitrary adaptive curve editor (BV-Q02). Samples describe shape coordinates,
//! not mesh output; the contour kernel supplies topology and correspondence.
use crate::{BevelOptions, Result};
use glam::DVec2;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BevelProfile {
    Linear,
    Smooth,
}
impl BevelProfile {
    pub fn aspect(self) -> f64 {
        match self {
            Self::Linear => 1.,
            Self::Smooth => 1. / (1.01393 + 0.000442005),
        }
    }
    pub fn samples(self, p: &BevelOptions) -> Result<Vec<DVec2>> {
        if ![1, 2, 3, 4, 8].contains(&p.segments) {
            // Spec: BV-Q02
            return Err(
                "Profile count must be 1, 2, 3, 4 or 8 (1 uses the sampled original shape)".into(),
            );
        }
        let points = [
            DVec2::ZERO,
            DVec2::new(0.2, 0.1),
            DVec2::new(0.7, 0.9),
            DVec2::ONE,
        ];
        let controls = [
            [
                DVec2::new(0.0973781, -0.000442005),
                DVec2::new(0.114173, 0.0219583),
            ],
            [
                DVec2::new(0.285827, 0.178042),
                DVec2::new(0.583263, 0.808089),
            ],
            [DVec2::new(0.816737, 0.991911), DVec2::new(0.86279, 1.01393)],
        ];
        let divisions = match (self, p.segments) {
            (Self::Linear, 8) => [3, 2, 2],
            (Self::Linear, _) => [1, 1, 1],
            (Self::Smooth, 8) => [2, 2, 3],
            (Self::Smooth, _) => [2, 4, 4],
        };
        let mut samples = vec![points[0]];
        for (segment, &count) in divisions.iter().enumerate() {
            for k in 1..=count {
                let t = k as f64 / count as f64;
                samples.push(match self {
                    Self::Linear => {
                        points[segment].lerp(points[segment + 1], t * t * (3. - 2. * t))
                    }
                    Self::Smooth => {
                        let q = 1. - t;
                        points[segment] * (q * q * q)
                            + controls[segment][0] * (3. * q * q * t)
                            + controls[segment][1] * (3. * q * t * t)
                            + points[segment + 1] * (t * t * t)
                    }
                });
            }
        }
        // Measured original smooth shape normalizes its sampled vertical extent;
        // the explicit eight-point workflow uses the authored control bounds.
        let (lo, hi) = if self == Self::Smooth && p.segments == 8 {
            (-0.000442005, 1.01393)
        } else {
            (
                samples.iter().map(|x| x.y).fold(0., f64::min),
                samples.iter().map(|x| x.y).fold(1., f64::max),
            )
        };
        for q in &mut samples {
            q.y = (q.y - lo) / (hi - lo);
        }
        if p.segments > 1 && samples.len() > p.segments as usize {
            let last = *samples.last().unwrap();
            samples.truncate(p.segments as usize - 1);
            samples.push(last);
        }
        if p.reverse_direction {
            samples.reverse();
            for q in &mut samples {
                q.y = 1. - q.y;
            }
        }
        for q in &mut samples {
            q.x = (1. - q.x) * if p.reverse_inset { -1. } else { 1. };
        }
        Ok(samples)
    }
}
