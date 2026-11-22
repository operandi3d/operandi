//! Polygon tab clean-up commands (modelling parity batch 6): Triple Fan and
//! Collapse Selection. Spec: modelling-parity-specs-2026-11-17
//! TOPOLOGY-CLEANUP-TOOLS v1 (TC-01..12). Open readings TC-Q2/Q3 in
//! docs/SPECIFICATION_QUESTIONS.md.
//!
//! Both are one undo step and leave nothing selected (TC-02, TC-12). Corner
//! data (UVs, corner maps) follows the source corners; Collapse Selection's
//! new centre corners blend the region's outline corners equally.
use crate::edge_commands::{OutFace, rebuild};
use crate::{Editor, Mesh, Mode, Result};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

fn key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// TC-01/03: each selected polygon that uses `pivot` becomes a fan of
/// triangles around it, keeping its winding (N corners → N − 2). Polygons
/// without the pivot, triangles and polylines are left as they are.
/// Returns `None` when nothing would change.
pub fn triple_fan_geometry(mesh: &Mesh, faces: &[u32], pivot: u32) -> Result<Option<Mesh>> {
    let chosen: BTreeSet<u32> = faces.iter().copied().collect();
    let mut out_faces: Vec<OutFace> = Vec::with_capacity(mesh.faces.len());
    let mut changed = false;
    for (fi, f) in mesh.faces.iter().enumerate() {
        let n = f.vertices.len();
        let start = f.vertices.iter().position(|&v| v == pivot);
        match start {
            Some(k) if chosen.contains(&(fi as u32)) && n > 3 && !mesh.is_polyline(fi) => {
                changed = true;
                for i in 1..n - 1 {
                    let corners = [k, (k + i) % n, (k + i + 1) % n];
                    out_faces.push(OutFace {
                        template: fi,
                        vertices: corners.iter().map(|&c| f.vertices[c]).collect(),
                        donors: corners.iter().map(|&c| Some((fi, c))).collect(),
                        blends: Vec::new(),
                    });
                }
            }
            _ => out_faces.push(OutFace::copy(mesh, fi, f.vertices.clone())),
        }
    }
    if !changed {
        return Ok(None);
    }
    let out = rebuild(mesh, mesh.vertices.clone(), out_faces, &[], "Triple Fan")?;
    out.validate()
        .map_err(|e| format!("Triple Fan result failed validation ({e}); nothing changed"))?;
    Ok(Some(out))
}

