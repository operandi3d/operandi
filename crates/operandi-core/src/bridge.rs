//! BRI-R01..06 Bridge. Bridge spans the area between the
//! first selected polygon (or cluster of polygons) and the second. Each
//! cluster is an edge-connected group of selected polygons with one boundary
//! loop; the two loops need the same number of edges (pairing unequal loops
//! is not specified yet). Sections may be nonplanar, concave and face any
//! direction. Maps on generated corners are left unassigned. Pure candidate
//! evaluation; no scene/history/UI ownership.
use crate::topology::edge_key;
use crate::*;
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BridgeMode {
    Linear,
    #[default]
    Curve,
    Smooth,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BridgeParameters {
    pub segments: u32,
    pub twist: i32,
    pub mode: BridgeMode,
    pub remove: bool,
    pub flip: bool,
    pub reverse: bool,
    pub tension: f64,
}
impl Default for BridgeParameters {
    fn default() -> Self {
        Self {
            segments: 0,
            twist: 0,
            mode: BridgeMode::Curve,
            remove: true,
            flip: false,
            reverse: false,
            tension: 1.,
        }
    }
}
impl BridgeParameters {
    pub fn segments_field(v: f64) -> Result<u32> {
        if !v.is_finite() || v > u32::MAX as f64 {
            return Err("Bridge Segments expression overflows the integer field".into());
        }
        Ok(v.trunc().max(0.) as u32)
    }
    pub fn twist_field(v: f64) -> Result<i32> {
        if !v.is_finite() {
            return Err("Bridge Twist must be finite".into());
        }
        Ok(v.trunc().clamp(-2147483647., 2147483647.) as i32)
    }
}
#[derive(Clone, Debug)]
pub struct BridgeResult {
    pub mesh: Mesh,
    pub selected: Vec<u32>,
}
type EdgeUses = BTreeMap<(u32, u32), Vec<(usize, u32, u32)>>;
struct Section {
    cycle: Vec<u32>,
    normal: DVec3,
    outside: BTreeMap<(u32, u32), usize>,
    donor: usize,
}
/// Newell normal of a polygon set (normalized), or an error when it vanishes.
fn cluster_normal(m: &Mesh, faces: &[usize]) -> Result<DVec3> {
    let mut n = DVec3::ZERO;
    for &fi in faces {
        let vs = &m.faces[fi].vertices;
        for i in 0..vs.len() {
            n += m.vertices[vs[i] as usize].cross(m.vertices[vs[(i + 1) % vs.len()] as usize]);
        }
    }
    let n = n.normalize_or_zero();
    if n == DVec3::ZERO {
        return Err("Bridge requires sections with a nonzero area".into());
    }
    Ok(n)
}
/// The boundary loop of one selected cluster, oriented the way its outside
/// neighbours traverse it (or the cluster's own winding when it has none).
fn section(m: &Mesh, cluster: &[usize], edges: &EdgeUses) -> Result<Section> {
    let normal = cluster_normal(m, cluster)?;
    let inside: BTreeSet<usize> = cluster.iter().copied().collect();
    let mut next: BTreeMap<u32, u32> = BTreeMap::new();
    let mut outside = BTreeMap::new();
    for &fi in cluster {
        let f = &m.faces[fi];
        for i in 0..f.vertices.len() {
            let a = f.vertices[i];
            let b = f.vertices[(i + 1) % f.vertices.len()];
            let uses = &edges[&edge_key(a, b)];
            if uses.len() > 2 {
                return Err("Bridge excludes nonmanifold section boundaries".into());
            }
            if uses.iter().any(|&(o, _, _)| o != fi && inside.contains(&o)) {
                continue;
            }
            for &(other, x, y) in uses {
                if other == fi {
                    continue;
                }
                if x != b || y != a {
                    return Err("Bridge requires consistently wound neighbors".into());
                }
                outside.insert(edge_key(a, b), other);
            }
            if next.insert(a, b).is_some() {
                return Err("Bridge sections need one simple boundary loop".into());
            }
        }
    }
    let start = *next
        .keys()
        .next()
        .ok_or("Bridge sections need a boundary")?;
    let mut cycle = vec![start];
    let mut v = next[&start];
    while v != start {
        if cycle.len() > next.len() {
            return Err("Bridge sections need one simple boundary loop".into());
        }
        cycle.push(v);
        v = *next
            .get(&v)
            .ok_or("Bridge sections need one simple boundary loop")?;
    }
    if cycle.len() != next.len() || cycle.len() < 3 {
        return Err("Bridge sections need one simple boundary loop (no holes)".into());
    }
    let donor = outside.values().next().copied().unwrap_or(cluster[0]);
    if !outside.is_empty() {
        cycle.reverse();
    }
    Ok(Section {
        cycle,
        normal,
        outside,
        donor,
    })
}
/// Edge-connected clusters of the selection, in order of first selection.
fn clusters(m: &Mesh, selected: &[u32], edges: &EdgeUses) -> Vec<Vec<usize>> {
    let chosen: BTreeSet<usize> = selected.iter().map(|&f| f as usize).collect();
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for &f in selected {
        let f = f as usize;
        if !seen.insert(f) {
            continue;
        }
        let mut group = vec![f];
        let mut stack = vec![f];
        while let Some(g) = stack.pop() {
            let vs = &m.faces[g].vertices;
            for i in 0..vs.len() {
                for &(h, _, _) in &edges[&edge_key(vs[i], vs[(i + 1) % vs.len()])] {
                    if chosen.contains(&h) && seen.insert(h) {
                        group.push(h);
                        stack.push(h);
                    }
                }
            }
        }
        out.push(group);
    }
    out
}
fn departure(m: &Mesh, s: &Section, v: u32, d: DVec3) -> Result<DVec3> {
    let i = s
        .cycle
        .iter()
        .position(|&x| x == v)
        .ok_or("Missing Bridge corner")?;
    let mut sum = DVec3::ZERO;
    for other in [
        s.cycle[(i + s.cycle.len() - 1) % s.cycle.len()],
        s.cycle[(i + 1) % s.cycle.len()],
    ] {
        let fi = s
            .outside
            .get(&edge_key(v, other))
            .copied()
            .unwrap_or(s.donor);
        let edge = (m.vertices[other as usize] - m.vertices[v as usize]).normalize();
        let mut u = m.normal(fi).cross(edge).normalize_or_zero();
        if u.dot(m.center(fi) - m.vertices[v as usize]) < 0. {
            u = -u;
        }
        if u.dot(d) < 0. {
            u = -u;
        }
        sum += u;
    }
    if sum.length() < 1e-12 {
        return Err("Bridge can't join sections whose Smooth directions cancel out".into());
    }
    Ok(sum.normalize())
}
fn derivatives(d: DVec3, u: DVec3, v: DVec3) -> (DVec3, DVec3) {
    let c = u.dot(v);
    let a = 3. * d.dot(u);
    let b = 3. * d.dot(v);
    let den = 4. - c * c;
    (u * ((2. * a - c * b) / den), v * ((2. * b - c * a) / den))
}
/// Retain surviving face records and append absent generated corners. Point
/// records/definitions remain untouched. No map interpolation or zero filling.
fn remap_corners<T: Clone>(
    maps: &mut BTreeMap<String, Vec<Vec<Option<T>>>>,
    donors: &[Option<usize>],
    faces: &[Face],
) {
    for map in maps.values_mut() {
        let old = map.clone();
        *map = donors
            .iter()
            .zip(faces)
            .map(|(d, f)| {
                d.map(|i| old[i].clone())
                    .unwrap_or_else(|| vec![None; f.vertices.len()])
            })
            .collect();
    }
}
pub fn bridge_geometry(
    source: &Mesh,
    selected: &[u32],
    p: BridgeParameters,
) -> Result<BridgeResult> {
    source.validate()?;
    source.require_surface_elements()?;
    if selected.iter().any(|&i| i as usize >= source.faces.len()) {
        return Err("Bridge selection names a missing polygon".into());
    }
    if !p.tension.is_finite() || p.twist == i32::MIN {
        return Err("Bridge requires finite Tension and normalized Twist".into());
    }
    let mut edges: EdgeUses = BTreeMap::new();
    for (fi, f) in source.faces.iter().enumerate() {
        for i in 0..f.vertices.len() {
            let a = f.vertices[i];
            let b = f.vertices[(i + 1) % f.vertices.len()];
            edges.entry(edge_key(a, b)).or_default().push((fi, a, b));
        }
    }
    // Nothing selected, or a single polygon: nothing happens, no message
    // (bridge-2026-11-05, BR18/BR19).
    if selected.len() < 2 {
        return Ok(BridgeResult {
            mesh: source.clone(),
            selected: selected.to_vec(),
        });
    }
    let groups = clusters(source, selected, &edges);
    let [first, second] = &groups[..] else {
        return Err(
            "Bridge needs two separate selections (polygons or clusters that share no edge)".into(),
        );
    };
    let a = section(source, first, &edges)?;
    let b = section(source, second, &edges)?;
    let n = a.cycle.len();
    if a.cycle.iter().any(|v| b.cycle.contains(v)) {
        return Err("Bridge sections cannot share vertices".into());
    }
    if n != b.cycle.len() {
        return unequal(source, selected, &a, &b, p);
    }
    let s = p.segments.max(1) as usize;
    if n.checked_mul(n).is_none_or(|v| v > 4_000_000)
        || n.checked_mul(s).is_none_or(|v| v > 100_000)
        || source
            .vertices
            .len()
            .saturating_add(n.saturating_mul(s - 1))
            > 5_000_000
        || source.faces.len().saturating_add(n.saturating_mul(s)) > 5_000_000
    {
        return Err("Bridge implementation budget: 100,000 generated quads, 4,000,000 phase comparisons and 5,000,000 mesh elements".into());
    }
    let map_count = source.uv_maps.len()
        + source.attributes.uv_records.len()
        + source.attributes.corner_normals.len()
        + source.attributes.corner_weights.len()
        + source.attributes.corner_colors.len()
        + source.attributes.typed_maps.len();
    if n.saturating_mul(s)
        .saturating_mul(4)
        .saturating_mul(map_count)
        > 8_000_000
    {
        return Err("Bridge implementation budget: 8,000,000 generated corner-map slots".into());
    }
    let mut target = b.cycle.clone();
    if !p.reverse {
        target.reverse();
    }
    let mut costs: Vec<_> = (0..n)
        .map(|phase| {
            (
                (0..n)
                    .map(|i| {
                        source.vertices[a.cycle[i] as usize]
                            .distance(source.vertices[target[(i + phase) % n] as usize])
                    })
                    .sum::<f64>(),
                phase,
            )
        })
        .collect();
    costs.sort_by(|a, b| a.0.total_cmp(&b.0));
    if (costs[1].0 - costs[0].0).abs() <= costs[0].0.max(1.) * 1e-12 {
        return Err(
            "Bridge can't decide how to match the edges: two matchings are equally good".into(),
        );
    }
    let twist = (p.twist as u32 as usize) % n;
    let phase = (costs[0].1 + twist) % n;
    target.rotate_left(phase);
    let mut out = source.clone();
    let mut rings = vec![a.cycle.clone()];
    let mut tangents = Vec::with_capacity(n);
    for (&av, &bv) in a.cycle.iter().zip(&target) {
        let d = source.vertices[bv as usize] - source.vertices[av as usize];
        tangents.push(match p.mode {
            BridgeMode::Linear => (d, d),
            // Sections whose normals are both perpendicular to the span (for
            // example two holes in one plane) give no curve tangent; arc out
            // of the first along its normal and into the second instead.
            BridgeMode::Curve
                if d.dot(a.normal).abs() <= 1e-9 * d.length()
                    && d.dot(b.normal).abs() <= 1e-9 * d.length() =>
            {
                (a.normal * d.length(), -b.normal * d.length())
            }
            BridgeMode::Curve => derivatives(d, a.normal, b.normal),
            BridgeMode::Smooth => {
                let u = departure(source, &a, av, d)?;
                let v = departure(source, &b, bv, d)?;
                if d.dot(u).abs() <= d.length() * 1e-14 && d.dot(v).abs() <= d.length() * 1e-14 {
                    (d, d)
                } else {
                    derivatives(d, u, v)
                }
            }
        });
    }
    for k in 1..s {
        let t = k as f64 / s as f64;
        let t2 = t * t;
        let t3 = t2 * t;
        let mut ring = Vec::with_capacity(n);
        for i in 0..n {
            let x = source.vertices[a.cycle[i] as usize];
            let y = source.vertices[target[i] as usize];
            let (m0, m1) = tangents[i];
            let point = x * (2. * t3 - 3. * t2 + 1.)
                + y * (-2. * t3 + 3. * t2)
                + p.tension * ((t3 - 2. * t2 + t) * m0 + (t3 - t2) * m1);
            if !point.is_finite() || point.abs().max_element() > 1e9 {
                return Err("Bridge candidate exceeds finite coordinate limits".into());
            }
            ring.push(out.vertices.len() as u32);
            out.vertices.push(point);
        }
        rings.push(ring);
    }
    rings.push(target);
    out.faces.clear();
    let attr = &source.attributes;
    let mut donors = Vec::new();
    let mut ownership = Vec::new();
    let mut remap = vec![None; source.faces.len()];
    for (fi, f) in source.faces.iter().enumerate() {
        if p.remove && selected.contains(&(fi as u32)) {
            continue;
        }
        remap[fi] = Some(out.faces.len() as u32);
        out.faces.push(f.clone());
        donors.push(Some(fi));
        ownership.push(fi);
    }
    for pair in rings.windows(2) {
        for i in 0..n {
            let j = (i + 1) % n;
            let mut f = source.faces[a.donor].clone();
            f.vertices = vec![pair[0][i], pair[1][i], pair[1][j], pair[0][j]];
            f.uv = None;
            if p.flip {
                f.vertices.reverse();
            }
            out.faces.push(f);
            donors.push(None);
            ownership.push(a.donor);
        }
    }
    remap_corners(&mut out.uv_maps, &donors, &out.faces);
    let at = &mut out.attributes;
    remap_corners(&mut at.uv_records, &donors, &out.faces);
    remap_corners(&mut at.corner_normals, &donors, &out.faces);
    remap_corners(&mut at.corner_weights, &donors, &out.faces);
    remap_corners(&mut at.corner_colors, &donors, &out.faces);
    for m in at.typed_maps.values_mut() {
        if !m.corners.is_empty() {
            let old = m.corners.clone();
            m.corners = donors
                .iter()
                .zip(&out.faces)
                .map(|(d, f)| {
                    d.map(|i| old[i].clone())
                        .unwrap_or_else(|| vec![None; f.vertices.len()])
                })
                .collect();
        }
    }
    for set in at.polygon_sets.values_mut() {
        *set = set.iter().filter_map(|&f| remap[f as usize]).collect();
    }
    at.degenerate_faces = attr
        .degenerate_faces
        .iter()
        .filter_map(|&f| remap[f as usize])
        .collect();
    if !attr.representations.is_empty() {
        at.representations = ownership.iter().map(|&i| attr.representations[i]).collect();
    }
    if !attr.smoothing_labels.is_empty() {
        at.smoothing_labels = ownership
            .iter()
            .map(|&i| attr.smoothing_labels[i].clone())
            .collect();
    }
    // R05 explicitly permits retained caps/Flip/Reverse to produce exceptional
    // boundary incidences. Reuse the versioned exact-incidence integrity ledger
    // (historically named extrude_edge_uses), never a global validation bypass.
    at.extrude_edge_uses.clear();
    let mut uses: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (fi, f) in out.faces.iter().enumerate() {
        for i in 0..f.vertices.len() {
            let x = f.vertices[i];
            let y = f.vertices[(i + 1) % f.vertices.len()];
            uses.entry(edge_key(x, y))
                .or_default()
                .push([fi as u32, x, y]);
        }
    }
    for ((x, y), u) in uses {
        if u.len() > 2 || (u.len() == 2 && u[0][1..] == u[1][1..]) {
            at.extrude_edge_uses.insert(format!("{x}:{y}"), u);
        }
    }
    out.validate()?;
    Ok(BridgeResult {
        mesh: out,
        selected: if p.remove {
            vec![]
        } else {
            selected.iter().filter_map(|&f| remap[f as usize]).collect()
        },
    })
}

/// Loops with different point counts (bridge-2026-11-05, BR15: a quad to a
/// triangle). Provisional (Q-BR-02, Q-BR-06): each point of the larger loop
/// pairs with the nearest point of the smaller; the pairing must run once
/// around both loops in order. An edge whose two ends pair with the same
/// point gets a triangle, otherwise a quad. One row only (Segments above 1
/// isn't specified for unequal loops), and Flip, Twist and Remove apply as
/// for equal loops.
fn unequal(
    source: &Mesh,
    selected: &[u32],
    a: &Section,
    b: &Section,
    p: BridgeParameters,
) -> Result<BridgeResult> {
    if p.segments > 1 {
        return Err("Bridge between loops of different sizes supports one segment for now".into());
    }
    if p.twist != 0 {
        return Err("Bridge Twist between loops of different sizes isn't specified yet".into());
    }
    let mut target = b.cycle.clone();
    if !p.reverse {
        target.reverse();
    }
    // Walk the larger loop; `big_is_a` keeps the faces wound as for equal loops.
    let big_is_a = a.cycle.len() > target.len();
    let (big, small) = if big_is_a {
        (&a.cycle, &target)
    } else {
        (&target, &a.cycle)
    };
    let (n, m) = (big.len(), small.len());
    let pos = |v: u32| source.vertices[v as usize];
    let partner: Vec<usize> = big
        .iter()
        .map(|&v| {
            (0..m)
                .min_by(|&x, &y| {
                    pos(v)
                        .distance(pos(small[x]))
                        .total_cmp(&pos(v).distance(pos(small[y])))
                })
                .unwrap()
        })
        .collect();
    let steps: usize = (0..n)
        .map(|i| (partner[(i + 1) % n] + m - partner[i]) % m)
        .sum();
    if steps != m || (0..n).any(|i| (partner[(i + 1) % n] + m - partner[i]) % m > 1) {
        return Err(
            "Bridge can't pair these two loops in order; select loops that face each other".into(),
        );
    }
    let mut out = source.clone();
    out.faces.clear();
    let attr = &source.attributes;
    let mut ownership = Vec::new();
    let mut remap = vec![None; source.faces.len()];
    for (fi, f) in source.faces.iter().enumerate() {
        if p.remove && selected.contains(&(fi as u32)) {
            continue;
        }
        remap[fi] = Some(out.faces.len() as u32);
        out.faces.push(f.clone());
        ownership.push(Some(fi));
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (x, y) = (small[partner[i]], small[partner[j]]);
        // As the equal-loop quad [a_i, t_i, t_j, a_j], with the larger loop
        // in a's place or t's.
        let mut vertices = if big_is_a {
            vec![big[i], x, y, big[j]]
        } else {
            vec![x, big[i], big[j], y]
        };
        vertices.dedup();
        if vertices.first() == vertices.last() {
            vertices.pop();
        }
        let mut f = source.faces[a.donor].clone();
        f.vertices = vertices;
        f.uv = None;
        if p.flip {
            f.vertices.reverse();
        }
        out.faces.push(f);
        ownership.push(None);
    }
    let rows: Vec<usize> = out.faces.iter().map(|f| f.vertices.len()).collect();
    let keep = |old: &Vec<Vec<Option<glam::DVec2>>>| -> Vec<Vec<Option<glam::DVec2>>> {
        ownership
            .iter()
            .zip(&rows)
            .map(|(o, &n)| o.map_or_else(|| vec![None; n], |i| old[i].clone()))
            .collect()
    };
    for map in out.uv_maps.values_mut() {
        *map = keep(map);
    }
    let at = &mut out.attributes;
    for map in at.uv_records.values_mut() {
        *map = ownership
            .iter()
            .zip(&rows)
            .map(|(o, &n)| o.map_or_else(|| vec![None; n], |i| map[i].clone()))
            .collect();
    }
    for set in at.polygon_sets.values_mut() {
        *set = set.iter().filter_map(|&f| remap[f as usize]).collect();
    }
    at.degenerate_faces = attr
        .degenerate_faces
        .iter()
        .filter_map(|&f| remap[f as usize])
        .collect();
    let owner = |o: &Option<usize>| o.unwrap_or(a.donor);
    if !attr.representations.is_empty() {
        at.representations = ownership
            .iter()
            .map(|o| attr.representations[owner(o)])
            .collect();
    }
    if !attr.smoothing_labels.is_empty() {
        at.smoothing_labels = ownership
            .iter()
            .map(|o| attr.smoothing_labels[owner(o)].clone())
            .collect();
    }
    if crate::corner_uv::has_uncarried_corner_data(source) {
        return Err("Bridge between loops of different sizes can't carry corner weights, colors or normals yet".into());
    }
    out.validate()?;
    Ok(BridgeResult {
        mesh: out,
        selected: if p.remove {
            vec![]
        } else {
            selected.iter().filter_map(|&f| remap[f as usize]).collect()
        },
    })
}
