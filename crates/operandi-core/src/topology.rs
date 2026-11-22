//! Small edge helpers shared by the mesh tools.

/// An edge's key: its two point indices, low first, so both directions of
/// the same edge compare equal.
#[inline]
pub fn edge_key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// The ends of `edges`, sorted, with no repeats.
pub fn edge_points(edges: &[(u32, u32)]) -> Vec<u32> {
    let mut points: Vec<u32> = edges.iter().flat_map(|&(a, b)| [a, b]).collect();
    points.sort_unstable();
    points.dedup();
    points
}
