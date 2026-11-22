//! Lathe (BEHAVIOR-LATHE.md; the reference's Radial Sweep): the Circle
//! placer (`duplicate.rs`) with the Sweep copier, which joins the
//! profile's turned positions into one surface (LT-20).
//!
//! - The profile (LT-10/11): in Polygon mode each selected line or curve
//!   sweeps as a line and each face polygon its outline; nothing selected
//!   means the whole mesh. In Edge mode the selected edges sweep, joined
//!   end to end; in Vertex mode the edges between selected points.
//! - Curves are drawn into straight spans first, as the viewport draws them
//!   (LT-12). Lines and curves are used up; faces stay (LT-13).
//! - A full turn closes round with no doubled seam points (LT-22); points
//!   on the axis are shared by every step, so the polygons meeting there
//!   are triangles (LT-23). The polygons face away from the axis on
//!   average (LT-26); caps close a part turn (LT-25).
//! - The first ring is the profile's own points when they can join the
//!   new surface without an edge holding three polygons: lines and curves
//!   (used up), and border edges on a part turn. Otherwise the surface is
//!   made of new points beside the profile (provisional, DUP-Q03).
use crate::duplicate::{Lathe, UvsAround, reverse_corners};
use crate::{Face, Mesh, Mode, PolygonRepresentation, Result, Scene};
use glam::{DMat4, DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// LT-23: a point this close to the axis (world metres) is on it.
pub const ON_AXIS: f64 = 1e-6;

/// What a Lathe sweeps, in the mesh's local space.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Profile {
    pub points: Vec<DVec3>,
    /// The mesh point each profile point is, when the first ring reuses it.
    pub reuse: Vec<Option<u32>>,
    /// Point lists, and whether each closes on itself.
    pub chains: Vec<(Vec<usize>, bool)>,
    /// Lines and curves used up by the sweep (LT-13).
    pub used_up: Vec<usize>,
    /// The polygon whose material, part and smoothing the new polygons
    /// take (LT-28).
    pub template: Option<usize>,
}

impl Profile {
    fn point(
        &mut self,
        at: &mut HashMap<(u32, bool), usize>,
        mesh: &Mesh,
        v: u32,
        reuse: bool,
    ) -> usize {
        *at.entry((v, reuse)).or_insert_with(|| {
            self.points.push(mesh.vertices[v as usize]);
            self.reuse.push(reuse.then_some(v));
            self.points.len() - 1
        })
    }
}

/// Undirected edges joined into chains, end to end (LT-11).
fn chains_of(edges: &[(u32, u32)]) -> Vec<(Vec<u32>, bool)> {
    let mut next: BTreeMap<u32, Vec<(u32, usize)>> = BTreeMap::new();
    for (i, &(a, b)) in edges.iter().enumerate() {
        next.entry(a).or_default().push((b, i));
        next.entry(b).or_default().push((a, i));
    }
    let mut used = vec![false; edges.len()];
    let mut out = Vec::new();
    let walk = |from: u32, used: &mut Vec<bool>| {
        let mut chain = vec![from];
        let mut at = from;
        loop {
            let step = next[&at].iter().find(|(_, e)| !used[*e]).copied();
            let Some((to, e)) = step else {
                break;
            };
            used[e] = true;
            if to == from {
                return (chain, true);
            }
            chain.push(to);
            at = to;
        }
        (chain, false)
    };
    // Open chains from their ends first, then the loops left.
    let ends: Vec<u32> = next
        .iter()
        .filter(|(_, n)| n.len() != 2)
        .map(|(&v, _)| v)
        .collect();
    for v in ends {
        while next[&v].iter().any(|(_, e)| !used[*e]) {
            out.push(walk(v, &mut used));
        }
    }
    for (i, &(a, _)) in edges.iter().enumerate() {
        if !used[i] {
            out.push(walk(a, &mut used));
        }
    }
    out
}

