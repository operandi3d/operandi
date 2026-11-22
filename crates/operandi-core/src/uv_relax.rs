//! UV Relax (E4, UVS-1 §3, BEHAVIOR-UV-TOOLS §4 UVT-21): smooths an
//! existing UV map island by island to take out stretch. 3D never moves.
//!
//! Modes:
//! - **Spring**: every UV edge pulls like a spring (each pass moves a UV
//!   position half way to its UV neighbours' average). Our own maths.
//! - **Conformal / Angle Based**: the island is solved again with
//!   least-squares conformal maps or angle based flattening (the Blender
//!   port in `operandi-uv`), fitted back onto where it was (turn, scale,
//!   move) unless its border is held.
//! - **Adaptive**: minimum stretch (SLIM) from the current UVs, so each
//!   polygon keeps its size relative to 3D.
//!
//! Iterations is the amount: 0 changes nothing (UVS-1 §3). For Spring and
//! Adaptive it is the number of passes; for Conformal and Angle Based each
//! iteration closes a tenth of the way to the solution (`SOLVE_STEP`).
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md UVR-P01..P05): the
//! Conformal / Angle Based step; Maintain keeps border points on their
//! border by letting them move only towards their border neighbours;
//! Straighten treats border points where the border turns more than
//! `CORNER_DEGREES` as corners and lines the points between up; the falloff
//! weighs each mesh point in 3D in either view (UVT-Q03); Interactive pins,
//! Pause, Save Constraints, UV Symmetry and Show Transform Handles aren't
//! built yet.
use crate::topology::edge_key;
use crate::{Mesh, Result, Scene};
use glam::DVec2;
use operandi_uv as uv;
use std::collections::{BTreeSet, HashMap};

/// How much of the way to the solved island one Conformal or Angle Based
/// iteration goes (UVR-P01, build guess): 10 iterations ≈ 65 %, 50 ≈ 99 %.
pub const SOLVE_STEP: f64 = 0.1;
/// UVR-P03 (build guess): Straighten's corners are where the border turns
/// more than this.
pub const CORNER_DEGREES: f64 = 30.;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RelaxMode {
    Spring,
    #[default]
    Conformal,
    AngleBased,
    Adaptive,
}
impl RelaxMode {
    pub const ALL: [Self; 4] = [
        Self::Spring,
        Self::Conformal,
        Self::AngleBased,
        Self::Adaptive,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Spring => "Spring",
            Self::Conformal => "Conformal",
            Self::AngleBased => "Angle Based",
            Self::Adaptive => "Adaptive",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RelaxBoundary {
    #[default]
    Smooth,
    Locked,
    Maintain,
    Straighten,
}
impl RelaxBoundary {
    pub const ALL: [Self; 4] = [Self::Smooth, Self::Locked, Self::Maintain, Self::Straighten];
    pub fn label(self) -> &'static str {
        match self {
            Self::Smooth => "Smooth",
            Self::Locked => "Locked",
            Self::Maintain => "Maintain",
            Self::Straighten => "Straighten",
        }
    }
}

/// UV Relax's Tool Properties (UVS-1 §3, in order; the unbuilt ones left
/// out).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RelaxOptions {
    #[serde(skip)]
    pub uv_map: Option<String>,
    pub mode: RelaxMode,
    pub iterations: u32,
    /// Conformal and Angle Based only.
    pub seal_holes: bool,
    /// Not in Spring mode.
    pub boundary: RelaxBoundary,
    /// Straighten only: border points evenly spaced.
    pub uniform_spans: bool,
    /// Adaptive only, 0..1: how strongly polygons keep their area.
    pub area_weight: f64,
    /// Off: islands holding a selected edge stay still.
    pub ignore_edge_selection: bool,
    /// Interactive (UVS-1 §3 Pins): clicks in the UV view place pins.
    pub interactive: bool,
    /// Pins: UV positions held at a place; dragging one drags the map
    /// with it, stretched between the pins.
    pub pins: Vec<UvPin>,
}

