//! Bind, Unbind and Rebind (BEHAVIOR-RIG-PHASE5 §3, SK-20..25): weights
//! for each joint of a skeleton from the mesh's shape, one weight map per
//! joint (named after it), and the skin piece (skin.rs) in the mesh's
//! stack. Every command is one undo step and needs Setup mode (SK-02).
//!
//! Bind Types (SK-21):
//! - Rigid: each point 100% to its nearest bone.
//! - Smooth (Distance): graded by distance to the bones; Falloff sets how
//!   quickly a farther bone's share fades.
//! - Smooth (Visible): Distance, counting only bones a point sees in a
//!   straight line through the inside of the mesh.
//! - Heat: weights diffused over the surface ("bone heat", Baran and
//!   Popović 2007, a public method): for each bone, solve
//!   (L + M·H) w = M·H·p, with L the cotangent Laplacian (weights clamped
//!   at 0, so it stays positive semi-definite), M the point areas, H the
//!   heat each point gets from its nearest visible bone (1/d²) and p 1 where
//!   that bone is the nearest. Solved by conjugate gradients (our own; no
//!   outside solver). Needs a closed volume with no loose or non-manifold
//!   parts and the joints inside.
//!
//! A bone is a joint's line to each child (SK-22: a joint with several
//! children is one bone towards all of them); a leaf joint, or Bind as
//! Segments off, weights by distance to the joint itself.
// Point-indexed loops read several parallel arrays at once.
#![allow(clippy::needless_range_loop)]
use crate::bake::bvh::Bvh;
use crate::mesh_stack::{MeshOp, MeshStack, OpKind};
use crate::skin::{self, SKIN_OP, SkinValues};
use crate::skin_commands::{skin_op, write_skin};
use crate::{Command, Editor, Mesh, Mode, Result, Scene};
use glam::{DMat4, DVec3, Vec3};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

type Args = Map<String, Value>;

/// SK-02's refusal.
pub const NEEDS_SETUP: &str = "Bind needs Setup mode. Turn on Setup and try again.";
/// SK-21's overlapping shells message.
pub const OVERLAPPING_SHELLS: &str = "Heat bind failed: overlapping shells; bind the main body only, move eyes/clothes to other items";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindKind {
    Rigid,
    Distance,
    Visible,
    Heat,
}

impl BindKind {
    pub fn parse(s: &str) -> Result<BindKind> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "rigid" => BindKind::Rigid,
            "distance" | "smooth" | "smoothdistance" => BindKind::Distance,
            "visible" | "smoothvisible" => BindKind::Visible,
            "heat" => BindKind::Heat,
            other => {
                return Err(format!(
                    "type is rigid, distance, visible or heat, not {other}"
                ));
            }
        })
    }
}

/// The Bind dialog's options (SK-20).
#[derive(Clone, Debug)]
pub struct BindOptions {
    pub kind: BindKind,
    /// Falloff: 0 Low (soft), 1 Medium, 2 High (sharp).
    pub falloff: u32,
    pub segments: bool,
    pub limit: bool,
    pub per_vertex: u32,
    pub minimum: f64,
    pub dual_quaternion: bool,
}

impl Default for BindOptions {
    fn default() -> Self {
        BindOptions {
            kind: BindKind::Heat,
            falloff: 1,
            segments: true,
            limit: false,
            per_vertex: 4,
            minimum: 0.,
            dual_quaternion: false,
        }
    }
}

impl BindOptions {
    /// How fast a farther bone's share fades (provisional, SK-Q09).
    fn power(&self) -> f64 {
        [2., 4., 8.][self.falloff.min(2) as usize]
    }
}

/// A failed bind: why, and the points to select so they can be fixed.
#[derive(Clone, Debug, Default)]
pub struct BindFailure {
    pub message: String,
    pub points: Vec<u32>,
}

/// One bone in the mesh's own space.
#[derive(Clone, Debug)]
pub struct Bone {
    pub joint: u64,
    pub at: DVec3,
    /// Its children's positions; empty for a leaf (a radius, SK-22).
    pub ends: Vec<DVec3>,
}

