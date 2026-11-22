//! Next Loop / Previous Loop: step the selected loop to the parallel loop
//! beside it, replacing the selection (SEL-LP-02/03, SEL-B2-01).
//!
//! Polygons (measured on a 4×4 grid): the loop is walked from its first
//! selected polygon; Next takes the neighbours on the right-hand side of
//! that walk (seen from the polygons' front, i.e. their winding), Previous
//! the left-hand side, in the same order. If that side is the open border
//! for the whole loop, the step bounces to the other side and the order is
//! reversed, so a further Next keeps moving away from the border
//! (SEL-O-L2-01-Polygon-next: [3,7,11,15] → [14,10,6,2]).
//!
//! Edges (measured): Next and Previous both cross the polygon whose winding
//! runs along the first selected edge from its lower to its higher point
//! (the only polygon on a border edge); the measured X=2 loop gave X=1 for
//! both commands. The opposite edge of that quad seeds a new edge loop; an
//! open-border edge gives the whole border loop, in walk order starting
//! there (SEL-LP-03).
//!
//! Points, the Shift "add" variants, and where the direction rules hold
//! beyond the measured grid are open (SEL-Q-LOOP-02). Everything here reads
//! only the selection's neighbourhood through the cached point → polygons
//! map.
use crate::topology_cache::PointFaces;
use crate::{Mesh, Result};

/// The polygon edges as (point, next point), in winding order.
fn sides(face: &[u32]) -> Vec<(u32, u32)> {
    face.iter()
        .zip(face.iter().cycle().skip(1))
        .map(|(&a, &b)| (a, b))
        .collect()
}

fn same_edge(a: (u32, u32), b: (u32, u32)) -> bool {
    a == b || a == (b.1, b.0)
}

/// The other polygon using edge `(a, b)`, if exactly one other does.
fn across(mesh: &Mesh, faces: &PointFaces, face: usize, edge: (u32, u32)) -> Option<usize> {
    let mut others = faces.of(edge.0).iter().map(|&f| f as usize).filter(|&f| {
        f != face
            && sides(&mesh.faces[f].vertices)
                .iter()
                .any(|&s| same_edge(s, edge))
    });
    let first = others.next()?;
    others.next().is_none().then_some(first)
}

/// Index in `face`'s sides of the edge shared with `other`.
fn shared_side(mesh: &Mesh, face: usize, other: usize) -> Option<usize> {
    let theirs = sides(&mesh.faces[other].vertices);
    sides(&mesh.faces[face].vertices)
        .iter()
        .position(|&s| theirs.iter().any(|&t| same_edge(s, t)))
}

/// The selected polygons as one walk: an open strip from an end, or a
/// closed band. Starts at the first selected polygon (or the strip end it
/// leads to) and heads towards the neighbour selected earliest.
fn polygon_walk(mesh: &Mesh, faces: &PointFaces, ordered: &[usize]) -> Result<(Vec<usize>, bool)> {
    const NOT_A_LOOP: &str = "Next/Previous Loop needs one polygon loop selected (use Loop first)";
    // Pick order of each selected polygon, looked up in constant time so long
    // loops on dense meshes stay linear.
    let order: std::collections::HashMap<usize, usize> = ordered
        .iter()
        .enumerate()
        .rev()
        .map(|(i, &f)| (f, i))
        .collect();
    let rank = |f: usize| order.get(&f).copied();
    let neighbours = |f: usize| -> Vec<usize> {
        let mut found: Vec<usize> = sides(&mesh.faces[f].vertices)
            .into_iter()
            .filter_map(|edge| across(mesh, faces, f, edge))
            .filter(|&n| rank(n).is_some())
            .collect();
        found.sort_by_key(|&n| rank(n));
        found.dedup();
        found
    };
    if ordered.len() < 2 {
        return Err(NOT_A_LOOP.into());
    }
    let links: Vec<Vec<usize>> = ordered.iter().map(|&f| neighbours(f)).collect();
    if links.iter().any(|l| l.is_empty() || l.len() > 2) {
        return Err(NOT_A_LOOP.into());
    }
    let ends: Vec<usize> = (0..ordered.len())
        .filter(|&i| links[i].len() == 1)
        .collect();
    let closed = ends.is_empty();
    if !closed && ends.len() != 2 {
        return Err(NOT_A_LOOP.into());
    }
    // Direction: from the first selected polygon towards its earliest
    // selected neighbour; an open strip is then walked from the end behind.
    let first = ordered[0];
    let ahead = links[0][0];
    let mut walk = vec![first, ahead];
    while walk.len() < ordered.len() {
        let here = *walk.last().unwrap();
        let back = walk[walk.len() - 2];
        let i = rank(here).unwrap();
        let Some(&next) = links[i].iter().find(|&&n| n != back) else {
            break;
        };
        if next == first {
            break;
        }
        walk.push(next);
    }
    if !closed {
        // Extend backwards from `first` to the other end.
        let mut behind = vec![];
        let mut here = first;
        let mut from = ahead;
        let mut taken: std::collections::HashSet<usize> = walk.iter().copied().collect();
        while let Some(&next) = links[rank(here).unwrap()].iter().find(|&&n| n != from) {
            if !taken.insert(next) {
                break;
            }
            behind.push(next);
            from = here;
            here = next;
        }
        behind.reverse();
        behind.extend(walk);
        walk = behind;
    }
    if walk.len() != ordered.len() {
        return Err(NOT_A_LOOP.into());
    }
    Ok((walk, closed))
}

