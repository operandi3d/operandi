//! `item_get`, `camera_set` and `light_set`: an item's own properties, as
//! the Properties panel shows them, read and written through the panel's
//! own edits (`Command::EditCamera` / `EditLight`), so ranges and refusals
//! are the panel's. One call is one undo step however many fields it sets.
//!
//! Units are the bridge's everywhere: metres, radians, fractions 0–1,
//! pixels for resolution, kelvins for temperature. The panel shows
//! millimetres, degrees and percent for the same values.
use crate::bridge::Result;
use operandi_core::camera_light::{
    AreaShape, CameraEdit, CameraSettings, FilmFit, LightEdit, LightFalloff, LightKind,
    LightRender, LightSettings, Projection, ShadowType,
};
use operandi_core::{Command, Editor, Scene};
use serde_json::{Map, Value, json};

/// Every camera field `item_get` reports and `camera_set` takes, with its
/// unit.
pub const CAMERA_FIELDS: &[(&str, &str)] = &[
    (
        "projection",
        "Perspective, Orthographic, Cylindrical, Cylindrical VR, Spherical, Spherical VR",
    ),
    ("focal_length", "m (0.05 = 50 mm)"),
    (
        "angle_of_view",
        "rad, horizontal; setting it changes focal_length",
    ),
    ("film_width", "m (0.036 = 36 mm)"),
    ("film_height", "m (0.024 = 24 mm)"),
    ("film_offset", "[x, y] m"),
    ("film_fit", "Fill, Overscan, Horizontal, Vertical"),
    ("target_distance", "m"),
    ("focus_distance", "m"),
    ("clipping", "bool"),
    ("clip_near", "m"),
    ("clip_far", "m"),
    ("dof", "bool, depth of field on (renders only)"),
    ("f_stop", "number >= 1"),
    ("iris_blades", "0 (round) or 3–16"),
    ("iris_rotation", "rad"),
    (
        "resolution",
        "[width, height] px: the camera's Resolution Override, on; false turns it off",
    ),
    ("render_camera", "bool: this camera renders (set true only)"),
];

/// Every light field `item_get` reports and `light_set` takes.
pub const LIGHT_FIELDS: &[(&str, &str)] = &[
    (
        "type",
        "Directional, Point, Spot, Area, Cylinder, Photometric, Mesh, Dome, Portal",
    ),
    (
        "intensity",
        "number as the panel shows it (W/srm²); it measures Radiant Exitance for Directional, Radiant Intensity for Point/Spot/Photometric, Radiance for the others (see intensity_measures); converted when type changes so the light is about as bright 1 m in front of it",
    ),
    ("color", "[r, g, b] linear, 0–1"),
    (
        "temperature",
        "K, 1000–40000: sets color (write only; not kept)",
    ),
    (
        "render",
        "Default, Yes, No (as the panel; On and Off are accepted for Yes and No)",
    ),
    ("shadow", "None, Ray Traced"),
    ("visible_to_camera", "bool"),
    ("dissolve", "0–1"),
    ("spread_angle", "rad (Directional: sun disc)"),
    ("affect_diffuse", "0–1"),
    ("affect_specular", "0–1"),
    (
        "affect_caustics",
        "0–1: share of the caustics the light throws through glass when the Render item's Direct Caustics is on",
    ),
    ("radius", "m (Point, Spot, Cylinder)"),
    (
        "falloff",
        "None, Inverse Distance, Inverse Distance Squared",
    ),
    ("cone_angle", "rad, full cone (Spot)"),
    ("soft_edge", "rad (Spot)"),
    ("shape", "Rectangular, Elliptical (Area, Portal)"),
    ("width", "m (Area, Portal)"),
    ("height", "m (Area, Portal)"),
    ("length", "m (Cylinder)"),
];

fn item<'a>(scene: &'a Scene, args: &Map<String, Value>) -> Result<&'a operandi_core::Item> {
    let id = match args.get("item").filter(|v| !v.is_null()) {
        Some(v) => v.as_u64().ok_or("item must be an integer id")?,
        None => scene.selection.item.ok_or("No active item; pass item")?,
    };
    scene
        .items
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| format!("No item {id}"))
}

fn units(fields: &[(&str, &str)]) -> Value {
    Value::Object(
        fields
            .iter()
            .map(|(k, u)| (k.to_string(), json!(u)))
            .collect(),
    )
}

