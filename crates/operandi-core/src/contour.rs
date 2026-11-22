//! Contour (Topology tab tool). Spec: missing-tools-g4 G5-CONTOUR v1,
//! MT13-01..14, cases CT-1..8.
//!
//! A line drawn across the background, and a slice direction (the view
//! direction when drawn), make a cutting plane (MT13-01). The plane cuts
//! the visible background meshes' polygons (MT13-02; the active mesh's own
//! with Intersect, MT13-13 provisional). The cut is kept only under the
//! line (MT13-03) and on the chosen Slicing Side (MT13-04); with Cull
//! Occluded Open Curves only the open piece facing the viewer stays. Each
//! piece becomes one new polygon in the active mesh with evenly spaced
//! points (MT13-05..08; Jason 2026-11-21: the same spacing for Curve,
//! Lines and Face, Q-CT-1).
use crate::{Mesh, Result, Scene};
use glam::{DMat4, DVec3};
use std::collections::BTreeMap;

/// The message when the line crosses nothing (MT13-12).
pub const MISSES: &str = "Contour: the line doesn't cross a background surface.";

/// Contour Type (MT13-08).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContourKind {
    Lines,
    #[default]
    Curve,
    Face,
}

/// Slicing Side (MT13-04): which side of the slice direction stays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SlicingSide {
    Positive,
    Negative,
    #[default]
    Both,
}

/// Contour's settings (the spec's defaults).
#[derive(Clone, Debug, PartialEq)]
pub struct ContourParameters {
    /// The drawn line's ends, world.
    pub start: DVec3,
    pub end: DVec3,
    /// The plane's second direction; zero takes the view's when drawn.
    pub direction: DVec3,
    pub kind: ContourKind,
    pub automatic: bool,
    pub segments: u32,
    pub uniform: bool,
    pub side: SlicingSide,
    pub cull: bool,
    pub intersect: bool,
    pub select_new: bool,
    pub angle_snap: bool,
    /// Angle snap step, degrees.
    pub angle: f64,
    pub fast: bool,
    /// A line has been drawn (or typed); nothing is made until then.
    pub placed: bool,
}
impl Default for ContourParameters {
    fn default() -> Self {
        Self {
            start: DVec3::ZERO,
            end: DVec3::ZERO,
            direction: DVec3::ZERO,
            kind: ContourKind::Curve,
            automatic: false,
            segments: 10,
            uniform: true,
            side: SlicingSide::Both,
            cull: true,
            intersect: false,
            select_new: true,
            angle_snap: false,
            angle: 45.,
            fast: true,
            placed: false,
        }
    }
}

/// One cut piece: its points in order (world) and whether it closes.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub points: Vec<DVec3>,
    pub closed: bool,
}

/// Where a cut point sits: on a mesh point or along an edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Point(usize, u32),
    Edge(usize, u32, u32),
}

/// The plane through the line and the direction: (origin, unit normal),
/// or an error when they don't make one.
fn plane(p: &ContourParameters) -> Result<(DVec3, DVec3)> {
    let along = p.end - p.start;
    if along.length() < 1e-12 {
        return Err("Contour: draw a line across the background".into());
    }
    let normal = along.cross(p.direction);
    if normal.length() < 1e-12 {
        return Err("Contour: the slice direction runs along the line; draw it in a view or set another direction".into());
    }
    Ok((p.start, normal.normalize()))
}

/// The raw cut of `meshes` (each with its world matrix) by the plane.
pub fn cut(meshes: &[(&Mesh, DMat4)], origin: DVec3, normal: DVec3) -> Vec<Piece> {
    let mut point: BTreeMap<Key, DVec3> = BTreeMap::new();
    let mut links: BTreeMap<Key, Vec<Key>> = BTreeMap::new();
    for (m, (mesh, world)) in meshes.iter().enumerate() {
        let w: Vec<DVec3> = mesh
            .vertices
            .iter()
            .map(|&v| world.transform_point3(v))
            .collect();
        let d: Vec<f64> = w.iter().map(|&p| (p - origin).dot(normal)).collect();
        for (fi, face) in mesh.faces.iter().enumerate() {
            if mesh.is_polyline(fi) {
                continue;
            }
            let k = face.vertices.len();
            // Crossings round the polygon: (key, going up).
            let mut crossings = Vec::new();
            for c in 0..k {
                let (a, b) = (face.vertices[c], face.vertices[(c + 1) % k]);
                let (da, db) = (d[a as usize], d[b as usize]);
                let (up_a, up_b) = (da > 0., db > 0.);
                if up_a == up_b {
                    continue;
                }
                let (key, at) = if da == 0. {
                    (Key::Point(m, a), w[a as usize])
                } else if db == 0. {
                    (Key::Point(m, b), w[b as usize])
                } else {
                    let t = da / (da - db);
                    (
                        Key::Edge(m, a.min(b), a.max(b)),
                        w[a as usize].lerp(w[b as usize], t),
                    )
                };
                point.insert(key, at);
                crossings.push((key, up_b));
            }
            // Pair each crossing going up with the next going down.
            if let Some(first) = crossings.iter().position(|c| c.1) {
                crossings.rotate_left(first);
            }
            for pair in crossings.chunks(2) {
                if let [(a, _), (b, _)] = pair
                    && a != b
                {
                    links.entry(*a).or_default().push(*b);
                    links.entry(*b).or_default().push(*a);
                }
            }
        }
    }
    chain(&point, &links)
}

