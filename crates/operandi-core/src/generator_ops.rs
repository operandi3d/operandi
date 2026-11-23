//! Generators and the on-hold ops (modelling-parity batch 5; spec
//! SCATTER-UVRECT-TRANSFER-CONSTRAINT v1, GO-xx; Mesh Stack TYPE-IDS):
//!
//! - Scatter: random copies in a box (the direct tool, GO-01..07) and an
//!   evenly spread set in a square or box (the op, GO-10..13).
//! - Surface Constraint (GO-40..47): points pushed onto a linked surface.
//! - UV Rectangle (GO-20..24): quad-grid islands straightened into exact
//!   rectangles.
//! - Transfer Vertex Map (GO-30..32, not probed): a map sampled from a
//!   linked source surface.
//! - UV Fit as an op (the Fit command's engine). The UnSubdivide op is
//!   the Mesh Stack builder's (`poly.unsubdiv.item`).
use crate::background_constraint::Background;
use crate::duplicate::{Copies, WeldAt, copy_faces, jitter};
use crate::meshop_types::{Field, FieldKind, OpContext, OpDef, Typed, TypedOp};
use crate::{Mesh, Result};
use glam::{DMat3, DMat4, DQuat, DVec2, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

/// Most copies a scatter makes (an allocation guard).
pub const MAX_SCATTER: u32 = 100_000;

// ---- Scatter ----

/// The direct Scatter Clone tool's generator (GO-02 starting values).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScatterBox {
    /// Copies, the source included (GO-03).
    pub count: u32,
    pub centre: DVec3,
    /// Half-size on each axis: copies land in Centre ± Range (GO-04).
    pub range: DVec3,
    /// Scale variation, fractions (0.2 = ±20%).
    pub scale: DVec3,
    /// Rotation variation, degrees.
    pub rotate: DVec3,
    pub seed: u32,
}
impl Default for ScatterBox {
    fn default() -> Self {
        Self {
            count: 8,
            centre: DVec3::ZERO,
            range: DVec3::ONE,
            scale: DVec3::ZERO,
            rotate: DVec3::ZERO,
            seed: 0,
        }
    }
}

/// A rotation from X, Y, Z degrees (applied X, then Y, then Z).
fn euler(deg: DVec3) -> DQuat {
    DQuat::from_rotation_z(deg.z.to_radians())
        * DQuat::from_rotation_y(deg.y.to_radians())
        * DQuat::from_rotation_x(deg.x.to_radians())
}

impl ScatterBox {
    /// One transform per copy (the source not included; with Replace
    /// Source the first takes its place at Centre, GO-07), for a source
    /// whose bounding-box centre is `centre_of_source`.
    pub fn transforms(&self, centre_of_source: DVec3, replace: bool) -> Result<Vec<DMat4>> {
        if self.count == 0 || self.count > MAX_SCATTER {
            return Err(format!("Count must be 1 to {MAX_SCATTER}"));
        }
        let finite = [self.centre, self.range, self.scale, self.rotate]
            .iter()
            .all(|v| v.is_finite());
        if !finite || self.range.min_element() < 0. {
            return Err("Scatter values must be numbers, Range 0 or more".into());
        }
        let salt = self.seed as u64;
        let mut out = Vec::new();
        if replace {
            out.push(DMat4::from_translation(self.centre - centre_of_source));
        }
        for k in 1..self.count as u64 {
            let r = |c: u64| jitter(salt, k, c);
            let at = self.centre + self.range * DVec3::new(r(0), r(1), r(2));
            let scale = DVec3::ONE + self.scale * DVec3::new(r(3), r(4), r(5));
            if scale.min_element() <= 0. {
                return Err("Scale variation must stay below 100%".into());
            }
            let turn = euler(self.rotate * DVec3::new(r(6), r(7), r(8)));
            out.push(
                DMat4::from_translation(at)
                    * DMat4::from_quat(turn)
                    * DMat4::from_scale(scale)
                    * DMat4::from_translation(-centre_of_source),
            );
        }
        Ok(out)
    }
}

/// `count` points spread evenly over the unit square (`flat`) or cube:
/// each new point is the best of many seeded candidates, the one farthest
/// from those placed so far (Mitchell's best candidate; provisional, GO-Q4).
pub fn even_points(count: u32, flat: bool, seed: u64) -> Vec<DVec3> {
    let mut out: Vec<DVec3> = Vec::with_capacity(count as usize);
    for k in 0..count as u64 {
        let tries = (out.len() * 10).clamp(1, 200);
        let mut best = (f64::NEG_INFINITY, DVec3::ZERO);
        for t in 0..tries as u64 {
            let r = |c: u64| (jitter(seed, k * 1000 + t, c) + 1.) / 2.;
            let p = DVec3::new(r(0), if flat { 0. } else { r(1) }, r(2));
            let d = out
                .iter()
                .map(|q| q.distance_squared(p))
                .fold(f64::INFINITY, f64::min);
            if d > best.0 {
                best = (d, p);
            }
        }
        out.push(best.1);
    }
    out
}

