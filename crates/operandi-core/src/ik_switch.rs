//! IK/FK switching, switch keys and the IK utilities (BEHAVIOR-RIG-PHASE6
//! IK-30..33). Each command is one undo step.
//!
//! No pop either way (IK-31, IK-Q07): a switch keys the chain's joints (to
//! FK) or the goal and pole (to IK, Q19) at the current time, and keys
//! Blend; when Blend had no keys yet, its old value is keyed one frame
//! earlier too, so the frames before the switch keep showing what they
//! showed.
use crate::animation::{self, AnimChannel, ChannelKey};
use crate::ik::{self, CHAIN, TWO_BONE};
use crate::ik_commands::{chain, solver};
use crate::rig::Piece;
use crate::time_tick::Tick;
use crate::{Command, Editor, Result, Scene};
use glam::{DMat4, DVec3};
use serde_json::{Map, Value};

type Args = Map<String, Value>;

const POS: [&str; 3] = ["pos.X", "pos.Y", "pos.Z"];
const ROT: [&str; 3] = ["rot.X", "rot.Y", "rot.Z"];

/// Keys `key` at `t` with its current value.
fn key_now(scene: &mut Scene, key: ChannelKey, t: Tick) {
    let Some(v) = animation::read(scene, &key) else {
        return;
    };
    let c = scene
        .animation
        .channels
        .entry(key)
        .or_insert_with(|| AnimChannel {
            curve: Default::default(),
            rest: v,
        });
    c.curve.set(t, v);
}

fn key_value(scene: &mut Scene, key: ChannelKey, t: Tick, v: f64) {
    let rest = animation::read(scene, &key).unwrap_or(v);
    let c = scene
        .animation
        .channels
        .entry(key)
        .or_insert_with(|| AnimChannel {
            curve: Default::default(),
            rest,
        });
    c.curve.set(t, v);
}

/// Keys Blend to `to` at `t`; with no keys yet, its old value one frame
/// earlier too.
fn key_blend(scene: &mut Scene, piece: u64, to: f64, t: Tick) {
    let key = ChannelKey::new(piece, "blend");
    let had = scene
        .animation
        .channels
        .get(&key)
        .is_some_and(|c| !c.curve.is_empty());
    if !had {
        let old = animation::read(scene, &key).unwrap_or(1.);
        let before = animation::step_frames(scene, t, -1);
        if before < t {
            key_value(scene, key.clone(), before, old);
        }
    }
    key_value(scene, key.clone(), t, to);
    let _ = animation::write(scene, &key, to);
}

/// The solved (evaluated) worlds of the chain's joints now.
fn solved(scene: &Scene, joints: &[u64]) -> Vec<DMat4> {
    let r = crate::rig_eval::evaluate(scene);
    joints
        .iter()
        .map(|j| {
            r.world
                .get(j)
                .copied()
                .unwrap_or_else(|| scene.world_matrix(*j).unwrap_or(DMat4::IDENTITY))
        })
        .collect()
}

/// Sets item `id`'s local transform so its world is `world` under a
/// parent whose world is `parent`.
fn place(scene: &mut Scene, id: u64, parent: DMat4, world: DMat4) {
    let (s, q, t) = (parent.inverse() * world).to_scale_rotation_translation();
    if let Some(item) = scene.item_mut(id) {
        item.transform.translation = t.map(crate::rig::clean);
        item.transform.rotation = item.transform.angles_for(q).map(crate::rig::clean);
        item.transform.scale = s.map(crate::rig::clean);
    }
}

fn parent_world(scene: &Scene, id: u64) -> DMat4 {
    match scene.item(id).and_then(|i| i.parent) {
        Some(p) => crate::rig_commands::evaluated_world(scene, p),
        None => DMat4::IDENTITY,
    }
}