impl Bone {
    /// The nearest point of the bone to `p`.
    pub fn nearest(&self, p: DVec3) -> DVec3 {
        let mut best = self.at;
        let mut d = p.distance_squared(self.at);
        for &e in &self.ends {
            let ab = e - self.at;
            let len = ab.length_squared();
            let t = if len < 1e-24 {
                0.
            } else {
                ((p - self.at).dot(ab) / len).clamp(0., 1.)
            };
            let q = self.at + ab * t;
            let dq = p.distance_squared(q);
            if dq < d {
                d = dq;
                best = q;
            }
        }
        best
    }
}

/// `root`'s joints depth first, as bones in mesh `item`'s space.
pub fn bones(scene: &Scene, item: u64, root: u64, segments: bool) -> Result<Vec<Bone>> {
    let into = scene.world_matrix(item)?.inverse();
    let at = |j: u64| -> DVec3 {
        let w = scene.world_matrix(j).unwrap_or(DMat4::IDENTITY);
        into.transform_point3(w.w_axis.truncate())
    };
    let mut out = Vec::new();
    let mut todo = vec![root];
    while let Some(j) = todo.pop() {
        let kids = crate::skeleton::children(scene, j);
        out.push(Bone {
            joint: j,
            at: at(j),
            ends: if segments {
                kids.iter().map(|&k| at(k)).collect()
            } else {
                Vec::new()
            },
        });
        todo.extend(kids.into_iter().rev());
    }
    Ok(out)
}

fn triangles(mesh: &Mesh) -> Vec<[u32; 3]> {
    let mut out = Vec::new();
    for f in &mesh.faces {
        let v = &f.vertices;
        for k in 1..v.len().saturating_sub(1) {
            out.push([v[0], v[k], v[k + 1]]);
        }
    }
    out
}

fn bvh(mesh: &Mesh, tris: &[[u32; 3]]) -> Bvh {
    let p = |i: u32| mesh.vertices[i as usize].as_vec3();
    Bvh::new(tris.iter().map(|t| [p(t[0]), p(t[1]), p(t[2])]).collect())
}

/// Whether the straight line from `p` to `q` crosses no surface (a small
/// gap at `p` skips the point's own polygons).
fn sees(bvh: &Bvh, p: DVec3, q: DVec3, gap: f64) -> bool {
    let d = q - p;
    let len = d.length();
    if len <= gap * 2. {
        return true;
    }
    let dir = (d / len).as_vec3();
    let from = (p + d / len * gap).as_vec3();
    bvh.closest(from, dir, (len - gap * 2.) as f32).is_none()
}