/// The Scatter op's generator (GO-10..13).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScatterOp {
    /// Copies, the source included.
    pub number: u32,
    /// 0 Plane (2D), 1 Cube (3D).
    #[serde(rename = "type")]
    pub kind: u32,
    /// The plane's normal for 2D: 0 X, 1 Y, 2 Z.
    pub axis: u32,
    #[serde(rename = "sizeX")]
    pub size_x: f64,
    #[serde(rename = "sizeY")]
    pub size_y: f64,
    #[serde(rename = "sizeZ")]
    pub size_z: f64,
    /// Radians.
    #[serde(rename = "angH")]
    pub ang_h: f64,
    #[serde(rename = "angP")]
    pub ang_p: f64,
    #[serde(rename = "angB")]
    pub ang_b: f64,
    #[serde(rename = "varyH")]
    pub vary_h: f64,
    #[serde(rename = "varyP")]
    pub vary_p: f64,
    #[serde(rename = "varyB")]
    pub vary_b: f64,
    pub scale: f64,
    #[serde(rename = "varyS")]
    pub vary_s: f64,
    pub replace: bool,
    pub flip: bool,
    pub merge: bool,
    pub dist: f64,
}
impl Default for ScatterOp {
    fn default() -> Self {
        Self {
            number: 1,
            kind: 0,
            axis: 1,
            size_x: 1.,
            size_y: 1.,
            size_z: 1.,
            ang_h: 0.,
            ang_p: 0.,
            ang_b: 0.,
            vary_h: 0.,
            vary_p: 0.,
            vary_b: 0.,
            scale: 1.,
            vary_s: 0.,
            replace: false,
            flip: false,
            merge: false,
            dist: 0.,
        }
    }
}
impl ScatterOp {
    /// The region's points (GO-11): 2D = 0..Size in the plane normal to
    /// Axis, 3D = the 0..Size box. The first slot is the source's.
    pub fn points(&self) -> Result<Vec<DVec3>> {
        if self.number == 0 || self.number > MAX_SCATTER {
            return Err(format!("Count must be 1 to {MAX_SCATTER}"));
        }
        let size = DVec3::new(self.size_x, self.size_y, self.size_z);
        crate::mesh_stack_builtin::finite(&size.to_array(), "Size")?;
        let flat = self.kind == 0;
        let unit = even_points(self.number, flat, 0);
        // The unit square's second axis is Z; turn it into the plane.
        let place = |p: DVec3| -> DVec3 {
            if !flat {
                return p * size;
            }
            match self.axis {
                0 => DVec3::new(0., p.x * size.y, p.z * size.z),
                2 => DVec3::new(p.x * size.x, p.z * size.y, 0.),
                _ => DVec3::new(p.x * size.x, 0., p.z * size.z),
            }
        };
        Ok(unit.into_iter().map(place).collect())
    }
    /// The region's centre (Replace Source's place, GO-12).
    fn centre(&self) -> DVec3 {
        let size = DVec3::new(self.size_x, self.size_y, self.size_z);
        if self.kind != 0 {
            return size / 2.;
        }
        match self.axis {
            0 => DVec3::new(0., size.y, size.z) / 2.,
            2 => DVec3::new(size.x, size.y, 0.) / 2.,
            _ => DVec3::new(size.x, 0., size.z) / 2.,
        }
    }
}

/// The source polygons' bounding-box centre.
fn faces_centre(mesh: &Mesh, faces: &[usize]) -> DVec3 {
    crate::mesh_stack_builtin::faces_centre(mesh, faces)
}

impl TypedOp for ScatterOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let source: Vec<usize> = ctx
            .chosen_or_all()?
            .into_iter()
            .map(|f| f as usize)
            .collect();
        if source.is_empty() || self.number <= 1 && !self.replace {
            return Ok(ctx.input.clone());
        }
        let c = faces_centre(ctx.input, &source);
        let points = self.points()?;
        let base = DVec3::new(self.ang_p, self.ang_h, self.ang_b) * (180. / std::f64::consts::PI);
        let vary =
            DVec3::new(self.vary_p, self.vary_h, self.vary_b) * (180. / std::f64::consts::PI);
        let mut placed = Vec::new();
        if self.replace {
            placed.push(DMat4::from_translation(self.centre() - c));
        }
        for (k, &at) in points.iter().enumerate().skip(1) {
            let r = |ch: u64| jitter(0x5ca7, k as u64, ch);
            let turn = euler(base + vary * DVec3::new(r(0), r(1), r(2)));
            let s = self.scale + self.vary_s * r(3);
            if s <= 0. {
                return Err("Scale must stay above 0%".into());
            }
            placed.push(
                DMat4::from_translation(at)
                    * DMat4::from_quat(turn)
                    * DMat4::from_scale(DVec3::splat(s))
                    * DMat4::from_translation(-c),
            );
        }
        let copies = Copies {
            replace_source: self.replace,
            flip: self.flip,
            merge: self.merge,
            distance: self.dist,
            item: Default::default(),
        };
        let r = copy_faces(
            ctx.input,
            &source,
            &placed,
            &copies,
            WeldAt::Original,
            self.dist,
        )?;
        let mut out = r.mesh;
        let end = out.faces.len() as u32;
        ctx.tag(
            &mut out,
            "clone",
            end.saturating_sub(r.new_faces as u32)..end,
        );
        Ok(out)
    }
}

// ---- Surface Constraint ----

/// How a point finds the surface (GO-40).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConstraintMode {
    #[default]
    Directional,
    /// Built as Directional (GO-43, GO-Q2).
    HalfRay,
    Closest,
}

/// Surface Constraint's values (GO-40 defaults).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstraintOptions {
    pub mode: ConstraintMode,
    /// 0 X, 1 Y, 2 Z.
    pub axis: usize,
    pub invert: bool,
    pub offset: f64,
    /// 0 = unlimited.
    pub max_distance: f64,
    pub double_sided: bool,
    /// 1 = 100%.
    pub strength: f64,
}
impl Default for ConstraintOptions {
    fn default() -> Self {
        Self {
            mode: ConstraintMode::Directional,
            axis: 1,
            invert: false,
            offset: 0.,
            max_distance: 0.,
            double_sided: false,
            strength: 1.,
        }
    }
}

/// GO-41..46: `points` of `mesh` moved onto `surface` (same space).
/// Points whose search misses, or lands farther than Max Distance, stay.
pub fn constrain_points(
    mesh: &mut Mesh,
    points: &[u32],
    surface: &Background,
    o: &ConstraintOptions,
) -> Result<usize> {
    if ![o.offset, o.max_distance, o.strength]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err("Surface Constraint values must be numbers".into());
    }
    let mut dir = DVec3::ZERO;
    dir[o.axis.min(2)] = if o.invert { -1. } else { 1. };
    let mut moved = 0;
    for &v in points {
        let Some(&p) = mesh.vertices.get(v as usize) else {
            continue;
        };
        let hit = match o.mode {
            ConstraintMode::Closest => surface.nearest(p),
            ConstraintMode::Directional | ConstraintMode::HalfRay => {
                surface.ray(p, dir, o.double_sided).map(|(_, h)| h)
            }
        };
        let Some(hit) = hit else {
            continue;
        };
        if o.max_distance > 0. && hit.point.distance(p) > o.max_distance {
            continue;
        }
        let target = hit.point + hit.normal * o.offset;
        mesh.vertices[v as usize] = p + (target - p) * o.strength;
        moved += 1;
    }
    Ok(moved)
}

