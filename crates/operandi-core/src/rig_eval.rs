//! Rig evaluation (RG-01..06, EVAL-GRAPH-PROPOSAL steps 3, 4, 6, 7): the
//! world matrix of every item a rig piece moves, in dependency order.
//!
//! Per item (RG-01): its own local channels (as posed) → local matrix
//! (with its pivot) → parent or dynamic parent → world → its constraints,
//! in order, on the world result → its children. A piece reading another
//! item sees that item's evaluated world.
//!
//! Loops (RG-02): a piece whose target depends on the piece's own item
//! (directly or not) is broken and evaluates as disabled, so a loaded loop
//! still evaluates; commands refuse to make one ([`would_loop`]).
//!
//! Results are held by the engine, never written into the scene (step 6):
//! [`RigCache`] keeps each item's world with a fingerprint of everything it
//! read (its own transform, pivot and pieces, the time and the worlds of
//! the items it reads). An item is recomputed only when that fingerprint
//! changes, so only what lies downstream of a change is evaluated again
//! (RG-06), and a cached world can never go stale: the fingerprint is
//! compared against the actual inputs each time (the plan's stale-view
//! check, built in).
use crate::Scene;
use crate::animation::ChannelKey;
use crate::rig::Piece;
use glam::{DMat4, DQuat, DVec3, EulerRot};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// The evaluated worlds of the items a rig moves, and the pieces broken by
/// a loop.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RigResult {
    pub world: HashMap<u64, DMat4>,
    pub broken: BTreeSet<u64>,
    /// Channel values links and modifiers set (RG-50..63), in the order
    /// they were evaluated.
    pub driven: Vec<(ChannelKey, f64)>,
    /// Items whose world a parent-space or constraint piece sets (and
    /// their children): the evaluated scene places these at `world`;
    /// the others follow their (driven) channels through the hierarchy.
    pub placed: HashSet<u64>,
}

/// Per-item memo (eval_state.rs keeps one per document).
#[derive(Clone, Debug, Default)]
pub struct RigCache {
    items: HashMap<u64, (u64, DMat4)>,
    /// Items computed (not taken from the memo) so far; tests and the
    /// playback probe count work with it.
    pub computed: u64,
}

/// What `item`'s world reads: its static parent or dynamic parents, and
/// the targets and up items of its enabled pieces.
fn dependencies(scene: &Scene, item: u64, broken: &BTreeSet<u64>) -> Vec<u64> {
    let mut out = Vec::new();
    let has_dynamic = scene
        .animation
        .rig
        .on(item)
        .any(|p| p.kind == "dynamicParent" && p.flag("enable") && !broken.contains(&p.id));
    if !has_dynamic && let Some(parent) = scene.item(item).and_then(|i| i.parent) {
        out.push(parent);
    }
    for p in scene.animation.rig.on(item) {
        if p.is_missing() || !p.flag("enable") || broken.contains(&p.id) || channel_piece(p) {
            continue;
        }
        out.extend(p.targets.iter().copied());
        out.extend(p.refs.values().copied());
    }
    out.retain(|d| scene.item(*d).is_some());
    out
}

/// The items whose world the rig changes: items with enabled pieces and
/// everything below them (static children follow their parent).
fn affected(scene: &Scene, seeds: impl IntoIterator<Item = u64>) -> HashSet<u64> {
    let mut out: HashSet<u64> = scene
        .animation
        .rig
        .pieces
        .iter()
        .filter(|p| {
            !p.is_missing() && !channel_piece(p) && p.flag("enable") && scene.item(p.item).is_some()
        })
        .map(|p| p.item)
        .chain(seeds)
        .collect();
    if out.is_empty() {
        return out;
    }
    // Children of affected items, to a fixed point (parent links are
    // acyclic: set_parent refuses loops).
    loop {
        let before = out.len();
        for i in &scene.items {
            if let Some(p) = i.parent
                && out.contains(&p)
            {
                out.insert(i.id);
            }
        }
        if out.len() == before {
            return out;
        }
    }
}