/// IK-31 Switch to FK: the solved pose into the joints' FK channels,
/// keyed, and Blend keyed to 0.
fn to_fk(scene: &mut Scene, p: &Piece, t: Tick) {
    let joints = chain(scene, p);
    let worlds = solved(scene, &joints);
    let root_parent = parent_world(scene, p.item);
    for (k, &j) in joints.iter().enumerate() {
        let parent = if k == 0 { root_parent } else { worlds[k - 1] };
        place(scene, j, parent, worlds[k]);
    }
    for &j in &joints {
        for c in POS.iter().chain(&ROT) {
            key_now(scene, ChannelKey::new(j, c), t);
        }
    }
    key_blend(scene, p.id, 0., t);
}

/// IK-31 Switch to IK: the goal (and pole) onto the FK pose, keyed (Q19),
/// and Blend keyed to 1.
fn to_ik(scene: &mut Scene, p: &Piece, t: Tick) {
    let joints = chain(scene, p);
    // The FK pose: the chain with Blend at 0.
    let mut fk = scene.clone();
    if let Some(q) = fk.animation.rig.piece_mut(p.id) {
        q.values.insert("blend".into(), 0.);
    }
    fk.animation
        .channels
        .remove(&ChannelKey::new(p.id, "blend"));
    let worlds = solved(&fk, &joints);
    if let Some(&goal) = p.targets.first() {
        let end = *worlds.last().expect("root");
        place(scene, goal, parent_world(scene, goal), end);
        for c in POS.iter().chain(&ROT) {
            key_now(scene, ChannelKey::new(goal, c), t);
        }
    }
    if let Some(&pole) = p.refs.get("pole")
        && worlds.len() >= 3
    {
        let (a, b, c) = (
            worlds[0].w_axis.truncate(),
            worlds[1].w_axis.truncate(),
            worlds[2].w_axis.truncate(),
        );
        let axis = (c - a).normalize_or_zero();
        let off = (b - a) - axis * (b - a).dot(axis);
        if let Some(off) = off.try_normalize() {
            let at = b + off * (b - a).length();
            let pw = parent_world(scene, pole);
            let mut w = scene.world_matrix(pole).unwrap_or(DMat4::IDENTITY);
            w.w_axis = at.extend(1.);
            place(scene, pole, pw, w);
            for c in POS {
                key_now(scene, ChannelKey::new(pole, c), t);
            }
        }
    }
    key_blend(scene, p.id, 1., t);
}

/// The switch channels (IK-32): Blend, the chain's joints' position and
/// rotation, the goal's position and rotation, the pole's position.
pub fn switch_channels(scene: &Scene, p: &Piece) -> Vec<ChannelKey> {
    let mut out = vec![ChannelKey::new(p.id, "blend")];
    for j in chain(scene, p) {
        out.extend(POS.iter().chain(&ROT).map(|c| ChannelKey::new(j, c)));
    }
    if let Some(&g) = p.targets.first() {
        out.extend(POS.iter().chain(&ROT).map(|c| ChannelKey::new(g, c)));
    }
    if let Some(&pole) = p.refs.get("pole") {
        out.extend(POS.iter().map(|c| ChannelKey::new(pole, c)));
    }
    out
}

/// The solvers a command works on: `piece`, else every solver touching
/// the selection.
fn solvers(scene: &Scene, a: &Args) -> Result<Vec<u64>> {
    if a.get("piece").is_some() {
        return Ok(vec![solver(scene, a)?]);
    }
    let picked: Vec<u64> = scene.selected_items();
    let mut out: Vec<u64> = Vec::new();
    for p in scene
        .animation
        .rig
        .pieces
        .iter()
        .filter(|p| ik::is_ik(&p.kind))
    {
        let mut touches = chain(scene, p);
        touches.extend(p.targets.iter().copied());
        touches.extend(p.refs.values().copied());
        if picked.iter().any(|i| touches.contains(i)) {
            out.push(p.id);
        }
    }
    if out.is_empty() {
        return Err("Select an IK chain's joint or goal".into());
    }
    Ok(out)
}

