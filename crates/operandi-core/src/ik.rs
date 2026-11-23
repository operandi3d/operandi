//! IK solvers (BEHAVIOR-RIG-PHASE6): rig pieces on a chain's root joint
//! that read the goal (and pole) and set the chain's evaluated rotations.
//! The joints' own channels (their FK pose and keys) are never written;
//! Blend mixes the solved rotations with the FK ones by quaternion slerp
//! (IK-30).
//!
//! Two-bone IK (IK-01..08): analytic. The chain's plane goes through the
//! root, the goal and the bend hint: the pole when there is one, else the
//! bend direction the chain had in Setup (stored on the piece as
//! `bend.X/Y/Z`, in the root's parent space, so the knee never flips),
//! turned by Orient round the root-to-goal line.
//!
//! The piece lives on the root joint; the middle and end joints are named
//! in `names` (`mid`, `end`), not as references, so the root doesn't read
//! its own children's worlds (RG-02).
use crate::Scene;
use crate::rig::Piece;
use glam::{DMat4, DQuat, DVec3};
use std::collections::HashMap;

pub const TWO_BONE: &str = "ik.twoBone";
pub const CHAIN: &str = "ik.chain";

/// The IK kinds rig.rs evaluates.
pub const KINDS: &[&str] = &[CHAIN, TWO_BONE];

pub fn is_ik(kind: &str) -> bool {
    KINDS.contains(&kind)
}

pub fn label(kind: &str) -> Option<&'static str> {
    Some(match kind {
        TWO_BONE => "Two-Bone IK",
        CHAIN => "Chain IK",
        _ => return None,
    })
}

/// IK-05/07/08 and IK-17 defaults; `None` for names the kind lacks.
pub fn default_value(kind: &str, name: &str) -> Option<f64> {
    match (kind, name) {
        (TWO_BONE | CHAIN, "blend") => Some(1.),
        (TWO_BONE, "orient" | "soft" | "stretch") => Some(0.),
        (TWO_BONE, "stretchBase" | "stretchMid") => Some(1.),
        (TWO_BONE, "stretchEnd") => Some(0.),
        (TWO_BONE | CHAIN, n) if n.starts_with("bend.") => Some(0.),
        (CHAIN, n) if n.starts_with("twist.") => Some(0.),
        (CHAIN, "iterations") => Some(10.),
        (CHAIN, "tolerance") => Some(0.001),
        (CHAIN, "rootLocked" | "twistSpread") => Some(1.),
        _ => None,
    }
}

/// The channels the piece shows (after `enable`), in order.
pub fn channel_names(kind: &str) -> Vec<String> {
    let names: &[&str] = match kind {
        TWO_BONE => &[
            "blend",
            "orient",
            "soft",
            "stretch",
            "stretchBase",
            "stretchMid",
            "stretchEnd",
        ],
        CHAIN => &[
            "blend",
            "iterations",
            "tolerance",
            "rootLocked",
            "twistSpread",
        ],
        _ => &[],
    };
    names.iter().map(|s| s.to_string()).collect()
}

/// A set value brought into range.
pub fn clamp(name: &str, value: f64) -> f64 {
    match name {
        "blend" | "soft" | "stretch" | "stretchBase" | "stretchMid" | "stretchEnd" => {
            value.clamp(0., 1.)
        }
        "iterations" => value.round().clamp(1., 1000.),
        "tolerance" => value.clamp(1e-9, 1.),
        "rootLocked" | "twistSpread" => {
            if value >= 0.5 {
                1.
            } else {
                0.
            }
        }
        _ => value,
    }
}

/// A chain joint named in `names` (`mid`, `end`).
pub fn named(p: &Piece, key: &str) -> Option<u64> {
    p.names.get(key).and_then(|s| s.parse().ok())
}

// ------------------------------------------------------------ two-bone

/// Two-bone options (IK-05..08).
#[derive(Clone, Copy, Debug, Default)]
pub struct TwoBone {
    /// 0..1 of the chain's reach where it eases into extension (IK-07).
    pub soft: f64,
    /// 0..1: how much the bones stretch to reach (IK-08).
    pub stretch: f64,
    pub stretch_base: f64,
    pub stretch_mid: f64,
}

