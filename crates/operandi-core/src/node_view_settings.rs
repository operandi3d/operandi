//! Texture layer settings a node-view wire can drive (node editing, NE-03
//! for texture layers; NE-Q01 closed 2026-11-21). The settings that are one
//! plain colour or number: a Constant's colour or value; the two zone
//! colours or values of a procedural, a weight Vertex Map Texture and an
//! Occlusion; an Image Map's Gamma, Brightness, Contrast and (scalar
//! effects) Low and High Value; a Gradient's Input Parameter, the number it looks
//! up on its curves. A layer used as a mask takes the same wires as its
//! source does. Wires go in as `Target::Layer` with these keys
//! (`texture.color`...), next to a Material layer's channel keys.
//!
//! Not wire targets: an Image Map's picture (a file, not a value) and its
//! switches (swizzle, alpha, invert, clamp, filtering); a Gradient's curve
//! keys (a list, not one value) and its Input choice itself; a colour
//! Vertex Map's colours (they are the map); Occlusion's rays, distance and
//! angles (Cycles' AO node reads them as fixed settings); zone alphas,
//! bias and gain (provisional, NE-Q07).
use crate::shader_tree::{NodeKind, TextureLayer, TextureSource};

/// Every texture setting key, its Properties label and MaterialX type.
pub const SETTINGS: [(&str, &str, &str); 12] = [
    ("texture.color", "Color", "color3"),
    ("texture.value", "Value", "float"),
    ("texture.zone1_color", "Color 1", "color3"),
    ("texture.zone1_value", "Value 1", "float"),
    ("texture.zone2_color", "Color 2", "color3"),
    ("texture.zone2_value", "Value 2", "float"),
    ("texture.gamma", "Gamma", "float"),
    ("texture.brightness", "Brightness", "float"),
    ("texture.contrast", "Contrast", "float"),
    ("texture.low_value", "Low Value", "float"),
    ("texture.high_value", "High Value", "float"),
    ("texture.gradient_input", "Input Parameter", "float"),
];

/// The two zone keys for a colour or a scalar effect.
const ZONE_COLORS: &[&str] = &["texture.zone1_color", "texture.zone2_color"];
const ZONE_VALUES: &[&str] = &["texture.zone1_value", "texture.zone2_value"];

/// Whether a texture layer's value is a colour (its effect's channel is).
pub fn colour_effect(t: &TextureLayer) -> bool {
    use crate::shader_tree::TextureEffect;
    t.effect == TextureEffect::Rgba
        || crate::shader_eval::Ch::of_effect(t.effect).is_some_and(|c| c.is_color())
}

/// The drivable settings of texture layer `t`: key, label, type, value.
pub fn settings(t: &TextureLayer) -> Vec<(&'static str, &'static str, &'static str, Vec<f64>)> {
    let colour = colour_effect(t);
    use crate::vertex_map_texture::VertexMapKind;
    let zones = if colour { ZONE_COLORS } else { ZONE_VALUES };
    let keys: &[&str] = match (&t.source, colour) {
        (TextureSource::Constant { .. }, true) => &["texture.color"],
        (TextureSource::Constant { .. }, false) => &["texture.value"],
        (TextureSource::Procedural(_) | TextureSource::Occlusion(_), _) => zones,
        // A colour map's colours are the map itself.
        (TextureSource::VertexMap { map, .. }, _)
            if map.as_ref().is_none_or(|m| m.kind == VertexMapKind::Weight) =>
        {
            zones
        }
        (TextureSource::ImageMap { .. }, true) => {
            &["texture.gamma", "texture.brightness", "texture.contrast"]
        }
        (TextureSource::ImageMap { .. }, false) => &[
            "texture.gamma",
            "texture.brightness",
            "texture.contrast",
            "texture.low_value",
            "texture.high_value",
        ],
        (TextureSource::Gradient(_), _) => &["texture.gradient_input"],
        _ => &[],
    };
    keys.iter()
        .filter_map(|k| {
            let (key, label, ty) = SETTINGS.iter().find(|(x, _, _)| x == k)?;
            Some((*key, *label, *ty, value(t, key)?))
        })
        .collect()
}

