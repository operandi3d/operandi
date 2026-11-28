//! Render Outputs: per-output settings and image processing (render
//! outputs v1, 2026-11-07).
//!
//! An output layer's Effect is its type (`shader_tree::OutputEffect`); its
//! settings live in `ShaderTree::outputs` by layer id. The renderer gives
//! each output a float RGBA picture; [`process`] then applies the output's
//! image processing, in RQ-1's order.
use crate::shader_tree::{Channel, NodeId, NodeKind, OutputEffect, ShaderTree};
use serde::{Deserialize, Serialize};

macro_rules! choice {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const ALL: &[$name] = &[$($name::$variant),+];
            pub fn label(self) -> &'static str {
                match self { $($name::$variant => $label),+ }
            }
        }
    };
}

choice! {
    /// Denoising: OIDN is the one Operandi has; the others are stored.
    Denoiser {
        None => "None",
        Oidn => "Intel Open Image Denoise",
        Optix => "OptiX",
        Amd => "AMD",
    }
}
choice! {
    ExposureControl {
        WhiteLevel => "White Level",
        Photographic => "Photographic",
    }
}
choice! {
    ToneMap {
        ReinhardRgb => "Reinhard RGB",
        ReinhardLuminance => "Reinhard Luminance",
    }
}
choice! {
    /// Which ids a Cryptomatte output ranks (Cycles' cryptomatte passes).
    CryptomatteType {
        Object => "Object",
        Material => "Material",
        Asset => "Asset",
    }
}
// The enums come from a macro, so the defaults are spelled out.
#[allow(clippy::derivable_impls)]
impl Default for CryptomatteType {
    fn default() -> Self {
        Self::Object
    }
}
impl CryptomatteType {
    /// The standard layer name prefix: "CryptoObject", "CryptoMaterial",
    /// "CryptoAsset" (followed by 00, 01, ... in a file).
    pub fn layer_name(self) -> &'static str {
        match self {
            Self::Object => "CryptoObject",
            Self::Material => "CryptoMaterial",
            Self::Asset => "CryptoAsset",
        }
    }
    fn is_default(&self) -> bool {
        *self == Self::Object
    }
}

choice! {
    /// The numbers an output writes (group C, 2026-11-20). Default: the
    /// output's kind and the file format decide (`writes_srgb`). There is
    /// no OCIO config yet, so only these three (RO-Q13).
    OutputColorspace {
        Default => "Default",
        Linear => "Linear",
        Srgb => "sRGB",
    }
}
#[allow(clippy::derivable_impls)]
impl Default for OutputColorspace {
    fn default() -> Self {
        Self::Default
    }
}
impl OutputColorspace {
    fn is_default(&self) -> bool {
        *self == Self::Default
    }
}

/// Cryptomatte ranks this many ids per pixel unless set (Cycles' depth).
pub const CRYPTOMATTE_LEVELS: u32 = 6;
fn is_default_levels(levels: &u32) -> bool {
    *levels == CRYPTOMATTE_LEVELS
}

choice! {
    /// Final Color contours: stored only.
    Contours {
        None => "None",
        SurfaceBoundaries => "Surface Boundaries",
        SegmentBoundaries => "Segment Boundaries",
    }
}

