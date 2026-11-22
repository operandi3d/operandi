//! Select More/Less, Select Between and Polygon Selection Fill. All traversal is topological: loops continue
//! straight through four-edge vertices, rings and polygon loops cross quads to
//! the opposite edge, and nothing is decided by spatial proximity.
use crate::{Mesh, Mode, Result};
use std::collections::{BTreeSet, HashMap, VecDeque};

/// Edge/face incidence built once per command; O(components) to construct.
struct Topology {
    edges: Vec<(u32, u32)>,
    face_edges: Vec<Vec<usize>>,
    edge_faces: Vec<Vec<usize>>,
    vertex_edges: Vec<Vec<usize>>,
}

/// One way of walking away from a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    /// Polygon loop leaving through this edge.
    Across(usize),
    /// Edge loop continuing through this vertex.
    LoopAt(u32),
    /// Edge ring continuing into this quad.
    RingIn(usize),
    /// Vertex loop starting along this edge.
    Along(usize),
}

impl Topology {
    fn new(mesh: &Mesh) -> Self {
        let edges = mesh.edges();
        let index: HashMap<(u32, u32), usize> =
            edges.iter().enumerate().map(|(i, &e)| (e, i)).collect();
        let mut edge_faces = vec![Vec::new(); edges.len()];
        let mut vertex_edges = vec![Vec::new(); mesh.vertices.len()];
        for (i, &(a, b)) in edges.iter().enumerate() {
            vertex_edges[a as usize].push(i);
            vertex_edges[b as usize].push(i);
        }
        let face_edges = mesh
            .faces
            .iter()
            .enumerate()
            .map(|(f, face)| {
                let n = face.vertices.len();
                (0..n)
                    .map(|k| {
                        let (a, b) = (face.vertices[k], face.vertices[(k + 1) % n]);
                        let e = index[&(a.min(b), a.max(b))];
                        edge_faces[e].push(f);
                        e
                    })
                    .collect()
            })
            .collect();
        Self {
            edges,
            face_edges,
            edge_faces,
            vertex_edges,
        }
    }

    fn other_end(&self, edge: usize, vertex: u32) -> u32 {
        let (a, b) = self.edges[edge];
        if a == vertex { b } else { a }
    }

    /// The edge across a quad from `edge`, or None for non-quads.
    fn opposite_in(&self, face: usize, edge: usize) -> Option<usize> {
        let es = &self.face_edges[face];
        if es.len() != 4 {
            return None;
        }
        es.iter().position(|&e| e == edge).map(|i| es[(i + 2) % 4])
    }

    fn other_face(&self, edge: usize, face: usize) -> Option<usize> {
        match self.edge_faces[edge].as_slice() {
            [a, b] if *a == face => Some(*b),
            [a, b] if *b == face => Some(*a),
            _ => None,
        }
    }

    /// Straight continuation of an edge loop at `vertex`: the single edge there
    /// that shares no polygon with `edge`. Poles and ambiguous fans stop the loop.
    fn continue_loop(&self, edge: usize, vertex: u32) -> Option<usize> {
        let mut candidates = self.vertex_edges[vertex as usize].iter().filter(|&&e| {
            e != edge
                && !self.edge_faces[e]
                    .iter()
                    .any(|f| self.edge_faces[edge].contains(f))
        });
        let first = candidates.next().copied();
        if candidates.next().is_some() {
            None
        } else {
            first
        }
    }

    fn directions(&self, mode: Mode, element: usize) -> Vec<Direction> {
        match mode {
            Mode::Polygon => self.face_edges[element]
                .iter()
                .map(|&e| Direction::Across(e))
                .collect(),
            Mode::Edge => {
                let (a, b) = self.edges[element];
                let mut d = vec![Direction::LoopAt(a), Direction::LoopAt(b)];
                d.extend(
                    self.edge_faces[element]
                        .iter()
                        .map(|&f| Direction::RingIn(f)),
                );
                d
            }
            Mode::Vertex => self.vertex_edges[element]
                .iter()
                .map(|&e| Direction::Along(e))
                .collect(),
            Mode::Item => Vec::new(),
        }
    }

