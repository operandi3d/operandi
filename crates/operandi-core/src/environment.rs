//! Environment items: what a ray sees when it misses every object (spec
//! environments-2026-11-09, "ENV" below).
//!
//! An environment is an item (`Item::environment`) with its own settings
//! (ENV §2) and a layer stack: the Environment Material (ENV §3) at the
//! bottom and texture layers above it (ENV §5). It lives in the Shader
//! Tree's Environments section. The colour a ray sees comes from
//! [`EnvironmentMaterial::color`] (ENV §4) times Intensity.
//!
//! Environment items are scene items, renamed, deleted and duplicated like
//! any item, but they have no transform and never show in the Item List:
//! only in the Shader Tree's Environments section (ENVIRONMENT-PLACEMENT
//! §1, §2; Jason, 2026-11-09). They draw nothing in the 3D view (ENV §9).
use crate::Result;
use crate::scene::{Item, Scene, Transform};
use crate::shader_tree::{Layer, Node, NodeId};
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// Environment layer ids start above this, clear of the Shader Tree's
/// (which count up from 1) [chosen].
pub const ENVIRONMENT_LAYER_IDS: NodeId = 1 << 32;

/// The new environment item's name (ENV §1).
pub const ENVIRONMENT_NAME: &str = "Environment";
/// Why an environment refuses an item transform (ENVIRONMENT-PLACEMENT §2).
pub const NO_TRANSFORM: &str =
    "Environments have no transform; rotate the sky with its image layer's Texture Locator";
/// The new Environment Material's name (ENV §1, §6).
pub const ENVIRONMENT_MATERIAL_NAME: &str = "Environment Material";

macro_rules! labelled_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const ALL: &[$name] = &[$($name::$variant),+];
            pub fn label(self) -> &'static str {
                match self { $($name::$variant => $label),+ }
            }
        }
    };
}

labelled_enum! {
    /// Environment Type (ENV §3).
    EnvironmentType {
        Constant => "Constant",
        TwoColor => "2 Color Gradient",
        FourColor => "4 Color Gradient",
        Overcast => "Overcast",
        Physical => "Physically-based Daylight",
    }
}

labelled_enum! {
    /// Environment Fog's Fog Type (ENV §3). Stored and shown; only None
    /// renders today (ENV §10).
    EnvironmentFogType {
        None => "None",
        Linear => "Linear",
        Exponential => "Exponential",
        Underwater => "Underwater",
        Layered => "Layered",
    }
}

/// The Physically-based Daylight block (ENV §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Daylight {
    /// The directional light that is the sun; none until one is picked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun_light: Option<u64>,
    /// Solar Disc Size, 1 = 100 %.
    pub disc_size: f64,
    /// Haze Amount (Q-ENV2: default 2.0 [chosen]).
    pub haze: f64,
    pub clamp_sky_brightness: bool,
    pub sky_gamma: f64,
    pub ground_albedo: [f64; 3],
    pub disc_color: [f64; 3],
    /// Disc In-Scatter, 0..1.
    pub disc_in_scatter: f64,
}

impl Default for Daylight {
    fn default() -> Self {
        Self {
            sun_light: None,
            disc_size: 1.0,
            haze: 2.0,
            clamp_sky_brightness: true,
            sky_gamma: 1.0,
            ground_albedo: [0.5; 3],
            disc_color: [1.0; 3],
            disc_in_scatter: 0.0,
        }
    }
}

/// The Environment Fog block (ENV §3). Stored and shown only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentFog {
    pub fog_type: EnvironmentFogType,
    /// Metres.
    pub start: f64,
    /// Metres.
    pub end: f64,
    pub density: f64,
    /// Metres.
    pub base_altitude: f64,
    pub altitude_falloff: f64,
}

impl Default for EnvironmentFog {
    fn default() -> Self {
        Self {
            fog_type: EnvironmentFogType::None,
            start: 0.0,
            end: 10.0,
            density: 0.1,
            base_altitude: 0.0,
            altitude_falloff: 0.5,
        }
    }
}

/// The Environment Material's settings (ENV §3). Colours are linear RGB.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentMaterial {
    pub kind: EnvironmentType,
    pub zenith: [f64; 3],
    pub sky: [f64; 3],
    pub ground: [f64; 3],
    pub nadir: [f64; 3],
    pub sky_exponent: f64,
    pub ground_exponent: f64,
    pub daylight: Daylight,
    pub fog: EnvironmentFog,
}

impl Default for EnvironmentMaterial {
    /// 4 Color Gradient: zenith 0.90, sky and ground 0.34, nadir 0.03,
    /// exponents 4 (ENV §3).
    fn default() -> Self {
        Self {
            kind: EnvironmentType::FourColor,
            zenith: [0.90; 3],
            sky: [0.34; 3],
            ground: [0.34; 3],
            nadir: [0.03; 3],
            sky_exponent: 4.0,
            ground_exponent: 4.0,
            daylight: Daylight::default(),
            fog: EnvironmentFog::default(),
        }
    }
}

