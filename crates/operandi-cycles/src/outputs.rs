//! Render Outputs from Cycles passes (render outputs v1 §3, first batch;
//! group D 2026-11-20: Illumination / Diffuse Shading (Unshadowed), Light
//! Path Expressions, Shaded AA Samples).
use super::{Image, MaterialValue, Pass, RenderMesh, Settings};
use operandi_core::light_path_expressions::{self as lpe, Group, Part, Scatter, Term};
use operandi_core::render_outputs::{
    CryptomatteType, RenderOutput, adaptive_min_samples, depth_value, heat_color, id_color, process,
};
use operandi_core::shader_tree::OutputEffect;

/// The passes an output reads.
fn passes_of(effect: OutputEffect) -> &'static [Pass] {
    use OutputEffect as O;
    use Pass::*;
    match effect {
        O::FinalColor | O::Alpha | O::SubsurfaceShading | O::MotionVector => &[],
        O::Depth => &[Depth],
        O::ShadingNormal | O::GeometricNormal => &[Normal],
        O::DiffuseShadingTotal => &[DiffuseDirect, DiffuseIndirect, DiffuseColor],
        O::DiffuseShadingDirect => &[DiffuseDirect, DiffuseColor],
        O::DiffuseShadingIndirect => &[DiffuseIndirect, DiffuseColor],
        O::SpecularShading | O::ReflectionShading => &[GlossyDirect, GlossyIndirect, GlossyColor],
        O::TransparentShading => &[TransmissionDirect, TransmissionIndirect, TransmissionColor],
        O::LuminousShading => &[Emission],
        O::DiffuseColor => &[DiffuseColor],
        O::AmbientOcclusion => &[Ao],
        O::ObjectCoordinates => &[ObjectCoordinates],
        O::ShadingIncidence => &[Incidence],
        // Material values come from their own AOVs (`output_values`).
        O::DiffuseAmount
        | O::DiffuseCoefficient
        | O::DiffuseEnergyConservation
        | O::DiffuseRoughness
        | O::SpecularCoefficient
        | O::ReflectionCoefficient
        | O::Roughness
        | O::SubsurfaceAmount
        | O::SubsurfaceColor
        | O::TransparentAmount
        | O::TransparentColor
        | O::Dpdu
        | O::Dpdv => &[],
        // The diffuse light passes are light divided by the albedo: the
        // light arriving, without the surface colour (RO-Q10).
        O::IlluminationDirect => &[DiffuseDirect],
        O::IlluminationIndirect => &[DiffuseIndirect],
        O::IlluminationTotal => &[DiffuseDirect, DiffuseIndirect],
        O::UvCoordinates => &[Uv],
        O::SegmentId => &[Segment],
        // Its passes depend on its settings: `crypto_passes`.
        O::Cryptomatte => &[],
        // Group D. Unshadowed light comes from the helper render
        // (`unshadowed_settings`); the expression's passes from its
        // recipe (`lpe_passes`).
        O::IlluminationUnshadowed => &[],
        O::DiffuseShadingUnshadowed => &[DiffuseColor],
        O::LightPathExpressions => &[],
        // Dropped again when adaptive sampling is off (`prepare_outputs`).
        O::ShadedAaSamples => &[SampleCount],
    }
}

/// The passes of one Light Path Expressions recipe term.
fn term_passes(term: &Term) -> Vec<Pass> {
    use Pass::*;
    let light = |direct: Pass, indirect: Pass, part: Part| match part {
        Part::Direct => vec![direct],
        Part::Indirect => vec![indirect],
        Part::Both => vec![direct, indirect],
    };
    match term {
        Term::Combined | Term::Group(_) => Vec::new(),
        Term::Background => vec![Background],
        Term::Emission => vec![Emission],
        Term::Volume(part) => light(VolumeDirect, VolumeIndirect, *part),
        Term::Shading(family, part) => {
            let (d, i, c) = family_passes(*family);
            let mut out = light(d, i, *part);
            out.push(c);
            out
        }
    }
}

/// A light pass family's direct, indirect and colour passes.
fn family_passes(family: Scatter) -> (Pass, Pass, Pass) {
    use Pass::*;
    match family {
        Scatter::Diffuse => (DiffuseDirect, DiffuseIndirect, DiffuseColor),
        Scatter::Glossy => (GlossyDirect, GlossyIndirect, GlossyColor),
        Scatter::Transmission => (TransmissionDirect, TransmissionIndirect, TransmissionColor),
    }
}

