//! Mesh curvature by rays, for the Curvature bake.
//!
//! Adapted from TextureCraft `crates/bake/src/map/curvature.rs` at
//! 9c1b9cbc (MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the
//! TextureCraft contributors; see docs/licenses/texturecraft-LICENSE-MIT.txt).
//! Changes: the low-poly tuning only (no high-poly source), rays traced one
//! pixel at a time on Operandi's own BVH instead of in batches.
//!
//! Four rings of rays around each point: two "wall" rings just above and
//! below the surface find hard concave and convex edges within 1.5 pixels;
//! a wider ring of rays dropped onto the surface reads how the smooth
//! normals turn (rounded shapes); a small ring compares smooth and flat
//! normals for true curvature. Positive is convex, negative concave.
use super::{BakeMesh, Surface};
use bvh::Hit;
use glam::Vec3;

use super::bvh;

const RING_COUNT: usize = 4;
const WALL_RADIUS_TEXELS: f32 = 1.5;
/// How far the smooth-shading ring reaches, in sampling radii.
const SPREAD_RADII: f32 = 4.0;
const CONCAVE_WEIGHT: f32 = 1.5;
const CONVEX_WEIGHT: f32 = 1.5;
const CURVATURE_RADIUS_RATIO: f32 = 0.003;
const CURVATURE_WEIGHT: f32 = 1.3;
const HARD_EDGE_NORMAL_CHANGE_THRESHOLD: f32 = 0.05;
const GENTLE_CURVATURE_THRESHOLD: f32 = 0.1;

pub(super) struct Curvature {
    walls: f32,
    spread: f32,
    true_radius: f32,
    fallback: f32,
    /// World size of one pixel, per triangle.
    texels: Vec<f32>,
    ring: [Vec<(f32, f32)>; RING_COUNT],
}

impl Curvature {
    /// `radius` is the sampling radius as a fraction of the mesh's size.
    pub(super) fn new(
        mesh: &BakeMesh,
        rays: usize,
        radius: f32,
        width: u32,
        height: u32,
    ) -> Curvature {
        let scale = mesh.scale;
        let resolution = (width as f32 * height as f32).sqrt();
        let fallback = scale * 1e-3;
        let texels = mesh
            .uvs
            .iter()
            .enumerate()
            .map(|(t, uv)| {
                let Some(uv) = uv else { return fallback };
                let p = mesh.bvh.triangle(t as u32);
                let world = (p[1] - p[0]).cross(p[2] - p[0]).length();
                let area = (uv[1] - uv[0]).perp_dot(uv[2] - uv[0]).abs();
                let v = (world / area).sqrt() / resolution;
                if area > 1e-12 && world > 0.0 && v.is_finite() {
                    v
                } else {
                    fallback
                }
            })
            .collect();
        let radius = radius * scale;
        let radius_scale = radius / (scale * 0.001);
        let ring = std::array::from_fn(|r| {
            let count = rays / RING_COUNT + usize::from(r < rays % RING_COUNT);
            (0..count)
                .map(|i| {
                    let a = std::f32::consts::TAU * i as f32 / count.max(1) as f32;
                    (a.cos(), a.sin())
                })
                .collect()
        });
        Curvature {
            walls: WALL_RADIUS_TEXELS * radius_scale,
            spread: radius * SPREAD_RADII,
            true_radius: scale * CURVATURE_RADIUS_RATIO * radius_scale,
            fallback,
            texels,
            ring,
        }
    }

