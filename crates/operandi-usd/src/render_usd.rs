//! Render settings through USD (usd-gaps-3): the Render item as a UsdRender
//! RenderSettings prim (`/Render/Settings`, named by the stage's
//! renderSettingsPrimPath) with its resolution, pixel aspect and render
//! camera, and one RenderProduct (`/Render/Products/Final`) whose
//! resolution is the picture the render camera makes (its Resolution
//! Override when on). Cycles' settings (samples, noise threshold, ray
//! depths, caustics, clamp, Preview limits) have no UsdRender attribute;
//! they go as customData `operandi:render`, Operandi's own settings rows,
//! so they come back exactly.
//!
//! Import reads the same: the settings' resolution is the frame, a
//! product of another size turns the render camera's Resolution Override
//! on, and Operandi's customData restores the rest. Other programs'
//! renderer-specific attributes are not read (reported when the settings
//! name one we don't know).
use operandi_core::Scene;
use operandi_core::render_settings::{MAX_FRAME, RenderSettings};
use serde::Deserialize;
use serde_json::{Value, json};

/// What the export writes.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderPlan {
    /// The Render item's frame.
    pub resolution: [u32; 2],
    /// The picture the render camera makes.
    pub product_resolution: [u32; 2],
    /// `operandi:render`: the Render item's rows.
    pub custom: Value,
}

/// The scene's render settings as USD.
pub fn plan(scene: &Scene) -> RenderPlan {
    let frame = scene.render.frame;
    let product = scene
        .render_camera()
        .and_then(|id| scene.items.iter().find(|i| i.id == id))
        .and_then(|i| i.camera.as_ref())
        .filter(|c| c.resolution.enable)
        .map_or(frame, |c| [c.resolution.width, c.resolution.height]);
    RenderPlan {
        resolution: frame,
        product_resolution: product,
        custom: json!({
            "render": serde_json::to_value(&scene.render).unwrap_or(Value::Null),
        }),
    }
}

/// The JSON `ou_define_render_settings` reads; `camera` is the render
/// camera's prim path when it was written.
pub fn json(plan: &RenderPlan, camera: Option<&str>) -> String {
    let mut v = json!({
        "scope": "/Render",
        "name": "Settings",
        "resolution": plan.resolution,
        "pixelAspect": 1.0,
        "product": {
            "name": "Final",
            "productName": "render.exr",
            "resolution": plan.product_resolution,
        },
        "custom": plan.custom,
    });
    if let Some(c) = camera {
        v["camera"] = c.into();
    }
    v.to_string()
}

/// The stage's render settings as the reader gives them.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RenderSettingsData {
    pub path: String,
    pub resolution: Option<[i64; 2]>,
    #[serde(rename = "pixelAspect")]
    pub pixel_aspect: Option<f64>,
    pub camera: Option<String>,
    pub custom: Option<Value>,
    pub product: Option<ProductData>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ProductData {
    pub path: String,
    pub resolution: Option<[i64; 2]>,
}

fn size(r: [i64; 2]) -> Option<[u32; 2]> {
    let ok = |v: i64| (1..=i64::from(MAX_FRAME)).contains(&v);
    (ok(r[0]) && ok(r[1])).then(|| [r[0] as u32, r[1] as u32])
}

/// Puts `data` in `scene` (after its cameras came in, so the render
/// camera is set); report lines go to `notes`.
pub fn apply(scene: &mut Scene, data: &RenderSettingsData, notes: &mut Vec<String>) {
    // Operandi's own rows first: exactly what was written.
    let ours = data
        .custom
        .as_ref()
        .and_then(|c| c.get("render"))
        .and_then(|r| serde_json::from_value::<RenderSettings>(r.clone()).ok())
        .filter(|r| r.validate().is_ok());
    let from_operandi = ours.is_some();
    if let Some(r) = ours {
        scene.render = r;
    }
    match data.resolution.map(size) {
        Some(Some(frame)) => scene.render.frame = frame,
        Some(None) => notes.push(format!(
            "Render settings: resolution out of range (1 to {MAX_FRAME}); kept {} × {}",
            scene.render.frame[0], scene.render.frame[1]
        )),
        None => {}
    }
    if let Some(a) = data.pixel_aspect
        && (a - 1.0).abs() > 1e-6
    {
        notes.push(format!(
            "Render settings: pixel aspect {a} read as square pixels"
        ));
    }
    // A product of another size is the render camera's own resolution.
    let product = data
        .product
        .as_ref()
        .and_then(|p| p.resolution)
        .and_then(size);
    if let Some(p) = product
        && p != scene.render.frame
    {
        let cam = scene.render_camera();
        match cam.and_then(|id| scene.items.iter_mut().find(|i| i.id == id)) {
            Some(item) => {
                if let Some(c) = item.camera.as_mut() {
                    c.resolution.enable = true;
                    c.resolution.width = p[0];
                    c.resolution.height = p[1];
                }
            }
            None => notes.push(format!(
                "Render settings: product size {} × {} left out (no render camera)",
                p[0], p[1]
            )),
        }
    }
    if !from_operandi {
        notes.push(format!(
            "Render settings from {}: resolution {} × {}; sampling and ray depths kept as Operandi's",
            data.path, scene.render.frame[0], scene.render.frame[1]
        ));
    }
}
