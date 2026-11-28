//! UV Transform as a Mesh Stack deformer (`uvtransform`, MESHOPS-TIDY
//! TB-40..43): moves the chosen points onto a Target mesh's surface, using
//! the Target's UV map as the address. Each point's address is its place
//! seen down an Axis (or its UV in a Source UV Map), in a Scale-sized
//! square centred on Offset; the point goes to the spot of the Target
//! whose UVs match, then off the surface by its depth (Wrap off) and Push.
//! It never edits UVs (Transform UV Map does that).
//!
//! Provisional (MOQ-69, TB-Q5): a Target polygon is split into triangles
//! and matched with barycentric weights; a point whose address is on no
//! Target polygon stays where it was.
use crate::mesh_stack_builtin::{check, choice, distance};
use crate::meshop_deformer_common::FOLDER;
use crate::meshop_types::{Field, FieldKind, OpContext, OpDef, Typed, TypedOp};
use crate::{Mesh, Result};
use glam::{DMat4, DVec2, DVec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const UV_TRANSFORM_ID: &str = "uvtransform";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UvTransformOp {
    /// The target mesh item (0 = none).
    pub target: u64,
    #[serde(rename = "targetuvmap")]
    pub target_uv_map: String,
    #[serde(rename = "sourceuvmap")]
    pub source_uv_map: String,
    /// 0 X, 1 Y, 2 Z.
    #[serde(rename = "sourceworldaxis")]
    pub axis: u32,
    #[serde(rename = "offset.U")]
    pub offset_u: f64,
    #[serde(rename = "offset.V")]
    pub offset_v: f64,
    pub scale: f64,
    pub wrap: bool,
    pub push: f64,
    #[serde(rename = "usesourcexfrm")]
    pub use_source_xfrm: bool,
}
impl Default for UvTransformOp {
    fn default() -> Self {
        Self {
            target: 0,
            target_uv_map: String::new(),
            source_uv_map: String::new(),
            axis: 2,
            offset_u: 0.5,
            offset_v: 0.5,
            scale: 1.,
            wrap: true,
            push: 0.,
            use_source_xfrm: false,
        }
    }
}

/// One Target triangle: its corners' UVs and places, and its polygon's
/// normal.
struct Tri {
    uv: [DVec2; 3],
    at: [DVec3; 3],
    normal: DVec3,
}
impl Tri {
    /// Barycentric weights of `p` in the UV triangle, if inside.
    fn weights(&self, p: DVec2) -> Option<DVec3> {
        let [a, b, c] = self.uv;
        let (v0, v1, v2) = (b - a, c - a, p - a);
        let d = v0.perp_dot(v1);
        if d.abs() < 1e-18 {
            return None;
        }
        let wb = v2.perp_dot(v1) / d;
        let wc = v0.perp_dot(v2) / d;
        let wa = 1. - wb - wc;
        let eps = -1e-9;
        (wa >= eps && wb >= eps && wc >= eps).then_some(DVec3::new(wa, wb, wc))
    }
}

/// The Target's triangles in UV space, bucketed on a grid so a lookup
/// reads only a few.
struct UvIndex {
    tris: Vec<Tri>,
    min: DVec2,
    cell: DVec2,
    n: usize,
    cells: Vec<Vec<u32>>,
}
impl UvIndex {
    fn new(target: &Mesh, map: &str) -> Option<Self> {
        let uvs = target.uv_maps.get(map)?;
        let mut tris = Vec::new();
        for (f, face) in target.faces.iter().enumerate() {
            if target.is_polyline(f) {
                continue;
            }
            let normal = target.normal(f);
            let Ok(split) = target.triangles(f) else {
                continue;
            };
            for c in split {
                let uv = c.map(|i| uvs.get(f).and_then(|r| r.get(i)).copied().flatten());
                let [Some(a), Some(b), Some(cc)] = uv else {
                    continue;
                };
                let at = c.map(|i| target.vertices[face.vertices[i] as usize]);
                tris.push(Tri {
                    uv: [a, b, cc],
                    at,
                    normal,
                });
            }
        }
        if tris.is_empty() {
            return None;
        }
        let (mut min, mut max) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
        for t in &tris {
            for p in t.uv {
                min = min.min(p);
                max = max.max(p);
            }
        }
        let n = ((tris.len() as f64).sqrt().ceil() as usize).clamp(1, 256);
        let cell = ((max - min) / n as f64).max(DVec2::splat(1e-12));
        let mut cells = vec![Vec::new(); n * n];
        let at = |p: DVec2| {
            let q = ((p - min) / cell).floor();
            (
                (q.x.max(0.) as usize).min(n - 1),
                (q.y.max(0.) as usize).min(n - 1),
            )
        };
        for (i, t) in tris.iter().enumerate() {
            let lo = at(t.uv[0].min(t.uv[1]).min(t.uv[2]));
            let hi = at(t.uv[0].max(t.uv[1]).max(t.uv[2]));
            for y in lo.1..=hi.1 {
                for x in lo.0..=hi.0 {
                    cells[y * n + x].push(i as u32);
                }
            }
        }
        Some(Self {
            tris,
            min,
            cell,
            n,
            cells,
        })
    }
    /// The Target place and normal at UV `p`, if a triangle holds it.
    fn find(&self, p: DVec2) -> Option<(DVec3, DVec3)> {
        let q = ((p - self.min) / self.cell).floor();
        let n = self.n as f64;
        // Just outside the grid can still be on an edge triangle.
        if q.x < -1. || q.y < -1. || q.x > n || q.y > n {
            return None;
        }
        let (x, y) = (
            (q.x.max(0.) as usize).min(self.n - 1),
            (q.y.max(0.) as usize).min(self.n - 1),
        );
        self.cells[y * self.n + x].iter().find_map(|&i| {
            let t = &self.tris[i as usize];
            let w = t.weights(p)?;
            Some((t.at[0] * w.x + t.at[1] * w.y + t.at[2] * w.z, t.normal))
        })
    }
}

/// Each point's UV in map `map` (the first corner that has one).
fn point_uvs(mesh: &Mesh, map: &str) -> Option<Vec<Option<DVec2>>> {
    let uvs = mesh.uv_maps.get(map)?;
    let mut out = vec![None; mesh.vertices.len()];
    for (f, face) in mesh.faces.iter().enumerate() {
        for (c, &v) in face.vertices.iter().enumerate() {
            if out[v as usize].is_none() {
                out[v as usize] = uvs.get(f).and_then(|r| r.get(c)).copied().flatten();
            }
        }
    }
    Some(out)
}

impl TypedOp for UvTransformOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let values = [self.offset_u, self.offset_v, self.scale, self.push];
        if values.iter().any(|v| !v.is_finite()) {
            return Err("UV Transform values must be numbers".into());
        }
        if self.axis > 2 {
            return Err("UV Transform Axis must be 0 (X), 1 (Y) or 2 (Z)".into());
        }
        if self.scale <= 0. {
            return Err("UV Transform needs a Scale above 0%".into());
        }
        // TB-43: no Target or map leaves the mesh as it is.
        let Some(source) = ctx.sources()?.into_iter().next() else {
            return Ok(ctx.input.clone());
        };
        if self.target_uv_map.trim().is_empty() {
            return Ok(ctx.input.clone());
        }
        let Some(index) = UvIndex::new(&source.mesh, &self.target_uv_map) else {
            ctx.note(crate::meshop_types::OpNote {
                item: 0,
                text: format!(
                    "The Target has no UV map \u{201c}{}\u{201d}: nothing moves",
                    self.target_uv_map
                ),
                problem: true,
            });
            return Ok(ctx.input.clone());
        };
        let keys = if self.source_uv_map.trim().is_empty() {
            None
        } else {
            match point_uvs(ctx.input, &self.source_uv_map) {
                Some(k) => Some(k),
                None => {
                    return Err(format!(
                        "UV Transform: the mesh has no UV map \u{201c}{}\u{201d}",
                        self.source_uv_map
                    ));
                }
            }
        };
        // This item's own world matrix, from the link's two (Use Source
        // Transform reads places in world space).
        let own_world: DMat4 = source.world * source.to_local.inverse();
        // Off, this item counts as sitting at the world origin: places go
        // through the Target's world placement only (TB table: moving the
        // source changes nothing, moving the Target carries the result).
        let to_local = if self.use_source_xfrm {
            source.to_local
        } else {
            source.world
        };
        crate::meshop_deformer_common::deform_points(ctx, 1., |i, p| {
            let (key, depth) = match &keys {
                Some(k) => match k.get(i).copied().flatten() {
                    Some(uv) => (uv, 0.),
                    None => return p,
                },
                None => {
                    let q = if self.use_source_xfrm {
                        own_world.transform_point3(p)
                    } else {
                        p
                    };
                    // TB-40: in-plane (a, b) and depth d for the Axis.
                    let (a, b, d) = match self.axis {
                        0 => (q.z, q.y, q.x),
                        1 => (q.z, q.x, q.y),
                        _ => (q.x, q.y, q.z),
                    };
                    let uv = DVec2::new(
                        (a - self.offset_u) / self.scale + 0.5,
                        (b - self.offset_v) / self.scale + 0.5,
                    );
                    (uv, d)
                }
            };
            let Some((on, normal)) = index.find(key) else {
                return p; // TB-43: a miss stays
            };
            let lift = if self.wrap { 0. } else { depth } + self.push;
            let n = to_local.transform_vector3(normal).normalize_or_zero();
            to_local.transform_point3(on) + n * lift
        })
    }
}

pub(crate) fn entries() -> Vec<Arc<dyn OpDef>> {
    vec![Arc::new(Typed::<UvTransformOp>::new(
        UV_TRANSFORM_ID,
        "UV Transform",
        FOLDER,
        true,
        vec![
            Field::new("target", "Target", FieldKind::Item),
            Field::new("targetuvmap", "Target UV Map", FieldKind::Text),
            Field::new("sourceuvmap", "Source UV Map", FieldKind::Text),
            choice("sourceworldaxis", "Axis", &["X", "Y", "Z"]),
            distance("offset.U", "Offset U"),
            distance("offset.V", "Offset V"),
            Field::new("scale", "Scale", FieldKind::Percent),
            check("wrap", "Wrap"),
            distance("push", "Push"),
            check("usesourcexfrm", "Use Source Transform"),
        ],
    ))]
}
