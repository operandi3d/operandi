//! Flatten Ngons (spec `offset-flatten-2026-11-05`).
//!
//! * Only n-gons: polygons with five or more corners. Triangles and quads
//!   never change, even when non-planar.
//! * The whole active mesh, whatever is selected, in Polygon or Vertex mode.
//! * Each non-planar n-gon's points move straight onto the plane through
//!   the centre of its bounding box with its Newell normal, along that
//!   normal (PF-08 v2, 2026-11-20; `flatten_normal`). A polygon whose
//!   Newell normal is zero is left alone.
//!   Shared points move with the n-gon, so neighbours change shape too.
//!   Where two n-gons share points the later one in polygon order wins
//!   (provisional, Q-FL-02).
//! * Afterwards the polygon and point selections are cleared, even when no
//!   n-gon needed flattening (FN01); the mode stays. With nothing to flatten
//!   and nothing selected there is no edit.
//!
//! Points are flattened in the mesh's own (item-local) space.
use crate::{Editor, Mesh, Result};
use glam::DVec3;

/// How Flatten Ngons treats one polygon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlattenFaceClass {
    /// Polyline element: not a surface face, never examined or changed.
    NotSurface,
    /// Four or fewer corners: never changed.
    NotNgon,
    /// Five or more corners, already planar: unchanged.
    Planar,
    /// Five or more corners, not planar: flattened.
    Candidate,
}

/// Why Flatten Ngons can't run (kept for callers that report it).
#[derive(Clone, Debug, PartialEq)]
pub struct FlattenRefusal {
    pub face: Option<u32>,
    pub detail: String,
}