/// One Render Output's settings, with the measured starting values.
/// Fractions are 0..1, distances metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OutputSettings {
    pub filename: String,
    pub format: String,
    pub antialiasing: bool,
    pub image_processing: bool,
    pub denoiser: Denoiser,
    pub exposure: ExposureControl,
    pub white_level: f64,
    pub iso: f64,
    pub tone_map: ToneMap,
    pub tone_amount: f64,
    pub gamma: f64,
    pub clamp: bool,
    pub clamp_threshold: f64,
    pub unpremultiply: bool,
    /// Old files only: the output's own Bloom, removed (Jason, BG-Q2:
    /// Bloom lives in the Render window's Image Processing tab,
    /// `RenderSettings::post`). Read so those files still open, moved
    /// into the scene's Bloom on load (`Scene::move_old_output_bloom`),
    /// never written and never applied here.
    #[serde(skip_serializing)]
    pub bloom: bool,
    #[serde(skip_serializing)]
    pub bloom_threshold: f64,
    /// Fraction of the picture's diagonal.
    #[serde(skip_serializing)]
    pub bloom_radius: f64,
    pub vignette: f64,
    pub input_black: f64,
    pub input_gray: f64,
    pub input_white: f64,
    /// Degrees.
    pub hue_offset: f64,
    pub saturation: f64,
    pub colorization: f64,
    pub target_color: [f64; 3],
    pub output_black: f64,
    pub output_white: f64,
    /// Depth / Motion Vector: remap to white..black over the maximum.
    pub remap: bool,
    pub max_depth: f64,
    pub max_motion: f64,
    /// Ambient Occlusion output.
    pub occlusion_rays: u32,
    /// 0 = no limit (RQ-2).
    pub occlusion_range: f64,
    pub contours: Contours,
    pub normalize: bool,
    // Native v52. Files leave these out at their starting values, so a
    // scene that doesn't use them still opens in older builds.
    #[serde(skip_serializing_if = "OutputColorspace::is_default")]
    pub colorspace: OutputColorspace,
    /// Cryptomatte output only.
    #[serde(skip_serializing_if = "CryptomatteType::is_default")]
    pub cryptomatte_type: CryptomatteType,
    /// Cryptomatte output only: ids ranked per pixel, 2..16, even.
    #[serde(skip_serializing_if = "is_default_levels")]
    pub cryptomatte_levels: u32,
    /// Light Path Expressions output only (group D, native v55): the
    /// expression (`light_path_expressions::parse`); empty renders black.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lpe: String,
}
impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            filename: String::new(),
            format: String::new(),
            antialiasing: true,
            image_processing: true,
            denoiser: Denoiser::None,
            exposure: ExposureControl::WhiteLevel,
            white_level: 1.,
            iso: 100.,
            tone_map: ToneMap::ReinhardRgb,
            tone_amount: 0.,
            gamma: 1.,
            clamp: false,
            clamp_threshold: 1.,
            unpremultiply: false,
            bloom: false,
            bloom_threshold: 1.,
            bloom_radius: 0.02,
            vignette: 0.,
            input_black: 0.,
            input_gray: 1.,
            input_white: 1.,
            hue_offset: 0.,
            saturation: 1.,
            colorization: 0.,
            target_color: [1.; 3],
            output_black: 0.,
            output_white: 1.,
            remap: false,
            max_depth: 10.,
            max_motion: 32.,
            occlusion_rays: 64,
            occlusion_range: 0.,
            contours: Contours::None,
            normalize: false,
            colorspace: OutputColorspace::Default,
            cryptomatte_type: CryptomatteType::Object,
            cryptomatte_levels: CRYPTOMATTE_LEVELS,
            lpe: String::new(),
        }
    }
}

impl OutputSettings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
    /// The panel's numbers and switches (menus are drawn by the panel).
    pub fn channels(&mut self) -> Vec<(&'static str, Channel<'_>)> {
        use Channel::*;
        vec![
            ("Antialiasing", Bool(&mut self.antialiasing)),
            ("Image Processing", Bool(&mut self.image_processing)),
            ("White Level", Number(&mut self.white_level, 1e-6, 1e6)),
            ("Film Speed (ISO)", Number(&mut self.iso, 1., 1e6)),
            ("Tone Map Amount", Percent(&mut self.tone_amount)),
            ("Gamma", Number(&mut self.gamma, 0.01, 10.)),
            ("Clamp Colors", Bool(&mut self.clamp)),
            (
                "Clamp Threshold",
                PercentIn(&mut self.clamp_threshold, 0., 100.),
            ),
            ("Unpremultiply Colors", Bool(&mut self.unpremultiply)),
            ("Vignette Amount", Percent(&mut self.vignette)),
            (
                "Input Black Level",
                Number(&mut self.input_black, -10., 10.),
            ),
            ("Input Gray Level", Number(&mut self.input_gray, 0.01, 10.)),
            (
                "Input White Level",
                Number(&mut self.input_white, -10., 10.),
            ),
            ("Hue Offset", Number(&mut self.hue_offset, -360., 360.)),
            ("Saturation", PercentIn(&mut self.saturation, 0., 10.)),
            ("Colorization", Percent(&mut self.colorization)),
            ("Target Color", Color(&mut self.target_color)),
            (
                "Output Black Level",
                PercentIn(&mut self.output_black, -10., 10.),
            ),
            (
                "Output White Level",
                PercentIn(&mut self.output_white, -10., 10.),
            ),
            ("Remap Pixel Values", Bool(&mut self.remap)),
            ("Maximum Depth", Distance(&mut self.max_depth)),
            ("Maximum Motion", Number(&mut self.max_motion, 0., 1e6)),
            ("Occlusion Range", Distance(&mut self.occlusion_range)),
            ("Normalize", Bool(&mut self.normalize)),
        ]
    }
    pub fn valid(&mut self) -> bool {
        self.channels().iter().all(|(_, c)| c.valid())
            && (1..=4096).contains(&self.occlusion_rays)
            && self.input_white != self.input_black
            && (2..=16).contains(&self.cryptomatte_levels)
            && self.cryptomatte_levels.is_multiple_of(2)
            && self.lpe.len() <= 1024
            && !self.lpe.chars().any(char::is_control)
    }
    /// Every setting as JSON, the ones files leave out included: what
    /// `item.channel` reads and writes.
    pub fn channel_json(&self) -> serde_json::Value {
        let mut value = serde_json::to_value(self).unwrap_or_default();
        if let (Some(map), serde_json::Value::Object(full)) = (
            value.as_object_mut(),
            serde_json::json!({
                "colorspace": self.colorspace,
                "cryptomatte_type": self.cryptomatte_type,
                "cryptomatte_levels": self.cryptomatte_levels,
                "lpe": self.lpe,
            }),
        ) {
            for (k, v) in full {
                map.entry(k).or_insert(v);
            }
        }
        value
    }
    /// Whether any setting needs native v53.
    pub fn needs_v53(&self) -> bool {
        !self.colorspace.is_default()
            || !self.cryptomatte_type.is_default()
            || !is_default_levels(&self.cryptomatte_levels)
    }
    /// Whether any setting needs native v55 (group D).
    pub fn needs_v55(&self) -> bool {
        !self.lpe.is_empty()
    }
}

