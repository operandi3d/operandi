//! Spin Quad: turn the edge shared by two equal-sided polygons one step
//! around their outline, using explicit source-corner donors for corner data.
use crate::{
    Editor, Mesh, Mode, Result,
    polygon_triple::{TRIPLE_AREA_TOLERANCE, TRIPLE_LENGTH_TOLERANCE, chart, require_simple},
};
use std::collections::{BTreeMap, BTreeSet};

type Donors = [(usize, Vec<(usize, usize)>); 2];
fn remap_rows<T: Clone>(rows: &mut [Vec<T>], donors: &Donors) {
    let source = rows.to_vec();
    for (face, corners) in donors {
        rows[*face] = corners.iter().map(|&(f, c)| source[f][c].clone()).collect();
    }
}
fn remap_maps<T: Clone>(maps: &mut BTreeMap<String, Vec<Vec<T>>>, donors: &Donors) {
    for rows in maps.values_mut() {
        remap_rows(rows, donors);
    }
}
/// Newell area vector (twice the vector area) of the given positions.
fn newell(points: &[glam::DVec3]) -> glam::DVec3 {
    (0..points.len()).fold(glam::DVec3::ZERO, |n, i| {
        n + points[i].cross(points[(i + 1) % points.len()])
    })
}
/// A spun polygon must face the pair's side with real area and, when planar,
/// stay simple. Nonplanar pairs (curved surfaces) use the facing test alone.
fn require_unfolded(points: &[glam::DVec3], side: glam::DVec3) -> Result<()> {
    let lo = points
        .iter()
        .copied()
        .fold(glam::DVec3::INFINITY, glam::DVec3::min);
    let hi = points
        .iter()
        .copied()
        .fold(glam::DVec3::NEG_INFINITY, glam::DVec3::max);
    let extent = (hi - lo).length();
    let area = newell(points);
    if area.length() <= 2. * TRIPLE_AREA_TOLERANCE * extent * extent || area.dot(side) <= 0. {
        return Err("Spin Quad would fold a polygon (the pair's outline is too concave)".into());
    }
    if let Ok(c) = chart(points) {
        require_simple(&c.points, TRIPLE_LENGTH_TOLERANCE * c.extent)
            .map_err(|_| "Spin Quad would make a self-intersecting polygon".to_string())?;
    }
    Ok(())
}
/// Spin the edge shared by two adjacent polygons with the same corner count
/// one step around their combined outline (Spin Quads works on two polygons with
/// the same edge number, keeping the surrounding mesh).
fn spin_mesh(source: &Mesh, selected: [usize; 2]) -> Result<Mesh> {
    let [a, b] = selected;
    let fa = &source.faces[a];
    let fb = &source.faces[b];
    let n = fa.vertices.len();
    if source.is_polyline(a)
        || source.is_polyline(b)
        || source.attributes.degenerate_faces.contains(&(a as u32))
        || source.attributes.degenerate_faces.contains(&(b as u32))
    {
        return Err("Spin Quad requires two ordinary nondegenerate polygons".into());
    }
    if fb.vertices.len() != n {
        return Err("Spin Quad requires two polygons with the same number of edges".into());
    }
    let points = |vs: &[u32]| {
        vs.iter()
            .map(|&v| source.vertices[v as usize])
            .collect::<Vec<_>>()
    };
    let mut shared = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if fa.vertices[i] == fb.vertices[(j + 1) % n]
                && fa.vertices[(i + 1) % n] == fb.vertices[j]
            {
                shared.push((i, j));
            }
        }
    }
    if shared.len() != 1 {
        return Err("Spin Quad needs exactly one oppositely oriented shared edge".into());
    }
    let (i, j) = shared[0];
    // Combined outline: all of A from the shared edge's far end, then B's
    // corners that are not on the shared edge.
    let mut boundary: Vec<u32> = (1..=n).map(|k| fa.vertices[(i + k) % n]).collect();
    boundary.extend((2..n).map(|k| fb.vertices[(j + k) % n]));
    if boundary.iter().copied().collect::<BTreeSet<_>>().len() != boundary.len() {
        return Err("Spin Quad requires a simple combined outline".into());
    }
    let m = boundary.len(); // 2n - 2
    let old = (
        boundary[0].min(boundary[n - 1]),
        boundary[0].max(boundary[n - 1]),
    );
    let new = (boundary[1].min(boundary[n]), boundary[1].max(boundary[n]));
    let incidence = source
        .faces
        .iter()
        .filter(|f| {
            (0..f.vertices.len()).any(|k| {
                let x = f.vertices[k];
                let y = f.vertices[(k + 1) % f.vertices.len()];
                (x.min(y), x.max(y)) == old
            })
        })
        .count();
    if incidence != 2 || source.edges().contains(&new) {
        return Err("Spin Quad cannot replace an occupied or exceptional edge".into());
    }
    if source
        .attributes
        .extrude_edge_uses
        .values()
        .flatten()
        .any(|u| u[0] as usize == a || u[0] as usize == b)
    {
        return Err("Spin Quad cannot preserve data on an exceptional edge".into());
    }
    // A' = [outline[n], outline[1..n]]; B' = [outline[0], outline[1], outline[n..m]].
    let mut loop_a = vec![boundary[n]];
    loop_a.extend_from_slice(&boundary[1..n]);
    let mut loop_b = vec![boundary[0], boundary[1]];
    loop_b.extend_from_slice(&boundary[n..m]);
    let loops = [loop_a, loop_b];
    let side = newell(&points(&fa.vertices)) + newell(&points(&fb.vertices));
    for l in &loops {
        require_unfolded(&points(l), side)?;
    }
    let mut donors: Donors = [(a, Vec::new()), (b, Vec::new())];
    for n in 0..2 {
        let own = selected[n];
        let other = selected[1 - n];
        for &v in &loops[n] {
            let (f, c) = if let Some(c) = source.faces[own].vertices.iter().position(|&x| x == v) {
                (own, c)
            } else {
                (
                    other,
                    source.faces[other]
                        .vertices
                        .iter()
                        .position(|&x| x == v)
                        .ok_or("Missing Spin corner donor")?,
                )
            };
            donors[n].1.push((f, c));
        }
    }
    let mut out = source.clone();
    for n in 0..2 {
        let own = selected[n];
        let all = donors[n]
            .1
            .iter()
            .map(|&(f, c)| source.faces[f].uv.as_ref().map(|uv| uv[c]))
            .collect::<Vec<_>>();
        if all.iter().any(Option::is_some) && all.iter().any(Option::is_none) {
            return Err("Spin Quad cannot represent mixed legacy UV presence".into());
        }
        out.faces[own].vertices = loops[n].clone();
        out.faces[own].uv = if all[0].is_some() {
            Some(all.into_iter().flatten().collect())
        } else {
            None
        };
    }
    out.attributes
        .creases
        .retain(|c| (c.a.min(c.b), c.a.max(c.b)) != old);
    remap_maps(&mut out.uv_maps, &donors);
    remap_maps(&mut out.attributes.uv_records, &donors);
    remap_maps(&mut out.attributes.corner_weights, &donors);
    remap_maps(&mut out.attributes.corner_colors, &donors);
    remap_maps(&mut out.attributes.corner_normals, &donors);
    for map in out.attributes.typed_maps.values_mut() {
        if !map.corners.is_empty() {
            remap_rows(&mut map.corners, &donors);
        }
    }
    out.validate()?;
    Ok(out)
}
impl Editor {
    pub(crate) fn spin_quads(&mut self) -> Result<()> {
        if self.i2_active() || self.session.pen_draft.is_some() {
            return Err("Finish or Cancel the active tool before Spin Quad".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish selection before Spin Quad".into());
        }
        if self.scene.selection.mode != Mode::Polygon || self.scene.selection.faces.len() != 2 {
            return Err("Select exactly two adjacent polygons in Polygon mode".into());
        }
        self.scene.validate()?;
        let mut indices = [
            self.scene.selection.faces[0] as usize,
            self.scene.selection.faces[1] as usize,
        ];
        indices.sort_unstable();
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh before Spin Quad")?;
        if indices[0] == indices[1] || indices[1] >= item.mesh.faces.len() {
            return Err("Select two distinct existing polygons".into());
        }
        let id = item.id;
        let mesh = spin_mesh(&item.mesh, indices)?;
        let ids = self
            .identities
            .current
            .meshes
            .get(&id)
            .ok_or("Missing mesh identity owner")?
            .clone();
        let next = self
            .identities
            .with_ids(|n| ids.spin_faces(&item.mesh, &mesh, indices, n))?;
        let before = self.scene.clone();
        self.scene.active_mut().ok_or("Missing active mesh")?.mesh = mesh.into();
        self.identities.current.meshes.insert(id, next.into());
        self.scene.validate()?;
        self.identities.observe(&self.scene)?;
        self.record("Spin Quad".into(), before);
        Ok(())
    }
}
