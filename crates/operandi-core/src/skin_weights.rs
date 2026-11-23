//! Weight tools for skins (BEHAVIOR-RIG-PHASE5 §5, SK-40..45). The brushes
//! (Paint, Smooth, Scale Down, Erase, Set Value) are the Vertex Map Tools
//! (sculpt_weights.rs, F11); for a joint's map on a bound mesh their
//! Normalize Weights works across that skin's joint maps only (SK-43).
//! The non-brush tools here (SK-42) and per-point reads and writes (§7
//! MCP) are `operandi.weight.*` commands, each one undo step. Weight maps
//! stay F7 point_weights maps (SK-45).
use crate::{Command, Editor, Mesh, Mode, Result, Scene};
use glam::DVec3;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};

type Args = Map<String, Value>;
type Weights = BTreeMap<u32, f64>;

/// The weight maps of mesh `item`'s skin, if it is bound.
pub fn skin_maps(scene: &Scene, item: u64) -> Option<Vec<String>> {
    crate::skin_commands::skin_op(scene, item)
        .ok()
        .map(|(_, v)| v.maps)
}

/// SK-40: the bound mesh and map of joint `joint` (the first skin that
/// uses it).
pub fn map_for_joint(scene: &Scene, joint: u64) -> Option<(u64, String)> {
    scene
        .items
        .iter()
        .filter(|i| i.mesh_ops.is_some())
        .find_map(|i| {
            let (_, v) = crate::skin_commands::skin_op(scene, i.id).ok()?;
            let k = v.joints.iter().position(|j| *j == joint)?;
            Some((i.id, v.maps[k].clone()))
        })
}

/// SK-23: a joint's map follows its name. Renames every skin map of
/// `joint` from `old` to its current name.
pub fn follow_rename(scene: &mut Scene, joint: u64, old: &str) {
    let Some(new) = scene.item(joint).map(|i| i.name.clone()) else {
        return;
    };
    if new == old {
        return;
    }
    let meshes: Vec<u64> = scene
        .items
        .iter()
        .filter(|i| i.mesh_ops.is_some())
        .map(|i| i.id)
        .collect();
    for m in meshes {
        let Ok((op, mut v)) = crate::skin_commands::skin_op(scene, m) else {
            continue;
        };
        let Some(k) = v.joints.iter().position(|j| *j == joint) else {
            continue;
        };
        if v.maps[k] != old {
            continue;
        }
        v.maps[k] = new.clone();
        let item = scene.item_mut(m).expect("listed");
        let mut mesh = item.mesh.open().clone();
        if let Some(w) = mesh.attributes.point_weights.remove(old) {
            mesh.attributes.point_weights.insert(new.clone(), w);
        }
        item.mesh = mesh.into();
        let _ = crate::skin_commands::write_skin(scene, m, op, &v);
    }
}

fn id_arg(a: &Args, name: &str) -> Result<Option<u64>> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{name} is an id")),
        Some(Value::String(s)) => s
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| format!("{name} is an id")),
        Some(_) => Err(format!("{name} is an id")),
    }
}

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

fn flag(a: &Args, name: &str, default: bool) -> Result<bool> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(b)) => Ok(*b),
        Some(Value::String(s)) => crate::command_args::boolean(name, s),
        Some(_) => Err(format!("{name} must be true or false")),
    }
}

fn text<'a>(a: &'a Args, name: &str) -> Option<&'a str> {
    a.get(name)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn points_arg(a: &Args) -> Result<Option<Vec<u32>>> {
    match a.get("points") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => s
            .split(',')
            .map(|p| {
                p.trim()
                    .parse()
                    .map_err(|_| "points are point indices".to_string())
            })
            .collect::<Result<Vec<u32>>>()
            .map(Some),
        Some(Value::Array(list)) => list
            .iter()
            .map(|v| {
                v.as_u64()
                    .map(|v| v as u32)
                    .ok_or_else(|| "points are point indices".to_string())
            })
            .collect::<Result<Vec<u32>>>()
            .map(Some),
        Some(Value::Number(n)) => Ok(Some(vec![
            n.as_u64().ok_or("points are point indices")? as u32
        ])),
        Some(_) => Err("points are point indices".into()),
    }
}