/// Weights per bone (index into `bones`): point → weight, normalised per
/// point, after Limit Weights and Minimum Weight (SK-20).
pub fn weights(
    mesh: &Mesh,
    bones: &[Bone],
    o: &BindOptions,
) -> std::result::Result<Vec<BTreeMap<u32, f64>>, BindFailure> {
    if bones.is_empty() {
        return Err(BindFailure {
            message: "Pick a root joint to bind to".into(),
            points: Vec::new(),
        });
    }
    let used = mesh.used_points();
    let n = mesh.vertices.len();
    let tris = triangles(mesh);
    let needs_bvh = matches!(o.kind, BindKind::Visible | BindKind::Heat);
    let bvh = needs_bvh.then(|| bvh(mesh, &tris));
    let size = bounds(mesh).map_or(1., |(lo, hi)| (hi - lo).length().max(1e-9));
    let gap = size * 1e-5;
    let visible = |p: DVec3, b: &Bone| bvh.as_ref().is_none_or(|t| sees(t, p, b.nearest(p), gap));
    let mut per_point: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    match o.kind {
        BindKind::Rigid | BindKind::Distance | BindKind::Visible => {
            for p in 0..n {
                if !used[p] {
                    continue;
                }
                let v = mesh.vertices[p];
                let d: Vec<f64> = bones
                    .iter()
                    .map(|b| v.distance(b.nearest(v)).max(1e-12))
                    .collect();
                let counts = |j: usize| o.kind != BindKind::Visible || visible(v, &bones[j]);
                let mut live: Vec<usize> = (0..bones.len()).filter(|&j| counts(j)).collect();
                if live.is_empty() {
                    // Seeing no bone: the nearest takes it (provisional).
                    live = (0..bones.len()).collect();
                }
                // Ties go to the later bone: a child (a leaf's radius) wins
                // over its parent's line that ends at it.
                let near = live
                    .iter()
                    .copied()
                    .reduce(|best, j| {
                        if d[j] <= d[best] * (1. + 1e-9) {
                            j
                        } else {
                            best
                        }
                    })
                    .expect("bones");
                if o.kind == BindKind::Rigid {
                    per_point[p].push((near, 1.));
                    continue;
                }
                let k = o.power();
                for j in live {
                    let w = (d[near] / d[j]).powf(k);
                    if w > 1e-4 {
                        per_point[p].push((j, w));
                    }
                }
            }
        }
        BindKind::Heat => {
            heat_check(mesh, bones, &used, bvh.as_ref().expect("built"))?;
            let solved = heat(mesh, &tris, bones, &used, bvh.as_ref().expect("built"), gap);
            for (j, w) in solved.into_iter().enumerate() {
                for (p, w) in w.into_iter().enumerate() {
                    if used[p] && w > 1e-4 {
                        per_point[p].push((j, w.min(1.)));
                    }
                }
            }
        }
    }
    let mut out = vec![BTreeMap::new(); bones.len()];
    for (p, mut slot) in per_point.into_iter().enumerate() {
        if slot.is_empty() {
            continue;
        }
        slot.sort_by(|a, b| b.1.total_cmp(&a.1));
        if o.limit {
            slot.truncate(o.per_vertex.clamp(1, 8) as usize);
        }
        let total: f64 = slot.iter().map(|s| s.1).sum();
        slot.iter_mut().for_each(|s| s.1 /= total);
        if o.minimum > 0. {
            let strongest = slot[0];
            slot.retain(|s| s.1 >= o.minimum);
            if slot.is_empty() {
                slot.push(strongest);
            }
            let total: f64 = slot.iter().map(|s| s.1).sum();
            slot.iter_mut().for_each(|s| s.1 /= total);
        }
        for (j, w) in slot {
            out[j].insert(p as u32, crate::rig::clean(w));
        }
    }
    Ok(out)
}

fn bounds(mesh: &Mesh) -> Option<(DVec3, DVec3)> {
    let mut it = mesh.vertices.iter();
    let first = *it.next()?;
    Some(it.fold((first, first), |(lo, hi), v| (lo.min(*v), hi.max(*v))))
}

/// SK-21: what Heat needs, or what's wrong (with the points to fix).
fn heat_check(
    mesh: &Mesh,
    bones: &[Bone],
    used: &[bool],
    bvh: &Bvh,
) -> std::result::Result<(), BindFailure> {
    let fail = |message: String, points: Vec<u32>| Err(BindFailure { message, points });
    let loose: Vec<u32> = (0..mesh.vertices.len() as u32)
        .filter(|&p| !used[p as usize])
        .collect();
    if !loose.is_empty() {
        return fail(
            format!(
                "Heat bind failed: {} loose points; remove them (Mesh Cleanup)",
                loose.len()
            ),
            loose,
        );
    }
    let open: Vec<u32> = mesh
        .faces
        .iter()
        .filter(|f| f.vertices.len() < 3)
        .flat_map(|f| f.vertices.iter().copied())
        .collect();
    if !open.is_empty() {
        return fail(
            "Heat bind failed: the mesh has loose edges or curves; remove them".into(),
            open,
        );
    }
    let bad: Vec<u32> = mesh
        .edge_counts()
        .iter()
        .filter(|(_, c)| **c > 2)
        .flat_map(|((a, b), _)| [*a, *b])
        .collect();
    if !bad.is_empty() {
        return fail(
            "Heat bind failed: non-manifold edges (more than two polygons on an edge)".into(),
            bad,
        );
    }
    // Shells whose boxes overlap (eyes in a head, clothes on a body).
    let shells = shells(mesh);
    if shells.len() > 1 {
        let boxes: Vec<(DVec3, DVec3)> = shells
            .iter()
            .map(|s| {
                s.iter()
                    .fold((DVec3::INFINITY, DVec3::NEG_INFINITY), |(lo, hi), &p| {
                        let v = mesh.vertices[p as usize];
                        (lo.min(v), hi.max(v))
                    })
            })
            .collect();
        for a in 0..boxes.len() {
            for b in a + 1..boxes.len() {
                let (la, ha) = boxes[a];
                let (lb, hb) = boxes[b];
                if la.cmple(hb).all() && lb.cmple(ha).all() {
                    let small = if shells[a].len() < shells[b].len() {
                        a
                    } else {
                        b
                    };
                    return fail(OVERLAPPING_SHELLS.into(), shells[small].clone());
                }
            }
        }
    }
    // Joints inside: a ray from each crosses the surface an odd number of
    // times (provisional, SK-Q09).
    for b in bones {
        if !inside(bvh, b.at) {
            return fail(
                "Heat bind failed: a joint is outside the mesh; move the joints inside".into(),
                Vec::new(),
            );
        }
    }
    Ok(())
}

