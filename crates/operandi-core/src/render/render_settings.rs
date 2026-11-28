//! The scene's final-render settings: the Render item at the top of the
//! Shader Tree (render item spec v1, RI-xx). Its Frame tab holds the
//! picture size; its Settings tab the sampling, ray depth and caustics.
//! Exposure, gamma, tone mapping and Denoising belong to the Final Color
//! Render Output (render_outputs.rs, RI-Q05). A render camera with
//! Resolution Override on wins over the frame size here.
//!
//! Old scenes must render as they did, so `Default` is what a file
//! without a row meant before the row existed (Cycles' own values), and
//! `RenderSettings::new_scene` holds what a new scene starts with.
use serde::{Deserialize, Serialize};

/// Samples an old scene without a samples row renders with (the
/// renderer's earlier fixed value).
pub const DEFAULT_SAMPLES: u32 = 16;
/// Maximum Quality a new scene starts with (RI-Q03: 64, read off the
/// reference's path-tracer panel; 16 is the old value).
pub const NEW_SCENE_SAMPLES: u32 = 64;
/// Noise Threshold a new scene starts with, as a fraction (10 %, RI-Q04).
pub const NEW_SCENE_NOISE_THRESHOLD: f64 = 0.1;
/// Most samples a render may ask for (the panel stops at
/// `PANEL_MAX_SAMPLES`; older scenes and scripts may go higher).
pub const MAX_SAMPLES: u32 = 1 << 20;
/// Maximum Quality's panel range (RI-S03).
pub const PANEL_MAX_SAMPLES: u32 = 65536;
/// Largest Frame Width or Height (RI-F03).
pub const MAX_FRAME: u32 = 16384;
/// Largest Diffuse, Reflection or Refraction Depth (RI-S06..S08).
pub const MAX_DEPTH: u32 = 128;
/// Largest Preview Max Render Time, minutes.
pub const MAX_PREVIEW_MINUTES: f64 = 10_000.0;
/// Cycles' bounce limits: what an old scene, saved before the ray depth
/// rows, rendered with.
pub const LEGACY_DEPTH: u32 = 7;

/// RI-S11: which focused light (caustics) bounced rays trace.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Caustics {
    None,
    ReflectionOnly,
    RefractionOnly,
    #[default]
    Both,
}

impl Caustics {
    pub const ALL: [Self; 4] = [
        Self::None,
        Self::ReflectionOnly,
        Self::RefractionOnly,
        Self::Both,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::ReflectionOnly => "Reflection Only",
            Self::RefractionOnly => "Refraction Only",
            Self::Both => "Both",
        }
    }
    /// Light focused by mirrors and glossy surfaces is traced.
    pub fn reflective(self) -> bool {
        matches!(self, Self::ReflectionOnly | Self::Both)
    }
    /// Light focused by glass and liquids is traced.
    pub fn refractive(self) -> bool {
        matches!(self, Self::RefractionOnly | Self::Both)
    }
}

/// Total Photons a Direct Caustics section starts with (Direct Caustics
/// spec v1).
pub const DEFAULT_TOTAL_PHOTONS: u32 = 100_000;
/// Local Photons it starts with.
pub const DEFAULT_LOCAL_PHOTONS: u32 = 32;
/// Most Total Photons the panel takes.
pub const MAX_TOTAL_PHOTONS: u32 = 100_000_000;
/// Most Local Photons the panel takes.
pub const MAX_LOCAL_PHOTONS: u32 = 10_000;

/// The Global Illumination tab's Caustics section (Direct Caustics spec
/// v1): caustics cast by the scene's own lights through glass. Operandi
/// renders them with Cycles' manifold next event estimation, not photons,
/// so Total Photons, Local Photons and Walkthrough Mode are stored only
/// (provisional, DC-Q02).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DirectCaustics {
    /// Direct Caustics: lights with Affect Caustics above 0 cast caustics
    /// through see-through Glass onto the other surfaces.
    pub enabled: bool,
    pub total_photons: u32,
    pub local_photons: u32,
    pub walkthrough: bool,
}

