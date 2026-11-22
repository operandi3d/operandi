//! Teapot primitive and Teapot Item. Spec: missing-tools-g2
//! G2-PLANE-TEAPOT v1, MT6-01..09.
//!
//! The shape is the public Utah Teapot: the 32 Bezier patches of the 1987
//! rounded-bottom version in `utah_teapot.patches` (credit and terms in
//! that file's header and the About window). Each patch is cut into
//! `TEAPOT_SEGMENTS` × `TEAPOT_SEGMENTS` quads; points that coincide where
//! patches meet are welded, and the rows that collapse to a pole (the lid
//! knob's top, the bottom) become triangles (MT6-04). Every polygon faces
//! out. The lid is part of the same mesh (MT6-03).
//!
//! Placement (MT6-01/02): Size is the bounding box's longest side (spout
//! tip to handle), Position its centre. With Axis Y, up is +Y, the spout
//! points −Z and the handle +Z; Axis X or Z turn "up" onto that axis.
use crate::{Face, Mesh, Result};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// The control points, Operandi's own text format (see its header).
const DATA: &str = include_str!("utah_teapot.patches");

/// Quads along each side of a patch (Operandi's own density, MT6-04).
pub const TEAPOT_SEGMENTS: u32 = 6;

/// Teapot tool settings (MT6 defaults).
#[derive(Clone, Debug, PartialEq)]
pub struct TeapotParameters {
    /// The bounding box's centre.
    pub position: DVec3,
    /// The bounding box's longest side, spout tip to handle.
    pub size: f64,
    /// The world axis that is up: 0 X, 1 Y, 2 Z.
    pub axis: usize,
    pub make_uvs: bool,
}
impl Default for TeapotParameters {
    fn default() -> Self {
        Self {
            position: DVec3::ZERO,
            size: 1.,
            axis: 1,
            make_uvs: true,
        }
    }
}

/// The 32 patches, 16 control points each (row by row), z up, spout +x.
fn patches() -> Result<Vec<[DVec3; 16]>> {
    let mut lines = DATA
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    let count = |line: Option<&str>, word: &str| -> Result<usize> {
        line.and_then(|l| l.strip_prefix(word))
            .and_then(|n| n.trim().parse().ok())
            .ok_or_else(|| format!("Teapot data: expected '{word} N'"))
    };
    let n = count(lines.next(), "points")?;
    let mut points = Vec::with_capacity(n);
    for _ in 0..n {
        let v: Vec<f64> = lines
            .next()
            .ok_or("Teapot data: too few points")?
            .split_whitespace()
            .map(|x| x.parse::<f64>().map_err(|e| e.to_string()))
            .collect::<Result<_>>()?;
        let [x, y, z] = v[..] else {
            return Err("Teapot data: a point needs x y z".into());
        };
        points.push(DVec3::new(x, y, z));
    }
    let m = count(lines.next(), "patches")?;
    let mut out = Vec::with_capacity(m);
    for _ in 0..m {
        let ids: Vec<usize> = lines
            .next()
            .ok_or("Teapot data: too few patches")?
            .split_whitespace()
            .map(|x| x.parse::<usize>().map_err(|e| e.to_string()))
            .collect::<Result<_>>()?;
        if ids.len() != 16 {
            return Err("Teapot data: a patch needs 16 points".into());
        }
        let mut patch = [DVec3::ZERO; 16];
        for (slot, &i) in patch.iter_mut().zip(&ids) {
            *slot = *points.get(i).ok_or("Teapot data: point out of range")?;
        }
        out.push(patch);
    }
    Ok(out)
}

fn bernstein(t: f64) -> [f64; 4] {
    let s = 1. - t;
    [s * s * s, 3. * t * s * s, 3. * t * t * s, t * t * t]
}

/// The data's z-up, spout-+x frame to Axis Y: up +Y, spout −Z, handle +Z.
fn to_y_up(p: DVec3) -> DVec3 {
    DVec3::new(-p.y, p.z, -p.x)
}

/// Turn Axis Y's "up" onto `axis` (a rotation, so polygons keep facing out).
fn onto_axis(p: DVec3, axis: usize) -> DVec3 {
    match axis {
        0 => DVec3::new(p.y, -p.x, p.z),
        2 => DVec3::new(p.x, -p.z, p.y),
        _ => p,
    }
}

/// One polygon with its corners' patch UVs.
struct Poly {
    vertices: Vec<u32>,
    uvs: Vec<DVec2>,
}