pub fn camera_json(scene: &Scene, id: u64, c: &CameraSettings) -> Value {
    json!({
        "projection": c.projection.label(),
        "focal_length": c.focal_length,
        "angle_of_view": c.angle_of_view(),
        "film_width": c.film_width,
        "film_height": c.film_height,
        "film_offset": c.film_offset,
        "film_fit": c.film_fit.label(),
        "target_distance": c.target_distance,
        "focus_distance": c.focus_distance,
        "clipping": c.clipping,
        "clip_near": c.clip_near,
        "clip_far": c.clip_far,
        "dof": c.dof.enable,
        "f_stop": c.dof.f_stop,
        "iris_blades": c.dof.iris_blades,
        "iris_rotation": c.dof.iris_rotation,
        "resolution": if c.resolution.enable {
            json!([c.resolution.width, c.resolution.height])
        } else {
            json!(false)
        },
        "render_size": c.render_size(scene.render.frame),
        "render_camera": scene.render_camera() == Some(id),
    })
}

pub fn light_json(l: &LightSettings) -> Value {
    json!({
        "type": l.kind.label(),
        "intensity": l.intensity,
        "intensity_measures": l.kind.intensity_label(),
        "color": l.color,
        "render": l.render.label(),
        "shadow": l.shadow.label(),
        "visible_to_camera": l.visible_to_camera,
        "dissolve": l.dissolve,
        "spread_angle": l.spread_angle,
        "affect_diffuse": l.affect_diffuse,
        "affect_specular": l.affect_specular,
        "affect_caustics": l.stored.affect_caustics,
        "radius": l.radius,
        "falloff": l.falloff.label(),
        "cone_angle": l.cone_angle,
        "soft_edge": l.soft_edge,
        "shape": l.shape.label(),
        "width": l.width,
        "height": l.height,
        "length": l.length,
    })
}

/// `item_get`: one item's kind, transform and own properties.
pub fn item_get(e: &Editor, args: &Map<String, Value>) -> Result<Value> {
    crate::commands::check_keys("item_get", args, &["item"])?;
    let scene = &e.scene;
    let i = item(scene, args)?;
    let kind = if i.camera.is_some() {
        "camera"
    } else if i.light.is_some() {
        "light"
    } else if i.backdrop.is_some() {
        "backdrop"
    } else if i.is_group_locator() {
        "groupLocator"
    } else {
        "mesh"
    };
    let t = &i.transform;
    let mut out = json!({
        "id": i.id, "name": i.name, "kind": kind, "parent": i.parent, "visible": i.visible,
        "transform": {
            "translation": [t.translation.x, t.translation.y, t.translation.z],
            "rotation": [t.rotation.x, t.rotation.y, t.rotation.z],
            "scale": [t.scale.x, t.scale.y, t.scale.z],
        },
        "transform_units": {"translation": "m", "rotation": "rad, Euler XYZ", "scale": "factor"},
    });
    if let Some(c) = &i.camera {
        out["camera"] = camera_json(scene, i.id, c);
        out["camera_units"] = units(CAMERA_FIELDS);
    }
    if let Some(l) = &i.light {
        out["light"] = light_json(l);
        out["light_units"] = units(LIGHT_FIELDS);
    }
    if let Some(b) = &i.backdrop {
        out["backdrop"] = backdrop_json(scene, b);
        out["backdrop_units"] = units(BACKDROP_FIELDS);
    }
    if kind == "mesh" {
        out["mesh"] = json!({
            "vertices": i.mesh.vertices.len(),
            "edges": i.mesh.edges().len(),
            "faces": i.mesh.faces.len(),
        });
        out["subdivision"] = crate::shading::subdivision_json(i);
        // BRQ-94: the Mesh Operations, bottom (first applied) to top.
        if let Some(stack) = &i.mesh_stack {
            let statuses = e
                .mesh_stack
                .current(scene, i)
                .map(|r| r.statuses)
                .unwrap_or_default();
            out["mesh_stack"] = json!(
                stack
                    .ops
                    .iter()
                    .enumerate()
                    .map(|(n, op)| json!({
                        "op": format!("op:{}:{}", i.id, op.id),
                        "id": op.id,
                        "type": op.kind.type_id(),
                        "label": op.kind.label(),
                        "enabled": op.enabled,
                        "values": op.kind.values(),
                        "selection": op.selection.as_ref().map(|s| s.polygons.clone()),
                        "status": statuses.get(n).map(crate::bridge::op_status),
                    }))
                    .collect::<Vec<_>>()
            );
        }
    }
    Ok(out)
}

