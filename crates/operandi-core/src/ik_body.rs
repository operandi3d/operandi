//! Full-body IK (BEHAVIOR-RIG-PHASE6 §3, IK-20..27): one solver on a
//! hierarchy's root joint and tasks on its joints. A position task pulls a
//! joint to its goal's position (or, with no goal, to where it was when the
//! task was added: a pin); an orientation task turns it to the goal's
//! orientation. The solver finds the best compromise by damped least
//! squares (a public method): each step solves (J·Jᵀ + λ²·I)·y = e and
//! turns the joints by Jᵀ·y, where J holds, for each task row and each
//! joint axis, how the task moves as the joint turns (axis × lever for
//! positions, the axis for orientations). Joint IK settings (IK-24) switch
//! axes off, weight them and limit them; Translate Root lets the root
//! move. Each frame starts from the FK pose, so the same frame always gives
//! the same pose (IK-27).
// Joint-indexed loops read several parallel lists at once.
#![allow(clippy::needless_range_loop)]
use crate::ik_joint::{self, JointIk};
use crate::rig::Piece;
use crate::{Command, Editor, Result, Scene};
use glam::{DMat4, DQuat, DVec3};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

pub const BODY: &str = "ik.body";

type Args = Map<String, Value>;

/// IK-23 defaults.
pub fn default_value(name: &str) -> Option<f64> {
    Some(match name {
        "blend" => 1.,
        "continuous" => 0.,
        "steps" => 1000.,
        "precision" => 2.,
        "taskStrength" => 1.,
        "limitsStrength" => 1.,
        "specifyTolerance" => 1.,
        "tolerance" => 1.,
        "translateRoot" => 0.,
        "rootWeight" => 1.,
        "drawChains" => 1.,
        _ => return None,
    })
}

pub const CHANNELS: &[&str] = &[
    "blend",
    "continuous",
    "steps",
    "precision",
    "taskStrength",
    "limitsStrength",
    "specifyTolerance",
    "tolerance",
    "translateRoot",
    "rootWeight",
    "drawChains",
];

pub fn clamp(name: &str, v: f64) -> f64 {
    match name {
        "blend" => v.clamp(0., 1.),
        "continuous" | "specifyTolerance" | "translateRoot" => {
            if v >= 0.5 {
                1.
            } else {
                0.
            }
        }
        "steps" => v.round().clamp(1., 100_000.),
        "precision" => v.clamp(0.01, 100.),
        "taskStrength" | "limitsStrength" | "rootWeight" => v.max(0.),
        "tolerance" => v.max(1e-6),
        "drawChains" => v.round().clamp(0., 2.),
        _ => v,
    }
}

fn yes() -> bool {
    true
}
fn one() -> f64 {
    1.
}
fn all() -> i64 {
    -1
}

/// A task on a joint (IK-22, IK-25).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Task {
    /// The solver piece.
    pub piece: u64,
    pub joint: u64,
    /// `position` or `orientation`.
    pub kind: String,
    /// The goal item; none pins the joint at `target`.
    #[serde(default)]
    pub goal: Option<u64>,
    /// World position (x, y, z) and orientation (w, x, y, z) at adding.
    pub target: [f64; 7],
    #[serde(default = "yes")]
    pub enable: bool,
    #[serde(default = "one")]
    pub weight: f64,
    #[serde(default = "one")]
    pub precision: f64,
    /// How far up the chain it reaches; −1 = all (IK-25).
    #[serde(default = "all")]
    pub joints: i64,
    /// A goal-less task that still yields to the pins (the Pose tool's
    /// dragged joint).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub second: bool,
}

/// The joints a body solver covers, parents first: the root's joint
/// descendants (stopping at non-joints and at joints another solver owns),
/// only the branches to `ends` when it names any, less `excluded`
/// branches (IK-20, IK-26).
pub fn covered(scene: &Scene, p: &Piece) -> Vec<u64> {
    let list = |key: &str| -> Vec<u64> {
        p.names
            .get(key)
            .map(|s| s.split(',').filter_map(|v| v.trim().parse().ok()).collect())
            .unwrap_or_default()
    };
    let (ends, excluded) = (list("ends"), list("excluded"));
    let owned: Vec<u64> = scene
        .animation
        .rig
        .pieces
        .iter()
        .filter(|q| q.id != p.id && crate::ik::is_ik(&q.kind))
        .flat_map(|q| crate::ik_commands::chain(scene, q))
        .collect();
    let on_path = |j: u64| -> bool {
        ends.is_empty()
            || ends.iter().any(|&e| {
                let mut at = Some(e);
                while let Some(a) = at {
                    if a == j {
                        return true;
                    }
                    at = scene.item(a).and_then(|i| i.parent);
                }
                false
            })
    };
    let mut out = vec![p.item];
    let mut k = 0;
    while k < out.len() {
        let j = out[k];
        let kids: Vec<u64> = if p.names.contains_key("any") {
            scene
                .items
                .iter()
                .filter(|i| i.parent == Some(j))
                .map(|i| i.id)
                .collect()
        } else {
            crate::skeleton::children(scene, j)
        };
        for c in kids {
            if !owned.contains(&c) && !excluded.contains(&c) && on_path(c) {
                out.push(c);
            }
        }
        k += 1;
        if out.len() > 2000 {
            break;
        }
    }
    out
}

