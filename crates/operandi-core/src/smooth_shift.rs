//! SST-1 independently implemented corner-response surface construction.
//! Candidates own their topology/correspondence; Editor owns transactions.
use crate::topology::edge_key;
use crate::*;
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmoothShiftParameters {
    pub offset: f64,
    pub scale: f64,
    pub angle: f64,
    pub thicken: bool,
    pub sharp: bool,
    pub angle_weight: bool,
    pub both_sides: bool,
}
impl Default for SmoothShiftParameters {
    fn default() -> Self {
        Self {
            offset: 0.,
            scale: 1.,
            angle: 89.5,
            thicken: false,
            sharp: false,
            angle_weight: false,
            both_sides: false,
        }
    }
}
impl SmoothShiftParameters {
    pub fn validate(self) -> Result<()> {
        if !self.offset.is_finite() || !self.scale.is_finite() || !self.angle.is_finite() {
            return Err(
                "Smooth Shift requires finite Offset, Scale and Max. Smoothing Angle".into(),
            );
        }
        Ok(())
    }
    /// Thicken's starting values, from its row in the tool preset table.
    pub fn thicken_initial() -> Self {
        crate::tool_preset::thicken_values()
    }
}
#[derive(Clone, Debug)]
pub struct SmoothShiftResult {
    pub mesh: Mesh,
    pub selected: Vec<u32>,
}
#[derive(Clone)]
struct OutputFace {
    owner: Option<usize>,
    corners: Vec<u32>,
    donors: Vec<Option<(usize, usize)>>,
    strip: bool,
    selected: bool,
    seed: usize,
}
fn unit(v: DVec3, cause: &str) -> Result<DVec3> {
    if !v.is_finite() || v.length_squared() == 0. {
        Err(format!(
            "Smooth Shift: {cause} is undefined here; nothing changed"
        ))
    } else {
        Ok(v.normalize())
    }
}
/// What Scale acts about (Q-SS-04, decided by Jason 2026-11-18: it follows
/// the selected action centre; an intentional difference from the
/// reference, which ignores it). Item-local. Centres taken from the
/// selection are measured on the shifted points, so Scale alone keeps
/// lifted caps in their plane and Offset alone doesn't resize them (user
/// correction 2026-10-30).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SmoothShiftPivot {
    /// The middle of the shifted selection's bounding box: no action
    /// centre, Selection, Selection Center Auto Axis and Automatic.
    SelectionBox,
    /// The average of the shifted selection's border points (Selection
    /// Border); the box middle when the selection has no border.
    SelectionBorder,
    /// One point (Origin, Element, Screen). With `mirror`, points on the
    /// far side of that symmetry plane scale about the point's mirror
    /// image and points on the plane about its foot on the plane, so a
    /// symmetric selection stays symmetric.
    Point {
        at: DVec3,
        mirror: Option<crate::Symmetry>,
    },
    /// Each connected selected piece about its own shifted box middle.
    Local,
    /// With Symmetry (ACS-20 through the pivot service): the box middle of
    /// the shifted points of the selection's + half (ACS-01, the − half
    /// when nothing is on +), the other side scaling about its mirror
    /// image and seam points about its foot on the plane, as `Point`.
    HalfBox(crate::Symmetry),
    /// As `HalfBox` with Selection Border's centre: the average of the
    /// half's shifted border points.
    HalfBorder(crate::Symmetry),
}

/// Weight interface applies only to Offset; positions remain item-local.
/// Scale acts about the item-local origin (the admitted command oracles).
pub fn smooth_shift_geometry(
    source: &Mesh,
    selected: &[u32],
    p: SmoothShiftParameters,
    weights: Option<&[f64]>,
) -> Result<SmoothShiftResult> {
    let origin = SmoothShiftPivot::Point {
        at: DVec3::ZERO,
        mirror: None,
    };
    smooth_shift_geometry_about(source, selected, p, weights, origin)
}

