//! Live patches (G7 PC-7, Jason 2026-11-21): a `PolygonRepresentation::Patch`
//! polygon lists only its 3 or 4 corner points. Its surface is the Coons
//! patch of `patch.rs` over the open curves of the same mesh that join each
//! pair of neighbouring corners (found by that pair of points), tessellated
//! into `cells` x `cells` at the item's Patch Subdivisions (16 by default).
//!
//! The evaluated scene (`evaluate.rs`) shows the tessellation, so the
//! viewport, Cycles and exports read it; the document keeps the patch
//! polygon, so saves keep it and moving a boundary curve reshapes it live.
//! Freeze turns it into those polygons for good.
//!
//! The first cell takes the patch polygon's place (polygon numbers of the
//! other polygons stay put) and the rest follow the mesh's own polygons;
//! the corners are the patch's own points and new points follow the mesh's
//! own points. Corner data (UVs, colours) is blended bilinearly from the
//! patch's corners.
//!
//! PROVISIONAL (PC-Q5): a patch whose boundary curves can't all be found
//! (one was deleted) is broken: it draws nothing and Freeze leaves it.
use crate::edge_commands::{CornerBlend, OutFace, rebuild};
use crate::patch::{PatchSpacing, coons_patch};
use crate::{Mesh, PolygonRepresentation, Result, Shared};
use glam::DVec3;

/// A new mesh's Patch Subdivisions (PC-7: 16 x 16 cells).
pub const DEFAULT_CELLS: u32 = 16;
/// The most cells a side may have.
pub const MAX_CELLS: u32 = 256;

/// Ends closer than this join a patch corner (as Draw Patch picks them).
const END_TOLERANCE: f64 = 1e-4;

/// Whether polygon `f` is a live patch.
pub fn is_patch(mesh: &Mesh, f: usize) -> bool {
    mesh.attributes.representations.get(f) == Some(&PolygonRepresentation::Patch)
}

/// Whether `mesh` has a live patch (reads only the representation list).
pub fn has_patches(mesh: &Mesh) -> bool {
    mesh.attributes
        .representations
        .iter()
        .take(mesh.faces.len())
        .any(|r| *r == PolygonRepresentation::Patch)
}

/// One open curve of the mesh: its sampled points and end point ids.
struct Boundary {
    samples: Vec<DVec3>,
    ends: [u32; 2],
}

fn boundaries(mesh: &Mesh) -> Vec<Boundary> {
    (0..mesh.faces.len())
        .filter_map(|f| match mesh.attributes.representations.get(f) {
            Some(PolygonRepresentation::Polyline {
                closed: false,
                curve,
            }) => {
                let ids = &mesh.faces[f].vertices;
                let control: Vec<DVec3> = ids.iter().map(|&v| mesh.vertices[v as usize]).collect();
                let samples =
                    crate::curves::sample(&control, *curve, false, crate::curves::FREEZE_STEPS);
                if samples.len() < 2 || ids.len() < 2 {
                    return None;
                }
                Some(Boundary {
                    samples,
                    ends: [ids[0], ids[ids.len() - 1]],
                })
            }
            _ => None,
        })
        .collect()
}

/// The boundary from corner `a` to corner `b`, oriented a → b: the curve
/// whose end points are `a` and `b`, or failing that whose ends sit on them.
fn find(mesh: &Mesh, curves: &[Boundary], a: u32, b: u32) -> Option<Vec<DVec3>> {
    let (pa, pb) = (mesh.vertices[a as usize], mesh.vertices[b as usize]);
    let near = |v: u32, p: DVec3| (mesh.vertices[v as usize] - p).length() <= END_TOLERANCE;
    let by_id = curves.iter().find(|c| c.ends == [a, b] || c.ends == [b, a]);
    let hit = by_id.or_else(|| {
        curves.iter().find(|c| {
            (near(c.ends[0], pa) && near(c.ends[1], pb))
                || (near(c.ends[0], pb) && near(c.ends[1], pa))
        })
    })?;
    let mut s = hit.samples.clone();
    if hit.ends[0] == b || (hit.ends[0] != a && !near(hit.ends[0], pa)) {
        s.reverse();
    }
    // The patch's own corner points are the ends exactly.
    s[0] = pa;
    *s.last_mut().unwrap() = pb;
    Some(s)
}