/// The profile of the active mesh for `full_turn` (LT-10/11/12/13).
pub fn profile(scene: &Scene, full_turn: bool) -> Profile {
    let mut out = Profile::default();
    let Some(item) = scene.active() else {
        return out;
    };
    let mesh = &item.mesh;
    let s = &scene.selection;
    let mut at = HashMap::new();
    let edges: Option<Vec<(u32, u32)>> = match s.mode {
        Mode::Edge if !s.edges.is_empty() => Some(
            s.edges
                .iter()
                .map(|&(a, b)| (a.min(b), a.max(b)))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        ),
        Mode::Vertex if !s.vertices.is_empty() => {
            let on: BTreeSet<u32> = s.vertices.iter().copied().collect();
            Some(
                mesh.edges()
                    .into_iter()
                    .filter(|(a, b)| on.contains(a) && on.contains(b))
                    .collect(),
            )
        }
        _ => None,
    };
    if let Some(edges) = edges {
        // Border edges can carry the first ring on a part turn.
        let counts = mesh.edge_counts();
        let reuse = !full_turn
            && edges
                .iter()
                .all(|e| counts.get(e).copied().unwrap_or(0) <= 1);
        for (chain, closed) in chains_of(&edges) {
            let ids = chain
                .iter()
                .map(|&v| out.point(&mut at, mesh, v, reuse))
                .collect();
            out.chains.push((ids, closed));
        }
        return out;
    }
    let faces: Vec<usize> = if s.mode == Mode::Polygon && !s.faces.is_empty() {
        let mut f: Vec<usize> = s.faces.iter().map(|&f| f as usize).collect();
        f.sort_unstable();
        f.dedup();
        f.retain(|&f| f < mesh.faces.len());
        f
    } else {
        (0..mesh.faces.len()).collect()
    };
    let mut outline: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for fi in faces {
        let face = &mesh.faces[fi];
        match mesh.attributes.representations.get(fi) {
            Some(PolygonRepresentation::Polyline { closed, curve }) => {
                out.used_up.push(fi);
                if curve.is_line() {
                    let ids = face
                        .vertices
                        .iter()
                        .map(|&v| out.point(&mut at, mesh, v, true))
                        .collect();
                    out.chains.push((ids, *closed));
                } else {
                    let control: Vec<DVec3> = face
                        .vertices
                        .iter()
                        .map(|&v| mesh.vertices[v as usize])
                        .collect();
                    // RS-10: about seven pieces a curved span, one a straight.
                    let samples = crate::curves::sweep_sample(&control, *curve, *closed);
                    let start = out.points.len();
                    out.points.extend(&samples);
                    out.reuse.extend(std::iter::repeat_n(None, samples.len()));
                    out.chains
                        .push(((start..out.points.len()).collect(), *closed));
                }
            }
            _ => {
                out.template.get_or_insert(fi);
                let n = face.vertices.len();
                for i in 0..n {
                    let (a, b) = (face.vertices[i], face.vertices[(i + 1) % n]);
                    *outline.entry((a.min(b), a.max(b))).or_insert(0) += 1;
                }
            }
        }
    }
    // Faces sweep the outline of their region: edges two of them share
    // would sweep twice into the same place.
    let border: Vec<(u32, u32)> = outline
        .into_iter()
        .filter(|&(_, n)| n == 1)
        .map(|(e, _)| e)
        .collect();
    for (chain, closed) in chains_of(&border) {
        let ids = chain
            .iter()
            .map(|&v| out.point(&mut at, mesh, v, false))
            .collect();
        out.chains.push((ids, closed));
    }
    out
}

fn newell(points: &[DVec3]) -> DVec3 {
    let mut n = DVec3::ZERO;
    for i in 0..points.len() {
        n += points[i].cross(points[(i + 1) % points.len()]);
    }
    n
}

/// One new polygon before it's added: its points (generated-mesh indices)
/// and corner UVs.
struct NewFace {
    vertices: Vec<u32>,
    uv: Vec<Option<DVec2>>,
    cap: bool,
}

