//! Chamfer Edit (`specifications/chamfer-edit-2026-11-05`). The tool
//! reworks an existing chamfer or bevel: it changes its width (Offset) and
//! how many segments it has and how round it is. The user selects the
//! chamfer's strip polygons.
//!
//! Operandi edits chains of quad strips, including chains that wrap around
//! corners (such as the ring a Polygon Bevel leaves). Each strip has two
//! rails (edges on the outer polygons) and two cross edges (to the next strip
//! or an end polygon). Offset widens the chamfer by moving every rail point
//! into its outer polygons so the rails stay parallel:
//! * a point with one outside edge (a side edge of the outer polygons) slides
//!   along it;
//! * a point at an inside corner of one outer polygon (two rails in that
//!   polygon) moves along the corner bisector in its plane.
//!
//! Segments split every strip into rows; each cross edge gets one rounded
//! profile (the decided arc, rule 5 and Q-CE-01, blended with the straight
//! chord by Depth), shared by the strips that meet there.
//! Profiles are not built.
//! No external profile asset.
//!
//! UV maps (provisional, SQ-UV-01): existing corners keep their UVs; a new
//! profile point takes, in each polygon using it, the blend of its cross
//! edge's end UVs at its position along the edge.
use crate::corner_uv::{corner_uv, lerp, records_from_uvs};
use crate::topology::edge_key;
use crate::{Face, Mesh, MeshAttributes, PolygonRepresentation, Result};

/// A corner's UV: `t` of the way from point `a` to point `b` in its polygon.
type UvSource = (u32, u32, f64);
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChamferParameters {
    pub offset: f64,
    pub segments: u32,
    pub source_segments: u32,
    pub depth: f64,
    /// Mode Width (rule 3): Offset changes the chamfer's width, not its
    /// inset.
    pub width_mode: bool,
    /// Absolute Size (rule 4): Offset is the inset (or width) itself.
    pub absolute: bool,
    /// Swap Side (rule 7): the strip runs the other way; Offset moves its
    /// other two edges.
    pub swap_side: bool,
    /// Reverse Direction / Reverse Depth: flip the profile. Without
    /// profiles they change nothing (rule 8, CE19).
    pub reverse_x: bool,
    pub reverse_y: bool,
}
impl Default for ChamferParameters {
    fn default() -> Self {
        Self {
            offset: 0.,
            segments: 1,
            source_segments: 1,
            depth: 100.,
            width_mode: false,
            absolute: false,
            swap_side: false,
            reverse_x: false,
            reverse_y: false,
        }
    }
}

fn edge(face: &Face, a: u32, b: u32) -> bool {
    face.vertices.iter().enumerate().any(|(i, &x)| {
        let y = face.vertices[(i + 1) % face.vertices.len()];
        (x == a && y == b) || (x == b && y == a)
    })
}
fn outer(face: &Face, a: u32, b: u32) -> Option<u32> {
    let i = face.vertices.iter().position(|&v| v == a)?;
    let prev = face.vertices[(i + face.vertices.len() - 1) % face.vertices.len()];
    let next = face.vertices[(i + 1) % face.vertices.len()];
    if prev == b {
        Some(next)
    } else if next == b {
        Some(prev)
    } else {
        None
    }
}

/// The chamfer strips a selection refers to. The user may
/// select the two outer polygons on either side of a chamfer: every quad with
/// two opposite edges on selected polygons and its other two edges free of
/// them is a strip between them. When no such quad exists, the selected
/// polygons are taken as the strips themselves (selecting the chamfer faces
/// directly also works).
pub fn chamfer_strips(source: &Mesh, selected: &[u32]) -> Vec<u32> {
    resolve(source, selected).0
}

/// Strips plus, in the outer-polygon workflow, the selected outer polygons.
fn resolve(source: &Mesh, selected: &[u32]) -> (Vec<u32>, Option<BTreeSet<usize>>) {
    let chosen: BTreeSet<usize> = selected
        .iter()
        .map(|&f| f as usize)
        .filter(|&f| f < source.faces.len())
        .collect();
    let on_selected = |x: u32, y: u32| chosen.iter().any(|&g| edge(&source.faces[g], x, y));
    let between: Vec<u32> = (0..source.faces.len())
        .filter(|f| {
            let v = &source.faces[*f].vertices;
            !chosen.contains(f)
                && v.len() == 4
                && (0..2).any(|s| {
                    let e = |k: usize| (v[(s + k) % 4], v[(s + k + 1) % 4]);
                    on_selected(e(0).0, e(0).1)
                        && on_selected(e(2).0, e(2).1)
                        && !on_selected(e(1).0, e(1).1)
                        && !on_selected(e(3).0, e(3).1)
                })
        })
        .map(|f| f as u32)
        .collect();
    if between.is_empty() {
        (chosen.into_iter().map(|f| f as u32).collect(), None)
    } else {
        (between, Some(chosen))
    }
}