/// The tasks of solver `piece`.
pub fn tasks(scene: &Scene, piece: u64) -> impl Iterator<Item = &Task> {
    scene
        .animation
        .rig
        .ik_tasks
        .iter()
        .filter(move |t| t.piece == piece)
}

/// A task's target now: its goal's world, or its stored pin.
fn target_of(t: &Task, look: &dyn Fn(u64) -> DMat4) -> (DVec3, DQuat) {
    match t.goal {
        Some(g) => {
            let (_, q, p) = look(g).to_scale_rotation_translation();
            (p, q)
        }
        None => (
            DVec3::new(t.target[0], t.target[1], t.target[2]),
            DQuat::from_xyzw(t.target[4], t.target[5], t.target[6], t.target[3]).normalize(),
        ),
    }
}

/// The rotation from `a` to `b` as a world rotation vector (axis × angle).
fn rot_vec(a: DQuat, b: DQuat) -> DVec3 {
    let mut d = b * a.inverse();
    if d.w < 0. {
        d = -d;
    }
    d.to_scaled_axis()
}

/// Solves (M + λ²I)·y = e for the small symmetric matrix M (Gaussian
/// elimination with partial pivots).
fn solve_small(m: &[Vec<f64>], e: &[f64], lambda2: f64) -> Vec<f64> {
    let n = e.len();
    let mut a: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let mut row = m[i].clone();
            row[i] += lambda2;
            row.push(e[i]);
            row
        })
        .collect();
    for c in 0..n {
        let piv = (c..n)
            .max_by(|x, y| a[*x][c].abs().total_cmp(&a[*y][c].abs()))
            .unwrap_or(c);
        a.swap(c, piv);
        let d = a[c][c];
        if d.abs() < 1e-300 {
            continue;
        }
        for r in 0..n {
            if r != c {
                let f = a[r][c] / d;
                if f != 0. {
                    for k in c..=n {
                        a[r][k] -= f * a[c][k];
                    }
                }
            }
        }
    }
    (0..n)
        .map(|i| {
            if a[i][i].abs() < 1e-300 {
                0.
            } else {
                a[i][n] / a[i][i]
            }
        })
        .collect()
}

/// Δ = Jᵀ(JJᵀ + λ²I)⁻¹e (damped least squares) for `n` dofs.
fn dls(j: &[Vec<f64>], e: &[f64], n: usize, lambda: f64) -> Vec<f64> {
    if j.is_empty() {
        return vec![0.; n];
    }
    let jjt: Vec<Vec<f64>> = (0..j.len())
        .map(|a| {
            (0..j.len())
                .map(|b| (0..n).map(|c| j[a][c] * j[b][c]).sum())
                .collect()
        })
        .collect();
    let y = solve_small(&jjt, e, lambda * lambda);
    (0..n)
        .map(|c| (0..j.len()).map(|r| j[r][c] * y[r]).sum())
        .collect()
}

/// J⁺ = Jᵀ(JJᵀ + λ²I)⁻¹ as an n × rows matrix.
fn dls_matrix(j: &[Vec<f64>], n: usize, lambda: f64) -> Vec<Vec<f64>> {
    let rows = j.len();
    let mut out = vec![vec![0f64; rows]; n];
    for r in 0..rows {
        let mut unit = vec![0f64; rows];
        unit[r] = 1.;
        let col = dls(j, &unit, n, lambda);
        for a in 0..n {
            out[a][r] = col[a];
        }
    }
    out
}

/// The state the solver works on: each joint's local rotation and
/// translation (the root's in its parent's space).
struct Pose {
    q: Vec<DQuat>,
    t: Vec<DVec3>,
    s: Vec<DVec3>,
}

