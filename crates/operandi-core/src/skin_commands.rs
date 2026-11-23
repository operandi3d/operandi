//! The skin piece's commands (BEHAVIOR-RIG-PHASE5 §4 and §7 MCP): set
//! its channels and influences, Bake Weights (SK-34), and the query that
//! reads its joints, a point's weights and its deformed position at a time.
//! Each edit is one undo step (UIC-224).
use crate::mesh_stack::{MeshOp, OpKind};
use crate::skin::{self, SKIN_OP, SkinValues};
use crate::{Command, Editor, Result, Scene};
use serde_json::{Map, Value, json};

type Args = Map<String, Value>;

const INTERPOLATIONS: [&str; 3] = ["local", "linear", "dualQuaternion"];

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

/// The mesh item a skin command works on: `item`, else the selection's
/// first mesh with a skin.
pub fn skinned_item(scene: &Scene, a: &Args) -> Result<u64> {
    if let Some(id) = id_arg(a, "item")? {
        skin_op(scene, id)?;
        return Ok(id);
    }
    scene
        .selected_items()
        .into_iter()
        .find(|&id| skin_op(scene, id).is_ok())
        .ok_or_else(|| "Select a bound mesh".into())
}

/// The skin op of mesh `item` and its values.
pub fn skin_op(scene: &Scene, item: u64) -> Result<(u64, SkinValues)> {
    let it = scene.item(item).ok_or_else(|| format!("No item {item}"))?;
    let op = it
        .mesh_stack
        .as_ref()
        .and_then(|s| s.ops.iter().find(|o| o.kind.type_id() == SKIN_OP))
        .ok_or_else(|| format!("{} has no skin", it.name))?;
    let v: SkinValues =
        serde_json::from_value(op.kind.values()).map_err(|e| format!("The skin's values: {e}"))?;
    Ok((op.id, v))
}

/// Writes `values` into skin op `op` of `item`.
pub fn write_skin(scene: &mut Scene, item: u64, op: u64, values: &SkinValues) -> Result<()> {
    let stack = scene
        .item_mut(item)
        .and_then(|i| i.mesh_stack.as_mut())
        .ok_or("No skin")?;
    let row: &mut MeshOp = stack.ops.iter_mut().find(|o| o.id == op).ok_or("No skin")?;
    row.kind = OpKind::Op {
        type_id: SKIN_OP.into(),
        values: serde_json::to_value(values).map_err(|e| e.to_string())?,
    };
    Ok(())
}

/// The handler for `operandi.skin.set` and `operandi.skin.bakeWeights`.
pub fn run(e: &mut Editor, name: &str, a: &Args) -> Result<Option<Command>> {
    let item = skinned_item(&e.scene, a)?;
    let (op, mut v) = skin_op(&e.scene, item)?;
    match name {
        "operandi.skin.set" => {
            if let Some(s) = a.get("interpolation").and_then(Value::as_str) {
                v.interpolation = match s.to_ascii_lowercase().as_str() {
                    "local" => skin::LOCAL,
                    "linear" => skin::LINEAR,
                    "dualquaternion" | "dq" => skin::DUAL_QUATERNION,
                    other => {
                        return Err(format!(
                            "interpolation is local, linear or dualQuaternion, not {other}"
                        ));
                    }
                };
            }
            if let Some(b) = flag(a, "limitSources")? {
                v.limit_sources = b;
            }
            if let Some(n) = num(a, "maxSources")? {
                if !(1. ..=8.).contains(&n) || n.fract() != 0. {
                    return Err("maxSources is 1 to 8".into());
                }
                v.max_sources = n as u32;
            }
            if let Some(n) = num(a, "threshold")? {
                if !(0. ..=1.).contains(&n) {
                    return Err("threshold is 0 to 1".into());
                }
                v.threshold = n;
            }
            if let Some(n) = num(a, "sharpness")? {
                if !n.is_finite() || n.abs() > 10. {
                    return Err("sharpness is -10 to 10".into());
                }
                v.sharpness = n;
            }
            if let Some(b) = flag(a, "world")? {
                v.world = b;
            }
            // One influence's Enable / Interpolation (SK-35).
            if let Some(j) = id_arg(a, "joint")? {
                let i = v
                    .joints
                    .iter()
                    .position(|x| *x == j)
                    .ok_or_else(|| format!("Joint {j} isn't in this skin"))?;
                if let Some(b) = flag(a, "enable")? {
                    v.enabled[i] = b;
                }
                if let Some(s) = a.get("influence").and_then(Value::as_str) {
                    v.smooth[i] = match s.to_ascii_lowercase().as_str() {
                        "linear" => false,
                        "smooth" => true,
                        other => return Err(format!("influence is linear or smooth, not {other}")),
                    };
                }
            }
            e.anim_step("Skin", |e| {
                write_skin(&mut e.scene, item, op, &v)?;
                Ok(true)
            })?;
            Ok(None)
        }
        "operandi.skin.bakeWeights" => {
            // SK-34: the effective weights into the maps, options neutral.
            e.anim_step("Bake Weights", |e| {
                let it = e.scene.item_mut(item).ok_or("No item")?;
                let mut mesh = it.mesh.open().clone();
                let live = vec![true; v.joints.len()];
                let weights = skin::influences(&mesh, &v, &live);
                for map in &v.maps {
                    mesh.attributes
                        .point_weights
                        .entry(map.clone())
                        .or_default()
                        .clear();
                }
                for (p, slot) in weights.iter().enumerate() {
                    for &(i, w) in slot {
                        mesh.attributes
                            .point_weights
                            .entry(v.maps[i as usize].clone())
                            .or_default()
                            .insert(p as u32, w);
                    }
                }
                it.mesh = mesh.into();
                v.limit_sources = false;
                v.threshold = 0.;
                v.sharpness = 0.;
                v.smooth.iter_mut().for_each(|s| *s = false);
                write_skin(&mut e.scene, item, op, &v)?;
                Ok(true)
            })?;
            Ok(None)
        }
        _ => Err(format!("Unknown skin command {name}")),
    }
}