fn lerp(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

impl EnvironmentMaterial {
    /// The colour along world direction `direction` (ENV §4), before
    /// Intensity. `direction` need not be unit length. Physically-based
    /// Daylight has no exact colours (Q-ENV1); here it gives the 4 Color
    /// Gradient of its colours, the approximation the viewport draws
    /// [chosen].
    pub fn color(&self, direction: DVec3) -> [f64; 3] {
        let s = direction.normalize_or_zero().y.clamp(-1.0, 1.0);
        self.color_at_height(s)
    }

    /// The colour at height `s` = sin(elevation), -1 straight down to 1
    /// straight up (ENV §4). The same formulas build the Cycles world
    /// (operandi-cycles `environment_color`); change both together.
    pub fn color_at_height(&self, s: f64) -> [f64; 3] {
        match self.kind {
            EnvironmentType::Constant => self.zenith,
            EnvironmentType::TwoColor => lerp(self.nadir, self.zenith, (1.0 + s) / 2.0),
            EnvironmentType::FourColor | EnvironmentType::Physical => {
                if s >= 0.0 {
                    lerp(self.zenith, self.sky, (1.0 - s).powf(self.sky_exponent))
                } else {
                    lerp(
                        self.nadir,
                        self.ground,
                        (1.0 - s.abs()).powf(self.ground_exponent),
                    )
                }
            }
            EnvironmentType::Overcast => {
                if s >= 0.0 {
                    self.zenith.map(|c| c * (1.0 + 2.0 * s) / 3.0)
                } else {
                    self.nadir
                }
            }
        }
    }

    fn validate(&self) -> Result<()> {
        let colours = [
            self.zenith,
            self.sky,
            self.ground,
            self.nadir,
            self.daylight.ground_albedo,
            self.daylight.disc_color,
        ];
        let numbers = [
            self.sky_exponent,
            self.ground_exponent,
            self.daylight.disc_size,
            self.daylight.haze,
            self.daylight.sky_gamma,
            self.daylight.disc_in_scatter,
            self.fog.start,
            self.fog.end,
            self.fog.density,
            self.fog.base_altitude,
            self.fog.altitude_falloff,
        ];
        if colours
            .iter()
            .flatten()
            .chain(&numbers)
            .any(|v| !v.is_finite())
        {
            return Err("Environment Material values must be finite".into());
        }
        if self.sky_exponent < 0.0 || self.ground_exponent < 0.0 {
            return Err("Environment exponents can't be negative".into());
        }
        Ok(())
    }
}

/// The Environment Material layer: the bottom of an environment's stack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentMaterialLayer {
    /// Unique among every environment's layers (`Scene::next_layer_id`).
    pub id: NodeId,
    pub name: String,
    /// Enable, Invert, Blend Mode, Opacity. Its Effect is always
    /// Environment Color.
    pub layer: Layer,
    pub settings: Box<EnvironmentMaterial>,
}

/// An environment item's settings and layers (ENV §2, §5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    /// Multiplies the final colour.
    pub intensity: f64,
    pub visible_to_camera: bool,
    pub visible_to_indirect: bool,
    pub visible_to_reflection: bool,
    pub visible_to_refraction: bool,
    /// Light Path Expression Label: stored only.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lpe_label: String,
    pub material: EnvironmentMaterialLayer,
    /// Texture layers above the Environment Material, top first, like the
    /// Shader Tree's own list (ENV §5). Ids as the material's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Node>,
}

impl Environment {
    /// ENV §2 defaults over the given material layer.
    pub fn new(material: EnvironmentMaterialLayer) -> Self {
        Self {
            intensity: 1.0,
            visible_to_camera: true,
            visible_to_indirect: true,
            visible_to_reflection: true,
            visible_to_refraction: true,
            lpe_label: String::new(),
            material,
            layers: Vec::new(),
        }
    }

    /// Whether rays of `kind` see this environment.
    pub fn visible_to(&self, kind: RayKind) -> bool {
        match kind {
            RayKind::Camera => self.visible_to_camera,
            RayKind::Indirect => self.visible_to_indirect,
            RayKind::Reflection => self.visible_to_reflection,
            RayKind::Refraction => self.visible_to_refraction,
        }
    }

    /// The colour along `direction` (in the item's frame) with Intensity:
    /// the layer stack as far as the CPU draws it (`stack_color`).
    pub fn radiance(&self, direction: DVec3) -> [f64; 3] {
        stack_color(self, direction).map(|c| c * self.intensity)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if !self.intensity.is_finite() {
            return Err("Environment Intensity must be finite".into());
        }
        if self.lpe_label.len() > 1024 || self.lpe_label.chars().any(char::is_control) {
            return Err("Invalid Light Path Expression Label".into());
        }
        let m = &self.material;
        if m.name.trim().is_empty() || m.name.len() > 1024 || m.name.chars().any(char::is_control) {
            return Err("Invalid Environment Material name".into());
        }
        if !(0.0..=1.0).contains(&m.layer.opacity) {
            return Err("Environment Material Opacity must be between 0% and 100%".into());
        }
        m.settings.validate()?;
        self.validate_layers()
    }
}

/// An environment picture as the viewport draws and lights with it
/// (`Scene::environment_sky`).
#[derive(Clone, Debug, PartialEq)]
pub struct EnvironmentSky {
    pub environment: u64,
    pub layer: NodeId,
    /// The picture file.
    pub path: String,
    /// World directions into the picture's frame (`environment_image_uv`).
    pub to_image: glam::DMat3,
    /// The picture is a Light Probe (a mirror-ball photo,
    /// `environment_probe_uv`) rather than latitude-longitude.
    pub light_probe: bool,
    /// Environment Intensity.
    pub intensity: f64,
    /// The layer's Opacity and Invert.
    pub opacity: f64,
    pub invert: bool,
}

/// The ray kinds an environment can be hidden from (ENV §2, §10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RayKind {
    Camera,
    Indirect,
    Reflection,
    Refraction,
}

impl RayKind {
    pub const ALL: [RayKind; 4] = [
        RayKind::Camera,
        RayKind::Indirect,
        RayKind::Reflection,
        RayKind::Refraction,
    ];
}

impl Scene {
    /// The environment holding layer `id` (its material or a texture
    /// layer), and that layer: `None` for the material.
    pub fn environment_layer(&self, id: NodeId) -> Option<(u64, Option<&Node>)> {
        self.items.iter().find_map(|i| {
            let env = i.environment.as_ref()?;
            if env.material.id == id {
                return Some((i.id, None));
            }
            env.layers
                .iter()
                .find(|n| n.id == id)
                .map(|n| (i.id, Some(n)))
        })
    }

    /// Environment items, newest (top-most, ENV §6) first. [chosen] Newer
    /// means later in the item list: a new environment is added at its end.
    pub fn environments(&self) -> impl Iterator<Item = &Item> {
        self.items.iter().rev().filter(|i| i.environment.is_some())
    }