/// Join the cut segments into pieces: open ones from their ends, then
/// the loops.
fn chain(point: &BTreeMap<Key, DVec3>, links: &BTreeMap<Key, Vec<Key>>) -> Vec<Piece> {
    let mut used: BTreeMap<(Key, Key), bool> = BTreeMap::new();
    let pair = |a: Key, b: Key| (a.min(b), a.max(b));
    let mut pieces = Vec::new();
    let ends: Vec<Key> = links
        .iter()
        .filter(|(_, n)| n.len() == 1)
        .map(|(&k, _)| k)
        .chain(links.keys().copied())
        .collect();
    for start in ends {
        let mut keys = vec![start];
        let mut at = start;
        let mut closed = false;
        while let Some(&next) = links[&at]
            .iter()
            .find(|&&n| !used.contains_key(&pair(at, n)))
        {
            used.insert(pair(at, next), true);
            if next == start {
                closed = keys.len() > 2;
                break;
            }
            keys.push(next);
            at = next;
        }
        if keys.len() < 2 {
            continue;
        }
        pieces.push(Piece {
            points: keys.iter().map(|k| point[k]).collect(),
            closed,
        });
    }
    pieces
}

/// The parts of `piece` where `f` is at least 0, cut where it crosses.
fn clip(piece: &Piece, f: &dyn Fn(DVec3) -> f64) -> Vec<Piece> {
    let pts = &piece.points;
    let n = pts.len();
    if pts.iter().all(|&p| f(p) >= 0.) {
        return vec![piece.clone()];
    }
    // A loop is walked from a point outside, so every kept run is whole.
    let order: Vec<DVec3> = if piece.closed {
        let s = pts.iter().position(|&p| f(p) < 0.).unwrap_or(0);
        (0..=n).map(|i| pts[(s + i) % n]).collect()
    } else {
        pts.clone()
    };
    let mut out = Vec::new();
    let mut run: Vec<DVec3> = Vec::new();
    for i in 0..order.len() {
        let p = order[i];
        let inside = f(p) >= 0.;
        if i > 0 {
            let q = order[i - 1];
            let (fq, fp) = (f(q), f(p));
            if (fq >= 0.) != (fp >= 0.) {
                let edge = q.lerp(p, fq / (fq - fp));
                run.push(edge);
                if !inside {
                    out.push(std::mem::take(&mut run));
                }
            }
        }
        if inside {
            run.push(p);
        }
    }
    if run.len() >= 2 {
        out.push(run);
    }
    out.into_iter()
        .filter(|r| r.len() >= 2)
        .map(|points| Piece {
            points,
            closed: false,
        })
        .collect()
}

/// The pieces Contour keeps for `p` from the raw cut (MT13-03/04, cull).
pub fn kept_pieces(p: &ContourParameters, raw: Vec<Piece>) -> Vec<Piece> {
    let along = p.end - p.start;
    let length = along.length();
    let u = along / length;
    // The slice direction across the line, for sides and the viewer.
    let across = (p.direction - u * p.direction.dot(u)).normalize_or_zero();
    let t = move |q: DVec3| (q - p.start).dot(u) / length;
    let s = move |q: DVec3| (q - p.start).dot(across);
    let mut pieces: Vec<Piece> = raw
        .iter()
        .flat_map(|piece| clip(piece, &|q| t(q)))
        .flat_map(|piece| clip(&piece, &|q| 1. - t(q)))
        .collect();
    match p.side {
        SlicingSide::Positive => {
            pieces = pieces.iter().flat_map(|c| clip(c, &s)).collect();
        }
        SlicingSide::Negative => {
            pieces = pieces.iter().flat_map(|c| clip(c, &|q| -s(q))).collect();
        }
        SlicingSide::Both => {}
    }
    // A closed piece the clipping left whole keeps its loop.
    for piece in &mut pieces {
        if !piece.closed {
            continue;
        }
        piece.closed = raw
            .iter()
            .any(|r| r.closed && r.points.len() == piece.points.len());
    }
    if p.cull {
        let open: Vec<usize> = (0..pieces.len()).filter(|&i| !pieces[i].closed).collect();
        let facing =
            |c: &Piece| c.points.iter().map(|&q| s(q)).sum::<f64>() / c.points.len() as f64;
        if let Some(&best) = open
            .iter()
            .max_by(|&&a, &&b| facing(&pieces[a]).total_cmp(&facing(&pieces[b])))
        {
            let mut i = 0;
            pieces.retain(|c| {
                let keep = c.closed || i == best;
                i += 1;
                keep
            });
        }
    }
    // Open pieces run from the end nearer the line's end (CT-7, CT-8).
    for piece in &mut pieces {
        if !piece.closed && t(piece.points[0]) < t(*piece.points.last().unwrap_or(&p.start)) {
            piece.points.reverse();
        }
    }
    pieces
}

