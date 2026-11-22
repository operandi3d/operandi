//! Milestone 2 Edge command rows (ME-C01..C13 one-shot commands).
//!
//! Each command acts on the selected edges of the active mesh, in a basic
//! Operandi profile:
//!
//! * **Edge Subdivide** — a point at the middle of every selected edge; every
//!   polygon using the edge takes it.
//! * **Collapse** — each connected group of selected edges merges into one
//!   point at its average; the mesh heals (no hole).
//! * **Remove** — selected interior edges are dissolved: their two polygons
//!   merge. The points stay; forward Delete (`edge_delete_geometry`) also
//!   takes out the points left between two interior edges or unused.
//! * **Spin** — each selected interior edge turns one step inside the two
//!   polygons it separates; they keep their sizes.
//! * **Split** — polygons are separated along the selected edges (points are
//!   duplicated so each side has its own).
//! * **Join** / **Join Averaged** — two selected edges weld: the first moves
//!   onto the second, or both meet at their average.
//! * **Grow Quads** — a row of quads grows from a selected border, matching
//!   the size of the row it grows from (Auto Offset).
//! * **Fill Quads** — a closed border loop with an even number of edges is
//!   filled with the fewest quads.
//! * **Edges To Curves** — each selected chain becomes a Polyline.
//! * **Unbevel** — the strip between two selected edge loops collapses into a
//!   sharp edge where its neighbours' sides meet (Convergence mode).
//! * **Split Normals** — the polygons on either side of each selected edge get
//!   hard (face) normals at its points in the existing vertex normal map
//!   (Level 100%).
//!
//! Corner data (UVs, corner normals, weights, colours, typed corner records)
//! follows each output corner from the source corner it came from. A new
//! corner without a source corner is refused while a map holds assigned
//! values there would be nothing to interpolate from yet; nothing changes.
use crate::topology::edge_key;
use crate::{Face, Mesh, PolygonRepresentation, Result};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};

/// The one-shot Edge commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeCommand {
    Subdivide,
    Collapse,
    Remove,
    Spin,
    Split,
    Join,
    JoinAveraged,
    GrowQuads,
    FillQuads,
    ToCurves,
    Unbevel,
    SplitNormals,
}
impl EdgeCommand {
    pub const ALL: [EdgeCommand; 12] = [
        Self::GrowQuads,
        Self::FillQuads,
        Self::Split,
        Self::SplitNormals,
        Self::Spin,
        Self::Join,
        Self::JoinAveraged,
        Self::Collapse,
        Self::Remove,
        Self::ToCurves,
        Self::Unbevel,
        Self::Subdivide,
    ];
    /// Panel and history label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Subdivide => "Edge Subdivide",
            Self::Collapse => "Collapse",
            Self::Remove => "Remove",
            Self::Spin => "Spin",
            Self::Split => "Split",
            Self::Join => "Join",
            Self::JoinAveraged => "Join Averaged",
            Self::GrowQuads => "Grow Quads",
            Self::FillQuads => "Fill Quads",
            Self::ToCurves => "Edges To Curves",
            Self::Unbevel => "Unbevel",
            Self::SplitNormals => "Split Normals",
        }
    }
    /// Command-line invocation: the verb, plus `average:true` for Join
    /// Averaged (names follow the name map, command_names.rs).
    pub fn verb(self) -> &'static str {
        match self {
            Self::Subdivide => "edge.subdivide",
            Self::Collapse => "edge.collapse",
            Self::Remove => "edge.remove",
            Self::Spin => "edge.spinQuads",
            Self::Split => "edge.split",
            Self::Join => "edge.join",
            Self::JoinAveraged => "edge.join average:true",
            Self::GrowQuads => "edge.growQuads",
            Self::FillQuads => "edge.fillQuads",
            Self::ToCurves => "edges.toCurve",
            Self::Unbevel => "edge.unbevel",
            Self::SplitNormals => "edge.splitNormals",
        }
    }
}

fn unit(v: DVec3, what: &str) -> Result<DVec3> {
    if !v.is_finite() || v.length() < 1e-12 {
        Err(format!("Degenerate {what}"))
    } else {
        Ok(v.normalize())
    }
}

/// Run `command` on `edges` (ordered as selected). Returns the new mesh and
/// the edges to select afterwards.
pub fn edge_command_geometry(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    command: EdgeCommand,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let ordered = checked_edges(mesh, edges, command.label())?;
    match command {
        EdgeCommand::Subdivide => subdivide(mesh, &ordered),
        EdgeCommand::Collapse => collapse(mesh, &ordered),
        EdgeCommand::Remove => remove(mesh, &ordered, false),
        EdgeCommand::Spin => spin(mesh, &ordered),
        EdgeCommand::Split => split(mesh, &ordered),
        EdgeCommand::Join => join(mesh, &ordered, false),
        EdgeCommand::JoinAveraged => join(mesh, &ordered, true),
        EdgeCommand::GrowQuads => grow_quads(mesh, &ordered),
        EdgeCommand::FillQuads => fill_quads(mesh, &ordered),
        EdgeCommand::ToCurves => to_curves(mesh, &ordered),
        EdgeCommand::Unbevel => unbevel(mesh, &ordered),
        EdgeCommand::SplitNormals => split_normals(mesh, &ordered),
    }
}

/// Forward Delete on edges (Jason, 2026-11-10): Remove, then the points the
/// removed edges leave behind go too (see `leftover_points`), so deleting
/// an edge loop leaves clean polygons. Backspace runs plain Remove, which
/// keeps the points (CMD-E-01).
///
/// No orphans either (Jason, 2026-11-15): every point the delete leaves
/// with nothing attached goes too, such as the far corners of a lone quad
/// whose edge took the quad with it. Points that were already unused stay.
/// A selected span of a polyline (a stray edge with no polygons) is cut
/// out of its line (`cut_line_spans`, provisional ED-Q01).
pub fn edge_delete_geometry(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<Mesh> {
    let ordered = checked_edges(mesh, edges, "Delete")?;
    let uses = edge_uses(mesh);
    let (surface, lines): (Vec<_>, Vec<_>) = ordered
        .into_iter()
        .partition(|&(a, b)| uses.contains_key(&edge_key(a, b)));
    let mut out = if lines.is_empty() {
        mesh.clone()
    } else {
        cut_line_spans(mesh, &lines)?
    };
    if !surface.is_empty() {
        out = remove(&out, &surface, true)?.0;
    }
    let was_used: BTreeSet<u32> = mesh
        .used_points()
        .iter()
        .enumerate()
        .filter_map(|(i, &used)| used.then_some(i as u32))
        .collect();
    let cleaned = crate::mesh_cleanup::remove_unreferenced(&out, Some(&was_used))?;
    finish(cleaned.mesh, "Delete")
}

/// Delete on polyline spans: each selected span leaves its line. An open
/// line splits there, a closed one opens there; a piece left with fewer
/// than two points goes. Points keep their indices.
fn cut_line_spans(mesh: &Mesh, spans: &[(u32, u32)]) -> Result<Mesh> {
    let cut: BTreeSet<(u32, u32)> = spans.iter().map(|&(a, b)| edge_key(a, b)).collect();
    let mut faces = Vec::new();
    let mut opened = Vec::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        let spans: Vec<(u32, u32)> = mesh.element_spans(fi).collect();
        let hit: Vec<bool> = spans
            .iter()
            .map(|&(a, b)| mesh.is_polyline(fi) && cut.contains(&edge_key(a, b)))
            .collect();
        if !hit.contains(&true) {
            faces.push(OutFace::copy(mesh, fi, f.vertices.clone()));
            continue;
        }
        let n = f.vertices.len();
        let closed = spans.len() == n;
        // Walk corners from just after a cut span (closed) or from the
        // start (open); each cut span ends one piece and starts the next.
        let start = if closed {
            (hit.iter().position(|&h| h).unwrap_or(0) + 1) % n
        } else {
            0
        };
        let mut piece: Vec<usize> = Vec::new();
        let mut pieces = Vec::new();
        for k in 0..n {
            let c = (start + k) % n;
            piece.push(c);
            let span_cut = hit.get(c).copied().unwrap_or(true);
            if span_cut || k + 1 == n {
                pieces.push(std::mem::take(&mut piece));
            }
        }
        for corners in pieces.into_iter().filter(|p| p.len() >= 2) {
            if closed {
                opened.push(faces.len());
            }
            faces.push(OutFace {
                template: fi,
                vertices: corners.iter().map(|&c| f.vertices[c]).collect(),
                donors: corners.iter().map(|&c| Some((fi, c))).collect(),
                blends: Vec::new(),
            });
        }
    }
    let mut out = rebuild(mesh, mesh.vertices.clone(), faces, &[], "Delete")?;
    for f in opened {
        if let Some(PolygonRepresentation::Polyline { closed, .. }) =
            out.attributes.representations.get_mut(f)
        {
            *closed = false;
        }
    }
    finish(out, "Delete")
}

/// The selected edges, validated and de-duplicated in selection order.
fn checked_edges(mesh: &Mesh, edges: &[(u32, u32)], what: &str) -> Result<Vec<(u32, u32)>> {
    mesh.validate()?;
    if !mesh.attributes.extrude_edge_uses.is_empty() {
        return Err(format!("{what} needs consistent winding; use Align first"));
    }
    let existing: BTreeSet<(u32, u32)> = mesh.edges().into_iter().collect();
    let mut ordered = Vec::new();
    for &(a, b) in edges {
        let k = edge_key(a, b);
        if !existing.contains(&k) {
            return Err("A selected edge no longer exists".into());
        }
        if !ordered.iter().any(|&(x, y)| edge_key(x, y) == k) {
            ordered.push((a, b));
        }
    }
    if ordered.is_empty() {
        return Err(format!("Select edges for {what}"));
    }
    Ok(ordered)
}

/// Surface polygons using each undirected edge, with their direction.
/// Directed uses `(polygon, from, to)` of each undirected edge.
type EdgeUses = BTreeMap<(u32, u32), Vec<(usize, u32, u32)>>;

fn edge_uses(mesh: &Mesh) -> EdgeUses {
    let mut uses: EdgeUses = BTreeMap::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(fi) {
            continue;
        }
        let n = f.vertices.len();
        for i in 0..n {
            let (x, y) = (f.vertices[i], f.vertices[(i + 1) % n]);
            uses.entry(edge_key(x, y)).or_default().push((fi, x, y));
        }
    }
    uses
}

/// One output polygon: the source polygon it takes its tags from, and per
/// corner the source corner `(face, corner)` its corner data comes from.
/// A corner with no donor may instead name a `blend` of source corners for
/// its UVs; `blends` is either empty or one entry per corner.
pub(crate) struct OutFace {
    pub(crate) template: usize,
    pub(crate) vertices: Vec<u32>,
    pub(crate) donors: Vec<Option<(usize, usize)>>,
    pub(crate) blends: Vec<Option<CornerBlend>>,
}

