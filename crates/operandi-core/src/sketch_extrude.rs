//! Sketch Extrude, Sweep mode: extrude the selected polygons along a path
//! (spec `sketch-extrude-2026-11-05`, v0: settings and modes only).
//!
//! Sketch Extrude extrudes the selected polygons by
//! drawing in the viewport; the extrusion follows the pointer, with a new
//! span every Precision pixels. Operandi builds the sweep as one Extrude per
//! path step: the selected polygons are extruded by the first step, the
//! resulting caps by the next, and so on, so every span gets connected walls
//! and the final caps end at the last path point. Any surface selection the
//! Extrude tool supports works here, with the same attribute and UV handling.
//!
//! The sweep geometry is provisional (Q-SE-01 is open). Precision starts at
//! 30 pixels, carries over between uses and Reset restores it (SE-D0).
//! Sections follow the path with Align to Path on (sketch-extrude-drag
//! E1); Align off is open (Q-SE-04), so there is no switch yet.
//! Current limits (spec settings not built yet): no Uniform, Scale, Spin,
//! Profiles or Make UVs choices;
//! Edit Path / Delete Knot / Uniform Spans / Straight modes are absent.
use crate::{ExtrudeMode, ExtrudeUvs, Mesh, Result, extrude_geometry};
use glam::{DQuat, DVec3};

/// Eligibility of the selection, checked when the tool activates.
pub fn validate_sketch_target(source: &Mesh, selected: &[u32]) -> Result<()> {
    source.validate_extrude_selection(selected)
}

/// Returns the swept mesh and the final cap polygons. Path points are world
/// positions; the first one is the anchor, and section positions are moved
/// by each point's offset from it.
pub fn sketch_extrude_geometry(
    source: &Mesh,
    selected: &[u32],
    path: &[DVec3],
) -> Result<(Mesh, Vec<u32>)> {
    validate_sketch_target(source, selected)?;
    if !(2..=257).contains(&path.len()) || path.iter().any(|p| !p.is_finite()) {
        return Err("Sketch Extrude path needs 2–257 finite points".into());
    }
    for triple in path.windows(3) {
        let (a, b) = (triple[1] - triple[0], triple[2] - triple[1]);
        if a.cross(b).length() <= 1e-10 * a.length() * b.length() && a.dot(b) < 0. {
            return Err("Sketch Extrude requires a path without retraced segments".into());
        }
    }
    // A step parallel to a section edge would make a zero-area wall.
    let edges: Vec<DVec3> = selected
        .iter()
        .flat_map(|&f| {
            let vs = &source.faces[f as usize].vertices;
            (0..vs.len()).map(move |i| (vs[i], vs[(i + 1) % vs.len()]))
        })
        .map(|(a, b)| source.vertices[b as usize] - source.vertices[a as usize])
        .collect();
    for pair in path.windows(2) {
        let step = pair[1] - pair[0];
        if edges
            .iter()
            .any(|e| step.cross(*e).length() <= 1e-10 * step.length() * e.length())
        {
            return Err("Sketch Extrude path creates a degenerate wall".into());
        }
    }
    let mut mesh = source.clone();
    let mut caps = selected.to_vec();
    // Align to Path (on, the default; off is open, Q-SE-04): each section
    // turns by the rotation from the first step's direction to the path's
    // direction at its point, about its own centre.
    let cap_points = |mesh: &Mesh, caps: &[u32]| {
        let mut points: Vec<u32> = caps
            .iter()
            .flat_map(|&f| mesh.faces[f as usize].vertices.iter().copied())
            .collect();
        points.sort_unstable();
        points.dedup();
        points
    };
    let start_points = cap_points(source, selected);
    let centre0 = start_points
        .iter()
        .map(|&v| source.vertices[v as usize])
        .sum::<DVec3>()
        / start_points.len().max(1) as f64;
    let first = (path[1] - path[0]).normalize();
    let tangent = |i: usize| {
        let last = path.len() - 1;
        let out = |j: usize| (path[j + 1] - path[j]).normalize();
        if i >= last {
            out(last - 1)
        } else {
            (out(i - 1) + out(i)).try_normalize().unwrap_or(out(i))
        }
    };
    let mut previous = DQuat::IDENTITY;
    for (k, pair) in path.windows(2).enumerate() {
        let step = pair[1] - pair[0];
        if step.length() < 1e-9 {
            return Err("Sketch Extrude path contains repeated points".into());
        }
        // Shift for every span: the section moves and walls join it to the
        // previous one, leaving no closing face behind (no spec covers
        // lone polygons; this keeps the admitted open-bottom fixture).
        let result = extrude_geometry(
            &mesh,
            &caps,
            step,
            1,
            ExtrudeMode::Shift,
            ExtrudeUvs::None,
            None,
        )
        .map_err(|e| format!("Sketch Extrude span {}: {e}", k + 1))?;
        mesh = result.mesh;
        caps = result.caps;
        // Turn the new section: each cap point came from `q - step`.
        let rotation = DQuat::from_rotation_arc(first, tangent(k + 1));
        let delta = rotation * previous.inverse();
        previous = rotation;
        let centre = centre0 + (pair[1] - path[0]);
        let previous_centre = centre0 + (pair[0] - path[0]);
        for v in cap_points(&mesh, &caps) {
            let q = mesh.vertices[v as usize];
            mesh.vertices[v as usize] = centre + delta * (q - step - previous_centre);
        }
    }
    mesh.validate()?;
    Ok((mesh, caps))
}
