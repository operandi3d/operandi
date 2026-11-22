//! Painting stage 7: Geometry to Brush (PNT-261). Turns an open patch of a
//! mesh into a Texture tip picture: the patch is relaxed toward its
//! border (the same neighbour averaging as Relax Mesh, borders held), and
//! each point's offset from its relaxed position is written into the
//! picture, laid out in the plane facing the Projection Axis.
//! Painting builds only. The maths is the spec's route [PNT-Q34 in its
//! numbering]; provisional.
use crate::{Mesh, Result};
use glam::{DVec2, DVec3};
use operandi_paint::{Depth, Format, Layout, PaintImage};
use std::collections::BTreeMap;

/// Height Field: greyscale, height only (undercuts lost). Vector
/// Displacement: colour, the whole 3D offset (undercuts kept).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GeometryBrushMode {
    #[default]
    HeightField,
    VectorDisplacement,
}

impl GeometryBrushMode {
    pub fn label(self) -> &'static str {
        match self {
            GeometryBrushMode::HeightField => "Height Field",
            GeometryBrushMode::VectorDisplacement => "Vector Displacement",
        }
    }
}

/// The dialog's values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometryBrush {
    pub mode: GeometryBrushMode,
    /// The picture's side in pixels.
    pub resolution: u32,
    /// The patch's facing direction; `None`: worked out from the patch.
    pub axis: Option<DVec3>,
    /// Relaxing passes (more: more accurate, slower).
    pub iterations: u32,
}

impl Default for GeometryBrush {
    fn default() -> Self {
        GeometryBrush {
            mode: GeometryBrushMode::HeightField,
            resolution: 256,
            axis: None,
            iterations: 1000,
        }
    }
}

/// A patch with no open border: nothing to make a brush from.
pub const CLOSED_PATCH: &str = "Geometry to Brush needs an open patch: a closed mesh gives nothing";