    /// The environment that rays of `kind` see: the newest one visible to
    /// them (ENV §6), or none (black). A hidden environment (its Shader
    /// Tree eye off, ENVIRONMENT-PLACEMENT §3.2) is seen by no ray.
    pub fn environment_for(&self, kind: RayKind) -> Option<&Item> {
        self.environments()
            .find(|i| i.visible && i.environment.as_ref().is_some_and(|e| e.visible_to(kind)))
    }

    /// What a ray of `kind` along world `direction` sees: the winning
    /// environment's colour with Intensity. Black with no environment.
    /// The item has no transform (ENVIRONMENT-PLACEMENT §2), so `direction`
    /// is used as it is; a sky turns on its layers' locators.
    pub fn environment_radiance(&self, kind: RayKind, direction: DVec3) -> [f64; 3] {
        self.environment_for(kind)
            .and_then(|item| item.environment.as_ref())
            .map_or([0.0; 3], |e| e.radiance(direction))
    }

    /// ENVIRONMENT-PLACEMENT §2 for a scene read from a file: an
    /// environment item has no transform and no parent, and nothing is
    /// parented under one. Builds before the placement spec could store
    /// both; they are dropped here, not folded into the layers [chosen]:
    /// the Environment Material's gradients have no placement of their
    /// own to take a rotation, and the image layers' locators may be
    /// shared with other layers, so folding would turn those too. An item
    /// that was parented under an environment is unparented where it
    /// stands.
    pub fn settle_environment_transforms(&mut self) {
        let environments: std::collections::BTreeSet<u64> = self
            .items
            .iter()
            .filter(|i| i.environment.is_some())
            .map(|i| i.id)
            .collect();
        if environments.is_empty() {
            return;
        }
        let children: Vec<u64> = self
            .items
            .iter()
            .filter(|i| i.parent.is_some_and(|p| environments.contains(&p)))
            .map(|i| i.id)
            .collect();
        for child in children {
            if self.set_parent(child, None, true).is_err()
                && let Some(item) = self.item_mut(child)
            {
                item.parent = None;
            }
        }
        for item in &mut self.items {
            if item.environment.is_some() {
                item.parent = None;
                item.transform = Transform::default();
            }
        }
    }

    /// The next free environment layer id: one more than any in use, and
    /// above [`ENVIRONMENT_LAYER_IDS`], so `item.channel … layer:<id>` names
    /// one layer whether it is in the Shader Tree or an environment.
    /// Nothing is stored, so saving and reopening can't change them.
    pub(crate) fn next_layer_id(&self) -> Result<NodeId> {
        let max = self
            .items
            .iter()
            .filter_map(|i| i.environment.as_ref())
            .flat_map(|e| std::iter::once(e.material.id).chain(e.layers.iter().map(|n| n.id)))
            .max()
            .unwrap_or(0)
            .max(ENVIRONMENT_LAYER_IDS);
        max.checked_add(1)
            .ok_or_else(|| "Environment layer ids exhausted".into())
    }

    /// A free Environment Material name: "Environment Material", then
    /// "Environment Material (2)" and so on, in the Item Index Style.
    fn free_material_name(&self) -> String {
        crate::user_defaults::free_name(ENVIRONMENT_MATERIAL_NAME, |name| {
            self.items
                .iter()
                .filter_map(|i| i.environment.as_ref())
                .any(|e| e.material.name == name)
        })
    }

    /// Item > Create Environment (ENV §6): a new "Environment" with its own
    /// Environment Material at the 4 Color Gradient defaults, added at the
    /// end of the item list (so it is the newest). The selection is left
    /// alone.
    pub fn add_environment(&mut self) -> Result<u64> {
        if self.next_id >= u64::MAX - 1 || self.items.len() >= crate::MAX_SCENE_ITEMS {
            return Err("Scene item/identity budget exhausted".into());
        }
        let material = EnvironmentMaterialLayer {
            id: self.next_layer_id()?,
            name: self.free_material_name(),
            layer: Layer::default(),
            settings: Box::default(),
        };
        let name = self.free_name(ENVIRONMENT_NAME);
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(Item {
            id,
            name,
            visible: true,
            transform: Transform::default(),
            mesh: crate::Mesh::default().into(),
            parent: None,
            source_id: None,
            source_display_name: None,
            subdivision: None,
            backdrop: None,
            camera: None,
            light: None,
            texture: None,
            locator: None,
            environment: Some(Box::new(Environment::new(material))),
            common: Default::default(),
            elements: Default::default(),
            hides_children: false,
            mesh_stack: None,
            kit: None,
            instance_of: None,
            replicator: None,
        });
        Ok(id)
    }

    /// For an imported scene or a file from before environments (native
    /// version 33): add the default environment when there is none, so it
    /// keeps a sky (coordinator's call, 2026-11-09; a scene saved with no
    /// environment on purpose keeps none).
    pub fn ensure_environment(&mut self) -> Result<()> {
        if self.environments().next().is_none() {
            self.add_environment()?;
        }
        Ok(())
    }

    /// Give `env` (a copy about to join the scene) fresh layer ids and a
    /// free Environment Material name (ENV §6 Duplicate). Its texture layers
    /// keep sharing their locators [chosen].
    pub fn renumber_environment(&self, env: &mut Environment) -> Result<()> {
        let first = self.next_layer_id()?;
        let mut renumbered = std::collections::BTreeMap::new();
        for (k, node) in env.layers.iter().enumerate() {
            renumbered.insert(node.id, first + 1 + k as NodeId);
        }
        env.material.id = first;
        env.material.name = self.free_material_name();
        for node in &mut env.layers {
            node.id = renumbered[&node.id];
            node.parent = node.parent.and_then(|p| renumbered.get(&p).copied());
        }
        Ok(())
    }