    /// Elements reached walking from `start` in `direction`, excluding `start`,
    /// stopping at an open end, a non-quad/pole, a revisit or `limit` steps.
    fn walk(&self, mode: Mode, start: usize, direction: Direction, limit: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::from([start]);
        match (mode, direction) {
            (Mode::Polygon, Direction::Across(mut exit)) => {
                let mut face = start;
                while out.len() < limit {
                    let Some(next) = self.other_face(exit, face) else {
                        break;
                    };
                    if !seen.insert(next) {
                        break;
                    }
                    out.push(next);
                    let Some(opposite) = self.opposite_in(next, exit) else {
                        break;
                    };
                    face = next;
                    exit = opposite;
                }
            }
            (Mode::Edge, Direction::LoopAt(mut vertex)) => {
                let mut edge = start;
                while out.len() < limit {
                    let Some(next) = self.continue_loop(edge, vertex) else {
                        break;
                    };
                    if !seen.insert(next) {
                        break;
                    }
                    out.push(next);
                    vertex = self.other_end(next, vertex);
                    edge = next;
                }
            }
            (Mode::Edge, Direction::RingIn(mut face)) => {
                let mut edge = start;
                while out.len() < limit {
                    let Some(next) = self.opposite_in(face, edge) else {
                        break;
                    };
                    if !seen.insert(next) {
                        break;
                    }
                    out.push(next);
                    let Some(across) = self.other_face(next, face) else {
                        break;
                    };
                    face = across;
                    edge = next;
                }
            }
            (Mode::Vertex, Direction::Along(mut edge)) => {
                let mut vertex = start as u32;
                while out.len() < limit {
                    let next = self.other_end(edge, vertex);
                    if !seen.insert(next as usize) {
                        break;
                    }
                    out.push(next as usize);
                    let Some(following) = self.continue_loop(edge, next) else {
                        break;
                    };
                    vertex = next;
                    edge = following;
                }
            }
            _ => {}
        }
        out
    }

    /// Neighbors for Select Between: one step along any loop or ring.
    fn neighbors(&self, mode: Mode, element: usize) -> Vec<usize> {
        self.directions(mode, element)
            .into_iter()
            .filter_map(|d| self.walk(mode, element, d, 1).first().copied())
            .collect()
    }

    /// One shortest route from `from` to `to`, both included. Breadth-first
    /// from `to`, neighbours in their natural order (a polygon's edges in
    /// winding order, a point's edges by index), each element keeping the
    /// first neighbour that reached it; the route follows those links back
    /// from `from`. This reproduces every measured tie (SEL-BT-05/06); the
    /// underlying rule is inferred (SEL-Q-B2).
    fn route(&self, mode: Mode, from: usize, to: usize, count: usize) -> Vec<usize> {
        let mut parent: Vec<Option<usize>> = vec![None; count];
        let mut seen = vec![false; count];
        seen[to] = true;
        let mut queue = VecDeque::from([to]);
        while let Some(at) = queue.pop_front() {
            if at == from {
                break;
            }
            for next in self.neighbors(mode, at) {
                if !seen[next] {
                    seen[next] = true;
                    parent[next] = Some(at);
                    queue.push_back(next);
                }
            }
        }
        let mut out = vec![from];
        let mut at = from;
        while let Some(next) = parent[at] {
            out.push(next);
            at = next;
        }
        out
    }

    fn distances(&self, mode: Mode, start: usize, count: usize) -> Vec<Option<usize>> {
        let mut distance = vec![None; count];
        distance[start] = Some(0);
        let mut queue = VecDeque::from([start]);
        while let Some(at) = queue.pop_front() {
            let d = distance[at].expect("queued elements have a distance");
            for next in self.neighbors(mode, at) {
                if distance[next].is_none() {
                    distance[next] = Some(d + 1);
                    queue.push_back(next);
                }
            }
        }
        distance
    }
}

