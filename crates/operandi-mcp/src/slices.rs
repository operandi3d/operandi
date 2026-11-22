//! `tool_run`'s `slice` parameters for Curve Slice, Primitive Slice and
//! Polygon Axis Slice (Dice) (test bridge request BRQ-10). Each call starts
//! the tool's options from their Reset values (BRQ-01), not the last used,
//! and sets only what it is given. Units: metres, radians.
use glam::DVec3;
use operandi_core::command_exec::{as_u32, boolean, num, vec3};
use operandi_core::{
    AxisSlice, AxisSliceMode, CurveSlice, PathCutter, PrimitiveShape, PrimitiveSlice, ShapePlace,
    SliceAxis, SliceOperation, SliceParameters, Tool,
};
use serde_json::{Map, Value};

pub type Result<T> = std::result::Result<T, String>;

/// The cutter keys every path slice takes.
const CUTTER_KEYS: [&str; 7] = [
    "operation",
    "axis",
    "vector",
    "front_face_only",
    "material",
    "select_sliced_edges",
    "edge_set",
];
const CURVE_KEYS: [&str; 6] = [
    "points",
    "closed",
    "steps",
    "auto_steps",
    "smooth_angle",
    "plane_normal",
];
const PRIMITIVE_KEYS: [&str; 17] = [
    "shape",
    "center",
    "normal",
    "half_size",
    "radius",
    "sides",
    "corner_radius",
    "corner_segments",
    "heading",
    "pitch",
    "bank",
    "flip_x",
    "flip_y",
    "clone_x",
    "clone_y",
    "gap",
    "center_clones",
];
const AXIS_KEYS: [&str; 4] = ["mode", "number", "size", "offset"];

/// The keys `slice` takes for `tool`, for the schema and refusals.
pub fn keys(tool: Tool) -> Vec<&'static str> {
    match tool {
        Tool::CurveSlice => CURVE_KEYS.iter().chain(&CUTTER_KEYS).copied().collect(),
        Tool::PrimitiveSlice => PRIMITIVE_KEYS.iter().chain(&CUTTER_KEYS).copied().collect(),
        Tool::AxisSlice => AXIS_KEYS.to_vec(),
        _ => Vec::new(),
    }
}

/// `slice`'s JSON schema (all three tools' keys; each tool refuses the
/// others').
pub fn schema() -> Value {
    let v3 = serde_json::json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3});
    serde_json::json!({"type": "object", "description": "Curve Slice, Primitive Slice and poly.julienne (Polygon Axis Slice, the Edit panel's Dice) options; each call starts from the tool's Reset values. Cutter (Curve and Primitive Slice): operation (core, tunnel, stencil, slice, union, intersect, subtract, add; default slice), axis (x, y, z, custom, action; Curve default y, Primitive default action = the placed face's normal), vector [x, y, z] (custom axis), front_face_only, material (Stencil), select_sliced_edges, edge_set. Curve Slice: points [[x, y, z], ...] (world, the curve passes through them), closed, steps (default 24), auto_steps, smooth_angle (radians), plane_normal [x, y, z] (the Action Axis; default Y). Primitive Slice: shape (rectangle, circle, ellipse), center [x, y, z] (world; required), normal [x, y, z] (the face the shape sits on; default Y), half_size [x, y] (metres, default 0.5), radius (circle), sides (circle/ellipse, default 24), corner_radius, corner_segments, heading/pitch/bank (radians), flip_x, flip_y, clone_x, clone_y, gap [x, y], center_clones. poly.julienne: mode (number or size), number [x, y, z] (cuts per axis), size [x, y, z] (spacing, metres), offset [x, y, z] (metres).",
        "properties": {
            "operation": {"type": "string"}, "axis": {"type": "string"}, "vector": v3,
            "front_face_only": {"type": "boolean"}, "material": {"type": "string"},
            "select_sliced_edges": {"type": "boolean"}, "edge_set": {"type": "string"},
            "points": {"type": "array", "items": v3}, "closed": {"type": "boolean"},
            "steps": {"type": "integer"}, "auto_steps": {"type": "boolean"},
            "smooth_angle": {"type": "number"}, "plane_normal": v3,
            "shape": {"enum": ["rectangle", "circle", "ellipse"]}, "center": v3, "normal": v3,
            "half_size": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2},
            "radius": {"type": "number"}, "sides": {"type": "integer"},
            "corner_radius": {"type": "number"}, "corner_segments": {"type": "integer"},
            "heading": {"type": "number"}, "pitch": {"type": "number"}, "bank": {"type": "number"},
            "flip_x": {"type": "boolean"}, "flip_y": {"type": "boolean"},
            "clone_x": {"type": "integer"}, "clone_y": {"type": "integer"},
            "gap": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2},
            "center_clones": {"type": "boolean"},
            "mode": {"enum": ["number", "size"]}, "number": {"type": "array", "items": {"type": "integer"}, "minItems": 3, "maxItems": 3},
            "size": v3, "offset": v3
        }, "additionalProperties": false})
}

fn text(v: &Value, key: &str) -> Result<String> {
    v.as_str()
        .map(str::to_ascii_lowercase)
        .ok_or(format!("slice {key} is a name"))
}

fn pair(v: &Value, key: &str) -> Result<(f64, f64)> {
    match v.as_array().map(Vec::as_slice) {
        Some([x, y]) => Ok((num(x)?, num(y)?)),
        _ => Err(format!("slice {key} is [x, y]")),
    }
}