/// Where the middle and end joints go: the root `a`, bone lengths `l1`,
/// `l2`, the `goal` and the bend `hint` (any vector off the root-to-goal
/// line on the side the middle joint goes). Returns (middle, end, the
/// two bone lengths after stretch).
pub fn solve_two_bone(
    a: DVec3,
    l1: f64,
    l2: f64,
    goal: DVec3,
    hint: DVec3,
    o: TwoBone,
) -> (DVec3, DVec3, f64, f64) {
    let to = goal - a;
    let d = to.length();
    let axis = if d > 1e-12 { to / d } else { DVec3::Y };
    // The plane's in-plane perpendicular, on the hint's side.
    let mut perp = hint - axis * hint.dot(axis);
    if perp.length_squared() < 1e-18 {
        perp = axis.any_orthonormal_vector();
    }
    let perp = perp.normalize();
    let (mut l1, mut l2) = (l1, l2);
    let reach = l1 + l2;
    if o.stretch > 0. && d > reach && reach > 0. {
        let r = 1. + o.stretch * (d / reach - 1.);
        l1 *= 1. + (r - 1.) * o.stretch_base;
        l2 *= 1. + (r - 1.) * o.stretch_mid;
    }
    let reach = l1 + l2;
    let soft = o.soft * reach;
    let mut d = d.min(f64::MAX);
    if soft > 0. {
        let da = reach - soft;
        if d > da {
            d = da + soft * (1. - (-(d - da) / soft).exp());
        }
    }
    let d = d.clamp((l1 - l2).abs(), reach);
    let x = if d > 1e-12 {
        (d * d + l1 * l1 - l2 * l2) / (2. * d)
    } else {
        0.
    };
    let h = (l1 * l1 - x * x).max(0.).sqrt();
    (a + axis * x + perp * h, a + axis * d, l1, l2)
}

/// The world rotation turning frame (`u0`, `n0`) onto (`u1`, `n1`): the
/// bone direction first, the plane normal second. With no usable normal
/// it is the shortest turn of the direction.
fn align(u0: DVec3, n0: DVec3, u1: DVec3, n1: DVec3) -> DQuat {
    let (u0, u1) = (u0.normalize_or_zero(), u1.normalize_or_zero());
    if u0 == DVec3::ZERO || u1 == DVec3::ZERO {
        return DQuat::IDENTITY;
    }
    let ok = |n: DVec3, u: DVec3| (n - u * n.dot(u)).length_squared() > 1e-18;
    if !ok(n0, u0) || !ok(n1, u1) {
        return DQuat::from_rotation_arc(u0, u1);
    }
    let frame = |u: DVec3, n: DVec3| {
        let n = (n - u * n.dot(u)).normalize();
        DQuat::from_mat3(&glam::DMat3::from_cols(u, n, u.cross(n)))
    };
    (frame(u1, n1) * frame(u0, n0).inverse()).normalize()
}

/// `m` turned by world rotation `q` about point `at`, moved to `to`.
fn turned(m: DMat4, q: DQuat, at: DVec3, to: DVec3) -> DMat4 {
    DMat4::from_translation(to) * DMat4::from_quat(q) * DMat4::from_translation(-at) * m
}

/// `a` and `b` mixed: translation and scale lerp, rotation slerp.
fn mix(a: DMat4, b: DMat4, t: f64) -> DMat4 {
    if t >= 1. {
        return b;
    }
    if t <= 0. {
        return a;
    }
    let (sa, qa, ta) = a.to_scale_rotation_translation();
    let (sb, qb, tb) = b.to_scale_rotation_translation();
    DMat4::from_scale_rotation_translation(sa.lerp(sb, t), qa.slerp(qb, t), ta.lerp(tb, t))
}

/// The bend hint a two-bone chain keeps (`bend.X/Y/Z`): the middle
/// joint's offset from the root-to-end line, in the root's parent space.
pub fn rest_bend(a: DVec3, b: DVec3, c: DVec3, parent: DMat4) -> Option<DVec3> {
    let axis = (c - a).normalize_or_zero();
    let off = (b - a) - axis * (b - a).dot(axis);
    if off.length_squared() < 1e-14 * (b - a).length_squared().max(1e-12) {
        return None;
    }
    Some(
        parent
            .inverse()
            .transform_vector3(off.normalize())
            .normalize(),
    )
}

