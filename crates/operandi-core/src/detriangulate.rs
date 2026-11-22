//! Detriangulate (specifications/duplicate-tools, BEHAVIOR-DETRIANGULATE.md):
//! pairs of triangles back into quads by removing the edge they share,
//! where the pair is flat enough and the quad is a decent shape.
//!
//! - DT-11: the selected polygons in Polygon mode; nothing selected, or
//!   another mode, the whole mesh. Only triangles are considered.
//! - DT-20..23: a pair joins when its fold is within Flatness, every corner
//!   of the quad is below Max Angle and the quad is convex, and no Keep …
//!   Boundaries option separates the two.
//! - DT-24: pairs are taken best first (flattest, then squarest), each
//!   triangle joining at most one other, so a triangulated grid comes back
//!   as its quads.
//! - DT-25..27: points and their data stay; one undo step; nothing
//!   qualifying changes nothing and says so.
use crate::edge_commands::{OutFace, rebuild};
use crate::{Editor, Mesh, Mode, Result};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// DT section 3 options. Flatness is a percentage of the shared edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DetriangulateOptions {
    pub flatness: f64,
    pub max_angle: f64,
    pub keep_material: bool,
    pub keep_part: bool,
    pub keep_smoothing: bool,
    pub keep_sets: bool,
    pub keep_uv_seams: bool,
}
impl Default for DetriangulateOptions {
    fn default() -> Self {
        Self {
            flatness: 20.,
            max_angle: 170.,
            keep_material: true,
            keep_part: true,
            keep_smoothing: true,
            // Off (probe).
            keep_sets: false,
            keep_uv_seams: true,
        }
    }
}
impl DetriangulateOptions {
    pub fn validate(&self) -> Result<()> {
        if !(self.flatness.is_finite() && (0. ..=1000.).contains(&self.flatness)) {
            return Err("Flatness must be from 0 % to 1000 %".into());
        }
        if !(self.max_angle.is_finite() && self.max_angle > 0. && self.max_angle <= 180.) {
            return Err("Max Angle must be more than 0° and at most 180°".into());
        }
        Ok(())
    }
}

/// What DT-27 says when no pair qualifies.
pub const NOTHING_JOINED: &str = "No triangle pairs met the settings.";

/// A pair that may join: the two triangles, the quad's corners as
/// (point, donor polygon, donor corner), and its score (lower is better).
#[derive(Clone)]
struct Pair {
    a: usize,
    b: usize,
    corners: [(u32, usize, usize); 4],
    score: f64,
}

/// The corner angles (degrees) of a polygon and whether it is convex.
fn corners(points: &[DVec3]) -> Option<(Vec<f64>, bool)> {
    let n = points.len();
    let mut normal = DVec3::ZERO;
    for i in 0..n {
        normal += points[i].cross(points[(i + 1) % n]);
    }
    let normal = normal.normalize_or_zero();
    if normal == DVec3::ZERO {
        return None;
    }
    let mut angles = Vec::with_capacity(n);
    let mut convex = true;
    for i in 0..n {
        let (prev, cur, next) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
        let (u, v) = (prev - cur, next - cur);
        if u.length() < 1e-12 || v.length() < 1e-12 {
            return None;
        }
        angles.push(u.angle_between(v).to_degrees());
        if (cur - prev).cross(next - cur).dot(normal) <= 0. {
            convex = false;
        }
    }
    Some((angles, convex))
}

/// Whether two polygons' UVs differ at a shared point (a UV seam there).
fn uv_seam(mesh: &Mesh, (fa, ca): (usize, usize), (fb, cb): (usize, usize)) -> bool {
    mesh.uv_maps
        .values()
        .any(|rows| match (rows[fa][ca], rows[fb][cb]) {
            (Some(x), Some(y)) => x.distance(y) > 1e-9,
            (None, None) => false,
            _ => true,
        })
        || match (&mesh.faces[fa].uv, &mesh.faces[fb].uv) {
            (Some(x), Some(y)) => x[ca].distance(y[cb]) > 1e-9,
            _ => false,
        }
}