fn inside(bvh: &Bvh, p: DVec3) -> bool {
    let dir = Vec3::new(0.577_35, 0.577_35, 0.577_35);
    let mut from = p.as_vec3();
    let mut crossings = 0;
    for _ in 0..1000 {
        let Some(hit) = bvh.closest(from, dir, f32::MAX) else {
            break;
        };
        crossings += 1;
        from += dir * (hit.t + 1e-5);
    }
    crossings % 2 == 1
}

/// Each connected set of polygons' points.
fn shells(mesh: &Mesh) -> Vec<Vec<u32>> {
    let n = mesh.vertices.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for f in &mesh.faces {
        for w in f.vertices.windows(2) {
            let (a, b) = (
                find(&mut parent, w[0] as usize),
                find(&mut parent, w[1] as usize),
            );
            parent[a] = b;
        }
    }
    let mut groups: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    let used = mesh.used_points();
    for p in 0..n {
        if used[p] {
            let r = find(&mut parent, p);
            groups.entry(r).or_default().push(p as u32);
        }
    }
    groups.into_values().collect()
}

/// Bone heat: one solve per bone (see the module notes).
fn heat(
    mesh: &Mesh,
    tris: &[[u32; 3]],
    bones: &[Bone],
    used: &[bool],
    bvh: &Bvh,
    gap: f64,
) -> Vec<Vec<f64>> {
    let n = mesh.vertices.len();
    let v = &mesh.vertices;
    // Cotangent weights (clamped at 0) and point areas.
    let mut off: Vec<BTreeMap<u32, f64>> = vec![BTreeMap::new(); n];
    let mut area = vec![0f64; n];
    for t in tris {
        let [a, b, c] = t.map(|i| i as usize);
        let tri_area = (v[b] - v[a]).cross(v[c] - v[a]).length() * 0.5;
        for i in [a, b, c] {
            area[i] += tri_area / 3.;
        }
        for (i, j, k) in [(a, b, c), (b, c, a), (c, a, b)] {
            // The angle at k faces edge i–j.
            let e1 = v[i] - v[k];
            let e2 = v[j] - v[k];
            let cross = e1.cross(e2).length();
            if cross < 1e-18 {
                continue;
            }
            let cot = (e1.dot(e2) / cross).max(0.) * 0.5;
            *off[i].entry(j as u32).or_default() += cot;
            *off[j].entry(i as u32).or_default() += cot;
        }
    }
    let diag_l: Vec<f64> = off.iter().map(|row| row.values().sum()).collect();
    // Each point's nearest visible bone and its heat.
    let mut nearest = vec![usize::MAX; n];
    let mut h = vec![0f64; n];
    for p in 0..n {
        if !used[p] {
            continue;
        }
        let mut best = (f64::INFINITY, usize::MAX);
        for (j, b) in bones.iter().enumerate() {
            let d = v[p].distance(b.nearest(v[p]));
            // Ties go to the later bone, as in `weights`.
            if d <= best.0 * (1. + 1e-9) && sees(bvh, v[p], b.nearest(v[p]), gap) {
                best = (d, j);
            }
        }
        if best.1 != usize::MAX {
            nearest[p] = best.1;
            h[p] = 1. / best.0.max(gap).powi(2);
        }
    }
    let diag: Vec<f64> = (0..n).map(|p| diag_l[p] + area[p] * h[p]).collect();
    let apply = |x: &[f64], y: &mut [f64]| {
        for p in 0..n {
            let mut s = diag[p] * x[p];
            for (&q, &w) in &off[p] {
                s -= w * x[q as usize];
            }
            y[p] = s;
        }
    };
    let mut out = Vec::with_capacity(bones.len());
    for j in 0..bones.len() {
        let b: Vec<f64> = (0..n)
            .map(|p| if nearest[p] == j { area[p] * h[p] } else { 0. })
            .collect();
        out.push(conjugate_gradient(&apply, &diag, &b, n));
    }
    out
}