    pub(crate) fn validate_environments(&self) -> Result<()> {
        let mut ids = std::collections::BTreeSet::new();
        for item in &self.items {
            let Some(env) = &item.environment else {
                continue;
            };
            // ENVIRONMENT-PLACEMENT §1/§2: not in the Item List and no
            // transform, so no parent, no children and the default
            // Transform. Every edit is checked here and rolled back, so
            // item.channel, Reset Transform and tool drops refuse alike.
            if item.parent.is_some() {
                return Err(format!("{}: an environment can't have a parent", item.name));
            }
            if self.items.iter().any(|i| i.parent == Some(item.id)) {
                return Err(format!(
                    "{}: items can't be parented to an environment",
                    item.name
                ));
            }
            if item.transform != Transform::default() {
                return Err(NO_TRANSFORM.into());
            }
            if item.camera.is_some()
                || item.light.is_some()
                || item.backdrop.is_some()
                || item.texture.is_some()
                || item.locator.is_some()
                || !item.mesh.vertices.is_empty()
            {
                return Err(format!(
                    "{}: an environment item holds nothing else",
                    item.name
                ));
            }
            env.validate().map_err(|e| format!("{}: {e}", item.name))?;
            for id in std::iter::once(env.material.id).chain(env.layers.iter().map(|n| n.id)) {
                if id <= ENVIRONMENT_LAYER_IDS {
                    return Err(format!(
                        "{}: environment layer id {id} is out of range",
                        item.name
                    ));
                }
                if !ids.insert(id) {
                    return Err(format!("{}: shader layer id {id} is used twice", item.name));
                }
            }
        }
        Ok(())
    }
}

/// Which of the four gradient colours (ENV §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientColor {
    Zenith,
    Sky,
    Ground,
    Nadir,
}

/// One environment property change: the panels' edits and the channel
/// setter's (ENV §11: each is one undo step, named by `label`).
#[derive(Clone, Debug, PartialEq)]
pub enum EnvironmentEdit {
    Intensity(f64),
    Visible(RayKind, bool),
    LpeLabel(String),
    MaterialName(String),
    Enable(bool),
    Invert(bool),
    Blend(crate::shader_tree::BlendMode),
    Opacity(f64),
    Kind(EnvironmentType),
    Color(GradientColor, [f64; 3]),
    SkyExponent(f64),
    GroundExponent(f64),
    SunLight(Option<u64>),
    DiscSize(f64),
    Haze(f64),
    ClampSkyBrightness(bool),
    SkyGamma(f64),
    GroundAlbedo([f64; 3]),
    DiscColor([f64; 3]),
    DiscInScatter(f64),
    FogType(EnvironmentFogType),
    FogStart(f64),
    FogEnd(f64),
    FogDensity(f64),
    FogBaseAltitude(f64),
    FogAltitudeFalloff(f64),
}

impl EnvironmentEdit {
    /// The undo step's name: the property's panel label.
    pub fn label(&self) -> &'static str {
        use EnvironmentEdit::*;
        match self {
            Intensity(_) => "Intensity",
            Visible(RayKind::Camera, _) => "Visible to Camera",
            Visible(RayKind::Indirect, _) => "Visible to Indirect Rays",
            Visible(RayKind::Reflection, _) => "Visible to Reflection Rays",
            Visible(RayKind::Refraction, _) => "Visible to Refraction Rays",
            LpeLabel(_) => "Light Path Expression Label",
            MaterialName(_) => "Rename Layer",
            Enable(_) => "Enable",
            Invert(_) => "Invert",
            Blend(_) => "Blend Mode",
            Opacity(_) => "Opacity",
            Kind(_) => "Environment Type",
            Color(GradientColor::Zenith, _) => "Zenith Color",
            Color(GradientColor::Sky, _) => "Sky Color",
            Color(GradientColor::Ground, _) => "Ground Color",
            Color(GradientColor::Nadir, _) => "Nadir Color",
            SkyExponent(_) => "Sky Exponent",
            GroundExponent(_) => "Ground Exponent",
            SunLight(_) => "Sun Light",
            DiscSize(_) => "Solar Disc Size",
            Haze(_) => "Haze Amount",
            ClampSkyBrightness(_) => "Clamp Sky Brightness",
            SkyGamma(_) => "Sky Gamma",
            GroundAlbedo(_) => "Ground Albedo",
            DiscColor(_) => "Disc Color",
            DiscInScatter(_) => "Disc In-Scatter",
            FogType(_) => "Fog Type",
            FogStart(_) => "Fog Start",
            FogEnd(_) => "Fog End",
            FogDensity(_) => "Fog Density",
            FogBaseAltitude(_) => "Base Altitude",
            FogAltitudeFalloff(_) => "Altitude Falloff",
        }
    }

    /// Apply to `env`, checking the result (an error leaves `env` as it
    /// was).
    pub fn apply(&self, env: &mut Environment) -> Result<()> {
        use EnvironmentEdit::*;
        let mut next = env.clone();
        {
            let m = &mut next.material.settings;
            match self.clone() {
                Intensity(v) => next.intensity = v,
                Visible(RayKind::Camera, v) => next.visible_to_camera = v,
                Visible(RayKind::Indirect, v) => next.visible_to_indirect = v,
                Visible(RayKind::Reflection, v) => next.visible_to_reflection = v,
                Visible(RayKind::Refraction, v) => next.visible_to_refraction = v,
                LpeLabel(v) => next.lpe_label = v,
                MaterialName(v) => next.material.name = v.trim().to_owned(),
                Enable(v) => next.material.layer.enable = v,
                Invert(v) => next.material.layer.invert = v,
                Blend(v) => next.material.layer.blend = v,
                Opacity(v) => next.material.layer.opacity = v,
                Kind(v) => m.kind = v,
                Color(GradientColor::Zenith, v) => m.zenith = v,
                Color(GradientColor::Sky, v) => m.sky = v,
                Color(GradientColor::Ground, v) => m.ground = v,
                Color(GradientColor::Nadir, v) => m.nadir = v,
                SkyExponent(v) => m.sky_exponent = v,
                GroundExponent(v) => m.ground_exponent = v,
                SunLight(v) => m.daylight.sun_light = v,
                DiscSize(v) => m.daylight.disc_size = v,
                Haze(v) => m.daylight.haze = v,
                ClampSkyBrightness(v) => m.daylight.clamp_sky_brightness = v,
                SkyGamma(v) => m.daylight.sky_gamma = v,
                GroundAlbedo(v) => m.daylight.ground_albedo = v,
                DiscColor(v) => m.daylight.disc_color = v,
                DiscInScatter(v) => m.daylight.disc_in_scatter = v,
                FogType(v) => m.fog.fog_type = v,
                FogStart(v) => m.fog.start = v,
                FogEnd(v) => m.fog.end = v,
                FogDensity(v) => m.fog.density = v,
                FogBaseAltitude(v) => m.fog.base_altitude = v,
                FogAltitudeFalloff(v) => m.fog.altitude_falloff = v,
            }
        }
        next.validate()?;
        *env = next;
        Ok(())
    }
}