impl OutputEffect {
    /// Data outputs hold numbers (depths, vectors, ids, coverage), not
    /// light: their Default colour space is always linear and their image
    /// processing starts as every output's does.
    pub fn is_data(self) -> bool {
        use OutputEffect::*;
        matches!(
            self,
            Alpha
                | Depth
                | ShadingNormal
                | GeometricNormal
                | ObjectCoordinates
                | UvCoordinates
                | MotionVector
                | ShadingIncidence
                | SegmentId
                | Cryptomatte
                // Material values that are numbers, not colours (RO-Q14).
                | DiffuseAmount
                | DiffuseCoefficient
                | DiffuseEnergyConservation
                | DiffuseRoughness
                | SpecularCoefficient
                | ReflectionCoefficient
                | Roughness
                | SubsurfaceAmount
                | TransparentAmount
                | Dpdu
                | Dpdv
                // A false-colour diagnostic picture (RO-Q29).
                | ShadedAaSamples
        )
    }
    /// Group D outputs (native v55).
    pub fn needs_v55(self) -> bool {
        use OutputEffect::*;
        matches!(
            self,
            IlluminationUnshadowed
                | DiffuseShadingUnshadowed
                | LightPathExpressions
                | ShadedAaSamples
        )
    }
    /// Outputs whose numbers are fixed by the renderer: the Colorspace row
    /// doesn't apply (greyed, UIC-117) and they are always written as they
    /// are. Cryptomatte's ids, the AA heat map's colours (RO-Q29).
    pub fn fixed_colorspace(self) -> bool {
        matches!(
            self,
            OutputEffect::Cryptomatte | OutputEffect::ShadedAaSamples
        )
    }
    /// Whether the output's Image Processing applies. The AA heat map is a
    /// diagnostic picture: never processed, its row greyed (RO-Q29).
    pub fn processes(self) -> bool {
        self != OutputEffect::ShadedAaSamples
    }
    /// The outputs that need the shadowless helper render (RO-Q19, D-Q2).
    pub fn needs_unshadowed_render(self) -> bool {
        matches!(
            self,
            OutputEffect::IlluminationUnshadowed | OutputEffect::DiffuseShadingUnshadowed
        )
    }
    /// Added with the Cycles built-in outputs (native v53).
    pub fn needs_v53(self) -> bool {
        use OutputEffect::*;
        matches!(
            self,
            IlluminationDirect
                | IlluminationIndirect
                | IlluminationTotal
                | UvCoordinates
                | SegmentId
                | Cryptomatte
        )
    }
}