impl Default for DirectCaustics {
    fn default() -> Self {
        Self {
            enabled: false,
            total_photons: DEFAULT_TOTAL_PHOTONS,
            local_photons: DEFAULT_LOCAL_PHOTONS,
            walkthrough: false,
        }
    }
}

impl DirectCaustics {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RenderSettings {
    /// Maximum Quality: samples per pixel at most (RI-S03).
    pub samples: u32,
    /// Denoise the finished picture: the Final Color output's Denoising
    /// row (RI-Q05, `Scene::render_denoise`).
    pub denoise: bool,
    /// Noise Threshold as a fraction (RI-S04, shown as a percent): a pixel
    /// stops sampling once its noise is below it. 0 turns adaptive
    /// sampling off (provisional, RI-B01: keeps old scenes as they were).
    pub adaptive_threshold: f64,
    /// Frame Width and Height in pixels (RI-F03).
    pub frame: [u32; 2],
    /// Diffuse, Reflection and Refraction Depth (RI-S06..S08).
    pub diffuse_depth: u32,
    pub reflection_depth: u32,
    pub refraction_depth: u32,
    /// Maximum Radiance (RI-S10): bounced light brighter than this is
    /// clamped; 0 is off. Direct light is never clamped.
    pub max_radiance: f64,
    /// Indirect Caustics (RI-S11).
    pub caustics: Caustics,
    /// Direct Caustics and its photon rows (native version 46; left out
    /// of files while they are the defaults).
    #[serde(skip_serializing_if = "DirectCaustics::is_default")]
    pub direct_caustics: DirectCaustics,
    /// Preview tab, Convergence Target (RI-P01) as a fraction: the Preview
    /// stops at this share of its sample target. Never used by F9.
    pub preview_convergence: f64,
    /// Preview tab, Max Render Time (RI-P02) in minutes; 0 is no limit.
    /// Never used by F9.
    pub preview_time_limit: f64,
    /// Bloom and Glare, applied to the finished picture (the Render
    /// window's Image Processing tab; post_effects.rs). Native version 68;
    /// left out of files while off at their defaults.
    #[serde(skip_serializing_if = "crate::post_effects::PostEffects::is_default")]
    pub post: crate::post_effects::PostEffects,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            samples: DEFAULT_SAMPLES,
            denoise: true,
            adaptive_threshold: 0.0,
            frame: crate::camera_light_defaults::RENDER_FRAME,
            diffuse_depth: LEGACY_DEPTH,
            reflection_depth: LEGACY_DEPTH,
            refraction_depth: LEGACY_DEPTH,
            // Cycles' own indirect clamp, so old scenes are unchanged.
            max_radiance: 10.0,
            caustics: Caustics::Both,
            direct_caustics: DirectCaustics::default(),
            // Older scenes' Preview ran to its full sample target with no
            // time limit.
            preview_convergence: 1.0,
            preview_time_limit: 0.0,
            post: Default::default(),
        }
    }
}

impl RenderSettings {
    /// What a new scene starts with (RI-05, RI-Q03, RI-Q04, RI-Q06).
    pub fn new_scene() -> Self {
        Self {
            samples: NEW_SCENE_SAMPLES,
            adaptive_threshold: NEW_SCENE_NOISE_THRESHOLD,
            diffuse_depth: 8,
            reflection_depth: 4,
            refraction_depth: 8,
            // Render fix F5: reflection caustics from bounced light add
            // noise for little gain; older scenes keep what they saved.
            caustics: Caustics::RefractionOnly,
            preview_convergence: 0.975,
            preview_time_limit: 5.0,
            ..Self::default()
        }
    }

