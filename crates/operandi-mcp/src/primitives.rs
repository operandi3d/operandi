//! `primitive_add`: the numeric primitives (Cone, Ellipsoid, Torus,
//! Capsule, Arc, Tube) through the same editor sessions as their tool
//! panels. Settings not given take the tool's Reset values, so a call does
//! the same thing whatever the app used last.
use glam::DVec3;
use operandi_core::{
    ArcParameters, CapsuleEnds, CapsuleParameters, Command, ConeCap, ConeParameters, Editor,
    EllipsoidParameters, ShapeKind, ShapeParameters, Tool, TorusParameters, TubeParameters,
};
use serde_json::{Map, Value, json};

type Result<T> = std::result::Result<T, String>;

pub const SHAPES: [&str; 9] = [
    "cube",
    "sphere",
    "cylinder",
    "cone",
    "ellipsoid",
    "torus",
    "capsule",
    "arc",
    "tube",
];

/// What each shape takes, for the description and `primitive_help`.
pub const SHAPE_PARAMS: [(&str, &str); 9] = [
    (
        "cube",
        "position [x,y,z] centre m; size [x,y,z] FULL edge lengths m (default 1,1,1); segments [x,y,z] or one integer (default 1); make_uvs",
    ),
    (
        "sphere",
        "position; radius [x,y,z] HALF extents m, or one number (default 0.5); sides (around Y, default 24); segments (latitude bands, default 12); make_uvs",
    ),
    (
        "cylinder",
        "position; radius [x,y,z] HALF extents m (Y = half the height), or one number (default 0.5, 0.75, 0.5); sides (around Y, default 24); segments (rows along the height, default 1); make_uvs",
    ),
    (
        "cone",
        "position (centre of the bounding box); size [x,y,z] HALF extents m (default 0.5,0.75,0.5; radius is accepted as the same thing); axis; sides; segments; caps / bottom_cap, top_cap (\"none\", \"single\", \"quad_grid\", \"radial\"); make_uvs",
    ),
    (
        "ellipsoid",
        "position; radius [x,y,z] HALF extents m, or one number; sides; segments; bulge_top, bulge_side (0-1); axis; make_uvs",
    ),
    (
        "torus",
        "position; axis; radius [x,y,z] (linked to cross_section/height keeping hole_size); ring_radius m (axis to the middle of the tube); cross_section m (HALF the tube's width across the ring, so the outer radius is ring_radius + cross_section); height m (HALF the tube's thickness along the axis: the torus is 2 x height tall); hole_size (= ring_radius / cross_section); sides; segments; bulge_top, bulge_side; make_uvs. Applied in that order",
    ),
    (
        "capsule",
        "position; radius [x,y,z] HALF extents m, or one number; sides; segments; end_segments; end_size (0-1); ends (\"polar\", \"quad_grid\"); bulge (0-1); axis; make_uvs",
    ),
    (
        "arc",
        "center, start, end [x,y,z] points m; radius m; angle radians (0 to pi); segments; reverse. Applied in that order",
    ),
    (
        "tube",
        "points [[x,y,z], ...] (required; at least 2, 3 when closed); radius m; sides; segments; caps (bool); closed; by_length; twist radians; make_uvs",
    ),
];

