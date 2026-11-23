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
        _ => Err(format!("Unknown rig command {name}")),
    }
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