/// The mesh: `item`, else the selected item if it is a mesh.
fn mesh_item(scene: &Scene, a: &Args) -> Result<u64> {
    let id = match id_arg(a, "item")? {
        Some(id) => id,
        None => scene
            .selected_items()
            .into_iter()
            .find(|&i| scene.item(i).is_some_and(|i| i.is_mesh()))
            .ok_or("Select a mesh")?,
    };
    match scene.item(id) {
        Some(i) if i.is_mesh() => Ok(id),
        Some(i) => Err(format!("{} isn't a mesh", i.name)),
        None => Err(format!("No item {id}")),
    }
}

/// The map: `map`, else the selected weight map.
fn map_name(e: &Editor, a: &Args) -> Result<String> {
    text(a, "map")
        .map(str::to_owned)
        .or_else(|| e.selected_weight_map.clone())
        .ok_or_else(|| "Give map, or select a weight map".into())
}

/// The points a tool works on: `points`, else the selected points of
/// `item`, else every point.
fn targets(scene: &Scene, item: u64, a: &Args, n: usize) -> Result<Vec<u32>> {
    if let Some(p) = points_arg(a)? {
        if let Some(bad) = p.iter().find(|p| **p as usize >= n) {
            return Err(format!("No point {bad}"));
        }
        return Ok(p);
    }
    let s = &scene.selection;
    if s.item == Some(item) && s.mode == Mode::Vertex && !s.vertices.is_empty() {
        return Ok(s
            .vertices
            .iter()
            .copied()
            .filter(|p| (*p as usize) < n)
            .collect());
    }
    Ok((0..n as u32).collect())
}

/// The maps Normalize works across: the skin's joint maps for a joint
/// map of a bound mesh, else every weight map.
fn family(scene: &Scene, item: u64, mesh: &Mesh, map: &str) -> Vec<String> {
    match skin_maps(scene, item) {
        Some(maps) if maps.iter().any(|m| m == map) => maps,
        _ => mesh.attributes.point_weights.keys().cloned().collect(),
    }
}

/// Lowers the other `family` maps at `p` so the total is at most 1.
fn cap(maps: &mut BTreeMap<String, Weights>, family: &[String], own: &str, p: u32) {
    let mine = maps.get(own).and_then(|m| m.get(&p)).copied().unwrap_or(0.);
    let rest: f64 = family
        .iter()
        .filter(|f| f.as_str() != own)
        .filter_map(|f| maps.get(f).and_then(|m| m.get(&p)))
        .sum();
    if rest > 0. && mine + rest > 1. {
        let scale = (1. - mine).max(0.) / rest;
        for f in family.iter().filter(|f| f.as_str() != own) {
            if let Some(w) = maps.get_mut(f).and_then(|m| m.get_mut(&p)) {
                *w *= scale;
            }
        }
    }
}

fn neighbours(mesh: &Mesh) -> Vec<Vec<u32>> {
    let mut out = vec![Vec::new(); mesh.vertices.len()];
    for (a, b) in mesh.edges() {
        out[a as usize].push(b);
        out[b as usize].push(a);
    }
    out
}

/// Point lookup by position within `tol` (mirror matching).
struct Grid {
    cell: f64,
    cells: HashMap<(i64, i64, i64), Vec<u32>>,
}

impl Grid {
    fn new(points: &[DVec3], cell: f64) -> Grid {
        let cell = cell.max(1e-12);
        let mut cells: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
        for (i, p) in points.iter().enumerate() {
            cells.entry(Self::key(*p, cell)).or_default().push(i as u32);
        }
        Grid { cell, cells }
    }
    fn key(p: DVec3, cell: f64) -> (i64, i64, i64) {
        let k = (p / cell).floor();
        (k.x as i64, k.y as i64, k.z as i64)
    }
    /// The nearest point to `q` within `tol`.
    fn find(&self, points: &[DVec3], q: DVec3, tol: f64) -> Option<u32> {
        let (x, y, z) = Self::key(q, self.cell);
        let mut best: Option<(f64, u32)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    for &i in self
                        .cells
                        .get(&(x + dx, y + dy, z + dz))
                        .into_iter()
                        .flatten()
                    {
                        let d = points[i as usize].distance(q);
                        if d <= tol && best.is_none_or(|b| d < b.0) {
                            best = Some((d, i));
                        }
                    }
                }
            }
        }
        best.map(|b| b.1)
    }
}

