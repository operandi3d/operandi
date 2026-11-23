//! Per-joint IK settings and joint limits (BEHAVIOR-RIG-PHASE6 IK-24,
//! IK-40..43). Each joint can carry settings on `Rig::joint_ik`: Active,
//! Enable Limits, Limit FK, the limits axis, Max Joint Velocity and per
//! axis translation/rotation switches, weights and Min/Max (radians).
//!
//! A limit is a soft wall that turns hard at the end (IK-41): inside the
//! last part of the range (its size set by Limits Strength) rotation
//! slows, and it stops at the value. Solvers clamp each joint's local
//! rotation, in its own rotation order, with [`limit`]. With Limit FK on,
//! edits that turn the joint past a limit are clamped too (IK-42, [OP]).
use crate::{Command, Editor, Result, Scene};
use glam::{DMat4, DQuat};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

type Args = Map<String, Value>;

fn yes() -> bool {
    true
}
fn one() -> f64 {
    1.
}
fn is_true(b: &bool) -> bool {
    *b
}
fn is_false(b: &bool) -> bool {
    !*b
}
fn is_one(v: &f64) -> bool {
    *v == 1.
}
fn is_zero(v: &f64) -> bool {
    *v == 0.
}

/// One axis's IK settings (IK-24).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AxisIk {
    #[serde(default, skip_serializing_if = "is_false")]
    pub translate: bool,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub translate_weight: f64,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub rotate: bool,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub rotate_weight: f64,
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_limits: bool,
    /// Radians.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub min: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub max: f64,
}

impl Default for AxisIk {
    fn default() -> Self {
        AxisIk {
            translate: false,
            translate_weight: 1.,
            rotate: true,
            rotate_weight: 1.,
            use_limits: false,
            min: 0.,
            max: 0.,
        }
    }
}

/// A joint's IK settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct JointIk {
    pub joint: u64,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub active: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub enable_limits: bool,
    /// IK-42 [OP]: limits clamp FK edits too.
    #[serde(default, skip_serializing_if = "is_false")]
    pub limit_fk: bool,
    /// The axis down the bone (SK-16: Z = 2).
    #[serde(default = "two", skip_serializing_if = "is_two")]
    pub limits_axis: u8,
    /// Radians a solver step may turn it (IK-24).
    #[serde(default = "velocity", skip_serializing_if = "is_velocity")]
    pub max_velocity: f64,
    #[serde(default)]
    pub axes: [AxisIk; 3],
}

fn two() -> u8 {
    2
}
fn is_two(v: &u8) -> bool {
    *v == 2
}
fn velocity() -> f64 {
    0.5
}
fn is_velocity(v: &f64) -> bool {
    *v == 0.5
}

impl JointIk {
    pub fn new(joint: u64) -> JointIk {
        JointIk {
            joint,
            active: true,
            enable_limits: false,
            limit_fk: false,
            limits_axis: 2,
            max_velocity: 0.5,
            axes: [AxisIk::default(); 3],
        }
    }
    pub fn is_default(&self) -> bool {
        *self == JointIk::new(self.joint)
    }
    /// Whether any axis limit is in force.
    pub fn limited(&self) -> bool {
        self.enable_limits && self.axes.iter().any(|a| a.use_limits)
    }
}

/// `joint`'s settings, if it has any.
pub fn settings(scene: &Scene, joint: u64) -> Option<&JointIk> {
    scene
        .animation
        .rig
        .joint_ik
        .iter()
        .find(|j| j.joint == joint)
}

/// IK-41: `v` kept in [`lo`, `hi`], slowing inside the last `strength` ×
/// 10% of the range and stopping at the wall.
pub fn soft_clamp(v: f64, lo: f64, hi: f64, strength: f64) -> f64 {
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    let m = (hi - lo) * 0.1 * strength.clamp(0., 5.);
    if m <= 1e-12 {
        return v.clamp(lo, hi);
    }
    let ease = |over: f64| m * (1. - (-over / m).exp());
    if v > hi - m {
        hi - m + ease(v - (hi - m))
    } else if v < lo + m {
        lo + m - ease((lo + m) - v)
    } else {
        v
    }
    .clamp(lo, hi)
}

/// Joint `joint`'s local matrix with its limited axes brought inside
/// their range (in its rotation order); unchanged with no limits.
pub fn limit(scene: &Scene, joint: u64, local: DMat4, strength: f64) -> DMat4 {
    let Some(j) = settings(scene, joint).filter(|j| j.limited()) else {
        return local;
    };
    let Some(item) = scene.item(joint) else {
        return local;
    };
    let (s, q, t) = local.to_scale_rotation_translation();
    let mut angles = item.transform.angles_for(q);
    let before = angles;
    for (k, a) in j.axes.iter().enumerate() {
        if a.use_limits {
            angles[k] = soft_clamp(angles[k], a.min, a.max, strength);
        }
    }
    if angles == before {
        return local;
    }
    let order = item.transform.rotation_order();
    DMat4::from_scale_rotation_translation(s, order.quat(angles), t)
}