/// The boundary curves of patch polygon `f`, or `None` when it is broken.
fn patch_boundary(mesh: &Mesh, curves: &[Boundary], f: usize) -> Option<Vec<Vec<DVec3>>> {
    let corners = &mesh.faces[f].vertices;
    let n = corners.len();
    let sides = match n {
        // Three corners: the side from the last corner back to the first is
        // the third curve; the patch collapses at the third corner (PC-Q1).
        3 | 4 => n,
        _ => return None,
    };
    (0..sides)
        .map(|k| find(mesh, curves, corners[k], corners[(k + 1) % n]))
        .collect()
}

/// Whether patch polygon `f` can't be worked out (a boundary curve is
/// missing): PC-Q5 flags it broken.
pub fn is_broken(mesh: &Mesh, f: usize) -> bool {
    is_patch(mesh, f) && patch_boundary(mesh, &boundaries(mesh), f).is_none()
}

/// The live patches of `mesh` that are broken.
pub fn broken_patches(mesh: &Mesh) -> Vec<u32> {
    if !has_patches(mesh) {
        return Vec::new();
    }
    let curves = boundaries(mesh);
    (0..mesh.faces.len())
        .filter(|&f| is_patch(mesh, f) && patch_boundary(mesh, &curves, f).is_none())
        .map(|f| f as u32)
        .collect()
}

/// `mesh` with the live patches `chosen` (all when `None`) replaced by their
/// tessellation. Broken patches are left out when `drop_broken`, else kept
/// as patch polygons. `None` when nothing was chosen.
pub fn tessellate(
    mesh: &Mesh,
    cells: u32,
    chosen: Option<&[bool]>,
    drop_broken: bool,
) -> Result<Option<Mesh>> {
    let wanted = |f: usize| is_patch(mesh, f) && chosen.is_none_or(|c| c.get(f) == Some(&true));
    if !(0..mesh.faces.len()).any(wanted) {
        return Ok(None);
    }
    let cells = cells.clamp(1, MAX_CELLS);
    let curves = boundaries(mesh);
    let mut vertices = mesh.vertices.clone();
    let mut faces: Vec<OutFace> = Vec::with_capacity(mesh.faces.len());
    let mut extra: Vec<OutFace> = Vec::new();
    let mut cell_faces: Vec<bool> = Vec::new();
    let mut extra_cells = 0usize;
    let keep = |f: usize| {
        let n = mesh.faces[f].vertices.len();
        OutFace {
            template: f,
            vertices: mesh.faces[f].vertices.clone(),
            donors: (0..n).map(|c| Some((f, c))).collect(),
            blends: vec![None; n],
        }
    };
    for f in 0..mesh.faces.len() {
        if !wanted(f) {
            faces.push(keep(f));
            cell_faces.push(false);
            continue;
        }
        let Some(boundary) = patch_boundary(mesh, &curves, f) else {
            if !drop_broken {
                faces.push(keep(f));
                cell_faces.push(false);
            }
            continue;
        };
        let sides: Vec<&[DVec3]> = boundary.iter().map(|b| b.as_slice()).collect();
        let grid = coons_patch(
            &sides,
            cells,
            cells,
            PatchSpacing::Length,
            PatchSpacing::Length,
            false,
        )?;
        let corner_ids = &mesh.faces[f].vertices;
        let three = corner_ids.len() == 3;
        let row = cells as usize + 1;
        // Grid point → mesh point: corners are the patch's own points.
        let mut map = Vec::with_capacity(grid.mesh.vertices.len());
        for (g, p) in grid.mesh.vertices.iter().enumerate() {
            if let Some(k) = grid.corners.iter().position(|&c| c as usize == g) {
                map.push(corner_ids[k]);
            } else {
                map.push(u32::try_from(vertices.len()).map_err(|_| "Mesh point budget exceeded")?);
                vertices.push(*p);
            }
        }
        // Bilinear weights of the patch's corners at grid point `g`.
        let weights = |g: usize| -> Vec<f64> {
            let (i, j) = (g % row, g / row);
            let (u, v) = (i as f64 / cells as f64, (j as f64 / cells as f64).min(1.));
            if three {
                vec![(1. - u) * (1. - v), u * (1. - v), v]
            } else {
                vec![(1. - u) * (1. - v), u * (1. - v), u * v, (1. - u) * v]
            }
        };
        for (c, cell) in grid.mesh.faces.iter().enumerate() {
            let mut donors = Vec::with_capacity(cell.vertices.len());
            let mut blends = Vec::with_capacity(cell.vertices.len());
            for &g in &cell.vertices {
                match grid.corners.iter().position(|&k| k == g) {
                    Some(k) => {
                        donors.push(Some((f, k)));
                        blends.push(None);
                    }
                    None => {
                        let blend: CornerBlend = weights(g as usize)
                            .into_iter()
                            .enumerate()
                            .filter(|(_, w)| *w > 0.)
                            .map(|(k, w)| ((f, k), w))
                            .collect();
                        donors.push(None);
                        blends.push(Some(blend));
                    }
                }
            }
            let out = OutFace {
                template: f,
                vertices: cell.vertices.iter().map(|&g| map[g as usize]).collect(),
                donors,
                blends,
            };
            if c == 0 {
                faces.push(out);
                cell_faces.push(true);
            } else {
                extra.push(out);
                extra_cells += 1;
            }
        }
    }
    faces.extend(extra);
    cell_faces.extend(std::iter::repeat_n(true, extra_cells));
    let mut out = rebuild(mesh, vertices, faces, &[], "Patch")?;
    let reps = &mut out.attributes.representations;
    if reps.len() < cell_faces.len() {
        reps.resize(cell_faces.len(), PolygonRepresentation::default());
    }
    for (f, cell) in cell_faces.iter().enumerate() {
        if *cell {
            reps[f] = PolygonRepresentation::Face;
        }
    }
    out.validate()?;
    Ok(Some(out))
}