/// A Relax pin: the UV position of mesh point `point` that started at
/// `at`, held at `to`.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UvPin {
    pub point: u32,
    pub at: DVec2,
    pub to: DVec2,
}
impl Default for RelaxOptions {
    fn default() -> Self {
        Self {
            uv_map: None,
            mode: RelaxMode::Conformal,
            iterations: 0,
            seal_holes: false,
            boundary: RelaxBoundary::Smooth,
            uniform_spans: false,
            area_weight: 0.1,
            ignore_edge_selection: true,
            interactive: false,
            pins: Vec::new(),
        }
    }
}
impl RelaxOptions {
    pub fn validate(&self) -> Result<()> {
        if !(self.area_weight.is_finite() && (0. ..=1.).contains(&self.area_weight)) {
            return Err("Area Weight must be from 0 % to 100 %".into());
        }
        Ok(())
    }
}

/// Quantised UV, so corners at one mesh point with the same UV are one UV
/// position (as `uv_pipe`).
fn key(p: DVec2) -> (i64, i64) {
    let q = |x: f64| (x / 1e-6).round() as i64;
    (q(p.x), q(p.y))
}

/// The UV layout of the used polygons as UV positions.
struct Layout {
    /// Proxy position of each used face's corners (empty when unused).
    corners: Vec<Vec<usize>>,
    /// UV of each position.
    uv: Vec<DVec2>,
    /// Mesh point of each position.
    point: Vec<u32>,
    /// Island of each position.
    island: Vec<usize>,
    islands: usize,
    /// Positions on an island border.
    border: Vec<bool>,
    /// UV edge neighbours of each position, and border neighbours.
    next: Vec<BTreeSet<usize>>,
    border_next: Vec<BTreeSet<usize>>,
}

fn layout(mesh: &Mesh, rows: &[Vec<Option<DVec2>>], used: &[bool]) -> Layout {
    let mut corners = vec![Vec::new(); mesh.faces.len()];
    let mut uvs = Vec::new();
    let mut point = Vec::new();
    let mut ids: HashMap<(u32, (i64, i64)), usize> = HashMap::new();
    for (f, face) in mesh.faces.iter().enumerate() {
        if !used[f] {
            continue;
        }
        corners[f] = face
            .vertices
            .iter()
            .enumerate()
            .map(|(c, &v)| {
                let q = rows[f][c].expect("used corners are mapped");
                *ids.entry((v, key(q))).or_insert_with(|| {
                    uvs.push(q);
                    point.push(v);
                    uvs.len() - 1
                })
            })
            .collect();
    }
    let n = uvs.len();
    // UV edges and how many polygons use each.
    let mut uses: HashMap<(usize, usize), u32> = HashMap::new();
    let mut next = vec![BTreeSet::new(); n];
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for ids in corners.iter().filter(|c| !c.is_empty()) {
        for i in 0..ids.len() {
            let (a, b) = (ids[i], ids[(i + 1) % ids.len()]);
            *uses.entry((a.min(b), a.max(b))).or_default() += 1;
            next[a].insert(b);
            next[b].insert(a);
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            if ra != rb {
                parent[ra.max(rb)] = ra.min(rb);
            }
        }
    }
    let mut border = vec![false; n];
    let mut border_next = vec![BTreeSet::new(); n];
    for (&(a, b), &count) in &uses {
        if count == 1 {
            border[a] = true;
            border[b] = true;
            border_next[a].insert(b);
            border_next[b].insert(a);
        }
    }
    let mut roots: HashMap<usize, usize> = HashMap::new();
    let island = (0..n)
        .map(|x| {
            let r = find(&mut parent, x);
            let k = roots.len();
            *roots.entry(r).or_insert(k)
        })
        .collect();
    Layout {
        corners,
        uv: uvs,
        point,
        island,
        islands: roots.len(),
        border,
        next,
        border_next,
    }
}