/// `operandi.skin.query`: the skin's channels and joints; with `vertex`,
/// that point's effective weights and its deformed position (at `time`
/// in seconds, default now).
pub fn answer(e: &Editor, name: &str, a: &Args) -> Option<Result<String>> {
    if name != "operandi.skin.query" {
        return None;
    }
    Some((|| {
        let item = skinned_item(&e.scene, a)?;
        let (op, v) = skin_op(&e.scene, item)?;
        let joints: Vec<Value> = v
            .joints
            .iter()
            .enumerate()
            .map(|(i, j)| {
                json!({
                    "joint": j,
                    "name": e.scene.item(*j).map(|it| it.name.clone()),
                    "map": v.maps[i],
                    "enable": v.enabled[i],
                    "influence": if v.smooth[i] { "smooth" } else { "linear" },
                    "missing": e.scene.item(*j).is_none(),
                })
            })
            .collect();
        let mut out = json!({
            "item": item,
            "op": op,
            "interpolation": INTERPOLATIONS[v.interpolation as usize],
            "limitSources": v.limit_sources,
            "maxSources": v.max_sources,
            "threshold": v.threshold,
            "sharpness": v.sharpness,
            "world": v.world,
            "joints": joints,
        });
        if let Some(p) = id_arg(a, "vertex")? {
            let mesh = e.scene.item(item).expect("checked").mesh.open();
            if p as usize >= mesh.vertices.len() {
                return Err(format!("No point {p}"));
            }
            let live = vec![true; v.joints.len()];
            let w = &skin::influences(mesh, &v, &live)[p as usize];
            let weights: Map<String, Value> = w
                .iter()
                .map(|&(i, w)| (v.maps[i as usize].clone(), json!(crate::rig::clean(w))))
                .collect();
            let time = match num(a, "time")? {
                Some(s) => crate::time_tick::Tick::from_seconds(s),
                None => e.scene.time.current,
            };
            let posed = crate::animation::evaluate_at(&e.scene, time);
            let shown = crate::evaluate::evaluate(
                &posed,
                &e.mesh_stack,
                &e.eval,
                time,
                crate::evaluate::Purpose::Final,
            );
            let at = shown
                .item(item)
                .map(|i| i.mesh.open().vertices[p as usize])
                .unwrap_or(mesh.vertices[p as usize]);
            out["weights"] = Value::Object(weights);
            out["position"] = json!(at.to_array().map(crate::rig::clean));
            out["rest"] = json!(mesh.vertices[p as usize].to_array().map(crate::rig::clean));
        }
        Ok(out.to_string())
    })())
}
