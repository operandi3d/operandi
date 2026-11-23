//! IK commands (BEHAVIOR-RIG-PHASE6 §7 MCP): Apply IK (Two-Bone), the
//! pole (Create / Assign / Remove Up Vector) and the IK query. Each edit
//! is one undo step; applying needs Setup mode, where the chain's rest
//! pose (and so its bend side) is read (§0).
use crate::ik::{self, CHAIN, TWO_BONE};
use crate::locator::LocatorKind;
use crate::rig::Piece;
use crate::{Command, Editor, Result, Scene};
use glam::{DMat4, DVec3};
use serde_json::{Map, Value, json};

type Args = Map<String, Value>;

/// IK-01's refusal.
pub const THREE_JOINTS: &str = "Select three joints in one chain, root first.";
/// Applying outside Setup (provisional, IK-Q01).
pub const NEEDS_SETUP: &str = "Apply IK needs Setup mode. Turn on Setup and try again.";
/// IK-15's refusal.
pub const ROOT_THEN_END: &str =
    "Select the chain's root joint, then its end joint (2 to 100 joints).";
/// IK-03: a straight chain has no bend side.
pub const STRAIGHT: &str =
    "The chain is straight: bend the middle joint slightly first, towards where it should bend.";

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
        Some(Value::Array(list)) => list
            .iter()
            .map(|v| v.as_u64().ok_or_else(bad))
            .collect::<Result<Vec<u64>>>()
            .map(Some),
        Some(_) => Err(bad()),
    }
}

fn one(a: &Args, name: &str) -> Result<Option<u64>> {
    Ok(ids(a, name)?.and_then(|v| v.first().copied()))
}

/// The selection in pick order.
fn picked(scene: &Scene) -> Vec<u64> {
    let s = &scene.selection;
    let mut out: Vec<u64> = s
        .also
        .iter()
        .copied()
        .filter(|i| scene.item(*i).is_some())
        .collect();
    if let Some(active) = s.item
        && !out.contains(&active)
    {
        out.push(active);
    }
    out
}

fn is_joint(scene: &Scene, id: u64) -> bool {
    scene.item(id).is_some_and(|i| i.is_joint())
}

fn pos(scene: &Scene, id: u64) -> DVec3 {
    scene
        .world_matrix(id)
        .map(|m| m.w_axis.truncate())
        .unwrap_or(DVec3::ZERO)
}

fn name_of(scene: &Scene, id: u64) -> String {
    scene.item(id).map(|i| i.name.clone()).unwrap_or_default()
}

/// The solver piece: `piece`, else the IK piece on the selected joint or
/// goal (or the joint a selected goal drives).
pub fn solver(scene: &Scene, a: &Args) -> Result<u64> {
    if let Some(p) = one(a, "piece")? {
        return match scene.animation.rig.piece(p) {
            Some(q) if ik::is_ik(&q.kind) => Ok(p),
            _ => Err(format!("{p} isn't an IK solver")),
        };
    }
    let picked = match one(a, "item")? {
        Some(i) => vec![i],
        None => picked(scene),
    };
    for id in picked {
        if let Some(p) = scene.animation.rig.pieces.iter().find(|p| {
            ik::is_ik(&p.kind)
                && (p.item == id
                    || p.targets.contains(&id)
                    || p.refs.values().any(|r| *r == id)
                    || ik::named(p, "mid") == Some(id)
                    || ik::named(p, "end") == Some(id))
        }) {
            return Ok(p.id);
        }
    }
    Err("Select an IK chain's joint or goal".into())
}

fn new_id(scene: &mut Scene) -> Result<u64> {
    if scene.next_id >= u64::MAX - 1 {
        return Err("Scene item/identity budget exhausted".into());
    }
    let id = scene.next_id;
    scene.next_id += 1;
    Ok(id)
}