/// The points of `faces` (all points when `None`).
fn points_of(mesh: &Mesh, faces: Option<&[u32]>) -> Vec<u32> {
    match faces {
        None => (0..mesh.vertices.len() as u32).collect(),
        Some(faces) => {
            let mut v: Vec<u32> = faces
                .iter()
                .filter_map(|&f| mesh.faces.get(f as usize))
                .flat_map(|f| f.vertices.iter().copied())
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceConstraintOp {
    /// 0 Directional, 1 Half Ray, 2 Closest.
    pub mode: u32,
    pub axis: u32,
    pub invert: bool,
    pub offset: f64,
    #[serde(rename = "maxDistance")]
    pub max_distance: f64,
    #[serde(rename = "dblSided")]
    pub double_sided: bool,
    /// Not built: no vertex normal map is written yet.
    #[serde(rename = "saveNormal")]
    pub save_normal: bool,
    pub strength: f64,
    /// The constraint surface item (SAFE-08: 0 = none).
    pub surface: u64,
}
impl Default for SurfaceConstraintOp {
    fn default() -> Self {
        let o = ConstraintOptions::default();
        Self {
            mode: 0,
            axis: o.axis as u32,
            invert: o.invert,
            offset: o.offset,
            max_distance: o.max_distance,
            double_sided: o.double_sided,
            save_normal: false,
            strength: o.strength,
            surface: 0,
        }
    }
}
impl TypedOp for SurfaceConstraintOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let mut out = ctx.input.clone();
        // GO-47: only the linked surface counts. No link yet: unchanged.
        let Some(source) = ctx.sources()?.into_iter().next() else {
            return Ok(out);
        };
        let surface = Background::of_mesh(&source.mesh, source.to_local)?;
        let o = ConstraintOptions {
            mode: match self.mode {
                1 => ConstraintMode::HalfRay,
                2 => ConstraintMode::Closest,
                _ => ConstraintMode::Directional,
            },
            axis: self.axis.min(2) as usize,
            invert: self.invert,
            offset: self.offset,
            max_distance: self.max_distance,
            double_sided: self.double_sided,
            strength: self.strength,
        };
        let points = points_of(ctx.input, ctx.selection);
        constrain_points(&mut out, &points, &surface, &o)?;
        Ok(out)
    }
}

// ---- UV Rectangle ----

/// UV Rectangle's options (GO-21 defaults).
#[derive(Clone, Debug, PartialEq)]
pub struct RectangleOptions {
    pub map: String,
    pub pack: bool,
    /// Pack's gap, a fraction (0.2 = the pack tool's 20%).
    pub gaps: f64,
    pub uniform_x: bool,
    pub uniform_y: bool,
    /// Turn each island so its grid lines up with U and V.
    pub orient: bool,
}
impl Default for RectangleOptions {
    fn default() -> Self {
        Self {
            map: "Texture".into(),
            pack: false,
            gaps: 0.2,
            uniform_x: false,
            uniform_y: false,
            orient: false,
        }
    }
}

/// One straightened island and the islands left as they were.
#[derive(Clone, Debug, PartialEq)]
pub struct RectangleResult {
    pub mesh: Mesh,
    pub straightened: usize,
    /// Islands that aren't a full grid of quads (GO-20: left as they are).
    pub skipped: usize,
}

/// Grid coordinates for each point of a quad island, or `None` when it
/// isn't one full rectangular grid.
fn grid_of(mesh: &Mesh, faces: &[usize]) -> Option<BTreeMap<u32, (i64, i64)>> {
    if faces.iter().any(|&f| mesh.faces[f].vertices.len() != 4) {
        return None;
    }
    let set: BTreeSet<usize> = faces.iter().copied().collect();
    let mut by_edge: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for &f in faces {
        let v = &mesh.faces[f].vertices;
        for i in 0..4 {
            let (a, b) = (v[i], v[(i + 1) % 4]);
            by_edge.entry((a.min(b), a.max(b))).or_default().push(f);
        }
    }
    let mut at: BTreeMap<u32, (i64, i64)> = BTreeMap::new();
    let first = faces[0];
    let v = &mesh.faces[first].vertices;
    for (k, c) in [(0, 0), (1, 0), (1, 1), (0, 1)].into_iter().enumerate() {
        at.insert(v[k], c);
    }
    let mut done = BTreeSet::from([first]);
    let mut queue = vec![first];
    while let Some(f) = queue.pop() {
        let v = mesh.faces[f].vertices.clone();
        for i in 0..4 {
            let (a, b) = (v[i], v[(i + 1) % 4]);
            // The points before `a` and after `b` in this polygon.
            let (pa, pb) = (v[(i + 3) % 4], v[(i + 2) % 4]);
            for &g in by_edge.get(&(a.min(b), a.max(b)))? {
                if g == f || !set.contains(&g) || done.contains(&g) {
                    continue;
                }
                let w = &mesh.faces[g].vertices;
                // Across the edge, each end's other neighbour sits one step
                // further on, away from this polygon.
                for (end, inner) in [(a, pa), (b, pb)] {
                    let j = w.iter().position(|&x| x == end)?;
                    let other = if w[(j + 1) % 4] == a || w[(j + 1) % 4] == b {
                        w[(j + 3) % 4]
                    } else {
                        w[(j + 1) % 4]
                    };
                    let (e, n) = (at[&end], at[&inner]);
                    let want = (2 * e.0 - n.0, 2 * e.1 - n.1);
                    match at.get(&other) {
                        Some(&had) if had != want => return None,
                        _ => {
                            at.insert(other, want);
                        }
                    }
                }
                done.insert(g);
                queue.push(g);
            }
        }
    }
    if done.len() != faces.len() {
        return None;
    }
    // A full rectangle: one point per grid spot, every cell a polygon.
    let (lo_i, hi_i) = (
        at.values().map(|c| c.0).min()?,
        at.values().map(|c| c.0).max()?,
    );
    let (lo_j, hi_j) = (
        at.values().map(|c| c.1).min()?,
        at.values().map(|c| c.1).max()?,
    );
    let spots: BTreeSet<(i64, i64)> = at.values().copied().collect();
    let cells = ((hi_i - lo_i) * (hi_j - lo_j)) as usize;
    (spots.len() == at.len() && cells == faces.len()).then_some(at)
}

/// GO-20..24: each UV island of `faces` (all faces when empty) that is a
/// full grid of quads becomes an exact grid of rectangles; spans follow the
/// 3D edge lengths of their row or column (Uniform: all equal). The island
/// keeps its total UV area, its first corner and its average turn (Orient:
/// lined up with U). Pack then fits the islands into 0..1.
pub fn uv_rectangle(mesh: &Mesh, faces: &[u32], o: &RectangleOptions) -> Result<RectangleResult> {
    let rows = mesh
        .uv_maps
        .get(&o.map)
        .ok_or_else(|| format!("The mesh has no UV map called {}", o.map))?
        .clone();
    let wanted: Vec<usize> = if faces.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        faces.iter().map(|&f| f as usize).collect()
    };
    let wanted: Vec<usize> = wanted
        .into_iter()
        .filter(|&f| f < mesh.faces.len() && !mesh.is_polyline(f))
        .filter(|&f| rows[f].iter().all(Option::is_some))
        .collect();
    // Islands: polygons joined by an edge whose two ends have the same UVs
    // on both sides.
    let uv = |f: usize, c: usize| rows[f][c].unwrap_or_default();
    let key = |p: DVec2| ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64);
    let mut corner_at: HashMap<(u32, usize), usize> = HashMap::new();
    for &f in &wanted {
        for (c, &v) in mesh.faces[f].vertices.iter().enumerate() {
            corner_at.insert((v, f), c);
        }
    }
    let mut by_edge: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for &f in &wanted {
        let v = &mesh.faces[f].vertices;
        for i in 0..v.len() {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            by_edge.entry((a.min(b), a.max(b))).or_default().push(f);
        }
    }
    let mut island_of: HashMap<usize, usize> = HashMap::new();
    let mut islands: Vec<Vec<usize>> = Vec::new();
    for &f in &wanted {
        if island_of.contains_key(&f) {
            continue;
        }
        let id = islands.len();
        let mut members = vec![f];
        island_of.insert(f, id);
        let mut i = 0;
        while i < members.len() {
            let g = members[i];
            i += 1;
            let v = mesh.faces[g].vertices.clone();
            for k in 0..v.len() {
                let (a, b) = (v[k], v[(k + 1) % v.len()]);
                for &h in &by_edge[&(a.min(b), a.max(b))] {
                    if island_of.contains_key(&h) {
                        continue;
                    }
                    let same =
                        |p: u32| key(uv(g, corner_at[&(p, g)])) == key(uv(h, corner_at[&(p, h)]));
                    if same(a) && same(b) {
                        island_of.insert(h, id);
                        members.push(h);
                    }
                }
            }
        }
        islands.push(members);
    }
    let mut out = mesh.clone();
    let mut new_rows = rows.clone();
    let (mut straightened, mut skipped) = (0, 0);
    for island in &islands {
        let Some(grid) = grid_of(mesh, island) else {
            skipped += 1;
            continue;
        };
        // Span lengths along each column (i) and row (j) from 3D edges.
        let (lo_i, hi_i) = (
            grid.values().map(|c| c.0).min().unwrap(),
            grid.values().map(|c| c.0).max().unwrap(),
        );
        let (lo_j, hi_j) = (
            grid.values().map(|c| c.1).min().unwrap(),
            grid.values().map(|c| c.1).max().unwrap(),
        );
        let spot: HashMap<(i64, i64), u32> = grid.iter().map(|(&v, &c)| (c, v)).collect();
        let p = |c: (i64, i64)| mesh.vertices[spot[&c] as usize];
        let mut widths: Vec<f64> = (lo_i..hi_i)
            .map(|i| {
                (lo_j..=hi_j)
                    .map(|j| p((i, j)).distance(p((i + 1, j))))
                    .sum::<f64>()
                    / (hi_j - lo_j + 1) as f64
            })
            .collect();
        let mut heights: Vec<f64> = (lo_j..hi_j)
            .map(|j| {
                (lo_i..=hi_i)
                    .map(|i| p((i, j)).distance(p((i, j + 1))))
                    .sum::<f64>()
                    / (hi_i - lo_i + 1) as f64
            })
            .collect();
        let even = |v: &mut Vec<f64>| {
            let m = v.iter().sum::<f64>() / v.len().max(1) as f64;
            v.iter_mut().for_each(|x| *x = m);
        };
        if o.uniform_x {
            even(&mut widths);
        }
        if o.uniform_y {
            even(&mut heights);
        }
        let xs: Vec<f64> = std::iter::once(0.)
            .chain(widths.iter().scan(0., |s, w| {
                *s += w;
                Some(*s)
            }))
            .collect();
        let ys: Vec<f64> = std::iter::once(0.)
            .chain(heights.iter().scan(0., |s, h| {
                *s += h;
                Some(*s)
            }))
            .collect();
        let target =
            |c: (i64, i64)| DVec2::new(xs[(c.0 - lo_i) as usize], ys[(c.1 - lo_j) as usize]);
        // Corners of the island: (point, target grid spot, old UV).
        let corners: Vec<(usize, usize, DVec2, DVec2)> = island
            .iter()
            .flat_map(|&f| {
                let grid = &grid;
                mesh.faces[f]
                    .vertices
                    .iter()
                    .enumerate()
                    .map(move |(c, v)| (f, c, target(grid[v]), uv(f, c)))
            })
            .collect();
        // The best turn (and mirror) from the grid to the old UVs.
        let n = corners.len() as f64;
        let (gm, um) = corners.iter().fold((DVec2::ZERO, DVec2::ZERO), |a, c| {
            (a.0 + c.2 / n, a.1 + c.3 / n)
        });
        let (mut sxx, mut sxy, mut syx, mut syy) = (0., 0., 0., 0.);
        for c in &corners {
            let (g, u) = (c.2 - gm, c.3 - um);
            sxx += g.x * u.x;
            sxy += g.x * u.y;
            syx += g.y * u.x;
            syy += g.y * u.y;
        }
        let mirrored = sxx * syy - sxy * syx < 0.;
        let flip_y = |g: DVec2| if mirrored { DVec2::new(g.x, -g.y) } else { g };
        let (a, b) = if mirrored {
            (sxx - syy, sxy + syx)
        } else {
            (sxx + syy, sxy - syx)
        };
        let angle = if o.orient { 0. } else { b.atan2(a) };
        let rot = DMat3::from_angle(angle);
        // Keep the island's total UV area (GO-24, GO-Q1).
        // Corners are listed polygon by polygon, in order.
        let starts: Vec<(usize, usize)> = island
            .iter()
            .scan(0, |at, &f| {
                let m = mesh.faces[f].vertices.len();
                let start = *at;
                *at += m;
                Some((start, m))
            })
            .collect();
        let area = |pts: &dyn Fn(usize) -> DVec2| -> f64 {
            starts
                .iter()
                .map(|&(start, m)| {
                    (0..m)
                        .map(|c| pts(start + c).perp_dot(pts(start + (c + 1) % m)))
                        .sum::<f64>()
                        .abs()
                        / 2.
                })
                .sum()
        };
        let old_area = area(&|k| corners[k].3);
        let grid_area = area(&|k| corners[k].2);
        let scale = if grid_area > 0. && old_area > 0. {
            (old_area / grid_area).sqrt()
        } else {
            1.
        };
        let place = |g: DVec2| rot.transform_vector2(flip_y(g)) * scale;
        // The island's first corner stays where it was.
        let anchor = corners[0].3 - place(corners[0].2);
        for &(f, c, g, _) in &corners {
            new_rows[f][c] = Some(anchor + place(g));
        }
        straightened += 1;
    }
    out.uv_maps.insert(o.map.clone(), new_rows);
    if o.pack && straightened > 0 {
        let options = crate::uv_tools::PackOptions {
            gaps: o.gaps,
            ..Default::default()
        };
        let selected: Vec<u32> = wanted.iter().map(|&f| f as u32).collect();
        out = crate::uv_tools::pack_mesh(&out, &o.map, &[], &selected, &options)?;
    }
    Ok(RectangleResult {
        mesh: out,
        straightened,
        skipped,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UvRectangleOp {
    pub name: String,
    pub pack: bool,
    pub gaps: f64,
    #[serde(rename = "uniformX")]
    pub uniform_x: bool,
    #[serde(rename = "uniformY")]
    pub uniform_y: bool,
    pub orient: bool,
}
impl Default for UvRectangleOp {
    fn default() -> Self {
        let o = RectangleOptions::default();
        Self {
            name: o.map,
            pack: o.pack,
            gaps: o.gaps,
            uniform_x: o.uniform_x,
            uniform_y: o.uniform_y,
            orient: o.orient,
        }
    }
}
impl UvRectangleOp {
    pub fn options(&self) -> RectangleOptions {
        RectangleOptions {
            map: self.name.clone(),
            pack: self.pack,
            gaps: self.gaps,
            uniform_x: self.uniform_x,
            uniform_y: self.uniform_y,
            orient: self.orient,
        }
    }
}
impl TypedOp for UvRectangleOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        if !ctx.input.uv_maps.contains_key(&self.name) {
            return Ok(ctx.input.clone());
        }
        let faces = ctx.chosen()?;
        Ok(uv_rectangle(ctx.input, &faces, &self.options())?.mesh)
    }
}

