//! `item.channel` tables for environments (docs/COMMAND_REGISTRY.md, "How
//! to add channels"; spec environments-2026-11-09 §2, §3, §5).
//!
//! - The Environment item (`item:<id>`): intensity, visCamera,
//!   visIndirect, visReflection, visRefraction, lpeLabel.
//! - Its Environment Material (`layer:<id>`): enable, invert, blend,
//!   opacity, type, zenColor, skyColor, gndColor, nadColor, skyExp, gndExp,
//!   discSize, haze, clampSky, skyGamma, albedo, discColor, discInScatter,
//!   fogType, fogStart, fogEnd, fogDensity, fogAltitude, fogFalloff. Its Sun
//!   Light is set with `environment.light`.
//! - A texture layer under an environment (`layer:<id>`): enable, invert,
//!   blend, opacity, effect, color and value (Constant), image (Image Map:
//!   a scene image's name, or none).
//!
//! Channel names are Operandi's own for now (ENV-Q-OP1). Every set is one
//! `Command` (one undo step).
use crate::channels::{Channel, ChannelType, ChannelValue, Result, Target};
use crate::environment::{
    ENVIRONMENT_EFFECTS, Environment, EnvironmentEdit, EnvironmentFogType, EnvironmentLayerEdit,
    EnvironmentMaterial, EnvironmentType, GradientColor, RayKind,
};
use crate::shader_tree::{BlendMode, Node, NodeKind, TextureSource};
use crate::{Command, Scene};

const TYPES: &[&str] = &[
    "Constant",
    "2 Color Gradient",
    "4 Color Gradient",
    "Overcast",
    "Physically-based Daylight",
];
const FOGS: &[&str] = &["None", "Linear", "Exponential", "Underwater", "Layered"];
const EFFECTS: &[&str] = &[
    "Environment Color",
    "Driver A",
    "Driver B",
    "Driver C",
    "Driver D",
    "Group Mask",
    "Layer Mask",
];

// ------------------------------------------------------------ helpers

fn item_env(scene: &Scene, target: Target) -> Option<&Environment> {
    match target {
        Target::Item(id) => scene.item(id)?.environment.as_deref(),
        _ => None,
    }
}

fn is_env_item(scene: &Scene, target: Target) -> bool {
    item_env(scene, target).is_some()
}

/// The environment whose Environment Material is `target`, and its id.
fn material(scene: &Scene, target: Target) -> Option<(u64, &Environment)> {
    let Target::Layer(id) = target else {
        return None;
    };
    scene.items.iter().find_map(|i| {
        let env = i.environment.as_deref()?;
        (env.material.id == id).then_some((i.id, env))
    })
}

fn is_material(scene: &Scene, target: Target) -> bool {
    material(scene, target).is_some()
}

fn settings(scene: &Scene, target: Target) -> Option<&EnvironmentMaterial> {
    material(scene, target).map(|(_, e)| e.material.settings.as_ref())
}

/// A texture layer under an environment: its environment's id and itself.
fn env_layer(scene: &Scene, target: Target) -> Option<(u64, &Node)> {
    let Target::Layer(id) = target else {
        return None;
    };
    match scene.environment_layer(id)? {
        (item, Some(node)) => Some((item, node)),
        (_, None) => None,
    }
}

fn is_env_layer(scene: &Scene, target: Target) -> bool {
    env_layer(scene, target).is_some()
}

fn is_constant_layer(scene: &Scene, target: Target) -> bool {
    env_layer(scene, target).is_some_and(|(_, n)| {
        matches!(&n.kind, NodeKind::Texture(t) if matches!(t.source, TextureSource::Constant { .. }))
    })
}

fn is_image_layer(scene: &Scene, target: Target) -> bool {
    env_layer(scene, target).is_some_and(|(_, n)| {
        matches!(&n.kind, NodeKind::Texture(t) if matches!(t.source, TextureSource::ImageMap { .. }))
    })
}