/// Effects a texture layer under an environment may have (ENV §5):
/// Environment Color first (the default), then the control effects.
pub const ENVIRONMENT_EFFECTS: &[crate::shader_tree::TextureEffect] = {
    use crate::shader_tree::TextureEffect::*;
    &[
        EnvironmentColor,
        DriverA,
        DriverB,
        DriverC,
        DriverD,
        GroupMask,
        LayerMask,
    ]
};

/// The layer types Add Layer puts under an environment (ENV §8): the
/// texture entries. [chosen] Groups and the mesh-only textures (Vertex
/// Map, Variation, Process, Occlusion) wait; Gradient is offered but only
/// its storage works yet.
pub fn environment_can_hold(layer: crate::shader_tree::NewLayer) -> bool {
    use crate::shader_tree::NewLayer as L;
    matches!(
        layer,
        L::Constant | L::ImageMap | L::Procedural(_) | L::Gradient
    )
}

/// The renderer mix that blends an environment layer (ENV §5, "stack like
/// material layers"). Provisional (ENV-Q-OP6): the blends that surfaces
/// build from several mixes (Subtract's clamp, Dodge, Burn, Soft Light,
/// Divide, Hard Light, the normal-map blends) blend as Normal here.
pub fn environment_mix(mode: crate::shader_tree::BlendMode) -> crate::shader_eval::Mix {
    use crate::shader_eval::Mix;
    use crate::shader_tree::BlendMode as B;
    match mode {
        B::Add => Mix::Add,
        B::Multiply => Mix::Multiply,
        B::Screen => Mix::Screen,
        B::Difference => Mix::Difference,
        B::Darken => Mix::Darken,
        B::Lighten => Mix::Lighten,
        B::Overlay => Mix::Overlay,
        _ => Mix::Blend,
    }
}

/// Where Add Layer puts a new environment layer (ENV §8: directly above
/// the selected layer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerPlace {
    /// At the top (the environment row was selected) [chosen].
    Top,
    /// Directly above this texture layer.
    Above(NodeId),
    /// Directly above the Environment Material: the bottom of the list.
    AboveMaterial,
}

/// An edit of the texture layers under an environment (ENV §5, §8), one
/// undo step each.
#[derive(Clone, Debug, PartialEq)]
pub enum EnvironmentLayerEdit {
    /// A new layer at `at`, with its Texture Locator.
    Add {
        layer: crate::shader_tree::NewLayer,
        at: LayerPlace,
    },
    /// Delete layers, and their locators no other layer uses.
    Remove(Vec<NodeId>),
    /// Replace one layer (an edit from its Properties or row).
    Replace(Box<Node>),
}

impl EnvironmentLayerEdit {
    pub fn label(&self) -> &'static str {
        match self {
            EnvironmentLayerEdit::Add { .. } => "Add Layer",
            EnvironmentLayerEdit::Remove(_) => "Delete Layer",
            EnvironmentLayerEdit::Replace(_) => "Edit Layer",
        }
    }
}

impl Environment {
    /// Every Texture Locator its layers use.
    pub fn locators(&self) -> Vec<u64> {
        self.layers
            .iter()
            .filter_map(|n| match &n.kind {
                crate::shader_tree::NodeKind::Texture(t) => t.locator,
                _ => None,
            })
            .collect()
    }

    fn validate_layers(&self) -> Result<()> {
        use crate::shader_tree::NodeKind;
        for node in &self.layers {
            match &node.kind {
                NodeKind::Texture(t) if ENVIRONMENT_EFFECTS.contains(&t.effect) => {}
                NodeKind::Texture(_) => {
                    return Err(format!("{}: not an environment effect", node.name));
                }
                _ => {
                    return Err(format!(
                        "{}: only texture layers sit under an environment",
                        node.name
                    ));
                }
            }
            if node.parent.is_some() {
                return Err(format!(
                    "{}: environment layers aren't nested yet",
                    node.name
                ));
            }
            node.validate()?;
        }
        Ok(())
    }
}