/// The scene reads the solver needs while the rig evaluates.
pub struct Look<'a> {
    pub scene: &'a Scene,
    /// Evaluated worlds of the piece's targets and references.
    pub world: &'a dyn Fn(u64) -> DMat4,
    /// The root joint's parent world.
    pub parent: DMat4,
}

/// Solves a two-bone piece on the root whose FK world is `root`. Returns
/// the root's new world and puts the middle (and, stretching, end)
/// joints' worlds in `overrides`.
pub fn two_bone(p: &Piece, root: DMat4, look: &Look, overrides: &mut HashMap<u64, DMat4>) -> DMat4 {
    let s = look.scene;
    let (Some(mid), Some(end), Some(&goal)) = (named(p, "mid"), named(p, "end"), p.targets.first())
    else {
        return root;
    };
    let (Some(mi), Some(ei)) = (s.item(mid), s.item(end)) else {
        return root;
    };
    let mid_local = s.local_matrix(mi);
    let end_local = s.local_matrix(ei);
    let mid_fk = root * mid_local;
    let end_fk = mid_fk * end_local;
    let a = root.w_axis.truncate();
    let b = mid_fk.w_axis.truncate();
    let c = end_fk.w_axis.truncate();
    let (l1, l2) = ((b - a).length(), (c - b).length());
    if l1 < 1e-12 || l2 < 1e-12 {
        return root;
    }
    let target = (look.world)(goal).w_axis.truncate();
    let axis = (target - a).normalize_or_zero();
    let mut hint = match p.refs.get("pole") {
        Some(&pole) => (look.world)(pole).w_axis.truncate() - a,
        None => look.parent.transform_vector3(p.vec3("bend.")),
    };
    let orient = p.num("orient");
    if orient != 0. && axis != DVec3::ZERO {
        hint = DQuat::from_axis_angle(axis, orient) * hint;
    }
    let o = TwoBone {
        soft: p.num("soft"),
        stretch: p.num("stretch"),
        stretch_base: p.num("stretchBase"),
        stretch_mid: p.num("stretchMid"),
    };
    let (b1, c1, _, _) = solve_two_bone(a, l1, l2, target, hint, o);
    // The root turns its bone onto the new one, its plane onto the new one.
    let n_fk = (b - a).cross(c - b);
    let n_new = (b1 - a).cross(c1 - b1);
    let q1 = align(b - a, n_fk, b1 - a, n_new);
    let root_ik = turned(root, q1, a, a);
    // IK-41: limits on the root.
    let root_ik =
        look.parent * crate::ik_joint::limit(s, p.item, look.parent.inverse() * root_ik, 1.);
    // The middle joint: at its (stretched) place, turned onto the end.
    let mid_moved = root_ik * mid_local;
    let at = mid_moved.w_axis.truncate();
    let mid_moved = turned(mid_moved, DQuat::IDENTITY, at, b1);
    let c_now = (mid_moved * end_local).w_axis.truncate();
    let q2 = DQuat::from_rotation_arc(
        (c_now - b1).normalize_or_zero(),
        (c1 - b1).normalize_or_zero(),
    );
    let mid_ik = turned(mid_moved, q2, b1, b1);
    // IK-41: the middle joint's bend clamps the same way.
    let mid_ik = root_ik * crate::ik_joint::limit(s, mid, root_ik.inverse() * mid_ik, 1.);
    let blend = p.num("blend");
    let root_out = mix(root, root_ik, blend);
    let mid_out = root_out * mix(mid_local, root_ik.inverse() * mid_ik, blend);
    overrides.insert(mid, mid_out);
    if o.stretch > 0. {
        let end_ik = turned(
            mid_ik * end_local,
            DQuat::IDENTITY,
            (mid_ik * end_local).w_axis.truncate(),
            c1,
        );
        overrides.insert(
            end,
            mid_out * mix(end_local, mid_ik.inverse() * end_ik, blend),
        );
    }
    root_out
}

// ------------------------------------------------------------ chains

