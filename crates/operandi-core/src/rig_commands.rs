//! The rig commands (RG-10..46, §8 MCP): parenting, pivots, dynamic
//! parents and constraints, each one undo step (UIC-224..229) and usable
//! from a script without a selection (items and targets as arguments).
//! Without them, the selection in pick order decides: the first picked is
//! the item, the rest its targets (RG-26), or with `multiple:true` every
//! item but the last gets its own piece on the last (Ctrl+click).
//!
//! `compensate` defaults to the editor's Compensation (RG-11/RG-30).
use crate::animation::{self, AnimChannel, ChannelKey};
use crate::rig::{self, Piece};
use crate::{Command, Editor, Scene};
use glam::{DMat4, DQuat};
use serde_json::{Map, Value, json};

type Args = Map<String, Value>;
type Out = Result<Option<Command>, String>;

fn ids(a: &Args, name: &str) -> Result<Option<Vec<u64>>, String> {
    let parse = |s: &str| -> Result<Vec<u64>, String> {
        s.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<u64>()
                    .map_err(|_| format!("{name}: {s} isn't an item id"))
            })
            .collect()
    };
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .map(|v| Some(vec![v]))
            .ok_or_else(|| format!("{name} is an item id")),
        Some(Value::String(s)) => parse(s).map(Some),
        Some(Value::Array(list)) => list
            .iter()
            .map(|v| v.as_u64().ok_or_else(|| format!("{name} lists item ids")))
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err(format!("{name} lists item ids")),
    }
}
fn id(a: &Args, name: &str) -> Result<Option<u64>, String> {
    Ok(ids(a, name)?.and_then(|v| v.first().copied()))
}
fn number(a: &Args, name: &str) -> Result<Option<f64>, String> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
            .filter(|f| f.is_finite())
            .map(Some)
            .ok_or_else(|| format!("{name} must be a number")),
    }
}
fn flag(a: &Args, name: &str) -> Result<Option<bool>, String> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(Value::String(s)) => crate::command_args::boolean(name, s).map(Some),
        Some(_) => Err(format!("{name} must be true or false")),
    }
}
fn vec3(a: &Args, name: &str) -> Result<Option<glam::DVec3>, String> {
    let Some(v) = a.get(name) else {
        return Ok(None);
    };
    let parts: Vec<f64> = match v {
        Value::Null => return Ok(None),
        Value::String(s) => s
            .split(',')
            .map(|p| {
                p.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("{name} is x,y,z"))
            })
            .collect::<Result<_, _>>()?,
        Value::Array(list) => list
            .iter()
            .map(|p| p.as_f64().ok_or_else(|| format!("{name} is x,y,z")))
            .collect::<Result<_, _>>()?,
        _ => return Err(format!("{name} is x,y,z")),
    };
    match parts.as_slice() {
        [x, y, z] if parts.iter().all(|v| v.is_finite()) => Ok(Some(glam::DVec3::new(*x, *y, *z))),
        _ => Err(format!("{name} is x,y,z")),
    }
}

/// The selection in pick order (the active item was picked last).
fn picked(e: &Editor) -> Vec<u64> {
    let s = &e.scene.selection;
    let mut out: Vec<u64> = s
        .also
        .iter()
        .copied()
        .filter(|i| e.scene.item(*i).is_some())
        .collect();
    if let Some(active) = s.item
        && !out.contains(&active)
    {
        out.push(active);
    }
    out
}

/// An item's evaluated world now (the rig included).
pub fn evaluated_world(scene: &Scene, item: u64) -> DMat4 {
    crate::rig_eval::evaluate(scene)
        .world
        .get(&item)
        .copied()
        .unwrap_or_else(|| scene.world_matrix(item).unwrap_or(DMat4::IDENTITY))
}

fn item_name(scene: &Scene, id: u64) -> String {
    scene
        .item(id)
        .map_or_else(|| format!("item {id}"), |i| i.name.clone())
}

fn check_item(scene: &Scene, id: u64) -> Result<(), String> {
    scene
        .item(id)
        .map(|_| ())
        .ok_or_else(|| format!("No item {id}"))
}

fn new_id(scene: &mut Scene) -> Result<u64, String> {
    if scene.next_id >= u64::MAX - 1 {
        return Err("Scene item/identity budget exhausted".into());
    }
    let id = scene.next_id;
    scene.next_id += 1;
    Ok(id)
}

/// Keys `key` at `t` with its current value; stepped makes the curve
/// stepped (TK-14) so a switch holds until the next key.
fn key_now(scene: &mut Scene, key: ChannelKey, t: crate::time_tick::Tick, stepped: bool) {
    let Some(v) = animation::read(scene, &key) else {
        return;
    };
    let channel = scene
        .animation
        .channels
        .entry(key)
        .or_insert_with(|| AnimChannel {
            curve: Default::default(),
            rest: v,
        });
    channel.curve.set(t, v);
    if stepped {
        channel.curve.stepped = true;
    }
}