/// Each Light Path Expressions output's recipe, `None` for the others.
fn recipes(outputs: &[RenderOutput]) -> Vec<Option<lpe::Recipe>> {
    outputs
        .iter()
        .map(|o| {
            (o.effect == OutputEffect::LightPathExpressions).then(|| lpe::recipe(&o.settings.lpe))
        })
        .collect()
}

/// Every pass the Light Path Expressions outputs read.
fn lpe_passes(outputs: &[RenderOutput]) -> Vec<Pass> {
    recipes(outputs)
        .into_iter()
        .flatten()
        .flat_map(|r| r.terms)
        .flat_map(|t| term_passes(&t))
        .collect()
}

/// Hidden light groups made only for Light Path Expressions: every light
/// without an LPE Label, every glowing surface, the world (RO-Q25). They
/// aren't shown or saved as light groups of their own.
pub const LPE_LIGHTS: &str = "__lpe_lights";
pub const LPE_EMITTERS: &str = "__lpe_emitters";
pub const LPE_WORLD: &str = "__lpe_world";
/// Whether a light group is one of the hidden ones.
pub fn is_hidden_group(name: &str) -> bool {
    name.starts_with("__lpe_")
}

/// What turning passes into pictures needs to know of the scene and the
/// render settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutputContext {
    /// The LPE Labels of the lights that render (`scene_light_groups`).
    pub light_labels: Vec<String>,
    /// The first environment's LPE Label (RO-Q25).
    pub world_label: Option<String>,
    /// The render's noise threshold; 0 = adaptive sampling off.
    pub adaptive_threshold: f32,
    /// The most samples a pixel may take (Maximum Quality): the heat map's
    /// red end.
    pub max_samples: u32,
}

impl OutputContext {
    pub fn of(scene: &operandi_core::scene::Scene, settings: &Settings) -> Self {
        let world_label = scene
            .items
            .iter()
            .filter(|i| scene.shows(i))
            .filter_map(|i| i.environment.as_ref())
            .map(|e| e.lpe_label.trim())
            .find(|l| !l.is_empty())
            .map(str::to_owned);
        Self {
            light_labels: super::scene_light_groups(scene),
            world_label,
            adaptive_threshold: settings.adaptive_threshold,
            max_samples: settings.samples,
        }
    }
    /// The light group pictures one recipe group reads.
    fn groups(&self, group: &Group) -> Vec<String> {
        match group {
            Group::AllLights => self
                .light_labels
                .iter()
                .cloned()
                .chain([LPE_LIGHTS.to_owned()])
                .collect(),
            Group::Label(label) => {
                let mut out = Vec::new();
                if self.light_labels.contains(label) {
                    out.push(label.clone());
                }
                if self.world_label.as_ref() == Some(label) {
                    out.push(LPE_WORLD.to_owned());
                }
                out
            }
            Group::Emitters => vec![LPE_EMITTERS.to_owned()],
            Group::World => vec![LPE_WORLD.to_owned()],
        }
    }
}

/// Group D settings that depend on the scene and the render settings:
/// the hidden light groups the Light Path Expressions outputs read (lights
/// without a label, glowing surfaces and the world joined to them), and no
/// sample count pass with adaptive sampling off. Returns notes for the
/// render window's status line.
pub fn prepare_outputs(
    settings: &mut Settings,
    meshes: &mut [RenderMesh],
    outputs: &[RenderOutput],
    context: &OutputContext,
) -> Vec<String> {
    let mut notes = Vec::new();
    if outputs
        .iter()
        .any(|o| o.effect == OutputEffect::ShadedAaSamples)
        && settings.adaptive_threshold <= 0.
    {
        settings.passes.retain(|p| *p != Pass::SampleCount);
        notes.push(AA_OFF_NOTE.to_owned());
    }
    let mut groups: Vec<Group> = Vec::new();
    for (o, r) in outputs.iter().zip(recipes(outputs)) {
        let Some(r) = r else { continue };
        if !r.status.renders() {
            notes.push(format!("{}: {}", o.name, r.status.line()));
            continue;
        }
        for t in r.terms {
            if let Term::Group(g) = t {
                if let Group::Label(l) = &g
                    && context.groups(&g).is_empty()
                {
                    notes.push(format!(
                        "{}: no light or environment has the LPE Label {l}",
                        o.name
                    ));
                }
                groups.push(g);
            }
        }
    }
    let add = |name: &str, settings: &mut Settings| {
        if !settings.light_groups.iter().any(|g| g == name) {
            settings.light_groups.push(name.to_owned());
        }
    };
    if groups.contains(&Group::AllLights) {
        for l in settings.lights.iter_mut().flatten() {
            if l.lightgroup.is_none() {
                l.lightgroup = Some(LPE_LIGHTS.to_owned());
            }
        }
        add(LPE_LIGHTS, settings);
    }
    if groups.contains(&Group::Emitters) {
        for m in meshes.iter_mut() {
            if let Some(sh) = m.shader.as_mut()
                && sh.lightgroup.is_none()
            {
                sh.lightgroup = Some(LPE_EMITTERS.to_owned());
            }
        }
        add(LPE_EMITTERS, settings);
    }
    let world = groups.iter().any(|g| match g {
        Group::World => true,
        Group::Label(l) => context.world_label.as_ref() == Some(l),
        _ => false,
    });
    if world {
        settings.world_lightgroup = Some(LPE_WORLD.to_owned());
        add(LPE_WORLD, settings);
    }
    notes
}