/// Rule 7 / Q-CE-03: `faces` (quads) as one chamfer running the other way.
/// Each polygon's two open ends (opposite edges no other polygon of the run
/// shares) are the borders: Offset pushes them outward along the run, and
/// Segments cuts the run into that many equal, straight slices across it;
/// Depth has no effect. A closed run (no open ends anywhere, a cube with
/// nothing selected) changes nothing.
fn run_geometry(source: &Mesh, faces: &[usize], p: ChamferParameters) -> Result<Mesh> {
    let set: BTreeSet<usize> = faces.iter().copied().collect();
    if set.is_empty() || set.iter().any(|&f| f >= source.faces.len()) {
        return Err("Chamfer Edit: select a run of polygons".into());
    }
    if set.iter().any(|&f| source.faces[f].vertices.len() != 4) {
        return Err("Chamfer Edit needs quad polygons in the run".into());
    }
    let mut uses: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (fi, f) in source.faces.iter().enumerate() {
        let n = f.vertices.len();
        for i in 0..n {
            uses.entry(edge_key(f.vertices[i], f.vertices[(i + 1) % n]))
                .or_default()
                .push(fi);
        }
    }
    let in_run = |f: usize, a: u32, b: u32| {
        uses.get(&edge_key(a, b))
            .is_some_and(|l| l.iter().any(|&g| g != f && set.contains(&g)))
    };
    let users = |a: u32, b: u32| uses.get(&edge_key(a, b)).map_or(0, Vec::len);
    // Each polygon as [a0, a1, b0, b1]: end (a0, a1), end (b0, b1), sides
    // a1→b0 and a0→b1.
    let mut rotated: BTreeMap<usize, [u32; 4]> = BTreeMap::new();
    for &f in &set {
        let v = &source.faces[f].vertices;
        let best = (0..2)
            .filter(|&s| !in_run(f, v[s], v[s + 1]) && !in_run(f, v[s + 2], v[(s + 3) % 4]))
            .min_by_key(|&s| users(v[s], v[s + 1]) + users(v[s + 2], v[(s + 3) % 4]));
        if let Some(s) = best {
            rotated.insert(f, [v[s], v[s + 1], v[s + 2], v[(s + 3) % 4]]);
        }
    }
    if rotated.is_empty() {
        return Ok(source.clone()); // closed all round: nothing to push or cut
    }
    if rotated.len() != set.len() {
        return Err("Chamfer Edit: the selection isn't one run of polygons".into());
    }
    if p.offset == 0. && p.segments == 1 {
        return Ok(source.clone());
    }
    let pos = &source.vertices;
    // Each end point's partner across its side.
    let mut partner: BTreeMap<u32, u32> = BTreeMap::new();
    for r in rotated.values() {
        for (x, y) in [(r[1], r[2]), (r[0], r[3])] {
            partner.insert(x, y);
            partner.insert(y, x);
        }
    }
    let mut out = source.clone();
    for (&x, &y) in &partner {
        let along = unit(pos[x as usize] - pos[y as usize], "run")?;
        out.vertices[x as usize] = pos[x as usize] + along * p.offset;
    }
    // Slice points along each side, from its lower point to its higher.
    let segments = p.segments.max(1);
    let mut cuts: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
    for r in rotated.values() {
        for (x, y) in [(r[1], r[2]), (r[0], r[3])] {
            let key = edge_key(x, y);
            if cuts.contains_key(&key) {
                continue;
            }
            let (lo, hi) = (out.vertices[key.0 as usize], out.vertices[key.1 as usize]);
            let mut ids = Vec::new();
            for k in 1..segments {
                let id =
                    u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
                out.vertices.push(lo.lerp(hi, k as f64 / segments as f64));
                ids.push(id);
            }
            cuts.insert(key, ids);
        }
    }
    let side = |from: u32, to: u32| -> Vec<u32> {
        let key = edge_key(from, to);
        let mut inner = cuts[&key].clone();
        if from != key.0 {
            inner.reverse();
        }
        let mut all = vec![from];
        all.extend(inner);
        all.push(to);
        all
    };
    let mut new_faces: Vec<Face> =
        Vec::with_capacity(out.faces.len() + set.len() * segments as usize);
    for (fi, face) in source.faces.iter().enumerate() {
        let Some(r) = rotated.get(&fi) else {
            new_faces.push(face.clone());
            continue;
        };
        let (s1, s2) = (side(r[1], r[2]), side(r[0], r[3]));
        for k in 0..segments as usize {
            let mut slice = face.clone();
            slice.vertices = vec![s2[k], s1[k], s1[k + 1], s2[k + 1]];
            new_faces.push(slice);
        }
    }
    out.faces = new_faces.into();
    out.attributes.representations.clear();
    Ok(out)
}