// ---- Transfer Vertex Map ----

/// What a transfer copies, in the reference's order (GO-31), which is also
/// the op's saved `type` index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransferType {
    /// The target points move to the sampled source positions (GO-35).
    #[default]
    Position,
    Weight,
    /// Not built: Operandi keeps no subdivision weight maps.
    SubdivisionWeight,
    Uv,
    RelativeMorph,
    /// Not built: Operandi keeps no absolute morph maps.
    AbsoluteMorph,
    /// A colour map's red, green and blue; the target keeps its alpha.
    Rgb,
    Rgba,
    /// The source's shading normals (its vertex normal map, else its
    /// smoothing) onto every corner of the target point (G9 VN-8).
    VertexNormal,
}
impl TransferType {
    pub const NAMES: [&'static str; 9] = [
        "Position",
        "Weight",
        "Subdivision Weight",
        "UV",
        "Relative Morph",
        "Absolute Morph",
        "RGB",
        "RGBA",
        "Vertex Normal",
    ];
    fn from_index(i: u32) -> Self {
        match i {
            1 => Self::Weight,
            2 => Self::SubdivisionWeight,
            3 => Self::Uv,
            4 => Self::RelativeMorph,
            5 => Self::AbsoluteMorph,
            6 => Self::Rgb,
            7 => Self::Rgba,
            8 => Self::VertexNormal,
            _ => Self::Position,
        }
    }
}