/// The pieces a loop breaks (RG-02): every piece one of whose targets
/// depends, through any chain of parents and pieces, on the piece's own
/// item.
pub fn broken_pieces(scene: &Scene) -> BTreeSet<u64> {
    let mut broken = BTreeSet::new();
    let none = BTreeSet::new();
    for p in &scene.animation.rig.pieces {
        if p.is_missing() || !p.flag("enable") {
            continue;
        }
        if channel_piece(p) {
            if channel_loop(scene, p) {
                broken.insert(p.id);
            }
            continue;
        }
        let reads: Vec<u64> = p.targets.iter().chain(p.refs.values()).copied().collect();
        if reads
            .iter()
            .any(|&t| t == p.item || depends_on(scene, t, p.item, &none))
        {
            broken.insert(p.id);
        }
    }
    broken
}

/// Whether `from`'s world reads `on`'s, through parents and pieces.
fn depends_on(scene: &Scene, from: u64, on: u64, broken: &BTreeSet<u64>) -> bool {
    let mut seen = HashSet::new();
    let mut stack = vec![from];
    while let Some(n) = stack.pop() {
        if n == on {
            return true;
        }
        if !seen.insert(n) {
            continue;
        }
        stack.extend(dependencies(scene, n, broken));
    }
    false
}

/// Refuses an edit that would make an item depend on itself (RG-02): `None`
/// when `after` has no loop, else the plain message naming the item.
pub fn would_loop(after: &Scene) -> Option<String> {
    let broken = broken_pieces(after);
    let piece = broken.iter().next()?;
    let p = after.animation.rig.piece(*piece)?;
    let name = after
        .item(p.item)
        .map_or_else(|| format!("item {}", p.item), |i| i.name.clone());
    Some(format!("This would make {name} depend on itself"))
}

/// Evaluates the rig of `scene` at its current time (fresh, no memo).
pub fn evaluate(scene: &Scene) -> RigResult {
    evaluate_cached(scene, &mut RigCache::default())
}

/// [`evaluate`] with a memo: unchanged items come from `cache`.
pub fn evaluate_cached(scene: &Scene, cache: &mut RigCache) -> RigResult {
    let broken = broken_pieces(scene);
    // Links and modifiers first (RG-01): they feed channels the local
    // matrices read.
    let (working, driven) = channels_stage(scene, &broken);
    let scene = working.as_ref().unwrap_or(scene);
    let moved = driven
        .iter()
        .filter(|(k, _)| {
            k.transform_part().is_some() || crate::rig::pivot_part(&k.channel).is_some()
        })
        .map(|(k, _)| k.item)
        .filter(|i| scene.item(*i).is_some());
    let moved: Vec<u64> = moved.collect();
    let placed = affected(scene, Vec::new());
    let affected = affected(scene, moved);
    let mut out = RigResult {
        broken,
        driven,
        placed,
        ..Default::default()
    };
    if affected.is_empty() {
        cache.items.clear();
        return out;
    }
    let mut eval = Eval {
        scene,
        affected: &affected,
        broken: &out.broken,
        world: HashMap::new(),
        cache,
        overrides: HashMap::new(),
    };
    // Scene order, each item after what it reads (`world` recurses).
    let mut ids: Vec<u64> = scene
        .items
        .iter()
        .filter(|i| affected.contains(&i.id))
        .map(|i| i.id)
        .collect();
    ids.sort_unstable();
    for id in &ids {
        eval.world_of(*id, 0);
    }
    eval.cache.items.retain(|id, _| affected.contains(id));
    out.world = ids.iter().map(|id| (*id, eval.world[id])).collect();
    out
}

struct Eval<'a> {
    scene: &'a Scene,
    affected: &'a HashSet<u64>,
    broken: &'a BTreeSet<u64>,
    world: HashMap<u64, DMat4>,
    cache: &'a mut RigCache,
    /// Worlds IK solvers set for their chain's joints below the root
    /// (ik.rs), taken instead of the joint's own.
    overrides: HashMap<u64, DMat4>,
}