/// `n` points spaced along `piece` (MT13-05..07).
pub fn spaced(piece: &Piece, p: &ContourParameters, n: usize) -> Vec<DVec3> {
    let pts = &piece.points;
    if pts.len() < 2 || n == 0 {
        return pts.clone();
    }
    if !p.uniform {
        // MT13-07 (provisional): even steps of the raw cut's point index.
        let span = if piece.closed {
            pts.len()
        } else {
            pts.len() - 1
        };
        let last = if piece.closed { n } else { (n - 1).max(1) };
        return (0..n)
            .map(|i| pts[((i * span) as f64 / last as f64).round() as usize % pts.len()])
            .collect();
    }
    let mut path = pts.clone();
    if piece.closed {
        path.push(pts[0]);
    }
    let lengths: Vec<f64> = path.windows(2).map(|w| w[0].distance(w[1])).collect();
    let total: f64 = lengths.iter().sum();
    let steps = if piece.closed { n } else { (n - 1).max(1) };
    let mut out = Vec::with_capacity(n);
    let (mut seg, mut walked) = (0, 0.);
    for i in 0..n {
        let target = total * i as f64 / steps as f64;
        while seg + 1 < lengths.len() && walked + lengths[seg] < target {
            walked += lengths[seg];
            seg += 1;
        }
        let f = if lengths[seg] > 0. {
            ((target - walked) / lengths[seg]).clamp(0., 1.)
        } else {
            0.
        };
        out.push(path[seg].lerp(path[seg + 1], f));
    }
    out
}

/// The contour polygons for `p` in `scene`: each piece's points (world),
/// and whether it closes. `MISSES` when nothing is under the line.
pub fn contour_pieces(scene: &Scene, p: &ContourParameters) -> Result<Vec<Piece>> {
    let (origin, normal) = plane(p)?;
    let active = scene.selection.item;
    let mut sources = Vec::new();
    for item in &scene.items {
        let wanted = if p.intersect {
            Some(item.id) == active
        } else {
            Some(item.id) != active && !scene.selection.also.contains(&item.id)
        };
        if !wanted || !item.is_mesh() || !scene.shows(item) {
            continue;
        }
        sources.push((&*item.mesh, scene.world_matrix(item.id)?));
    }
    let raw = cut(&sources, origin, normal);
    let pieces = kept_pieces(p, raw);
    if pieces.is_empty() {
        return Err(MISSES.into());
    }
    Ok(pieces
        .iter()
        .map(|piece| {
            let n = if p.automatic {
                // MT13-06 (provisional): the cut's own points, plus one.
                piece.points.len() + usize::from(piece.closed)
            } else {
                p.segments.max(1) as usize + usize::from(!piece.closed)
            };
            // MT13-11: a loop starts at the cut point nearest the line's
            // start.
            let mut piece = piece.clone();
            if piece.closed
                && let Some(first) = (0..piece.points.len()).min_by(|&a, &b| {
                    piece.points[a]
                        .distance(p.start)
                        .total_cmp(&piece.points[b].distance(p.start))
                })
            {
                piece.points.rotate_left(first);
            }
            Piece {
                points: spaced(&piece, p, n),
                closed: piece.closed,
            }
        })
        .collect())
}

/// One live preview: the contour polygons added to the active mesh (an
/// empty mesh is fine), selected when Select New Polygons is on (MT13-10).
pub fn preview(scene: &mut Scene, p: &ContourParameters) -> Result<()> {
    if !p.placed {
        return Ok(());
    }
    let pieces = contour_pieces(scene, p)?;
    let item = scene
        .selection
        .item
        .ok_or("Contour: select a mesh to add the contour to")?;
    let to_local = scene.world_matrix(item)?.inverse();
    let target = scene.item_mut(item).ok_or("Contour: select a mesh")?;
    let mesh = &mut *target.mesh;
    let mut made = Vec::new();
    for piece in pieces {
        let first = mesh.vertices.len() as u32;
        mesh.vertices
            .extend(piece.points.iter().map(|&q| to_local.transform_point3(q)));
        let ids: Vec<u32> = (first..mesh.vertices.len() as u32).collect();
        crate::vertex_commands::append_face(mesh, ids, None);
        let f = mesh.faces.len() - 1;
        let curve = match p.kind {
            ContourKind::Face => None,
            ContourKind::Curve => Some(crate::curves::CurveKind::Curve {
                start_control: false,
                end_control: false,
            }),
            ContourKind::Lines => Some(crate::curves::CurveKind::Line),
        };
        if let Some(curve) = curve {
            crate::curve_commands::set_representation(
                mesh,
                f,
                crate::PolygonRepresentation::Polyline {
                    closed: piece.closed,
                    curve,
                },
            );
        }
        made.push(f as u32);
    }
    mesh.validate()?;
    if p.select_new {
        scene.selection.faces = made;
    }
    Ok(())
}
