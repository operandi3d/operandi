//! The Pose tool's edits (BEHAVIOR-RIG-PHASE6 §6, IK-50..54), as one
//! command so a drag can be scripted (§7): `operandi.pose.drag` moves a
//! joint of a hierarchy to a world position and the chain up to the
//! nearest pin (or the root) follows, solved by the full-body IK solver
//! (ik_body.rs) with the joints' IK settings; pins hold their joints.
//! The answer is written into the joints' own channels (FK), keyed with
//! Auto Key (IK-54); each call is one undo step, and the app folds a
//! drag's calls into one (UIC-161).
//!
//! Modes (IK-51): `ik` (a plain or Ctrl drag), `rigid` (the root or empty
//! space: the whole hierarchy moves), `twist` (Shift: turn the joint about
//! its bone by `angle`), `fk` (middle drag: only that joint turns, aiming
//! its bone at the position).
use crate::animation::{self, AnimChannel, AutoKey, ChannelKey};
use crate::ik_body::{BODY, Task};
use crate::rig::Piece;
use crate::{Command, Editor, Result, Scene};
use glam::{DMat4, DQuat, DVec3};
use serde_json::{Map, Value};

type Args = Map<String, Value>;

fn ids(a: &Args, name: &str) -> Result<Vec<u64>> {
    let bad = || format!("{name} are item ids");
    match a.get(name) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Number(n)) => n.as_u64().map(|v| vec![v]).ok_or_else(bad),
        Some(Value::String(s)) => s
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<u64>().map_err(|_| bad()))
            .collect(),
        Some(Value::Array(l)) => l.iter().map(|v| v.as_u64().ok_or_else(bad)).collect(),
        Some(_) => Err(bad()),
    }
}