impl Scene {
    /// Apply `edit` to environment `item`'s layers (ENV §5, §8).
    pub fn edit_environment_layers(&mut self, item: u64, edit: EnvironmentLayerEdit) -> Result<()> {
        use crate::shader_tree::NodeKind;
        let env = self
            .item(item)
            .and_then(|i| i.environment.as_ref())
            .ok_or("Select an environment")?
            .clone();
        let mut next_env = env.clone();
        let mut dropped = Vec::new();
        match edit {
            EnvironmentLayerEdit::Add { layer, at } => {
                if !environment_can_hold(layer) {
                    return Err(format!(
                        "{} can't go under an environment yet",
                        layer.label()
                    ));
                }
                let mut kind = layer.kind();
                let NodeKind::Texture(t) = &mut kind else {
                    return Err("Only texture layers go under an environment".into());
                };
                // ENV §5: Environment Color, stored explicitly [chosen].
                t.effect = crate::shader_tree::TextureEffect::EnvironmentColor;
                let base = layer.label();
                let name = crate::user_defaults::free_name(base, |n| {
                    next_env.layers.iter().any(|l| l.name == n)
                });
                let locator = environment_starting_locator(&t.source);
                let id = self.next_layer_id()?;
                let mut next = self.next_id;
                let locator = self.add_texture_locator(&mut next, &name, locator)?;
                self.next_id = next;
                self.start_environment_locator(locator);
                t.locator = Some(locator);
                let node = Node {
                    id,
                    parent: None,
                    name,
                    layer: Layer::default(),
                    kind,
                    extras: Default::default(),
                };
                let index = match at {
                    LayerPlace::Top => 0,
                    LayerPlace::AboveMaterial => next_env.layers.len(),
                    LayerPlace::Above(a) => next_env
                        .layers
                        .iter()
                        .position(|l| l.id == a)
                        .ok_or("No such layer under this environment")?,
                };
                next_env.layers.insert(index, node);
            }
            EnvironmentLayerEdit::Remove(ids) => {
                for id in &ids {
                    if !next_env.layers.iter().any(|l| l.id == *id) {
                        return Err("No such layer under this environment".into());
                    }
                }
                dropped = next_env
                    .layers
                    .iter()
                    .filter(|l| ids.contains(&l.id))
                    .filter_map(|l| match &l.kind {
                        NodeKind::Texture(t) => t.locator,
                        _ => None,
                    })
                    .collect();
                next_env.layers.retain(|l| !ids.contains(&l.id));
            }
            EnvironmentLayerEdit::Replace(node) => {
                let slot = next_env
                    .layers
                    .iter_mut()
                    .find(|l| l.id == node.id)
                    .ok_or("No such layer under this environment")?;
                *slot = *node;
            }
        }
        next_env.validate_layers()?;
        if let Some(e) = self.item_mut(item).and_then(|i| i.environment.as_mut()) {
            *e = next_env;
        }
        self.drop_unused_locators(&dropped);
        Ok(())
    }

    /// Environment `item`'s image layer: its top Image Map layer (the one
    /// Load Image… fills and Rotation turns), if it has one.
    pub fn environment_image_layer(&self, item: u64) -> Option<&Node> {
        use crate::shader_tree::{NodeKind, TextureSource};
        self.item(item)?
            .environment
            .as_ref()?
            .layers
            .iter()
            .find(|n| {
                matches!(&n.kind, NodeKind::Texture(t)
                    if matches!(t.source, TextureSource::ImageMap { .. }))
            })
    }

    /// The picture environment `item`'s image layer shows, if it has one.
    pub fn environment_image(&self, item: u64) -> Option<&crate::backdrop::SceneImage> {
        use crate::shader_tree::{NodeKind, TextureSource};
        match &self.environment_image_layer(item)?.kind {
            NodeKind::Texture(t) => match &t.source {
                TextureSource::ImageMap {
                    image: Some(id), ..
                } => self.image(*id),
                _ => None,
            },
            _ => None,
        }
    }

    /// The Texture Locator of environment `item`'s image layer and its
    /// heading: the rotation about +Y in radians, which turns the picture
    /// on the sky (positive turns it from +Z toward +X, the same in the
    /// viewport and in Cycles: `environment_image_uv`).
    pub fn environment_image_heading(&self, item: u64) -> Option<(u64, f64)> {
        use crate::shader_tree::NodeKind;
        let NodeKind::Texture(t) = &self.environment_image_layer(item)?.kind else {
            return None;
        };
        let locator = t.locator?;
        let l = self.item(locator)?;
        Some((locator, l.transform.rotation.y))
    }

    /// World directions into the frame of texture layers placed by
    /// `locator`: its world rotation undone (scale ignored). Identity for a
    /// missing or degenerate locator. Cycles and the viewport both use it
    /// for environment pictures.
    ///
    /// The locator's Projection Axis is the picture's pole (Y, the
    /// default, keeps the picture upright). Position, Size and the other
    /// projection settings don't move a picture at infinity. Light Probe
    /// reads the picture as a mirror ball (`environment_probe_uv`); every
    /// other Projection Type reads it as latitude-longitude.
    pub fn locator_to_image(&self, locator: u64) -> glam::DMat3 {
        let undo = self
            .world_matrix(locator)
            .ok()
            .map(glam::DMat3::from_mat4)
            .and_then(|m| {
                let cols = [m.x_axis, m.y_axis, m.z_axis].map(|c| c.normalize_or_zero());
                let r = glam::DMat3::from_cols(cols[0], cols[1], cols[2]);
                (r.determinant().abs() > 1e-9).then(|| r.inverse())
            })
            .unwrap_or(glam::DMat3::IDENTITY);
        let axis = self.texture_locator(locator).map_or(1, |s| s.axis);
        pole_frame(axis) * undo
    }

    /// A new environment layer's locator item turns in ZXY order
    /// (env-texture-locator brief, 2026-11-13).
    fn start_environment_locator(&mut self, locator: u64) {
        if let Some(item) = self.item_mut(locator) {
            item.transform
                .set_rotation_order(crate::scene::RotationOrder::Zxy);
        }
    }

    /// Give environment texture layers without a Texture Locator one, at
    /// identity (older files and layers written elsewhere). Returns
    /// whether anything changed.
    pub fn give_environment_layers_locators(&mut self, next: &mut u64) -> Result<bool> {
        use crate::shader_tree::NodeKind;
        let missing: Vec<(u64, NodeId, String, crate::shader_tree::TextureSource)> = self
            .items
            .iter()
            .filter_map(|i| Some((i.id, i.environment.as_ref()?)))
            .flat_map(|(item, env)| {
                env.layers.iter().filter_map(move |n| match &n.kind {
                    NodeKind::Texture(t) if t.locator.is_none() => {
                        Some((item, n.id, n.name.clone(), t.source.clone()))
                    }
                    _ => None,
                })
            })
            .collect();
        for (item, layer, name, source) in &missing {
            let locator =
                self.add_texture_locator(next, name, environment_starting_locator(source))?;
            self.start_environment_locator(locator);
            if let Some(node) = self
                .item_mut(*item)
                .and_then(|i| i.environment.as_mut())
                .and_then(|e| e.layers.iter_mut().find(|n| n.id == *layer))
                && let NodeKind::Texture(t) = &mut node.kind
            {
                t.locator = Some(locator);
            }
        }
        Ok(!missing.is_empty())
    }