pub use operandi_core::command_exec::fold_steps;

/// A label from `all` that matches `given`, ignoring case, spaces and `_`.
fn pick<T: Copy>(all: &[T], label: fn(T) -> &'static str, key: &str, given: &Value) -> Result<T> {
    let text = given.as_str().ok_or_else(|| format!("{key} is a name"))?;
    let norm = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    };
    all.iter()
        .copied()
        .find(|v| norm(label(*v)) == norm(text))
        .ok_or_else(|| {
            format!(
                "{key} is one of {}",
                all.iter().map(|v| label(*v)).collect::<Vec<_>>().join(", ")
            )
        })
}

fn number(key: &str, v: &Value) -> Result<f64> {
    v.as_f64().ok_or_else(|| format!("{key} must be a number"))
}
fn flag(key: &str, v: &Value) -> Result<bool> {
    v.as_bool()
        .ok_or_else(|| format!("{key} must be true or false"))
}
fn whole(key: &str, v: &Value) -> Result<u32> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| format!("{key} must be a whole number"))
}
fn pair(key: &str, v: &Value) -> Result<[f64; 2]> {
    match v.as_array().map(Vec::as_slice) {
        Some([a, b]) => Ok([number(key, a)?, number(key, b)?]),
        _ => Err(format!("{key} is [x, y]")),
    }
}
fn rgb(key: &str, v: &Value) -> Result<[f64; 3]> {
    match v.as_array().map(Vec::as_slice) {
        Some([r, g, b]) => Ok([number(key, r)?, number(key, g)?, number(key, b)?]),
        _ => Err(format!("{key} is [r, g, b]")),
    }
}

fn fields_of(args: &Map<String, Value>, known: &[(&str, &str)], tool: &str) -> Result<()> {
    let mut names: Vec<&str> = known.iter().map(|(k, _)| *k).collect();
    names.push("item");
    crate::commands::check_keys(tool, args, &names)
}

/// `camera_set`: camera properties, one undo step.
pub fn camera_set(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    fields_of(args, CAMERA_FIELDS, "camera_set")?;
    let id = item(&e.scene, args)?.id;
    if !e
        .scene
        .items
        .iter()
        .any(|i| i.id == id && i.camera.is_some())
    {
        return Err(format!("Item {id} is not a camera"));
    }
    let start = e.history_mark();
    let mut edits = Vec::new();
    let mut render_camera = false;
    // In the table's order, so focal_length then angle_of_view is defined.
    for (key, _) in CAMERA_FIELDS {
        let Some(v) = args.get(*key).filter(|v| !v.is_null()) else {
            continue;
        };
        let edit = match *key {
            "projection" => {
                CameraEdit::Projection(pick(&Projection::ALL, Projection::label, key, v)?)
            }
            "focal_length" => CameraEdit::FocalLength(number(key, v)?),
            "angle_of_view" => CameraEdit::AngleOfView(number(key, v)?),
            "film_width" => CameraEdit::FilmWidth(number(key, v)?),
            "film_height" => CameraEdit::FilmHeight(number(key, v)?),
            "film_offset" => {
                let [x, y] = pair(key, v)?;
                edits.push(CameraEdit::FilmOffsetX(x));
                CameraEdit::FilmOffsetY(y)
            }
            "film_fit" => CameraEdit::FilmFit(pick(&FilmFit::ALL, FilmFit::label, key, v)?),
            "target_distance" => CameraEdit::TargetDistance(number(key, v)?),
            "focus_distance" => CameraEdit::FocusDistance(number(key, v)?),
            "clipping" => CameraEdit::Clipping(flag(key, v)?),
            "clip_near" => CameraEdit::ClipNear(number(key, v)?),
            "clip_far" => CameraEdit::ClipFar(number(key, v)?),
            "dof" => CameraEdit::DofEnable(flag(key, v)?),
            "f_stop" => CameraEdit::FStop(number(key, v)?),
            "iris_blades" => CameraEdit::IrisBlades(whole(key, v)?),
            "iris_rotation" => CameraEdit::IrisRotation(number(key, v)?),
            "resolution" => match v {
                Value::Bool(false) => CameraEdit::ResolutionEnable(false),
                Value::Array(a) if a.len() == 2 => CameraEdit::ResolutionFromClip {
                    width: whole(key, &a[0])?,
                    height: whole(key, &a[1])?,
                },
                _ => return Err("resolution is [width, height] pixels, or false".into()),
            },
            "render_camera" => {
                if !flag(key, v)? {
                    return Err(
                        "render_camera can only be set true; set it on another camera instead"
                            .into(),
                    );
                }
                render_camera = true;
                continue;
            }
            _ => unreachable!("every camera field is handled"),
        };
        edits.push(edit);
    }
    for edit in edits {
        e.dispatch(Command::EditCamera { item: id, edit })?;
    }
    if render_camera {
        e.dispatch(Command::SetRenderCamera(id))?;
    }
    fold_steps(e, start, "Camera Properties");
    Ok(())
}