/// The swept surface of `profile` as its own mesh (local space, the
/// first ring as new points), with the generated point standing for each
/// reused profile point.
pub fn sweep_geometry(
    profile: &Profile,
    world: DMat4,
    lathe: &Lathe,
    template: Face,
) -> Result<(Mesh, Vec<(usize, u32)>)> {
    let c = &lathe.circle;
    c.axis_direction()?;
    if c.count == 0 || c.count > 100_000 {
        return Err("Radial Sweep Count must be from 1 to 100000".into());
    }
    if !(c.start.is_finite() && c.end.is_finite() && c.offset.is_finite() && c.center.is_finite()) {
        return Err("Radial Sweep values must be finite".into());
    }
    if (c.end - c.start).abs() < 1e-9 {
        return Err("Radial Sweep needs End Angle different from Start Angle".into());
    }
    let steps = c.count as usize;
    let full = c.whole_turns() && c.offset == 0.;
    let rings = if full { steps } else { steps + 1 };
    let segments: usize = profile
        .chains
        .iter()
        .map(|(ch, closed)| ch.len().saturating_sub(usize::from(!closed)))
        .sum();
    if segments.saturating_mul(steps) > crate::duplicate::NEW_POLYGON_LIMIT {
        return Err(format!(
            "Too many copies ({} polygons)",
            segments.saturating_mul(steps)
        ));
    }
    // Three spans meeting at a point would put three polygons on an edge.
    let mut degree = vec![0usize; profile.points.len()];
    for (chain, closed) in &profile.chains {
        for j in 0..chain.len() - usize::from(!closed).min(chain.len()) {
            degree[chain[j]] += 1;
            degree[chain[(j + 1) % chain.len()]] += 1;
        }
    }
    if degree.iter().any(|&d| d > 2) {
        // LT-13b.
        return Err("Radial Sweep needs a profile without branches.".into());
    }
    let local = world.inverse();
    let at_world = |p: DVec3| world.transform_point3(p);
    let n = profile.points.len();
    let pole: Vec<bool> = profile
        .points
        .iter()
        .map(|&p| {
            let w = at_world(p);
            c.offset == 0. && w.distance(c.foot(w)) <= ON_AXIS
        })
        .collect();
    let mut mesh = Mesh::default();
    // Generated index of (ring, profile point).
    let mut index = vec![u32::MAX; rings * n];
    let mut pole_index = vec![u32::MAX; n];
    for (i, &is_pole) in pole.iter().enumerate() {
        if is_pole {
            pole_index[i] = mesh.vertices.len() as u32;
            mesh.vertices.push(profile.points[i]);
        }
    }
    let turns: Vec<DMat4> = (0..rings)
        .map(|k| {
            let angle = c.start + (c.end - c.start) * k as f64 / steps as f64;
            c.turn(angle, c.offset * k as f64)
                .map(|t| local * t * world)
        })
        .collect::<Result<_>>()?;
    for (k, turn) in turns.iter().enumerate() {
        for i in 0..n {
            index[k * n + i] = if pole[i] {
                pole_index[i]
            } else {
                mesh.vertices.push(turn.transform_point3(profile.points[i]));
                mesh.vertices.len() as u32 - 1
            };
        }
    }
    let ring = |k: usize, i: usize| index[(k % rings) * n + i];
    let uv = |around: f64, along: f64| {
        Some(match lathe.sweep.uvs_around {
            UvsAround::U => DVec2::new(around, along),
            UvsAround::V => DVec2::new(along, around),
        })
    };
    let mut faces: Vec<NewFace> = Vec::new();
    for (chain, closed) in &profile.chains {
        if chain.len() < 2 {
            continue;
        }
        let spans: Vec<(usize, usize)> = (0..chain.len() - usize::from(!closed))
            .map(|j| (j, (j + 1) % chain.len()))
            .collect();
        // Arc length along the chain, for the UVs (LT-27).
        let mut along = vec![0.; chain.len() + 1];
        for (j, &(a, b)) in spans.iter().enumerate() {
            along[j + 1] = along[j] + profile.points[chain[a]].distance(profile.points[chain[b]]);
        }
        let total = along[spans.len()].max(1e-12);
        for (j, &(a, b)) in spans.iter().enumerate() {
            let (pa, pb) = (chain[a], chain[b]);
            if pole[pa] && pole[pb] {
                continue; // LT-24: an edge along the axis makes nothing.
            }
            let (sa, sb) = (along[j] / total, along[j + 1] / total);
            for k in 0..steps {
                let (u0, u1) = (k as f64 / steps as f64, (k + 1) as f64 / steps as f64);
                let corners = [
                    (ring(k, pa), uv(u0, sa)),
                    (ring(k, pb), uv(u0, sb)),
                    (ring(k + 1, pb), uv(u1, sb)),
                    (ring(k + 1, pa), uv(u1, sa)),
                ];
                let mut vertices = Vec::new();
                let mut uvs = Vec::new();
                for (v, t) in corners {
                    if vertices.last() != Some(&v) && vertices.first() != Some(&v) {
                        vertices.push(v);
                        uvs.push(t);
                    }
                }
                // A span crossing the axis turns into a bow tie with no
                // area (LT-29); it is left out rather than made invalid.
                let area = newell(
                    &vertices
                        .iter()
                        .map(|&v| at_world(mesh.vertices[v as usize]))
                        .collect::<Vec<_>>(),
                );
                if vertices.len() >= 3 && area.length() > 1e-12 {
                    faces.push(NewFace {
                        vertices,
                        uv: uvs,
                        cap: false,
                    });
                }
            }
        }
    }
    // LT-26: face away from the axis on average.
    let world_points: Vec<DVec3> = mesh.vertices.iter().map(|&p| at_world(p)).collect();
    let vote: f64 = faces
        .iter()
        .map(|f| {
            let pts: Vec<DVec3> = f
                .vertices
                .iter()
                .map(|&v| world_points[v as usize])
                .collect();
            let centre = pts.iter().sum::<DVec3>() / pts.len() as f64;
            newell(&pts).dot(centre - c.foot(centre))
        })
        .sum();
    let outward = vote >= 0.;
    // LT-25: caps close a part turn, one per profile chain. A cap uses
    // each ring edge the other way round from the sweep polygon on it, so
    // the surface stays one consistent piece: in chain order a start cap
    // runs with the first step's polygons, an end cap against the last's.
    if !full {
        for (chain, closed) in &profile.chains {
            for (k, at_start, on) in [
                (0, true, lathe.sweep.cap_start),
                (rings - 1, false, lathe.sweep.cap_end),
            ] {
                if !on || chain.len() < 2 {
                    continue;
                }
                let mut vertices: Vec<u32> = chain.iter().map(|&i| ring(k, i)).collect();
                if !closed {
                    // Open profiles close against the axis (LT-25 [OP]).
                    for &end in [chain[chain.len() - 1], chain[0]].iter() {
                        if !pole[end] {
                            let w = world_points[ring(k, end) as usize];
                            mesh.vertices.push(local.transform_point3(c.foot(w)));
                            vertices.push(mesh.vertices.len() as u32 - 1);
                        }
                    }
                }
                let mut v: Vec<u32> = Vec::new();
                for x in vertices {
                    let p = mesh.vertices[x as usize];
                    if !v
                        .iter()
                        .any(|&y| y == x || mesh.vertices[y as usize].distance(p) < 1e-12)
                    {
                        v.push(x);
                    }
                }
                if v.len() < 3 {
                    continue;
                }
                let pts: Vec<DVec3> = v
                    .iter()
                    .map(|&x| at_world(mesh.vertices[x as usize]))
                    .collect();
                if newell(&pts).length() < 1e-12 {
                    continue;
                }
                if at_start == outward {
                    v[1..].reverse();
                }
                let corners = v.len();
                faces.push(NewFace {
                    vertices: v,
                    uv: vec![None; corners],
                    cap: true,
                });
            }
        }
    }
    let texture: Vec<Vec<Option<DVec2>>> = faces.iter().map(|f| f.uv.clone()).collect();
    for f in &faces {
        mesh.faces.push(Face {
            vertices: f.vertices.clone(),
            uv: None,
            ..template.clone()
        });
    }
    mesh.uv_maps.insert("Texture".into(), texture);
    for (fi, f) in faces.iter().enumerate() {
        if (!outward && !f.cap) != lathe.sweep.flip {
            reverse_corners(&mut mesh, fi);
        }
    }
    let reused: Vec<(usize, u32)> = (0..n)
        .filter_map(|i| Some((i, profile.reuse[i]?)))
        .filter(|&(i, _)| {
            // The first ring stands on the profile only at angle 0.
            pole[i] || (c.start / 360. - (c.start / 360.).round()).abs() < 1e-12
        })
        .map(|(i, v)| (index[i] as usize, v))
        .collect();
    Ok((mesh, reused))
}