fn hash_mat(m: &DMat4, h: &mut impl Hasher) {
    for v in m.to_cols_array() {
        v.to_bits().hash(h);
    }
}

impl Eval<'_> {
    /// An item's evaluated world. Items the rig doesn't touch keep the
    /// document's world (targets and sun aim included).
    fn world_of(&mut self, id: u64, depth: u32) -> DMat4 {
        if let Some(w) = self.world.get(&id) {
            return *w;
        }
        if !self.affected.contains(&id) || depth > 512 {
            let w = self.scene.world_matrix(id).unwrap_or(DMat4::IDENTITY);
            self.world.insert(id, w);
            return w;
        }
        let deps = dependencies(self.scene, id, self.broken);
        let dep_worlds: Vec<(u64, DMat4)> = deps
            .iter()
            .map(|&d| (d, self.world_of(d, depth + 1)))
            .collect();
        let Some(item) = self.scene.item(id) else {
            return DMat4::IDENTITY;
        };
        let local = self.scene.local_matrix(item);
        let pieces: Vec<&Piece> = self
            .scene
            .animation
            .rig
            .on(id)
            .filter(|p| !p.is_missing() && p.flag("enable") && !self.broken.contains(&p.id))
            .collect();
        // Everything this world reads, for the memo.
        let mut h = std::collections::hash_map::DefaultHasher::new();
        hash_mat(&local, &mut h);
        item.parent.hash(&mut h);
        for (d, w) in &dep_worlds {
            d.hash(&mut h);
            hash_mat(w, &mut h);
        }
        for p in &pieces {
            p.id.hash(&mut h);
            p.kind.hash(&mut h);
            p.targets.hash(&mut h);
            for (k, v) in &p.values {
                k.hash(&mut h);
                v.to_bits().hash(&mut h);
            }
            for (k, v) in &p.refs {
                k.hash(&mut h);
                v.hash(&mut h);
            }
            // Paths read the curves' shapes.
            if p.kind.starts_with("path") {
                for t in &p.targets {
                    if let Some(c) = self.scene.item(*t) {
                        c.mesh.stamp().hash(&mut h);
                    }
                }
            }
        }
        let key = h.finish();
        // An IK chain's joints come from the solve on its root, which
        // reads more than this hash holds: neither uses the memo.
        if let Some(w) = self.overrides.get(&id).copied() {
            self.world.insert(id, w);
            return w;
        }
        let solves = pieces.iter().any(|p| crate::ik::is_ik(&p.kind));
        if !solves
            && let Some((k, w)) = self.cache.items.get(&id)
            && *k == key
        {
            let w = *w;
            self.world.insert(id, w);
            return w;
        }
        self.cache.computed += 1;
        let worlds: HashMap<u64, DMat4> = dep_worlds.into_iter().collect();
        let look = |t: u64| worlds.get(&t).copied().unwrap_or(DMat4::IDENTITY);
        // Parent space: a dynamic parent (RG-20) or the static parent.
        let dynamic = pieces.iter().find(|p| p.kind == "dynamicParent");
        let mut w = match dynamic {
            Some(dp) => {
                let index = dp.num("parent").round() as usize;
                let parent = match index {
                    0 => DMat4::IDENTITY,
                    n => dp.targets.get(n - 1).map_or(DMat4::IDENTITY, |&t| look(t)),
                };
                parent * dp.offset_matrix() * local
            }
            None => match item.parent {
                Some(p) => look(p) * local,
                None => local,
            },
        };
        for p in pieces
            .iter()
            .filter(|p| p.kind != "dynamicParent" && !crate::ik::is_ik(&p.kind))
        {
            w = constrain(self.scene, p, w, &look);
        }
        // IK last (ik.rs): it turns the root and sets its chain below.
        let parent = item.parent.map_or(DMat4::IDENTITY, look);
        let ik_on = !self.scene.animation.rig.ik_disabled;
        for p in pieces.iter().filter(|p| ik_on && crate::ik::is_ik(&p.kind)) {
            let l = crate::ik::Look {
                scene: self.scene,
                world: &look,
                parent,
            };
            w = crate::ik::solve(p, w, &l, &mut self.overrides);
        }
        if !w.is_finite() {
            w = self.scene.world_matrix(id).unwrap_or(DMat4::IDENTITY);
        }
        self.cache.items.insert(id, (key, w));
        self.world.insert(id, w);
        w
    }
}

