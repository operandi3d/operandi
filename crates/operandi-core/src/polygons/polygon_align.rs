//! ALN-B01 Align: make the polygons of a selected patch face one way.
//!
//! Align makes the selected polygons face the same
//! direction as the first polygon you select. With one polygon selected,
//! its whole edge-connected piece is
//! aligned to it; otherwise the selected polygons are aligned.
//!
//! Orientation spreads from the reference across edges shared by exactly two
//! selected surface polygons. Each other edge-connected piece of the
//! selection is aligned to its first selected face. Nonmanifold edges (three or
//! more uses) are not crossed. A selection that cannot be oriented
//! consistently (a nonorientable surface) is refused.
//!
//! The reversal keeps the first corner in place and reverses the rest
//! (`[a, b, c, d]` -> `[a, d, c, b]`), matching the admitted fixture loops.
//! Every corner-associated record moves with its source vertex; authored
//! normal and tangent-basis data on a reversed face are refused because their
//! transformation is unspecified.
use crate::{Editor, Mesh, Mode, PolygonRepresentation, Result};
use std::collections::BTreeSet;

/// Apply the anchored reversal to one corner row.
fn anchored_reversal<T>(row: &mut [T]) {
    if row.len() > 1 {
        row[1..].reverse();
    }
}

/// Authored normals or tangent bases on a face whose winding would change.
fn has_authored_orientation_data(mesh: &Mesh, face: usize) -> bool {
    let loop_ = &mesh.faces[face].vertices;
    mesh.attributes
        .corner_normals
        .values()
        .any(|map| map[face].iter().any(Option::is_some))
        || mesh.attributes.typed_maps.values().any(|map| {
            matches!(map.kind.as_str(), "normal3" | "tangent_basis6")
                && ((!map.corners.is_empty() && map.corners[face].iter().any(Option::is_some))
                    || loop_.iter().any(|v| map.points.contains_key(v)))
        })
}

/// Face slots that must be reversed, empty when the selection is already
/// consistent. Pure: never mutates `mesh`.
pub(crate) fn align_plan(mesh: &Mesh, selected: &[u32]) -> Result<BTreeSet<usize>> {
    if selected.is_empty() {
        return Err("Select polygons before Align".into());
    }
    let eligible = |fi: usize| {
        !mesh.is_polyline(fi)
            && !mesh
                .attributes
                .representations
                .get(fi)
                .is_some_and(|r| *r != PolygonRepresentation::Face)
            && !mesh.attributes.degenerate_faces.contains(&(fi as u32))
    };
    for &f in selected {
        let fi = f as usize;
        if fi >= mesh.faces.len() {
            return Err("Selected polygon no longer exists".into());
        }
        if !eligible(fi) {
            return Err("Align requires ordinary nondegenerate surface polygons".into());
        }
    }
    // Undirected edge -> surface uses (face, from, to).
    let mut uses: std::collections::BTreeMap<(u32, u32), Vec<(usize, u32)>> = Default::default();
    for (fi, face) in mesh.faces.iter().enumerate() {
        if !eligible(fi) {
            continue;
        }
        let n = face.vertices.len();
        for i in 0..n {
            let (x, y) = (face.vertices[i], face.vertices[(i + 1) % n]);
            uses.entry((x.min(y), x.max(y))).or_default().push((fi, x));
        }
    }
    let one: BTreeSet<usize> = selected.iter().map(|&f| f as usize).collect();
    let whole_piece = one.len() == 1;
    let in_scope = |fi: usize| whole_piece || one.contains(&fi);
    // flipped[f]: whether f is reversed in the result.
    let mut flipped: std::collections::BTreeMap<usize, bool> = Default::default();
    // Selection order: the first selected polygon of each piece is its
    // reference.
    let mut order: Vec<usize> = Vec::new();
    for &f in selected {
        if !order.contains(&(f as usize)) {
            order.push(f as usize);
        }
    }
    for &start in &order {
        if flipped.contains_key(&start) {
            continue;
        }
        flipped.insert(start, false);
        let mut stack = vec![start];
        while let Some(f) = stack.pop() {
            let face = &mesh.faces[f].vertices;
            let n = face.len();
            for i in 0..n {
                let (x, y) = (face[i], face[(i + 1) % n]);
                let edge_uses = &uses[&(x.min(y), x.max(y))];
                if edge_uses.len() != 2 {
                    continue;
                }
                let Some(&(g, from)) = edge_uses.iter().find(|(g, _)| *g != f) else {
                    continue;
                };
                if !in_scope(g) {
                    continue;
                }
                // Consistent neighbours traverse the shared edge in opposite
                // directions; `same` means g disagrees with f in the source.
                let same = from == x;
                let want = flipped[&f] ^ same;
                match flipped.get(&g) {
                    Some(&have) if have != want => {
                        return Err(
                            "Align cannot orient this selection consistently (nonorientable surface)"
                                .into(),
                        );
                    }
                    Some(_) => {}
                    None => {
                        flipped.insert(g, want);
                        stack.push(g);
                    }
                }
            }
        }
    }
    let out: BTreeSet<usize> = flipped
        .into_iter()
        .filter_map(|(f, flip)| flip.then_some(f))
        .collect();
    if let Some(&f) = out
        .iter()
        .find(|&&f| has_authored_orientation_data(mesh, f))
    {
        return Err(format!(
            "Align of authored normals or tangents is not supported (polygon {f})"
        ));
    }
    Ok(out)
}