/// TC-10/12 (TC-Q3 provisional: one centre per connected region): every
/// region of selected polygons is replaced by triangles from each outline
/// edge to one new point at the average of the outline points. Inner
/// points used by nothing else are removed. A region whose outline isn't
/// one loop (a ring, or the whole of a closed mesh) is refused.
pub fn collapse_selection_geometry(mesh: &Mesh, faces: &[u32]) -> Result<Mesh> {
    let n = mesh.faces.len();
    let chosen: BTreeSet<usize> = faces
        .iter()
        .map(|&f| f as usize)
        .filter(|&f| f < n && !mesh.is_polyline(f))
        .collect();
    if chosen.is_empty() {
        return Err("Collapse Selection: select polygons first".into());
    }
    // Regions: selected polygons joined through shared edges.
    let edge_faces = mesh.edge_faces();
    let mut region = vec![usize::MAX; n];
    let mut regions: Vec<Vec<usize>> = Vec::new();
    for &f in &chosen {
        if region[f] != usize::MAX {
            continue;
        }
        let id = regions.len();
        let mut stack = vec![f];
        let mut members = Vec::new();
        region[f] = id;
        while let Some(g) = stack.pop() {
            members.push(g);
            let v = &mesh.faces[g].vertices;
            for i in 0..v.len() {
                for &h in edge_faces
                    .get(&key(v[i], v[(i + 1) % v.len()]))
                    .into_iter()
                    .flatten()
                {
                    if chosen.contains(&h) && region[h] == usize::MAX {
                        region[h] = id;
                        stack.push(h);
                    }
                }
            }
        }
        regions.push(members);
    }
    let mut vertices = mesh.vertices.clone();
    let mut new_faces: Vec<OutFace> = Vec::new();
    let mut touched_points: BTreeSet<u32> = BTreeSet::new();
    for members in &regions {
        let set: BTreeSet<usize> = members.iter().copied().collect();
        // Directed outline edges a → b (with the corner of a in its polygon),
        // following the polygons' winding.
        let mut next: BTreeMap<u32, (u32, usize, usize)> = BTreeMap::new();
        for &g in members {
            let v = &mesh.faces[g].vertices;
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                touched_points.insert(a);
                let inside = edge_faces
                    .get(&key(a, b))
                    .into_iter()
                    .flatten()
                    .filter(|h| set.contains(h))
                    .count();
                if inside == 1 && next.insert(a, (b, g, i)).is_some() {
                    return Err(
                        "Collapse Selection: a region touches itself at a point; select a simpler patch"
                            .into(),
                    );
                }
            }
        }
        let Some((&first, _)) = next.iter().next() else {
            return Err(
                "Collapse Selection: the selection has no outline (a whole closed mesh); nothing changed"
                    .into(),
            );
        };
        let mut outline = vec![first];
        let mut at = first;
        loop {
            let (b, _, _) = next[&at];
            if b == first {
                break;
            }
            if outline.len() > next.len() {
                return Err("Collapse Selection: the outline doesn't close".into());
            }
            outline.push(b);
            at = b;
        }
        if outline.len() != next.len() {
            return Err(
                "Collapse Selection: the selection has a hole (more than one outline); select a patch without holes"
                    .into(),
            );
        }
        let centre = outline
            .iter()
            .map(|&p| mesh.vertices[p as usize])
            .sum::<DVec3>()
            / outline.len() as f64;
        let c = vertices.len() as u32;
        vertices.push(centre);
        // The centre's corner data: the outline corners, equally weighted.
        let w = 1. / outline.len() as f64;
        let blend: Vec<((usize, usize), f64)> = outline
            .iter()
            .map(|p| ((next[p].1, next[p].2), w))
            .collect();
        for &a in &outline {
            let (b, g, i) = next[&a];
            let len = mesh.faces[g].vertices.len();
            new_faces.push(OutFace {
                template: g,
                vertices: vec![a, b, c],
                donors: vec![Some((g, i)), Some((g, (i + 1) % len)), None],
                blends: vec![None, None, Some(blend.clone())],
            });
        }
    }
    let mut out_faces: Vec<OutFace> = (0..n)
        .filter(|f| !chosen.contains(f))
        .map(|f| OutFace::copy(mesh, f, mesh.faces[f].vertices.clone()))
        .collect();
    out_faces.extend(new_faces);
    let out = rebuild(mesh, vertices, out_faces, &[], "Collapse Selection")?;
    // Inner points the region orphaned go; points used before stay.
    let out = crate::mesh_cleanup::remove_unreferenced(&out, Some(&touched_points))?.mesh;
    out.validate().map_err(|e| {
        format!("Collapse Selection result failed validation ({e}); nothing changed")
    })?;
    Ok(out)
}

impl Editor {
    fn fan_collapse_target(&self, what: &str) -> Result<u64> {
        self.require_idle()?;
        if self.scene.selection.mode != Mode::Polygon {
            return Err(format!("{what} works in Polygon mode: select polygons"));
        }
        let item = self
            .scene
            .active()
            .ok_or_else(|| format!("Select a mesh for {what}"))?;
        if self.scene.selection.faces.is_empty() {
            return Err(format!("{what}: select polygons first"));
        }
        Ok(item.id)
    }

    /// Triple Fan (TC-01..03): fan the selected polygons from the last
    /// selected vertex; no vertex, or none on a selected polygon: a message
    /// and no change.
    pub(crate) fn triple_fan(&mut self) -> Result<()> {
        let item = self.fan_collapse_target("Triple Fan")?;
        let Some(&pivot) = self.scene.selection.vertices.last() else {
            return Err("Triple Fan: also select the vertex to fan from (Vertex mode, then back to Polygon mode)".into());
        };
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let Some(out) = triple_fan_geometry(mesh, &self.scene.selection.faces, pivot)? else {
            return Err("Triple Fan: the selected vertex isn't a corner of a selected polygon with four or more corners".into());
        };
        let before = self.scene.clone();
        self.scene.selection.vertices.clear();
        self.scene.selection.edges.clear();
        self.publish_polygon_edit_from(item, out, "Triple Fan", Vec::new(), before)
    }