/// Normalised weights (RG-28); `None` when they sum to nothing (the item
/// keeps its own transform).
fn weights(p: &Piece) -> Option<Vec<f64>> {
    let w: Vec<f64> = (0..p.targets.len())
        .map(|i| p.num(&format!("weight.{i}")).max(0.))
        .collect();
    let sum: f64 = w.iter().sum();
    (sum > 0.).then(|| w.iter().map(|v| v / sum).collect())
}

fn trs(m: DMat4) -> (DVec3, DQuat, DVec3) {
    let (s, q, t) = m.to_scale_rotation_translation();
    (s, q.normalize(), t)
}

/// A weighted, sign-aligned, normalised quaternion average (RG-41).
fn blend_quats(qs: &[(DQuat, f64)]) -> DQuat {
    let first = qs.first().map_or(DQuat::IDENTITY, |q| q.0);
    let mut sum = glam::DVec4::ZERO;
    for (q, w) in qs {
        let q = if q.dot(first) < 0. { -*q } else { *q };
        sum += glam::DVec4::from(q) * *w;
    }
    if sum.length_squared() < 1e-24 {
        return first;
    }
    DQuat::from_vec4(sum).normalize()
}

fn euler_quat(e: DVec3) -> DQuat {
    DQuat::from_euler(EulerRot::XYZ, e.x, e.y, e.z)
}

/// Affect X/Y/Z and Influence (RG-31) between the item's own world parts
/// and the constrained ones.
fn mix(
    p: &Piece,
    own: (DVec3, DQuat, DVec3),
    new: (DVec3, DQuat, DVec3),
    parts: [bool; 3],
) -> DMat4 {
    let affect = [p.flag("affect.X"), p.flag("affect.Y"), p.flag("affect.Z")];
    let influence = p.num("influence").clamp(0., 1.);
    let pick = |a: DVec3, b: DVec3| {
        DVec3::new(
            if affect[0] { b.x } else { a.x },
            if affect[1] { b.y } else { a.y },
            if affect[2] { b.z } else { a.z },
        )
    };
    let (os, oq, ot) = own;
    let (ns, nq, nt) = new;
    let s = if parts[2] {
        os.lerp(pick(os, ns), influence)
    } else {
        os
    };
    let q = if parts[1] {
        let q = if affect.iter().all(|a| *a) {
            nq
        } else {
            let (ox, oy, oz) = oq.to_euler(EulerRot::XYZ);
            let (nx, ny, nz) = nq.to_euler(EulerRot::XYZ);
            euler_quat(pick(DVec3::new(ox, oy, oz), DVec3::new(nx, ny, nz)))
        };
        oq.slerp(q, influence)
    } else {
        oq
    };
    let t = if parts[0] {
        ot.lerp(pick(ot, nt), influence)
    } else {
        ot
    };
    DMat4::from_scale_rotation_translation(s, q, t)
}

/// A target's reference point: its centre, or its pivot with Use Pivots
/// (the pivot point is `pivot.position` in the item's own space).
fn point_of(scene: &Scene, p: &Piece, target: u64, world: DMat4) -> DVec3 {
    if p.flag("usePivots")
        && let Some(pivot) = scene.animation.rig.pivots.get(&target)
    {
        return world.transform_point3(pivot.position);
    }
    world.w_axis.truncate()
}