/// How a target point finds its spot on the source (GO-31 order: Raycast,
/// Distance).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransferSampling {
    /// Along the target point's normal (both ways); a miss leaves the
    /// point without a value (GO-34).
    Raycast,
    /// The nearest spot on the source, so points past its edge take the
    /// edge's value (GO-33).
    #[default]
    Distance,
}

/// The target points' normals: the average of their polygons'.
fn point_normals(mesh: &Mesh) -> Vec<DVec3> {
    let mut sums = vec![DVec3::ZERO; mesh.vertices.len()];
    for f in 0..mesh.faces.len() {
        if mesh.is_polyline(f) {
            continue;
        }
        let n = mesh.normal(f);
        for &v in &mesh.faces[f].vertices {
            sums[v as usize] += n;
        }
    }
    sums.into_iter().map(DVec3::normalize_or_zero).collect()
}

/// The source triangle the line through `p` along `dir` crosses nearest
/// `p`, and the barycentric weights of the crossing.
fn ray_on(
    source: &Mesh,
    tris: &[(usize, [usize; 3])],
    p: DVec3,
    dir: DVec3,
) -> Option<(usize, [usize; 3], DVec3)> {
    let mut best: Option<(f64, usize, [usize; 3], DVec3)> = None;
    for &(f, t) in tris {
        let [a, b, c] = t.map(|c| source.vertices[source.faces[f].vertices[c] as usize]);
        // Möller–Trumbore, both directions along the line.
        let (e1, e2) = (b - a, c - a);
        let h = dir.cross(e2);
        let det = e1.dot(h);
        if det.abs() < 1e-14 {
            continue;
        }
        let s = p - a;
        let u = s.dot(h) / det;
        let q = s.cross(e1);
        let v = dir.dot(q) / det;
        let eps = 1e-9;
        if u < -eps || v < -eps || u + v > 1. + eps {
            continue;
        }
        let t_hit = e2.dot(q) / det;
        if best.is_none_or(|b| t_hit.abs() < b.0) {
            best = Some((t_hit.abs(), f, t, DVec3::new(1. - u - v, u, v)));
        }
    }
    best.map(|(_, f, t, w)| (f, t, w))
}

/// The source triangle nearest `p` and the barycentric weights of the
/// nearest spot on it.
fn nearest_on(
    source: &Mesh,
    tris: &[(usize, [usize; 3])],
    p: DVec3,
) -> Option<(usize, [usize; 3], DVec3)> {
    let mut best: Option<(f64, usize, [usize; 3], DVec3)> = None;
    for &(f, t) in tris {
        let [a, b, c] = t.map(|c| source.vertices[source.faces[f].vertices[c] as usize]);
        let w = closest_barycentric(p, a, b, c);
        let q = a * w.x + b * w.y + c * w.z;
        let d = q.distance_squared(p);
        if best.is_none_or(|b| d < b.0) {
            best = Some((d, f, t, w));
        }
    }
    best.map(|(_, f, t, w)| (f, t, w))
}

