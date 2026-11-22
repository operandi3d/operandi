//! A bounding volume hierarchy over triangles for the bakes' rays:
//! median split on the longest axis of the centroids, four triangles per
//! leaf, closest-hit queries with Möller–Trumbore.
use glam::Vec3;

const LEAF: usize = 4;

/// What a ray hit: triangle, distance and barycentric (u, v) of corners
/// 1 and 2.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub tri: u32,
    pub t: f32,
    pub u: f32,
    pub v: f32,
}

#[derive(Clone, Copy, Debug)]
struct Node {
    min: Vec3,
    max: Vec3,
    /// Leaf: first index into `order`; inner: the right child (the left
    /// child is the next node).
    start: u32,
    /// Leaf: triangle count; inner: 0.
    count: u32,
}

pub struct Bvh {
    nodes: Vec<Node>,
    order: Vec<u32>,
    tris: Vec<[Vec3; 3]>,
}

impl Bvh {
    pub fn new(tris: Vec<[Vec3; 3]>) -> Bvh {
        let mut order: Vec<u32> = (0..tris.len() as u32).collect();
        let centroids: Vec<Vec3> = tris.iter().map(|t| (t[0] + t[1] + t[2]) / 3.0).collect();
        let mut nodes = Vec::with_capacity(tris.len() / 2 + 1);
        if !tris.is_empty() {
            build(&tris, &centroids, &mut order, 0, tris.len(), &mut nodes);
        }
        Bvh { nodes, order, tris }
    }

    pub fn triangle(&self, tri: u32) -> &[Vec3; 3] {
        &self.tris[tri as usize]
    }

    /// The closest hit along `dir` (unit length) within `max`.
    pub fn closest(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<Hit> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv = dir.recip();
        let mut best: Option<Hit> = None;
        let mut limit = max;
        let mut stack = [0u32; 64];
        let mut top = 1;
        while top > 0 {
            top -= 1;
            let at = stack[top];
            let node = self.nodes[at as usize];
            if !slab(origin, inv, node.min, node.max, limit) {
                continue;
            }
            if node.count > 0 {
                for &tri in &self.order[node.start as usize..(node.start + node.count) as usize] {
                    if let Some(h) = intersect(&self.tris[tri as usize], origin, dir, limit) {
                        limit = h.0;
                        best = Some(Hit {
                            tri,
                            t: h.0,
                            u: h.1,
                            v: h.2,
                        });
                    }
                }
            } else if top + 2 <= stack.len() {
                // The left child is stored right after its parent.
                stack[top] = node.start;
                stack[top + 1] = at + 1;
                top += 2;
            }
        }
        best
    }
}

fn build(
    tris: &[[Vec3; 3]],
    centroids: &[Vec3],
    order: &mut [u32],
    start: usize,
    end: usize,
    nodes: &mut Vec<Node>,
) -> usize {
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    let (mut cmin, mut cmax) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for &t in &order[start..end] {
        for p in tris[t as usize] {
            min = min.min(p);
            max = max.max(p);
        }
        cmin = cmin.min(centroids[t as usize]);
        cmax = cmax.max(centroids[t as usize]);
    }
    let index = nodes.len();
    nodes.push(Node {
        min,
        max,
        start: start as u32,
        count: (end - start) as u32,
    });
    let extent = cmax - cmin;
    if end - start <= LEAF || extent.max_element() <= 0.0 {
        return index;
    }
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };
    let mid = (start + end) / 2;
    order[start..end].select_nth_unstable_by(mid - start, |a, b| {
        centroids[*a as usize][axis].total_cmp(&centroids[*b as usize][axis])
    });
    let left = build(tris, centroids, order, start, mid, nodes);
    debug_assert_eq!(left, index + 1);
    let right = build(tris, centroids, order, mid, end, nodes);
    nodes[index].start = right as u32;
    nodes[index].count = 0;
    index
}

fn slab(origin: Vec3, inv: Vec3, min: Vec3, max: Vec3, limit: f32) -> bool {
    let a = (min - origin) * inv;
    let b = (max - origin) * inv;
    let near = a.min(b).max_element().max(0.0);
    let far = a.max(b).min_element().min(limit);
    near <= far
}

/// Möller–Trumbore, both sides: (t, u, v).
fn intersect(t: &[Vec3; 3], origin: Vec3, dir: Vec3, limit: f32) -> Option<(f32, f32, f32)> {
    let e1 = t[1] - t[0];
    let e2 = t[2] - t[0];
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = origin - t[0];
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let d = e2.dot(q) * inv;
    (d > 0.0 && d < limit).then_some((d, u, v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_nearest_of_many_triangles_like_brute_force() {
        let mut tris = Vec::new();
        for i in 0..200 {
            let z = i as f32 * 0.1;
            let x = (i % 7) as f32 * 0.3 - 1.0;
            tris.push([
                Vec3::new(x - 1.0, -1.0, z),
                Vec3::new(x + 1.0, -1.0, z),
                Vec3::new(x, 1.0, z),
            ]);
        }
        let bvh = Bvh::new(tris.clone());
        for k in 0..50 {
            let origin = Vec3::new(k as f32 * 0.02 - 0.5, 0.0, -1.0 + k as f32 * 0.37);
            let dir = Vec3::Z;
            let brute = tris
                .iter()
                .enumerate()
                .filter_map(|(i, t)| intersect(t, origin, dir, 100.0).map(|h| (i as u32, h.0)))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            let got = bvh.closest(origin, dir, 100.0).map(|h| (h.tri, h.t));
            assert_eq!(got.map(|g| g.1), brute.map(|b| b.1), "ray {k}");
        }
        assert!(
            bvh.closest(Vec3::new(0.0, 5.0, -1.0), Vec3::Z, 100.0)
                .is_none()
        );
    }
}