/// The triangle pairs Detriangulate joins in `chosen` polygons of `mesh`
/// (all when empty), best first (DT-24), and which polygons they use. The
/// edit and its preview both come from here, so they can't disagree.
fn joined_pairs(
    mesh: &Mesh,
    chosen: &[u32],
    o: &DetriangulateOptions,
) -> Result<(BTreeMap<usize, Pair>, Vec<bool>)> {
    o.validate()?;
    let considered: BTreeSet<usize> = if chosen.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        chosen.iter().map(|&f| f as usize).collect()
    };
    let triangle = |f: usize| {
        considered.contains(&f) && !mesh.is_polyline(f) && mesh.faces[f].vertices.len() == 3
    };
    // Each directed edge of a considered triangle, and its corner.
    let mut uses: BTreeMap<(u32, u32), Vec<(usize, usize)>> = BTreeMap::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        if !triangle(fi) {
            continue;
        }
        for c in 0..3 {
            let (a, b) = (f.vertices[c], f.vertices[(c + 1) % 3]);
            uses.entry((a.min(b), a.max(b))).or_default().push((fi, c));
        }
    }
    let counts = mesh.edge_counts();
    let set_of = |f: usize| -> Vec<&String> {
        mesh.attributes
            .polygon_sets
            .iter()
            .filter(|(_, s)| s.contains(&(f as u32)))
            .map(|(k, _)| k)
            .collect()
    };
    let p = |v: u32| mesh.vertices[v as usize];
    let mut pairs = Vec::new();
    for (edge, list) in &uses {
        if list.len() != 2 || counts.get(edge).copied() != Some(2) {
            continue;
        }
        let [(fa, ca), (fb, cb)] = [list[0], list[1]];
        let (va, vb) = (&mesh.faces[fa].vertices, &mesh.faces[fb].vertices);
        // Opposite directions: A runs a→b, B runs b→a.
        let (a, b, c) = (va[ca], va[(ca + 1) % 3], va[(ca + 2) % 3]);
        if vb[cb] != b || vb[(cb + 1) % 3] != a {
            continue;
        }
        let d = vb[(cb + 2) % 3];
        let (ta, tb) = (&mesh.faces[fa], &mesh.faces[fb]);
        if (o.keep_material && ta.material != tb.material)
            || (o.keep_part && ta.part != tb.part)
            || (o.keep_smoothing
                && (ta.smooth != tb.smooth
                    || mesh.attributes.smoothing_labels.get(fa)
                        != mesh.attributes.smoothing_labels.get(fb)))
            || (o.keep_sets && set_of(fa) != set_of(fb))
            || (o.keep_uv_seams
                && (uv_seam(mesh, (fa, ca), (fb, (cb + 1) % 3))
                    || uv_seam(mesh, (fa, (ca + 1) % 3), (fb, cb))))
        {
            continue;
        }
        // DT-21: the fourth corner's height off the first triangle's plane.
        let normal = (p(b) - p(a)).cross(p(c) - p(a)).normalize_or_zero();
        let shared = p(a).distance(p(b));
        if normal == DVec3::ZERO || shared < 1e-12 {
            continue;
        }
        let fold = (p(d) - p(a)).dot(normal).abs() / shared;
        if fold > o.flatness / 100. + 1e-12 {
            continue;
        }
        // DT-22: every corner below Max Angle, and convex.
        let quad = [a, d, b, c];
        let Some((angles, convex)) = corners(&quad.map(p)) else {
            continue;
        };
        if !convex || angles.iter().any(|&x| x >= o.max_angle) {
            continue;
        }
        let square: f64 = angles.iter().map(|x| (x - 90.).abs()).sum::<f64>() / 360.;
        pairs.push(Pair {
            a: fa,
            b: fb,
            corners: [
                (a, fa, ca),
                (d, fb, (cb + 2) % 3),
                (b, fa, (ca + 1) % 3),
                (c, fa, (ca + 2) % 3),
            ],
            score: fold + square,
        });
    }
    // DT-24: best first; each triangle joins once.
    pairs.sort_by(|x, y| {
        x.score
            .total_cmp(&y.score)
            .then((x.a, x.b).cmp(&(y.a, y.b)))
    });
    let mut used = vec![false; mesh.faces.len()];
    let mut joined: BTreeMap<usize, Pair> = BTreeMap::new();
    for pair in pairs {
        if used[pair.a] || used[pair.b] {
            continue;
        }
        used[pair.a] = true;
        used[pair.b] = true;
        joined.insert(pair.a.min(pair.b), pair);
    }
    if joined.is_empty() {
        return Err(NOTHING_JOINED.into());
    }
    Ok((joined, used))
}