/// Fits `new` onto `old` with one turn, uniform scale and move (and a
/// mirror first when the solver flipped it), returning the fitted points.
fn fit_similar(old: &[DVec2], new: &[DVec2]) -> Vec<DVec2> {
    let n = old.len().max(1) as f64;
    let (ca, cb) = (old.iter().sum::<DVec2>() / n, new.iter().sum::<DVec2>() / n);
    let signed = |p: &[DVec2], c: DVec2| -> f64 {
        // A rough orientation: the summed cross of successive offsets.
        p.windows(2).map(|w| (w[0] - c).perp_dot(w[1] - c)).sum()
    };
    let flip = signed(old, ca) * signed(new, cb) < 0.;
    let mirror = |p: DVec2| if flip { DVec2::new(-p.x, p.y) } else { p };
    let (mut re, mut im, mut den) = (0., 0., 0.);
    for (a, b) in old.iter().zip(new) {
        let (a, b) = (*a - ca, mirror(*b - cb));
        re += a.x * b.x + a.y * b.y;
        im += a.y * b.x - a.x * b.y;
        den += b.length_squared();
    }
    if den < 1e-24 {
        return old.to_vec();
    }
    let z = DVec2::new(re / den, im / den);
    new.iter()
        .map(|b| {
            let b = mirror(*b - cb);
            ca + DVec2::new(z.x * b.x - z.y * b.y, z.x * b.y + z.y * b.x)
        })
        .collect()
}

/// Straighten (UVR-P03): each border run between corners laid on the
/// straight line between them, evenly (`uniform`) or by its UV lengths.
fn straighten(l: &Layout, uv: &mut [DVec2], uniform: bool) {
    let n = uv.len();
    let mut seen = vec![false; n];
    for start in 0..n {
        if !l.border[start] || seen[start] || l.border_next[start].len() != 2 {
            continue;
        }
        // Walk the loop.
        let mut ring = vec![start];
        seen[start] = true;
        let mut prev = start;
        let mut at = *l.border_next[start].iter().next().unwrap();
        while at != start && !seen[at] && l.border_next[at].len() == 2 {
            seen[at] = true;
            ring.push(at);
            let step = *l.border_next[at]
                .iter()
                .find(|&&x| x != prev)
                .unwrap_or(&start);
            prev = at;
            at = step;
        }
        if at != start || ring.len() < 4 {
            continue;
        }
        let m = ring.len();
        let corner = |i: usize| {
            let (a, b, c) = (
                uv[ring[(i + m - 1) % m]],
                uv[ring[i]],
                uv[ring[(i + 1) % m]],
            );
            let (u, v) = (b - a, c - b);
            u.length() > 1e-12
                && v.length() > 1e-12
                && u.angle_to(v).abs().to_degrees() > CORNER_DEGREES
        };
        let corners: Vec<usize> = (0..m).filter(|&i| corner(i)).collect();
        if corners.len() < 2 {
            continue;
        }
        for (k, &c0) in corners.iter().enumerate() {
            let c1 = corners[(k + 1) % corners.len()];
            let span: Vec<usize> = (0..=((c1 + m - c0) % m)).map(|j| (c0 + j) % m).collect();
            let (a, b) = (uv[ring[span[0]]], uv[ring[*span.last().unwrap()]]);
            let lengths: Vec<f64> = span
                .windows(2)
                .map(|w| uv[ring[w[1]]].distance(uv[ring[w[0]]]))
                .collect();
            let total: f64 = lengths.iter().sum::<f64>().max(1e-12);
            let mut run = 0.;
            let last = span.len() - 1;
            let mut placed = Vec::with_capacity(span.len());
            for j in 0..=last {
                let t = if uniform {
                    j as f64 / last.max(1) as f64
                } else {
                    if j > 0 {
                        run += lengths[j - 1];
                    }
                    run / total
                };
                placed.push(a.lerp(b, t));
            }
            for (j, p) in placed.into_iter().enumerate() {
                uv[ring[span[j]]] = p;
            }
        }
    }
}