/// Weighted source corners `((face, corner), weight)`; weights sum to 1.
pub(crate) type CornerBlend = Vec<((usize, usize), f64)>;

/// Provisional UV rule for new corners (Jason, 2026-11-01): within one
/// source polygon, blend the UVs of the corners the new point was cut
/// between. Staying inside one polygon keeps UV seams intact. Absent if any
/// blended source corner has no UV.
pub(crate) fn blend_uv(rows: &[Vec<Option<DVec2>>], blend: &CornerBlend) -> Option<DVec2> {
    blend.iter().try_fold(DVec2::ZERO, |sum, &((f, c), w)| {
        Some(sum + *rows.get(f)?.get(c)?.as_ref()? * w)
    })
}

/// UV rows for `faces`: donors copy, blends interpolate, other new corners
/// are refused when the map has any UVs (as `remap_rows`).
fn remap_uv_rows(
    rows: &[Vec<Option<DVec2>>],
    faces: &[OutFace],
    what: &str,
) -> Result<Vec<Vec<Option<DVec2>>>> {
    let assigned = rows.iter().flatten().any(Option::is_some);
    let mut out = Vec::with_capacity(faces.len());
    for f in faces {
        let mut row = Vec::with_capacity(f.donors.len());
        for (c, donor) in f.donors.iter().enumerate() {
            match (donor, f.blends.get(c).and_then(Option::as_ref)) {
                (Some((sf, sc)), _) => {
                    row.push(rows.get(*sf).and_then(|r| r.get(*sc)).cloned().flatten())
                }
                (None, Some(blend)) => row.push(blend_uv(rows, blend)),
                (None, None) if assigned => {
                    return Err(format!(
                        "{what} cannot yet interpolate UVs or corner maps for new points; nothing changed"
                    ));
                }
                (None, None) => row.push(None),
            }
        }
        out.push(row);
    }
    Ok(out)
}

/// UV record ids for `faces`: donors copy; a blended corner with a UV gets a
/// fresh id of its own (records only say which corners share a UV).
fn remap_record_rows(
    rows: &[Vec<Option<u32>>],
    uvs: &[Vec<Option<DVec2>>],
    faces: &[OutFace],
    what: &str,
) -> Result<Vec<Vec<Option<u32>>>> {
    let mut next = rows.iter().flatten().flatten().max().map_or(0, |m| m + 1);
    let assigned = rows.iter().flatten().any(Option::is_some);
    let mut out = Vec::with_capacity(faces.len());
    for (fi, f) in faces.iter().enumerate() {
        let mut row = Vec::with_capacity(f.donors.len());
        for (c, donor) in f.donors.iter().enumerate() {
            match (donor, f.blends.get(c).and_then(Option::as_ref)) {
                (Some((sf, sc)), _) => {
                    row.push(rows.get(*sf).and_then(|r| r.get(*sc)).cloned().flatten())
                }
                (None, Some(_)) => row.push(uvs[fi][c].map(|_| {
                    next += 1;
                    next - 1
                })),
                (None, None) if assigned => {
                    return Err(format!(
                        "{what} cannot yet interpolate UVs or corner maps for new points; nothing changed"
                    ));
                }
                (None, None) => row.push(None),
            }
        }
        out.push(row);
    }
    Ok(out)
}
impl OutFace {
    /// An unchanged copy of source polygon `f` with new point ids.
    pub(crate) fn copy(mesh: &Mesh, f: usize, vertices: Vec<u32>) -> Self {
        Self {
            template: f,
            donors: (0..mesh.faces[f].vertices.len())
                .map(|c| Some((f, c)))
                .collect(),
            blends: Vec::new(),
            vertices,
        }
    }
}

fn remap_rows<T: Clone>(
    rows: &[Vec<Option<T>>],
    faces: &[OutFace],
    what: &str,
) -> Result<Vec<Vec<Option<T>>>> {
    let assigned = rows.iter().flatten().any(Option::is_some);
    let mut out = Vec::with_capacity(faces.len());
    for f in faces {
        let mut row = Vec::with_capacity(f.donors.len());
        for donor in &f.donors {
            match donor {
                Some((sf, sc)) => {
                    row.push(rows.get(*sf).and_then(|r| r.get(*sc)).cloned().flatten())
                }
                None if assigned => {
                    return Err(format!(
                        "{what} cannot yet interpolate UVs or corner maps for new points; nothing changed"
                    ));
                }
                None => row.push(None),
            }
        }
        out.push(row);
    }
    Ok(out)
}

/// Build the output mesh from `vertices` and `faces`, carrying per-polygon
/// tags/attributes from templates and corner data from donors. `copies`
/// lists `(new point, source point)` duplicates whose point records follow.
pub(crate) fn rebuild(
    source: &Mesh,
    vertices: Vec<DVec3>,
    faces: Vec<OutFace>,
    copies: &[(u32, u32)],
    what: &str,
) -> Result<Mesh> {
    if source.faces.iter().any(|f| f.uv.is_some()) {
        return Err(format!(
            "{what} cannot yet remap legacy per-polygon UVs; nothing changed"
        ));
    }
    let mut out = source.clone();
    out.vertices = vertices;
    let at = &source.attributes;
    for (name, rows) in &source.uv_maps {
        out.uv_maps
            .insert(name.clone(), remap_uv_rows(rows, &faces, what)?);
    }
    let o = &mut out.attributes;
    for (name, rows) in &at.uv_records {
        let uvs = out
            .uv_maps
            .get(name)
            .ok_or("UV record map has no coordinate map")?;
        o.uv_records
            .insert(name.clone(), remap_record_rows(rows, uvs, &faces, what)?);
    }
    for (name, rows) in &at.corner_weights {
        o.corner_weights
            .insert(name.clone(), remap_rows(rows, &faces, what)?);
    }
    for (name, rows) in &at.corner_colors {
        o.corner_colors
            .insert(name.clone(), remap_rows(rows, &faces, what)?);
    }
    for (name, rows) in &at.corner_normals {
        o.corner_normals
            .insert(name.clone(), remap_rows(rows, &faces, what)?);
    }
    for (name, map) in &at.typed_maps {
        if !map.corners.is_empty() {
            let corners = remap_rows(&map.corners, &faces, what)?;
            o.typed_maps.get_mut(name).unwrap().corners = corners;
        }
    }
    if !at.smoothing_labels.is_empty() {
        o.smoothing_labels = faces
            .iter()
            .map(|f| at.smoothing_labels[f.template].clone())
            .collect();
    }
    if !at.representations.is_empty() {
        o.representations = faces
            .iter()
            .map(|f| at.representations[f.template])
            .collect();
    }
    let follow = |set: &BTreeSet<u32>| -> BTreeSet<u32> {
        faces
            .iter()
            .enumerate()
            .filter(|(_, f)| set.contains(&(f.template as u32)))
            .map(|(i, _)| i as u32)
            .collect()
    };
    for set in o.polygon_sets.values_mut() {
        *set = follow(set);
    }
    o.degenerate_faces.clear();
    for &(new, old) in copies {
        for set in o.vertex_sets.values_mut() {
            if set.contains(&old) {
                set.insert(new);
            }
        }
        for m in o.point_uvs.values_mut() {
            if let Some(&v) = m.get(&old) {
                m.insert(new, v);
            }
        }
        for m in o.relative_morphs.values_mut() {
            if let Some(&v) = m.get(&old) {
                m.insert(new, v);
            }
        }
        for m in o.point_weights.values_mut() {
            if let Some(&v) = m.get(&old) {
                m.insert(new, v);
            }
        }
        for m in o.point_colors.values_mut() {
            if let Some(&v) = m.get(&old) {
                m.insert(new, v);
            }
        }
        for m in o.typed_maps.values_mut() {
            if let Some(v) = m.points.get(&old).cloned() {
                m.points.insert(new, v);
            }
        }
    }
    out.faces = faces
        .into_iter()
        .map(|f| Face {
            vertices: f.vertices,
            ..source.faces[f.template].clone()
        })
        .collect();
    let edges: BTreeSet<(u32, u32)> = out.edges().into_iter().collect();
    out.attributes
        .creases
        .retain(|c| edges.contains(&edge_key(c.a, c.b)));
    Ok(out)
}

/// Merge each group of points into one point at its position, heal the
/// polygons (drop repeated corners and polygons left with < 3 corners) and
/// remove the merged-away points. Returns the mesh and the old→new point map.
pub(crate) fn weld(
    mesh: &Mesh,
    groups: &[(Vec<u32>, DVec3)],
    what: &str,
) -> Result<(Mesh, Vec<Option<u32>>)> {
    weld_with(mesh, groups, false, what)
}

/// `weld`, optionally keeping polygons squashed to two corners as two-point
/// lines (Keep 1-Vertex Polygons, VX-M05).
pub(crate) fn weld_with(
    mesh: &Mesh,
    groups: &[(Vec<u32>, DVec3)],
    keep_small: bool,
    what: &str,
) -> Result<(Mesh, Vec<Option<u32>>)> {
    weld_options(
        mesh,
        groups,
        |members| members.iter().min().copied(),
        keep_small,
        what,
    )
}

/// `weld`, with `keep` choosing which member of each group survives (its
/// identity and point attributes carry the merged point). Weld drags keep
/// the destination so the dragged point is the one that disappears.
pub(crate) fn weld_keeping(
    mesh: &Mesh,
    groups: &[(Vec<u32>, DVec3)],
    keep: impl Fn(&[u32]) -> Option<u32>,
    what: &str,
) -> Result<(Mesh, Vec<Option<u32>>)> {
    weld_options(mesh, groups, keep, false, what)
}