/// The render window's notice for Shaded AA Samples without adaptive
/// sampling (the output is then solid red).
pub const AA_OFF_NOTE: &str = "Noise Threshold is off: every pixel gets every sample";

/// Whether an enabled output needs the helper render.
pub fn needs_unshadowed(outputs: &[RenderOutput]) -> bool {
    outputs.iter().any(|o| o.effect.needs_unshadowed_render())
}

/// The render window's progress label for the helper render.
pub const UNSHADOWED_STEP: &str = "Unshadowed pass";

/// The Illumination (Unshadowed) helper render (D-Q1 v2, D-Q2: Jason
/// 2026-11-20): the main render's camera, size, samples, seed and noise
/// threshold, lit by the light items alone with their shadows off (no
/// world, no dome, no glowing surfaces, no bounces), giving only the
/// Diffuse Direct pass. `None` when no enabled output needs it.
pub fn unshadowed_settings(settings: &Settings, outputs: &[RenderOutput]) -> Option<Settings> {
    if !needs_unshadowed(outputs) {
        return None;
    }
    let mut s = settings.clone();
    s.passes = vec![Pass::DiffuseDirect];
    s.values = Vec::new();
    s.light_groups = Vec::new();
    s.world_lightgroup = None;
    // Lamps only: no environment light, no glowing surfaces (RO-Q18).
    s.world = Vec::new();
    s.dome = [0.0; 3];
    s.lamps_only = true;
    // Direct Caustics' caustics-only copies of lights would become plain
    // lights without it, and light the picture twice.
    if let Some(lights) = s.lights.as_mut() {
        lights.retain(|l| !l.caustics_only);
    }
    s.rays.direct_caustics = false;
    s.light_paths.shadows = false;
    // Bounce limits 0: direct light at the first hit only. Transparent
    // bounces stay the main render's (they follow Refraction Depth).
    s.light_paths.diffuse = false;
    s.light_paths.glossy = false;
    s.light_paths.transmission = false;
    s.rays.max_bounces = 0;
    // The pass isn't denoised either way; the picture isn't kept.
    s.denoise = false;
    Some(s)
}

/// Adds the helper render's Diffuse Direct to the main render's passes as
/// `Pass::UnshadowedDirect`.
pub fn attach_unshadowed(image: &mut Image, helper: Image) {
    if let Some((_, pixels)) = helper
        .passes
        .into_iter()
        .find(|(p, _)| *p == Pass::DiffuseDirect)
    {
        image.passes.push((Pass::UnshadowedDirect, pixels));
    }
}

/// Cycles keeps one cryptomatte depth for every type: the most levels any
/// Cryptomatte output asks for, as passes of two levels each (RO-Q15).
fn crypto_depth(outputs: &[RenderOutput]) -> usize {
    outputs
        .iter()
        .filter(|o| o.effect == OutputEffect::Cryptomatte)
        .map(|o| o.settings.cryptomatte_levels.clamp(2, 16).div_ceil(2) as usize)
        .max()
        .unwrap_or(0)
}

/// The cryptomatte types asked for, in first-use order, each once.
fn crypto_kinds(outputs: &[RenderOutput]) -> Vec<CryptomatteType> {
    let mut kinds = Vec::new();
    for o in outputs
        .iter()
        .filter(|o| o.effect == OutputEffect::Cryptomatte)
    {
        if !kinds.contains(&o.settings.cryptomatte_type) {
            kinds.push(o.settings.cryptomatte_type);
        }
    }
    kinds
}

/// Every cryptomatte pass: each type asked for gets the full depth (the
/// kernel writes every enabled type at the same depth).
fn crypto_passes(outputs: &[RenderOutput]) -> Vec<Pass> {
    let depth = crypto_depth(outputs);
    crypto_kinds(outputs)
        .into_iter()
        .flat_map(|kind| (0..depth).filter_map(move |k| Pass::crypto(kind, k)))
        .collect()
}

