//! Mesh lights through USD (usd-gaps-3): an Operandi Mesh light (a light
//! item whose prototype mesh glows, LP-MS) is UsdLux's MeshLightAPI on its
//! prototype's Mesh prim, not normalized, so `inputs:intensity` is the
//! glow's radiance, as Operandi's Intensity is (the same surface as an
//! area light of the same radiance renders alike). The mesh keeps its
//! material, which doesn't respond (`noMaterialResponse`): in Operandi a
//! mesh light's prototype renders only as the light. The light item's
//! name goes in the mesh's customData `operandi:meshLight`.
//!
//! Only lights that glow where their mesh is (Use Prototype Transform
//! on) can sit on the mesh; the others, lights without a mesh and lights
//! of Mesh Instances are left out and reported.
//!
//! Import: a Mesh prim with MeshLightAPI comes in as its mesh plus a Mesh
//! light item using it, beside it, named as written (else "<mesh> Light").
use crate::stage::LightData;
use operandi_core::camera_light::{LightKind, LightSettings};
use operandi_core::scene::Transform;
use operandi_core::{Result, Scene};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// A light carried by its prototype's mesh: the light item's name and
/// what `ou_define_mesh_extras` writes as "light".
#[derive(Clone, Debug, PartialEq)]
pub struct Carried {
    pub name: String,
    pub light: Value,
}

/// The mesh lights each prototype mesh carries, by prototype item id;
/// those that can't be carried are reported in `notes`.
pub fn carried(scene: &Scene, notes: &mut Vec<String>) -> BTreeMap<u64, Carried> {
    let mut out = BTreeMap::new();
    let mut left = Vec::new();
    for item in &scene.items {
        let Some(l) = item.light.as_ref().filter(|l| l.kind == LightKind::Mesh) else {
            continue;
        };
        let proto = l
            .prototype
            .and_then(|p| scene.items.iter().find(|i| i.id == p && i.is_mesh()));
        let why = match proto {
            None => Some("no mesh"),
            Some(_) if !l.use_prototype_transform => Some("Use Prototype Transform off"),
            Some(p) if p.instance_of.is_some() => Some("its mesh is a Mesh Instance"),
            Some(p) if out.contains_key(&p.id) => Some("another light uses its mesh"),
            Some(_) => None,
        };
        if let Some(why) = why {
            left.push(format!("{} ({why})", item.name));
            continue;
        }
        let keep = 1.0 - l.dissolve.clamp(0.0, 1.0);
        out.insert(
            proto.expect("checked").id,
            Carried {
                name: item.name.clone(),
                light: json!({
                    "intensity": l.intensity * keep,
                    "color": l.color,
                    "diffuse": l.affect_diffuse,
                    "specular": l.affect_specular,
                }),
            },
        );
    }
    if !left.is_empty() {
        notes.push(format!("Mesh lights not exported: {}", left.join(", ")));
    }
    out
}

/// Adds the Mesh light `light` (a MeshLightAPI prim's attributes) for
/// mesh item `mesh`, under `parent`; `custom` is the prim's Operandi
/// customData.
pub fn add(
    scene: &mut Scene,
    mesh: u64,
    parent: Option<u64>,
    mesh_name: &str,
    light: &LightData,
    custom: Option<&Value>,
) -> Result<u64> {
    let mut s = LightSettings::new(LightKind::Mesh);
    // Not normalized: radiance = intensity × 2^exposure (UsdLux).
    s.intensity = light.intensity.unwrap_or(1.0) * 2f64.powf(light.exposure.unwrap_or(0.0));
    if let Some(c) = light.color {
        s.color = c;
    }
    if let Some(d) = light.diffuse {
        s.affect_diffuse = d;
    }
    if let Some(sp) = light.specular {
        s.affect_specular = sp;
    }
    s.prototype = Some(mesh);
    s.use_prototype_transform = true;
    let id = scene.add_light(s, Transform::default())?;
    let name = custom
        .and_then(|c| c.get("meshLight"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{mesh_name} Light"));
    if let Some(item) = scene.items.iter_mut().find(|i| i.id == id) {
        item.name = name;
        item.parent = parent;
    }
    Ok(id)
}
