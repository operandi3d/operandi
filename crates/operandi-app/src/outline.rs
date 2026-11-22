//! Render-only shared-edge ownership and screen-space stroke tessellation.
//! Mesh identity, not coincident world position, determines a shared edge.
use super::{camera::Camera, camera::View, gpu::Vertex};
use eframe::egui::*;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
#[derive(Clone, Copy)]
pub struct Edge {
    pub ends: [(Pos2, f64); 2],
    pub gradient: Vec2,
    pub color: Color32,
    pub priority: u8,
}
/// Edges in first-insertion order, found through a hash index. A sorted
/// tree here cost about half of each frame on large meshes.
#[derive(Default)]
pub struct Outlines {
    index: HashMap<(u64, u32, u32), usize, BuildHasherDefault<KeyHasher>>,
    edges: Vec<Edge>,
}
/// Multiply-rotate hashing for small integer keys (FxHash's scheme).
#[derive(Default)]
pub struct KeyHasher(u64);
impl Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }
    fn write_u32(&mut self, n: u32) {
        self.write_u64(n as u64);
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}
impl Outlines {
    /// Room for about `edges` edges without regrowing.
    pub fn with_capacity(edges: usize) -> Self {
        Self {
            index: HashMap::with_capacity_and_hasher(edges, Default::default()),
            edges: Vec::with_capacity(edges),
        }
    }
    pub fn insert(&mut self, item: u64, a: u32, b: u32, edge: Edge) {
        let key = (item, a.min(b), a.max(b));
        match self.index.get(&key) {
            Some(&i) => {
                // A neutral neighbor must never paint over a selected boundary.
                if edge.priority > self.edges[i].priority {
                    self.edges[i] = edge;
                }
            }
            None => {
                self.index.insert(key, self.edges.len());
                self.edges.push(edge);
            }
        }
    }
    pub fn tessellate(&self, camera: &Camera, r: Rect, wire: bool) -> Vec<Vertex> {
        let mut ink = Vec::with_capacity(self.edges.len() * 6);
        let mut edges: Vec<_> = self.edges.iter().collect();
        edges.sort_by_key(|e| e.priority);
        for e in edges {
            let highlighted = e.priority > 0;
            stroke(
                &mut ink,
                camera,
                r,
                e,
                if highlighted {
                    2.
                } else if wire {
                    1.5
                } else {
                    1.
                },
                e.color,
            );
        }
        ink
    }
}
pub fn vertex(camera: &Camera, r: Rect, point: (Pos2, f64), color: Color32) -> Vertex {
    let depth = if camera.view == View::Perspective {
        1. - camera.near() / point.1
    } else {
        let (offset, range) = camera.ortho_depth();
        (point.1 + offset) / range
    };
    Vertex {
        position: [
            2. * (point.0.x - r.left()) / r.width() - 1.,
            1. - 2. * (point.0.y - r.top()) / r.height(),
            (depth as f32).clamp(0., 1.),
        ],
        color,
    }
}
fn stroke(out: &mut Vec<Vertex>, camera: &Camera, r: Rect, e: &Edge, width: f32, color: Color32) {
    let [a, b] = e.ends;
    if a.0.distance_sq(b.0) < 1e-10 {
        return;
    }
    let d = (b.0 - a.0).normalized().rot90() * width * 0.5;
    for (pos, depth, source) in [
        (a.0 - d, a.1, a.0),
        (a.0 + d, a.1, a.0),
        (b.0 + d, b.1, b.0),
        (a.0 - d, a.1, a.0),
        (b.0 + d, b.1, b.0),
        (b.0 - d, b.1, b.0),
    ] {
        let mut v = vertex(camera, r, (pos, depth), color);
        // Continue the owner surface's depth plane across the widened stroke.
        // GPU slope/constant bias handles precision; strokes never write depth.
        v.position[2] = (v.position[2] + e.gradient.dot(pos - source)).clamp(0., 1.);
        out.push(v);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::viewport as palette;
    #[test]
    fn shared_edge_has_one_selected_owner_independent_of_face_order() {
        let plain = Edge {
            ends: [(pos2(10., 20.), 2.), (pos2(50., 25.), 3.)],
            gradient: Vec2::ZERO,
            color: palette::wireframe(),
            priority: 0,
        };
        let selected = Edge {
            color: palette::selection(),
            priority: 2,
            ..plain
        };
        for reversed in [false, true] {
            let mut o = Outlines::default();
            for e in if reversed {
                [selected, plain]
            } else {
                [plain, selected]
            } {
                o.insert(
                    1,
                    if e.priority == 0 { 2 } else { 3 },
                    if e.priority == 0 { 3 } else { 2 },
                    e,
                );
            }
            assert_eq!(o.edges.len(), 1);
            let ink = o.tessellate(
                &Camera::default(),
                Rect::from_min_size(Pos2::ZERO, vec2(100., 100.)),
                false,
            );
            assert_eq!(ink.len(), 6);
            assert!(ink.iter().all(|v| v.color == palette::selection()));
            // Identical coordinates in another item are not welded by rendering.
            o.insert(2, 2, 3, plain);
            assert_eq!(o.edges.len(), 2);
        }
    }
}