/// `light_set`: light properties, one undo step.
pub fn light_set(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    fields_of(args, LIGHT_FIELDS, "light_set")?;
    let id = item(&e.scene, args)?.id;
    if !e
        .scene
        .items
        .iter()
        .any(|i| i.id == id && i.light.is_some())
    {
        return Err(format!("Item {id} is not a light"));
    }
    let start = e.history_mark();
    let mut edits = Vec::new();
    for (key, _) in LIGHT_FIELDS {
        let Some(v) = args.get(*key).filter(|v| !v.is_null()) else {
            continue;
        };
        edits.push(match *key {
            "type" => LightEdit::Kind(pick(&LightKind::ALL, LightKind::label, key, v)?),
            "intensity" => LightEdit::Intensity(number(key, v)?),
            "color" => LightEdit::Color(rgb(key, v)?),
            "temperature" => LightEdit::Temperature(number(key, v)?),
            // The panel says Yes / No; On / Off name the same settings.
            "render" => {
                LightEdit::Render(match v.as_str().map(str::to_ascii_lowercase).as_deref() {
                    Some("on") => LightRender::On,
                    Some("off") => LightRender::Off,
                    _ => pick(&LightRender::ALL, LightRender::label, key, v)?,
                })
            }
            "shadow" => LightEdit::Shadow(pick(&ShadowType::ALL, ShadowType::label, key, v)?),
            "visible_to_camera" => LightEdit::VisibleToCamera(flag(key, v)?),
            "dissolve" => LightEdit::Dissolve(number(key, v)?),
            "spread_angle" => LightEdit::SpreadAngle(number(key, v)?),
            "affect_diffuse" => LightEdit::AffectDiffuse(number(key, v)?),
            "affect_specular" => LightEdit::AffectSpecular(number(key, v)?),
            "affect_caustics" => LightEdit::AffectCaustics(number(key, v)?),
            "radius" => LightEdit::Radius(number(key, v)?),
            "falloff" => LightEdit::Falloff(pick(&LightFalloff::ALL, LightFalloff::label, key, v)?),
            "cone_angle" => LightEdit::ConeAngle(number(key, v)?),
            "soft_edge" => LightEdit::SoftEdge(number(key, v)?),
            "shape" => LightEdit::Shape(pick(&AreaShape::ALL, AreaShape::label, key, v)?),
            "width" => LightEdit::Width(number(key, v)?),
            "height" => LightEdit::Height(number(key, v)?),
            "length" => LightEdit::Length(number(key, v)?),
            _ => unreachable!("every light field is handled"),
        });
    }
    for edit in edits {
        e.dispatch(Command::EditLight { item: id, edit })?;
    }
    fold_steps(e, start, "Light Properties");
    Ok(())
}

/// Input schema properties for a field table.
pub fn schema(fields: &[(&str, &str)]) -> Value {
    let mut props: Map<String, Value> = fields
        .iter()
        .map(|(k, u)| (k.to_string(), json!({"description": u})))
        .collect();
    props.insert(
        "item".into(),
        json!({"type": "integer", "description": "Item id; default the active item."}),
    );
    json!({"type": "object", "properties": props, "additionalProperties": false})
}