/// Reverse each listed face with the anchored reversal and carry all corner
/// data with it; the exceptional-incidence ledger is rebuilt for the result,
/// which is validated.
pub(crate) fn align_mesh(source: &Mesh, faces: &BTreeSet<usize>) -> Result<Mesh> {
    let mut out = source.clone();
    for &face in faces {
        anchored_reversal(&mut out.faces[face].vertices);
        if let Some(uv) = &mut out.faces[face].uv {
            anchored_reversal(uv);
        }
        for rows in out.uv_maps.values_mut() {
            anchored_reversal(&mut rows[face]);
        }
        for rows in out.attributes.uv_records.values_mut() {
            anchored_reversal(&mut rows[face]);
        }
        for rows in out.attributes.corner_weights.values_mut() {
            anchored_reversal(&mut rows[face]);
        }
        for rows in out.attributes.corner_colors.values_mut() {
            anchored_reversal(&mut rows[face]);
        }
        for rows in out.attributes.corner_normals.values_mut() {
            anchored_reversal(&mut rows[face]);
        }
        for map in out.attributes.typed_maps.values_mut() {
            if !map.corners.is_empty() {
                anchored_reversal(&mut map.corners[face]);
            }
        }
    }
    // Rebuild the exceptional ledger exactly as Mesh::validate collects it.
    let mut edges: std::collections::BTreeMap<(u32, u32), Vec<[u32; 3]>> = Default::default();
    for (fi, f) in out.faces.iter().enumerate() {
        if out.is_polyline(fi) {
            continue;
        }
        let n = f.vertices.len();
        for i in 0..n {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % n]);
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push([fi as u32, a, b]);
        }
    }
    out.attributes.extrude_edge_uses = edges
        .into_iter()
        .filter(|(_, u)| u.len() > 2 || (u.len() == 2 && u[0][1..] == u[1][1..]))
        .map(|((a, b), u)| (format!("{a}:{b}"), u))
        .collect();
    out.validate()?;
    Ok(out)
}

impl Editor {
    /// Resolve the active mesh id and the face to reverse without mutation.
    fn align_target(&self) -> Result<(u64, BTreeSet<usize>)> {
        if self.i2_active() || self.session.pen_draft.is_some() {
            return Err("Finish or Cancel the active tool before Align".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish selection before Align".into());
        }
        if self.scene.selection.mode != Mode::Polygon {
            return Err("Align requires Polygon mode".into());
        }
        self.scene.validate()?;
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh before Align")?;
        Ok((
            item.id,
            align_plan(&item.mesh, &self.scene.selection.faces)?,
        ))
    }

    /// Read-only count of polygons Align would reverse now, from the same
    /// plan the command executes. Used for the acknowledgment.
    pub fn align_flip_count(&self) -> Result<usize> {
        self.align_target().map(|(_, faces)| faces.len())
    }

    pub(crate) fn align_polygons(&mut self) -> Result<()> {
        let (id, faces) = self.align_target()?;
        // An already consistent selection is a successful no-change: no history.
        if faces.is_empty() {
            return Ok(());
        }
        let item = self.scene.active().ok_or("Missing active mesh")?;
        let mesh = align_mesh(&item.mesh, &faces)?;
        let next = self
            .identities
            .current
            .meshes
            .get(&id)
            .ok_or("Missing mesh identity owner")?
            .anchored_reversed_faces(&item.mesh, &mesh, &faces)?;
        let before = self.scene.clone();
        self.scene.active_mut().ok_or("Missing active mesh")?.mesh = mesh.into();
        self.identities.current.meshes.insert(id, next.into());
        self.scene.validate()?;
        self.identities.observe(&self.scene)?;
        self.record("Align".into(), before);
        Ok(())
    }
}