/// UV Relax of the scene's active mesh's map. `weights` (per mesh point)
/// scale the amount (UVT-21); `selected_edges` hold their islands still
/// when Ignore Edge Selection is off.
pub fn relax_mesh(scene: &Scene, o: &RelaxOptions, weights: Option<&[f64]>) -> Result<Mesh> {
    o.validate()?;
    let item = scene.active().ok_or("Select a mesh")?;
    let mesh = &item.mesh;
    let map = crate::uv_tools::target_uv_map(mesh, o.uv_map.as_deref());
    if o.iterations == 0 {
        return Ok((**mesh).clone());
    }
    let rows = mesh
        .uv_maps
        .get(&map)
        .ok_or("The mesh has no UV map to relax: UV Unwrap or Project makes one")?;
    let hidden = crate::Protection::of(mesh, &item.elements)
        .map(|p| p.face_hidden)
        .unwrap_or_default();
    let used: Vec<bool> = mesh
        .faces
        .iter()
        .enumerate()
        .map(|(f, face)| {
            !hidden.get(f).copied().unwrap_or(false)
                && !mesh.is_polyline(f)
                && rows.get(f).is_some_and(|r| {
                    r.len() == face.vertices.len() && r.iter().all(Option::is_some)
                })
        })
        .collect();
    let l = layout(mesh, rows, &used);
    if l.uv.is_empty() {
        return Err("No visible polygons have UVs in this map".into());
    }
    // Islands held still (Ignore Edge Selection off).
    let mut held = vec![false; l.islands];
    if !o.ignore_edge_selection && scene.selection.mode == crate::Mode::Edge {
        let chosen: BTreeSet<(u32, u32)> = scene
            .selection
            .edges
            .iter()
            .map(|&(a, b)| edge_key(a, b))
            .collect();
        for ids in l.corners.iter().filter(|c| !c.is_empty()) {
            for i in 0..ids.len() {
                let (a, b) = (ids[i], ids[(i + 1) % ids.len()]);
                if chosen.contains(&edge_key(l.point[a], l.point[b])) {
                    held[l.island[a]] = true;
                }
            }
        }
    }
    let boundary = if o.mode == RelaxMode::Spring {
        RelaxBoundary::Smooth
    } else {
        o.boundary
    };
    let mut target = l.uv.clone();
    if boundary == RelaxBoundary::Straighten {
        straighten(&l, &mut target, o.uniform_spans);
    }
    // Interactive pins: the positions they hold, and where.
    let pin_at: HashMap<usize, DVec2> = if o.interactive {
        o.pins
            .iter()
            .filter_map(|pin| {
                (0..l.uv.len())
                    .find(|&x| l.point[x] == pin.point && l.uv[x].distance(pin.at) < 1e-6)
                    .map(|x| (x, pin.to))
            })
            .collect()
    } else {
        HashMap::new()
    };
    for (&x, &to) in &pin_at {
        target[x] = to;
    }
    let pinned = |x: usize| {
        pin_at.contains_key(&x)
            || (matches!(boundary, RelaxBoundary::Locked | RelaxBoundary::Straighten)
                && l.border[x])
    };
    match o.mode {
        RelaxMode::Spring => {
            for _ in 0..o.iterations {
                let before = target.clone();
                for x in 0..target.len() {
                    if l.next[x].is_empty() {
                        continue;
                    }
                    if pin_at.contains_key(&x) {
                        continue;
                    }
                    let avg = l.next[x].iter().map(|&y| before[y]).sum::<DVec2>()
                        / l.next[x].len() as f64;
                    target[x] = before[x].lerp(avg, 0.5);
                }
                // Free borders would shrink the island: keep its size
                // (not when pins hold it).
                if pin_at.is_empty() {
                    keep_size(&l, &before, &mut target);
                }
            }
        }
        RelaxMode::Conformal | RelaxMode::AngleBased | RelaxMode::Adaptive => {
            let corners: Vec<usize> = (0..mesh.faces.len()).filter(|&f| used[f]).collect();
            let input = uv::UvMesh {
                points: mesh.vertices.clone(),
                weights: None,
                faces: corners
                    .iter()
                    .map(|&f| {
                        l.corners[f]
                            .iter()
                            .map(|&x| uv::Corner {
                                point: mesh.faces[f].vertices
                                    [l.corners[f].iter().position(|&y| y == x).unwrap()],
                                uv: target[x],
                                pin: pinned(x),
                                select: true,
                            })
                            .collect()
                    })
                    .collect(),
                seams: Vec::new(),
            };
            let construct = uv::ConstructOptions {
                fill_holes: o.seal_holes && o.mode != RelaxMode::Adaptive,
                topology_from_uvs: true,
                aspect_y: 1.,
            };
            let mut p = uv::Param::new(&input, construct).map_err(|e| e.to_string())?;
            if o.mode == RelaxMode::Adaptive {
                let opts = uv::SlimOptions {
                    iterations: o.iterations as usize,
                    weight_influence: o.area_weight,
                    ..Default::default()
                };
                p.slim_relax(&opts);
            } else {
                p.lscm_begin_all(false, o.mode == RelaxMode::AngleBased, false);
                p.lscm_solve();
                p.lscm_end();
            }
            p.flush();
            let solved = p.corner_uvs();
            let mut new = target.clone();
            for (k, &f) in corners.iter().enumerate() {
                for (c, &x) in l.corners[f].iter().enumerate() {
                    new[x] = solved[k][c];
                }
            }
            // An island whose border wasn't held is fitted back onto where
            // it was, so relaxing never throws it across the tile.
            for i in 0..l.islands {
                let members: Vec<usize> = (0..new.len()).filter(|&x| l.island[x] == i).collect();
                if members.iter().any(|&x| pinned(x)) || o.mode == RelaxMode::Adaptive {
                    continue;
                }
                let old: Vec<DVec2> = members.iter().map(|&x| target[x]).collect();
                let got: Vec<DVec2> = members.iter().map(|&x| new[x]).collect();
                for (&x, q) in members.iter().zip(fit_similar(&old, &got)) {
                    new[x] = q;
                }
            }
            let t = if o.mode == RelaxMode::Adaptive || !pin_at.is_empty() {
                1.
            } else {
                1. - (1. - SOLVE_STEP).powi(o.iterations as i32)
            };
            for x in 0..target.len() {
                target[x] = target[x].lerp(new[x], t);
            }
        }
    }
    // Maintain: border points keep to their border (UVR-P02).
    if boundary == RelaxBoundary::Maintain {
        for (x, t) in target.iter_mut().enumerate() {
            if l.border[x] && l.border_next[x].len() == 2 {
                let mut it = l.border_next[x].iter();
                let (a, b) = (l.uv[*it.next().unwrap()], l.uv[*it.next().unwrap()]);
                let d = (b - a).normalize_or_zero();
                let start = l.uv[x];
                *t = start + d * (*t - start).dot(d);
            } else if l.border[x] {
                *t = l.uv[x];
            }
        }
    }
    // Held islands, locked borders and the falloff.
    for x in 0..target.len() {
        let w = weights.map_or(1., |w| w.get(l.point[x] as usize).copied().unwrap_or(0.));
        if let Some(&to) = pin_at.get(&x) {
            target[x] = to;
        } else if held[l.island[x]] || (pinned(x) && boundary == RelaxBoundary::Locked) {
            target[x] = l.uv[x];
        } else {
            target[x] = l.uv[x].lerp(target[x], w);
        }
    }
    let mut out = (**mesh).clone();
    let rows = out.uv_maps.get_mut(&map).expect("checked above");
    for (f, ids) in l.corners.iter().enumerate() {
        for (c, &x) in ids.iter().enumerate() {
            rows[f][c] = Some(target[x]);
        }
    }
    out.attributes.point_uvs.remove(&map);
    if out.attributes.uv_records.contains_key(&map) {
        let records = crate::corner_uv::records_from_uvs(&out.faces, &out.uv_maps[&map]);
        out.attributes.uv_records.insert(map.clone(), records);
    }
    out.validate()?;
    Ok(out)
}