/// Rule 6: the selected chamfer of `count` strips (a ladder of quads
/// between two outer polygons) as one strip: the first strip's outer rail
/// joined to the last strip's, the rungs between them and the points only
/// they used gone. A selection that isn't `count` strips across is refused
/// (decided Q-CE-02). Returns the merged mesh and the strip's index.
fn merge_source_segments(source: &Mesh, selected: &[u32], count: u32) -> Result<(Mesh, u32)> {
    let chosen: BTreeSet<usize> = selected.iter().map(|&f| f as usize).collect();
    let mismatch = || {
        format!(
            "Chamfer Edit: Source Segments is {count}, but the selected chamfer isn't {count} strips across; nothing changed"
        )
    };
    if chosen.len() != count as usize
        || chosen
            .iter()
            .any(|&f| f >= source.faces.len() || source.faces[f].vertices.len() != 4)
    {
        return Err(mismatch());
    }
    let mut uses: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (fi, f) in source.faces.iter().enumerate() {
        let n = f.vertices.len();
        for i in 0..n {
            uses.entry(edge_key(f.vertices[i], f.vertices[(i + 1) % n]))
                .or_default()
                .push(fi);
        }
    }
    // An edge of strip `f` whose other polygon is an unselected quad: a rail.
    let outer = |f: usize, a: u32, b: u32| {
        uses.get(&edge_key(a, b)).is_some_and(|l| {
            l.iter()
                .any(|&g| g != f && !chosen.contains(&g) && source.faces[g].vertices.len() == 4)
        })
    };
    let shared = |f: usize, a: u32, b: u32| {
        uses.get(&edge_key(a, b))
            .and_then(|l| l.iter().copied().find(|&g| g != f && chosen.contains(&g)))
    };
    // The first strip: one rail to an outer polygon, the opposite edge to
    // the next strip.
    let mut start = None;
    for &f in &chosen {
        let v = &source.faces[f].vertices;
        for s in 0..4 {
            let r = [v[s], v[(s + 1) % 4], v[(s + 2) % 4], v[(s + 3) % 4]];
            if outer(f, r[0], r[1]) && shared(f, r[2], r[3]).is_some() {
                start = Some((f, r));
                break;
            }
        }
        if start.is_some() {
            break;
        }
    }
    let (first, rot) = start.ok_or_else(mismatch)?;
    let (mut f, mut c2, mut c3) = (first, rot[2], rot[3]);
    let mut walked = vec![first];
    while let Some(next) = shared(f, c2, c3) {
        let v = &source.faces[next].vertices;
        // The shared edge runs c3 → c2 in the next strip.
        let k = (0..4)
            .find(|&k| v[k] == c3 && v[(k + 1) % 4] == c2)
            .ok_or_else(mismatch)?;
        (f, c2, c3) = (next, v[(k + 2) % 4], v[(k + 3) % 4]);
        walked.push(next);
        if walked.len() > chosen.len() {
            return Err(mismatch());
        }
    }
    if walked.len() != chosen.len() || !outer(f, c2, c3) {
        return Err(mismatch());
    }
    let mut out = source.clone();
    let mut faces: Vec<Face> = Vec::with_capacity(out.faces.len());
    let mut strip = 0;
    for (fi, face) in source.faces.iter().enumerate() {
        if fi == first {
            strip = faces.len() as u32;
            let mut merged = face.clone();
            merged.vertices = vec![rot[0], rot[1], c2, c3];
            faces.push(merged);
        } else if !chosen.contains(&fi) {
            faces.push(face.clone());
        }
    }
    out.faces = faces.into();
    out.attributes.representations.clear();
    let was_used: BTreeSet<u32> = (0..source.vertices.len() as u32).collect();
    let cleaned = crate::mesh_cleanup::remove_unreferenced(&out, Some(&was_used))?;
    Ok((cleaned.mesh, strip))
}

