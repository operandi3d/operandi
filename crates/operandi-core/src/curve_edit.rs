//! One undoable edit of a drawn curve (Curve, B-Spline, Bezier, polyline):
//! the Draw tools' clicks and drags each end in one of these (DRW-01: one
//! undo step per click or drag, the tool stays on).
//!
//! The edited curve is rebuilt at the end of its mesh's polygon list, so
//! the tool always finds it as the mesh's last polygon. Its old points are
//! removed unless another polygon uses them.
use crate::clipboard::{KeptFace, subset};
use crate::curves::CurveKind;
use crate::{Editor, Face, Mesh, PolygonRepresentation, Result};
use glam::DVec3;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub struct CurveEdit {
    /// The mesh to draw into; `None` = the active mesh, or a new one.
    pub item: Option<u64>,
    /// The curve polygon being changed; `None` adds a new one.
    pub face: Option<u32>,
    /// Control points in world space. Empty removes the curve.
    pub points: Vec<DVec3>,
    pub closed: bool,
    pub kind: CurveKind,
    /// Undo label, such as "Curve Point".
    pub label: String,
}

/// `mesh` without polygon `face` and the points only it used.
pub fn without_face(mesh: &Mesh, face: usize) -> Result<Mesh> {
    let kept: Vec<KeptFace> = (0..mesh.faces.len())
        .filter(|&f| f != face)
        .map(|f| KeptFace {
            source: f,
            corners: (0..mesh.faces[f].vertices.len()).collect(),
        })
        .collect();
    let own: BTreeSet<u32> = mesh.faces[face].vertices.iter().copied().collect();
    let used = mesh.used_points();
    // Loose points stay loose; points only this curve used go.
    let loose: BTreeSet<u32> = (0..mesh.vertices.len() as u32)
        .filter(|v| !used[*v as usize] && !own.contains(v))
        .collect();
    Ok(subset(mesh, &kept, &loose)?.0)
}

/// A one-polygon mesh holding the curve.
pub fn curve_mesh(points: Vec<DVec3>, closed: bool, kind: CurveKind) -> Mesh {
    let mut m = Mesh::default();
    let n = points.len() as u32;
    m.vertices = points;
    m.faces.push(Face {
        vertices: (0..n).collect(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    });
    m.attributes
        .representations
        .push(PolygonRepresentation::Polyline {
            closed,
            curve: kind,
        });
    m
}

/// World-space geometry to add to the active mesh (or a new one) as one
/// undo step: Sketch strokes and Draw Patch surfaces.
#[derive(Clone, Debug, PartialEq)]
pub struct AddGeometry {
    pub item: Option<u64>,
    pub mesh: Mesh,
    pub label: String,
    /// `(added point, existing point)` pairs: the added point merges into
    /// the target mesh's existing one, so a Patch Curves grid reuses the
    /// curves' own corner points (G7 PC-5). Empty for plain additions.
    pub weld: Vec<(u32, u32)>,
    /// `(existing point, existing point)` pairs of the target mesh merged
    /// too: a live patch joins its curves' separate end points at each
    /// corner, so moving a corner moves both curves (G7 PC-7).
    pub join: Vec<(u32, u32)>,
}

/// A mesh to add, its `(added, existing)` welds and `(existing, existing)`
/// joins (see [`AddGeometry`]).
type Addition = (Mesh, Vec<(u32, u32)>, Vec<(u32, u32)>);

impl Editor {
    pub(crate) fn add_geometry(&mut self, add: AddGeometry) -> Result<()> {
        if add.mesh.faces.is_empty() {
            return Err("Nothing to add".into());
        }
        add.mesh.validate()?;
        self.edit_world(
            add.item,
            None,
            Some((add.mesh, add.weld, add.join)),
            add.label,
        )
    }

    pub(crate) fn set_curve(&mut self, edit: CurveEdit) -> Result<()> {
        if !edit.points.is_empty() && edit.points.len() < 2 {
            return Err("A curve needs at least two points".into());
        }
        let mesh = (!edit.points.is_empty())
            .then(|| curve_mesh(edit.points.clone(), edit.closed, edit.kind));
        self.edit_world(
            edit.item,
            edit.face,
            mesh.map(|m| (m, Vec::new(), Vec::new())),
            edit.label,
        )
    }

    /// Remove curve `face` (if any) and append `add` (world space) to the
    /// target mesh, as one undo step labelled `label`.
    fn edit_world(
        &mut self,
        item: Option<u64>,
        face: Option<u32>,
        add: Option<Addition>,
        label: String,
    ) -> Result<()> {
        self.require_idle()?;
        let before = self.scene.clone();
        let ids_before = self.identities.current.clone();
        let mut scene = self.scene.clone();
        let target = match item
            .filter(|id| scene.items.iter().any(|i| i.id == *id && i.is_mesh()))
            .or_else(|| scene.active().filter(|i| i.is_mesh()).map(|i| i.id))
        {
            Some(id) => id,
            None => {
                let next = scene.next_id.max(self.next_item_id);
                if next >= u64::MAX - 1 || scene.items.len() >= crate::MAX_SCENE_ITEMS {
                    return Err("Scene item/identity budget exhausted".into());
                }
                scene.next_id = next;
                scene.new_mesh()?;
                scene.active().ok_or("Missing new mesh")?.id
            }
        };
        let world = scene.world_matrix(target)?;
        if world.determinant().abs() < 1e-15 {
            return Err("The mesh's transform is flat; can't draw in it".into());
        }
        let inverse = world.inverse();
        let entry = scene.item_mut(target).ok_or("Missing mesh")?;
        let mut mesh: Mesh = (*entry.mesh).clone();
        if let Some(f) = face {
            let f = f as usize;
            if f >= mesh.faces.len() || !mesh.is_polyline(f) {
                return Err("The curve being edited no longer exists".into());
            }
            mesh = without_face(&mesh, f)?;
        }
        if let Some((mut add, weld, join)) = add {
            for v in &mut add.vertices {
                *v = inverse.transform_point3(*v);
            }
            let offset = mesh.vertices.len() as u32;
            crate::append_mesh(&mut mesh, add)?;
            if !weld.is_empty() || !join.is_empty() {
                let mut pairs: Vec<(u32, u32)> = Vec::new();
                for &(a, b) in &join {
                    if a >= offset || b >= offset {
                        return Err("A point to join no longer exists".into());
                    }
                    if a != b {
                        pairs.push((a, b));
                    }
                }
                for &(new, old) in &weld {
                    if old >= offset {
                        return Err("A point to join no longer exists".into());
                    }
                    pairs.push((new + offset, old));
                }
                mesh = crate::topology_pen::weld_geometry(&mesh, &pairs, false)?;
            }
        }
        entry.mesh = mesh.into();
        scene.selection.vertices.clear();
        scene.selection.edges.clear();
        scene.selection.faces.clear();
        if scene.selection.item != Some(target) {
            scene.select_item(Some(target));
        }
        self.scene = scene;
        self.next_item_id = self.next_item_id.max(self.scene.next_id);
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.synchronize(&self.scene))
        {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.record(label, before);
        Ok(())
    }
}