/// A setting's own value: three numbers for a colour, one for a value.
pub fn value(t: &TextureLayer, key: &str) -> Option<Vec<f64>> {
    if let Some(z) = zones(&t.source) {
        return Some(match key {
            "texture.zone1_color" => z.zone1_color.to_vec(),
            "texture.zone2_color" => z.zone2_color.to_vec(),
            "texture.zone1_value" => vec![z.zone1_value],
            "texture.zone2_value" => vec![z.zone2_value],
            _ => return None,
        });
    }
    Some(match (&t.source, key) {
        (TextureSource::Constant { color, .. }, "texture.color") => color.to_vec(),
        (TextureSource::Constant { value, .. }, "texture.value") => vec![*value],
        (TextureSource::ImageMap { settings: s, .. }, _) => vec![match key {
            "texture.gamma" => s.gamma,
            "texture.brightness" => s.brightness,
            "texture.contrast" => s.contrast,
            "texture.low_value" => s.low_value,
            "texture.high_value" => s.high_value,
            _ => return None,
        }],
        // No number of its own: the Input choice says what it reads, and
        // only a wire gives it a value.
        (TextureSource::Gradient(_), "texture.gradient_input") => Vec::new(),
        _ => return None,
    })
}

/// The two-zone block of a source that has one.
fn zones(source: &TextureSource) -> Option<&crate::shader_tree::TwoZone> {
    match source {
        TextureSource::Procedural(p) => Some(&p.zones),
        TextureSource::VertexMap { zones, .. } => Some(zones),
        TextureSource::Occlusion(o) => Some(&o.zones),
        _ => None,
    }
}

fn zones_mut(source: &mut TextureSource) -> Option<&mut crate::shader_tree::TwoZone> {
    match source {
        TextureSource::Procedural(p) => Some(&mut p.zones),
        TextureSource::VertexMap { zones, .. } => Some(zones),
        TextureSource::Occlusion(o) => Some(&mut o.zones),
        _ => None,
    }
}

/// Sets a setting's own value (`v` as [`value`] gives it); false when the
/// layer has no such setting.
pub fn set(t: &mut TextureLayer, key: &str, v: &[f64]) -> bool {
    let c = |v: &[f64]| [0, 1, 2].map(|i| v.get(i).or(v.first()).copied().unwrap_or(0.));
    let s = v.first().copied().unwrap_or(0.);
    if let Some(z) = zones_mut(&mut t.source) {
        match key {
            "texture.zone1_color" => z.zone1_color = c(v),
            "texture.zone2_color" => z.zone2_color = c(v),
            "texture.zone1_value" => z.zone1_value = s,
            "texture.zone2_value" => z.zone2_value = s,
            _ => return false,
        }
        return true;
    }
    match (&mut t.source, key) {
        (TextureSource::Constant { color, .. }, "texture.color") => *color = c(v),
        (TextureSource::Constant { value, .. }, "texture.value") => *value = s,
        (TextureSource::ImageMap { settings: i, .. }, _) => match key {
            "texture.gamma" => i.gamma = s,
            "texture.brightness" => i.brightness = s,
            "texture.contrast" => i.contrast = s,
            "texture.low_value" => i.low_value = s,
            "texture.high_value" => i.high_value = s,
            _ => return false,
        },
        _ => return false,
    }
    true
}

/// The type a wire into layer `kind`'s setting `key` must carry, `None`
/// when that layer has no such drivable setting.
pub fn setting_type(kind: &NodeKind, key: &str) -> Option<&'static str> {
    let NodeKind::Texture(t) = kind else {
        return None;
    };
    settings(t)
        .into_iter()
        .find(|(k, _, _, _)| *k == key)
        .map(|(_, _, ty, _)| ty)
}

/// A setting's label ("Color 1"), for undo names.
pub fn label(key: &str) -> Option<&'static str> {
    SETTINGS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, l, _)| *l)
}

#[cfg(test)]
#[path = "node_view_settings_tests.rs"]
mod tests;
