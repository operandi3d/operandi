//! Joints and the Skeleton tool's edits (BEHAVIOR-RIG-PHASE5 §2,
//! SK-10..19). A joint is an ordinary item in the hierarchy, a locator of
//! kind Joint, so parenting, constraints and links work on it unchanged
//! (§0). Its bone runs to its first child (SK-17: the first in item
//! order), which also decides its alignment (SK-16).
//!
//! Every edit is one `operandi.skel.*` command and one undo step
//! (SK-19); the Skeleton tool in the app runs them on clicks and drags.
use crate::locator::LocatorKind;
use crate::scene::{Item, Scene, Transform};
use crate::{Editor, Result};
use glam::{DMat3, DMat4, DQuat, DVec3};
use serde_json::{Map, Value, json};

type Args = Map<String, Value>;

impl Item {
    pub fn is_joint(&self) -> bool {
        self.locator == Some(LocatorKind::Joint)
    }
}

/// The joint's children in item order; the first decides its bone.
pub fn children(scene: &Scene, joint: u64) -> Vec<u64> {
    scene
        .items
        .iter()
        .filter(|i| i.parent == Some(joint) && i.is_joint())
        .map(|i| i.id)
        .collect()
}

pub fn first_child(scene: &Scene, joint: u64) -> Option<u64> {
    children(scene, joint).first().copied()
}

fn world_pos(scene: &Scene, id: u64) -> DVec3 {
    scene
        .world_matrix(id)
        .map(|m| m.w_axis.truncate())
        .unwrap_or(DVec3::ZERO)
}

/// The bone length: to the first child, or 0 for a leaf.
pub fn bone_length(scene: &Scene, joint: u64) -> f64 {
    first_child(scene, joint).map_or(0., |c| {
        world_pos(scene, c).distance(world_pos(scene, joint))
    })
}

/// Mirror axis for symmetry (SK-14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symmetry {
    Off,
    Axis(usize),
}

impl Symmetry {
    pub fn parse(s: &str) -> Result<Symmetry> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "off" | "" => Symmetry::Off,
            "x" => Symmetry::Axis(0),
            "y" => Symmetry::Axis(1),
            "z" => Symmetry::Axis(2),
            other => return Err(format!("symmetry is off, X, Y or Z, not {other}")),
        })
    }
    fn mirror(self, p: DVec3) -> DVec3 {
        match self {
            Symmetry::Off => p,
            Symmetry::Axis(a) => {
                let mut q = p;
                q[a] = -q[a];
                q
            }
        }
    }
}

/// Naming (SK-15): prefix, name and a number, with the side's suffix.
#[derive(Clone, Debug)]
pub struct Naming {
    pub prefix: String,
    pub name: String,
    pub plus: String,
    pub minus: String,
}

impl Default for Naming {
    fn default() -> Self {
        Naming {
            prefix: String::new(),
            name: "Joint".into(),
            plus: "_L".into(),
            minus: "_R".into(),
        }
    }
}

impl Naming {
    /// The next free name: prefix + name + number + suffix.
    fn next(&self, scene: &Scene, suffix: &str) -> String {
        let mut n = 1;
        loop {
            let candidate = format!("{}{}{}{}", self.prefix, self.name, n, suffix);
            if !scene.items.iter().any(|i| i.name == candidate) {
                return candidate;
            }
            n += 1;
        }
    }
}

/// A joint at `world`, under `parent` (its local transform placed so the
/// world position is `world`). Returns its id.
pub fn add_joint(
    scene: &mut Scene,
    parent: Option<u64>,
    world: DVec3,
    name: String,
) -> Result<u64> {
    let id = scene.add_group_locator_named(Some(name))?;
    let parent_world = match parent {
        Some(p) => scene.world_matrix(p)?,
        None => DMat4::IDENTITY,
    };
    let local = parent_world.inverse().transform_point3(world);
    let item = scene.item_mut(id).expect("made");
    item.locator = Some(LocatorKind::Joint);
    item.parent = parent;
    item.transform = Transform {
        translation: local,
        ..Default::default()
    };
    Ok(id)
}

/// Sets `id`'s world matrix; its children keep their world places (the
/// Skeleton tool's Compensation, SK-13, and alignment, SK-16).
fn set_world_keep_children(scene: &mut Scene, id: u64, world: DMat4) -> Result<()> {
    let kids: Vec<(u64, DMat4)> = scene
        .items
        .iter()
        .filter(|i| i.parent == Some(id))
        .map(|i| (i.id, scene.world_matrix(i.id).unwrap_or(DMat4::IDENTITY)))
        .collect();
    set_world(scene, id, world)?;
    for (k, w) in kids {
        set_world(scene, k, w)?;
    }
    Ok(())
}

