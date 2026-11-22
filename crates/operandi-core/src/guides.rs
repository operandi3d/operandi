//! Guides (G9 Part B, GD-0..GD-3): up to ten construction lines per scene,
//! numbered 0–9, that draw in the views and that dragged points snap to.
//!
//! - GD-1: each guide is Two Points (a line through Position and End) or a
//!   Planar Ray (from Position, lying in a plane, at an Angle in it, of a
//!   set Distance). Enable, Show and Snap are off for a new guide; Lock
//!   stops it being moved. Current Guide is the one the Guides tool edits.
//! - GD-2: a guide that is on with Snap to Guide is one more snapping
//!   target (the app's snap service).
//! - GD-3 (PROVISIONAL): guides belong to the scene and save with it.
//!
//! PROVISIONAL (GD-Q1): a Planar Ray's Angle is measured from the plane's
//! first axis towards its second (YZ: from +Y to +Z; XZ: +X to +Z; XY: +X
//! to +Y); a new guide's Distance is 1 m and End sits 1 m along +X.
use crate::Result;
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// GD-1: how many guides a scene has.
pub const COUNT: usize = 10;

/// GD-1 Mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuideMode {
    #[default]
    TwoPoints,
    PlanarRay,
}

/// The plane a Planar Ray lies in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuidePlane {
    #[default]
    Yz,
    Xz,
    Xy,
}

impl GuidePlane {
    pub const ALL: [GuidePlane; 3] = [GuidePlane::Yz, GuidePlane::Xz, GuidePlane::Xy];
    pub fn label(self) -> &'static str {
        match self {
            GuidePlane::Yz => "YZ",
            GuidePlane::Xz => "XZ",
            GuidePlane::Xy => "XY",
        }
    }
    /// The plane's first and second axes.
    fn axes(self) -> (DVec3, DVec3) {
        match self {
            GuidePlane::Yz => (DVec3::Y, DVec3::Z),
            GuidePlane::Xz => (DVec3::X, DVec3::Z),
            GuidePlane::Xy => (DVec3::X, DVec3::Y),
        }
    }
}

/// One guide (GD-1).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Guide {
    pub enabled: bool,
    pub show: bool,
    pub snap: bool,
    pub lock: bool,
    pub mode: GuideMode,
    pub position: DVec3,
    pub end: DVec3,
    pub plane: GuidePlane,
    /// Degrees.
    pub angle: f64,
    /// Metres.
    pub distance: f64,
}

impl Default for Guide {
    fn default() -> Self {
        Self {
            enabled: false,
            show: false,
            snap: false,
            lock: false,
            mode: GuideMode::TwoPoints,
            position: DVec3::ZERO,
            end: DVec3::X,
            plane: GuidePlane::Yz,
            angle: 0.,
            distance: 1.,
        }
    }
}

impl Guide {
    /// The guide's two ends in world space.
    pub fn segment(&self) -> (DVec3, DVec3) {
        match self.mode {
            GuideMode::TwoPoints => (self.position, self.end),
            GuideMode::PlanarRay => {
                let (a, b) = self.plane.axes();
                let t = self.angle.to_radians();
                (
                    self.position,
                    self.position + (a * t.cos() + b * t.sin()) * self.distance,
                )
            }
        }
    }
    /// Drawn in the views (Enable and Show on).
    pub fn drawn(&self) -> bool {
        self.enabled && self.show
    }
    /// A snapping target (Enable and Snap to Guide on).
    pub fn snaps(&self) -> bool {
        self.enabled && self.snap
    }
    pub fn validate(&self) -> Result<()> {
        let finite = self.position.is_finite()
            && self.end.is_finite()
            && self.angle.is_finite()
            && self.distance.is_finite();
        if !finite || self.position.abs().max_element() > 1e9 || self.end.abs().max_element() > 1e9
        {
            return Err("A guide's values must be finite and within the coordinate budget".into());
        }
        if self.distance < 0. {
            return Err("A guide's Distance can't be negative".into());
        }
        Ok(())
    }
}

/// A scene's guides and the one being edited (GD-1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Guides {
    pub current: usize,
    pub guides: Vec<Guide>,
}

impl Default for Guides {
    fn default() -> Self {
        Self {
            current: 0,
            guides: vec![Guide::default(); COUNT],
        }
    }
}

impl Guides {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
    pub fn validate(&self) -> Result<()> {
        if self.guides.len() != COUNT || self.current >= COUNT {
            return Err(format!("A scene has {COUNT} guides, numbered 0–9"));
        }
        self.guides.iter().try_for_each(Guide::validate)
    }
    /// The guides that draw, as world segments.
    pub fn drawn(&self) -> Vec<(DVec3, DVec3)> {
        self.guides
            .iter()
            .filter(|g| g.drawn())
            .map(Guide::segment)
            .collect()
    }
    /// The guides points snap to, as world segments.
    pub fn snapping(&self) -> Vec<(DVec3, DVec3)> {
        self.guides
            .iter()
            .filter(|g| g.snaps())
            .map(Guide::segment)
            .collect()
    }
}

/// The point of segment `(a, b)` nearest `p`.
pub fn nearest_on(a: DVec3, b: DVec3, p: DVec3) -> DVec3 {
    let d = b - a;
    let len = d.length_squared();
    if len < 1e-30 {
        return a;
    }
    a + d * ((p - a).dot(d) / len).clamp(0., 1.)
}

impl crate::Editor {
    /// GD-1: the scene's guides, one undo step ("Guides").
    pub(crate) fn set_guides(&mut self, guides: Guides) -> Result<()> {
        guides.validate()?;
        if self.scene.guides == guides {
            return Ok(());
        }
        let before = self.scene.clone();
        self.scene.guides = guides;
        self.record("Guides".into(), before);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_planar_ray_lies_in_its_plane_at_its_angle() {
        let g = Guide {
            mode: GuideMode::PlanarRay,
            plane: GuidePlane::Xz,
            angle: 90.,
            distance: 2.,
            position: DVec3::Y,
            ..Default::default()
        };
        let (a, b) = g.segment();
        assert_eq!(a, DVec3::Y);
        assert!(b.abs_diff_eq(DVec3::new(0., 1., 2.), 1e-12));
    }

    #[test]
    fn new_guides_are_off() {
        let g = Guides::default();
        assert_eq!(g.guides.len(), 10);
        assert!(g.drawn().is_empty() && g.snapping().is_empty());
    }
}
