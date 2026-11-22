//! Selection-only operations. Topological connectivity never uses spatial proximity.
use crate::selection_continuation as continuation;
use crate::{Mode, Result, Scene};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionOperation {
    CloseLoop,
    Grow,
    Shrink,
    Loop,
    Ring,
    All,
    Invert,
    Connected,
    InvertConnected,
    Convert(Mode),
    Boundary,
    /// MB-Q9: Boundary that adds the border edges to the edge selection
    /// already stored instead of replacing it.
    BoundaryAdd,
    More,
    Less,
    Between,
    /// Step the selected loop to the parallel loop beside it.
    NextLoop,
    PreviousLoop,
}
impl SelectionOperation {
    pub fn label(self) -> &'static str {
        match self {
            Self::CloseLoop => "Close Loop",
            Self::Grow => "Grow",
            Self::Shrink => "Shrink",
            Self::Loop => "Loop",
            Self::Ring => "Ring",
            Self::All => "Select All",
            Self::Invert => "Invert",
            Self::Connected => "Connected",
            Self::InvertConnected => "Invert Connected",
            Self::Convert(Mode::Vertex) => "Convert to Vertices",
            Self::Convert(Mode::Edge) => "Convert to Edges",
            Self::Convert(Mode::Polygon) => "Convert to Polygons",
            Self::Convert(Mode::Item) => "Convert to Items",
            Self::Boundary => "Boundary",
            Self::BoundaryAdd => "Add Boundary",
            Self::More => "Select More",
            Self::Less => "Select Less",
            Self::Between => "Select Between",
            Self::NextLoop => "Next Loop",
            Self::PreviousLoop => "Previous Loop",
        }
    }
}
impl Scene {
    /// Edge-mode double-click: select `edge` of `item` plus its edge loop.
    /// With `add` the loop joins the existing edge selection (kept first,
    /// in order); otherwise it replaces it. Where Loop can't walk from this
    /// edge, only the clicked edge is selected (or added) and this returns
    /// `Ok(false)`; `Ok(true)` means the loop reached further edges.
    pub fn select_edge_loop(&mut self, item: u64, edge: (u32, u32), add: bool) -> Result<bool> {
        // Another active mesh's edge: its loop goes to that mesh's
        // selection (a plain double-click clears every active mesh first).
        if self.is_other_active_mesh(item) {
            let mut probe = self.clone();
            if !add {
                probe.clear_active_components(Mode::Edge);
            }
            let grew = probe.as_sole_mesh(item, |s| s.select_edge_loop(item, edge, true))?;
            *self = probe;
            return Ok(grew);
        }
        let edge = (edge.0.min(edge.1), edge.0.max(edge.1));
        // Validates the edge and makes `item` active (clearing another
        // item's components) exactly as an ordinary click would.
        self.select_component(item, crate::Component::Edge(edge.0, edge.1), !add, false)?;
        let kept = std::mem::replace(&mut self.selection.edges, vec![edge]);
        if self.selection_operation(SelectionOperation::Loop).is_err() {
            self.selection.edges = kept;
            return Ok(false);
        }
        let walked = std::mem::take(&mut self.selection.edges);
        let grew = walked.iter().any(|&e| e != edge);
        let mut seen: std::collections::HashSet<(u32, u32)> = kept.iter().copied().collect();
        let mut merged = kept;
        merged.extend(walked.into_iter().filter(|e| seen.insert(*e)));
        self.selection.edges = merged;
        Ok(grew)
    }
    /// Polygon-mode double-click: select `face` of `item` plus every polygon
    /// connected to it (the whole piece, as `select.connect` walks it). With
    /// `add` the piece joins the existing polygon selection (kept first, in
    /// order); otherwise it replaces it. Other selected pieces are never
    /// grown, so a Shift double-click adds only the clicked piece. Returns
    /// how many polygons the piece has.
    pub fn select_connected_polygons(&mut self, item: u64, face: u32, add: bool) -> Result<usize> {
        // Another active mesh's polygon: its piece goes to that mesh's
        // selection (a plain double-click clears every active mesh first).
        if self.is_other_active_mesh(item) {
            let mut probe = self.clone();
            if !add {
                probe.clear_active_components(Mode::Polygon);
            }
            let count =
                probe.as_sole_mesh(item, |s| s.select_connected_polygons(item, face, true))?;
            *self = probe;
            return Ok(count);
        }
        if self.selection.mode != Mode::Polygon {
            return Err("Select Connected from a double-click works in Polygons mode".into());
        }
        // Validates the polygon and makes `item` active (clearing another
        // item's components) exactly as an ordinary click would.
        self.select_component(item, crate::Component::Polygon(face), !add, false)?;
        let kept = std::mem::replace(&mut self.selection.faces, vec![face]);
        if let Err(e) = self.selection_operation(SelectionOperation::Connected) {
            self.selection.faces = kept;
            return Err(e);
        }
        let piece = std::mem::take(&mut self.selection.faces);
        let count = piece.len();
        let mut seen: std::collections::HashSet<u32> = kept.iter().copied().collect();
        let mut merged = kept;
        merged.extend(piece.into_iter().filter(|f| seen.insert(*f)));
        self.selection.faces = merged;
        Ok(count)
    }
    pub fn selection_operation(&mut self, operation: SelectionOperation) -> Result<()> {
        self.validate()?;
        let item = self.active().ok_or("Select a visible mesh")?;
        if !self.shows(item) {
            return Err("Show the selected mesh before selecting components".into());
        }
        let mesh = &item.mesh;
        if mesh.has_polylines()
            && !matches!(
                operation,
                SelectionOperation::All | SelectionOperation::Invert
            )
        {
            return Err("This selection command doesn't work on polylines yet; click components or use Select All or Invert".into());
        }
        let mode = self.selection.mode;
        if mode == Mode::Item {
            return Err("This selection command requires a component context".into());
        }
        let edges = mesh.edges();
        let selected: BTreeSet<usize> = match mode {
            Mode::Vertex => self
                .selection
                .vertices
                .iter()
                .map(|&v| v as usize)
                .collect(),
            Mode::Edge => self
                .selection
                .edges
                .iter()
                .filter_map(|e| edges.binary_search(e).ok())
                .collect(),
            Mode::Polygon => self.selection.faces.iter().map(|&f| f as usize).collect(),
            Mode::Item => unreachable!(),
        };
        let ordered: Vec<usize> = match mode {
            Mode::Vertex => self
                .selection
                .vertices
                .iter()
                .map(|&v| v as usize)
                .collect(),
            Mode::Edge => self
                .selection
                .edges
                .iter()
                .filter_map(|e| edges.binary_search(e).ok())
                .collect(),
            Mode::Polygon => self.selection.faces.iter().map(|&f| f as usize).collect(),
            Mode::Item => unreachable!(),
        };
        let members: Vec<Vec<u32>> = match mode {
            Mode::Vertex => (0..mesh.vertices.len() as u32).map(|v| vec![v]).collect(),
            Mode::Edge => edges.iter().map(|&(a, b)| vec![a, b]).collect(),
            Mode::Polygon => mesh.faces.iter().map(|f| f.vertices.clone()).collect(),
            Mode::Item => unreachable!(),
        };
        let (target, result): (Mode, Vec<usize>) = match operation {
            SelectionOperation::More => (mode, continuation::more(mesh, mode, &ordered)?),
            SelectionOperation::Less => (mode, continuation::less(&ordered)?),
            SelectionOperation::Between => (mode, continuation::between(mesh, mode, &ordered)?),
            SelectionOperation::NextLoop | SelectionOperation::PreviousLoop => {
                use crate::selection_neighbour_loop as neighbour;
                let next = operation == SelectionOperation::NextLoop;
                match mode {
                    _ if ordered.is_empty() => {
                        return Err("Select a loop first (Loop), then Next or Previous Loop".into());
                    }
                    Mode::Polygon => (mode, neighbour::polygons(mesh, &ordered, next)?),
                    Mode::Edge => match neighbour::edges(mesh, edges[ordered[0]])? {
                        neighbour::EdgeStep::Border(border) => (
                            mode,
                            border
                                .iter()
                                .filter_map(|e| edges.binary_search(e).ok())
                                .collect(),
                        ),
                        neighbour::EdgeStep::Loop(seed) => {
                            let mut probe = self.clone();
                            probe.selection.edges = vec![seed];
                            probe.selection_operation(SelectionOperation::Loop)?;
                            let result = probe
                                .selection
                                .edges
                                .iter()
                                .filter_map(|e| edges.binary_search(e).ok())
                                .collect();
                            (mode, result)
                        }
                    },
                    _ => (mode, self.next_point_loop(mesh, next)?),
                }
            }
            // PF-05: three or more polygons along a loop complete its ring.
            SelectionOperation::CloseLoop
                if mode == Mode::Polygon && self.selection.faces.len() > 2 =>
            {
                (mode, self.closed_polygon_ring(&edges)?)
            }
            SelectionOperation::CloseLoop => {
                if mode != Mode::Polygon || self.selection.faces.len() != 2 {
                    return Err("Close Loop requires two adjacent polygon direction seeds".into());
                }
                let mut probe = self.clone();
                probe.selection_operation(SelectionOperation::Loop)?;
                // Flood the polygons off the band through shared edges,
                // once each: linear in the mesh size.
                let incidence = LoopIncidence::new(mesh, &edges);
                let mut visited = vec![false; mesh.faces.len()];
                for &f in &probe.selection.faces {
                    visited[f as usize] = true;
                }
                let mut regions = Vec::new();
                for seed in 0..mesh.faces.len() {
                    if visited[seed] {
                        continue;
                    }
                    visited[seed] = true;
                    let mut region = Vec::new();
                    let mut pending = vec![seed];
                    while let Some(face) = pending.pop() {
                        region.push(face);
                        for &edge in &incidence.faces[face] {
                            for &other in &incidence.edge_faces[edge] {
                                if !visited[other] {
                                    visited[other] = true;
                                    pending.push(other);
                                }
                            }
                        }
                    }
                    regions.push(region);
                    if regions.len() > 2 {
                        break;
                    }
                }
                if regions.len() != 2 || regions[0].len() == regions[1].len() {
                    // PF-05: a loop that closes on itself is the answer;
                    // otherwise, Spec: SEL-U06.
                    let ring = self.closed_polygon_ring(&edges).map_err(|_| {
                        "Close Loop needs the loop to split the mesh into two regions of different size"
                            .to_string()
                    })?;
                    (mode, ring)
                } else {
                    let side = if regions[0].len() < regions[1].len() {
                        0
                    } else {
                        1
                    };
                    let mut result: Vec<_> =
                        probe.selection.faces.iter().map(|&f| f as usize).collect();
                    result.extend(regions[side].iter().copied());
                    (mode, result)
                }
            }
            SelectionOperation::Grow | SelectionOperation::Shrink => {
                // Any polygon mesh: adjacency is topological and needs no
                // quads (SEL-054/055). Only the selection's neighbourhood is
                // read, through the cached point → polygons map.
                if selected.is_empty() {
                    // Spec: SEL-U06
                    return Err("Select something before Grow or Shrink".into());
                }
                let point_faces = mesh.point_faces();
                let neighbours =
                    |i: usize| neighbours(mesh, &point_faces, &edges, mode, &members[i], i);
                if operation == SelectionOperation::Grow {
                    let mut grown = selected.clone();
                    for &index in &selected {
                        grown.extend(neighbours(index));
                    }
                    (mode, grown.into_iter().collect())
                } else {
                    if selected.len() == members.len() && mesh.is_closed() {
                        return Err(
                            // Spec: SEL-U06
                            "Can't shrink a selection that covers a whole closed mesh".into(),
                        );
                    }
                    let counts = mesh.edge_counts();
                    let open = |a: u32, b: u32| counts.get(&(a.min(b), a.max(b))) == Some(&1);
                    let kept = selected
                        .iter()
                        .copied()
                        .filter(|&i| {
                            let at_boundary = match mode {
                                Mode::Vertex => {
                                    let v = members[i][0];
                                    point_faces.of(v).iter().any(|&f| {
                                        face_edges(&mesh.faces[f as usize].vertices)
                                            .iter()
                                            .any(|&(a, b)| (a == v || b == v) && open(a, b))
                                    })
                                }
                                Mode::Edge => open(members[i][0], members[i][1]),
                                Mode::Polygon => {
                                    face_edges(&members[i]).iter().any(|&(a, b)| open(a, b))
                                }
                                Mode::Item => false,
                            };
                            // PF-02: the mesh's open border isn't
                            // "unselected": points and polygons shrink only
                            // from unselected neighbours (Edge mode keeps
                            // its border rule; no case measured).
                            let border_counts = mode == Mode::Edge && at_boundary;
                            !border_counts && neighbours(i).iter().all(|j| selected.contains(j))
                        })
                        .collect();
                    (mode, kept)
                }
            }
            SelectionOperation::Loop | SelectionOperation::Ring => {
                // Incidence is built once (linear in the mesh), so each walk
                // step only reads its own neighbourhood: no size limit.
                let incidence = LoopIncidence::new(mesh, &edges);
                let faces = &incidence.faces;
                let incident = |edge: usize| -> &[usize] { &incidence.edge_faces[edge] };
                if operation == SelectionOperation::Ring {
                    if mode != Mode::Edge || selected.is_empty() {
                        // Spec: SEL-U05
                        return Err("Ring needs selected edges".into());
                    }
                    if selected.len() > 2 {
                        return Err(
                            "Ring supports one seed or two seeds in the same regular quad strip"
                                .into(),
                        );
                    }
                    let first = *selected.first().ok_or("Select an edge")?;
                    let mut reached = BTreeSet::from([first]);
                    let mut pending = vec![first];
                    while let Some(edge) = pending.pop() {
                        // A ring stops where it reaches a non-quad, or an
                        // edge shared by more than two polygons.
                        if incident(edge).len() > 2 {
                            continue;
                        }
                        for face in incident(edge)
                            .iter()
                            .copied()
                            .filter(|&f| faces[f].len() == 4)
                        {
                            let index = faces[face]
                                .iter()
                                .position(|&e| e == edge)
                                .ok_or("Invalid ring incidence")?;
                            let next = faces[face][(index + 2) % 4];
                            if reached.insert(next) {
                                pending.push(next);
                            }
                        }
                    }
                    if !selected.is_subset(&reached) {
                        // Spec: SEL-U05
                        return Err(
                            "The selected edges aren't in one quad strip; nothing changed".into(),
                        );
                    }
                    (mode, reached.into_iter().collect())
                } else if mode == Mode::Polygon && self.selection.faces.len() == 1 {
                    // SEL-LP-01: one polygon gives the loop across its first
                    // and third edges (the column through polygon 5 of a grid).
                    let first = self.selection.faces[0] as usize;
                    if faces[first].len() != 4 {
                        return Err("Polygon Loop from one polygon needs a quad".into());
                    }
                    let mut reached = selected.clone();
                    for start_exit in [faces[first][0], faces[first][2]] {
                        let mut face = first;
                        let mut exit = start_exit;
                        while let Some(next) = incidence.across(exit, face) {
                            if !reached.insert(next) {
                                break;
                            }
                            let Some(index) = faces[next].iter().position(|&e| e == exit) else {
                                break;
                            };
                            if faces[next].len() != 4 {
                                break;
                            }
                            face = next;
                            exit = faces[next][(index + 2) % 4];
                        }
                    }
                    (mode, reached.into_iter().collect())
                } else if mode == Mode::Polygon {
                    if self.selection.faces.len() != 2 {
                        return Err(
                            "Polygon Loop needs one polygon, or two adjacent ordered seeds".into(),
                        );
                    }
                    let first = self.selection.faces[0] as usize;
                    let second = self.selection.faces[1] as usize;
                    let shared: Vec<_> = faces[first]
                        .iter()
                        .filter(|e| faces[second].contains(e))
                        .copied()
                        .collect();
                    if shared.len() != 1 {
                        return Err(
                            "Polygon Loop requires one shared edge between its two seeds".into(),
                        );
                    }
                    let mut reached = selected.clone();
                    for start in [first, second] {
                        let mut face = start;
                        let mut entered = shared[0];
                        loop {
                            // A polygon loop stops at a non-quad (it stays
                            // selected, as in the one-seed walk).
                            if faces[face].len() != 4 {
                                break;
                            }
                            let index = faces[face]
                                .iter()
                                .position(|&e| e == entered)
                                .ok_or("Invalid loop incidence")?;
                            let exit = faces[face][(index + 2) % 4];
                            let Some(next) = incidence.across(exit, face) else {
                                break;
                            };
                            if !reached.insert(next) {
                                break;
                            }
                            face = next;
                            entered = exit;
                        }
                    }
                    let mut result = vec![first, second];
                    result.extend(reached.into_iter().filter(|f| *f != first && *f != second));
                    (mode, result)
                } else if mode == Mode::Vertex {
                    if self.selection.vertices.len() != 2 {
                        return Err(
                            "Vertex Loop requires exactly two consecutive ordered vertices".into(),
                        );
                    }
                    let a = self.selection.vertices[0];
                    let b = self.selection.vertices[1];
                    if !edges.contains(&(a.min(b), a.max(b))) {
                        return Err("Vertex Loop seeds must be endpoints of one edge".into());
                    }
                    let mut probe = self.clone();
                    probe.selection.mode = Mode::Edge;
                    probe.selection.edges = vec![(a.min(b), a.max(b))];
                    probe.selection_operation(SelectionOperation::Loop)?;
                    let mut vertices = BTreeSet::new();
                    for (a, b) in probe.selection.edges {
                        vertices.extend([a, b]);
                    }
                    let mut result = vec![a as usize, b as usize];
                    result.extend(
                        vertices
                            .into_iter()
                            .filter(|&v| v != a && v != b)
                            .map(|v| v as usize),
                    );
                    (mode, result)
                } else if mode == Mode::Edge {
                    if selected.is_empty() {
                        // Spec: SEL-U05
                        return Err("Edge Loop needs a selected edge".into());
                    }
                    // Walk one seed's loop. `None` means the walk met a
                    // point where the direction is ambiguous.
                    // LOOP-NGON-01 (provisional, to be confirmed): where
                    // the straight walk can't go on at a point and the edge
                    // borders one n-gon (more than four sides, such as a
                    // cylinder cap), the loop follows that n-gon's perimeter
                    // through the point, and from then on only goes on
                    // around the same n-gon. Triangles are left out for now
                    // so triangle fans keep their pole behaviour (the
                    // existing fan test, SEL-058).
                    let perimeter_step = |edge: usize, vertex: u32, around: Option<usize>| {
                        // Only edges with a polygon on each side: open
                        // borders keep their own rule.
                        if incident(edge).len() != 2 {
                            return None;
                        }
                        let polygon = match around {
                            Some(p) => p,
                            None => {
                                let mut others = incident(edge)
                                    .iter()
                                    .copied()
                                    .filter(|&f| faces[f].len() > 4);
                                match (others.next(), others.next()) {
                                    (Some(p), None) => p,
                                    // None, or n-gons on both sides: no
                                    // single perimeter to follow.
                                    _ => return None,
                                }
                            }
                        };
                        if !incident(edge).contains(&polygon) {
                            return None;
                        }
                        let mut next = faces[polygon].iter().copied().filter(|&e| {
                            e != edge && (edges[e].0 == vertex || edges[e].1 == vertex)
                        });
                        match (next.next(), next.next()) {
                            (Some(e), None) => Some((e, polygon)),
                            _ => None,
                        }
                    };
                    let walk = |seed: usize| -> Option<BTreeSet<usize>> {
                        let mut reached = BTreeSet::from([seed]);
                        // Each edge to walk on from, with the polygon whose
                        // perimeter brought the walk there (LOOP-NGON-01).
                        let mut pending: Vec<(usize, Option<usize>)> = vec![(seed, None)];
                        while let Some((edge, around)) = pending.pop() {
                            for vertex in [edges[edge].0, edges[edge].1] {
                                if around.is_some() {
                                    // Already going around an n-gon: keep
                                    // to its perimeter (LOOP-NGON-01).
                                    if let Some((next, polygon)) =
                                        perimeter_step(edge, vertex, around)
                                        && reached.insert(next)
                                    {
                                        pending.push((next, Some(polygon)));
                                    }
                                    continue;
                                }
                                let at_vertex = &incidence.vertex_edges[vertex as usize];
                                if !incidence.fan_connected(vertex) {
                                    // Polygons around this point don't join
                                    // up (a non-manifold point): the loop
                                    // stops here.
                                    continue;
                                }
                                if at_vertex.len() != 4
                                    && !at_vertex.iter().any(|&e| incident(e).len() == 1)
                                {
                                    // A pole or other non-four-edge point ends
                                    // the loop on this side (SEL-058: stop at
                                    // nonstandard junctions), unless the edge
                                    // runs around a non-quad polygon
                                    // (LOOP-NGON-01).
                                    if let Some((next, polygon)) =
                                        perimeter_step(edge, vertex, around)
                                        && reached.insert(next)
                                    {
                                        pending.push((next, Some(polygon)));
                                    }
                                    continue;
                                }
                                let candidates: Vec<usize> = at_vertex
                                    .iter()
                                    .copied()
                                    .filter(|&i| {
                                        i != edge
                                            && !incident(edge)
                                                .iter()
                                                .any(|&f| faces[f].contains(&i))
                                    })
                                    .collect();
                                // N02 (provisional): a border edge that
                                // reaches a corner of a hole (two spokes
                                // and the next border edge) follows the
                                // border, as the grid's outer border does
                                // in SEL-LP-03, instead of giving up.
                                let candidates =
                                    if candidates.len() > 1 && incident(edge).len() == 1 {
                                        let border: Vec<usize> = candidates
                                            .iter()
                                            .copied()
                                            .filter(|&i| incident(i).len() == 1)
                                            .collect();
                                        if border.len() == 1 {
                                            border
                                        } else {
                                            candidates
                                        }
                                    } else {
                                        candidates
                                    };
                                // PF-03: a border edge at a corner of the
                                // open border (only two border edges meet
                                // there) turns the corner and goes on
                                // along the border.
                                let candidates =
                                    if candidates.is_empty() && incident(edge).len() == 1 {
                                        let border: Vec<usize> = at_vertex
                                            .iter()
                                            .copied()
                                            .filter(|&i| i != edge && incident(i).len() == 1)
                                            .collect();
                                        if border.len() == 1 {
                                            border
                                        } else {
                                            candidates
                                        }
                                    } else {
                                        candidates
                                    };
                                if candidates.len() > 1 {
                                    return None;
                                }
                                if let Some(&next) = candidates.first() {
                                    if reached.insert(next) {
                                        pending.push((next, None));
                                    }
                                } else if let Some((next, polygon)) =
                                    perimeter_step(edge, vertex, around)
                                    && reached.insert(next)
                                {
                                    // Nowhere straight to go: around a
                                    // non-quad polygon (LOOP-NGON-01).
                                    pending.push((next, Some(polygon)));
                                }
                            }
                        }
                        Some(reached)
                    };
                    if selected.len() == 1 {
                        let Some(reached) = walk(ordered[0]) else {
                            // Spec: SEL-U05
                            return Err("Loop reached a point where the direction is ambiguous; nothing changed".into());
                        };
                        if reached == selected {
                            // Spec: SEL-U05
                            return Err("Edge Loop reached a point where it can't continue straight; nothing changed".into());
                        }
                        (mode, reached.into_iter().collect())
                    } else {
                        // Several seeds (Jason, 2026-11-06): the loop of each
                        // seed not already reached, joined. The selection
                        // keeps its order with the new edges after it; a
                        // seed whose walk stops at once, or meets an
                        // ambiguous point, just stays selected.
                        let mut result: Vec<usize> = Vec::with_capacity(ordered.len());
                        let mut reached = vec![false; edges.len()];
                        for &edge in &ordered {
                            if !reached[edge] {
                                reached[edge] = true;
                                result.push(edge);
                            }
                        }
                        let kept = result.len();
                        // Edges some seed's walk already covered: a seed on
                        // an earlier loop adds nothing new.
                        let mut walked_over = vec![false; edges.len()];
                        for &seed in &ordered {
                            if walked_over[seed] {
                                continue;
                            }
                            let Some(walked) = walk(seed) else {
                                continue;
                            };
                            for edge in walked {
                                walked_over[edge] = true;
                                if !reached[edge] {
                                    reached[edge] = true;
                                    result.push(edge);
                                }
                            }
                        }
                        if result.len() == kept {
                            // Spec: SEL-U05
                            return Err("Edge Loop reached a point where it can't continue straight; nothing changed".into());
                        }
                        (mode, result)
                    }
                } else {
                    return Err("Vertex Loop directional continuation is not yet implemented; use edge or polygon seeds".into());
                }
            }
            SelectionOperation::All => (mode, (0..members.len()).collect()),
            SelectionOperation::Invert => (
                mode,
                (0..members.len())
                    .filter(|i| !selected.contains(i))
                    .collect(),
            ),
            SelectionOperation::Connected | SelectionOperation::InvertConnected => {
                // Traverse actual vertex/edge incidence. Coincident disconnected points stay separate.
                let mut adjacency = vec![Vec::new(); mesh.vertices.len()];
                for &(a, b) in &edges {
                    adjacency[a as usize].push(b);
                    adjacency[b as usize].push(a);
                }
                let mut reached = BTreeSet::new();
                let mut pending: Vec<u32> = selected
                    .iter()
                    .flat_map(|&i| members[i].iter().copied())
                    .collect();
                while let Some(v) = pending.pop() {
                    if reached.insert(v) {
                        pending.extend(adjacency[v as usize].iter().copied());
                    }
                }
                let result = members
                    .iter()
                    .enumerate()
                    .filter_map(|(i, vs)| {
                        (vs.iter().any(|v| reached.contains(v))
                            && (operation != SelectionOperation::InvertConnected
                                || !selected.contains(&i)))
                        .then_some(i)
                    })
                    .collect();
                (mode, result)
            }
            SelectionOperation::Convert(target) => {
                if target == Mode::Item {
                    return Err(
                        "Component-to-item conversion is outside this bounded command".into(),
                    );
                }
                let vertices: BTreeSet<u32> = selected
                    .iter()
                    .flat_map(|&i| members[i].iter().copied())
                    .collect();
                let selected_edges: BTreeSet<_> = self.selection.edges.iter().copied().collect();
                let result = match target {
                    Mode::Vertex => vertices.iter().map(|&v| v as usize).collect(),
                    Mode::Edge => edges
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &(a, b))| {
                            let include = if mode == Mode::Polygon {
                                selected
                                    .iter()
                                    .any(|&f| face_edges(&mesh.faces[f].vertices).contains(&(a, b)))
                            } else if mode == Mode::Edge {
                                selected.contains(&i)
                            } else {
                                vertices.contains(&a) && vertices.contains(&b)
                            };
                            include.then_some(i)
                        })
                        .collect(),
                    Mode::Polygon => mesh
                        .faces
                        .iter()
                        .enumerate()
                        .filter_map(|(i, f)| {
                            let include = if mode == Mode::Edge {
                                face_edges(&f.vertices)
                                    .iter()
                                    .all(|e| selected_edges.contains(e))
                            } else if mode == Mode::Polygon {
                                selected.contains(&i)
                            } else {
                                f.vertices.iter().all(|v| vertices.contains(v))
                            };
                            include.then_some(i)
                        })
                        .collect(),
                    Mode::Item => unreachable!(),
                };
                (target, result)
            }
            SelectionOperation::Boundary | SelectionOperation::BoundaryAdd => {
                // PF-04: in Edge mode with nothing selected, every open
                // border edge of the mesh (as with no polygons selected).
                let whole_mesh = mode == Mode::Edge && selected.is_empty();
                if mode != Mode::Polygon && !whole_mesh {
                    return Err(
                        "Boundary requires Polygon context; it is separate from Convert to Edges"
                            .into(),
                    );
                }
                let mut incidence = vec![0usize; edges.len()];
                for (i, face) in mesh.faces.iter().enumerate() {
                    if selected.is_empty() || selected.contains(&i) {
                        for edge in face_edges(&face.vertices) {
                            if let Ok(index) = edges.binary_search(&edge) {
                                incidence[index] += 1;
                            }
                        }
                    }
                }
                (
                    Mode::Edge,
                    incidence
                        .iter()
                        .enumerate()
                        .filter_map(|(i, n)| (n % 2 == 1).then_some(i))
                        .collect(),
                )
            }
        };
        self.selection.mode = target;
        if operation == SelectionOperation::BoundaryAdd {
            // Keep the stored edges in their order; append new border edges.
            let mut kept: BTreeSet<(u32, u32)> = self.selection.edges.iter().copied().collect();
            for i in result {
                if kept.insert(edges[i]) {
                    self.selection.edges.push(edges[i]);
                }
            }
            return Ok(());
        }
        match target {
            Mode::Vertex => {
                self.selection.vertices = result.into_iter().map(|v| v as u32).collect()
            }
            Mode::Edge => self.selection.edges = result.into_iter().map(|i| edges[i]).collect(),
            Mode::Polygon => self.selection.faces = result.into_iter().map(|f| f as u32).collect(),
            Mode::Item => unreachable!(),
        }
        Ok(())
    }
}
impl Scene {
    /// PF-05: the polygon loop through the first two selected polygons,
    /// when it closes on itself and holds every selected polygon.
    fn closed_polygon_ring(&self, edges: &[(u32, u32)]) -> Result<Vec<usize>> {
        let faces = &self.selection.faces;
        if faces.len() < 2 {
            return Err("Close Loop needs polygons along a loop".into());
        }
        let mut probe = self.clone();
        probe.selection.faces = faces[..2].to_vec();
        probe.selection_operation(SelectionOperation::Loop)?;
        let ring: BTreeSet<u32> = probe.selection.faces.iter().copied().collect();
        if !faces.iter().all(|f| ring.contains(f)) {
            return Err("The selected polygons aren't on one loop".into());
        }
        let mesh = &self.active().ok_or("Select a mesh")?.mesh;
        // Closed: every polygon of the loop meets two others of it.
        let mut uses = vec![0u8; edges.len()];
        for &f in &ring {
            for e in face_edges(&mesh.faces[f as usize].vertices) {
                if let Ok(i) = edges.binary_search(&e) {
                    uses[i] = uses[i].saturating_add(1);
                }
            }
        }
        let closed = ring.iter().all(|&f| {
            face_edges(&mesh.faces[f as usize].vertices)
                .iter()
                .filter(|e| edges.binary_search(e).is_ok_and(|i| uses[i] > 1))
                .count()
                >= 2
        });
        if !closed {
            return Err("The loop doesn't close on itself".into());
        }
        let mut out: Vec<usize> = faces.iter().map(|&f| f as usize).collect();
        out.extend(
            ring.iter()
                .map(|&f| f as usize)
                .filter(|f| !faces.contains(&(*f as u32))),
        );
        Ok(out)
    }