/// Jacobi-preconditioned conjugate gradients for the symmetric positive
/// (semi-)definite system `apply(x) = b`. Rows with a zero diagonal (loose
/// points) stay 0.
fn conjugate_gradient(
    apply: &dyn Fn(&[f64], &mut [f64]),
    diag: &[f64],
    b: &[f64],
    n: usize,
) -> Vec<f64> {
    let inv: Vec<f64> = diag
        .iter()
        .map(|d| if *d > 1e-300 { 1. / d } else { 0. })
        .collect();
    let mut x = vec![0f64; n];
    let mut r = b.to_vec();
    let mut z: Vec<f64> = r.iter().zip(&inv).map(|(r, i)| r * i).collect();
    let mut p = z.clone();
    let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
    let norm_b: f64 = b.iter().map(|v| v * v).sum::<f64>().sqrt();
    if norm_b == 0. {
        return x;
    }
    let mut ap = vec![0f64; n];
    for _ in 0..(4 * n).clamp(50, 4000) {
        apply(&p, &mut ap);
        let pap: f64 = p.iter().zip(&ap).map(|(a, b)| a * b).sum();
        if pap.abs() < 1e-300 {
            break;
        }
        let alpha = rz / pap;
        for k in 0..n {
            x[k] += alpha * p[k];
            r[k] -= alpha * ap[k];
        }
        let norm_r: f64 = r.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm_r < 1e-9 * norm_b {
            break;
        }
        for k in 0..n {
            z[k] = r[k] * inv[k];
        }
        let rz_new: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
        let beta = rz_new / rz;
        rz = rz_new;
        for k in 0..n {
            p[k] = z[k] + beta * p[k];
        }
    }
    x
}

// ------------------------------------------------------------ commands

fn ids(a: &Args, name: &str) -> Result<Option<Vec<u64>>> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => Ok(Some(vec![
            n.as_u64().ok_or(format!("{name} are item ids"))?,
        ])),
        Some(Value::String(s)) => s
            .split(',')
            .map(|p| p.trim().parse().map_err(|_| format!("{name} are item ids")))
            .collect::<Result<Vec<u64>>>()
            .map(Some),
        Some(Value::Array(list)) => list
            .iter()
            .map(|v| v.as_u64().ok_or_else(|| format!("{name} are item ids")))
            .collect::<Result<Vec<u64>>>()
            .map(Some),
        Some(_) => Err(format!("{name} are item ids")),
    }
}

fn flag(a: &Args, name: &str, default: bool) -> Result<bool> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(b)) => Ok(*b),
        Some(Value::String(s)) => crate::command_args::boolean(name, s),
        Some(_) => Err(format!("{name} must be true or false")),
    }
}

fn number(a: &Args, name: &str) -> Result<Option<f64>> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => Ok(n.as_f64()),
        Some(Value::String(s)) => s
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| format!("{name} is a number")),
        Some(_) => Err(format!("{name} is a number")),
    }
}