/// The picture for `faces` of `mesh` (all its polygons when empty).
pub fn geometry_to_brush(mesh: &Mesh, faces: &[u32], o: &GeometryBrush) -> Result<PaintImage> {
    let picked: Vec<usize> = if faces.is_empty() {
        (0..mesh.faces.len())
            .filter(|&f| !mesh.is_polyline(f))
            .collect()
    } else {
        faces
            .iter()
            .map(|&f| f as usize)
            .filter(|&f| f < mesh.faces.len())
            .collect()
    };
    if picked.is_empty() {
        return Err("Select the patch to make a brush from".into());
    }
    // Edge use counts inside the patch: an edge used once is its border.
    let mut edges: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    let mut neighbours: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &f in &picked {
        let v = &mesh.faces[f].vertices;
        for k in 0..v.len() {
            let (a, b) = (v[k], v[(k + 1) % v.len()]);
            *edges.entry((a.min(b), a.max(b))).or_default() += 1;
            neighbours.entry(a).or_default().push(b);
            neighbours.entry(b).or_default().push(a);
        }
    }
    let border: std::collections::BTreeSet<u32> = edges
        .iter()
        .filter(|(_, n)| **n == 1)
        .flat_map(|(&(a, b), _)| [a, b])
        .collect();
    if border.is_empty() {
        return Err(CLOSED_PATCH.into());
    }
    for n in neighbours.values_mut() {
        n.sort_unstable();
        n.dedup();
    }
    let pos = |v: u32| mesh.vertices[v as usize];
    // Projection Axis: the patch's average facing.
    let axis = o
        .axis
        .unwrap_or_else(|| picked.iter().map(|&f| mesh.normal(f)).sum::<DVec3>())
        .normalize_or(DVec3::Z);
    // Relax: interior points to their neighbours' average, the border
    // held, `iterations` passes (capped by a settling test).
    let mut relaxed: BTreeMap<u32, DVec3> = neighbours.keys().map(|&v| (v, pos(v))).collect();
    for _ in 0..o.iterations.max(1) {
        let mut most = 0f64;
        let next: Vec<(u32, DVec3)> = neighbours
            .iter()
            .filter(|(v, _)| !border.contains(v))
            .map(|(&v, ns)| {
                let avg = ns.iter().map(|n| relaxed[n]).sum::<DVec3>() / ns.len() as f64;
                most = most.max(avg.distance(relaxed[&v]));
                (v, avg)
            })
            .collect();
        for (v, p) in next {
            relaxed.insert(v, p);
        }
        if most < 1e-9 {
            break;
        }
    }
    // The plane facing the axis, and the patch's extent in it.
    let (u_dir, v_dir) = axis.any_orthonormal_pair();
    let flat = |p: DVec3| DVec2::new(p.dot(u_dir), p.dot(v_dir));
    let (mut lo, mut hi) = (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY));
    for p in relaxed.values() {
        let q = flat(*p);
        lo = lo.min(q);
        hi = hi.max(q);
    }
    let span = (hi - lo).max_element().max(1e-12);
    let centre = (lo + hi) / 2.;
    let side = o.resolution.clamp(16, 4096);
    let to_px = |q: DVec2| {
        let n = (q - centre) / span + DVec2::splat(0.5);
        // Image y runs down; v runs up.
        DVec2::new(n.x * f64::from(side), (1. - n.y) * f64::from(side))
    };
    // Each point's offset from its relaxed place, in (u, v, axis).
    let offset = |v: u32| {
        let d = pos(v) - relaxed[&v];
        DVec3::new(d.dot(u_dir), d.dot(v_dir), d.dot(axis))
    };
    let reach = neighbours
        .keys()
        .map(|&v| match o.mode {
            GeometryBrushMode::HeightField => offset(v).z.abs(),
            GeometryBrushMode::VectorDisplacement => offset(v).abs().max_element(),
        })
        .fold(0f64, f64::max)
        .max(1e-12);
    let format = match o.mode {
        GeometryBrushMode::HeightField => Format::Gray,
        GeometryBrushMode::VectorDisplacement => Format::Rgb,
    };
    let mut px = PaintImage::new(Layout {
        width: side,
        height: side,
        format,
        depth: Depth::F32,
    })?;
    // Mid-grey everywhere: the zero point (Offset 50 %).
    for i in 0..px.pixel_count() {
        px.set(i, [0.5, 0.5, 0.5, 1.]);
    }
    let value = |d: DVec3| -> [f32; 4] {
        match o.mode {
            GeometryBrushMode::HeightField => {
                let g = (0.5 + 0.5 * d.z / reach) as f32;
                [g, g, g, 1.]
            }
            GeometryBrushMode::VectorDisplacement => {
                let c = d / reach * 0.5 + DVec3::splat(0.5);
                [c.x as f32, c.y as f32, c.z as f32, 1.]
            }
        }
    };
    for &f in &picked {
        let Ok(tris) = mesh.surface_triangles(f) else {
            continue;
        };
        let vs = &mesh.faces[f].vertices;
        for t in tris {
            let ids = t.map(|c| vs[c]);
            let p = ids.map(|v| to_px(flat(relaxed[&v])));
            let d = ids.map(offset);
            let (min, max) = (p[0].min(p[1]).min(p[2]), p[0].max(p[1]).max(p[2]));
            let area = (p[1] - p[0]).perp_dot(p[2] - p[0]);
            if area.abs() < 1e-12 {
                continue;
            }
            let (x0, y0) = (min.x.floor().max(0.) as u32, min.y.floor().max(0.) as u32);
            let (x1, y1) = (
                (max.x.ceil() as u32).min(side - 1),
                (max.y.ceil() as u32).min(side - 1),
            );
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let q = DVec2::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
                    let w0 = (p[1] - q).perp_dot(p[2] - q) / area;
                    let w1 = (p[2] - q).perp_dot(p[0] - q) / area;
                    let w2 = 1. - w0 - w1;
                    if w0 < -1e-6 || w1 < -1e-6 || w2 < -1e-6 {
                        continue;
                    }
                    px.set(
                        (y * side + x) as usize,
                        value(d[0] * w0 + d[1] * w1 + d[2] * w2),
                    );
                }
            }
        }
    }
    Ok(px)
}
