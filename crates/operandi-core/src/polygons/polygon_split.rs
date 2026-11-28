//! SPL-B01/I01 Split: insert dividing edges through selected vertices.
//!
//! Selecting two vertices of a polygon and running Split
//! forces an edge between them; vertices can also be selected in succession
//! across polygons. Each consecutive pair in the ordered vertex selection is
//! one cut, applied in order to the mesh produced by the previous cuts:
//!
//! * A pair that is already joined by an edge, or shares no polygon, needs
//!   no cut and is skipped; when no pair cuts, nothing changes and there is
//!   no message (make-split-2026-11-05 rule 14).
//! * Otherwise the pair must be two nonadjacent corners of exactly one surface
//!   polygon, which is divided into two polygons that keep its winding.
//! * A cut is refused when its pieces would lose area or reverse orientation,
//!   or, for a planar polygon, when the new edge would leave the polygon
//!   (a concave corner in the way). Nonplanar polygons are cut on the Newell
//!   orientation test alone.
//!
//! All cuts form one undoable edit; any refusal leaves the scene unchanged.
use crate::{
    Editor, Mesh, Mode, Result,
    polygon_triple::{
        FaceSplitOrigin, TRIPLE_AREA_TOLERANCE, TRIPLE_LENGTH_TOLERANCE, chart, remap_mesh,
        require_simple,
    },
};
use glam::DVec3;
use std::collections::BTreeMap;

/// Newell area vector (twice the vector area) of the given corner positions.
fn newell(points: &[DVec3]) -> DVec3 {
    (0..points.len()).fold(DVec3::ZERO, |n, i| {
        n + points[i].cross(points[(i + 1) % points.len()])
    })
}

/// One cut of `face` between its corners `a` and `b` (corner ordinals).
/// Returns the two pieces' corner ordinals in source corner order.
fn cut_pieces(mesh: &Mesh, face: usize, a: usize, b: usize) -> Result<[Vec<usize>; 2]> {
    let n = mesh.faces[face].vertices.len();
    let walk = |from: usize, to: usize| {
        let mut out = vec![from];
        let mut i = from;
        while i != to {
            i = (i + 1) % n;
            out.push(i);
        }
        out
    };
    // The second piece starts at `a` as well: [a, b, b+1, …, a-1].
    let mut second = walk(b, a);
    second.rotate_right(1);
    let pieces = [walk(a, b), second];
    let position = |c: usize| mesh.vertices[mesh.faces[face].vertices[c] as usize];
    let all: Vec<DVec3> = (0..n).map(position).collect();
    let whole = newell(&all);
    let lo = all.iter().copied().fold(DVec3::INFINITY, DVec3::min);
    let hi = all.iter().copied().fold(DVec3::NEG_INFINITY, DVec3::max);
    let extent = (hi - lo).length();
    for piece in &pieces {
        let points: Vec<DVec3> = piece.iter().map(|&c| position(c)).collect();
        let area = newell(&points);
        if area.length() <= 2. * TRIPLE_AREA_TOLERANCE * extent * extent || area.dot(whole) <= 0. {
            return Err("Split would fold or flatten the polygon; nothing changed".into());
        }
    }
    if let Ok(plane) = chart(&all) {
        for piece in &pieces {
            let flat: Vec<_> = piece.iter().map(|&c| plane.points[c]).collect();
            require_simple(&flat, TRIPLE_LENGTH_TOLERANCE * plane.extent).map_err(|_| {
                "Split edge would leave the polygon (concave corner in the way); nothing changed"
                    .to_string()
            })?;
        }
    }
    Ok(pieces)
}