/// A goal or pole locator at world `at` under `parent` (IK-02).
fn add_locator(
    scene: &mut Scene,
    name: String,
    parent: Option<u64>,
    at: DVec3,
    kind: LocatorKind,
) -> Result<u64> {
    let id = scene.add_group_locator_named(Some(name))?;
    let pw = match parent {
        Some(p) => scene.world_matrix(p)?,
        None => DMat4::IDENTITY,
    };
    let local = pw.inverse().transform_point3(at);
    let item = scene.item_mut(id).expect("made");
    item.locator = Some(kind);
    item.parent = parent;
    item.transform.translation = local.map(crate::rig::clean);
    Ok(id)
}

/// IK-02: `<root>_IKRoot` takes the root's place under its parent and
/// holds the root and `<root>_IKGoal`, a goal at `end` (placed and turned
/// like it). Returns the goal.
fn ik_root_and_goal(scene: &mut Scene, root: u64, root_name: &str, end: DMat4) -> Result<u64> {
    let parent = scene.item(root).and_then(|i| i.parent);
    let group = scene.add_group_locator_named(Some(format!("{root_name}_IKRoot")))?;
    scene.item_mut(group).expect("made").parent = parent;
    scene.set_parent(root, Some(group), true)?;
    let goal = add_locator(
        scene,
        format!("{root_name}_IKGoal"),
        Some(group),
        end.w_axis.truncate(),
        LocatorKind::IkGoal,
    )?;
    let gw = scene.world_matrix(group)?;
    let (_, q, _) = (gw.inverse() * end).to_scale_rotation_translation();
    let item = scene.item_mut(goal).expect("made");
    item.transform.rotation = item.transform.angles_for(q).map(crate::rig::clean);
    Ok(goal)
}

