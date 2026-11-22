//! The shared Profile layer (spec loop-slice-2026-11-15,
//! BEHAVIOR-DRAG-LAYER.md section 3; BEHAVIOR-LOOP-SLICE.md LS-40..LS-44).
//!
//! A 1D profile is a small cross-section shape: a polyline across a unit
//! width (x from 0 to 1) with a depth at each point (y, in units of that
//! width, positive = into the surface). A tool that takes a profile maps the
//! width onto whatever it cuts across (Loop Slice: the ring) and pushes each
//! point by its depth. Loop Slice is the first user; Bevel, Edge
//! Bevel/Chamfer, Edge Slide, Bridge, Sweep and Tube can pick from the same
//! list later.
//!
//! The shapes are Operandi's own (LS-44): drawn here from plain numbers,
//! never taken from another application's content.
use glam::DVec2;

/// One of Operandi's built-in profile shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProfileShape {
    /// A half-round groove across the middle third and a bit.
    RoundGroove,
    /// A V with straight sides.
    VGroove,
    /// A flat-bottomed slot with straight walls.
    SquareSlot,
    /// A narrow neck opening into a wider undercut.
    TSlot,
    /// A one-sided notch: a slope down, then a straight wall back up. Not
    /// symmetric, so Reverse Direction visibly flips it.
    SawNotch,
}

impl ProfileShape {
    /// Every shape, in list order.
    pub const ALL: [ProfileShape; 5] = [
        Self::RoundGroove,
        Self::VGroove,
        Self::SquareSlot,
        Self::TSlot,
        Self::SawNotch,
    ];

    /// The name shown in lists.
    pub fn label(self) -> &'static str {
        match self {
            Self::RoundGroove => "Round Groove",
            Self::VGroove => "V Groove",
            Self::SquareSlot => "Square Slot",
            Self::TSlot => "T Slot",
            Self::SawNotch => "Saw Notch",
        }
    }

    /// A stable lowercase name for scripts and saved settings.
    pub fn slug(self) -> &'static str {
        match self {
            Self::RoundGroove => "round-groove",
            Self::VGroove => "v-groove",
            Self::SquareSlot => "square-slot",
            Self::TSlot => "t-slot",
            Self::SawNotch => "saw-notch",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.slug() == slug)
    }

    /// The shape's points across the unit width, in order from x = 0's
    /// side: (x, depth), depth in units of the width.
    pub fn points(self) -> Vec<DVec2> {
        let p = DVec2::new;
        match self {
            Self::RoundGroove => {
                // Seven points round a half circle of radius 0.3.
                (0..=6)
                    .map(|k| {
                        let a = std::f64::consts::PI * k as f64 / 6.;
                        p(0.5 - 0.3 * a.cos(), 0.3 * a.sin())
                    })
                    .collect()
            }
            Self::VGroove => vec![p(0.3, 0.), p(0.5, 0.25), p(0.7, 0.)],
            Self::SquareSlot => vec![p(0.35, 0.), p(0.35, 0.3), p(0.65, 0.3), p(0.65, 0.)],
            Self::TSlot => vec![
                p(0.4, 0.),
                p(0.4, 0.15),
                p(0.2, 0.15),
                p(0.2, 0.4),
                p(0.8, 0.4),
                p(0.8, 0.15),
                p(0.6, 0.15),
                p(0.6, 0.),
            ],
            Self::SawNotch => vec![p(0.3, 0.), p(0.7, 0.3), p(0.7, 0.)],
        }
    }

    /// The deepest point's depth (units of the width).
    pub fn depth(self) -> f64 {
        self.points().iter().fold(0., |m, q| m.max(q.y.abs()))
    }
}

/// The profile part of a tool's options: the picked shape and the switches
/// that only mean something with one (LS-42, LS-43).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfilePick {
    /// `None` = no profile (the tool's plain behaviour).
    pub shape: Option<ProfileShape>,
    /// Read the shape from the other end (x becomes 1 - x).
    pub reverse_direction: bool,
    /// Flip in and out: a groove becomes a ridge.
    pub reverse_inset: bool,
    /// The depth follows the shape's own width-to-depth ratio, scaled to
    /// the width it is laid across; off, the tool's Inset sets the depth.
    pub keep_aspect: bool,
}

impl Default for ProfilePick {
    fn default() -> Self {
        Self {
            shape: None,
            reverse_direction: false,
            reverse_inset: false,
            // Keep Aspect starts on (LS-42, STARTING-VALUES 1A).
            keep_aspect: true,
        }
    }
}

/// One profile point laid across a width: `x` across (0..1) and the push
/// into the surface in metres (negative = out).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfilePoint {
    pub x: f64,
    pub depth: f64,
}

impl ProfilePick {
    /// Whether a shape is picked (the Reverse and Keep Aspect switches are
    /// greyed without one; Inset is greyed while Keep Aspect sets it).
    pub fn active(&self) -> bool {
        self.shape.is_some()
    }

    /// Whether the tool's own Inset sets the depth: with no profile, or
    /// with one and Keep Aspect off.
    pub fn uses_inset(&self) -> bool {
        !(self.active() && self.keep_aspect)
    }

    /// The picked shape laid across `width` metres, in order along x from
    /// the start side (reversed with Reverse Direction). `inset` is the
    /// tool's Inset, the deepest point's depth when Keep Aspect is off.
    /// `None` with no shape picked.
    pub fn lay(&self, width: f64, inset: f64) -> Option<Vec<ProfilePoint>> {
        let shape = self.shape?;
        let mut points = shape.points();
        if self.reverse_direction {
            points.reverse();
            for q in &mut points {
                q.x = 1. - q.x;
            }
        }
        let scale = if self.keep_aspect {
            width
        } else {
            let deepest = shape.depth();
            if deepest > 0. { inset / deepest } else { 0. }
        };
        let sign = if self.reverse_inset { -1. } else { 1. };
        Some(
            points
                .into_iter()
                .map(|q| ProfilePoint {
                    x: q.x,
                    depth: q.y * scale * sign,
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_starts_and_ends_on_the_surface_inside_the_width() {
        for s in ProfileShape::ALL {
            let pts = s.points();
            assert!(pts.len() >= 3, "{s:?}");
            assert!(pts[0].y.abs() < 1e-12 && pts.last().unwrap().y.abs() < 1e-12);
            assert!(pts.iter().all(|q| q.x > 0. && q.x < 1. && q.y >= -1e-12));
            assert_eq!(ProfileShape::from_slug(s.slug()), Some(s));
        }
    }

    #[test]
    fn lay_scales_reverses_and_flips() {
        let mut p = ProfilePick {
            shape: Some(ProfileShape::SawNotch),
            ..Default::default()
        };
        let laid = p.lay(2., 0.).unwrap();
        assert_eq!(laid.len(), 3);
        assert!((laid[1].x - 0.7).abs() < 1e-12 && (laid[1].depth - 0.6).abs() < 1e-12);
        p.reverse_direction = true;
        let laid = p.lay(2., 0.).unwrap();
        assert!((laid[0].x - 0.3).abs() < 1e-12 && (laid[1].x - 0.3).abs() < 1e-12);
        p.reverse_inset = true;
        assert!(p.lay(2., 0.).unwrap()[1].depth < 0.);
        // Keep Aspect off: Inset is the deepest point's depth.
        p.keep_aspect = false;
        p.reverse_inset = false;
        assert!((p.lay(2., 0.1).unwrap()[1].depth - 0.1).abs() < 1e-12);
        assert!(ProfilePick::default().lay(1., 0.).is_none());
    }
}