fn weld_options(
    mesh: &Mesh,
    groups: &[(Vec<u32>, DVec3)],
    keep: impl Fn(&[u32]) -> Option<u32>,
    keep_small: bool,
    what: &str,
) -> Result<(Mesh, Vec<Option<u32>>)> {
    let mut to: Vec<u32> = (0..mesh.vertices.len() as u32).collect();
    let mut vertices = mesh.vertices.clone();
    let mut gone = BTreeSet::new();
    for (members, position) in groups {
        let keep = keep(members).ok_or("Empty weld group")?;
        vertices[keep as usize] = *position;
        for &m in members {
            to[m as usize] = keep;
            if m != keep {
                gone.insert(m);
            }
        }
    }
    let mut faces = Vec::new();
    let mut squashed = Vec::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        let polyline = mesh.is_polyline(fi);
        let mut vs: Vec<u32> = Vec::new();
        let mut donors = Vec::new();
        for (c, &v) in f.vertices.iter().enumerate() {
            let w = to[v as usize];
            if vs.last() != Some(&w) {
                vs.push(w);
                donors.push(Some((fi, c)));
            }
        }
        while vs.len() > 1 && !polyline && vs.first() == vs.last() {
            vs.pop();
            donors.pop();
        }
        let distinct: BTreeSet<_> = vs.iter().collect();
        if distinct.len() != vs.len() {
            return Err(format!(
                "{what} would fold a polygon onto itself; nothing changed"
            ));
        }
        let small = !polyline && vs.len() < 3;
        // Operandi has no one-point polygons yet: a polygon squashed to one
        // corner goes even when kept (provisional, VXM-Q2).
        if vs.len() >= if polyline { 2 } else { 3 } || (keep_small && vs.len() == 2) {
            if small {
                squashed.push(faces.len());
            }
            faces.push(OutFace {
                template: fi,
                vertices: vs,
                donors,
                blends: Vec::new(),
            });
        }
    }
    let mut healed = rebuild(mesh, vertices, faces, &[], what)?;
    // A squashed polygon of two corners stays as a two-point line, as Make
    // Polygon makes them.
    if healed
        .faces
        .iter()
        .enumerate()
        .any(|(i, f)| squashed.contains(&i) && f.vertices.len() == 2)
    {
        let count = healed.faces.len();
        let line = crate::PolygonRepresentation::Polyline {
            closed: false,
            curve: crate::curves::CurveKind::Line,
        };
        let a = &mut *healed.attributes;
        if a.representations.is_empty() {
            a.representations = vec![crate::PolygonRepresentation::Face; count];
        }
        for &i in &squashed {
            if healed.faces[i].vertices.len() == 2 {
                a.representations[i] = line;
            }
        }
    }
    healed
        .validate()
        .map_err(|e| format!("{what} would leave an invalid mesh ({e}); nothing changed"))?;
    let cleaned = crate::mesh_cleanup::remove_unreferenced(&healed, Some(&gone))?;
    let mut remap = vec![None; mesh.vertices.len()];
    let mut new_of_old = vec![None; mesh.vertices.len()];
    for (new, &old) in cleaned.survivors.iter().enumerate() {
        new_of_old[old] = Some(new as u32);
    }
    for (old, slot) in remap.iter_mut().enumerate() {
        *slot = new_of_old[to[old] as usize];
    }
    Ok((cleaned.mesh, remap))
}

pub(crate) fn finish(out: Mesh, what: &str) -> Result<Mesh> {
    out.validate()
        .map_err(|e| format!("{what} produced an invalid mesh ({e}); nothing changed"))?;
    Ok(out)
}

fn subdivide(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<(Mesh, Vec<(u32, u32)>)> {
    edge_subdivide_geometry(mesh, edges, 2, None)
}

/// CMD-B3-01: Subdivide each edge into `count` equal intervals (at least two,
/// so Counts 1 and 2 both add the midpoint). With Count 1 a `percent` puts
/// the single point that far from the edge's lower-numbered end. The edge
/// pieces are returned.
pub fn edge_subdivide_geometry(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    count: u32,
    percent: Option<f64>,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    if !(1..=256).contains(&count) {
        return Err("Subdivide Count must be 1–256".into());
    }
    let cuts: Vec<f64> = match percent {
        Some(p) if count == 1 => {
            if !(p.is_finite() && p > 0. && p < 100.) {
                return Err("Subdivide Percent must be between 0 and 100".into());
            }
            vec![p / 100.]
        }
        _ => {
            let n = count.max(2);
            (1..n).map(|i| i as f64 / n as f64).collect()
        }
    };
    let mut out = mesh.clone();
    let mut pieces = Vec::new();
    // Appending keeps earlier point ids, so every source edge stays valid.
    for &(a, b) in edges {
        let (a, b) = (a.min(b), a.max(b));
        let mut from = a;
        let mut done = 0.;
        for &t in &cuts {
            // Split what is left of the edge, from `from` to `b`.
            let (next, mid) = crate::add_point(&out, from, b, (t - done) / (1. - done))?;
            out = next;
            pieces.push(edge_key(from, mid));
            from = mid;
            done = t;
        }
        pieces.push(edge_key(from, b));
    }
    Ok((out, pieces))
}

fn collapse(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<(Mesh, Vec<(u32, u32)>)> {
    // Connected selected edges collapse together.
    let mut parent: BTreeMap<u32, u32> = BTreeMap::new();
    fn root(p: &mut BTreeMap<u32, u32>, v: u32) -> u32 {
        let mut r = v;
        while let Some(&q) = p.get(&r) {
            if q == r {
                break;
            }
            r = q;
        }
        p.insert(v, r);
        r
    }
    for &(a, b) in edges {
        parent.entry(a).or_insert(a);
        parent.entry(b).or_insert(b);
        let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
        parent.insert(ra, rb);
    }
    let mut groups: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &v in parent.clone().keys() {
        let r = root(&mut parent, v);
        groups.entry(r).or_default().push(v);
    }
    let groups: Vec<(Vec<u32>, DVec3)> = groups
        .into_values()
        .map(|m| {
            let c = m.iter().map(|&v| mesh.vertices[v as usize]).sum::<DVec3>() / m.len() as f64;
            (m, c)
        })
        .collect();
    let (out, _) = weld(mesh, &groups, "Collapse")?;
    Ok((finish(out, "Collapse")?, Vec::new()))
}

fn join(mesh: &Mesh, edges: &[(u32, u32)], averaged: bool) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let what = join_label(averaged);
    let [first, second] = edges[..] else {
        return Err(format!("{what} needs exactly two selected edges"));
    };
    let (groups, (c, d)) = join_groups(mesh, first, second, averaged)?;
    let (out, remap) = weld(mesh, &groups, what)?;
    let edge = match (remap[c as usize], remap[d as usize]) {
        (Some(x), Some(y)) => vec![edge_key(x, y)],
        _ => Vec::new(),
    };
    Ok((finish(out, what)?, edge))
}

fn join_label(averaged: bool) -> &'static str {
    if averaged { "Join Averaged" } else { "Join" }
}

/// Points that weld into one, and where they go.
type WeldGroup = (Vec<u32>, DVec3);

/// The weld groups that join `first` onto `second` (each end onto the
/// nearer end of the other edge), and `second` as paired.
fn join_groups(
    mesh: &Mesh,
    (a, b): (u32, u32),
    (c, d): (u32, u32),
    averaged: bool,
) -> Result<(Vec<WeldGroup>, (u32, u32))> {
    let what = join_label(averaged);
    let p = |v: u32| mesh.vertices[v as usize];
    let ((a, b), (c, d)) =
        if p(a).distance(p(c)) + p(b).distance(p(d)) <= p(a).distance(p(d)) + p(b).distance(p(c)) {
            ((a, b), (c, d))
        } else {
            ((a, b), (d, c))
        };
    if [a, b].iter().any(|v| [c, d].contains(v)) && !(a == c || b == d) {
        return Err(format!("{what} needs two edges that can pair their ends"));
    }
    let target = |x: u32, y: u32| if averaged { (p(x) + p(y)) * 0.5 } else { p(y) };
    let mut groups = Vec::new();
    for (x, y) in [(a, c), (b, d)] {
        if x != y {
            groups.push((vec![x, y], target(x, y)));
        }
    }
    Ok((groups, (c, d)))
}

/// Join / Join Averaged with Symmetry on (Jason, 2026-11-15; provisional,
/// SYM-JOIN-Q1): symmetric picking selects a pair of edges on each side,
/// and each side's pair joins by itself, in one weld so a point the two
/// pairs share on the plane welds once. Two selected edges (one side, on
/// the centre line, or an edge and its mirror) join as without Symmetry.
/// Anything else refuses and says what it found.
pub fn edge_join_symmetric_geometry(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    symmetry: crate::Symmetry,
    averaged: bool,
) -> Result<Mesh> {
    let what = join_label(averaged);
    let command = if averaged {
        EdgeCommand::JoinAveraged
    } else {
        EdgeCommand::Join
    };
    let ordered = checked_edges(mesh, edges, what)?;
    if symmetry.axis.is_none() || ordered.len() == 2 {
        return Ok(edge_command_geometry(mesh, &ordered, command)?.0);
    }
    let sides = crate::symmetry_sides::by_side(mesh, symmetry, &ordered, |&(a, b)| vec![a, b]);
    let (positive, negative) = match (&sides.positive[..], &sides.negative[..]) {
        (&[p0, p1], &[n0, n1]) if sides.centre.is_empty() => ((p0, p1), (n0, n1)),
        _ => {
            return Err(format!(
                "{what} needs two selected edges, or two on each side with Symmetry on \
                 (selected: {} on +{axis}, {} on -{axis}, {} on the centre line)",
                sides.positive.len(),
                sides.negative.len(),
                sides.centre.len(),
                axis = symmetry.label(),
            ));
        }
    };
    let mut groups = join_groups(mesh, positive.0, positive.1, averaged)?.0;
    for group in join_groups(mesh, negative.0, negative.1, averaged)?.0 {
        // Pairs that meet on the plane name the same weld there.
        let same = |g: &WeldGroup| {
            let mut x = g.0.clone();
            let mut y = group.0.clone();
            x.sort_unstable();
            y.sort_unstable();
            x == y
        };
        if groups.iter().any(same) {
            continue;
        }
        if groups
            .iter()
            .any(|g| g.0.iter().any(|v| group.0.contains(v)))
        {
            return Err(format!("{what}: the two sides' pairs overlap"));
        }
        groups.push(group);
    }
    let (out, _) = weld(mesh, &groups, what)?;
    finish(out, what)
}