    /// PF-06: Next / Previous Loop on a point loop. The selected points run
    /// along an edge loop; the next loop is the edge loop beside it (Next
    /// on the side of the polygon wound from the second point back to the
    /// first, Previous on the other), returned as its points. At the open
    /// border that is the whole border loop (PF-03).
    fn next_point_loop(&self, mesh: &crate::Mesh, next: bool) -> Result<Vec<usize>> {
        let points = &self.selection.vertices;
        let a = *points.first().ok_or("Select a loop of points first")?;
        let adjacent = |x: u32, y: u32| {
            mesh.faces.iter().any(|f| {
                let n = f.vertices.len();
                (0..n).any(|k| {
                    let (p, q) = (f.vertices[k], f.vertices[(k + 1) % n]);
                    (p, q) == (x, y) || (p, q) == (y, x)
                })
            })
        };
        let b = *points
            .iter()
            .skip(1)
            .find(|&&p| adjacent(a, p))
            .ok_or("Next/Previous Loop needs points along an edge loop")?;
        // The quad on the chosen side: it has the edge wound b→a for Next,
        // a→b for Previous.
        let (from, to) = if next { (b, a) } else { (a, b) };
        let face = mesh
            .faces
            .iter()
            .find(|f| {
                let n = f.vertices.len();
                (0..n).any(|k| (f.vertices[k], f.vertices[(k + 1) % n]) == (from, to))
            })
            .ok_or("There is no loop on that side of these points")?;
        if face.vertices.len() != 4 {
            return Err(
                "Next/Previous Loop stops at a pole: the polygon beside this loop isn't a quad"
                    .into(),
            );
        }
        let k = face
            .vertices
            .iter()
            .position(|&v| v == from)
            .ok_or("Invalid loop incidence")?;
        let (c, d) = (face.vertices[(k + 2) % 4], face.vertices[(k + 3) % 4]);
        let mut probe = self.clone();
        probe.selection.mode = Mode::Edge;
        probe.selection.edges = vec![(c.min(d), c.max(d))];
        probe.selection_operation(SelectionOperation::Loop)?;
        let mut out = BTreeSet::new();
        for (p, q) in &probe.selection.edges {
            out.insert(*p as usize);
            out.insert(*q as usize);
        }
        Ok(out.into_iter().collect())
    }