    /// Collapse Selection (TC-10..12).
    pub(crate) fn collapse_selection(&mut self) -> Result<()> {
        let item = self.fan_collapse_target("Collapse Selection")?;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let out = collapse_selection_geometry(mesh, &self.scene.selection.faces)?;
        let before = self.scene.clone();
        self.scene.selection.vertices.clear();
        self.scene.selection.edges.clear();
        self.publish_polygon_edit_from(item, out, "Collapse Selection", Vec::new(), before)
    }
}

impl Editor {
    /// The first background surface hit along a world ray (front faces of
    /// the shown, inactive meshes), for Retopo Sketch and Topology Pen.
    pub fn background_hit(&self, origin: DVec3, dir: DVec3) -> Option<DVec3> {
        let background = self.background_cache.get(&self.scene, None).ok()?;
        background.ray(origin, dir, false).map(|(_, hit)| hit.point)
    }

    /// Topology Pen Duplicate (TC-45): edge `a`–`b` of `item` (the active
    /// mesh) pulls out a new quad; its new edge is the old one moved by
    /// `offset` (world) and laid on the nearest background surface (the
    /// offset point itself when there's no background). One undo step.
    pub(crate) fn topology_duplicate(
        &mut self,
        item: u64,
        a: u32,
        b: u32,
        offset: DVec3,
    ) -> Result<()> {
        self.require_idle()?;
        if !offset.is_finite() || offset.length() < 1e-9 {
            return Err("Topology Pen: drag the edge out to make a strip".into());
        }
        let active = self.scene.active().ok_or("Select a mesh")?;
        if active.id != item {
            return Err("Topology Pen works on the active mesh".into());
        }
        let world = self.scene.world_matrix(item)?;
        let local = world.inverse();
        let mesh: Mesh = (*active.mesh).clone();
        let n = mesh.vertices.len() as u32;
        if a >= n || b >= n || a == b {
            return Err("Topology Pen: that edge isn't in the mesh".into());
        }
        // The polygon on the edge, and the edge's direction in it.
        let found = mesh.faces.iter().enumerate().find_map(|(fi, f)| {
            let v = &f.vertices;
            let k = v.len();
            (0..k).find_map(|i| {
                let (x, y) = (v[i], v[(i + 1) % k]);
                if (x, y) == (a, b) {
                    Some((fi, i, (i + 1) % k, true))
                } else if (x, y) == (b, a) {
                    Some((fi, (i + 1) % k, i, false))
                } else {
                    None
                }
            })
        });
        let Some((template, ca, cb, forward)) = found.filter(|&(fi, ..)| !mesh.is_polyline(fi))
        else {
            return Err("Topology Pen: Shift+drag an edge of a polygon".into());
        };
        let background = self.background_cache.get(&self.scene, None)?;
        let land = |p: DVec3| background.nearest(p).map_or(p, |hit| hit.point);
        let new_a = land(world.transform_point3(mesh.vertices[a as usize]) + offset);
        let new_b = land(world.transform_point3(mesh.vertices[b as usize]) + offset);
        let mut vertices = mesh.vertices.clone();
        let (ia, ib) = (n, n + 1);
        vertices.push(local.transform_point3(new_a));
        vertices.push(local.transform_point3(new_b));
        let copy = |c: usize| Some(vec![((template, c), 1.)]);
        // The shared edge runs the other way in the new quad, so both
        // polygons face the same side.
        let (verts, donors, blends) = if forward {
            (
                vec![b, a, ia, ib],
                vec![Some((template, cb)), Some((template, ca)), None, None],
                vec![None, None, copy(ca), copy(cb)],
            )
        } else {
            (
                vec![a, b, ib, ia],
                vec![Some((template, ca)), Some((template, cb)), None, None],
                vec![None, None, copy(cb), copy(ca)],
            )
        };
        let mut faces: Vec<OutFace> = (0..mesh.faces.len())
            .map(|f| OutFace::copy(&mesh, f, mesh.faces[f].vertices.clone()))
            .collect();
        faces.push(OutFace {
            template,
            vertices: verts,
            donors,
            blends,
        });
        let out = rebuild(&mesh, vertices, faces, &[(ia, a), (ib, b)], "Topology Pen")?;
        out.validate()
            .map_err(|e| format!("Topology Pen result failed validation ({e}); nothing changed"))?;
        let mut selection_faces = self.scene.selection.faces.clone();
        selection_faces.retain(|&f| (f as usize) < out.faces.len());
        let before = self.scene.clone();
        self.publish_polygon_edit_from(item, out, "Topology Pen Duplicate", selection_faces, before)
    }
}