    pub fn validate(&self) -> crate::Result<()> {
        if !(1..=MAX_SAMPLES).contains(&self.samples) {
            return Err(format!(
                "Maximum Quality must be 1 to {MAX_SAMPLES} samples"
            ));
        }
        if !(self.adaptive_threshold.is_finite() && (0.0..=1.0).contains(&self.adaptive_threshold))
        {
            return Err("Noise Threshold must be 0 to 100 %".into());
        }
        if self.frame.iter().any(|&s| !(1..=MAX_FRAME).contains(&s)) {
            return Err(format!(
                "Frame Width and Height must be 1 to {MAX_FRAME} pixels"
            ));
        }
        if [
            self.diffuse_depth,
            self.reflection_depth,
            self.refraction_depth,
        ]
        .iter()
        .any(|&d| d > MAX_DEPTH)
        {
            return Err(format!("Ray depths must be 0 to {MAX_DEPTH}"));
        }
        if !(self.max_radiance.is_finite() && self.max_radiance >= 0.0) {
            return Err("Maximum Radiance must be 0 or more".into());
        }
        let dc = &self.direct_caustics;
        if !(1..=MAX_TOTAL_PHOTONS).contains(&dc.total_photons) {
            return Err(format!("Total Photons must be 1 to {MAX_TOTAL_PHOTONS}"));
        }
        if !(1..=MAX_LOCAL_PHOTONS).contains(&dc.local_photons) {
            return Err(format!("Local Photons must be 1 to {MAX_LOCAL_PHOTONS}"));
        }
        if !(self.preview_convergence.is_finite()
            && (0.0..=1.0).contains(&self.preview_convergence))
        {
            return Err("Convergence Target must be 0 to 100 %".into());
        }
        if !(self.preview_time_limit.is_finite()
            && (0.0..=MAX_PREVIEW_MINUTES).contains(&self.preview_time_limit))
        {
            return Err(format!(
                "Max Render Time must be 0 to {MAX_PREVIEW_MINUTES} minutes"
            ));
        }
        self.post.validate()
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Whether a row newer than native version 27 (Samples, Denoise and
    /// Noise Threshold) leaves its old-file value: such a scene needs
    /// version 36.
    pub(crate) fn has_render_item_rows(&self) -> bool {
        let old_rows = Self {
            samples: self.samples,
            denoise: self.denoise,
            adaptive_threshold: self.adaptive_threshold,
            direct_caustics: self.direct_caustics.clone(),
            ..Self::default()
        };
        *self != old_rows
    }

    /// The Preview's sample target for `samples` (its own Samples setting):
    /// Convergence Target's share of it, at least 1 (RI-P01).
    pub fn preview_samples(&self, samples: u32) -> u32 {
        ((f64::from(samples) * self.preview_convergence).ceil() as u32).clamp(1, samples.max(1))
    }

    /// The Preview's time limit in seconds; 0 is none (RI-P02).
    pub fn preview_seconds(&self) -> f64 {
        self.preview_time_limit * 60.0
    }

    /// The most bounces of any kind a path may take (Cycles' total
    /// bounce limit): the largest of the three depths, so no single limit
    /// cuts a kind short (RI-B02: the spec's "+ 1" left out so old scenes
    /// keep Cycles' 7).
    pub fn max_bounces(&self) -> u32 {
        self.diffuse_depth
            .max(self.reflection_depth)
            .max(self.refraction_depth)
    }
}

impl crate::Scene {
    /// Whether the final render denoises. The Final Color output's
    /// Denoising row shows and sets this one switch (RI-Q05, provisional
    /// RI-B03): every Final Color output agrees, and older scenes keep it.
    pub fn render_denoise(&self) -> bool {
        self.render.denoise
    }

    /// What every new scene (and a scene made from an imported file)
    /// starts with (RI-05).
    pub fn use_new_scene_render_settings(&mut self) {
        self.render = RenderSettings::new_scene();
    }
}

impl crate::Editor {
    /// `Command::SetRenderSettings`: one "Render Settings" undo step.
    pub(crate) fn set_render_settings(&mut self, settings: RenderSettings) -> crate::Result<()> {
        settings.validate()?;
        if self.scene.render == settings {
            return Ok(());
        }
        let mut candidate = self.scene.clone();
        candidate.render = settings;
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record("Render Settings".into(), before);
        Ok(())
    }
}