impl Editor {
    pub(crate) fn split_polygon(&mut self) -> Result<()> {
        if self.i2_active() {
            return Err("Finish or Cancel Extrude before Split".into());
        }
        if self.session.pen_draft.is_some() {
            return Err("Finish or Cancel Pen before Split".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish selecting vertices before Split".into());
        }
        if self.scene.selection.mode != Mode::Vertex {
            return Err("Split requires Vertex mode".into());
        }
        self.scene.validate()?;
        let selected = self.scene.selection.vertices.clone();
        if selected.len() < 2 {
            return Err("Select two or more vertices on a polygon to Split".into());
        }
        if selected.windows(2).any(|w| w[0] == w[1]) {
            return Err("Split needs distinct consecutive vertices".into());
        }
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh before Split")?;
        let item_id = item.id;
        let source_mesh: Mesh = (*item.mesh).clone();
        if selected
            .iter()
            .any(|&v| v as usize >= source_mesh.vertices.len())
        {
            return Err("Missing selected vertex".into());
        }
        let ids = self
            .identities
            .current
            .meshes
            .get(&item_id)
            .ok_or("Missing mesh identity owner")?;
        let mut ids: crate::MeshIdentity = (**ids).clone();
        let mut mesh = source_mesh;
        let mut face_selection = self.scene.selection.faces.clone();
        let mut cuts = 0usize;
        for pair in selected.windows(2) {
            let (p, q) = (pair[0], pair[1]);
            if mesh.edges().contains(&(p.min(q), p.max(q))) {
                continue;
            }
            let candidates: Vec<(usize, usize, usize)> = mesh
                .faces
                .iter()
                .enumerate()
                .filter(|(f, _)| {
                    !mesh.is_polyline(*f)
                        && !mesh.attributes.degenerate_faces.contains(&(*f as u32))
                })
                .filter_map(|(f, face)| {
                    let a = face.vertices.iter().position(|&v| v == p)?;
                    let b = face.vertices.iter().position(|&v| v == q)?;
                    Some((f, a, b))
                })
                .collect();
            let (face_index, a, b) = match candidates[..] {
                // No polygon in common: nothing to cut, no message
                // (make-split-2026-11-05 rule 14).
                [] => continue,
                [one] => one,
                _ => {
                    return Err(format!(
                        "Split: vertices {p} and {q} share more than one polygon; nothing changed"
                    ));
                }
            };
            let [first, second] = cut_pieces(&mesh, face_index, a, b)?;
            let mut origins: Vec<_> = (0..mesh.faces.len())
                .map(|face| FaceSplitOrigin {
                    face,
                    corners: None,
                })
                .collect();
            if origins.len() >= crate::MAX_MESH_ELEMENTS {
                return Err("Split exceeds the mesh face budget".into());
            }
            let appended = u32::try_from(origins.len()).map_err(|_| "Mesh face budget exceeded")?;
            origins[face_index].corners = Some(first);
            origins.push(FaceSplitOrigin {
                face: face_index,
                corners: Some(second),
            });
            let mut pieces = BTreeMap::new();
            pieces.insert(face_index as u32, vec![face_index as u32, appended]);
            let next = remap_mesh(&mesh, &origins, &pieces)?;
            next.validate()?;
            ids = self
                .identities
                .with_ids(|n| ids.split_faces(&mesh, &next, &origins, n))?;
            if face_selection.contains(&(face_index as u32)) {
                face_selection.push(appended);
            }
            mesh = next;
            cuts += 1;
        }
        // Already connected, or nothing in common: nothing changes and no
        // message (rule 14).
        if cuts == 0 {
            return Ok(());
        }
        let before = self.scene.clone();
        let identities_before = self.identities.current.clone();
        self.scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
        self.scene.selection.faces = face_selection;
        self.identities.current.meshes.insert(item_id, ids.into());
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.observe(&self.scene))
        {
            self.scene = before;
            self.identities.current = identities_before;
            return Err(e);
        }
        self.record("Split".into(), before);
        Ok(())
    }
}

/// Cut polygon `face` between its corners `a` and `b` (corner ordinals;
/// pure, no identities). The first piece keeps the slot, the second is
/// appended.
pub(crate) fn cut_face(mesh: &Mesh, face: usize, a: usize, b: usize) -> Result<Mesh> {
    let [first, second] = cut_pieces(mesh, face, a, b)?;
    let mut origins: Vec<_> = (0..mesh.faces.len())
        .map(|face| FaceSplitOrigin {
            face,
            corners: None,
        })
        .collect();
    let appended = u32::try_from(origins.len()).map_err(|_| "Mesh face budget exceeded")?;
    origins[face].corners = Some(first);
    origins.push(FaceSplitOrigin {
        face,
        corners: Some(second),
    });
    let mut pieces = BTreeMap::new();
    pieces.insert(face as u32, vec![face as u32, appended]);
    let next = remap_mesh(mesh, &origins, &pieces)?;
    next.validate()?;
    Ok(next)
}
