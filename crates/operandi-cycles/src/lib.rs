//! Final-quality renders through Cycles, Blender's path tracer, used as a
//! library (Apache-2.0).
//!
//! Spike scope: the scene's visible meshes with their materials (a
//! principled surface each), a camera matching the viewport (or the scene's
//! render camera), one sun (or the scene's directional lights) and a sky
//! of environment layers (`EnvironmentLayer`). Rendering runs on
//! the CPU. The C++ side lives in `cycles/` and is reached through a small C
//! interface, so this crate only deals in plain arrays.
use glam::{DMat4, DVec3};
use operandi_core::Result;

mod outputs;
use operandi_core::camera_light::{
    AreaShape, LightFalloff, LightKind, LightSettings, LinkMode, Projection, ShadowType,
};
pub use operandi_core::material_values::MaterialValue;
pub use operandi_core::render_outputs::CryptomatteType;
use operandi_core::scene::Scene;
use operandi_core::shader_eval::{Context, Expr, Input, Principled, SeeThroughKind};
use operandi_core::shader_tree::{AlphaType, NodeKind, ShaderChannels};
use operandi_core::texture_locator::Tiling;
pub use outputs::{
    AA_OFF_NOTE, CryptoLayers, LPE_EMITTERS, LPE_LIGHTS, LPE_WORLD, OutputContext, UNSHADOWED_STEP,
    ao_distance, attach_unshadowed, cryptomatte_layers, is_hidden_group, needs_unshadowed,
    output_passes, output_pictures, output_pictures_in, output_values, prepare_outputs,
    unshadowed_settings,
};

/// One mesh ready for Cycles: points and triangles in the mesh's own
/// space (placed by `transform`), all with one surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderMesh {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    /// One flag per triangle: smooth unless the scene's Smoothing is 0 %
    /// or the material's Smoothing is off.
    pub smooth: Vec<u8>,
    /// Texture coordinates, three per triangle (its corners), when the
    /// surface reads an image or a normal map; empty otherwise.
    pub uvs: Vec<[f32; 2]>,
    /// `None` renders with the settings' default material (studio clay):
    /// polygons the shader tree leaves at its defaults.
    pub shader: Option<RenderShader>,
    /// LP-30..32: the Cycles receiver (and shadow blocker) light set of
    /// the item this mesh comes from; 0 = lit by every light.
    pub light_set: u32,
    /// For a glowing surface that is a light: the receiver sets it lights;
    /// `None` = all.
    pub light_membership: Option<u64>,
    /// The shader's corner attributes (`RenderShader::attributes`), RGBA per
    /// triangle corner, the attributes of one corner side by side.
    pub attributes: Vec<[f32; 4]>,
    /// The shading normal at each triangle corner (three per triangle), in
    /// the mesh's own space, as the viewport shades it. Empty: Cycles
    /// averages the triangles around each point.
    pub normals: Vec<[f32; 3]>,
    /// Row-major 3x4 matrix placing the mesh in the world; `None`: the
    /// points are in world space already.
    pub transform: Option<[f32; 12]>,
    /// Cryptomatte names: the item's name, its top-level ancestor's name
    /// (the item's own without one) and the surface's material tag
    /// (render outputs RO-Q16).
    pub name: String,
    pub asset_name: String,
    pub material_name: String,
    /// The Segment ID output's colour for this piece
    /// (`render_outputs::segment_color`).
    pub segment_color: [f32; 3],
}

impl RenderMesh {
    /// The points in world space (`transform` applied).
    pub fn world_positions(&self) -> Vec<[f32; 3]> {
        let Some(m) = self.transform else {
            return self.positions.clone();
        };
        self.positions
            .iter()
            .map(|p| {
                std::array::from_fn(|r| {
                    m[r * 4] * p[0] + m[r * 4 + 1] * p[1] + m[r * 4 + 2] * p[2] + m[r * 4 + 3]
                })
            })
            .collect()
    }
}

/// [`transform_rows`] back to a matrix.
pub fn rows_matrix(t: [f32; 12]) -> DMat4 {
    let r = |i: usize| glam::Vec4::from_slice(&t[i * 4..i * 4 + 4]).as_dvec4();
    DMat4::from_cols(r(0), r(1), r(2), glam::DVec4::W).transpose()
}

/// The first three rows of `m`, row-major (a 3x4 affine transform).
pub fn transform_rows(m: DMat4) -> [f32; 12] {
    let mut out = [0.0f32; 12];
    for r in 0..3 {
        let row = m.row(r);
        out[r * 4..r * 4 + 4].copy_from_slice(&row.as_vec4().to_array());
    }
    out
}

/// One node of a compiled surface (shader_eval::Expr, flattened). `a`, `b`
/// and `fac` index earlier nodes; `image` indexes `RenderShader::images`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShaderOp {
    pub op: u32,
    pub mode: u32,
    pub a: i32,
    pub b: i32,
    pub fac: i32,
    pub value: [f32; 3],
    pub image: i32,
    /// LOCATOR: world -> locator space, row-major 3×4. PATTERN: the
    /// pattern's parameters (see `pattern_params`).
    pub xform: [f32; 12],
}
impl ShaderOp {
    pub const CONST: u32 = 0;
    pub const IMAGE_COLOR: u32 = 1;
    pub const IMAGE_ALPHA: u32 = 2;
    pub const LUMA: u32 = 3;
    pub const MIX: u32 = 4;
    /// Component `mode` (0 red, 1 green, 2 blue) of `a`, as a grey.
    pub const CHANNEL: u32 = 5;
    /// `a` raised to `value[0]` per component (Cycles' Gamma node).
    pub const GAMMA: u32 = 6;
    /// The shading point (world position) in a Texture Locator's space:
    /// `xform` applied to it (a vector).
    pub const LOCATOR: u32 = 7;
    /// Planar coordinates of locator-space vector `a` for axis `mode`
    /// (0 X, 1 Y, 2 Z): ((in-plane + value[0]) × value[1..3]), depth 0.
    pub const PLANAR: u32 = 8;
    /// The mesh UVs × (value[0], value[1]), rotated value[2] radians about
    /// (0, 0) (a vector).
    pub const UV: u32 = 9;
    /// Vector `a` tiled per axis: `mode` = horizontal | vertical << 2, each
    /// an `OC_TILE_*`.
    pub const TILE: u32 = 10;
    /// 1 where vector `a` lies in 0..1 on each clipped axis (`mode` bit 0
    /// horizontal, bit 1 vertical), else 0.
    pub const INSIDE: u32 = 11;
    /// A procedural pattern's t (0..1) at vector `a`: `mode` 0 Checker,
    /// 1 Grid, 2 Dots, 3 Noise, 4 Cellular, 5 Wood, 6 Ripples, 7 Weave,
    /// 8 Gabor Noise, 9 Poisson Cells;
    /// parameters in `xform` (`pattern_params`).
    pub const PATTERN: u32 = 12;
    /// Perlin bias (value[0]) then gain (value[1]) of `a`.
    pub const BIAS_GAIN: u32 = 13;
    /// Cylindrical (`mode` bit 2 clear) or Spherical coordinates of
    /// locator vector `a` around axis `mode & 3`: ((u0, v0) + value[0]) ×
    /// value[1..3].
    pub const WRAP: u32 = 14;
    /// The world normal through the 3×3 matrix in `xform[0..9]`, normalised.
    pub const NORMAL: u32 = 15;
    /// A projection face's weight from normal `a`: `mode` = face |
    /// kind << 3 (kind 0 Box signed, 1 Cubic dominant, 2 Triplanar power
    /// value[0]).
    pub const FACE_WEIGHT: u32 = 16;
    /// 1 where normal `a` has component `mode` > 0, else 0.
    pub const FACING: u32 = 17;
    /// Falloff opacity of locator point `a`: `mode` 0 Cubic, 1 Spherical,
    /// 2..4 Linear X/Y/Z; amounts in `value`.
    pub const FALLOFF: u32 = 18;
    /// Three random numbers (value, hue, saturation) for the pattern cell
    /// at vector `a`; `mode` and `xform` as PATTERN.
    pub const CELL_RANDOM: u32 = 19;
    /// Colour `a` with brightness, hue and saturation varied by randoms
    /// `b` by up to value[0..3] (shader_eval::hsv_vary).
    pub const HSV_VARY: u32 = 20;
    pub const PROBE: u32 = 21;
    /// Gradient family: a geometry value (`mode` = `OC_GEOMETRY_*`).
    pub const GEOMETRY: u32 = 40;
    /// Metres from the shading point to `value`.
    pub const DISTANCE: u32 = 41;
    pub const LOCATOR_INCIDENCE: u32 = 48;
    /// A Gradient ramp read at `a` from `value[0]` to `value[1]`: `mode`
    /// stops of `RenderShader::ramps` from `image`; its colour or alpha.
    pub const RAMP_COLOR: u32 = 42;
    pub const RAMP_ALPHA: u32 = 43;
    /// A mesh corner attribute (`RenderShader::attributes[image]`): its
    /// colour or alpha.
    pub const ATTRIBUTE_COLOR: u32 = 44;
    pub const ATTRIBUTE_ALPHA: u32 = 45;
    /// Process: `a` in HSV, hue × value[0] + value[1]°, saturation × value[2].
    pub const HSV: u32 = 46;
    /// Occlusion: the open fraction of `image` rays within value[0] metres;
    /// `mode` bit 0 same object only, bit 1 fired inside.
    pub const OCCLUSION: u32 = 47;
    /// A graph node's output: socket `RenderGraph::sockets[image]` of
    /// node `a` of the surface's node list (node editing, NE-03).
    pub const NODE: u32 = 49;
}

/// A pattern's op mode and parameters for the C bridge (`OC_EXPR_PATTERN`):
/// [subtype, axis, ...]. Checker: transition. Grid: line width,
/// transition. Dots: dot width, transition. Noise: octaves, frequency
/// ratio, amplitude ratio, smooth tops, flow (0/1), flow phase.
pub fn pattern_params(p: &operandi_core::procedural::Pattern, axis: u8) -> (u32, [f32; 12]) {
    p.packed(axis)
}

/// Tiling / image extension codes for the C bridge (`OC_TILE_*`).
pub fn tile_code(t: Tiling) -> u32 {
    match t {
        Tiling::Repeat => 0,
        Tiling::Edge => 1,
        Tiling::Reset => 2,
        Tiling::Mirror => 3,
    }
}

/// Image filtering codes for the C bridge (`OC_FILTER_*`).
fn filter_code(f: operandi_core::shader_tree::Filtering) -> u32 {
    use operandi_core::shader_tree::Filtering;
    match f {
        Filtering::Nearest => 0,
        Filtering::Bilinear => 1,
        Filtering::Bicubic => 2,
    }
}

/// A shader tree surface compiled for Cycles' principled shader.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderShader {
    pub ops: Vec<ShaderOp>,
    /// Per principled input (`shader_eval::Input` order): its node, or -1
    /// to keep Cycles' default.
    pub inputs: [i32; Input::COUNT],
    /// Image file paths the ops read; the same file read with two
    /// filterings is listed twice.
    pub images: Vec<String>,
    /// Per image: its filtering (`OC_FILTER_*`).
    pub image_filters: Vec<u32>,
    /// Per image: 1 when it is read as plain numbers (a normal map), not
    /// as colour.
    pub image_raw: Vec<u32>,
    /// Per image: the node giving its lookup vector (an earlier op), or -1
    /// for the mesh UVs.
    pub image_vectors: Vec<i32>,
    /// Per image: its extension (`OC_TILE_*`).
    pub image_extensions: Vec<u32>,
    /// Gradient ramp stops (RGBA), the RAMP ops' tables back to back.
    pub ramps: Vec<[f32; 4]>,
    /// Corner attributes the ops read (vertex maps, variation numbers).
    pub attributes: Vec<String>,
    /// The UV map the mesh exports for the UV ops: `None` the first map.
    /// One map per surface: when layers ask for different maps the first
    /// one asked wins (provisional, TL-Q04).
    pub uv_map: Option<String>,
    /// Node giving the Bump node's height in scene units (Distance 1,
    /// Strength 1), or -1 for no bump (G-07).
    pub bump: i32,
    /// Node giving the tangent-space normal colour, or -1.
    pub normal_map: i32,
    /// Node giving how far the surface moves along its normal, in scene
    /// units, or -1: no displacement, and no subdivision cost.
    pub displacement: i32,
    /// How much of the bump the clear coat follows (0..1).
    pub coat_bump: f32,
    /// The see-through part (`OC_TRANSPARENCY_*`): 0 none, 1 a Transparent
    /// BSDF, 2 a Glass BSDF, mixed over the principled surface by
    /// `transparency_amount`. The nodes below are -1 for none.
    pub transparency_mode: u32,
    pub transparency_amount: i32,
    pub transparency_color: i32,
    pub refraction_ior: i32,
    pub refraction_roughness: i32,
    pub visible_to_camera: bool,
    pub visible_to_diffuse: bool,
    pub visible_to_glossy: bool,
    pub visible_to_transmission: bool,
    pub cast_shadows: bool,
    pub shadow_catcher: bool,
    /// A light's visible beam (LP-40): the mesh is a volume, not a surface.
    pub volume: Option<VolumeBeam>,
    /// LP-22: the light group a glowing surface counts toward.
    pub lightgroup: Option<String>,
    /// The winning Shader's Alpha Type Constant and fog (shader item v1).
    pub look: SurfaceLook,
    /// Rounded Edge Width in metres (0 = none) and the see-through
    /// absorption (colour, density per metre; density 0 = none).
    pub rounded_edge: f32,
    pub absorption: ([f32; 3], f32),
    /// OpenPBR (OPM-03): the see-through inside's scattering (colour,
    /// density per metre, anisotropy; density 0 = none), Thin Wall, and
    /// the see-through glass built as a principled transmission.
    pub scatter: ([f32; 3], f32, f32),
    pub thin_wall: bool,
    pub glass_principled: bool,
    /// RO-Q03 v2: bit k set where the k-th grouped Render Output's
    /// Material Group applies to this surface.
    pub output_groups: u32,
    /// A Node Material's graph mixed over this surface (NM-31), `None`
    /// for layer-only materials (which then render exactly as before).
    pub node_graph: Option<RenderGraph>,
    /// Per material value (`MaterialValue::ALL` order): the node an output
    /// AOV writes, or -1 (material_values.rs).
    pub values: [i32; MaterialValue::COUNT],
}

/// A node graph for the C bridge's general node list (NM-30).
#[derive(Clone, Debug, PartialEq)]
pub struct RenderGraph {
    /// Cycles nodes and links (node_graph::StoredGraph::flatten);
    /// `FlatValue::Driven(k)` reads op `drivers[k]`.
    pub graph: operandi_core::node_graph::FlatGraph,
    pub drivers: Vec<i32>,
    /// The op giving the Mix Shader factor (the layer's coverage), or -1
    /// for full coverage.
    pub mix: i32,
    /// Output socket names `ShaderOp::NODE` ops read (node editing).
    pub sockets: Vec<String>,
    /// Free nodes already in `graph`, with the index of their first node.
    pub free: Vec<(std::sync::Arc<operandi_core::node_view::NodeOut>, usize)>,
}
impl RenderGraph {
    /// Whether the graph moves the surface (its output's Displacement).
    pub fn displaces(&self) -> bool {
        self.graph.feeding("Displacement").is_some()
    }
}

/// What the winning Shader adds over the surface (shader item v1, §6, §8).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SurfaceLook {
    /// Alpha Type Constant: how much of the surface is held out of the
    /// picture's alpha (1 − Alpha Value); 0 for the other types.
    pub holdout: f32,
    pub fog: Option<Fog>,
}

/// Shader fog (§8): the surface's colour blends toward `color` by a factor
/// from the camera distance; the C side builds it for camera rays only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    /// `OC_FOG_*`: 1 Linear, 2 Exponential, 3 Layered, 4 Underwater.
    pub kind: u32,
    pub color: [f32; 3],
    /// Underwater only: the world colour instead of `color` (SH-5).
    pub use_environment: bool,
    pub start: f32,
    pub end: f32,
    pub density: f32,
}
impl Fog {
    pub fn of(shader: &operandi_core::shader_tree::ShaderChannels) -> Option<Fog> {
        use operandi_core::shader_tree::FogType;
        let m = &shader.more;
        let kind = match m.fog_type {
            FogType::None => return None,
            FogType::Linear => 1,
            FogType::Exponential => 2,
            FogType::Layered => 3,
            FogType::Underwater => 4,
        };
        Some(Fog {
            kind,
            color: m.fog_color.map(|c| c as f32),
            use_environment: kind == 4 && m.fog_use_environment,
            start: m.fog_start as f32,
            end: m.fog_end as f32,
            density: m.fog_density as f32,
        })
    }
}

/// LM-30..32 as a Cycles volume scatter: colour, density and the fade
/// with distance from the light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VolumeBeam {
    pub color: [f32; 3],
    pub density: f32,
    pub attenuation: f32,
    pub origin: [f32; 3],
}
impl RenderShader {
    /// Compiles a tree surface through its MaterialX form (materials
    /// architecture stages 3-4, docs/MATERIALX_GRAPHS.md): the surface is
    /// written as its MaterialX graph, and what Cycles renders is that
    /// graph read back, the same text a scene file saves. This is the only
    /// way a surface reaches Cycles; the direct route from the layers
    /// retired with stage 4. Debug builds check the graph against the
    /// surface it came from; a graph that can't be read back (a writer
    /// bug, which the round-trip tests guard) renders as the default
    /// clay rather than from the layers.
    pub fn compile_through_graph(p: &Principled, scene: &Scene) -> Self {
        use operandi_core::materialx::tree;
        let graph = match tree::read(&tree::write(p, scene, "Surface")) {
            Ok(graph) => {
                debug_assert!(
                    graph == *p,
                    "the tree's MaterialX graph reads back differently"
                );
                graph
            }
            Err(e) => {
                debug_assert!(false, "the tree's MaterialX graph doesn't read back: {e}");
                eprintln!(
                    "Operandi: a material's MaterialX graph doesn't read back ({e}); rendering clay"
                );
                operandi_core::shader_tree::ShaderTree::default()
                    .evaluate(&operandi_core::shader_eval::Context {
                        item: 0,
                        ancestors: &[],
                        material: operandi_core::material::DEFAULT_MATERIAL,
                        part: None,
                        sets: &[],
                        locators: &[],
                    })
                    .principled()
            }
        };
        Self::compile(&graph, scene)
    }

    /// Compiles a surface read from its MaterialX graph; images resolve
    /// through `scene`. Private: callers go through
    /// [`RenderShader::compile_through_graph`].
    fn compile(p: &Principled, scene: &Scene) -> Self {
        let mut out = Self {
            ops: Vec::new(),
            inputs: [-1; Input::COUNT],
            images: Vec::new(),
            image_filters: Vec::new(),
            image_raw: Vec::new(),
            image_vectors: Vec::new(),
            image_extensions: Vec::new(),
            ramps: Vec::new(),
            attributes: Vec::new(),
            uv_map: None,
            bump: -1,
            normal_map: -1,
            displacement: -1,
            coat_bump: p.coat_bump as f32,
            transparency_mode: 0,
            transparency_amount: -1,
            transparency_color: -1,
            refraction_ior: -1,
            refraction_roughness: -1,
            visible_to_camera: p.shader.visible_to_camera,
            visible_to_diffuse: p.shader.visible_to_indirect,
            visible_to_glossy: p.shader.visible_to_reflection,
            visible_to_transmission: p.shader.visible_to_refraction,
            cast_shadows: p.shader.cast_shadows,
            shadow_catcher: p.shader.alpha_type == AlphaType::ShadowCatcher,
            volume: None,
            lightgroup: None,
            look: SurfaceLook {
                holdout: if p.shader.alpha_type == AlphaType::Constant {
                    (1.0 - p.shader.alpha_value.clamp(0.0, 1.0)) as f32
                } else {
                    0.0
                },
                fog: Fog::of(&p.shader),
            },
            rounded_edge: p.rounded_edge.unwrap_or(0.0) as f32,
            absorption: p
                .absorption
                .map_or(([1.0; 3], 0.0), |(c, d)| (c.map(|v| v as f32), d as f32)),
            scatter: p.scatter.map_or(([1.0; 3], 0.0, 0.0), |(c, d, g)| {
                (c.map(|v| v as f32), d as f32, g as f32)
            }),
            thin_wall: p.thin_walled,
            glass_principled: p.glass_principled,
            output_groups: 0,
            node_graph: None,
            values: [-1; MaterialValue::COUNT],
        };
        let mut seen: Vec<(Expr, bool, i32)> = Vec::new();
        // Colour inputs read their pictures as sRGB; numbers (roughness,
        // metal, alpha, heights) read them as plain data, so an 8-bit
        // roughness map isn't darkened by the sRGB curve.
        for (input, expr) in &p.inputs {
            out.inputs[*input as usize] = out.emit(expr, !input.is_colour(), scene, &mut seen);
        }
        if let Some(e) = &p.bump {
            out.bump = out.emit(e, true, scene, &mut seen);
        }
        // Normal maps are directions, not colours: their images are read
        // without the colour conversion.
        if let Some(e) = &p.normal_map {
            out.normal_map = out.emit(e, true, scene, &mut seen);
        }
        if let Some(e) = &p.displacement {
            out.displacement = out.emit(e, true, scene, &mut seen);
        }
        if let Some(s) = &p.see_through {
            out.transparency_amount = out.emit(&s.amount, true, scene, &mut seen);
            out.transparency_color = out.emit(&s.color, false, scene, &mut seen);
            out.transparency_mode = match s.kind {
                SeeThroughKind::Transparent => 1,
                SeeThroughKind::Glass => {
                    out.refraction_ior = out.emit(&s.ior, true, scene, &mut seen);
                    out.refraction_roughness = out.emit(&s.roughness, true, scene, &mut seen);
                    2
                }
            };
        }
        if let Some(n) = &p.node
            && let Some(g) = out.compile_graph(n, scene, &mut seen)
        {
            // Free nodes driving channels (NE-03) came first; the Node
            // Material's nodes follow them.
            match &mut out.node_graph {
                Some(free) => {
                    let at = free.graph.nodes.len();
                    free.graph.nodes.extend(g.graph.nodes);
                    free.graph
                        .links
                        .extend(g.graph.links.into_iter().map(|mut l| {
                            l.from += at;
                            l.to += at;
                            l
                        }));
                    free.drivers = g.drivers;
                    free.mix = g.mix;
                }
                None => out.node_graph = Some(g),
            }
        }
        for (v, e) in &p.values {
            out.values[v.index()] = out.emit(e, !v.is_color(), scene, &mut seen);
        }
        out
    }

    /// The Node Material's graph with its drivers and coverage as ops. A
    /// graph that can't flatten (missing, unknown nodes) renders nothing:
    /// the layers show (NM-03).
    fn compile_graph(
        &mut self,
        n: &operandi_core::shader_eval::NodeUse,
        scene: &Scene,
        seen: &mut Vec<(Expr, bool, i32)>,
    ) -> Option<RenderGraph> {
        let g = scene.shader_tree.node_graphs.get(n.graph)?;
        let mut drivers = Vec::new();
        let mut values = operandi_core::node_graph::ExposedValues {
            values: n.values.clone(),
            ..Default::default()
        };
        for (name, e) in &n.drivers {
            values.drivers.insert(name.clone(), drivers.len());
            drivers.push(self.emit(e, false, scene, seen));
        }
        let graph = g.flatten(&values).ok()?;
        let mix = if n.coverage == Expr::scalar(1.) {
            -1
        } else {
            self.emit(&n.coverage, false, scene, seen)
        };
        Some(RenderGraph {
            graph,
            drivers,
            mix,
            sockets: Vec::new(),
            free: Vec::new(),
        })
    }
    /// Flattens `e` (and what it reads) into `ops`; `raw` reads its images
    /// as plain numbers.
    fn emit(
        &mut self,
        e: &Expr,
        raw: bool,
        scene: &Scene,
        seen: &mut Vec<(Expr, bool, i32)>,
    ) -> i32 {
        if let Some((_, _, i)) = seen.iter().find(|(x, r, _)| x == e && *r == raw) {
            return *i;
        }
        let blank = ShaderOp {
            op: ShaderOp::CONST,
            mode: 0,
            a: -1,
            b: -1,
            fac: -1,
            value: [0.; 3],
            image: -1,
            xform: [0.; 12],
        };
        let op = match e {
            Expr::Const(c) => ShaderOp {
                value: c.map(|v| v as f32),
                ..blank
            },
            // Graph layers render through their own node list (NM-30); a
            // tree surface never holds these viewport sums. Their average
            // if one ever arrives.
            Expr::Func(_) => ShaderOp {
                value: e.eval(&|_, _| [0.5; 4]).map(|v| v as f32),
                ..blank
            },
            // A free node's output (NE-03): its Cycles nodes join the
            // surface's node list once, and the op reads their output.
            Expr::Nodes(n) => {
                let g = self.node_graph.get_or_insert_with(|| RenderGraph {
                    graph: Default::default(),
                    drivers: Vec::new(),
                    mix: -1,
                    sockets: Vec::new(),
                    free: Vec::new(),
                });
                let start = match g.free.iter().find(|(x, _)| **x == **n) {
                    Some((_, at)) => *at,
                    None => {
                        let at = g.graph.nodes.len();
                        g.graph.nodes.extend(n.flat.nodes.iter().cloned());
                        g.graph
                            .links
                            .extend(n.flat.links.iter().cloned().map(|mut l| {
                                l.from += at;
                                l.to += at;
                                l
                            }));
                        g.free.push((n.clone(), at));
                        at
                    }
                };
                let socket = match g.sockets.iter().position(|s| *s == n.at.1) {
                    Some(i) => i,
                    None => {
                        g.sockets.push(n.at.1.clone());
                        g.sockets.len() - 1
                    }
                };
                ShaderOp {
                    op: ShaderOp::NODE,
                    a: (start + n.at.0) as i32,
                    image: socket as i32,
                    ..blank
                }
            }
            Expr::ImageColor(l) | Expr::ImageAlpha(l) => {
                let path = scene
                    .images
                    .iter()
                    .find(|i| i.id == l.image)
                    .map_or(String::new(), |i| i.path.clone());
                let filter = filter_code(l.filtering);
                // The vector comes first, so the image node (made at its
                // first read) only reads earlier nodes.
                let vector = self.emit(&l.vector, raw, scene, seen);
                let extension = tile_code(l.extension);
                let raw = u32::from(raw);
                let image = match (0..self.images.len()).position(|i| {
                    self.images[i] == path
                        && self.image_filters[i] == filter
                        && self.image_raw[i] == raw
                        && self.image_vectors[i] == vector
                        && self.image_extensions[i] == extension
                }) {
                    Some(i) => i,
                    None => {
                        self.images.push(path);
                        self.image_filters.push(filter);
                        self.image_raw.push(raw);
                        self.image_vectors.push(vector);
                        self.image_extensions.push(extension);
                        self.images.len() - 1
                    }
                } as i32;
                ShaderOp {
                    op: if matches!(e, Expr::ImageColor(..)) {
                        ShaderOp::IMAGE_COLOR
                    } else {
                        ShaderOp::IMAGE_ALPHA
                    },
                    image,
                    ..blank
                }
            }
            Expr::Locator(m) => ShaderOp {
                op: ShaderOp::LOCATOR,
                xform: m.map(|v| v as f32),
                ..blank
            },
            Expr::Planar {
                of,
                axis,
                centre,
                repeat,
                mirror,
            } => ShaderOp {
                op: ShaderOp::PLANAR,
                mode: u32::from((*axis).min(2)) | u32::from(*mirror & 3) << 2,
                a: self.emit(of, raw, scene, seen),
                value: [*centre as f32, repeat[0] as f32, repeat[1] as f32],
                ..blank
            },
            Expr::Wrap {
                of,
                axis,
                spherical,
                centre,
                repeat,
            } => ShaderOp {
                op: ShaderOp::WRAP,
                mode: u32::from((*axis).min(2)) | u32::from(*spherical) << 2,
                a: self.emit(of, raw, scene, seen),
                value: [*centre as f32, repeat[0] as f32, repeat[1] as f32],
                ..blank
            },
            Expr::Probe { of, axis, repeat } => ShaderOp {
                op: ShaderOp::PROBE,
                mode: u32::from((*axis).min(2)),
                a: self.emit(of, raw, scene, seen),
                value: [repeat[0] as f32, repeat[1] as f32, 0.],
                ..blank
            },
            Expr::Normal(m) => {
                let mut xform = [0f32; 12];
                for (k, v) in m.iter().enumerate() {
                    xform[k] = *v as f32;
                }
                ShaderOp {
                    op: ShaderOp::NORMAL,
                    xform,
                    ..blank
                }
            }
            Expr::FaceWeight { normal, face, kind } => {
                use operandi_core::shader_eval::FaceKind;
                let (code, k) = match kind {
                    FaceKind::Signed => (0, 0.),
                    FaceKind::Dominant => (1, 0.),
                    FaceKind::Power(k) => (2, *k),
                };
                ShaderOp {
                    op: ShaderOp::FACE_WEIGHT,
                    mode: u32::from((*face).min(5)) | code << 3,
                    a: self.emit(normal, raw, scene, seen),
                    value: [k as f32, 0., 0.],
                    ..blank
                }
            }
            Expr::Facing(normal, axis) => ShaderOp {
                op: ShaderOp::FACING,
                mode: u32::from((*axis).min(2)),
                a: self.emit(normal, raw, scene, seen),
                ..blank
            },
            Expr::Falloff { of, kind, amount } => {
                use operandi_core::texture_locator::FalloffType as F;
                ShaderOp {
                    op: ShaderOp::FALLOFF,
                    mode: match kind {
                        F::Cubic => 0,
                        F::Spherical => 1,
                        F::LinearX => 2,
                        F::LinearY => 3,
                        F::LinearZ => 4,
                    },
                    a: self.emit(of, raw, scene, seen),
                    value: amount.map(|v| v as f32),
                    ..blank
                }
            }
            Expr::MeshUv {
                map,
                repeat,
                rotation,
            } => {
                if self.uv_map.is_none() {
                    self.uv_map = map.clone();
                }
                ShaderOp {
                    op: ShaderOp::UV,
                    value: [
                        repeat[0] as f32,
                        repeat[1] as f32,
                        rotation.to_radians() as f32,
                    ],
                    ..blank
                }
            }
            Expr::Tile(a, tiling) => ShaderOp {
                op: ShaderOp::TILE,
                mode: tile_code(tiling[0]) | tile_code(tiling[1]) << 2,
                a: self.emit(a, raw, scene, seen),
                ..blank
            },
            Expr::Inside(a, clip) => ShaderOp {
                op: ShaderOp::INSIDE,
                mode: u32::from(clip[0]) | u32::from(clip[1]) << 1,
                a: self.emit(a, raw, scene, seen),
                ..blank
            },
            Expr::Pattern(p) => {
                let (mode, params) = pattern_params(&p.pattern, p.axis);
                ShaderOp {
                    op: ShaderOp::PATTERN,
                    mode,
                    a: self.emit(&p.vector, raw, scene, seen),
                    xform: params,
                    ..blank
                }
            }
            Expr::CellRandom(p) => {
                let (mode, params) = pattern_params(&p.pattern, p.axis);
                ShaderOp {
                    op: ShaderOp::CELL_RANDOM,
                    mode,
                    a: self.emit(&p.vector, raw, scene, seen),
                    xform: params,
                    ..blank
                }
            }
            Expr::HsvVary {
                color,
                random,
                amounts,
            } => ShaderOp {
                op: ShaderOp::HSV_VARY,
                a: self.emit(color, raw, scene, seen),
                b: self.emit(random, raw, scene, seen),
                value: amounts.map(|v| v as f32),
                ..blank
            },
            Expr::BiasGain(a, bias, gain) => ShaderOp {
                op: ShaderOp::BIAS_GAIN,
                a: self.emit(a, raw, scene, seen),
                value: [*bias as f32, *gain as f32, 0.],
                ..blank
            },
            // A picture read through Luma is a number: an image on a
            // number effect (Diffuse Amount, a mask...) is always read that
            // way in shader_eval, even where it is folded into a colour
            // input, as Diffuse Amount is into Base Color. Its picture
            // stays plain data, not sRGB-decoded.
            Expr::Luma(a) => ShaderOp {
                op: ShaderOp::LUMA,
                a: self.emit(a, true, scene, seen),
                ..blank
            },
            Expr::Channel(a, k) => ShaderOp {
                op: ShaderOp::CHANNEL,
                mode: u32::from(*k),
                a: self.emit(a, raw, scene, seen),
                ..blank
            },
            Expr::Gamma(a, power) => ShaderOp {
                op: ShaderOp::GAMMA,
                a: self.emit(a, raw, scene, seen),
                value: [*power as f32; 3],
                ..blank
            },
            Expr::Mix { mode, fac, a, b } => ShaderOp {
                op: ShaderOp::MIX,
                mode: mode.cycles_code(),
                fac: self.emit(fac, raw, scene, seen),
                a: self.emit(a, raw, scene, seen),
                b: self.emit(b, raw, scene, seen),
                ..blank
            },
            Expr::Attribute { name, alpha } => {
                let at = match self.attributes.iter().position(|a| a == name) {
                    Some(i) => i,
                    None => {
                        self.attributes.push(name.clone());
                        self.attributes.len() - 1
                    }
                };
                ShaderOp {
                    op: if *alpha {
                        ShaderOp::ATTRIBUTE_ALPHA
                    } else {
                        ShaderOp::ATTRIBUTE_COLOR
                    },
                    image: at as i32,
                    ..blank
                }
            }
            Expr::Hsv {
                of,
                hue_scale,
                hue_shift,
                saturation_scale,
            } => ShaderOp {
                op: ShaderOp::HSV,
                a: self.emit(of, raw, scene, seen),
                value: [
                    *hue_scale as f32,
                    *hue_shift as f32,
                    *saturation_scale as f32,
                ],
                ..blank
            },
            Expr::Occlusion {
                rays,
                distance,
                same_surface,
                inside,
                toward,
            } => ShaderOp {
                op: ShaderOp::OCCLUSION,
                mode: u32::from(*same_surface) | u32::from(*inside) << 1 | toward.code() << 2,
                value: [*distance as f32, 0.0, 0.0],
                image: (*rays).min(1024) as i32,
                ..blank
            },
            Expr::Geometry(g) => ShaderOp {
                op: ShaderOp::GEOMETRY,
                mode: g.code(),
                ..blank
            },
            Expr::LocatorIncidence(p) => ShaderOp {
                op: ShaderOp::LOCATOR_INCIDENCE,
                value: p.map(|v| v as f32),
                ..blank
            },
            Expr::DistanceTo(p) => ShaderOp {
                op: ShaderOp::DISTANCE,
                value: p.map(|v| v as f32),
                ..blank
            },
            Expr::Ramp {
                input,
                lo,
                hi,
                stops,
                alpha,
            } => {
                let a = self.emit(input, raw, scene, seen);
                let table: Vec<[f32; 4]> = stops.iter().map(|s| s.map(|v| v as f32)).collect();
                // The colour and alpha reads of one curve share a table.
                let start = self
                    .ramps
                    .windows(table.len().max(1))
                    .position(|w| w == table.as_slice())
                    .unwrap_or_else(|| {
                        self.ramps.extend(&table);
                        self.ramps.len() - table.len()
                    });
                ShaderOp {
                    op: if *alpha {
                        ShaderOp::RAMP_ALPHA
                    } else {
                        ShaderOp::RAMP_COLOR
                    },
                    mode: table.len() as u32,
                    a,
                    value: [*lo as f32, *hi as f32, 0.0],
                    image: start as i32,
                    ..blank
                }
            }
        };
        self.ops.push(op);
        let i = (self.ops.len() - 1) as i32;
        seen.push((e.clone(), raw, i));
        i
    }
    /// An input's value when it is a plain constant.
    pub fn constant(&self, input: Input) -> Option<[f32; 3]> {
        let i = self.inputs[input as usize];
        let op = self.ops.get(usize::try_from(i).ok()?)?;
        (op.op == ShaderOp::CONST).then_some(op.value)
    }
    /// Every node reads only nodes before it, images exist, numbers are
    /// finite: what the C bridge relies on.
    pub fn is_valid(&self) -> bool {
        let earlier = |i: usize, r: i32| r >= 0 && (r as usize) < i;
        self.ops.iter().enumerate().all(|(i, o)| {
            o.value.iter().chain(&o.xform).all(|v| v.is_finite())
                && match o.op {
                    ShaderOp::CONST | ShaderOp::LOCATOR | ShaderOp::UV => true,
                    ShaderOp::IMAGE_COLOR | ShaderOp::IMAGE_ALPHA => {
                        o.image >= 0
                            && (o.image as usize) < self.images.len()
                            && self
                                .image_vectors
                                .get(o.image as usize)
                                .is_some_and(|&v| v == -1 || earlier(i, v))
                    }
                    ShaderOp::LUMA | ShaderOp::GAMMA => earlier(i, o.a),
                    ShaderOp::PLANAR => earlier(i, o.a) && (o.mode & 3) < 3 && o.mode < 16,
                    ShaderOp::WRAP => earlier(i, o.a) && (o.mode & 3) < 3 && o.mode < 8,
                    ShaderOp::PROBE => earlier(i, o.a) && o.mode < 3,
                    ShaderOp::NORMAL => true,
                    ShaderOp::FACE_WEIGHT => earlier(i, o.a) && (o.mode & 7) < 6 && o.mode < 24,
                    ShaderOp::FACING => earlier(i, o.a) && o.mode < 3,
                    ShaderOp::FALLOFF => earlier(i, o.a) && o.mode < 5,
                    ShaderOp::TILE => earlier(i, o.a) && o.mode < 16,
                    ShaderOp::INSIDE => earlier(i, o.a) && o.mode < 4,
                    ShaderOp::PATTERN | ShaderOp::CELL_RANDOM => earlier(i, o.a) && o.mode < 10,
                    ShaderOp::HSV_VARY => earlier(i, o.a) && earlier(i, o.b),
                    ShaderOp::BIAS_GAIN => earlier(i, o.a),
                    ShaderOp::CHANNEL => earlier(i, o.a) && o.mode < 3,
                    ShaderOp::MIX => earlier(i, o.a) && earlier(i, o.b) && earlier(i, o.fac),
                    ShaderOp::GEOMETRY => o.mode < 7,
                    ShaderOp::DISTANCE | ShaderOp::LOCATOR_INCIDENCE => true,
                    ShaderOp::HSV => earlier(i, o.a),
                    ShaderOp::OCCLUSION => o.mode < 16 && (1..=1024).contains(&o.image),
                    ShaderOp::ATTRIBUTE_COLOR | ShaderOp::ATTRIBUTE_ALPHA => {
                        o.image >= 0 && (o.image as usize) < self.attributes.len()
                    }
                    // Checked against the node list below.
                    ShaderOp::NODE => true,
                    ShaderOp::RAMP_COLOR | ShaderOp::RAMP_ALPHA => {
                        earlier(i, o.a)
                            && o.mode >= 2
                            && o.image >= 0
                            && o.image as usize + o.mode as usize <= self.ramps.len()
                    }
                    _ => false,
                }
        }) && self.ramps.iter().flatten().all(|v| v.is_finite())
            && self.image_filters.len() == self.images.len()
            && self.image_raw.len() == self.images.len()
            && self.image_vectors.len() == self.images.len()
            && self.image_extensions.len() == self.images.len()
            && self.image_extensions.iter().all(|&e| e < 4)
            && self.image_filters.iter().all(|&f| f < 3)
            && self.coat_bump.is_finite()
            && self.transparency_mode < 3
            && (self.transparency_mode == 0
                || (self.transparency_amount >= 0 && self.transparency_color >= 0))
            && (self.transparency_mode != 2
                || (self.refraction_ior >= 0 && self.refraction_roughness >= 0))
            && self
                .inputs
                .iter()
                .chain([
                    &self.bump,
                    &self.normal_map,
                    &self.displacement,
                    &self.transparency_amount,
                    &self.transparency_color,
                    &self.refraction_ior,
                    &self.refraction_roughness,
                ])
                .chain(
                    self.node_graph
                        .iter()
                        .flat_map(|g| g.drivers.iter().chain([&g.mix])),
                )
                .all(|&i| i == -1 || (i >= 0 && (i as usize) < self.ops.len()))
            && self.ops.iter().all(|o| {
                o.op != ShaderOp::NODE
                    || self.node_graph.as_ref().is_some_and(|g| {
                        o.a >= 0
                            && (o.a as usize) < g.graph.nodes.len()
                            && o.image >= 0
                            && (o.image as usize) < g.sockets.len()
                    })
            })
            && self.node_graph.as_ref().is_none_or(|g| {
                let n = g.graph.nodes.len();
                g.graph.links.iter().all(|l| l.from < n && l.to < n)
                    && g.graph
                        .nodes
                        .iter()
                        .flat_map(|x| &x.values)
                        .all(|(_, v)| match v {
                            operandi_core::node_graph::FlatValue::Driven(k) => *k < g.drivers.len(),
                            operandi_core::node_graph::FlatValue::Float(x) => x.is_finite(),
                            operandi_core::node_graph::FlatValue::Vec3(x) => {
                                x.iter().all(|c| c.is_finite())
                            }
                            operandi_core::node_graph::FlatValue::Ramp(t) => {
                                t.iter().flatten().all(|c| c.is_finite())
                            }
                            _ => true,
                        })
            })
    }
    pub fn reads_images(&self) -> bool {
        !self.images.is_empty()
    }
    /// The mesh must carry UVs: something reads them (UV Map
    /// projection), or a normal map needs its tangent space.
    pub fn needs_uvs(&self) -> bool {
        // A node graph may read UVs anywhere (Texture Coordinate, image
        // textures, normal maps); always give it them.
        self.ops.iter().any(|o| o.op == ShaderOp::UV)
            || self.normal_map >= 0
            || self.node_graph.is_some()
    }
    pub fn displaces(&self) -> bool {
        self.displacement >= 0 || self.node_graph.as_ref().is_some_and(RenderGraph::displaces)
    }
}

/// Where the camera is and how wide it sees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub position: DVec3,
    pub right: DVec3,
    pub up: DVec3,
    /// Points into the scene.
    pub forward: DVec3,
    /// Tangent of half the vertical field of view.
    pub half_height_tan: f64,
    /// An orthographic camera: half the picture's height in metres.
    pub orthographic: Option<f64>,
    /// Film offset: the picture moved by this many half-widths (x) and
    /// half-heights (y).
    pub shift: [f64; 2],
    /// Near and far clip distances.
    pub clip: (f64, f64),
    /// CP-V13: a Spherical or Cylindrical camera; `None` for perspective
    /// and orthographic.
    pub panorama: Option<Panorama>,
    /// CP-E01 depth of field; `None` renders everything sharp.
    pub dof: Option<Dof>,
}

/// A panoramic camera (CAM-Q01, CAM-Q02).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panorama {
    /// Cylindrical (360° around, height from the picture's shape) rather
    /// than Spherical (equirectangular).
    pub cylindrical: bool,
    /// The VR variants: the view direction at the picture's centre. The
    /// others put the camera's back there.
    pub vr: bool,
}

impl Panorama {
    /// The Cycles ranges (`OcSettings::panorama_range`) for a picture of
    /// `width` × `height` pixels.
    pub fn range(&self, width: u32, height: u32) -> [f64; 4] {
        use std::f64::consts::{FRAC_PI_2, PI};
        // Longitude runs from max at the left to min at the right (Cycles'
        // convention), centred on min + max = 0 (forward) or −2π (back).
        let (lo, hi) = if self.vr { (-PI, PI) } else { (-2.0 * PI, 0.0) };
        if self.cylindrical {
            // Square pixels: the height on the unit cylinder spans
            // 2π × height ÷ width.
            let half = PI * f64::from(height) / f64::from(width.max(1));
            [lo, hi, -half, half]
        } else {
            [lo, hi, -FRAC_PI_2, FRAC_PI_2]
        }
    }
}

/// Depth of field for Cycles (CP-E01..E06).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dof {
    /// Metres: Focal Length ÷ (2 × F-Stop).
    pub aperture_radius: f64,
    pub focus_distance: f64,
    /// 0 = round.
    pub blades: u32,
    /// Radians.
    pub blades_rotation: f64,
}

impl View {
    /// A pinhole view with no offset and the default clip range.
    pub fn perspective(
        position: DVec3,
        right: DVec3,
        up: DVec3,
        forward: DVec3,
        half_height_tan: f64,
    ) -> Self {
        Self {
            position,
            right,
            up,
            forward,
            half_height_tan,
            orthographic: None,
            shift: [0.0, 0.0],
            clip: DEFAULT_CLIP,
            panorama: None,
            dof: None,
        }
    }
}

/// Clip range when a camera's Clipping is off.
pub const DEFAULT_CLIP: (f64, f64) = (0.01, 1.0e5);

/// How a scene light is shaped in Cycles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightShape {
    /// Angular diameter, radians (the light's Spread Angle).
    Sun {
        angle: f32,
    },
    Point {
        radius: f32,
    },
    /// `size` is the full cone angle; `blend` the soft fraction (0..1).
    Spot {
        radius: f32,
        size: f32,
        blend: f32,
    },
    /// Width and height in metres along the light's right and up axes.
    Area {
        size: [f32; 2],
        ellipse: bool,
    },
    /// LP-PO: a Cycles area-light portal of that size. It makes no light;
    /// it guides how the world's light is sampled through it.
    Portal {
        size: [f32; 2],
        ellipse: bool,
    },
}

/// One scene light for Cycles.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneLight {
    pub shape: LightShape,
    pub position: DVec3,
    pub right: DVec3,
    pub up: DVec3,
    /// Direction the light travels (the light item's -Z in world space).
    pub direction: DVec3,
    /// Strength per colour channel: colour × power × (1 − dissolve).
    pub strength: [f32; 3],
    pub cast_shadow: bool,
    pub visible_to_camera: bool,
    pub affects_diffuse: bool,
    pub affects_specular: bool,
    /// LP-20 / LP-21: the light's shape is seen in reflections /
    /// through refraction. Cycles can't tell the two apart: either one
    /// lets every bounced ray hit the light (multiple importance sampling
    /// on), provisional (LPROP-Q05).
    pub visible_to_reflection: bool,
    pub visible_to_refraction: bool,
    /// LP-PH: the photometric light's IES file text; Cycles shapes the
    /// light by it (the strength then multiplies the file's own watts).
    pub ies: Option<std::sync::Arc<str>>,
    /// LP-22: the light group (its own pass) the light counts toward.
    pub lightgroup: Option<String>,
    /// Light-material image layers projected through a spot's cone:
    /// image, Cycles Mix type, opacity.
    pub layers: Vec<(String, u32, f32)>,
    /// tan(half the cone) for the layers' projection; 0 = none.
    pub gobo_tan_half: f32,
    /// LP-30..32: the receiver light sets it lights (and whose blockers
    /// shadow it); `None` = all.
    pub light_membership: Option<u64>,
    /// The sets whose blockers shadow it when that differs from
    /// `light_membership` (a shader item v1 light copy); `None` = the same.
    pub shadow_membership: Option<u64>,
    /// Direct Caustics: this light only casts caustics through glass
    /// (a copy of a scene light at its Affect Caustics share).
    pub caustics_only: bool,
}

/// What a surface's winning Shader asks of the lights (shader item v1):
/// Light Linking (§7), the Direct Illum Multiplier (§4) and Receive
/// Shadows (§5).
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderLightRule {
    /// Include or Exclude, and the light group's members.
    pub link: Option<(LinkMode, Vec<u64>)>,
    pub direct: f64,
    pub receive_shadows: bool,
}
impl Default for ShaderLightRule {
    fn default() -> Self {
        Self {
            link: None,
            direct: 1.0,
            receive_shadows: true,
        }
    }
}
impl ShaderLightRule {
    /// The rule of `shader`; a light group that is gone links nothing.
    pub fn of(shader: &ShaderChannels, scene: &Scene) -> Self {
        let m = &shader.more;
        Self {
            link: m
                .light_group
                .filter(|g| scene.group(*g).is_some())
                .map(|g| (m.link_mode, scene.group_members(g))),
            direct: m.direct_multiplier.max(0.0),
            receive_shadows: shader.receive_shadows,
        }
    }
    pub fn is_plain(&self) -> bool {
        *self == Self::default()
    }
    /// Whether light item `light` lights these surfaces at all.
    fn lights(&self, light: u64) -> bool {
        match &self.link {
            None => true,
            Some((LinkMode::Include, members)) => members.contains(&light),
            Some((LinkMode::Exclude, members)) => !members.contains(&light),
        }
    }
    /// The light copy these surfaces need: (strength factor, shadows).
    fn variant(&self) -> (u64, bool) {
        (self.direct.to_bits(), self.receive_shadows)
    }
}

/// One extra copy of a light for surfaces whose shader scales direct
/// light or ignores shadows: the light's strength × `direct`, lighting
/// only `membership`; with `receive_shadows` off nothing shadows it.
#[derive(Clone, Debug, PartialEq)]
pub struct LightVariant {
    pub light: u64,
    pub direct: f64,
    pub receive_shadows: bool,
    pub membership: u64,
}

/// LP-30..32 Light Linking as Cycles light sets. Each linked light is one
/// rule; every item gets the receiver set of the rules that leave it out
/// (set 0 = no rule leaves it out). A light lights a set unless its own rule
/// leaves that set's items out. Shadows follow the same sets, so an item a
/// light doesn't light doesn't cast that light's shadow either (GRP-Q06).
/// Cycles has 64 sets; items past the 63rd distinct combination share set
/// 0 (`light_notes` says so).
///
/// Shader item v1: a surface's winning Shader can add its own Light
/// Linking, Direct Illum Multiplier and Receive Shadows off. Those surfaces
/// get receiver sets of their own (`surfaces`); every light then has a
/// rule, and each (multiplier, shadows) combination other than the plain
/// one gets copies of the lights (`variants`) that light only those sets.
/// Approximations: the default studio sun, cylinder and mesh lights, and
/// the world are not copied (they light such surfaces at full strength).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkSets {
    pub receiver: std::collections::BTreeMap<u64, u32>,
    /// Receiver sets of surfaces with a non-plain shader rule.
    pub surfaces: Vec<(u64, ShaderLightRule, u32)>,
    pub membership: std::collections::BTreeMap<u64, u64>,
    /// The sets whose blockers shadow each light: every set it lights,
    /// whatever the variant. Empty when it equals `membership`.
    pub shadow: std::collections::BTreeMap<u64, u64>,
    pub variants: Vec<LightVariant>,
    pub overflow: bool,
}

impl LinkSets {
    /// The receiver set of item `item`'s surfaces under `rule`.
    pub fn set_for(&self, item: u64, rule: &ShaderLightRule) -> u32 {
        if !rule.is_plain()
            && let Some((.., set)) = self
                .surfaces
                .iter()
                .find(|(i, r, _)| *i == item && r == rule)
        {
            return *set;
        }
        self.receiver.get(&item).copied().unwrap_or(0)
    }
}

/// The distinct non-plain shader rules on each mesh item's polygons.
fn shader_rules(scene: &Scene) -> Vec<(u64, ShaderLightRule)> {
    let tree = scene.shading_tree();
    let special = tree.nodes.iter().any(|n| match &n.kind {
        NodeKind::Shader(s) => !ShaderLightRule::of(s, scene).is_plain(),
        _ => false,
    });
    if !special {
        return Vec::new();
    }
    let mut out = Vec::new();
    for item in scene.items.iter().filter(|i| i.is_mesh()) {
        let ancestors = scene.item_ancestors(item.id);
        let locators = scene.locator_frames(item.id);
        let mut sets: Vec<Vec<&str>> = vec![Vec::new(); item.mesh.faces.len()];
        for (name, faces) in &item.mesh.attributes.polygon_sets {
            for &f in faces {
                if let Some(list) = sets.get_mut(f as usize) {
                    list.push(name.as_str());
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        for (face, sets) in item.mesh.faces.iter().zip(&sets) {
            if !seen.insert((face.material.as_str(), face.part.as_deref(), sets.clone())) {
                continue;
            }
            let ctx = Context {
                item: item.id,
                ancestors: &ancestors,
                material: &face.material,
                part: face.part.as_deref(),
                sets,
                locators: &locators,
            };
            let surface = tree.evaluate(&ctx);
            // A saved graph pinned here brings its own Shader settings.
            let shader = if scene.material_pins.is_empty() {
                surface.shader
            } else {
                operandi_core::materialx::saved::surface(scene, &ctx, surface.principled()).shader
            };
            let rule = ShaderLightRule::of(&shader, scene);
            if !rule.is_plain() && !out.iter().any(|(i, r)| *i == item.id && *r == rule) {
                out.push((item.id, rule));
            }
        }
    }
    out
}

pub fn link_sets(scene: &Scene) -> LinkSets {
    let surface_rules = shader_rules(scene);
    // The lights that render, each with its own linking rule (if any) and
    // its group's members.
    type Rule<'a> = (
        u64,
        Option<&'a operandi_core::camera_light::LightLinking>,
        Option<Vec<u64>>,
    );
    let rules: Vec<Rule> = scene
        .items
        .iter()
        .filter_map(|i| {
            let l = i.light.as_ref()?;
            if !l.renders(scene.shows(i)) || !l.kind.emits() {
                return None;
            }
            let link = l.linking.as_ref().filter(|k| k.control);
            // Without shader rules only linked lights need sets.
            if link.is_none() && surface_rules.is_empty() {
                return None;
            }
            let members = link
                .and_then(|k| k.group)
                .filter(|g| scene.group(*g).is_some())
                .map(|g| scene.group_members(g));
            Some((i.id, link, members))
        })
        .collect();
    let mut out = LinkSets::default();
    if rules.is_empty() {
        return out;
    }
    let plain = ShaderLightRule::default();
    // Every receiver: each item under the plain rule, then the surfaces
    // with rules of their own.
    let receivers: Vec<(u64, &ShaderLightRule)> = scene
        .items
        .iter()
        .map(|i| (i.id, &plain))
        .chain(surface_rules.iter().map(|(i, r)| (*i, r)))
        .collect();
    // A set is a pattern of lights left out plus the light copy it needs.
    type Pattern = (Vec<bool>, (u64, bool));
    let mut patterns: Vec<Pattern> = vec![(vec![false; rules.len()], plain.variant())];
    for (item, rule) in receivers {
        let left_out: Vec<bool> = rules
            .iter()
            .map(|(light, link, members)| {
                link.is_some_and(|k| !k.lights(item, members.as_deref())) || !rule.lights(*light)
            })
            .collect();
        let pattern = (left_out, rule.variant());
        let set = match patterns.iter().position(|p| *p == pattern) {
            Some(s) => s,
            None if patterns.len() < 64 => {
                patterns.push(pattern);
                patterns.len() - 1
            }
            None => {
                out.overflow = true;
                0
            }
        };
        if rule.is_plain() {
            out.receiver.insert(item, set as u32);
        } else {
            out.surfaces.push((item, rule.clone(), set as u32));
        }
    }
    let mut variants: Vec<(u64, bool)> = Vec::new();
    for (_, v) in &patterns {
        if *v != plain.variant() && !variants.contains(v) {
            variants.push(*v);
        }
    }
    for (k, (light, ..)) in rules.iter().enumerate() {
        let lit = |want: Option<(u64, bool)>| {
            let mut mask = 0u64;
            for (s, (p, v)) in patterns.iter().enumerate() {
                if !p[k] && want.is_none_or(|w| w == *v) {
                    mask |= 1 << s;
                }
            }
            mask
        };
        out.membership.insert(*light, lit(Some(plain.variant())));
        if !variants.is_empty() {
            out.shadow.insert(*light, lit(None));
        }
        for &(direct, receive_shadows) in &variants {
            let membership = lit(Some((direct, receive_shadows)));
            if membership != 0 {
                out.variants.push(LightVariant {
                    light: *light,
                    direct: f64::from_bits(direct),
                    receive_shadows,
                    membership,
                });
            }
        }
    }
    out
}

/// LM-01 / LM-04: the light material's Enable and Opacity scale the light
/// (with one material, Opacity dims it; off is off).
pub fn material_factor(light: &LightSettings) -> f64 {
    if light.stored.material_enabled {
        light.stored.opacity.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Cycles Mix Color type (NodeMix) for a light blend mode.
pub fn mix_type(blend: operandi_core::camera_light::LightBlend) -> u32 {
    use operandi_core::camera_light::LightBlend as B;
    match blend {
        B::Normal => 0,
        B::Add => 1,
        B::Multiply => 2,
        B::Subtract => 3,
        B::Screen => 4,
        B::Overlay => 9,
    }
}

/// LP-DM-01: the scene's dome lights as one even world radiance (linear
/// RGB): colour × Radiance × (1 − Dissolve), added to the world for every
/// ray but camera rays.
pub fn scene_dome(scene: &Scene) -> [f32; 3] {
    let mut out = [0.0f64; 3];
    for item in &scene.items {
        let Some(l) = item.light.as_ref() else {
            continue;
        };
        if l.kind != LightKind::Dome || !l.renders(scene.shows(item)) {
            continue;
        }
        let k = l.intensity * material_factor(l) * (1.0 - l.dissolve);
        for (o, c) in out.iter_mut().zip(l.color) {
            *o += c * k;
        }
    }
    out.map(|v| v as f32)
}

/// LP-22: the LPE Labels of lights that render, each once, in item order.
pub fn scene_light_groups(scene: &Scene) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in &scene.items {
        if let Some(l) = item.light.as_ref()
            && l.renders(scene.shows(item))
            && l.kind.has_visible_rows()
            && !l.lpe_label.is_empty()
            && !out.contains(&l.lpe_label)
        {
            out.push(l.lpe_label.clone());
        }
    }
    out
}

/// Segments round a beam volume.
const BEAM_SEGMENTS: usize = 32;

/// LP-40..44: each light's visible beam as a closed volume mesh — a
/// cylinder down a directional light's −Z (Radius × Height), a cone from a
/// spot (Base widening by the Cone Angle over Height), a sphere round a
/// point or photometric light (Sphere Radius) — filled with Scattering
/// Color at Density, fading by Light Attenuation per metre from the light.
/// Light Shift isn't rendered (LPROP-Q20).
pub fn light_volumes(scene: &Scene) -> Result<Vec<RenderMesh>> {
    let mut out = Vec::new();
    for item in &scene.items {
        let Some(l) = item.light.as_ref() else {
            continue;
        };
        let Some(v) = l
            .volumetrics
            .as_ref()
            .filter(|v| v.enabled && v.density > 0.0)
        else {
            continue;
        };
        if !l.renders(scene.shows(item)) || !l.kind.has_volumetrics() {
            continue;
        }
        let world = scene.world_matrix(item.id)?;
        let origin = world.transform_point3(DVec3::ZERO);
        let (rings, top, bottom): (Vec<(f64, f64)>, bool, bool) = match l.kind {
            LightKind::Directional => (vec![(0.0, v.radius), (-v.height, v.radius)], true, true),
            LightKind::Spot => {
                let spread = (l.cone_angle / 2.0).min(1.5).tan();
                (
                    vec![(0.0, v.radius), (-v.height, v.radius + v.height * spread)],
                    true,
                    true,
                )
            }
            _ => {
                let n = 16;
                let rings = (1..n)
                    .map(|k| {
                        let a = std::f64::consts::PI * k as f64 / n as f64;
                        (v.sphere_radius * a.cos(), v.sphere_radius * a.sin())
                    })
                    .collect();
                (rings, false, false)
            }
        };
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut triangles: Vec<[u32; 3]> = Vec::new();
        let p = |x: f64, y: f64, z: f64| {
            world
                .transform_point3(DVec3::new(x, y, z))
                .as_vec3()
                .to_array()
        };
        let n = BEAM_SEGMENTS as u32;
        for (z, r) in &rings {
            for k in 0..BEAM_SEGMENTS {
                let a = k as f64 / BEAM_SEGMENTS as f64 * std::f64::consts::TAU;
                positions.push(p(r * a.cos(), r * a.sin(), *z));
            }
        }
        for ring in 0..rings.len() as u32 - 1 {
            for k in 0..n {
                let (a, b) = (ring * n + k, ring * n + (k + 1) % n);
                let (c, d) = (a + n, b + n);
                triangles.push([a, c, d]);
                triangles.push([a, d, b]);
            }
        }
        // Caps (cylinder, cone) or poles (sphere) close the volume.
        let last = (rings.len() as u32 - 1) * n;
        let (first_z, last_z) = (rings[0].0, rings[rings.len() - 1].0);
        let close = |positions: &mut Vec<[f32; 3]>,
                     triangles: &mut Vec<[u32; 3]>,
                     z: f64,
                     start: u32,
                     flip: bool| {
            let centre = positions.len() as u32;
            positions.push(p(0.0, 0.0, z));
            for k in 0..n {
                let (a, b) = (start + k, start + (k + 1) % n);
                triangles.push(if flip { [centre, b, a] } else { [centre, a, b] });
            }
        };
        if top {
            close(&mut positions, &mut triangles, first_z, 0, true);
        } else {
            close(&mut positions, &mut triangles, v.sphere_radius, 0, true);
        }
        if bottom {
            close(&mut positions, &mut triangles, last_z, last, false);
        } else {
            close(
                &mut positions,
                &mut triangles,
                -v.sphere_radius,
                last,
                false,
            );
        }
        let smooth = vec![0; triangles.len()];
        out.push(RenderMesh {
            positions,
            triangles,
            smooth,
            uvs: Vec::new(),
            light_set: 0,
            light_membership: None,
            attributes: Vec::new(),
            normals: Vec::new(),
            transform: None,
            name: item.name.clone(),
            asset_name: item.name.clone(),
            material_name: item.name.clone(),
            segment_color: operandi_core::render_outputs::segment_color(item.id, ""),
            shader: Some(RenderShader {
                ops: Vec::new(),
                inputs: [-1; Input::COUNT],
                images: Vec::new(),
                image_filters: Vec::new(),
                // A beam is a volume only: no surface relief or see-through part.
                image_raw: Vec::new(),
                image_vectors: Vec::new(),
                image_extensions: Vec::new(),
                ramps: Vec::new(),
                attributes: Vec::new(),
                uv_map: None,
                bump: -1,
                normal_map: -1,
                displacement: -1,
                coat_bump: 0.0,
                transparency_mode: 0,
                transparency_amount: -1,
                transparency_color: -1,
                refraction_ior: -1,
                refraction_roughness: -1,
                visible_to_camera: true,
                visible_to_diffuse: true,
                visible_to_glossy: true,
                visible_to_transmission: true,
                cast_shadows: true,
                shadow_catcher: false,
                volume: Some(VolumeBeam {
                    color: v.color.map(|c| c as f32),
                    density: v.density as f32,
                    attenuation: v.attenuation as f32,
                    origin: origin.as_vec3().to_array(),
                }),
                lightgroup: None,
                look: SurfaceLook::default(),
                rounded_edge: 0.0,
                absorption: ([1.0; 3], 0.0),
                scatter: ([1.0; 3], 0.0, 0.0),
                thin_wall: false,
                glass_principled: false,
                output_groups: 0,
                node_graph: None,
                values: [-1; MaterialValue::COUNT],
            }),
        });
    }
    Ok(out)
}

/// One environment of the render world (environments-2026-11-09, "ENV"):
/// its Environment Material, Intensity, which ray kinds see it, and its
/// item's up axis. `Settings::world` lists them newest first; each ray kind
/// sees the first one visible to it (ENV §6).
#[derive(Clone, Debug, PartialEq)]
pub struct EnvironmentLayer {
    /// Type, colours and exponents (ENV §3).
    pub material: operandi_core::environment::EnvironmentMaterial,
    /// Multiplies the colour (the Background node's Strength).
    pub intensity: f32,
    /// The gradients' "up" in world space (unit). Always +Y for a scene's
    /// environment: it has no transform (ENVIRONMENT-PLACEMENT §2).
    pub up: DVec3,
    /// Seen by camera, indirect (diffuse), reflection (glossy) and
    /// refraction (transmission) rays, in `RayKind::ALL` order.
    pub visible: [bool; 4],
    /// The Environment Material's Opacity and Invert: it blends over black.
    pub material_opacity: f32,
    pub material_invert: bool,
    /// Texture layers over the material, bottom first (ENV §5).
    pub layers: Vec<EnvironmentTexture>,
    /// World directions into the sky's frame (texture layers carry their
    /// own `to_image`). Identity for a scene's environment: it has no
    /// transform (ENVIRONMENT-PLACEMENT §2).
    pub item_inverse: glam::DMat3,
    /// Physically-based Daylight with a Sun Light: Cycles' Sky Texture
    /// (ENV §4). `None` draws the material's 4 Color Gradient.
    pub sky: Option<PhysicalSky>,
}

/// Cycles' single-scattering ("Nishita") sky for Physically-based
/// Daylight (ENV §4, Q-ENV1: its colours are Cycles' own).
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalSky {
    /// World direction toward the sun: opposite the Sun Light's travel
    /// direction (the light's -Z) [chosen: the sun sits where its light
    /// comes from].
    pub toward_sun: DVec3,
    /// The sun's angular diameter, radians: Solar Disc Size × 0.545°
    /// [chosen, Cycles' default sun], 0 = no disc.
    pub sun_size: f32,
    /// Ground Albedo's mean.
    pub ground_albedo: f32,
    /// Aerosol (dust) density: Haze Amount / 2, so the default haze 2 is
    /// Cycles' default 1 [chosen, Q-ENV2].
    pub aerosol: f32,
}

/// What an environment texture layer shows.
#[derive(Clone, Debug, PartialEq)]
pub enum EnvironmentTextureSource {
    /// One colour everywhere.
    Constant([f32; 3]),
    /// An image file: latitude-longitude (Spherical, ENV §5), or a Light
    /// Probe's mirror ball (`EnvironmentTexture::light_probe`).
    Image(String),
}

/// One texture layer of an environment, as Cycles renders it.
#[derive(Clone, Debug, PartialEq)]
pub struct EnvironmentTexture {
    pub source: EnvironmentTextureSource,
    pub opacity: f32,
    /// Cycles' Mix Color mode (`environment_mix`).
    pub mix: u32,
    pub invert: bool,
    /// World direction to the image's own frame (+Y the pole, longitude 0
    /// toward +Z): the locator's rotation undone (the item has none,
    /// ENVIRONMENT-PLACEMENT §2).
    pub to_image: glam::DMat3,
    /// An image read as a Light Probe (mirror ball,
    /// `operandi_core::environment::environment_probe_uv`).
    pub light_probe: bool,
}

impl EnvironmentLayer {
    /// The same colour in every direction, seen by every ray (a Constant
    /// environment).
    pub fn flat(color: [f32; 3], strength: f32) -> Self {
        Self {
            material: operandi_core::environment::EnvironmentMaterial {
                kind: operandi_core::environment::EnvironmentType::Constant,
                zenith: color.map(f64::from),
                ..Default::default()
            },
            intensity: strength,
            up: DVec3::Y,
            visible: [true; 4],
            material_opacity: 1.0,
            material_invert: false,
            layers: Vec::new(),
            item_inverse: glam::DMat3::IDENTITY,
            sky: None,
        }
    }

    /// Environment item `item` of `scene` as Cycles renders it. `None` for
    /// any other item. A disabled Environment Material shows black.
    pub fn of(scene: &Scene, item: &operandi_core::scene::Item) -> Option<Self> {
        use operandi_core::environment::RayKind;
        let env = item.environment.as_ref()?;
        // ENVIRONMENT-PLACEMENT §2: an environment item has no transform,
        // so its gradients stay world-up and nothing undoes an item
        // rotation; a sky turns on its layers' locators only.
        let up = DVec3::Y;
        let mut material = (*env.material.settings).clone();
        if !env.material.layer.enable {
            material.kind = operandi_core::environment::EnvironmentType::Constant;
            material.zenith = [0.0; 3];
        }
        let rotation = |id: u64| scene.locator_to_image(id);
        let item_inverse = glam::DMat3::IDENTITY;
        let mut layers = Vec::new();
        // Bottom first; only enabled Environment Color layers render.
        // Procedural, Gradient and the control effects aren't rendered yet
        // (ENV-Q-OP7).
        for node in env.layers.iter().rev() {
            use operandi_core::shader_tree::{NodeKind, TextureEffect, TextureSource};
            let NodeKind::Texture(t) = &node.kind else {
                continue;
            };
            if !node.layer.enable || t.effect != TextureEffect::EnvironmentColor {
                continue;
            }
            let source = match &t.source {
                TextureSource::Constant { color, .. } => {
                    EnvironmentTextureSource::Constant(color.map(|c| c as f32))
                }
                TextureSource::ImageMap {
                    image: Some(image), ..
                } => match scene.image(*image) {
                    Some(img) if !img.path.is_empty() => {
                        EnvironmentTextureSource::Image(img.path.clone())
                    }
                    _ => continue,
                },
                _ => continue,
            };
            let locator = t.locator.map_or(glam::DMat3::IDENTITY, rotation);
            let light_probe = matches!(source, EnvironmentTextureSource::Image(_))
                && t.locator
                    .and_then(|l| scene.texture_locator(l))
                    .is_some_and(|l| {
                        l.projection == operandi_core::shader_tree::Projection::LightProbe
                    });
            layers.push(EnvironmentTexture {
                source,
                opacity: node.layer.opacity as f32,
                mix: operandi_core::environment::environment_mix(node.layer.blend).cycles_code(),
                invert: node.layer.invert,
                to_image: locator * item_inverse,
                light_probe,
            });
        }
        // ENV §4: Physically-based Daylight follows its Sun Light. Without
        // one (or with a missing one) it draws its gradient [chosen].
        let sky = (material.kind == operandi_core::environment::EnvironmentType::Physical)
            .then_some(material.daylight.sun_light)
            .flatten()
            .and_then(|sun| {
                let light = scene.items.iter().find(|i| i.id == sun)?;
                light.light.as_ref()?;
                let travel = scene
                    .world_matrix(light.id)
                    .ok()?
                    .transform_vector3(-DVec3::Z)
                    .try_normalize()?;
                let d = &material.daylight;
                Some(PhysicalSky {
                    toward_sun: -travel,
                    sun_size: (0.545f64.to_radians() * d.disc_size.max(0.0)) as f32,
                    ground_albedo: (d.ground_albedo.iter().sum::<f64>() / 3.0).clamp(0.0, 1.0)
                        as f32,
                    aerosol: (d.haze / 2.0).max(0.0) as f32,
                })
            });
        Some(Self {
            material,
            intensity: env.intensity as f32,
            up,
            // A hidden environment is seen by no ray (ENVIRONMENT-PLACEMENT
            // §3.2, `Scene::environment_for`).
            visible: RayKind::ALL.map(|k| item.visible && env.visible_to(k)),
            material_opacity: env.material.layer.opacity as f32,
            material_invert: env.material.layer.invert,
            layers,
            item_inverse,
            sky,
        })
    }

    /// The radiance (colour × Intensity) along world `direction`: the Rust
    /// reference for the Cycles graph (`environment_color` in
    /// cycles/operandi_cycles.cpp), both from ENV §4 through
    /// `EnvironmentMaterial::color_at_height`.
    /// Image layers can't be read here and leave the colour as it was.
    pub fn radiance(&self, direction: DVec3) -> [f32; 3] {
        let s = direction.normalize_or_zero().dot(self.up).clamp(-1.0, 1.0);
        let mut c = self.material.color_at_height(s).map(|c| {
            let c = if self.material_invert { 1.0 - c } else { c };
            c * self.material_opacity as f64
        });
        for layer in &self.layers {
            let EnvironmentTextureSource::Constant(color) = &layer.source else {
                continue;
            };
            let mix = mix_of_code(layer.mix);
            for i in 0..3 {
                let v = if layer.invert {
                    1.0 - color[i]
                } else {
                    color[i]
                } as f64;
                c[i] = mix.apply(c[i], v, layer.opacity as f64);
            }
        }
        c.map(|v| v as f32 * self.intensity)
    }
}

/// Operandi's spherical image frame (+Y pole, u = 0.5 along +Z growing
/// toward -X, `environment_image_uv`) in Cycles' equirectangular one (+Z
/// pole, u from -atan2(y, x)): Cycles (x, y, z) = Operandi (z, x, y). A
/// reflection, because Operandi's longitude runs the other way round from
/// Cycles' for the same picture; it only picks the pixel.
#[cfg_attr(not(feature = "cycles"), allow(dead_code))]
pub(crate) fn image_frame_to_cycles() -> glam::DMat3 {
    glam::DMat3::from_cols(
        glam::DVec3::new(0.0, 1.0, 0.0),
        glam::DVec3::new(0.0, 0.0, 1.0),
        glam::DVec3::new(1.0, 0.0, 0.0),
    )
}

/// A Light Probe's image frame (the ball seen from +Z, +Y up, +X right,
/// `environment_probe_uv`) in Cycles' mirror-ball one (the ball seen from
/// -Y, u from +X, v from +Z up): Cycles (x, y, z) = Operandi (x, -z, y).
#[cfg_attr(not(feature = "cycles"), allow(dead_code))]
pub(crate) fn probe_frame_to_cycles() -> glam::DMat3 {
    glam::DMat3::from_cols(
        glam::DVec3::new(1.0, 0.0, 0.0),
        glam::DVec3::new(0.0, 0.0, 1.0),
        glam::DVec3::new(0.0, -1.0, 0.0),
    )
}

/// Whether Cycles importance-samples the world (HDRI stage 5): when any
/// environment has an image layer, its background light is built (the C
/// bridge's `sync_lights` makes the same test), so a sun in the picture
/// lights the scene without fireflies.
pub fn world_samples_image(world: &[EnvironmentLayer]) -> bool {
    world.iter().any(|l| {
        l.layers
            .iter()
            .any(|t| matches!(&t.source, EnvironmentTextureSource::Image(p) if !p.is_empty()))
    })
}

fn mix_of_code(code: u32) -> operandi_core::shader_eval::Mix {
    use operandi_core::shader_eval::Mix;
    [
        Mix::Blend,
        Mix::Add,
        Mix::Multiply,
        Mix::Screen,
        Mix::Difference,
        Mix::Darken,
        Mix::Lighten,
        Mix::Overlay,
    ]
    .into_iter()
    .find(|m| m.cycles_code() == code)
    .unwrap_or(Mix::Blend)
}

/// The scene's environments for `Settings::world`, newest first (ENV §6).
/// No environment: an empty world, black (ENV §6 [chosen]).
pub fn scene_world(scene: &Scene) -> Vec<EnvironmentLayer> {
    scene
        .environments()
        .filter_map(|item| EnvironmentLayer::of(scene, item))
        .collect()
}

/// What a ray of `kind` along `direction` sees in `world`: the first
/// (newest) environment visible to it, else black.
pub fn world_radiance(
    world: &[EnvironmentLayer],
    kind: operandi_core::environment::RayKind,
    direction: DVec3,
) -> [f32; 3] {
    let k = operandi_core::environment::RayKind::ALL
        .iter()
        .position(|&r| r == kind)
        .unwrap_or(0);
    world
        .iter()
        .find(|e| e.visible[k])
        .map_or([0.0; 3], |e| e.radiance(direction))
}

/// The colour fog with "use environment" takes: what the camera sees at
/// the horizon, with Intensity [chosen; was the flat sky colour before
/// environments]. Black with no environment the camera sees.
pub fn environment_fog_color(world: &[EnvironmentLayer]) -> [f32; 3] {
    world.iter().find(|e| e.visible[0]).map_or([0.0; 3], |e| {
        e.material
            .color_at_height(0.0)
            .map(|c| c as f32 * e.intensity)
    })
}

/// The old flat studio sky, now the default world.
pub const STUDIO_WORLD_COLOR: [f32; 3] = [0.55, 0.6, 0.7];
pub const STUDIO_WORLD_STRENGTH: f32 = 0.6;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub view: View,
    /// Direction sunlight travels, from the sun toward the scene.
    pub sun_direction: DVec3,
    pub sun_strength: f32,
    /// The render world: environment layers, added together. Empty is a
    /// black world. Dome lights add to it separately (`dome`).
    pub world: Vec<EnvironmentLayer>,
    pub base_color: [f32; 3],
    pub roughness: f32,
    /// Render on the GPU (Metal on macOS) when there is one.
    pub gpu: bool,
    /// Clean up the remaining noise with OpenImageDenoise.
    pub denoise: bool,
    /// The scene's lights. When set they replace the studio sun.
    pub lights: Option<Vec<SceneLight>>,
    /// LP-DM-01: dome lights' even world radiance (`scene_dome`).
    pub dome: [f32; 3],
    /// LP-22: a pass per light group name (`scene_light_groups`).
    pub light_groups: Vec<String>,
    /// Render Output passes to make besides the combined picture
    /// (`output_passes`).
    pub passes: Vec<Pass>,
    /// Material values to write besides the passes (`output_values`).
    pub values: Vec<MaterialValue>,
    /// The Ambient Occlusion pass's ray length in metres.
    pub ao_distance: f32,
    /// Render only this part of the picture.
    pub region: Option<Region>,
    /// Which kinds of light the render traces (the Preview's light path
    /// options); all on for a final render.
    pub light_paths: LightPaths,
    /// Adaptive sampling noise threshold; 0 turns adaptive sampling off.
    pub adaptive_threshold: f32,
    /// Ray depths, clamp and caustics (the Render item).
    pub rays: Rays,
    /// Stop after this many seconds, keeping what is done; 0 is no limit
    /// (the Preview's Max Render Time; final renders never set it).
    pub time_limit: f32,
    /// CPU render threads; 0 uses every core. GPU devices ignore it.
    pub threads: u32,
    /// Transparent film: the world still lights the scene but leaves alpha
    /// 0, so alpha is coverage.
    pub transparent: bool,
    /// Displaced surfaces are diced into pieces about this many pixels
    /// across (provisional fixed rate, ST-Q20).
    pub dicing_rate: f32,
    /// The world lights diffuse surfaces (on for every render; off only
    /// for calibration renders that isolate reflection).
    pub world_lights_diffuse: bool,
    /// Group D: the light group the world counts toward (Light Path
    /// Expressions `C.*B`); `None` for none.
    pub world_lightgroup: Option<String>,
    /// Group D: only light items light the scene, glowing surfaces light
    /// nothing (the Illumination (Unshadowed) helper render, RO-Q19).
    pub lamps_only: bool,
}

/// Kinds of light a render traces. Off means: no light casts shadows, or
/// that kind of indirect bounce is cut to 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LightPaths {
    pub shadows: bool,
    /// Diffuse bounces (global illumination).
    pub diffuse: bool,
    /// Glossy bounces (reflections).
    pub glossy: bool,
    /// Transmission bounces (refractions).
    pub transmission: bool,
}

impl Default for LightPaths {
    fn default() -> Self {
        Self {
            shadows: true,
            diffuse: true,
            glossy: true,
            transmission: true,
        }
    }
}

impl LightPaths {
    /// Bounce limits for Cycles (diffuse, glossy, transmission): `depths`
    /// for the kinds that are on, 0 for those turned off.
    pub fn bounce_limits(&self, depths: [u32; 3]) -> [i32; 3] {
        let on = [self.diffuse, self.glossy, self.transmission];
        std::array::from_fn(|i| if on[i] { depths[i] as i32 } else { 0 })
    }
}

/// The Render item's ray settings (render item spec v1, RI-S06..S11).
/// The defaults are Cycles' own, so a render that doesn't set them is as
/// before.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rays {
    /// Diffuse, glossy (Reflection) and transmission (Refraction) depth.
    /// Transparent bounces follow Refraction Depth.
    pub depths: [u32; 3],
    /// The total bounce limit.
    pub max_bounces: u32,
    /// Maximum Radiance: Cycles' indirect sample clamp; 0 is off.
    pub max_radiance: f32,
    /// Reflective and refractive caustics.
    pub caustics: [bool; 2],
    /// Direct Caustics: caustics-only lights (`SceneLight::caustics_only`)
    /// cast caustics through see-through Glass onto the other meshes.
    pub direct_caustics: bool,
}

impl Default for Rays {
    fn default() -> Self {
        Self::from_render(&operandi_core::render_settings::RenderSettings::default())
    }
}

impl Rays {
    /// A scene's Render item rows.
    pub fn from_render(r: &operandi_core::render_settings::RenderSettings) -> Self {
        Self {
            depths: [r.diffuse_depth, r.reflection_depth, r.refraction_depth],
            max_bounces: r.max_bounces(),
            max_radiance: r.max_radiance as f32,
            caustics: [r.caustics.reflective(), r.caustics.refractive()],
            direct_caustics: r.direct_caustics.enabled,
        }
    }
}

/// A rectangle of the picture in pixels, from its top-left corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Region {
    /// True when the region is at least one pixel and lies inside a
    /// `width` × `height` picture.
    pub fn fits(&self, width: u32, height: u32) -> bool {
        self.width > 0
            && self.height > 0
            && self.x < width
            && self.y < height
            && self.width <= width - self.x
            && self.height <= height - self.y
    }

    /// The whole picture.
    pub fn whole(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}

/// What a running render has done so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub samples_done: u32,
    pub samples_total: u32,
    /// True once the render has actually stopped for a pause (it pauses
    /// between sample batches, so this can lag the request a little).
    pub paused: bool,
    /// Goes up each time a newer preview arrives.
    pub preview_generation: u64,
    /// Group D: a render after the main one ("Unshadowed pass"), whose
    /// pictures don't replace the main render's in-progress picture.
    pub step: Option<&'static str>,
}

/// Steers a render from another thread and receives its progress and
/// in-progress picture. Share it (in an `Arc`) between the UI and the
/// thread that calls [`render_live`].
#[derive(Default)]
pub struct RenderControl {
    /// 0 run, 1 pause, 2 finish, 3 cancel (`OC_*` in operandi_cycles.h).
    command: std::sync::atomic::AtomicU32,
    live: std::sync::Mutex<Live>,
}

#[derive(Default)]
struct Live {
    progress: Progress,
    /// The rendered area so far (the region, or the whole picture), linear
    /// RGBA, top row first.
    preview: Vec<f32>,
}

const RUN: u32 = 0;
const PAUSE: u32 = 1;
const FINISH: u32 = 2;
const CANCEL: u32 = 3;

impl RenderControl {
    fn command(&self) -> u32 {
        self.command.load(std::sync::atomic::Ordering::Acquire)
    }
    fn set_command(&self, command: u32) {
        self.command
            .store(command, std::sync::atomic::Ordering::Release);
    }
    /// Hold the render between sample batches.
    pub fn pause(&self) {
        if self.command() == RUN {
            self.set_command(PAUSE);
        }
    }
    pub fn resume(&self) {
        if self.command() == PAUSE {
            self.set_command(RUN);
        }
    }
    /// Stop adding samples; the render denoises and delivers what it has.
    pub fn finish(&self) {
        if matches!(self.command(), RUN | PAUSE) {
            self.set_command(FINISH);
        }
    }
    /// Stop without a picture; [`render_live`] then returns an error.
    pub fn cancel(&self) {
        self.set_command(CANCEL);
    }
    /// True after [`pause`](Self::pause) until resumed or stopped.
    pub fn pause_requested(&self) -> bool {
        self.command() == PAUSE
    }
    pub fn finish_requested(&self) -> bool {
        self.command() == FINISH
    }
    pub fn cancelled(&self) -> bool {
        self.command() == CANCEL
    }
    pub fn progress(&self) -> Progress {
        self.live.lock().map(|l| l.progress).unwrap_or_default()
    }
    /// The newest in-progress picture of the rendered area when it's newer
    /// than `seen` (a `preview_generation`), with its generation.
    pub fn preview_since(&self, seen: u64) -> Option<(u64, Vec<f32>)> {
        let live = self.live.lock().ok()?;
        (live.progress.preview_generation > seen && !live.preview.is_empty())
            .then(|| (live.progress.preview_generation, live.preview.clone()))
    }
    #[cfg_attr(not(feature = "cycles"), allow(dead_code))]
    fn report_progress(&self, done: u32, total: u32, paused: bool) {
        if let Ok(mut live) = self.live.lock() {
            live.progress.samples_done = done.min(total);
            live.progress.samples_total = total;
            live.progress.paused = paused;
        }
    }
    /// Names the render step that follows the main render (shown with
    /// the progress); `None` back to the main render.
    pub fn set_step(&self, step: Option<&'static str>) {
        if let Ok(mut live) = self.live.lock() {
            live.progress.step = step;
        }
    }
    #[cfg_attr(not(feature = "cycles"), allow(dead_code))]
    fn report_preview(&self, rgba: &[f32]) {
        if let Ok(mut live) = self.live.lock() {
            // A later step's own picture isn't the render's.
            if live.progress.step.is_some() {
                return;
            }
            live.preview.clear();
            live.preview.extend_from_slice(rgba);
            live.progress.preview_generation += 1;
        }
    }
}

/// Copies `area` (a `region`-sized RGBA block, top row first) into
/// `picture` (`width` pixels wide) at the region's place.
pub fn paste_region(picture: &mut [f32], width: u32, region: Region, area: &[f32]) {
    let row = region.width as usize * 4;
    if area.len() < row * region.height as usize {
        return;
    }
    for y in 0..region.height as usize {
        let start = ((region.y as usize + y) * width as usize + region.x as usize) * 4;
        if let Some(dst) = picture.get_mut(start..start + row) {
            dst.copy_from_slice(&area[y * row..(y + 1) * row]);
        }
    }
}

impl Settings {
    /// Plain studio look: grey clay, a sun over the camera's shoulder and a
    /// soft grey-blue sky.
    pub fn studio(width: u32, height: u32, view: View) -> Self {
        let sun = (view.forward - view.up * 0.8 + view.right * 0.5).normalize_or_zero();
        Self {
            width,
            height,
            samples: 16,
            view,
            sun_direction: if sun == DVec3::ZERO { -DVec3::Y } else { sun },
            sun_strength: 3.0,
            world: vec![EnvironmentLayer::flat(
                STUDIO_WORLD_COLOR,
                STUDIO_WORLD_STRENGTH,
            )],
            base_color: [0.8, 0.8, 0.8],
            roughness: 0.5,
            gpu: true,
            denoise: true,
            lights: None,
            dome: [0.0; 3],
            light_groups: Vec::new(),
            passes: Vec::new(),
            values: Vec::new(),
            ao_distance: 1e10,
            region: None,
            light_paths: LightPaths::default(),
            adaptive_threshold: 0.0,
            rays: Rays::default(),
            time_limit: 0.0,
            threads: 0,
            transparent: false,
            dicing_rate: 1.0,
            world_lights_diffuse: true,
            world_lightgroup: None,
            lamps_only: false,
        }
    }

    /// Render the scene's environments instead of the studio sky (ENV §6;
    /// none is black).
    pub fn with_scene_world(mut self, scene: &Scene) -> Self {
        self.world = scene_world(scene);
        self
    }

    /// Light with the scene's lights when it has any light item;
    /// otherwise keep the studio sun (CL-1).
    pub fn with_scene_lights(mut self, scene: &Scene) -> Self {
        if scene.items.iter().any(|i| i.light.is_some()) {
            self.lights = Some(scene_lights(scene));
            self.dome = scene_dome(scene);
            self.light_groups = scene_light_groups(scene);
        }
        self
    }

    /// Light with the scene's lights only, even when it has none (render
    /// fix F3): the final render and the Preview never add a light the
    /// user didn't, so an environment-only scene is lit by the
    /// environment (and glowing surfaces) alone. Thumbnails and tests
    /// keep the studio sun through `with_scene_lights`.
    pub fn with_only_scene_lights(mut self, scene: &Scene) -> Self {
        self.lights = Some(scene_lights(scene));
        self.dome = scene_dome(scene);
        self.light_groups = scene_light_groups(scene);
        self
    }

    /// Cycles takes the field of view across the image's shorter side.
    pub fn cycles_fov(&self) -> f64 {
        let aspect = self.width as f64 / self.height.max(1) as f64;
        let half = if aspect >= 1.0 {
            self.view.half_height_tan
        } else {
            self.view.half_height_tan * aspect
        };
        2.0 * half.atan()
    }
}

/// The scene's render camera as a Cycles view for a picture of `aspect`
/// (width / height), or `None` when the scene has no camera. Film Fit picks
/// which side the film spans (`CameraSettings::half_tangents`); Cycles
/// itself then takes the field of view across the shorter side.
pub fn scene_camera_view(scene: &Scene, aspect: f64) -> Option<View> {
    camera_view(scene, scene.render_camera()?, aspect)
}

/// Like [`scene_camera_view`], through camera item `id` (the Preview's
/// Camera popup); `None` when `id` is not a camera.
pub fn camera_view(scene: &Scene, id: u64, aspect: f64) -> Option<View> {
    let item = scene.items.iter().find(|i| i.id == id)?;
    let camera = item.camera.as_ref()?;
    let frame = operandi_core::camera_light::item_frame(scene, id)?;
    let (_, half_h) = camera.half_tangents(aspect);
    Some(View {
        position: frame.position,
        right: frame.right,
        up: frame.up,
        forward: frame.forward,
        half_height_tan: half_h,
        orthographic: (camera.projection == Projection::Orthographic)
            .then(|| camera.orthographic_half_size(aspect).1),
        shift: camera.film_shift(aspect),
        clip: if camera.clipping {
            (camera.clip_near, camera.clip_far)
        } else {
            DEFAULT_CLIP
        },
        panorama: match camera.projection {
            Projection::Cylindrical => Some(Panorama {
                cylindrical: true,
                vr: false,
            }),
            Projection::CylindricalVr => Some(Panorama {
                cylindrical: true,
                vr: true,
            }),
            Projection::Spherical => Some(Panorama {
                cylindrical: false,
                vr: false,
            }),
            Projection::SphericalVr => Some(Panorama {
                cylindrical: false,
                vr: true,
            }),
            Projection::Perspective | Projection::Orthographic => None,
        },
        dof: camera.dof.enable.then(|| Dof {
            aperture_radius: camera.dof.aperture_radius(camera.focal_length),
            focus_distance: scene
                .camera_focus_distance(id)
                .unwrap_or(camera.focus_distance),
            blades: camera.dof.iris_blades,
            blades_rotation: camera.dof.iris_rotation,
        }),
    })
}

/// Light power conversions (CL-Q14, all provisional): a point or spot of
/// radiant intensity I (W/sr) emits I·4π W; an area light of radiance L
/// (W/sr/m²) emits L·area·π W; a sun's strength is its exitance in W/m².
pub fn light_power(light: &LightSettings, area: f64) -> f64 {
    match light.kind {
        LightKind::Directional => {
            light.intensity
                * operandi_core::camera_light_defaults::CYCLES_SUN_STRENGTH_PER_INTENSITY
        }
        LightKind::Point | LightKind::Spot => light.intensity * 4.0 * std::f64::consts::PI,
        LightKind::Area => light.intensity * area * std::f64::consts::PI,
        // Without a file a photometric light is a point light of its
        // Radiant Intensity.
        LightKind::Photometric => light.intensity * 4.0 * std::f64::consts::PI,
        // Cylinder and mesh lights render as glowing surfaces
        // (`light_meshes`); a portal makes no light.
        LightKind::Portal | LightKind::Cylinder | LightKind::Mesh | LightKind::Dome => 0.0,
    }
}

/// Cycles emission strength for a glowing surface of radiance `radiance`
/// (W/sr/m²): an Emission closure of strength L renders pixel L seen head
/// on, so the strength is L itself, no π (LXP-B02a; a radiant exitance E
/// would be E/π). Pixel-tested in `emission_units_tests`.
pub fn emission_strength(radiance: f64) -> f64 {
    radiance
}

/// The scene's lights for Cycles (CL-1, CL-2; provisional): strength =
/// colour × power × (1 − Dissolve) (`light_power`), Shadow Type None turns
/// shadows off. Directional → sun (angle = Spread Angle); Point → point
/// (radius); Spot → spot (size = the full cone, blend = Soft Edge over half
/// the cone); Area → rectangle or ellipse sized Width/Height × item scale.
/// Render Off, or Render Default on a hidden item, leaves a light out; so
/// does a light that affects neither diffuse nor specular. LightFalloff other
/// than inverse square and partial Affect Diffuse/Specular are not passed
/// on (`light_notes` says so); any Affect above 0 % counts as on (LPQ-07).
/// Portal → a portal area light sized like an area light; it ignores
/// strength, colour, Dissolve and the visibility rows (LP-PO).
///
/// Tier 2: Physical Sun sets a directional light's strength (`PhysicalSun::
/// strength`; below the horizon it's left out). Photometric → point (no
/// cone limit) or spot with its IES file: strength = Intensity ÷ the file's
/// own Radiant Intensity, so a just-loaded light matches the file. Cylinder
/// and mesh lights come from `light_meshes` instead.
pub fn scene_lights(scene: &Scene) -> Vec<SceneLight> {
    let links = link_sets(scene);
    scene
        .items
        .iter()
        .filter_map(|item| {
            let light = item.light.as_ref()?;
            if !light.renders(scene.shows(item))
                || matches!(
                    light.kind,
                    LightKind::Cylinder | LightKind::Mesh | LightKind::Dome
                )
            {
                return None;
            }
            let world = scene.world_matrix(item.id).ok()?;
            let frame = operandi_core::camera_light::Frame::from_matrix(world)?;
            // The item's scale stretches an area light.
            let size = [
                light.width * world.transform_vector3(DVec3::X).length(),
                light.height * world.transform_vector3(DVec3::Y).length(),
            ];
            let area = match light.shape {
                AreaShape::Rectangular => size[0] * size[1],
                AreaShape::Elliptical => std::f64::consts::FRAC_PI_4 * size[0] * size[1],
            };
            if light.kind == LightKind::Portal {
                return Some((
                    item.id,
                    SceneLight {
                        shape: LightShape::Portal {
                            size: size.map(|v| v as f32),
                            ellipse: light.shape == AreaShape::Elliptical,
                        },
                        position: frame.position,
                        right: frame.right,
                        up: frame.up,
                        direction: frame.forward,
                        strength: [1.0; 3],
                        cast_shadow: false,
                        visible_to_camera: false,
                        affects_diffuse: true,
                        affects_specular: true,
                        visible_to_reflection: false,
                        visible_to_refraction: false,
                        ies: None,
                        lightgroup: None,
                        layers: Vec::new(),
                        gobo_tan_half: 0.0,
                        light_membership: None,
                        shadow_membership: None,
                        caustics_only: false,
                    },
                    true,
                    0.0,
                ));
            }
            // LP-11: Dissolve dims the light, 100 % is off.
            let mut scale = light_power(light, area) * (1.0 - light.dissolve);
            if light.kind == LightKind::Directional && light.sun.enabled {
                scale = light.sun.strength(light.intensity) * (1.0 - light.dissolve);
            }
            let ies = (light.kind == LightKind::Photometric && !light.photometry_file.is_empty())
                .then(|| {
                    let text = std::fs::read_to_string(&light.photometry_file).ok()?;
                    let file = operandi_core::photometry::Photometry::parse(&text).ok()?;
                    Some((std::sync::Arc::<str>::from(text), file.radiant_intensity()))
                })
                .flatten();
            if let Some((_, file_intensity)) = &ies {
                scale = if *file_intensity > 0.0 {
                    light.intensity / file_intensity * (1.0 - light.dissolve)
                } else {
                    0.0
                };
            }
            // LM-01 / LM-04.
            scale *= material_factor(light);
            let affects_diffuse = light.affect_diffuse > 0.0;
            let affects_specular = light.affect_specular > 0.0;
            // Direct Caustics: Affect Caustics' share goes to a copy that
            // only casts caustics, so it is independent of Affect Diffuse
            // and Specular (DC-01..03).
            let caustics = if scene.render.direct_caustics.enabled {
                light.stored.affect_caustics.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let lights_surfaces = affects_diffuse || affects_specular;
            if scale <= 0.0 || !(lights_surfaces || caustics > 0.0) {
                return None;
            }
            let shape = match light.kind {
                LightKind::Directional => LightShape::Sun {
                    angle: light.spread_angle as f32,
                },
                LightKind::Point => LightShape::Point {
                    radius: light.radius as f32,
                },
                LightKind::Spot => LightShape::Spot {
                    radius: light.radius as f32,
                    size: light.cone_angle as f32,
                    blend: light.spot_blend() as f32,
                },
                LightKind::Area => LightShape::Area {
                    size: size.map(|v| v as f32),
                    ellipse: light.shape == AreaShape::Elliptical,
                },
                // LP-PH-03/04: the glowing part's size softens it; a cone
                // under 180° limits it like a spot (Cycles spots stop at
                // 180°), wider is a point light.
                LightKind::Photometric => {
                    let radius = (0.5 * light.width.max(light.height)) as f32;
                    if light.cone_angle < std::f64::consts::PI - 1e-6 {
                        LightShape::Spot {
                            radius,
                            size: light.cone_angle as f32,
                            blend: light.spot_blend() as f32,
                        }
                    } else {
                        LightShape::Point { radius }
                    }
                }
                LightKind::Portal | LightKind::Cylinder | LightKind::Mesh | LightKind::Dome => {
                    unreachable!("handled above")
                }
            };
            Some((
                item.id,
                SceneLight {
                    shape,
                    position: frame.position,
                    right: frame.right,
                    up: frame.up,
                    direction: frame.forward,
                    strength: light.color.map(|c| (c * scale) as f32),
                    cast_shadow: light.shadow != ShadowType::None,
                    visible_to_camera: light.visible_to_camera,
                    affects_diffuse,
                    affects_specular,
                    visible_to_reflection: light.stored.visible_to_reflection,
                    visible_to_refraction: light.stored.visible_to_refraction,
                    ies: ies.map(|(text, _)| text),
                    lightgroup: (!light.lpe_label.is_empty()).then(|| light.lpe_label.clone()),
                    layers: if matches!(shape, LightShape::Spot { .. }) {
                        light
                            .layers
                            .iter()
                            .filter(|l| l.enabled && !l.image.is_empty())
                            .map(|l| (l.image.clone(), mix_type(l.blend), l.opacity as f32))
                            .collect()
                    } else {
                        Vec::new()
                    },
                    gobo_tan_half: if matches!(shape, LightShape::Spot { .. }) {
                        (light.cone_angle / 2.0).min(1.5).tan() as f32
                    } else {
                        0.0
                    },
                    light_membership: links.membership.get(&item.id).copied(),
                    shadow_membership: links.shadow.get(&item.id).copied(),
                    caustics_only: false,
                },
                lights_surfaces,
                caustics,
            ))
        })
        .flat_map(|(id, light, lights_surfaces, caustics)| {
            // The caustics-only copy: it reaches surfaces only through
            // glass, has no shape for rays to hit and is never seen.
            let caustic = (caustics > 0.0).then(|| SceneLight {
                strength: light.strength.map(|c| c * caustics as f32),
                cast_shadow: true,
                visible_to_camera: false,
                affects_diffuse: true,
                affects_specular: true,
                visible_to_reflection: false,
                visible_to_refraction: false,
                caustics_only: true,
                ..light.clone()
            });
            if !lights_surfaces {
                return caustic.into_iter().collect::<Vec<_>>();
            }
            // Shader item v1: copies for surfaces that scale direct light
            // or ignore shadows.
            let copies: Vec<SceneLight> = links
                .variants
                .iter()
                .filter(|v| v.light == id)
                .map(|v| SceneLight {
                    strength: light.strength.map(|c| c * v.direct as f32),
                    light_membership: Some(v.membership),
                    shadow_membership: Some(if v.receive_shadows {
                        light
                            .shadow_membership
                            .or(light.light_membership)
                            .unwrap_or(u64::MAX)
                    } else {
                        0
                    }),
                    ..light.clone()
                })
                .collect();
            std::iter::once(light)
                .chain(copies)
                .chain(caustic)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Mesh-light prototypes that render as a light, so their mesh doesn't
/// also render as itself (LP-MS-01).
pub fn mesh_light_prototypes(scene: &Scene) -> std::collections::BTreeSet<u64> {
    scene
        .items
        .iter()
        .filter_map(|i| {
            let l = i.light.as_ref()?;
            (l.kind == LightKind::Mesh && l.renders(scene.shows(i)))
                .then_some(l.prototype)
                .flatten()
        })
        .collect()
}

/// The emission-only surface a cylinder or mesh light renders with.
fn emitter_shader(light: &LightSettings) -> Option<RenderShader> {
    let strength =
        emission_strength(light.intensity) * material_factor(light) * (1.0 - light.dissolve);
    let diffuse = light.affect_diffuse > 0.0;
    let glossy = light.affect_specular > 0.0;
    if strength <= 0.0 || !(diffuse || glossy) {
        return None;
    }
    let constant = |v: [f64; 3]| ShaderOp {
        op: ShaderOp::CONST,
        mode: 0,
        a: -1,
        b: -1,
        fac: -1,
        value: v.map(|c| c as f32),
        image: -1,
        xform: [0.0; 12],
    };
    let ops = vec![
        constant([0.0; 3]),
        constant(light.color),
        constant([strength; 3]),
        constant([1.0; 3]),
    ];
    let mut inputs = [-1; Input::COUNT];
    inputs[Input::BaseColor as usize] = 0;
    inputs[Input::SpecularIorLevel as usize] = 0;
    inputs[Input::EmissionColor as usize] = 1;
    inputs[Input::EmissionStrength as usize] = 2;
    inputs[Input::Roughness as usize] = 3;
    Some(RenderShader {
        ops,
        inputs,
        images: Vec::new(),
        image_filters: Vec::new(),
        image_raw: Vec::new(),
        image_vectors: Vec::new(),
        image_extensions: Vec::new(),
        ramps: Vec::new(),
        attributes: Vec::new(),
        uv_map: None,
        // A light's emitter has no bump, normal map or displacement.
        bump: -1,
        normal_map: -1,
        displacement: -1,
        coat_bump: 0.0,
        // Nor a see-through part.
        transparency_mode: 0,
        transparency_amount: -1,
        transparency_color: -1,
        refraction_ior: -1,
        refraction_roughness: -1,
        visible_to_camera: light.visible_to_camera,
        visible_to_diffuse: diffuse,
        visible_to_glossy: glossy,
        visible_to_transmission: true,
        // A light's own shape doesn't block other lights.
        cast_shadows: false,
        shadow_catcher: false,
        volume: None,
        lightgroup: (!light.lpe_label.is_empty()).then(|| light.lpe_label.clone()),
        look: SurfaceLook::default(),
        rounded_edge: 0.0,
        absorption: ([1.0; 3], 0.0),
        scatter: ([1.0; 3], 0.0, 0.0),
        thin_wall: false,
        glass_principled: false,
        output_groups: 0,
        node_graph: None,
        values: [-1; MaterialValue::COUNT],
    })
}

/// Segments round a cylinder light's tube.
const CYLINDER_SEGMENTS: usize = 32;

/// The cylinder light's open tube along local X (LPQ-02: ends dark) in
/// world space: positions and outward-facing triangles.
pub fn cylinder_tube(world: DMat4, radius: f64, length: f64) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let h = length / 2.0;
    let mut positions = Vec::new();
    for k in 0..CYLINDER_SEGMENTS {
        let a = k as f64 / CYLINDER_SEGMENTS as f64 * std::f64::consts::TAU;
        for x in [-h, h] {
            let p = world.transform_point3(DVec3::new(x, radius * a.cos(), radius * a.sin()));
            positions.push(p.as_vec3().to_array());
        }
    }
    let n = CYLINDER_SEGMENTS as u32;
    let mut triangles = Vec::new();
    for k in 0..n {
        let (a0, a1) = (2 * k, 2 * k + 1);
        let (b0, b1) = (2 * ((k + 1) % n), 2 * ((k + 1) % n) + 1);
        triangles.push([a0, b0, b1]);
        triangles.push([a0, b1, a1]);
    }
    (positions, triangles)
}

/// Cylinder lights (LP-CY, tube along local X with dark ends, LPQ-02) and
/// mesh lights (LP-MS) as glowing surfaces: emission = colour × Radiance
/// × (1 − Dissolve) (`emission_strength`, LXP-B02a), seen by camera rays
/// only with Visible to Camera, by diffuse / glossy rays with Affect Diffuse / Specular, casting no
/// shadows. Falloff is ignored as for every light (LPROP-Q17).
pub fn light_meshes(scene: &Scene, render_level: bool) -> Result<Vec<RenderMesh>> {
    let mut out = Vec::new();
    let links = link_sets(scene);
    for item in &scene.items {
        let first = out.len();
        let Some(light) = item.light.as_ref() else {
            continue;
        };
        if !light.renders(scene.shows(item)) {
            continue;
        }
        let Some(shader) = emitter_shader(light) else {
            continue;
        };
        match light.kind {
            LightKind::Cylinder if light.radius > 0.0 => {
                let world = scene.world_matrix(item.id)?;
                let (positions, triangles) = cylinder_tube(world, light.radius, light.length);
                let smooth = vec![1; triangles.len()];
                out.push(RenderMesh {
                    positions,
                    triangles,
                    smooth,
                    uvs: Vec::new(),
                    shader: Some(shader),
                    light_set: 0,
                    light_membership: None,
                    attributes: Vec::new(),
                    normals: Vec::new(),
                    transform: None,
                    name: item.name.clone(),
                    asset_name: item.name.clone(),
                    material_name: item.name.clone(),
                    segment_color: operandi_core::render_outputs::segment_color(item.id, ""),
                });
            }
            LightKind::Mesh => {
                let Some(proto) = light
                    .prototype
                    .and_then(|p| scene.items.iter().find(|i| i.id == p && i.is_mesh()))
                else {
                    continue;
                };
                // The prototype alone, visible, through the ordinary mesh
                // path; then placed by the light when it doesn't use the
                // prototype's transform.
                let mut only = scene.clone();
                for i in &mut only.items {
                    i.visible = i.id == proto.id;
                }
                let move_to = if light.use_prototype_transform {
                    None
                } else {
                    let proto_world = scene.world_matrix(proto.id)?;
                    if proto_world.determinant().abs() < 1e-12 {
                        continue;
                    }
                    Some(scene.world_matrix(item.id)? * proto_world.inverse())
                };
                for mut mesh in scene_meshes_of(&only, render_level, false)? {
                    if let Some(m) = move_to {
                        let placed = mesh.transform.map_or(DMat4::IDENTITY, rows_matrix);
                        mesh.transform = Some(transform_rows(m * placed));
                    }
                    mesh.shader = Some(shader.clone());
                    mesh.uvs.clear();
                    out.push(mesh);
                }
            }
            _ => {}
        }
        // A glowing light surface lights only the sets its rule allows (at
        // full strength for shader light copies' sets too: no copies).
        let membership = links
            .shadow
            .get(&item.id)
            .or(links.membership.get(&item.id))
            .copied();
        for m in &mut out[first..] {
            m.light_membership = membership;
        }
    }
    Ok(out)
}

/// What a render can't carry over from the scene's lights (CL-Q14, CL-Q15),
/// one line per kind of problem, for the status bar.
pub fn light_notes(scene: &Scene) -> Vec<String> {
    let lights: Vec<&LightSettings> = scene
        .items
        .iter()
        .filter_map(|i| {
            i.light
                .as_ref()
                .filter(|l| l.kind.emits() && l.renders(scene.shows(i)))
        })
        .collect();
    let mut notes = Vec::new();
    if lights.iter().any(|l| {
        l.kind == LightKind::Photometric
            && !l.photometry_file.is_empty()
            && std::fs::read_to_string(&l.photometry_file)
                .ok()
                .and_then(|t| operandi_core::photometry::Photometry::parse(&t).ok())
                .is_none()
    }) {
        notes.push(
            "A photometric light's file can't be read; it renders as a plain point light"
                .to_string(),
        );
    }
    if lights.iter().any(|l| {
        l.kind != LightKind::Directional && l.falloff != LightFalloff::InverseDistanceSquared
    }) {
        notes.push("Cycles uses inverse-square falloff for every light".to_string());
    }
    let partial = |v: f64| v > 0.0 && (v - 1.0).abs() > 1e-9;
    if lights
        .iter()
        .any(|l| partial(l.affect_diffuse) || partial(l.affect_specular))
    {
        notes
            .push("Affect Diffuse/Specular between 0 % and 100 % render as on (100 %)".to_string());
    }
    notes
}

/// What a render can't show from the shader tree, one line per kind of
/// problem, for the status bar: a Texture U/V Gradient reads 0 on a mesh
/// without UVs (GR-Q04).
pub fn shading_notes(scene: &Scene) -> Vec<String> {
    use operandi_core::gradient::GradientInput as I;
    use operandi_core::shader_tree::TextureSource;
    let reads_uvs = scene.shader_tree.nodes.iter().any(|n| {
        n.layer.enable
            && matches!(&n.kind, NodeKind::Texture(t) if matches!(
                &t.source,
                TextureSource::Gradient(g) if matches!(g.input, I::TextureU | I::TextureV)
            ))
    });
    let bare_mesh = scene.items.iter().any(|i| {
        scene.shows(i)
            && i.light.is_none()
            && !i.mesh.faces.is_empty()
            && i.mesh.faces.iter().all(|f| f.uv.is_none())
            && i.mesh
                .uv_maps
                .values()
                .flatten()
                .flatten()
                .all(Option::is_none)
    });
    if reads_uvs && bare_mesh {
        vec!["A Texture U/V gradient reads 0 on a mesh without UVs".to_string()]
    } else {
        Vec::new()
    }
}

/// Visible, non-empty items of `scene`, triangulated, in each item's own
/// space with its world matrix as the transform, one mesh per item and
/// shader tree surface. Polylines and polygons
/// that can't be triangulated are skipped.
pub fn scene_meshes(scene: &Scene) -> Result<Vec<RenderMesh>> {
    scene_meshes_at(scene, true)
}

/// [`scene_meshes`] with subdivision surfaces at their render level
/// (`render_level`) or their viewport level (the Preview below Final Render
/// Quality).
pub fn scene_meshes_at(scene: &Scene, render_level: bool) -> Result<Vec<RenderMesh>> {
    scene_meshes_of(scene, render_level, true)
}

/// `scene_meshes_at`; `with_lights` adds the glowing cylinder and mesh
/// lights and leaves out the mesh lights' prototypes.
fn scene_meshes_of(
    scene: &Scene,
    render_level: bool,
    with_lights: bool,
) -> Result<Vec<RenderMesh>> {
    let mut out = Vec::new();
    let tree = scene.shading_tree();
    // Surfaces that render as the studio clay (ST-Q11).
    let clay = Principled::clay_surfaces();
    // Material-value outputs (material_values.rs), read once: the material
    // values go into every surface; dPdu/dPdv need the mesh's UVs.
    let wanted_values = outputs::output_values(&tree.render_outputs());
    let material_values: Vec<MaterialValue> = wanted_values
        .iter()
        .copied()
        .filter(|v| !v.is_geometry())
        .collect();
    let uv_tangents = wanted_values.iter().any(|v| v.is_geometry());
    let prototypes = if with_lights {
        mesh_light_prototypes(scene)
    } else {
        Default::default()
    };
    let links = link_sets(scene);
    // A UV Coordinates output needs every mesh's UVs, not only those of
    // surfaces that read images.
    let uv_output = tree
        .render_outputs()
        .iter()
        .any(|o| o.effect == operandi_core::shader_tree::OutputEffect::UvCoordinates);
    if with_lights {
        out.extend(light_meshes(scene, render_level)?);
        out.extend(light_volumes(scene)?);
    }
    let lights_done = out.len();
    // G8 RP-4: a Replicator's copies (Mesh Instances in the evaluated
    // scene) are built once per prototype mesh and placed by their own
    // matrix; equal geometry keys then make the Cycles session instance
    // one mesh. Keyed by (mesh content stamp, prototype): the range of
    // `out` the first copy made.
    let mut replicas: std::collections::HashMap<(u64, u64), std::ops::Range<usize>> =
        Default::default();
    for item in scene
        .items
        .iter()
        .filter(|i| scene.shows(i) && i.is_mesh() && !prototypes.contains(&i.id))
    {
        if item.mesh.faces.is_empty() {
            continue;
        }
        let world: DMat4 = scene.world_matrix(item.id)?;
        let replica = operandi_core::replicator::copy_replicator(item.id).is_some();
        // A copy shades as its prototype (its item masks and materials).
        let shade_id = if replica {
            item.instance_of.unwrap_or(item.id)
        } else {
            item.id
        };
        let replica_key = (item.mesh.stamp(), shade_id);
        if replica && let Some(range) = replicas.get(&replica_key).cloned() {
            for n in range {
                let mut r = out[n].clone();
                r.transform = Some(transform_rows(world));
                r.name = item.name.clone();
                r.asset_name = item.name.clone();
                r.segment_color =
                    operandi_core::render_outputs::segment_color(item.id, &r.material_name);
                out.push(r);
            }
            continue;
        }
        let first = out.len();
        let surface = render_surface(item, &scene.smoothing, render_level);
        let mesh = &surface.mesh;
        // Each triangle corner carries the viewport's shading normal (the
        // angle rule SS-02 and the Smoothing amount), as Blender's sync
        // hands Cycles its corner normals; points stay shared. Points are
        // in the item's own space and the item's world matrix places them,
        // so moving an item leaves its points as they were.
        let normals = &surface.normals;
        let smooth = !scene.smoothing.is_flat();
        // Faces grouped by surface, in first-use order: each polygon context
        // (material and part tag) is evaluated through the shader tree, and
        // contexts that come out the same share one mesh. Polygons the tree
        // leaves at its defaults keep the studio clay.
        let ancestors = scene.item_ancestors(shade_id);
        // Cryptomatte Asset: the top-level ancestor (RO-Q16).
        let asset_name = ancestors
            .last()
            .and_then(|&a| scene.item(a))
            .map_or_else(|| item.name.clone(), |a| a.name.clone());
        // Texture Locators as this item sees them.
        let locators = scene.locator_frames(item.id);
        // A normal map needs UVs; on a mesh without any it does nothing
        // (G-07).
        let has_uvs = mesh.faces.iter().any(|f| f.uv.is_some());
        let islands = operandi_core::vertex_map_texture::islands(&item.mesh);
        // Selection sets per cage polygon (G-01); a subdivided piece takes
        // its cage polygon's sets.
        let mut cage_sets: Vec<Vec<&str>> = vec![Vec::new(); item.mesh.faces.len()];
        for (name, faces) in &item.mesh.attributes.polygon_sets {
            for &f in faces {
                if let Some(list) = cage_sets.get_mut(f as usize) {
                    list.push(name.as_str());
                }
            }
        }
        let mut groups: Vec<(Option<Principled>, RenderMesh)> = Vec::new();
        type Key<'k> = (&'k str, Option<&'k str>, &'k [&'k str]);
        let mut contexts: std::collections::HashMap<Key, usize> = std::collections::HashMap::new();
        // Per group: render point of each item point it uses.
        let mut splits: Vec<std::collections::HashMap<u32, u32>> = Vec::new();
        for (index, face) in mesh.faces.iter().enumerate() {
            let Ok(tris) = mesh.surface_triangles(index) else {
                continue;
            };
            let sets: &[&str] = surface
                .source(index)
                .and_then(|f| cage_sets.get(f))
                .map_or(&[], Vec::as_slice);
            let key = (face.material.as_str(), face.part.as_deref(), sets);
            let g = match contexts.get(&key) {
                Some(&g) => g,
                None => {
                    let ctx = Context {
                        item: shade_id,
                        ancestors: &ancestors,
                        material: key.0,
                        part: key.1,
                        sets: key.2,
                        locators: &locators,
                    };
                    // The saved graph where one is pinned (materials stage 4).
                    let mut surface = operandi_core::materialx::saved::surface(
                        scene,
                        &ctx,
                        tree.evaluate(&ctx).principled_with_values(&material_values),
                    );
                    if !has_uvs {
                        surface = surface.without_uvs();
                    }
                    // RO-Q03 v2: which grouped outputs this context fills.
                    let bits = tree.output_group_bits(&ctx);
                    // MV-Q07: with material-value outputs no surface is
                    // swapped for the clay, whose shader writes no values.
                    let principled = (!clay.contains(&surface.normalized())
                        || bits != 0
                        || !wanted_values.is_empty())
                    .then_some(surface);
                    let g = match groups.iter().position(|(p, r)| {
                        *p == principled && r.shader.as_ref().map_or(0, |s| s.output_groups) == bits
                    }) {
                        Some(g) => g,
                        None => {
                            let shader = principled.as_ref().map(|p| {
                                let mut shader = RenderShader::compile_through_graph(p, scene);
                                shader.output_groups = bits;
                                shader
                            });
                            groups.push((
                                principled,
                                RenderMesh {
                                    shader,
                                    transform: Some(transform_rows(world)),
                                    name: item.name.clone(),
                                    asset_name: asset_name.clone(),
                                    material_name: key.0.to_owned(),
                                    segment_color: operandi_core::render_outputs::segment_color(
                                        item.id, key.0,
                                    ),
                                    ..Default::default()
                                },
                            ));
                            splits.push(Default::default());
                            groups.len() - 1
                        }
                    };
                    contexts.insert(key, g);
                    g
                }
            };
            let stored = scene.material(&face.material);
            let (_, r) = &mut groups[g];
            let wants_uvs = uv_output
                || r.shader.as_ref().is_some_and(RenderShader::needs_uvs)
                || (uv_tangents && has_uvs);
            // The UV map the surface reads: a named one, else the polygon's
            // own (legacy, imported) UVs, else the first map. Primitives
            // keep theirs in a map ("Texture"), so without this fallback
            // they rendered with every UV at 0.
            let legacy = face.uv.is_some();
            let wanted = r.shader.as_ref().and_then(|s| s.uv_map.as_deref());
            let implicit = wanted == Some(operandi_core::texture_locator::IMPLICIT_UV_MAP);
            let named = match wanted {
                // Implicit UV is filled from the polygon itself (TL-Q32).
                Some(_) if implicit => None,
                Some(name) => mesh.uv_maps.get(name),
                None if legacy => None,
                None => mesh.uv_maps.values().next(),
            }
            .and_then(|faces| faces.get(index));
            let face_smooth = smooth && stored.is_none_or(|m| m.smoothing);
            let split = &mut splits[g];
            for t in tris {
                let corners = t.map(|corner| {
                    let point = face.vertices[corner];
                    let uv = match named {
                        _ if implicit => Some(operandi_core::texture_locator::implicit_uv(
                            face.vertices.len(),
                            corner,
                        )),
                        Some(corners) => corners.get(corner).copied().flatten(),
                        None => face.uv.as_ref().and_then(|uv| uv.get(corner)).copied(),
                    }
                    .unwrap_or_default();
                    let at = *split.entry(point).or_insert_with(|| {
                        r.positions
                            .push(mesh.vertices[point as usize].as_vec3().to_array());
                        (r.positions.len() - 1) as u32
                    });
                    (at, uv.as_vec2().to_array())
                });
                r.triangles.push(corners.map(|(at, _)| at));
                r.normals
                    .extend(t.map(|corner| normals[index][corner].as_vec3().to_array()));
                if wants_uvs {
                    r.uvs.extend(corners.map(|(_, uv)| uv));
                }
                // Vertex maps and variation numbers (gradient family §3-4):
                // a subdivided piece only knows its cage polygon, so it
                // gets the variation but no map (provisional, VM-Q02).
                if let Some(names) = r.shader.as_ref().map(|s| &s.attributes)
                    && !names.is_empty()
                {
                    let cage = surface.source(index).unwrap_or(index);
                    let exact = !surface.subdivided[index];
                    for corner in t {
                        for name in names {
                            let v = if exact || name.starts_with("operandi:") {
                                operandi_core::vertex_map_texture::corner_value(
                                    name,
                                    item.id,
                                    &item.mesh,
                                    cage,
                                    corner,
                                    Some(&islands),
                                )
                            } else {
                                [0.0; 4]
                            };
                            r.attributes.push(v.map(|c| c as f32));
                        }
                    }
                }
                r.smooth
                    .push((face_smooth || surface.subdivided[index]) as u8);
            }
        }
        // LP-30..32: the receiver light set of the item, or of its
        // surfaces whose shader links lights of its own (shader item v1).
        out.extend(
            groups
                .into_iter()
                .map(|(p, mut r)| {
                    let rule = p
                        .as_ref()
                        .map(|p| ShaderLightRule::of(&p.shader, scene))
                        .unwrap_or_default();
                    r.light_set = links.set_for(item.id, &rule);
                    r
                })
                .filter(|r| !r.triangles.is_empty()),
        );
        if replica {
            replicas.insert(replica_key, first..out.len());
        }
    }
    let _ = lights_done;
    Ok(out)
}

/// The polygons an item renders: its cage, with subdivision-surface
/// polygons replaced by their smooth surface at the mesh's render levels
/// (SDV-03), plus each polygon's corner normals.
struct RenderSurface<'a> {
    mesh: std::borrow::Cow<'a, operandi_core::Mesh>,
    normals: Vec<Vec<DVec3>>,
    /// Per polygon: a piece of a smooth surface. Those are smooth whatever
    /// the scene's Smoothing says, as in the viewport.
    subdivided: Vec<bool>,
    /// Per polygon: the cage polygon it comes from (`None`: the cage).
    sources: Option<Vec<usize>>,
}
impl RenderSurface<'_> {
    fn source(&self, polygon: usize) -> Option<usize> {
        match &self.sources {
            Some(s) => s.get(polygon).copied(),
            None => Some(polygon),
        }
    }
}

/// The cage when the item has no smooth polygons or its surface can't be
/// evaluated (the viewport then draws the cage too).
fn render_surface<'a>(
    item: &'a operandi_core::Item,
    smoothing: &operandi_core::shading::SmoothingSettings,
    render_level: bool,
) -> RenderSurface<'a> {
    use operandi_core::subdivision;
    let cage: &operandi_core::Mesh = &item.mesh;
    let evaluated = subdivision::has_smooth(cage)
        .then(|| {
            let mut settings = item.subdivision.clone().unwrap_or_default();
            if render_level {
                settings.subdivision_level = settings.subdivision_render_level;
                settings.catmull_clark_level = settings.catmull_clark_render_level;
            }
            subdivision::display_surface(cage, &settings).ok()
        })
        .flatten();
    let Some((mesh, sources)) = evaluated else {
        return RenderSurface {
            normals: operandi_core::shading::mesh_corner_normals(cage, smoothing),
            subdivided: vec![false; cage.faces.len()],
            mesh: std::borrow::Cow::Borrowed(cage),
            sources: None,
        };
    };
    let subdivided: Vec<bool> = sources
        .iter()
        .map(|&s| subdivision::is_smooth_polygon(cage, s))
        .collect();
    let mut normals = operandi_core::shading::mesh_corner_normals(&mesh, smoothing);
    // A smooth piece's corner takes the average normal of every piece
    // around its point, like the viewport, so the surface shades without
    // creases at any Smoothing angle.
    let face_normals: Vec<DVec3> = (0..mesh.faces.len())
        .map(|f| mesh.normal(f).normalize_or_zero())
        .collect();
    let around = mesh.point_faces();
    for (f, face) in mesh.faces.iter().enumerate() {
        if !subdivided[f] {
            continue;
        }
        for (corner, &v) in face.vertices.iter().enumerate() {
            let n = around
                .of(v)
                .iter()
                .map(|&g| face_normals[g as usize])
                .sum::<DVec3>()
                .normalize_or_zero();
            normals[f][corner] = if n == DVec3::ZERO { face_normals[f] } else { n };
        }
    }
    RenderSurface {
        mesh: std::borrow::Cow::Owned(mesh),
        normals,
        subdivided,
        sources: Some(sources),
    }
}

/// A finished render: linear RGBA floats, top row first.
#[derive(Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<f32>,
    /// The device that rendered it, e.g. "Apple M1 Max (Metal)" or "CPU".
    pub device: String,
    pub used_gpu: bool,
    pub denoised: bool,
    /// Samples per pixel actually rendered (fewer than asked when the
    /// render was finished early).
    pub samples: u32,
    /// LP-22: one picture per light group (Settings::light_groups), laid
    /// out like `rgba`.
    pub light_groups: Vec<(String, Vec<f32>)>,
    /// One picture per `Settings::passes` entry, laid out like `rgba`.
    pub passes: Vec<(Pass, Vec<f32>)>,
    /// One picture per `Settings::values` entry, laid out like `rgba`.
    pub values: Vec<(MaterialValue, Vec<f32>)>,
    /// Cycles' Cryptomatte manifest (JSON: name to hex hash) per type the
    /// passes asked for.
    pub cryptomatte_manifests: Vec<(CryptomatteType, String)>,
}

/// A Cycles pass a Render Output reads (`OC_PASS_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pass {
    Depth = 0,
    Normal = 1,
    DiffuseDirect = 2,
    DiffuseIndirect = 3,
    DiffuseColor = 4,
    GlossyDirect = 5,
    GlossyIndirect = 6,
    GlossyColor = 7,
    TransmissionDirect = 8,
    TransmissionIndirect = 9,
    TransmissionColor = 10,
    Emission = 11,
    Ao = 12,
    /// Written by every surface: incidence 0 facing to 1 edge-on.
    Incidence = 13,
    /// Written by every surface: object-space position.
    ObjectCoordinates = 14,
    /// Written by every surface: 1 where the k-th grouped output's
    /// Material Group applies (RO-Q03 v2), so its coverage.
    Group0 = 15,
    Group1 = 16,
    Group2 = 17,
    Group3 = 18,
    Group4 = 19,
    Group5 = 20,
    Group6 = 21,
    Group7 = 22,
    /// Written by every surface: the mesh UVs (u, v, 0).
    Uv = 23,
    /// Written by every surface: its Segment ID colour.
    Segment = 24,
    /// Cycles' cryptomatte passes, two ranked (id, coverage) pairs each:
    /// levels 0-1 in the first, 2-3 in the second, ...
    CryptoObject0 = 25,
    CryptoObject1 = 26,
    CryptoObject2 = 27,
    CryptoObject3 = 28,
    CryptoObject4 = 29,
    CryptoObject5 = 30,
    CryptoObject6 = 31,
    CryptoObject7 = 32,
    CryptoMaterial0 = 33,
    CryptoMaterial1 = 34,
    CryptoMaterial2 = 35,
    CryptoMaterial3 = 36,
    CryptoMaterial4 = 37,
    CryptoMaterial5 = 38,
    CryptoMaterial6 = 39,
    CryptoMaterial7 = 40,
    CryptoAsset0 = 41,
    CryptoAsset1 = 42,
    CryptoAsset2 = 43,
    CryptoAsset3 = 44,
    CryptoAsset4 = 45,
    CryptoAsset5 = 46,
    CryptoAsset6 = 47,
    CryptoAsset7 = 48,
    /// Group D: each pixel's samples over the samples rendered (0..1);
    /// filled only with adaptive sampling on (Shaded AA Samples).
    SampleCount = 49,
    /// Light scattered in volumes, first hit and later (Light Path
    /// Expressions).
    VolumeDirect = 50,
    VolumeIndirect = 51,
    /// The background seen directly (Light Path Expressions `CB`).
    Background = 52,
    /// Never sent to Cycles: the lamps-only, shadowless helper render's
    /// Diffuse Direct (Illumination (Unshadowed), RO-Q19), added to the
    /// main render's passes by `outputs::attach_unshadowed`.
    UnshadowedDirect = 1000,
}
impl Pass {
    /// The k-th cryptomatte pass (levels 2k, 2k+1) of `kind` (k < 8).
    pub fn crypto(kind: CryptomatteType, k: usize) -> Option<Pass> {
        use Pass::*;
        let all: [Pass; 8] = match kind {
            CryptomatteType::Object => [
                CryptoObject0,
                CryptoObject1,
                CryptoObject2,
                CryptoObject3,
                CryptoObject4,
                CryptoObject5,
                CryptoObject6,
                CryptoObject7,
            ],
            CryptomatteType::Material => [
                CryptoMaterial0,
                CryptoMaterial1,
                CryptoMaterial2,
                CryptoMaterial3,
                CryptoMaterial4,
                CryptoMaterial5,
                CryptoMaterial6,
                CryptoMaterial7,
            ],
            CryptomatteType::Asset => [
                CryptoAsset0,
                CryptoAsset1,
                CryptoAsset2,
                CryptoAsset3,
                CryptoAsset4,
                CryptoAsset5,
                CryptoAsset6,
                CryptoAsset7,
            ],
        };
        all.get(k).copied()
    }
    /// Which cryptomatte type a pass belongs to.
    pub fn crypto_kind(self) -> Option<CryptomatteType> {
        match self as u32 {
            25..=32 => Some(CryptomatteType::Object),
            33..=40 => Some(CryptomatteType::Material),
            41..=48 => Some(CryptomatteType::Asset),
            _ => None,
        }
    }
    /// The coverage pass of the k-th grouped output (k < 8).
    pub fn group(k: usize) -> Option<Pass> {
        use Pass::*;
        [
            Group0, Group1, Group2, Group3, Group4, Group5, Group6, Group7,
        ]
        .get(k)
        .copied()
    }
}

/// Where Cycles' kernel sources live; GPU devices compile them on first use.
/// In order: `OPERANDI_CYCLES_RESOURCES`, the app bundle's
/// `Contents/Resources/cycles`, then the development build folder.
pub fn resource_path() -> Option<std::path::PathBuf> {
    let has_source = |p: &std::path::Path| p.join("source").is_dir();
    if let Some(p) = std::env::var_os("OPERANDI_CYCLES_RESOURCES") {
        return Some(p.into());
    }
    if let Some(bundle) = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.parent()?.join("Resources/cycles")))
        .filter(|p| has_source(p))
    {
        return Some(bundle);
    }
    option_env!("OPERANDI_CYCLES_DEV_RESOURCES")
        .map(std::path::PathBuf::from)
        .filter(|p| has_source(p))
}

impl Image {
    /// 8-bit sRGB RGBA for saving as PNG or showing on screen.
    pub fn to_srgb8(&self) -> Vec<u8> {
        srgb8(&self.rgba)
    }
}

/// Linear RGBA floats to 8-bit sRGB RGBA.
pub fn srgb8(rgba: &[f32]) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let c = |v: f32| {
                let v = v.clamp(0.0, 1.0);
                let s = if v <= 0.003_130_8 {
                    v * 12.92
                } else {
                    1.055 * v.powf(1.0 / 2.4) - 0.055
                };
                (s * 255.0 + 0.5) as u8
            };
            [
                c(p[0]),
                c(p[1]),
                c(p[2]),
                (p[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            ]
        })
        .collect()
}

/// True when this build includes Cycles.
pub fn available() -> bool {
    cfg!(feature = "cycles")
}

/// Cycles' version, when it's included.
pub fn cycles_version() -> Option<String> {
    #[cfg(feature = "cycles")]
    {
        Some(ffi::version())
    }
    #[cfg(not(feature = "cycles"))]
    {
        None
    }
}

/// Renders `meshes` with Cycles. Blocks until done; call it off the UI thread.
pub fn render(meshes: &[RenderMesh], settings: &Settings) -> Result<Image> {
    render_live(meshes, settings, None, &RenderControl::default())
}

/// [`render`], steered by `control` while it runs. With a region in
/// `settings`, only the region is rendered and the rest of the picture is
/// `base` (black without one; `base` must hold width × height × 4 floats).
/// A cancelled render returns an error. Each call starts a Cycles of its
/// own; [`Session`] keeps one between renders.
pub fn render_live(
    meshes: &[RenderMesh],
    settings: &Settings,
    base: Option<Vec<f32>>,
    control: &RenderControl,
) -> Result<Image> {
    let rgba = checked_picture(meshes, settings, base, control)?;
    #[cfg(feature = "cycles")]
    {
        ffi::render(meshes, settings, rgba, control, None)
    }
    #[cfg(not(feature = "cycles"))]
    {
        let _ = rgba;
        Err("This build of Operandi doesn't include the Cycles renderer".into())
    }
}

/// The checks every render makes first, and the picture to render into.
fn checked_picture(
    meshes: &[RenderMesh],
    settings: &Settings,
    base: Option<Vec<f32>>,
    control: &RenderControl,
) -> Result<Vec<f32>> {
    let pixels = settings.width as usize * settings.height as usize * 4;
    if base.as_ref().is_some_and(|b| b.len() != pixels) {
        return Err("The picture to render into has the wrong size".into());
    }
    if settings
        .region
        .is_some_and(|r| !r.fits(settings.width, settings.height))
    {
        return Err("The render region must lie inside the picture".into());
    }
    if control.cancelled() {
        return Err("Cancelled".into());
    }
    if settings.width == 0
        || settings.height == 0
        || settings.width > 16384
        || settings.height > 16384
    {
        return Err("Image size must be between 1 and 16384 pixels".into());
    }
    for m in meshes {
        let n = m.positions.len() as u64;
        if n > u32::MAX as u64
            || m.smooth.len() != m.triangles.len()
            || (!m.uvs.is_empty() && m.uvs.len() != m.triangles.len() * 3)
            || (!m.normals.is_empty() && m.normals.len() != m.triangles.len() * 3)
            || m.shader.as_ref().is_some_and(|sh| !sh.is_valid())
            || m.triangles.iter().flatten().any(|&v| v as u64 >= n)
        {
            return Err("Render mesh has an invalid triangle".into());
        }
    }
    Ok(base.unwrap_or_else(|| vec![0.0; pixels]))
}

/// A Cycles renderer kept between renders, the way Blender's interactive
/// Cycles sync works: the device, compiled kernels, images, and the
/// meshes, surfaces, world and lights of the last render stay loaded, and
/// the next render only rebuilds what changed (moving the camera or an
/// item rebuilds nothing). Image files are read once, so a session starts
/// afresh when one of them changes on disk. One render at a time.
#[derive(Default)]
pub struct Session {
    #[cfg(feature = "cycles")]
    raw: Option<ffi::RawSession>,
    /// The image files the kept renderer read: path, size, modified time.
    images: Vec<(String, Option<(u64, std::time::SystemTime)>)>,
    /// Denoise as the kept renderer was last asked. A Cycles session that
    /// first rendered without denoising never denoises later renders (it
    /// hands back the noisy picture), so a change starts afresh.
    denoise: Option<bool>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// [`render_live`] through this session.
    pub fn render(
        &mut self,
        meshes: &[RenderMesh],
        settings: &Settings,
        base: Option<Vec<f32>>,
        control: &RenderControl,
    ) -> Result<Image> {
        let rgba = checked_picture(meshes, settings, base, control)?;
        let images = image_files(meshes, settings);
        // Cycles keeps each image it read: start afresh when one of them
        // changed on disk (a new image is simply read).
        let changed = self
            .images
            .iter()
            .any(|(path, stamp)| images.iter().any(|(p, s)| p == path && s != stamp));
        self.images = images;
        let changed = changed || self.denoise.is_some_and(|d| d != settings.denoise);
        self.denoise = Some(settings.denoise);
        #[cfg(feature = "cycles")]
        {
            if changed {
                self.raw = None;
            }
            if self.raw.is_none() {
                self.raw = Some(ffi::RawSession::new()?);
            }
            ffi::render(meshes, settings, rgba, control, self.raw.as_mut())
        }
        #[cfg(not(feature = "cycles"))]
        {
            let _ = (rgba, changed);
            Err("This build of Operandi doesn't include the Cycles renderer".into())
        }
    }

    /// Lets go of the kept renderer and everything it holds.
    pub fn release(&mut self) {
        #[cfg(feature = "cycles")]
        {
            self.raw = None;
        }
        self.images.clear();
        self.denoise = None;
    }

    /// True while a renderer is kept.
    pub fn is_kept(&self) -> bool {
        #[cfg(feature = "cycles")]
        {
            self.raw.is_some()
        }
        #[cfg(not(feature = "cycles"))]
        {
            false
        }
    }
}

/// Every image file a render reads, with its size and modified time.
fn image_files(
    meshes: &[RenderMesh],
    settings: &Settings,
) -> Vec<(String, Option<(u64, std::time::SystemTime)>)> {
    let mut paths: Vec<&str> = Vec::new();
    for m in meshes {
        if let Some(sh) = &m.shader {
            paths.extend(sh.images.iter().map(String::as_str));
        }
    }
    for l in &settings.world {
        for t in &l.layers {
            if let EnvironmentTextureSource::Image(path) = &t.source {
                paths.push(path);
            }
        }
    }
    for l in settings.lights.iter().flatten() {
        paths.extend(l.layers.iter().map(|(p, ..)| p.as_str()));
    }
    paths.sort_unstable();
    paths.dedup();
    paths
        .into_iter()
        .map(|p| {
            let stamp = std::fs::metadata(p)
                .ok()
                .and_then(|m| Some((m.len(), m.modified().ok()?)));
            (p.to_owned(), stamp)
        })
        .collect()
}

/// The renderer the Preview keeps while it is open.
static KEPT: std::sync::Mutex<Option<Session>> = std::sync::Mutex::new(None);

/// [`render_live`] through the one kept [`Session`] (the Preview's): a
/// later call waits for an earlier one to finish.
pub fn render_kept(
    meshes: &[RenderMesh],
    settings: &Settings,
    base: Option<Vec<f32>>,
    control: &RenderControl,
) -> Result<Image> {
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    kept.get_or_insert_with(Session::new)
        .render(meshes, settings, base, control)
}

/// Frees the kept renderer (the Preview closed). Waits for a render using
/// it to finish; cancel that first.
pub fn release_kept() {
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    *kept = None;
}

/// Checks that the Cycles library speaks the same C interface as this app.
///
/// liboperandi_cycles is built separately (scripts/build-cycles.sh), so a
/// library left over from an older run can sit next to a newer app. Reading
/// structs of the wrong shape would be undefined behaviour, so before the
/// first render the app compares OC_ABI_VERSION and the size of every shared
/// struct with what the library reports, and refuses to render on a mismatch.
///
/// The library is linked at load time (build.rs), not opened with dlopen, so
/// a library too old to have `oc_abi_version` at all already fails when the
/// app starts (the dynamic loader reports the missing symbol).
#[cfg_attr(not(feature = "cycles"), allow(dead_code))]
mod abi {
    /// Must equal OC_ABI_VERSION in cycles/operandi_cycles.h. Bump both
    /// whenever a shared struct, enum value or function signature changes.
    /// v18: the general node list (Nodes N1, NM-30).
    /// v26 (material values): OcSurface.values, OcSettings.value_*.
    /// v27 (Cycles built-in outputs): OcMesh names and segment colour,
    /// the manifest callback, the UV / Segment / cryptomatte passes.
    pub const OC_ABI_VERSION: u32 = 29;

    /// One shared struct: its name, its size here and its size in the library.
    pub struct StructSize {
        pub name: &'static str,
        pub app: usize,
        pub library: usize,
    }

    /// `None` when the library matches; otherwise the message to show.
    pub fn mismatch(
        app_version: u32,
        library_version: u32,
        sizes: &[StructSize],
    ) -> Option<String> {
        if library_version != app_version {
            let state = if library_version < app_version {
                "out of date"
            } else {
                "newer than this app"
            };
            return Some(format!(
                "Cycles library is {state} (ABI v{library_version}, app expects \
                 v{app_version}). Rerun scripts/build-cycles.sh."
            ));
        }
        let wrong: Vec<String> = sizes
            .iter()
            .filter(|s| s.app != s.library)
            .map(|s| {
                format!(
                    "{} is {} bytes in the library, {} in the app",
                    s.name, s.library, s.app
                )
            })
            .collect();
        if wrong.is_empty() {
            return None;
        }
        Some(format!(
            "Cycles library doesn't match this app (both ABI v{app_version}): {}. \
             Rerun scripts/build-cycles.sh; if that doesn't help, \
             OC_ABI_VERSION needs bumping.",
            wrong.join("; ")
        ))
    }
}

#[cfg(feature = "cycles")]
mod ffi {
    use super::abi::{OC_ABI_VERSION, StructSize, mismatch};
    use super::{Image, RenderControl, RenderMesh, Settings};
    use operandi_core::Result;
    use std::ffi::c_void;
    use std::ffi::{CStr, CString, c_char};
    use std::sync::OnceLock;

    #[repr(C)]
    struct OcMesh {
        positions: *const f32,
        vertex_count: u32,
        triangles: *const u32,
        smooth: *const u8,
        triangle_count: u32,
        has_material: u32,
        base_color: [f32; 3],
        ior: f32,
        surface: *const OcSurface,
        uvs: *const f32,
        light_set: u32,
        light_membership: u64,
        attribute_values: *const f32,
        normals: *const f32,
        transform: [f32; 12],
        geometry_key: u64,
        surface_key: u64,
        name: *const c_char,
        asset_name: *const c_char,
        material_name: *const c_char,
        segment_color: [f32; 3],
    }

    #[repr(C)]
    struct OcExpr {
        op: u32,
        mode: u32,
        a: i32,
        b: i32,
        fac: i32,
        value: [f32; 3],
        image: i32,
        xform: [f32; 12],
    }

    #[repr(C)]
    struct OcSurface {
        exprs: *const OcExpr,
        expr_count: u32,
        inputs: [i32; operandi_core::shader_eval::Input::COUNT],
        images: *const *const c_char,
        image_count: u32,
        visible_to_camera: u32,
        visible_to_diffuse: u32,
        visible_to_glossy: u32,
        visible_to_transmission: u32,
        cast_shadows: u32,
        shadow_catcher: u32,
        image_filters: *const u32,
        image_raw: *const u32,
        bump: i32,
        normal_map: i32,
        displacement: i32,
        coat_bump: f32,
        transparency_mode: u32,
        transparency_amount: i32,
        transparency_color: i32,
        refraction_ior: i32,
        refraction_roughness: i32,
        volume: u32,
        volume_color: [f32; 3],
        volume_density: f32,
        volume_attenuation: f32,
        volume_origin: [f32; 3],
        lightgroup: *const c_char,
        image_vectors: *const i32,
        image_extensions: *const u32,
        holdout: f32,
        fog_type: u32,
        fog_color: [f32; 3],
        fog_use_environment: u32,
        fog_start: f32,
        fog_end: f32,
        fog_density: f32,
        ramps: *const f32,
        ramp_count: u32,
        attributes: *const *const c_char,
        attribute_count: u32,
        rounded_edge: f32,
        absorption_color: [f32; 3],
        absorption_density: f32,
        nodes: *const OcNode,
        node_count: u32,
        node_links: *const OcNodeLink,
        node_link_count: u32,
        node_mix: i32,
        scatter_color: [f32; 3],
        scatter_density: f32,
        scatter_anisotropy: f32,
        thin_wall: u32,
        glass_principled: u32,
        output_groups: u32,
        values: [i32; super::MaterialValue::COUNT],
        expr_sockets: *const *const c_char,
        expr_socket_count: u32,
    }

    #[repr(C)]
    struct OcNodeValue {
        socket: *const c_char,
        kind: u32,
        value: [f32; 4],
        text: *const c_char,
        array: *const f32,
        array_count: u32,
        expr: i32,
    }

    #[repr(C)]
    struct OcNode {
        type_: *const c_char,
        values: *const OcNodeValue,
        value_count: u32,
    }

    #[repr(C)]
    struct OcNodeLink {
        from: u32,
        from_socket: *const c_char,
        to: u32,
        to_socket: *const c_char,
    }

    /// A surface's node graph as C structs, with everything they point at.
    /// The vectors are filled once and never grow afterwards, so the
    /// pointers into them stay valid while this lives.
    #[derive(Default)]
    struct OwnedGraph {
        _strings: Vec<CString>,
        _ramps: Vec<Vec<f32>>,
        _values: Vec<Vec<OcNodeValue>>,
        nodes: Vec<OcNode>,
        links: Vec<OcNodeLink>,
        sockets: Vec<*const c_char>,
    }
    impl OwnedGraph {
        fn of(g: &super::RenderGraph) -> Self {
            use operandi_core::node_graph::FlatValue;
            let mut out = OwnedGraph::default();
            let cstr = |strings: &mut Vec<CString>, s: &str| {
                let c = CString::new(s).unwrap_or_default();
                // A CString's bytes live on the heap: moving it into the
                // vector keeps the pointer.
                let p = c.as_ptr();
                strings.push(c);
                p
            };
            for n in &g.graph.nodes {
                let mut values = Vec::with_capacity(n.values.len());
                for (socket, v) in &n.values {
                    let mut ov = OcNodeValue {
                        socket: cstr(&mut out._strings, socket),
                        kind: 0,
                        value: [0.0; 4],
                        text: std::ptr::null(),
                        array: std::ptr::null(),
                        array_count: 0,
                        expr: -1,
                    };
                    match v {
                        FlatValue::Float(x) => {
                            ov.kind = 0;
                            ov.value[0] = *x as f32;
                        }
                        FlatValue::Vec3(x) => {
                            ov.kind = 1;
                            ov.value = [x[0] as f32, x[1] as f32, x[2] as f32, 0.0];
                        }
                        FlatValue::Int(x) => {
                            ov.kind = 2;
                            ov.value[0] = *x as f32;
                        }
                        FlatValue::Bool(b) => {
                            ov.kind = 3;
                            ov.value[0] = f32::from(u8::from(*b));
                        }
                        FlatValue::Text(t) => {
                            ov.kind = 4;
                            ov.text = cstr(&mut out._strings, t);
                        }
                        FlatValue::Ramp(table) => {
                            ov.kind = 5;
                            let flat: Vec<f32> =
                                table.iter().flatten().map(|&c| c as f32).collect();
                            ov.array = flat.as_ptr();
                            ov.array_count = table.len() as u32;
                            out._ramps.push(flat);
                        }
                        FlatValue::Driven(k) => {
                            ov.kind = 6;
                            ov.expr = g.drivers.get(*k).copied().unwrap_or(-1);
                        }
                    }
                    values.push(ov);
                }
                out.nodes.push(OcNode {
                    type_: cstr(&mut out._strings, n.cycles),
                    values: values.as_ptr(),
                    value_count: values.len() as u32,
                });
                out._values.push(values);
            }
            for l in &g.graph.links {
                out.links.push(OcNodeLink {
                    from: l.from as u32,
                    from_socket: cstr(&mut out._strings, &l.from_socket),
                    to: l.to as u32,
                    to_socket: cstr(&mut out._strings, &l.to_socket),
                });
            }
            for s in &g.sockets {
                let p = cstr(&mut out._strings, s);
                out.sockets.push(p);
            }
            out
        }
    }

    #[repr(C)]
    struct OcEnvironment {
        kind: u32,
        zenith: [f32; 3],
        sky: [f32; 3],
        ground: [f32; 3],
        nadir: [f32; 3],
        sky_exponent: f32,
        ground_exponent: f32,
        intensity: f32,
        up: [f32; 3],
        visible: [u32; 4],
        material_opacity: f32,
        material_invert: u32,
        layers: *const OcEnvLayer,
        layer_count: u32,
        physical: u32,
        to_sky: [f32; 9],
        sun_elevation: f32,
        sun_rotation: f32,
        sun_size: f32,
        ground_albedo: f32,
        aerosol: f32,
    }

    #[repr(C)]
    struct OcEnvLayer {
        /// 0 = constant colour, 1 = latitude-longitude image, 2 = Light
        /// Probe (mirror-ball) image.
        kind: u32,
        color: [f32; 3],
        opacity: f32,
        mix: u32,
        invert: u32,
        image: *const c_char,
        /// Rows of the matrix taking a world direction to Cycles' Z-up
        /// equirectangular frame.
        to_image: [f32; 9],
    }

    #[repr(C)]
    struct OcSettings {
        width: u32,
        height: u32,
        samples: u32,
        camera_right: [f32; 3],
        camera_up: [f32; 3],
        camera_forward: [f32; 3],
        camera_position: [f32; 3],
        fov: f32,
        sun_direction: [f32; 3],
        sun_strength: f32,
        environments: *const OcEnvironment,
        environment_count: u32,
        environment_fog_color: [f32; 3],
        base_color: [f32; 3],
        roughness: f32,
        use_gpu: u32,
        denoise: u32,
        resource_path: *const c_char,
        camera_orthographic: u32,
        ortho_half_short: f32,
        shift: [f32; 2],
        clip_near: f32,
        clip_far: f32,
        lights: *const OcLight,
        light_count: u32,
        use_scene_lights: u32,
        region: [u32; 4],
        max_diffuse_bounces: i32,
        max_glossy_bounces: i32,
        max_transmission_bounces: i32,
        no_shadows: u32,
        adaptive_threshold: f32,
        threads: u32,
        transparent_film: u32,
        dicing_rate: f32,
        world_hidden_from_diffuse: u32,
        dome: [f32; 3],
        light_groups: *const *const c_char,
        light_group_out: *const *mut f32,
        light_group_count: u32,
        panorama: u32,
        panorama_range: [f32; 4],
        aperture_size: f32,
        focal_distance: f32,
        blades: u32,
        blades_rotation: f32,
        passes: *const u32,
        pass_out: *const *mut f32,
        pass_count: u32,
        ao_distance: f32,
        max_bounces: i32,
        max_transparent_bounces: i32,
        sample_clamp_indirect: f32,
        caustics_reflective: u32,
        caustics_refractive: u32,
        direct_caustics: u32,
        time_limit: f32,
        world_key: u64,
        lights_key: u64,
        value_passes: *const u32,
        value_out: *const *mut f32,
        value_count: u32,
        manifest_user: *mut c_void,
        on_cryptomatte_manifest: Option<unsafe extern "C" fn(*mut c_void, u32, *const c_char)>,
        world_lightgroup: *const c_char,
        lamps_only: u32,
    }

    /// Collects Cycles' Cryptomatte manifests (`manifest_user` is a
    /// `Vec<(u32, String)>`).
    unsafe extern "C" fn on_manifest(user: *mut c_void, kind: u32, json: *const c_char) {
        if user.is_null() || json.is_null() {
            return;
        }
        // SAFETY: `user` is the `&mut Vec` `render` passes for this call
        // only, and `json` a NUL-terminated string borrowed for the call.
        let (list, text) = unsafe {
            (
                &mut *user.cast::<Vec<(u32, String)>>(),
                CStr::from_ptr(json).to_string_lossy().into_owned(),
            )
        };
        list.push((kind, text));
    }

    #[repr(C)]
    struct OcControl {
        command: *const u32,
        user: *mut c_void,
        on_progress: Option<unsafe extern "C" fn(*mut c_void, u32, u32, u32)>,
        on_preview: Option<unsafe extern "C" fn(*mut c_void, *const f32, u32, u32)>,
    }

    unsafe extern "C" fn on_progress(user: *mut c_void, done: u32, total: u32, paused: u32) {
        // SAFETY: `user` is the `RenderControl` that `render` borrows for the
        // whole oc_render call, and callbacks only run during that call.
        let control = unsafe { &*(user as *const RenderControl) };
        control.report_progress(done, total, paused != 0);
    }

    unsafe extern "C" fn on_preview(user: *mut c_void, rgba: *const f32, width: u32, height: u32) {
        if rgba.is_null() {
            return;
        }
        // SAFETY: as in `on_progress`; the C side passes width*height*4
        // floats that stay valid for this call.
        let (control, pixels) = unsafe {
            (
                &*(user as *const RenderControl),
                std::slice::from_raw_parts(rgba, width as usize * height as usize * 4),
            )
        };
        control.report_preview(pixels);
    }

    #[repr(C)]
    struct OcLight {
        kind: u32,
        position: [f32; 3],
        right: [f32; 3],
        up: [f32; 3],
        direction: [f32; 3],
        strength: [f32; 3],
        angle: f32,
        radius: f32,
        spot_size: f32,
        spot_blend: f32,
        size: [f32; 2],
        ellipse: u32,
        cast_shadow: u32,
        visible_to_camera: u32,
        affects_diffuse: u32,
        affects_specular: u32,
        visible_to_reflection: u32,
        visible_to_refraction: u32,
        ies: *const c_char,
        lightgroup: *const c_char,
        layer_images: *const *const c_char,
        layer_blend: *const u32,
        layer_opacity: *const f32,
        layer_count: u32,
        gobo_tan_half: f32,
        light_membership: u64,
        shadow_membership: u64,
        caustics: u32,
    }

    /// The C side's kept renderer; only ever behind a pointer.
    #[repr(C)]
    pub struct OcSession {
        _opaque: [u8; 0],
    }

    /// An `OcSession` owned by Rust: freed on drop.
    pub struct RawSession(*mut OcSession);

    // SAFETY: the C session has no thread affinity (Cycles runs its own
    // threads and locks its scene); Rust only hands it to one render at a
    // time through `&mut`.
    unsafe impl Send for RawSession {}

    impl RawSession {
        pub fn new() -> Result<Self> {
            check_abi()?;
            // SAFETY: takes nothing; returns a new session or NULL.
            let raw = unsafe { oc_session_new() };
            if raw.is_null() {
                return Err("Cycles could not make a renderer".into());
            }
            Ok(Self(raw))
        }
    }

    impl Drop for RawSession {
        fn drop(&mut self) {
            // SAFETY: `self.0` came from oc_session_new, is freed only here,
            // and no render is using it (renders borrow `&mut self`).
            unsafe { oc_session_free(self.0) }
        }
    }

    /// A content key: `value`'s Debug text hashed (never 0). Debug text
    /// covers every field and prints floats exactly, so equal keys mean
    /// equal content.
    fn debug_key(value: &impl std::fmt::Debug) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::hash::DefaultHasher::new();
        format!("{value:?}").hash(&mut h);
        h.finish().max(1)
    }

    /// `bytes_of` for plain number arrays: their memory as bytes.
    fn hash_slice<T: Copy>(h: &mut impl std::hash::Hasher, items: &[T]) {
        h.write_usize(items.len());
        // SAFETY: only called with arrays of f32/u32/u8 (no padding, every
        // bit pattern valid); the byte view covers exactly the slice.
        let bytes = unsafe {
            std::slice::from_raw_parts(items.as_ptr().cast::<u8>(), std::mem::size_of_val(items))
        };
        h.write(bytes);
    }

    /// The key of everything `fill_mesh` reads from a mesh (C side).
    pub(crate) fn geometry_key(m: &RenderMesh) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::hash::DefaultHasher::new();
        hash_slice(&mut h, &m.positions);
        hash_slice(&mut h, &m.triangles);
        hash_slice(&mut h, &m.smooth);
        hash_slice(&mut h, &m.uvs);
        hash_slice(&mut h, &m.normals);
        hash_slice(&mut h, &m.attributes);
        m.shader.as_ref().map(|s| &s.attributes).hash(&mut h);
        h.finish().max(1)
    }

    #[repr(C)]
    struct OcReport {
        device: [c_char; 128],
        used_gpu: u32,
        denoised: u32,
        samples: u32,
    }

    unsafe extern "C" {
        fn oc_render(
            meshes: *const OcMesh,
            mesh_count: u32,
            settings: *const OcSettings,
            rgba_out: *mut f32,
            control: *mut OcControl,
            report: *mut OcReport,
            error: *mut c_char,
            error_len: usize,
        ) -> i32;
        fn oc_session_new() -> *mut OcSession;
        fn oc_session_free(session: *mut OcSession);
        fn oc_session_render(
            session: *mut OcSession,
            meshes: *const OcMesh,
            mesh_count: u32,
            settings: *const OcSettings,
            rgba_out: *mut f32,
            control: *mut OcControl,
            report: *mut OcReport,
            error: *mut c_char,
            error_len: usize,
        ) -> i32;
        fn oc_cycles_version() -> *const c_char;
        fn oc_abi_version() -> u32;
        fn oc_struct_size(which: u32) -> usize;
    }

    /// The shared structs, in OC_STRUCT_* order (operandi_cycles.h).
    const STRUCTS: [(&str, usize); 12] = [
        ("OcExpr", size_of::<OcExpr>()),
        ("OcSurface", size_of::<OcSurface>()),
        ("OcMesh", size_of::<OcMesh>()),
        ("OcLight", size_of::<OcLight>()),
        ("OcSettings", size_of::<OcSettings>()),
        ("OcReport", size_of::<OcReport>()),
        ("OcControl", size_of::<OcControl>()),
        ("OcEnvironment", size_of::<OcEnvironment>()),
        ("OcEnvLayer", size_of::<OcEnvLayer>()),
        ("OcNode", size_of::<OcNode>()),
        ("OcNodeValue", size_of::<OcNodeValue>()),
        ("OcNodeLink", size_of::<OcNodeLink>()),
    ];

    /// [`super::abi`]'s check against the linked library, done once.
    pub fn check_abi() -> Result<()> {
        static CHECK: OnceLock<Result<()>> = OnceLock::new();
        CHECK
            .get_or_init(|| check_abi_against(OC_ABI_VERSION))
            .clone()
    }

    /// The check itself, with the app's version passed in so a test can
    /// fake an out-of-date library.
    pub fn check_abi_against(app_version: u32) -> Result<()> {
        // SAFETY: both functions take plain integers, return plain integers
        // and touch no other state.
        let library_version = unsafe { oc_abi_version() };
        let sizes: Vec<StructSize> = STRUCTS
            .iter()
            .enumerate()
            .map(|(which, &(name, app))| StructSize {
                name,
                app,
                // SAFETY: as above; unknown ids just return 0.
                library: unsafe { oc_struct_size(which as u32) },
            })
            .collect();
        match mismatch(app_version, library_version, &sizes) {
            Some(message) => Err(message),
            None => Ok(()),
        }
    }

    pub fn version() -> String {
        // SAFETY: oc_cycles_version returns a pointer to a static,
        // NUL-terminated string literal inside the library.
        unsafe { CStr::from_ptr(oc_cycles_version()) }
            .to_string_lossy()
            .into_owned()
    }

    pub fn render(
        meshes: &[RenderMesh],
        s: &Settings,
        mut rgba: Vec<f32>,
        control: &RenderControl,
        session: Option<&mut RawSession>,
    ) -> Result<Image> {
        check_abi()?;
        let v = |d: glam::DVec3| d.as_vec3().to_array();
        // Everything a surface points at, kept alive for the whole call.
        struct Owned {
            exprs: Vec<OcExpr>,
            // Owns the strings `path_ptrs` points at.
            _paths: Vec<CString>,
            path_ptrs: Vec<*const c_char>,
            _attributes: Vec<CString>,
            attribute_ptrs: Vec<*const c_char>,
            graph: Option<OwnedGraph>,
        }
        let owned: Vec<Option<Owned>> = meshes
            .iter()
            .map(|m| {
                m.shader.as_ref().map(|sh| {
                    let paths: Vec<CString> = sh
                        .images
                        .iter()
                        .map(|p| CString::new(p.as_str()).unwrap_or_default())
                        .collect();
                    let path_ptrs = paths.iter().map(|p| p.as_ptr()).collect();
                    let attributes: Vec<CString> = sh
                        .attributes
                        .iter()
                        .map(|a| CString::new(a.as_str()).unwrap_or_default())
                        .collect();
                    Owned {
                        exprs: sh
                            .ops
                            .iter()
                            .map(|o| OcExpr {
                                op: o.op,
                                mode: o.mode,
                                a: o.a,
                                b: o.b,
                                fac: o.fac,
                                value: o.value,
                                image: o.image,
                                xform: o.xform,
                            })
                            .collect(),
                        _paths: paths,
                        path_ptrs,
                        attribute_ptrs: attributes.iter().map(|a| a.as_ptr()).collect(),
                        _attributes: attributes,
                        graph: sh.node_graph.as_ref().map(OwnedGraph::of),
                    }
                })
            })
            .collect();
        let surface_groups: Vec<Option<CString>> = meshes
            .iter()
            .map(|m| {
                m.shader
                    .as_ref()
                    .and_then(|s| s.lightgroup.as_deref())
                    .and_then(|g| CString::new(g).ok())
            })
            .collect();
        let surfaces: Vec<Option<OcSurface>> = meshes
            .iter()
            .zip(&owned)
            .zip(&surface_groups)
            .map(|((m, o), group)| {
                let (sh, o) = (m.shader.as_ref()?, o.as_ref()?);
                let beam = sh.volume;
                Some(OcSurface {
                    exprs: o.exprs.as_ptr(),
                    expr_count: o.exprs.len() as u32,
                    inputs: sh.inputs,
                    images: o.path_ptrs.as_ptr(),
                    image_count: o.path_ptrs.len() as u32,
                    visible_to_camera: sh.visible_to_camera as u32,
                    visible_to_diffuse: sh.visible_to_diffuse as u32,
                    visible_to_glossy: sh.visible_to_glossy as u32,
                    visible_to_transmission: sh.visible_to_transmission as u32,
                    cast_shadows: sh.cast_shadows as u32,
                    shadow_catcher: sh.shadow_catcher as u32,
                    image_filters: sh.image_filters.as_ptr(),
                    image_raw: sh.image_raw.as_ptr(),
                    bump: sh.bump,
                    normal_map: sh.normal_map,
                    displacement: sh.displacement,
                    coat_bump: sh.coat_bump,
                    transparency_mode: sh.transparency_mode,
                    transparency_amount: sh.transparency_amount,
                    transparency_color: sh.transparency_color,
                    refraction_ior: sh.refraction_ior,
                    refraction_roughness: sh.refraction_roughness,
                    volume: beam.is_some() as u32,
                    volume_color: beam.map_or([0.0; 3], |b| b.color),
                    volume_density: beam.map_or(0.0, |b| b.density),
                    volume_attenuation: beam.map_or(0.0, |b| b.attenuation),
                    volume_origin: beam.map_or([0.0; 3], |b| b.origin),
                    lightgroup: group.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
                    image_vectors: sh.image_vectors.as_ptr(),
                    image_extensions: sh.image_extensions.as_ptr(),
                    holdout: sh.look.holdout,
                    fog_type: sh.look.fog.map_or(0, |f| f.kind),
                    fog_color: sh.look.fog.map_or([0.0; 3], |f| f.color),
                    fog_use_environment: sh.look.fog.is_some_and(|f| f.use_environment) as u32,
                    fog_start: sh.look.fog.map_or(0.0, |f| f.start),
                    fog_end: sh.look.fog.map_or(0.0, |f| f.end),
                    fog_density: sh.look.fog.map_or(0.0, |f| f.density),
                    ramps: sh.ramps.as_ptr().cast(),
                    ramp_count: sh.ramps.len() as u32,
                    attributes: o.attribute_ptrs.as_ptr(),
                    attribute_count: o.attribute_ptrs.len() as u32,
                    rounded_edge: sh.rounded_edge,
                    absorption_color: sh.absorption.0,
                    absorption_density: sh.absorption.1,
                    nodes: o
                        .graph
                        .as_ref()
                        .map_or(std::ptr::null(), |g| g.nodes.as_ptr()),
                    node_count: o.graph.as_ref().map_or(0, |g| g.nodes.len() as u32),
                    node_links: o
                        .graph
                        .as_ref()
                        .map_or(std::ptr::null(), |g| g.links.as_ptr()),
                    node_link_count: o.graph.as_ref().map_or(0, |g| g.links.len() as u32),
                    node_mix: sh.node_graph.as_ref().map_or(-1, |g| g.mix),
                    scatter_color: sh.scatter.0,
                    scatter_density: sh.scatter.1,
                    scatter_anisotropy: sh.scatter.2,
                    thin_wall: sh.thin_wall as u32,
                    glass_principled: sh.glass_principled as u32,
                    output_groups: sh.output_groups,
                    values: sh.values,
                    expr_sockets: o
                        .graph
                        .as_ref()
                        .map_or(std::ptr::null(), |g| g.sockets.as_ptr()),
                    expr_socket_count: o.graph.as_ref().map_or(0, |g| g.sockets.len() as u32),
                })
            })
            .collect();
        // Cryptomatte names, kept alive for the call.
        let c = |t: &str| CString::new(t.replace('\0', "")).unwrap_or_default();
        let names: Vec<[CString; 3]> = meshes
            .iter()
            .map(|m| [c(&m.name), c(&m.asset_name), c(&m.material_name)])
            .collect();
        let raw: Vec<OcMesh> = meshes
            .iter()
            .zip(&surfaces)
            .zip(&names)
            .map(|((m, surface), names)| OcMesh {
                name: names[0].as_ptr(),
                asset_name: names[1].as_ptr(),
                material_name: names[2].as_ptr(),
                segment_color: m.segment_color,
                positions: m.positions.as_ptr().cast(),
                vertex_count: m.positions.len() as u32,
                triangles: m.triangles.as_ptr().cast(),
                smooth: m.smooth.as_ptr(),
                triangle_count: m.triangles.len() as u32,
                has_material: 0,
                base_color: [0.0; 3],
                ior: 1.5,
                surface: surface
                    .as_ref()
                    .map_or(std::ptr::null(), |s| s as *const OcSurface),
                uvs: if m.uvs.len() == m.triangles.len() * 3 && !m.uvs.is_empty() {
                    m.uvs.as_ptr().cast()
                } else {
                    std::ptr::null()
                },
                light_set: m.light_set,
                light_membership: m.light_membership.unwrap_or(u64::MAX),
                normals: if m.normals.len() == m.triangles.len() * 3 && !m.normals.is_empty() {
                    m.normals.as_ptr().cast()
                } else {
                    std::ptr::null()
                },
                transform: m
                    .transform
                    .unwrap_or([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
                geometry_key: geometry_key(m),
                // The material name is the Cryptomatte Material id, so
                // equal surfaces of different materials stay apart.
                surface_key: m
                    .shader
                    .as_ref()
                    .map_or(0, |sh| debug_key(&(sh, &m.material_name))),
                attribute_values: if !m.attributes.is_empty()
                    && m.shader.as_ref().is_some_and(|s| {
                        m.attributes.len() == m.triangles.len() * 3 * s.attributes.len()
                    }) {
                    m.attributes.as_ptr().cast()
                } else {
                    std::ptr::null()
                },
            })
            .collect();
        // Keep the C string alive for the whole call.
        let resources = super::resource_path()
            .and_then(|p| CString::new(p.to_string_lossy().into_owned()).ok());
        // IES texts, kept alive for the whole call; NUL bytes can't be in a
        // C string, so such a file is dropped (the light renders plain).
        let ies: Vec<Option<CString>> = s
            .lights
            .iter()
            .flatten()
            .map(|l| l.ies.as_deref().and_then(|t| CString::new(t).ok()))
            .collect();
        // Light groups and gobo layers, kept alive for the call.
        let light_extras: Vec<(
            Option<CString>,
            Vec<CString>,
            Vec<*const c_char>,
            Vec<u32>,
            Vec<f32>,
        )> = s
            .lights
            .iter()
            .flatten()
            .map(|l| {
                let group = l.lightgroup.as_deref().and_then(|g| CString::new(g).ok());
                let images: Vec<CString> = l
                    .layers
                    .iter()
                    .map(|(p, ..)| CString::new(p.as_str()).unwrap_or_default())
                    .collect();
                let ptrs = images.iter().map(|c| c.as_ptr()).collect();
                let blends = l.layers.iter().map(|(_, b, _)| *b).collect();
                let opacities = l.layers.iter().map(|(.., o)| *o).collect();
                (group, images, ptrs, blends, opacities)
            })
            .collect();
        let lights: Vec<OcLight> = s
            .lights
            .iter()
            .flatten()
            .zip(&ies)
            .zip(&light_extras)
            .map(|((l, ies), extra)| {
                use super::LightShape;
                let (kind, angle, radius, spot_size, spot_blend, size, ellipse) = match l.shape {
                    LightShape::Sun { angle } => (0, angle, 0.0, 0.0, 0.0, [0.0; 2], false),
                    LightShape::Point { radius } => (1, 0.0, radius, 0.0, 0.0, [0.0; 2], false),
                    LightShape::Spot {
                        radius,
                        size,
                        blend,
                    } => (2, 0.0, radius, size, blend, [0.0; 2], false),
                    LightShape::Area { size, ellipse } => (3, 0.0, 0.0, 0.0, 0.0, size, ellipse),
                    LightShape::Portal { size, ellipse } => (4, 0.0, 0.0, 0.0, 0.0, size, ellipse),
                };
                (
                    l,
                    (ies, extra),
                    kind,
                    angle,
                    radius,
                    spot_size,
                    spot_blend,
                    size,
                    ellipse,
                )
            })
            .map(
                |(l, (ies, extra), kind, angle, radius, spot_size, spot_blend, size, ellipse)| {
                    OcLight {
                        kind,
                        position: v(l.position),
                        right: v(l.right),
                        up: v(l.up),
                        direction: v(l.direction),
                        strength: l.strength,
                        angle,
                        radius,
                        spot_size,
                        spot_blend,
                        size,
                        ellipse: ellipse as u32,
                        cast_shadow: l.cast_shadow as u32,
                        visible_to_camera: l.visible_to_camera as u32,
                        affects_diffuse: l.affects_diffuse as u32,
                        affects_specular: l.affects_specular as u32,
                        visible_to_reflection: l.visible_to_reflection as u32,
                        visible_to_refraction: l.visible_to_refraction as u32,
                        ies: ies.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
                        lightgroup: extra.0.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
                        layer_images: extra.2.as_ptr(),
                        layer_blend: extra.3.as_ptr(),
                        layer_opacity: extra.4.as_ptr(),
                        layer_count: extra.2.len() as u32,
                        gobo_tan_half: l.gobo_tan_half,
                        light_membership: l.light_membership.unwrap_or(u64::MAX),
                        shadow_membership: l
                            .shadow_membership
                            .or(l.light_membership)
                            .unwrap_or(u64::MAX),
                        caustics: if l.caustics_only { 2 } else { 0 },
                    }
                },
            )
            .collect();
        // LP-22: a picture per light group, written by the C side.
        let group_names: Vec<CString> = s
            .light_groups
            .iter()
            .filter_map(|g| CString::new(g.as_str()).ok())
            .collect();
        let group_name_ptrs: Vec<*const c_char> = group_names.iter().map(|c| c.as_ptr()).collect();
        // Group D: the world's light group (Light Path Expressions).
        let world_group: Option<CString> = s
            .world_lightgroup
            .as_deref()
            .and_then(|g| CString::new(g).ok());
        let mut group_pixels: Vec<Vec<f32>> = group_names
            .iter()
            .map(|_| vec![0.0; s.width as usize * s.height as usize * 4])
            .collect();
        let group_out_ptrs: Vec<*mut f32> =
            group_pixels.iter_mut().map(|p| p.as_mut_ptr()).collect();
        // Render Output passes, written by the C side.
        let pass_codes: Vec<u32> = s.passes.iter().map(|p| *p as u32).collect();
        let mut pass_pixels: Vec<Vec<f32>> = s
            .passes
            .iter()
            .map(|_| vec![0.0; s.width as usize * s.height as usize * 4])
            .collect();
        let pass_out_ptrs: Vec<*mut f32> = pass_pixels.iter_mut().map(|p| p.as_mut_ptr()).collect();
        // Material values (OC_VALUE_*), written like the passes.
        let value_codes: Vec<u32> = s.values.iter().map(|v| v.index() as u32).collect();
        let mut value_pixels: Vec<Vec<f32>> = s
            .values
            .iter()
            .map(|_| vec![0.0; s.width as usize * s.height as usize * 4])
            .collect();
        let value_out_ptrs: Vec<*mut f32> =
            value_pixels.iter_mut().map(|p| p.as_mut_ptr()).collect();
        let mut manifests: Vec<(u32, String)> = Vec::new();
        let aspect = s.width as f64 / s.height.max(1) as f64;
        // Cycles' orthographic view plane is sized by its shorter side.
        let ortho_half_short = s.view.orthographic.map_or(0.0, |half_h| {
            if aspect >= 1.0 {
                half_h
            } else {
                half_h * aspect
            }
        });
        let bounces = s.light_paths.bounce_limits(s.rays.depths);
        // Cycles' sky is Z-up: Operandi (x, y, z) is its (x, -z, y), with
        // `item_inverse` applied (identity for a scene's environment).
        let sky_frame = |l: &super::EnvironmentLayer| {
            let zup = glam::DMat3::from_cols(
                glam::DVec3::new(1.0, 0.0, 0.0),
                glam::DVec3::new(0.0, 0.0, 1.0),
                glam::DVec3::new(0.0, -1.0, 0.0),
            );
            zup * l.item_inverse
        };
        let rows = |m: glam::DMat3| {
            let mut out = [0.0f32; 9];
            for r in 0..3 {
                let row = m.row(r);
                out[r * 3] = row.x as f32;
                out[r * 3 + 1] = row.y as f32;
                out[r * 3 + 2] = row.z as f32;
            }
            out
        };
        // Cycles stores 2π - r and then places the sun at
        // (-cos e sin(2π - r), cos e cos(2π - r), sin e), which is
        // (cos e sin r, cos e cos r, sin e).
        let sun_angles = |l: &super::EnvironmentLayer| {
            l.sky.as_ref().map_or((0.0, 0.0), |s| {
                let d = (sky_frame(l) * s.toward_sun).normalize_or_zero();
                let e = d.z.clamp(-1.0, 1.0).asin();
                let r = d.x.atan2(d.y);
                (e as f32, r as f32)
            })
        };
        // Image paths and layer arrays live until the render returns.
        let mut env_paths: Vec<CString> = Vec::new();
        for l in &s.world {
            for t in &l.layers {
                if let super::EnvironmentTextureSource::Image(path) = &t.source {
                    env_paths.push(CString::new(path.as_str()).map_err(|e| e.to_string())?);
                }
            }
        }
        let mut path_at = env_paths.iter();
        let to_cycles = super::image_frame_to_cycles();
        let probe_to_cycles = super::probe_frame_to_cycles();
        let env_layers: Vec<Vec<OcEnvLayer>> = s
            .world
            .iter()
            .map(|l| {
                l.layers
                    .iter()
                    .map(|t| {
                        let m = if t.light_probe {
                            probe_to_cycles
                        } else {
                            to_cycles
                        } * t.to_image;
                        let rows = [m.row(0), m.row(1), m.row(2)];
                        let mut to_image = [0.0f32; 9];
                        for (r, row) in rows.iter().enumerate() {
                            to_image[r * 3] = row.x as f32;
                            to_image[r * 3 + 1] = row.y as f32;
                            to_image[r * 3 + 2] = row.z as f32;
                        }
                        let (kind, color, image) = match &t.source {
                            super::EnvironmentTextureSource::Constant(c) => {
                                (0, *c, std::ptr::null())
                            }
                            super::EnvironmentTextureSource::Image(_) => (
                                if t.light_probe { 2 } else { 1 },
                                [0.0; 3],
                                path_at.next().map_or(std::ptr::null(), |p| p.as_ptr()),
                            ),
                        };
                        OcEnvLayer {
                            kind,
                            color,
                            opacity: t.opacity,
                            mix: t.mix,
                            invert: t.invert as u32,
                            image,
                            to_image,
                        }
                    })
                    .collect()
            })
            .collect();
        let environments: Vec<OcEnvironment> = s
            .world
            .iter()
            .zip(&env_layers)
            .map(|(l, layers)| {
                use operandi_core::environment::EnvironmentType as T;
                let m = &l.material;
                let c = |c: [f64; 3]| c.map(|v| v as f32);
                OcEnvironment {
                    // OC_ENV_*; Physically-based Daylight draws as its 4
                    // Color Gradient until the sky texture (stage 5).
                    kind: match m.kind {
                        T::Constant => 0,
                        T::TwoColor => 1,
                        T::FourColor | T::Physical => 2,
                        T::Overcast => 3,
                    },
                    zenith: c(m.zenith),
                    sky: c(m.sky),
                    ground: c(m.ground),
                    nadir: c(m.nadir),
                    sky_exponent: m.sky_exponent as f32,
                    ground_exponent: m.ground_exponent as f32,
                    intensity: l.intensity,
                    up: v(l.up),
                    visible: l.visible.map(u32::from),
                    material_opacity: l.material_opacity,
                    material_invert: l.material_invert as u32,
                    layers: layers.as_ptr(),
                    layer_count: layers.len() as u32,
                    physical: l.sky.is_some() as u32,
                    to_sky: rows(sky_frame(l)),
                    sun_elevation: sun_angles(l).0,
                    sun_rotation: sun_angles(l).1,
                    sun_size: l.sky.as_ref().map_or(0.0, |s| s.sun_size),
                    ground_albedo: l.sky.as_ref().map_or(0.0, |s| s.ground_albedo),
                    aerosol: l.sky.as_ref().map_or(0.0, |s| s.aerosol),
                }
            })
            .collect();
        let settings = OcSettings {
            width: s.width,
            height: s.height,
            samples: s.samples.max(1),
            camera_right: v(s.view.right),
            camera_up: v(s.view.up),
            camera_forward: v(s.view.forward),
            camera_position: v(s.view.position),
            fov: s.cycles_fov() as f32,
            sun_direction: v(s.sun_direction),
            sun_strength: s.sun_strength,
            environments: environments.as_ptr(),
            environment_count: environments.len() as u32,
            environment_fog_color: super::environment_fog_color(&s.world),
            base_color: s.base_color,
            roughness: s.roughness,
            use_gpu: s.gpu as u32,
            denoise: s.denoise as u32,
            resource_path: resources.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
            camera_orthographic: s.view.orthographic.is_some() as u32,
            ortho_half_short: ortho_half_short as f32,
            shift: s.view.shift.map(|v| v as f32),
            clip_near: s.view.clip.0 as f32,
            clip_far: s.view.clip.1 as f32,
            lights: lights.as_ptr(),
            light_count: lights.len() as u32,
            use_scene_lights: s.lights.is_some() as u32,
            region: s.region.map_or([0; 4], |r| [r.x, r.y, r.width, r.height]),
            max_diffuse_bounces: bounces[0],
            max_glossy_bounces: bounces[1],
            max_transmission_bounces: bounces[2],
            no_shadows: !s.light_paths.shadows as u32,
            adaptive_threshold: s.adaptive_threshold.max(0.0),
            threads: s.threads,
            transparent_film: s.transparent as u32,
            dicing_rate: if s.dicing_rate > 0.0 {
                s.dicing_rate
            } else {
                1.0
            },
            world_hidden_from_diffuse: !s.world_lights_diffuse as u32,
            dome: s.dome,
            light_groups: group_name_ptrs.as_ptr(),
            light_group_out: group_out_ptrs.as_ptr(),
            light_group_count: group_name_ptrs.len() as u32,
            panorama: match s.view.panorama {
                None => 0,
                Some(p) if p.cylindrical => 2,
                Some(_) => 1,
            },
            panorama_range: s
                .view
                .panorama
                .map_or([0.0; 4], |p| p.range(s.width, s.height).map(|v| v as f32)),
            aperture_size: s.view.dof.map_or(0.0, |d| d.aperture_radius as f32),
            focal_distance: s.view.dof.map_or(0.0, |d| d.focus_distance as f32),
            blades: s.view.dof.map_or(0, |d| d.blades),
            blades_rotation: s.view.dof.map_or(0.0, |d| d.blades_rotation as f32),
            passes: pass_codes.as_ptr(),
            pass_out: pass_out_ptrs.as_ptr(),
            pass_count: pass_codes.len() as u32,
            ao_distance: s.ao_distance,
            max_bounces: s.rays.max_bounces as i32,
            max_transparent_bounces: s.rays.depths[2] as i32,
            sample_clamp_indirect: s.rays.max_radiance,
            caustics_reflective: s.rays.caustics[0] as u32,
            caustics_refractive: s.rays.caustics[1] as u32,
            direct_caustics: s.rays.direct_caustics as u32,
            time_limit: s.time_limit.max(0.0),
            world_key: debug_key(&(&s.world, s.dome)),
            // The world's importance sampling (HDRI stage 5) is built
            // with the lights, so it is part of their key.
            lights_key: debug_key(&(
                &s.lights,
                s.sun_direction,
                s.sun_strength,
                s.light_paths.shadows,
                s.rays.direct_caustics,
                super::world_samples_image(&s.world),
            )),
            value_passes: value_codes.as_ptr(),
            value_out: value_out_ptrs.as_ptr(),
            value_count: value_codes.len() as u32,
            manifest_user: (&mut manifests as *mut Vec<(u32, String)>).cast(),
            on_cryptomatte_manifest: Some(on_manifest),
            world_lightgroup: world_group
                .as_ref()
                .map_or(std::ptr::null(), |c| c.as_ptr()),
            lamps_only: s.lamps_only as u32,
        };
        let mut report = OcReport {
            device: [0; 128],
            used_gpu: 0,
            denoised: 0,
            samples: 0,
        };
        let mut raw_control = OcControl {
            command: control.command.as_ptr(),
            user: control as *const RenderControl as *mut c_void,
            on_progress: Some(on_progress),
            on_preview: Some(on_preview),
        };
        let mut error = [0 as c_char; 512];
        // SAFETY: every pointer refers to memory owned by this function or
        // borrowed from `meshes` (or `resources`, `lights`, `control`) for the
        // whole call; lengths match the counts passed (checked in
        // `super::render_live`), `rgba` holds width*height*4 floats, and the
        // C side keeps no pointer after returning. `command` points at an
        // AtomicU32 (same layout as u32) that C only reads atomically;
        // callbacks only run during the call. It
        // catches all C++ exceptions.
        let status = unsafe {
            match session {
                // SAFETY (as above): the session pointer is live and used
                // by this call only (`&mut`).
                Some(session) => oc_session_render(
                    session.0,
                    raw.as_ptr(),
                    raw.len() as u32,
                    &settings,
                    rgba.as_mut_ptr(),
                    &mut raw_control,
                    &mut report,
                    error.as_mut_ptr(),
                    error.len(),
                ),
                None => oc_render(
                    raw.as_ptr(),
                    raw.len() as u32,
                    &settings,
                    rgba.as_mut_ptr(),
                    &mut raw_control,
                    &mut report,
                    error.as_mut_ptr(),
                    error.len(),
                ),
            }
        };
        if status != 0 {
            // SAFETY: the C side always NUL-terminates `error`.
            let message = unsafe { CStr::from_ptr(error.as_ptr()) };
            return Err(message.to_string_lossy().into_owned());
        }
        // SAFETY: `report.device` starts zeroed and the C side writes a
        // NUL-terminated string into it with snprintf.
        let device = unsafe { CStr::from_ptr(report.device.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        Ok(Image {
            width: s.width,
            height: s.height,
            rgba,
            device,
            used_gpu: report.used_gpu != 0,
            denoised: report.denoised != 0,
            samples: report.samples,
            light_groups: group_names
                .iter()
                .map(|c| c.to_string_lossy().into_owned())
                .zip(group_pixels)
                .collect(),
            passes: s.passes.iter().copied().zip(pass_pixels).collect(),
            values: s.values.iter().copied().zip(value_pixels).collect(),
            cryptomatte_manifests: manifests
                .into_iter()
                .filter_map(|(kind, json)| {
                    let kind = match kind {
                        0 => super::CryptomatteType::Object,
                        1 => super::CryptomatteType::Material,
                        2 => super::CryptomatteType::Asset,
                        _ => return None,
                    };
                    Some((kind, json))
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::mesh::Mesh;

    fn view() -> View {
        View::perspective(
            DVec3::new(0., 0., 5.),
            DVec3::X,
            DVec3::Y,
            -DVec3::Z,
            1.0 / 1.8,
        )
    }

    #[test]
    fn camera_dof_and_panoramas_reach_the_view() {
        let mut scene = camera_light_scene();
        let id = scene.render_camera().unwrap();
        fn cam(s: &mut Scene, id: u64) -> &mut operandi_core::camera_light::CameraSettings {
            s.items
                .iter_mut()
                .find(|i| i.id == id)
                .unwrap()
                .camera
                .as_mut()
                .unwrap()
        }
        let view = scene_camera_view(&scene, 16.0 / 9.0).unwrap();
        assert!(view.dof.is_none() && view.panorama.is_none());
        cam(&mut scene, id).dof.enable = true;
        cam(&mut scene, id).dof.iris_blades = 6;
        let dof = scene_camera_view(&scene, 16.0 / 9.0).unwrap().dof.unwrap();
        // 50 mm at f/4: 6.25 mm radius; the new scene focuses at 4 m.
        assert!((dof.aperture_radius - 0.00625).abs() < 1e-12);
        assert_eq!(dof.focus_distance, 4.0);
        assert_eq!(dof.blades, 6);
        cam(&mut scene, id).projection = Projection::SphericalVr;
        let p = scene_camera_view(&scene, 2.0).unwrap().panorama.unwrap();
        assert!(!p.cylindrical && p.vr);
        use std::f64::consts::{FRAC_PI_2, PI};
        assert_eq!(p.range(2000, 1000), [-PI, PI, -FRAC_PI_2, FRAC_PI_2]);
        // Not VR: the camera's back at the centre.
        let back = Panorama { vr: false, ..p };
        assert_eq!(back.range(2000, 1000)[..2], [-2.0 * PI, 0.0]);
        // Cylindrical: square pixels, 2π around over the width.
        let cyl = Panorama {
            cylindrical: true,
            vr: true,
        };
        let r = cyl.range(2000, 1000);
        assert!((r[3] - r[2] - 2.0 * PI * 0.5).abs() < 1e-12);
    }

    fn camera_light_scene() -> Scene {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        scene.add_default_camera_and_light().unwrap();
        scene
    }

    #[test]
    fn scene_camera_maps_film_and_fit() {
        use operandi_core::camera_light::{FilmFit, Projection};
        let mut scene = camera_light_scene();
        // No camera: no scene view (the viewport camera is used).
        assert!(scene_camera_view(&Scene::default(), 1.5).is_none());
        let view = scene_camera_view(&scene, 1.5).unwrap();
        // 36 mm film, 50 mm lens, Horizontal fit: half width tan 0.36.
        assert!((view.half_height_tan * 1.5 - 0.36).abs() < 1e-9);
        let settings = Settings::studio(720, 480, view);
        // Cycles' fov spans the shorter side (height): 2·atan(0.24).
        assert!((settings.cycles_fov() - 2.0 * 0.24f64.atan()).abs() < 1e-9);
        // The new-scene camera (spec v1) sits at (0, 0.75, 4).
        assert!((view.position - DVec3::new(0.0, 0.75, 4.0)).length() < 1e-12);
        assert!(view.orthographic.is_none());
        // Film Fit Fill at the 1920 × 1080 render frame: the 3:2 gate covers
        // the wider frame, so widths match (half width 0.36) and the gate
        // overhangs top and bottom.
        let [w, h] = operandi_core::camera_light_defaults::RENDER_FRAME;
        assert_eq!([w, h], [1920, 1080]);
        let aspect = f64::from(w) / f64::from(h);
        let view = scene_camera_view(&scene, aspect).unwrap();
        assert!((view.half_height_tan * aspect - 0.36).abs() < 1e-9);
        assert!(view.half_height_tan < 0.24);
        // A frame taller than the gate matches heights instead.
        let tall = scene_camera_view(&scene, 1.0).unwrap();
        assert!((tall.half_height_tan - 0.24).abs() < 1e-9);
        let camera_id = scene.render_camera().unwrap();
        let camera = scene
            .items
            .iter_mut()
            .find(|i| i.id == camera_id)
            .unwrap()
            .camera
            .as_mut()
            .unwrap();
        camera.film_fit = FilmFit::Vertical;
        camera.projection = Projection::Orthographic;
        camera.clipping = true;
        let view = scene_camera_view(&scene, 2.0).unwrap();
        assert!((view.half_height_tan - 0.24).abs() < 1e-9);
        // Orthographic: the frame at the 4 m Target Distance.
        assert!((view.orthographic.unwrap() - 0.96).abs() < 1e-9);
        assert_eq!(view.clip, (4.0, 1.0e5));
    }

    #[test]
    fn directional_lights_become_suns() {
        use operandi_core::camera_light::{LightRender, ShadowType};
        let mut scene = camera_light_scene();
        let id = scene.items.iter().find(|i| i.light.is_some()).unwrap().id;
        let suns = scene_lights(&scene);
        assert_eq!(suns.len(), 1);
        let sun = suns[0].clone();
        assert_eq!(sun.strength, [3.0, 3.0, 3.0]);
        assert_eq!(sun.shape, LightShape::Sun { angle: 0.0 });
        // Visible to Camera is off by default (spec v1).
        assert!(sun.cast_shadow && !sun.visible_to_camera);
        // Travels along the light's -Z: downward for the default light.
        assert!(sun.direction.y < 0.0);
        let item = || scene.items.iter().position(|i| i.id == id).unwrap();
        let at = item();
        {
            let light = scene.items[at].light.as_mut().unwrap();
            light.color = [1.0, 0.5, 0.0];
            light.dissolve = 0.5;
            light.spread_angle = 0.1;
            light.shadow = ShadowType::None;
        }
        let sun = scene_lights(&scene)[0].clone();
        assert_eq!(sun.strength, [1.5, 0.75, 0.0]);
        assert_eq!(sun.shape, LightShape::Sun { angle: 0.1 });
        assert!(!sun.cast_shadow);
        // Render Default follows visibility; Off never renders; On always.
        scene.items[at].visible = false;
        assert!(scene_lights(&scene).is_empty());
        scene.items[at].light.as_mut().unwrap().render = LightRender::On;
        assert_eq!(scene_lights(&scene).len(), 1);
        scene.items[at].visible = true;
        scene.items[at].light.as_mut().unwrap().render = LightRender::Off;
        assert!(scene_lights(&scene).is_empty());
        // A scene with light items uses them (even none rendering); one
        // without keeps the studio sun.
        let s = Settings::studio(32, 24, view()).with_scene_lights(&scene);
        assert_eq!(s.lights, Some(vec![]));
        let s = Settings::studio(32, 24, view()).with_scene_lights(&Scene::default());
        assert!(s.lights.is_none());
        // Render fix F3: final renders and the Preview add no light.
        let s = Settings::studio(32, 24, view()).with_only_scene_lights(&Scene::default());
        assert_eq!(s.lights, Some(vec![]));
    }

    /// A scene with one light of `kind` at `transform`.
    fn one_light(kind: LightKind, transform: operandi_core::scene::Transform) -> (Scene, u64) {
        let mut scene = Scene::default();
        let id = scene
            .add_light(LightSettings::new(kind), transform)
            .unwrap();
        (scene, id)
    }
    fn light_mut(scene: &mut Scene, id: u64) -> &mut LightSettings {
        scene
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .unwrap()
            .light
            .as_mut()
            .unwrap()
    }
    fn close(a: f32, b: f64) -> bool {
        (f64::from(a) - b).abs() < 1e-4 * b.abs().max(1.0)
    }

    const IES_FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/photometry/operandi-downlight.ies"
    );

    /// LP-CY: an open tube along local X, outward-facing, glowing L.
    #[test]
    fn cylinder_lights_become_glowing_tubes() {
        let (mut scene, id) = one_light(LightKind::Cylinder, Default::default());
        assert!(scene_lights(&scene).is_empty(), "not a Cycles lamp");
        let meshes = light_meshes(&scene, true).unwrap();
        assert_eq!(meshes.len(), 1);
        let m = &meshes[0];
        // Every point sits on the 0.2 m tube, within ±0.5 m along X.
        for p in &m.positions {
            assert!((f64::from(p[1]).hypot(f64::from(p[2])) - 0.2).abs() < 1e-6);
            assert!((f64::from(p[0].abs()) - 0.5).abs() < 1e-6);
        }
        // Outward: each triangle's normal points away from the axis.
        for t in &m.triangles {
            let v = |i: u32| DVec3::from(m.positions[i as usize].map(f64::from));
            let n = (v(t[1]) - v(t[0])).cross(v(t[2]) - v(t[0]));
            let c = (v(t[0]) + v(t[1]) + v(t[2])) / 3.0;
            assert!(n.dot(DVec3::new(0.0, c.y, c.z)) > 0.0);
        }
        let shader = m.shader.as_ref().unwrap();
        let strength = shader.ops[shader.inputs[Input::EmissionStrength as usize] as usize].value;
        assert!(close(strength[0], 3.0));
        assert!(!shader.visible_to_camera && !shader.cast_shadows);
        // Dissolve, Affect and Render rules as for any light.
        light_mut(&mut scene, id).visible_to_camera = true;
        assert!(
            light_meshes(&scene, true).unwrap()[0]
                .shader
                .as_ref()
                .unwrap()
                .visible_to_camera
        );
        light_mut(&mut scene, id).dissolve = 1.0;
        assert!(light_meshes(&scene, true).unwrap().is_empty());
        // The whole scene's meshes include it.
        light_mut(&mut scene, id).dissolve = 0.0;
        assert_eq!(scene_meshes(&scene).unwrap().len(), 1);
    }

    /// LP-MS: the prototype glows instead of rendering as itself; off Use
    /// Prototype Transform, it's placed by the light.
    #[test]
    fn mesh_lights_glow_with_their_prototype() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        let proto = scene.items[0].id;
        let id = scene
            .add_light(
                LightSettings::new(LightKind::Mesh),
                operandi_core::scene::Transform {
                    translation: DVec3::new(10.0, 0.0, 0.0),
                    ..Default::default()
                },
            )
            .unwrap();
        // No prototype: the box renders as itself, nothing glows.
        let plain = scene_meshes(&scene).unwrap();
        assert!(plain.iter().all(|m| m.shader.is_none()));
        light_mut(&mut scene, id).prototype = Some(proto);
        let meshes = scene_meshes(&scene).unwrap();
        assert!(!meshes.is_empty());
        assert!(
            meshes.iter().all(|m| m.shader.is_some()),
            "prototype hidden"
        );
        let centre = |ms: &[RenderMesh]| {
            let ps: Vec<_> = ms.iter().flat_map(|m| m.world_positions()).collect();
            ps.iter()
                .map(|p| DVec3::from(p.map(f64::from)))
                .sum::<DVec3>()
                / ps.len() as f64
        };
        assert!(centre(&meshes).length() < 1e-6, "where the mesh is");
        light_mut(&mut scene, id).use_prototype_transform = false;
        let moved = scene_meshes(&scene).unwrap();
        assert!((centre(&moved) - DVec3::new(10.0, 0.0, 0.0)).length() < 1e-6);
        // Hidden mesh light: the box is back.
        light_mut(&mut scene, id).render = operandi_core::camera_light::LightRender::Off;
        assert!(
            scene_meshes(&scene)
                .unwrap()
                .iter()
                .all(|m| m.shader.is_none())
        );
    }

    /// LP-PH: a photometric light carries its IES text; strength is
    /// Intensity over the file's own; no file = a point light.
    #[test]
    fn photometric_lights_carry_their_file() {
        let (mut scene, id) = one_light(LightKind::Photometric, Default::default());
        let plain = scene_lights(&scene)[0].clone();
        assert!(plain.ies.is_none());
        assert_eq!(plain.shape, LightShape::Point { radius: 0.0 });
        assert!(close(plain.strength[0], 3.0 * 4.0 * std::f64::consts::PI));
        let file =
            operandi_core::photometry::Photometry::load(std::path::Path::new(IES_FIXTURE)).unwrap();
        {
            let l = light_mut(&mut scene, id);
            l.photometry_file = IES_FIXTURE.into();
            l.intensity = 2.0 * file.radiant_intensity();
            l.width = 0.3;
            l.cone_angle = 1.0;
        }
        let lit = scene_lights(&scene)[0].clone();
        assert!(lit.ies.as_deref().unwrap().contains("TILT=NONE"));
        assert!(close(lit.strength[0], 2.0));
        assert!(
            matches!(lit.shape, LightShape::Spot { radius, .. } if (radius - 0.15).abs() < 1e-6)
        );
        assert!(light_notes(&scene).is_empty());
        light_mut(&mut scene, id).photometry_file = "/nonexistent.ies".into();
        assert!(scene_lights(&scene)[0].ies.is_none());
        assert_eq!(light_notes(&scene).len(), 1);
    }

    /// LP-SUN-06: Physical Sun sets the sun's strength; at night it's out.
    #[test]
    fn physical_sun_sets_strength() {
        let mut scene = camera_light_scene();
        let id = scene.items.iter().find(|i| i.light.is_some()).unwrap().id;
        light_mut(&mut scene, id).sun.enabled = true;
        let s = scene_lights(&scene)[0].strength[0];
        let want = light_mut(&mut scene, id).sun.strength(3.0);
        assert!(close(s, want) && s > 0.0 && s <= 3.0);
        light_mut(&mut scene, id).sun.time = 1.0;
        assert!(scene_lights(&scene).is_empty());
    }

    /// Cylinder, mesh and photometric lights light a plane in Cycles.
    #[cfg(feature = "cycles")]
    #[test]
    fn tier2_lights_light_the_plane() {
        let dark = plane_centre_value_with(None);
        for kind in [LightKind::Cylinder, LightKind::Photometric, LightKind::Mesh] {
            let lit = plane_centre_value_with(Some(kind));
            assert!(lit > dark + 0.02, "{kind:?}: {lit} vs unlit {dark}");
        }
    }

    /// Centre value of a grey plane lit only by one Tier 2 light 2 m above.
    #[cfg(feature = "cycles")]
    fn plane_centre_value_with(kind: Option<LightKind>) -> f32 {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::new(0.0, 0.0, -0.05), DVec3::new(40.0, 40.0, 0.1), 1)
                .unwrap()
                .into();
        let at = operandi_core::scene::Transform {
            translation: DVec3::new(0.0, 0.0, 2.0),
            ..Default::default()
        };
        if let Some(kind) = kind {
            let mut light = LightSettings::new(kind);
            if kind == LightKind::Photometric {
                light.photometry_file = IES_FIXTURE.into();
                light.intensity = 30.0;
            }
            if kind == LightKind::Mesh {
                let mut glow = Scene::default();
                glow.new_mesh().unwrap();
                let mut item = glow.items[0].clone();
                item.id = scene.next_id;
                scene.next_id += 1;
                item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(0.5), 1)
                    .unwrap()
                    .into();
                item.transform = at.clone();
                light.prototype = Some(item.id);
                scene.items.push(item);
            }
            scene.add_light(light, at).unwrap();
        }
        let eye = DVec3::new(1.5, 0.0, 1.0);
        let forward = (-eye).normalize();
        let right = forward.cross(DVec3::Y).normalize();
        let up = right.cross(forward);
        let mut settings =
            Settings::studio(17, 17, View::perspective(eye, right, up, forward, 0.05))
                .with_scene_lights(&scene);
        settings.lights.get_or_insert_with(Vec::new);
        settings.world[0].intensity = 0.0;
        settings.base_color = [0.6; 3];
        settings.roughness = 1.0;
        settings.samples = 64;
        settings.gpu = false;
        settings.denoise = false;
        let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
        let mut sum = 0.0;
        for y in 7..10 {
            for x in 7..10 {
                sum += image.rgba[(y * 17 + x) * 4 + 1];
            }
        }
        sum / 9.0
    }

    /// LP-DM-01: domes add even world light; Render, Dissolve, Enable and
    /// Opacity count; they aren't Cycles lamps.
    #[test]
    fn dome_lights_add_world_light() {
        let (mut scene, id) = one_light(LightKind::Dome, Default::default());
        assert!(scene_lights(&scene).is_empty());
        assert_eq!(scene_dome(&scene), [3.0; 3]);
        {
            let l = light_mut(&mut scene, id);
            l.color = [1.0, 0.5, 0.0];
            l.dissolve = 0.5;
            l.stored.opacity = 0.5;
        }
        assert_eq!(scene_dome(&scene), [0.75, 0.375, 0.0]);
        light_mut(&mut scene, id).stored.material_enabled = false;
        assert_eq!(scene_dome(&scene), [0.0; 3]);
        let s = Settings::studio(8, 8, view()).with_scene_lights(&scene);
        assert_eq!(s.dome, [0.0; 3]);
    }

    /// LM-01 / LM-04 scale a lamp; LP-22 names its light group.
    #[test]
    fn material_and_light_groups_reach_lamps() {
        let (mut scene, id) = one_light(LightKind::Point, Default::default());
        let full = scene_lights(&scene)[0].strength[0];
        light_mut(&mut scene, id).stored.opacity = 0.25;
        assert!(close(
            scene_lights(&scene)[0].strength[0],
            f64::from(full) * 0.25
        ));
        light_mut(&mut scene, id).stored.material_enabled = false;
        assert!(scene_lights(&scene).is_empty());
        light_mut(&mut scene, id).stored.material_enabled = true;
        light_mut(&mut scene, id).lpe_label = "Key".into();
        assert_eq!(scene_lights(&scene)[0].lightgroup.as_deref(), Some("Key"));
        let second = scene
            .add_light(LightSettings::new(LightKind::Cylinder), Default::default())
            .unwrap();
        light_mut(&mut scene, second).lpe_label = "Key".into();
        let third = scene
            .add_light(LightSettings::new(LightKind::Spot), Default::default())
            .unwrap();
        light_mut(&mut scene, third).lpe_label = "Rim".into();
        assert_eq!(scene_light_groups(&scene), ["Key", "Rim"]);
        // The cylinder's glowing tube counts toward its group too.
        let tube = &light_meshes(&scene, true).unwrap()[0];
        assert_eq!(
            tube.shader.as_ref().unwrap().lightgroup.as_deref(),
            Some("Key")
        );
        let s = Settings::studio(8, 8, view()).with_scene_lights(&scene);
        assert_eq!(s.light_groups, ["Key", "Rim"]);
    }

    /// Gobo layers go to spot lamps only, enabled ones with a picture.
    #[test]
    fn layers_project_through_spots_only() {
        use operandi_core::camera_light::{LightBlend, LightLayer};
        let layer = |image: &str, enabled| LightLayer {
            image: image.into(),
            enabled,
            blend: LightBlend::Screen,
            opacity: 0.5,
        };
        let (mut scene, id) = one_light(LightKind::Spot, Default::default());
        light_mut(&mut scene, id).layers = vec![
            layer("/a.png", true),
            layer("/b.png", false),
            layer("", true),
        ];
        let l = scene_lights(&scene)[0].clone();
        assert_eq!(l.layers, [("/a.png".to_string(), 4, 0.5)]);
        let half = (LightSettings::new(LightKind::Spot).cone_angle / 2.0).tan();
        assert!(close(l.gobo_tan_half, half));
        let (mut point, pid) = one_light(LightKind::Point, Default::default());
        light_mut(&mut point, pid).layers = vec![layer("/a.png", true)];
        assert!(scene_lights(&point)[0].layers.is_empty());
    }

    /// LP-40..44: closed beam volumes of the right size per kind.
    #[test]
    fn volumetric_beams_by_kind() {
        use operandi_core::camera_light::Volumetrics;
        let beam = |kind: LightKind| {
            let (mut scene, id) = one_light(kind, Default::default());
            assert!(light_volumes(&scene).unwrap().is_empty(), "off by default");
            let mut v = Volumetrics::new(kind);
            v.enabled = true;
            v.height = 4.0;
            light_mut(&mut scene, id).volumetrics = Some(v);
            let meshes = light_volumes(&scene).unwrap();
            assert_eq!(meshes.len(), 1);
            let m = meshes[0].clone();
            // Closed: every edge is shared by exactly two triangles.
            let mut edges = std::collections::HashMap::new();
            for t in &m.triangles {
                for k in 0..3 {
                    let (a, b) = (t[k].min(t[(k + 1) % 3]), t[k].max(t[(k + 1) % 3]));
                    *edges.entry((a, b)).or_insert(0) += 1;
                }
            }
            assert!(edges.values().all(|n| *n == 2), "{kind:?} not closed");
            let shader = m.shader.as_ref().unwrap();
            let b = shader.volume.unwrap();
            assert_eq!((b.density, b.attenuation), (0.5, 0.1));
            m
        };
        let span = |m: &RenderMesh, axis: usize| {
            let v: Vec<f32> = m.positions.iter().map(|p| p[axis]).collect();
            (
                v.iter().cloned().fold(f32::MAX, f32::min),
                v.iter().cloned().fold(f32::MIN, f32::max),
            )
        };
        // Directional: 0.5 m radius, 4 m down −Z.
        let d = beam(LightKind::Directional);
        assert!(close(span(&d, 2).0, -4.0) && close(span(&d, 2).1, 0.0));
        assert!(close(span(&d, 0).1, 0.5));
        // Spot: widens by the 45° cone over 4 m from a point base.
        let s = beam(LightKind::Spot);
        let r = 4.0 * (22.5f64).to_radians().tan();
        assert!(close(span(&s, 0).1, r));
        // Point: a 1 m sphere.
        let p = beam(LightKind::Point);
        assert!(close(span(&p, 1).1, 1.0) && close(span(&p, 1).0, -1.0));
        beam(LightKind::Photometric);
        // A volume needs density.
        let (mut scene, id) = one_light(LightKind::Point, Default::default());
        let mut v = Volumetrics::new(LightKind::Point);
        v.enabled = true;
        v.density = 0.0;
        light_mut(&mut scene, id).volumetrics = Some(v);
        assert!(light_volumes(&scene).unwrap().is_empty());
    }

    /// Renders: a dome lights the plane without showing in the background;
    /// a light group's pass holds its light; a beam glows in the air; a
    /// black gobo darkens a spot.
    #[cfg(feature = "cycles")]
    #[test]
    fn tier3_renders() {
        use operandi_core::camera_light::{LightBlend, LightLayer, Volumetrics};
        let size = 17;
        let render_scene = |scene: &Scene, world: bool| {
            let eye = DVec3::new(1.5, 0.0, 1.0);
            let forward = (-eye).normalize();
            let right = forward.cross(DVec3::Y).normalize();
            let up = right.cross(forward);
            let mut settings =
                Settings::studio(size, size, View::perspective(eye, right, up, forward, 0.05))
                    .with_scene_lights(scene);
            settings.lights.get_or_insert_with(Vec::new);
            settings.world[0].intensity = if world { 1.0 } else { 0.0 };
            settings.base_color = [0.6; 3];
            settings.roughness = 1.0;
            settings.samples = 32;
            settings.gpu = false;
            settings.denoise = false;
            render(&scene_meshes(scene).unwrap(), &settings).unwrap()
        };
        let plane = || {
            let mut scene = Scene::default();
            scene.new_mesh().unwrap();
            scene.active_mut().unwrap().mesh =
                Mesh::box_mesh(DVec3::new(0.0, 0.0, -0.05), DVec3::new(2.0, 2.0, 0.1), 1)
                    .unwrap()
                    .into();
            scene
        };
        let mean =
            |img: &Image| img.rgba.chunks(4).map(|p| p[1]).sum::<f32>() / (size * size) as f32;
        // Dome.
        let dark = render_scene(&plane(), false);
        let mut domed = plane();
        domed
            .add_light(LightSettings::new(LightKind::Dome), Default::default())
            .unwrap();
        let lit = render_scene(&domed, false);
        assert!(mean(&lit) > mean(&dark) + 0.05, "dome lights the plane");
        // Looking away from the plane, into the sky: black with a dome.
        let sky = {
            let mut settings = Settings::studio(
                8,
                8,
                View::perspective(DVec3::new(0.0, 0.0, 5.0), DVec3::X, DVec3::Y, DVec3::Z, 0.3),
            )
            .with_scene_lights(&domed);
            settings.world[0].intensity = 0.0;
            settings.samples = 8;
            settings.gpu = false;
            settings.denoise = false;
            render(&scene_meshes(&domed).unwrap(), &settings).unwrap()
        };
        assert!(
            sky.rgba
                .chunks(4)
                .all(|p| p[0] < 1e-3 && p[1] < 1e-3 && p[2] < 1e-3),
            "dome seen as background"
        );
        // Light group pass: one labelled spot, so its pass ≈ the picture.
        let mut spot_scene = plane();
        let mut spot = LightSettings::new(LightKind::Spot);
        spot.lpe_label = "Key".into();
        spot.cone_angle = 1.2;
        let at = operandi_core::scene::Transform {
            translation: DVec3::new(0.0, 0.0, 2.0),
            ..Default::default()
        };
        let sid = spot_scene.add_light(spot, at.clone()).unwrap();
        let img = render_scene(&spot_scene, false);
        assert_eq!(img.light_groups.len(), 1);
        let (name, pass) = &img.light_groups[0];
        assert_eq!(name, "Key");
        let pass_mean = pass.chunks(4).map(|p| p[1]).sum::<f32>() / (size * size) as f32;
        assert!(
            pass_mean > 0.01 && (pass_mean - mean(&img)).abs() < 0.2 * mean(&img),
            "{pass_mean} vs {}",
            mean(&img)
        );
        // Gobo: a black picture multiplied over the spot darkens it.
        let dir = std::env::temp_dir().join(format!("operandi-gobo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let black = dir.join("black.png");
        image::save_buffer(
            &black,
            &[0u8; 4 * 4 * 3],
            4,
            4,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
        light_mut(&mut spot_scene, sid).layers = vec![LightLayer {
            image: black.to_string_lossy().into_owned(),
            enabled: true,
            blend: LightBlend::Multiply,
            opacity: 1.0,
        }];
        let gobo = render_scene(&spot_scene, false);
        assert!(
            mean(&gobo) < 0.3 * mean(&img),
            "gobo {} vs {}",
            mean(&gobo),
            mean(&img)
        );
        let _ = std::fs::remove_dir_all(&dir);
        // Volumetric beam: a point light's glowing ball of air, seen from
        // the camera, brightens the picture.
        let mut air = plane();
        let pid = air
            .add_light(LightSettings::new(LightKind::Point), at)
            .unwrap();
        let clear = render_scene(&air, false);
        let mut v = Volumetrics::new(LightKind::Point);
        v.enabled = true;
        v.density = 2.0;
        light_mut(&mut air, pid).volumetrics = Some(v);
        let foggy = render_scene(&air, false);
        assert!(
            (mean(&foggy) - mean(&clear)).abs() > 0.005,
            "beam {} vs {}",
            mean(&foggy),
            mean(&clear)
        );
    }

    /// Two boxes (left and right) under one point light, its linking set to
    /// `link` with a group holding the left box.
    fn linked_scene(
        link: Option<(operandi_core::camera_light::LinkMode, bool)>,
    ) -> (Scene, u64, u64) {
        use operandi_core::camera_light::LightLinking;
        let mut scene = Scene::default();
        let mut boxes = Vec::new();
        for x in [-0.75, 0.75] {
            scene.new_mesh().unwrap();
            let item = scene.items.last_mut().unwrap();
            item.mesh = Mesh::box_mesh(DVec3::new(x, 0.0, 0.0), DVec3::new(1.0, 1.0, 1.0), 1)
                .unwrap()
                .into();
            boxes.push(item.id);
        }
        let group = scene.new_group(Some("Left"), &[boxes[0]]).unwrap();
        let mut light = LightSettings::new(LightKind::Point);
        light.intensity = 10.0;
        if let Some((mode, with_group)) = link {
            light.linking = Some(LightLinking {
                control: true,
                group: with_group.then_some(group),
                mode,
            });
        }
        scene
            .add_light(
                light,
                operandi_core::scene::Transform {
                    translation: DVec3::new(0.0, 0.0, 3.0),
                    ..Default::default()
                },
            )
            .unwrap();
        (scene, boxes[0], boxes[1])
    }

    #[test]
    fn link_sets_give_receivers_and_memberships() {
        use operandi_core::camera_light::LinkMode;
        let (scene, left, right) = linked_scene(None);
        assert_eq!(link_sets(&scene), LinkSets::default(), "no rules, no sets");
        let (scene, left2, right2) = linked_scene(Some((LinkMode::Include, true)));
        let sets = link_sets(&scene);
        let light = scene.items.iter().find(|i| i.light.is_some()).unwrap().id;
        let (l, r) = (sets.receiver[&left2], sets.receiver[&right2]);
        assert_eq!(l, 0, "lit by every light");
        assert_ne!(r, 0);
        let m = sets.membership[&light];
        assert!(m & (1 << l) != 0 && m & (1 << r) == 0);
        // Meshes carry their set; the lamp its membership.
        let meshes = scene_meshes(&scene).unwrap();
        assert!(meshes.iter().any(|x| x.light_set == r));
        assert_eq!(scene_lights(&scene)[0].light_membership, Some(m));
        let _ = (left, right);
        // Include with no group lights nothing: no set has its bit.
        let (scene, ..) = linked_scene(Some((LinkMode::Include, false)));
        let sets = link_sets(&scene);
        let light = scene.items.iter().find(|i| i.light.is_some()).unwrap().id;
        let m = sets.membership[&light];
        assert!(
            scene
                .items
                .iter()
                .all(|i| m & (1 << sets.receiver[&i.id]) == 0)
        );
        // Exclude with no group lights everything.
        let (scene, ..) = linked_scene(Some((LinkMode::Exclude, false)));
        let sets = link_sets(&scene);
        let light = scene.items.iter().find(|i| i.light.is_some()).unwrap().id;
        let m = sets.membership[&light];
        assert!(
            scene
                .items
                .iter()
                .all(|i| m & (1 << sets.receiver[&i.id]) != 0)
        );
    }

    /// LP-30..32 in Cycles: Include lights only the left box, Exclude only
    /// the right one.
    #[cfg(feature = "cycles")]
    #[test]
    fn light_linking_renders() {
        use operandi_core::camera_light::LinkMode;
        let halves = |link| {
            let (scene, ..) = linked_scene(link);
            let eye = DVec3::new(0.0, 0.0, 8.0);
            let mut settings = Settings::studio(
                32,
                16,
                View::perspective(eye, DVec3::X, DVec3::Y, -DVec3::Z, 0.25),
            )
            .with_scene_lights(&scene);
            settings.world[0].intensity = 0.0;
            settings.base_color = [0.6; 3];
            settings.roughness = 1.0;
            settings.samples = 16;
            settings.gpu = false;
            settings.denoise = false;
            let img = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
            let side = |x0: usize, x1: usize| {
                let mut sum = 0.0;
                for y in 4..12 {
                    for x in x0..x1 {
                        sum += img.rgba[(y * 32 + x) * 4 + 1];
                    }
                }
                sum / ((x1 - x0) * 8) as f32
            };
            (side(4, 12), side(20, 28))
        };
        let (l, r) = halves(None);
        assert!(l > 0.005 && r > 0.005, "both lit: {l} {r}");
        let (l, r) = halves(Some((LinkMode::Include, true)));
        assert!(l > 0.005 && r < 0.2 * l, "include: {l} {r}");
        let (l, r) = halves(Some((LinkMode::Exclude, true)));
        assert!(r > 0.005 && l < 0.2 * r, "exclude: {l} {r}");
    }

    /// LP-11: strength × (1 − Dissolve); 100 % leaves the light out.
    #[test]
    fn dissolve_scales_strength() {
        let (mut scene, id) = one_light(LightKind::Point, Default::default());
        let full = scene_lights(&scene)[0].strength[0];
        light_mut(&mut scene, id).dissolve = 0.25;
        assert!(close(
            scene_lights(&scene)[0].strength[0],
            f64::from(full) * 0.75
        ));
        light_mut(&mut scene, id).dissolve = 1.0;
        assert!(scene_lights(&scene).is_empty());
    }

    /// LP-19..21 and LM-12/13 reach the Cycles light; a partial Affect
    /// counts as on and is noted (LPQ-07).
    #[test]
    fn visibility_rows_and_affect_switches_map_to_cycles() {
        let (mut scene, id) = one_light(LightKind::Area, Default::default());
        let l = scene_lights(&scene)[0].clone();
        assert!(!l.visible_to_camera && !l.visible_to_reflection && !l.visible_to_refraction);
        assert!(l.affects_diffuse && l.affects_specular);
        {
            let light = light_mut(&mut scene, id);
            light.visible_to_camera = true;
            light.stored.visible_to_reflection = true;
            light.affect_specular = 0.0;
        }
        let l = scene_lights(&scene)[0].clone();
        assert!(l.visible_to_camera && l.visible_to_reflection && !l.visible_to_refraction);
        assert!(l.affects_diffuse && !l.affects_specular);
        assert!(light_notes(&scene).is_empty());
        light_mut(&mut scene, id).affect_diffuse = 0.4;
        assert!(scene_lights(&scene)[0].affects_diffuse);
        assert_eq!(light_notes(&scene).len(), 1);
        // Neither switch on: the light is left out.
        light_mut(&mut scene, id).affect_diffuse = 0.0;
        assert!(scene_lights(&scene).is_empty());
    }

    /// LP-PO: a portal becomes a Cycles portal sized like an area light
    /// (× item scale), whatever its colour, Dissolve or Affect rows say.
    #[test]
    fn portals_become_cycles_portals() {
        let t = operandi_core::scene::Transform {
            scale: DVec3::new(2.0, 3.0, 1.0),
            ..Default::default()
        };
        let (mut scene, id) = one_light(LightKind::Portal, t);
        {
            let p = light_mut(&mut scene, id);
            p.dissolve = 1.0;
            p.affect_diffuse = 0.0;
            p.affect_specular = 0.0;
            p.falloff = LightFalloff::None;
            p.shape = AreaShape::Elliptical;
        }
        let lights = scene_lights(&scene);
        assert_eq!(lights.len(), 1);
        assert_eq!(
            lights[0].shape,
            LightShape::Portal {
                size: [2.0, 3.0],
                ellipse: true
            }
        );
        assert!(!lights[0].cast_shadow && !lights[0].visible_to_camera);
        // No notes about a portal's unused rows.
        assert!(light_notes(&scene).is_empty());
        // Render No leaves it out like any light.
        light_mut(&mut scene, id).render = operandi_core::camera_light::LightRender::Off;
        assert!(scene_lights(&scene).is_empty());
        assert_eq!(
            light_power(&LightSettings::new(LightKind::Portal), 1.0),
            0.0
        );
    }

    /// A portal over a world-lit plane adds no light of its own: the
    /// render matches the one without it, within sampling noise.
    #[cfg(feature = "cycles")]
    #[test]
    fn portal_renders_without_adding_light() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::new(0.0, 0.0, -0.05), DVec3::new(40.0, 40.0, 0.1), 1)
                .unwrap()
                .into();
        let eye = DVec3::new(1.5, 0.0, 1.0);
        let forward = (-eye).normalize();
        let right = forward.cross(DVec3::Y).normalize();
        let up = right.cross(forward);
        let mut settings =
            Settings::studio(17, 17, View::perspective(eye, right, up, forward, 0.05));
        settings.world[0].intensity = 1.0;
        settings.samples = 128;
        settings.gpu = false;
        settings.denoise = false;
        settings.lights = Some(Vec::new());
        let meshes = scene_meshes(&scene).unwrap();
        let mean = |settings: &Settings| {
            let image = render(&meshes, settings).unwrap();
            image.rgba.chunks(4).map(|p| p[1]).sum::<f32>() / (17.0 * 17.0)
        };
        let without = mean(&settings);
        scene
            .add_light(
                LightSettings::new(LightKind::Portal),
                operandi_core::scene::Transform {
                    translation: DVec3::new(0.0, 0.0, 1.0),
                    // Facing down into the "room" (its −Z).
                    ..Default::default()
                },
            )
            .unwrap();
        settings.lights = Some(scene_lights(&scene));
        assert!(matches!(
            settings.lights.as_ref().unwrap()[0].shape,
            LightShape::Portal { .. }
        ));
        let with = mean(&settings);
        assert!(with.is_finite() && without > 0.0);
        assert!(
            (with - without).abs() < 0.2 * without,
            "with portal {with}, without {without}"
        );
    }

    #[test]
    fn point_light_power_is_intensity_times_four_pi() {
        let t = operandi_core::scene::Transform {
            translation: DVec3::new(1.0, 2.0, 3.0),
            ..Default::default()
        };
        let (mut scene, id) = one_light(LightKind::Point, t);
        light_mut(&mut scene, id).radius = 0.25;
        let l = scene_lights(&scene)[0].clone();
        assert_eq!(l.shape, LightShape::Point { radius: 0.25 });
        assert_eq!(l.position, DVec3::new(1.0, 2.0, 3.0));
        let w = 3.0 * 4.0 * std::f64::consts::PI;
        assert!(l.strength.iter().all(|&c| close(c, w)), "{:?}", l.strength);
        // Inverse-square is the only falloff Cycles has.
        assert!(light_notes(&scene).is_empty());
        light_mut(&mut scene, id).falloff = LightFalloff::InverseDistance;
        assert_eq!(light_notes(&scene).len(), 1);
    }

    #[test]
    fn spot_cone_and_soft_edge_map_to_size_and_blend() {
        let (mut scene, id) = one_light(LightKind::Spot, Default::default());
        let l = scene_lights(&scene)[0].clone();
        // 45° full cone, 5° soft edge: blend = (cos 17.5° − cos 22.5°) /
        // (1 − cos 22.5°) (spec v2).
        let LightShape::Spot {
            radius,
            size,
            blend,
        } = l.shape
        else {
            panic!("{:?}", l.shape)
        };
        assert_eq!(radius, 0.0);
        let (h, e) = (22.5f64.to_radians(), 5f64.to_radians());
        let want = ((h - e).cos() - h.cos()) / (1.0 - h.cos());
        assert!(close(size, 45f64.to_radians()) && close(blend, want));
        assert!(
            l.strength
                .iter()
                .all(|&c| close(c, 12.0 * std::f64::consts::PI))
        );
        // Shines down local -Z.
        assert_eq!(l.direction, -DVec3::Z);
        // An edge wider than half the cone is all soft.
        light_mut(&mut scene, id).soft_edge = 1.0;
        let LightShape::Spot { blend, .. } = scene_lights(&scene)[0].shape else {
            panic!()
        };
        assert_eq!(blend, 1.0);
        let l = light_mut(&mut scene, id);
        assert!((l.spot_full_angle() - 0.0).abs() < 1e-12);
    }

    #[test]
    fn area_light_size_follows_item_scale_and_power_its_area() {
        use operandi_core::camera_light::AreaShape;
        let t = operandi_core::scene::Transform {
            scale: DVec3::new(2.0, 3.0, 1.0),
            ..Default::default()
        };
        let (mut scene, id) = one_light(LightKind::Area, t);
        let l = scene_lights(&scene)[0].clone();
        assert_eq!(
            l.shape,
            LightShape::Area {
                size: [2.0, 3.0],
                ellipse: false
            }
        );
        // Radiance 3 × 6 m² × π.
        assert!(
            l.strength
                .iter()
                .all(|&c| close(c, 18.0 * std::f64::consts::PI))
        );
        light_mut(&mut scene, id).shape = AreaShape::Elliptical;
        let l = scene_lights(&scene)[0].clone();
        assert!(matches!(l.shape, LightShape::Area { ellipse: true, .. }));
        assert!(
            l.strength
                .iter()
                .all(|&c| close(c, 18.0 * std::f64::consts::FRAC_PI_4 * std::f64::consts::PI))
        );
        // Partial Affect Specular is noted, 0 % turns specular off.
        light_mut(&mut scene, id).affect_specular = 0.5;
        assert_eq!(light_notes(&scene).len(), 1);
        light_mut(&mut scene, id).affect_specular = 0.0;
        assert!(!scene_lights(&scene)[0].affects_specular);
        assert!(light_notes(&scene).is_empty());
    }

    /// Regression (patchy renders, 2026-11-05): a smooth sphere keeps one
    /// render point per mesh point; corner normals that differ only by
    /// rounding used to split every point and shade each polygon apart.
    #[test]
    fn smooth_sphere_points_stay_shared() {
        use operandi_core::{Command, Editor, RadialParameters, Tool};
        let mut e = Editor::default();
        e.scene.new_mesh().unwrap();
        e.dispatch(Command::Activate(Tool::Sphere)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.radial = Some(RadialParameters {
            radius: DVec3::splat(0.5),
            sides: 24,
            segments: 12,
        });
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Apply).unwrap();
        e.dispatch(Command::Drop).unwrap();
        let points = e.scene.items[0].mesh.vertices.len();
        let m = &scene_meshes(&e.scene).unwrap()[0];
        assert_eq!(m.positions.len(), points);
    }

    #[test]
    fn cameras_and_lights_are_not_render_meshes() {
        let scene = camera_light_scene();
        assert_eq!(scene_meshes(&scene).unwrap().len(), 1);
    }

    #[test]
    fn box_becomes_twelve_triangles_placed_by_its_transform() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let item = scene.active_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        item.transform.translation = DVec3::new(10., 0., 0.);
        let meshes = scene_meshes(&scene).unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].triangles.len(), 12);
        // The points stay in the item's space; the transform moves them.
        assert!(meshes[0].positions.iter().all(|p| p[0].abs() <= 0.5));
        assert!(
            meshes[0]
                .world_positions()
                .iter()
                .all(|p| p[0] >= 9.5 && p[0] <= 10.5)
        );
    }

    #[test]
    fn creases_come_from_corner_normals_on_shared_points() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        // 40°: every cube corner is a crease on all three sides, so each
        // corner's normal is its side's; the 8 points stay shared.
        let m = &scene_meshes(&scene).unwrap()[0];
        assert_eq!(m.positions.len(), 8);
        assert_eq!(m.normals.len(), m.triangles.len() * 3);
        assert!(m.smooth.iter().all(|&s| s == 1));
        let axis_aligned = |n: &[f32; 3]| n.iter().filter(|c| c.abs() > 0.999).count() == 1;
        assert!(m.normals.iter().all(axis_aligned));
        // 100°: the sides blend, each corner leans out along the diagonal.
        scene.smoothing.angle = 100.;
        let m = &scene_meshes(&scene).unwrap()[0];
        assert_eq!(m.positions.len(), 8);
        assert!(
            m.normals
                .iter()
                .all(|n| n.iter().all(|c| (c.abs() - 1.0 / 3f32.sqrt()).abs() < 1e-4))
        );
        // 0 %: flat triangles.
        scene.smoothing.amount = 0.;
        assert!(
            scene_meshes(&scene).unwrap()[0]
                .smooth
                .iter()
                .all(|&s| s == 0)
        );
    }

    #[test]
    fn each_material_becomes_its_own_mesh() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let item = scene.active_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        let mesh = &mut item.mesh;
        mesh.faces[0].material = "Red".into();
        mesh.faces[1].material = "Unstored".into();
        // No stored materials: one default mesh.
        let meshes = scene_meshes(&scene).unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].shader, None);
        let red = operandi_core::Material {
            color: [1.0, 0.0, 0.0],
            diffuse: 0.5,
            smoothing: false,
            ..Default::default()
        };
        scene.materials.insert("Red".into(), red);
        let meshes = scene_meshes(&scene).unwrap();
        assert_eq!(meshes.len(), 2);
        let red_mesh = meshes.iter().find(|m| m.shader.is_some()).unwrap();
        assert_eq!(red_mesh.triangles.len(), 2);
        assert!(red_mesh.smooth.iter().all(|&s| s == 0));
        let sh = red_mesh.shader.as_ref().unwrap();
        // Base Color is the Diffuse Color; Diffuse Amount (50 %) is ignored
        // by the Principled model.
        assert_eq!(sh.constant(Input::BaseColor), Some([1.0, 0.0, 0.0]));
        // Specular 50 % is the usual IOR 1.5 (F0 0.04).
        assert!((sh.constant(Input::Ior).unwrap()[0] - 1.5).abs() < 1e-4);
        let other = meshes.iter().find(|m| m.shader.is_none()).unwrap();
        assert_eq!(other.triangles.len(), 10);
    }

    /// The shader tree drives renders: a Constant texture over the Base
    /// Material, a masked group, and shader visibility (ST-4).
    #[test]
    fn shader_tree_layers_reach_the_render() {
        use operandi_core::shader_tree::{NewLayer, NodeKind, TagMask, TextureSource};
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let item = scene.active_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        item.mesh.faces[0].material = "Glow".into();
        let tree = &mut scene.shader_tree;
        // A group for "Glow" holding a blue Constant at 50 %, and a shader
        // that hides it from the camera.
        let group = tree.add(NewLayer::Group, Some(4)).unwrap();
        let mut g = tree.get(group).unwrap().clone();
        if let NodeKind::Group(m) = &mut g.kind {
            m.tag = TagMask::Name("Glow".into());
        }
        tree.replace(g).unwrap();
        let tex = tree.add(NewLayer::Constant, Some(group)).unwrap();
        let mut t = tree.get(tex).unwrap().clone();
        t.layer.opacity = 0.5;
        if let NodeKind::Texture(x) = &mut t.kind {
            x.source = TextureSource::Constant {
                color: [0., 0., 1.],
                value: 0.,
            };
        }
        tree.replace(t).unwrap();
        let shader = tree.add(NewLayer::Shader, Some(group)).unwrap();
        let mut sh = tree.get(shader).unwrap().clone();
        if let NodeKind::Shader(x) = &mut sh.kind {
            x.visible_to_camera = false;
        }
        tree.replace(sh).unwrap();
        // The group sits below the Base Shader: its shader loses.
        let meshes = scene_meshes(&scene).unwrap();
        assert_eq!(meshes.len(), 2);
        let glow = meshes.iter().find_map(|m| m.shader.as_ref()).unwrap();
        // Base 0.6 grey (Diffuse Amount ignored); colour halfway to blue.
        let base = glow.constant(Input::BaseColor).unwrap();
        assert!(
            (base[0] - 0.3).abs() < 1e-5 && (base[2] - 0.8).abs() < 1e-5,
            "{base:?}"
        );
        assert!(glow.visible_to_camera);
        // On top of Render, the group's shader wins.
        tree_to_top(&mut scene, group);
        let meshes = scene_meshes(&scene).unwrap();
        let glow = meshes.iter().find_map(|m| m.shader.as_ref()).unwrap();
        assert!(!glow.visible_to_camera);
    }

    fn tree_to_top(scene: &mut Scene, id: operandi_core::shader_tree::NodeId) {
        use operandi_core::shader_tree::Drop;
        scene
            .shader_tree
            .move_to(
                &[id],
                Drop {
                    parent: None,
                    index: 0,
                },
            )
            .unwrap();
    }

    #[test]
    fn image_maps_carry_uvs_and_a_node_program() {
        use operandi_core::shader_tree::{NewLayer, NodeKind, TextureSource};
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        scene.images.push(operandi_core::backdrop::SceneImage {
            id: 7,
            name: "grid".into(),
            path: "/tmp/grid.png".into(),
            width: 4,
            height: 4,
        });
        let tex = scene.shader_tree.add(NewLayer::ImageMap, Some(4)).unwrap();
        let mut t = scene.shader_tree.get(tex).unwrap().clone();
        if let NodeKind::Texture(x) = &mut t.kind
            && let TextureSource::ImageMap { image, .. } = &mut x.source
        {
            *image = Some(7);
        }
        scene.shader_tree.replace(t).unwrap();
        let meshes = scene_meshes(&scene).unwrap();
        assert_eq!(meshes.len(), 1);
        let m = &meshes[0];
        let sh = m.shader.as_ref().unwrap();
        assert_eq!(sh.images, vec!["/tmp/grid.png".to_string()]);
        assert!(sh.is_valid());
        assert_eq!(sh.constant(Input::BaseColor), None);
        assert_eq!(m.uvs.len(), m.triangles.len() * 3);
    }

    /// A 4 m plane at z = 0 facing +Z, with UVs, whose Base Material glows
    /// blue (diffuse black), and an Image Map layer driving Luminous Color
    /// with picture `path` (Nearest filtering), made through the editor so
    /// it has its Texture Locator. Returns the editor, layer and locator.
    fn locator_scene(path: &str) -> (operandi_core::Editor, u64, u64) {
        use operandi_core::Command;
        use operandi_core::shader_tree::{
            Filtering, NewLayer, NodeKind, ShaderTreeEdit, TextureEffect, TextureSource,
        };
        let mut e = operandi_core::Editor::default();
        e.scene.new_mesh().unwrap();
        let mut plane =
            Mesh::box_mesh(DVec3::ZERO, DVec3::new(4.0, 4.0, 0.0), 2).unwrap_or_else(|_| {
                Mesh::box_mesh(DVec3::ZERO, DVec3::new(4.0, 4.0, 1e-3), 2).unwrap()
            });
        for f in &mut plane.faces {
            let n = f.vertices.len();
            f.uv = Some(
                (0..n)
                    .map(|k| glam::DVec2::new(k as f64 / n as f64, 0.5))
                    .collect(),
            );
        }
        e.scene.active_mut().unwrap().mesh = plane.into();
        let mut base = e.scene.shader_tree.get(4).unwrap().clone();
        if let NodeKind::Material(m) = &mut base.kind {
            m.diffuse_color = [0.0; 3];
            m.specular_amount = 0.0;
            m.luminous_color = [0.0, 0.0, 1.0];
            m.luminous_intensity = 1.0;
            m.double_sided = true;
        }
        e.scene.shader_tree.replace(base).unwrap();
        let layer = e.scene.shader_tree.next_id;
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::ImageMap,
            selected: Some(4),
        }))
        .unwrap();
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetImage {
            node: layer,
            image: Some((path.into(), 2, 2)),
        }))
        .unwrap();
        let mut node = e.scene.shader_tree.get(layer).unwrap().clone();
        let NodeKind::Texture(t) = &mut node.kind else {
            panic!()
        };
        t.effect = TextureEffect::LuminousColor;
        if let TextureSource::ImageMap { settings, .. } = &mut t.source {
            settings.filtering = Filtering::Nearest;
        }
        let locator = t.locator.unwrap();
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node,
            continuing: false,
        }))
        .unwrap();
        (e, layer, locator)
    }

    fn set_projection(
        e: &mut operandi_core::Editor,
        locator: u64,
        f: impl FnOnce(&mut operandi_core::texture_locator::TextureLocator),
    ) {
        let item = e.scene.items.iter().find(|i| i.id == locator).unwrap();
        let mut settings = item.texture_locator().unwrap().clone();
        f(&mut settings);
        let transform = item.transform.clone();
        e.dispatch(operandi_core::Command::ShaderTree(
            operandi_core::shader_tree::ShaderTreeEdit::SetLocator {
                locator,
                settings: Box::new(settings),
                transform,
                continuing: false,
            },
        ))
        .unwrap();
    }

    #[test]
    fn locator_projections_compile_to_the_op_list() {
        use operandi_core::shader_tree::Projection;
        use operandi_core::texture_locator::Tiling;
        let (mut e, _, locator) = locator_scene("/tmp/quads.png");
        // UV Map (starting): one UV op, the image reads it, the mesh
        // exports UVs.
        let m = &scene_meshes(&e.scene).unwrap()[0];
        let sh = m.shader.as_ref().unwrap();
        assert!(sh.is_valid());
        let uv = sh.ops.iter().position(|o| o.op == ShaderOp::UV).unwrap();
        assert_eq!(sh.image_vectors, vec![uv as i32]);
        assert_eq!(sh.image_extensions, vec![tile_code(Tiling::Repeat)]);
        assert!(sh.needs_uvs() && !m.uvs.is_empty());
        // Planar Z with Reset: locator point, planar coordinates, a clip
        // extension and the coverage mask; no UVs needed.
        set_projection(&mut e, locator, |l| {
            l.projection = Projection::Planar;
            l.axis = 2;
            l.repeat = [2.0, 3.0];
            l.tiling = [Tiling::Reset; 2];
        });
        let m = &scene_meshes(&e.scene).unwrap()[0];
        let sh = m.shader.as_ref().unwrap();
        assert!(sh.is_valid());
        let loc = sh
            .ops
            .iter()
            .position(|o| o.op == ShaderOp::LOCATOR)
            .unwrap();
        let planar = sh
            .ops
            .iter()
            .position(|o| o.op == ShaderOp::PLANAR)
            .unwrap();
        assert_eq!(sh.ops[planar].a, loc as i32);
        assert_eq!(sh.ops[planar].mode, 2);
        assert_eq!(sh.ops[planar].value, [0.5, 2.0, 3.0]);
        // A 1 m locator at the origin on an unmoved item: the identity.
        assert_eq!(
            sh.ops[loc].xform,
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]
        );
        assert_eq!(sh.image_vectors, vec![planar as i32]);
        assert_eq!(sh.image_extensions, vec![tile_code(Tiling::Reset)]);
        assert!(
            sh.ops
                .iter()
                .any(|o| o.op == ShaderOp::INSIDE && o.mode == 3)
        );
        assert!(!sh.needs_uvs() && m.uvs.is_empty());
        // Mixed tiling tiles the vector first.
        set_projection(&mut e, locator, |l| {
            l.tiling = [Tiling::Mirror, Tiling::Edge]
        });
        let sh = scene_meshes(&e.scene).unwrap()[0].shader.clone().unwrap();
        let tile = sh.ops.iter().position(|o| o.op == ShaderOp::TILE).unwrap();
        assert_eq!(
            sh.ops[tile].mode,
            tile_code(Tiling::Mirror) | tile_code(Tiling::Edge) << 2
        );
        assert_eq!(sh.image_vectors, vec![tile as i32]);
        assert_eq!(sh.image_extensions, vec![tile_code(Tiling::Edge)]);
        // Solid: an image shows nothing, so the plane renders its base.
        set_projection(&mut e, locator, |l| l.projection = Projection::Solid);
        let sh = scene_meshes(&e.scene).unwrap()[0].shader.clone().unwrap();
        assert!(sh.images.is_empty());
        // A named UV map is the one the mesh exports.
        set_projection(&mut e, locator, |l| {
            l.projection = Projection::UvMap;
            l.uv_map = Some("Second".into());
        });
        let item = e.scene.selection.item.unwrap();
        let mesh = &mut e
            .scene
            .items
            .iter_mut()
            .find(|i| i.id == item)
            .unwrap()
            .mesh;
        let faces: Vec<Vec<Option<glam::DVec2>>> = mesh
            .faces
            .iter()
            .map(|f| vec![Some(glam::DVec2::new(0.25, 0.75)); f.vertices.len()])
            .collect();
        let mut owned: Mesh = (**mesh).clone();
        owned.uv_maps.insert("Second".into(), faces);
        *mesh = owned.into();
        let m = &scene_meshes(&e.scene).unwrap()[0];
        assert_eq!(m.shader.as_ref().unwrap().uv_map.as_deref(), Some("Second"));
        assert!(m.uvs.iter().all(|uv| *uv == [0.25, 0.75]));
    }

    /// The Planar image on a plane at axis Z lands where the formula puts
    /// it (u = x + 0.5, v = y + 0.5, +Y up), and Reset leaves the lower
    /// layer (the blue base glow) visible outside the tile.
    #[cfg(feature = "cycles")]
    #[test]
    fn planar_image_renders_where_the_formula_puts_it() {
        use operandi_core::shader_tree::Projection;
        use operandi_core::texture_locator::Tiling;
        let dir = std::env::temp_dir().join(format!("operandi-locator-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("quads.png");
        // Top row first: red, green / white, magenta.
        #[rustfmt::skip]
        let pixels: [u8; 12] = [
            255, 0, 0,     0, 255, 0,
            255, 255, 255, 255, 0, 255,
        ];
        image::save_buffer(&path, &pixels, 2, 2, image::ExtendedColorType::Rgb8).unwrap();
        let (mut e, _, locator) = locator_scene(path.to_str().unwrap());
        set_projection(&mut e, locator, |l| {
            l.projection = Projection::Planar;
            l.axis = 2;
            l.tiling = [Tiling::Reset; 2];
        });
        let size = 65;
        // Straight down from 1 m: the view spans x, y in -1..1.
        let view = View::perspective(
            DVec3::new(0.0, 0.0, 1.0),
            DVec3::X,
            DVec3::Y,
            -DVec3::Z,
            1.0,
        );
        let mut settings = Settings::studio(size, size, view);
        settings.samples = 4;
        settings.gpu = false;
        settings.denoise = false;
        settings.world[0].intensity = 0.0;
        settings.lights = Some(Vec::new());
        let img = render(&scene_meshes(&e.scene).unwrap(), &settings).unwrap();
        // The pixel over world (x, y): top row first.
        let at = |x: f64, y: f64| {
            let col = ((x + 1.0) / 2.0 * size as f64) as usize;
            let row = ((1.0 - y) / 2.0 * size as f64) as usize;
            let i = (row * size as usize + col) * 4;
            [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]]
        };
        let is = |c: [f32; 3], want: [f32; 3]| (0..3).all(|k| (c[k] - want[k]).abs() < 0.15);
        let checks = [
            ((-0.25, 0.25), [1.0, 0.0, 0.0]),
            ((0.25, 0.25), [0.0, 1.0, 0.0]),
            ((-0.25, -0.25), [1.0, 1.0, 1.0]),
            ((0.25, -0.25), [1.0, 0.0, 1.0]),
            // Outside the 1 m tile: Reset shows the blue glow below.
            ((0.75, 0.0), [0.0, 0.0, 1.0]),
            ((-0.75, 0.6), [0.0, 0.0, 1.0]),
        ];
        for ((x, y), want) in checks {
            let c = at(x, y);
            assert!(is(c, want), "at ({x}, {y}): {c:?}, want {want:?}");
        }
        // Repeat instead: the tile to the right repeats the image, so
        // x = 0.75 (u = 1.25) reads u = 0.25, the red quarter.
        set_projection(&mut e, locator, |l| l.tiling = [Tiling::Repeat; 2]);
        let img2 = render(&scene_meshes(&e.scene).unwrap(), &settings).unwrap();
        let i = (((1.0 - 0.25) / 2.0 * size as f64) as usize * size as usize
            + ((0.75 + 1.0) / 2.0 * size as f64) as usize)
            * 4;
        let c = [img2.rgba[i], img2.rgba[i + 1], img2.rgba[i + 2]];
        assert!(is(c, [1.0, 0.0, 0.0]), "repeat at (0.75, 0.25): {c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A plane (as `locator_scene`) whose Base Material glows blue, with a
    /// procedural layer driving Luminous Color: zone 1 red, zone 2 green,
    /// locator Solid on axis Z. Returns the editor and layer.
    fn procedural_scene(
        kind: operandi_core::procedural::ProceduralKind,
        f: impl FnOnce(&mut operandi_core::procedural::Procedural),
    ) -> (operandi_core::Editor, u64) {
        use operandi_core::Command;
        use operandi_core::shader_tree::{
            NewLayer, NodeKind, ShaderTreeEdit, TextureEffect, TextureSource,
        };
        let (mut e, image_layer, _) = locator_scene("/tmp/unused.png");
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Remove(vec![
            image_layer,
        ])))
        .unwrap();
        let layer = e.scene.shader_tree.next_id;
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::Procedural(kind),
            selected: None,
        }))
        .unwrap();
        let mut node = e.scene.shader_tree.get(layer).unwrap().clone();
        let NodeKind::Texture(t) = &mut node.kind else {
            panic!()
        };
        t.effect = TextureEffect::LuminousColor;
        let TextureSource::Procedural(p) = &mut t.source else {
            panic!()
        };
        p.zones.zone1_color = [1.0, 0.0, 0.0];
        p.zones.zone2_color = [0.0, 1.0, 0.0];
        f(p);
        let locator = t.locator.unwrap();
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node,
            continuing: false,
        }))
        .unwrap();
        set_projection(&mut e, locator, |l| l.axis = 2);
        (e, layer)
    }

    #[test]
    fn procedurals_compile_to_pattern_ops() {
        use operandi_core::procedural::{Pattern, ProceduralKind};
        let (_, _) = procedural_scene(ProceduralKind::Checker, |_| {});
        for kind in ProceduralKind::ALL {
            let (e, _) = procedural_scene(*kind, |p| p.zones.bias = 0.7);
            let sh = scene_meshes(&e.scene).unwrap()[0].shader.clone().unwrap();
            assert!(sh.is_valid());
            let pattern = sh
                .ops
                .iter()
                .position(|o| o.op == ShaderOp::PATTERN)
                .unwrap();
            let loc = sh.ops[pattern].a as usize;
            assert_eq!(sh.ops[loc].op, ShaderOp::LOCATOR);
            assert_eq!(sh.ops[pattern].xform[1], 2.0, "axis Z");
            // Gabor and Poisson use Brightness/Contrast instead.
            let bias = operandi_core::procedural::Procedural::new(*kind)
                .brightness_contrast()
                .is_none();
            assert_eq!(
                sh.ops
                    .iter()
                    .any(|o| o.op == ShaderOp::BIAS_GAIN && o.value[0] == 0.7),
                bias,
                "{kind:?}"
            );
            // Solid needs no UVs and reads no image.
            assert!(!sh.needs_uvs() && sh.images.is_empty());
        }
        let (e, _) = procedural_scene(ProceduralKind::Noise, |p| {
            if let Pattern::Noise(n) = &mut p.pattern {
                n.frequencies = 6;
            }
        });
        let sh = scene_meshes(&e.scene).unwrap()[0].shader.clone().unwrap();
        let pattern = sh.ops.iter().find(|o| o.op == ShaderOp::PATTERN).unwrap();
        assert_eq!((pattern.mode, pattern.xform[2]), (3, 6.0));
    }

    /// Renders the plane from 1 m up (x, y in -1..1) with only emission.
    #[cfg(feature = "cycles")]
    fn render_from_above(scene: &Scene, size: u32) -> Image {
        let view = View::perspective(
            DVec3::new(0.0, 0.0, 1.0),
            DVec3::X,
            DVec3::Y,
            -DVec3::Z,
            1.0,
        );
        let mut settings = Settings::studio(size, size, view);
        settings.samples = 4;
        settings.gpu = false;
        settings.denoise = false;
        settings.world[0].intensity = 0.0;
        settings.lights = Some(Vec::new());
        render(&scene_meshes(scene).unwrap(), &settings).unwrap()
    }

    /// Each pattern's render matches operandi-core's reference value at
    /// the pixel centres (red = zone 1, green = zone 2), but for pixels on
    /// an edge; Noise matches in range and mean.
    #[cfg(feature = "cycles")]
    #[test]
    fn procedural_patterns_render_like_the_reference() {
        use operandi_core::procedural::{CheckerType, DotsType, GridType, Pattern, ProceduralKind};
        let size = 64u32;
        use operandi_core::shader_tree::TwoZone;
        // (kind, pattern, bias, gain)
        let cases: Vec<(ProceduralKind, Pattern, f64, f64)> = vec![
            (
                ProceduralKind::Checker,
                Pattern::Checker {
                    kind: CheckerType::Square,
                    transition: 0.1,
                    variation: Default::default(),
                },
                0.5,
                0.5,
            ),
            // Wide ramps, so Bias and Gain show everywhere.
            (
                ProceduralKind::Checker,
                Pattern::Checker {
                    kind: CheckerType::Square,
                    transition: 1.0,
                    variation: Default::default(),
                },
                0.75,
                0.3,
            ),
            (
                ProceduralKind::Grid,
                Pattern::Grid {
                    kind: GridType::Triangle,
                    line_width: 0.1,
                    transition: 0.1,
                    variation: Default::default(),
                },
                0.5,
                0.5,
            ),
            (
                ProceduralKind::Grid,
                Pattern::Grid {
                    kind: GridType::Hexagon,
                    line_width: 0.1,
                    transition: 0.1,
                    variation: Default::default(),
                },
                0.5,
                0.5,
            ),
            (
                ProceduralKind::Dots,
                Pattern::Dots {
                    kind: DotsType::Hexagon,
                    dot_width: 0.8,
                    transition: 0.1,
                    variation: Default::default(),
                },
                0.5,
                0.5,
            ),
        ];
        for (kind, pattern, bias, gain) in cases {
            let (e, _) = procedural_scene(kind, |p| {
                p.pattern = pattern.clone();
                p.zones.bias = bias;
                p.zones.gain = gain;
            });
            let img = render_from_above(&e.scene, size);
            let mut matched = 0;
            for row in 0..size {
                for col in 0..size {
                    let x = (col as f64 + 0.5) / size as f64 * 2.0 - 1.0;
                    let y = 1.0 - (row as f64 + 0.5) / size as f64 * 2.0;
                    // The plane's top sits 0.5 mm up; 2D patterns ignore it.
                    let t = TwoZone::gain(gain, TwoZone::bias(bias, pattern.value(2, [x, y, 0.0])));
                    let i = ((row * size + col) * 4) as usize;
                    let (r, g) = (img.rgba[i] as f64, img.rgba[i + 1] as f64);
                    if (r - (1.0 - t)).abs() < 0.12 && (g - t).abs() < 0.12 {
                        matched += 1;
                    }
                }
            }
            let share = matched as f64 / (size * size) as f64;
            assert!(share > 0.9, "{pattern:?}: {share:.3} of pixels match");
        }
        // Noise: soft, low contrast, centred near 0.5 (t = green).
        let (e, _) = procedural_scene(ProceduralKind::Noise, |_| {});
        let img = render_from_above(&e.scene, 32);
        let greens: Vec<f32> = img.rgba.chunks(4).map(|p| p[1]).collect();
        let mean = greens.iter().sum::<f32>() / greens.len() as f32;
        let low = greens.iter().cloned().fold(f32::INFINITY, f32::min);
        let high = greens.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!((0.35..=0.65).contains(&mean), "noise mean {mean}");
        assert!(
            low > 0.1 && high < 0.9 && high - low > 0.05,
            "noise {low}..{high}"
        );
    }

    /// Stage 3: each projection, Cull Backfacing and Falloff render like
    /// operandi-core's reference at the pixel centres (but for pixels on a
    /// seam or edge).
    #[cfg(feature = "cycles")]
    #[test]
    fn projections_render_like_the_reference() {
        use operandi_core::shader_eval::{Ch, Context, ShadingPoint, extended};
        use operandi_core::shader_tree::Projection;
        use operandi_core::texture_locator::{FalloffType, FrontCulling, TextureLocator, Tiling};
        let dir = std::env::temp_dir().join(format!("operandi-proj-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("quads.png");
        // Top row first: red, green / white, magenta.
        #[rustfmt::skip]
        let pixels: [u8; 12] = [
            255, 0, 0,     0, 255, 0,
            255, 255, 255, 255, 0, 255,
        ];
        image::save_buffer(&path, &pixels, 2, 2, image::ExtendedColorType::Rgb8).unwrap();
        // The picture with Nearest filtering: v = 0 is the bottom row.
        let picture = |l: &operandi_core::shader_eval::ImageLookup, uv: [f64; 2]| -> [f64; 4] {
            match extended(uv, l.extension) {
                Some([u, v]) => {
                    let col = ((u * 2.0).floor() as i64).clamp(0, 1);
                    let top = ((v * 2.0).floor() as i64).clamp(0, 1) == 1;
                    match (top, col) {
                        (true, 0) => [1.0, 0.0, 0.0, 1.0],
                        (true, _) => [0.0, 1.0, 0.0, 1.0],
                        (false, 0) => [1.0, 1.0, 1.0, 1.0],
                        (false, _) => [1.0, 0.0, 1.0, 1.0],
                    }
                }
                None => [0.0; 4],
            }
        };
        let turn = |z: f64| DVec3::new(0.0, 0.0, z.to_radians());
        type Setup = (
            Projection,
            u8,
            [f64; 3],
            DVec3,
            f64,
            Box<dyn Fn(&mut TextureLocator)>,
        );
        let cases: Vec<(&str, Setup)> = vec![
            (
                "cylindrical",
                (
                    Projection::Cylindrical,
                    2,
                    [0.0, 0.0, -0.25],
                    DVec3::ZERO,
                    1.0,
                    Box::new(|_| {}),
                ),
            ),
            (
                "spherical",
                (
                    Projection::Spherical,
                    2,
                    [0.0, 0.0, -0.3],
                    DVec3::ZERO,
                    1.0,
                    Box::new(|_| {}),
                ),
            ),
            (
                "box",
                (
                    Projection::Box,
                    0,
                    [0.1, 0.0, 0.0],
                    turn(45.0),
                    1.0,
                    Box::new(|_| {}),
                ),
            ),
            (
                "cubic",
                (
                    Projection::Cubic,
                    0,
                    [0.0; 3],
                    turn(30.0),
                    1.0,
                    Box::new(|_| {}),
                ),
            ),
            (
                "triplanar",
                (
                    Projection::Triplanar,
                    0,
                    [0.0; 3],
                    DVec3::ZERO,
                    0.8,
                    Box::new(|_| {}),
                ),
            ),
            (
                "cull",
                (
                    Projection::Planar,
                    2,
                    [0.0; 3],
                    DVec3::new(std::f64::consts::PI, 0.0, 0.0),
                    1.0,
                    Box::new(|l: &mut TextureLocator| {
                        l.front_culling = FrontCulling::CullBackfacing
                    }),
                ),
            ),
            (
                "falloff",
                (
                    Projection::Planar,
                    2,
                    [0.0; 3],
                    DVec3::ZERO,
                    1.0,
                    Box::new(|l: &mut TextureLocator| {
                        l.falloff_type = FalloffType::Spherical;
                        l.falloff = [1.0, 1.0, 1.0];
                        l.tiling = [Tiling::Repeat; 2];
                    }),
                ),
            ),
        ];
        let size = 48u32;
        for (name, (projection, axis, at, rotation, scale, extra)) in cases {
            let (mut e, _, locator) = locator_scene(path.to_str().unwrap());
            let item_id = e.scene.selection.item.unwrap();
            let item = e.scene.items.iter().find(|i| i.id == locator).unwrap();
            let mut settings = item.texture_locator().unwrap().clone();
            settings.projection = projection;
            settings.axis = axis;
            extra(&mut settings);
            let mut transform = item.transform.clone();
            transform.translation = DVec3::from(at);
            transform.rotation = rotation;
            transform.scale = DVec3::splat(scale);
            e.dispatch(operandi_core::Command::ShaderTree(
                operandi_core::shader_tree::ShaderTreeEdit::SetLocator {
                    locator,
                    settings: Box::new(settings),
                    transform,
                    continuing: false,
                },
            ))
            .unwrap();
            let img = render_from_above(&e.scene, size);
            let frames = e.scene.locator_frames(item_id);
            let ancestors = e.scene.item_ancestors(item_id);
            let surface = e.scene.shader_tree.evaluate(&Context {
                item: item_id,
                ancestors: &ancestors,
                material: "Default",
                part: None,
                sets: &[],
                locators: &frames,
            });
            let glow = surface.get(Ch::LuminousColor);
            let mut matched = 0;
            for row in 0..size {
                for col in 0..size {
                    let x = (col as f64 + 0.5) / size as f64 * 2.0 - 1.0;
                    let y = 1.0 - (row as f64 + 0.5) / size as f64 * 2.0;
                    let want = glow.eval_at(
                        &ShadingPoint {
                            position: [x, y, 0.0005],
                            uv: [0.0; 2],
                            normal: [0.0, 0.0, 1.0],
                        },
                        &picture,
                    );
                    let i = ((row * size + col) * 4) as usize;
                    if (0..3).all(|k| (img.rgba[i + k] as f64 - want[k]).abs() < 0.12) {
                        matched += 1;
                    }
                }
            }
            let share = matched as f64 / (size * size) as f64;
            assert!(share > 0.88, "{name}: {share:.3} of pixels match");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Stage 4: Wood (no waves or grain), Ripples and Weave render like the
    /// core reference; Cellular has cells and filler in about the
    /// reference's share; colour variation gives squares different colours.
    #[cfg(feature = "cycles")]
    #[test]
    fn stage4_patterns_render_like_the_reference() {
        use operandi_core::procedural::{Pattern, ProceduralKind};
        let size = 64u32;
        let cases: Vec<(ProceduralKind, Pattern)> = vec![
            (
                ProceduralKind::Wood,
                Pattern::Wood {
                    ring_scale: 0.25,
                    waviness: 0.0,
                    wave_scale: 1.0,
                    graininess: 0.0,
                    grain_scale: 0.1,
                },
            ),
            (
                ProceduralKind::Ripples,
                Pattern::Ripples {
                    sources: 8,
                    wavelength: 0.2,
                    phase: 0.1,
                },
            ),
            (
                ProceduralKind::Weave,
                Pattern::Weave {
                    yarn_width: 0.6,
                    roundness: 0.6,
                },
            ),
        ];
        let pixel_t = |img: &Image, row: u32, col: u32| {
            let i = ((row * size + col) * 4) as usize;
            (img.rgba[i] as f64, img.rgba[i + 1] as f64)
        };
        for (kind, pattern) in cases {
            let (e, _) = procedural_scene(kind, |p| {
                p.pattern = pattern.clone();
                p.zones.zone1_color = [1.0, 0.0, 0.0];
                p.zones.zone2_color = [0.0, 1.0, 0.0];
            });
            let img = render_from_above(&e.scene, size);
            let mut matched = 0;
            for row in 0..size {
                for col in 0..size {
                    let x = (col as f64 + 0.5) / size as f64 * 2.0 - 1.0;
                    let y = 1.0 - (row as f64 + 0.5) / size as f64 * 2.0;
                    let t = pattern.value(2, [x, y, 0.0005]);
                    let (r, g) = pixel_t(&img, row, col);
                    if (r - (1.0 - t)).abs() < 0.12 && (g - t).abs() < 0.12 {
                        matched += 1;
                    }
                }
            }
            let share = matched as f64 / (size * size) as f64;
            assert!(share > 0.85, "{kind:?}: {share:.3} of pixels match");
        }
        // Cellular: the share of cell (red) pixels near the reference's.
        let cellular = operandi_core::procedural::Procedural::new(ProceduralKind::Cellular);
        let (e, _) = procedural_scene(ProceduralKind::Cellular, |p| {
            p.pattern = cellular.pattern.clone()
        });
        let img = render_from_above(&e.scene, size);
        let mut red = 0;
        let mut want = 0;
        for row in 0..size {
            for col in 0..size {
                let x = (col as f64 + 0.5) / size as f64 * 2.0 - 1.0;
                let y = 1.0 - (row as f64 + 0.5) / size as f64 * 2.0;
                if pixel_t(&img, row, col).0 > 0.5 {
                    red += 1;
                }
                // Many samples of the reference over a wider area.
                if cellular.pattern.value(2, [x * 3.0 + 7.0, y * 3.0, 0.3]) < 0.5 {
                    want += 1;
                }
            }
        }
        let (got, expect) = (red as f64 / 4096.0, want as f64 / 4096.0);
        assert!(
            (got - expect).abs() < 0.2,
            "cells {got:.2} vs reference {expect:.2}"
        );
        assert!(got > 0.1 && got < 0.95, "cells {got:.2}");
        // Hue variation 100 %: the squares' colours differ.
        let (e, _) = procedural_scene(ProceduralKind::Checker, |p| {
            p.zones.zone1_color = [1.0, 0.2, 0.2];
            p.zones.zone2_color = [1.0, 0.2, 0.2];
            if let Pattern::Checker { variation, .. } = &mut p.pattern {
                variation.hue = 1.0;
            }
        });
        let img = render_from_above(&e.scene, size);
        let centres: Vec<[f32; 3]> = [(8, 8), (8, 24), (24, 8), (24, 24), (40, 40), (56, 8)]
            .iter()
            .map(|&(r, c)| {
                let i = ((r * size + c) * 4) as usize;
                [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]]
            })
            .collect();
        let distinct = centres
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                centres[..*i]
                    .iter()
                    .all(|b| (0..3).any(|k| (a[k] - b[k]).abs() > 0.02))
            })
            .count();
        assert!(distinct >= 4, "{centres:?}");
    }

    /// Materials stage 3: every procedural over a locator-projected image
    /// renders through the tree's MaterialX graph with the same Cycles
    /// nodes as compiling the layer stack straight.
    #[test]
    fn tree_materials_render_through_their_materialx_graph() {
        use operandi_core::materialx::tree;
        use operandi_core::procedural::ProceduralKind;
        let (e, _, _) = locator_scene("/tmp/unused.png");
        let mut scenes = vec![e.scene];
        for kind in ProceduralKind::ALL {
            scenes.push(procedural_scene(*kind, |_| {}).0.scene);
        }
        for scene in &scenes {
            let tree_ = &scene.shader_tree;
            let item = scene
                .items
                .iter()
                .find(|i| !i.mesh.faces.is_empty())
                .unwrap();
            let ctx = Context {
                item: item.id,
                ancestors: &[item.id],
                material: &item.mesh.faces[0].material,
                part: None,
                sets: &[],
                locators: &scene.locator_frames(item.id),
            };
            let p = tree_.evaluate(&ctx).principled();
            let text = tree::write(&p, scene, "Surface");
            assert!(text.contains("<operandi_surface "), "{text}");
            assert_eq!(tree::read(&text).as_ref(), Ok(&p), "{text}");
            assert_eq!(
                RenderShader::compile_through_graph(&p, scene),
                RenderShader::compile(&p, scene)
            );
        }
        // The render path itself goes through the graph (its debug check
        // compares the graph with the surface it came from).
        for scene in &scenes {
            assert!(
                scene_meshes(scene)
                    .unwrap()
                    .iter()
                    .any(|m| m.shader.is_some())
            );
        }
    }

    /// Stage 5: Gabor Noise renders mostly zone 2 with specks; Poisson
    /// Cells' cell share is near the core reference's.
    #[cfg(feature = "cycles")]
    #[test]
    fn gabor_and_poisson_render() {
        use operandi_core::procedural::{Procedural, ProceduralKind};
        let size = 64u32;
        let red_share = |img: &Image| {
            img.rgba.chunks(4).filter(|p| p[0] > 0.5).count() as f64 / (size * size) as f64
        };
        let (e, _) = procedural_scene(ProceduralKind::GaborNoise, |_| {});
        let gabor = red_share(&render_from_above(&e.scene, size));
        assert!(gabor < 0.4, "Gabor zone 1 share {gabor:.2}");
        let poisson = Procedural::new(ProceduralKind::PoissonCells);
        let (e, _) = procedural_scene(ProceduralKind::PoissonCells, |p| {
            *p = Procedural {
                zones: p.zones.clone(),
                pattern: poisson.pattern.clone(),
            }
        });
        let got = red_share(&render_from_above(&e.scene, size));
        let mut want = 0;
        for k in 0..size * size {
            let x = ((k % size) as f64 + 0.5) / size as f64 * 2.0 - 1.0;
            let y = 1.0 - ((k / size) as f64 + 0.5) / size as f64 * 2.0;
            if poisson.pattern.value(2, [x, y, 0.0]) < 0.5 {
                want += 1;
            }
        }
        let want = want as f64 / (size * size) as f64;
        assert!(
            (got - want).abs() < 0.2,
            "Poisson cells {got:.2} vs reference {want:.2}"
        );
        assert!(got > 0.2, "Poisson cells {got:.2}");
    }

    /// PT-Q09: Flow Noise in Cycles loops every 1.0 of Flow Phase, like the
    /// viewport: phases a whole number apart render the same picture, a
    /// half cycle renders a different one, and without Flow the phase is
    /// ignored.
    #[cfg(feature = "cycles")]
    #[test]
    fn flow_noise_loops_every_whole_phase() {
        use operandi_core::procedural::{Pattern, ProceduralKind};
        let size = 32u32;
        let noise = |flow: bool, phase: f64| {
            let (e, _) = procedural_scene(ProceduralKind::Noise, |p| {
                if let Pattern::Noise(n) = &mut p.pattern {
                    n.flow = flow;
                    n.flow_phase = phase;
                }
            });
            render_from_above(&e.scene, size)
        };
        // Mean absolute difference of the red channel (zone 1 share).
        let diff = |a: &Image, b: &Image| {
            a.rgba
                .chunks(4)
                .zip(b.rgba.chunks(4))
                .map(|(p, q)| (p[0] - q[0]).abs() as f64)
                .sum::<f64>()
                / (size * size) as f64
        };
        let at0 = noise(true, 0.0);
        let half = noise(true, 0.5);
        let quarter = noise(true, 0.25);
        let d = diff(&quarter, &noise(true, 1.25));
        assert!(d < 0.01, "0.25 vs 1.25: {d:.4}");
        let d = diff(&at0, &noise(true, 1.0));
        assert!(d < 0.01, "0.0 vs 1.0: {d:.4}");
        // Just short of a full cycle comes back to the start.
        let d = diff(&at0, &noise(true, 0.999));
        assert!(d < 0.02, "0.0 vs 0.999: {d:.4}");
        let d = diff(&at0, &half);
        assert!(d > 0.05, "0.0 vs 0.5 should differ: {d:.4}");
        let d = diff(&quarter, &half);
        assert!(d > 0.02, "0.25 vs 0.5 should differ: {d:.4}");
        // Flow off: the phase changes nothing.
        let off = noise(false, 0.0);
        let d = diff(&off, &noise(false, 0.37));
        assert!(d < 1e-6, "flow off, phase 0 vs 0.37: {d:.6}");
    }

    #[test]
    fn subdivision_surfaces_render_smooth_at_the_render_level() {
        use operandi_core::subdivision::{SubdivisionSettings, Toggle, toggle};
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let cube: Mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let all: Vec<u32> = (0..cube.faces.len() as u32).collect();
        scene.active_mut().unwrap().mesh =
            toggle(&cube, &all, Toggle::CatmullClark).unwrap().into();
        // Flat Smoothing everywhere else; the surface still shades smooth.
        scene.smoothing.amount = 0.;
        // Render level 2: each side becomes 16 quads, 32 triangles.
        let m = &scene_meshes(&scene).unwrap()[0];
        assert_eq!(m.triangles.len(), 6 * 32);
        assert!(m.smooth.iter().all(|&s| s == 1));
        // One shared point per surface point: no crease splits it, even at
        // the cube's old corners.
        assert_eq!(m.positions.len(), 6 * 16 + 2);
        // The surface pulls in from the cage's corners.
        assert!(m.positions.iter().all(|p| p.iter().all(|c| c.abs() < 0.5)));
        // The render level, not the display level, decides.
        scene.active_mut().unwrap().subdivision = Some(SubdivisionSettings {
            catmull_clark_level: 3,
            catmull_clark_render_level: 1,
            ..Default::default()
        });
        assert_eq!(scene_meshes(&scene).unwrap()[0].triangles.len(), 6 * 8);
    }

    #[test]
    fn hidden_and_empty_items_are_left_out() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        scene.active_mut().unwrap().visible = false;
        assert!(scene_meshes(&scene).unwrap().is_empty());
    }

    #[test]
    fn fov_spans_the_shorter_side() {
        let wide = Settings::studio(1600, 900, view());
        let tall = Settings::studio(900, 1600, view());
        assert!((wide.cycles_fov() - 2.0 * (1.0f64 / 1.8).atan()).abs() < 1e-12);
        assert!(tall.cycles_fov() < wide.cycles_fov());
    }

    #[test]
    fn srgb_conversion_hits_the_ends() {
        let image = Image {
            width: 2,
            height: 1,
            rgba: vec![0., 0., 0., 1., 1., 1., 1., 1.],
            device: "CPU".into(),
            used_gpu: false,
            denoised: false,
            samples: 1,
            light_groups: Vec::new(),
            passes: Vec::new(),
            values: Vec::new(),
            cryptomatte_manifests: Vec::new(),
        };
        assert_eq!(image.to_srgb8(), vec![0, 0, 0, 255, 255, 255, 255, 255]);
    }

    #[test]
    fn render_commands_follow_run_pause_finish_cancel() {
        let c = RenderControl::default();
        c.resume(); // nothing to resume
        assert!(!c.pause_requested());
        c.pause();
        assert!(c.pause_requested());
        c.resume();
        assert!(!c.pause_requested());
        c.pause();
        c.finish(); // finishing a paused render lets it go on to finish
        assert!(c.finish_requested() && !c.pause_requested());
        c.pause(); // too late to pause
        assert!(c.finish_requested());
        c.cancel();
        assert!(c.cancelled());
        c.finish();
        assert!(c.cancelled());
    }

    #[test]
    fn abi_mismatches_name_the_version_or_the_struct() {
        use abi::{StructSize, mismatch};
        let size = |name, app, library| StructSize { name, app, library };
        let same = [size("OcMesh", 72, 72), size("OcLight", 108, 108)];
        assert_eq!(mismatch(1, 1, &same), None);

        let old = mismatch(2, 1, &same).unwrap();
        assert_eq!(
            old,
            "Cycles library is out of date (ABI v1, app expects v2). \
             Rerun scripts/build-cycles.sh."
        );
        assert!(
            mismatch(1, 2, &same)
                .unwrap()
                .contains("newer than this app")
        );

        let resized = [size("OcMesh", 80, 72), size("OcLight", 108, 108)];
        let message = mismatch(1, 1, &resized).unwrap();
        assert!(message.contains("OcMesh is 72 bytes in the library, 80 in the app"));
        assert!(!message.contains("OcLight"));
        assert!(message.contains("scripts/build-cycles.sh"));
    }

    #[cfg(feature = "cycles")]
    #[test]
    fn the_linked_cycles_library_matches_this_app() {
        assert_eq!(ffi::check_abi(), Ok(()));
        let message = ffi::check_abi_against(abi::OC_ABI_VERSION + 1).unwrap_err();
        assert!(message.contains("out of date"), "{message}");
    }

    #[test]
    fn previews_arrive_newest_first_and_only_once() {
        let c = RenderControl::default();
        assert_eq!(c.preview_since(0), None);
        c.report_preview(&[1.0; 8]);
        c.report_preview(&[2.0; 8]);
        let (generation, area) = c.preview_since(0).unwrap();
        assert_eq!((generation, area[0]), (2, 2.0));
        assert_eq!(c.preview_since(generation), None);
        c.report_progress(40, 16, true);
        let p = c.progress();
        assert_eq!((p.samples_done, p.samples_total, p.paused), (16, 16, true));
    }

    #[test]
    fn regions_must_lie_inside_the_picture() {
        let r = |x, y, width, height| Region {
            x,
            y,
            width,
            height,
        };
        assert!(r(0, 0, 10, 10).fits(10, 10));
        assert!(r(9, 9, 1, 1).fits(10, 10));
        assert!(!r(9, 9, 2, 1).fits(10, 10));
        assert!(!r(10, 0, 1, 1).fits(10, 10));
        assert!(!r(0, 0, 0, 5).fits(10, 10));
        assert!(!r(u32::MAX, 0, 2, 2).fits(10, 10));
        let mut settings = Settings::studio(10, 10, view());
        settings.region = Some(r(5, 5, 6, 2));
        let err = render(&[], &settings).unwrap_err();
        assert!(err.contains("region"), "{err}");
        settings.region = None;
        let err = render_live(
            &[],
            &settings,
            Some(vec![0.0; 3]),
            &RenderControl::default(),
        )
        .unwrap_err();
        assert!(err.contains("size"), "{err}");
    }

    #[test]
    fn pasting_a_region_writes_only_its_rows() {
        let mut picture = vec![0.0; 4 * 3 * 4];
        let r = Region {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        };
        paste_region(&mut picture, 4, r, &[1.0; 16]);
        let lit: Vec<usize> = (0..12).filter(|&i| picture[i * 4] == 1.0).collect();
        assert_eq!(lit, vec![5, 6, 9, 10]);
        // Too little data: left alone.
        paste_region(&mut picture, 4, Region::whole(4, 3), &[2.0; 4]);
        assert!(!picture.contains(&2.0));
    }

    #[cfg(feature = "cycles")]
    fn lit_box() -> Vec<RenderMesh> {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        scene_meshes(&scene).unwrap()
    }

    /// A region render fills only the region and keeps the rest of `base`;
    /// inside it matches a full render of the same pixels.
    #[cfg(feature = "cycles")]
    #[test]
    fn renders_only_the_region() {
        let meshes = lit_box();
        let mut settings = Settings::studio(32, 24, view());
        settings.samples = 4;
        settings.gpu = false;
        settings.denoise = false;
        let full = render(&meshes, &settings).unwrap();
        settings.region = Some(Region {
            x: 8,
            y: 4,
            width: 12,
            height: 10,
        });
        let base = vec![-1.0; 32 * 24 * 4];
        let control = RenderControl::default();
        let part = render_live(&meshes, &settings, Some(base), &control).unwrap();
        let at = |img: &Image, x: usize, y: usize| img.rgba[(y * 32 + x) * 4];
        assert_eq!(at(&part, 7, 4), -1.0);
        assert_eq!(at(&part, 20, 4), -1.0);
        assert_eq!(at(&part, 8, 14), -1.0);
        for (x, y) in [(8, 4), (19, 13), (16, 12)] {
            assert!(
                (at(&part, x, y) - at(&full, x, y)).abs() < 0.15,
                "({x}, {y}): {} vs {}",
                at(&part, x, y),
                at(&full, x, y)
            );
        }
        // The in-progress picture covered the region only.
        let (_, area) = control.preview_since(0).unwrap();
        assert_eq!(area.len(), 12 * 10 * 4);
        assert_eq!(control.progress().samples_total, 4);
    }

    /// Pause holds the render; Finish ends it early with a picture; Cancel
    /// ends it with none.
    #[cfg(feature = "cycles")]
    #[test]
    fn pauses_finishes_early_and_cancels() {
        use std::sync::Arc;
        use std::time::{Duration, Instant};
        let meshes = Arc::new(lit_box());
        let mut settings = Settings::studio(256, 192, view());
        settings.samples = 100_000;
        settings.gpu = false;
        settings.denoise = false;
        let run = |control: &Arc<RenderControl>| {
            let (m, s, c) = (Arc::clone(&meshes), settings.clone(), Arc::clone(control));
            std::thread::spawn(move || render_live(&m, &s, None, &c))
        };
        let wait_for = |what: &dyn Fn() -> bool| {
            let start = Instant::now();
            while !what() {
                assert!(start.elapsed() < Duration::from_secs(60), "timed out");
                std::thread::sleep(Duration::from_millis(10));
            }
        };

        let control = Arc::new(RenderControl::default());
        let job = run(&control);
        wait_for(&|| control.progress().samples_done > 0);
        control.pause();
        wait_for(&|| control.progress().paused);
        let held = control.progress().samples_done;
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(control.progress().samples_done, held, "still rendering");
        control.resume();
        wait_for(&|| control.progress().samples_done > held);
        control.finish();
        let image = job.join().unwrap().unwrap();
        assert!(
            image.samples > 0 && image.samples < 100_000,
            "{}",
            image.samples
        );
        assert!(image.rgba.iter().any(|&v| v > 0.0));

        let control = Arc::new(RenderControl::default());
        let job = run(&control);
        wait_for(&|| control.progress().samples_done > 0);
        control.cancel();
        let err = job.join().unwrap().unwrap_err();
        assert_eq!(err, "Cancelled");
    }

    #[test]
    fn light_paths_map_to_bounce_limits() {
        let all = LightPaths::default();
        assert!(all.shadows && all.diffuse && all.glossy && all.transmission);
        assert_eq!(all.bounce_limits([4, 8, 8]), [4, 8, 8]);
        let some = LightPaths {
            diffuse: false,
            transmission: false,
            ..all
        };
        assert_eq!(some.bounce_limits([4, 8, 8]), [0, 8, 0]);
        // A render that doesn't set the Render item rows keeps Cycles' own
        // limits (7 each), clamp (10) and caustics (old scenes, RI-05).
        let rays = Rays::default();
        assert_eq!(rays.depths, [7, 7, 7]);
        assert_eq!(rays.max_bounces, 7);
        assert_eq!(rays.max_radiance, 10.0);
        assert_eq!(rays.caustics, [true, true]);
        // A final render keeps every light path, no adaptive sampling and
        // all threads.
        let s = Settings::studio(8, 8, view());
        assert_eq!(s.light_paths, all);
        assert_eq!(
            (s.adaptive_threshold, s.threads, s.transparent),
            (0.0, 0, false)
        );
    }

    /// The Preview's options reach Cycles: transparent film makes the sky
    /// alpha 0 over an opaque box; shadows off, bounces off, adaptive
    /// sampling and a thread count all render.
    #[cfg(feature = "cycles")]
    #[test]
    fn preview_options_reach_cycles() {
        let meshes = lit_box();
        let mut settings = Settings::studio(32, 24, view());
        settings.samples = 8;
        settings.gpu = false;
        settings.denoise = false;
        settings.transparent = true;
        let image = render(&meshes, &settings).unwrap();
        let alpha = |x: usize, y: usize| image.rgba[(y * 32 + x) * 4 + 3];
        assert!(alpha(0, 0) < 0.01, "sky alpha {}", alpha(0, 0));
        assert!(alpha(16, 12) > 0.99, "box alpha {}", alpha(16, 12));
        settings.transparent = false;
        settings.light_paths = LightPaths {
            shadows: false,
            diffuse: false,
            glossy: false,
            transmission: false,
        };
        settings.adaptive_threshold = 0.05;
        settings.threads = 1;
        let image = render(&meshes, &settings).unwrap();
        assert!(image.rgba.iter().all(|v| v.is_finite()));
        assert!(image.rgba.chunks(4).all(|p| p[3] > 0.99));
    }

    /// Needs liboperandi_cycles: `cargo test -p operandi-cycles --features cycles`.
    #[cfg(feature = "cycles")]
    #[test]
    fn renders_a_lit_box_over_the_sky() {
        let meshes = lit_box();
        let mut settings = Settings::studio(32, 24, view());
        settings.samples = 4;
        settings.gpu = false;
        settings.denoise = false;
        let image = render(&meshes, &settings).unwrap();
        assert_eq!(image.rgba.len(), 32 * 24 * 4);
        let pixel = |x: usize, y: usize| &image.rgba[(y * 32 + x) * 4..(y * 32 + x) * 4 + 3];
        let (centre, corner) = (pixel(16, 12), pixel(0, 0));
        assert!(centre.iter().all(|v| v.is_finite()));
        // The corner is sky (blue-ish); the centre is the grey box.
        assert!(corner[2] > corner[0], "{corner:?}");
        assert!((centre[0] - corner[0]).abs() + (centre[2] - corner[2]).abs() > 0.02);
        assert!(cycles_version().unwrap().starts_with('5'));
        assert!(!image.used_gpu);
    }

    /// CP-V13 panoramas and CP-E01 depth of field reach Cycles: VR
    /// variants put the box (straight ahead) in the middle, the others put
    /// it at the left and right edges; depth of field blurs the picture.
    #[cfg(feature = "cycles")]
    #[test]
    fn renders_panoramas_and_depth_of_field() {
        use operandi_core::camera_light::Projection;
        let mut scene = camera_light_scene();
        let id = scene.render_camera().unwrap();
        fn cam(s: &mut Scene, id: u64) -> &mut operandi_core::camera_light::CameraSettings {
            s.items
                .iter_mut()
                .find(|i| i.id == id)
                .unwrap()
                .camera
                .as_mut()
                .unwrap()
        }
        let shoot = |scene: &Scene, w: u32, h: u32| {
            let view = scene_camera_view(scene, f64::from(w) / f64::from(h)).unwrap();
            let mut settings = Settings::studio(w, h, view).with_scene_lights(scene);
            settings.samples = 4;
            settings.gpu = false;
            settings.denoise = false;
            render(&scene_meshes(scene).unwrap(), &settings).unwrap()
        };
        let (w, h) = (64usize, 32usize);
        for (projection, ahead_in_middle) in [
            (Projection::SphericalVr, true),
            (Projection::Spherical, false),
            (Projection::CylindricalVr, true),
            (Projection::Cylindrical, false),
        ] {
            cam(&mut scene, id).projection = projection;
            let image = shoot(&scene, w as u32, h as u32);
            let pixel = |x: usize, y: usize| image.rgba[(y * w + x) * 4];
            // The sky's colour is the corner above the horizon line.
            let sky = pixel(w / 4, 1);
            let (middle, edge) = (pixel(w / 2, h / 2), pixel(0, h / 2));
            let box_at = if ahead_in_middle { middle } else { edge };
            let empty = if ahead_in_middle { edge } else { middle };
            assert!(
                (box_at - sky).abs() > 0.02 && (empty - sky).abs() < 0.02,
                "{projection:?}: box {box_at} empty {empty} sky {sky}"
            );
        }
        cam(&mut scene, id).projection = Projection::Perspective;
        let sharp = shoot(&scene, 36, 24);
        let c = cam(&mut scene, id);
        c.dof.enable = true;
        c.dof.f_stop = 1.0;
        c.focus_distance = 0.2;
        c.focal_length = 0.2;
        let blurred = shoot(&scene, 36, 24);
        let diff: f32 = sharp
            .rgba
            .iter()
            .zip(&blurred.rgba)
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(blurred.rgba.iter().all(|v| v.is_finite()));
        assert!(diff > 1.0, "depth of field changed nothing ({diff})");
    }

    /// The scene camera (perspective and orthographic) and a scene sun
    /// reach Cycles: the box at the origin is in the middle of the picture.
    #[cfg(feature = "cycles")]
    #[test]
    fn renders_through_the_scene_camera_and_light() {
        use operandi_core::camera_light::Projection;
        let mut scene = camera_light_scene();
        for projection in [Projection::Perspective, Projection::Orthographic] {
            let id = scene.render_camera().unwrap();
            scene
                .items
                .iter_mut()
                .find(|i| i.id == id)
                .unwrap()
                .camera
                .as_mut()
                .unwrap()
                .projection = projection;
            let view = scene_camera_view(&scene, 36.0 / 24.0).unwrap();
            let mut settings = Settings::studio(36, 24, view).with_scene_lights(&scene);
            settings.samples = 4;
            settings.gpu = false;
            settings.denoise = false;
            let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
            let pixel = |x: usize, y: usize| &image.rgba[(y * 36 + x) * 4..(y * 36 + x) * 4 + 3];
            let (centre, corner) = (pixel(18, 12), pixel(0, 0));
            assert!(centre.iter().all(|v| v.is_finite()));
            assert!(
                (centre[0] - corner[0]).abs() + (centre[2] - corner[2]).abs() > 0.02,
                "{projection:?}: {centre:?} {corner:?}"
            );
        }
    }

    /// Horizontal centre (0..1) of the pixels that differ from the corner
    /// (the background), weighted by how much they differ.
    #[cfg(feature = "cycles")]
    fn object_centre_x(image: &Image) -> f64 {
        let w = image.width as usize;
        let px = |i: usize| &image.rgba[i * 4..i * 4 + 3];
        let bg = px(0).to_vec();
        let (mut sum, mut weight) = (0.0, 0.0);
        for i in 0..image.rgba.len() / 4 {
            let d: f32 = px(i).iter().zip(&bg).map(|(a, b)| (a - b).abs()).sum();
            if d > 0.05 {
                sum += (i % w) as f64 * f64::from(d);
                weight += f64::from(d);
            }
        }
        assert!(weight > 0.0, "nothing but background");
        sum / weight / w as f64
    }

    /// Regression (mirrored renders, 2026-11-05): a box at world +X in
    /// front of the default camera lands on the right of the picture, both
    /// through the scene camera and through a viewport-style view.
    #[cfg(feature = "cycles")]
    #[test]
    fn plus_x_renders_on_the_right() {
        let mut scene = camera_light_scene();
        scene.items[0].mesh = Mesh::box_mesh(DVec3::new(1.0, 0.75, 0.0), DVec3::splat(0.6), 1)
            .unwrap()
            .into();
        let scene_view = scene_camera_view(&scene, 48.0 / 27.0).unwrap();
        let viewport_view = View::perspective(
            DVec3::new(0.0, 0.75, 4.0),
            DVec3::X,
            DVec3::Y,
            -DVec3::Z,
            1.0 / 1.8,
        );
        for (name, view) in [("scene camera", scene_view), ("viewport", viewport_view)] {
            let mut settings = Settings::studio(48, 27, view);
            settings.samples = 2;
            settings.gpu = false;
            settings.denoise = false;
            let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
            let x = object_centre_x(&image);
            assert!(x > 0.55, "{name}: box centre at {x:.2} of the width");
        }
    }

    /// Point, spot and area lights reach Cycles and light the box.
    #[cfg(feature = "cycles")]
    #[test]
    fn renders_point_spot_and_area_lights() {
        let mut dark = None;
        for kind in [
            None,
            Some(LightKind::Point),
            Some(LightKind::Spot),
            Some(LightKind::Area),
        ] {
            let mut scene = Scene::default();
            scene.new_mesh().unwrap();
            scene.active_mut().unwrap().mesh =
                Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
            let eye = DVec3::new(0.0, 0.0, 2.5);
            let mut settings = Settings::studio(32, 24, view());
            settings.world[0].intensity = 0.0;
            if let Some(kind) = kind {
                // Light the box's front from in front of it.
                scene
                    .add_light(
                        LightSettings::new(kind),
                        operandi_core::scene::Transform {
                            translation: eye,
                            ..Default::default()
                        },
                    )
                    .unwrap();
            } else {
                settings.sun_strength = 0.0;
            }
            let mut settings = settings.with_scene_lights(&scene);
            settings.samples = 4;
            settings.gpu = false;
            settings.denoise = false;
            let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
            let centre = &image.rgba[(12 * 32 + 16) * 4..(12 * 32 + 16) * 4 + 3];
            assert!(centre.iter().all(|v| v.is_finite()));
            let lum: f32 = centre.iter().sum();
            match dark {
                None => dark = Some(lum),
                Some(d) => assert!(lum > d + 0.05, "{kind:?}: {lum} vs unlit {d}"),
            }
        }
    }

    /// Linear value of the picture's centre pixel for a lit plane:
    /// albedo 0.6, no sky, one light of `kind` (Intensity 3) 2 m above the
    /// plane's centre, shining straight down; the camera looks at that
    /// point from off to the side so no highlight lands there.
    #[cfg(feature = "cycles")]
    fn plane_centre_value(kind: LightKind, falloff: LightFalloff) -> f32 {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::new(0.0, 0.0, -0.05), DVec3::new(40.0, 40.0, 0.1), 1)
                .unwrap()
                .into();
        let mut light = LightSettings::new(kind);
        light.falloff = falloff;
        scene
            .add_light(
                light,
                operandi_core::scene::Transform {
                    translation: DVec3::new(0.0, 0.0, 2.0),
                    ..Default::default()
                },
            )
            .unwrap();
        let eye = DVec3::new(1.5, 0.0, 1.0);
        let forward = (-eye).normalize();
        let right = forward.cross(DVec3::Y).normalize();
        let up = right.cross(forward);
        let mut settings =
            Settings::studio(33, 33, View::perspective(eye, right, up, forward, 0.05))
                .with_scene_lights(&scene);
        settings.world[0].intensity = 0.0;
        settings.base_color = [0.6; 3];
        settings.roughness = 1.0;
        settings.samples = 64;
        settings.gpu = false;
        settings.denoise = false;
        let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
        let px = &image.rgba[(16 * 33 + 16) * 4..(16 * 33 + 16) * 4 + 3];
        // The middle few pixels, averaged against sampling noise.
        let mut sum = 0.0;
        for y in 15..18 {
            for x in 15..18 {
                sum += image.rgba[(y * 33 + x) * 4 + 1];
            }
        }
        let _ = px;
        sum / 9.0
    }

    /// Spec v2 brightness law (no tone mapping): pixel = albedo/π ·
    /// irradiance, with irradiance I/d² for a point light and I for a sun.
    /// Measured with I = 3 at 2 m: 0.1426 and 0.5703.
    #[cfg(feature = "cycles")]
    #[test]
    fn light_brightness_matches_the_measured_values() {
        let point = plane_centre_value(LightKind::Point, LightFalloff::InverseDistanceSquared);
        let spot = plane_centre_value(LightKind::Spot, LightFalloff::InverseDistanceSquared);
        let sun = plane_centre_value(LightKind::Directional, LightFalloff::InverseDistanceSquared);
        eprintln!("point {point} spot {spot} sun {sun}");
        assert!((point - 0.1426).abs() < 0.1426 * 0.02, "point {point}");
        assert!((spot - 0.1426).abs() < 0.1426 * 0.02, "spot {spot}");
        assert!((sun - 0.5703).abs() < 0.5703 * 0.02, "sun {sun}");
    }

    /// The GPU path, with denoising. Falls back to the CPU on machines
    /// without a supported GPU, so it passes anywhere; it prints the device.
    #[cfg(feature = "cycles")]
    #[test]
    fn renders_on_the_gpu_when_there_is_one() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        let mut settings = Settings::studio(64, 48, view());
        settings.samples = 8;
        let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
        eprintln!(
            "device: {} (gpu {}, denoised {})",
            image.device, image.used_gpu, image.denoised
        );
        assert!(image.rgba.iter().all(|v| v.is_finite()));
        let centre = &image.rgba[(24 * 64 + 32) * 4..(24 * 64 + 32) * 4 + 3];
        let corner = &image.rgba[0..3];
        assert!((centre[0] - corner[0]).abs() + (centre[2] - corner[2]).abs() > 0.02);
    }

    /// A red Base Material renders red; a shader hidden from the camera
    /// leaves only the sky (ST-4).
    #[cfg(feature = "cycles")]
    #[test]
    fn shader_tree_colours_and_hides_in_the_render() {
        use operandi_core::shader_tree::NodeKind;
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        let mut base = scene.shader_tree.get(4).unwrap().clone();
        if let NodeKind::Material(m) = &mut base.kind {
            m.diffuse_color = [1., 0., 0.];
        }
        scene.shader_tree.replace(base).unwrap();
        let mut settings = Settings::studio(32, 24, view());
        settings.samples = 4;
        settings.gpu = false;
        settings.denoise = false;
        let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
        let centre = &image.rgba[(12 * 32 + 16) * 4..(12 * 32 + 16) * 4 + 3];
        assert!(
            centre[0] > 2. * centre[1] && centre[0] > 2. * centre[2],
            "{centre:?}"
        );
        let mut shader = scene.shader_tree.get(3).unwrap().clone();
        if let NodeKind::Shader(s) = &mut shader.kind {
            s.visible_to_camera = false;
        }
        scene.shader_tree.replace(shader).unwrap();
        let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
        let centre = &image.rgba[(12 * 32 + 16) * 4..(12 * 32 + 16) * 4 + 3];
        let corner = &image.rgba[0..3];
        let d: f32 = centre.iter().zip(corner).map(|(a, b)| (a - b).abs()).sum();
        assert!(d < 0.05, "centre {centre:?} vs sky {corner:?}");
    }

    /// The mean linear colour of a unit box's front face, filling the
    /// frame, with one image layer of `effect` reading `path`, Specular
    /// Amount 0, under an even white sky of strength 1 and no sun: there a
    /// diffuse surface shows its albedo.
    #[cfg(feature = "cycles")]
    fn flat_map_under_even_sky(
        path: &std::path::Path,
        effect: operandi_core::shader_tree::TextureEffect,
    ) -> [f32; 3] {
        use operandi_core::shader_tree::{NewLayer, NodeKind, TextureSource};
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        scene.images.push(operandi_core::backdrop::SceneImage {
            id: 9,
            name: "map".into(),
            path: path.to_string_lossy().into_owned(),
            width: 4,
            height: 4,
        });
        let tex = scene.shader_tree.add(NewLayer::ImageMap, Some(4)).unwrap();
        let mut t = scene.shader_tree.get(tex).unwrap().clone();
        if let NodeKind::Texture(x) = &mut t.kind {
            x.effect = effect;
            if let TextureSource::ImageMap { image, .. } = &mut x.source {
                *image = Some(9);
            }
        }
        scene.shader_tree.replace(t).unwrap();
        for n in &mut scene.shader_tree.nodes {
            if let NodeKind::Material(m) = &mut n.kind {
                // Physically Based: Diffuse Amount scales the colour.
                m.shading_model = operandi_core::shader_tree::ShadingModel::PhysicallyBased;
                m.diffuse_color = [0.8; 3];
                m.diffuse_amount = 1.;
                m.specular_amount = 0.;
            }
        }
        let meshes = scene_meshes(&scene).unwrap();
        assert!(meshes[0].shader.as_ref().unwrap().is_valid());
        let view = View::perspective(DVec3::new(0., 0., 20.), DVec3::X, DVec3::Y, -DVec3::Z, 0.02);
        let mut settings = Settings::studio(16, 16, view);
        settings.samples = 64;
        settings.gpu = false;
        settings.denoise = false;
        settings.sun_strength = 0.;
        settings.world = vec![EnvironmentLayer::flat([1., 1., 1.], 1.)];
        let image = render(&meshes, &settings).unwrap();
        let mut sum = [0f32; 3];
        for p in image.rgba.chunks(4) {
            for c in 0..3 {
                sum[c] += p[c];
            }
        }
        sum.map(|v| v / (16 * 16) as f32)
    }

    /// An 8-bit colour map is sRGB: code 128 renders as 0.216 linear, not
    /// as 0.502 (the colour maps came out pale when Cycles was left to
    /// guess). The same picture on a data effect stays a plain number, and
    /// a float EXR colour map is already linear.
    #[cfg(feature = "cycles")]
    #[test]
    fn colour_maps_render_srgb_decoded_and_data_maps_as_numbers() {
        use operandi_core::shader_tree::TextureEffect;
        let dir = std::env::temp_dir().join(format!("operandi-srgb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("grey128.png");
        image::RgbImage::from_pixel(4, 4, image::Rgb([128, 128, 128]))
            .save(&png)
            .unwrap();
        let exr = dir.join("grey30.exr");
        image::Rgb32FImage::from_pixel(4, 4, image::Rgb([0.3, 0.3, 0.3]))
            .save(&exr)
            .unwrap();
        let near = |got: [f32; 3], want: f32| {
            assert!(
                got.iter().all(|v| (v - want).abs() < 0.02),
                "{got:?} vs {want}"
            );
        };
        // Diffuse Color reads the picture as a colour.
        near(
            flat_map_under_even_sky(&png, TextureEffect::DiffuseColor),
            0.216,
        );
        near(
            flat_map_under_even_sky(&exr, TextureEffect::DiffuseColor),
            0.3,
        );
        // Diffuse Amount is a number: the same picture is 0.502 (not
        // decoded) and scales the 0.8 diffuse colour.
        let amount = flat_map_under_even_sky(&png, TextureEffect::DiffuseAmount);
        let _ = std::fs::remove_dir_all(&dir);
        near(amount, 0.8 * 0.502);
    }

    #[cfg(feature = "cycles")]
    #[test]
    fn image_swizzle_gamma_and_filtering_render() {
        // G-05 in Cycles: a plain red picture as Diffuse Color. Swizzled
        // to Red it renders grey (all three channels from red); swizzled
        // to Green, black; as RGBA, red. Gamma and Nearest compile too.
        use operandi_core::shader_tree::{Filtering, NewLayer, NodeKind, Swizzle, TextureSource};
        let path = std::env::temp_dir().join(format!("operandi-red-{}.png", std::process::id()));
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .save(&path)
            .unwrap();
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        scene.images.push(operandi_core::backdrop::SceneImage {
            id: 7,
            name: "red".into(),
            path: path.to_string_lossy().into_owned(),
            width: 2,
            height: 2,
        });
        let tex = scene.shader_tree.add(NewLayer::ImageMap, Some(4)).unwrap();
        let mut settings = Settings::studio(32, 24, view());
        settings.samples = 4;
        settings.gpu = false;
        settings.denoise = false;
        let mut centre = |swizzle: Swizzle| {
            let mut t = scene.shader_tree.get(tex).unwrap().clone();
            if let NodeKind::Texture(x) = &mut t.kind
                && let TextureSource::ImageMap { image, settings } = &mut x.source
            {
                *image = Some(7);
                settings.swizzle = swizzle;
                settings.gamma = 2.;
                settings.filtering = Filtering::Nearest;
            }
            scene.shader_tree.replace(t).unwrap();
            let meshes = scene_meshes(&scene).unwrap();
            assert!(meshes[0].shader.as_ref().unwrap().is_valid());
            let image = render(&meshes, &settings).unwrap();
            let at = (12 * 32 + 16) * 4;
            [image.rgba[at], image.rgba[at + 1], image.rgba[at + 2]]
        };
        let red = centre(Swizzle::Rgba);
        let grey = centre(Swizzle::Red);
        let black = centre(Swizzle::Green);
        let _ = std::fs::remove_file(&path);
        assert!(red[0] > 2. * red[1], "{red:?}");
        assert!(
            (grey[0] - grey[1]).abs() < 0.2 * grey[0] && grey[1] > 2. * red[1],
            "{grey:?}"
        );
        assert!(black[0] < 0.5 * grey[0], "{black:?} vs {grey:?}");
    }

    /// Colour maps are read as sRGB; roughness, metal and height maps as
    /// plain data (Blender's Non-Color), so an 8-bit roughness map isn't
    /// darkened by the sRGB curve.
    #[test]
    fn data_maps_are_read_without_the_colour_conversion() {
        use operandi_core::shader_tree::{NewLayer, NodeKind, TextureEffect, TextureSource};
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.active_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        let effects = [
            ("colour.png", TextureEffect::DiffuseColor),
            ("rough.png", TextureEffect::Roughness),
            ("metal.png", TextureEffect::Metallic),
            ("height.png", TextureEffect::Displacement),
        ];
        for (k, (name, effect)) in effects.iter().enumerate() {
            let id = k as u64 + 1;
            scene.images.push(operandi_core::backdrop::SceneImage {
                id,
                name: (*name).into(),
                path: format!("/maps/{name}"),
                width: 2,
                height: 2,
            });
            let tex = scene.shader_tree.add(NewLayer::ImageMap, Some(4)).unwrap();
            let mut t = scene.shader_tree.get(tex).unwrap().clone();
            if let NodeKind::Texture(x) = &mut t.kind {
                x.effect = *effect;
                if let TextureSource::ImageMap { image, .. } = &mut x.source {
                    *image = Some(id);
                }
            }
            scene.shader_tree.replace(t).unwrap();
        }
        if let Some(n) = scene
            .shader_tree
            .nodes
            .iter_mut()
            .find(|n| matches!(n.kind, NodeKind::Material(_)))
            && let NodeKind::Material(m) = &mut n.kind
        {
            m.displacement_distance = 0.01;
        }
        let meshes = scene_meshes(&scene).unwrap();
        let shader = meshes[0].shader.as_ref().unwrap();
        let raw = |name: &str| {
            let i = shader
                .images
                .iter()
                .position(|p| p.ends_with(name))
                .unwrap_or_else(|| panic!("{name} not in {:?}", shader.images));
            shader.image_raw[i]
        };
        assert_eq!(raw("colour.png"), 0);
        assert_eq!(raw("rough.png"), 1);
        assert_eq!(raw("metal.png"), 1);
        assert_eq!(raw("height.png"), 1);
    }

    /// A unit box at the origin, its UVs (when `uvs`) laid flat from x
    /// and y, with one texture layer of `effect` set by `f` on top of the
    /// Base Material.
    fn relief_scene(
        uvs: bool,
        effect: operandi_core::shader_tree::TextureEffect,
        f: impl FnOnce(&mut operandi_core::shader_tree::TextureLayer),
    ) -> Scene {
        use operandi_core::shader_tree::{NewLayer, NodeKind};
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        if uvs {
            let points = mesh.vertices.clone();
            for face in &mut mesh.faces {
                face.uv = Some(
                    face.vertices
                        .iter()
                        .map(|&v| {
                            let p = points[v as usize];
                            glam::DVec2::new(p.x + 0.5, p.y + 0.5)
                        })
                        .collect(),
                );
            }
        }
        scene.active_mut().unwrap().mesh = mesh.into();
        let id = scene.shader_tree.add(NewLayer::Constant, Some(4)).unwrap();
        let mut node = scene.shader_tree.get(id).unwrap().clone();
        if let NodeKind::Texture(t) = &mut node.kind {
            t.effect = effect;
            f(t);
        }
        scene.shader_tree.replace(node).unwrap();
        scene
    }

    #[test]
    fn relief_compiles_to_its_own_nodes() {
        use operandi_core::shader_tree::{TextureEffect, TextureSource};
        // Displacement: a node; the surface still renders its colours.
        let scene = relief_scene(false, TextureEffect::Displacement, |t| {
            t.source = TextureSource::Constant {
                color: [0.; 3],
                value: 0.5,
            };
        });
        let meshes = scene_meshes(&scene).unwrap();
        let sh = meshes[0].shader.as_ref().unwrap();
        assert!(sh.is_valid() && sh.displaces());
        assert_eq!(sh.ops[sh.displacement as usize].value[0], 0.01);
        assert_eq!((sh.bump, sh.normal_map), (-1, -1));
        // A normal map on a mesh without UVs does nothing: studio clay.
        let normal = |t: &mut operandi_core::shader_tree::TextureLayer| {
            t.source = TextureSource::Constant {
                color: [0.9, 0.5, 0.6],
                value: 0.,
            };
        };
        let bare = scene_meshes(&relief_scene(false, TextureEffect::Normal, normal)).unwrap();
        assert_eq!(bare[0].shader, None);
        // With UVs it compiles, and the mesh carries them.
        let mapped = scene_meshes(&relief_scene(true, TextureEffect::Normal, normal)).unwrap();
        let sh = mapped[0].shader.as_ref().unwrap();
        assert!(sh.is_valid() && sh.normal_map >= 0 && !sh.displaces());
        assert_eq!(mapped[0].uvs.len(), mapped[0].triangles.len() * 3);
    }

    /// Pixels that aren't the sky (the top-left corner's colour).
    #[cfg(feature = "cycles")]
    fn covered(image: &Image) -> usize {
        let sky = [image.rgba[0], image.rgba[1], image.rgba[2]];
        image
            .rgba
            .chunks(4)
            .filter(|p| (0..3).map(|i| (p[i] - sky[i]).abs()).sum::<f32>() > 0.05)
            .count()
    }

    #[cfg(feature = "cycles")]
    #[test]
    fn displacement_moves_the_surface_in_the_render() {
        // G-07: a Displacement constant of 1 with Distance 0.25 m pushes
        // every face of the unit box out by 0.25 m: it covers more of the
        // picture. Value 0 leaves it as it was.
        use operandi_core::shader_tree::{NodeKind, TextureEffect, TextureSource};
        let push = |value: f64| {
            let mut scene = relief_scene(false, TextureEffect::Displacement, |t| {
                t.source = TextureSource::Constant {
                    color: [0.; 3],
                    value,
                };
            });
            let mut base = scene.shader_tree.get(4).unwrap().clone();
            if let NodeKind::Material(m) = &mut base.kind {
                m.displacement_distance = 0.25;
                // Not clay, so both renders use the tree's surface.
                m.diffuse_color = [1., 0.2, 0.2];
            }
            scene.shader_tree.replace(base).unwrap();
            let mut settings = Settings::studio(64, 48, view());
            settings.samples = 2;
            settings.gpu = false;
            settings.denoise = false;
            covered(&render(&scene_meshes(&scene).unwrap(), &settings).unwrap())
        };
        let (flat, pushed) = (push(0.), push(1.));
        // The box's edges are hard, so each face moves out on its own: the
        // front face comes 0.25 m closer (about 1.25 times the area) and the
        // sides show past it.
        assert!(
            pushed as f64 > 1.25 * flat as f64,
            "flat {flat} px, displaced {pushed} px"
        );
    }

    #[cfg(feature = "cycles")]
    #[test]
    fn bump_and_normal_maps_change_the_shading() {
        // G-07: a left-to-right ramp as Bump (with a large amplitude) tilts
        // the front face and changes its brightness; so does a tilted
        // Constant normal map on a mesh with UVs. Without UVs the normal
        // map renders exactly like no layer at all.
        use operandi_core::shader_tree::{NodeKind, TextureEffect, TextureSource};
        let path = std::env::temp_dir().join(format!("operandi-ramp-{}.png", std::process::id()));
        image::RgbaImage::from_fn(16, 1, |x, _| {
            let v = (x * 255 / 15) as u8;
            image::Rgba([v, v, v, 255])
        })
        .save(&path)
        .unwrap();
        let mut settings = Settings::studio(32, 24, view());
        settings.samples = 8;
        settings.gpu = false;
        settings.denoise = false;
        let centre = |scene: &Scene| {
            let image = render(&scene_meshes(scene).unwrap(), &settings).unwrap();
            let at = (12 * 32 + 16) * 4;
            image.rgba[at..at + 3].iter().sum::<f32>()
        };
        let none = relief_scene(true, TextureEffect::Bump, |t| {
            t.source = TextureSource::Constant {
                color: [0.; 3],
                value: 0.,
            };
        });
        let plain = centre(&none);

        let mut bumped = relief_scene(true, TextureEffect::Bump, |t| {
            t.source = TextureSource::ImageMap {
                image: Some(7),
                settings: Default::default(),
            };
        });
        bumped.images.push(operandi_core::backdrop::SceneImage {
            id: 7,
            name: "ramp".into(),
            path: path.to_string_lossy().into_owned(),
            width: 16,
            height: 1,
        });
        let mut base = bumped.shader_tree.get(4).unwrap().clone();
        if let NodeKind::Material(m) = &mut base.kind {
            m.bump_amplitude = 0.5;
        }
        bumped.shader_tree.replace(base).unwrap();
        assert!(
            scene_meshes(&bumped).unwrap()[0]
                .shader
                .as_ref()
                .unwrap()
                .bump
                >= 0
        );
        let with_bump = centre(&bumped);
        // The same ramp as Displacement: diced finely and moved, with UVs
        // on the diced faces (renders, and the face is no longer flat-lit).
        let mut displaced = bumped.clone();
        for node in &mut displaced.shader_tree.nodes {
            if let NodeKind::Texture(t) = &mut node.kind
                && t.effect == TextureEffect::Bump
            {
                t.effect = TextureEffect::Displacement;
            }
        }
        assert!(
            scene_meshes(&displaced).unwrap()[0]
                .shader
                .as_ref()
                .unwrap()
                .displaces()
        );
        let with_displacement = centre(&displaced);

        let tilt = |t: &mut operandi_core::shader_tree::TextureLayer| {
            t.source = TextureSource::Constant {
                color: [0.95, 0.5, 0.6],
                value: 0.,
            };
        };
        let with_normal = centre(&relief_scene(true, TextureEffect::Normal, tilt));
        let _ = std::fs::remove_file(&path);
        assert!(
            (with_bump - plain).abs() > 0.1 * plain,
            "plain {plain}, bumped {with_bump}"
        );
        assert!(
            (with_displacement - plain).abs() > 0.1 * plain,
            "plain {plain}, displaced {with_displacement}"
        );
        assert!(
            (with_normal - plain).abs() > 0.1 * plain,
            "plain {plain}, normal-mapped {with_normal}"
        );
    }

    #[test]
    fn bad_input_is_an_error_not_a_crash() {
        let bad = RenderMesh {
            positions: vec![[0.; 3]],
            triangles: vec![[0, 1, 2]],
            smooth: vec![0],
            ..Default::default()
        };
        assert!(render(&[bad], &Settings::studio(8, 8, view())).is_err());
        assert!(render(&[], &Settings::studio(0, 8, view())).is_err());
    }

    #[test]
    fn selection_set_groups_split_render_meshes() {
        // G-01: a group masked by selection set "Top" colours only the
        // polygons in that set, also once the cage is subdivided.
        use operandi_core::shader_tree::{NewLayer, NodeKind, TagMask, TagType, TextureSource};
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let item = scene.active_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        item.mesh
            .attributes
            .polygon_sets
            .insert("Top".into(), [0u32, 1].into_iter().collect());
        let tree = &mut scene.shader_tree;
        let group = tree.add(NewLayer::Group, Some(4)).unwrap();
        let mut g = tree.get(group).unwrap().clone();
        if let NodeKind::Group(m) = &mut g.kind {
            m.tag_type = TagType::SelectionSet;
            m.tag = TagMask::Name("Top".into());
        }
        tree.replace(g).unwrap();
        let tex = tree.add(NewLayer::Constant, Some(group)).unwrap();
        let mut t = tree.get(tex).unwrap().clone();
        if let NodeKind::Texture(x) = &mut t.kind {
            x.source = TextureSource::Constant {
                color: [1., 0., 0.],
                value: 0.,
            };
        }
        tree.replace(t).unwrap();
        let meshes = scene_meshes(&scene).unwrap();
        assert_eq!(meshes.len(), 2);
        let red = meshes.iter().find(|m| m.shader.is_some()).unwrap();
        assert_eq!(red.triangles.len(), 4);
        let base = red
            .shader
            .as_ref()
            .unwrap()
            .constant(Input::BaseColor)
            .unwrap();
        assert!((base[0] - 1.0).abs() < 1e-6 && base[1].abs() < 1e-6);
        // Subdivided (render level 2, 32 triangles a side): every piece of
        // the two set polygons stays red.
        use operandi_core::subdivision::{Toggle, toggle};
        let item = scene.active_mut().unwrap();
        let all: Vec<u32> = (0..item.mesh.faces.len() as u32).collect();
        item.mesh = toggle(&item.mesh, &all, Toggle::CatmullClark)
            .unwrap()
            .into();
        assert!(item.mesh.attributes.polygon_sets.contains_key("Top"));
        let meshes = scene_meshes(&scene).unwrap();
        let red = meshes.iter().find(|m| m.shader.is_some()).unwrap();
        assert_eq!(red.triangles.len(), 2 * 32);
    }

    #[test]
    fn image_adjustments_compile_to_channel_gamma_and_filters() {
        use operandi_core::shader_tree::{
            Filtering, NewLayer, NodeKind, Swizzle, TextureEffect, TextureSource,
        };
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let item = scene.active_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        scene.images.push(operandi_core::backdrop::SceneImage {
            id: 7,
            name: "grid".into(),
            path: "/tmp/grid.png".into(),
            width: 4,
            height: 4,
        });
        // Two layers read the same picture with different filtering.
        for (filter, effect) in [
            (Filtering::Nearest, TextureEffect::DiffuseColor),
            (Filtering::Bicubic, TextureEffect::Roughness),
        ] {
            let tex = scene.shader_tree.add(NewLayer::ImageMap, Some(4)).unwrap();
            let mut t = scene.shader_tree.get(tex).unwrap().clone();
            if let NodeKind::Texture(x) = &mut t.kind {
                x.effect = effect;
                if let TextureSource::ImageMap { image, settings } = &mut x.source {
                    *image = Some(7);
                    settings.filtering = filter;
                    settings.swizzle = Swizzle::Green;
                    settings.gamma = 2.2;
                }
            }
            scene.shader_tree.replace(t).unwrap();
        }
        let meshes = scene_meshes(&scene).unwrap();
        let sh = meshes[0].shader.as_ref().unwrap();
        assert!(sh.is_valid());
        assert_eq!(sh.images, vec!["/tmp/grid.png".to_owned(); 2]);
        let mut filters = sh.image_filters.clone();
        filters.sort();
        assert_eq!(filters, vec![0, 2]);
        assert!(
            sh.ops
                .iter()
                .any(|o| o.op == ShaderOp::CHANNEL && o.mode == 1)
        );
        assert!(
            sh.ops
                .iter()
                .any(|o| o.op == ShaderOp::GAMMA && (o.value[0] - 1. / 2.2).abs() < 1e-6)
        );
    }

    /// The see-through part and the new specular reach the compiled surface.
    #[test]
    fn transparency_compiles_to_a_mix_with_its_own_nodes() {
        use operandi_core::shader_tree::{NodeKind, ShaderTree};
        let surface = |f: &dyn Fn(&mut operandi_core::shader_tree::MaterialChannels)| {
            let mut tree = ShaderTree::default();
            let mut base = tree.get(4).unwrap().clone();
            if let NodeKind::Material(m) = &mut base.kind {
                f(m);
            }
            tree.replace(base).unwrap();
            let p = tree.evaluate(&calibration_ctx()).principled();
            RenderShader::compile(&p, &Scene::default())
        };
        let sh = surface(&|_| {});
        assert!(sh.is_valid());
        assert_eq!(sh.transparency_mode, 0);
        assert_eq!(sh.constant(Input::Ior), Some([1.5; 3]));
        assert_eq!(sh.constant(Input::SpecularIorLevel), Some([0.5; 3]));
        assert_eq!(sh.constant(Input::TransmissionWeight), None);
        let sh = surface(&|m| {
            m.transparent_amount = 0.5;
            m.transparent_color = [0.2, 0.4, 0.6];
            m.refractive_index = 1.5;
            m.refraction_roughness = 0.3;
        });
        assert!(sh.is_valid());
        assert_eq!(sh.transparency_mode, 2);
        let value = |i: i32| sh.ops[i as usize].value;
        assert_eq!(value(sh.transparency_amount), [0.5; 3]);
        // Render fix F4: a solid at Absorption Distance 0 passes the
        // square root at each crossing.
        assert_eq!(
            value(sh.transparency_color),
            [0.2f32, 0.4, 0.6].map(|v| (v as f64).sqrt() as f32)
        );
        assert_eq!(value(sh.refraction_ior), [1.5; 3]);
        assert_eq!(value(sh.refraction_roughness), [0.3; 3]);
        assert_eq!(sh.constant(Input::TransmissionWeight), None);
        let sh = surface(&|m| m.transparent_amount = 1.0);
        assert_eq!(sh.transparency_mode, 1);
        assert_eq!(sh.refraction_ior, -1);
        let mut broken = sh.clone();
        broken.transparency_color = -1;
        assert!(!broken.is_valid());
    }

    fn calibration_ctx() -> Context<'static> {
        Context {
            item: 1,
            ancestors: &[],
            material: "Default",
            part: None,
            sets: &[],
            locators: &[],
        }
    }

    /// The centre pixel (linear RGB) of a 40 m plane facing +Z, seen
    /// straight down from 1 m (18 mm lens on a 36 mm square film, 65 px),
    /// with no global illumination. `sun`: a directional light of
    /// intensity 3 straight down over a black sky; otherwise no light and
    /// a 0.5 grey sky that lights only reflections and refractions.
    /// The material starts from Diffuse Color 0.6 at 100 %, everything
    /// else 0, Roughness 40 %, Refractive Index 1.0, Principled.
    #[cfg(feature = "cycles")]
    fn calibration_pixel(
        sun: bool,
        f: impl FnOnce(&mut operandi_core::shader_tree::MaterialChannels),
    ) -> [f32; 3] {
        use operandi_core::shader_tree::{MaterialChannels, NodeKind, ShaderTree};
        let mut m = MaterialChannels {
            diffuse_color: [0.6; 3],
            diffuse_amount: 1.0,
            specular_amount: 0.0,
            fresnel: 0.0,
            roughness: 0.4,
            refractive_index: 1.0,
            subsurface_amount: 0.0,
            luminous_intensity: 0.0,
            bump_amplitude: 0.0,
            displacement_distance: 0.0,
            double_sided: true,
            ..MaterialChannels::default()
        };
        f(&mut m);
        let mut tree = ShaderTree::default();
        let mut base = tree.get(4).unwrap().clone();
        base.kind = NodeKind::Material(Box::new(m));
        tree.replace(base).unwrap();
        let p = tree.evaluate(&calibration_ctx()).principled();
        let shader = RenderShader::compile(&p, &Scene::default());
        assert!(shader.is_valid());
        let plane = RenderMesh {
            positions: vec![
                [-20.0, -20.0, 0.0],
                [20.0, -20.0, 0.0],
                [20.0, 20.0, 0.0],
                [-20.0, 20.0, 0.0],
            ],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
            smooth: vec![0, 0],
            uvs: Vec::new(),
            shader: Some(shader),
            light_set: 0,
            light_membership: None,
            attributes: Vec::new(),
            normals: Vec::new(),
            transform: None,
            ..Default::default()
        };
        // 18 mm over half of 36 mm: tan(half angle) = 1.
        let view = View::perspective(
            DVec3::new(0.0, 0.0, 1.0),
            DVec3::X,
            DVec3::Y,
            -DVec3::Z,
            1.0,
        );
        let mut settings = Settings::studio(65, 65, view);
        settings.samples = 16;
        settings.gpu = false;
        settings.denoise = false;
        settings.light_paths = LightPaths {
            shadows: true,
            diffuse: false,
            glossy: false,
            transmission: false,
        };
        if sun {
            settings.world[0].intensity = 0.0;
            settings.lights = Some(vec![SceneLight {
                shape: LightShape::Sun { angle: 0.0 },
                position: DVec3::new(0.0, 0.0, 10.0),
                right: DVec3::X,
                up: DVec3::Y,
                direction: -DVec3::Z,
                strength: [3.0; 3],
                cast_shadow: true,
                visible_to_camera: false,
                affects_diffuse: true,
                affects_specular: true,
                visible_to_reflection: false,
                visible_to_refraction: false,
                ies: None,
                lightgroup: None,
                layers: Vec::new(),
                gobo_tan_half: 0.0,
                light_membership: None,
                shadow_membership: None,
                caustics_only: false,
            }]);
        } else {
            settings.lights = Some(Vec::new());
            settings.world = vec![EnvironmentLayer::flat([0.5; 3], 1.0)];
            settings.world_lights_diffuse = false;
        }
        let image = render(&[plane], &settings).unwrap();
        let at = (32 * 65 + 32) * 4;
        [image.rgba[at], image.rgba[at + 1], image.rgba[at + 2]]
    }

    /// Translation map v0.1 (2026-11-06): the reference renders' centre
    /// values, within 3 %.
    #[cfg(feature = "cycles")]
    #[test]
    fn material_settings_render_at_the_reference_brightness() {
        type Edit = Box<dyn FnOnce(&mut operandi_core::shader_tree::MaterialChannels)>;
        let red = |m: &mut operandi_core::shader_tree::MaterialChannels| {
            m.diffuse_color = [0.8, 0.1, 0.1];
        };
        let cases: Vec<(&str, bool, Edit, [f32; 3])> = vec![
            ("d100", true, Box::new(|_| {}), [0.573; 3]),
            (
                "d80",
                true,
                Box::new(|m| m.diffuse_amount = 0.8),
                [0.573; 3],
            ),
            (
                "s25",
                true,
                Box::new(|m| {
                    m.diffuse_color = [0.0; 3];
                    m.specular_amount = 0.25;
                }),
                [0.187; 3],
            ),
            (
                "s50",
                true,
                Box::new(|m| {
                    m.diffuse_color = [0.0; 3];
                    m.specular_amount = 0.5;
                }),
                [0.374; 3],
            ),
            (
                "s100",
                true,
                Box::new(|m| {
                    m.diffuse_color = [0.0; 3];
                    m.specular_amount = 1.0;
                }),
                [0.749; 3],
            ),
            (
                "ds",
                true,
                Box::new(|m| m.specular_amount = 0.5),
                [0.925; 3],
            ),
            (
                "default",
                true,
                Box::new(|m| {
                    m.diffuse_amount = 0.8;
                    m.specular_amount = 0.5;
                }),
                [0.925; 3],
            ),
            (
                "refl100",
                false,
                Box::new(move |m| {
                    red(m);
                    m.diffuse_amount = 0.8;
                    m.reflection_amount = 1.0;
                    m.roughness = 0.0;
                }),
                [0.0; 3],
            ),
            (
                "tran100",
                false,
                Box::new(move |m| {
                    red(m);
                    m.diffuse_amount = 0.8;
                    m.transparent_amount = 1.0;
                    m.transparent_color = [1.0; 3];
                }),
                [0.5; 3],
            ),
            (
                "tran50",
                false,
                Box::new(move |m| {
                    red(m);
                    m.diffuse_amount = 0.8;
                    m.transparent_amount = 0.5;
                    m.transparent_color = [1.0; 3];
                }),
                [0.25; 3],
            ),
            (
                "metal",
                false,
                Box::new(move |m| {
                    red(m);
                    m.metallic = 1.0;
                    m.roughness = 0.0;
                }),
                [0.40, 0.05, 0.05],
            ),
        ];
        let mut misses = Vec::new();
        for (name, sun, edit, want) in cases {
            let got = calibration_pixel(sun, edit);
            eprintln!("{name}: measured {got:?}, expected {want:?}");
            // 3 % of the expected value; a black target allows 3 % of the
            // sky (0.015).
            let ok = got
                .iter()
                .zip(want)
                .all(|(g, w)| (g - w).abs() <= (w * 0.03).max(if w == 0.0 { 0.015 } else { 0.0 }));
            if !ok {
                misses.push(format!("{name}: {got:?} vs {want:?}"));
            }
        }
        assert!(misses.is_empty(), "{}", misses.join("; "));
    }
}

#[cfg(test)]
mod anim_render_tests;

#[cfg(test)]
mod shader_settings_tests;

#[cfg(test)]
mod gradient_tests;

#[cfg(test)]
mod locator_projection_tests;

#[cfg(test)]
mod environment_tests;

#[cfg(test)]
mod vertex_map_tests;

#[cfg(test)]
mod process_tests;

#[cfg(test)]
mod material_extras_tests;

#[cfg(test)]
mod caustics_tests;

#[cfg(test)]
mod emission_units_tests;
#[cfg(all(test, feature = "cycles"))]
mod render_fixes_tests;

#[cfg(test)]
mod outputs_render_tests;

#[cfg(test)]
mod outputs_d_render_tests;

#[cfg(test)]
mod session_tests;

#[cfg(test)]
mod materialx_graph_tests;

#[cfg(test)]
mod node_editing_tests;

#[cfg(test)]
mod node_graph_tests;

#[cfg(test)]
mod openpbr_tests;

#[cfg(test)]
mod patch_render_tests;

#[cfg(test)]
mod replica_render_tests;