/// The edges Detriangulate would remove from `chosen` polygons of `mesh`
/// (all when empty): each joined pair's shared edge, (low, high) point.
pub fn detriangulate_removed_edges(
    mesh: &Mesh,
    chosen: &[u32],
    o: &DetriangulateOptions,
) -> Result<Vec<(u32, u32)>> {
    let (joined, _) = joined_pairs(mesh, chosen, o)?;
    // Corners run a, d, b, c: the shared edge is a–b.
    Ok(joined
        .values()
        .map(|pair| {
            let (a, b) = (pair.corners[0].0, pair.corners[2].0);
            (a.min(b), a.max(b))
        })
        .collect())
}

/// Detriangulate `chosen` polygons of `mesh` (all when empty). Returns the
/// new mesh and the quads and leftover triangles (DT-26), or an error.
pub fn detriangulate_geometry(
    mesh: &Mesh,
    chosen: &[u32],
    o: &DetriangulateOptions,
) -> Result<(Mesh, Vec<u32>)> {
    let (joined, used) = joined_pairs(mesh, chosen, o)?;
    let considered: BTreeSet<usize> = if chosen.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        chosen.iter().map(|&f| f as usize).collect()
    };
    let triangle = |f: usize| {
        considered.contains(&f) && !mesh.is_polyline(f) && mesh.faces[f].vertices.len() == 3
    };
    let mut faces = Vec::with_capacity(mesh.faces.len() - joined.len());
    let mut selected = Vec::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        if let Some(pair) = joined.get(&fi) {
            selected.push(faces.len() as u32);
            faces.push(OutFace {
                template: pair.a,
                vertices: pair.corners.iter().map(|c| c.0).collect(),
                donors: pair.corners.iter().map(|c| Some((c.1, c.2))).collect(),
                blends: Vec::new(),
            });
            continue;
        }
        if used[fi] {
            continue; // the second triangle of a joined pair
        }
        if triangle(fi) {
            selected.push(faces.len() as u32);
        }
        faces.push(OutFace {
            template: fi,
            vertices: f.vertices.clone(),
            donors: (0..f.vertices.len()).map(|c| Some((fi, c))).collect(),
            blends: Vec::new(),
        });
    }
    let out = rebuild(mesh, mesh.vertices.clone(), faces, &[], "Detriangulate")?;
    out.validate()?;
    Ok((out, selected))
}

impl Editor {
    /// The polygons Detriangulate looks at: the selected ones in Polygon
    /// mode, else all (empty).
    fn detriangulate_chosen(&self) -> Vec<u32> {
        if self.scene.selection.mode == Mode::Polygon {
            self.scene.selection.faces.clone()
        } else {
            Vec::new()
        }
    }

    /// The active mesh's edges Detriangulate with `options` would remove
    /// now, (low, high) point; for its preview. Same pairing as the edit.
    pub fn detriangulate_preview(&self, options: &DetriangulateOptions) -> Result<Vec<(u32, u32)>> {
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        detriangulate_removed_edges(mesh, &self.detriangulate_chosen(), options)
    }