/// Smooth Shift with Scale about `pivot` (see [`SmoothShiftPivot`]).
pub fn smooth_shift_geometry_about(
    source: &Mesh,
    selected: &[u32],
    p: SmoothShiftParameters,
    weights: Option<&[f64]>,
    pivot: SmoothShiftPivot,
) -> Result<SmoothShiftResult> {
    source.validate()?;
    source.require_surface_elements()?;
    p.validate()?;
    for (name, map) in &source.attributes.typed_maps {
        if map.kind == "pose_shape3" && map.corners.iter().flatten().any(Option::is_some) {
            return Err(format!(
                "Smooth Shift can't keep pose-shape map {name}; nothing changed"
            ));
        }
    }

    source.require_disjoint_exception_component(selected)?;
    let nothing_selected = selected.is_empty();
    let chosen: BTreeSet<usize> = if selected.is_empty() {
        (0..source.faces.len()).collect()
    } else {
        selected.iter().map(|&f| f as usize).collect()
    };
    if chosen.is_empty()
        || chosen.iter().any(|&f| f >= source.faces.len())
        || (!selected.is_empty() && chosen.len() != selected.len())
    {
        return Err("Select valid distinct polygons".into());
    }
    if source.faces.iter().map(|f| f.vertices.len()).sum::<usize>() > 100_000 {
        return Err("Smooth Shift exceeds 100,000 source corners".into());
    }
    if weights.is_some_and(|w| w.len() != source.vertices.len() || w.iter().any(|v| !v.is_finite()))
    {
        return Err("Invalid Smooth Shift falloff weights".into());
    }
    // Edge uses are read only for edges at the selection's points (walls and
    // seeds), so only those are collected: a drag rebuilds this every frame.
    let mut at_chosen = vec![false; source.vertices.len()];
    for &f in &chosen {
        for &v in &source.faces[f].vertices {
            at_chosen[v as usize] = true;
        }
    }
    let mut incident = vec![Vec::new(); source.vertices.len()];
    let mut uses: BTreeMap<_, Vec<(usize, usize)>> = BTreeMap::new();
    for (fi, f) in source.faces.iter().enumerate() {
        for (ci, &v) in f.vertices.iter().enumerate() {
            incident[v as usize].push((fi, ci));
            let w = f.vertices[(ci + 1) % f.vertices.len()];
            if at_chosen[v as usize] || at_chosen[w as usize] {
                uses.entry(edge_key(v, w)).or_default().push((fi, ci));
            }
        }
    }
    let normals: Vec<_> = (0..source.faces.len()).map(|i| source.normal(i)).collect();
    let local = |fi: usize, ci: usize| -> Result<(DVec3, f64)> {
        let f = &source.faces[fi];
        let n = f.vertices.len();
        let v = source.vertices[f.vertices[ci] as usize];
        let a = unit(
            source.vertices[f.vertices[(ci + 1) % n] as usize] - v,
            "zero corner edge",
        )?;
        let b = unit(
            source.vertices[f.vertices[(ci + n - 1) % n] as usize] - v,
            "zero corner edge",
        )?;
        let mut c = unit(a.cross(b), "zero corner direction")?;
        if c.dot(normals[fi]) < 0. {
            c = -c;
        }
        Ok((c, a.dot(b).clamp(-1., 1.).acos()))
    };
    let threshold = if p.angle.rem_euclid(360.) == 90. || p.angle.rem_euclid(360.) == 270. {
        0.
    } else {
        p.angle.to_radians().cos()
    };
    let response = |fi: usize, v: u32| -> Result<DVec3> {
        let mut sum = DVec3::ZERO;
        for &(g, c) in &incident[v as usize] {
            if g == fi || normals[fi].dot(normals[g]) > threshold {
                sum += normals[g] * if p.angle_weight { local(g, c)?.1 } else { 1. };
            }
        }
        let u = unit(sum, "canceling face directions")?;
        if !p.sharp {
            return Ok(u);
        }
        // Sharp: lengthen the move so the pushed faces of this vertex's
        // smoothing group sit at (on average) the full Offset from their old
        // planes, where those offset planes meet (spec, "Sharp"). Only the
        // group's own polygons count, so a vertex whose group is a single
        // face moves exactly Offset (Operandi's rule for Q-SS-03).
        let mut dots = Vec::new();
        for &(g, _) in &incident[v as usize] {
            if g == fi || normals[fi].dot(normals[g]) > threshold {
                dots.push(u.dot(normals[g]));
            }
        }
        let divisor = dots.iter().sum::<f64>() / dots.len() as f64;
        if divisor <= 1e-9 || !divisor.is_finite() {
            return Err("Smooth Shift: Sharp is undefined at a corner; nothing changed".into());
        }
        Ok(u / divisor)
    };
    let seed = |v: u32| -> usize {
        incident[v as usize]
            .iter()
            .find(|&&(f, c)| {
                let vs = &source.faces[f].vertices;
                uses[&edge_key(v, vs[(c + 1) % vs.len()])].len() == 1
            })
            .unwrap_or(&incident[v as usize][0])
            .0
    };
    let mut out = source.clone();
    out.faces.clear();
    out.uv_maps.clear();
    out.attributes = Default::default();
    out.attributes.normal_map_order = source.attributes.normal_map_order.clone();
    let mut origins: Vec<u32> = (0..source.vertices.len() as u32).collect();
    // Components prohibit welding coincident disconnected islands, including s=0.
    let mut components: Vec<usize> = (0..source.vertices.len()).collect();
    fn root(c: &[usize], mut i: usize) -> usize {
        while c[i] != i {
            i = c[i];
        }
        i
    }
    for &f in &chosen {
        let vs = &source.faces[f].vertices;
        for &v in &vs[1..] {
            let a = root(&components, vs[0] as usize);
            let b = root(&components, v as usize);
            components[b] = a;
        }
    }
    let mut generated: BTreeMap<(usize, [u64; 3]), u32> = BTreeMap::new();
    let mut caps = BTreeMap::new();
    let mut source_donor = vec![None; source.vertices.len()];
    for &fi in &chosen {
        for (ci, &v) in source.faces[fi].vertices.iter().enumerate() {
            source_donor[v as usize].get_or_insert((fi, ci));
        }
    }
    // Scale acts about `pivot` (`SmoothShiftPivot`), below.
    let mut shifted = BTreeMap::new();
    for &fi in &chosen {
        for &v in &source.faces[fi].vertices {
            let q = source.vertices[v as usize]
                + if p.both_sides { 0.5 } else { 1. }
                    * p.offset
                    * weights.map_or(1., |w| w[v as usize])
                    * response(fi, v)?;
            shifted.insert((fi, v), q);
        }
    }
    let mut back = BTreeMap::new();
    if p.both_sides {
        for (v, d) in source_donor.iter().enumerate() {
            if d.is_some() {
                let w = weights.map_or(1., |w| w[v]);
                let q =
                    source.vertices[v] - 0.5 * p.offset * w * response(seed(v as u32), v as u32)?;
                back.insert(v, q);
            }
        }
    }
    // Scale's centre for each source point (see `SmoothShiftPivot`).
    let box_middle = |points: &mut dyn Iterator<Item = DVec3>| {
        let (low, high) = points.fold(
            (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
            |(l, h), q| (l.min(q), h.max(q)),
        );
        if low.is_finite() && high.is_finite() {
            (low + high) * 0.5
        } else {
            DVec3::ZERO
        }
    };
    let all_box = || box_middle(&mut shifted.values().chain(back.values()).copied());
    let border_of_selection = || -> BTreeSet<u32> {
        let mut chosen_uses: BTreeMap<_, usize> = BTreeMap::new();
        for &fi in &chosen {
            let vs = &source.faces[fi].vertices;
            for (i, &v) in vs.iter().enumerate() {
                *chosen_uses
                    .entry(edge_key(v, vs[(i + 1) % vs.len()]))
                    .or_default() += 1;
            }
        }
        chosen_uses
            .iter()
            .filter(|(_, n)| **n == 1)
            .flat_map(|(&(a, b), _)| [a, b])
            .collect()
    };
    // ACS-01 (pivot_service::half_points): the half a Symmetry centre is
    // measured on, by the side each shifted point started on.
    let pivot = match pivot {
        SmoothShiftPivot::HalfBox(symmetry) | SmoothShiftPivot::HalfBorder(symmetry) => {
            let starts: Vec<u32> = shifted.keys().map(|&(_, v)| v).collect();
            let half: BTreeSet<u32> = crate::pivot_service::half_points(source, &starts, symmetry)
                .into_iter()
                .collect();
            let border = matches!(pivot, SmoothShiftPivot::HalfBorder(_))
                .then(border_of_selection)
                .unwrap_or_default();
            let on_border: Vec<DVec3> = shifted
                .iter()
                .filter(|((_, v), _)| half.contains(v) && border.contains(v))
                .map(|(_, &q)| q)
                .collect();
            let at = if on_border.is_empty() {
                box_middle(
                    &mut shifted
                        .iter()
                        .filter(|((_, v), _)| half.contains(v))
                        .map(|(_, &q)| q),
                )
            } else {
                on_border.iter().sum::<DVec3>() / on_border.len() as f64
            };
            SmoothShiftPivot::Point {
                at,
                mirror: Some(symmetry),
            }
        }
        other => other,
    };
    let mut piece_centres: BTreeMap<usize, DVec3> = BTreeMap::new();
    let single = match pivot {
        SmoothShiftPivot::SelectionBox => Some(all_box()),
        SmoothShiftPivot::HalfBox(_) | SmoothShiftPivot::HalfBorder(_) => {
            unreachable!("resolved to a point above")
        }
        SmoothShiftPivot::SelectionBorder => {
            let border = border_of_selection();
            let points: Vec<DVec3> = shifted
                .iter()
                .filter(|((_, v), _)| border.contains(v))
                .map(|(_, &q)| q)
                .collect();
            Some(if points.is_empty() {
                all_box()
            } else {
                points.iter().sum::<DVec3>() / points.len() as f64
            })
        }
        SmoothShiftPivot::Point { mirror: None, at } => Some(at),
        SmoothShiftPivot::Point {
            mirror: Some(_), ..
        } => None,
        SmoothShiftPivot::Local => {
            let mut boxes: BTreeMap<usize, (DVec3, DVec3)> = BTreeMap::new();
            let points = shifted
                .iter()
                .map(|(&(_, v), &q)| (v as usize, q))
                .chain(back.iter().map(|(&v, &q)| (v, q)));
            for (v, q) in points {
                let e = boxes
                    .entry(root(&components, v))
                    .or_insert((DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)));
                *e = (e.0.min(q), e.1.max(q));
            }
            piece_centres = boxes
                .into_iter()
                .map(|(r, (lo, hi))| (r, (lo + hi) * 0.5))
                .collect();
            None
        }
    };
    let tolerance = match pivot {
        SmoothShiftPivot::Point {
            mirror: Some(_), ..
        } => crate::symmetry::match_tolerance(source),
        _ => 0.,
    };
    let centre_of = |v: u32| -> DVec3 {
        if let Some(c) = single {
            return c;
        }
        match pivot {
            SmoothShiftPivot::Point {
                at,
                mirror: Some(symmetry),
            } => {
                let here = symmetry.side(source.vertices[v as usize], tolerance);
                let there = symmetry.side(at, tolerance);
                match (here, there) {
                    (Some(crate::symmetry::Side::Plane), _) => symmetry.onto_plane(at),
                    (Some(a), Some(b)) if b != crate::symmetry::Side::Plane && a != b => {
                        symmetry.mirror(at)
                    }
                    _ => at,
                }
            }
            _ => piece_centres
                .get(&root(&components, v as usize))
                .copied()
                .unwrap_or(DVec3::ZERO),
        }
    };
    let scaled = |v: u32, q: DVec3| {
        let center = centre_of(v);
        center + p.scale * (q - center)
    };
    for (&v, &q) in &back {
        out.vertices[v] = scaled(v as u32, q);
    }
    for &fi in &chosen {
        let mut face = Vec::new();
        for &v in &source.faces[fi].vertices {
            let q = scaled(v, shifted[&(fi, v)]);
            if !q.is_finite() {
                return Err("Nonfinite Smooth Shift output".into());
            }
            let key = (
                root(&components, v as usize),
                q.to_array().map(|x| if x == 0. { 0 } else { x.to_bits() }),
            );
            let id = *generated.entry(key).or_insert_with(|| {
                let id = out.vertices.len() as u32;
                out.vertices.push(q);
                origins.push(v);
                id
            });
            face.push(id);
        }
        caps.insert(fi, face);
    }
    let mut faces = Vec::<OutputFace>::new();
    let mut mixed_walls = Vec::new();
    let mut virtual_strips = Vec::new();
    let mut push = |owner: Option<usize>,
                    corners: Vec<u32>,
                    donors: Vec<Option<(usize, usize)>>,
                    strip: bool,
                    sel: bool,
                    seed: usize| {
        let (mut corners, mut donors) = (corners, donors);
        if !sel {
            let mut i = 0;
            while corners.len() > 3 && i < corners.len() {
                let j = (i + 1) % corners.len();
                if corners[i] == corners[j] {
                    corners.remove(i);
                    donors.remove(i);
                } else {
                    i += 1;
                }
            }
        }
        faces.push(OutputFace {
            owner,
            corners,
            donors,
            strip,
            selected: sel,
            seed,
        });
    };
    for (fi, f) in source.faces.iter().enumerate() {
        let ds: Vec<_> = (0..f.vertices.len()).map(|c| Some((fi, c))).collect();
        if chosen.contains(&fi) {
            push(Some(fi), caps[&fi].clone(), ds.clone(), false, true, fi);
            if p.thicken {
                let mut vs = f.vertices.clone();
                let mut ds = ds;
                vs[1..].reverse();
                ds[1..].reverse();
                push(Some(fi), vs, ds, false, false, fi);
            }
        } else {
            push(Some(fi), f.vertices.clone(), ds, false, false, fi);
        }
    }
    for (&(a, b), inc) in &uses {
        let chosen_inc: Vec<_> = inc
            .iter()
            .copied()
            .filter(|(f, _)| chosen.contains(f))
            .collect();
        if chosen_inc.is_empty() {
            continue;
        }
        let (f, c) = chosen_inc[0];
        let vs = &source.faces[f].vertices;
        let next = (c + 1) % vs.len();
        let va = vs[c];
        let vb = vs[next];
        let ca = caps[&f][c];
        let cb = caps[&f][next];
        let ds = vec![Some((f, next)), Some((f, c)), Some((f, c)), Some((f, next))];
        if chosen_inc.len() == 1 {
            push(Some(f), vec![cb, ca, va, vb], ds, false, false, f);
        } else {
            let (g, j) = chosen_inc[1];
            let gv = &source.faces[g].vertices;
            let k = (j + 1) % gv.len();
            let ga = caps[&g][if gv[j] == va { j } else { k }];
            let gb = caps[&g][if gv[j] == vb { j } else { k }];
            let ty = |f: usize| {
                source
                    .attributes
                    .representations
                    .get(f)
                    .copied()
                    .unwrap_or(PolygonRepresentation::Face)
            };
            if ty(f) != ty(g) {
                mixed_walls.push((f, vec![cb, ca, va, vb]));
                virtual_strips.push(vec![cb, ca, ga, gb]);
                push(Some(f), vec![cb, ca, va, vb], ds.clone(), false, false, f);
                let mut v = vec![cb, ca, va, vb];
                let mut d = ds;
                v[1..].reverse();
                d[1..].reverse();
                push(Some(f), v, d, false, false, f);
            } else if ca != ga || cb != gb {
                let mut v = vec![cb, ca, ga, gb];
                let mut d = ds;
                let mut i = 0;
                while v.len() > 2 && i < v.len() {
                    let j = (i + 1) % v.len();
                    if v[i] == v[j] {
                        v.remove(i);
                        d.remove(i);
                    } else {
                        i += 1;
                    }
                }
                if v.len() >= 3 {
                    push(Some(f), v, d, true, true, f);
                }
            }
        }
        let _ = (a, b);
    }
    // Unpaired same-origin side edges bound precisely the residual corner holes.
    let mut boundary: BTreeMap<(u32, u32), Vec<(u32, u32)>> = BTreeMap::new();
    for f in &faces {
        if mixed_walls
            .iter()
            .any(|(o, v)| f.owner == Some(*o) && v.iter().all(|i| f.corners.contains(i)))
        {
            continue;
        }
        for i in 0..f.corners.len() {
            let a = f.corners[i];
            let b = f.corners[(i + 1) % f.corners.len()];
            if a != b && origins[a as usize] == origins[b as usize] {
                boundary.entry(edge_key(a, b)).or_default().push((a, b));
            }
        }
    }
    for vs in virtual_strips {
        for i in 0..vs.len() {
            let a = vs[i];
            let b = vs[(i + 1) % vs.len()];
            if a != b && origins[a as usize] == origins[b as usize] {
                boundary.entry(edge_key(a, b)).or_default().push((a, b));
            }
        }
    }
    let mut by_origin: BTreeMap<u32, Vec<(u32, u32)>> = BTreeMap::new();
    for es in boundary.values() {
        if es.len() == 1 {
            let (a, b) = es[0];
            by_origin
                .entry(origins[a as usize])
                .or_default()
                .push((b, a));
        }
    }
    for (v, mut es) in by_origin {
        while let Some((a, b)) = es.pop() {
            let mut cycle = vec![a, b];
            while *cycle.last().unwrap() != a {
                let tail = *cycle.last().unwrap();
                let Some(i) = es.iter().position(|&(x, _)| x == tail) else {
                    return Err("Smooth Shift: the polygons around a corner are ambiguous".into());
                };
                cycle.push(es.remove(i).1);
                if cycle.len() > 4097 {
                    return Err("Corner fill limit".into());
                }
            }
            cycle.pop();
            let anchor = if cycle.contains(&v) {
                v
            } else {
                *cycle.iter().min().unwrap()
            };
            let start = cycle.iter().position(|&x| x == anchor).unwrap();
            cycle.rotate_left(start);
            if cycle.len() >= 3 {
                faces.push(OutputFace {
                    owner: None,
                    donors: vec![None; cycle.len()],
                    corners: cycle,
                    strip: false,
                    selected: false,
                    seed: seed(v),
                });
            }
        }
    }
    if p.thicken && p.offset < 0. {
        for f in &mut faces {
            if f.owner.is_none_or(|o| chosen.contains(&o)) {
                f.corners[1..].reverse();
                f.donors[1..].reverse();
            }
        }
    }
    let mut selected = Vec::new();
    for (f, of) in faces.iter().enumerate() {
        let mut face = of.owner.map(|o| source.faces[o].clone()).unwrap_or(Face {
            vertices: vec![],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
        face.vertices = of.corners.clone();
        face.uv = None;
        out.faces.push(face);
        // Selection after: the shifted polygons keep their selection, and
        // the bridge quads where edges split are selected. With nothing
        // selected the shifted polygons weren't selected, so only the bridge
        // quads end up selected (SS10, TH12; Q-SS-10).
        if of.selected && (of.strip || !nothing_selected) {
            selected.push(f as u32);
        }
        out.attributes.representations.push(
            source
                .attributes
                .representations
                .get(of.seed)
                .copied()
                .unwrap_or(PolygonRepresentation::Face),
        );
        out.attributes.smoothing_labels.push(
            of.owner
                .and_then(|o| source.attributes.smoothing_labels.get(o).cloned().flatten()),
        );
    }
    transfer(source, &mut out, &chosen, &caps, &origins, &faces)?;
    // Degenerate repeated cycles are explicit output, never an implicit cleanup.
    for f in 0..out.faces.len() {
        if out.normal(f) == DVec3::ZERO
            || out.faces[f].vertices.iter().collect::<BTreeSet<_>>().len()
                != out.faces[f].vertices.len()
        {
            out.attributes.degenerate_faces.insert(f as u32);
        }
    }
    // Exact incidence records retain admitted shell seams without relaxing input checks.
    let mut incidence: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (fi, f) in out.faces.iter().enumerate() {
        for i in 0..f.vertices.len() {
            let a = f.vertices[i];
            let b = f.vertices[(i + 1) % f.vertices.len()];
            incidence
                .entry(edge_key(a, b))
                .or_default()
                .push([fi as u32, a, b]);
        }
    }
    for ((a, b), u) in incidence {
        if u.len() > 2 || (u.len() == 2 && u[0][1..] == u[1][1..]) {
            out.attributes
                .extrude_edge_uses
                .insert(format!("{a}:{b}"), u);
        }
    }
    compact(source, &mut out);
    out.validate()?;
    Ok(SmoothShiftResult {
        mesh: out,
        selected,
    })
}

type RemappedMap<T> = (BTreeMap<u32, T>, Vec<Vec<Option<T>>>, BTreeSet<usize>);
// All arguments describe one source-to-result correspondence, independent of
// attribute type. Kept explicit so map evaluation never acquires mesh ownership.
#[allow(clippy::too_many_arguments)]
fn remap<T: Clone + PartialEq>(
    source: &Mesh,
    out: &Mesh,
    chosen: &BTreeSet<usize>,
    caps: &BTreeMap<usize, Vec<u32>>,
    origins: &[u32],
    faces: &[OutputFace],
    point: &BTreeMap<u32, T>,
    corners: &[Vec<Option<T>>],
    uv: bool,
) -> RemappedMap<T> {
    let effective = |f: usize, c: usize| {
        corners
            .get(f)
            .and_then(|x| x[c].clone())
            .or_else(|| point.get(&source.faces[f].vertices[c]).cloned())
    };
    let mut points = point.clone();
    let mut earlier = BTreeMap::new();
    for &f in chosen {
        for (c, &v) in source.faces[f].vertices.iter().enumerate() {
            let kind = source
                .attributes
                .representations
                .get(f)
                .copied()
                .unwrap_or_default();
            let key = (
                v,
                match kind {
                    PolygonRepresentation::Face => 0,
                    PolygonRepresentation::Subdivs => 1,
                    PolygonRepresentation::CatmullClark => 2,
                    PolygonRepresentation::Polyline { .. } => 3,
                    PolygonRepresentation::Patch => 4,
                },
            );
            let donor = effective(f, c).or_else(|| earlier.get(&key).cloned());
            if let Some(value) = donor {
                earlier.entry(key).or_insert(value.clone());
                points.entry(caps[&f][c]).or_insert(value);
            }
        }
    }
    // Copies can merge across different source vertices only within a collapsed region.
    for (v, &origin) in origins.iter().enumerate().skip(source.vertices.len()) {
        if let Some(x) = point.get(&origin) {
            points.entry(v as u32).or_insert_with(|| x.clone());
        }
    }
    let mut seam = BTreeSet::new();
    let mut first = BTreeMap::new();
    let mut discontinuities = BTreeSet::new();
    if uv {
        for (fi, f) in source.faces.iter().enumerate() {
            for (ci, &v) in f.vertices.iter().enumerate() {
                if let Some(value) = effective(fi, ci) {
                    if let Some(old) = first.get(&v) {
                        if old != &value {
                            discontinuities.insert(v);
                        }
                    } else {
                        first.insert(v, value);
                    }
                }
            }
        }
    }
    let maps = faces
        .iter()
        .enumerate()
        .map(|(fi, f)| {
            let discontinuous = uv
                && f.strip
                && f.donors
                    .iter()
                    .flatten()
                    .any(|&(a, c)| discontinuities.contains(&source.faces[a].vertices[c]));
            let override_value = if discontinuous {
                f.donors
                    .iter()
                    .flatten()
                    .find_map(|&(a, c)| effective(a, c))
            } else {
                None
            };
            let first_present = f.donors[0].is_some_and(|(a, c)| effective(a, c).is_some());
            let start_index = f.donors.iter().enumerate().position(|(i, d)| {
                d.is_some_and(|(a, c)| effective(a, c).is_some())
                    && f.owner
                        .is_some_and(|o| caps.get(&o).is_some_and(|vs| vs.contains(&f.corners[i])))
            });
            if override_value.is_some() {
                seam.insert(fi);
            }
            let fill = if f.owner.is_none() {
                points.get(&f.corners[0]).cloned()
            } else {
                None
            };
            f.donors
                .iter()
                .enumerate()
                .map(|(ci, d)| {
                    if let Some(value) = &fill {
                        return (points.get(&f.corners[ci]) != Some(value)).then(|| value.clone());
                    }
                    if discontinuous && override_value.is_some() {
                        if first_present || ci != 0 {
                            return override_value.clone().filter(|value| {
                                Some(ci) != start_index || points.get(&f.corners[ci]) != Some(value)
                            });
                        }
                        return None;
                    }
                    d.and_then(|(a, c)| {
                        // Unselected source/base keeps authored absence. New caps keep
                        // explicit seams; continuous fallback lives on generated points.
                        if f.selected && !f.strip {
                            corners.get(a).and_then(|v| v[c].clone())
                        } else if f.corners[ci] as usize >= source.vertices.len() && !f.strip {
                            effective(a, c)
                                .filter(|value| points.get(&f.corners[ci]) != Some(value))
                        } else if f.strip {
                            effective(a, c)
                                .filter(|value| points.get(&f.corners[ci]) != Some(value))
                        } else {
                            corners.get(a).and_then(|v| v[c].clone())
                        }
                    })
                })
                .collect()
        })
        .collect();
    let _ = out;
    (points, maps, seam)
}
fn transfer(
    source: &Mesh,
    out: &mut Mesh,
    chosen: &BTreeSet<usize>,
    caps: &BTreeMap<usize, Vec<u32>>,
    origins: &[u32],
    faces: &[OutputFace],
) -> Result<()> {
    let a = &source.attributes;
    for (name, map) in &a.typed_maps {
        let (points, corners, seams) = remap(
            source,
            out,
            chosen,
            caps,
            origins,
            faces,
            &map.points,
            &map.corners,
            map.kind == "uv2",
        );
        out.attributes.typed_maps.insert(
            name.clone(),
            TypedMap {
                kind: map.kind.clone(),
                dimension: map.dimension,
                zero_default: map.zero_default,
                points,
                corners: if map.kind == "relative_morph3" {
                    Vec::new()
                } else {
                    corners
                },
            },
        );
        for fi in seams {
            out.faces[fi].material = "Side3".into();
        }
    }
    for (name, set) in &a.polygon_sets {
        out.attributes.polygon_sets.insert(
            name.clone(),
            faces
                .iter()
                .enumerate()
                .filter_map(|(f, of)| {
                    of.owner
                        .filter(|o| set.contains(&(*o as u32)))
                        .map(|_| f as u32)
                })
                .collect(),
        );
    }
    for (name, set) in &a.vertex_sets {
        let mut members = set.clone();
        for (&f, vs) in caps {
            for (c, &v) in vs.iter().enumerate() {
                if set.contains(&source.faces[f].vertices[c]) {
                    members.insert(v);
                }
            }
        }
        out.attributes.vertex_sets.insert(name.clone(), members);
    }
    // Source points keep their numbers until `compact`, which renumbers
    // these; edges the shift replaced leave their sets when it is recorded.
    out.attributes.edge_sets = a.edge_sets.clone();
    out.attributes.set_groups = a.set_groups.clone();

    for (name, map) in &source.uv_maps {
        let empty = BTreeMap::new();
        let point = a.point_uvs.get(name).unwrap_or(&empty);
        let (points, maps, seams) =
            remap(source, out, chosen, caps, origins, faces, point, map, true);
        out.attributes.point_uvs.insert(name.clone(), points);
        out.uv_maps.insert(name.clone(), maps);
        for fi in seams {
            out.faces[fi].material = "Side3".into();
        }
    }
    // Retain known UV assignment identities on surviving source corners. New
    // vertex copies receive distinct identities; equal values never erase seams.
    for (name, records) in &a.uv_records {
        let mut next = records
            .iter()
            .flatten()
            .flatten()
            .max()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("UV identity budget")?;
        let mut copies = BTreeMap::new();
        let mut result = Vec::new();
        for (fi, f) in faces.iter().enumerate() {
            let mut row = Vec::new();
            for (ci, d) in f.donors.iter().enumerate() {
                let value = out.uv_maps[name][fi][ci];
                let original = d.and_then(|(a, c)| records[a][c]);
                let id = if let (Some(value), Some(original)) = (value, original) {
                    let v = f.corners[ci];
                    if d.is_some_and(|(a, c)| {
                        source.faces[a].vertices[c] == v
                            && source.uv_maps[name][a][c] == Some(value)
                    }) {
                        Some(original)
                    } else {
                        let key = (original, v, value.to_array().map(f64::to_bits));
                        if let Some(id) = copies.get(&key) {
                            Some(*id)
                        } else {
                            let id = next;
                            next = next.checked_add(1).ok_or("UV identity budget")?;
                            copies.insert(key, id);
                            Some(id)
                        }
                    }
                } else {
                    None
                };
                row.push(id);
            }
            result.push(row);
        }
        out.attributes.uv_records.insert(name.clone(), result);
    }
    for (name, map) in &a.corner_normals {
        let (points, mut maps, _) = remap(
            source,
            out,
            chosen,
            caps,
            origins,
            faces,
            &BTreeMap::new(),
            map,
            false,
        );
        for (fi, f) in faces.iter().enumerate() {
            for (ci, &v) in f.corners.iter().enumerate() {
                if maps[fi][ci].is_none() {
                    maps[fi][ci] = points.get(&v).copied();
                }
            }
        }
        out.attributes.corner_normals.insert(name.clone(), maps);
    }
    for name in a
        .point_weights
        .keys()
        .chain(a.corner_weights.keys())
        .collect::<BTreeSet<_>>()
    {
        let (points, maps, _) = remap(
            source,
            out,
            chosen,
            caps,
            origins,
            faces,
            a.point_weights.get(name).unwrap_or(&BTreeMap::new()),
            a.corner_weights.get(name).map(Vec::as_slice).unwrap_or(&[]),
            false,
        );
        out.attributes.point_weights.insert(name.clone(), points);
        out.attributes.corner_weights.insert(name.clone(), maps);
    }
    for name in a
        .point_colors
        .keys()
        .chain(a.corner_colors.keys())
        .collect::<BTreeSet<_>>()
    {
        let (points, maps, _) = remap(
            source,
            out,
            chosen,
            caps,
            origins,
            faces,
            a.point_colors.get(name).unwrap_or(&BTreeMap::new()),
            a.corner_colors.get(name).map(Vec::as_slice).unwrap_or(&[]),
            false,
        );
        out.attributes.point_colors.insert(name.clone(), points);
        out.attributes.corner_colors.insert(name.clone(), maps);
    }
    for (name, map) in &a.relative_morphs {
        let (points, _, _) = remap(source, out, chosen, caps, origins, faces, map, &[], false);
        out.attributes.relative_morphs.insert(name.clone(), points);
    }
    // Legacy single-map payload has no map identity. Carry it independently.
    if source.faces.iter().any(|f| f.uv.is_some()) {
        for (fi, f) in faces.iter().enumerate() {
            let values: Option<Vec<_>> = f
                .donors
                .iter()
                .map(|d| d.and_then(|(a, c)| source.faces[a].uv.as_ref().map(|v| v[c])))
                .collect();
            out.faces[fi].uv = values;
        }
    }
    for c in &a.creases {
        let mut descendants = BTreeSet::new();
        descendants.insert(edge_key(c.a, c.b));
        for &f in chosen {
            let vs = &source.faces[f].vertices;
            for i in 0..vs.len() {
                if edge_key(vs[i], vs[(i + 1) % vs.len()]) == edge_key(c.a, c.b) {
                    descendants.insert(edge_key(caps[&f][i], caps[&f][(i + 1) % vs.len()]));
                }
            }
        }
        let edges: BTreeSet<_> = out.edges().into_iter().collect();
        for (a, b) in descendants {
            if a != b && edges.contains(&(a, b)) {
                out.attributes.creases.push(MeshCrease {
                    a,
                    b,
                    weight: c.weight,
                });
            }
        }
    }
    Ok(())
}

/// Drop the points `out`'s polygons no longer use (loose points of `source`
/// stay) and renumber. Returns each old point's new index. Also used by
/// Edge Extrude, whose cut corners leave the old corner point unused.
pub(crate) fn compact(source: &Mesh, out: &mut Mesh) -> Vec<Option<u32>> {
    // Points kept: those the output polygons use, and points no source
    // polygon used (loose points stay). Flags and an index table instead of
    // sets and maps: this runs on every frame of a drag.
    let source_used = source.used_points();
    let mut keep = out.used_points();
    for (v, &used) in source_used.iter().enumerate() {
        if !used && let Some(k) = keep.get_mut(v) {
            *k = true;
        }
    }
    let mut new_of_old: Vec<Option<u32>> = vec![None; out.vertices.len()];
    let mut vertices = Vec::new();
    for (v, _) in keep.iter().enumerate().filter(|(_, k)| **k) {
        new_of_old[v] = Some(vertices.len() as u32);
        vertices.push(out.vertices[v]);
    }
    out.vertices = vertices;
    let id = |v: u32| new_of_old.get(v as usize).copied().flatten();
    for f in &mut out.faces {
        for v in &mut f.vertices {
            *v = id(*v).expect("a used point is kept");
        }
    }
    fn points<T>(m: &mut BTreeMap<String, BTreeMap<u32, T>>, id: &impl Fn(u32) -> Option<u32>) {
        for map in m.values_mut() {
            *map = std::mem::take(map)
                .into_iter()
                .filter_map(|(v, x)| id(v).map(|i| (i, x)))
                .collect();
        }
    }
    for map in out.attributes.typed_maps.values_mut() {
        map.points = std::mem::take(&mut map.points)
            .into_iter()
            .filter_map(|(v, x)| id(v).map(|i| (i, x)))
            .collect();
    }
    for set in out.attributes.vertex_sets.values_mut() {
        *set = set.iter().filter_map(|&v| id(v)).collect();
    }
    out.attributes.edge_sets =
        crate::selection_sets::remap_edge_sets(&out.attributes.edge_sets, id);
    points(&mut out.attributes.point_uvs, &id);
    points(&mut out.attributes.point_weights, &id);
    points(&mut out.attributes.point_colors, &id);
    points(&mut out.attributes.relative_morphs, &id);
    for c in &mut out.attributes.creases {
        let a = id(c.a).expect("a creased point is kept");
        let b = id(c.b).expect("a creased point is kept");
        c.a = a.min(b);
        c.b = a.max(b);
    }
    out.attributes.extrude_edge_uses = std::mem::take(&mut out.attributes.extrude_edge_uses)
        .into_values()
        .map(|mut us| {
            for u in &mut us {
                u[1] = id(u[1]).expect("an edge-use point is kept");
                u[2] = id(u[2]).expect("an edge-use point is kept");
            }
            let a = us[0][1];
            let b = us[0][2];
            (format!("{}:{}", a.min(b), a.max(b)), us)
        })
        .collect();
    new_of_old
}
