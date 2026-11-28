//! Modifier extras for the transform tools (HM rules, Jason 2026-11-07):
//! Shift-drag continues the drag just committed (HM-02), middle-drag starts
//! from the previous drag's values (HM-04), and Ctrl+Shift in a component
//! mode drags a copy of the selected polygons (HM-06).
use crate::*;
use std::collections::BTreeSet;

/// One committed Move/Rotate/Scale/Transform drag: where it started, the
/// values it reached and the scene it left.
#[derive(Clone, PartialEq)]
pub struct TransformStep {
    pub tool: Tool,
    pub base: Scene,
    pub params: Parameters,
    pub action_frame: Option<ActionFrame>,
    pub identity_base: Option<IdentityScene>,
    pub applied: bool,
    /// The scene right after the drag; the step only counts while the scene
    /// is still exactly this (no undo, no other edit since).
    pub after: Scene,
}

impl Editor {
    /// The last committed step of the live transform tool, if it is still
    /// the latest change and no drag is in progress.
    fn current_transform_step(&self) -> Option<(&Live, &TransformStep)> {
        let live = self
            .session
            .live
            .as_ref()
            .filter(|l| l.tool.is_transform() && l.tool != Tool::Tweak)?;
        if live.base != self.scene {
            return None;
        }
        let step = self
            .session
            .last_transform_step
            .as_deref()
            .filter(|s| s.tool == live.tool && s.after == self.scene)?;
        Some((live, step))
    }

    /// HM-02: take up the last committed drag again, so a Shift-drag adds to
    /// its values. With nothing to continue the next drag starts fresh.
    pub(crate) fn continue_transform_step(&mut self) -> Result<()> {
        let Some((live, step)) = self.current_transform_step() else {
            return Ok(());
        };
        let mut next = live.clone();
        next.base = step.base.clone();
        next.params = step.params.clone();
        next.action_frame = step.action_frame;
        next.identity_base = step.identity_base.clone();
        next.identity_preview = None;
        next.applied = step.applied;
        self.session.live = Some(next);
        Ok(())
    }

    /// HM-04: the live tool's values with the previous drag's amounts
    /// applied again from where the tool is now, for a middle-drag or
    /// middle-click. The caller previews them.
    pub fn repeat_transform_params(&self) -> Result<Parameters> {
        let (live, step) = self
            .current_transform_step()
            .ok_or("Nothing to repeat yet: drag the tool once first")?;
        let mut params = live.params.clone();
        params.delta = step.params.delta;
        params.rotation_delta = step.params.rotation_delta;
        params.scale_delta = step.params.scale_delta;
        Ok(params)
    }

    /// HM-06 in a component mode: copy the whole polygons the selection
    /// covers into the same mesh, as one undo step, and select the copy in
    /// the current mode. Vertex and Edge mode copy the polygons whose points
    /// or edges are all selected.
    pub(crate) fn duplicate_selected_polygons(&mut self) -> Result<()> {
        self.require_idle()?;
        let sel = &self.scene.selection;
        let mode = sel.mode;
        let item = self
            .scene
            .active()
            .filter(|i| i.is_mesh() && self.scene.shows(i))
            .ok_or("Select a visible mesh to copy polygons from")?;
        let mesh = &item.mesh;
        let faces: BTreeSet<usize> = match mode {
            Mode::Item => {
                return Err(
                    "Ctrl+Shift-drag copies polygons in Vertex, Edge or Polygon mode".into(),
                );
            }
            Mode::Polygon => sel.faces.iter().map(|&f| f as usize).collect(),
            Mode::Vertex => {
                let points: BTreeSet<u32> = sel.vertices.iter().copied().collect();
                (0..mesh.faces.len())
                    .filter(|&f| {
                        !mesh.is_polyline(f)
                            && mesh.faces[f].vertices.iter().all(|v| points.contains(v))
                    })
                    .collect()
            }
            Mode::Edge => {
                let edges: BTreeSet<(u32, u32)> = sel
                    .edges
                    .iter()
                    .map(|&(a, b)| (a.min(b), a.max(b)))
                    .collect();
                (0..mesh.faces.len())
                    .filter(|&f| {
                        let v = &mesh.faces[f].vertices;
                        !mesh.is_polyline(f)
                            && (0..v.len()).all(|i| {
                                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                                edges.contains(&(a.min(b), a.max(b)))
                            })
                    })
                    .collect()
            }
        };
        if faces.is_empty() {
            return Err(
                "Nothing whole to copy: Ctrl+Shift-drag copies complete polygons, so select every point or edge of at least one polygon"
                    .into(),
            );
        }
        let kept = crate::clipboard::whole_faces(mesh, |f| faces.contains(&f));
        let (copy, _) = crate::clipboard::subset(mesh, &kept, &BTreeSet::new())?;
        let (point_start, face_start) = (mesh.vertices.len() as u32, mesh.faces.len() as u32);
        let copy_faces: Vec<Vec<u32>> = copy
            .faces
            .iter()
            .map(|f| f.vertices.iter().map(|v| v + point_start).collect())
            .collect();
        let added_points = copy.vertices.len() as u32;
        let mut scene = self.scene.clone();
        let target = scene.active_mut().ok_or("Select a mesh")?;
        crate::append_mesh(&mut target.mesh, copy)?;
        let s = &mut scene.selection;
        s.vertices.clear();
        s.edges.clear();
        s.faces.clear();
        match mode {
            Mode::Polygon => {
                s.faces = (face_start..face_start + copy_faces.len() as u32).collect();
            }
            Mode::Vertex => s.vertices = (point_start..point_start + added_points).collect(),
            _ => {
                let mut edges: BTreeSet<(u32, u32)> = BTreeSet::new();
                for f in &copy_faces {
                    for i in 0..f.len() {
                        let (a, b) = (f[i], f[(i + 1) % f.len()]);
                        edges.insert((a.min(b), a.max(b)));
                    }
                }
                s.edges = edges.into_iter().collect();
            }
        }
        scene.validate()?;
        let before = std::mem::replace(&mut self.scene, scene);
        self.record("Duplicate Polygons".into(), before);
        Ok(())
    }
}