/// The `tools/list` entry.
pub fn definition() -> Value {
    let vec3 = json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3});
    let axis = json!({"description": "0, 1, 2 or \"x\", \"y\", \"z\" (default y)", "type": ["integer", "string"]});
    let per_shape: Vec<String> = SHAPE_PARAMS
        .iter()
        .map(|(shape, params)| format!("{shape}: {params}"))
        .collect();
    let mut shape_names: Vec<String> = SHAPES.iter().map(|s| s.to_string()).collect();
    shape_names.extend(SHAPES.iter().map(|s| {
        let mut c = s.chars();
        c.next()
            .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
            .unwrap_or_default()
    }));
    json!({
        "name": "primitive_add",
        "description": format!("Add a primitive to the active mesh (one undo step). position is the centre in world space (default the world origin; the active mesh's transform is allowed for). Settings not given take the tool's Reset values, so a call does the same thing whatever the app used last. Units: metres, radians. shape is required (any case). Per shape — {}. A setting the shape doesn't take is an error.", per_shape.join(" | ")),
        "inputSchema": {"type": "object", "properties": {
            "shape": {"enum": shape_names, "description": "Which primitive (case doesn't matter)."},
            "params": {"type": "object", "properties": {
                "position": described(&vec3, "Centre in WORLD space, metres (default the world origin). The active mesh's own transform is allowed for, so the primitive lands at this world point; the work plane and action centre are not used. cone: centre of its bounding box. Not taken by arc (center/start/end) or tube (points), which are world-space points themselves."),
                "size": described(&vec3, "cube: FULL edge lengths; cone: HALF extents. Metres."),
                "radius": {"description": "HALF extents [x, y, z] (or one number) for sphere, cylinder, cone, ellipsoid, torus, capsule; one number for arc and tube. Metres.", "type": ["array", "number"]},
                "axis": axis,
                "sides": {"type": "integer", "description": "Divisions around the axis."},
                "segments": {"description": "cube: [x, y, z] or one integer; others: divisions along the axis (sphere: latitude bands).", "type": ["array", "integer"]},
                "caps": {"description": "cone: \"none\", \"single\", \"quad_grid\" or \"radial\" (bottom); tube: boolean"},
                "bottom_cap": {"enum": ["none", "single", "quad_grid", "radial"]},
                "top_cap": {"enum": ["none", "single", "quad_grid", "radial"]},
                "make_uvs": {"type": "boolean"},
                "bulge_top": {"type": "number"}, "bulge_side": {"type": "number"},
                "ring_radius": {"type": "number", "description": "torus, metres: from the axis to the middle of the tube"},
                "cross_section": {"type": "number", "description": "torus, metres: HALF the tube's width across the ring"},
                "height": {"type": "number", "description": "torus, metres: HALF the tube's thickness along the axis (the torus is 2 x height tall)"},
                "hole_size": {"type": "number", "description": "torus: ring_radius / cross_section"},
                "end_segments": {"type": "integer"}, "end_size": {"type": "number"},
                "ends": {"enum": ["polar", "quad_grid"]}, "bulge": {"type": "number"},
                "center": vec3, "start": vec3, "end": vec3,
                "angle": {"type": "number", "description": "arc: radians, 0 to pi"},
                "reverse": {"type": "boolean"},
                "points": {"type": "array", "items": vec3},
                "closed": {"type": "boolean"}, "by_length": {"type": "boolean"},
                "twist": {"type": "number", "description": "tube: radians from the first ring to the last"}
            }, "additionalProperties": false}
        }, "required": ["shape"], "additionalProperties": false}
    })
}

fn described(schema: &Value, description: &str) -> Value {
    let mut s = schema.clone();
    s["description"] = json!(description);
    s
}