/// Whether an output's numbers are written sRGB-encoded into a file
/// (`float_file`: EXR, HDR). Default: beauty outputs are display (sRGB) in
/// 8/16-bit files and linear in float files; data outputs always linear.
/// Linear / sRGB force it. Cryptomatte's ids and coverage are always
/// linear: the setting doesn't apply to it.
pub fn writes_srgb(effect: OutputEffect, colorspace: OutputColorspace, float_file: bool) -> bool {
    if effect.fixed_colorspace() {
        return false;
    }
    match colorspace {
        OutputColorspace::Linear => false,
        OutputColorspace::Srgb => true,
        OutputColorspace::Default => !effect.is_data() && !float_file,
    }
}

/// How the render window shows an output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputView {
    /// The window's own view switch (beauty outputs at Default).
    #[default]
    Window,
    /// The raw numbers, as the window's Linear view.
    Raw,
    /// Through sRGB whatever the window's switch says.
    Srgb,
}
/// The render window's view of an output: linear outputs show their raw
/// numbers, sRGB ones through sRGB, Default beauty ones by the switch.
pub fn output_view(effect: OutputEffect, colorspace: OutputColorspace) -> OutputView {
    match colorspace {
        _ if effect.fixed_colorspace() => OutputView::Raw,
        OutputColorspace::Linear => OutputView::Raw,
        OutputColorspace::Srgb => OutputView::Srgb,
        OutputColorspace::Default if effect.is_data() => OutputView::Raw,
        OutputColorspace::Default => OutputView::Window,
    }
}

/// A Segment ID colour: bright and distinct, the same for the same item and
/// surface every render and after reopening (a hash of the two, not of
/// anything that changes between runs). Hue from the hash, full
/// saturation, value 0.5..1.
pub fn segment_color(item: u64, material: &str) -> [f32; 3] {
    // FNV-1a, then a finaliser so near ids land far apart.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in item
        .to_le_bytes()
        .iter()
        .chain(&[0])
        .chain(material.as_bytes())
    {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    hash_color(h)
}

/// A Cryptomatte preview colour for an id (its float's bits).
pub fn id_color(id: f32) -> [f32; 3] {
    let mut h = u64::from(id.to_bits());
    h = h.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    h ^= h >> 29;
    hash_color(h)
}

fn hash_color(h: u64) -> [f32; 3] {
    let hue = (h >> 40) as f64 / f64::from(1u32 << 24);
    let value = 0.5 + 0.5 * ((h >> 16) & 0xffff) as f64 / 65_535.;
    crate::process_occlusion::hsv_to_rgb([hue, 1., value]).map(|c| c as f32)
}

/// Shaded AA Samples (RO-Q29): the heat map's colour for `t`, the share of
/// the extra samples a pixel took (0 = the adaptive minimum, 1 = every
/// sample): hue 240° (blue) down to 0° (red), full saturation and value,
/// through cyan, green and yellow.
pub fn heat_color(t: f32) -> [f32; 3] {
    let t = f64::from(t.clamp(0., 1.));
    crate::process_occlusion::hsv_to_rgb([(1. - t) * 240. / 360., 1., 1.]).map(|c| c as f32)
}

/// The `t` a heat-map colour came from (its hue back to 0..1), for the
/// raw value an OpenEXR Multilayer save keeps beside it (RO-Q30).
pub fn heat_value(rgb: [f32; 3]) -> f32 {
    let [h, ..] = crate::process_occlusion::rgb_to_hsv(rgb.map(|c| f64::from(c.max(0.))));
    // Hues above blue don't occur; read them as blue.
    let h = if h > 300. / 360. { 0. } else { h };
    (1. - h * 360. / 240.).clamp(0., 1.) as f32
}

/// The fewest samples Cycles' adaptive sampling gives a pixel at noise
/// threshold `threshold` (its automatic minimum: 16 / threshold^0.3, at
/// least 4), capped at `samples`; `None` with adaptive sampling off.
/// Cycles' own rule (integrator.cpp, `get_adaptive_sampling`), kept here so
/// the heat map's blue end is the real minimum (RO-Q29).
pub fn adaptive_min_samples(threshold: f32, samples: u32) -> Option<u32> {
    if threshold <= 0. {
        return None;
    }
    let min = (16. / threshold.powf(0.3)).ceil().max(4.) as u32;
    Some(min.min(samples.max(1)))
}

/// MurmurHash3 (x86, 32-bit), seed 0: Cryptomatte's name hash.
pub fn murmur3_32(data: &[u8]) -> u32 {
    const C1: u32 = 0xcc9e_2d51;
    const C2: u32 = 0x1b87_3593;
    let mut h: u32 = 0;
    let (blocks, tail) = data.as_chunks::<4>();
    for b in blocks {
        let k = u32::from_le_bytes(*b)
            .wrapping_mul(C1)
            .rotate_left(15)
            .wrapping_mul(C2);
        h = (h ^ k)
            .rotate_left(13)
            .wrapping_mul(5)
            .wrapping_add(0xe654_6b64);
    }
    if !tail.is_empty() {
        let mut k = 0u32;
        for (i, b) in tail.iter().enumerate() {
            k ^= u32::from(*b) << (8 * i);
        }
        h ^= k.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2);
    }
    h ^= data.len() as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^ (h >> 16)
}