/// Sets `id`'s local transform so its world matrix is `world` (children
/// follow).
fn set_world(scene: &mut Scene, id: u64, world: DMat4) -> Result<()> {
    let parent = match scene.item(id).ok_or("No such item")?.parent {
        Some(p) => scene.world_matrix(p)?,
        None => DMat4::IDENTITY,
    };
    let local = parent.inverse() * world;
    let (s, q, t) = local.to_scale_rotation_translation();
    let item = scene.item_mut(id).expect("checked");
    item.transform.translation = t.map(crate::rig::clean);
    item.transform.rotation = item.transform.angles_for(q).map(crate::rig::clean);
    item.transform.scale = s.map(crate::rig::clean);
    Ok(())
}

/// Aims `joint`'s `direction` axis down its bone with `axis` across it
/// (SK-16); a leaf keeps its orientation. Children keep their places.
pub fn align(scene: &mut Scene, joint: u64, direction: usize, axis: usize) -> Result<()> {
    let Some(child) = first_child(scene, joint) else {
        return Ok(());
    };
    let p = world_pos(scene, joint);
    let d = world_pos(scene, child) - p;
    if d.length_squared() < 1e-18 || direction == axis {
        return Ok(());
    }
    let d = d.normalize();
    // The across axis: as near world up's perpendicular as it can be.
    let mut up = DVec3::Y;
    if d.cross(up).length_squared() < 1e-12 {
        up = DVec3::Z;
    }
    let across = up.cross(d).normalize();
    let mut cols = [DVec3::ZERO; 3];
    cols[direction] = d;
    cols[axis] = across;
    let third = 3 - direction - axis;
    // Right-handed: X × Y = Z.
    cols[third] = cols[(third + 1) % 3].cross(cols[(third + 2) % 3]);
    let q = DQuat::from_mat3(&DMat3::from_cols(cols[0], cols[1], cols[2])).normalize();
    let world = scene.world_matrix(joint)?;
    let (s, _, t) = world.to_scale_rotation_translation();
    set_world_keep_children(
        scene,
        joint,
        DMat4::from_scale_rotation_translation(s, q, t),
    )
}

fn axis_arg(a: &Args, name: &str, default: usize) -> Result<usize> {
    Ok(match a.get(name).and_then(Value::as_str) {
        None => default,
        Some("X" | "x") => 0,
        Some("Y" | "y") => 1,
        Some("Z" | "z") => 2,
        Some(other) => return Err(format!("{name} is X, Y or Z, not {other}")),
    })
}

fn vec3(a: &Args, name: &str) -> Result<Option<DVec3>> {
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
            .collect::<Result<_>>()?,
        Value::Array(list) => list
            .iter()
            .map(|p| p.as_f64().ok_or_else(|| format!("{name} is x,y,z")))
            .collect::<Result<_>>()?,
        _ => return Err(format!("{name} is x,y,z")),
    };
    match parts.as_slice() {
        [x, y, z] if parts.iter().all(|v| v.is_finite()) => Ok(Some(DVec3::new(*x, *y, *z))),
        _ => Err(format!("{name} is x,y,z")),
    }
}

fn id_arg(a: &Args, name: &str) -> Result<Option<u64>> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{name} is an item id")),
        Some(Value::String(s)) => s
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| format!("{name} is an item id")),
        Some(_) => Err(format!("{name} is an item id")),
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

fn text<'a>(a: &'a Args, name: &str) -> Option<&'a str> {
    a.get(name).and_then(Value::as_str)
}

fn check_joint(scene: &Scene, id: u64) -> Result<()> {
    match scene.item(id) {
        Some(i) if i.is_joint() => Ok(()),
        Some(i) => Err(format!("{} isn't a joint", i.name)),
        None => Err(format!("No item {id}")),
    }
}

/// The mirror of joint `id` by name (its other side's suffix).
fn mirror_of(scene: &Scene, id: u64, naming: &Naming) -> Option<u64> {
    let name = &scene.item(id)?.name;
    let other = if let Some(base) = name.strip_suffix(&naming.plus) {
        format!("{base}{}", naming.minus)
    } else if let Some(base) = name.strip_suffix(&naming.minus) {
        format!("{base}{}", naming.plus)
    } else {
        return Some(id);
    };
    scene
        .items
        .iter()
        .find(|i| i.name == other && i.is_joint())
        .map(|i| i.id)
}