/// Remove; with `clean`, also the points the removed edges leave unneeded.
fn remove(mesh: &Mesh, edges: &[(u32, u32)], clean: bool) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let uses = edge_uses(mesh);
    let chosen: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    // CMD-E-02: removing border edges (one polygon each) removes their
    // points from every polygon, as Vertex Remove does. Mixed border and
    // interior selections are not measured and still refuse below.
    if !chosen.is_empty()
        && chosen
            .iter()
            .all(|e| uses.get(e).is_some_and(|u| u.len() == 1))
    {
        let points: Vec<u32> = chosen
            .iter()
            .flat_map(|&(a, b)| [a, b])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if clean {
            // Delete drops every orphan at the end (edge_delete_geometry).
            let set = points.into_iter().collect();
            let out = crate::vertex_commands::remove_from_polygons(mesh, &set)?;
            return Ok((finish(out, "Delete")?, Vec::new()));
        }
        let (out, _) = crate::vertex_commands::remove(mesh, &points)?;
        return Ok((out, Vec::new()));
    }
    // Polygons joined through the removed edges merge into one.
    let mut group: Vec<usize> = (0..mesh.faces.len()).collect();
    fn root(g: &mut [usize], mut i: usize) -> usize {
        while g[i] != i {
            g[i] = g[g[i]];
            i = g[i];
        }
        i
    }
    for e in &chosen {
        let [(f, _, _), (g, _, _)] = uses.get(e).map(|u| &u[..]).unwrap_or(&[])[..] else {
            return Err("Remove dissolves interior edges (two neighbouring polygons)".into());
        };
        if f == g {
            return Err("Remove cannot dissolve an edge inside one polygon".into());
        }
        let (rf, rg) = (root(&mut group, f), root(&mut group, g));
        group[rf] = rg;
    }
    let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for f in 0..mesh.faces.len() {
        let r = root(&mut group, f);
        members.entry(r).or_default().push(f);
    }
    let mut faces = Vec::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        let r = root(&mut group, fi);
        let fs = &members[&r];
        if fs.len() == 1 {
            faces.push(OutFace::copy(mesh, fi, f.vertices.clone()));
            continue;
        }
        if fs[0] != fi {
            continue;
        }
        // Boundary of the merged region: half-edges whose edge is not removed.
        let mut next: BTreeMap<u32, (u32, (usize, usize))> = BTreeMap::new();
        for &g in fs {
            let vs = &mesh.faces[g].vertices;
            for c in 0..vs.len() {
                let (x, y) = (vs[c], vs[(c + 1) % vs.len()]);
                if chosen.contains(&edge_key(x, y)) {
                    continue;
                }
                if next.insert(x, (y, (g, c))).is_some() {
                    return Err(
                        "Remove would leave a polygon touching itself; nothing changed".into(),
                    );
                }
            }
        }
        let start = *next
            .keys()
            .next()
            .ok_or("Remove would leave an empty polygon")?;
        let (mut cycle, mut donors) = (Vec::new(), Vec::new());
        let mut v = start;
        loop {
            let (w, donor) = next[&v];
            cycle.push(v);
            donors.push(Some(donor));
            v = w;
            if v == start || cycle.len() > next.len() {
                break;
            }
        }
        if cycle.len() != next.len() || cycle.len() < 3 {
            return Err("Remove would leave a polygon with a hole; nothing changed".into());
        }
        faces.push(OutFace {
            template: fi,
            vertices: cycle,
            donors,
            blends: Vec::new(),
        });
    }
    let out = finish(
        rebuild(mesh, mesh.vertices.clone(), faces, &[], "Remove")?,
        "Remove",
    )?;
    let leftover = if clean {
        leftover_points(&out, &chosen)
    } else {
        Vec::new()
    };
    if leftover.is_empty() {
        return Ok((out, Vec::new()));
    }
    let set = leftover.into_iter().collect();
    let out = crate::vertex_commands::remove_from_polygons(&out, &set)?;
    Ok((finish(out, "Delete")?, Vec::new()))
}

/// Points of the removed edges that nothing needs any more (Jason,
/// 2026-11-10, provisional ER-Q01): a point no polygon uses, or one left
/// between exactly two interior edges, as removing an edge loop leaves all
/// along it. Such a point only splits a polygon side, so it goes from its
/// polygons and the mesh. Border points stay (CMD-E-01/02 measured only
/// points that keep other edges), as do points a polyline uses, and a
/// point is kept if taking it out would leave a polygon under three sides.
fn leftover_points(out: &Mesh, chosen: &BTreeSet<(u32, u32)>) -> Vec<u32> {
    let uses = edge_uses(out);
    let mut around: BTreeMap<u32, Vec<(u32, u32)>> = BTreeMap::new();
    for &(a, b) in chosen {
        around.entry(a).or_default();
        around.entry(b).or_default();
    }
    for &(a, b) in uses.keys() {
        for v in [a, b] {
            if let Some(edges) = around.get_mut(&v) {
                edges.push((a, b));
            }
        }
    }
    let on_polyline: BTreeSet<u32> = (0..out.faces.len())
        .filter(|&f| out.is_polyline(f))
        .flat_map(|f| out.faces[f].vertices.iter().copied())
        .collect();
    // Sides each polygon has lost so far.
    let mut lost: BTreeMap<usize, usize> = BTreeMap::new();
    let mut leftover = Vec::new();
    for (&v, edges) in &around {
        let unused = edges.is_empty();
        let in_a_run = edges.len() == 2 && edges.iter().all(|e| uses[e].len() == 2);
        if on_polyline.contains(&v) || !(unused || in_a_run) {
            continue;
        }
        let faces: BTreeSet<usize> = edges
            .iter()
            .flat_map(|e| uses[e].iter().map(|&(f, _, _)| f))
            .collect();
        let sides = |f: usize| out.faces[f].vertices.len() - lost.get(&f).copied().unwrap_or(0);
        if faces.iter().any(|&f| sides(f) <= 3) {
            continue;
        }
        for f in faces {
            *lost.entry(f).or_default() += 1;
        }
        leftover.push(v);
    }
    leftover
}

fn spin(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let uses = edge_uses(mesh);
    let mut replaced: BTreeMap<usize, OutFace> = BTreeMap::new();
    let mut new_edges = Vec::new();
    for &(a, b) in edges {
        let [(f1, x1, _), (f2, _, _)] =
            uses.get(&edge_key(a, b)).map(|u| &u[..]).unwrap_or(&[])[..]
        else {
            return Err("Spin turns interior edges (two neighbouring polygons)".into());
        };
        if f1 == f2 || replaced.contains_key(&f1) || replaced.contains_key(&f2) {
            return Err("Spin edges must not share polygons".into());
        }
        // F1 traverses a→b, F2 traverses b→a.
        let (a, b) = if x1 == a { (a, b) } else { (b, a) };
        let rotate = |f: usize, start: u32| -> Vec<(u32, (usize, usize))> {
            let vs = &mesh.faces[f].vertices;
            let i = vs.iter().position(|&v| v == start).unwrap_or(0);
            (0..vs.len())
                .map(|k| (i + k) % vs.len())
                .map(|c| (vs[c], (f, c)))
                .collect()
        };
        // Ring: b, F1 after b ... a, F2 after a ... (back to b).
        let one = rotate(f1, b); // b, f1_2.., a
        let two = rotate(f2, a); // a, f2_2.., b
        let k = one.len() - 1; // index of a in the ring
        let mut ring: Vec<(u32, (usize, usize))> = one.clone();
        ring.extend(two[1..two.len() - 1].iter().copied());
        let n = ring.len();
        if one.len() < 3 || two.len() < 3 || n < 4 {
            return Err("Spin needs two polygons".into());
        }
        let (nb, na) = (1, (k + 1) % n);
        let side = |from: usize, to: usize, template: usize| {
            let mut idx = from;
            let mut verts = Vec::new();
            let mut donors = Vec::new();
            loop {
                let (v, (df, dc)) = ring[idx];
                verts.push(v);
                // Corners keep data from their own polygon when they had one.
                donors.push(if df == template {
                    Some((df, dc))
                } else {
                    let vs = &mesh.faces[template].vertices;
                    vs.iter()
                        .position(|&x| x == v)
                        .map(|c| (template, c))
                        .or(Some((df, dc)))
                });
                if idx == to {
                    break;
                }
                idx = (idx + 1) % n;
            }
            OutFace {
                template,
                vertices: verts,
                donors,
                blends: Vec::new(),
            }
        };
        let g1 = side(nb, na, f1);
        let g2 = side(na, nb, f2);
        let e = edge_key(ring[nb].0, ring[na].0);
        if uses.contains_key(&e) {
            return Err("Spin would duplicate an existing edge".into());
        }
        new_edges.push(e);
        replaced.insert(f1, g1);
        replaced.insert(f2, g2);
    }
    let faces = (0..mesh.faces.len())
        .map(|f| {
            replaced
                .remove(&f)
                .unwrap_or_else(|| OutFace::copy(mesh, f, mesh.faces[f].vertices.clone()))
        })
        .collect();
    let out = rebuild(mesh, mesh.vertices.clone(), faces, &[], "Spin")?;
    Ok((finish(out, "Spin")?, new_edges))
}

fn split(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let chosen: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    let points: BTreeSet<u32> = chosen.iter().flat_map(|&(a, b)| [a, b]).collect();
    let mut vertices = mesh.vertices.clone();
    let mut faces: Vec<Vec<u32>> = mesh.faces.iter().map(|f| f.vertices.clone()).collect();
    let mut copies = Vec::new();
    for &v in &points {
        // Polygons around v stay together across unselected edges.
        let around: Vec<usize> = (0..mesh.faces.len())
            .filter(|&f| !mesh.is_polyline(f) && mesh.faces[f].vertices.contains(&v))
            .collect();
        let mut group: Vec<usize> = (0..around.len()).collect();
        fn root(g: &mut [usize], mut i: usize) -> usize {
            while g[i] != i {
                i = g[i];
            }
            i
        }
        let neighbours = |f: usize| -> [u32; 2] {
            let vs = &mesh.faces[f].vertices;
            let i = vs.iter().position(|&x| x == v).unwrap_or(0);
            [vs[(i + vs.len() - 1) % vs.len()], vs[(i + 1) % vs.len()]]
        };
        for i in 0..around.len() {
            for j in i + 1..around.len() {
                let shared = neighbours(around[i])
                    .into_iter()
                    .find(|w| neighbours(around[j]).contains(w));
                if let Some(w) = shared
                    && !chosen.contains(&edge_key(v, w))
                {
                    let (ri, rj) = (root(&mut group, i), root(&mut group, j));
                    group[ri] = rj;
                }
            }
        }
        let roots: BTreeSet<usize> = (0..around.len()).map(|i| root(&mut group, i)).collect();
        for r in roots.into_iter().skip(1) {
            let new = u32::try_from(vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
            vertices.push(mesh.vertices[v as usize]);
            copies.push((new, v));
            for (i, &f) in around.iter().enumerate() {
                if root(&mut group, i) == r {
                    for x in &mut faces[f] {
                        if *x == v {
                            *x = new;
                        }
                    }
                }
            }
        }
    }
    let out_faces = faces
        .into_iter()
        .enumerate()
        .map(|(f, vs)| OutFace::copy(mesh, f, vs))
        .collect();
    let out = rebuild(mesh, vertices, out_faces, &copies, "Split")?;
    let out = finish(out, "Split")?;
    // PF-10: split all round a closed loop, the cut is two loops and
    // nothing stays selected. Open runs keep what still exists.
    let closed = points
        .iter()
        .all(|&v| chosen.iter().filter(|&&(a, b)| a == v || b == v).count() == 2);
    if closed {
        return Ok((out, Vec::new()));
    }
    let now: BTreeSet<(u32, u32)> = out.edges().into_iter().collect();
    let kept = chosen.into_iter().filter(|e| now.contains(e)).collect();
    Ok((out, kept))
}

/// Ordered boundary chains of the selected edges, each oriented along its
/// neighbouring polygon's traversal. Returns (points, closed, faces per edge).
type Chain = (Vec<u32>, bool, Vec<usize>);
fn border_chains(mesh: &Mesh, edges: &[(u32, u32)], what: &str) -> Result<Vec<Chain>> {
    let uses = edge_uses(mesh);
    let mut next: BTreeMap<u32, (u32, usize)> = BTreeMap::new();
    let mut has_prev = BTreeSet::new();
    for &(a, b) in edges {
        let [(f, x, y)] = uses.get(&edge_key(a, b)).map(|u| &u[..]).unwrap_or(&[])[..] else {
            return Err(format!(
                "{what} needs border edges (one neighbouring polygon)"
            ));
        };
        if next.insert(x, (y, f)).is_some() || !has_prev.insert(y) {
            return Err(format!("{what} needs simple border chains"));
        }
    }
    let mut chains = Vec::new();
    let mut seen = BTreeSet::new();
    let starts: Vec<u32> = next
        .keys()
        .copied()
        .filter(|v| !has_prev.contains(v))
        .chain(next.keys().copied())
        .collect();
    for start in starts {
        if seen.contains(&start) || !next.contains_key(&start) {
            continue;
        }
        let (mut points, mut faces) = (vec![start], Vec::new());
        seen.insert(start);
        let mut v = start;
        let mut closed = false;
        while let Some(&(w, f)) = next.get(&v) {
            faces.push(f);
            if w == start {
                closed = true;
                break;
            }
            points.push(w);
            seen.insert(w);
            v = w;
        }
        chains.push((points, closed, faces));
    }
    Ok(chains)
}

/// `(face, corner)` where `face` uses point `v`.
fn corner_in(mesh: &Mesh, face: usize, v: u32) -> Option<(usize, usize)> {
    let c = mesh.faces[face].vertices.iter().position(|&x| x == v)?;
    Some((face, c))
}

/// The corner of point `v` in whichever of `faces` faces most like
/// `normal` (so a point shared with other sides takes this side's UVs).
fn corner_among(mesh: &Mesh, faces: &[usize], v: u32, normal: DVec3) -> Option<(usize, usize)> {
    faces
        .iter()
        .filter_map(|&f| Some((corner_in(mesh, f, v)?, mesh.normal(f).dot(normal))))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(corner, _)| corner)
}