/// The Lathe preview on every active mesh.
pub(crate) fn preview(scene: &mut Scene, lathe: &Lathe, symmetry: crate::Symmetry) -> Result<()> {
    let ids = scene.active_meshes();
    let ids = if ids.is_empty() {
        scene.active().map(|i| i.id).into_iter().collect()
    } else {
        ids
    };
    for id in ids {
        scene.as_sole_mesh(id, |s| {
            // SYM3-12: the other side's profile turns about the mirrored
            // axis, the other way round (a reflection reverses the turn).
            let world = s.world_matrix(id)?;
            let Some(m) = crate::symmetry_sides::WorldMirror::new(symmetry, world) else {
                return mesh_lathe(s, lathe);
            };
            let c = &lathe.circle;
            let mirrored = Lathe {
                circle: crate::duplicate::Circle {
                    center: m.point(c.center),
                    direction: m.vector(c.direction),
                    start: -c.start,
                    end: -c.end,
                    ..*c
                },
                ..*lathe
            };
            crate::symmetry_sides::two_pass(s, symmetry, |s, other| {
                mesh_lathe(s, if other { &mirrored } else { lathe })
            })?;
            Ok(())
        })?;
    }
    Ok(())
}

fn mesh_lathe(scene: &mut Scene, lathe: &Lathe) -> Result<()> {
    let Some(item) = scene.active() else {
        return Ok(());
    };
    let id = item.id;
    let full = lathe.circle.whole_turns() && lathe.circle.offset == 0.;
    let profile = profile(scene, full);
    if profile.chains.is_empty() {
        return Ok(());
    }
    let world = scene.world_matrix(id)?;
    if world.determinant() == 0. {
        return Err("Radial Sweep needs an invertible item transform".into());
    }
    let mesh = &scene.active().ok_or("Select a mesh")?.mesh;
    let template = match profile.template {
        Some(f) => Face {
            vertices: Vec::new(),
            uv: None,
            ..mesh.faces[f].clone()
        },
        None => Face {
            vertices: Vec::new(),
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        },
    };
    let (generated, reused) = sweep_geometry(&profile, world, lathe, template)?;
    let loose: BTreeSet<u32> = mesh
        .used_points()
        .iter()
        .enumerate()
        .filter(|(_, u)| !**u)
        .map(|(v, _)| v as u32)
        .collect();
    let mut out: Mesh = (**mesh).clone();
    let start = out.vertices.len() as u32;
    crate::append_mesh(&mut out, generated)?;
    // The first ring joins the profile's own points.
    let groups: Vec<(Vec<u32>, DVec3)> = reused
        .iter()
        .map(|&(g, v)| (vec![v, start + g as u32], out.vertices[v as usize]))
        .collect();
    let (mut out, remap) = if groups.is_empty() {
        let n = out.vertices.len() as u32;
        (out, (0..n).map(Some).collect::<Vec<_>>())
    } else {
        crate::edge_commands::weld(&out, &groups, "Radial Sweep")?
    };
    // LT-13: lines and curves are used up; the points only they used go.
    let mut face_map: Vec<Option<u32>> = (0..out.faces.len() as u32).map(Some).collect();
    let mut point_map: Vec<Option<u32>> = remap;
    if !profile.used_up.is_empty() {
        let gone: BTreeSet<usize> = profile.used_up.iter().copied().collect();
        let kept = crate::clipboard::whole_faces(&out, |f| !gone.contains(&f));
        face_map = vec![None; out.faces.len()];
        for (i, k) in kept.iter().enumerate() {
            face_map[k.source] = Some(i as u32);
        }
        let keep_loose: BTreeSet<u32> = loose
            .iter()
            .filter_map(|&v| point_map[v as usize])
            .collect();
        let (smaller, points) = crate::clipboard::subset(&out, &kept, &keep_loose)?;
        point_map = point_map
            .iter()
            .map(|v| v.and_then(|v| points[v as usize]))
            .collect();
        out = smaller;
    }
    out.validate()?;
    scene.active_mut().ok_or("Select a mesh")?.mesh = out.into();
    let s = &mut scene.selection;
    let point = |v: u32| point_map.get(v as usize).copied().flatten();
    s.vertices = s.vertices.iter().filter_map(|&v| point(v)).collect();
    s.edges = s
        .edges
        .iter()
        .filter_map(|&(a, b)| Some((point(a)?, point(b)?)))
        .collect();
    s.faces = s
        .faces
        .iter()
        .filter_map(|&f| face_map.get(f as usize).copied().flatten())
        .collect();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chains_join_end_to_end() {
        let chains = chains_of(&[(0, 1), (1, 2), (5, 6)]);
        assert_eq!(chains.len(), 2);
        assert!(chains.contains(&(vec![0, 1, 2], false)));
        let ring = chains_of(&[(0, 1), (1, 2), (0, 2)]);
        assert_eq!(ring.len(), 1);
        assert!(ring[0].1);
        assert_eq!(ring[0].0.len(), 3);
    }
}