/// FABRIK options (IK-17).
#[derive(Clone, Copy, Debug)]
pub struct Fabrik {
    pub iterations: u32,
    /// A fraction of the chain's length (IK-17: 0.001).
    pub tolerance: f64,
    pub root_locked: bool,
}

impl Default for Fabrik {
    fn default() -> Self {
        Fabrik {
            iterations: 10,
            tolerance: 0.001,
            root_locked: true,
        }
    }
}

/// FABRIK (IK-15..18): the chain's joint positions with the end on the
/// goal and every bone its length. Starts from `points` (the FK pose), so
/// joints move as little as they need to; out of reach, the chain points
/// straight at the goal. With a `pole`, each inner joint turns round its
/// neighbours' line to the pole's side.
pub fn fabrik(points: &[DVec3], goal: DVec3, pole: Option<DVec3>, o: Fabrik) -> Vec<DVec3> {
    let n = points.len();
    if n < 2 {
        return points.to_vec();
    }
    let lengths: Vec<f64> = points.windows(2).map(|w| w[0].distance(w[1])).collect();
    let total: f64 = lengths.iter().sum();
    let tol = o.tolerance * total.max(1e-12);
    let mut p = points.to_vec();
    let root = p[0];
    if o.root_locked && root.distance(goal) >= total {
        let dir = (goal - root).normalize_or_zero();
        for i in 1..n {
            p[i] = p[i - 1] + dir * lengths[i - 1];
        }
        return p;
    }
    if p[n - 1].distance(goal) <= tol {
        return p;
    }
    // A straight chain can't find a side by itself: lean the inner joints
    // a little towards the pole first.
    if let Some(pole) = pole {
        let axis = (p[n - 1] - p[0]).normalize_or_zero();
        let side = (pole - p[0]) - axis * (pole - p[0]).dot(axis);
        if let Some(side) = side.try_normalize() {
            for q in p.iter_mut().take(n - 1).skip(1) {
                *q += side * total * 1e-3;
            }
        }
    }
    for _ in 0..o.iterations.max(1) {
        // Backward: the end on the goal, each joint pulled after it.
        p[n - 1] = goal;
        for i in (0..n - 1).rev() {
            let d = (p[i] - p[i + 1]).normalize_or(DVec3::Y);
            p[i] = p[i + 1] + d * lengths[i];
        }
        if o.root_locked {
            // Forward: the root back home, each joint pulled after it.
            p[0] = root;
            for i in 1..n {
                let d = (p[i] - p[i - 1]).normalize_or(DVec3::Y);
                p[i] = p[i - 1] + d * lengths[i - 1];
            }
        }
        if let Some(pole) = pole {
            for i in 1..n - 1 {
                p[i] = towards_pole(p[i - 1], p[i], p[i + 1], pole);
            }
        }
        if p[n - 1].distance(goal) <= tol {
            break;
        }
    }
    p
}

/// Joint `b` between `a` and `c`, turned round the line a–c to the
/// pole's side (lengths kept).
fn towards_pole(a: DVec3, b: DVec3, c: DVec3, pole: DVec3) -> DVec3 {
    let axis = (c - a).normalize_or_zero();
    if axis == DVec3::ZERO {
        return b;
    }
    let flat = |v: DVec3| v - axis * v.dot(axis);
    let (from, to) = (flat(b - a), flat(pole - a));
    if from.length_squared() < 1e-18 || to.length_squared() < 1e-18 {
        return b;
    }
    let angle = from.angle_between(to);
    let sign = from.cross(to).dot(axis).signum();
    a + DQuat::from_axis_angle(axis, angle * sign) * (b - a)
}

/// The chain from `root` down to `end` (root first); `None` when `end`
/// isn't below `root`.
pub fn chain_down(scene: &Scene, root: u64, end: u64) -> Option<Vec<u64>> {
    let mut out = vec![end];
    let mut at = end;
    while at != root {
        at = scene.item(at)?.parent?;
        out.push(at);
        if out.len() > 1000 {
            return None;
        }
    }
    out.reverse();
    Some(out)
}

