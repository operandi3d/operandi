//! Headless `command_run` entries added for the behaviour cases (Research's
//! bridge request, 2026-11-20): Set Position, Unify Polygons, Poly Haul and
//! Chamfer Edit run on the current selection of the active mesh as one
//! undo step, in metres, radians and factors (1 = 100 %). A refusal is the
//! message the user would see; nothing changes.
use crate::command_exec::{opt_bool, req_num, req_str};
use crate::{Command, Editor, Result};
use serde_json::{Map, Value};

/// The command for `name`, or `None` when it isn't one of these.
pub(crate) fn script_command(
    _e: &mut Editor,
    name: &str,
    a: &Map<String, Value>,
) -> Result<Option<Command>> {
    Ok(Some(match name {
        "vert.set" => {
            use crate::set_position::{PositionAxis, PositionSpace};
            let axis = match req_str(a, "axis")?.to_ascii_lowercase().as_str() {
                "x" => PositionAxis::X,
                "y" => PositionAxis::Y,
                "z" => PositionAxis::Z,
                other => return Err(format!("axis is x, y or z, not {other}")),
            };
            let work_plane = opt_bool(a, "workPlane")?.unwrap_or(false);
            let world = opt_bool(a, "worldSpace")?.unwrap_or(false);
            let space = match (work_plane, world) {
                (true, _) => PositionSpace::WorkPlane,
                (false, true) => PositionSpace::World,
                _ => PositionSpace::Local,
            };
            Command::SetPosition(crate::PositionAssignment {
                axis,
                value: req_num(a, "value")?,
                space,
                expected: None,
            })
        }
        "poly.unify" => Command::UnifyPolygons(crate::UnifyRequest {
            force: opt_bool(a, "force")?.unwrap_or(false),
            expected: None,
        }),
        "poly.haul" => Command::PolyHaulOnce(req_num(a, "offset")?),
        "chamfer.edit" => {
            let reset = crate::ChamferParameters::default();
            let count = |k: &str, d: u32| -> Result<u32> {
                match a.get(k) {
                    None | Some(Value::Null) => Ok(d),
                    Some(v) => v
                        .as_u64()
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or_else(|| format!("{k} must be a whole number")),
                }
            };
            let factor = |k: &str| -> Result<Option<f64>> {
                a.get(k)
                    .filter(|v| !v.is_null())
                    .map(crate::command_exec::num)
                    .transpose()
            };
            // On/off options as booleans or 0/1, as recorded.
            let flag = |k: &str| -> Result<bool> {
                match a.get(k) {
                    None | Some(Value::Null) => Ok(false),
                    Some(Value::Bool(b)) => Ok(*b),
                    Some(v) => v
                        .as_f64()
                        .map(|n| n != 0.)
                        .ok_or_else(|| format!("{k} is true or false")),
                }
            };
            let width_mode = match a.get("mode").and_then(Value::as_str) {
                None | Some("inset") => false,
                Some("width") => true,
                Some(other) => return Err(format!("mode is inset or width, not {other}")),
            };
            Command::ChamferOnce(crate::ChamferParameters {
                width_mode,
                absolute: flag("absSize")?,
                swap_side: flag("swapSide")?,
                reverse_x: flag("reverseX")?,
                reverse_y: flag("reverseY")?,
                offset: factor("offset")?.unwrap_or(reset.offset),
                segments: count("segments", reset.segments)?,
                source_segments: count("sourceSegs", reset.source_segments)?,
                // A factor here; the tool keeps percent.
                depth: factor("depth")?.map_or(reset.depth, |d| d * 100.),
            })
        }
        _ => return Ok(None),
    }))
}