/// Barycentric weights of the point of triangle abc nearest `p`.
fn closest_barycentric(p: DVec3, a: DVec3, b: DVec3, c: DVec3) -> DVec3 {
    // Ericson, Real-Time Collision Detection, 5.1.5.
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0. && d2 <= 0. {
        return DVec3::X;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0. && d4 <= d3 {
        return DVec3::Y;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        let v = d1 / (d1 - d3);
        return DVec3::new(1. - v, v, 0.);
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0. && d5 <= d6 {
        return DVec3::Z;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        let w = d2 / (d2 - d6);
        return DVec3::new(1. - w, 0., w);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return DVec3::new(0., 1. - w, w);
    }
    let denom = 1. / (va + vb + vc);
    let (v, w) = (vb * denom, vc * denom);
    DVec3::new(1. - v - w, v, w)
}

/// GO-30..35: for each of `points` of `target`, its spot on the source
/// surface (`sampling`; both meshes already in the space the Sampling
/// Space asks for), and the source map's value there, interpolated across
/// the source polygon (GO-32), written into the target map of the same
/// name. Returns how many points got a value.
pub fn transfer_vertex_map(
    target: &mut Mesh,
    points: &[u32],
    source: &Mesh,
    kind: TransferType,
    map: &str,
    sampling: TransferSampling,
) -> Result<usize> {
    if matches!(
        kind,
        TransferType::SubdivisionWeight | TransferType::AbsoluteMorph
    ) {
        return Err(format!(
            "Transfer Vertex Map: {} maps aren't in Operandi yet",
            TransferType::NAMES[kind as usize]
        ));
    }
    let normals = point_normals(target);
    let mut tris = Vec::new();
    for f in 0..source.faces.len() {
        if source.is_polyline(f) {
            continue;
        }
        for t in source.triangles(f)? {
            tris.push((f, t));
        }
    }
    if tris.is_empty() {
        return Err("Transfer Vertex Map: the source has no polygons".into());
    }
    let sa = &source.attributes;
    // G9 VN-8: the source's corner normals, read once.
    let source_normals = (kind == TransferType::VertexNormal)
        .then(|| crate::shading::mesh_corner_normals(source, &Default::default()));
    match kind {
        TransferType::Position => {}
        TransferType::RelativeMorph if !sa.relative_morphs.contains_key(map) => {
            return Err(format!("The source has no morph map called {map}"));
        }
        TransferType::Weight if !sa.point_weights.contains_key(map) => {
            return Err(format!("The source has no weight map called {map}"));
        }
        TransferType::Rgba | TransferType::Rgb if !sa.point_colors.contains_key(map) => {
            return Err(format!("The source has no colour map called {map}"));
        }
        TransferType::Uv if !source.uv_maps.contains_key(map) => {
            return Err(format!("The source has no UV map called {map}"));
        }
        _ => {}
    }
    let mut done = 0;
    for &v in points {
        let Some(&p) = target.vertices.get(v as usize) else {
            continue;
        };
        let found = match sampling {
            TransferSampling::Distance => nearest_on(source, &tris, p),
            TransferSampling::Raycast => {
                let n = normals.get(v as usize).copied().unwrap_or_default();
                if n == DVec3::ZERO {
                    None
                } else {
                    ray_on(source, &tris, p, n)
                }
            }
        };
        let Some((f, t, w)) = found else {
            continue;
        };
        let ids = t.map(|c| source.faces[f].vertices[c]);
        let blend = |get: &dyn Fn(u32) -> Option<glam::DVec4>| -> Option<glam::DVec4> {
            let vals: Vec<glam::DVec4> = ids.iter().map(|&i| get(i)).collect::<Option<_>>()?;
            Some(vals[0] * w.x + vals[1] * w.y + vals[2] * w.z)
        };
        let ta = &mut target.attributes;
        match kind {
            TransferType::Position => {
                let q = ids.map(|i| source.vertices[i as usize]);
                target.vertices[v as usize] = q[0] * w.x + q[1] * w.y + q[2] * w.z;
            }
            TransferType::RelativeMorph => {
                let m = &sa.relative_morphs[map];
                let d = blend(&|i| Some(m.get(&i).copied().unwrap_or_default().extend(0.)))
                    .unwrap_or_default()
                    .truncate();
                ta.relative_morphs
                    .entry(map.into())
                    .or_default()
                    .insert(v, d);
            }
            TransferType::Weight => {
                let m = &sa.point_weights[map];
                let x = blend(&|i| Some(glam::DVec4::splat(m.get(&i).copied().unwrap_or(0.))))
                    .unwrap_or_default()
                    .x;
                ta.point_weights.entry(map.into()).or_default().insert(v, x);
            }
            TransferType::Rgba | TransferType::Rgb => {
                let m = &sa.point_colors[map];
                let mut c = blend(&|i| Some(m.get(&i).copied().unwrap_or(glam::DVec4::ONE)))
                    .unwrap_or(glam::DVec4::ONE);
                let colours = ta.point_colors.entry(map.into()).or_default();
                if kind == TransferType::Rgb {
                    c.w = colours.get(&v).map_or(1., |old| old.w);
                }
                colours.insert(v, c);
            }
            TransferType::SubdivisionWeight | TransferType::AbsoluteMorph => {
                unreachable!("refused above")
            }
            TransferType::VertexNormal => {
                let rows = source_normals.as_ref().expect("read above");
                let n = t.map(|c| rows[f].get(c).copied().unwrap_or_default());
                let n = (n[0] * w.x + n[1] * w.y + n[2] * w.z).normalize_or_zero();
                if n == DVec3::ZERO {
                    continue;
                }
                let faces: Vec<(usize, usize)> = target
                    .faces
                    .iter()
                    .enumerate()
                    .flat_map(|(g, face)| {
                        face.vertices
                            .iter()
                            .enumerate()
                            .filter(|(_, x)| **x == v)
                            .map(move |(c, _)| (g, c))
                    })
                    .collect();
                let shapes: Vec<usize> = target.faces.iter().map(|f| f.vertices.len()).collect();
                let polylines: Vec<bool> = (0..target.faces.len())
                    .map(|g| target.is_polyline(g))
                    .collect();
                let ta = &mut target.attributes;
                let rows = ta
                    .corner_normals
                    .entry(map.into())
                    .or_insert_with(|| shapes.iter().map(|&k| vec![None; k]).collect());
                for (g, c) in faces {
                    if !polylines[g] {
                        rows[g][c] = Some(n);
                    }
                }
                if !ta.normal_map_order.iter().any(|m| m == map) {
                    ta.normal_map_order.insert(0, map.into());
                }
            }
            TransferType::Uv => {
                let rows = &source.uv_maps[map];
                let uv = t.map(|c| rows[f][c].unwrap_or_default());
                let at = uv[0] * w.x + uv[1] * w.y + uv[2] * w.z;
                // Every corner of the target point in that map takes it.
                let n = target.faces.len();
                let target_rows = target
                    .uv_maps
                    .entry(map.into())
                    .or_insert_with(|| vec![Vec::new(); n]);
                for (g, face) in target.faces.iter().enumerate() {
                    let row = &mut target_rows[g];
                    if row.len() != face.vertices.len() {
                        *row = vec![None; face.vertices.len()];
                    }
                    for (c, &x) in face.vertices.iter().enumerate() {
                        if x == v {
                            row[c] = Some(at);
                        }
                    }
                }
            }
        }
        done += 1;
    }
    Ok(done)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferVertexMapOp {
    /// The source (and target) map's name; Position needs none.
    pub source: String,
    /// TransferType index, in the reference's order (GO-31).
    #[serde(rename = "type")]
    pub kind: u32,
    /// Sampling Space: 0 Local (the items' own coordinates, placements
    /// ignored), 1 World (where they sit) (GO-33). World by default
    /// (provisional, GO-Q1).
    #[serde(default = "world_space")]
    pub space: u32,
    /// Sampling Method: 0 Raycast, 1 Distance (GO-34). Distance by
    /// default (provisional, GO-Q1).
    #[serde(default = "distance_method")]
    pub method: u32,
    /// Not built: 0 Off.
    pub flip: u32,
    /// Not built (UV seams).
    pub completion: bool,
    /// The source mesh item (SAFE-08: 0 = none).
    pub item: u64,
}
fn world_space() -> u32 {
    1
}
fn distance_method() -> u32 {
    1
}
impl Default for TransferVertexMapOp {
    fn default() -> Self {
        Self {
            source: String::new(),
            kind: 0,
            space: world_space(),
            method: distance_method(),
            flip: 0,
            completion: true,
            item: 0,
        }
    }
}
impl TypedOp for TransferVertexMapOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let mut out = ctx.input.clone();
        let Some(source) = ctx.sources()?.into_iter().next() else {
            return Ok(out);
        };
        // GO-33: World places the source where it sits relative to this
        // item; Local reads both in their own coordinates.
        let placed = if self.space == 0 {
            source.placed_mesh(glam::DMat4::IDENTITY)
        } else {
            source.local_mesh()
        };
        let points = points_of(ctx.input, ctx.selection);
        transfer_vertex_map(
            &mut out,
            &points,
            &placed,
            TransferType::from_index(self.kind),
            &self.source,
            if self.method == 0 {
                TransferSampling::Raycast
            } else {
                TransferSampling::Distance
            },
        )?;
        Ok(out)
    }
}

// ---- UV Fit as an op (UnSubdivide's op is on the Mesh Stack branch) ----

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UvFitOp {
    pub name: String,
    /// 0 Entire, 1 Island, 2 Polygon (the Fit command's grouping).
    pub sepa: u32,
    pub keep: bool,
    /// Fraction (0.1 = 10%).
    pub gaps: f64,
    pub udim: u32,
    /// Not built: gaps in pixels.
    #[serde(rename = "byPixel")]
    pub by_pixel: bool,
    #[serde(rename = "gapsByPixel")]
    pub gaps_by_pixel: f64,
    #[serde(rename = "imageSize")]
    pub image_size: u32,
}
impl Default for UvFitOp {
    fn default() -> Self {
        Self {
            name: "Texture".into(),
            sepa: 0,
            keep: true,
            gaps: 0.,
            udim: 1001,
            by_pixel: false,
            gaps_by_pixel: 10.,
            image_size: 1024,
        }
    }
}
impl TypedOp for UvFitOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let mut out = ctx.input.clone();
        if !out.uv_maps.contains_key(&self.name) {
            return Ok(out);
        }
        use crate::uv_edit::{FitOptions, Grouping, IslandOp, IslandRule};
        let selection = crate::Selection {
            mode: crate::Mode::Polygon,
            faces: ctx.chosen()?,
            ..Default::default()
        };
        let plan = crate::uv_edit::plan(&out, &self.name, &selection, &[], false)?;
        let op = IslandOp {
            rule: IslandRule::Fit(FitOptions {
                keep_proportion: self.keep,
                gaps: self.gaps * 100.,
                udim: self.udim,
            }),
            grouping: match self.sepa {
                1 => Grouping::Island,
                2 => Grouping::Polygon,
                _ => Grouping::Entire,
            },
        };
        crate::uv_edit::apply_island_op(&mut out, &plan, &op)?;
        Ok(out)
    }
}