/// A name hash as the float Cryptomatte stores ("uint32_to_float32": the
/// exponent kept off 0 and 255 so it is never a denormal, inf or NaN).
pub fn hash_to_float(hash: u32) -> f32 {
    let exponent = ((hash >> 23) & 0xff).clamp(1, 254);
    f32::from_bits((hash & 0x8000_0000) | (exponent << 23) | (hash & 0x007f_ffff))
}

/// The key of a Cryptomatte layer's metadata: "cryptomatte/<id>/" with the
/// first 7 hex digits of the layer name's hash (the Cryptomatte spec).
pub fn cryptomatte_metadata_prefix(layer: &str) -> String {
    let hex = format!("{:08x}", murmur3_32(layer.as_bytes()));
    format!("cryptomatte/{}/", &hex[..7])
}

/// One output a render makes.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderOutput {
    pub id: NodeId,
    pub name: String,
    pub effect: OutputEffect,
    pub settings: OutputSettings,
    /// Inside a Material Group: only that group's surfaces fill it
    /// (RO-Q03 v2).
    pub in_group: bool,
}

/// Most grouped outputs a render masks to their groups (RO-Q03 v2); later
/// ones record every surface, as before.
pub const MAX_GROUPED_OUTPUTS: usize = 8;

impl ShaderTree {
    /// A Render Output layer's settings (starting values when unset).
    pub fn output_settings(&self, id: NodeId) -> OutputSettings {
        self.outputs.get(&id).cloned().unwrap_or_default()
    }
    /// Every enabled Render Output in panel order (§1).
    pub fn render_outputs(&self) -> Vec<RenderOutput> {
        self.rows(&|_| false)
            .into_iter()
            .filter_map(|(n, _)| {
                let NodeKind::RenderOutput(effect) = n.kind else {
                    return None;
                };
                // Enabled itself and inside enabled layers only.
                let mut at = Some(n.id);
                while let Some(id) = at {
                    let node = self.get(id)?;
                    if !node.layer.enable {
                        return None;
                    }
                    at = node.parent;
                }
                Some(RenderOutput {
                    id: n.id,
                    name: n.name.clone(),
                    effect,
                    settings: self.output_settings(n.id),
                    in_group: n.parent.is_some(),
                })
            })
            .collect()
    }
}

impl ShaderTree {
    /// RO-Q03 v2: per grouped output (in `render_outputs` order, the first
    /// `MAX_GROUPED_OUTPUTS`), whether its groups all apply to polygon
    /// context `ctx`: bit k for the k-th grouped output.
    pub fn output_group_bits(&self, ctx: &crate::shader_eval::Context) -> u32 {
        let mut bits = 0;
        let grouped = self.render_outputs().into_iter().filter(|o| o.in_group);
        for (k, output) in grouped.take(MAX_GROUPED_OUTPUTS).enumerate() {
            let mut at = self.get(output.id).and_then(|n| n.parent);
            let mut applies = true;
            while let Some(id) = at {
                let Some(node) = self.get(id) else { break };
                if let crate::shader_tree::NodeKind::Group(g) = &node.kind
                    && !crate::shader_eval::mask_matches(g, ctx)
                {
                    applies = false;
                    break;
                }
                at = node.parent;
            }
            if applies {
                bits |= 1 << k;
            }
        }
        bits
    }
}

/// Pixel values a Depth output stores: metres, or white at the camera to
/// black at Maximum Depth with Remap on.
pub fn depth_value(metres: f64, s: &OutputSettings) -> f64 {
    if !s.remap {
        return metres;
    }
    if s.max_depth <= 0. {
        return 0.;
    }
    (1. - metres / s.max_depth).clamp(0., 1.)
}