/// The options from a bind line's arguments (SK-20 defaults).
pub fn options(a: &Args) -> Result<BindOptions> {
    let mut o = BindOptions::default();
    if let Some(t) = a.get("type").and_then(Value::as_str) {
        o.kind = BindKind::parse(t)?;
    }
    if let Some(f) = a.get("falloff").and_then(Value::as_str) {
        o.falloff = match f.to_ascii_lowercase().as_str() {
            "low" => 0,
            "medium" => 1,
            "high" => 2,
            other => return Err(format!("falloff is low, medium or high, not {other}")),
        };
    }
    o.segments = flag(a, "segments", true)?;
    o.limit = flag(a, "limitWeights", false)?;
    if let Some(n) = number(a, "weightsPerVertex")? {
        if !(1. ..=8.).contains(&n) || n.fract() != 0. {
            return Err("weightsPerVertex is 1 to 8".into());
        }
        o.per_vertex = n as u32;
    }
    if let Some(m) = number(a, "minimumWeight")? {
        if !(0. ..=1.).contains(&m) {
            return Err("minimumWeight is 0 to 1".into());
        }
        o.minimum = m;
    }
    o.dual_quaternion = flag(a, "dualQuaternion", false)?;
    Ok(o)
}

fn is_joint(scene: &Scene, id: u64) -> bool {
    scene.item(id).is_some_and(|i| i.is_joint())
}

/// The meshes (`items`, else the selected meshes) and the root (`root`,
/// else the last selected joint).
fn targets(scene: &Scene, a: &Args) -> Result<(Vec<u64>, Option<u64>)> {
    let selected = scene.selected_items();
    let meshes = match ids(a, "items")? {
        Some(m) => m,
        None => selected
            .iter()
            .copied()
            .filter(|&i| scene.item(i).is_some_and(|i| i.is_mesh()))
            .collect(),
    };
    for &m in &meshes {
        if !scene.item(m).is_some_and(|i| i.is_mesh()) {
            return Err(format!("{m} isn't a mesh item"));
        }
    }
    if meshes.is_empty() {
        return Err("Select the mesh items, then the root joint".into());
    }
    let root = match ids(a, "root")? {
        Some(r) => r.first().copied(),
        None => selected.iter().rev().copied().find(|&i| is_joint(scene, i)),
    };
    if let Some(r) = root
        && !is_joint(scene, r)
    {
        return Err(format!("{r} isn't a joint"));
    }
    Ok((meshes, root))
}