    /// Detriangulate the active mesh, one undo step (DT-27).
    pub(crate) fn detriangulate(&mut self, options: DetriangulateOptions) -> Result<()> {
        let item = self.edge_insert_target("Detriangulate")?;
        options.validate()?;
        self.detriangulate_preset = options.clone();
        let polygon_mode = self.scene.selection.mode == Mode::Polygon;
        let chosen = self.detriangulate_chosen();
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, selected) = detriangulate_geometry(mesh, &chosen, &options)?;
        let faces = if polygon_mode {
            selected
        } else {
            self.scene.selection.faces.clone()
        };
        self.publish_polygon_edit(item, out, "Detriangulate", faces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;

    fn face(vertices: Vec<u32>, material: &str) -> Face {
        Face {
            vertices,
            uv: None,
            material: material.into(),
            smooth: false,
            part: None,
            material_label: None,
        }
    }

    /// An `n` x `n` grid in XZ, each quad split on one diagonal, or on
    /// alternating ones.
    fn triangulated(n: u32, alternate: bool) -> Mesh {
        let mut m = Mesh::default();
        for j in 0..=n {
            for i in 0..=n {
                m.vertices.push(DVec3::new(i as f64, 0., j as f64));
            }
        }
        for j in 0..n {
            for i in 0..n {
                let a = j * (n + 1) + i;
                let (b, c, d) = (a + n + 1, a + n + 2, a + 1);
                if alternate && (i + j) % 2 == 1 {
                    m.faces.push(face(vec![a, b, d], "M"));
                    m.faces.push(face(vec![b, c, d], "M"));
                } else {
                    m.faces.push(face(vec![a, b, c], "M"));
                    m.faces.push(face(vec![a, c, d], "M"));
                }
            }
        }
        m.validate().unwrap();
        m
    }

    #[test]
    fn triangulated_grids_come_back_as_quads() {
        // DT-30 and DT-31.
        for alternate in [false, true] {
            let (m, selected) =
                detriangulate_geometry(&triangulated(4, alternate), &[], &Default::default())
                    .unwrap();
            assert_eq!(m.faces.len(), 16, "alternate {alternate}");
            assert!(m.faces.iter().all(|f| f.vertices.len() == 4));
            assert_eq!(m.vertices.len(), 25);
            assert_eq!(selected.len(), 16);
        }
    }

    #[test]
    fn the_preview_lists_exactly_the_edges_the_edit_removes() {
        let strict = DetriangulateOptions {
            keep_material: true,
            ..Default::default()
        };
        let mut mixed = triangulated(4, true);
        for f in mixed.faces.iter_mut().step_by(3) {
            f.material = "Other".into();
        }
        for (mesh, chosen, o) in [
            (
                triangulated(4, false),
                vec![],
                DetriangulateOptions::default(),
            ),
            (
                triangulated(4, true),
                vec![0, 1, 2, 3, 4, 5],
                Default::default(),
            ),
            (mixed, vec![], strict),
        ] {
            let removed: BTreeSet<_> = detriangulate_removed_edges(&mesh, &chosen, &o)
                .unwrap()
                .into_iter()
                .collect();
            let (out, _) = detriangulate_geometry(&mesh, &chosen, &o).unwrap();
            let before: BTreeSet<_> = mesh.edges().into_iter().collect();
            let after: BTreeSet<_> = out.edges().into_iter().collect();
            let gone: BTreeSet<_> = before.difference(&after).copied().collect();
            assert_eq!(removed, gone);
            assert!(!removed.is_empty());
        }
        // Nothing to join: the preview refuses as the edit does.
        let quads = detriangulate_geometry(&triangulated(2, false), &[], &Default::default())
            .unwrap()
            .0;
        assert!(detriangulate_removed_edges(&quads, &[], &Default::default()).is_err());
    }

    #[test]
    fn a_triangulated_cube_keeps_its_corners() {
        // DT-32: 12 triangles to 6 quads; no pair across a cube edge.
        let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
        let mut m = Mesh {
            vertices: cube.vertices.clone(),
            ..Default::default()
        };
        for f in &cube.faces {
            let v = &f.vertices;
            m.faces.push(face(vec![v[0], v[1], v[2]], ""));
            m.faces.push(face(vec![v[0], v[2], v[3]], ""));
        }
        m.validate().unwrap();
        let (out, _) = detriangulate_geometry(&m, &[], &Default::default()).unwrap();
        assert_eq!(out.faces.len(), 6);
        assert!(out.faces.iter().all(|f| f.vertices.len() == 4));
    }

    #[test]
    fn material_boundaries_and_slivers_stay() {
        // DT-33.
        let mut m = Mesh {
            vertices: vec![
                DVec3::ZERO,
                DVec3::new(0., 0., 1.),
                DVec3::new(1., 0., 1.),
                DVec3::new(1., 0., 0.),
            ],
            ..Default::default()
        };
        m.faces.push(face(vec![0, 1, 2], "A"));
        m.faces.push(face(vec![0, 2, 3], "B"));
        let err = detriangulate_geometry(&m, &[], &Default::default()).unwrap_err();
        assert_eq!(err, NOTHING_JOINED);
        let off = DetriangulateOptions {
            keep_material: false,
            ..Default::default()
        };
        assert_eq!(
            detriangulate_geometry(&m, &[], &off).unwrap().0.faces.len(),
            1
        );
        // DT-34: a quad with a 175° corner stays two triangles.
        let mut s = Mesh {
            vertices: vec![
                DVec3::ZERO,
                DVec3::new(0., 0., 1.),
                DVec3::new(2., 0., 0.59),
                DVec3::new(4., 0., 0.),
            ],
            ..Default::default()
        };
        s.faces.push(face(vec![0, 1, 3], ""));
        s.faces.push(face(vec![1, 2, 3], ""));
        s.validate().unwrap();
        assert!(detriangulate_geometry(&s, &[], &Default::default()).is_err());
    }
}
