//! Edge Weight tool (ME-T10, EW-01/EW-07): set the subdivision weight of edges.
//!
//! The selected edges get weight `weight`, a fraction shown as a percentage
//! Any value is stored as typed (EW2-01); the surface clamps the effect at
//! ±100 %. With Additive (EW2-05) `weight` is added to each edge's current
//! weight instead of replacing it. Weight 0 removes their records. Geometry
//! is unchanged. Weights are stored per edge (`MeshCrease`), saved with the
//! scene and drawn in the viewport. Positive weights crease smooth polygons
//! (sharpness = 10 × weight, see `subdivision`); negative weights soften
//! Subdivision (Tab) polygons only (EW2-04).

use crate::{Mesh, MeshCrease, Result};
use std::collections::BTreeSet;

/// Set (or with `additive`, add to) the weight of the selected edges of
/// `mesh`.
pub fn edge_crease_geometry(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    weight: f64,
    additive: bool,
) -> Result<Mesh> {
    mesh.validate()?;
    if !weight.is_finite() {
        return Err("Edge Weight must be a number".into());
    }
    let selected: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
    if selected.is_empty() {
        return Err("Select edges to weight".into());
    }
    let existing: BTreeSet<(u32, u32)> = mesh.edges().into_iter().collect();
    if !selected.is_subset(&existing) {
        return Err("A selected edge no longer exists".into());
    }
    let mut out = mesh.clone();
    let creases = &mut out.attributes.creases;
    let current: std::collections::BTreeMap<(u32, u32), f64> =
        creases.iter().map(|c| ((c.a, c.b), c.weight)).collect();
    creases.retain(|c| !selected.contains(&(c.a, c.b)));
    for &(a, b) in &selected {
        let weight = if additive {
            current.get(&(a, b)).copied().unwrap_or(0.) + weight
        } else {
            weight
        };
        if weight != 0. {
            creases.push(MeshCrease { a, b, weight });
        }
    }
    creases.sort_by_key(|c| (c.a, c.b));
    out.validate()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    #[test]
    fn weights_are_stored_as_typed_and_zero_removes() {
        // EW2-01: any value is kept; the surface clamps the effect.
        let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let e = cube.edges()[0];
        let neg = edge_crease_geometry(&cube, &[e], -0.5, false).unwrap();
        assert_eq!(neg.attributes.creases[0].weight, -0.5);
        let big = edge_crease_geometry(&cube, &[e], 3.0, false).unwrap();
        assert_eq!(big.attributes.creases[0].weight, 3.0);
        let small = edge_crease_geometry(&cube, &[e], -3.0, false).unwrap();
        assert_eq!(small.attributes.creases[0].weight, -3.0);
        let cleared = edge_crease_geometry(&neg, &[e], 0.0, false).unwrap();
        assert!(cleared.attributes.creases.is_empty());
        assert!(edge_crease_geometry(&cube, &[e], f64::NAN, false).is_err());
        assert!(edge_crease_geometry(&cube, &[], 0.5, false).is_err());
    }

    #[test]
    fn additive_adds_to_each_edges_weight() {
        // EW2-05: Additive adds to the current values instead of replacing.
        let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let (e, f) = (cube.edges()[0], cube.edges()[1]);
        let one = edge_crease_geometry(&cube, &[e], 0.5, false).unwrap();
        let both = edge_crease_geometry(&one, &[e, f], 0.25, true).unwrap();
        let w: Vec<f64> = both.attributes.creases.iter().map(|c| c.weight).collect();
        assert_eq!(w, vec![0.75, 0.25]);
        // Back to zero removes the record.
        let back = edge_crease_geometry(&both, &[f], -0.25, true).unwrap();
        assert_eq!(back.attributes.creases.len(), 1);
    }
}
