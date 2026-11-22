//! Peeler (E5, BEHAVIOR-UV-TOOLS §5.1, UVT-30/31): lays clean quad strips
//! and patches (belts, straps, arms, pipes) out as a regular grid. The
//! selected edges are the cut: the grid spreads across the quads loop by
//! loop until it meets the cut again from the other side.
//!
//! Each quad region the walk reaches gets lattice coordinates: the first
//! quad's corners are (0,0) (1,0) (1,1) (0,1), and each quad across an
//! uncut edge continues the lattice away from it. Polygons that aren't
//! quads stop the walk (UVT-31: poor results there are expected).
//! Uniformity blends each column and row's width from following its 3D
//! length (0 %) to all the same (100 %).
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md UVT-Q04, UVE5-P01/P02):
//! Uniformity starts at 0 %; every grid region is stretched to fill the
//! Min/Max U/V box; the walk starts at the lowest-numbered quad touching
//! the cut (or the first quad).
use crate::topology::edge_key;
use crate::{Mesh, Result, Scene};
use glam::{DVec2, IVec2};
use std::collections::{BTreeSet, VecDeque};

/// Peeler's Tool Properties (UVT-30).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PeelOptions {
    pub min: DVec2,
    pub max: DVec2,
    /// 0..1: 0 follows 3D sizes, 1 makes an even grid.
    pub uniformity: f64,
}
impl Default for PeelOptions {
    fn default() -> Self {
        Self {
            min: DVec2::ZERO,
            max: DVec2::ONE,
            uniformity: 0.,
        }
    }
}
impl PeelOptions {
    pub fn validate(&self) -> Result<()> {
        if !(self.min.is_finite() && self.max.is_finite()) {
            return Err("Peeler's box must be numbers".into());
        }
        if self.max.x <= self.min.x || self.max.y <= self.min.y {
            return Err("Peeler's Max U and Max V must be above Min U and Min V".into());
        }
        if !(0. ..=1.).contains(&self.uniformity) {
            return Err("Uniformity runs from 0 % to 100 %".into());
        }
        Ok(())
    }
}