/// IK-42: joints with Limit FK on have their rotation channels brought
/// inside their limits (a hard clamp). Returns whether any changed.
pub fn clamp_fk(scene: &mut Scene) -> bool {
    let limited: Vec<JointIk> = scene
        .animation
        .rig
        .joint_ik
        .iter()
        .filter(|j| j.limit_fk && j.limited())
        .cloned()
        .collect();
    let mut changed = false;
    for j in limited {
        let Some(item) = scene.item_mut(j.joint) else {
            continue;
        };
        for (k, a) in j.axes.iter().enumerate() {
            if a.use_limits {
                let (lo, hi) = (a.min.min(a.max), a.min.max(a.max));
                let v = item.transform.rotation[k];
                let c = v.clamp(lo, hi);
                if c != v {
                    item.transform.rotation[k] = c;
                    changed = true;
                }
            }
        }
    }
    changed
}

/// The local rotation `q` (in `joint`'s order) with limits applied.
pub fn limit_quat(scene: &Scene, joint: u64, q: DQuat, strength: f64) -> DQuat {
    let (_, out, _) =
        limit(scene, joint, DMat4::from_quat(q), strength).to_scale_rotation_translation();
    out
}

const AXES: [&str; 3] = ["X", "Y", "Z"];

fn num(a: &Args, name: &str) -> Result<Option<f64>> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => Ok(n.as_f64().filter(|v| v.is_finite())),
        Some(Value::String(s)) => s
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(Some)
            .ok_or_else(|| format!("{name} is a number")),
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

fn joint_arg(e: &Editor, a: &Args) -> Result<u64> {
    let id = match a.get("joint") {
        Some(Value::Number(n)) => n.as_u64(),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => e.scene.selection.item,
    }
    .ok_or("Select a joint")?;
    match e.scene.item(id) {
        Some(i) if i.is_joint() => Ok(id),
        Some(i) => Err(format!("{} isn't a joint", i.name)),
        None => Err(format!("No item {id}")),
    }
}

/// `operandi.ik.jointSet`: a joint's IK settings (angles in degrees).
pub fn run(e: &mut Editor, _name: &str, a: &Args) -> Result<Option<Command>> {
    let joint = joint_arg(e, a)?;
    let mut j = settings(&e.scene, joint)
        .cloned()
        .unwrap_or_else(|| JointIk::new(joint));
    if let Some(b) = flag(a, "active")? {
        j.active = b;
    }
    if let Some(b) = flag(a, "enableLimits")? {
        j.enable_limits = b;
    }
    if let Some(b) = flag(a, "limitFk")? {
        j.limit_fk = b;
    }
    if let Some(s) = a.get("limitsAxis").and_then(Value::as_str) {
        j.limits_axis = AXES
            .iter()
            .position(|x| x.eq_ignore_ascii_case(s))
            .ok_or("limitsAxis is X, Y or Z")? as u8;
    }
    if let Some(v) = num(a, "maxVelocity")? {
        j.max_velocity = v.to_radians().max(1e-6);
    }
    for (k, axis) in AXES.iter().enumerate() {
        let x = &mut j.axes[k];
        if let Some(b) = flag(a, &format!("useLimits{axis}"))? {
            x.use_limits = b;
        }
        if let Some(v) = num(a, &format!("min{axis}"))? {
            x.min = v.to_radians();
        }
        if let Some(v) = num(a, &format!("max{axis}"))? {
            x.max = v.to_radians();
        }
        if let Some(b) = flag(a, &format!("translate{axis}"))? {
            x.translate = b;
        }
        if let Some(v) = num(a, &format!("translateWeight{axis}"))? {
            x.translate_weight = v.max(0.);
        }
        if let Some(b) = flag(a, &format!("rotate{axis}"))? {
            x.rotate = b;
        }
        if let Some(v) = num(a, &format!("rotateWeight{axis}"))? {
            x.rotate_weight = v.max(0.);
        }
    }
    e.anim_step("Joint IK Settings", |e| {
        let list = &mut e.scene.animation.rig.joint_ik;
        list.retain(|x| x.joint != joint);
        if !j.is_default() {
            list.push(j.clone());
            list.sort_by_key(|x| x.joint);
        }
        clamp_fk(&mut e.scene);
        Ok(true)
    })?;
    Ok(None)
}

/// A joint's settings as JSON (angles in degrees), for `operandi.ik.jointQuery`.
pub fn answer(e: &Editor, name: &str, a: &Args) -> Option<Result<String>> {
    if name != "operandi.ik.jointQuery" {
        return None;
    }
    Some((|| {
        let joint = joint_arg(e, a)?;
        let j = settings(&e.scene, joint)
            .cloned()
            .unwrap_or_else(|| JointIk::new(joint));
        let axes: Map<String, Value> = AXES
            .iter()
            .enumerate()
            .map(|(k, n)| {
                let x = &j.axes[k];
                (
                    (*n).to_string(),
                    json!({
                        "translate": x.translate,
                        "translateWeight": x.translate_weight,
                        "rotate": x.rotate,
                        "rotateWeight": x.rotate_weight,
                        "useLimits": x.use_limits,
                        "min": crate::rig::clean(x.min.to_degrees()),
                        "max": crate::rig::clean(x.max.to_degrees()),
                    }),
                )
            })
            .collect();
        Ok(json!({
            "joint": joint,
            "active": j.active,
            "enableLimits": j.enable_limits,
            "limitFk": j.limit_fk,
            "limitsAxis": AXES[j.limits_axis.min(2) as usize],
            "maxVelocity": crate::rig::clean(j.max_velocity.to_degrees()),
            "axes": axes,
        })
        .to_string())
    })())
}
