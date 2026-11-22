//! Symmetry Tool (symmetry-2026-11-13 section 9 and v3.1, SYM-B3-09): fix
//! a nearly symmetric mesh. Dragging raises Threshold; each point whose
//! mirror partner lies within Threshold of its mirror position snaps to it.
//! Reverse off moves the negative side to match the positive side (all
//! three coordinates), Reverse on the other way; points further apart than
//! Threshold don't move. Use Topology finds partners through connectivity,
//! so a point pulled well off its mirror position still pairs up.
//!
//! Provisional (SYM3-Q16): a point within half of Threshold of the plane,
//! with no other partner, snaps onto the plane; unpaired points are the ones
//! still without a partner afterwards (the viewport marks them).
use crate::symmetry::{Side, Symmetry, match_tolerance, topology_counterparts};
use crate::{Mesh, Result};
use glam::DVec3;

/// The Symmetry Tool's options.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SymmetryFix {
    /// World distance (metres).
    pub threshold: f64,
    pub topology: bool,
    pub reverse: bool,
}
impl Default for SymmetryFix {
    /// v3.1: Threshold 1 mm, Use Topology on, Reverse off.
    fn default() -> Self {
        Self {
            threshold: 0.001,
            topology: true,
            reverse: false,
        }
    }
}

/// What the fix did.
#[derive(Clone, Debug, PartialEq)]
pub struct FixOutcome {
    pub mesh: Mesh,
    /// Points that moved.
    pub moved: usize,
    /// Points still without a mirror partner (marked in the viewport).
    pub unpaired: Vec<u32>,
}

/// Fix `mesh` (local space; `threshold` local too).
pub fn symmetry_fix(
    mesh: &Mesh,
    symmetry: Symmetry,
    threshold: f64,
    topology: bool,
    reverse: bool,
) -> Result<FixOutcome> {
    let Some(n) = symmetry.normal() else {
        return Err("Turn Symmetry on (X, Y or Z) to use the Symmetry Tool".into());
    };
    if !threshold.is_finite() || threshold < 0. {
        return Err("Symmetry Tool Threshold must be zero or more".into());
    }
    let mut out = mesh.clone();
    let tolerance = match_tolerance(mesh);
    let distance = |p: DVec3| n.dot(p) - symmetry.offset;
    let moving = if reverse {
        Side::Positive
    } else {
        Side::Negative
    };
    // Seam first: points that are their own mirror within Threshold land on
    // the plane, so the topology walk can start from them.
    let near_plane = |p: DVec3| distance(p).abs() * 2. <= threshold.max(tolerance);
    let partners: Vec<Option<u32>> = if topology {
        let mut seamed = mesh.clone();
        for p in &mut seamed.vertices {
            if near_plane(*p) {
                *p = symmetry.onto_plane(*p);
            }
        }
        topology_counterparts(&seamed, symmetry)
    } else {
        let targets: Vec<DVec3> = mesh.vertices.iter().map(|&p| symmetry.mirror(p)).collect();
        crate::symmetry_sides::points_at(mesh, &targets, threshold.max(tolerance))
    };
    let mut moved = 0;
    for (v, partner) in partners.iter().enumerate() {
        let p = mesh.vertices[v];
        let partner = partner.filter(|&q| q as usize != v);
        match partner {
            None => {
                if near_plane(p) && distance(p).abs() > 0. {
                    out.vertices[v] = symmetry.onto_plane(p);
                    moved += 1;
                }
            }
            Some(q) => {
                if symmetry.side(p, 0.) != Some(moving) {
                    continue;
                }
                let target = symmetry.mirror(mesh.vertices[q as usize]);
                if p.distance(target) <= threshold && p != target {
                    out.vertices[v] = target;
                    moved += 1;
                }
            }
        }
    }
    let unpaired = crate::symmetry::unpaired_points(&out, symmetry, tolerance);
    Ok(FixOutcome {
        mesh: out,
        moved,
        unpaired,
    })
}