/// One constraint on a world matrix (RG-40..46).
fn constrain(scene: &Scene, p: &Piece, w: DMat4, look: &dyn Fn(u64) -> DMat4) -> DMat4 {
    let own = trs(w);
    let Some(weights) = weights(p) else {
        return w;
    };
    let targets: Vec<(DMat4, f64)> = p
        .targets
        .iter()
        .map(|&t| look(t))
        .zip(weights.iter().copied())
        .collect();
    let points = || -> DVec3 {
        p.targets
            .iter()
            .zip(&targets)
            .map(|(&t, (m, wt))| point_of(scene, p, t, *m) * *wt)
            .sum()
    };
    let (s, q, t) = own;
    match p.kind.as_str() {
        "position" => {
            let to = points() + p.vec3("offset.pos.");
            mix(p, own, (s, q, to), [true, false, false])
        }
        "rotation" => {
            let qs: Vec<(DQuat, f64)> = targets.iter().map(|(m, wt)| (trs(*m).1, *wt)).collect();
            let to = blend_quats(&qs) * euler_quat(p.vec3("offset.rot."));
            mix(p, own, (s, to, t), [false, true, false])
        }
        "scale" => {
            let to: DVec3 = targets.iter().map(|(m, wt)| trs(*m).0 * *wt).sum::<DVec3>()
                * p.vec3("offset.scl.");
            mix(p, own, (to, q, t), [false, false, true])
        }
        "transform" => {
            let offset = p.offset_matrix();
            let parts: Vec<(DVec3, DQuat, DVec3, f64)> = targets
                .iter()
                .map(|(m, wt)| {
                    let (s, q, t) = trs(*m * offset);
                    (s, q, t, *wt)
                })
                .collect();
            let ts: DVec3 = parts.iter().map(|x| x.2 * x.3).sum();
            let ss: DVec3 = parts.iter().map(|x| x.0 * x.3).sum();
            let qq = blend_quats(&parts.iter().map(|x| (x.1, x.3)).collect::<Vec<_>>());
            mix(p, own, (ss, qq, ts), [true, true, true])
        }
        "direction" => {
            let aim = points() - t;
            let up = match (p.refs.get("upItem"), p.num("upAxis").round() as i64) {
                (Some(&u), axis) if axis >= 0 => {
                    let m = look(u);
                    [m.x_axis, m.y_axis, m.z_axis][axis.clamp(0, 2) as usize].truncate()
                }
                (Some(&u), _) => look(u).w_axis.truncate() - t,
                (None, _) => p.vec3("up."),
            };
            let Some(basis) = aim_rotation(aim, up, p) else {
                return w;
            };
            let to = basis * euler_quat(p.vec3("offset.rot."));
            mix(p, own, (s, to, t), [false, true, false])
        }
        "distance" => {
            let target = points();
            let d = t - target;
            let len = d.length();
            let want = p.num("distance");
            let dir = if len > 1e-12 { d / len } else { DVec3::X };
            let to = match p.num("clamp").round() as i64 {
                1 if len >= want => t,
                2 if len <= want => t,
                _ => target + dir * want,
            } + p.vec3("offset.pos.");
            mix(p, own, (s, q, to), [true, false, false])
        }
        "path" | "pathPosition" | "pathNormal" => {
            let mut pos = DVec3::ZERO;
            let mut tangents: Vec<(DQuat, f64)> = Vec::new();
            let mut any = false;
            for (&curve, (m, wt)) in p.targets.iter().zip(&targets) {
                let Some((point, tangent)) = path_point(scene, curve, *m, p) else {
                    continue;
                };
                any = true;
                pos += point * *wt;
                if p.kind != "pathPosition" {
                    let up = if p.flag("localUp") {
                        m.transform_vector3(p.vec3("up."))
                    } else {
                        p.vec3("up.")
                    };
                    if let Some(r) = aim_rotation(tangent, up, p) {
                        tangents.push((r, *wt));
                    }
                }
            }
            if !any {
                return w;
            }
            let rot = if tangents.is_empty() {
                q
            } else {
                blend_quats(&tangents) * euler_quat(p.vec3("offset.rot."))
            };
            let pos = pos + p.vec3("offset.pos.");
            match p.kind.as_str() {
                "pathPosition" => mix(p, own, (s, q, pos), [true, false, false]),
                "pathNormal" => mix(p, own, (s, rot, t), [false, true, false]),
                _ => mix(p, own, (s, rot, pos), [true, true, false]),
            }
        }
        _ => w,
    }
}