/// The handler for `operandi.skin.bind`, `unbind` and `rebind`.
pub fn run(e: &mut Editor, name: &str, a: &Args) -> Result<Option<Command>> {
    if !e.scene.animation.setup {
        return Err(NEEDS_SETUP.into());
    }
    let (meshes, root) = targets(&e.scene, a)?;
    match name {
        "operandi.skin.bind" => {
            let root = root.ok_or("Select the mesh items, then the root joint")?;
            let o = options(a)?;
            for &m in &meshes {
                if skin_op(&e.scene, m).is_ok() {
                    return Err(format!(
                        "{} is bound already; use Rebind to add joints",
                        e.scene.item(m).expect("checked").name
                    ));
                }
            }
            let mut failure: Option<(u64, BindFailure)> = None;
            let result = e.anim_step("Bind", |e| {
                for &m in &meshes {
                    let bones = bones(&e.scene, m, root, o.segments)?;
                    let mesh = e.scene.item(m).expect("checked").mesh.open().clone();
                    let w = match weights(&mesh, &bones, &o) {
                        Ok(w) => w,
                        Err(f) => {
                            let message = f.message.clone();
                            failure = Some((m, f));
                            return Err(message);
                        }
                    };
                    bind_mesh(&mut e.scene, m, &bones, w, &o)?;
                }
                Ok(true)
            });
            if let Err(message) = result {
                // SK-21: nothing changes, but the bad points are selected.
                if let Some((m, f)) = failure
                    && !f.points.is_empty()
                {
                    let s = &mut e.scene;
                    s.select_item(Some(m));
                    s.selection.mode = Mode::Vertex;
                    s.selection.vertices = f.points;
                }
                return Err(message);
            }
            Ok(None)
        }
        "operandi.skin.unbind" => {
            let keep = flag(a, "keepMaps", true)?;
            for &m in &meshes {
                skin_op(&e.scene, m)?;
            }
            e.anim_step("Unbind", |e| {
                for &m in &meshes {
                    let (op, v) = skin_op(&e.scene, m)?;
                    let item = e.scene.item_mut(m).expect("checked");
                    if let Some(stack) = item.mesh_stack.as_mut() {
                        stack.ops.retain(|o| o.id != op);
                        if stack.ops.is_empty() {
                            item.mesh_stack = None;
                        }
                    }
                    if !keep {
                        let mut mesh = item.mesh.open().clone();
                        for map in &v.maps {
                            mesh.attributes.point_weights.remove(map);
                        }
                        item.mesh = mesh.into();
                    }
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.skin.rebind" => {
            // SK-25: joints not yet bound join with empty maps.
            let joints: Vec<u64> = match ids(a, "joints")? {
                Some(j) => j,
                None => e
                    .scene
                    .selected_items()
                    .into_iter()
                    .filter(|&i| is_joint(&e.scene, i))
                    .collect(),
            };
            if joints.is_empty() {
                return Err("Select the joints to add".into());
            }
            for &j in &joints {
                if !is_joint(&e.scene, j) {
                    return Err(format!("{j} isn't a joint"));
                }
            }
            e.anim_step("Rebind", |e| {
                let mut changed = false;
                for &m in &meshes {
                    let (op, mut v) = skin_op(&e.scene, m)?;
                    let item = e.scene.item_mut(m).expect("checked");
                    let mut mesh = item.mesh.open().clone();
                    let mut names = Vec::new();
                    for &j in &joints {
                        if v.joints.contains(&j) {
                            continue;
                        }
                        names.push(j);
                    }
                    for j in names {
                        let name = e.scene.item(j).expect("checked").name.clone();
                        mesh.attributes
                            .point_weights
                            .entry(name.clone())
                            .or_default();
                        v.add(j, &name);
                        skin::ensure_setup_channels(&mut e.scene, j);
                        changed = true;
                    }
                    e.scene.item_mut(m).expect("checked").mesh = mesh.into();
                    write_skin(&mut e.scene, m, op, &v)?;
                }
                Ok(changed)
            })?;
            Ok(None)
        }
        _ => Err(format!("Unknown bind command {name}")),
    }
}

/// SK-23: the maps, the skin piece at the bottom of the stack, and setup
/// values on every joint (its bind pose).
fn bind_mesh(
    scene: &mut Scene,
    m: u64,
    bones: &[Bone],
    w: Vec<BTreeMap<u32, f64>>,
    o: &BindOptions,
) -> Result<()> {
    let mut values = SkinValues {
        interpolation: if o.dual_quaternion {
            skin::DUAL_QUATERNION
        } else {
            skin::LOCAL
        },
        ..Default::default()
    };
    if o.limit {
        values.limit_sources = true;
        values.max_sources = o.per_vertex.clamp(1, 8);
    }
    let names: Vec<String> = bones
        .iter()
        .map(|b| {
            scene
                .item(b.joint)
                .map(|i| i.name.clone())
                .unwrap_or_default()
        })
        .collect();
    let item = scene.item_mut(m).ok_or("No mesh")?;
    let mut mesh = item.mesh.open().clone();
    for (k, map) in w.into_iter().enumerate() {
        mesh.attributes.point_weights.insert(names[k].clone(), map);
        values.add(bones[k].joint, &names[k]);
    }
    item.mesh = mesh.into();
    insert_skin(scene, m, &values)?;
    for b in bones {
        skin::ensure_setup_channels(scene, b.joint);
    }
    Ok(())
}

/// Puts a skin piece with `values` at the bottom of mesh `m`'s stack
/// (SK-36: morphs, then the skin, then other deformers).
pub fn insert_skin(scene: &mut Scene, m: u64, values: &SkinValues) -> Result<()> {
    let item = scene.item_mut(m).ok_or("No mesh")?;
    let stack = item.mesh_stack.get_or_insert_with(MeshStack::default);
    let id = stack.next_id;
    stack.next_id += 1;
    stack.ops.insert(
        0,
        MeshOp {
            id,
            enabled: true,
            kind: OpKind::Op {
                type_id: SKIN_OP.into(),
                values: serde_json::to_value(values).map_err(|e| e.to_string())?,
            },
            selection: None,
            selops: Vec::new(),
        },
    );
    Ok(())
}