fn unit(v: DVec3, what: &str) -> Result<DVec3> {
    if !v.is_finite() || v.length() < 1e-9 {
        Err(format!("Degenerate Chamfer {what}"))
    } else {
        Ok(v.normalize())
    }
}

/// Rebuild the selected chamfer (see the module notes). Unsupported
/// associated attributes and shapes are refused before any output exists.
pub fn chamfer_geometry(source: &Mesh, selected: &[u32], p: ChamferParameters) -> Result<Mesh> {
    source.validate()?;
    // CE12/CE14 (rule 6): a chamfer of several segments is first merged
    // into the one strip between its outer polygons, then rebuilt.
    if p.source_segments > 1 && p.source_segments <= 64 {
        let (merged, strip) = merge_source_segments(source, selected, p.source_segments)?;
        return chamfer_geometry(
            &merged,
            &[strip],
            ChamferParameters {
                source_segments: 1,
                ..p
            },
        );
    }
    if !p.offset.is_finite()
        || !p.depth.is_finite()
        || !(-1000. ..=1000.).contains(&p.depth)
        || !(1..=64).contains(&p.segments)
        || p.source_segments != 1
    {
        return Err("Chamfer Edit requires a finite Offset, 1–64 Segments, Source Segments 1 and Depth −1000–1000%".into());
    }
    let mut attrs = source.attributes.clone();
    if attrs
        .representations
        .iter()
        .any(|r| *r != PolygonRepresentation::Face)
    {
        return Err("Chamfer Edit supports Face polygons only".into());
    }
    attrs.representations.clear();
    // Smoothing labels and face tags (material, material label, part) are
    // per face: every output face keeps its source face's values.
    attrs.smoothing_labels.clear();
    attrs.uv_records.clear();
    if attrs != MeshAttributes::default() || source.faces.iter().any(|f| f.uv.is_some()) {
        return Err(
            "Chamfer Edit cannot yet remap vertex/corner maps or polygon sets; nothing changed"
                .into(),
        );
    }
    // Rule 7 / Q-CE-03 (answered 2026-11-20): with Swap Side, or with no
    // unselected polygon beside the selection (everything or nothing
    // selected), the selected run is a chamfer running the other way.
    if p.swap_side || selected.is_empty() {
        let faces: Vec<usize> = if selected.is_empty() {
            (0..source.faces.len()).collect()
        } else {
            selected.iter().map(|&f| f as usize).collect()
        };
        return run_geometry(source, &faces, p);
    }
    let (strips, outer_set) = resolve(source, selected);
    if strips.is_empty() {
        return Err("Chamfer Edit: select the two polygons on either side of a chamfer".into());
    }
    let strip_set: BTreeSet<usize> = strips.iter().map(|&f| f as usize).collect();
    if strip_set
        .iter()
        .any(|&f| source.faces[f].vertices.len() != 4)
    {
        return Err("Chamfer Edit needs quad chamfer strips".into());
    }
    if p.offset == 0. && p.segments == 1 {
        return Ok(source.clone());
    }
    let pos = &source.vertices;
    let mut uses: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (fi, f) in source.faces.iter().enumerate() {
        let n = f.vertices.len();
        for i in 0..n {
            uses.entry(edge_key(f.vertices[i], f.vertices[(i + 1) % n]))
                .or_default()
                .push(fi);
        }
    }
    let others = |f: usize, x: u32, y: u32| -> Vec<usize> {
        uses.get(&edge_key(x, y))
            .map(|l| l.iter().copied().filter(|&g| g != f).collect())
            .unwrap_or_default()
    };
    // Orient every strip: rot = [v0, v1, v2, v3] with rails (v0,v1), (v2,v3)
    // and cross edges (v1,v2), (v3,v0). A rail's outer polygon is recorded.
    let mut oriented: BTreeMap<usize, [u32; 4]> = BTreeMap::new();
    let mut rail_face: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for &f in &strip_set {
        let v = &source.faces[f].vertices;
        let mut candidates = Vec::new();
        for s in 0..2 {
            let rot = [v[s], v[(s + 1) % 4], v[(s + 2) % 4], v[(s + 3) % 4]];
            let rail = |x: u32, y: u32| -> Option<usize> {
                let o = others(f, x, y);
                match &outer_set {
                    Some(outer) => o.into_iter().find(|g| outer.contains(g)),
                    None => match o[..] {
                        [g] if !strip_set.contains(&g) && source.faces[g].vertices.len() == 4 => {
                            Some(g)
                        }
                        _ => None,
                    },
                }
            };
            if let (Some(l), Some(r)) = (rail(rot[0], rot[1]), rail(rot[2], rot[3])) {
                let chained = [(rot[1], rot[2]), (rot[3], rot[0])]
                    .iter()
                    .any(|&(x, y)| others(f, x, y).iter().any(|g| strip_set.contains(g)));
                candidates.push((rot, l, r, chained));
            }
        }
        if candidates.len() == 2 {
            candidates.retain(|c| c.3);
        }
        let [(rot, l, r, _)] = candidates[..] else {
            const AMBIGUOUS: &str =
                "Chamfer Edit needs strips between two unambiguous outer polygons";
            // Q-CE-03: no unselected polygon beside the run on both sides
            // (all selected, or a run that ends on the mesh's border). A
            // strip between two outer polygons stays a chamfer, refused
            // here (a Source Segments mismatch, CE13).
            let beside: BTreeSet<usize> = strip_set
                .iter()
                .flat_map(|&f| {
                    let v = &source.faces[f].vertices;
                    (0..4).flat_map(move |i| others(f, v[i], v[(i + 1) % 4]))
                })
                .filter(|g| !strip_set.contains(g))
                .collect();
            if outer_set.is_none() && beside.len() < 2 {
                let faces: Vec<usize> = selected.iter().map(|&f| f as usize).collect();
                return run_geometry(source, &faces, p).map_err(|_| AMBIGUOUS.to_string());
            }
            return Err(AMBIGUOUS.into());
        };
        oriented.insert(f, rot);
        for (x, y, g) in [(rot[0], rot[1], l), (rot[2], rot[3], r)] {
            if rail_face.insert(edge_key(x, y), g).is_some() {
                return Err("Chamfer Edit strips overlap".into());
            }
        }
    }
    let cross: BTreeSet<(u32, u32)> = oriented
        .values()
        .flat_map(|r| [edge_key(r[1], r[2]), edge_key(r[3], r[0])])
        .collect();
    if cross.iter().any(|e| rail_face.contains_key(e)) {
        return Err("Chamfer Edit strips overlap".into());
    }
    // Rails at each point: (outer polygon, other rail end).
    let mut rails_at: BTreeMap<u32, Vec<(usize, u32)>> = BTreeMap::new();
    for (&(x, y), &g) in &rail_face {
        rails_at.entry(x).or_default().push((g, y));
        rails_at.entry(y).or_default().push((g, x));
    }
    // Displacement per unit Offset, and the profile direction, per point.
    let mut motion: BTreeMap<u32, (DVec3, DVec3)> = BTreeMap::new();
    // How far each side point may travel before it reaches the side's end.
    let mut reach: BTreeMap<u32, f64> = BTreeMap::new();
    for (&v, rails) in &rails_at {
        let incident = |allowed: &dyn Fn(&Face) -> bool| -> Result<()> {
            for (fi, f) in source.faces.iter().enumerate() {
                if !strip_set.contains(&fi) && f.vertices.contains(&v) && !allowed(f) {
                    return Err(
                        "Chamfer Edit cannot remap additional incident polygons".to_string()
                    );
                }
            }
            Ok(())
        };
        let corner = rails
            .iter()
            .find(|(g, _)| rails.iter().filter(|(h, _)| h == g).count() > 1);
        if let Some(&(g, _)) = corner {
            if rails.len() != 2 {
                return Err("Chamfer Edit: unsupported chamfer corner".into());
            }
            // Inside corner of outer polygon g: offset both rails in its plane.
            let face = &source.faces[g];
            let normal = source.normal(g);
            let mut normals = Vec::new();
            for &(_, w) in rails {
                let i = face.vertices.iter().position(|&x| x == v).unwrap_or(0);
                let next = face.vertices[(i + 1) % face.vertices.len()];
                let (from, to) = if next == w { (v, w) } else { (w, v) };
                normals.push(unit(
                    normal.cross(pos[to as usize] - pos[from as usize]),
                    "corner",
                )?);
            }
            let den = 1. + normals[0].dot(normals[1]);
            if den < 1e-6 {
                return Err("Chamfer Edit: unsupported chamfer corner".into());
            }
            let m = (normals[0] + normals[1]) / den;
            motion.insert(v, (m, unit(m, "corner")?));
            incident(&|f: &Face| std::ptr::eq(f, face))?;
        } else {
            let mut outside = None;
            for &(g, w) in rails {
                let o = outer(&source.faces[g], v, w).ok_or("Invalid Chamfer outer polygon")?;
                if outside.is_some_and(|x| x != o) {
                    return Err("Chamfer Edit: unsupported chamfer corner".into());
                }
                outside = Some(o);
            }
            let o = outside.ok_or("Invalid Chamfer outer polygon")?;
            let dir = unit(pos[o as usize] - pos[v as usize], "side")?;
            let along = unit(pos[rails[0].1 as usize] - pos[v as usize], "rail")?;
            let sine = dir.cross(along).length();
            if sine < 1e-6 {
                return Err("Degenerate Chamfer side".into());
            }
            let m = dir / sine;
            reach.insert(v, pos[o as usize].distance(pos[v as usize]));
            motion.insert(v, (m, dir));
            let rail_faces: Vec<&Face> = rails.iter().map(|&(g, _)| &source.faces[g]).collect();
            incident(&|f: &Face| rail_faces.iter().any(|r| std::ptr::eq(*r, f)) || edge(f, v, o))?;
        }
    }
    // Each point's Offset (rules 2-4): the same for all in Inset mode; in
    // Width mode and with Absolute Size worked out per cross edge from the
    // virtual sharp corner. Where the two insets differ, Absolute Size sets
    // both to Offset and Width mode scales both by the same factor
    // (Operandi's reading of rules 3 and 4; the probes only had equal insets).
    let cross_edges: BTreeSet<(u32, u32)> = oriented
        .values()
        .flat_map(|r| [edge_key(r[1], r[2]), edge_key(r[3], r[0])])
        .collect();
    let mut amount: BTreeMap<u32, f64> = motion.keys().map(|&v| (v, p.offset)).collect();
    if p.width_mode || p.absolute {
        for &(x, y) in &cross_edges {
            let (Some(&(mx, dx)), Some(&(my, dy))) = (motion.get(&x), motion.get(&y)) else {
                continue;
            };
            let (p0, p1) = (pos[x as usize], pos[y as usize]);
            let (d0, d1) = (-dx, -dy);
            let r = p0 - p1;
            let b = d0.dot(d1);
            let den = 1. - b * b;
            if den <= 1e-9 {
                continue; // a flat strip: no corner, Offset as given
            }
            // Distances along each face from its border to the corner.
            let t = (b * d1.dot(r) - d0.dot(r)) / den;
            let s = (d1.dot(r) - b * d0.dot(r)) / den;
            let width = r.length();
            let (sx, sy) = (1. / mx.length(), 1. / my.length());
            let (ax, ay) = if p.width_mode {
                let target = if p.absolute {
                    p.offset
                } else {
                    width + p.offset
                };
                let k = target / width;
                (t * (k - 1.) * sx, s * (k - 1.) * sy)
            } else {
                ((p.offset - t) * sx, (p.offset - s) * sy)
            };
            amount.insert(x, ax);
            amount.insert(y, ay);
        }
    }
    for (&v, &(m, _)) in &motion {
        if let Some(&limit) = reach.get(&v)
            && amount[&v] * m.length() >= limit - 1e-9
        {
            return Err("Chamfer Offset reaches or crosses a side boundary".into());
        }
    }
    let mut out = source.clone();
    for (&v, &(m, _)) in &motion {
        out.vertices[v as usize] += m * amount[&v];
    }
    // Rule 4 (CE08): Absolute Size 0 closes the chamfer into a sharp corner:
    // the strips go and their borders meet.
    if p.absolute
        && !cross_edges.is_empty()
        && cross_edges
            .iter()
            .all(|&(x, y)| out.vertices[x as usize].distance(out.vertices[y as usize]) < 1e-9)
    {
        let mut into: BTreeMap<u32, u32> = BTreeMap::new();
        for &(x, y) in &cross_edges {
            into.insert(y.max(x), y.min(x));
        }
        let faces: Vec<Face> = out
            .faces
            .iter()
            .enumerate()
            .filter(|(fi, _)| !strip_set.contains(fi))
            .map(|(_, f)| {
                let mut f = f.clone();
                for v in &mut f.vertices {
                    *v = *into.get(v).unwrap_or(v);
                }
                f
            })
            .collect();
        out.faces = faces.into();
        let was_used: BTreeSet<u32> = (0..source.vertices.len() as u32).collect();
        return Ok(crate::mesh_cleanup::remove_unreferenced(&out, Some(&was_used))?.mesh);
    }
    // Outer polygons must not fold over, and no rail may shrink through zero
    // (an inset cap whose corners cross keeps its normal).
    let folded = rail_face
        .values()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .any(|&g| out.normal(g).dot(source.normal(g)) <= 1e-9)
        || rail_face.keys().any(|&(x, y)| {
            let before = pos[y as usize] - pos[x as usize];
            let after = out.vertices[y as usize] - out.vertices[x as usize];
            after.dot(before) <= 1e-9 * before.length_squared()
        });
    if folded {
        return Err("Chamfer Offset reaches or crosses a side boundary".into());
    }
    // One profile per cross edge, interior points ordered from low to high id.
    let mut profile: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
    for &(x, y) in &cross {
        let (p0, p1) = (out.vertices[x as usize], out.vertices[y as usize]);
        let (d0, d1) = (-motion[&x].1, -motion[&y].1);
        // Closest approach of the two outer directions: the original corner.
        let r = p0 - p1;
        let b = d0.dot(d1);
        let den = 1. - b * b;
        // The round profile (chamfer-edit-2026-11-05, decided Q-CE-01): points
        // evenly spaced in angle on the circular arc tangent to both faces at
        // the chamfer's borders; Depth blends from the evenly spaced chord
        // (0 %) to the arc (100 %), and mirrors it inward below 0 %.
        //
        // Where the borders aren't the same distance from the sharp corner no
        // such arc exists. Operandi's own rule (not observed, Q-CE-06): build
        // the decided arc for equal distances of 1 and stretch it along each
        // face to that border's distance. The result still meets each face
        // tangentially at its border, and with equal distances it is exactly
        // the decided arc.
        struct Curve {
            corner: DVec3,
            u: DVec3,
            w: DVec3,
            lengths: (f64, f64),
            center: DVec3,
            axis: DVec3,
            angle: f64,
        }
        let curve = if den > 1e-9 && p.segments > 1 {
            let t = (b * d1.dot(r) - d0.dot(r)) / den;
            let s = (d1.dot(r) - b * d0.dot(r)) / den;
            let corner = (p0 + d0 * t + p1 + d1 * s) * 0.5;
            match (unit(p0 - corner, "profile"), unit(p1 - corner, "profile")) {
                (Ok(u), Ok(w)) => {
                    let lengths = ((p0 - corner).length(), (p1 - corner).length());
                    let axis = u.cross(w).normalize_or_zero();
                    if axis == DVec3::ZERO {
                        None
                    } else {
                        // The decided arc for distances of 1: from corner + u
                        // to corner + w, centre off corner + u at right angles
                        // to its face, inward.
                        let (q0, q1) = (corner + u, corner + w);
                        let inward = axis.cross(u).normalize();
                        let inward = if inward.dot(q1 - q0) < 0. {
                            -inward
                        } else {
                            inward
                        };
                        let half = u.angle_between(w) / 2.;
                        let center = q0 + inward * half.tan();
                        let angle = (q0 - center).angle_between(q1 - center);
                        let sign = if (q0 - center).cross(q1 - center).dot(axis) < 0. {
                            -1.
                        } else {
                            1.
                        };
                        Some(Curve {
                            corner,
                            u,
                            w,
                            lengths,
                            center,
                            axis: axis * sign,
                            angle,
                        })
                    }
                }
                _ => None,
            }
        } else {
            None
        };
        // No sharp corner between the faces (a flat strip, CE18): the
        // segments stay on the chord.
        let mut ids = Vec::new();
        for j in 1..p.segments {
            let t = j as f64 / p.segments as f64;
            let chord = p0.lerp(p1, t);
            let point = match &curve {
                Some(c) => {
                    // The unit arc's point, written as a u + b w from the
                    // corner, then stretched to the real distances.
                    let turn = glam::DQuat::from_axis_angle(c.axis, c.angle * t);
                    let v = c.center + turn * (c.corner + c.u - c.center) - c.corner;
                    let g = c.u.dot(c.w);
                    let k = 1. - g * g;
                    let along_u = (v.dot(c.u) - g * v.dot(c.w)) / k;
                    let along_w = (v.dot(c.w) - g * v.dot(c.u)) / k;
                    let arc =
                        c.corner + c.u * (along_u * c.lengths.0) + c.w * (along_w * c.lengths.1);
                    chord.lerp(arc, p.depth / 100.)
                }
                None => chord,
            };
            ids.push(u32::try_from(out.vertices.len()).map_err(|_| "Chamfer mesh too large")?);
            out.vertices.push(point);
        }
        profile.insert((x, y), ids);
    }
    // Interior profile points from `x` toward `y`.
    let between = |x: u32, y: u32| -> Vec<u32> {
        let mut ids = profile.get(&edge_key(x, y)).cloned().unwrap_or_default();
        if x > y {
            ids.reverse();
        }
        ids
    };
    let mut faces = Vec::new();
    let mut repr = Vec::new();
    let mut smoothing = Vec::new();
    // Per output face: its source face and, per corner, the UV source as
    // `(a, b, t)`: `t` of the way from point a to point b in that face.
    let mut uv_sources: Vec<(usize, Vec<UvSource>)> = Vec::new();
    let segments = f64::from(p.segments);
    for (index, old) in source.faces.iter().enumerate() {
        let mut keep = |f: Face, corners: Vec<UvSource>| {
            uv_sources.push((index, corners));
            faces.push(f);
            if !source.attributes.representations.is_empty() {
                repr.push(PolygonRepresentation::Face);
            }
            if !source.attributes.smoothing_labels.is_empty() {
                smoothing.push(source.attributes.smoothing_labels[index].clone());
            }
        };
        if let Some(rot) = oriented.get(&index) {
            let left = between(rot[0], rot[3]);
            let right = between(rot[1], rot[2]);
            let mut rows = vec![(rot[0], rot[1])];
            rows.extend(left.into_iter().zip(right));
            rows.push((rot[3], rot[2]));
            // Row k sits k/segments of the way from the first rail to the second.
            let side = |k: usize| {
                let t = k as f64 / segments;
                ((rot[0], rot[3], t), (rot[1], rot[2], t))
            };
            for (k, pair) in rows.windows(2).enumerate() {
                let mut f = old.clone();
                f.vertices = vec![pair[0].0, pair[0].1, pair[1].1, pair[1].0];
                let ((l0, r0), (l1, r1)) = (side(k), side(k + 1));
                keep(f, vec![l0, r0, r1, l1]);
            }
        } else {
            let mut f = old.clone();
            let n = old.vertices.len();
            let mut corners = Vec::new();
            f.vertices = (0..n)
                .flat_map(|i| {
                    let (x, y) = (old.vertices[i], old.vertices[(i + 1) % n]);
                    corners.push((x, x, 0.));
                    let inserted = if cross.contains(&edge_key(x, y)) {
                        between(x, y)
                    } else {
                        Vec::new()
                    };
                    for j in 1..=inserted.len() {
                        corners.push((x, y, j as f64 / segments));
                    }
                    std::iter::once(x).chain(inserted)
                })
                .collect();
            keep(f, corners);
        }
    }
    out.faces = faces.into();
    for (name, map) in out.uv_maps.iter_mut() {
        let rows = &source.uv_maps[name];
        *map = uv_sources
            .iter()
            .map(|(face, corners)| {
                corners
                    .iter()
                    .map(|&(a, b, t)| {
                        lerp(
                            corner_uv(source, rows, *face, a),
                            corner_uv(source, rows, *face, b),
                            t,
                        )
                    })
                    .collect()
            })
            .collect();
    }
    for (name, records) in out.attributes.uv_records.iter_mut() {
        *records = records_from_uvs(&out.faces, &out.uv_maps[name]);
    }
    out.attributes.representations = repr;
    out.attributes.smoothing_labels = smoothing;
    out.validate()
        .map_err(|e| format!("Chamfer Edit produced an invalid mesh ({e}); nothing changed"))?;
    Ok(out)
}
