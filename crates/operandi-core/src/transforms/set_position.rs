//! Set Position (spec `set-position-2026-11-05`): every affected point gets
//! the same coordinate on one axis, in local, world or work-plane space.
//! Target snapshots bind the dialog to its original document and scene.
use crate::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionAxis {
    X,
    Y,
    Z,
}
impl PositionAxis {
    fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}
/// Which space Value is measured in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PositionSpace {
    /// The mesh item's own coordinates; the item transform is ignored.
    #[default]
    Local,
    /// A world coordinate on the world axis (World Space on).
    World,
    /// Along the work plane's axis from its centre (Work Plane on).
    WorkPlane,
}
impl PositionSpace {
    /// From the World Space and Work Plane switches. Both on is refused
    /// (provisional, Q-ST-01: which one wins wasn't observed).
    pub fn from_switches(work_plane: bool, world_space: bool) -> Result<Self> {
        match (work_plane, world_space) {
            (false, false) => Ok(Self::Local),
            (false, true) => Ok(Self::World),
            (true, false) => Ok(Self::WorkPlane),
            (true, true) => Err("Set Position: turn on World Space or Work Plane, not both".into()),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PositionTarget {
    document: DocumentToken,
    scene: Scene,
    item: u64,
    vertices: Vec<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PositionAssignment {
    pub axis: PositionAxis,
    pub value: f64,
    pub space: PositionSpace,
    pub expected: Option<PositionTarget>,
}
impl Editor {
    pub fn set_position_target(&self) -> Result<PositionTarget> {
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before Set Position".into());
        }
        if self.session.pen_draft.is_some() {
            return Err("Drop or Cancel Pen before Set Position".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Set Position".into());
        }
        let mode = self.scene.selection.mode;
        if self.action_context != ActionContext::default() || self.spatial_falloff.is_some() {
            return Err("Set Position requires the neutral construction and action context".into());
        }
        self.scene.validate()?;
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh for Set Position")?;
        let mesh = &item.mesh;
        let every_point = || -> Vec<u32> {
            let used = mesh.used_points();
            (0..mesh.vertices.len() as u32)
                .filter(|&v| used[v as usize])
                .collect()
        };
        let selection = &self.scene.selection;
        let vertices: Vec<u32> = match mode {
            // Item mode: every point of the selected mesh (ST16).
            Mode::Item => every_point(),
            // Nothing selected: every point (ST04).
            Mode::Vertex if selection.vertices.is_empty() => every_point(),
            Mode::Vertex => {
                let mut set = BTreeSet::new();
                for &v in &selection.vertices {
                    if v as usize >= mesh.vertices.len() {
                        return Err("Selected point no longer exists".into());
                    }
                    set.insert(v);
                }
                set.into_iter().collect()
            }
            // Nothing selected: every point, as in the other modes (SI-12,
            // measured M05).
            Mode::Edge if selection.edges.is_empty() => every_point(),
            Mode::Edge => {
                let existing: BTreeSet<(u32, u32)> = mesh.edges().iter().copied().collect();
                let mut set = BTreeSet::new();
                for &(a, b) in &selection.edges {
                    if !existing.contains(&(a.min(b), a.max(b))) {
                        return Err("Selected edge no longer exists".into());
                    }
                    set.extend([a, b]);
                }
                set.into_iter().collect()
            }
            // Polygon mode, nothing selected: every point (ST07).
            Mode::Polygon if selection.faces.is_empty() => every_point(),
            Mode::Polygon => {
                let mut set = BTreeSet::new();
                for &f in &selection.faces {
                    let face = mesh
                        .faces
                        .get(f as usize)
                        .ok_or("Selected polygon no longer exists")?;
                    set.extend(face.vertices.iter().copied());
                }
                set.into_iter().collect()
            }
        };
        Ok(PositionTarget {
            document: self.identities.document_token(),
            scene: self.scene.clone(),
            item: item.id,
            vertices,
        })
    }
    pub(crate) fn set_position(&mut self, assignment: PositionAssignment) -> Result<()> {
        if !assignment.value.is_finite() || assignment.value.abs() > 1e9 {
            return Err("Set Position requires a finite coordinate within the scene range".into());
        }
        let target = self.set_position_target()?;
        if assignment
            .expected
            .as_ref()
            .is_some_and(|expected| *expected != target)
        {
            return Err("Set Position target changed since the dialog opened".into());
        }
        let axis = assignment.axis.index();
        let source = &self.scene.active().ok_or("Missing target")?.mesh;
        // Local → measuring space and back. World: the item's world matrix.
        // Work Plane: world, then relative to the work plane's centre; the
        // work plane's axes are the world axes.
        let to_space = match assignment.space {
            PositionSpace::Local => glam::DMat4::IDENTITY,
            PositionSpace::World => self.scene.world_matrix(target.item)?,
            PositionSpace::WorkPlane => {
                glam::DMat4::from_translation(-self.scene.workplane.center)
                    * self.scene.world_matrix(target.item)?
            }
        };
        if to_space.determinant().abs() < 1e-300 || !to_space.is_finite() {
            return Err("Set Position: the item's transform can't be inverted".into());
        }
        let from_space = to_space.inverse();
        let mut moved = Vec::with_capacity(target.vertices.len());
        for &v in &target.vertices {
            let old = source.vertices[v as usize];
            let mut p = to_space.transform_point3(old);
            p[axis] = assignment.value;
            let new = if assignment.space == PositionSpace::Local {
                p
            } else {
                from_space.transform_point3(p)
            };
            if !new.is_finite() {
                return Err("Set Position: a point would not be finite".into());
            }
            if new != old {
                moved.push((v, new));
            }
        }
        // Points already at Value don't move; nothing to move is no edit.
        if moved.is_empty() {
            return Ok(());
        }
        let mut candidate = self.scene.clone();
        let mesh = &mut candidate
            .active_mut()
            .filter(|i| i.id == target.item)
            .ok_or("Missing target")?
            .mesh;
        let mut touched = vec![false; mesh.vertices.len()];
        for (vertex, position) in moved {
            mesh.vertices[vertex as usize] = position;
            touched[vertex as usize] = true;
        }
        // Flattening a polygon onto the axis (ST04: every Y of a flat XY
        // grid set to 0.5) leaves it zero-area; keep it, marked as the
        // deliberate degenerate polygon it is.
        for f in 0..mesh.faces.len() {
            if !mesh.is_polyline(f)
                && mesh.faces[f].vertices.iter().any(|&v| touched[v as usize])
                && mesh.normal(f) == glam::DVec3::ZERO
            {
                mesh.attributes.degenerate_faces.insert(f as u32);
            }
        }
        candidate.validate()?;
        // Only coordinates changed: observe the existing explicit layout before
        // generic synchronization, so component IDs survive this operation.
        let before = std::mem::replace(&mut self.scene, candidate);
        if let Err(error) = self.identities.observe(&self.scene) {
            self.scene = before;
            return Err(error);
        }
        self.record("Set Position".into(), before);
        Ok(())
    }
}