/// The handler for every `operandi.skel.*` line.
pub fn run(e: &mut Editor, name: &str, a: &Args) -> Result<Option<crate::Command>> {
    let naming = Naming {
        prefix: text(a, "prefix").unwrap_or("").into(),
        name: text(a, "name").unwrap_or("Joint").into(),
        plus: text(a, "suffixPlus").unwrap_or("_L").into(),
        minus: text(a, "suffixMinus").unwrap_or("_R").into(),
    };
    let direction = axis_arg(a, "direction", 2)?;
    let across = axis_arg(a, "axis", 0)?;
    let preserve = flag(a, "preserveAlignment", false)?;
    match name {
        "operandi.skel.add" => {
            let at = vec3(a, "position")?.ok_or("Give position x,y,z")?;
            let parent = id_arg(a, "parent")?;
            if let Some(p) = parent {
                check_joint(&e.scene, p)?;
            }
            let symmetry = Symmetry::parse(text(a, "symmetry").unwrap_or("off"))?;
            e.anim_step("Add Joint", |e| {
                let on_plane = match symmetry {
                    Symmetry::Off => true,
                    Symmetry::Axis(ax) => at[ax].abs() < 1e-9,
                };
                let side = |p: DVec3| match symmetry {
                    Symmetry::Axis(ax) if p[ax] < 0. => naming.minus.clone(),
                    Symmetry::Axis(_) => naming.plus.clone(),
                    Symmetry::Off => String::new(),
                };
                let mut made = Vec::new();
                let suffix = if on_plane { String::new() } else { side(at) };
                let first_name = naming.next(&e.scene, &suffix);
                let first = add_joint(&mut e.scene, parent, at, first_name)?;
                made.push((first, parent));
                if !on_plane {
                    let mirrored = symmetry.mirror(at);
                    let mparent = parent.and_then(|p| mirror_of(&e.scene, p, &naming));
                    // The mirror's name keeps the number of its partner.
                    let partner = e.scene.item(first).expect("made").name.clone();
                    let base = partner.strip_suffix(&suffix).unwrap_or(&partner).to_owned();
                    let mname = format!("{base}{}", side(mirrored));
                    let m = add_joint(&mut e.scene, mparent, mirrored, mname)?;
                    made.push((m, mparent));
                }
                if !preserve {
                    for (_, p) in &made {
                        if let Some(p) = p
                            && first_child(&e.scene, *p)
                                .is_some_and(|c| made.iter().any(|m| m.0 == c))
                        {
                            align(&mut e.scene, *p, direction, across)?;
                        }
                    }
                }
                e.scene.select_item(Some(first));
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.skel.move" => {
            let joint = id_arg(a, "joint")?.ok_or("Give joint")?;
            check_joint(&e.scene, joint)?;
            let at = vec3(a, "position")?.ok_or("Give position x,y,z")?;
            let compensate = flag(a, "compensate", true)?;
            e.anim_step("Move Joint", |e| {
                let world = e.scene.world_matrix(joint)?;
                let (s, q, _) = world.to_scale_rotation_translation();
                let new = DMat4::from_scale_rotation_translation(s, q, at);
                if compensate {
                    set_world_keep_children(&mut e.scene, joint, new)?;
                } else {
                    set_world(&mut e.scene, joint, new)?;
                }
                if !preserve {
                    if let Some(p) = e.scene.item(joint).and_then(|i| i.parent)
                        && e.scene.item(p).is_some_and(Item::is_joint)
                    {
                        align(&mut e.scene, p, direction, across)?;
                    }
                    align(&mut e.scene, joint, direction, across)?;
                }
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.skel.insert" => {
            // SK-12 Insert: split the bone from joint to its first child.
            let joint = id_arg(a, "joint")?.ok_or("Give joint")?;
            check_joint(&e.scene, joint)?;
            let child = first_child(&e.scene, joint).ok_or("A leaf joint has no bone to split")?;
            e.anim_step("Insert Joint", |e| {
                let mid = (world_pos(&e.scene, joint) + world_pos(&e.scene, child)) / 2.;
                let name = naming.next(&e.scene, "");
                let new = add_joint(&mut e.scene, Some(joint), mid, name)?;
                // The new joint takes the child's place first under joint.
                let at = e.scene.item_index(child).expect("child");
                let item = e.scene.items.remove(e.scene.item_index(new).expect("made"));
                e.scene.items.insert(at, item);
                e.scene.set_parent(child, Some(new), true)?;
                if !preserve {
                    align(&mut e.scene, joint, direction, across)?;
                    align(&mut e.scene, new, direction, across)?;
                }
                e.scene.select_item(Some(new));
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.skel.delete" => {
            // SK-12 Delete: children move up to the joint's parent.
            let joint = id_arg(a, "joint")?.ok_or("Give joint")?;
            check_joint(&e.scene, joint)?;
            e.anim_step("Delete Joint", |e| {
                let up = e.scene.item(joint).and_then(|i| i.parent);
                for c in e
                    .scene
                    .items
                    .iter()
                    .filter(|i| i.parent == Some(joint))
                    .map(|i| i.id)
                    .collect::<Vec<_>>()
                {
                    e.scene.set_parent(c, up, true)?;
                }
                e.scene
                    .delete_items(&[joint], crate::item_delete::ChildrenOnDelete::MoveToRoot)?;
                e.scene.select_item(up);
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.skel.align" => {
            let joints: Vec<u64> = match id_arg(a, "joint")? {
                Some(j) => vec![j],
                None => e
                    .scene
                    .selected_items()
                    .into_iter()
                    .filter(|i| e.scene.item(*i).is_some_and(Item::is_joint))
                    .collect(),
            };
            if joints.is_empty() {
                return Err("Select joints to align".into());
            }
            let chain = flag(a, "chain", true)?;
            e.anim_step("Align Joints", |e| {
                let mut todo = joints.clone();
                while let Some(j) = todo.pop() {
                    check_joint(&e.scene, j)?;
                    align(&mut e.scene, j, direction, across)?;
                    if chain {
                        todo.extend(children(&e.scene, j));
                    }
                }
                Ok(true)
            })?;
            Ok(None)
        }
        _ => Err(format!("Unknown skeleton command {name}")),
    }
}

/// `operandi.skel.query`: every joint (or `root`'s chain) with its
/// parent, first child, world position, bone length and axes.
pub fn answer(e: &Editor, name: &str, a: &Args) -> Option<Result<String>> {
    if name != "operandi.skel.query" {
        return None;
    }
    Some((|| {
        let root = id_arg(a, "root")?;
        let mut ids: Vec<u64> = Vec::new();
        match root {
            Some(r) => {
                check_joint(&e.scene, r)?;
                let mut todo = vec![r];
                while let Some(j) = todo.pop() {
                    ids.push(j);
                    let mut kids = children(&e.scene, j);
                    kids.reverse();
                    todo.extend(kids);
                }
            }
            None => {
                ids = e
                    .scene
                    .items
                    .iter()
                    .filter(|i| i.is_joint())
                    .map(|i| i.id)
                    .collect()
            }
        }
        let joints: Vec<Value> = ids
            .iter()
            .map(|&j| {
                let item = e.scene.item(j).expect("listed");
                let w = e.scene.world_matrix(j).unwrap_or(DMat4::IDENTITY);
                json!({
                    "id": j,
                    "name": item.name,
                    "parent": item.parent,
                    "firstChild": first_child(&e.scene, j),
                    "children": children(&e.scene, j),
                    "position": w.w_axis.truncate().to_array().map(crate::rig::clean),
                    "boneLength": crate::rig::clean(bone_length(&e.scene, j)),
                    "axes": {
                        "X": w.x_axis.truncate().normalize_or_zero().to_array().map(crate::rig::clean),
                        "Y": w.y_axis.truncate().normalize_or_zero().to_array().map(crate::rig::clean),
                        "Z": w.z_axis.truncate().normalize_or_zero().to_array().map(crate::rig::clean),
                    },
                })
            })
            .collect();
        Ok(json!({ "joints": joints }).to_string())
    })())
}

/// Where a click places a joint (SK-11): with Intersection, halfway
/// between the first surface the ray enters and the one it leaves by;
/// else (or with one surface) the first hit. `None` when the ray misses
/// every shown mesh.
pub fn placement(scene: &Scene, origin: DVec3, dir: DVec3, intersection: bool) -> Option<DVec3> {
    let dir = dir.normalize();
    let mut hits: Vec<f64> = Vec::new();
    for item in scene.items.iter().filter(|i| i.is_mesh() && scene.shows(i)) {
        let Ok(w) = scene.world_matrix(item.id) else {
            continue;
        };
        let mesh = item.mesh.open();
        let p: Vec<DVec3> = mesh
            .vertices
            .iter()
            .map(|v| w.transform_point3(*v))
            .collect();
        for f in mesh.faces.iter() {
            if f.vertices.len() < 3 {
                continue;
            }
            let a = p[f.vertices[0] as usize];
            for k in 1..f.vertices.len() - 1 {
                let (b, c) = (p[f.vertices[k] as usize], p[f.vertices[k + 1] as usize]);
                if let Some(t) = ray_triangle(origin, dir, a, b, c) {
                    hits.push(t);
                }
            }
        }
    }
    hits.sort_by(|a, b| a.total_cmp(b));
    hits.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    match hits.as_slice() {
        [] => None,
        [t0, t1, ..] if intersection => Some(origin + dir * (t0 + t1) / 2.),
        [t0, ..] => Some(origin + dir * *t0),
    }
}

fn ray_triangle(o: DVec3, d: DVec3, a: DVec3, b: DVec3, c: DVec3) -> Option<f64> {
    let e1 = b - a;
    let e2 = c - a;
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-14 {
        return None;
    }
    let inv = 1. / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(0. ..=1.).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0. || u + v > 1. {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-9).then_some(t)
}