/// The rotation that points the piece's Axis (Negative Axis turns it)
/// along `aim`, with the next axis as near `up` as it can be, then Roll
/// about the aim axis (RG-44). Up parallel to the aim falls back to world
/// +X (+Z when the aim lies along X), so there is no NaN and no flip.
fn aim_rotation(aim: DVec3, up: DVec3, p: &Piece) -> Option<DQuat> {
    if aim.length_squared() < 1e-24 {
        return None;
    }
    let mut d = aim.normalize();
    if p.flag("negative") {
        d = -d;
    }
    let axis = p.num("axis").round().clamp(0., 2.) as usize;
    let mut u = if up.length_squared() < 1e-24 {
        DVec3::Y
    } else {
        up.normalize()
    };
    if d.cross(u).length_squared() < 1e-18 {
        u = if d.x.abs() > 0.99 { DVec3::Z } else { DVec3::X };
    }
    // Columns X, Y, Z of the rotation.
    let (x, y, z) = match axis {
        // Aim Z, Y towards up.
        2 => {
            let z = d;
            let x = u.cross(z).normalize();
            (x, z.cross(x), z)
        }
        // Aim X, Y towards up.
        0 => {
            let x = d;
            let z = x.cross(u).normalize();
            (x, z.cross(x), z)
        }
        // Aim Y, Z towards up.
        _ => {
            let y = d;
            let x = y.cross(u).normalize();
            (x, y, x.cross(y))
        }
    };
    let basis = DQuat::from_mat3(&glam::DMat3::from_cols(x, y, z)).normalize();
    let roll_axis = [DVec3::X, DVec3::Y, DVec3::Z][axis];
    Some(basis * DQuat::from_axis_angle(roll_axis, p.num("roll")))
}

/// The point and direction at Percent + Offset along a curve item, by arc
/// length, in world space (RG-46). Wrap goes round (200% = twice);
/// without it the ends hold.
fn path_point(scene: &Scene, curve: u64, world: DMat4, p: &Piece) -> Option<(DVec3, DVec3)> {
    let item = scene.item(curve)?;
    let mesh = item.mesh.open();
    let (face, closed, kind) = mesh.faces.iter().enumerate().find_map(|(i, f)| {
        match mesh.attributes.representations.get(i) {
            Some(crate::PolygonRepresentation::Polyline { closed, curve }) => {
                Some((f, *closed, *curve))
            }
            _ => None,
        }
    })?;
    let points: Vec<DVec3> = face
        .vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .collect();
    if points.len() < 2 {
        return None;
    }
    let steps = if kind.is_line() { 1 } else { 24 };
    let mut samples = crate::curves::sample(&points, kind, closed, steps);
    if closed && samples.first() != samples.last() {
        samples.push(samples[0]);
    }
    let samples: Vec<DVec3> = samples.iter().map(|v| world.transform_point3(*v)).collect();
    let mut lengths = vec![0.];
    for pair in samples.windows(2) {
        lengths.push(lengths.last().unwrap() + pair[0].distance(pair[1]));
    }
    let total = *lengths.last()?;
    if total <= 0. {
        return None;
    }
    let mut u = p.num("percent") + p.num("pathOffset");
    u = if p.flag("wrap") {
        u.rem_euclid(1.).min(1.)
    } else {
        u.clamp(0., 1.)
    };
    // A full lap on wrap: 100% is the end, 200% the end again.
    if p.flag("wrap") && u == 0. && (p.num("percent") + p.num("pathOffset")) > 0. {
        u = 1.;
    }
    let at = u * total;
    let i = lengths
        .partition_point(|l| *l < at)
        .clamp(1, samples.len() - 1);
    let (a, b) = (samples[i - 1], samples[i]);
    let span = lengths[i] - lengths[i - 1];
    let f = if span > 0. {
        (at - lengths[i - 1]) / span
    } else {
        0.
    };
    Some((a.lerp(b, f), b - a))
}