/// The teapot mesh for `p`.
pub fn teapot_geometry(p: &TeapotParameters) -> Result<Mesh> {
    if !p.size.is_finite() || p.size <= 0. || p.size > 1e8 {
        return Err("Teapot Size must be more than 0 and within the size budget".into());
    }
    if !p.position.is_finite() || p.position.abs().max_element() > 1e9 {
        return Err("Teapot position must be finite and within the coordinate budget".into());
    }
    let n = TEAPOT_SEGMENTS as usize;
    let mut points: Vec<DVec3> = Vec::new();
    let mut welded: HashMap<[i64; 3], u32> = HashMap::new();
    let mut polys: Vec<Poly> = Vec::new();
    let all = patches()?;
    for (pi, patch) in all.iter().enumerate() {
        // The patch's grid of points, welded with its neighbours'.
        let mut grid = vec![0u32; (n + 1) * (n + 1)];
        for i in 0..=n {
            let bs = bernstein(i as f64 / n as f64);
            for j in 0..=n {
                let bt = bernstein(j as f64 / n as f64);
                let mut q = DVec3::ZERO;
                for r in 0..4 {
                    for c in 0..4 {
                        q += patch[r * 4 + c] * bs[r] * bt[c];
                    }
                }
                let q = to_y_up(q);
                let key = (q * 1e7).round().to_array().map(|v| v as i64);
                grid[i * (n + 1) + j] = *welded.entry(key).or_insert_with(|| {
                    points.push(q);
                    points.len() as u32 - 1
                });
            }
        }
        // Packed UVs: patch tiles 8 across, 4 up, a small margin each.
        let tile = DVec2::new((pi % 8) as f64, (pi / 8) as f64);
        let uv = |i: usize, j: usize| {
            let local = DVec2::new(j as f64, i as f64) / n as f64;
            (tile + DVec2::splat(0.02) + local * 0.96) / DVec2::new(8., 4.)
        };
        for i in 0..n {
            for j in 0..n {
                let corners = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
                let mut vertices = Vec::with_capacity(4);
                let mut uvs = Vec::with_capacity(4);
                for (a, b) in corners {
                    let v = grid[a * (n + 1) + b];
                    if vertices.last() != Some(&v) {
                        vertices.push(v);
                        uvs.push(uv(a, b));
                    }
                }
                if vertices.len() > 1 && vertices.first() == vertices.last() {
                    vertices.pop();
                    uvs.pop();
                }
                // A cell squashed to a line at a pole goes.
                if vertices.len() >= 3 {
                    polys.push(Poly { vertices, uvs });
                }
            }
        }
    }
    orient(&points, &mut polys);
    // Size and place by the bounding box (MT6-01/02).
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for &q in &points {
        lo = lo.min(q);
        hi = hi.max(q);
    }
    let scale = p.size / (hi - lo).max_element();
    let centre = (lo + hi) / 2.;
    let mut mesh = Mesh {
        vertices: points
            .iter()
            .map(|&q| onto_axis((q - centre) * scale, p.axis) + p.position)
            .collect(),
        ..Default::default()
    };
    let mut rows = Vec::with_capacity(polys.len());
    for poly in polys {
        rows.push(poly.uvs.into_iter().map(Some).collect::<Vec<_>>());
        mesh.faces.push(Face {
            vertices: poly.vertices,
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
    }
    if p.make_uvs {
        mesh.uv_maps.insert(crate::PRIMITIVE_UV_MAP.into(), rows);
    }
    mesh.validate()?;
    Ok(mesh)
}

/// Wind every polygon the same way as its neighbours, then turn each
/// connected piece so the polygons round its highest point face up: on
/// every piece of the teapot (body, lid, handle, spout) that is outward.
fn orient(points: &[DVec3], polys: &mut [Poly]) {
    let mut by_edge: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (f, poly) in polys.iter().enumerate() {
        let k = poly.vertices.len();
        for c in 0..k {
            let (a, b) = (poly.vertices[c], poly.vertices[(c + 1) % k]);
            by_edge.entry((a.min(b), a.max(b))).or_default().push(f);
        }
    }
    let runs = |poly: &Poly, a: u32, b: u32| {
        let k = poly.vertices.len();
        (0..k).any(|c| poly.vertices[c] == a && poly.vertices[(c + 1) % k] == b)
    };
    let flip = |poly: &mut Poly| {
        poly.vertices.reverse();
        poly.uvs.reverse();
    };
    let mut piece = vec![usize::MAX; polys.len()];
    let mut pieces = 0;
    for start in 0..polys.len() {
        if piece[start] != usize::MAX {
            continue;
        }
        piece[start] = pieces;
        let mut queue = VecDeque::from([start]);
        while let Some(f) = queue.pop_front() {
            let k = polys[f].vertices.len();
            for c in 0..k {
                let (a, b) = (polys[f].vertices[c], polys[f].vertices[(c + 1) % k]);
                for &g in &by_edge[&(a.min(b), a.max(b))] {
                    if piece[g] != usize::MAX {
                        continue;
                    }
                    // A neighbour runs a shared side the other way round.
                    if runs(&polys[g], a, b) {
                        flip(&mut polys[g]);
                    }
                    piece[g] = pieces;
                    queue.push_back(g);
                }
            }
        }
        pieces += 1;
    }
    for which in 0..pieces {
        let members: Vec<usize> = (0..polys.len()).filter(|&f| piece[f] == which).collect();
        let Some(top) = members
            .iter()
            .flat_map(|&f| polys[f].vertices.iter().copied())
            .max_by(|&a, &b| points[a as usize].y.total_cmp(&points[b as usize].y))
        else {
            continue;
        };
        let up: f64 = members
            .iter()
            .filter(|&&f| polys[f].vertices.contains(&top))
            .map(|&f| newell(points, &polys[f].vertices).y)
            .sum();
        if up < 0. {
            for &f in &members {
                flip(&mut polys[f]);
            }
        }
    }
}

fn newell(points: &[DVec3], vertices: &[u32]) -> DVec3 {
    let k = vertices.len();
    (0..k)
        .map(|c| {
            let a = points[vertices[c] as usize];
            let b = points[vertices[(c + 1) % k] as usize];
            DVec3::new(
                (a.y - b.y) * (a.z + b.z),
                (a.z - b.z) * (a.x + b.x),
                (a.x - b.x) * (a.y + b.y),
            )
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The data file reads as 32 patches of 16 points.
    #[test]
    fn the_data_has_thirty_two_patches() {
        assert_eq!(patches().unwrap().len(), 32);
    }
}
