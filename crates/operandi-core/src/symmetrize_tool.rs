//! Symmetrize, the Duplicate tab's tool form (BEHAVIOR-DUPLICATE-PANEL
//! DX-40..45): delete one side of the active mesh across a plane and put a
//! mirror copy of the other side in its place, live, with its options in
//! Tool Properties. Polygons crossing the plane are cut at the plane first
//! (DX-42), so the kept side keeps its part of them; Merge welds the copy to
//! the kept half on the plane. With Topology on, points move to mirror
//! their partners instead (the Symmetry Symmetrize's points engine).
//!
//! DX-43 (DXQ-3, the maintainer's OK 2026-11-20): the whole active mesh
//! is used; a component selection is ignored and cleared.
//! The plane is in the mesh item's own space, like Symmetry's Symmetrize.
use crate::{Mesh, Result, Scene, SymmetrizeSide, Symmetry};
use glam::DVec3;
use std::collections::BTreeSet;

/// Symmetrize's options (DX-41) and starting values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Symmetrize {
    /// 0 X, 1 Y, 2 Z: the plane's normal axis.
    pub axis: usize,
    /// Positive (Positive to Negative): keep the + side and copy it over.
    pub direction: SymmetrizeSide,
    pub merge: bool,
    /// Moves the plane along the axis (metres, item space).
    pub offset: f64,
    pub topology: bool,
}
impl Default for Symmetrize {
    fn default() -> Self {
        Self {
            axis: 0,
            direction: SymmetrizeSide::Positive,
            merge: true,
            offset: 0.,
            topology: false,
        }
    }
}
impl Symmetrize {
    /// The plane as a Symmetry setting.
    pub fn symmetry(&self) -> Result<Symmetry> {
        if self.axis > 2 {
            return Err("Axis must be X, Y or Z".into());
        }
        if !self.offset.is_finite() {
            return Err("Offset must be a finite number".into());
        }
        Ok(Symmetry {
            axis: Some(self.axis),
            offset: self.offset,
            topology: self.topology,
            ..Symmetry::OFF
        })
    }
}

/// `mesh` cut along the plane: every polygon crossing it is split there.
pub fn cut_at_plane(mesh: &Mesh, symmetry: Symmetry) -> Result<Mesh> {
    let Some(normal) = symmetry.normal() else {
        return Ok(mesh.clone());
    };
    let tolerance = crate::symmetry::match_tolerance(mesh);
    // Only polygons with points on both sides need cutting.
    let crossing: BTreeSet<usize> = (0..mesh.faces.len())
        .filter(|&f| {
            let sides = mesh.faces[f]
                .vertices
                .iter()
                .filter_map(|&v| symmetry.side(mesh.vertices[v as usize], tolerance));
            let (mut neg, mut pos) = (false, false);
            for s in sides {
                neg |= s == crate::symmetry::Side::Negative;
                pos |= s == crate::symmetry::Side::Positive;
            }
            neg && pos
        })
        .collect();
    if crossing.is_empty() {
        return Ok(mesh.clone());
    }
    let origin: DVec3 = normal * symmetry.offset;
    let along = normal.any_orthonormal_vector();
    let ((out, _, _), _) =
        crate::slice::plane_cut(mesh, origin, normal, along, 0., true, &crossing)?;
    Ok(out)
}

/// The tool's result on the active mesh of `scene` (the whole mesh, DX-43).
pub(crate) fn preview(scene: &mut Scene, o: &Symmetrize) -> Result<()> {
    let symmetry = o.symmetry()?;
    let item = scene.active().ok_or("Select a mesh for Symmetrize")?;
    if !item.is_mesh() || item.mesh.faces.is_empty() {
        return Err("Select a mesh with polygons for Symmetrize".into());
    }
    // DX-43: a component selection is ignored.
    scene.clear_components();
    if o.topology {
        return scene.symmetrize(symmetry, o.direction);
    }
    let cut = cut_at_plane(&scene.active().ok_or("Select a mesh")?.mesh, symmetry)?;
    scene.active_mut().ok_or("Select a mesh")?.mesh = cut.into();
    scene.symmetrize_polygons(symmetry, o.direction, o.merge)
}