impl Pose {
    fn worlds(&self, parent_of: &[Option<usize>], root_parent: DMat4) -> Vec<DMat4> {
        let mut w: Vec<DMat4> = Vec::with_capacity(self.q.len());
        for k in 0..self.q.len() {
            let local = DMat4::from_scale_rotation_translation(self.s[k], self.q[k], self.t[k]);
            let up = parent_of[k].map_or(root_parent, |p| w[p]);
            w.push(up * local);
        }
        w
    }
}

/// One dof column: which joint, the kind and the world direction.
#[derive(Clone, Copy)]
enum Dof {
    Rotate {
        joint: usize,
        axis: DVec3,
        frame_axis: usize,
        weight: f64,
    },
    Move {
        joint: usize,
        axis: DVec3,
        frame_axis: usize,
        weight: f64,
    },
}

/// Solves a body piece on the root whose FK world is `root`.
pub fn solve(
    p: &Piece,
    root: DMat4,
    look: &crate::ik::Look,
    overrides: &mut HashMap<u64, DMat4>,
) -> DMat4 {
    let s = look.scene;
    let ids = covered(s, p);
    let my_tasks: Vec<&Task> = tasks(s, p.id)
        .filter(|t| t.enable && ids.contains(&t.joint))
        .collect();
    if my_tasks.is_empty() || ids.is_empty() {
        return root;
    }
    let index: HashMap<u64, usize> = ids.iter().enumerate().map(|(i, j)| (*j, i)).collect();
    let parent_of: Vec<Option<usize>> = ids
        .iter()
        .map(|j| {
            s.item(*j)
                .and_then(|i| i.parent)
                .and_then(|p| index.get(&p).copied())
        })
        .collect();
    let settings: Vec<Option<&JointIk>> = ids.iter().map(|j| ik_joint::settings(s, *j)).collect();
    let fk_locals: Vec<DMat4> = ids
        .iter()
        .enumerate()
        .map(|(k, j)| {
            if k == 0 {
                look.parent.inverse() * root
            } else {
                s.item(*j).map_or(DMat4::IDENTITY, |i| s.local_matrix(i))
            }
        })
        .collect();
    let mut pose = Pose {
        q: Vec::new(),
        t: Vec::new(),
        s: Vec::new(),
    };
    for l in &fk_locals {
        let (sc, q, t) = l.to_scale_rotation_translation();
        pose.q.push(q);
        pose.t.push(t);
        pose.s.push(sc);
    }
    let targets: Vec<(usize, bool, DVec3, DQuat, f64, i64)> = my_tasks
        .iter()
        .map(|t| {
            let (pos, rot) = target_of(t, look.world);
            (
                index[&t.joint],
                t.kind == "orientation",
                pos,
                rot,
                t.weight * t.precision.max(0.01),
                t.joints,
            )
        })
        .collect();
    // Pins (tasks with no goal) come first; goals work in what the pins
    // leave free (IK-Q08), so a far goal can't drag a planted foot.
    let pinned: Vec<bool> = my_tasks
        .iter()
        .map(|t| t.goal.is_none() && !t.second)
        .collect();
    let strength = p.num("taskStrength");
    let lambda = 0.1 / p.num("precision").max(0.01);
    let tol = if p.flag("specifyTolerance") {
        p.num("tolerance") * 1e-3
    } else {
        1e-3
    };
    let limits_strength = p.num("limitsStrength");
    let translate_root = p.flag("translateRoot");
    let root_weight = p.num("rootWeight");
    // Is `a` (within `n` joints) above `b`?
    let ancestor = |a: usize, b: usize, n: i64| -> bool {
        let mut at = Some(b);
        let mut depth = 0;
        while let Some(x) = at {
            if x == a {
                return true;
            }
            depth += 1;
            if n >= 0 && depth > n {
                return false;
            }
            at = parent_of[x];
        }
        false
    };
    let steps = p.num("steps").max(1.) as usize;
    // The hierarchy's size, for step lengths.
    let size = {
        let w = pose.worlds(&parent_of, look.parent);
        let r = w[0].w_axis.truncate();
        w.iter()
            .map(|m| m.w_axis.truncate().distance(r))
            .fold(0., f64::max)
            .max(1e-6)
    };
    let step_len = size * 0.1;
    // Stops early when solved, or when a step no longer gets closer (a
    // goal out of reach): the best compromise is found (IK-23).
    let mut last_error = f64::INFINITY;
    let mut stalled = 0;
    for _ in 0..steps {
        let w = pose.worlds(&parent_of, look.parent);
        let mut e: Vec<f64> = Vec::new();
        let mut done = true;
        let mut raw = 0.;
        for &(j, ori, pos, rot, weight, _) in &targets {
            let (_, qj, pj) = w[j].to_scale_rotation_translation();
            let v = if ori { rot_vec(qj, rot) } else { pos - pj };
            raw += v.length_squared();
            if v.length() > tol {
                done = false;
            }
            // A far goal pulls a bounded step at a time (error clamping),
            // so steps stay small enough for the pins to keep up.
            let v = v.clamp_length_max(step_len) * strength * weight.min(10.);
            e.extend([v.x, v.y, v.z]);
        }
        if done {
            break;
        }
        let error: f64 = raw.sqrt();
        if last_error - error < size * 1e-6 {
            stalled += 1;
            if stalled >= 5 {
                break;
            }
        } else {
            stalled = 0;
        }
        last_error = last_error.min(error);
        // The dofs.
        let mut dofs: Vec<Dof> = Vec::new();
        for (k, st) in settings.iter().enumerate() {
            if st.is_some_and(|s| !s.active) {
                continue;
            }
            let frame = if k == 0 {
                look.parent
            } else {
                w[parent_of[k].expect("non-root has a parent")]
            };
            for (a, axis) in [frame.x_axis, frame.y_axis, frame.z_axis]
                .iter()
                .enumerate()
            {
                let axis = axis.truncate().normalize_or_zero();
                let x = st.map(|s| s.axes[a]).unwrap_or_default();
                if x.rotate && x.rotate_weight > 0. {
                    dofs.push(Dof::Rotate {
                        joint: k,
                        axis,
                        frame_axis: a,
                        weight: x.rotate_weight,
                    });
                }
                let moves = if k == 0 { translate_root } else { x.translate };
                let mw = if k == 0 {
                    root_weight
                } else {
                    x.translate_weight
                };
                if moves && mw > 0. {
                    dofs.push(Dof::Move {
                        joint: k,
                        axis,
                        frame_axis: a,
                        weight: mw,
                    });
                }
            }
        }
        if dofs.is_empty() {
            break;
        }
        // The Jacobian, row by row.
        let rows = e.len();
        let mut jac = vec![vec![0f64; dofs.len()]; rows];
        for (r, &(j, ori, _, _, _, n)) in targets.iter().enumerate() {
            let pj = w[j].w_axis.truncate();
            for (c, d) in dofs.iter().enumerate() {
                let (joint, col) = match *d {
                    Dof::Rotate {
                        joint,
                        axis,
                        weight,
                        ..
                    } => {
                        if !ancestor(joint, j, n) {
                            continue;
                        }
                        let lever = pj - w[joint].w_axis.truncate();
                        (
                            joint,
                            if ori {
                                axis * weight
                            } else {
                                axis.cross(lever) * weight
                            },
                        )
                    }
                    Dof::Move {
                        joint,
                        axis,
                        weight,
                        ..
                    } => {
                        if ori || !ancestor(joint, j, n) {
                            continue;
                        }
                        (joint, axis * weight)
                    }
                };
                let _ = joint;
                for i in 0..3 {
                    jac[r * 3 + i][c] = col[i];
                }
            }
        }
        // Δ₁ = J₁⁺e₁ for the pins, then Δ₂ for the goals in the pins'
        // null space: Δ = Δ₁ + N·(J₂N)⁺(e₂ − J₂Δ₁), N = I − J₁⁺J₁.
        let n = dofs.len();
        let pick = |want: bool| -> (Vec<Vec<f64>>, Vec<f64>) {
            let mut j = Vec::new();
            let mut v = Vec::new();
            for (t, &pin) in pinned.iter().enumerate() {
                if pin == want {
                    for i in 0..3 {
                        j.push(jac[t * 3 + i].clone());
                        v.push(e[t * 3 + i]);
                    }
                }
            }
            (j, v)
        };
        let (j1, e1) = pick(true);
        let (j2, e2) = pick(false);
        let d1 = dls(&j1, &e1, n, lambda);
        let delta = if j2.is_empty() {
            d1
        } else if j1.is_empty() {
            dls(&j2, &e2, n, lambda)
        } else {
            // N = I − J₁⁺J₁, with J₁⁺ = J₁ᵀ(J₁J₁ᵀ + λ²I)⁻¹.
            let pinv = dls_matrix(&j1, n, lambda);
            let mut null = vec![vec![0f64; n]; n];
            for a in 0..n {
                for b in 0..n {
                    let pj: f64 = (0..j1.len()).map(|r| pinv[a][r] * j1[r][b]).sum();
                    null[a][b] = if a == b { 1. } else { 0. } - pj;
                }
            }
            let j2n: Vec<Vec<f64>> = j2
                .iter()
                .map(|row| {
                    (0..n)
                        .map(|b| (0..n).map(|c| row[c] * null[c][b]).sum())
                        .collect()
                })
                .collect();
            let rest: Vec<f64> = j2
                .iter()
                .zip(&e2)
                .map(|(row, v)| v - (0..n).map(|c| row[c] * d1[c]).sum::<f64>())
                .collect();
            let d2 = dls(&j2n, &rest, n, lambda);
            (0..n)
                .map(|a| d1[a] + (0..n).map(|b| null[a][b] * d2[b]).sum::<f64>())
                .collect()
        };
        // The whole step shrinks together (keeping it in the pins' null
        // space) when any turn or move is too big.
        let mut biggest: f64 = 0.;
        for (c, d) in dofs.iter().enumerate() {
            biggest = biggest.max(match *d {
                Dof::Rotate { weight, .. } => (delta[c] * weight).abs() / 0.2,
                Dof::Move { weight, .. } => (delta[c] * weight).abs() / (0.05 * size),
            });
        }
        let delta: Vec<f64> = if biggest > 1. {
            delta.iter().map(|d| d / biggest).collect()
        } else {
            delta
        };
        // Apply: rotations in each joint's parent frame, moves likewise.
        let mut turn = vec![DVec3::ZERO; ids.len()];
        let mut shift = vec![DVec3::ZERO; ids.len()];
        for (c, d) in dofs.iter().enumerate() {
            match *d {
                Dof::Rotate {
                    joint,
                    frame_axis,
                    weight,
                    ..
                } => turn[joint][frame_axis] += delta[c] * weight,
                Dof::Move {
                    joint,
                    frame_axis,
                    weight,
                    ..
                } => shift[joint][frame_axis] += delta[c] * weight,
            }
        }
        let mut moved = false;
        for k in 0..ids.len() {
            let max = settings[k].map_or(0.5, |s| s.max_velocity);
            let mut v = turn[k];
            if v.length() > max {
                v = v.normalize() * max;
            }
            if v.length_squared() > 1e-24 {
                // The frame axes are the parent frame's: a rotation vector
                // in those coordinates turns the local rotation directly.
                let q = DQuat::from_scaled_axis(v) * pose.q[k];
                pose.q[k] = ik_joint::limit_quat(s, ids[k], q.normalize(), limits_strength);
                moved = true;
            }
            if shift[k].length_squared() > 1e-24 {
                // In the parent frame's coordinates, like the turns.
                pose.t[k] += shift[k];
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    let solved = pose.worlds(&parent_of, look.parent);
    let blend = p.num("blend");
    let mix = |a: DMat4, b: DMat4| -> DMat4 {
        let (sa, qa, ta) = a.to_scale_rotation_translation();
        let (sb, qb, tb) = b.to_scale_rotation_translation();
        DMat4::from_scale_rotation_translation(
            sa.lerp(sb, blend),
            qa.slerp(qb, blend),
            ta.lerp(tb, blend),
        )
    };
    let mut out: Vec<DMat4> = Vec::with_capacity(ids.len());
    for k in 0..ids.len() {
        let ik_local = match parent_of[k] {
            Some(pk) => solved[pk].inverse() * solved[k],
            None => look.parent.inverse() * solved[k],
        };
        let local = mix(fk_locals[k], ik_local);
        let up = parent_of[k].map_or(look.parent, |pk| out[pk]);
        out.push(up * local);
    }
    for k in 1..ids.len() {
        overrides.insert(ids[k], out[k]);
    }
    out[0]
}

// ------------------------------------------------------------ commands

fn ids(a: &Args, name: &str) -> Result<Option<Vec<u64>>> {
    let bad = || format!("{name} are item ids");
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n.as_u64().map(|v| Some(vec![v])).ok_or_else(bad),
        Some(Value::String(s)) => s
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<u64>().map_err(|_| bad()))
            .collect::<Result<Vec<u64>>>()
            .map(Some),
        Some(Value::Array(l)) => l
            .iter()
            .map(|v| v.as_u64().ok_or_else(bad))
            .collect::<Result<Vec<u64>>>()
            .map(Some),
        Some(_) => Err(bad()),
    }
}

fn num(a: &Args, name: &str) -> Result<Option<f64>> {
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

fn flag(a: &Args, name: &str) -> Result<Option<bool>> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(Value::String(s)) => crate::command_args::boolean(name, s).map(Some),
        Some(_) => Err(format!("{name} must be true or false")),
    }
}