    fn directions(&self, ring: usize, t: Vec3, b: Vec3) -> impl Iterator<Item = Vec3> + '_ {
        self.ring[ring].iter().map(move |(c, s)| t * *c + b * *s)
    }

    /// The raw (unnormalised) curvature at `s`, in -1..1.
    pub(super) fn value(&self, mesh: &BakeMesh, s: &Surface) -> f32 {
        let texel = self
            .texels
            .get(s.tri as usize)
            .copied()
            .unwrap_or(self.fallback);
        let gn = s.geo;
        let (t, b) = gn.any_orthonormal_pair();
        let r = self.walls * texel;
        // Rings 0 and 1: along the surface, just above and just below it.
        let wall_hits = |offset: f32| -> Vec<Option<Hit>> {
            self.directions(if offset > 0.0 { 0 } else { 1 }, t, b)
                .map(|d| mesh.closest(s.pos + gn * (offset * r), d, r))
                .collect()
        };
        let above_hits = wall_hits(0.15);
        let below_hits = wall_hits(-0.15);
        // A wall above counts only where the floor continues under it
        // (a ground ray from just short of the wall lands close by).
        let grounded: Vec<bool> = above_hits
            .iter()
            .zip(self.directions(0, t, b))
            .map(|(h, d)| {
                h.is_some_and(|h| {
                    let from = s.pos + gn * (0.15 * r) + d * (h.t * 0.85);
                    mesh.closest(from, -gn, 0.5 * r)
                        .is_some_and(|g| g.t < 0.3 * r)
                })
            })
            .collect();
        let above = self.wall(mesh, s, r, &above_hits, Some(&grounded));
        let below = self.wall(mesh, s, r, &below_hits, None);
        // Rings 2 and 3: rays dropped onto the surface around the point.
        let drop = |ring: usize, radius: f32| -> (f32, f32) {
            let dirs: Vec<Vec3> = self.directions(ring, t, b).collect();
            let hits: Vec<Option<Hit>> = dirs
                .iter()
                .map(|d| {
                    mesh.closest(
                        s.pos + *d * radius + gn * (0.25 * radius),
                        -gn,
                        2.25 * radius,
                    )
                })
                .collect();
            spread(mesh, s, &dirs, &hits)
        };
        let (shading, _) = drop(2, self.spread);
        let (true_shading, true_faces) = drop(3, self.true_radius);
        let value = -above * CONCAVE_WEIGHT
            + (below - above).max(0.0) * CONVEX_WEIGHT
            + shading * 0.5
            + tight(agreed(true_shading, true_faces) * CURVATURE_WEIGHT);
        (value * 2.0).clamp(-1.0, 1.0)
    }

    /// How much hard-edged wall the ring's hits found, nearer counting more.
    fn wall(
        &self,
        mesh: &BakeMesh,
        s: &Surface,
        radius: f32,
        hits: &[Option<Hit>],
        keep: Option<&[bool]>,
    ) -> f32 {
        let mut sum = 0.0;
        for (i, hit) in hits.iter().enumerate() {
            let Some(hit) = hit else { continue };
            if keep.is_some_and(|k| !k[i]) {
                continue;
            }
            let upright = mesh
                .face_normal(hit.tri)
                .map_or(1.0, |n| (1.0 - n.dot(s.geo).powi(2)).max(0.0).sqrt());
            let shaded = mesh.shading_normal(hit.tri, hit.u, hit.v).unwrap_or(s.nrm);
            let hard =
                ((1.0 - shaded.dot(s.nrm)) / HARD_EDGE_NORMAL_CHANGE_THRESHOLD).clamp(0.0, 1.0);
            sum += (1.0 - hit.t / radius) * upright * hard;
        }
        sum / hits.len().max(1) as f32
    }
}

/// How the smooth and flat normals turn away along each direction.
fn spread(mesh: &BakeMesh, s: &Surface, dirs: &[Vec3], hits: &[Option<Hit>]) -> (f32, f32) {
    let (mut shading, mut faces) = (0.0, 0.0);
    for (hit, dir) in hits.iter().zip(dirs) {
        let Some(hit) = hit else { continue };
        let normal = mesh.shading_normal(hit.tri, hit.u, hit.v).unwrap_or(s.nrm);
        shading += (normal - s.nrm).dot(*dir);
        if let Some(f) = mesh.face_normal(hit.tri) {
            let f = if f.dot(s.geo) < 0.0 { -f } else { f };
            faces += (f - s.geo).dot(*dir);
        }
    }
    let n = hits.len().max(1) as f32;
    (shading / n, faces / n)
}

fn tight(v: f32) -> f32 {
    v.signum() * (v.abs() - GENTLE_CURVATURE_THRESHOLD).max(0.0)
}

fn agreed(shading: f32, faces: f32) -> f32 {
    if shading * faces <= 0.0 {
        0.0
    } else {
        shading.abs().min(faces.abs()).copysign(shading)
    }
}