/// Sets the slice options `tool` reads from `given`, over Reset values.
pub fn apply(tool: Tool, p: &mut SliceParameters, given: &Value) -> Result<()> {
    let given = given.as_object().ok_or("slice is an object")?;
    let takes = keys(tool);
    if takes.is_empty() {
        return Err(format!("{} takes no slice options", tool.label()));
    }
    if let Some(k) = given.keys().find(|k| !takes.contains(&k.as_str())) {
        return Err(format!(
            "{} has no slice option \"{k}\"; it takes {}",
            tool.label(),
            takes.join(", ")
        ));
    }
    match tool {
        Tool::CurveSlice => {
            let mut o = CurveSlice::default();
            cutter(&mut o.cutter, given)?;
            for (key, v) in given {
                match key.as_str() {
                    "points" => {
                        o.points = v
                            .as_array()
                            .ok_or("slice points is a list of [x, y, z]")?
                            .iter()
                            .map(vec3)
                            .collect::<Result<_>>()?
                    }
                    "closed" => o.closed = boolean(v)?,
                    "steps" => o.steps = as_u32(v)?.max(1),
                    "auto_steps" => o.auto_steps = boolean(v)?,
                    "smooth_angle" => o.smooth_angle = num(v)?.to_degrees(),
                    "plane_normal" => o.plane_normal = vec3(v)?,
                    _ => {}
                }
            }
            if o.points.len() < 2 {
                return Err("Curve Slice needs at least two points".into());
            }
            *p.curve_slice = o;
        }
        Tool::PrimitiveSlice => {
            let mut o = PrimitiveSlice::default();
            cutter(&mut o.cutter, given)?;
            let mut center = None;
            let mut normal = DVec3::Y;
            for (key, v) in given {
                match key.as_str() {
                    "shape" => {
                        o.shape = match text(v, key)?.as_str() {
                            "rectangle" => PrimitiveShape::Rectangle,
                            "circle" => PrimitiveShape::Circle,
                            "ellipse" => PrimitiveShape::Ellipse,
                            _ => return Err("slice shape is rectangle, circle or ellipse".into()),
                        }
                    }
                    "center" => center = Some(vec3(v)?),
                    "normal" => normal = vec3(v)?,
                    "half_size" => (o.half_x, o.half_y) = pair(v, key)?,
                    "radius" => o.radius = num(v)?,
                    "sides" => o.sides = as_u32(v)?,
                    "corner_radius" => o.corner_radius = num(v)?,
                    "corner_segments" => o.corner_segments = as_u32(v)?,
                    // Radians here; the tool keeps degrees.
                    "heading" => o.heading = num(v)?.to_degrees(),
                    "pitch" => o.pitch = num(v)?.to_degrees(),
                    "bank" => o.bank = num(v)?.to_degrees(),
                    "flip_x" => o.flip_x = boolean(v)?,
                    "flip_y" => o.flip_y = boolean(v)?,
                    "clone_x" => o.clone_x = as_u32(v)?.max(1),
                    "clone_y" => o.clone_y = as_u32(v)?.max(1),
                    "gap" => (o.gap_x, o.gap_y) = pair(v, key)?,
                    "center_clones" => o.center_clones = boolean(v)?,
                    _ => {}
                }
            }
            let center = center.ok_or("Primitive Slice needs a center")?;
            o.place = Some(ShapePlace::on(center, normal));
            // The bridge cuts at once: no Lazy Apply drag is held.
            o.held = false;
            *p.primitive_slice = o;
        }
        Tool::AxisSlice => {
            let mut o = AxisSlice::default();
            for (key, v) in given {
                match key.as_str() {
                    "mode" => {
                        o.mode = match text(v, key)?.as_str() {
                            "number" => AxisSliceMode::Number,
                            "size" => AxisSliceMode::Size,
                            _ => return Err("slice mode is number or size".into()),
                        }
                    }
                    "number" => {
                        let n = v
                            .as_array()
                            .filter(|a| a.len() == 3)
                            .ok_or("slice number is [x, y, z]")?;
                        o.number = [as_u32(&n[0])?, as_u32(&n[1])?, as_u32(&n[2])?];
                    }
                    "size" => o.size = vec3(v)?,
                    "offset" => o.offset = vec3(v)?,
                    _ => {}
                }
            }
            *p.axis_slice = o;
        }
        _ => unreachable!("keys() is empty for other tools"),
    }
    Ok(())
}

/// The shared cutter options.
fn cutter(c: &mut PathCutter, given: &Map<String, Value>) -> Result<()> {
    for (key, v) in given {
        match key.as_str() {
            "operation" => {
                let name = text(v, key)?;
                c.operation = SliceOperation::ALL
                    .into_iter()
                    .find(|o| o.label().eq_ignore_ascii_case(&name))
                    .ok_or("slice operation is core, tunnel, stencil, slice, union, intersect, subtract or add")?;
            }
            "axis" => {
                c.axis = match text(v, key)?.as_str() {
                    "x" => SliceAxis::X,
                    "y" => SliceAxis::Y,
                    "z" => SliceAxis::Z,
                    "custom" => SliceAxis::Custom,
                    "action" | "actionaxis" | "action axis" => SliceAxis::ActionAxis,
                    _ => return Err("slice axis is x, y, z, custom or action".into()),
                }
            }
            "vector" => c.vector = vec3(v)?,
            "front_face_only" => c.front_face_only = boolean(v)?,
            "material" => c.material = v.as_str().ok_or("slice material is a name")?.into(),
            "select_sliced_edges" => c.select_sliced_edges = boolean(v)?,
            "edge_set" => c.edge_set = v.as_str().ok_or("slice edge_set is a name")?.into(),
            _ => {}
        }
    }
    Ok(())
}