fn picked(scene: &Scene) -> Vec<u64> {
    scene.picked_items()
}

fn is_joint(scene: &Scene, id: u64) -> bool {
    scene.item(id).is_some_and(|i| i.is_joint())
}

/// The body solver covering `joint`.
pub fn solver_of(scene: &Scene, joint: u64) -> Option<&Piece> {
    scene
        .animation
        .rig
        .pieces
        .iter()
        .find(|p| p.kind == BODY && covered(scene, p).contains(&joint))
}

fn joint_arg(scene: &Scene, a: &Args) -> Result<u64> {
    let j = match ids(a, "joint")? {
        Some(v) => v.first().copied(),
        None => scene.selection.item,
    }
    .ok_or("Select a joint")?;
    if !is_joint(scene, j) {
        return Err("Select a joint".into());
    }
    Ok(j)
}

fn world_target(scene: &Scene, j: u64) -> [f64; 7] {
    let (_, q, p) = crate::rig_commands::evaluated_world(scene, j).to_scale_rotation_translation();
    [p.x, p.y, p.z, q.w, q.x, q.y, q.z].map(crate::rig::clean)
}

/// Keeps a solver's targets equal to its tasks' goals (for the rig's
/// dependency order).
fn sync_targets(scene: &mut Scene, piece: u64) {
    let goals: Vec<u64> = {
        let mut g: Vec<u64> = tasks(scene, piece).filter_map(|t| t.goal).collect();
        g.sort_unstable();
        g.dedup();
        g
    };
    if let Some(p) = scene.animation.rig.piece_mut(piece) {
        p.targets = goals;
    }
}