/// Applies the output's image processing to a linear RGBA picture, in
/// RQ-1's order: exposure, input levels, hue / saturation / colorize, tone
/// map, vignette, gamma, output levels, clamp; Unpremultiply last. Bloom
/// is no longer an output setting: the Render window adds it to the
/// finished picture (post_effects.rs, BG-Q2).
/// Off (or all at their starting values) it leaves the picture as it is.
pub fn process(s: &OutputSettings, rgba: &mut [f32], width: u32, height: u32) {
    if !s.image_processing {
        return;
    }
    let exposure = match s.exposure {
        ExposureControl::WhiteLevel => 1. / s.white_level.max(1e-9),
        // ISO 100 = 1; doubling the speed doubles the brightness.
        ExposureControl::Photographic => s.iso / 100.,
    };
    let levels = s.input_black != 0. || s.input_white != 1. || s.input_gray != 1.;
    let colour = s.hue_offset != 0. || s.saturation != 1. || s.colorization != 0.;
    for px in rgba.chunks_mut(4) {
        let mut c = [px[0], px[1], px[2]].map(f64::from);
        if exposure != 1. {
            c = c.map(|v| v * exposure);
        }
        if levels {
            let span = s.input_white - s.input_black;
            c = c.map(|v| {
                let t = ((v - s.input_black) / span).max(0.);
                t.powf(1. / s.input_gray.max(1e-6))
            });
        }
        if colour {
            use crate::process_occlusion::{hsv_to_rgb, rgb_to_hsv};
            let [h, sat, v] = rgb_to_hsv(c.map(|x| x.max(0.)));
            let mut out = hsv_to_rgb([
                (h + s.hue_offset / 360.).rem_euclid(1.),
                (sat * s.saturation).clamp(0., 1.),
                v,
            ]);
            if s.colorization > 0. {
                // Toward the target colour at the pixel's brightness.
                let l = 0.2126 * out[0] + 0.7152 * out[1] + 0.0722 * out[2];
                for (o, t) in out.iter_mut().zip(s.target_color) {
                    *o += (t * l - *o) * s.colorization;
                }
            }
            c = out;
        }
        if s.tone_amount > 0. {
            let mapped = match s.tone_map {
                ToneMap::ReinhardRgb => c.map(|v| v / (1. + v.max(0.))),
                ToneMap::ReinhardLuminance => {
                    let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                    let k = if l > 0. { (l / (1. + l)) / l } else { 1. };
                    c.map(|v| v * k)
                }
            };
            for k in 0..3 {
                c[k] += (mapped[k] - c[k]) * s.tone_amount;
            }
        }
        for k in 0..3 {
            px[k] = c[k] as f32;
        }
    }
    let (w, h) = (width.max(1) as f64, height.max(1) as f64);
    let half_diag = (w * w + h * h).sqrt() / 2.;
    for (i, px) in rgba.chunks_mut(4).enumerate() {
        let mut c = [px[0], px[1], px[2]].map(f64::from);
        if s.vignette > 0. {
            // cos⁴ falloff toward the corners, scaled by the amount.
            let x = (i as u32 % width.max(1)) as f64 + 0.5 - w / 2.;
            let y = (i as u32 / width.max(1)) as f64 + 0.5 - h / 2.;
            let r = (x * x + y * y).sqrt() / half_diag;
            let angle = r.atan();
            let fall = angle.cos().powi(4);
            let k = 1. - s.vignette * (1. - fall);
            c = c.map(|v| v * k);
        }
        if s.gamma != 1. {
            c = c.map(|v| if v > 0. { v.powf(1. / s.gamma) } else { v });
        }
        if s.output_black != 0. || s.output_white != 1. {
            c = c.map(|v| s.output_black + v * (s.output_white - s.output_black));
        }
        if s.clamp {
            c = c.map(|v| v.clamp(0., s.clamp_threshold));
        }
        if s.unpremultiply && px[3] > 0. {
            let a = f64::from(px[3]);
            c = c.map(|v| v / a);
        }
        for k in 0..3 {
            px[k] = c[k] as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture() -> Vec<f32> {
        vec![
            0.25, 0.5, 0.75, 1., //
            2.0, 0.1, 0.0, 1., //
            0.0, 0.0, 0.0, 0.5, //
            0.5, 0.5, 0.5, 1.,
        ]
    }

    #[test]
    fn starting_values_change_nothing() {
        let mut p = picture();
        process(&OutputSettings::default(), &mut p, 2, 2);
        assert_eq!(p, picture());
    }

    #[test]
    fn exposure_gamma_clamp_and_levels() {
        let run = |f: fn(&mut OutputSettings)| {
            let mut s = OutputSettings::default();
            f(&mut s);
            let mut p = picture();
            process(&s, &mut p, 2, 2);
            p
        };
        assert_eq!(run(|s| s.white_level = 2.)[0], 0.125);
        assert!((run(|s| s.gamma = 2.)[0] - 0.5).abs() < 1e-6);
        assert_eq!(run(|s| s.clamp = true)[4], 1.0);
        assert_eq!(run(|s| s.output_white = 0.5)[1], 0.25);
        // Off: nothing at all.
        assert_eq!(
            run(|s| {
                s.image_processing = false;
                s.gamma = 3.;
            }),
            picture()
        );
        // Tone map 100 %: v / (1 + v).
        assert!((run(|s| s.tone_amount = 1.)[4] - 2. / 3.).abs() < 1e-6);
        // Saturation 0: grey.
        let p = run(|s| s.saturation = 0.);
        assert!((p[0] - p[1]).abs() < 1e-6 && (p[1] - p[2]).abs() < 1e-6);
        // Unpremultiply divides by alpha.
        let s = OutputSettings {
            unpremultiply: true,
            ..Default::default()
        };
        let mut p = vec![0.25, 0.25, 0.25, 0.5];
        process(&s, &mut p, 1, 1);
        assert_eq!(p[0], 0.5);
    }

    #[test]
    fn old_output_bloom_no_longer_glows_and_vignette_darkens_corners() {
        // Bloom moved to the Render window (BG-Q2): an old file's output
        // Bloom does nothing here, and isn't written back.
        let s = OutputSettings {
            bloom: true,
            bloom_radius: 0.2,
            ..Default::default()
        };
        let (w, h) = (9u32, 9u32);
        let mut p = vec![0f32; (w * h * 4) as usize];
        let centre = ((4 * w + 4) * 4) as usize;
        p[centre] = 10.;
        process(&s, &mut p, w, h);
        assert_eq!(p[centre + 4], 0., "no glow");
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("bloom"), "{text}");
        let s = OutputSettings {
            vignette: 1.,
            ..Default::default()
        };
        let mut p = vec![1f32; (w * h * 4) as usize];
        process(&s, &mut p, w, h);
        assert!(p[0] < p[centre]);
    }

    #[test]
    fn murmur3_matches_the_reference_and_floats_stay_finite() {
        assert_eq!(murmur3_32(b""), 0);
        assert_eq!(murmur3_32(b"hello"), 0x248b_fa47);
        assert_eq!(
            murmur3_32(b"The quick brown fox jumps over the lazy dog"),
            0x2e4f_f723
        );
        for h in [0u32, 0xffff_ffff, 0x7f80_0000, 0x0000_0001] {
            assert!(hash_to_float(h).is_normal(), "{h:08x}");
        }
        // Mantissa and sign kept: the manifest hash is readable back.
        let h = murmur3_32(b"Cube");
        assert_eq!(hash_to_float(h).to_bits() & 0x807f_ffff, h & 0x807f_ffff);
        assert!(cryptomatte_metadata_prefix("CryptoObject").starts_with("cryptomatte/"));
        assert_eq!(cryptomatte_metadata_prefix("CryptoObject").len(), 20);
    }

    #[test]
    fn segment_colours_are_stable_bright_and_distinct() {
        let a = segment_color(7, "Default");
        assert_eq!(a, segment_color(7, "Default"), "stable");
        let colours: Vec<[f32; 3]> = (0..12)
            .map(|i| segment_color(i, "Default"))
            .chain([segment_color(7, "Metal")])
            .collect();
        for (i, c) in colours.iter().enumerate() {
            let max = c.iter().copied().fold(0f32, f32::max);
            let min = c.iter().copied().fold(1f32, f32::min);
            assert!((0.5..=1.0).contains(&max), "value 0.5..1: {c:?}");
            assert!(min < 1e-6, "full saturation: {c:?}");
            for d in &colours[i + 1..] {
                assert_ne!(c, d);
            }
        }
        assert_ne!(id_color(1.0), id_color(2.0));
    }

    #[test]
    fn colour_space_by_output_kind_and_file() {
        use OutputEffect as O;
        let s = OutputSettings::default();
        assert!(writes_srgb(O::FinalColor, s.colorspace, false));
        assert!(!writes_srgb(O::FinalColor, s.colorspace, true));
        assert!(writes_srgb(O::IlluminationTotal, s.colorspace, false));
        for data in [O::Depth, O::ShadingNormal, O::UvCoordinates, O::SegmentId] {
            assert!(!writes_srgb(data, s.colorspace, false), "{data:?}");
            assert_eq!(output_view(data, s.colorspace), OutputView::Raw);
        }
        assert_eq!(output_view(O::FinalColor, s.colorspace), OutputView::Window);
        let linear = OutputSettings {
            colorspace: OutputColorspace::Linear,
            ..Default::default()
        };
        let srgb = OutputSettings {
            colorspace: OutputColorspace::Srgb,
            ..Default::default()
        };
        assert!(!writes_srgb(O::FinalColor, linear.colorspace, false));
        assert_eq!(
            output_view(O::FinalColor, linear.colorspace),
            OutputView::Raw
        );
        assert!(
            writes_srgb(O::Depth, srgb.colorspace, true),
            "sRGB floats in an EXR"
        );
        assert_eq!(output_view(O::Depth, srgb.colorspace), OutputView::Srgb);
        // Cryptomatte ignores it.
        assert!(!writes_srgb(O::Cryptomatte, srgb.colorspace, false));
        assert_eq!(
            output_view(O::Cryptomatte, srgb.colorspace),
            OutputView::Raw
        );
    }

    #[test]
    fn new_settings_start_valid_and_older_json_loads() {
        let mut s = OutputSettings::default();
        assert!(s.valid() && !s.needs_v53());
        assert_eq!(s.cryptomatte_levels, 6);
        for bad in [0, 1, 3, 18] {
            s.cryptomatte_levels = bad;
            assert!(!s.valid(), "{bad}");
        }
        s.cryptomatte_levels = 16;
        assert!(s.valid() && s.needs_v53());
        // Files leave the starting values out; channels list them all.
        let plain = serde_json::to_value(OutputSettings::default()).unwrap();
        assert!(plain.get("colorspace").is_none());
        let all = OutputSettings::default().channel_json();
        assert_eq!(all["colorspace"], "Default");
        assert_eq!(all["cryptomatte_levels"], 6);
        let back: OutputSettings = serde_json::from_value(all).unwrap();
        assert_eq!(back, OutputSettings::default());
        let set = OutputSettings {
            colorspace: OutputColorspace::Srgb,
            cryptomatte_type: CryptomatteType::Asset,
            cryptomatte_levels: 8,
            ..Default::default()
        };
        let text = serde_json::to_string(&set).unwrap();
        assert_eq!(serde_json::from_str::<OutputSettings>(&text).unwrap(), set);
    }

    /// Group D (RO-Q29): blue → green → red, and back to the share.
    #[test]
    fn heat_map_colours_and_adaptive_minimum() {
        assert_eq!(heat_color(0.), [0., 0., 1.], "blue");
        assert_eq!(heat_color(0.5), [0., 1., 0.], "green");
        assert_eq!(heat_color(1.), [1., 0., 0.], "red");
        assert_eq!(heat_color(7.), heat_color(1.), "clamped");
        for t in [0., 0.1, 0.25, 0.5, 0.9, 1.] {
            assert!((heat_value(heat_color(t)) - t).abs() < 1e-5, "{t}");
        }
        assert_eq!(adaptive_min_samples(0., 128), None, "adaptive off");
        // Cycles' automatic minimum: 16 / threshold^0.3, at least 4.
        assert_eq!(adaptive_min_samples(0.1, 128), Some(32));
        assert_eq!(adaptive_min_samples(0.01, 128), Some(64));
        assert_eq!(adaptive_min_samples(0.01, 20), Some(20), "capped");
    }

    #[test]
    fn depth_remaps_white_to_black() {
        let mut s = OutputSettings::default();
        assert_eq!(depth_value(4., &s), 4.);
        s.remap = true;
        assert!((depth_value(4., &s) - 0.6).abs() < 1e-12);
        assert_eq!(depth_value(40., &s), 0.);
    }
}