/// Peeler on the scene's active mesh: the visible quads' UVs in the
/// selected map ("Texture" made when missing).
pub fn peel_mesh(scene: &Scene, o: &PeelOptions) -> Result<Mesh> {
    o.validate()?;
    let item = scene.active().ok_or("Select a mesh")?;
    let mesh = &item.mesh;
    let hidden = crate::Protection::of(mesh, &item.elements)
        .map(|p| p.face_hidden)
        .unwrap_or_default();
    let quad = |f: usize| {
        !hidden.get(f).copied().unwrap_or(false)
            && !mesh.is_polyline(f)
            && mesh.faces[f].vertices.len() == 4
    };
    let cut: BTreeSet<(u32, u32)> = if scene.selection.mode == crate::Mode::Edge {
        scene
            .selection
            .edges
            .iter()
            .map(|&(a, b)| edge_key(a, b))
            .collect()
    } else {
        BTreeSet::new()
    };
    let edge_faces = mesh.edge_faces();
    // Lattice coordinates per polygon corner, island per polygon.
    let mut coord: Vec<Option<[IVec2; 4]>> = vec![None; mesh.faces.len()];
    let mut island: Vec<usize> = vec![usize::MAX; mesh.faces.len()];
    let mut islands = 0;
    let start_order: Vec<usize> = {
        let touching: Vec<usize> = (0..mesh.faces.len())
            .filter(|&f| quad(f))
            .filter(|&f| {
                let v = &mesh.faces[f].vertices;
                (0..4).any(|i| cut.contains(&edge_key(v[i], v[(i + 1) % 4])))
            })
            .collect();
        touching
            .into_iter()
            .chain((0..mesh.faces.len()).filter(|&f| quad(f)))
            .collect()
    };
    for seed in start_order {
        if coord[seed].is_some() {
            continue;
        }
        coord[seed] = Some([
            IVec2::new(0, 0),
            IVec2::new(1, 0),
            IVec2::new(1, 1),
            IVec2::new(0, 1),
        ]);
        island[seed] = islands;
        let mut queue = VecDeque::from([seed]);
        while let Some(f) = queue.pop_front() {
            let v = mesh.faces[f].vertices.clone();
            let c = coord[f].expect("queued polygons have coordinates");
            for k in 0..4 {
                let (a, b) = (v[k], v[(k + 1) % 4]);
                let e = edge_key(a, b);
                if cut.contains(&e) {
                    continue;
                }
                let Some(list) = edge_faces.get(&e) else {
                    continue;
                };
                for &g in list {
                    if g == f || !quad(g) || coord[g].is_some() {
                        continue;
                    }
                    // Away from f: the step from the shared edge into f,
                    // reversed.
                    let inward = c[(k + 2) % 4] - c[(k + 1) % 4];
                    let w = &mesh.faces[g].vertices;
                    let (Some(ia), Some(ib)) = (
                        w.iter().position(|&x| x == a),
                        w.iter().position(|&x| x == b),
                    ) else {
                        continue;
                    };
                    let mut out = [IVec2::ZERO; 4];
                    out[ia] = c[k];
                    out[ib] = c[(k + 1) % 4];
                    // The corner after a (or before, by winding) sits
                    // across from a, the other across from b.
                    let next_of = |i: usize| (i + 1) % 4;
                    let prev_of = |i: usize| (i + 3) % 4;
                    let (across_a, across_b) = if next_of(ib) == ia {
                        // g runs b -> a: after a comes the corner across a.
                        (next_of(ia), prev_of(ib))
                    } else {
                        (prev_of(ia), next_of(ib))
                    };
                    out[across_a] = c[k] - inward;
                    out[across_b] = c[(k + 1) % 4] - inward;
                    coord[g] = Some(out);
                    island[g] = islands;
                    queue.push_back(g);
                }
            }
        }
        islands += 1;
    }
    if islands == 0 {
        return Err("Peeler needs quads: the visible polygons have none".into());
    }
    let map = crate::uv_tools::target_uv_map(mesh, None);
    let mut faces = Vec::new();
    let mut rows = Vec::new();
    for i in 0..islands {
        let members: Vec<usize> = (0..mesh.faces.len()).filter(|&f| island[f] == i).collect();
        let (lo, hi) = members
            .iter()
            .flat_map(|&f| coord[f].unwrap())
            .fold((IVec2::MAX, IVec2::MIN), |(lo, hi), p| {
                (lo.min(p), hi.max(p))
            });
        let size = (hi - lo).max(IVec2::ONE);
        // Column (x) and row (y) widths: the mean 3D length of the lattice
        // edges spanning each step.
        let mut widths = [vec![0f64; size.x as usize], vec![0f64; size.y as usize]];
        let mut counts = [vec![0u32; size.x as usize], vec![0u32; size.y as usize]];
        for &f in &members {
            let c = coord[f].unwrap();
            let v = &mesh.faces[f].vertices;
            for k in 0..4 {
                let (p, q) = (c[k] - lo, c[(k + 1) % 4] - lo);
                let len =
                    mesh.vertices[v[k] as usize].distance(mesh.vertices[v[(k + 1) % 4] as usize]);
                if p.y == q.y && p.x != q.x {
                    let s = p.x.min(q.x) as usize;
                    widths[0][s] += len;
                    counts[0][s] += 1;
                } else if p.x == q.x && p.y != q.y {
                    let s = p.y.min(q.y) as usize;
                    widths[1][s] += len;
                    counts[1][s] += 1;
                }
            }
        }
        let place = |axis: usize| -> Vec<f64> {
            let n = widths[axis].len();
            let mean: Vec<f64> = (0..n)
                .map(|s| widths[axis][s] / counts[axis][s].max(1) as f64)
                .collect();
            let total: f64 = mean.iter().sum::<f64>().max(1e-12);
            let mut at = vec![0.; n + 1];
            for s in 0..n {
                let geo = mean[s] / total;
                let even = 1. / n as f64;
                at[s + 1] = at[s] + geo * (1. - o.uniformity) + even * o.uniformity;
            }
            at
        };
        let (us, vs) = (place(0), place(1));
        for &f in &members {
            let c = coord[f].unwrap();
            let row: Vec<DVec2> = c
                .iter()
                .map(|&p| {
                    let p = p - lo;
                    let t = DVec2::new(us[p.x as usize], vs[p.y as usize]);
                    o.min + (o.max - o.min) * t
                })
                .collect();
            faces.push(f as u32);
            rows.push(row);
        }
    }
    let out = crate::uv_project::write_rows(mesh, &map, &faces, &rows)?;
    out.validate()?;
    Ok(out)
}