/// The scene as the rig shows it (evaluate.rs): each item a rig piece
/// places gets the local transform that puts it at its evaluated world
/// under its document parent (the hierarchy stays, so exports keep it),
/// and the channels links and modifiers drive get their values. The rig's
/// pieces are left out of the copy, so nothing evaluates twice; with
/// `keep_source` the copy remembers the document scene (`Rig::source`) so
/// an export can evaluate it at other times (§8 USD). `None` when the rig
/// changes nothing.
pub fn apply(scene: &Scene, result: &RigResult, keep_source: bool) -> Option<Scene> {
    if result.world.is_empty() && result.driven.is_empty() {
        return None;
    }
    let mut out = scene.clone();
    // The skin's bind pose reads these (skin.rs): what the rig moves keeps
    // its setup pose from before.
    let moved = result
        .placed
        .iter()
        .copied()
        .chain(result.driven.iter().map(|(k, _)| k.item));
    for id in moved {
        out.animation
            .rig
            .setup_locals
            .entry(id)
            .or_insert_with(|| crate::skin::setup_local(scene, id));
    }
    for (k, v) in &result.driven {
        let _ = crate::animation::write(&mut out, k, *v);
    }
    let world_of = |id: u64| -> DMat4 {
        result
            .world
            .get(&id)
            .copied()
            .unwrap_or_else(|| out.world_matrix(id).unwrap_or(DMat4::IDENTITY))
    };
    let placed: Vec<(u64, DMat4)> = result
        .world
        .iter()
        .filter(|(id, _)| result.placed.contains(id))
        .map(|(id, w)| {
            let parent = out
                .item(*id)
                .and_then(|i| i.parent)
                .map_or(DMat4::IDENTITY, world_of);
            (*id, parent.inverse() * *w)
        })
        .collect();
    for (id, local) in placed {
        let Some(item) = out.item_mut(id) else {
            continue;
        };
        let (s, q, t) = local.to_scale_rotation_translation();
        item.transform.translation = t;
        item.transform.rotation = item.transform.angles_for(q);
        item.transform.scale = s;
        out.animation.rig.pivots.remove(&id);
    }
    out.animation.rig.pieces.clear();
    if keep_source {
        out.animation.rig.source = Some(std::sync::Arc::new(scene.clone()));
    }
    Some(out)
}

/// A piece that works on channel values (links, modifiers), not worlds.
pub fn channel_piece(p: &Piece) -> bool {
    p.kind == "link" || p.kind.starts_with("mod.")
}

/// A channel piece's output channel.
fn output(p: &Piece) -> Option<ChannelKey> {
    Some(ChannelKey::new(p.item, p.names.get("driven")?))
}

/// A channel piece's input channels.
fn inputs(p: &Piece) -> Vec<ChannelKey> {
    match p.kind.as_str() {
        "link" => match (p.targets.first(), p.names.get("driver")) {
            (Some(&i), Some(c)) => vec![ChannelKey::new(i, c)],
            _ => Vec::new(),
        },
        // Modifiers (RG-60..62): each wired input, in name order.
        _ => crate::rig::modifier_inputs(p)
            .into_iter()
            .map(|(_, k)| k)
            .collect(),
    }
}

/// Whether a channel piece's output feeds back into its own inputs.
fn channel_loop(scene: &Scene, piece: &Piece) -> bool {
    let Some(out) = output(piece) else {
        return false;
    };
    let pieces: Vec<&Piece> = scene
        .animation
        .rig
        .pieces
        .iter()
        .filter(|p| channel_piece(p) && p.flag("enable"))
        .collect();
    let mut seen = HashSet::new();
    let mut stack = inputs(piece);
    while let Some(k) = stack.pop() {
        if k == out {
            return true;
        }
        if !seen.insert(k.clone()) {
            continue;
        }
        for p in &pieces {
            if output(p).as_ref() == Some(&k) {
                stack.extend(inputs(p));
            }
        }
    }
    false
}

