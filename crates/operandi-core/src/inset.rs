//! Inset (spec `inset-2026-11-05`): each selected polygon gets a smaller copy
//! inside it, joined to its outline by a ring of quads.
//!
//! * Each polygon is inset on its own; no new points are shared.
//! * Edge offset, not scaling: each edge moves `inset` inward, measured
//!   perpendicular to it in the polygon's plane; corners are where the moved
//!   edges meet (miter). Negative insets move outward. Nothing is clamped: an
//!   inset past the middle crosses over.
//! * Inset 0 still builds the inner points (on the originals) and the quads.
//! * The inner polygon takes the original's slot (same corner order); side
//!   quad `[a, b, b′, a′]` per original edge a→b is appended. Polygon order
//!   is free (decided Q-IN-01).
//! * Nothing selected insets every polygon.
//!
//! Provisional (Q-IN-03, Q-IN-04): the plane is the polygon's Newell plane;
//! new polygons keep their polygon's tags; each new corner's UV comes from
//! the polygon's own corner UVs, mapped affinely across the polygon. Other
//! per-corner data is refused.
use crate::corner_uv::records_from_uvs;
use crate::{Mesh, Result};
use glam::{DVec2, DVec3};

#[derive(Clone, Debug)]
pub struct InsetResult {
    pub mesh: Mesh,
    /// The inner polygons (the selected slots).
    pub selected: Vec<u32>,
}

/// Inset `selected` polygons (all when empty) by `inset`.
pub fn inset_geometry(source: &Mesh, selected: &[u32], inset: f64) -> Result<InsetResult> {
    source.validate()?;
    if !inset.is_finite() {
        return Err("Inset must be a finite distance".into());
    }
    if crate::corner_uv::has_uncarried_corner_data(source) {
        return Err(
            "Inset can't carry corner weights, colors or normals yet; nothing changed".into(),
        );
    }
    let mut chosen: Vec<usize> = if selected.is_empty() {
        (0..source.faces.len()).collect()
    } else {
        selected.iter().map(|&f| f as usize).collect()
    };
    chosen.sort_unstable();
    chosen.dedup();
    if chosen.iter().any(|&f| f >= source.faces.len()) {
        return Err("A selected polygon no longer exists".into());
    }
    let p = |v: u32| source.vertices[v as usize];
    let mut out = source.clone();
    // Per new face: its source polygon (for tags and UVs).
    let mut new_faces: Vec<(Vec<u32>, usize)> = Vec::new();
    // Per selected polygon: its inner corner points and positions.
    let mut inner_of: Vec<(usize, Vec<u32>)> = Vec::new();
    for &f in &chosen {
        let ring = &source.faces[f].vertices;
        let n = ring.len();
        if n < 3 || source.is_polyline(f) {
            return Err("Inset works on polygons with three or more corners".into());
        }
        let normal = source.normal(f).normalize_or_zero();
        if normal == DVec3::ZERO {
            return Err("Inset needs polygons with a defined facing".into());
        }
        // Inward direction of each edge i (corner i → i+1), in the plane.
        let inward: Vec<DVec3> = (0..n)
            .map(|i| {
                let e = p(ring[(i + 1) % n]) - p(ring[i]);
                normal.cross(e - normal * normal.dot(e)).normalize_or_zero()
            })
            .collect();
        if inward.contains(&DVec3::ZERO) {
            return Err("Inset needs polygons without zero-length edges".into());
        }
        let mut inner = Vec::with_capacity(n);
        for i in 0..n {
            let (d0, d1) = (inward[(i + n - 1) % n], inward[i]);
            let denominator = 1. + d0.dot(d1);
            if denominator < 1e-9 {
                return Err("Inset can't offset a corner whose edges fold back".into());
            }
            // m·d0 = m·d1 = inset: where the two moved edges meet.
            let q = p(ring[i]) + (d0 + d1) * (inset / denominator);
            let id = u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
            out.vertices.push(q);
            inner.push(id);
        }
        for i in 0..n {
            let j = (i + 1) % n;
            new_faces.push((vec![ring[i], ring[j], inner[j], inner[i]], f));
        }
        inner_of.push((f, inner));
    }
    for (f, inner) in &inner_of {
        out.faces[*f].vertices = inner.clone();
    }
    let attributes = &source.attributes;
    for (vertices, template) in &new_faces {
        let mut face = source.faces[*template].clone();
        face.vertices = vertices.clone();
        face.uv = None;
        out.faces.push(face);
        if !attributes.smoothing_labels.is_empty() {
            let l = attributes.smoothing_labels[*template].clone();
            out.attributes.smoothing_labels.push(l);
        }
        if !attributes.representations.is_empty() {
            out.attributes
                .representations
                .push(attributes.representations[*template]);
        }
    }
    // UVs: an affine map from each polygon's plane to its corner UVs.
    let homes: Vec<usize> = (0..source.faces.len())
        .chain(new_faces.iter().map(|&(_, t)| t))
        .collect();
    for (name, map) in out.uv_maps.iter_mut() {
        let rows = &source.uv_maps[name];
        let fits: std::collections::BTreeMap<usize, Option<Affine>> = chosen
            .iter()
            .map(|&f| (f, Affine::fit(source, f, &rows[f])))
            .collect();
        *map = out
            .faces
            .iter()
            .zip(&homes)
            .enumerate()
            .map(|(fi, (face, &home))| {
                let Some(fit) = fits.get(&home) else {
                    return rows[fi].clone();
                };
                face.vertices
                    .iter()
                    .map(|&v| {
                        let original = source.faces[home].vertices.iter().position(|&x| x == v);
                        match original {
                            Some(c) => rows[home][c],
                            None => fit.as_ref().map(|a| a.at(out.vertices[v as usize])),
                        }
                    })
                    .collect()
            })
            .collect();
    }
    for (name, records) in out.attributes.uv_records.iter_mut() {
        *records = records_from_uvs(&out.faces, &out.uv_maps[name]);
    }
    // Inset 0 builds zero-area side quads on purpose (rule 4); mark them as
    // the deliberate degenerate polygons they are.
    for f in source.faces.len()..out.faces.len() {
        if out.normal(f) == DVec3::ZERO {
            out.attributes.degenerate_faces.insert(f as u32);
        }
    }
    out.validate()
        .map_err(|e| format!("Inset produced an invalid mesh ({e}); nothing changed"))?;
    Ok(InsetResult {
        mesh: out,
        selected: if selected.is_empty() {
            Vec::new()
        } else {
            chosen.iter().map(|&f| f as u32).collect()
        },
    })
}

