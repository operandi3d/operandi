//! Frame All while a creation tool is open frames the shape being made,
//! not the whole scene (Jason, 2026-11-21, Model the Mug tutorial). Shared
//! by every creation tool: the live primitives (Cube, Sphere, Cylinder)
//! and the numeric tools whose preview stands in for the scene (Cone,
//! Ellipsoid, Torus, Capsule...). Nothing here knows one tool.
use super::*;

/// The size framed around a creation tool's centre when it has nothing
/// drawn yet (or no extent): a 1 m box, the starting Cube. Provisional.
const EMPTY_SHAPE_SIZE: f64 = 1.;

impl Operandi {
    /// World points for Frame All with a creation tool open: the geometry
    /// the tool adds; or, before it adds any (or while it is still flat
    /// to a point), a starting-size box at the tool's centre. `None` when
    /// no creation tool is open, so Frame All frames the scene as usual.
    pub(super) fn creation_tool_frame_points(&self) -> Option<Vec<DVec3>> {
        let (before, after, centre) = if let Some(live) = self
            .editor
            .session
            .live
            .as_ref()
            .filter(|l| l.tool.is_primitive())
        {
            (
                &live.base,
                std::borrow::Cow::Borrowed(&self.editor.scene),
                Some(live.params.position),
            )
        } else {
            let preview = self.editor.tool_preview_display()?;
            (&self.editor.scene, std::borrow::Cow::Owned(preview), None)
        };
        let mut points = Vec::new();
        for item in after.items.iter().filter(|i| after.shows(i)) {
            let old = before.item(item.id).map_or(0, |b| b.mesh.vertices.len());
            points.extend(
                item.mesh
                    .vertices
                    .iter()
                    .skip(old)
                    .map(|&v| item.transform.point(v)),
            );
        }
        let lo = points.iter().copied().fold(DVec3::INFINITY, DVec3::min);
        let hi = points.iter().copied().fold(DVec3::NEG_INFINITY, DVec3::max);
        let centre = match (points.is_empty(), centre) {
            (false, _) if (hi - lo).length() > 1e-9 => return Some(points),
            (false, _) => lo,
            (true, Some(c)) => c,
            // A numeric preview with no new points: frame the scene.
            (true, None) => return None,
        };
        let h = DVec3::splat(EMPTY_SHAPE_SIZE / 2.);
        Some(vec![centre - h, centre + h])
    }
}