    /// Polygon Selection Fill seeded by the double-clicked polygon.
    pub fn fill_polygon_selection(&mut self, item: u64, face: u32) -> Result<()> {
        if self.is_other_active_mesh(item) {
            return self.as_sole_mesh(item, |s| s.fill_polygon_selection(item, face));
        }
        self.validate()?;
        if self.selection.mode != Mode::Polygon {
            return Err("Selection Fill works in Polygons mode".into());
        }
        let active = self.active().ok_or("Select a visible mesh")?;
        if active.id != item || !self.shows(active) {
            return Err("Selection Fill works on the active visible mesh".into());
        }
        let ordered: Vec<usize> = self.selection.faces.iter().map(|&f| f as usize).collect();
        let result = continuation::fill(&active.mesh, &ordered, face as usize)?;
        self.selection.faces = result.into_iter().map(|f| f as u32).collect();
        Ok(())
    }
}
fn face_edges(vertices: &[u32]) -> Vec<(u32, u32)> {
    vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .map(|(&a, &b)| (a.min(b), a.max(b)))
        .collect()
}

/// Components directly adjacent to component `index` (`member` is its point
/// list): vertices joined by an edge, edges sharing a point, polygons sharing
/// an edge. Reads only the polygons around `member`'s points.
fn neighbours(
    mesh: &crate::Mesh,
    point_faces: &crate::topology_cache::PointFaces,
    edges: &[(u32, u32)],
    mode: Mode,
    member: &[u32],
    index: usize,
) -> BTreeSet<usize> {
    let mut found = BTreeSet::new();
    let around = member
        .iter()
        .flat_map(|&v| point_faces.of(v).iter().map(|&f| f as usize));
    match mode {
        Mode::Vertex => {
            let v = member[0];
            for f in around {
                for (a, b) in face_edges(&mesh.faces[f].vertices) {
                    if a == v {
                        found.insert(b as usize);
                    } else if b == v {
                        found.insert(a as usize);
                    }
                }
            }
        }
        Mode::Edge => {
            for f in around {
                for edge in face_edges(&mesh.faces[f].vertices) {
                    if (member.contains(&edge.0) || member.contains(&edge.1))
                        && let Ok(e) = edges.binary_search(&edge)
                    {
                        found.insert(e);
                    }
                }
            }
        }
        // PF-01: every polygon sharing a point, diagonals included.
        Mode::Polygon => {
            found.extend(around);
        }
        Mode::Item => {}
    }
    found.remove(&index);
    found
}