fn grow_quads(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let chains = border_chains(mesh, edges, "Grow Quads")?;
    let p = |v: u32| mesh.vertices[v as usize];
    let mut vertices = mesh.vertices.clone();
    let mut faces: Vec<OutFace> = (0..mesh.faces.len())
        .map(|f| OutFace::copy(mesh, f, mesh.faces[f].vertices.clone()))
        .collect();
    let mut outer = Vec::new();
    for (points, _closed, polys) in chains {
        let n = points.len();
        // Outward in-plane direction and preceding-row size per point.
        let mut dir = vec![DVec3::ZERO; n];
        let mut size = vec![(0., 0); n];
        // Per border polygon, the inner side point of each end (for UVs).
        let mut sides: Vec<[u32; 2]> = Vec::with_capacity(polys.len());
        for (e, &f) in polys.iter().enumerate() {
            let mut pair = [0; 2];
            let (i, j) = (e, (e + 1) % n);
            let (x, y) = (points[i], points[j]);
            let normal = mesh.normal(f);
            let out_dir = unit((p(y) - p(x)).cross(normal), "Grow Quads border")?;
            let vs = &mesh.faces[f].vertices;
            for (k, v, other) in [(i, x, y), (j, y, x)] {
                dir[k] += out_dir;
                let c = vs.iter().position(|&u| u == v).unwrap_or(0);
                let (prev, nxt) = (vs[(c + vs.len() - 1) % vs.len()], vs[(c + 1) % vs.len()]);
                let side = if prev == other { nxt } else { prev };
                size[k].0 += p(v).distance(p(side));
                size[k].1 += 1;
                pair[usize::from(k == j)] = side;
            }
            sides.push(pair);
        }
        let mut grown = Vec::with_capacity(n);
        for k in 0..n {
            let d = unit(dir[k], "Grow Quads direction")?;
            let len = size[k].0 / size[k].1.max(1) as f64;
            let id = u32::try_from(vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
            vertices.push(p(points[k]) + d * len);
            grown.push(id);
        }
        for (e, &template) in polys.iter().enumerate() {
            let (i, j) = (e, (e + 1) % n);
            // Provisional UVs (SQ-UV-01): border corners keep their UVs in
            // the border polygon (but see the continuous UVs below); grown
            // points continue them outward by one row's step
            // (2 * border - inner side point).
            let at = |v: u32| corner_in(mesh, template, v);
            let grown_uv = |v: u32, side: u32| -> Option<CornerBlend> {
                Some(vec![(at(v)?, 2.), (at(side)?, -1.)])
            };
            faces.push(OutFace {
                template,
                vertices: vec![points[j], points[i], grown[i], grown[j]],
                donors: vec![None; 4],
                blends: vec![
                    at(points[j]).map(|c| vec![(c, 1.)]),
                    at(points[i]).map(|c| vec![(c, 1.)]),
                    grown_uv(points[i], sides[e][0]),
                    grown_uv(points[j], sides[e][1]),
                ],
            });
            outer.push(edge_key(grown[i], grown[j]));
        }
    }
    let mut out = rebuild(mesh, vertices, faces, &[], "Grow Quads")?;
    // UVN-T-06: a grown quad's two border corners carry their points'
    // continuous UVs, not the border polygon's (they differ at a seam, so
    // the measured quad jumps across it). A stored point UV says what that
    // is (`corner_uv::continuous_uv`). A mesh with corner UVs only keeps
    // none, so it is recovered where the border polygon sits in an island
    // moved by a constant shift from a lower-numbered polygon across a
    // shared edge (the seam of UVN-O-grow-quads-seam): its UV plus that
    // shift (`seam_shift`, provisional). Otherwise the border polygon's UV
    // stays.
    let mut patched = false;
    let point_faces = mesh.point_faces();
    for (name, rows) in out.uv_maps.iter_mut() {
        let points = mesh.attributes.point_uvs.get(name);
        let before = mesh.uv_maps.get(name);
        for (f, row) in rows.iter_mut().enumerate().skip(mesh.faces.len()) {
            let border = &out.faces[f].vertices;
            let shift = match (points, before, border.get(..2)) {
                (None, Some(input), Some(&[a, b])) => point_faces
                    .of(a)
                    .iter()
                    .map(|&t| t as usize)
                    .find(|&t| mesh.faces[t].vertices.contains(&b))
                    .and_then(|t| seam_shift(mesh, input, t)),
                _ => None,
            };
            for (corner, &v) in row.iter_mut().zip(border).take(2) {
                let uv = match (points, shift) {
                    (Some(points), _) => crate::corner_uv::continuous_uv(Some(points), v, []),
                    (None, Some(shift)) => corner.map(|uv| uv + shift),
                    (None, None) => None,
                };
                if let Some(uv) = uv
                    && *corner != Some(uv)
                {
                    *corner = Some(uv);
                    patched = true;
                }
            }
        }
    }
    if patched {
        let o = &mut out.attributes;
        for (name, records) in o.uv_records.iter_mut() {
            *records = crate::corner_uv::records_from_uvs(&out.faces, &out.uv_maps[name]);
        }
    }
    Ok((finish(out, "Grow Quads")?, outer))
}

fn fill_quads(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let chains = border_chains(mesh, edges, "Fill Quads")?;
    let [(points, true, polys)] = &chains[..] else {
        return Err("Fill Quads needs one closed border loop".into());
    };
    let n = points.len();
    if n < 4 || n % 2 != 0 {
        return Err("Fill Quads needs a loop with an even number of edges (4 or more)".into());
    }
    // Fill runs against the border's traversal.
    let fill: Vec<u32> = points.iter().rev().copied().collect();
    let p = |v: u32| mesh.vertices[v as usize];
    let quads = if n == 8 {
        fan_quads(&fill, edges, p)
    } else {
        ladder_quads(&fill, p)
    };
    let mut faces: Vec<OutFace> = (0..mesh.faces.len())
        .map(|f| OutFace::copy(mesh, f, mesh.faces[f].vertices.clone()))
        .collect();
    // Polygons using each border point, for UVs.
    let mut around: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (f, face) in mesh.faces.iter().enumerate() {
        for &x in &face.vertices {
            if points.contains(&x) {
                around.entry(x).or_default().push(f);
            }
        }
    }
    for vertices in quads {
        let vertices = vertices.to_vec();
        // Provisional UVs (SQ-UV-01): each corner takes the UV its point has
        // in the polygon using it that faces most like the fill.
        let normal = (p(vertices[2]) - p(vertices[0])).cross(p(vertices[3]) - p(vertices[1]));
        let unit_normal = normal.normalize_or_zero();
        let sources: Vec<Option<(usize, usize)>> = vertices
            .iter()
            .map(|&x| corner_among(mesh, &around[&x], x, normal))
            .collect();
        // A corner whose polygons all face another way (a point on a cube
        // edge whose same-side polygon was the hole) would borrow another
        // side's UVs. With the other three trusted, complete the
        // parallelogram from them instead.
        let trusted = |c: Option<(usize, usize)>| {
            c.is_some_and(|(f, _)| mesh.normal(f).normalize_or_zero().dot(unit_normal) > 0.9)
        };
        let odd: Vec<usize> = (0..4).filter(|&k| !trusted(sources[k])).collect();
        let mut blends: Vec<Option<CornerBlend>> =
            sources.iter().map(|c| c.map(|c| vec![(c, 1.)])).collect();
        if let [k] = odd[..] {
            let (prev, next, opposite) = (
                sources[(k + 3) % 4],
                sources[(k + 1) % 4],
                sources[(k + 2) % 4],
            );
            blends[k] = prev
                .zip(next)
                .zip(opposite)
                .map(|((a, b), c)| vec![(a, 1.), (b, 1.), (c, -1.)]);
        }
        faces.push(OutFace {
            template: polys[0],
            vertices,
            donors: vec![None; 4],
            blends,
        });
    }
    let mut out = rebuild(mesh, mesh.vertices.clone(), faces, &[], "Fill Quads")?;
    let ring: BTreeSet<usize> = around.values().flatten().copied().collect();
    if n == 8 && crate::corner_uv::flat(mesh, ring) {
        fan_seam_uvs(mesh, &mut out);
    }
    Ok((
        finish(out, "Fill Quads")?,
        edges.iter().map(|&(a, b)| edge_key(a, b)).collect(),
    ))
}

/// Fill Quads' ladder: parallel rungs across the loop, `fill` in the new
/// polygons' winding, starting where the rungs are shortest in all. Loops
/// of 4, 6 and 10 or more edges (a 2×1 hole gets its middle rung).
fn ladder_quads(fill: &[u32], p: impl Fn(u32) -> DVec3) -> Vec<[u32; 4]> {
    let n = fill.len();
    let m = n / 2;
    let rungs = |s: usize| -> f64 {
        (0..m.saturating_sub(1))
            .map(|i| p(fill[(s + i + 1) % n]).distance(p(fill[(s + n - 2 - i) % n])))
            .sum()
    };
    let start = (0..m)
        .min_by(|&a, &b| rungs(a).total_cmp(&rungs(b)))
        .unwrap_or(0);
    let v = |i: usize| fill[(start + i) % n];
    (0..m - 1)
        .map(|i| [v(i), v(i + 1), v(n - 2 - i), v(n - 1 - i)])
        .collect()
}

/// Fill Quads on an 8-edge loop (FQ-Q01, provisional): three border edges
/// at a time from a start point `s`, walking the way the selection runs, so
/// both cuts fan from `s + 3`: [s..s+3], [s+3..s+6], [s+6, s+7, s, s+3].
/// `s` makes the two cuts shortest in all; among ties the one whose last
/// cut is shorter, then the first from the selection's start. That gives
/// the measured 3×1 hole (a square at the start end, then a diagonal from
/// the far end's start-side corner to the top of its rung) and UVN-T-07's
/// fan from the middle of one side of a 2×2 hole. Corners are returned in
/// `fill`'s winding.
fn fan_quads(fill: &[u32], edges: &[(u32, u32)], p: impl Fn(u32) -> DVec3) -> Vec<[u32; 4]> {
    let n = fill.len();
    let at = |v: u32| fill.iter().position(|&x| x == v);
    // The selection's start and direction: the first edge's point that the
    // second edge doesn't share, toward the one it does. Without that, the
    // loop's own order.
    let (first, forward) = match edges {
        [(a, b), (c, d), ..] => {
            let (start, next) = if *a == *c || *a == *d {
                (*b, *a)
            } else {
                (*a, *b)
            };
            match (at(start), at(next)) {
                (Some(i), Some(j)) => (i, j == (i + 1) % n),
                _ => (0, true),
            }
        }
        _ => (0, true),
    };
    let sel = |i: usize| {
        let i = i % n;
        if forward {
            fill[(first + i) % n]
        } else {
            fill[(first + n - i) % n]
        }
    };
    let cut = |a: usize, b: usize| p(sel(a)).distance(p(sel(b)));
    let cost = |s: usize| (cut(s, s + 3) + cut(s + 3, s + 6), cut(s + 3, s + 6));
    let near = |a: f64, b: f64| (a - b).abs() <= 1e-9 * (1. + a.abs().max(b.abs()));
    let mut best = 0;
    for s in 1..n {
        let ((total, last), (best_total, best_last)) = (cost(s), cost(best));
        if (!near(total, best_total) && total < best_total)
            || (near(total, best_total) && !near(last, best_last) && last < best_last)
        {
            best = s;
        }
    }
    let s = best;
    // Each lists the fan point second, as the reference's polygons do;
    // `fan_seam_uvs` reads the first edge.
    [
        [s + 2, s + 3, s, s + 1],
        [s + 6, s + 3, s + 4, s + 5],
        [s, s + 3, s + 6, s + 7],
    ]
    .map(|q| {
        let mut q = q.map(sel);
        if !forward {
            q.reverse();
            q.rotate_left(1);
        }
        q
    })
    .to_vec()
}

/// UVN-O-fill-quads-seam (FQ-Q01, provisional; flat holes of 8 edges): a
/// new polygon's first two corners take the UVs of the polygon across its
/// first edge when that edge is on the border (so they stay on that side
/// of a seam); every other corner takes its point's continuous UV. A mesh
/// with corner UVs only keeps no continuous UV, so it is recovered as in
/// Grow Quads: a point's UV in a polygon of the lowest-numbered island, or
/// in a seam island plus the island's constant shift (`island_shift`).
fn fan_seam_uvs(mesh: &Mesh, out: &mut Mesh) {
    let base = mesh.faces.len();
    let uses = edge_uses(mesh);
    let point_faces = mesh.point_faces();
    let mut patched = false;
    for (name, rows) in out.uv_maps.iter_mut() {
        let Some(input) = mesh.uv_maps.get(name) else {
            continue;
        };
        let points = mesh.attributes.point_uvs.get(name);
        let uv = |f: usize, v: u32| crate::corner_uv::corner_uv(mesh, input, f, v);
        let continuous = |v: u32| -> Option<DVec2> {
            if let Some(&uv) = points.and_then(|m| m.get(&v)) {
                return Some(uv);
            }
            let around = point_faces.of(v);
            let shifts: Vec<(usize, Option<DVec2>)> = around
                .iter()
                .map(|&f| (f as usize, island_shift(mesh, input, f as usize)))
                .collect();
            shifts
                .iter()
                .find(|(_, s)| s.is_none())
                .and_then(|&(f, _)| uv(f, v))
                .or_else(|| shifts.iter().find_map(|&(f, s)| Some(uv(f, v)? + s?)))
        };
        for (f, row) in rows.iter_mut().enumerate().skip(base) {
            let vertices = &out.faces[f].vertices;
            let lender = uses
                .get(&edge_key(vertices[0], vertices[1]))
                .and_then(|u| u.first())
                .map(|u| u.0);
            for (c, (corner, &v)) in row.iter_mut().zip(vertices).enumerate() {
                let new = match lender {
                    Some(t) if c < 2 => uv(t, v),
                    _ => continuous(v),
                };
                if let Some(new) = new
                    && *corner != Some(new)
                {
                    *corner = Some(new);
                    patched = true;
                }
            }
        }
    }
    if patched {
        let o = &mut out.attributes;
        for (name, records) in o.uv_records.iter_mut() {
            *records = crate::corner_uv::records_from_uvs(&out.faces, &out.uv_maps[name]);
        }
    }
}

/// The constant UV shift that brings polygon `t`'s UV island back to a
/// neighbouring island numbered below all its polygons (the island holds
/// the polygons joined to `t` across edges where both agree on the UVs);
/// `None` for the lowest-numbered island or with no such neighbour.
fn island_shift(mesh: &Mesh, rows: &crate::corner_uv::UvRows, t: usize) -> Option<DVec2> {
    let point_faces = mesh.point_faces();
    let uv = |f: usize, v: u32| crate::corner_uv::corner_uv(mesh, rows, f, v);
    let across = |f: usize, a: u32, b: u32| {
        point_faces
            .of(a)
            .iter()
            .map(|&o| o as usize)
            .filter(move |&o| o != f && mesh.faces[o].vertices.contains(&b))
    };
    let edges = |f: usize| {
        let face = &mesh.faces[f].vertices;
        (0..face.len()).map(move |i| (face[i], face[(i + 1) % face.len()]))
    };
    let shift = |f: usize, o: usize, a: u32, b: u32| -> Option<DVec2> {
        let (sa, sb) = (uv(o, a)? - uv(f, a)?, uv(o, b)? - uv(f, b)?);
        (sa.distance(sb) <= 1e-9).then_some(sa)
    };
    let mut island = BTreeSet::from([t]);
    let mut queue = vec![t];
    while let Some(f) = queue.pop() {
        for (a, b) in edges(f) {
            for o in across(f, a, b) {
                if !island.contains(&o) && shift(f, o, a, b).is_some_and(|s| s.length() <= 1e-9) {
                    island.insert(o);
                    queue.push(o);
                }
            }
        }
    }
    let lowest = *island.first()?;
    island.iter().find_map(|&f| {
        edges(f).find_map(|(a, b)| {
            across(f, a, b)
                .filter(|&o| o < lowest)
                .find_map(|o| shift(f, o, a, b).filter(|s| s.length() > 1e-9))
        })
    })
}

fn to_curves(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<(Mesh, Vec<(u32, u32)>)> {
    // Ordered chains through the selected edges (any polygon side).
    let mut adjacency: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(a, b) in edges {
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
    }
    if adjacency.values().any(|n| n.len() > 2) {
        return Err("Edges To Curves needs simple chains (no branches)".into());
    }
    let mut used: BTreeSet<(u32, u32)> = BTreeSet::new();
    let mut chains: Vec<(Vec<u32>, bool)> = Vec::new();
    // First selected edge's first point starts its chain when it is an end.
    let mut starts: Vec<u32> = adjacency
        .iter()
        .filter(|(_, n)| n.len() == 1)
        .map(|(&v, _)| v)
        .collect();
    starts.sort_by_key(|v| edges.iter().position(|&(a, b)| a == *v || b == *v));
    starts.extend(edges.iter().map(|&(a, _)| a));
    for s in starts {
        let mut v = s;
        let mut chain = vec![s];
        let mut closed = false;
        while let Some(&w) = adjacency[&v]
            .iter()
            .find(|&&w| !used.contains(&edge_key(v, w)))
        {
            used.insert(edge_key(v, w));
            if w == s {
                closed = true;
                break;
            }
            chain.push(w);
            v = w;
        }
        if chain.len() > 1 {
            chains.push((chain, closed));
        }
    }
    let mut out = mesh.clone();
    if out.attributes.representations.is_empty() {
        out.attributes.representations = vec![PolygonRepresentation::Face; out.faces.len()];
    }
    let mut selection = Vec::new();
    for (chain, closed) in chains {
        let base = u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
        out.vertices
            .extend(chain.iter().map(|&v| mesh.vertices[v as usize]));
        let ids: Vec<u32> = (0..chain.len() as u32).map(|i| base + i).collect();
        for w in ids.windows(2) {
            selection.push(edge_key(w[0], w[1]));
        }
        if closed {
            selection.push(edge_key(ids[0], *ids.last().unwrap()));
        }
        let k = ids.len();
        out.faces.push(Face {
            vertices: ids,
            uv: None,
            material: mesh
                .faces
                .first()
                .map(|f| f.material.clone())
                .unwrap_or_default(),
            smooth: false,
            part: None,
            material_label: None,
        });
        out.attributes
            .representations
            .push(PolygonRepresentation::Polyline {
                closed,
                curve: Default::default(),
            });
        if !out.attributes.smoothing_labels.is_empty() {
            out.attributes.smoothing_labels.push(None);
        }
        for rows in out.uv_maps.values_mut() {
            rows.push(vec![None; k]);
        }
        let at = &mut out.attributes;
        for rows in at.uv_records.values_mut() {
            rows.push(vec![None; k]);
        }
        for rows in at.corner_weights.values_mut() {
            rows.push(vec![None; k]);
        }
        for rows in at.corner_colors.values_mut() {
            rows.push(vec![None; k]);
        }
        for rows in at.corner_normals.values_mut() {
            rows.push(vec![None; k]);
        }
        for m in at.typed_maps.values_mut() {
            if !m.corners.is_empty() {
                m.corners.push(vec![None; k]);
            }
        }
    }
    Ok((finish(out, "Edges To Curves")?, selection))
}

fn unbevel(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let chosen: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    // Strip quads: two opposite edges selected, the other two not.
    let mut cross: BTreeSet<(u32, u32)> = BTreeSet::new();
    let mut strips = BTreeSet::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        let v = &f.vertices;
        if v.len() != 4 || mesh.is_polyline(fi) {
            continue;
        }
        for s in 0..2 {
            let e = |k: usize| edge_key(v[(s + k) % 4], v[(s + k + 1) % 4]);
            if chosen.contains(&e(0))
                && chosen.contains(&e(2))
                && !chosen.contains(&e(1))
                && !chosen.contains(&e(3))
            {
                strips.insert(fi);
                cross.insert(e(1));
                cross.insert(e(3));
            }
        }
    }
    if strips.is_empty() {
        return Err("Unbevel: select the two edge loops on either side of a bevel".into());
    }
    let in_strip = |x: u32, y: u32| {
        strips.iter().any(|&f| {
            let vs = &mesh.faces[f].vertices;
            (0..4).any(|i| edge_key(vs[i], vs[(i + 1) % 4]) == edge_key(x, y))
        })
    };
    let p = |v: u32| mesh.vertices[v as usize];
    // The side edge leaving a strip point (not on any strip polygon).
    let side = |u: u32| -> Result<DVec3> {
        let mut found: Option<u32> = None;
        for e in mesh.edges() {
            let w = if e.0 == u {
                e.1
            } else if e.1 == u {
                e.0
            } else {
                continue;
            };
            if !in_strip(u, w) && !chosen.contains(&e) {
                if found.is_some_and(|x| x != w) {
                    return Err("Unbevel needs one side edge at each bevel point".to_string());
                }
                found = Some(w);
            }
        }
        if let Some(w) = found {
            return unit(p(u) - p(w), "Unbevel side");
        }
        // Inside corner of one neighbouring polygon (e.g. a bevelled cap):
        // continue outward along the corner bisector in its plane.
        let rails: Vec<u32> = chosen
            .iter()
            .filter_map(|&(x, y)| (x == u).then_some(y).or((y == u).then_some(x)))
            .collect();
        let [r0, r1] = rails[..] else {
            return Err("Unbevel needs one side edge at each bevel point".into());
        };
        let owner = (0..mesh.faces.len())
            .find(|&f| {
                let vs = &mesh.faces[f].vertices;
                !strips.contains(&f)
                    && (0..vs.len())
                        .any(|i| edge_key(vs[i], vs[(i + 1) % vs.len()]) == edge_key(u, r0))
                    && (0..vs.len())
                        .any(|i| edge_key(vs[i], vs[(i + 1) % vs.len()]) == edge_key(u, r1))
            })
            .ok_or("Unbevel needs one side edge at each bevel point")?;
        let vs = &mesh.faces[owner].vertices;
        let normal = mesh.normal(owner);
        let i = vs.iter().position(|&x| x == u).unwrap_or(0);
        let (prev, next) = (vs[(i + vs.len() - 1) % vs.len()], vs[(i + 1) % vs.len()]);
        let n1 = unit(normal.cross(p(next) - p(u)), "Unbevel corner")?;
        let n2 = unit(normal.cross(p(u) - p(prev)), "Unbevel corner")?;
        unit(-(n1 + n2), "Unbevel corner")
    };
    let mut groups = Vec::new();
    for &(u, v) in &cross {
        let (du, dv) = (side(u)?, side(v)?);
        let r = p(u) - p(v);
        let b = du.dot(dv);
        let den = 1. - b * b;
        if den < 1e-9 {
            return Err("Unbevel sides are parallel; there is no edge to restore".into());
        }
        let t = (b * dv.dot(r) - du.dot(r)) / den;
        let s = (dv.dot(r) - b * du.dot(r)) / den;
        groups.push((vec![u, v], (p(u) + du * t + p(v) + dv * s) * 0.5));
    }
    let (out, remap) = weld(mesh, &groups, "Unbevel")?;
    let mut sharp: BTreeSet<(u32, u32)> = BTreeSet::new();
    for &(a, b) in &chosen {
        if let (Some(x), Some(y)) = (remap[a as usize], remap[b as usize])
            && x != y
        {
            sharp.insert(edge_key(x, y));
        }
    }
    Ok((finish(out, "Unbevel")?, sharp.into_iter().collect()))
}

fn split_normals(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let at = &mesh.attributes;
    let name = match (at.corner_normals.len(), at.normal_map_order.first()) {
        (0, _) => {
            return Err("Split Normals needs a vertex normal map; create one first".into());
        }
        (1, _) => at.corner_normals.keys().next().unwrap().clone(),
        (_, Some(first)) => first.clone(),
        _ => return Err("Split Normals: several normal maps and no order".into()),
    };
    let uses = edge_uses(mesh);
    let mut out = mesh.clone();
    let rows = out
        .attributes
        .corner_normals
        .get_mut(&name)
        .ok_or("Missing normal map")?;
    for &(a, b) in edges {
        for &(f, _, _) in uses.get(&edge_key(a, b)).into_iter().flatten() {
            let n = mesh.normal(f);
            for (c, &v) in mesh.faces[f].vertices.iter().enumerate() {
                if v == a || v == b {
                    rows[f][c] = Some(n);
                }
            }
        }
    }
    Ok((
        finish(out, "Split Normals")?,
        edges.iter().map(|&(a, b)| edge_key(a, b)).collect(),
    ))
}

impl crate::Editor {
    /// Run an Edge command row on the selected edges as one undoable edit.
    pub(crate) fn edge_command(&mut self, command: EdgeCommand) -> Result<()> {
        let item = self.edge_insert_target(command.label())?;
        if self.scene.selection.mode != crate::Mode::Edge {
            return Err(format!("{} works in Edge mode", command.label()));
        }
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let edges = &self.scene.selection.edges;
        let (out, selection) = match command {
            // With Symmetry, each side's pair joins (SYM-JOIN-Q1).
            EdgeCommand::Join | EdgeCommand::JoinAveraged => {
                let averaged = command == EdgeCommand::JoinAveraged;
                let symmetry = self.active_symmetry();
                (
                    edge_join_symmetric_geometry(mesh, edges, symmetry, averaged)?,
                    Vec::new(),
                )
            }
            _ => edge_command_geometry(mesh, edges, command)?,
        };
        if out == **mesh {
            return Ok(());
        }
        let old_points = mesh.vertices.len();
        let selection = match command {
            // CMD-O-edge-join-two: Join leaves nothing selected.
            EdgeCommand::Join | EdgeCommand::JoinAveraged => Vec::new(),
            _ => selection,
        };
        self.publish_edge_edit(item, out, command.label(), Some(selection))?;
        if command == EdgeCommand::Subdivide {
            self.select_new_points(old_points);
        }
        Ok(())
    }

    /// CMD-E-06 / CMD-B3-01: after Subdivide the new points are selected
    /// (in the point selection, alongside their edges in the edge
    /// selection). Subdivide only appends points, so the new ones are the
    /// indices from `old_points` on.
    fn select_new_points(&mut self, old_points: usize) {
        if let Some(item) = self.scene.active() {
            let new = old_points as u32..item.mesh.vertices.len() as u32;
            self.scene.selection.vertices = new.collect();
        }
    }

    pub(crate) fn subdivide_edges(&mut self, count: u32, percent: Option<f64>) -> Result<()> {
        let item = self.edge_insert_target("Subdivide")?;
        if self.scene.selection.mode != crate::Mode::Edge {
            return Err("Subdivide works in Edge mode".into());
        }
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, selection) =
            edge_subdivide_geometry(mesh, &self.scene.selection.edges, count, percent)?;
        if out == **mesh {
            return Ok(());
        }
        let old_points = mesh.vertices.len();
        self.publish_edge_edit(item, out, "Subdivide", Some(selection))?;
        self.select_new_points(old_points);
        Ok(())
    }
}