/// The one command for an edit of the environment `target` belongs to
/// (the item itself, or the item of its material).
fn env_edit(scene: &Scene, target: Target, edit: EnvironmentEdit) -> Result<Command> {
    let item = match target {
        Target::Item(id) if is_env_item(scene, target) => id,
        Target::Layer(_) => {
            material(scene, target)
                .ok_or("Not an Environment Material")?
                .0
        }
        _ => return Err("Not an environment".into()),
    };
    Ok(Command::EditEnvironment { item, edit })
}

/// An environment texture layer changed by `edit`, as one Replace.
fn layer_edit(scene: &Scene, target: Target, edit: impl FnOnce(&mut Node)) -> Result<Command> {
    let (item, node) = env_layer(scene, target).ok_or("Not an environment layer")?;
    let mut node = node.clone();
    edit(&mut node);
    Ok(Command::EditEnvironmentLayers {
        item,
        edit: EnvironmentLayerEdit::Replace(Box::new(node)),
    })
}

fn float(v: ChannelValue) -> Result<f64> {
    match v {
        ChannelValue::Float(f) => Ok(f),
        _ => Err("Expected a number".into()),
    }
}

fn boolean(v: ChannelValue) -> Result<bool> {
    match v {
        ChannelValue::Bool(b) => Ok(b),
        _ => Err("Expected true or false".into()),
    }
}

fn colour(v: ChannelValue) -> Result<[f64; 3]> {
    match v {
        ChannelValue::Colour(c) => Ok(c),
        _ => Err("Expected r,g,b".into()),
    }
}

fn index(v: ChannelValue) -> Result<usize> {
    match v {
        ChannelValue::Enum(i) => Ok(i),
        _ => Err("Expected one of the names".into()),
    }
}

fn blend_index(b: BlendMode) -> usize {
    BlendMode::ALL.iter().position(|&x| x == b).unwrap_or(0)
}

// ------------------------------------------------------------ the item

/// The Environment item's channels (ENV §2).
pub(crate) static ITEM: &[Channel] = &[
    Channel {
        name: "intensity",
        ty: ChannelType::Float,
        applies: is_env_item,
        get: |s, t| item_env(s, t).map(|e| ChannelValue::Float(e.intensity)),
        set: |s, t, v| env_edit(s, t, EnvironmentEdit::Intensity(float(v)?)),
    },
    Channel {
        name: "visCamera",
        ty: ChannelType::Bool,
        applies: is_env_item,
        get: |s, t| item_env(s, t).map(|e| ChannelValue::Bool(e.visible_to_camera)),
        set: |s, t, v| env_edit(s, t, EnvironmentEdit::Visible(RayKind::Camera, boolean(v)?)),
    },
    Channel {
        name: "visIndirect",
        ty: ChannelType::Bool,
        applies: is_env_item,
        get: |s, t| item_env(s, t).map(|e| ChannelValue::Bool(e.visible_to_indirect)),
        set: |s, t, v| {
            env_edit(
                s,
                t,
                EnvironmentEdit::Visible(RayKind::Indirect, boolean(v)?),
            )
        },
    },
    Channel {
        name: "visReflection",
        ty: ChannelType::Bool,
        applies: is_env_item,
        get: |s, t| item_env(s, t).map(|e| ChannelValue::Bool(e.visible_to_reflection)),
        set: |s, t, v| {
            env_edit(
                s,
                t,
                EnvironmentEdit::Visible(RayKind::Reflection, boolean(v)?),
            )
        },
    },
    Channel {
        name: "visRefraction",
        ty: ChannelType::Bool,
        applies: is_env_item,
        get: |s, t| item_env(s, t).map(|e| ChannelValue::Bool(e.visible_to_refraction)),
        set: |s, t, v| {
            env_edit(
                s,
                t,
                EnvironmentEdit::Visible(RayKind::Refraction, boolean(v)?),
            )
        },
    },
    Channel {
        name: "lpeLabel",
        ty: ChannelType::Text,
        applies: is_env_item,
        get: |s, t| item_env(s, t).map(|e| ChannelValue::Text(e.lpe_label.clone())),
        set: |s, t, v| match v {
            ChannelValue::Text(text) => env_edit(s, t, EnvironmentEdit::LpeLabel(text)),
            _ => Err("Expected text".into()),
        },
    },
];