/// Solves a chain piece on the root whose FK world is `root`.
pub fn chain(p: &Piece, root: DMat4, look: &Look, overrides: &mut HashMap<u64, DMat4>) -> DMat4 {
    let s = look.scene;
    let (Some(end), Some(&goal)) = (named(p, "end"), p.targets.first()) else {
        return root;
    };
    let Some(ids) = chain_down(s, p.item, end) else {
        return root;
    };
    let locals: Vec<DMat4> = ids[1..]
        .iter()
        .map(|&j| s.item(j).map_or(DMat4::IDENTITY, |i| s.local_matrix(i)))
        .collect();
    let mut fk = vec![root];
    for l in &locals {
        fk.push(*fk.last().expect("root") * *l);
    }
    let points: Vec<DVec3> = fk.iter().map(|m| m.w_axis.truncate()).collect();
    let target = (look.world)(goal);
    let pole = p
        .refs
        .get("pole")
        .map(|&q| (look.world)(q).w_axis.truncate());
    let o = Fabrik {
        iterations: p.num("iterations").max(1.) as u32,
        tolerance: p.num("tolerance"),
        root_locked: p.flag("rootLocked"),
    };
    let solved = fabrik(&points, target.w_axis.truncate(), pole, o);
    // Twist Spread: the goal's turn about the end bone, shared out.
    let twist = if p.flag("twistSpread") {
        let rest = p.vec3("twist.");
        let q_rest = DQuat::from_euler(glam::EulerRot::XYZ, rest.x, rest.y, rest.z);
        let (_, q_goal, _) = target.to_scale_rotation_translation();
        let delta = q_goal * q_rest.inverse();
        let n = ids.len();
        let bone = (solved[n - 1] - solved[n - 2]).normalize_or_zero();
        twist_about(delta, bone)
    } else {
        0.
    };
    // Each joint turns its bone onto the solved one, parents first.
    let n = ids.len();
    let mut ik_world = Vec::with_capacity(n);
    let mut parent_ik = DMat4::IDENTITY;
    for k in 0..n {
        let base = if k == 0 {
            root
        } else {
            parent_ik * locals[k - 1]
        };
        let at = base.w_axis.truncate();
        let base = turned(base, DQuat::IDENTITY, at, solved[k]);
        let w = if k + 1 < n {
            let next = (base * locals[k]).w_axis.truncate();
            let q = DQuat::from_rotation_arc(
                (next - solved[k]).normalize_or_zero(),
                (solved[k + 1] - solved[k]).normalize_or_zero(),
            );
            let spin = DQuat::from_axis_angle(
                (solved[k + 1] - solved[k]).normalize_or(DVec3::Y),
                twist / (n - 1) as f64,
            );
            turned(base, spin * q, solved[k], solved[k])
        } else {
            base
        };
        // IK-41: each joint's limits.
        let up = if k == 0 { look.parent } else { parent_ik };
        let w = up * crate::ik_joint::limit(s, ids[k], up.inverse() * w, 1.);
        ik_world.push(w);
        parent_ik = w;
    }
    let blend = p.num("blend");
    let root_out = mix(root, ik_world[0], blend);
    let mut parent_out = root_out;
    for k in 1..n {
        let local_ik = ik_world[k - 1].inverse() * ik_world[k];
        let w = parent_out * mix(locals[k - 1], local_ik, blend);
        overrides.insert(ids[k], w);
        parent_out = w;
    }
    root_out
}

/// The angle of `q`'s twist about unit `axis`.
fn twist_about(q: DQuat, axis: DVec3) -> f64 {
    if axis == DVec3::ZERO {
        return 0.;
    }
    let v = DVec3::new(q.x, q.y, q.z);
    let proj = axis * v.dot(axis);
    let t = DQuat::from_xyzw(proj.x, proj.y, proj.z, q.w);
    if t.length_squared() < 1e-24 {
        return 0.;
    }
    let t = t.normalize();
    let angle = 2. * t.w.clamp(-1., 1.).acos();
    let angle = if DVec3::new(t.x, t.y, t.z).dot(axis) < 0. {
        -angle
    } else {
        angle
    };
    // The shorter way round.
    if angle > std::f64::consts::PI {
        angle - std::f64::consts::TAU
    } else if angle < -std::f64::consts::PI {
        angle + std::f64::consts::TAU
    } else {
        angle
    }
}