    /// The Texture Locators of every environment's Image Map layers.
    pub(crate) fn environment_image_locators(&self) -> Vec<u64> {
        use crate::shader_tree::{NodeKind, TextureSource};
        self.items
            .iter()
            .filter_map(|i| i.environment.as_ref())
            .flat_map(|e| e.layers.iter())
            .filter_map(|n| match &n.kind {
                NodeKind::Texture(t) if matches!(t.source, TextureSource::ImageMap { .. }) => {
                    t.locator
                }
                _ => None,
            })
            .collect()
    }

    /// Before version 44: an environment image layer's locator set to
    /// Light Probe goes back to Spherical. Light Probe drew as
    /// latitude-longitude until v44 (and was this branch's unreleased
    /// default), so Spherical keeps the picture where it was.
    pub(crate) fn environment_light_probes_to_spherical(&mut self) {
        use crate::shader_tree::Projection;
        let image_locators = self.environment_image_locators();
        for item in self
            .items
            .iter_mut()
            .filter(|i| image_locators.contains(&i.id))
        {
            if let Some(crate::texture_locator::TextureItem::Locator(s)) = &mut item.texture
                && s.projection == Projection::LightProbe
            {
                s.projection = Projection::Spherical;
            }
        }
    }

    /// The picture rays of `kind` see behind everything, as the viewport
    /// draws it: the top enabled Environment Color Image Map layer (with a
    /// picture file) of the environment visible to them.
    pub fn environment_sky(&self, kind: RayKind) -> Option<EnvironmentSky> {
        use crate::shader_tree::{NodeKind, TextureEffect, TextureSource};
        let item = self.environment_for(kind)?;
        let env = item.environment.as_ref()?;
        env.layers.iter().find_map(|node| {
            let NodeKind::Texture(t) = &node.kind else {
                return None;
            };
            if !node.layer.enable || t.effect != TextureEffect::EnvironmentColor {
                return None;
            }
            let TextureSource::ImageMap {
                image: Some(id), ..
            } = &t.source
            else {
                return None;
            };
            let image = self.image(*id).filter(|i| !i.path.is_empty())?;
            let light_probe = t
                .locator
                .and_then(|l| self.texture_locator(l))
                .is_some_and(|l| l.projection == crate::shader_tree::Projection::LightProbe);
            Some(EnvironmentSky {
                environment: item.id,
                layer: node.id,
                path: image.path.clone(),
                to_image: t
                    .locator
                    .map_or(glam::DMat3::IDENTITY, |l| self.locator_to_image(l)),
                light_probe,
                intensity: env.intensity,
                opacity: node.layer.opacity,
                invert: node.layer.invert,
            })
        })
    }

    /// The Texture Locators the layers of environments `ids` use.
    pub fn environment_locators(&self, ids: &[u64]) -> Vec<u64> {
        self.items
            .iter()
            .filter(|i| ids.contains(&i.id))
            .filter_map(|i| i.environment.as_ref())
            .flat_map(|e| e.locators())
            .collect()
    }

    /// Remove the locators of deleted environments' layers that nothing
    /// else uses (ENV §6: deleting an environment deletes its layers and
    /// their locators).
    pub fn drop_environment_locators(&mut self, candidates: &[u64]) {
        self.drop_unused_locators(candidates);
    }
}

/// The colour of environment layer stack `env` along `local` (the item's
/// frame) as far as the CPU can tell: the Environment Material, then each
/// enabled Constant layer blended over it (ENV §5). Image Map and
/// procedural layers are drawn by the renderer only; here they leave the
/// colour as it was.
pub fn stack_color(env: &Environment, local: DVec3) -> [f64; 3] {
    use crate::shader_tree::{NodeKind, TextureEffect, TextureSource};
    let mut c = if env.material.layer.enable {
        // [chosen] The material blends over black at its Opacity.
        let m = env.material.settings.color(local);
        let m = if env.material.layer.invert {
            m.map(|v| 1.0 - v)
        } else {
            m
        };
        m.map(|v| v * env.material.layer.opacity)
    } else {
        [0.0; 3]
    };
    for node in env.layers.iter().rev() {
        let NodeKind::Texture(t) = &node.kind else {
            continue;
        };
        if !node.layer.enable || t.effect != TextureEffect::EnvironmentColor {
            continue;
        }
        let TextureSource::Constant { color, .. } = &t.source else {
            continue;
        };
        let layer = if node.layer.invert {
            color.map(|v| 1.0 - v)
        } else {
            *color
        };
        let mix = environment_mix(node.layer.blend);
        c = [0, 1, 2].map(|i| mix.apply(c[i], layer[i], node.layer.opacity));
    }
    c
}

/// The starting locator settings of a new environment layer (ENV §5):
/// Spherical (latitude-longitude) about Y for an Image Map (ETL-Q02:
/// Light Probe is a mirror-ball photo), Solid otherwise [chosen].
pub fn environment_starting_locator(
    source: &crate::shader_tree::TextureSource,
) -> crate::texture_locator::TextureLocator {
    use crate::shader_tree::{Projection, TextureSource};
    match source {
        TextureSource::ImageMap { .. } => crate::texture_locator::TextureLocator {
            axis: 1,
            ..crate::texture_locator::TextureLocator::new(Projection::Spherical)
        },
        _ => crate::texture_locator::TextureLocator::new(Projection::Solid),
    }
}