/// Run one `primitive_add` call on the (staged) editor.
pub fn primitive_add(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    crate::commands::check_keys("primitive_add", args, &["shape", "params"])?;
    let shape = args
        .get("shape")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("shape is required: one of {}", SHAPES.join(", ")))?
        .to_ascii_lowercase();
    let shape = shape.as_str();
    let empty = Map::new();
    let p = match args.get("params") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(m)) => m,
        _ => return Err("params must be an object".into()),
    };
    let mut fields = Fields {
        m: p,
        used: Vec::new(),
    };
    let (arm, configure) = match shape {
        "cube" | "sphere" | "cylinder" => {
            return interactive_primitive(e, shape, &mut fields, p);
        }
        "cone" => {
            let mut c = ConeParameters::default();
            fields.vec3("position", &mut c.position)?;
            fields.vec3("size", &mut c.size)?;
            // Same meaning as size (half extents), as for the sphere.
            fields.radius("radius", &mut c.size)?;
            fields.axis(&mut c.axis)?;
            fields.count("sides", &mut c.sides)?;
            fields.count("segments", &mut c.segments)?;
            let cap = |v: &serde_json::Value| -> Result<ConeCap> {
                Ok(match v.as_str() {
                    Some("none") => ConeCap::None,
                    Some("single") => ConeCap::Single,
                    Some("quad_grid") => ConeCap::QuadGrid,
                    Some("radial") => ConeCap::Radial,
                    _ => {
                        return Err(
                            "Cone caps are \"none\", \"single\", \"quad_grid\" or \"radial\""
                                .into(),
                        );
                    }
                })
            };
            // `caps` sets the bottom cap, as before; `top_cap` the top.
            if let Some(v) = fields.take("caps") {
                c.bottom_cap = cap(v)?;
            }
            if let Some(v) = fields.take("bottom_cap") {
                c.bottom_cap = cap(v)?;
            }
            if let Some(v) = fields.take("top_cap") {
                c.top_cap = cap(v)?;
            }
            fields.flag("make_uvs", &mut c.make_uvs)?;
            (Command::ArmCone, Command::ConfigureCone(c))
        }
        "ellipsoid" => {
            let mut s = EllipsoidParameters::default();
            fields.vec3("position", &mut s.position)?;
            fields.radius("radius", &mut s.radius)?;
            fields.count("sides", &mut s.sides)?;
            fields.count("segments", &mut s.segments)?;
            fields.number("bulge_top", &mut s.bulge_top)?;
            fields.number("bulge_side", &mut s.bulge_side)?;
            fields.axis(&mut s.axis)?;
            fields.flag("make_uvs", &mut s.make_uvs)?;
            (Command::ArmEllipsoid, Command::ConfigureEllipsoid(s))
        }
        "torus" => {
            let mut t = TorusParameters::default();
            fields.vec3("position", &mut t.position)?;
            fields.axis(&mut t.axis)?;
            let mut radius = None;
            if let Some(v) = fields.take("radius") {
                radius = Some(vec3_or_number(v)?);
            }
            if let Some(r) = radius {
                for a in 0..3 {
                    t.set_radius(a, r[a]);
                }
            }
            fields.number("cross_section", &mut t.cross_section)?;
            fields.number("height", &mut t.height)?;
            fields.number("ring_radius", &mut t.ring_radius)?;
            let mut hole = None;
            fields.number_opt("hole_size", &mut hole)?;
            if let Some(h) = hole {
                t.set_hole(h);
            }
            fields.count("sides", &mut t.sides)?;
            fields.count("segments", &mut t.segments)?;
            fields.number("bulge_top", &mut t.bulge_top)?;
            fields.number("bulge_side", &mut t.bulge_side)?;
            fields.flag("make_uvs", &mut t.make_uvs)?;
            shape_commands(ShapeParameters::Torus(t))
        }
        "capsule" => {
            let mut c = CapsuleParameters::default();
            fields.vec3("position", &mut c.position)?;
            fields.radius("radius", &mut c.radius)?;
            fields.count("sides", &mut c.sides)?;
            fields.count("segments", &mut c.segments)?;
            fields.count("end_segments", &mut c.end_segments)?;
            fields.number("end_size", &mut c.end_size)?;
            if let Some(v) = fields.take("ends") {
                c.ends = match v.as_str() {
                    Some("polar") => CapsuleEnds::Polar,
                    Some("quad_grid") => CapsuleEnds::QuadGrid,
                    _ => return Err("Capsule ends is \"polar\" or \"quad_grid\"".into()),
                };
            }
            fields.number("bulge", &mut c.bulge)?;
            fields.axis(&mut c.axis)?;
            fields.flag("make_uvs", &mut c.make_uvs)?;
            shape_commands(ShapeParameters::Capsule(c))
        }
        "arc" => {
            let mut a = ArcParameters::default();
            let mut point = None;
            fields.vec3_opt("center", &mut point)?;
            if let Some(c) = point.take() {
                a.set_center(c);
            }
            fields.number("radius", &mut a.radius)?;
            fields.vec3_opt("start", &mut point)?;
            if let Some(s) = point.take() {
                a.set_start(s);
            }
            fields.vec3_opt("end", &mut point)?;
            if let Some(end) = point.take() {
                a.set_end(end);
            }
            let mut angle = None;
            fields.number_opt("angle", &mut angle)?;
            if let Some(angle) = angle {
                // Radians here; the tool keeps degrees.
                a.set_angle(angle.to_degrees());
            }
            fields.count("segments", &mut a.segments)?;
            fields.flag("reverse", &mut a.reverse)?;
            shape_commands(ShapeParameters::Arc(a))
        }
        "tube" => {
            let mut t = TubeParameters::default();
            let points = fields
                .take("points")
                .ok_or("Tube needs points: [[x, y, z], ...]")?;
            t.points = points
                .as_array()
                .ok_or("points is a list of [x, y, z]")?
                .iter()
                .map(vec3)
                .collect::<Result<_>>()?;
            fields.count("sides", &mut t.sides)?;
            fields.count("segments", &mut t.segments)?;
            fields.number("radius", &mut t.radius)?;
            fields.flag("caps", &mut t.caps)?;
            fields.flag("closed", &mut t.closed)?;
            fields.flag("by_length", &mut t.by_length)?;
            let mut twist = None;
            fields.number_opt("twist", &mut twist)?;
            if let Some(twist) = twist {
                // Radians here; the tool keeps degrees.
                t.twist = twist.to_degrees();
            }
            fields.flag("make_uvs", &mut t.make_uvs)?;
            shape_commands(ShapeParameters::Tube(t))
        }
        other => {
            return Err(format!(
                "Unknown shape {other}; use one of {}",
                SHAPES.join(", ")
            ));
        }
    };
    if let Some(unused) = p.keys().find(|k| !fields.used.contains(&k.as_str())) {
        return Err(format!(
            "{shape} has no setting {unused}; it takes {}",
            SHAPE_PARAMS
                .iter()
                .find(|(s, _)| *s == shape)
                .map_or("", |(_, p)| *p)
        ));
    }
    e.dispatch(arm)?;
    e.dispatch(configure)?;
    // Apply appends the preview as one undo step; Drop then closes the tool.
    e.dispatch(Command::Apply)?;
    e.dispatch(Command::Drop)
}