fn vec3(a: &Args, name: &str) -> Result<Option<DVec3>> {
    let parts: Vec<f64> = match a.get(name) {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(s)) => s
            .split(',')
            .map(|p| {
                p.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("{name} is x,y,z"))
            })
            .collect::<Result<_>>()?,
        Some(Value::Array(l)) => l
            .iter()
            .map(|v| v.as_f64().ok_or_else(|| format!("{name} is x,y,z")))
            .collect::<Result<_>>()?,
        Some(_) => return Err(format!("{name} is x,y,z")),
    };
    match parts.as_slice() {
        [x, y, z] if parts.iter().all(|v| v.is_finite()) => Ok(Some(DVec3::new(*x, *y, *z))),
        _ => Err(format!("{name} is x,y,z")),
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

/// The top of `id`'s hierarchy (its topmost ancestor).
pub fn top(scene: &Scene, mut id: u64) -> u64 {
    while let Some(p) = scene.item(id).and_then(|i| i.parent) {
        id = p;
    }
    id
}

/// How many joints up from `joint` the nearest pin (or `root`) is.
fn depth_to_pin(scene: &Scene, joint: u64, root: u64, pins: &[u64]) -> i64 {
    let mut depth = 0;
    let mut at = joint;
    while at != root {
        let Some(p) = scene.item(at).and_then(|i| i.parent) else {
            break;
        };
        depth += 1;
        if pins.contains(&p) {
            break;
        }
        at = p;
    }
    depth
}

/// The evaluated world of `id` (the rig included).
fn world(scene: &Scene, id: u64) -> DMat4 {
    crate::rig_commands::evaluated_world(scene, id)
}

/// Writes each `(joint, world)` into the joints' local channels, parents
/// first (their parents' worlds as given or as evaluated).
fn write_worlds(scene: &mut Scene, worlds: &[(u64, DMat4)]) {
    let look = |scene: &Scene, id: u64| -> DMat4 {
        worlds
            .iter()
            .find(|(j, _)| *j == id)
            .map(|(_, w)| *w)
            .unwrap_or_else(|| world(scene, id))
    };
    let mut out: Vec<(u64, DMat4)> = Vec::new();
    for &(j, w) in worlds {
        let parent = scene
            .item(j)
            .and_then(|i| i.parent)
            .map_or(DMat4::IDENTITY, |p| look(scene, p));
        out.push((j, parent.inverse() * w));
    }
    for (j, local) in out {
        let (s, q, t) = local.to_scale_rotation_translation();
        if let Some(item) = scene.item_mut(j) {
            item.transform.translation = t.map(crate::rig::clean);
            item.transform.rotation = item.transform.angles_for(q).map(crate::rig::clean);
            item.transform.scale = s.map(crate::rig::clean);
        }
    }
}

/// IK-54: with Auto Key on, keys every transform channel that changed.
fn auto_key(before: &Scene, scene: &mut Scene, items: &[u64]) {
    if scene.animation.settings.auto_key == AutoKey::Off || scene.animation.setup {
        return;
    }
    let t = scene.time.current;
    for &id in items {
        for name in animation::TRANSFORM_CHANNELS {
            let key = ChannelKey::new(id, name);
            let (Some(old), Some(new)) =
                (animation::read(before, &key), animation::read(scene, &key))
            else {
                continue;
            };
            if animation::differs(old, new) {
                let c = scene
                    .animation
                    .channels
                    .entry(key)
                    .or_insert_with(|| AnimChannel {
                        curve: Default::default(),
                        rest: old,
                    });
                c.curve.set(t, new);
            }
        }
    }
}

/// `operandi.pose.drag`.
pub fn run(e: &mut Editor, _name: &str, a: &Args) -> Result<Option<Command>> {
    let joint = match ids(a, "joint")?.first() {
        Some(j) => *j,
        None => e.scene.selection.item.ok_or("Give joint")?,
    };
    if e.scene.item(joint).is_none() {
        return Err(format!("No item {joint}"));
    }
    let mode = a
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("ik")
        .to_owned();
    let pins = ids(a, "pins")?;
    let root = match ids(a, "root")?.first() {
        Some(r) => *r,
        None => top(&e.scene, joint),
    };
    let root_weight = num(a, "rootWeight")?.unwrap_or(0.);
    let pin_weight = num(a, "pinWeight")?.unwrap_or(1.);
    let pin_precision = num(a, "pinPrecision")?.unwrap_or(1.);
    let before = e.scene.clone();
    match mode.as_str() {
        "rigid" => {
            let by = vec3(a, "by")?
                .or(vec3(a, "to")?.map(|t| t - world(&e.scene, joint).w_axis.truncate()))
                .ok_or("Give to (x,y,z) or by")?;
            e.anim_step("Pose", |e| {
                let w = world(&e.scene, root);
                write_worlds(&mut e.scene, &[(root, DMat4::from_translation(by) * w)]);
                auto_key(&before, &mut e.scene, &[root]);
                Ok(true)
            })?;
        }
        "twist" => {
            let angle = num(a, "angle")?.ok_or("Give angle (degrees)")?.to_radians();
            let child = crate::skeleton::first_child(&e.scene, joint);
            e.anim_step("Pose", |e| {
                let w = world(&e.scene, joint);
                let at = w.w_axis.truncate();
                let axis = child
                    .map(|c| world(&e.scene, c).w_axis.truncate() - at)
                    .and_then(|v| v.try_normalize())
                    .unwrap_or(w.z_axis.truncate().normalize_or(DVec3::Z));
                let turned = DMat4::from_translation(at)
                    * DMat4::from_quat(DQuat::from_axis_angle(axis, angle))
                    * DMat4::from_translation(-at)
                    * w;
                write_worlds(&mut e.scene, &[(joint, turned)]);
                auto_key(&before, &mut e.scene, &[joint]);
                Ok(true)
            })?;
        }
        "fk" => {
            // Turn only the joint's parent so the joint aims at `to`? No:
            // the joint itself turns so its bone points at `to`.
            let to = vec3(a, "to")?.ok_or("Give to (x,y,z)")?;
            let child = crate::skeleton::first_child(&e.scene, joint)
                .ok_or("A leaf joint has no bone to aim")?;
            e.anim_step("Pose", |e| {
                let w = world(&e.scene, joint);
                let at = w.w_axis.truncate();
                let from = (world(&e.scene, child).w_axis.truncate() - at).normalize_or_zero();
                let q = DQuat::from_rotation_arc(from, (to - at).normalize_or_zero());
                let turned = DMat4::from_translation(at)
                    * DMat4::from_quat(q)
                    * DMat4::from_translation(-at)
                    * w;
                write_worlds(&mut e.scene, &[(joint, turned)]);
                auto_key(&before, &mut e.scene, &[joint]);
                Ok(true)
            })?;
        }
        "ik" => {
            let to = vec3(a, "to")?.ok_or("Give to (x,y,z)")?;
            if joint == root {
                // Dragging the root moves the whole hierarchy (IK-51).
                let by = to - world(&e.scene, joint).w_axis.truncate();
                e.anim_step("Pose", |e| {
                    let w = world(&e.scene, root);
                    write_worlds(&mut e.scene, &[(root, DMat4::from_translation(by) * w)]);
                    auto_key(&before, &mut e.scene, &[root]);
                    Ok(true)
                })?;
                return Ok(None);
            }
            // A scratch full-body solver over the hierarchy: the joint to
            // `to` (reaching up to the nearest pin), the pins held.
            let mut scratch = e.scene.clone();
            let piece_id = scratch.next_id;
            scratch.next_id += 1;
            let mut p = Piece::new(piece_id, BODY, root, Vec::new());
            if root_weight > 0. {
                p.values.insert("translateRoot".into(), 1.);
                p.values.insert("rootWeight".into(), root_weight);
            }
            // The scratch solve stands alone: other solvers left out, and
            // any item of the hierarchy can move (IK-50).
            scratch
                .animation
                .rig
                .pieces
                .retain(|q| !crate::ik::is_ik(&q.kind));
            p.names.insert("any".into(), "1".into());
            scratch.animation.rig.pieces.push(p.clone());
            let covered = crate::ik_body::covered(&scratch, &p);
            if !covered.contains(&joint) {
                return Err("That joint isn't in the selected hierarchy".into());
            }
            let pin_at = |id: u64| {
                let (_, q, t) = world(&e.scene, id).to_scale_rotation_translation();
                [t.x, t.y, t.z, q.w, q.x, q.y, q.z]
            };
            let reach = depth_to_pin(&e.scene, joint, root, &pins);
            scratch.animation.rig.ik_tasks.push(Task {
                piece: piece_id,
                joint,
                kind: "position".into(),
                goal: None,
                target: [to.x, to.y, to.z, 1., 0., 0., 0.],
                enable: true,
                weight: 1.,
                precision: 1.,
                joints: reach,
                second: true,
            });
            for &pin in pins.iter().filter(|p| covered.contains(p) && **p != joint) {
                scratch.animation.rig.ik_tasks.push(Task {
                    piece: piece_id,
                    joint: pin,
                    kind: "position".into(),
                    goal: None,
                    target: pin_at(pin),
                    enable: true,
                    weight: pin_weight,
                    precision: pin_precision,
                    joints: -1,
                    second: false,
                });
            }
            let result = crate::rig_eval::evaluate(&scratch);
            let solved: Vec<(u64, DMat4)> = covered
                .iter()
                .map(|j| {
                    (
                        *j,
                        result
                            .world
                            .get(j)
                            .copied()
                            .unwrap_or_else(|| world(&e.scene, *j)),
                    )
                })
                .collect();
            e.anim_step("Pose", |e| {
                write_worlds(&mut e.scene, &solved);
                auto_key(&before, &mut e.scene, &covered);
                Ok(true)
            })?;
        }
        other => return Err(format!("mode is ik, rigid, twist or fk, not {other}")),
    }
    Ok(None)
}