/// Every field `render_settings_get` reports and `render_settings_set`
/// takes.
pub const RENDER_FIELDS: &[(&str, &str)] = &[
    (
        "samples",
        "Maximum Quality: samples per pixel, 1 or more (new scenes 256, older scenes 16)",
    ),
    (
        "denoise",
        "bool: denoise the finished picture, the Final Color output's Denoising (default true)",
    ),
    (
        "adaptive_threshold",
        "Noise Threshold as a 0-1 fraction (0.01 = 1 %); 0 = adaptive sampling off",
    ),
    (
        "frame",
        "[width, height] px: the Render item's Frame Width and Height (default [1920, 1080]); a camera's Resolution Override wins",
    ),
    (
        "diffuse_depth",
        "Diffuse Depth, 0-128 bounces (new scenes 4)",
    ),
    (
        "reflection_depth",
        "Reflection Depth, 0-128 bounces (new scenes 8)",
    ),
    (
        "refraction_depth",
        "Refraction Depth, 0-128 bounces (new scenes 8)",
    ),
    (
        "max_radiance",
        "Maximum Radiance: bounced light is clamped to this; 0 = off (default 10)",
    ),
    (
        "caustics",
        "Indirect Caustics: None, Reflection Only, Refraction Only or Both (default)",
    ),
    (
        "direct_caustics",
        "Direct Caustics: lights (by their affect_caustics) throw caustics through glass (default false)",
    ),
    (
        "total_photons",
        "Total Photons, 1-100000000 (default 100000); kept, Operandi's caustics don't count photons",
    ),
    (
        "local_photons",
        "Local Photons, 1-10000 (default 32); kept, Operandi's caustics don't count photons",
    ),
    (
        "walkthrough",
        "Walkthrough Mode (default false); kept, doesn't change the render",
    ),
    (
        "exposure",
        "Final Color output's exposure control: White Level or Photographic",
    ),
    (
        "white_level",
        "Final Color, White Level control: the value shown as white (default 1; 0.5 = one stop brighter)",
    ),
    (
        "iso",
        "Final Color, Photographic control: brightness x ISO/100 (default 100)",
    ),
    ("gamma", "Final Color output gamma (default 1)"),
    (
        "resolution",
        "[width, height] px of the render camera's picture (sets its Resolution Override); false turns the override off",
    ),
    (
        "bloom",
        "bool: Bloom, a soft glow around bright parts of the finished picture (Render window > Image Processing; default false). Changes the picture, not the render",
    ),
    (
        "bloom_threshold",
        "Bloom Threshold: linear brightness where the glow starts, 0-1000 (default 1 = white)",
    ),
    (
        "bloom_intensity",
        "Bloom Intensity as a 0-10 factor (0.5 = 50 %, the default): share of the light over the threshold spread into the glow",
    ),
    (
        "bloom_radius",
        "Bloom Radius as a 0.001-1 fraction of the picture's width (default 0.05)",
    ),
    (
        "bloom_tint",
        "Bloom Tint [r, g, b] linear, 0-1 (default [1, 1, 1])",
    ),
    (
        "glare",
        "bool: Glare, straight streaks out of bright points (a star filter; default false)",
    ),
    (
        "glare_threshold",
        "Glare Threshold: linear brightness where streaks start, 0-1000 (default 2)",
    ),
    (
        "glare_intensity",
        "Glare Intensity as a 0-10 factor (default 0.3 = 30 %)",
    ),
    (
        "glare_streaks",
        "Glare Streaks: rays per bright point, 2, 4, 6 or 8 (default 4)",
    ),
    (
        "glare_angle",
        "Glare Angle in radians, the first ray counter-clockwise from the right (default pi/4)",
    ),
    (
        "glare_length",
        "Glare Length as a 0.005-2 fraction of the picture's width (default 0.2)",
    ),
    (
        "glare_tint",
        "Glare Tint [r, g, b] linear, 0-1 (default [1, 1, 1])",
    ),
];