/// Cube, Sphere and Cylinder: their interactive tools, from the tool's
/// starting values (not the segment counts kept from earlier uses).
fn interactive_primitive(
    e: &mut Editor,
    shape: &str,
    fields: &mut Fields<'_>,
    given: &Map<String, Value>,
) -> Result<()> {
    let tool = match shape {
        "cube" => Tool::Box,
        "sphere" => Tool::Sphere,
        _ => Tool::Cylinder,
    };
    // The tool's Reset values, from an editor with no remembered settings.
    let mut fresh = crate::Bridge::new().editor;
    fresh.dispatch(Command::Activate(tool))?;
    let reset = fresh
        .session
        .live
        .as_ref()
        .map(|l| l.params.clone())
        .ok_or("The tool did not start a live session")?;
    // This session's own parameters, with the numeric settings put back to
    // the Reset values.
    e.dispatch(Command::Activate(tool))?;
    let mut p = e
        .session
        .live
        .as_ref()
        .map(|l| l.params.clone())
        .ok_or("The tool did not start a live session")?;
    p.position = reset.position;
    p.size = reset.size;
    p.segments = reset.segments;
    p.radial = reset.radial;
    p.make_uvs = reset.make_uvs;
    p.cylinder_caps = reset.cylinder_caps;
    fields.vec3("position", &mut p.position)?;
    fields.flag("make_uvs", &mut p.make_uvs)?;
    if tool == Tool::Box {
        fields.vec3("size", &mut p.size)?;
        if let Some(v) = fields.take("segments") {
            p.segments = match v {
                Value::Array(a) if a.len() == 3 => [
                    whole("segments", &a[0])?,
                    whole("segments", &a[1])?,
                    whole("segments", &a[2])?,
                ],
                n => [whole("segments", n)?; 3],
            };
        }
    } else {
        let r = p.radial.as_mut().ok_or("The tool has no radius")?;
        fields.radius("radius", &mut r.radius)?;
        fields.count("sides", &mut r.sides)?;
        fields.count("segments", &mut r.segments)?;
    }
    if let Some(unused) = given.keys().find(|k| !fields.used.contains(&k.as_str())) {
        return Err(format!(
            "{shape} has no setting {unused}; it takes {}",
            SHAPE_PARAMS
                .iter()
                .find(|(s, _)| *s == shape)
                .map_or("", |(_, p)| *p)
        ));
    }
    e.dispatch(Command::Preview(p))?;
    e.dispatch(Command::Apply)?;
    e.dispatch(Command::Drop)
}