/// Edge and polygon incidence for the loop walks (Loop, Ring, Close Loop),
/// built once per command in time linear in the mesh. Every walk step then
/// reads only the components it touches, so dense imported meshes loop as
/// quickly as small ones.
struct LoopIncidence {
    /// Each polygon's edges (indices into the sorted edge list), in winding order.
    faces: Vec<Vec<usize>>,
    /// The polygons using each edge.
    edge_faces: Vec<Vec<usize>>,
    /// The edges at each point.
    vertex_edges: Vec<Vec<usize>>,
}
impl LoopIncidence {
    fn new(mesh: &crate::Mesh, edges: &[(u32, u32)]) -> Self {
        let mut edge_faces = vec![Vec::new(); edges.len()];
        let mut vertex_edges = vec![Vec::new(); mesh.vertices.len()];
        for (i, &(a, b)) in edges.iter().enumerate() {
            vertex_edges[a as usize].push(i);
            vertex_edges[b as usize].push(i);
        }
        let faces = mesh
            .faces
            .iter()
            .enumerate()
            .map(|(f, face)| {
                face_edges(&face.vertices)
                    .iter()
                    .map(|edge| {
                        let e = edges.binary_search(edge).expect("validated edge");
                        edge_faces[e].push(f);
                        e
                    })
                    .collect()
            })
            .collect();
        Self {
            faces,
            edge_faces,
            vertex_edges,
        }
    }