/// `render_settings_get`: the scene's render settings and picture size.
pub fn render_settings_get(e: &Editor, args: &Map<String, Value>) -> Result<Value> {
    crate::commands::check_keys("render_settings_get", args, &[])?;
    let scene = &e.scene;
    let r = &scene.render;
    let fc = final_color(scene);
    let camera = scene.render_camera();
    let size = camera
        .and_then(|id| scene.items.iter().find(|i| i.id == id))
        .and_then(|i| i.camera.as_ref())
        .map(|c| c.render_size(r.frame))
        .or(Some(r.frame));
    Ok(json!({
        "samples": r.samples,
        "denoise": scene.render_denoise(),
        "adaptive_threshold": r.adaptive_threshold,
        "frame": r.frame,
        "diffuse_depth": r.diffuse_depth,
        "reflection_depth": r.reflection_depth,
        "refraction_depth": r.refraction_depth,
        "max_radiance": r.max_radiance,
        "caustics": r.caustics.label(),
        "direct_caustics": r.direct_caustics.enabled,
        "total_photons": r.direct_caustics.total_photons,
        "local_photons": r.direct_caustics.local_photons,
        "walkthrough": r.direct_caustics.walkthrough,
        "bloom": r.post.bloom.enabled,
        "bloom_threshold": r.post.bloom.threshold,
        "bloom_intensity": r.post.bloom.intensity,
        "bloom_radius": r.post.bloom.radius,
        "bloom_tint": r.post.bloom.tint,
        "glare": r.post.glare.enabled,
        "glare_threshold": r.post.glare.threshold,
        "glare_intensity": r.post.glare.intensity,
        "glare_streaks": r.post.glare.streaks,
        "glare_angle": r.post.glare.angle.to_radians(),
        "glare_length": r.post.glare.length,
        "glare_tint": r.post.glare.tint,
        "exposure": fc.as_ref().map(|o| o.settings.exposure.label()),
        "white_level": fc.as_ref().map(|o| o.settings.white_level),
        "iso": fc.as_ref().map(|o| o.settings.iso),
        "gamma": fc.as_ref().map(|o| o.settings.gamma),
        "final_color_output": fc.as_ref().map(|o| o.id),
        "resolution": size,
        "render_camera": camera,
        "units": units(RENDER_FIELDS),
        "note": "Without a camera, renders use the viewport's size. render's width/height/samples override these for one render.",
    }))
}

