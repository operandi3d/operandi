//! Box Transform (BEHAVIOR-UV-TOOLS §6.2, UVT-43): a UV-only tool that
//! draws a box round the selected UVs and drags them: move inside it,
//! turn by its rotation handle about the pivot, scale by right-dragging a
//! side (that axis, from the opposite side) or a corner (both axes, from
//! the opposite corner). It runs through the UV pipe (uv_pipe.rs), so seams,
//! Tear Off, falloffs and undo work as for Move.
//!
//! The edit: each selected UV is scaled about `anchor`, turned by `angle`
//! about `pivot`, then moved by `offset`.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md UVB-P01..P03): the pivot
//! starts at the box's centre; Shift-snapping lands the box's lower-left
//! corner on the 0.1 grid (UVT-Q05); Box U/V and Gaps aren't built.
use glam::{DMat2, DVec2, DVec3};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BoxTransform {
    pub offset: DVec2,
    /// The turn's centre; `None` is the box's centre.
    pub pivot: Option<DVec2>,
    /// Degrees, counter-clockwise.
    pub angle: f64,
    pub scale: DVec2,
    /// Where scaling is measured from; `None` is the pivot.
    pub anchor: Option<DVec2>,
    pub show_rotation_handle: bool,
    /// Degrees per Ctrl step of the rotation handle (15° start).
    pub snap_angle: f64,
}
impl Default for BoxTransform {
    fn default() -> Self {
        Self {
            offset: DVec2::ZERO,
            pivot: None,
            angle: 0.,
            scale: DVec2::ONE,
            anchor: None,
            show_rotation_handle: true,
            snap_angle: 15.,
        }
    }
}
impl BoxTransform {
    pub fn validate(&self) -> crate::Result<()> {
        let finite = self.offset.is_finite()
            && self.pivot.is_none_or(|p| p.is_finite())
            && self.anchor.is_none_or(|p| p.is_finite())
            && self.angle.is_finite()
            && self.scale.is_finite();
        if !finite {
            return Err("Box Transform's values must be numbers".into());
        }
        if self.scale.abs().min_element() < 1e-9 {
            return Err("Box Transform can't scale to nothing".into());
        }
        Ok(())
    }
    /// The pivot for a box `lo`..`hi`.
    pub fn pivot_of(&self, lo: DVec2, hi: DVec2) -> DVec2 {
        self.pivot.unwrap_or((lo + hi) * 0.5)
    }
    /// Where `p` goes, for a box `lo`..`hi`.
    pub fn apply(&self, p: DVec2, lo: DVec2, hi: DVec2) -> DVec2 {
        let pivot = self.pivot_of(lo, hi);
        let anchor = self.anchor.unwrap_or(pivot);
        let scaled = anchor + (p - anchor) * self.scale;
        let turned = pivot + DMat2::from_angle(self.angle.to_radians()) * (scaled - pivot);
        turned + self.offset
    }
}

/// The bounding box of `points` of `vertices`.
pub fn bounds(vertices: &[DVec3], points: impl Iterator<Item = u32>) -> Option<(DVec2, DVec2)> {
    let mut out: Option<(DVec2, DVec2)> = None;
    for v in points {
        let p = vertices.get(v as usize)?.truncate();
        out = Some(match out {
            None => (p, p),
            Some((lo, hi)) => (lo.min(p), hi.max(p)),
        });
    }
    out
}

impl crate::Editor {
    /// The live Box Transform's box at the session start (the selected UVs'
    /// bounds on the UV proxy), or `None` outside it.
    pub fn uv_box_base(&mut self) -> Option<(DVec2, DVec2)> {
        if self.session.live.as_ref()?.tool != crate::Tool::UvBoxTransform {
            return None;
        }
        let proxy = self.uv_tool_proxy()?;
        let points = crate::transform_vertices(&proxy.scene).ok()?;
        bounds(&proxy.mesh().vertices, points.into_iter())
    }
}