fn flag(a: &Args, name: &str, default: bool) -> Result<bool> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(b)) => Ok(*b),
        Some(Value::String(s)) => crate::command_args::boolean(name, s),
        Some(_) => Err(format!("{name} must be true or false")),
    }
}

/// Whether solver `p` shows IK now (Blend at least half).
pub fn in_ik(scene: &Scene, p: &Piece) -> bool {
    animation::read(scene, &ChannelKey::new(p.id, "blend")).unwrap_or_else(|| p.num("blend")) >= 0.5
}

/// The handler for the switch and utility commands.
pub fn run(e: &mut Editor, name: &str, a: &Args) -> Result<Option<Command>> {
    let t = e.scene.time.current;
    match name {
        "operandi.ik.switch" => {
            let ids = solvers(&e.scene, a)?;
            let mode = a
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("toggle")
                .to_owned();
            if !matches!(mode.as_str(), "toggle" | "fk" | "ik") {
                return Err(format!("mode is toggle, fk or ik, not {mode}"));
            }
            e.anim_step("Switch IK/FK", |e| {
                for id in &ids {
                    let p = e
                        .scene
                        .animation
                        .rig
                        .piece(*id)
                        .cloned()
                        .ok_or("No solver")?;
                    let fk = match mode.as_str() {
                        "fk" => true,
                        "ik" => false,
                        _ => in_ik(&e.scene, &p),
                    };
                    if fk {
                        to_fk(&mut e.scene, &p, t);
                    } else {
                        to_ik(&mut e.scene, &p, t);
                    }
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.ik.setKeys" => {
            // IK-32: key every switch channel now; delete removes those keys.
            let ids = solvers(&e.scene, a)?;
            let delete = flag(a, "delete", false)?;
            e.anim_step(
                if delete {
                    "Delete IK/FK Keys"
                } else {
                    "Set IK/FK Keys"
                },
                |e| {
                    let mut changed = false;
                    for id in &ids {
                        let p = e
                            .scene
                            .animation
                            .rig
                            .piece(*id)
                            .cloned()
                            .ok_or("No solver")?;
                        for key in switch_channels(&e.scene, &p) {
                            if delete {
                                if let Some(c) = e.scene.animation.channels.get_mut(&key)
                                    && let Some(i) = c.curve.key_at(t)
                                {
                                    c.curve.keys.remove(i);
                                    changed = true;
                                }
                            } else {
                                key_now(&mut e.scene, key, t);
                                changed = true;
                            }
                        }
                    }
                    Ok(changed)
                },
            )?;
            Ok(None)
        }
        "operandi.ik.selectSwitch" | "operandi.ik.selectSolvers" => {
            // IK-32 Select Switch Channels (their items); IK-33 Select IK
            // Solvers (the joints they sit on). Selection only.
            let ids = solvers(&e.scene, a)?;
            let mut items: Vec<u64> = Vec::new();
            for id in &ids {
                let p = e.scene.animation.rig.piece(*id).ok_or("No solver")?;
                if name == "operandi.ik.selectSolvers" {
                    items.push(p.item);
                } else {
                    let keys = switch_channels(&e.scene, p);
                    for k in keys {
                        if e.scene.item(k.item).is_some() && !items.contains(&k.item) {
                            items.push(k.item);
                        }
                    }
                }
            }
            e.scene.select_item(items.first().copied());
            for i in items.iter().skip(1) {
                e.scene.toggle_item(*i);
            }
            Ok(None)
        }
        "operandi.ik.disable" => {
            // IK-33 Disable IK (per scene, saved): in Setup mode only.
            if !e.scene.animation.setup {
                return Err("Disable IK works in Setup mode. Turn on Setup and try again.".into());
            }
            let on = match a.get("on").and_then(Value::as_str) {
                None | Some("toggle") => !e.scene.animation.rig.ik_disabled,
                Some(s) => crate::command_args::boolean("on", s)?,
            };
            e.anim_step(if on { "Disable IK" } else { "Enable IK" }, |e| {
                let changed = e.scene.animation.rig.ik_disabled != on;
                e.scene.animation.rig.ik_disabled = on;
                Ok(changed)
            })?;
            Ok(None)
        }
        "operandi.ik.updateRest" => {
            // IK-33 Update Rest Pose: the bend side (two-bone) and twist
            // reference (chain) from the joints as they are now.
            let all = flag(a, "all", false)?;
            let ids: Vec<u64> = if all {
                e.scene
                    .animation
                    .rig
                    .pieces
                    .iter()
                    .filter(|p| ik::is_ik(&p.kind))
                    .map(|p| p.id)
                    .collect()
            } else {
                solvers(&e.scene, a)?
            };
            e.anim_step("Update Rest Pose", |e| {
                for id in &ids {
                    let p = e
                        .scene
                        .animation
                        .rig
                        .piece(*id)
                        .cloned()
                        .ok_or("No solver")?;
                    let joints = chain(&e.scene, &p);
                    let w: Vec<DVec3> = joints
                        .iter()
                        .map(|j| {
                            e.scene
                                .world_matrix(*j)
                                .map(|m| m.w_axis.truncate())
                                .unwrap_or_default()
                        })
                        .collect();
                    let parent = match e.scene.item(p.item).and_then(|i| i.parent) {
                        Some(q) => e.scene.world_matrix(q)?,
                        None => DMat4::IDENTITY,
                    };
                    let q = e.scene.animation.rig.piece_mut(*id).expect("found");
                    match p.kind.as_str() {
                        TWO_BONE if w.len() == 3 => {
                            let bend = ik::rest_bend(w[0], w[1], w[2], parent)
                                .ok_or(crate::ik_commands::STRAIGHT)?;
                            for (k, axis) in ["X", "Y", "Z"].iter().enumerate() {
                                q.values
                                    .insert(format!("bend.{axis}"), crate::rig::clean(bend[k]));
                            }
                        }
                        CHAIN => {
                            if let Some(&g) = p.targets.first() {
                                let (_, r, _) =
                                    e.scene.world_matrix(g)?.to_scale_rotation_translation();
                                let (x, y, z) = r.to_euler(glam::EulerRot::XYZ);
                                let q = e.scene.animation.rig.piece_mut(*id).expect("found");
                                for (axis, v) in ["X", "Y", "Z"].iter().zip([x, y, z]) {
                                    q.values
                                        .insert(format!("twist.{axis}"), crate::rig::clean(v));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.ik.match" => {
            // IK-33 Match Position / Rotation: the goal onto its end joint.
            let ids = solvers(&e.scene, a)?;
            let what = a
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("both")
                .to_owned();
            if !matches!(what.as_str(), "position" | "rotation" | "both") {
                return Err(format!("mode is position, rotation or both, not {what}"));
            }
            e.anim_step("Match Goal", |e| {
                for id in &ids {
                    let p = e
                        .scene
                        .animation
                        .rig
                        .piece(*id)
                        .cloned()
                        .ok_or("No solver")?;
                    let Some(&goal) = p.targets.first() else {
                        continue;
                    };
                    let end = *chain(&e.scene, &p).last().expect("root");
                    let target = e.scene.world_matrix(end)?;
                    let now = e.scene.world_matrix(goal)?;
                    let (s0, q0, t0) = now.to_scale_rotation_translation();
                    let (_, q1, t1) = target.to_scale_rotation_translation();
                    let (q, tr) = match what.as_str() {
                        "position" => (q0, t1),
                        "rotation" => (q1, t0),
                        _ => (q1, t1),
                    };
                    let pw = parent_world(&e.scene, goal);
                    place(
                        &mut e.scene,
                        goal,
                        pw,
                        DMat4::from_scale_rotation_translation(s0, q, tr),
                    );
                }
                Ok(true)
            })?;
            Ok(None)
        }
        _ => Err(format!("Unknown IK command {name}")),
    }
}