/// Shift (tool audit Tier 1, provisional TA1-Q08): each selected polygon
/// (all when none are) moves `shift` along its own normal on new points,
/// joined to its old outline by side quads. Neighbours share no new points
/// (Inset rule 1); topology, order, tags and UVs are Inset's at 0, so the
/// moved caps keep their polygons' slots and UVs. Negative moves inward.
pub fn shift_geometry(source: &Mesh, selected: &[u32], shift: f64) -> Result<InsetResult> {
    if !shift.is_finite() {
        return Err("Shift must be a finite distance".into());
    }
    let mut result = inset_geometry(source, selected, 0.)?;
    let caps: Vec<usize> = if selected.is_empty() {
        (0..source.faces.len()).collect()
    } else {
        result.selected.iter().map(|&f| f as usize).collect()
    };
    let out = &mut result.mesh;
    for &f in &caps {
        let normal = source.normal(f).normalize_or_zero();
        // Each cap's points are its own (no sharing), so none moves twice.
        for i in 0..out.faces[f].vertices.len() {
            let v = out.faces[f].vertices[i] as usize;
            out.vertices[v] += normal * shift;
        }
    }
    // Side quads that now have area are no longer the deliberate
    // degenerate polygons Inset 0 marked.
    for f in source.faces.len()..out.faces.len() {
        if out.normal(f) != DVec3::ZERO {
            out.attributes.degenerate_faces.remove(&(f as u32));
        }
    }
    out.validate()
        .map_err(|e| format!("Shift produced an invalid mesh ({e}); nothing changed"))?;
    Ok(result)
}

/// Least-squares affine map from a polygon's plane to its corner UVs.
struct Affine {
    origin: DVec3,
    u: DVec3,
    v: DVec3,
    /// uv = c + a·s + b·t for plane coordinates (s, t).
    c: DVec2,
    a: DVec2,
    b: DVec2,
}
impl Affine {
    fn fit(m: &Mesh, f: usize, uvs: &[Option<DVec2>]) -> Option<Self> {
        let ring = &m.faces[f].vertices;
        let normal = m.normal(f).try_normalize()?;
        let u = normal.any_orthonormal_vector();
        let v = normal.cross(u);
        let origin = m.vertices[ring[0] as usize];
        let samples: Vec<(f64, f64, DVec2)> = ring
            .iter()
            .zip(uvs)
            .map(|(&x, uv)| {
                let d = m.vertices[x as usize] - origin;
                Some((d.dot(u), d.dot(v), (*uv)?))
            })
            .collect::<Option<_>>()?;
        // Normal equations for [1, s, t] → uv.
        let mut ata = [[0.; 3]; 3];
        let mut atb = [DVec2::ZERO; 3];
        for &(s, t, w) in &samples {
            let row = [1., s, t];
            for i in 0..3 {
                for j in 0..3 {
                    ata[i][j] += row[i] * row[j];
                }
                atb[i] += w * row[i];
            }
        }
        let m3 = glam::DMat3::from_cols_array_2d(&ata).transpose();
        if m3.determinant().abs() < 1e-18 {
            return None;
        }
        let inv = m3.inverse();
        let solve = |k: usize| inv.row(k);
        let pick = |r: glam::DVec3| atb[0] * r.x + atb[1] * r.y + atb[2] * r.z;
        Some(Self {
            origin,
            u,
            v,
            c: pick(solve(0)),
            a: pick(solve(1)),
            b: pick(solve(2)),
        })
    }
    fn at(&self, q: DVec3) -> DVec2 {
        let d = q - self.origin;
        self.c + self.a * d.dot(self.u) + self.b * d.dot(self.v)
    }
}