/// `render_settings_set`: one "Render Settings" undo step.
pub fn render_settings_set(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    let names: Vec<&str> = RENDER_FIELDS.iter().map(|(k, _)| *k).collect();
    crate::commands::check_keys("render_settings_set", args, &names)?;
    let start = e.history_mark();
    let mut s = e.scene.render.clone();
    let get = |k: &str| args.get(k).filter(|v| !v.is_null());
    if let Some(v) = get("samples") {
        s.samples = whole("samples", v)?;
    }
    if let Some(v) = get("denoise") {
        s.denoise = flag("denoise", v)?;
    }
    if let Some(v) = get("adaptive_threshold") {
        s.adaptive_threshold = number("adaptive_threshold", v)?;
    }
    if let Some(v) = get("frame") {
        let pair = v
            .as_array()
            .filter(|a| a.len() == 2)
            .ok_or("frame must be [width, height]")?;
        s.frame = [whole("frame", &pair[0])?, whole("frame", &pair[1])?];
    }
    if let Some(v) = get("diffuse_depth") {
        s.diffuse_depth = whole("diffuse_depth", v)?;
    }
    if let Some(v) = get("reflection_depth") {
        s.reflection_depth = whole("reflection_depth", v)?;
    }
    if let Some(v) = get("refraction_depth") {
        s.refraction_depth = whole("refraction_depth", v)?;
    }
    if let Some(v) = get("max_radiance") {
        s.max_radiance = number("max_radiance", v)?;
    }
    if let Some(v) = get("caustics") {
        use operandi_core::render_settings::Caustics;
        s.caustics = pick(&Caustics::ALL, Caustics::label, "caustics", v)?;
    }
    if let Some(v) = get("direct_caustics") {
        s.direct_caustics.enabled = flag("direct_caustics", v)?;
    }
    if let Some(v) = get("total_photons") {
        s.direct_caustics.total_photons = whole("total_photons", v)?;
    }
    if let Some(v) = get("local_photons") {
        s.direct_caustics.local_photons = whole("local_photons", v)?;
    }
    if let Some(v) = get("walkthrough") {
        s.direct_caustics.walkthrough = flag("walkthrough", v)?;
    }
    let (b, g) = (&mut s.post.bloom, &mut s.post.glare);
    if let Some(v) = get("bloom") {
        b.enabled = flag("bloom", v)?;
    }
    if let Some(v) = get("bloom_threshold") {
        b.threshold = number("bloom_threshold", v)?;
    }
    if let Some(v) = get("bloom_intensity") {
        b.intensity = number("bloom_intensity", v)?;
    }
    if let Some(v) = get("bloom_radius") {
        b.radius = number("bloom_radius", v)?;
    }
    if let Some(v) = get("bloom_tint") {
        b.tint = rgb("bloom_tint", v)?;
    }
    if let Some(v) = get("glare") {
        g.enabled = flag("glare", v)?;
    }
    if let Some(v) = get("glare_threshold") {
        g.threshold = number("glare_threshold", v)?;
    }
    if let Some(v) = get("glare_intensity") {
        g.intensity = number("glare_intensity", v)?;
    }
    if let Some(v) = get("glare_streaks") {
        g.streaks = whole("glare_streaks", v)?;
    }
    if let Some(v) = get("glare_angle") {
        // The bridge's angles are radians; the panel shows degrees.
        g.angle = number("glare_angle", v)?.to_degrees();
    }
    if let Some(v) = get("glare_length") {
        g.length = number("glare_length", v)?;
    }
    if let Some(v) = get("glare_tint") {
        g.tint = rgb("glare_tint", v)?;
    }
    e.dispatch(Command::SetRenderSettings(s))?;
    // Exposure and gamma: the Final Color Render Output's settings.
    if ["exposure", "white_level", "iso", "gamma"]
        .iter()
        .any(|k| get(k).is_some())
    {
        use operandi_core::render_outputs::ExposureControl;
        let o = final_color(&e.scene)
            .ok_or("The shader tree has no enabled Final Color output, which holds exposure")?;
        let mut out = o.settings.clone();
        if let Some(v) = get("exposure") {
            out.exposure = pick(ExposureControl::ALL, ExposureControl::label, "exposure", v)?;
        }
        if let Some(v) = get("white_level") {
            out.white_level = number("white_level", v)?;
        }
        if let Some(v) = get("iso") {
            out.iso = number("iso", v)?;
        }
        if let Some(v) = get("gamma") {
            out.gamma = number("gamma", v)?;
        }
        if !out.valid() {
            return Err("Exposure, white level, ISO or gamma is out of range".into());
        }
        e.dispatch(Command::ShaderTree(
            operandi_core::ShaderTreeEdit::SetOutput {
                node: o.id,
                settings: Box::new(out),
                continuing: false,
            },
        ))?;
    }
    if let Some(v) = get("resolution") {
        let id = e
            .scene
            .render_camera()
            .ok_or("The scene has no camera to set the resolution on")?;
        let mut a = Map::new();
        a.insert("item".into(), json!(id));
        a.insert("resolution".into(), v.clone());
        camera_set(e, &a)?;
    }
    fold_steps(e, start, "Render Settings");
    Ok(())
}

/// The first enabled Final Color Render Output.
fn final_color(scene: &Scene) -> Option<operandi_core::render_outputs::RenderOutput> {
    scene
        .shader_tree
        .render_outputs()
        .into_iter()
        .find(|o| o.effect == operandi_core::shader_tree::OutputEffect::FinalColor)
}

/// Every Backdrop field `item_get` reports and `backdrop_set` takes
/// (BRQ-30), as the Backdrop panel shows them.
pub const BACKDROP_FIELDS: &[(&str, &str)] = &[
    (
        "image",
        "a picture file path (loaded into the scene's images), an image id, or null for none",
    ),
    (
        "projection",
        "Front, Back, Top, Bottom, Left, Right or Camera",
    ),
    ("show_in_perspective", "bool"),
    ("keep_aspect", "bool"),
    ("flip", "bool"),
    ("invert", "bool"),
    ("blend", "bool"),
    ("contrast", "fraction (0.5 = 50 %)"),
    ("brightness", "fraction"),
    ("transparency", "fraction 0-1"),
    ("uniform_size", "bool: Use Uniform Size"),
    (
        "pixel_size",
        "m per image pixel (only with uniform_size off)",
    ),
];