/// Scale each island of `after` about its centre back to the UV area its
/// positions spanned in `before` (Spring's free borders shrink it).
fn keep_size(l: &Layout, before: &[DVec2], after: &mut [DVec2]) {
    for i in 0..l.islands {
        let members: Vec<usize> = (0..after.len()).filter(|&x| l.island[x] == i).collect();
        let spread = |p: &[DVec2], c: DVec2| {
            members
                .iter()
                .map(|&x| p[x].distance_squared(c))
                .sum::<f64>()
        };
        let n = members.len().max(1) as f64;
        let c0 = members.iter().map(|&x| before[x]).sum::<DVec2>() / n;
        let c1 = members.iter().map(|&x| after[x]).sum::<DVec2>() / n;
        let (s0, s1) = (spread(before, c0), spread(after, c1));
        if s1 < 1e-24 {
            continue;
        }
        let k = (s0 / s1).sqrt();
        for &x in &members {
            after[x] = c0 + (after[x] - c1) * k;
        }
    }
}

/// Unwrap & Relax (UVS-1 §1.5, UVT-20): the Unwrap engine, then the Relax
/// engine, in one tool session; each half can be turned off.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PeltOptions {
    pub unwrap_on: bool,
    pub unwrap: crate::uv_tools::UnwrapOptions,
    pub relax_on: bool,
    pub relax: RelaxOptions,
}
impl Default for PeltOptions {
    /// UVS-1 §1.5's own starting values: Iterations 0 for both halves,
    /// relax Mode Angle Based, Ignore Edge Selection on, Seal Holes and
    /// Update Layout on.
    fn default() -> Self {
        Self {
            unwrap_on: true,
            unwrap: crate::uv_tools::UnwrapOptions {
                iterations: 0,
                ..Default::default()
            },
            relax_on: true,
            relax: RelaxOptions {
                mode: RelaxMode::AngleBased,
                iterations: 0,
                ignore_edge_selection: true,
                ..Default::default()
            },
        }
    }
}