impl FlattenRefusal {
    pub fn message(&self) -> String {
        match self.face {
            Some(f) => format!(
                "Flatten Ngons: polygon {f}: {}; nothing changed",
                self.detail
            ),
            None => format!("Flatten Ngons: {}; nothing changed", self.detail),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FlattenOutcome {
    /// Nothing to flatten: scene, history and redo stay untouched.
    NoChange,
    /// New local positions, in ascending point order.
    Change(Vec<(u32, DVec3)>),
    Refused(FlattenRefusal),
}

/// Read-only Flatten Ngons plan for the active mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct FlattenPlan {
    pub item: u64,
    pub classes: Vec<FlattenFaceClass>,
    pub outcome: FlattenOutcome,
}

/// The direction Flatten Ngons moves a polygon's points along (PF-08 v2,
/// decided 2026-11-20): the Newell normal of its points, or `None` when
/// that is zero. The one place to change the normal rule.
pub(crate) fn flatten_normal(points: &[DVec3]) -> Option<DVec3> {
    let n = points.len();
    let newell: DVec3 = (0..n).map(|i| points[i].cross(points[(i + 1) % n])).sum();
    newell.try_normalize()
}

/// How far from its plane, as a share of its bounding box's longest side,
/// a point may be for its polygon to count as flat (PF-08). The Statistics
/// panel's Not flat check uses the same rule (statistics.rs, Q5).
pub(crate) const FLAT_TOLERANCE: f64 = 1e-12;

/// Plan for one mesh. Pure: never mutates.
pub(crate) fn plan_mesh(mesh: &Mesh) -> (Vec<FlattenFaceClass>, FlattenOutcome) {
    let mut classes = Vec::with_capacity(mesh.faces.len());
    let mut moved: Vec<Option<DVec3>> = vec![None; mesh.vertices.len()];
    for (f, face) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(f) {
            classes.push(FlattenFaceClass::NotSurface);
            continue;
        }
        if face.vertices.len() < 5 {
            classes.push(FlattenFaceClass::NotNgon);
            continue;
        }
        // Later n-gons see earlier n-gons' moves (Q-FL-02).
        let points: Vec<DVec3> = face
            .vertices
            .iter()
            .map(|&v| moved[v as usize].unwrap_or(mesh.vertices[v as usize]))
            .collect();
        // PF-08 v2: the plane goes through the centre of the polygon's
        // bounding box, with the normal from `flatten_normal`.
        let (lo, hi) = points.iter().fold(
            (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
            |(lo, hi), &p| (lo.min(p), hi.max(p)),
        );
        let centre = (lo + hi) * 0.5;
        let extent = (hi - lo).max_element();
        let Some(normal) = flatten_normal(&points) else {
            // No normal (a degenerate polygon): left alone.
            classes.push(FlattenFaceClass::Planar);
            continue;
        };
        let distance = |p: DVec3| (p - centre).dot(normal);
        if points
            .iter()
            .all(|&p| distance(p).abs() <= FLAT_TOLERANCE * extent.max(1e-300))
        {
            classes.push(FlattenFaceClass::Planar);
            continue;
        }
        classes.push(FlattenFaceClass::Candidate);
        for (&v, &p) in face.vertices.iter().zip(&points) {
            moved[v as usize] = Some(p - normal * distance(p));
        }
    }
    let changes: Vec<(u32, DVec3)> = moved
        .iter()
        .enumerate()
        .filter_map(|(v, p)| p.map(|p| (v as u32, p)))
        .filter(|&(v, p)| p != mesh.vertices[v as usize])
        .collect();
    let outcome = if changes.is_empty() {
        FlattenOutcome::NoChange
    } else if changes.iter().any(|(_, p)| !p.is_finite()) {
        FlattenOutcome::Refused(FlattenRefusal {
            face: None,
            detail: "a flattened point would not be finite".into(),
        })
    } else {
        FlattenOutcome::Change(changes)
    };
    (classes, outcome)
}

impl Editor {
    /// Read-only Flatten Ngons plan for the current scene. Never mutates the
    /// editor; the command executes this same plan.
    pub fn flatten_ngons_plan(&self) -> Result<FlattenPlan> {
        if self.i2_active() || self.session.pen_draft.is_some() {
            return Err("Finish or Cancel the active tool before Flatten Ngons".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish selection before Flatten Ngons".into());
        }
        self.scene.validate()?;
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh before Flatten Ngons")?;
        let (classes, outcome) = plan_mesh(&item.mesh);
        Ok(FlattenPlan {
            item: item.id,
            classes,
            outcome,
        })
    }

    /// One atomic edit: n-gon points move and the polygon and point
    /// selections clear together; the mode is kept.
    pub(crate) fn flatten_ngons(&mut self) -> Result<()> {
        let plan = self.flatten_ngons_plan()?;
        let moves = match plan.outcome {
            FlattenOutcome::Refused(refusal) => return Err(refusal.message()),
            FlattenOutcome::Change(moves) => moves,
            FlattenOutcome::NoChange => Vec::new(),
        };
        let selected = !self.scene.selection.faces.is_empty()
            || !self.scene.selection.vertices.is_empty()
            || !self.scene.selection.edges.is_empty();
        // Nothing to flatten and nothing to deselect: no edit at all.
        if moves.is_empty() && !selected {
            return Ok(());
        }
        let mut scene = self.scene.clone();
        let item = scene
            .item_mut(plan.item)
            .ok_or("Missing Flatten Ngons target")?;
        for (vertex, position) in moves {
            item.mesh.vertices[vertex as usize] = position;
        }
        // PF-08 v2: nothing selected afterwards, in any mode.
        scene.selection.faces.clear();
        scene.selection.vertices.clear();
        scene.selection.edges.clear();
        scene.validate()?;
        // Only coordinates (and the selection) changed: observe the existing
        // layout so component identities survive, as Set Position does.
        let before = std::mem::replace(&mut self.scene, scene);
        if let Err(error) = self.identities.observe(&self.scene) {
            self.scene = before;
            return Err(error);
        }
        self.record("Flatten Ngons".into(), before);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PF-08 v2: a polygon whose Newell normal is zero is left alone (a
    /// valid scene refuses one; the plan still copes).
    #[test]
    fn a_zero_newell_polygon_is_left_alone() {
        let points = vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(2., 2., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(0., 2., 0.),
            DVec3::new(0., 1., 0.),
        ];
        assert_eq!(flatten_normal(&points), None);
        let mesh = Mesh {
            vertices: points,
            faces: vec![crate::Face {
                vertices: vec![0, 1, 2, 3, 4],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            }]
            .into(),
            ..Default::default()
        };
        let (classes, outcome) = plan_mesh(&mesh);
        assert_eq!(classes, [FlattenFaceClass::Planar]);
        assert_eq!(outcome, FlattenOutcome::NoChange);
    }
}