/// Every material value the enabled outputs write, once each.
pub fn output_values(outputs: &[RenderOutput]) -> Vec<MaterialValue> {
    let mut out: Vec<MaterialValue> = outputs
        .iter()
        .filter_map(|o| o.effect.material_value())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Every pass the enabled outputs need, once each.
pub fn output_passes(outputs: &[RenderOutput]) -> Vec<Pass> {
    let mut out: Vec<Pass> = outputs
        .iter()
        .flat_map(|o| passes_of(o.effect).iter().copied())
        .chain(group_passes(outputs).into_iter().flatten())
        .chain(crypto_passes(outputs))
        .chain(lpe_passes(outputs))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// RO-Q03 v2: per output, the coverage pass of its Material Group (the
/// first `MAX_GROUPED_OUTPUTS` grouped outputs), `None` for the rest.
pub fn group_passes(outputs: &[RenderOutput]) -> Vec<Option<Pass>> {
    let mut k = 0;
    outputs
        .iter()
        .map(|o| {
            if !o.in_group {
                return None;
            }
            let pass = Pass::group(k);
            k += 1;
            pass
        })
        .collect()
}

/// The Ambient Occlusion pass's ray length: the first AO output's Occlusion
/// Range, 0 = no limit (RQ-2).
pub fn ao_distance(outputs: &[RenderOutput]) -> f32 {
    outputs
        .iter()
        .find(|o| o.effect == OutputEffect::AmbientOcclusion)
        .map_or(1e10, |o| {
            if o.settings.occlusion_range > 0. {
                o.settings.occlusion_range as f32
            } else {
                1e10
            }
        })
}

/// Each output's picture (linear RGBA, laid out like `image.rgba`), after
/// its image processing. Final Color is the combined picture.
pub fn output_pictures(outputs: &[RenderOutput], image: &Image) -> Vec<(String, Vec<f32>)> {
    output_pictures_in(outputs, image, &OutputContext::default())
}

/// [`output_pictures`] with what group D's outputs need to know of the
/// scene and the render (`OutputContext::of`).
pub fn output_pictures_in(
    outputs: &[RenderOutput],
    image: &Image,
    context: &OutputContext,
) -> Vec<(String, Vec<f32>)> {
    let n = image.width as usize * image.height as usize;
    let light_group = |name: &str| {
        image
            .light_groups
            .iter()
            .find(|(g, _)| g == name)
            .map(|(_, v)| v)
    };
    let recipes = recipes(outputs);
    // Shaded AA Samples (RO-Q29): the pass holds each pixel's samples as a
    // share of the samples rendered; the heat map runs from the adaptive
    // minimum (blue) to Maximum Quality (red).
    let rendered = image.samples.max(1) as f32;
    let most = context.max_samples.max(image.samples).max(1);
    let least = adaptive_min_samples(context.adaptive_threshold, most).unwrap_or(most) as f32;
    let most = most as f32;
    let pass = |p: Pass| image.passes.iter().find(|(q, _)| *q == p).map(|(_, v)| v);
    let px =
        |p: Pass, i: usize| pass(p).map_or([0.; 3], |v| [v[i * 4], v[i * 4 + 1], v[i * 4 + 2]]);
    let value = |v: MaterialValue| image.values.iter().find(|(w, _)| *w == v).map(|(_, p)| p);
    let groups = group_passes(outputs);
    outputs
        .iter()
        .zip(groups)
        .zip(&recipes)
        .map(|((o, group), recipe)| {
            let group_pictures: Vec<&Vec<f32>> = recipe
                .iter()
                .flat_map(|r| &r.terms)
                .filter_map(|t| match t {
                    Term::Group(g) => Some(context.groups(g)),
                    _ => None,
                })
                .flatten()
                .filter_map(|name| light_group(&name))
                .collect();
            let px_group = |i: usize| {
                let mut c = [0f32; 3];
                for p in &group_pictures {
                    for k in 0..3 {
                        c[k] += p[i * 4 + k];
                    }
                }
                c
            };
            use OutputEffect as O;
            let mut rgba = vec![0f32; n * 4];
            for i in 0..n {
                let alpha = image.rgba[i * 4 + 3];
                let c: [f32; 3] = match o.effect {
                    O::FinalColor => [0, 1, 2].map(|k| image.rgba[i * 4 + k]),
                    O::Alpha => [alpha; 3],
                    O::Depth => {
                        // Nothing hit: Cycles' far value; remapped it is black.
                        let d = f64::from(px(Pass::Depth, i)[0]);
                        let d = if alpha <= 0. { f64::from(f32::MAX) } else { d };
                        [depth_value(d, &o.settings) as f32; 3]
                    }
                    O::ShadingNormal | O::GeometricNormal => {
                        let v = px(Pass::Normal, i);
                        if o.settings.normalize {
                            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                            if l > 0. { v.map(|x| x / l) } else { v }
                        } else {
                            v
                        }
                    }
                    O::DiffuseShadingTotal
                    | O::DiffuseShadingDirect
                    | O::DiffuseShadingIndirect => {
                        let (d, ind) = (px(Pass::DiffuseDirect, i), px(Pass::DiffuseIndirect, i));
                        let col = px(Pass::DiffuseColor, i);
                        [0, 1, 2].map(|k| {
                            let light = match o.effect {
                                O::DiffuseShadingDirect => d[k],
                                O::DiffuseShadingIndirect => ind[k],
                                _ => d[k] + ind[k],
                            };
                            light * col[k]
                        })
                    }
                    O::SpecularShading | O::ReflectionShading => {
                        let (d, ind, col) = (
                            px(Pass::GlossyDirect, i),
                            px(Pass::GlossyIndirect, i),
                            px(Pass::GlossyColor, i),
                        );
                        [0, 1, 2].map(|k| (d[k] + ind[k]) * col[k])
                    }
                    O::TransparentShading => {
                        let (d, ind, col) = (
                            px(Pass::TransmissionDirect, i),
                            px(Pass::TransmissionIndirect, i),
                            px(Pass::TransmissionColor, i),
                        );
                        [0, 1, 2].map(|k| (d[k] + ind[k]) * col[k])
                    }
                    O::LuminousShading => px(Pass::Emission, i),
                    O::DiffuseColor => px(Pass::DiffuseColor, i),
                    O::AmbientOcclusion => [px(Pass::Ao, i)[0]; 3],
                    O::ObjectCoordinates => px(Pass::ObjectCoordinates, i),
                    O::ShadingIncidence => [px(Pass::Incidence, i)[0]; 3],
                    // Material values: grey ones carry their number in the
                    // first float (MV-Q05; nothing hit stays 0).
                    O::DiffuseAmount
                    | O::DiffuseCoefficient
                    | O::DiffuseEnergyConservation
                    | O::DiffuseRoughness
                    | O::SpecularCoefficient
                    | O::ReflectionCoefficient
                    | O::Roughness
                    | O::SubsurfaceAmount
                    | O::SubsurfaceColor
                    | O::TransparentAmount
                    | O::TransparentColor
                    | O::Dpdu
                    | O::Dpdv => {
                        let v = o.effect.material_value().expect("a material value");
                        value(v).map_or([0.; 3], |p| {
                            if v.is_color() {
                                [p[i * 4], p[i * 4 + 1], p[i * 4 + 2]]
                            } else {
                                [p[i * 4]; 3]
                            }
                        })
                    }
                    O::IlluminationDirect => px(Pass::DiffuseDirect, i),
                    O::IlluminationIndirect => px(Pass::DiffuseIndirect, i),
                    O::IlluminationTotal => {
                        let (d, ind) = (px(Pass::DiffuseDirect, i), px(Pass::DiffuseIndirect, i));
                        [0, 1, 2].map(|k| d[k] + ind[k])
                    }
                    O::UvCoordinates => px(Pass::Uv, i),
                    O::SegmentId => px(Pass::Segment, i),
                    // The preview: each pixel in its highest-coverage id's
                    // colour (Cycles sorts the ranks, so that's the first).
                    O::Cryptomatte => {
                        match Pass::crypto(o.settings.cryptomatte_type, 0).and_then(pass) {
                            Some(v) if v[i * 4 + 1] > 0. => id_color(v[i * 4]),
                            _ => [0.; 3],
                        }
                    }
                    // No Cycles pass yet: empty (RO-Q02).
                    O::SubsurfaceShading | O::MotionVector => [0.; 3],
                    // RO-Q18: the lamps' direct light with no shadows,
                    // nothing else; Diffuse Shading adds the colour.
                    O::IlluminationUnshadowed => px(Pass::UnshadowedDirect, i),
                    O::DiffuseShadingUnshadowed => {
                        let (d, col) = (px(Pass::UnshadowedDirect, i), px(Pass::DiffuseColor, i));
                        [0, 1, 2].map(|k| d[k] * col[k])
                    }
                    O::LightPathExpressions => {
                        let mut c = [0f32; 3];
                        for t in recipe.iter().flat_map(|r| &r.terms) {
                            let add = match t {
                                Term::Combined => [0, 1, 2].map(|k| image.rgba[i * 4 + k]),
                                Term::Group(_) => [0.; 3],
                                Term::Shading(family, part) => {
                                    let (d, ind, col) = family_passes(*family);
                                    let (d, ind, col) = (px(d, i), px(ind, i), px(col, i));
                                    [0, 1, 2].map(|k| {
                                        let light = match part {
                                            Part::Direct => d[k],
                                            Part::Indirect => ind[k],
                                            Part::Both => d[k] + ind[k],
                                        };
                                        light * col[k]
                                    })
                                }
                                other => {
                                    let mut sum = [0f32; 3];
                                    for p in term_passes(other) {
                                        let v = px(p, i);
                                        for k in 0..3 {
                                            sum[k] += v[k];
                                        }
                                    }
                                    sum
                                }
                            };
                            for k in 0..3 {
                                c[k] += add[k];
                            }
                        }
                        let g = px_group(i);
                        [0, 1, 2].map(|k| c[k] + g[k])
                    }
                    // RO-Q29: every pixel red without adaptive sampling.
                    O::ShadedAaSamples => {
                        let t = match pass(Pass::SampleCount) {
                            Some(v) if least < most => {
                                (v[i * 4] * rendered - least) / (most - least)
                            }
                            _ => 1.,
                        };
                        heat_color(t)
                    }
                };
                // RO-Q03 v2: only the group's surfaces fill it.
                let cover = group.map_or(1., |g| px(g, i)[0].clamp(0., 1.));
                rgba[i * 4..i * 4 + 3].copy_from_slice(&c.map(|v| v * cover));
                rgba[i * 4 + 3] = if o.effect == O::FinalColor {
                    alpha * cover
                } else {
                    1.
                };
            }
            if o.effect.processes() {
                process(&o.settings, &mut rgba, image.width, image.height);
            }
            (o.name.clone(), rgba)
        })
        .collect()
}

/// One cryptomatte type's raw layers for a file: `levels / 2` pictures of
/// ranked (id, coverage, id, coverage) per pixel, linear, and Cycles'
/// manifest.
#[derive(Clone, Debug, PartialEq)]
pub struct CryptoLayers {
    pub kind: CryptomatteType,
    /// "CryptoObject" / "CryptoMaterial" / "CryptoAsset"; the file's
    /// layers are this plus 00, 01, ...
    pub name: &'static str,
    pub levels: u32,
    pub passes: Vec<Vec<f32>>,
    /// JSON object: name to 8-digit hex hash.
    pub manifest: String,
}

/// The raw cryptomatte layers of the render, one entry per type the
/// Cryptomatte outputs ask for, with the most levels any of them asks for
/// (RO-Q15).
pub fn cryptomatte_layers(outputs: &[RenderOutput], image: &Image) -> Vec<CryptoLayers> {
    crypto_kinds(outputs)
        .into_iter()
        .map(|kind| {
            let levels = outputs
                .iter()
                .filter(|o| {
                    o.effect == OutputEffect::Cryptomatte && o.settings.cryptomatte_type == kind
                })
                .map(|o| o.settings.cryptomatte_levels.clamp(2, 16))
                .max()
                .unwrap_or(2);
            let passes = (0..levels.div_ceil(2) as usize)
                .map(|k| {
                    Pass::crypto(kind, k)
                        .and_then(|p| image.passes.iter().find(|(q, _)| *q == p))
                        .map_or_else(
                            || vec![0.; image.width as usize * image.height as usize * 4],
                            |(_, v)| v.clone(),
                        )
                })
                .collect();
            let manifest = image
                .cryptomatte_manifests
                .iter()
                .find(|(k, _)| *k == kind)
                .map_or_else(|| "{}".to_owned(), |(_, m)| clean_manifest(m));
            CryptoLayers {
                kind,
                name: kind.layer_name(),
                levels,
                passes,
                manifest,
            }
        })
        .collect()
}

/// Cycles' manifest as valid JSON: it writes names unescaped and includes
/// the nameless objects it adds itself (lights, the world). Entries are
/// read back as `"name":"hash"` pairs and written out properly.
fn clean_manifest(raw: &str) -> String {
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(raw) {
        let kept: serde_json::Map<String, serde_json::Value> =
            map.into_iter().filter(|(k, _)| !k.is_empty()).collect();
        return serde_json::Value::Object(kept).to_string();
    }
    // A name with a quote in it: split on the `":"` and `","` separators.
    let inner = raw.trim().trim_start_matches('{').trim_end_matches('}');
    let mut map = serde_json::Map::new();
    for entry in inner.split("\",\"") {
        let entry = entry.trim_matches('"');
        if let Some((name, hash)) = entry.rsplit_once("\":\"")
            && !name.is_empty()
        {
            map.insert(name.to_owned(), serde_json::Value::String(hash.to_owned()));
        }
    }
    serde_json::Value::Object(map).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::render_outputs::OutputSettings;

    fn output(effect: OutputEffect) -> RenderOutput {
        RenderOutput {
            id: 1,
            name: effect.layer_name().into(),
            effect,
            settings: OutputSettings::default(),
            in_group: false,
        }
    }

    #[test]
    fn passes_are_asked_for_once() {
        let outs = [
            output(OutputEffect::DiffuseShadingTotal),
            output(OutputEffect::DiffuseColor),
            output(OutputEffect::FinalColor),
        ];
        assert_eq!(
            output_passes(&outs),
            vec![
                Pass::DiffuseDirect,
                Pass::DiffuseIndirect,
                Pass::DiffuseColor
            ]
        );
        assert_eq!(ao_distance(&outs), 1e10);
    }

    #[test]
    fn diffuse_shading_is_light_times_colour() {
        let image = Image {
            width: 1,
            height: 1,
            rgba: vec![0.5, 0.5, 0.5, 1.],
            device: String::new(),
            used_gpu: false,
            denoised: false,
            samples: 1,
            light_groups: Vec::new(),
            values: Vec::new(),
            passes: vec![
                (Pass::DiffuseDirect, vec![2., 2., 2., 1.]),
                (Pass::DiffuseIndirect, vec![1., 1., 1., 1.]),
                (Pass::DiffuseColor, vec![0.5, 0.25, 0., 1.]),
                (Pass::Depth, vec![4., 0., 0., 0.]),
            ],
            cryptomatte_manifests: Vec::new(),
        };
        let pics = output_pictures(
            &[
                output(OutputEffect::DiffuseShadingTotal),
                output(OutputEffect::DiffuseShadingDirect),
                output(OutputEffect::Depth),
                output(OutputEffect::Alpha),
            ],
            &image,
        );
        assert_eq!(&pics[0].1[..3], &[1.5, 0.75, 0.]);
        assert_eq!(&pics[1].1[..3], &[1., 0.5, 0.]);
        assert_eq!(pics[2].1[0], 4.);
        assert_eq!(&pics[3].1[..4], &[1., 1., 1., 1.]);
    }

    #[test]
    fn material_values_read_their_own_pictures() {
        let outs = [
            output(OutputEffect::Roughness),
            output(OutputEffect::DiffuseCoefficient),
            output(OutputEffect::Roughness),
            output(OutputEffect::Dpdu),
            output(OutputEffect::FinalColor),
        ];
        assert!(output_passes(&outs).is_empty());
        assert_eq!(
            output_values(&outs),
            vec![
                MaterialValue::DiffuseCoefficient,
                MaterialValue::Roughness,
                MaterialValue::Dpdu
            ]
        );
        let image = Image {
            width: 1,
            height: 1,
            rgba: vec![0.5, 0.5, 0.5, 1.],
            device: String::new(),
            used_gpu: false,
            denoised: false,
            samples: 1,
            light_groups: Vec::new(),
            passes: Vec::new(),
            cryptomatte_manifests: Vec::new(),
            values: vec![
                // A grey value: only the first float is written.
                (MaterialValue::Roughness, vec![0.42, 0., 0., 0.]),
                (MaterialValue::DiffuseCoefficient, vec![0.3, 0.15, 0.05, 0.]),
                (MaterialValue::Dpdu, vec![-1., 0., 0., 0.]),
            ],
        };
        let pics = output_pictures(&outs[..4], &image);
        assert_eq!(&pics[0].1[..4], &[0.42, 0.42, 0.42, 1.]);
        assert_eq!(&pics[1].1[..4], &[0.3, 0.15, 0.05, 1.]);
        assert_eq!(&pics[3].1[..4], &[-1., 0., 0., 1.]);
        // Missing (no value picture): black.
        let empty = Image {
            values: Vec::new(),
            ..image
        };
        assert_eq!(&output_pictures(&outs[..1], &empty)[0].1[..3], &[0.; 3]);
    }
}

#[cfg(test)]
mod built_in_tests {
    use super::*;
    use operandi_core::render_outputs::{OutputSettings, hash_to_float, murmur3_32};

    fn output(effect: OutputEffect) -> RenderOutput {
        RenderOutput {
            id: 1,
            name: effect.layer_name().into(),
            effect,
            settings: OutputSettings::default(),
            in_group: false,
        }
    }

    fn crypto(kind: CryptomatteType, levels: u32) -> RenderOutput {
        let mut o = output(OutputEffect::Cryptomatte);
        o.settings.cryptomatte_type = kind;
        o.settings.cryptomatte_levels = levels;
        o
    }

    fn image(passes: Vec<(Pass, Vec<f32>)>) -> Image {
        Image {
            width: 1,
            height: 1,
            rgba: vec![0.5, 0.5, 0.5, 1.],
            device: String::new(),
            used_gpu: false,
            denoised: false,
            samples: 1,
            light_groups: Vec::new(),
            passes,
            values: Vec::new(),
            cryptomatte_manifests: vec![(
                CryptomatteType::Object,
                r#"{"Cube":"0000002a","":"00000000","Ball":"0000002b"}"#.into(),
            )],
        }
    }

    #[test]
    fn illumination_uv_and_segment_read_their_passes() {
        let outs = [
            output(OutputEffect::IlluminationTotal),
            output(OutputEffect::IlluminationDirect),
            output(OutputEffect::IlluminationIndirect),
            output(OutputEffect::UvCoordinates),
            output(OutputEffect::SegmentId),
        ];
        assert_eq!(
            output_passes(&outs),
            vec![
                Pass::DiffuseDirect,
                Pass::DiffuseIndirect,
                Pass::Uv,
                Pass::Segment
            ]
        );
        let img = image(vec![
            (Pass::DiffuseDirect, vec![2., 1., 0.5, 1.]),
            (Pass::DiffuseIndirect, vec![0.5, 0.5, 0.5, 1.]),
            (Pass::Uv, vec![0.25, 0.75, 0., 1.]),
            (Pass::Segment, vec![1., 0., 0.5, 1.]),
        ]);
        let pics = output_pictures(&outs, &img);
        assert_eq!(&pics[0].1[..4], &[2.5, 1.5, 1., 1.]);
        assert_eq!(&pics[1].1[..3], &[2., 1., 0.5]);
        assert_eq!(&pics[2].1[..3], &[0.5, 0.5, 0.5]);
        assert_eq!(&pics[3].1[..3], &[0.25, 0.75, 0.]);
        assert_eq!(&pics[4].1[..3], &[1., 0., 0.5]);
    }

    #[test]
    fn cryptomatte_asks_for_every_type_at_the_deepest_levels() {
        let outs = [
            crypto(CryptomatteType::Object, 2),
            crypto(CryptomatteType::Asset, 6),
            crypto(CryptomatteType::Object, 4),
        ];
        assert_eq!(
            output_passes(&outs),
            vec![
                Pass::CryptoObject0,
                Pass::CryptoObject1,
                Pass::CryptoObject2,
                Pass::CryptoAsset0,
                Pass::CryptoAsset1,
                Pass::CryptoAsset2,
            ]
        );
        assert_eq!(
            Pass::CryptoMaterial3.crypto_kind(),
            Some(CryptomatteType::Material)
        );
        assert_eq!(Pass::Uv.crypto_kind(), None);
        assert_eq!(Pass::crypto(CryptomatteType::Asset, 8), None);
    }

    #[test]
    fn cryptomatte_preview_and_raw_layers() {
        let cube = hash_to_float(murmur3_32(b"Cube"));
        let ball = hash_to_float(murmur3_32(b"Ball"));
        let outs = [crypto(CryptomatteType::Object, 4)];
        let img = image(vec![
            (Pass::CryptoObject0, vec![cube, 0.7, ball, 0.3]),
            (Pass::CryptoObject1, vec![0., 0., 0., 0.]),
        ]);
        let pics = output_pictures(&outs, &img);
        assert_eq!(&pics[0].1[..3], &id_color(cube), "top id's colour");
        let layers = cryptomatte_layers(&outs, &img);
        assert_eq!(layers.len(), 1);
        let l = &layers[0];
        assert_eq!((l.name, l.levels, l.passes.len()), ("CryptoObject", 4, 2));
        assert_eq!(l.passes[0], vec![cube, 0.7, ball, 0.3]);
        // Valid JSON without Cycles' nameless objects.
        let m: serde_json::Value = serde_json::from_str(&l.manifest).unwrap();
        assert_eq!(m["Cube"], "0000002a");
        assert!(m.get("").is_none());
        // Nothing covered: black.
        let empty = image(vec![(Pass::CryptoObject0, vec![0.; 4])]);
        assert_eq!(&output_pictures(&outs, &empty)[0].1[..3], &[0.; 3]);
    }

    #[test]
    fn unescaped_manifest_names_are_repaired() {
        let m = clean_manifest(r#"{"Say "hi"":"00000001","Box":"00000002"}"#);
        let v: serde_json::Value = serde_json::from_str(&m).unwrap();
        assert_eq!(v["Say \"hi\""], "00000001");
        assert_eq!(v["Box"], "00000002");
    }
}