/// A Backdrop's panel values, and its image's file and pixel size.
pub fn backdrop_json(scene: &Scene, b: &operandi_core::backdrop::Backdrop) -> Value {
    let image = b.image.and_then(|id| scene.image(id)).map(|im| {
        json!({"id": im.id, "name": im.name, "path": im.path, "width": im.width, "height": im.height})
    });
    json!({
        "image": image,
        "projection": b.projection.label(),
        "show_in_perspective": b.show_in_perspective,
        "keep_aspect": b.keep_aspect,
        "flip": b.flip,
        "invert": b.invert,
        "blend": b.blend,
        "contrast": b.contrast,
        "brightness": b.brightness,
        "transparency": b.transparency,
        "uniform_size": b.uniform_size,
        "pixel_size": b.pixel_size,
    })
}

/// `backdrop_set`: Backdrop properties through the panel's own edits, one
/// undo step (BRQ-30).
pub fn backdrop_set(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    use operandi_core::backdrop::{BackdropEdit, Projection as BackdropProjection};
    fields_of(args, BACKDROP_FIELDS, "backdrop_set")?;
    let id = item(&e.scene, args)?.id;
    if !e
        .scene
        .items
        .iter()
        .any(|i| i.id == id && i.backdrop.is_some())
    {
        return Err(format!("Item {id} is not a backdrop"));
    }
    let start = e.history_mark();
    // Use Uniform Size first, so a Pixel Size in the same call applies.
    let order = ["image", "uniform_size"];
    let keys = order.iter().copied().chain(
        BACKDROP_FIELDS
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| !order.contains(k)),
    );
    for key in keys {
        let Some(v) = args.get(key) else {
            continue;
        };
        if key == "image" {
            match v {
                Value::Null => e.dispatch(Command::EditBackdrop {
                    item: id,
                    edit: BackdropEdit::Image(None),
                })?,
                Value::Number(n) => e.dispatch(Command::EditBackdrop {
                    item: id,
                    edit: BackdropEdit::Image(Some(
                        n.as_u64().ok_or("image id is a whole number")?,
                    )),
                })?,
                Value::String(path) => {
                    let (width, height) =
                        operandi_core::node_graph::picture_size(std::path::Path::new(path))?;
                    e.dispatch(Command::BackdropLoadImage {
                        item: id,
                        path: path.clone(),
                        width,
                        height,
                    })?
                }
                _ => return Err("image is a file path, an image id or null".into()),
            }
            continue;
        }
        let edit = match key {
            "projection" => BackdropEdit::Projection(pick(
                &BackdropProjection::ALL,
                BackdropProjection::label,
                key,
                v,
            )?),
            "show_in_perspective" => BackdropEdit::ShowInPerspective(flag(key, v)?),
            "keep_aspect" => BackdropEdit::KeepAspect(flag(key, v)?),
            "flip" => BackdropEdit::Flip(flag(key, v)?),
            "invert" => BackdropEdit::Invert(flag(key, v)?),
            "blend" => BackdropEdit::Blend(flag(key, v)?),
            "contrast" => BackdropEdit::Contrast(number(key, v)?),
            "brightness" => BackdropEdit::Brightness(number(key, v)?),
            "transparency" => BackdropEdit::Transparency(number(key, v)?),
            "uniform_size" => BackdropEdit::UniformSize(flag(key, v)?),
            "pixel_size" => BackdropEdit::PixelSize(number(key, v)?),
            _ => unreachable!("every backdrop field is handled"),
        };
        e.dispatch(Command::EditBackdrop { item: id, edit })?;
    }
    fold_steps(e, start, "Backdrop Properties");
    Ok(())
}

/// `meshop_set`: one Mesh Operation's values, one undo step (BRQ-93).
pub fn meshop_set(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    crate::commands::check_keys("meshop_set", args, &["op", "values"])?;
    let text = args
        .get("op")
        .and_then(Value::as_str)
        .ok_or("op is required, written op:<item>:<op>")?;
    let op = operandi_core::mesh_stack_editor::OpRef::parse(text)
        .ok_or("op must be written op:<item>:<op>")?;
    let values = args
        .get("values")
        .and_then(Value::as_object)
        .ok_or("values is an object of option names and values")?
        .clone();
    e.dispatch(Command::MeshOpSet { op, values })
}