// ------------------------------------------------------------ the material

macro_rules! material_colour {
    ($name:literal, $field:ident, $which:expr) => {
        Channel {
            name: $name,
            ty: ChannelType::Colour,
            applies: is_material,
            get: |s, t| settings(s, t).map(|m| ChannelValue::Colour(m.$field)),
            set: |s, t, v| env_edit(s, t, EnvironmentEdit::Color($which, colour(v)?)),
        }
    };
}

macro_rules! material_number {
    ($name:literal, $ty:expr, |$m:ident| $get:expr, $make:expr) => {
        Channel {
            name: $name,
            ty: $ty,
            applies: is_material,
            get: |s, t| settings(s, t).map(|$m| ChannelValue::Float($get)),
            set: |s, t, v| env_edit(s, t, $make(float(v)?)),
        }
    };
}

/// The Environment Material's channels (ENV §3), on `layer:<id>`.
pub(crate) static MATERIAL: &[Channel] = &[
    Channel {
        name: "enable",
        ty: ChannelType::Bool,
        applies: is_material,
        get: |s, t| material(s, t).map(|(_, e)| ChannelValue::Bool(e.material.layer.enable)),
        set: |s, t, v| env_edit(s, t, EnvironmentEdit::Enable(boolean(v)?)),
    },
    Channel {
        name: "invert",
        ty: ChannelType::Bool,
        applies: is_material,
        get: |s, t| material(s, t).map(|(_, e)| ChannelValue::Bool(e.material.layer.invert)),
        set: |s, t, v| env_edit(s, t, EnvironmentEdit::Invert(boolean(v)?)),
    },
    Channel {
        name: "blend",
        ty: ChannelType::Enum(BlendMode::LABELS),
        applies: is_material,
        get: |s, t| {
            material(s, t).map(|(_, e)| ChannelValue::Enum(blend_index(e.material.layer.blend)))
        },
        set: |s, t, v| {
            let blend = *BlendMode::ALL.get(index(v)?).ok_or("Unknown blend mode")?;
            env_edit(s, t, EnvironmentEdit::Blend(blend))
        },
    },
    Channel {
        name: "opacity",
        ty: ChannelType::Percent,
        applies: is_material,
        get: |s, t| material(s, t).map(|(_, e)| ChannelValue::Float(e.material.layer.opacity)),
        set: |s, t, v| env_edit(s, t, EnvironmentEdit::Opacity(float(v)?)),
    },
    Channel {
        name: "type",
        ty: ChannelType::Enum(TYPES),
        applies: is_material,
        get: |s, t| {
            settings(s, t).map(|m| {
                ChannelValue::Enum(
                    EnvironmentType::ALL
                        .iter()
                        .position(|&k| k == m.kind)
                        .unwrap_or(0),
                )
            })
        },
        set: |s, t, v| {
            let kind = *EnvironmentType::ALL
                .get(index(v)?)
                .ok_or("Unknown environment type")?;
            env_edit(s, t, EnvironmentEdit::Kind(kind))
        },
    },
    material_colour!("zenColor", zenith, GradientColor::Zenith),
    material_colour!("skyColor", sky, GradientColor::Sky),
    material_colour!("gndColor", ground, GradientColor::Ground),
    material_colour!("nadColor", nadir, GradientColor::Nadir),
    material_number!(
        "skyExp",
        ChannelType::Float,
        |m| m.sky_exponent,
        EnvironmentEdit::SkyExponent
    ),
    material_number!(
        "gndExp",
        ChannelType::Float,
        |m| m.ground_exponent,
        EnvironmentEdit::GroundExponent
    ),
    material_number!(
        "discSize",
        ChannelType::Percent,
        |m| m.daylight.disc_size,
        EnvironmentEdit::DiscSize
    ),
    material_number!(
        "haze",
        ChannelType::Float,
        |m| m.daylight.haze,
        EnvironmentEdit::Haze
    ),
    Channel {
        name: "clampSky",
        ty: ChannelType::Bool,
        applies: is_material,
        get: |s, t| settings(s, t).map(|m| ChannelValue::Bool(m.daylight.clamp_sky_brightness)),
        set: |s, t, v| env_edit(s, t, EnvironmentEdit::ClampSkyBrightness(boolean(v)?)),
    },
    material_number!(
        "skyGamma",
        ChannelType::Float,
        |m| m.daylight.sky_gamma,
        EnvironmentEdit::SkyGamma
    ),
    Channel {
        name: "albedo",
        ty: ChannelType::Colour,
        applies: is_material,
        get: |s, t| settings(s, t).map(|m| ChannelValue::Colour(m.daylight.ground_albedo)),
        set: |s, t, v| env_edit(s, t, EnvironmentEdit::GroundAlbedo(colour(v)?)),
    },
    Channel {
        name: "discColor",
        ty: ChannelType::Colour,
        applies: is_material,
        get: |s, t| settings(s, t).map(|m| ChannelValue::Colour(m.daylight.disc_color)),
        set: |s, t, v| env_edit(s, t, EnvironmentEdit::DiscColor(colour(v)?)),
    },
    material_number!(
        "discInScatter",
        ChannelType::Percent,
        |m| m.daylight.disc_in_scatter,
        EnvironmentEdit::DiscInScatter
    ),
    Channel {
        name: "fogType",
        ty: ChannelType::Enum(FOGS),
        applies: is_material,
        get: |s, t| {
            settings(s, t).map(|m| {
                ChannelValue::Enum(
                    EnvironmentFogType::ALL
                        .iter()
                        .position(|&k| k == m.fog.fog_type)
                        .unwrap_or(0),
                )
            })
        },
        set: |s, t, v| {
            let kind = *EnvironmentFogType::ALL
                .get(index(v)?)
                .ok_or("Unknown fog type")?;
            env_edit(s, t, EnvironmentEdit::FogType(kind))
        },
    },
    material_number!(
        "fogStart",
        ChannelType::Distance,
        |m| m.fog.start,
        EnvironmentEdit::FogStart
    ),
    material_number!(
        "fogEnd",
        ChannelType::Distance,
        |m| m.fog.end,
        EnvironmentEdit::FogEnd
    ),
    material_number!(
        "fogDensity",
        ChannelType::Float,
        |m| m.fog.density,
        EnvironmentEdit::FogDensity
    ),
    material_number!(
        "fogAltitude",
        ChannelType::Distance,
        |m| m.fog.base_altitude,
        EnvironmentEdit::FogBaseAltitude
    ),
    material_number!(
        "fogFalloff",
        ChannelType::Float,
        |m| m.fog.altitude_falloff,
        EnvironmentEdit::FogAltitudeFalloff
    ),
];