/// Select More: append the element one stride beyond the last pick, where the
/// stride is the loop distance between the last two picks.
pub(crate) fn more(mesh: &Mesh, mode: Mode, ordered: &[usize]) -> Result<Vec<usize>> {
    let [.., previous, last] = ordered else {
        return Err("Select More needs at least two picks to set a direction".into());
    };
    let topology = Topology::new(mesh);
    let limit = component_count(mesh, &topology, mode);
    let mut found: Option<(usize, Direction)> = None;
    let mut tied = false;
    for direction in topology.directions(mode, *previous) {
        let path = topology.walk(mode, *previous, direction, limit);
        if let Some(index) = path.iter().position(|e| e == last) {
            let stride = index + 1;
            match found {
                Some((best, _)) if best < stride => {}
                Some((best, _)) if best == stride => tied = true,
                _ => {
                    found = Some((stride, direction));
                    tied = false;
                }
            }
        }
    }
    let Some((stride, direction)) = found else {
        return Err("Select More: the last two picks are not on one loop or ring".into());
    };
    if tied {
        return Err("Select More: the last two picks are equally far apart both ways around; pick a nearer pair".into());
    }
    let path = topology.walk(mode, *previous, direction, 2 * stride);
    let next = *path
        .get(2 * stride - 1)
        .ok_or("Select More: the loop ends before the next element")?;
    if ordered.contains(&next) {
        return Err("Select More: the next element is already selected".into());
    }
    let mut result = ordered.to_vec();
    result.push(next);
    Ok(result)
}

/// Select Less: drop the most recent pick.
pub(crate) fn less(ordered: &[usize]) -> Result<Vec<usize>> {
    match ordered.split_last() {
        Some((_, rest)) => Ok(rest.to_vec()),
        None => Err("Select Less: nothing is selected".into()),
    }
}

/// Select Between (SEL-BT-01..06). Only the last two picks are connected;
/// earlier picks stay (SEL-BT-03). When the two lie on one loop or ring, one
/// shortest route is added in order from the first to the second, ties
/// resolved like the reference (SEL-BT-05, see `route`). Polygons or edges
/// on no shared loop get the whole rectangle of shortest routes (SEL-BT-02);
/// points on no shared loop add nothing (SEL-BT-04).
pub(crate) fn between(mesh: &Mesh, mode: Mode, ordered: &[usize]) -> Result<Vec<usize>> {
    if mode == Mode::Item {
        return Err("Select Between works on points, edges or polygons".into());
    }
    let &[.., first, second] = ordered else {
        return Err("Select Between needs two picks".into());
    };
    let topology = Topology::new(mesh);
    let count = component_count(mesh, &topology, mode);
    let shared_loop = topology
        .directions(mode, first)
        .into_iter()
        .any(|d| topology.walk(mode, first, d, count).contains(&second));
    let mut result = ordered[..ordered.len() - 2].to_vec();
    let mut added: std::collections::HashSet<usize> = result.iter().copied().collect();
    let mut add = |result: &mut Vec<usize>, e: usize| {
        if added.insert(e) {
            result.push(e);
        }
    };
    if shared_loop {
        for e in topology.route(mode, first, second, count) {
            add(&mut result, e);
        }
        return Ok(result);
    }
    if mode == Mode::Vertex {
        return Ok(ordered.to_vec());
    }
    let from_first = topology.distances(mode, first, count);
    let from_second = topology.distances(mode, second, count);
    let total = from_first[second]
        .ok_or("Select Between: the two picks are not connected by loops or rings")?;
    let mut rectangle: Vec<(usize, usize)> = (0..count)
        .filter_map(|i| match (from_first[i], from_second[i]) {
            (Some(a), Some(b)) if a + b == total => Some((a, i)),
            _ => None,
        })
        .collect();
    rectangle.sort_unstable();
    if mode == Mode::Polygon
        && let Some(lines) = rectangle_in_lines(&topology, first, &rectangle)
    {
        rectangle = lines;
    }
    // Picks first, then the added elements.
    result = ordered.to_vec();
    let mut added: std::collections::HashSet<usize> = result.iter().copied().collect();
    for (_, e) in rectangle {
        if added.insert(e) {
            result.push(e);
        }
    }
    Ok(result)
}