// ---- the table ----

fn field(key: &'static str, label: &'static str, kind: FieldKind) -> Field {
    Field::new(key, label, kind)
}
fn choice(key: &'static str, label: &'static str, names: &[&'static str]) -> Field {
    field(
        key,
        label,
        FieldKind::Choice(names.iter().map(|n| (*n).into()).collect()),
    )
}
fn hidden(key: &'static str, label: &'static str) -> Field {
    field(key, label, FieldKind::Hidden)
}

const CLONE_GROUP: &[(&str, &str)] = &[("clone", "Cloned Polygons")];

/// The batch 5 ops, for the MeshOp type table.
pub(crate) fn entries() -> Vec<Arc<dyn OpDef>> {
    let check = |k, l| field(k, l, FieldKind::Check);
    let dist = |k, l| field(k, l, FieldKind::Distance);
    let angle = |k, l| field(k, l, FieldKind::Angle);
    vec![
        Arc::new(
            Typed::<ScatterOp>::new(
                "scatter.clone.item",
                "Scatter Clone",
                "Duplicate",
                true,
                vec![
                    field(
                        "number",
                        "Count",
                        FieldKind::Count {
                            min: 1,
                            max: MAX_SCATTER,
                        },
                    ),
                    choice("type", "Type", &["Plane", "Cube"]),
                    choice("axis", "Axis", &["X", "Y", "Z"]),
                    dist("sizeX", "Size X"),
                    dist("sizeY", "Size Y"),
                    dist("sizeZ", "Size Z"),
                    angle("angP", "Rotate X"),
                    angle("angH", "Rotate Y"),
                    angle("angB", "Rotate Z"),
                    angle("varyP", "Vary Rotation X"),
                    angle("varyH", "Vary Rotation Y"),
                    angle("varyB", "Vary Rotation Z"),
                    field("scale", "Scale", FieldKind::Percent),
                    field("varyS", "Vary Scale", FieldKind::Percent),
                    check("replace", "Replace Source"),
                    check("flip", "Invert Polygons"),
                    check("merge", "Merge Vertices"),
                    dist("dist", "Distance"),
                ],
            )
            .with_groups(CLONE_GROUP),
        ),
        Arc::new(Typed::<SurfaceConstraintOp>::new(
            "surfaceConstraint.meshop.item",
            "Surface Constraint",
            "Deform",
            true,
            vec![
                field("surface", "Constraint Surface", FieldKind::Item),
                choice("mode", "Mode", &["Directional", "Half Ray", "Closest"]),
                choice("axis", "Axis", &["X", "Y", "Z"]),
                check("invert", "Invert"),
                dist("offset", "Offset"),
                dist("maxDistance", "Max Distance"),
                check("dblSided", "Double Sided"),
                hidden("saveNormal", "Save Surface Normal to Vertex Normal Map"),
                field("strength", "Strength", FieldKind::Percent),
            ],
        )),
        Arc::new(Typed::<UvRectangleOp>::new(
            "uvrect.meshop.item",
            "UV Rectangle",
            "UV / Vertex Map",
            true,
            vec![
                field("name", "Name", FieldKind::Text),
                check("pack", "Pack"),
                field("gaps", "Gaps", FieldKind::Percent),
                check("uniformX", "Uniform X-Spans"),
                check("uniformY", "Uniform Y-Spans"),
                check("orient", "Orient UV Islands"),
            ],
        )),
        Arc::new(Typed::<TransferVertexMapOp>::new(
            "vertexmaptransfer",
            "Transfer Vertex Map",
            "UV / Vertex Map",
            true,
            vec![
                field("item", "Source Item", FieldKind::Item),
                field("source", "Source Vertex Map", FieldKind::Text),
                choice("type", "Type", &TransferType::NAMES),
                choice("space", "Sampling Space", &["Local", "World"]),
                choice("method", "Sampling Method", &["Raycast", "Distance"]),
                hidden("flip", "Flip UVs"),
                hidden("completion", "UV Completion"),
            ],
        )),
        Arc::new(Typed::<UvFitOp>::new(
            "uvfit.meshop.item",
            "UV Fit",
            "UV / Vertex Map",
            true,
            vec![
                field("name", "Name", FieldKind::Text),
                choice("sepa", "Fit", &["Entire", "Island", "Polygon"]),
                check("keep", "Keep Proportion"),
                field("gaps", "Gaps", FieldKind::Percent),
                field(
                    "udim",
                    "UDIM",
                    FieldKind::Count {
                        min: 1001,
                        max: 1999,
                    },
                ),
                hidden("byPixel", "Set Gaps by Pixel"),
                hidden("gapsByPixel", "Pixel Size"),
                hidden("imageSize", "Texel Density Image Size"),
            ],
        )),
    ]
}

/// The Scatter Clone tool's settings (GO-01: the Scatter generator plus the
/// Clone effector).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScatterTool {
    pub generator: ScatterBox,
    pub copies: Copies,
}