/// UV Unwrap of the scene's active mesh with `o` (the live tool's run of
/// `uv_tools::unwrap_mesh`): visible polygons, the edge selection as seams.
pub fn unwrap_scene(scene: &Scene, o: &crate::uv_tools::UnwrapOptions) -> Result<Mesh> {
    let item = scene.active().ok_or("Select a mesh")?;
    let hidden = crate::Protection::of(&item.mesh, &item.elements)
        .map(|p| p.face_hidden)
        .unwrap_or_else(|| vec![false; item.mesh.faces.len()]);
    let edges = if scene.selection.mode == crate::Mode::Edge {
        scene.selection.edges.clone()
    } else {
        Vec::new()
    };
    let map = match &o.uv_map {
        Some(name) if !name.trim().is_empty() => name.trim().to_owned(),
        _ => crate::uv_tools::target_uv_map(&item.mesh, None),
    };
    crate::uv_tools::unwrap_mesh(&item.mesh, &map, &hidden, &edges, o)
}

/// One preview of a UV tab live tool on the scene's active mesh; `None`
/// when it has nothing to write yet (Project from View before its click).
pub fn uv_tool_geometry(
    tool: crate::Tool,
    scene: &Scene,
    p: &crate::Parameters,
    weights: Option<&[f64]>,
) -> Result<Option<Mesh>> {
    use crate::Tool;
    Ok(Some(match tool {
        Tool::UvProject => {
            let faces = crate::uv_project::target_faces(scene)?;
            crate::uv_project::project_mesh(scene, &faces, &p.uv_project)?
        }
        Tool::UvViewProject => {
            if p.uv_view_project.view.is_none() {
                return Ok(None);
            }
            let faces = crate::uv_project::target_faces(scene)?;
            crate::uv_project::view_project_mesh(scene, &faces, &p.uv_view_project)?
        }
        Tool::UvUnwrap => unwrap_scene(scene, &p.uv_unwrap)?,
        Tool::UvPeeler => crate::uv_peel::peel_mesh(scene, &p.uv_peel)?,
        Tool::UvRelax => relax_mesh(scene, &p.uv_relax, weights)?,
        Tool::UvPelt => {
            let o = &p.uv_pelt;
            let mut scene = scene.clone();
            if o.unwrap_on {
                let mesh = unwrap_scene(&scene, &o.unwrap)?;
                scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
            }
            if o.relax_on && o.relax.iterations > 0 {
                let relax = RelaxOptions {
                    uv_map: o.unwrap.uv_map.clone(),
                    ..o.relax.clone()
                };
                relax_mesh(&scene, &relax, weights)?
            } else {
                (*scene.active().ok_or("Select a mesh")?.mesh).clone()
            }
        }
        _ => return Err(format!("{} isn't a UV tool", tool.label())),
    }))
}