// ------------------------------------------------------------ texture layers

/// Texture layers under an environment (ENV §5), on `layer:<id>`.
pub(crate) static LAYER: &[Channel] = &[
    Channel {
        name: "enable",
        ty: ChannelType::Bool,
        applies: is_env_layer,
        get: |s, t| env_layer(s, t).map(|(_, n)| ChannelValue::Bool(n.layer.enable)),
        set: |s, t, v| {
            let b = boolean(v)?;
            layer_edit(s, t, |n| n.layer.enable = b)
        },
    },
    Channel {
        name: "invert",
        ty: ChannelType::Bool,
        applies: is_env_layer,
        get: |s, t| env_layer(s, t).map(|(_, n)| ChannelValue::Bool(n.layer.invert)),
        set: |s, t, v| {
            let b = boolean(v)?;
            layer_edit(s, t, |n| n.layer.invert = b)
        },
    },
    Channel {
        name: "blend",
        ty: ChannelType::Enum(BlendMode::LABELS),
        applies: is_env_layer,
        get: |s, t| env_layer(s, t).map(|(_, n)| ChannelValue::Enum(blend_index(n.layer.blend))),
        set: |s, t, v| {
            let blend = *BlendMode::ALL.get(index(v)?).ok_or("Unknown blend mode")?;
            layer_edit(s, t, |n| n.layer.blend = blend)
        },
    },
    Channel {
        name: "opacity",
        ty: ChannelType::Percent,
        applies: is_env_layer,
        get: |s, t| env_layer(s, t).map(|(_, n)| ChannelValue::Float(n.layer.opacity)),
        set: |s, t, v| {
            let o = float(v)?;
            layer_edit(s, t, |n| n.layer.opacity = o)
        },
    },
    Channel {
        name: "effect",
        ty: ChannelType::Enum(EFFECTS),
        applies: is_env_layer,
        get: |s, t| {
            env_layer(s, t).and_then(|(_, n)| match &n.kind {
                NodeKind::Texture(tx) => ENVIRONMENT_EFFECTS
                    .iter()
                    .position(|&e| e == tx.effect)
                    .map(ChannelValue::Enum),
                _ => None,
            })
        },
        set: |s, t, v| {
            let effect = *ENVIRONMENT_EFFECTS.get(index(v)?).ok_or("Unknown effect")?;
            layer_edit(s, t, |n| {
                if let NodeKind::Texture(tx) = &mut n.kind {
                    tx.effect = effect;
                }
            })
        },
    },
    Channel {
        name: "color",
        ty: ChannelType::Colour,
        applies: is_constant_layer,
        get: |s, t| {
            env_layer(s, t).and_then(|(_, n)| match &n.kind {
                NodeKind::Texture(tx) => match &tx.source {
                    TextureSource::Constant { color, .. } => Some(ChannelValue::Colour(*color)),
                    _ => None,
                },
                _ => None,
            })
        },
        set: |s, t, v| {
            let c = colour(v)?;
            layer_edit(s, t, |n| {
                if let NodeKind::Texture(tx) = &mut n.kind
                    && let TextureSource::Constant { color, .. } = &mut tx.source
                {
                    *color = c;
                }
            })
        },
    },
    Channel {
        name: "value",
        ty: ChannelType::Float,
        applies: is_constant_layer,
        get: |s, t| {
            env_layer(s, t).and_then(|(_, n)| match &n.kind {
                NodeKind::Texture(tx) => match &tx.source {
                    TextureSource::Constant { value, .. } => Some(ChannelValue::Float(*value)),
                    _ => None,
                },
                _ => None,
            })
        },
        set: |s, t, v| {
            let x = float(v)?;
            layer_edit(s, t, |n| {
                if let NodeKind::Texture(tx) = &mut n.kind
                    && let TextureSource::Constant { value, .. } = &mut tx.source
                {
                    *value = x;
                }
            })
        },
    },
    Channel {
        // A scene image by name (`none` clears it).
        name: "image",
        ty: ChannelType::Text,
        applies: is_image_layer,
        get: |s, t| {
            env_layer(s, t).and_then(|(_, n)| match &n.kind {
                NodeKind::Texture(tx) => match &tx.source {
                    TextureSource::ImageMap { image, .. } => Some(ChannelValue::Text(
                        image
                            .and_then(|id| s.image(id))
                            .map_or("none".into(), |i| i.name.clone()),
                    )),
                    _ => None,
                },
                _ => None,
            })
        },
        set: |s, t, v| {
            let ChannelValue::Text(name) = v else {
                return Err("Expected an image name".into());
            };
            let picked = if name.eq_ignore_ascii_case("none") {
                None
            } else {
                Some(
                    s.images
                        .iter()
                        .find(|i| i.name == name)
                        .ok_or_else(|| format!("No scene image named {name}"))?
                        .id,
                )
            };
            layer_edit(s, t, |n| {
                if let NodeKind::Texture(tx) = &mut n.kind
                    && let TextureSource::ImageMap { image, .. } = &mut tx.source
                {
                    *image = picked;
                }
            })
        },
    },
];