impl crate::Editor {
    /// GO-20..24 UV Rectangle on the selected polygons (all when none), one
    /// undo step. Says how many islands weren't a full quad grid.
    pub(crate) fn uv_rectangle_command(&mut self, options: RectangleOptions) -> Result<()> {
        let item = self.edge_insert_target("UV Rectangle")?;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let faces = if self.scene.selection.mode == crate::Mode::Polygon {
            self.scene.selection.faces.clone()
        } else {
            Vec::new()
        };
        let r = uv_rectangle(mesh, &faces, &options)?;
        if r.straightened == 0 {
            return Err(format!(
                "UV Rectangle: no island is a full grid of quads ({} left as they are)",
                r.skipped
            ));
        }
        self.command_note = (r.skipped > 0).then(|| {
            format!(
                "UV Rectangle: {} island(s) aren't a full grid of quads and were left as they are",
                r.skipped
            )
        });
        let keep = self.scene.selection.faces.clone();
        self.publish_polygon_edit(item, r.mesh, "UV Rectangle", keep)
    }

    /// GO-30..32 Transfer Vertex Map (provisional): from the first shown
    /// background mesh onto the active mesh's selected points (all when
    /// none), Distance sampling in world space. One undo step.
    pub(crate) fn transfer_vertex_map_command(
        &mut self,
        map: String,
        kind: TransferType,
        world: bool,
        sampling: TransferSampling,
    ) -> Result<()> {
        let item = self.edge_insert_target("Transfer Vertex Map")?;
        let source_id = crate::background_constraint::background_ids(&self.scene)
            .into_iter()
            .find(|&id| id != item)
            .ok_or("Transfer Vertex Map: show a background mesh to copy from")?;
        let source = self.scene.item(source_id).ok_or("Missing item")?;
        let to_local = if world {
            self.scene.world_matrix(item)?.inverse() * self.scene.world_matrix(source_id)?
        } else {
            glam::DMat4::IDENTITY
        };
        let placed = crate::duplicate::transformed_copy(&source.mesh, to_local, false);
        let mut mesh = self
            .scene
            .active()
            .ok_or("Select a mesh")?
            .mesh
            .to_owned_value();
        let points = match self.scene.selection.mode {
            crate::Mode::Vertex if !self.scene.selection.vertices.is_empty() => {
                self.scene.selection.vertices.clone()
            }
            crate::Mode::Polygon if !self.scene.selection.faces.is_empty() => {
                points_of(&mesh, Some(&self.scene.selection.faces))
            }
            _ => (0..mesh.vertices.len() as u32).collect(),
        };
        transfer_vertex_map(&mut mesh, &points, &placed, kind, &map, sampling)?;
        let keep = self.scene.selection.faces.clone();
        self.publish_polygon_edit(item, mesh, "Transfer Vertex Map", keep)
    }
}

impl TransferType {
    /// The `type` argument's word.
    pub fn named(word: &str) -> Result<Self> {
        Ok(match word.to_ascii_lowercase().as_str() {
            "position" => Self::Position,
            "morph" | "relativemorph" | "relative morph" => Self::RelativeMorph,
            "weight" => Self::Weight,
            "rgba" | "color" | "colour" => Self::Rgba,
            "rgb" => Self::Rgb,
            "uv" => Self::Uv,
            "normal" | "vertexnormal" | "vertex normal" => Self::VertexNormal,
            other => return Err(format!("Transfer Vertex Map doesn't know type {other}")),
        })
    }
}