/// Locator-frame directions into the picture's frame (+Y the pole) for
/// Projection Axis `axis` (0, 1, 2 = X, Y, Z): the axis becomes the pole by
/// a cyclic swap of the axes, a proper rotation, so the picture isn't
/// mirrored.
pub fn pole_frame(axis: u8) -> glam::DMat3 {
    match axis {
        // Image (x, y, z) = locator (z, x, y): +X is the pole.
        0 => glam::DMat3::from_cols(DVec3::Y, DVec3::Z, DVec3::X),
        // Image (x, y, z) = locator (y, z, x): +Z is the pole.
        2 => glam::DMat3::from_cols(DVec3::Z, DVec3::X, DVec3::Y),
        _ => glam::DMat3::IDENTITY,
    }
}

/// Where world direction `direction` lands on a latitude-longitude picture
/// of an environment image layer: (u, v) in 0..1, u from the left edge, v
/// from the TOP row. `to_image` takes world directions into the image's
/// frame (the layer's Texture Locator rotation undone; +Y the pole). u =
/// 0.5 looks along +Z and grows toward -X, so turning right (looking down
/// -Z, from -Z toward +X) moves right across the picture and its text
/// reads the right way round. Cycles' equirectangular Environment Texture
/// reads the same pixel (operandi-cycles maps the frame with `to_cycles`);
/// the viewport's sky shader (environment_gpu.wgsl) uses the same formula.
pub fn environment_image_uv(to_image: glam::DMat3, direction: DVec3) -> [f64; 2] {
    let d = (to_image * direction).normalize_or_zero();
    let u = 0.5 - d.x.atan2(d.z) / std::f64::consts::TAU;
    let v = 0.5 - d.y.clamp(-1.0, 1.0).asin() / std::f64::consts::PI;
    [u.rem_euclid(1.0), v]
}

/// Where world direction `direction` lands on a Light Probe picture: a
/// photo of a mirror ball filling the square image, taken from far away
/// along -Z in the image's frame (`to_image`, as for
/// `environment_image_uv`), +Y up and +X to the right. (u, v) in 0..1, u
/// from the left edge, v from the TOP row. The ball's centre reflects +Z
/// (back toward the camera); its rim is -Z. A true reflection: the ball's
/// normal halves the angle between the view ray and `direction`
/// (ETL-Q02b: mirror ball, not the angular map). Cycles' Mirror Ball
/// Environment Texture reads the same pixel (operandi-cycles
/// `probe_frame_to_cycles`), and the viewport unwraps the picture with this
/// before drawing it.
pub fn environment_probe_uv(to_image: glam::DMat3, direction: DVec3) -> [f64; 2] {
    let d = (to_image * direction).normalize_or_zero();
    // The normal is along (the reflected ray) - (the view ray, -Z).
    let n = (d + DVec3::Z).normalize_or_zero();
    [0.5 + 0.5 * n.x, 0.5 - 0.5 * n.y]
}

#[cfg(test)]
mod image_tests {
    use super::*;

    /// The image's centre (u 0.5, v 0.5) looks along +Z; the top row is
    /// straight up; u grows from +Z toward -X, so the picture isn't
    /// mirrored: looking down -Z, +X (on the right) is right of -Z on it.
    #[test]
    fn image_uv_frame() {
        let id = glam::DMat3::IDENTITY;
        let near =
            |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9;
        assert!(near(environment_image_uv(id, DVec3::Z), [0.5, 0.5]));
        assert!(near(environment_image_uv(id, -DVec3::X), [0.75, 0.5]));
        assert!(near(environment_image_uv(id, DVec3::X), [0.25, 0.5]));
        // Looking down -Z: a little right of ahead is a little right on
        // the picture (u wraps from 1 to 0 at -Z).
        let ahead = environment_image_uv(id, -DVec3::Z)[0];
        let right = environment_image_uv(id, DVec3::new(0.1, 0.0, -1.0))[0];
        assert!(ahead > 0.99 || ahead < 1e-9);
        assert!(right > 0.0 && right < 0.1, "{right}");
        assert!(environment_image_uv(id, DVec3::Y)[1].abs() < 1e-9);
        assert!((environment_image_uv(id, -DVec3::Y)[1] - 1.0).abs() < 1e-9);
    }

    /// Rotation's sign: a positive heading turns the picture from +Z toward
    /// +X, so what was straight ahead (+Z) is seen along +X after +90°.
    /// `to_image` is the locator's rotation undone, as Cycles builds it
    /// (operandi-cycles `EnvironmentLayer::of`).
    #[test]
    fn positive_heading_turns_the_picture_from_z_toward_x() {
        let turned = glam::DMat3::from_rotation_y(90f64.to_radians());
        let to_image = turned.inverse();
        let ahead = environment_image_uv(glam::DMat3::IDENTITY, DVec3::Z);
        let seen = environment_image_uv(to_image, DVec3::X);
        assert!((ahead[0] - seen[0]).abs() < 1e-9 && (ahead[1] - seen[1]).abs() < 1e-9);
    }

    /// Light Probe: the ball's centre reflects +Z, its top +Y, its right
    /// edge +X, and directions just short of -Z land near the rim.
    #[test]
    fn probe_uv_is_a_mirror_ball() {
        let id = glam::DMat3::IDENTITY;
        let near =
            |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9;
        assert!(near(environment_probe_uv(id, DVec3::Z), [0.5, 0.5]));
        let h = 0.5 + 0.5 * std::f64::consts::FRAC_1_SQRT_2;
        // 90° off the view axis reflects off the normal at 45°.
        assert!(near(environment_probe_uv(id, DVec3::X), [h, 0.5]));
        assert!(near(environment_probe_uv(id, DVec3::Y), [0.5, 1.0 - h]));
        let rim = environment_probe_uv(id, DVec3::new(0.0, 1e-4, -1.0));
        let r = ((rim[0] - 0.5).powi(2) + (rim[1] - 0.5).powi(2)).sqrt();
        assert!((r - 0.5).abs() < 1e-3, "{rim:?}");
    }
}