/// ME-T04 Bridge for edge selections with straight rows (Linear mode); see
/// [`edge_bridge`] for the full option set.
pub fn edge_bridge_geometry(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    segments: u32,
    twist: i32,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let p = crate::BridgeParameters {
        segments,
        twist,
        mode: crate::BridgeMode::Linear,
        ..Default::default()
    };
    edge_bridge(mesh, edges, p)
}

/// One selected border run for Bridge: its points in its border polygons'
/// corner order, whether it closes into a loop, and per span the border
/// polygon it borrows tags and UVs from.
struct EdgeRun {
    points: Vec<u32>,
    closed: bool,
    faces: Vec<usize>,
}

/// Split selected border edges into simple runs (paths or loops), each in
/// its border polygons' corner order. Edges inside the surface are refused:
/// they already have polygons on both sides, so a bridge there would give
/// an edge three polygons, which meshes here don't allow.
fn edge_runs(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<Vec<EdgeRun>> {
    let uses = edge_uses(mesh);
    let keys: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    let mut next: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(a, b) in &keys {
        match uses.get(&(a, b)).map(Vec::len) {
            Some(1) => {}
            Some(_) => {
                return Err(
                    "Bridge needs border edges (one polygon each); edges inside the surface already have polygons on both sides"
                        .into(),
                );
            }
            None => return Err("Bridge needs edges of the mesh's polygons".into()),
        }
        next.entry(a).or_default().push(b);
        next.entry(b).or_default().push(a);
    }
    if next.values().any(|n| n.len() > 2) {
        return Err("Bridge needs simple edge runs (no point joins three selected edges)".into());
    }
    let mut seen = BTreeSet::new();
    let ends: Vec<u32> = next
        .iter()
        .filter(|(_, n)| n.len() == 1)
        .map(|(&v, _)| v)
        .chain(next.keys().copied())
        .collect();
    let mut runs = Vec::new();
    for start in ends {
        if !seen.insert(start) {
            continue;
        }
        let mut points = vec![start];
        let (mut prev, mut v) = (u32::MAX, start);
        let mut closed = false;
        while let Some(&w) = next[&v].iter().find(|&&w| w != prev) {
            if w == start {
                closed = points.len() > 2;
                break;
            }
            if !seen.insert(w) {
                break;
            }
            points.push(w);
            (prev, v) = (v, w);
        }
        // Turn the run to its border polygons' corner order (the mesh is
        // consistently wound, so every span agrees with the first).
        let n = points.len();
        let spans = if closed { n } else { n - 1 };
        if uses[&edge_key(points[0], points[1])][0].1 != points[0] {
            points.reverse();
            if closed {
                // Keep the same starting point after turning the loop.
                points.rotate_right(1);
            }
        }
        let faces = (0..spans)
            .map(|i| {
                let (x, y) = (points[i], points[(i + 1) % n]);
                uses[&edge_key(x, y)][0].0
            })
            .collect();
        runs.push(EdgeRun {
            points,
            closed,
            faces,
        });
    }
    Ok(runs)
}

/// The direction a bridge leaves `run` at chain position `i`: in the plane
/// of the run's polygon(s) at that point, across the edge, turned towards
/// `d` (the bridge's way across). Falls back to `d` itself.
fn edge_departure(mesh: &Mesh, run: &EdgeRun, i: usize, d: DVec3) -> DVec3 {
    let n = run.points.len();
    let spans = run.faces.len();
    // The one or two spans that meet at point i.
    let adjacent: Vec<usize> = if run.closed {
        vec![(i + n - 1) % n, i]
    } else {
        [i.checked_sub(1), (i < spans).then_some(i)]
            .into_iter()
            .flatten()
            .collect()
    };
    let mut sum = DVec3::ZERO;
    for s in adjacent {
        let (x, y) = (run.points[s], run.points[(s + 1) % n]);
        let edge = mesh.vertices[y as usize] - mesh.vertices[x as usize];
        let mut u = mesh.normal(run.faces[s]).cross(edge).normalize_or_zero();
        if u.dot(d) < 0. {
            u = -u;
        }
        sum += u;
    }
    if sum.length() < 1e-12 {
        d.normalize_or_zero()
    } else {
        sum.normalize()
    }
}

/// ME-T04 Bridge for edge selections: join two selected border runs (or
/// loops) with equal point counts by `segments` rows of quads (0 behaves
/// like 1). Mode, Tension and Twist (loops) apply as in polygon mode.
///
/// The bridge always runs against both borders' corner order, so its
/// polygons face the same way as the polygons it joins and the surface
/// stays one consistently wound piece. That leaves nothing for Reverse
/// Direction or Flip Polygons to change (either would give an invalid,
/// inside-out join), and Remove Polygons has nothing to remove, so all
/// three are ignored here and greyed in the panel (BRI-Q08, provisional).
/// When the two sides face opposite ways the only valid bridge would cross
/// itself, so it is refused with a reason instead.
///
/// Returns the mesh and the selected edges, which stay the source edges
/// (bridge-2026-11-05, BR16/BR17).
pub fn edge_bridge(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    p: crate::BridgeParameters,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    use crate::BridgeMode;
    mesh.validate()?;
    if p.segments > 256 {
        return Err("Bridge Segments must be 0–256 for edge bridges".into());
    }
    if !p.tension.is_finite() || p.twist == i32::MIN {
        return Err("Bridge requires finite Tension and normalized Twist".into());
    }
    let runs = edge_runs(mesh, edges)?;
    let [ra, rb] = &runs[..] else {
        return Err("Bridge needs two separate selected edge runs".into());
    };
    let (a, b) = (&ra.points, &rb.points);
    if a.len() != b.len() || ra.closed != rb.closed {
        return Err(
            "Bridge needs two edge runs with the same number of edges (both open or both closed)"
                .into(),
        );
    }
    let n = a.len();
    let p_of = |v: u32| mesh.vertices[v as usize];
    // Pair a_i with b at (shift - i) ("against": the reverse of B's own
    // order, which closes the surface when both are borders) or at
    // (shift + i) ("along").
    // "Along" is never built (it can't be wound validly); it is only
    // measured, to catch sides that face opposite ways.
    let partner = |against: bool, shift: usize, i: usize| {
        if against {
            b[(shift + n - i % n) % n]
        } else {
            b[(shift + i) % n]
        }
    };
    let cost = |against: bool, s: usize| -> f64 {
        (0..n)
            .map(|i| p_of(a[i]).distance(p_of(partner(against, s, i))))
            .sum()
    };
    let best = |against: bool| -> (f64, usize) {
        if ra.closed {
            (0..n)
                .map(|s| (cost(against, s), s))
                .min_by(|x, y| x.0.total_cmp(&y.0))
                .unwrap_or((0., 0))
        } else {
            let s = if against { n - 1 } else { 0 };
            (cost(against, s), s)
        }
    };
    let (against_cost, against_shift) = best(true);
    let (along_cost, _) = best(false);
    if along_cost < against_cost * (1. - 1e-6) {
        return Err(
            "Bridge would cross itself: the two sides' polygons face opposite ways. Flip one side's polygons, then bridge"
                .into(),
        );
    }
    let mut shift = against_shift;
    if ra.closed {
        shift = (shift as i64 + p.twist as i64).rem_euclid(n as i64) as usize;
    }
    let to = |i: usize| partner(true, shift, i);
    let rows = p.segments.max(1) as usize;
    // Curve leaves each run in one shared direction (its points' average
    // departure); Smooth leaves each point in its own (provisional,
    // BRI-Q07).
    let b_index: BTreeMap<u32, usize> = b.iter().enumerate().map(|(k, &v)| (v, k)).collect();
    let mut departures = Vec::with_capacity(n);
    let (mut curve_a, mut curve_b) = (DVec3::ZERO, DVec3::ZERO);
    for (i, &from) in a.iter().enumerate() {
        let d = p_of(to(i)) - p_of(from);
        let pair = (
            edge_departure(mesh, ra, i, d),
            edge_departure(mesh, rb, b_index[&to(i)], d),
        );
        curve_a += pair.0;
        curve_b += pair.1;
        departures.push(pair);
    }
    // Departures that cancel out (round a loop) give no shared direction.
    let average = |sum: DVec3| {
        if sum.length() > 1e-9 * n as f64 {
            sum.normalize()
        } else {
            DVec3::ZERO
        }
    };
    let (curve_a, curve_b) = (average(curve_a), average(curve_b));
    let mut vertices = mesh.vertices.clone();
    let mut grid: Vec<Vec<u32>> = Vec::with_capacity(n);
    // Provisional UVs (SQ-UV-01): each column blends from its A point's UV
    // to its B point's UV, each taken from that run's own polygons.
    let mut uv_grid: Vec<Vec<Option<CornerBlend>>> = Vec::with_capacity(n);
    for (i, &from) in a.iter().enumerate() {
        let target = to(i);
        let (x, y) = (p_of(from), p_of(target));
        let d = y - x;
        let (m0, m1) = match p.mode {
            BridgeMode::Linear => (d, d),
            BridgeMode::Curve | BridgeMode::Smooth => {
                let (u, v) = if p.mode == BridgeMode::Curve {
                    let flip = |u: DVec3| if u.dot(d) < 0. { -u } else { u };
                    (flip(curve_a), flip(curve_b))
                } else {
                    departures[i]
                };
                // A side whose polygons don't lead towards the other side
                // (a lid's rim, say, seen from straight below) leaves
                // straight across instead of bulging sideways.
                let across = d.normalize_or_zero();
                let lead = |w: DVec3| if w.dot(across) < 0.1 { across } else { w };
                hermite_derivatives(d, lead(u), lead(v))
            }
        };
        let (ua, ub) = (
            corner_among(mesh, &ra.faces, from, mesh.normal(ra.faces[0])),
            corner_among(mesh, &rb.faces, target, mesh.normal(rb.faces[0])),
        );
        let mut column = vec![from];
        let mut uv_column = vec![ua.map(|c| vec![(c, 1.)])];
        for r in 1..rows {
            let t = r as f64 / rows as f64;
            let (t2, t3) = (t * t, t * t * t);
            let point = x * (2. * t3 - 3. * t2 + 1.)
                + y * (-2. * t3 + 3. * t2)
                + p.tension * ((t3 - 2. * t2 + t) * m0 + (t3 - t2) * m1);
            if !point.is_finite() || point.abs().max_element() > 1e9 {
                return Err("Bridge candidate exceeds finite coordinate limits".into());
            }
            let id = u32::try_from(vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
            vertices.push(point);
            column.push(id);
            uv_column.push(ua.zip(ub).map(|(c0, c1)| vec![(c0, 1. - t), (c1, t)]));
        }
        column.push(target);
        uv_column.push(ub.map(|c| vec![(c, 1.)]));
        grid.push(column);
        uv_grid.push(uv_column);
    }
    let mut faces: Vec<OutFace> = (0..mesh.faces.len())
        .map(|f| OutFace::copy(mesh, f, mesh.faces[f].vertices.clone()))
        .collect();
    let spans = if ra.closed { n } else { n - 1 };
    for i in 0..spans {
        let j = (i + 1) % n;
        for r in 0..rows {
            let corners = [(j, r), (i, r), (i, r + 1), (j, r + 1)];
            faces.push(OutFace {
                template: ra.faces[i],
                vertices: corners.iter().map(|&(c, k)| grid[c][k]).collect(),
                donors: vec![None; 4],
                blends: corners
                    .iter()
                    .map(|&(c, k)| uv_grid[c][k].clone())
                    .collect(),
            });
        }
    }
    let out = rebuild(mesh, vertices, faces, &[], "Bridge")?;
    Ok((
        finish(out, "Bridge")?,
        edges.iter().map(|&(x, y)| edge_key(x, y)).collect(),
    ))
}

/// End tangents of a cubic Hermite span `d` long whose ends leave along
/// unit `u` and arrive along unit `v`, sized so the curve stays as even as
/// it can (the polygon Bridge's rule, `bridge.rs`).
fn hermite_derivatives(d: DVec3, u: DVec3, v: DVec3) -> (DVec3, DVec3) {
    let c = u.dot(v);
    let a = 3. * d.dot(u);
    let b = 3. * d.dot(v);
    let den = 4. - c * c;
    (u * ((2. * a - c * b) / den), v * ((2. * b - c * a) / den))
}

/// The constant UV shift from polygon `t` to a lower-numbered polygon
/// across one of its edges, when the two disagree there by the same
/// nonzero amount at both ends (a translated seam island); `None` when no
/// such neighbour exists. Grow Quads uses it to recover continuous UVs on
/// a mesh that keeps corner UVs only (UVN-T-06, provisional).
fn seam_shift(mesh: &Mesh, rows: &crate::corner_uv::UvRows, t: usize) -> Option<glam::DVec2> {
    let face = &mesh.faces.get(t)?.vertices;
    let point_faces = mesh.point_faces();
    let n = face.len();
    let uv = |f: usize, v: u32| crate::corner_uv::corner_uv(mesh, rows, f, v);
    let mut best: Option<(usize, glam::DVec2)> = None;
    for i in 0..n {
        let (a, b) = (face[i], face[(i + 1) % n]);
        for &other in point_faces.of(a) {
            let other = other as usize;
            if other >= t || !mesh.faces[other].vertices.contains(&b) {
                continue;
            }
            let (Some(ta), Some(tb), Some(oa), Some(ob)) =
                (uv(t, a), uv(t, b), uv(other, a), uv(other, b))
            else {
                continue;
            };
            let (sa, sb) = (oa - ta, ob - tb);
            if sa.distance(sb) <= 1e-9 && sa.length() > 1e-9 && best.is_none_or(|(f, _)| other < f)
            {
                best = Some((other, sa));
            }
        }
    }
    best.map(|(_, shift)| shift)
}