fn new_id(scene: &mut Scene) -> Result<u64> {
    if scene.next_id >= u64::MAX - 1 {
        return Err("Scene item/identity budget exhausted".into());
    }
    let id = scene.next_id;
    scene.next_id += 1;
    Ok(id)
}

/// The handler for the full-body IK commands.
pub fn run(e: &mut Editor, name: &str, a: &Args) -> Result<Option<Command>> {
    match name {
        "operandi.ik.assign" => {
            // IK-20: the root (and optionally end joints), in Setup.
            if !e.scene.animation.setup {
                return Err(crate::ik_commands::NEEDS_SETUP.into());
            }
            let picked = match ids(a, "joints")? {
                Some(j) => j,
                None => picked(&e.scene),
            };
            let (&root, ends) = picked
                .split_first()
                .ok_or("Select the hierarchy's root joint (then any end joints)")?;
            if !is_joint(&e.scene, root) || !ends.iter().all(|j| is_joint(&e.scene, *j)) {
                return Err("Select the hierarchy's root joint (then any end joints)".into());
            }
            if e.scene.animation.rig.pieces.iter().any(|p| {
                crate::ik::is_ik(&p.kind) && crate::ik_commands::chain(&e.scene, p).contains(&root)
            }) {
                return Err("That joint is in another IK chain already".into());
            }
            for &end in ends {
                if crate::ik::chain_down(&e.scene, root, end).is_none() {
                    return Err("All items must be in one hierarchy".into());
                }
            }
            let mut probe = Piece::new(0, BODY, root, Vec::new());
            if !ends.is_empty() {
                probe.names.insert(
                    "ends".into(),
                    ends.iter()
                        .map(u64::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
            for j in covered(&e.scene, &probe) {
                let sc = e
                    .scene
                    .item(j)
                    .map(|i| i.transform.scale)
                    .unwrap_or(DVec3::ONE);
                if (sc - DVec3::ONE).abs().max_element() > 1e-9 {
                    return Err(format!(
                        "Freeze scale on {} before assigning IK",
                        e.scene.item(j).map(|i| i.name.clone()).unwrap_or_default()
                    ));
                }
            }
            e.anim_step("Assign IK", |e| {
                let id = new_id(&mut e.scene)?;
                let mut p = probe.clone();
                p.id = id;
                let joints = covered(&e.scene, &p);
                e.scene.animation.rig.pieces.push(p);
                for j in joints {
                    crate::skin::ensure_setup_channels(&mut e.scene, j);
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.ik.addGoal" | "operandi.ik.assignGoal" | "operandi.ik.addTask" => {
            let joint = joint_arg(&e.scene, a)?;
            let piece = solver_of(&e.scene, joint)
                .ok_or("That joint has no full-body IK: Assign IK first")?;
            if piece.item == joint && name == "operandi.ik.addGoal" {
                return Err("Add goals to joints below the root".into());
            }
            let piece = piece.id;
            let mode = a
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("both")
                .to_owned();
            let kinds: Vec<&str> = match (name, mode.as_str()) {
                ("operandi.ik.addTask", "position") => vec!["position"],
                ("operandi.ik.addTask", "orientation") => vec!["orientation"],
                ("operandi.ik.addTask", m) => {
                    return Err(format!("mode is position or orientation, not {m}"));
                }
                (_, "position") => vec!["position"],
                (_, "rotation" | "orientation") => vec!["orientation"],
                (_, "both") => vec!["position", "orientation"],
                (_, m) => return Err(format!("mode is position, rotation or both, not {m}")),
            };
            let goal = if name == "operandi.ik.assignGoal" {
                let g = ids(a, "goal")?
                    .and_then(|v| v.first().copied())
                    .ok_or("Give goal (an item)")?;
                if e.scene.item(g).is_none() {
                    return Err(format!("No item {g}"));
                }
                Some(g)
            } else {
                None
            };
            let target = world_target(&e.scene, joint);
            let label = match name {
                "operandi.ik.addGoal" => "Add Goal",
                "operandi.ik.assignGoal" => "Assign Goal",
                _ => "Add Task",
            };
            let joint_name = e
                .scene
                .item(joint)
                .map(|i| i.name.clone())
                .unwrap_or_default();
            e.anim_step(label, |e| {
                let goal = if name == "operandi.ik.addGoal" {
                    // A box goal at the joint's world place (top level).
                    let g = e
                        .scene
                        .add_group_locator_named(Some(format!("{joint_name}_IKGoal")))?;
                    let w = crate::rig_commands::evaluated_world(&e.scene, joint);
                    let (_, q, t) = w.to_scale_rotation_translation();
                    let it = e.scene.item_mut(g).expect("made");
                    it.locator = Some(crate::locator::LocatorKind::IkGoal);
                    it.transform.translation = t;
                    it.transform.rotation = it.transform.angles_for(q);
                    Some(g)
                } else {
                    goal
                };
                let list = &mut e.scene.animation.rig.ik_tasks;
                for k in &kinds {
                    list.retain(|t| !(t.piece == piece && t.joint == joint && t.kind == *k));
                    list.push(Task {
                        piece,
                        joint,
                        kind: (*k).into(),
                        goal,
                        target,
                        enable: true,
                        weight: 1.,
                        precision: 1.,
                        joints: -1,
                        second: false,
                    });
                }
                sync_targets(&mut e.scene, piece);
                if let Some(err) = crate::rig_eval::would_loop(&e.scene) {
                    return Err(err);
                }
                if name == "operandi.ik.addGoal"
                    && let Some(g) = goal
                {
                    e.scene.select_item(Some(g));
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.ik.removeTask" | "operandi.ik.taskSet" => {
            let joint = joint_arg(&e.scene, a)?;
            let piece = solver_of(&e.scene, joint)
                .ok_or("That joint has no full-body IK")?
                .id;
            let kind = a.get("kind").and_then(Value::as_str).map(str::to_owned);
            let has = tasks(&e.scene, piece)
                .any(|t| t.joint == joint && kind.as_ref().is_none_or(|k| *k == t.kind));
            if !has {
                return Err("That joint has no such task".into());
            }
            let (enable, weight, precision, joints) = (
                flag(a, "enable")?,
                num(a, "weight")?,
                num(a, "precision")?,
                num(a, "joints")?,
            );
            e.anim_step(
                if name == "operandi.ik.removeTask" {
                    "Remove Task"
                } else {
                    "Task Settings"
                },
                |e| {
                    let hit = |t: &Task| {
                        t.piece == piece
                            && t.joint == joint
                            && kind.as_ref().is_none_or(|k| *k == t.kind)
                    };
                    if name == "operandi.ik.removeTask" {
                        e.scene.animation.rig.ik_tasks.retain(|t| !hit(t));
                    } else {
                        for t in e.scene.animation.rig.ik_tasks.iter_mut().filter(|t| hit(t)) {
                            if let Some(v) = enable {
                                t.enable = v;
                            }
                            if let Some(v) = weight {
                                t.weight = v.max(0.);
                            }
                            if let Some(v) = precision {
                                t.precision = v.max(0.01);
                            }
                            if let Some(v) = joints {
                                t.joints = v.round().max(-1.) as i64;
                            }
                        }
                    }
                    sync_targets(&mut e.scene, piece);
                    Ok(true)
                },
            )?;
            Ok(None)
        }
        "operandi.ik.addToChain" | "operandi.ik.removeFromChain" => {
            // IK-26.
            let joints = match ids(a, "joints")? {
                Some(j) => j,
                None => picked(&e.scene),
            };
            let piece = match ids(a, "piece")? {
                Some(v) => v[0],
                None => joints
                    .iter()
                    .find_map(|j| {
                        let mut at = Some(*j);
                        while let Some(x) = at {
                            if let Some(p) = e
                                .scene
                                .animation
                                .rig
                                .pieces
                                .iter()
                                .find(|p| p.kind == BODY && p.item == x)
                            {
                                return Some(p.id);
                            }
                            at = e.scene.item(x).and_then(|i| i.parent);
                        }
                        None
                    })
                    .ok_or("Select joints below a full-body IK root")?,
            };
            let add = name == "operandi.ik.addToChain";
            e.anim_step(
                if add {
                    "Add to Chain"
                } else {
                    "Remove from Chain"
                },
                |e| {
                    let p = e.scene.animation.rig.piece_mut(piece).ok_or("No solver")?;
                    let read = |p: &Piece, k: &str| -> Vec<u64> {
                        p.names
                            .get(k)
                            .map(|s| s.split(',').filter_map(|v| v.parse().ok()).collect())
                            .unwrap_or_default()
                    };
                    let mut excluded = read(p, "excluded");
                    let mut ends = read(p, "ends");
                    for j in &joints {
                        if add {
                            excluded.retain(|x| x != j);
                            if !ends.is_empty() && !ends.contains(j) {
                                ends.push(*j);
                            }
                        } else if !excluded.contains(j) {
                            excluded.push(*j);
                        }
                    }
                    let join =
                        |v: &[u64]| v.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
                    for (k, v) in [("excluded", &excluded), ("ends", &ends)] {
                        if v.is_empty() {
                            p.names.remove(k);
                        } else {
                            p.names.insert(k.into(), join(v));
                        }
                    }
                    Ok(true)
                },
            )?;
            Ok(None)
        }
        "operandi.ik.remove" => {
            // IK-26 Remove IK: the solver, its tasks and the joints' settings.
            let piece = crate::ik_commands::solver(&e.scene, a)?;
            e.anim_step("Remove IK", |e| {
                let p = e
                    .scene
                    .animation
                    .rig
                    .piece(piece)
                    .cloned()
                    .ok_or("No solver")?;
                let joints = if p.kind == BODY {
                    covered(&e.scene, &p)
                } else {
                    crate::ik_commands::chain(&e.scene, &p)
                };
                let rig = &mut e.scene.animation.rig;
                rig.pieces.retain(|q| q.id != piece);
                rig.ik_tasks.retain(|t| t.piece != piece);
                rig.joint_ik.retain(|j| !joints.contains(&j.joint));
                e.scene.animation.channels.retain(|k, _| k.item != piece);
                Ok(true)
            })?;
            Ok(None)
        }
        _ => Err(format!("Unknown IK command {name}")),
    }
}

/// The tasks of solver `p` as JSON with their errors now (for
/// `operandi.ik.query`).
pub fn tasks_json(
    scene: &Scene,
    p: &Piece,
    world: &dyn Fn(u64) -> DMat4,
    tol: f64,
) -> (Vec<Value>, bool) {
    let mut all_met = true;
    let list = tasks(scene, p.id)
        .map(|t| {
            let (pos, rot) = target_of(t, world);
            let (_, q, at) = world(t.joint).to_scale_rotation_translation();
            let err = if t.kind == "orientation" {
                rot_vec(q, rot).length()
            } else {
                at.distance(pos)
            };
            let met = !t.enable || err <= tol;
            all_met &= met;
            json!({
                "joint": t.joint,
                "kind": t.kind,
                "goal": t.goal,
                "enable": t.enable,
                "weight": t.weight,
                "precision": t.precision,
                "joints": t.joints,
                "error": crate::rig::clean(err),
                "met": met,
            })
        })
        .collect();
    (list, all_met)
}