/// Evaluates the links and modifiers in dependency order on a copy of the
/// scene; `None` and nothing when there are none.
fn channels_stage(
    scene: &Scene,
    broken: &BTreeSet<u64>,
) -> (Option<Scene>, Vec<(ChannelKey, f64)>) {
    let pieces: Vec<&Piece> = scene
        .animation
        .rig
        .pieces
        .iter()
        .filter(|p| channel_piece(p) && p.flag("enable") && !broken.contains(&p.id))
        .collect();
    if pieces.is_empty() {
        return (None, Vec::new());
    }
    // Order: a piece after every piece that writes one of its inputs.
    let mut order: Vec<usize> = Vec::new();
    let mut state = vec![0u8; pieces.len()];
    fn visit(i: usize, pieces: &[&Piece], state: &mut [u8], order: &mut Vec<usize>) {
        if state[i] != 0 {
            return;
        }
        state[i] = 1;
        for input in inputs(pieces[i]) {
            for (j, p) in pieces.iter().enumerate() {
                if output(p).as_ref() == Some(&input) {
                    visit(j, pieces, state, order);
                }
            }
        }
        state[i] = 2;
        order.push(i);
    }
    for i in 0..pieces.len() {
        visit(i, &pieces, &mut state, &mut order);
    }
    let mut working = scene.clone();
    let mut driven = Vec::new();
    for i in order {
        let p = pieces[i];
        let Some(out) = output(p) else {
            continue;
        };
        let Some(v) = channel_value(&working, p) else {
            continue;
        };
        if crate::animation::write(&mut working, &out, v).is_ok() {
            driven.push((out, v));
        }
    }
    (Some(working), driven)
}

/// A channel piece's output value from `scene` (inputs already driven).
fn channel_value(scene: &Scene, p: &Piece) -> Option<f64> {
    match p.kind.as_str() {
        "link" => {
            let v = crate::animation::read(scene, inputs(p).first()?)?;
            match (p.names.get("mode").map(String::as_str), &p.curve) {
                // RG-52: through the relationship curve (driver value on
                // the horizontal axis).
                (Some("relationship"), Some(c)) if !c.is_empty() => {
                    Some(c.value_at(crate::time_tick::Tick::from_seconds(v)))
                }
                (Some("relationship"), _) => None,
                // RG-51: the raw value, no unit conversion.
                _ => Some(v),
            }
        }
        k if k.starts_with("mod.") => {
            let inputs = crate::rig::modifier_inputs(p);
            let value = |name: &str| -> Option<f64> {
                inputs
                    .iter()
                    .find(|(n, _)| n == name)
                    .and_then(|(_, k)| crate::animation::read(scene, k))
            };
            crate::rig::modifier_value(p, &value, scene).filter(|v| v.is_finite())
        }
        _ => None,
    }
}

/// The scene with the rig's results written as plain transforms under
/// each item's document parent, the rig left out ([`apply`] at the
/// scene's own time; USD export samples it, §8).
pub fn baked_scene(scene: &Scene) -> Scene {
    apply(scene, &evaluate(scene), false).unwrap_or_else(|| {
        let mut out = scene.clone();
        out.animation.rig.pieces.clear();
        out
    })
}

/// The items the rig moves (their world or a transform channel).
pub fn rigged_items(scene: &Scene) -> HashSet<u64> {
    if scene.animation.rig.pieces.is_empty() {
        return HashSet::new();
    }
    let r = evaluate(scene);
    let mut out: HashSet<u64> = r.world.keys().copied().collect();
    out.extend(r.driven.iter().map(|(k, _)| k.item));
    out
}