/// The other side's name for a map ending in `_L`/`_R` (SK-15 suffixes).
fn other_side(name: &str, plus: &str, minus: &str) -> String {
    if let Some(base) = name.strip_suffix(plus) {
        format!("{base}{minus}")
    } else if let Some(base) = name.strip_suffix(minus) {
        format!("{base}{plus}")
    } else {
        name.to_owned()
    }
}

fn axis(a: &Args) -> Result<usize> {
    Ok(match text(a, "axis").unwrap_or("X") {
        "X" | "x" => 0,
        "Y" | "y" => 1,
        "Z" | "z" => 2,
        other => return Err(format!("axis is X, Y or Z, not {other}")),
    })
}

/// The handler for every `operandi.weight.*` edit.
pub fn run(e: &mut Editor, name: &str, a: &Args) -> Result<Option<Command>> {
    let item = mesh_item(&e.scene, a)?;
    let mesh = e.scene.item(item).expect("checked").mesh.open().clone();
    let n = mesh.vertices.len();
    let mut maps = mesh.attributes.point_weights.clone();
    let label: &str = match name {
        "operandi.weight.set" => {
            // §7: write values per point (erase: no value at all).
            let map = map_name(e, a)?;
            let points = points_arg(a)?.ok_or("Give points")?;
            let erase = flag(a, "erase", false)?;
            let value = if erase {
                None
            } else {
                Some(num(a, "value")?.ok_or("Give value or erase:true")?)
            };
            if value.is_some_and(|v| !(0. ..=1.).contains(&v)) {
                return Err("value is 0 to 1".into());
            }
            let normalize = flag(a, "normalize", false)?;
            let fam = family(&e.scene, item, &mesh, &map);
            let m = maps.entry(map.clone()).or_default();
            for &p in &points {
                if p as usize >= n {
                    return Err(format!("No point {p}"));
                }
                match value {
                    Some(v) => m.insert(p, v),
                    None => m.remove(&p),
                };
            }
            if normalize {
                for &p in &points {
                    cap(&mut maps, &fam, &map, p);
                }
            }
            "Set Weights"
        }
        "operandi.weight.adjust" => {
            // SK-42 Adjust Weights: Additive adds Weight, else sets it.
            let map = map_name(e, a)?;
            let weight = num(a, "weight")?.ok_or("Give weight")?;
            let additive = flag(a, "additive", true)?;
            let normalize = flag(a, "normalize", false)?;
            let points = targets(&e.scene, item, a, n)?;
            let fam = family(&e.scene, item, &mesh, &map);
            let m = maps.entry(map.clone()).or_default();
            for &p in &points {
                let old = m.get(&p).copied().unwrap_or(0.);
                let new = if additive { old + weight } else { weight }.clamp(0., 1.);
                m.insert(p, new);
            }
            if normalize {
                for &p in &points {
                    cap(&mut maps, &fam, &map, p);
                }
            }
            "Adjust Weights"
        }
        "operandi.weight.isolate" => {
            let map = map_name(e, a)?;
            let keep: BTreeSet<u32> = targets(&e.scene, item, a, n)?.into_iter().collect();
            maps.get_mut(&map)
                .ok_or_else(|| format!("No weight map {map}"))?
                .retain(|p, _| keep.contains(p));
            "Isolate Weights"
        }
        "operandi.weight.smooth" => {
            // One blur pass towards the neighbours' mean (missing = 0).
            let map = map_name(e, a)?;
            let amount = num(a, "amount")?.unwrap_or(0.5).clamp(0., 1.);
            let points = targets(&e.scene, item, a, n)?;
            let near = neighbours(&mesh);
            let m = maps
                .get(&map)
                .cloned()
                .ok_or_else(|| format!("No weight map {map}"))?;
            let out = maps.get_mut(&map).expect("checked");
            for &p in &points {
                let ns = &near[p as usize];
                if ns.is_empty() {
                    continue;
                }
                let own = m.get(&p).copied();
                let mean = ns
                    .iter()
                    .map(|q| m.get(q).copied().unwrap_or(0.))
                    .sum::<f64>()
                    / ns.len() as f64;
                let new = own.unwrap_or(0.) * (1. - amount) + mean * amount;
                if own.is_some() || new > 0. {
                    out.insert(p, new);
                }
            }
            "Smooth Weights"
        }
        "operandi.weight.normalize" => {
            // Each point's family total becomes 1 (points with none stay).
            let map = text(a, "map")
                .map(str::to_owned)
                .or_else(|| e.selected_weight_map.clone());
            let fam = match (&map, skin_maps(&e.scene, item)) {
                (_, Some(skin)) => skin,
                (Some(m), None) => family(&e.scene, item, &mesh, m),
                (None, None) => maps.keys().cloned().collect(),
            };
            for p in targets(&e.scene, item, a, n)? {
                let total: f64 = fam
                    .iter()
                    .filter_map(|f| maps.get(f).and_then(|m| m.get(&p)))
                    .sum();
                if total > 0. {
                    for f in &fam {
                        if let Some(w) = maps.get_mut(f).and_then(|m| m.get_mut(&p)) {
                            *w /= total;
                        }
                    }
                }
            }
            "Normalize Weights"
        }
        "operandi.weight.mirror" => {
            // Across X by default: + side to − side (from:minus the other
            // way), matching points by position, _L/_R maps swapped.
            let ax = axis(a)?;
            let tol = num(a, "tolerance")?.unwrap_or(1e-4).max(0.);
            let plus_to_minus = text(a, "from").is_none_or(|f| f != "minus" && f != "-");
            let (sp, sm) = (
                text(a, "suffixPlus").unwrap_or("_L"),
                text(a, "suffixMinus").unwrap_or("_R"),
            );
            let only: Option<String> = text(a, "map").map(str::to_owned);
            let grid = Grid::new(&mesh.vertices, tol.max(1e-6) * 2.);
            let source = |p: &DVec3| {
                if plus_to_minus {
                    p[ax] > tol
                } else {
                    p[ax] < -tol
                }
            };
            let pairs: Vec<(u32, u32)> = mesh
                .vertices
                .iter()
                .enumerate()
                .filter(|(_, p)| source(p))
                .filter_map(|(i, p)| {
                    let mut q = *p;
                    q[ax] = -q[ax];
                    grid.find(&mesh.vertices, q, tol).map(|j| (i as u32, j))
                })
                .collect();
            if pairs.is_empty() {
                return Err("No points match across the mirror".into());
            }
            let names: Vec<String> = maps
                .keys()
                .filter(|k| only.as_ref().is_none_or(|o| o == *k))
                .cloned()
                .collect();
            let before = maps.clone();
            for name in &names {
                let to = other_side(name, sp, sm);
                let from_map = before.get(name).cloned().unwrap_or_default();
                let target = maps.entry(to).or_default();
                for &(i, j) in &pairs {
                    match from_map.get(&i) {
                        Some(w) => target.insert(j, *w),
                        None => target.remove(&j),
                    };
                }
            }
            "Mirror Weights"
        }
        "operandi.weight.merge" => {
            let from = text(a, "from").ok_or("Give from (a map)")?.to_owned();
            let into = text(a, "into")
                .map(str::to_owned)
                .or_else(|| e.selected_weight_map.clone())
                .ok_or("Give into (a map)")?;
            if from == into {
                return Err("Merge needs two different maps".into());
            }
            let src = maps
                .get(&from)
                .cloned()
                .ok_or_else(|| format!("No weight map {from}"))?;
            let dst = maps.entry(into).or_default();
            for (p, w) in src {
                let v = dst.entry(p).or_insert(0.);
                *v = (*v + w).min(1.);
            }
            if flag(a, "remove", true)? {
                maps.remove(&from);
            }
            "Merge Weights"
        }
        "operandi.weight.split" => {
            // One map into _L (+ side) and _R (− side); points on the plane
            // go half to each (provisional, SK-Q10).
            let map = map_name(e, a)?;
            let ax = axis(a)?;
            let (sp, sm) = (
                text(a, "suffixPlus").unwrap_or("_L"),
                text(a, "suffixMinus").unwrap_or("_R"),
            );
            let src = maps
                .get(&map)
                .cloned()
                .ok_or_else(|| format!("No weight map {map}"))?;
            let (mut l, mut r) = (Weights::new(), Weights::new());
            for (p, w) in src {
                let x = mesh.vertices[p as usize][ax];
                if x.abs() < 1e-9 {
                    l.insert(p, w * 0.5);
                    r.insert(p, w * 0.5);
                } else if x > 0. {
                    l.insert(p, w);
                } else {
                    r.insert(p, w);
                }
            }
            maps.insert(format!("{map}{sp}"), l);
            maps.insert(format!("{map}{sm}"), r);
            if flag(a, "remove", true)? {
                maps.remove(&map);
            }
            "Split Weights"
        }
        "operandi.weight.transfer" => {
            // From another mesh: each point takes the nearest source
            // point's weights, in world space (provisional, SK-Q10).
            let from = id_arg(a, "from")?.ok_or("Give from (a mesh item)")?;
            let src_item = e
                .scene
                .item(from)
                .filter(|i| i.is_mesh())
                .ok_or("from must be a mesh item")?;
            let src = src_item.mesh.open().clone();
            let sw = e.scene.world_matrix(from)?;
            let dw = e.scene.world_matrix(item)?;
            let sp: Vec<DVec3> = src
                .vertices
                .iter()
                .map(|v| sw.transform_point3(*v))
                .collect();
            if sp.is_empty() {
                return Err("The source mesh has no points".into());
            }
            let only: Option<String> = text(a, "map").map(str::to_owned);
            let names: Vec<&String> = src
                .attributes
                .point_weights
                .keys()
                .filter(|k| only.as_ref().is_none_or(|o| o == *k))
                .collect();
            for name in &names {
                maps.insert((*name).clone(), Weights::new());
            }
            for p in targets(&e.scene, item, a, n)? {
                let q = dw.transform_point3(mesh.vertices[p as usize]);
                let near = (0..sp.len())
                    .min_by(|x, y| {
                        sp[*x]
                            .distance_squared(q)
                            .total_cmp(&sp[*y].distance_squared(q))
                    })
                    .expect("points") as u32;
                for name in &names {
                    if let Some(w) = src.attributes.point_weights[*name].get(&near) {
                        maps.get_mut(*name).expect("made").insert(p, *w);
                    }
                }
            }
            "Transfer Weights"
        }
        _ => return Err(format!("Unknown weight command {name}")),
    };
    e.anim_step(label, |e| {
        let it = e.scene.item_mut(item).ok_or("No mesh")?;
        let mut m = it.mesh.open().clone();
        if m.attributes.point_weights == maps {
            return Ok(false);
        }
        m.attributes.point_weights = maps;
        it.mesh = m.into();
        Ok(true)
    })?;
    Ok(None)
}