/// The polygon rectangle of Select Between in the reference's order
/// (SEL-B2-02, SEL-B2-04): line by line, starting from the first pick.
/// Of the two sides of the first pick that lead into the rectangle, the
/// later one in its corner list sets the direction of each line (the
/// same "later side edge" rule as the ring ties, SEL-B2-03); the other
/// side steps from line to line. So picks 0 then 10 on Grid4 fill columns
/// (0, 4, 8, then 1, 5, 9, then 2, 6, 10) and 10 then 0 fills rows. Each
/// element is placed by its distance, inside the rectangle, from the first
/// line and from the first pick's cross line. `None` (keep the distance
/// order) when the first pick doesn't have exactly two such sides.
fn rectangle_in_lines(
    topology: &Topology,
    first: usize,
    rectangle: &[(usize, usize)],
) -> Option<Vec<(usize, usize)>> {
    let inside: BTreeSet<usize> = rectangle.iter().map(|&(_, e)| e).collect();
    let sides: Vec<usize> = topology.face_edges[first]
        .iter()
        .copied()
        .filter(|&e| {
            topology
                .other_face(e, first)
                .is_some_and(|f| inside.contains(&f))
        })
        .collect();
    let &[across_lines, along_line] = sides.as_slice() else {
        return None;
    };
    // The first pick's row along each side, while it stays inside.
    let chain = |exit: usize| -> Vec<usize> {
        let walked = topology.walk(Mode::Polygon, first, Direction::Across(exit), inside.len());
        std::iter::once(first)
            .chain(walked.into_iter().take_while(|f| inside.contains(f)))
            .collect()
    };
    // Breadth-first distance inside the rectangle from a set of polygons.
    let distance_from = |sources: &[usize]| -> HashMap<usize, usize> {
        let mut distance: HashMap<usize, usize> = sources.iter().map(|&f| (f, 0)).collect();
        let mut queue: VecDeque<usize> = sources.iter().copied().collect();
        while let Some(at) = queue.pop_front() {
            let d = distance[&at];
            for next in topology.neighbors(Mode::Polygon, at) {
                if inside.contains(&next) && !distance.contains_key(&next) {
                    distance.insert(next, d + 1);
                    queue.push_back(next);
                }
            }
        }
        distance
    };
    let line = distance_from(&chain(along_line));
    let place = distance_from(&chain(across_lines));
    let mut ordered = Vec::with_capacity(rectangle.len());
    for &(_, e) in rectangle {
        ordered.push(((*line.get(&e)?, *place.get(&e)?), e));
    }
    ordered.sort_unstable();
    Some(ordered.into_iter().map(|(_, e)| (0, e)).collect())
}

/// Polygon Selection Fill: add the unselected region containing `seed` when the
/// selected polygons fully enclose it, keeping the perimeter.
pub(crate) fn fill(mesh: &Mesh, ordered: &[usize], seed: usize) -> Result<Vec<usize>> {
    if seed >= mesh.faces.len() {
        return Err("Selection Fill: the clicked polygon does not exist".into());
    }
    if ordered.is_empty() {
        return Err("Selection Fill: select a closed perimeter of polygons first".into());
    }
    if ordered.contains(&seed) {
        return Err("Selection Fill: double-click inside the selected perimeter, not on it".into());
    }
    let topology = Topology::new(mesh);
    let selected: BTreeSet<usize> = ordered.iter().copied().collect();
    let flood = |blocked: &BTreeSet<usize>| {
        let mut reached = vec![seed];
        let mut seen = BTreeSet::from([seed]);
        let mut at = 0;
        while let Some(&face) = reached.get(at) {
            at += 1;
            for &edge in &topology.face_edges[face] {
                for &next in &topology.edge_faces[edge] {
                    if !blocked.contains(&next) && seen.insert(next) {
                        reached.push(next);
                    }
                }
            }
        }
        reached
    };
    let on_border = |faces: &[usize]| {
        faces.iter().any(|&f| {
            topology.face_edges[f]
                .iter()
                .any(|&e| topology.edge_faces[e].len() == 1)
        })
    };
    let region = flood(&selected);
    let piece = flood(&BTreeSet::new());
    // Open piece: the area must not leak to the mesh border. Closed piece: the
    // selection must split the unselected polygons, or nothing is enclosed.
    let enclosed = if on_border(&piece) {
        !on_border(&region)
    } else {
        region.len() < piece.iter().filter(|f| !selected.contains(f)).count()
    };
    if !enclosed {
        return Err("Selection Fill: the selected polygons do not enclose this area".into());
    }
    let mut result = ordered.to_vec();
    result.extend(region);
    Ok(result)
}

fn component_count(mesh: &Mesh, topology: &Topology, mode: Mode) -> usize {
    match mode {
        Mode::Vertex => mesh.vertices.len(),
        Mode::Edge => topology.edges.len(),
        Mode::Polygon => mesh.faces.len(),
        Mode::Item => 0,
    }
}