/// Refuses an edit that makes a loop (RG-02); otherwise keeps it.
fn refuse_loops(e: &Editor) -> Result<(), String> {
    match crate::rig_eval::would_loop(&e.scene) {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

/// Sets the new piece's offset so its item stays at `before` (RG-30).
fn compensate(scene: &mut Scene, piece: u64, before: DMat4) {
    let Some(p) = scene.animation.rig.piece(piece).cloned() else {
        return;
    };
    let after = evaluated_world(scene, p.item);
    let (s0, q0, t0) = before.to_scale_rotation_translation();
    let (s1, q1, t1) = after.to_scale_rotation_translation();
    let Some(p) = scene.animation.rig.piece_mut(piece) else {
        return;
    };
    let set3 = |p: &mut Piece, group: &str, v: glam::DVec3| {
        for (i, a) in ["X", "Y", "Z"].iter().enumerate() {
            let _ = p.set(&format!("{group}{a}"), rig::clean(v[i]));
        }
    };
    let has = |p: &Piece, n: &str| rig::channel_names(p).iter().any(|c| c == n);
    match p.kind.as_str() {
        "transform" => {
            // The offset is in the targets' space: after = blend · O.
            let o = after.inverse() * before;
            p.set_offset_matrix(p.offset_matrix() * o);
        }
        "dynamicParent" => {}
        _ => {
            if has(p, "offset.pos.X") {
                set3(p, "offset.pos.", p.vec3("offset.pos.") + (t0 - t1));
            }
            if has(p, "offset.rot.X") {
                let r = p.vec3("offset.rot.");
                let current = DQuat::from_euler(glam::EulerRot::XYZ, r.x, r.y, r.z);
                let q = (current * q1.inverse() * q0).normalize();
                let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
                set3(p, "offset.rot.", glam::DVec3::new(x, y, z));
            }
            if has(p, "offset.scl.X") && s1.min_element().abs() > 1e-12 {
                set3(p, "offset.scl.", p.vec3("offset.scl.") * s0 / s1);
            }
        }
    }
}

/// The handler for every `operandi.rig.*` line.
pub fn run(e: &mut Editor, name: &str, a: &Args) -> Out {
    let comp = flag(a, "compensate")?.unwrap_or(e.rig_compensation);
    match name {
        "operandi.rig.compensation" => {
            e.rig_compensation = match a.get("on").and_then(Value::as_str) {
                None | Some("toggle") => !e.rig_compensation,
                Some(s) => crate::command_args::boolean("on", s)?,
            };
            Ok(None)
        }
        "operandi.rig.constrain" => {
            let kind = a
                .get("kind")
                .and_then(Value::as_str)
                .ok_or("Give kind: position, rotation, scale, transform, direction, distance, path, pathPosition or pathNormal")?;
            if !rig::CONSTRAINTS.contains(&kind) {
                return Err(format!("{kind} isn't a constraint kind"));
            }
            let multiple = flag(a, "multiple")?.unwrap_or(false);
            let (items, targets) = match (ids(a, "items")?.or(ids(a, "item")?), ids(a, "targets")?)
            {
                (Some(items), Some(targets)) => (items, targets),
                (None, None) => {
                    let picked = picked(e);
                    if picked.len() < 2 {
                        return Err("Select the item to constrain, then its target(s)".into());
                    }
                    if multiple {
                        let (last, rest) = picked.split_last().expect("two or more");
                        (rest.to_vec(), vec![*last])
                    } else {
                        (vec![picked[0]], picked[1..].to_vec())
                    }
                }
                _ => return Err("Give both items and targets, or neither (the selection)".into()),
            };
            if targets.is_empty() {
                return Err("Give at least one target".into());
            }
            for i in items.iter().chain(&targets) {
                check_item(&e.scene, *i)?;
            }
            let label = format!("Apply {}", rig::label(kind));
            e.anim_step(&label, |e| {
                for &item in &items {
                    let before = evaluated_world(&e.scene, item);
                    let id = new_id(&mut e.scene)?;
                    let targets: Vec<u64> =
                        targets.iter().copied().filter(|t| *t != item).collect();
                    if targets.is_empty() {
                        return Err(format!("{} can't target itself", item_name(&e.scene, item)));
                    }
                    e.scene
                        .animation
                        .rig
                        .pieces
                        .push(Piece::new(id, kind, item, targets));
                    if comp {
                        compensate(&mut e.scene, id, before);
                    }
                }
                refuse_loops(e)?;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.unconstrain" => {
            let kind = a.get("kind").and_then(Value::as_str).ok_or("Give kind")?;
            let items = ids(a, "items")?
                .or(ids(a, "item")?)
                .unwrap_or_else(|| picked(e));
            let gone: Vec<u64> = e
                .scene
                .animation
                .rig
                .pieces
                .iter()
                .filter(|p| p.kind == kind && items.contains(&p.item))
                .map(|p| p.id)
                .collect();
            if gone.is_empty() {
                return Err(format!("No {} on the selected items", rig::label(kind)));
            }
            e.anim_step(&format!("Remove {}", rig::label(kind)), |e| {
                remove_pieces(&mut e.scene, &gone);
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.remove" => {
            let piece = id(a, "piece")?.ok_or("Give piece")?;
            let p = e
                .scene
                .animation
                .rig
                .piece(piece)
                .cloned()
                .ok_or(format!("No rig piece {piece}"))?;
            e.anim_step(&format!("Delete {}", rig::label(&p.kind)), |e| {
                let before = evaluated_world(&e.scene, p.item);
                remove_pieces(&mut e.scene, &[piece]);
                if comp && p.kind == "dynamicParent" {
                    keep_in_place(&mut e.scene, p.item, before)?;
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.set" => {
            let piece = id(a, "piece")?.ok_or("Give piece")?;
            let channel = a
                .get("channel")
                .and_then(Value::as_str)
                .ok_or("Give channel")?;
            let value = number(a, "value")?.ok_or("Give value")?;
            let p = e
                .scene
                .animation
                .rig
                .piece(piece)
                .ok_or(format!("No rig piece {piece}"))?;
            let label = format!("{} {channel}", rig::label(&p.kind));
            e.anim_step(&label, |e| {
                e.scene
                    .animation
                    .rig
                    .piece_mut(piece)
                    .expect("checked")
                    .set(channel, value)?;
                refuse_loops(e)?;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.upVector" => {
            let piece = id(a, "piece")?.ok_or("Give piece")?;
            let item = id(a, "item")?;
            let axis = match a.get("axis").and_then(Value::as_str) {
                None => -1.,
                Some("X" | "x") => 0.,
                Some("Y" | "y") => 1.,
                Some("Z" | "z") => 2.,
                Some(other) => return Err(format!("axis is X, Y or Z, not {other}")),
            };
            let p = e
                .scene
                .animation
                .rig
                .piece(piece)
                .ok_or(format!("No rig piece {piece}"))?;
            if p.kind != "direction" {
                return Err("Up vectors belong to direction constraints".into());
            }
            if let Some(i) = item {
                check_item(&e.scene, i)?;
            }
            let label = match (item, axis >= 0.) {
                (None, _) => "Remove Up Vector",
                (Some(_), false) => "Assign Up Vector",
                (Some(_), true) => "Assign Axis Up Vector",
            };
            e.anim_step(label, |e| {
                let p = e.scene.animation.rig.piece_mut(piece).expect("checked");
                match item {
                    Some(i) => {
                        p.refs.insert("upItem".into(), i);
                        p.set("upAxis", axis)?;
                    }
                    None => {
                        p.refs.remove("upItem");
                        p.set("upAxis", -1.)?;
                    }
                }
                refuse_loops(e)?;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.dynamicParentAdd" => {
            let (item, parents) = match (id(a, "item")?, ids(a, "parents")?) {
                (Some(i), Some(p)) => (i, p),
                (None, None) => {
                    let picked = picked(e);
                    if picked.len() < 2 {
                        return Err("Select the child, then its parent(s)".into());
                    }
                    (picked[0], picked[1..].to_vec())
                }
                _ => return Err("Give both item and parents, or neither (the selection)".into()),
            };
            for i in std::iter::once(&item).chain(&parents) {
                check_item(&e.scene, *i)?;
            }
            if parents.contains(&item) {
                return Err("An item can't be its own parent".into());
            }
            e.anim_step("Add Parent", |e| {
                let before = evaluated_world(&e.scene, item);
                let existing = e
                    .scene
                    .animation
                    .rig
                    .on(item)
                    .find(|p| p.kind == "dynamicParent")
                    .map(|p| p.id);
                let piece = match existing {
                    Some(p) => p,
                    None => {
                        let id = new_id(&mut e.scene)?;
                        // Before the item's constraints: parent space first (RG-01).
                        let at = e
                            .scene
                            .animation
                            .rig
                            .pieces
                            .iter()
                            .position(|p| p.item == item)
                            .unwrap_or(e.scene.animation.rig.pieces.len());
                        e.scene
                            .animation
                            .rig
                            .pieces
                            .insert(at, Piece::new(id, "dynamicParent", item, Vec::new()));
                        id
                    }
                };
                let p = e.scene.animation.rig.piece_mut(piece).expect("made");
                for parent in &parents {
                    if !p.targets.contains(parent) {
                        p.targets.push(*parent);
                    }
                }
                let index = p.targets.len() as f64;
                p.set("parent", index)?;
                refuse_loops(e)?;
                if comp {
                    switch_offsets(&mut e.scene, piece, before);
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.dynamicParentSet" => {
            let piece = match (id(a, "piece")?, id(a, "item")?) {
                (Some(p), _) => p,
                (None, Some(i)) => e
                    .scene
                    .animation
                    .rig
                    .on(i)
                    .find(|p| p.kind == "dynamicParent")
                    .map(|p| p.id)
                    .ok_or_else(|| format!("{} has no dynamic parent", item_name(&e.scene, i)))?,
                (None, None) => return Err("Give item or piece".into()),
            };
            let index = number(a, "parent")?.ok_or("Give parent (0 = World)")?;
            let p = e
                .scene
                .animation
                .rig
                .piece(piece)
                .ok_or(format!("No rig piece {piece}"))?;
            if p.kind != "dynamicParent" {
                return Err("That piece isn't a dynamic parent".into());
            }
            if index < 0. || index.round() as usize > p.targets.len() {
                return Err(format!("parent is 0 (World) to {}", p.targets.len()));
            }
            let item = p.item;
            let key_key = ChannelKey::new(piece, "parent");
            let animated = e.scene.animation.channels.contains_key(&key_key);
            let auto = e.scene.animation.settings.auto_key != animation::AutoKey::Off;
            let key = flag(a, "key")?.unwrap_or(animated && auto);
            e.anim_step("Switch Parent", |e| {
                let before = evaluated_world(&e.scene, item);
                e.scene
                    .animation
                    .rig
                    .piece_mut(piece)
                    .expect("checked")
                    .set("parent", index.round())?;
                if comp {
                    switch_offsets(&mut e.scene, piece, before);
                }
                if key {
                    // RG-23 [OP]: the switch and its offsets keyed together,
                    // stepped, so scrubbing back restores the old offset.
                    let t = e.anim.key_time.unwrap_or(e.scene.time.current);
                    let mut names = vec!["parent".to_owned()];
                    for g in ["offset.pos.", "offset.rot.", "offset.scl."] {
                        for a in ["X", "Y", "Z"] {
                            names.push(format!("{g}{a}"));
                        }
                    }
                    for n in names {
                        key_now(&mut e.scene, ChannelKey::new(piece, &n), t, true);
                    }
                }
                Ok(true)
            })?;
            if !comp && !e.anim.warned_compensation {
                e.anim.warned_compensation = true;
                e.anim.note = Some("Turn on Compensation to keep the item in place".into());
            }
            Ok(None)
        }
        "operandi.rig.dynamicParentRemove" => {
            let item = id(a, "item")?
                .or_else(|| picked(e).first().copied())
                .ok_or("Give item")?;
            let piece = e
                .scene
                .animation
                .rig
                .on(item)
                .find(|p| p.kind == "dynamicParent")
                .map(|p| p.id)
                .ok_or_else(|| format!("{} has no dynamic parent", item_name(&e.scene, item)))?;
            e.anim_step("Remove Dynamic Parent", |e| {
                let before = evaluated_world(&e.scene, item);
                remove_pieces(&mut e.scene, &[piece]);
                if comp {
                    keep_in_place(&mut e.scene, item, before)?;
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.parent" => {
            let (items, parent) = match (ids(a, "items")?.or(ids(a, "item")?), id(a, "parent")?) {
                (Some(i), Some(p)) => (i, p),
                (None, None) => {
                    let picked = picked(e);
                    let (last, rest) = picked
                        .split_last()
                        .ok_or("Select the items, then their parent last")?;
                    if rest.is_empty() {
                        return Err("Select the items, then their parent last".into());
                    }
                    (rest.to_vec(), *last)
                }
                _ => return Err("Give both items and parent, or neither (the selection)".into()),
            };
            check_item(&e.scene, parent)?;
            e.anim_step("Parent", |e| {
                for i in &items {
                    e.scene.set_parent(*i, Some(parent), comp)?;
                }
                refuse_loops(e)?;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.unparent" => {
            let items = ids(a, "items")?
                .or(ids(a, "item")?)
                .unwrap_or_else(|| picked(e));
            if items.is_empty() {
                return Err("Select the items to unparent".into());
            }
            e.anim_step("Unparent", |e| {
                for i in &items {
                    e.scene.set_parent(*i, None, comp)?;
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.insert" => {
            // RG-12: the first item becomes the second's parent, in the
            // second's old place.
            let (first, second) = match (id(a, "item")?, id(a, "child")?) {
                (Some(f), Some(s)) => (f, s),
                (None, None) => match picked(e).as_slice() {
                    [f, s, ..] => (*f, *s),
                    _ => return Err("Select the new parent, then the item to put under it".into()),
                },
                _ => return Err("Give both item and child, or neither (the selection)".into()),
            };
            check_item(&e.scene, first)?;
            check_item(&e.scene, second)?;
            e.anim_step("Insert", |e| {
                let place = e.scene.item(second).and_then(|i| i.parent);
                e.scene.set_parent(first, place, comp)?;
                e.scene.set_parent(second, Some(first), comp)?;
                refuse_loops(e)?;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.extract" => {
            let item = id(a, "item")?
                .or_else(|| picked(e).first().copied())
                .ok_or("Give item")?;
            check_item(&e.scene, item)?;
            e.anim_step("Extract", |e| {
                let up = e.scene.item(item).and_then(|i| i.parent);
                let children: Vec<u64> = e
                    .scene
                    .items
                    .iter()
                    .filter(|i| i.parent == Some(item))
                    .map(|i| i.id)
                    .collect();
                for c in children {
                    e.scene.set_parent(c, up, comp)?;
                }
                e.scene.set_parent(item, None, comp)?;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.pivot" => {
            let item = id(a, "item")?
                .or(e.scene.selection.item)
                .ok_or("Give item")?;
            check_item(&e.scene, item)?;
            let position = vec3(a, "position")?;
            let rotation = vec3(a, "rotation")?;
            if position.is_none() && rotation.is_none() {
                return Err("Give position and/or rotation (x,y,z)".into());
            }
            e.anim_step("Set Pivot", |e| {
                let before = e.scene.world_matrix(item)?;
                let old = e
                    .scene
                    .animation
                    .rig
                    .pivots
                    .get(&item)
                    .copied()
                    .unwrap_or_default();
                let new = rig::Pivot {
                    position: position.unwrap_or(old.position),
                    rotation: rotation.unwrap_or(old.rotation),
                };
                if new == rig::Pivot::default() {
                    e.scene.animation.rig.pivots.remove(&item);
                } else {
                    e.scene.animation.rig.pivots.insert(item, new);
                }
                // RG-14: the geometry stays; the position compensates.
                let after = e.scene.world_matrix(item)?;
                let shift = before.w_axis.truncate() - after.w_axis.truncate();
                let parent = match e.scene.item(item).and_then(|i| i.parent) {
                    Some(p) => e.scene.world_matrix(p)?,
                    None => DMat4::IDENTITY,
                };
                let local_shift = parent.inverse().transform_vector3(shift);
                let it = e.scene.item_mut(item).expect("checked");
                it.transform.translation += local_shift;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.linkAdd" => {
            let driven = text_arg(a, "driven")?.ok_or("Give driven (item:channel)")?;
            let driver = text_arg(a, "driver")?.ok_or("Give driver (item:channel)")?;
            let mode = match text_arg(a, "mode")?.unwrap_or("direct") {
                "direct" => "direct",
                "relationship" => "relationship",
                other => return Err(format!("mode is direct or relationship, not {other}")),
            };
            let pairs = link_pairs(
                &e.scene,
                &ChannelKey::parse(driven)?,
                &ChannelKey::parse(driver)?,
            )?;
            let label = if mode == "direct" {
                "Add Link"
            } else {
                "Add Relationship"
            };
            e.anim_step(label, |e| {
                for (to, from) in &pairs {
                    if e.scene.animation.rig.pieces.iter().any(|p| {
                        p.kind == "link"
                            && p.item == to.item
                            && p.names.get("driven") == Some(&to.channel)
                    }) {
                        return Err(format!("{} is already driven", to.channel));
                    }
                    let id = new_id(&mut e.scene)?;
                    let mut p = Piece::new(id, "link", to.item, vec![from.item]);
                    p.names.insert("driven".into(), to.channel.clone());
                    p.names.insert("driver".into(), from.channel.clone());
                    p.names.insert("mode".into(), mode.into());
                    if mode == "relationship" {
                        p.curve = Some(Default::default());
                    }
                    e.scene.animation.rig.pieces.push(p);
                }
                refuse_loops(e)?;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.linkRemove" => {
            let gone: Vec<u64> = match (id(a, "piece")?, text_arg(a, "driven")?) {
                (Some(p), _) => vec![p],
                (None, Some(d)) => {
                    let k = ChannelKey::parse(d)?;
                    e.scene
                        .animation
                        .rig
                        .pieces
                        .iter()
                        .filter(|p| {
                            p.kind == "link"
                                && p.item == k.item
                                && p.names.get("driven").is_some_and(|c| {
                                    *c == k.channel || c.starts_with(&format!("{}.", k.channel))
                                })
                        })
                        .map(|p| p.id)
                        .collect()
                }
                (None, None) => return Err("Give piece or driven".into()),
            };
            if gone.is_empty()
                || gone.iter().any(|g| {
                    e.scene
                        .animation
                        .rig
                        .piece(*g)
                        .is_none_or(|p| p.kind != "link")
                })
            {
                return Err("No such link".into());
            }
            e.anim_step("Remove Link", |e| {
                remove_pieces(&mut e.scene, &gone);
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.relationshipKey" | "operandi.rig.relationshipKeyRemove" => {
            let piece = id(a, "piece")?.ok_or("Give piece")?;
            let p = e
                .scene
                .animation
                .rig
                .piece(piece)
                .ok_or(format!("No rig piece {piece}"))?;
            if p.curve.is_none() {
                return Err("That piece isn't a relationship link".into());
            }
            let from = match (p.targets.first(), p.names.get("driver")) {
                (Some(&i), Some(c)) => ChannelKey::new(i, c),
                _ => return Err("The link has no driver".into()),
            };
            let to = ChannelKey::new(p.item, p.names.get("driven").ok_or("No driven channel")?);
            let driver = match number(a, "driver")? {
                Some(v) => v,
                None => animation::read(&e.scene, &from).ok_or("The driver has no value")?,
            };
            let remove = name == "operandi.rig.relationshipKeyRemove";
            let driven = match number(a, "driven")? {
                Some(v) => v,
                None => animation::read(&e.scene, &to).ok_or("The driven channel has no value")?,
            };
            e.anim_step(
                if remove {
                    "Delete Relationship Key"
                } else {
                    "Relationship Key"
                },
                |e| {
                    let c = e
                        .scene
                        .animation
                        .rig
                        .piece_mut(piece)
                        .and_then(|p| p.curve.as_mut())
                        .expect("checked");
                    let t = crate::time_tick::Tick::from_seconds(driver);
                    if remove {
                        if !c.remove(t) {
                            return Err("No relationship key at that driver value".into());
                        }
                    } else {
                        c.set(t, driven);
                    }
                    Ok(true)
                },
            )?;
            Ok(None)
        }
        "operandi.rig.userChannelAdd" => {
            let item = id(a, "item")?
                .or(e.scene.selection.item)
                .ok_or("Give item")?;
            check_item(&e.scene, item)?;
            let name = text_arg(a, "name")?.ok_or("Give name")?.trim().to_owned();
            if name.is_empty() || name.contains(|c: char| c.is_whitespace() || c == ':' || c == ',')
            {
                return Err("A user channel name has no spaces, colons or commas".into());
            }
            let ty = match text_arg(a, "type")? {
                None => rig::UserType::Float,
                Some(t) => rig::UserType::parse(t).ok_or_else(|| format!("Unknown type {t}"))?,
            };
            let mode = text_arg(a, "mode")?
                .unwrap_or("scalar")
                .to_ascii_lowercase();
            rig::mode_components(&mode).ok_or_else(|| format!("Unknown mode {mode}"))?;
            let channel = rig::UserChannel {
                item,
                name: name.clone(),
                display: text_arg(a, "display")?.unwrap_or("").to_owned(),
                ty,
                mode: if mode.is_empty() {
                    "scalar".into()
                } else {
                    mode
                },
                default: number(a, "default")?.unwrap_or(0.),
                min: number(a, "min")?,
                max: number(a, "max")?,
                choices: text_arg(a, "choices")?
                    .map(|c| c.split(',').map(|s| s.trim().to_owned()).collect())
                    .unwrap_or_default(),
                values: Default::default(),
                text: String::new(),
            };
            let taken = |n: &str| {
                animation::keyable_channels(&e.scene, item)
                    .iter()
                    .any(|c| c == n)
                    || e.scene
                        .animation
                        .rig
                        .user
                        .iter()
                        .any(|u| u.item == item && u.name == name)
            };
            if channel.channel_names().iter().any(|n| taken(n)) || taken(&name) {
                return Err(format!(
                    "{} already has a channel {name}",
                    item_name(&e.scene, item)
                ));
            }
            e.anim_step("Add User Channel", |e| {
                e.scene.animation.rig.user.push(channel);
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.userChannelEdit" => {
            let item = id(a, "item")?
                .or(e.scene.selection.item)
                .ok_or("Give item")?;
            let name = text_arg(a, "name")?.ok_or("Give name")?.to_owned();
            let index = e
                .scene
                .animation
                .rig
                .user
                .iter()
                .position(|u| u.item == item && u.name == name)
                .ok_or_else(|| format!("No user channel {name}"))?;
            let rename = text_arg(a, "rename")?.map(str::to_owned);
            let display = text_arg(a, "display")?.map(str::to_owned);
            let (min, max, default) = (number(a, "min")?, number(a, "max")?, number(a, "default")?);
            e.anim_step("Edit User Channel", |e| {
                let old_names = e.scene.animation.rig.user[index].channel_names();
                let u = &mut e.scene.animation.rig.user[index];
                if let Some(d) = display {
                    u.display = d;
                }
                if min.is_some() {
                    u.min = min;
                }
                if max.is_some() {
                    u.max = max;
                }
                if let Some(d) = default {
                    u.default = d;
                }
                if let Some(r) = rename {
                    u.name = r;
                    // Keys and links follow the new name.
                    let new_names = e.scene.animation.rig.user[index].channel_names();
                    for (old, new) in old_names.iter().zip(&new_names) {
                        if let Some(c) = e
                            .scene
                            .animation
                            .channels
                            .remove(&ChannelKey::new(item, old))
                        {
                            e.scene
                                .animation
                                .channels
                                .insert(ChannelKey::new(item, new), c);
                        }
                        for p in &mut e.scene.animation.rig.pieces {
                            if p.item == item && p.names.get("driven") == Some(old) {
                                p.names.insert("driven".into(), new.clone());
                            }
                            if p.targets.first() == Some(&item)
                                && p.names.get("driver") == Some(old)
                            {
                                p.names.insert("driver".into(), new.clone());
                            }
                        }
                    }
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.userChannelRemove" => {
            let item = id(a, "item")?
                .or(e.scene.selection.item)
                .ok_or("Give item")?;
            let name = text_arg(a, "name")?.ok_or("Give name")?.to_owned();
            let u = e
                .scene
                .animation
                .rig
                .user
                .iter()
                .find(|u| u.item == item && u.name == name)
                .cloned()
                .ok_or_else(|| format!("No user channel {name}"))?;
            let names = u.channel_names();
            let links: Vec<u64> = e
                .scene
                .animation
                .rig
                .pieces
                .iter()
                .filter(|p| {
                    (p.item == item && p.names.get("driven").is_some_and(|d| names.contains(d)))
                        || (p.targets.first() == Some(&item)
                            && p.names.get("driver").is_some_and(|d| names.contains(d)))
                })
                .map(|p| p.id)
                .collect();
            e.anim_step("Delete User Channel", |e| {
                e.scene
                    .animation
                    .rig
                    .user
                    .retain(|x| !(x.item == item && x.name == name));
                for n in &names {
                    e.scene.animation.channels.remove(&ChannelKey::new(item, n));
                }
                remove_pieces(&mut e.scene, &links);
                Ok(true)
            })?;
            if !links.is_empty() {
                e.anim.note = Some(format!(
                    "Deleted {name} and the {} link(s) using it",
                    links.len()
                ));
            }
            Ok(None)
        }
        "operandi.rig.bindUserChannel" => {
            // RG-54: a user channel of the same kind on target, linked
            // into the channel, which becomes driven.
            let source =
                ChannelKey::parse(text_arg(a, "channel")?.ok_or("Give channel (item:name)")?)?;
            let target = id(a, "target")?.ok_or("Give target")?;
            check_item(&e.scene, target)?;
            let value = animation::read(&e.scene, &source)
                .ok_or_else(|| format!("{} isn't a number channel", source.channel))?;
            let ty = match source.transform_part().map(|(g, _)| g) {
                Some("pos") => rig::UserType::Distance,
                Some("rot") => rig::UserType::Angle,
                Some(_) => rig::UserType::Percentage,
                None => rig::UserType::Float,
            };
            let base = text_arg(a, "name")?
                .map(str::to_owned)
                .unwrap_or_else(|| source.channel.replace('.', "_"));
            let mut name = base.clone();
            let mut n = 2;
            while e
                .scene
                .animation
                .rig
                .user
                .iter()
                .any(|u| u.item == target && u.name == name)
            {
                name = format!("{base}{n}");
                n += 1;
            }
            e.anim_step("Bind into User Channel", |e| {
                e.scene.animation.rig.user.push(rig::UserChannel {
                    item: target,
                    name: name.clone(),
                    display: String::new(),
                    ty,
                    mode: "scalar".into(),
                    default: value,
                    min: None,
                    max: None,
                    choices: Vec::new(),
                    values: Default::default(),
                    text: String::new(),
                });
                let id = new_id(&mut e.scene)?;
                let mut p = Piece::new(id, "link", source.item, vec![target]);
                p.names.insert("driven".into(), source.channel.clone());
                p.names.insert("driver".into(), name.clone());
                p.names.insert("mode".into(), "direct".into());
                e.scene.animation.rig.pieces.push(p);
                refuse_loops(e)?;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.modifierAdd" | "operandi.rig.modifierSet" => {
            let adding = name == "operandi.rig.modifierAdd";
            let (piece, kind) = if adding {
                let kind = text_arg(a, "kind")?
                    .ok_or("Give kind: expression, blend, math, clamp or switch")?;
                let kind = format!("mod.{kind}");
                if !rig::MODIFIERS.contains(&kind.as_str()) {
                    return Err(format!("{} isn't a modifier kind", &kind[4..]));
                }
                (None, kind)
            } else {
                let p = id(a, "piece")?.ok_or("Give piece")?;
                let piece = e
                    .scene
                    .animation
                    .rig
                    .piece(p)
                    .ok_or(format!("No rig piece {p}"))?;
                if !piece.kind.starts_with("mod.") {
                    return Err("That piece isn't a modifier".into());
                }
                (Some(p), piece.kind.clone())
            };
            // output:item:channel, inputs:A=3:pos.X,B=4:rot.Y (an empty
            // channel unwires), expr, op.
            let output = text_arg(a, "output")?.map(ChannelKey::parse).transpose()?;
            let mut wires: Vec<(String, Option<ChannelKey>)> = Vec::new();
            if let Some(list) = text_arg(a, "inputs")? {
                for part in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                    let (n, k) = part
                        .split_once('=')
                        .ok_or_else(|| format!("Inputs are name=item:channel, not {part}"))?;
                    let n = n.trim().to_owned();
                    if n.is_empty() || !n.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        return Err(format!("Input names are letters, digits and _, not {n}"));
                    }
                    let k = k.trim();
                    wires.push((
                        n,
                        if k.is_empty() {
                            None
                        } else {
                            Some(ChannelKey::parse(k)?)
                        },
                    ));
                }
            }
            if wires.iter().filter(|w| w.1.is_some()).count() > 16 {
                return Err("A modifier takes up to 16 inputs".into());
            }
            for k in wires
                .iter()
                .filter_map(|w| w.1.as_ref())
                .chain(output.as_ref())
            {
                if animation::read(&e.scene, k).is_none() {
                    return Err(format!("{}:{} isn't a number channel", k.item, k.channel));
                }
            }
            let expr = text_arg(a, "expr")?.map(str::to_owned);
            if let Some(x) = &expr {
                crate::rig_expression::Expr::parse(x).map_err(|err| {
                    format!("Expression error at {}: {}", err.position + 1, err.message)
                })?;
            }
            let op = text_arg(a, "op")?.map(str::to_owned);
            if let Some(o) = &op
                && !rig::MATH_OPS.contains(&o.as_str())
            {
                return Err(format!("op is one of {}", rig::MATH_OPS.join(", ")));
            }
            let owner = match (&output, piece) {
                (Some(k), _) => k.item,
                (None, Some(p)) => e.scene.animation.rig.piece(p).expect("checked").item,
                (None, None) => id(a, "item")?
                    .or(e.scene.selection.item)
                    .ok_or("Give output or item")?,
            };
            check_item(&e.scene, owner)?;
            let label = if adding {
                format!("Add {}", rig::label(&kind))
            } else {
                format!("Edit {}", rig::label(&kind))
            };
            e.anim_step(&label, |e| {
                let piece_id = match piece {
                    Some(p) => p,
                    None => {
                        let id = new_id(&mut e.scene)?;
                        let mut p = Piece::new(id, &kind, owner, Vec::new());
                        if kind == "mod.expression" && expr.is_none() {
                            p.names.insert("expr".into(), "A".into());
                        }
                        e.scene.animation.rig.pieces.push(p);
                        id
                    }
                };
                let p = e.scene.animation.rig.piece_mut(piece_id).expect("made");
                if let Some(k) = &output {
                    p.item = k.item;
                    p.names.insert("driven".into(), k.channel.clone());
                }
                for (n, k) in &wires {
                    match k {
                        Some(k) => {
                            p.names
                                .insert(format!("in.{n}"), format!("{}:{}", k.item, k.channel));
                        }
                        None => {
                            p.names.remove(&format!("in.{n}"));
                        }
                    }
                }
                if let Some(x) = &expr {
                    p.names.insert("expr".into(), x.clone());
                }
                if let Some(o) = &op {
                    p.names.insert("op".into(), o.clone());
                }
                refuse_loops(e)?;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.bake" => {
            // §8 Bake: the evaluated world of each item as plain keys per
            // frame over the current range, its rig pieces disabled.
            let items = ids(a, "items")?
                .or(ids(a, "item")?)
                .unwrap_or_else(|| e.scene.selected_items());
            if items.is_empty() {
                return Err("Select the items to bake".into());
            }
            for i in &items {
                check_item(&e.scene, *i)?;
            }
            // Phase 6 §7: an IK chain bakes whole, and its solver is then
            // turned to FK (Blend keyed to 0) rather than disabled.
            let solvers: Vec<u64> = e
                .scene
                .animation
                .rig
                .pieces
                .iter()
                .filter(|p| crate::ik::is_ik(&p.kind))
                .filter(|p| {
                    crate::ik_commands::chain(&e.scene, p)
                        .iter()
                        .any(|j| items.contains(j))
                })
                .map(|p| p.id)
                .collect();
            let mut items = items;
            for &s in &solvers {
                let p = e.scene.animation.rig.piece(s).expect("listed");
                for j in crate::ik_commands::chain(&e.scene, p) {
                    if !items.contains(&j) {
                        items.push(j);
                    }
                }
            }
            let rate = e.scene.time.rate;
            let (ra, rb) = animation::current_range(&e.scene);
            let start = number(a, "start")?.map_or(ra, |f| animation::at_frames(rate, f));
            let end = number(a, "end")?.map_or(rb, |f| animation::at_frames(rate, f));
            if end < start {
                return Err("end is before start".into());
            }
            let (fa, fb) = (rate.frame_at(start), rate.frame_at(end));
            if fb - fa > 100_000 {
                return Err("That range is too long to bake".into());
            }
            // Sample first, on a copy: posing other times doesn't touch
            // the document until the step records.
            let mut samples: Vec<(u64, Vec<(crate::time_tick::Tick, DMat4)>)> =
                items.iter().map(|i| (*i, Vec::new())).collect();
            let mut probe = e.scene.clone();
            for f in fa..=fb {
                let t = rate.tick_of(f);
                probe.time.current = t;
                animation::pose(&mut probe, t);
                let r = crate::rig_eval::evaluate(&probe);
                for (item, list) in &mut samples {
                    let world =
                        r.world.get(item).copied().unwrap_or_else(|| {
                            probe.world_matrix(*item).unwrap_or(DMat4::IDENTITY)
                        });
                    // The parent's world as it will be once the item's own
                    // pieces are off (its static parent, evaluated).
                    let parent =
                        probe
                            .item(*item)
                            .and_then(|i| i.parent)
                            .map_or(DMat4::IDENTITY, |p| {
                                r.world.get(&p).copied().unwrap_or_else(|| {
                                    probe.world_matrix(p).unwrap_or(DMat4::IDENTITY)
                                })
                            });
                    list.push((t, parent.inverse() * world));
                }
            }
            e.anim_step("Bake Selected", |e| {
                for (item, list) in &samples {
                    let mut previous: Option<glam::DVec3> = None;
                    for (t, local) in list {
                        let (s, q, tr) = local.to_scale_rotation_translation();
                        let mut r = e
                            .scene
                            .item(*item)
                            .expect("checked")
                            .transform
                            .angles_for(q);
                        // Keep angles continuous across frames.
                        if let Some(p) = previous {
                            for a in 0..3 {
                                r[a] += ((p[a] - r[a]) / std::f64::consts::TAU).round()
                                    * std::f64::consts::TAU;
                            }
                        }
                        previous = Some(r);
                        for (g, v) in [("pos", tr), ("rot", r), ("scl", s)] {
                            for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
                                let key = ChannelKey::new(*item, &format!("{g}.{axis}"));
                                let rest = animation::read(&e.scene, &key).unwrap_or(0.);
                                let ch =
                                    e.scene.animation.channels.entry(key).or_insert_with(|| {
                                        AnimChannel {
                                            curve: Default::default(),
                                            rest,
                                        }
                                    });
                                ch.curve.set(*t, rig::clean(v[i]));
                            }
                        }
                    }
                    // Disable (don't delete) the pieces that moved it.
                    for p in e
                        .scene
                        .animation
                        .rig
                        .pieces
                        .iter_mut()
                        .filter(|p| p.item == *item && !crate::ik::is_ik(&p.kind))
                    {
                        let _ = p.set("enable", 0.);
                    }
                }
                for &s in &solvers {
                    let key = ChannelKey::new(s, "blend");
                    let ch = e
                        .scene
                        .animation
                        .channels
                        .entry(key)
                        .or_insert_with(|| AnimChannel {
                            curve: Default::default(),
                            rest: 1.,
                        });
                    ch.curve.keys.clear();
                    ch.curve.set(start, 0.);
                    if let Some(p) = e.scene.animation.rig.piece_mut(s) {
                        let _ = p.set("blend", 0.);
                    }
                }
                let now = e.scene.time.current;
                animation::pose(&mut e.scene, now);
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.rig.unwire" => {
            // RG-72: deleting a wire in the Schematic.
            let piece = id(a, "piece")?.ok_or("Give piece")?;
            let port = text_arg(a, "port")?.ok_or("Give port")?.to_owned();
            let p = e
                .scene
                .animation
                .rig
                .piece(piece)
                .cloned()
                .ok_or(format!("No rig piece {piece}"))?;
            e.anim_step("Delete Wire", |e| {
                if p.kind == "link" {
                    remove_pieces(&mut e.scene, &[piece]);
                    return Ok(true);
                }
                let q = e.scene.animation.rig.piece_mut(piece).expect("checked");
                if q.kind.starts_with("mod.") {
                    if port == "out" {
                        q.names.remove("driven");
                    } else if q.names.remove(&format!("in.{port}")).is_none() {
                        return Err(format!("{port} isn't wired"));
                    }
                    return Ok(true);
                }
                // A constraint's or dynamic parent's target: drop it, and
                // the piece with its last one.
                let index = port
                    .rsplit(' ')
                    .next()
                    .and_then(|n| n.parse::<usize>().ok())
                    .and_then(|n| n.checked_sub(1))
                    .filter(|i| *i < q.targets.len());
                match index {
                    Some(i) => {
                        q.targets.remove(i);
                        // Weights after it move down one.
                        let weights: Vec<(usize, f64)> = (i + 1..=q.targets.len())
                            .filter_map(|n| {
                                q.values.remove(&format!("weight.{n}")).map(|w| (n - 1, w))
                            })
                            .collect();
                        q.values.remove(&format!("weight.{i}"));
                        for (n, w) in weights {
                            q.values.insert(format!("weight.{n}"), w);
                        }
                        if q.targets.is_empty() {
                            remove_pieces(&mut e.scene, &[piece]);
                        }
                    }
                    None if q.refs.remove(&port).is_some() => {}
                    None => return Err(format!("No wire at {port}")),
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.anim.setup" => {
            // SK-01: a view state (UI class): no undo step.
            let on = match a.get("on").and_then(Value::as_str) {
                None | Some("toggle") => !e.scene.animation.setup,
                Some(s) => crate::command_args::boolean("on", s)?,
            };
            if on != e.scene.animation.setup {
                let tool = e.settle_tool();
                e.scene.animation.setup = on;
                let now = e.scene.time.current;
                animation::pose(&mut e.scene, now);
                e.resume_tool(tool);
            }
            Ok(None)
        }
        "operandi.anim.itemToSetup" | "operandi.anim.restoreSetup" => {
            // SK-03: per item, and per channel with channels:.
            let to_setup = name == "operandi.anim.itemToSetup";
            let items = ids(a, "items")?
                .or(ids(a, "item")?)
                .unwrap_or_else(|| e.scene.selected_items());
            let only: Option<Vec<String>> = text_arg(a, "channels")?.map(|c| {
                c.split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect()
            });
            if items.is_empty() {
                return Err("Select the items".into());
            }
            let keys: Vec<ChannelKey> = e
                .scene
                .animation
                .channels
                .keys()
                .filter(|k| items.contains(&k.item))
                .filter(|k| only.as_ref().is_none_or(|o| o.contains(&k.channel)))
                .cloned()
                .collect();
            if keys.is_empty() {
                return Err("No animated channels to change".into());
            }
            e.anim_step(
                if to_setup {
                    "Item to Setup"
                } else {
                    "Restore Setup"
                },
                |e| {
                    for k in &keys {
                        let now = animation::read(&e.scene, k);
                        let Some(c) = e.scene.animation.channels.get_mut(k) else {
                            continue;
                        };
                        if to_setup {
                            // The current (edit) value becomes the setup value.
                            if let Some(v) = now {
                                c.rest = v;
                            }
                        } else {
                            // The edit values go: no keys, the setup value back.
                            let rest = c.rest;
                            e.scene.animation.channels.remove(k);
                            let _ = animation::write(&mut e.scene, k, rest);
                        }
                    }
                    Ok(true)
                },
            )?;
            Ok(None)
        }
        _ => Err(format!("Unknown rig command {name}")),
    }
}

fn text_arg<'a>(a: &'a Args, name: &str) -> Result<Option<&'a str>, String> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.as_str())),
        Some(_) => Err(format!("{name} is text")),
    }
}

/// The (driven, driver) channel pairs a link makes (RG-51): one pair, or
/// one per component when both name a group (`pos`, a vector user
/// channel). Both must hold numbers.
fn link_pairs(
    scene: &Scene,
    to: &ChannelKey,
    from: &ChannelKey,
) -> Result<Vec<(ChannelKey, ChannelKey)>, String> {
    check_item(scene, to.item)?;
    check_item(scene, from.item)?;
    let parts = |k: &ChannelKey| -> Vec<ChannelKey> {
        let mut names: Vec<String> = animation::keyable_channels(scene, k.item)
            .into_iter()
            .filter(|n| {
                n.strip_prefix(&k.channel)
                    .is_some_and(|r| r.starts_with('.') && !r[1..].contains('.'))
            })
            .collect();
        names.sort();
        names
            .into_iter()
            .map(|n| ChannelKey::new(k.item, &n))
            .collect()
    };
    let number = |k: &ChannelKey| animation::read(scene, k).is_some();
    let text = |k: &ChannelKey| {
        scene
            .animation
            .rig
            .user_channel(k.item, &k.channel)
            .is_some_and(|u| !u.ty.numeric())
    };
    if text(to) || text(from) {
        return Err("Text channels can't be linked to numbers".into());
    }
    if number(to) && number(from) {
        return Ok(vec![(to.clone(), from.clone())]);
    }
    let (a, b) = (parts(to), parts(from));
    if !a.is_empty() && a.len() == b.len() {
        return Ok(a.into_iter().zip(b).collect());
    }
    Err(format!(
        "{} and {} can't be linked (each needs a number, or both the same number of parts)",
        to.channel, from.channel
    ))
}

/// Removes pieces and their channels' keys.
fn remove_pieces(scene: &mut Scene, ids: &[u64]) {
    scene.animation.rig.pieces.retain(|p| !ids.contains(&p.id));
    scene
        .animation
        .channels
        .retain(|k, _| !ids.contains(&k.item));
    scene
        .animation
        .shadows
        .retain(|k, _| !ids.contains(&k.item));
}

/// Rewrites `item`'s own transform so its world stays `before` (RG-25).
fn keep_in_place(scene: &mut Scene, item: u64, before: DMat4) -> Result<(), String> {
    let now = evaluated_world(scene, item);
    let parent_part = now
        * scene
            .local_matrix(scene.item(item).ok_or("No item")?)
            .inverse();
    let local = parent_part.inverse() * before;
    let (s, q, t) = local.to_scale_rotation_translation();
    let it = scene.item_mut(item).ok_or("No item")?;
    it.transform.translation = t.map(rig::clean);
    it.transform.rotation = it.transform.angles_for(q).map(rig::clean);
    it.transform.scale = s.map(rig::clean);
    Ok(())
}

/// After a switch, sets the dynamic parent's offsets so the item stays at
/// `before` (RG-22/23): world = parent · O · local.
fn switch_offsets(scene: &mut Scene, piece: u64, before: DMat4) {
    let Some(p) = scene.animation.rig.piece(piece).cloned() else {
        return;
    };
    let Some(item) = scene.item(p.item) else {
        return;
    };
    let local = scene.local_matrix(item);
    let index = p.num("parent").round() as usize;
    let parent = match index {
        0 => DMat4::IDENTITY,
        n => p
            .targets
            .get(n - 1)
            .map_or(DMat4::IDENTITY, |&t| evaluated_world(scene, t)),
    };
    let o = parent.inverse() * before * local.inverse();
    if let Some(p) = scene.animation.rig.piece_mut(piece) {
        p.set_offset_matrix(o);
    }
}

/// `operandi.rig.query item?`: every piece (on the item, or all) with its
/// channels, inputs, outputs and state, and the item's evaluated world.
pub fn answer(e: &Editor, name: &str, a: &Args) -> Option<Result<String, String>> {
    if name != "operandi.rig.query" {
        return None;
    }
    Some((|| {
        let item = id(a, "item")?;
        let rig = &e.scene.animation.rig;
        let result = e.eval.rig(&e.scene);
        let pieces: Vec<Value> = rig
            .pieces
            .iter()
            .filter(|p| item.is_none_or(|i| p.item == i))
            .map(|p| {
                let channels: Map<String, Value> = rig::channel_names(p)
                    .into_iter()
                    .map(|n| {
                        let v = p.get(&n).unwrap_or(0.);
                        (n, json!(v))
                    })
                    .collect();
                json!({
                    "id": p.id,
                    "kind": p.kind,
                    "label": rig::label(&p.kind),
                    "item": p.item,
                    "targets": p.targets,
                    "refs": p.refs,
                    "channels": channels,
                    "missing": p.is_missing(),
                    "broken": result.broken.contains(&p.id),
                    "names": p.names,
                    "error": p.names.get("expr").and_then(|x| crate::rig_expression::Expr::parse(x).err()).map(|err| json!({"position": err.position, "message": err.message})),
                    "value": crate::rig_eval::evaluate(&e.scene).driven.iter().rev().find(|(k, _)| k.item == p.item && p.names.get("driven") == Some(&k.channel)).map(|(_, v)| *v),
                    "curve": p.curve.as_ref().map(|c| c.keys.iter().map(|k| [k.time.seconds(), k.value]).collect::<Vec<_>>()),
                    "inputs": p.targets.iter().chain(p.refs.values()).collect::<Vec<_>>(),
                    "outputs": [p.item],
                })
            })
            .collect();
        let mut out = json!({ "pieces": pieces, "compensation": e.rig_compensation });
        if let Some(i) = item {
            check_item(&e.scene, i)?;
            let w = result
                .world
                .get(&i)
                .copied()
                .unwrap_or_else(|| e.scene.world_matrix(i).unwrap_or(DMat4::IDENTITY));
            out["world"] = world_json(w);
            let driven: Vec<Value> = crate::animation::TRANSFORM_CHANNELS
                .iter()
                .filter_map(|n| {
                    rig::driven_by(&e.scene, i, n).map(|p| json!({"channel": n, "piece": p.id}))
                })
                .collect();
            out["driven"] = json!(driven);
            out["driver"] = json!(rig::drives(&e.scene, i));
            let user: Vec<Value> = rig
                .user
                .iter()
                .filter(|u| u.item == i)
                .map(|u| {
                    let values: Map<String, Value> = u
                        .channel_names()
                        .into_iter()
                        .map(|n| {
                            let v = u.get(&n).unwrap_or(0.);
                            (n, json!(v))
                        })
                        .collect();
                    json!({"name": u.name, "display": u.display, "type": u.ty.name(), "mode": u.mode,
                           "default": u.default, "min": u.min, "max": u.max, "choices": u.choices,
                           "values": values})
                })
                .collect();
            out["user"] = json!(user);
            if let Some(pv) = rig.pivots.get(&i) {
                out["pivot"] =
                    json!({"position": pv.position.to_array(), "rotation": pv.rotation.to_array()});
            }
        }
        Ok(out.to_string())
    })())
}

/// A world matrix as position, rotation (XYZ radians), scale and its axes.
pub fn world_json(w: DMat4) -> Value {
    let (s, q, t) = w.to_scale_rotation_translation();
    let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
    let round = |v: glam::DVec3| v.to_array().map(rig::clean);
    json!({
        "position": round(t),
        "rotation": [rig::clean(x), rig::clean(y), rig::clean(z)],
        "scale": round(s),
        "axes": {
            "X": round(w.x_axis.truncate().normalize_or_zero()),
            "Y": round(w.y_axis.truncate().normalize_or_zero()),
            "Z": round(w.z_axis.truncate().normalize_or_zero()),
        },
    })
}