fn whole(key: &str, v: &Value) -> Result<u32> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| format!("{key} must be a whole number"))
}

fn shape_commands(p: ShapeParameters) -> (Command, Command) {
    let kind: ShapeKind = p.kind();
    (Command::ArmShape(kind), Command::ConfigureShape(p))
}

/// Reads the settings, remembering which keys were used so unknown ones
/// are refused.
struct Fields<'a> {
    m: &'a Map<String, Value>,
    used: Vec<&'a str>,
}
impl<'a> Fields<'a> {
    fn take(&mut self, key: &'a str) -> Option<&'a Value> {
        let v = self.m.get(key).filter(|v| !v.is_null())?;
        self.used.push(key);
        Some(v)
    }
    fn number(&mut self, key: &'a str, out: &mut f64) -> Result<()> {
        if let Some(v) = self.take(key) {
            *out = num(v)?;
        }
        Ok(())
    }
    fn number_opt(&mut self, key: &'a str, out: &mut Option<f64>) -> Result<()> {
        if let Some(v) = self.take(key) {
            *out = Some(num(v)?);
        }
        Ok(())
    }
    fn count(&mut self, key: &'a str, out: &mut u32) -> Result<()> {
        if let Some(v) = self.take(key) {
            *out = v
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| format!("{key} must be a whole number"))?;
        }
        Ok(())
    }
    fn flag(&mut self, key: &'a str, out: &mut bool) -> Result<()> {
        if let Some(v) = self.take(key) {
            *out = v
                .as_bool()
                .ok_or_else(|| format!("{key} must be true or false"))?;
        }
        Ok(())
    }
    fn vec3(&mut self, key: &'a str, out: &mut DVec3) -> Result<()> {
        if let Some(v) = self.take(key) {
            *out = vec3(v)?;
        }
        Ok(())
    }
    /// Half extents: [x, y, z], or one number for all three.
    fn radius(&mut self, key: &'a str, out: &mut DVec3) -> Result<()> {
        if let Some(v) = self.take(key) {
            *out = vec3_or_number(v)?;
        }
        Ok(())
    }
    fn vec3_opt(&mut self, key: &'a str, out: &mut Option<DVec3>) -> Result<()> {
        if let Some(v) = self.take(key) {
            *out = Some(vec3(v)?);
        }
        Ok(())
    }
    fn axis(&mut self, out: &mut usize) -> Result<()> {
        if let Some(v) = self.take("axis") {
            *out = match (
                v.as_u64(),
                v.as_str().map(str::to_ascii_lowercase).as_deref(),
            ) {
                (Some(n), _) if n < 3 => n as usize,
                (_, Some("x")) => 0,
                (_, Some("y")) => 1,
                (_, Some("z")) => 2,
                _ => return Err("axis is 0, 1, 2 or \"x\", \"y\", \"z\"".into()),
            };
        }
        Ok(())
    }
}
fn num(v: &Value) -> Result<f64> {
    v.as_f64()
        .ok_or_else(|| format!("Expected a number, got {v}"))
}
fn vec3_or_number(v: &Value) -> Result<DVec3> {
    match v.as_f64() {
        Some(n) => Ok(DVec3::splat(n)),
        None => vec3(v).map_err(|_| "Expected [x, y, z] or a number".to_string()),
    }
}
fn vec3(v: &Value) -> Result<DVec3> {
    match v.as_array().map(|a| a.as_slice()) {
        Some([x, y, z]) => Ok(DVec3::new(num(x)?, num(y)?, num(z)?)),
        _ => Err("Expected [x, y, z]".into()),
    }
}