/// What the evaluated scene shows for a mesh with live patches.
struct Shown {
    cells: u32,
    /// A slot, so its content stamp stays the same while the mesh does.
    mesh: Option<crate::MeshSlot>,
}

/// `mesh` as drawn, rendered and exported: its live patches tessellated at
/// `cells`, broken ones left out. `None` when it has no live patch (or the
/// tessellation fails: the mesh then shows as stored). Remembered per
/// shared mesh, so an unchanged mesh gives the same shared result every
/// frame and the viewport's caches keep it.
pub fn shown(mesh: &Shared<Mesh>, cells: u32) -> Option<crate::MeshSlot> {
    let compute = |m: &Mesh| Shown {
        cells,
        mesh: has_patches(m)
            .then(|| tessellate(m, cells, None, true).ok().flatten())
            .flatten()
            .map(crate::MeshSlot::from),
    };
    let memo = mesh.derived(compute);
    if memo.cells == cells {
        return memo.mesh.clone();
    }
    compute(mesh).mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CurveKind, Face};

    fn line(m: &mut Mesh, a: u32, b: u32) {
        m.faces.push(Face {
            vertices: vec![a, b],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
        let f = m.faces.len() - 1;
        let reps = &mut m.attributes.representations;
        reps.resize(f, PolygonRepresentation::Face);
        reps.push(PolygonRepresentation::Polyline {
            closed: false,
            curve: CurveKind::Line,
        });
    }

    fn patch(m: &mut Mesh, corners: Vec<u32>) {
        m.faces.push(Face {
            vertices: corners,
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
        let f = m.faces.len() - 1;
        let reps = &mut m.attributes.representations;
        reps.resize(f, PolygonRepresentation::Face);
        reps.push(PolygonRepresentation::Patch);
    }

    fn square() -> Mesh {
        let mut m = Mesh {
            vertices: vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 0., 1.), DVec3::Z],
            ..Default::default()
        };
        for k in 0..4 {
            line(&mut m, k, (k + 1) % 4);
        }
        patch(&mut m, vec![0, 1, 2, 3]);
        m
    }

    #[test]
    fn a_patch_tessellates_in_place() {
        let m = square();
        m.validate().unwrap();
        let t = tessellate(&m, 4, None, true).unwrap().unwrap();
        assert_eq!(t.faces.len(), 4 + 16);
        assert_eq!(t.vertices.len(), 4 + 25 - 4);
        assert!(!has_patches(&t));
        // The first cell sits in the patch's place, on its first corner.
        assert!(t.faces[4].vertices.contains(&0));
    }

    #[test]
    fn a_missing_curve_breaks_the_patch() {
        let mut m = square();
        // Delete the curve joining corners 1 and 2.
        m.faces.remove(1);
        m.attributes.representations.remove(1);
        m.validate().unwrap();
        assert!(is_broken(&m, 3));
        assert_eq!(broken_patches(&m), vec![3]);
        let t = tessellate(&m, 4, None, true).unwrap().unwrap();
        assert_eq!(t.faces.len(), 3, "a broken patch draws nothing");
    }
}