/// Solves an IK piece on its root (two-bone or chain).
pub fn solve(p: &Piece, root: DMat4, look: &Look, overrides: &mut HashMap<u64, DMat4>) -> DMat4 {
    match p.kind.as_str() {
        TWO_BONE => two_bone(p, root, look, overrides),
        CHAIN => chain(p, root, look, overrides),
        _ => root,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: DVec3, b: DVec3) {
        assert!(a.distance(b) < 1e-3, "{a} vs {b}");
    }

    /// IK-04: bones 1.0198, rest bend toward −Z, root at the origin.
    #[test]
    fn ik04_two_bone_numbers() {
        let l = 1.0198;
        let hint = DVec3::NEG_Z;
        let o = TwoBone::default();
        let (m, e, ..) = solve_two_bone(DVec3::ZERO, l, l, DVec3::new(0., 1.5, 0.), hint, o);
        near(m, DVec3::new(0., 0.75, -0.691));
        near(e, DVec3::new(0., 1.5, 0.));
        let (m, e, ..) = solve_two_bone(DVec3::ZERO, l, l, DVec3::new(0., 1., 1.), hint, o);
        near(m, DVec3::new(0., 1.0196, -0.0196));
        near(e, DVec3::new(0., 1., 1.));
        let (m, e, ..) = solve_two_bone(DVec3::ZERO, l, l, DVec3::new(0., 3., 0.), hint, o);
        near(m, DVec3::new(0., l, 0.));
        near(e, DVec3::new(0., 2.0396, 0.));
    }

    /// IK-18: four unit bones along +Y.
    #[test]
    fn ik18_fabrik_cases() {
        let pts: Vec<DVec3> = (0..5).map(|i| DVec3::new(0., i as f64, 0.)).collect();
        let o = Fabrik::default();
        assert_eq!(
            fabrik(&pts, DVec3::new(0., 4., 0.), None, o),
            pts,
            "no change"
        );
        let far = fabrik(&pts, DVec3::new(0., 10., 0.), None, o);
        for (i, p) in far.iter().enumerate() {
            near(*p, DVec3::new(0., i as f64, 0.));
        }
        let goal = DVec3::new(2., 2., 0.);
        let bent = fabrik(&pts, goal, None, o);
        assert!(bent[4].distance(goal) <= 0.004 + 1e-12, "{}", bent[4]);
        for w in bent.windows(2) {
            assert!((w[0].distance(w[1]) - 1.).abs() < 1e-6);
        }
        assert_eq!(bent[0], DVec3::ZERO, "root locked");
    }

    #[test]
    fn fabrik_pole_bends_towards_it() {
        let pts: Vec<DVec3> = (0..5).map(|i| DVec3::new(0., i as f64, 0.)).collect();
        let o = Fabrik {
            iterations: 30,
            ..Default::default()
        };
        let goal = DVec3::new(0., 3., 0.);
        let bent = fabrik(&pts, goal, Some(DVec3::new(0., 1.5, 5.)), o);
        assert!(bent[2].z > 0.1, "{bent:?}");
        assert!(bent[4].distance(goal) < 0.01);
    }

    #[test]
    fn ik07_soft_eases_before_full_reach() {
        let o = TwoBone {
            soft: 0.1,
            ..Default::default()
        };
        let (_, e, ..) = solve_two_bone(DVec3::ZERO, 1., 1., DVec3::new(0., 1.95, 0.), DVec3::Z, o);
        assert!(e.y < 1.95 && e.y > 1.8, "{e}");
        let (_, e, ..) = solve_two_bone(DVec3::ZERO, 1., 1., DVec3::new(0., 5., 0.), DVec3::Z, o);
        assert!(e.y < 2., "never quite straight: {e}");
    }

    #[test]
    fn ik08_stretch_reaches() {
        let o = TwoBone {
            stretch: 1.,
            stretch_base: 1.,
            stretch_mid: 1.,
            ..Default::default()
        };
        let (m, e, l1, l2) =
            solve_two_bone(DVec3::ZERO, 1., 1., DVec3::new(0., 3., 0.), DVec3::Z, o);
        near(e, DVec3::new(0., 3., 0.));
        near(m, DVec3::new(0., 1.5, 0.));
        assert!((l1 - 1.5).abs() < 1e-9 && (l2 - 1.5).abs() < 1e-9);
    }
}