/// The handler for every `operandi.ik.*` edit.
pub fn run(e: &mut Editor, name: &str, a: &Args) -> Result<Option<Command>> {
    match name {
        "operandi.ik.applyTwoBone" => {
            if !e.scene.animation.setup {
                return Err(NEEDS_SETUP.into());
            }
            let joints = match ids(a, "joints")? {
                Some(j) => j,
                None => picked(&e.scene),
            };
            let [root, mid, end] = joints.as_slice() else {
                return Err(THREE_JOINTS.into());
            };
            let (root, mid, end) = (*root, *mid, *end);
            let s = &e.scene;
            if ![root, mid, end].iter().all(|j| is_joint(s, *j))
                || s.item(mid).and_then(|i| i.parent) != Some(root)
                || s.item(end).and_then(|i| i.parent) != Some(mid)
            {
                return Err(THREE_JOINTS.into());
            }
            if s.animation.rig.pieces.iter().any(|p| {
                ik::is_ik(&p.kind) && (p.item == root || ik::named(p, "mid") == Some(root))
            }) {
                return Err(format!("{} already has IK", name_of(s, root)));
            }
            let parent = s.item(root).and_then(|i| i.parent);
            let pw = match parent {
                Some(p) => s.world_matrix(p)?,
                None => DMat4::IDENTITY,
            };
            let (pa, pb, pc) = (pos(s, root), pos(s, mid), pos(s, end));
            let bend = ik::rest_bend(pa, pb, pc, pw).ok_or(STRAIGHT)?;
            let root_name = name_of(s, root);
            let end_world = s.world_matrix(end)?;
            e.anim_step("Apply IK (Two-Bone)", |e| {
                let goal = ik_root_and_goal(&mut e.scene, root, &root_name, end_world)?;
                let id = new_id(&mut e.scene)?;
                let mut p = Piece::new(id, TWO_BONE, root, vec![goal]);
                p.names.insert("mid".into(), mid.to_string());
                p.names.insert("end".into(), end.to_string());
                for (k, axis) in ["X", "Y", "Z"].iter().enumerate() {
                    p.values
                        .insert(format!("bend.{axis}"), crate::rig::clean(bend[k]));
                }
                e.scene.animation.rig.pieces.push(p);
                for j in [root, mid, end] {
                    crate::skin::ensure_setup_channels(&mut e.scene, j);
                }
                e.scene.select_item(Some(goal));
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.ik.applyChain" => {
            // IK-15: root then end, 2 to 100 joints.
            if !e.scene.animation.setup {
                return Err(NEEDS_SETUP.into());
            }
            let joints = match ids(a, "joints")? {
                Some(j) => j,
                None => picked(&e.scene),
            };
            let [root, end] = joints.as_slice() else {
                return Err(ROOT_THEN_END.into());
            };
            let (root, end) = (*root, *end);
            let s = &e.scene;
            let chain = ik::chain_down(s, root, end).ok_or(ROOT_THEN_END)?;
            if chain.len() < 2 || chain.len() > 100 || !chain.iter().all(|j| is_joint(s, *j)) {
                return Err(ROOT_THEN_END.into());
            }
            if s.animation
                .rig
                .pieces
                .iter()
                .any(|p| ik::is_ik(&p.kind) && chain.contains(&p.item))
            {
                return Err(format!("{} already has IK", name_of(s, root)));
            }
            let end_world = s.world_matrix(end)?;
            let root_name = name_of(s, root);
            e.anim_step("Apply IK (Chain)", |e| {
                let goal = ik_root_and_goal(&mut e.scene, root, &root_name, end_world)?;
                let id = new_id(&mut e.scene)?;
                let mut p = Piece::new(id, CHAIN, root, vec![goal]);
                p.names.insert("end".into(), end.to_string());
                let gw = e.scene.world_matrix(goal)?;
                let (_, q, _) = gw.to_scale_rotation_translation();
                let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
                for (axis, v) in ["X", "Y", "Z"].iter().zip([x, y, z]) {
                    p.values
                        .insert(format!("twist.{axis}"), crate::rig::clean(v));
                }
                e.scene.animation.rig.pieces.push(p);
                for j in &chain {
                    crate::skin::ensure_setup_channels(&mut e.scene, *j);
                }
                e.scene.select_item(Some(goal));
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.ik.pole" => {
            // IK-06: create (default), assign an item, or remove.
            let piece = solver(&e.scene, a)?;
            let mode = a.get("mode").and_then(Value::as_str).unwrap_or("create");
            let p = e.scene.animation.rig.piece(piece).expect("found").clone();
            match mode {
                "create" => {
                    let (Some(mid), Some(end)) = (ik::named(&p, "mid"), ik::named(&p, "end"))
                    else {
                        return Err("This solver has no middle joint".into());
                    };
                    let w = crate::rig_commands::evaluated_world;
                    let (a0, b0, c0) = (
                        w(&e.scene, p.item).w_axis.truncate(),
                        w(&e.scene, mid).w_axis.truncate(),
                        w(&e.scene, end).w_axis.truncate(),
                    );
                    let axis = (c0 - a0).normalize_or_zero();
                    let mut off = (b0 - a0) - axis * (b0 - a0).dot(axis);
                    if off.length_squared() < 1e-18 {
                        let parent = e.scene.item(p.item).and_then(|i| i.parent);
                        let pw = parent.map_or(DMat4::IDENTITY, |q| w(&e.scene, q));
                        off = pw.transform_vector3(p.vec3("bend."));
                    }
                    let at = b0 + off.normalize_or_zero() * (b0 - a0).length();
                    let holder = e.scene.item(p.item).and_then(|i| i.parent);
                    let root_name = name_of(&e.scene, p.item);
                    e.anim_step("Create IK Up Vector", |e| {
                        let pole = add_locator(
                            &mut e.scene,
                            format!("{root_name}_IKPole"),
                            holder,
                            at,
                            LocatorKind::Group,
                        )?;
                        e.scene
                            .animation
                            .rig
                            .piece_mut(piece)
                            .expect("found")
                            .refs
                            .insert("pole".into(), pole);
                        Ok(true)
                    })?;
                }
                "assign" => {
                    let item = one(a, "pole")?.ok_or("Give pole (an item)")?;
                    if e.scene.item(item).is_none() || item == p.item {
                        return Err("Pick another item as the pole".into());
                    }
                    e.anim_step("Assign Up Vector", |e| {
                        e.scene
                            .animation
                            .rig
                            .piece_mut(piece)
                            .expect("found")
                            .refs
                            .insert("pole".into(), item);
                        if let Some(err) = crate::rig_eval::would_loop(&e.scene) {
                            return Err(err);
                        }
                        Ok(true)
                    })?;
                }
                "remove" => {
                    e.anim_step("Remove Up Vector", |e| {
                        Ok(e.scene
                            .animation
                            .rig
                            .piece_mut(piece)
                            .expect("found")
                            .refs
                            .remove("pole")
                            .is_some())
                    })?;
                }
                other => return Err(format!("mode is create, assign or remove, not {other}")),
            }
            Ok(None)
        }
        _ => Err(format!("Unknown IK command {name}")),
    }
}

/// The chain's joints, root first.
pub fn chain(scene: &Scene, p: &Piece) -> Vec<u64> {
    if p.kind == CHAIN
        && let Some(end) = ik::named(p, "end")
        && let Some(c) = ik::chain_down(scene, p.item, end)
    {
        return c;
    }
    let mut out = vec![p.item];
    if let Some(m) = ik::named(p, "mid") {
        out.push(m);
    }
    if let Some(e) = ik::named(p, "end") {
        out.push(e);
    }
    out
}

/// `operandi.ik.query`: every IK solver (or one: `piece`) with its
/// chain's solved world positions and rotations (at `time` seconds,
/// default now), its goal, the distance left to the goal and whether it
/// is met (within `tolerance`, default 0.001).
pub fn answer(e: &Editor, name: &str, a: &Args) -> Option<Result<String>> {
    if name != "operandi.ik.query" {
        return None;
    }
    Some((|| {
        let only = one(a, "piece")?;
        let time = match a
            .get("time")
            .and_then(|v| v.as_f64().or_else(|| v.as_str()?.parse().ok()))
        {
            Some(s) => crate::time_tick::Tick::from_seconds(s),
            None => e.scene.time.current,
        };
        let tol = a.get("tolerance").and_then(Value::as_f64).unwrap_or(1e-3);
        let posed = crate::animation::evaluate_at(&e.scene, time);
        let result = crate::rig_eval::evaluate(&posed);
        let world = |id: u64| {
            result
                .world
                .get(&id)
                .copied()
                .unwrap_or_else(|| posed.world_matrix(id).unwrap_or(DMat4::IDENTITY))
        };
        let solvers: Vec<Value> = posed
            .animation
            .rig
            .pieces
            .iter()
            .filter(|p| ik::is_ik(&p.kind) && only.is_none_or(|o| o == p.id))
            .map(|p| {
                let joints: Vec<Value> = chain(&posed, p)
                    .into_iter()
                    .map(|j| {
                        let (_, q, t) = world(j).to_scale_rotation_translation();
                        let rotation = [q.w, q.x, q.y, q.z].map(crate::rig::clean);
                        json!({
                            "joint": j,
                            "name": name_of(&posed, j),
                            "position": t.to_array().map(crate::rig::clean),
                            "rotation": rotation,
                        })
                    })
                    .collect();
                let goal = p.targets.first().copied();
                let end = *chain(&posed, p).last().expect("root");
                let left = goal.map(|g| {
                    world(g)
                        .w_axis
                        .truncate()
                        .distance(world(end).w_axis.truncate())
                });
                json!({
                    "piece": p.id,
                    "kind": p.kind,
                    "root": p.item,
                    "goal": goal,
                    "pole": p.refs.get("pole"),
                    "blend": p.num("blend"),
                    "enable": p.flag("enable"),
                    "joints": joints,
                    "distance": left.map(crate::rig::clean),
                    "met": left.is_some_and(|d| d <= tol),
                })
            })
            .collect();
        Ok(json!({ "solvers": solvers }).to_string())
    })())
}