/// `operandi.weight.query`: the mesh's weight maps; with `map`, its
/// values (or one point's with `vertex`); with `vertex` alone, every
/// map's value there and the total.
pub fn answer(e: &Editor, name: &str, a: &Args) -> Option<Result<String>> {
    if name != "operandi.weight.query" {
        return None;
    }
    Some((|| {
        let item = mesh_item(&e.scene, a)?;
        let mesh = e.scene.item(item).expect("checked").mesh.open();
        let maps = &mesh.attributes.point_weights;
        let vertex = id_arg(a, "vertex")?.map(|v| v as u32);
        if let Some(v) = vertex
            && v as usize >= mesh.vertices.len()
        {
            return Err(format!("No point {v}"));
        }
        let mut out = json!({
            "item": item,
            "maps": maps.keys().collect::<Vec<_>>(),
            "skin": skin_maps(&e.scene, item),
            "selected": e.selected_weight_map,
        });
        match (text(a, "map"), vertex) {
            (Some(m), Some(v)) => {
                let w = maps.get(m).ok_or_else(|| format!("No weight map {m}"))?;
                out["value"] = json!(w.get(&v));
            }
            (Some(m), None) => {
                let w = maps.get(m).ok_or_else(|| format!("No weight map {m}"))?;
                out["values"] = json!(
                    w.iter()
                        .map(|(p, w)| (p.to_string(), *w))
                        .collect::<BTreeMap<_, _>>()
                );
            }
            (None, Some(v)) => {
                let at: BTreeMap<&String, f64> = maps
                    .iter()
                    .filter_map(|(k, m)| m.get(&v).map(|w| (k, *w)))
                    .collect();
                out["total"] = json!(at.values().sum::<f64>());
                out["weights"] = json!(at);
            }
            (None, None) => {}
        }
        Ok(out.to_string())
    })())
}