/// Polygon Next (`next`) or Previous Loop.
pub(crate) fn polygons(mesh: &Mesh, ordered: &[usize], next: bool) -> Result<Vec<usize>> {
    let faces = mesh.point_faces();
    let (walk, closed) = polygon_walk(mesh, &faces, ordered)?;
    // For each polygon: the side it is left through (towards the next one
    // in the walk; the last of an open strip uses the side opposite its
    // entry).
    let mut exits = Vec::with_capacity(walk.len());
    for (i, &f) in walk.iter().enumerate() {
        if mesh.faces[f].vertices.len() != 4 {
            return Err("Next/Previous Loop works on quad loops; this loop has a non-quad".into());
        }
        let exit = if i + 1 < walk.len() || closed {
            shared_side(mesh, f, walk[(i + 1) % walk.len()])
        } else {
            shared_side(mesh, f, walk[i - 1]).map(|entry| (entry + 2) % 4)
        };
        exits.push(exit.ok_or("Invalid loop incidence")?);
    }
    let step = |right: bool| -> Vec<Option<usize>> {
        walk.iter()
            .zip(&exits)
            .map(|(&f, &exit)| {
                let side = sides(&mesh.faces[f].vertices)[(exit + if right { 3 } else { 1 }) % 4];
                across(mesh, &faces, f, side)
            })
            .collect()
    };
    let mut result = step(next);
    let mut bounced = false;
    if result.iter().all(Option::is_none) {
        result = step(!next);
        bounced = true;
    }
    if result.iter().any(Option::is_none) {
        return Err("Next/Previous Loop reached an open border partway along the loop".into());
    }
    let mut result: Vec<usize> = result.into_iter().flatten().collect();
    if result.iter().any(|&f| mesh.faces[f].vertices.len() != 4) {
        return Err(
            "Next/Previous Loop stops at a pole: the next loop has triangles or n-gons".into(),
        );
    }
    let mut unique = result.clone();
    unique.sort_unstable();
    unique.dedup();
    let on_walk: std::collections::HashSet<usize> = walk.iter().copied().collect();
    if unique.len() != result.len() || result.iter().any(|f| on_walk.contains(f)) {
        return Err("Next/Previous Loop can't find a separate parallel loop here".into());
    }
    if bounced {
        result.reverse();
    }
    Ok(result)
}

/// Where an edge Next/Previous Loop step lands.
pub(crate) enum EdgeStep {
    /// An inner edge: the new selection is its edge loop.
    Loop((u32, u32)),
    /// The open border: the whole border loop, in walk order.
    Border(Vec<(u32, u32)>),
}

/// Edge Next/Previous Loop from the first selected edge `seed` (stored low,
/// high).
pub(crate) fn edges(mesh: &Mesh, seed: (u32, u32)) -> Result<EdgeStep> {
    let faces = mesh.point_faces();
    let using: Vec<usize> = faces
        .of(seed.0)
        .iter()
        .map(|&f| f as usize)
        .filter(|&f| {
            sides(&mesh.faces[f].vertices)
                .iter()
                .any(|&s| same_edge(s, seed))
        })
        .collect();
    let face = match using.as_slice() {
        [only] => *only,
        [_, _] => *using
            .iter()
            .find(|&&f| sides(&mesh.faces[f].vertices).contains(&seed))
            .ok_or("Next/Previous Loop needs consistently wound polygons")?,
        _ => return Err("Next/Previous Loop needs edges with one or two polygons".into()),
    };
    let ring = sides(&mesh.faces[face].vertices);
    if ring.len() != 4 {
        return Err(
            "Next/Previous Loop stops at a pole: the polygon beside this loop isn't a quad".into(),
        );
    }
    let index = ring
        .iter()
        .position(|&s| same_edge(s, seed))
        .ok_or("Invalid loop incidence")?;
    let (a, b) = ring[(index + 2) % 4];
    let opposite = (a.min(b), a.max(b));
    let counts = mesh.edge_counts();
    if counts.get(&opposite) != Some(&1) {
        return Ok(EdgeStep::Loop(opposite));
    }
    // Walk the open border from the opposite edge, through each point's
    // other open edge, until it closes or ends.
    let open_at = |v: u32| -> Vec<(u32, u32)> {
        let mut found = vec![];
        for &f in faces.of(v) {
            for (p, q) in sides(&mesh.faces[f as usize].vertices) {
                let e = (p.min(q), p.max(q));
                if (p == v || q == v) && counts.get(&e) == Some(&1) && !found.contains(&e) {
                    found.push(e);
                }
            }
        }
        found
    };
    let mut border = vec![opposite];
    let mut on_border = std::collections::HashSet::from([opposite]);
    let mut point = opposite.1;
    loop {
        let here = *border.last().unwrap();
        let next = open_at(point).into_iter().find(|&e| e != here);
        let Some(next) = next else { break };
        if !on_border.insert(next) {
            break;
        }
        point = if next.0 == point { next.1 } else { next.0 };
        border.push(next);
    }
    Ok(EdgeStep::Border(border))
}