    /// The polygon on the other side of `edge` from `face`, when exactly two
    /// polygons share it.
    fn across(&self, edge: usize, face: usize) -> Option<usize> {
        match self.edge_faces[edge].as_slice() {
            [a, b] if *a == face => Some(*b),
            [a, b] if *b == face => Some(*a),
            _ => None,
        }
    }

    /// Whether the polygons around `vertex` form one fan joined through
    /// their edges at the point (false for a non-manifold "bow tie" point).
    fn fan_connected(&self, vertex: u32) -> bool {
        let at = &self.vertex_edges[vertex as usize];
        let fan: Vec<usize> = {
            let mut fan: Vec<usize> = at
                .iter()
                .flat_map(|&e| self.edge_faces[e].iter().copied())
                .collect();
            fan.sort_unstable();
            fan.dedup();
            fan
        };
        let Some(&first) = fan.first() else {
            return true;
        };
        let mut reached = vec![first];
        let mut pending = vec![first];
        while let Some(face) = pending.pop() {
            for &edge in at {
                if !self.edge_faces[edge].contains(&face) {
                    continue;
                }
                for &other in &self.edge_faces[edge] {
                    if !reached.contains(&other) {
                        reached.push(other);
                        pending.push(other);
                    }
                }
            }
        }
        reached.len() == fan.len()
    }
}
