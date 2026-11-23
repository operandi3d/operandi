//! Meshes kept on the graphics card. Each visible item's filled polygons and
//! wireframe are built once, in world space, and reused across frames and
//! across views; the shader projects them with each view's camera. They are
//! rebuilt only when the item's mesh, placement, selection colors or
//! subdivision change. Orbiting, panning and zooming cost no per-polygon CPU
//! work. Rollover and live tool previews stay small per-frame overlays.
//!
//! Projection matches `Camera::project` and `outline::vertex` exactly:
//! perspective clip = (x·sx, y·sy, z − near, z), so depth = 1 − near/z
//! (`Camera::near`, 0.01 unless a wheel zoom to surface came closer);
//! orthographic clip = (x·sx, y·sy, (z + offset) / range, 1), the slab
//! `Camera::ortho_depth` fixes about the orbit centre (offset in eye.w).
use crate::camera::{Camera, View};
use crate::theme::viewport as palette;
use eframe::egui::{Color32, Rect};
use glam::{DMat4, DVec3};
use operandi_core::shading::SmoothingSettings;
use operandi_core::{Item, Mesh, Shared};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

/// A filled-triangle corner in world space: its smooth-shading normal
/// (the polygon's own for faceted polygons; the shader makes its flat
/// normal itself), its unlit color and its vertex-map color. The view's
/// display style lights it in the shader (gpu.rs), so one geometry serves
/// every view and style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// The unlit colour; for a polygon drawn by a material program with a
    /// per-corner pattern value (`shader_wgsl::Program::slot`), that value
    /// instead, stored linearly.
    pub color: Color32,
    pub map: Color32,
    /// The UV map a material program reads at this corner.
    pub uv: [f32; 2],
    /// The material program instance drawing the polygon
    /// (`GpuMaterials`), 0 for the vertex colour.
    pub program: u32,
    /// HT-5/HT-6 tints (`Tint`): the selected weight map's value here and
    /// the polygon's UV distortion colour. The view's toggles pick which
    /// show (gpu.rs `Look`), so one geometry serves every view.
    pub tint: Tint,
}

/// A fill corner's two tints, each RGBA8 with alpha 0 for none:
/// `weight` = (the weight map's value w as (w + 1) × 127, clamped to ±1;
/// -; -; 255 where the mesh has the map), coloured per pixel in the
/// shader like `viewport::weight_colour`; `distortion` = the polygon's UV
/// view distortion colour (`uv_view::polygon_distortion`), unpremultiplied.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tint {
    pub weight: [u8; 4],
    pub distortion: [u8; 4],
}
impl Tint {
    /// A weight as the tint's first byte: -1 → 0, 0 → 127, +1 → 254.
    pub fn weight(w: f64) -> [u8; 4] {
        let w = if w.is_finite() { w.clamp(-1., 1.) } else { 0. };
        [((w + 1.) * 127.).round() as u8, 0, 0, 255]
    }
    /// A Color32 (premultiplied) as unpremultiplied RGBA8.
    pub fn colour(c: Color32) -> [u8; 4] {
        c.to_srgba_unmultiplied()
    }
}

/// How much of the colour under it the weight tint covers (HT-5):
/// all of it, so the shading shows VMT-30's colours lit (provisional,
/// HT-Q3).
pub const WEIGHT_TINT: f32 = 1.;

/// The fill shader's `op_tint` (HT-5/HT-6): the look's toggles
/// (`glass.z` weights, `glass.w` distortion) pick the corner's tints;
/// the weight wins where the mesh has the map. The weight is coloured per
/// pixel from its interpolated value, as `viewport::weight_colour` does.
pub fn tint_wgsl() -> String {
    let v = |c: [f64; 3]| {
        format!(
            "vec3<f32>({:.6}, {:.6}, {:.6})",
            c[0] / 255.,
            c[1] / 255.,
            c[2] / 255.
        )
    };
    format!(
        r#"
// HT-5 Show Weightmaps / HT-6 Show UV Distortion (gpu_scene.rs Tint).
fn op_weight_colour(w: f32) -> vec3<f32> {{
  let zero = {zero};
  var end = {neg};
  if (w >= 0.0) {{ end = {pos}; }}
  return zero + (end - zero) * min(abs(w), 1.0);
}}
fn op_tint(base: vec3<f32>, tw: vec4<f32>, td: vec4<f32>) -> vec3<f32> {{
  if (look.glass.z > 0.5 && tw.a > 0.5) {{
    let w = tw.r * 255.0 / 127.0 - 1.0;
    return mix(base, op_weight_colour(w), {opacity:.3});
  }}
  if (look.glass.w > 0.5 && td.a > 0.0) {{ return mix(base, td.rgb, td.a); }}
  return base;
}}
"#,
        zero = v(crate::viewport::WEIGHT_ZERO),
        pos = v(crate::viewport::WEIGHT_POSITIVE),
        neg = v(crate::viewport::WEIGHT_NEGATIVE),
        opacity = WEIGHT_TINT,
    )
}

/// What the tints baked into the geometry show (`set_tints`): the selected
/// weight map when some view shows weights, and whether some view shows UV
/// distortion. Off, nothing is computed for them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tints {
    pub weights: Option<String>,
    pub distortion: bool,
}

/// One corner of an edge stroke: the edge's two ends, which end this corner
/// sits at (0 or 1), which side (−1 or +1), its width in points (0 for the
/// view's wire width; highlighted edges are 2), its color and how many
/// polygons it stands for (`uses`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeVertex {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub end: f32,
    pub side: f32,
    pub width: f32,
    pub color: Color32,
    /// 1, or 2 for the stroke of an edge two polygons share, drawn once
    /// with the opacity of two blended strokes (the edge shader's `twice`):
    /// every polygon still strokes its own sides, but the graphics card
    /// draws half as many strokes.
    pub uses: f32,
    /// The world normals of the polygons on either side of the edge (zero
    /// where there is none), so Fade Wireframe on Small Polygons can keep
    /// outline and sharp edges (gpu.rs `edge_out_fading`).
    pub sides: [[f32; 3]; 2],
    /// Never fades (a subdivided polygon's cage edge, traced in pieces
    /// far shorter than the edge).
    pub always: bool,
}

/// What Fade Wireframe on Small Polygons needs to know about a stroke.
#[derive(Clone, Copy, Default)]
struct Feature {
    sides: [[f32; 3]; 2],
    always: bool,
}

/// One corner of a point dot: its center, which corner of the square
/// (−1/+1, −1/+1), its radius in points and its color. The fragment shader
/// cuts the square to a disc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DotVertex {
    pub center: [f32; 3],
    pub corner: [f32; 2],
    pub radius: f32,
    pub color: Color32,
}

/// Six square corners (two triangles) for a dot at `p`.
pub fn dot(out: &mut Vec<DotVertex>, p: DVec3, radius: f32, color: Color32) {
    let center = p.as_vec3().to_array();
    for corner in [
        [-1., -1.],
        [1., -1.],
        [1., 1.],
        [-1., -1.],
        [1., 1.],
        [-1., 1.],
    ] {
        out.push(DotVertex {
            center,
            corner,
            radius,
            color,
        });
    }
}

/// Six stroke corners (two triangles) for edge `a`–`b`.
pub fn stroke(out: &mut Vec<EdgeVertex>, a: DVec3, b: DVec3, highlighted: bool, color: Color32) {
    stroke_width(out, a, b, if highlighted { 2. } else { 0. }, color);
}

/// A stroke `width` points wide; 0 for the plain edge width.
pub fn stroke_width(out: &mut Vec<EdgeVertex>, a: DVec3, b: DVec3, width: f32, color: Color32) {
    stroke_uses(out, a, b, width, color, 1., Feature::default());
}

/// `stroke_width` for `uses` polygons' strokes of the same edge.
fn stroke_uses(
    out: &mut Vec<EdgeVertex>,
    a: DVec3,
    b: DVec3,
    width: f32,
    color: Color32,
    uses: f32,
    feature: Feature,
) {
    let (a, b) = (a.as_vec3().to_array(), b.as_vec3().to_array());
    for (end, side) in [
        (0., -1.),
        (0., 1.),
        (1., 1.),
        (0., -1.),
        (1., 1.),
        (1., -1.),
    ] {
        out.push(EdgeVertex {
            a,
            b,
            end,
            side,
            width,
            color,
            uses,
            sides: feature.sides,
            always: feature.always,
        });
    }
}

/// An item's geometry, ready to upload.
#[derive(Default, Debug, Clone)]
pub struct ItemGeometry {
    pub fill: Vec<WorldVertex>,
    /// The wireframe, every polygon's sides in the plain color.
    pub edges: Vec<EdgeVertex>,
    /// Selected polygons' sides in the selection color, drawn on top.
    pub highlight: Vec<EdgeVertex>,
    /// Per cage polygon, its block in `fill` and in `edges`.
    pub fill_ranges: Vec<std::ops::Range<usize>>,
    pub edge_ranges: Vec<std::ops::Range<usize>>,
    /// The placement the geometry was built at: material programs read the
    /// item's own points and normals through its inverse.
    pub built: DMat4,
    /// Subdivision polygons drawn as patches on the graphics card
    /// (subd_gpu.rs); their blocks above are empty.
    pub patches: Option<Arc<PatchDraw>>,
    /// Some polygon's material is see-through (Transparent Amount), so the
    /// viewport's glass pass draws this item (gpu.rs, viewport
    /// transparency). Whatever the polygons' selection state.
    pub clear: bool,
    /// Some polygon's material has cut-outs, so its shadow is lighter
    /// (VM-Q10).
    pub cut: bool,
}

/// An item's Subdivision polygons as patches (docs/SUBD_GPU_DESIGN.md):
/// the controls, each cage polygon's look, and the placement the patches
/// are drawn at (they stay in the item's own space).
#[derive(Debug, Clone)]
pub struct PatchDraw {
    pub net: Arc<operandi_core::subdivision::patches::PatchNet>,
    /// Per cage polygon: fill colour, vertex-map colour (alpha 0 when
    /// highlighted, as `marked`), flags (`PATCH_HIDDEN`, `PATCH_FACETED`),
    /// material program instance (0: none).
    /// Then the HT-5/HT-6 tints (`Tint`), the polygon's average weight.
    pub looks: Vec<[u32; PATCH_LOOK]>,
    /// Per patch (cage corner), the UV its polygon's material program
    /// reads there (SDV-15: the patch's UVs are bilinear over the corner,
    /// its sides' middles and the polygon's middle); `None` when no patch
    /// polygon has a program.
    pub uvs: Option<Arc<Vec<[f32; 2]>>>,
    /// The plain wire colour.
    pub wire: Color32,
}
/// `PatchDraw::looks` flags.
pub const PATCH_HIDDEN: u32 = 1;
/// u32s per cage polygon in `PatchDraw::looks`.
pub const PATCH_LOOK: usize = 6;
pub const PATCH_FACETED: u32 = 2;

/// What changed since the last upload: the item's blocks to rewrite on the
/// graphics card, if it still holds generation `from`.
#[derive(Debug, Clone)]
pub struct Patch {
    pub from: u64,
    pub fill: Vec<std::ops::Range<usize>>,
    pub edges: Vec<std::ops::Range<usize>>,
    /// The selection highlight was rebuilt (else it is as uploaded).
    pub highlight: bool,
}

/// A mesh in a cache key, equal only to the same allocation: comparing a
/// million points every frame to find a drag's copy different costs more
/// than the patch that follows, which diffs the points once anyway (and
/// patches nothing when an equal copy turns up).
#[derive(Clone)]
struct MeshRef(Shared<Mesh>);
impl PartialEq for MeshRef {
    fn eq(&self, other: &Self) -> bool {
        Shared::same(&self.0, &other.0)
    }
}
impl std::ops::Deref for MeshRef {
    type Target = Mesh;
    fn deref(&self) -> &Mesh {
        &self.0
    }
}
impl From<Shared<Mesh>> for MeshRef {
    fn from(mesh: Shared<Mesh>) -> Self {
        Self(mesh)
    }
}

/// What an item's geometry depends on; equal keys reuse the geometry.
#[derive(Clone, PartialEq)]
struct Key {
    mesh: MeshRef,
    selected: Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
    subdivision: Option<operandi_core::subdivision::SubdivisionSettings>,
    /// The default material's smoothing (SS-01): corner normals depend on it.
    smoothing: SmoothingSettings,
    /// The colour theme (theme::generation): a Preferences colour change
    /// rebuilds the colours baked into the geometry.
    palette: u64,
    /// The scene's material colors and flat materials.
    materials: LooksRef,
    /// Locked and hidden geometry, and whether locked shows (LCK-23/25):
    /// a change rebuilds. Empty for most items, so cheap to compare.
    elements: operandi_core::ElementState,
    show_locked: bool,
    /// Subdivision polygons drawn as patches.
    patched: bool,
    /// HT-5/HT-6: what the tints show.
    tints: Tints,
}

/// What the viewport takes from the scene's shader tree (shader_eval.rs):
/// each polygon's colour is the tree folded for its item, material tag and
/// part, the same way renders fold it, then Diffuse Color × Diffuse Amount
/// worked out on the CPU (images stand in as their average colour until
/// the viewport draws image maps). Polygons whose colour comes out at the
/// tree's defaults keep the theme's plain fill colour, so a scene without
/// materials looks as it always did. Smoothing still comes from the stored
/// materials, as it does in renders.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MaterialLooks {
    /// `None`: an untouched tree and no stored materials, plain fill
    /// everywhere without folding anything.
    tree: Option<operandi_core::shader_tree::ShaderTree>,
    /// The tree was never edited (`Scene::stored_materials_tree`): a tag
    /// with no stored material keeps the plain fill, as renders give it
    /// default values.
    mapped: bool,
    stored: BTreeSet<String>,
    flat: BTreeSet<String>,
    /// Defer Smoothing During Interaction: every polygon shades flat while
    /// a drag changes the mesh; smoothing comes back on the mouse-up.
    pub all_flat: bool,
    /// Item → parent, for item-group masks.
    parents: std::collections::BTreeMap<u64, u64>,
    /// Image id → its pixels and average linear colour (`None`: not
    /// readable).
    images: std::collections::BTreeMap<u64, Option<Arc<ImageData>>>,
    /// The scene's Texture Locators. The viewport shows an image's average,
    /// so only the projection type matters (a Solid image covers nothing);
    /// placement isn't drawn (provisional).
    locators: Vec<operandi_core::texture_locator::LocatorFrame>,
    /// Per mesh item, its placed Texture Locators, taking the item's own
    /// points and normals: procedural layers are evaluated at the mesh's
    /// points (stage 5; coarse on low-poly meshes).
    frames: std::collections::BTreeMap<u64, Vec<operandi_core::texture_locator::LocatorFrame>>,
    /// Polygon contexts whose colour reads images, as programs the fill
    /// shader runs per pixel; filled in as polygons ask (derived data, so
    /// equal looks compare equal whatever it holds).
    programs: Arc<ProgramTable>,
    /// Per mesh item, where a graph layer's Generated and world coordinates
    /// come from (MaterialX stage 5, `graph_view`).
    graph_spaces: std::collections::BTreeMap<u64, operandi_core::graph_view::GraphSpace>,
    /// Graph pictures by path, as their ids in `images`.
    graph_images: std::collections::BTreeMap<String, u64>,
    /// Graphs flattened per layer settings (derived data, like `programs`).
    graph_flats: Arc<GraphFlats>,
}
/// The colour a polygon context folds to, or `None` for the plain fill.
type Looked = Option<Color32>;

/// Flattened node graphs by graph id and exposed values, so folding a
/// polygon context doesn't flatten (or translate a MaterialX graph) again.
#[derive(Default)]
pub struct GraphFlats(
    std::sync::Mutex<HashMap<String, Option<Arc<operandi_core::node_graph::FlatGraph>>>>,
);
impl std::fmt::Debug for GraphFlats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GraphFlats")
    }
}
impl PartialEq for GraphFlats {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl MaterialLooks {
    /// The looks of `scene`; images count as unreadable. The viewport uses
    /// `Resident::sync_materials`, which also averages image files.
    #[cfg(test)]
    pub fn of(scene: &operandi_core::Scene) -> Self {
        Self::with_images(scene, |_| None)
    }
    pub fn with_images(
        scene: &operandi_core::Scene,
        mut image: impl FnMut(&operandi_core::backdrop::SceneImage) -> Option<Arc<ImageData>>,
    ) -> Self {
        let mut looks = Self::default();
        for (name, m) in &scene.materials {
            looks.stored.insert(name.clone());
            if !m.smoothing {
                looks.flat.insert(name.clone());
            }
        }
        let untouched = scene.shader_tree.is_default();
        if untouched && scene.materials.is_empty() {
            return looks;
        }
        looks.tree = Some(match scene.stored_materials_tree() {
            Ok(Some(tree)) => {
                looks.mapped = true;
                tree
            }
            // The Library left out, references resolved (layer menu §4).
            _ => scene.shader_tree.resolved().into_owned(),
        });
        looks.mapped &= untouched;
        looks.parents = scene
            .items
            .iter()
            .filter_map(|i| Some((i.id, i.parent?)))
            .collect();
        looks.images = scene.images.iter().map(|i| (i.id, image(i))).collect();
        // Graph layers (MaterialX stage 5): their pictures, and each mesh's
        // Generated and world coordinates.
        if let Some(tree) = looks
            .tree
            .as_ref()
            .filter(|t| !t.node_graphs.graphs.is_empty())
        {
            for path in operandi_core::graph_view::image_paths(tree) {
                let id = operandi_core::graph_view::image_id(&path);
                let picture = operandi_core::backdrop::SceneImage {
                    id,
                    name: path.clone(),
                    path: path.clone(),
                    width: 0,
                    height: 0,
                };
                looks.images.insert(id, image(&picture));
                looks.graph_images.insert(path, id);
            }
            looks.graph_spaces = scene
                .items
                .iter()
                .filter(|i| i.is_mesh())
                .filter_map(|i| {
                    let (lo, hi) = i.mesh.shared().bounds()?;
                    let world = scene.world_matrix(i.id).ok()?;
                    Some((i.id, graph_space(lo, hi, world)))
                })
                .collect();
        }
        // Procedural layers and image maps are evaluated at the mesh's own
        // points (patterns per corner, images per pixel on the GPU).
        let placed = scene.shader_tree.nodes.iter().any(|n| {
            matches!(&n.kind, operandi_core::shader_tree::NodeKind::Texture(t)
                if matches!(t.source, operandi_core::shader_tree::TextureSource::Procedural(_)
                    | operandi_core::shader_tree::TextureSource::ImageMap { .. }))
        });
        if placed {
            // In each item's own space, so moving an item needs no new
            // colours (World Coordinates off follows the item anyway).
            looks.frames = scene
                .items
                .iter()
                .filter(|i| i.is_mesh())
                .filter_map(|i| {
                    let world = scene.world_matrix(i.id).ok()?;
                    let normal_world = glam::DMat3::from_mat4(world).inverse().transpose();
                    let frames = scene
                        .locator_frames(i.id)
                        .into_iter()
                        .map(|mut f| {
                            f.world_to_locator *= world;
                            f.world_to_box *= world;
                            f.normal_to_locator *= normal_world;
                            f.normal_to_space *= normal_world;
                            f
                        })
                        .collect();
                    Some((i.id, frames))
                })
                .collect();
        }
        looks.locators = locator_settings(scene)
            .into_iter()
            .map(|(id, settings, _)| (id, settings))
            .map(|(id, settings)| {
                operandi_core::texture_locator::LocatorFrame::unplaced(id, settings)
            })
            .collect();
        looks
    }
    fn key(tag: &str) -> &str {
        if operandi_core::material::is_default_name(tag) {
            operandi_core::material::DEFAULT_MATERIAL
        } else {
            tag
        }
    }
    /// The colour of a polygon context whose surface carries a procedural
    /// pattern, as an expression to evaluate at each mesh point; `None` for
    /// the flat colour of `look`.
    fn pattern(
        &self,
        item: u64,
        material: &str,
        part: Option<&str>,
    ) -> Option<operandi_core::shader_eval::Expr> {
        let (base, textures) = self.base(item, material, part)?;
        (textures && base.varies_by_pattern()).then_some(base)
    }
    /// A polygon context's Diffuse Color × Amount, and whether the winning
    /// shader shows textures in the viewport.
    fn base(
        &self,
        item: u64,
        material: &str,
        part: Option<&str>,
    ) -> Option<(operandi_core::shader_eval::Expr, bool)> {
        let surface = self.surface(item, material, part)?;
        Some(Self::base_of(&surface))
    }
    fn base_of(
        surface: &operandi_core::shader_eval::Surface,
    ) -> (operandi_core::shader_eval::Expr, bool) {
        use operandi_core::shader_eval::{Ch, Expr};
        let base = Expr::times(
            surface.get(Ch::DiffuseColor).clone(),
            surface.get(Ch::DiffuseAmount).clone(),
        );
        (base, surface.shader.gl_display)
    }
    /// The tree folded for one polygon context, `None` for the plain fill.
    fn surface(
        &self,
        item: u64,
        material: &str,
        part: Option<&str>,
    ) -> Option<operandi_core::shader_eval::Surface> {
        use operandi_core::shader_eval::Context;
        let tree = self.tree.as_ref()?;
        if self.mapped
            && !operandi_core::material::is_default_name(material)
            && !self.stored.contains(material)
        {
            return None;
        }
        let mut ancestors = Vec::new();
        let mut at = self.parents.get(&item);
        while let Some(&id) = at {
            if ancestors.contains(&id) {
                break;
            }
            ancestors.push(id);
            at = self.parents.get(&id);
        }
        let surface = tree.evaluate(&Context {
            item,
            ancestors: &ancestors,
            material,
            part,
            // Provisional: the viewport colour isn't split by selection set
            // yet, so set masks (G-01) show only in renders.
            sets: &[],
            locators: self.frames.get(&item).unwrap_or(&self.locators),
        });
        Some(self.with_graph(tree, surface, item))
    }
    /// A surface with its Node Material drawn from the graph's nodes
    /// (MaterialX stage 5), or as NM-40's stand-in colour when the graph's
    /// surface can't be drawn.
    fn with_graph(
        &self,
        tree: &operandi_core::shader_tree::ShaderTree,
        surface: operandi_core::shader_eval::Surface,
        item: u64,
    ) -> operandi_core::shader_eval::Surface {
        use operandi_core::graph_view;
        let Some(n) = surface.node.clone() else {
            return surface;
        };
        let (exposed, drivers) = graph_view::exposed(&n);
        let key = format!("{}|{:?}|{:?}", n.graph, exposed.values, exposed.drivers);
        let flat = {
            let mut flats = self.graph_flats.0.lock().unwrap_or_else(|e| e.into_inner());
            flats
                .entry(key)
                .or_insert_with(|| {
                    let g = tree.node_graphs.get(n.graph)?;
                    g.flatten(&exposed).ok().map(Arc::new)
                })
                .clone()
        };
        let space = self.graph_spaces.get(&item).cloned().unwrap_or_default();
        let view = flat.and_then(|f| {
            graph_view::surface(&f, &drivers, &space, &mut |path| {
                self.graph_images.get(path).copied()
            })
        });
        match view {
            Some(v) => graph_view::apply(surface, &n.coverage, &v),
            None => node_stand_in(tree, surface),
        }
    }
    /// The Advanced style's principled values for a folded surface (images
    /// as their average), `None` when they are the plain fill's.
    fn principled_of(
        &self,
        surface: &operandi_core::shader_eval::Surface,
        textures: bool,
        plain: bool,
    ) -> Option<operandi_core::blender_principled::Params> {
        use operandi_core::blender_principled::Params;
        use operandi_core::shader_eval::Ch;
        use operandi_core::shader_tree::ShadingModel;
        // The other styles draw Diffuse Color × Amount; the renderer's base
        // colour is Diffuse Color alone, except in Physically Based. A
        // textured Amount can't be taken back out, so it stays in
        // (provisional, VM-Q01). A polygon drawn in the plain fill shows
        // the theme's colour, not the material's, so keeps it as it is.
        let amount = surface.get(Ch::DiffuseAmount).constant();
        let base_scale = match (surface.shading_model, amount) {
            _ if plain => 1.,
            (ShadingModel::PhysicallyBased, _) => 1.,
            (_, Some([a, ..])) if a > 1e-4 => 1. / a,
            _ => 1.,
        };
        let params = Params::of(&surface.principled(), base_scale, &|e| {
            e.eval(&|id, _| self.image_average(id, textures))
        });
        (params != Params::default()).then_some(params)
    }
    /// The colour a value folds to at one point (images as their average),
    /// `None` when it is the plain fill.
    fn colour_of(&self, value: [f64; 3]) -> Looked {
        let plain = plain_base();
        if (0..3).all(|i| (value[i] - plain[i]).abs() < 1e-6) {
            return None;
        }
        let [r, g, b] = value.map(|c| c.clamp(0., 1.) as f32);
        // Linear colour to the gamma-encoded vertex colour.
        Some(Color32::from(eframe::egui::Rgba::from_rgb(r, g, b)))
    }
    /// An image's average (mid grey when unreadable).
    fn image_average(&self, id: u64, textures: bool) -> [f64; 4] {
        match self.images.get(&id).and_then(|i| i.as_ref()?.average) {
            Some(c) if textures => c,
            Some(_) => [0.; 4],
            None => [0.5, 0.5, 0.5, if textures { 1. } else { 0. }],
        }
    }
    /// The material program drawing a polygon context per pixel: where the
    /// winning shader shows textures and the colour reads an image that
    /// the GPU can draw (`shader_wgsl::translate`). `None` keeps the
    /// vertex colour (images as their average).
    fn program(&self, item: u64, material: &str, part: Option<&str>) -> Option<Arc<Instance>> {
        let key = (item, material.to_owned(), part.map(str::to_owned));
        if let Some(found) = self.programs.lock().contexts.get(&key) {
            return found.clone();
        }
        let made = self.make_program(item, material, part);
        let mut table = self.programs.lock();
        let found = made.map(|m| table.add(m));
        // `add` may have filled image slots the translation asked for.
        table.contexts.insert(key, found.clone());
        found
    }
    /// The program drawing a context's colour per pixel (if its colour
    /// reads images) and its Advanced-style principled values (if not the
    /// plain fill's); `None` when it needs neither.
    fn make_program(&self, item: u64, material: &str, part: Option<&str>) -> Option<Made> {
        let surface = self.surface(item, material, part)?;
        let (base, textures) = Self::base_of(&surface);
        let plain = base.constant().is_some_and(|c| self.colour_of(c).is_none());
        let params = self.principled_of(&surface, textures, plain);
        let list = if textures {
            self.list_of(&surface)
        } else {
            None
        };
        // Per pixel when the colour reads an image or the view (an
        // Incidence gradient); otherwise the CPU colour is as good.
        let program = if !textures || base.constant().is_some() {
            None
        } else {
            let table = &self.programs;
            operandi_core::shader_wgsl::translate(&base, &mut |id| table.lock().slot_of(id))
                .filter(|p| p.per_pixel())
        };
        let clear = self.clear_of(&surface, textures);
        let cut = if textures {
            self.cut_share(&surface)
        } else {
            0.
        };
        (program.is_some() || params.is_some() || list.is_some() || clear.is_some() || cut > 0.)
            .then_some(Made {
                program,
                params,
                list,
                clear,
                cut,
            })
    }
    /// How much of a surface its cut-outs remove, on average (images as
    /// their average): Stencil's and an RGBA layer's cut value. Only for
    /// lighter shadows (VM-Q10): the Advanced style cuts per pixel.
    fn cut_share(&self, surface: &operandi_core::shader_eval::Surface) -> f32 {
        use operandi_core::shader_eval::{Ch, Expr};
        let mut keep = 1.;
        for ch in [Ch::Stencil, Ch::RgbaCut] {
            let v = surface.get(ch);
            if *v != Expr::scalar(0.) {
                let cut = v.eval(&|id, _| self.image_average(id, true))[0];
                keep *= 1. - cut.clamp(0., 1.);
            }
        }
        let share = (1. - keep) as f32;
        if share.is_finite() && share > CLEAR_MIN as f32 {
            share
        } else {
            0.
        }
    }
    /// How see-through a folded surface draws in the viewport: (Transparent
    /// Color as a gamma-encoded tint, how much shows through), with Dissolve
    /// folded in as an untinted share; `None` when (next to) opaque. Images
    /// count as their average.
    fn clear_of(
        &self,
        surface: &operandi_core::shader_eval::Surface,
        textures: bool,
    ) -> Option<[f32; 4]> {
        use operandi_core::shader_eval::Ch;
        let value = |ch| {
            surface
                .get(ch)
                .eval(&|id, _| self.image_average(id, textures))
                .map(|c| c.clamp(0., 1.))
        };
        let amount = value(Ch::TransparentAmount)[0];
        let dissolve = value(Ch::Dissolve)[0];
        let through = 1. - (1. - amount) * (1. - dissolve);
        // NaN (a broken input) counts as opaque too.
        if through.is_nan() || through <= CLEAR_MIN {
            return None;
        }
        // What shows through: the dissolved share untinted, the rest
        // through Transparent Color.
        let tint = value(Ch::TransparentColor)
            .map(|c| (dissolve + (1. - dissolve) * amount * c) / through);
        let [r, g, b] = tint.map(|c| c.clamp(0., 1.) as f32);
        let encoded = Color32::from(eframe::egui::Rgba::from_rgb(r, g, b));
        let [r, g, b, _] = encoded.to_array().map(|v| f32::from(v) / 255.);
        Some([r, g, b, through as f32])
    }
    /// The Advanced style's per-pixel values (blender_principled `list`)
    /// for a surface whose principled inputs, bump, normal map or cut-out
    /// read images; `None` when none does or they can't be drawn.
    fn list_of(
        &self,
        surface: &operandi_core::shader_eval::Surface,
    ) -> Option<operandi_core::shader_wgsl::Program> {
        use operandi_core::shader_eval::{Ch, Expr};
        let p = surface.principled();
        // Cut-out effects v1: Stencil above 50 % or an RGBA cut removes the
        // surface (the renderer's alpha without Dissolve and masks).
        let mut keep = Expr::scalar(1.);
        for cut in [Ch::Stencil, Ch::RgbaCut] {
            let v = surface.get(cut).clone();
            if v != Expr::scalar(0.) {
                keep = Expr::times(keep, Expr::one_minus(Expr::step_above(v, 0.5)));
            }
        }
        let cut = (keep != Expr::scalar(1.)).then(|| Expr::one_minus(keep));
        let items = operandi_core::blender_principled::per_pixel(&p, cut.as_ref());
        // Images or the view (an Incidence gradient) vary per pixel;
        // constants are the CPU values already.
        // A displacement always needs the list: the vertex shader reads it.
        if p.displacement.is_none() && items.iter().all(|(e, _)| e.constant().is_some()) {
            return None;
        }
        let table = &self.programs;
        let displaced = p.displacement.is_some();
        operandi_core::shader_wgsl::translate_list(&items, &mut |id| table.lock().slot_of(id))
            .filter(|l| displaced || l.per_pixel())
    }
    /// A patterned surface's colour at one world point.
    fn pattern_colour(
        &self,
        e: &operandi_core::shader_eval::Expr,
        at: &operandi_core::shader_eval::ShadingPoint,
    ) -> Color32 {
        let value = e.eval_at(at, &|l, _| self.image_average(l.image, true));
        let [r, g, b] = value.map(|c| c.clamp(0., 1.) as f32);
        Color32::from(eframe::egui::Rgba::from_rgb(r, g, b))
    }
    /// Folds the tree for one polygon context.
    fn look(&self, item: u64, material: &str, part: Option<&str>) -> Looked {
        let (base, textures) = self.base(item, material, part)?;
        // GL Display off on the winning shader: no textures in the
        // viewport, so image layers cover nothing.
        let value = base.eval(&|id, _filtering| self.image_average(id, textures));
        self.colour_of(value)
    }
    fn flat(&self, tag: &str) -> bool {
        self.all_flat || (!self.flat.is_empty() && self.flat.contains(Self::key(tag)))
    }
}

/// Diffuse Color × Diffuse Amount of an untouched tree: polygons that
/// come out at it keep the plain fill.
fn plain_base() -> [f64; 3] {
    use operandi_core::shader_eval::{Ch, Context, Expr};
    static PLAIN: std::sync::OnceLock<[f64; 3]> = std::sync::OnceLock::new();
    *PLAIN.get_or_init(|| {
        let s = operandi_core::shader_tree::ShaderTree::default().evaluate(&Context {
            item: 0,
            ancestors: &[],
            material: operandi_core::material::DEFAULT_MATERIAL,
            part: None,
            sets: &[],
            locators: &[],
        });
        Expr::times(
            s.get(Ch::DiffuseColor).clone(),
            s.get(Ch::DiffuseAmount).clone(),
        )
        .eval(&|_, _| [0.; 4])
    })
}

/// Largest side of an image texture (the GPU's limit is at least this on
/// every desktop adapter); bigger pictures are scaled down to fit.
pub const MAX_IMAGE_SIDE: u32 = 4096;

/// A picture file read for the viewport: its pixels (sRGB-encoded RGBA,
/// row 0 at the top, scaled to fit `MAX_IMAGE_SIDE`) for the GPU texture,
/// and its average linear colour and alpha for the CPU colour path.
/// Equal when it is the same read (`key`).
pub struct ImageData {
    /// Unique per read: Reload Image makes a new one, which the GPU uploads
    /// in place of the old.
    pub key: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// `None` when the file can't be read (the texture is mid grey).
    pub average: Option<[f64; 4]>,
}
impl PartialEq for ImageData {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl std::fmt::Debug for ImageData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ImageData({} {}x{})", self.key, self.width, self.height)
    }
}
impl ImageData {
    /// Pixels as given, averaged (tests and `load_image`).
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self::with_key(
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            width,
            height,
            rgba,
        )
    }
    /// `new` keeping an earlier picture's key: the card already shows
    /// these pixels under it (a finished stroke, written in as it went).
    pub fn with_key(key: u64, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        // Linear light per 8-bit value, once (a 2048 picture has 4M pixels).
        static LINEAR: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
        let linear = LINEAR.get_or_init(|| {
            std::array::from_fn(|v| eframe::egui::ecolor::linear_f32_from_gamma_u8(v as u8))
        });
        let n = (rgba.len() / 4).max(1) as f64;
        let mut sum = [0f64; 4];
        for px in rgba.as_chunks::<4>().0 {
            for c in 0..3 {
                sum[c] += f64::from(linear[px[c] as usize]);
            }
            sum[3] += f64::from(px[3]) / 255.;
        }
        Self {
            key,
            width,
            height,
            rgba,
            average: Some(sum.map(|v| v / n)),
        }
    }
    /// A file that can't be read: one mid-grey pixel (linear 0.5, opaque),
    /// as the CPU path shows it.
    pub fn unreadable() -> Self {
        let grey = eframe::egui::ecolor::gamma_u8_from_linear_f32(0.5);
        Self {
            average: None,
            ..Self::new(1, 1, vec![grey, grey, grey, 255])
        }
    }
}

/// Read a picture file for the viewport (`ImageData`).
pub fn load_image(path: &std::path::Path) -> ImageData {
    let Ok((pixels, _)) = crate::backdrop_ui::decode(path) else {
        return ImageData::unreadable();
    };
    fit_image(pixels.width, pixels.height, pixels.rgba)
}

/// `ImageData` for 8-bit sRGB pixels, shrunk to the viewport's largest side.
fn fit_image(w: u32, h: u32, rgba: Vec<u8>) -> ImageData {
    let (mut w, mut h, mut rgba) = (w, h, rgba);
    if w.max(h) > MAX_IMAGE_SIDE
        && let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone())
    {
        let k = f64::from(MAX_IMAGE_SIDE) / f64::from(w.max(h));
        let (nw, nh) = (
            ((f64::from(w) * k).round() as u32).clamp(1, MAX_IMAGE_SIDE),
            ((f64::from(h) * k).round() as u32).clamp(1, MAX_IMAGE_SIDE),
        );
        rgba =
            image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle).into_raw();
        (w, h) = (nw, nh);
    }
    ImageData::new(w, h, rgba)
}

/// The viewport cache's key for a painted image's current pixels: a new
/// buffer (every edit makes one) is a new key.
#[cfg(feature = "painting")]
fn painted_key(id: u64, px: &Arc<operandi_core::paint::pixels::PaintImage>) -> String {
    format!("\u{0}paint:{id}:{:p}", Arc::as_ptr(px))
}

#[cfg(feature = "painting")]
fn painted_key_in_use(key: &str, scene: &operandi_core::Scene) -> bool {
    scene
        .paint
        .buffers
        .iter()
        .any(|(id, px)| painted_key(*id, px) == key)
        || scene.images.iter().any(|i| {
            picked_psd_layer(scene, i.id)
                .is_some_and(|(k, doc)| psd_layer_key(i.id, k, &doc) == key)
        })
}

/// The Photoshop layer the image's Image Map layers show (stage 6): the
/// first that picks one; the viewport draws it instead of the composite.
#[cfg(feature = "painting")]
fn picked_psd_layer(
    scene: &operandi_core::Scene,
    image: u64,
) -> Option<(usize, Arc<operandi_core::paint::pixels::psd::PsdDoc>)> {
    use operandi_core::paint;
    let doc = scene.paint.psd.get(&image)?;
    let name = scene
        .shader_tree
        .nodes
        .iter()
        .filter(|n| paint::layer_image(scene, n.id) == Some(image))
        .find_map(|n| paint::psd_layer_of(scene, n.id))?;
    Some((doc.layer_named(&name)?, doc.clone()))
}

#[cfg(feature = "painting")]
fn psd_layer_key(
    id: u64,
    index: usize,
    doc: &Arc<operandi_core::paint::pixels::psd::PsdDoc>,
) -> String {
    format!("\u{0}psd:{id}:{index}:{:p}", Arc::as_ptr(doc))
}
#[cfg(not(feature = "painting"))]
fn painted_key_in_use(_: &str, _: &operandi_core::Scene) -> bool {
    false
}

#[cfg(feature = "painting")]
fn paint_unchanged(last: &LookInputs, scene: &operandi_core::Scene) -> bool {
    last.paint == scene.paint
}
#[cfg(not(feature = "painting"))]
fn paint_unchanged(_: &LookInputs, _: &operandi_core::Scene) -> bool {
    true
}

/// A painted image's pixels for the viewport: 8- and 16-bit samples are
/// taken as stored (sRGB, as image files are), floats as linear.
#[cfg(feature = "painting")]
pub fn painted_image(px: &operandi_core::paint::pixels::PaintImage) -> ImageData {
    let rgba = painted_rgba(px, (0, 0, px.width - 1, px.height - 1));
    fit_image(px.width, px.height, rgba)
}

/// The viewport's 8-bit sRGB pixels for rectangle `rect` (x0, y0, x1, y1,
/// inclusive) of a painted image, row by row (`painted_image`'s rule).
#[cfg(feature = "painting")]
pub fn painted_rgba(
    px: &operandi_core::paint::pixels::PaintImage,
    rect: (u32, u32, u32, u32),
) -> Vec<u8> {
    use operandi_core::paint::pixels::{Format, Pixels};
    let (x0, y0, x1, y1) = rect;
    let (rw, rh) = ((x1 - x0 + 1) as usize, (y1 - y0 + 1) as usize);
    let w = px.width as usize;
    let mut rgba = Vec::with_capacity(rw * rh * 4);
    // 8-bit RGBA is stored as shown: rows copy straight across.
    if let (Format::Rgba, Pixels::U8(v)) = (px.format, &px.pixels) {
        for y in y0 as usize..=y1 as usize {
            let at = (y * w + x0 as usize) * 4;
            rgba.extend_from_slice(&v[at..at + rw * 4]);
        }
        return rgba;
    }
    let float = px.depth.is_float();
    for y in y0 as usize..=y1 as usize {
        for x in x0 as usize..=x1 as usize {
            let p = px.get(y * w + x);
            for (k, v) in p.iter().enumerate() {
                rgba.push(if float && k < 3 {
                    eframe::egui::ecolor::gamma_u8_from_linear_f32(*v)
                } else {
                    (v.clamp(0., 1.) * 255.).round() as u8
                });
            }
        }
    }
    rgba
}

/// Below this Transparent Amount (half a step of an 8-bit colour) a
/// surface draws solid.
pub const CLEAR_MIN: f64 = 0.5 / 255.;

/// One polygon context's material program: its instance number (the
/// `program` vertex attribute) and what the vertices must carry for it.
#[derive(Debug)]
pub struct Instance {
    pub id: u32,
    pub uv_map: Option<String>,
    pub slot: Option<operandi_core::shader_eval::Expr>,
    /// Whether a program draws the colour (else only the Advanced style's
    /// principled values differ, and the vertex colour is the colour).
    pub coloured: bool,
    /// The surface is see-through (`MaterialLooks::clear_of`).
    pub clear: bool,
    /// The surface has cut-outs (`MaterialLooks::cut_share`).
    pub cut: bool,
}

/// The material programs of one set of looks (see `MaterialLooks`).
pub struct ProgramTable {
    /// Unique per table, so a snapshot is never mistaken for another's.
    id: u64,
    inner: std::sync::Mutex<Programs>,
}
impl Default for ProgramTable {
    fn default() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            inner: Default::default(),
        }
    }
}
impl std::fmt::Debug for ProgramTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProgramTable")
    }
}
impl PartialEq for ProgramTable {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl ProgramTable {
    fn lock(&self) -> std::sync::MutexGuard<'_, Programs> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
type ContextKey = (u64, String, Option<String>);
/// What one polygon context draws with (`MaterialLooks::make_program`):
/// the colour program, the Advanced style's principled values and its
/// per-pixel list.
struct Made {
    program: Option<operandi_core::shader_wgsl::Program>,
    params: Option<operandi_core::blender_principled::Params>,
    list: Option<operandi_core::shader_wgsl::Program>,
    /// See-through: (tint, how much shows through).
    clear: Option<[f32; 4]>,
    /// The average share its cut-outs remove (shadows only).
    cut: f32,
}
/// An opaque instance's `Programs::clear` row.
pub const OPAQUE: [f32; 4] = [1., 1., 1., 0.];
#[derive(Default)]
struct Programs {
    contexts: HashMap<ContextKey, Option<Arc<Instance>>>,
    /// Distinct generated functions; code k + 1 is `bodies[k]`.
    bodies: Vec<String>,
    /// Distinct list functions (`shader_wgsl::translate_list`), the same
    /// way.
    lists: Vec<String>,
    /// Per instance (index 0 unused): its code (0: the vertex colour), its
    /// first parameter row, its principled values' first row, and bit 0
    /// whether it has them (else the plain fill's) with its list's code
    /// above (its parameters follow the principled rows).
    instances: Vec<[u32; 4]>,
    /// Per instance, its see-through tint and amount (`OPAQUE` for none),
    /// read by the fill shaders' `op_clear`.
    clear: Vec<[f32; 4]>,
    /// Per instance, the share of its shadow that light gets through
    /// (`w`): the see-through amount or the cut-outs' share, the larger.
    shadow: Vec<[f32; 4]>,
    params: Vec<[f32; 4]>,
    /// Texture slot → image id.
    images: Vec<u64>,
    /// Bumped on every change, so the GPU side rewrites its buffers.
    version: u64,
}
impl Programs {
    fn slot_of(&mut self, id: u64) -> Option<u32> {
        if let Some(k) = self.images.iter().position(|&i| i == id) {
            return Some(k as u32);
        }
        if self.images.len() >= operandi_core::shader_wgsl::MAX_IMAGES {
            return None;
        }
        self.images.push(id);
        self.version += 1;
        Some(self.images.len() as u32 - 1)
    }
    fn add(&mut self, made: Made) -> Arc<Instance> {
        let Made {
            program: p,
            params: surface,
            list,
            clear,
            cut,
        } = made;
        if self.instances.is_empty() {
            self.instances.push([0; 4]);
            self.clear.push(OPAQUE);
            self.shadow.push(OPAQUE);
        }
        let mut entry = [0u32; 4];
        let surface = surface.or(list.as_ref().map(|_| Default::default()));
        if let Some(rows) = surface.map(|s: operandi_core::blender_principled::Params| s.rows()) {
            entry[2] = self.params.len() as u32;
            entry[3] = 1;
            self.params.extend(rows);
        }
        let mut list_uv = None;
        if let Some(l) = list {
            let code = match self.lists.iter().position(|b| *b == l.body) {
                Some(k) => k + 1,
                None => {
                    self.lists.push(l.body);
                    self.lists.len()
                }
            };
            entry[3] |= (code as u32) << 1;
            // Right after the principled rows (`op_list`'s base).
            self.params.extend(l.params);
            list_uv = Some(l.uv_map);
        }
        let (uv_map, slot, coloured) = match p {
            Some(p) => {
                let code = match self.bodies.iter().position(|b| *b == p.body) {
                    Some(k) => k + 1,
                    None => {
                        self.bodies.push(p.body);
                        self.bodies.len()
                    }
                };
                entry[0] = code as u32;
                entry[1] = self.params.len() as u32;
                self.params.extend(p.params);
                (p.uv_map, p.slot, true)
            }
            // Without a colour program the vertices carry the UVs the
            // list reads.
            None => (list_uv.flatten(), None, false),
        };
        self.instances.push(entry);
        self.clear.push(clear.unwrap_or(OPAQUE));
        let through = clear.map_or(0., |c| c[3]).max(cut);
        self.shadow.push([1., 1., 1., through]);
        self.version += 1;
        Arc::new(Instance {
            id: self.instances.len() as u32 - 1,
            uv_map,
            slot,
            coloured,
            clear: clear.is_some(),
            cut: cut > 0.,
        })
    }
}

/// What the fill shader needs from the material programs: the generated
/// WGSL (`shader_wgsl::module`), the instance table, the parameters and the
/// image in each texture slot.
#[derive(Debug)]
pub struct GpuMaterials {
    /// Changes whenever anything below does.
    pub generation: u64,
    pub source: String,
    pub instances: Vec<[u32; 4]>,
    pub params: Vec<[f32; 4]>,
    pub images: Vec<Arc<ImageData>>,
    /// Per instance, see-through tint and amount (`Programs::clear`).
    pub clear: Vec<[f32; 4]>,
    /// Per instance, how much light its shadow lets through
    /// (`Programs::shadow`).
    pub shadow: Vec<[f32; 4]>,
    /// The painted images among `images`: (image id, picture key). The
    /// card keeps their mip levels so a stroke rewrites only its rectangle.
    pub painted: Vec<(u64, u64)>,
}

/// The materials' inputs `sync_materials` last folded, compared each
/// frame so the tree is folded again only when one changes.
#[derive(Clone, Default, PartialEq)]
struct LookInputs {
    tree: operandi_core::shader_tree::ShaderTree,
    materials: std::collections::BTreeMap<String, operandi_core::Material>,
    parents: Vec<(u64, Option<u64>)>,
    images: Vec<operandi_core::backdrop::SceneImage>,
    /// Painted images' pixels (compared by buffer, not by pixel).
    #[cfg(feature = "painting")]
    paint: operandi_core::paint::PaintImages,
    locators: Vec<(
        u64,
        operandi_core::texture_locator::TextureLocator,
        operandi_core::scene::Transform,
    )>,
}

/// Every Texture Locator's settings, by item id.
/// Every Texture Locator's settings and placement, by item id.
fn locator_settings(
    scene: &operandi_core::Scene,
) -> Vec<(
    u64,
    operandi_core::texture_locator::TextureLocator,
    operandi_core::scene::Transform,
)> {
    scene
        .items
        .iter()
        .filter_map(|i| Some((i.id, i.texture_locator()?.clone(), i.transform.clone())))
        .collect()
}

/// The material looks an item's geometry was built with. Equal when it is
/// the same set (`Resident` swaps it only when the looks change), so the
/// per-item key check doesn't compare whole shader trees every frame.
#[derive(Clone, Default)]
struct LooksRef(Arc<MaterialLooks>);
impl PartialEq for LooksRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl std::ops::Deref for LooksRef {
    type Target = MaterialLooks;
    fn deref(&self) -> &MaterialLooks {
        &self.0
    }
}

struct Entry {
    key: Key,
    /// The placement the geometry was built at (see `delta`).
    matrix: DMat4,
    geometry: Arc<ItemGeometry>,
    generation: u64,
}

/// Where cached geometry built at placement `built` must move to sit at
/// `now`: drawn through this matrix on the graphics card (gpu.rs), so a
/// whole-item Move, Rotate or Scale drag reuses the geometry instead of
/// rebuilding it every frame. `None` means rebuild: outside a live drag
/// (the settled geometry is exact again), and for a mirroring change,
/// whose triangles must be wound the other way.
fn delta(now: DMat4, built: DMat4, live: bool) -> Option<glam::Mat4> {
    if now.to_cols_array() == built.to_cols_array() {
        return Some(glam::Mat4::IDENTITY);
    }
    if !live || built.determinant().abs() < 1e-12 {
        return None;
    }
    let d = now * built.inverse();
    (d.is_finite() && d.determinant() > 0.).then(|| d.as_mat4())
}

/// What an item's point dots depend on.
#[derive(Clone, PartialEq)]
struct DotKey {
    mesh: MeshRef,
    /// Vertex mode shows the selected points and points no polygon uses,
    /// other modes only the latter; `all` (the view's Show Verts rules,
    /// view_display.rs) shows every point.
    vertex_mode: bool,
    all: bool,
    selected: Vec<u32>,
    mirror: BTreeSet<u32>,
    palette: u64,
    /// Plain and selected point radius (Selection preferences).
    radii: [u32; 2],
    /// Locked and hidden points (LCK-23, HID-03), and whether locked
    /// points show.
    elements: operandi_core::ElementState,
    locked: bool,
    /// The item's subdivision settings when its points show on a smooth
    /// surface (where depends on the level as well as the mesh).
    smooth: Option<operandi_core::subdivision::SubdivisionSettings>,
    /// Topology Mode's point look, when on (missing-tools-g3 MT7).
    topology: Option<TopologyDots>,
}

/// Topology Mode's points (MT7-02/03): plain points in Topo Vertices; with
/// Hide back-facing topology points on, points whose polygons all face
/// away from the camera don't draw. The camera is part of the key, so the
/// dots rebuild as the view turns (the preference warns it costs speed).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TopologyDots {
    pub color: Color32,
    /// The eye (perspective) or the view direction (orthographic), when
    /// back-facing points hide.
    pub hide_back: Option<(DVec3, bool)>,
}

/// What an item's UV seam strokes depend on.
#[derive(Clone, PartialEq)]
struct SeamKey {
    mesh: MeshRef,
    palette: u64,
}

struct SeamEntry {
    key: SeamKey,
    matrix: DMat4,
    seams: Arc<Vec<EdgeVertex>>,
    generation: u64,
}

struct DotEntry {
    key: DotKey,
    matrix: DMat4,
    dots: Arc<Vec<DotVertex>>,
    /// The points drawn, in drawing order (`dots_still_match`).
    drawn: Vec<u32>,
    generation: u64,
}

/// A mesh change the cached dots don't see: the same polygons, point count
/// and attributes, and none of the drawn points moved. Most component drags
/// in Polygon or Edge mode draw no dots at all (only points no polygon uses),
/// so their dots needn't be rebuilt every frame.
fn dots_still_match(entry: &DotEntry, key: &DotKey) -> bool {
    let (old, new) = (&*entry.key.mesh, &*key.mesh);
    let mut same_inputs = entry.key.clone();
    same_inputs.mesh = key.mesh.clone();
    same_inputs == *key
        // On a smooth surface a drawn point moves with its neighbours.
        && (key.smooth.is_none() || entry.drawn.is_empty())
        && operandi_core::Shared::same(&old.faces, &new.faces)
        && old.vertices.len() == new.vertices.len()
        && old.attributes == new.attributes
        && entry
            .drawn
            .iter()
            .all(|&v| old.vertices[v as usize] == new.vertices[v as usize])
}

/// The CPU side: geometry per item, with a generation that changes whenever
/// it is rebuilt (the GPU side uploads on a new generation).
#[derive(Default)]
pub struct Resident {
    items: operandi_core::fxhash::FxHashMap<u64, Entry>,
    dots: operandi_core::fxhash::FxHashMap<u64, DotEntry>,
    seams: operandi_core::fxhash::FxHashMap<u64, SeamEntry>,
    next_generation: u64,
    /// Full geometry builds (patches and point dots don't count).
    full_builds: u64,
    /// Patches of moved polygons (speed guards).
    patches: u64,
    /// Cage polygons rewritten by those patches (Window > Frame Time).
    patched_polygons: u64,
    /// Smooth pieces those patches rewrote, and the piece normals they
    /// computed for them (speed guards: each normal once, not per corner).
    patched_pieces: u64,
    patch_piece_normals: u64,
    /// The current scene's material looks (`set_materials`).
    materials: LooksRef,
    /// What they were folded from (`sync_materials`).
    look_inputs: Option<LookInputs>,
    /// Image files read so far, by path (`load_image`).
    image_files: HashMap<String, Arc<ImageData>>,
    /// How many times an image file was read (speed guards).
    image_reads: u64,
    /// Painted images' picture keys (`ImageData::key`) as last folded, by
    /// image id: their textures take a stroke's changed rectangles.
    painted_images: std::collections::BTreeMap<u64, u64>,
    /// The last `gpu_materials` snapshot: from which looks and table
    /// version.
    gpu_materials: Option<(u64, u64, Arc<GpuMaterials>)>,
    gpu_generation: u64,
}

/// The vertex map the Vertex Map style shows: the first color map, else
/// the first weight map (provisional: Operandi has no "selected map" yet).
enum VertexMap<'a> {
    None,
    Colors(&'a [Vec<Option<glam::DVec4>>]),
    Weights(&'a [Vec<Option<f64>>]),
    Typed(&'a operandi_core::TypedMap, bool),
}
impl<'a> VertexMap<'a> {
    fn of(mesh: &'a Mesh) -> Self {
        let a = &mesh.attributes;
        let typed = |color: bool| {
            a.typed_maps.values().find(|m| {
                let kind = m.kind.to_ascii_lowercase();
                if color {
                    (kind.contains("rgb") || kind.contains("color")) && m.dimension >= 3
                } else {
                    (kind.contains("weight") || kind.contains("wght")) && m.dimension >= 1
                }
            })
        };
        if let Some(map) = a.corner_colors.values().next() {
            VertexMap::Colors(map)
        } else if let Some(map) = typed(true) {
            VertexMap::Typed(map, true)
        } else if let Some(map) = a.corner_weights.values().next() {
            VertexMap::Weights(map)
        } else if let Some(map) = typed(false) {
            VertexMap::Typed(map, false)
        } else {
            VertexMap::None
        }
    }
    /// Corner `c` of polygon `f`: a color map's color; a weight map's
    /// weight as red (positive), green (zero or unassigned) or blue
    /// (negative); neutral grey without a map.
    fn color(&self, mesh: &Mesh, f: usize, c: usize) -> Color32 {
        fn corner<T: Copy>(rows: &[Vec<Option<T>>], f: usize, c: usize) -> Option<T> {
            rows.get(f).and_then(|r| r.get(c).copied().flatten())
        }
        match self {
            VertexMap::None => palette::vertex_map_none(),
            VertexMap::Colors(rows) => corner(rows, f, c)
                .map(|v| rgb(v.x, v.y, v.z))
                .unwrap_or(palette::vertex_map_none()),
            VertexMap::Weights(rows) => weight_color(corner(rows, f, c).unwrap_or(0.)),
            VertexMap::Typed(map, true) => map
                .effective(mesh, f, c)
                .filter(|v| v.len() >= 3)
                .map(|v| rgb(v[0], v[1], v[2]))
                .unwrap_or(palette::vertex_map_none()),
            VertexMap::Typed(map, false) => weight_color(
                map.effective(mesh, f, c)
                    .and_then(|v| v.first().copied())
                    .unwrap_or(0.),
            ),
        }
    }
}
fn rgb(r: f64, g: f64, b: f64) -> Color32 {
    let c = |x: f64| (x.clamp(0., 1.) * 255.).round() as u8;
    Color32::from_rgb(c(r), c(g), c(b))
}
/// Weight shading: zero green, toward red at +1 and blue at −1.
pub fn weight_color(w: f64) -> Color32 {
    let t = w.abs().clamp(0., 1.);
    let zero = [0.25, 0.7, 0.25];
    let end = if w >= 0. {
        [0.9, 0.15, 0.1]
    } else {
        [0.15, 0.3, 0.95]
    };
    rgb(
        zero[0] + (end[0] - zero[0]) * t,
        zero[1] + (end[1] - zero[1]) * t,
        zero[2] + (end[2] - zero[2]) * t,
    )
}

/// `newell` over `n` points read through `at`, without collecting them
/// (a drag computes thousands of these per frame).
fn newell_by(n: usize, at: impl Fn(usize) -> DVec3) -> DVec3 {
    let p0 = at(0);
    (0..n)
        .map(|i| (at(i) - p0).cross(at((i + 1) % n) - p0))
        .sum::<DVec3>()
        .normalize_or_zero()
}

/// Triangles of a polygon as corner indices; a fan for shapes the ear
/// clipper refuses, like the picking surface.
fn corners(mesh: &Mesh, face: usize) -> Vec<[usize; 3]> {
    mesh.surface_triangles(face).unwrap_or_else(|_| {
        (1..mesh.faces[face].vertices.len().saturating_sub(1))
            .map(|i| [0, i, i + 1])
            .collect()
    })
}

impl Resident {
    /// The item's geometry, rebuilt only when its key changed. `selection`
    /// is (whole item selected, selected polygons, mirrored polygons) for
    /// Material colors for the following `item` calls; set once a frame.
    pub fn set_materials(&mut self, looks: MaterialLooks) {
        if *self.materials != looks {
            self.materials = LooksRef(Arc::new(looks));
        }
    }
    /// Reload Image (shader layer menu §7): read `path` again next fold.
    pub fn reload_image(&mut self, path: &str) {
        self.image_files.remove(path);
        self.look_inputs = None;
    }
    /// Image files read so far (speed guards: a drag reads none).
    #[cfg(test)]
    pub fn image_reads(&self) -> u64 {
        self.image_reads
    }
    /// The picture key of painted image `image` as the card has it, for
    /// writing a stroke's changed rectangle into it.
    #[cfg(feature = "painting")]
    pub fn painted_texture(&self, image: u64) -> Option<u64> {
        let (_, _, m) = self.gpu_materials.as_ref()?;
        m.painted
            .iter()
            .find(|(id, _)| *id == image)
            .map(|(_, k)| *k)
    }
    /// A finished stroke's pixels `new` replace `old` for image `image`,
    /// and the card already shows them (its rectangles were written in as
    /// the stroke went): the picture keeps its key, so nothing is uploaded
    /// again. `old`'s picture is forgotten, so an undo back to it uploads
    /// afresh.
    #[cfg(feature = "painting")]
    pub fn adopt_painted(
        &mut self,
        image: u64,
        old: &Arc<operandi_core::paint::pixels::PaintImage>,
        new: &Arc<operandi_core::paint::pixels::PaintImage>,
    ) {
        let Some(was) = self.image_files.remove(&painted_key(image, old)) else {
            return;
        };
        if (was.width, was.height) != (new.width, new.height) {
            return;
        }
        let rgba = painted_rgba(new, (0, 0, new.width - 1, new.height - 1));
        self.image_files.insert(
            painted_key(image, new),
            Arc::new(ImageData::with_key(was.key, new.width, new.height, rgba)),
        );
    }
    /// The current material snapshot's generation (speed guards: a drag
    /// makes no new one, so nothing is compiled or uploaded).
    #[cfg(test)]
    pub fn material_generation(&self) -> u64 {
        self.gpu_materials
            .as_ref()
            .map_or(0, |(_, _, m)| m.generation)
    }
    /// The material programs the current looks' polygons asked for so far,
    /// for the fill shader; `None` when no polygon reads an image. Call after
    /// this frame's `item_placed` calls.
    pub fn gpu_materials(&mut self) -> Option<Arc<GpuMaterials>> {
        let looks = &self.materials;
        let ptr = looks.programs.id;
        let table = looks.programs.lock();
        if table.instances.len() <= 1 {
            return None;
        }
        if let Some((p, v, m)) = &self.gpu_materials
            && *p == ptr
            && *v == table.version
        {
            return Some(m.clone());
        }
        self.gpu_generation += 1;
        let images = table
            .images
            .iter()
            .map(|id| {
                looks
                    .images
                    .get(id)
                    .cloned()
                    .flatten()
                    .unwrap_or_else(|| Arc::new(ImageData::unreadable()))
            })
            .collect();
        let m = Arc::new(GpuMaterials {
            generation: self.gpu_generation,
            source: operandi_core::shader_wgsl::module_with_lists(&table.bodies, &table.lists),
            instances: table.instances.clone(),
            clear: table.clear.clone(),
            shadow: table.shadow.clone(),
            params: table.params.clone(),
            painted: table
                .images
                .iter()
                .filter_map(|id| Some((*id, *self.painted_images.get(id)?)))
                .collect(),
            images,
        });
        let version = table.version;
        drop(table);
        self.gpu_materials = Some((ptr, version, m.clone()));
        Some(m)
    }
    /// `set_materials` for the viewport: folds `scene`'s shader tree again
    /// only when the tree, the stored materials, the item hierarchy or the
    /// images changed, reading each image file once. `all_flat` shades every
    /// polygon flat (Defer Smoothing During Interaction).
    pub fn sync_materials(&mut self, scene: &operandi_core::Scene, all_flat: bool) {
        if let Some(last) = &self.look_inputs
            && last.tree == scene.shader_tree
            && last.materials == scene.materials
            && last.images == scene.images
            && paint_unchanged(last, scene)
            && (last.locators.is_empty() && scene.texture_group().is_none()
                || last.locators == locator_settings(scene))
            && last.parents.len() == scene.items.len()
            && last
                .parents
                .iter()
                .zip(&scene.items)
                .all(|(&(id, parent), i)| id == i.id && parent == i.parent)
        {
            if self.materials.all_flat != all_flat {
                let mut looks = (*self.materials.0).clone();
                looks.all_flat = all_flat;
                self.set_materials(looks);
            }
            return;
        }
        let inputs = LookInputs {
            tree: scene.shader_tree.clone(),
            materials: scene.materials.clone(),
            parents: scene.items.iter().map(|i| (i.id, i.parent)).collect(),
            images: scene.images.clone(),
            #[cfg(feature = "painting")]
            paint: scene.paint.clone(),
            locators: locator_settings(scene),
        };
        let files = &mut self.image_files;
        let reads = &mut self.image_reads;
        let painted_ids = &mut self.painted_images;
        painted_ids.clear();
        // Forget files no image uses any more (their textures go too).
        let graph_pictures = operandi_core::graph_view::image_paths(&scene.shader_tree);
        files.retain(|path, _| {
            scene.images.iter().any(|i| i.path == *path)
                || graph_pictures.contains(path)
                || painted_key_in_use(path, scene)
        });
        let looks = MaterialLooks::with_images(scene, |image| {
            // A Photoshop image whose map picks a layer shows that layer.
            #[cfg(feature = "painting")]
            if let Some((index, doc)) = picked_psd_layer(scene, image.id) {
                let key = psd_layer_key(image.id, index, &doc);
                if let Some(data) = files.get(&key) {
                    return Some(data.clone());
                }
                if let Ok((px, _)) = (*doc)
                    .clone()
                    .picture(operandi_core::paint::pixels::psd::Pick::Layer(index))
                {
                    let data = Arc::new(painted_image(&px));
                    files.insert(key, data.clone());
                    return Some(data);
                }
            }
            // A painted image shows its pixels, saved or not.
            #[cfg(feature = "painting")]
            if let Some(px) = scene.paint.buffers.get(&image.id) {
                let key = painted_key(image.id, px);
                let data = files
                    .entry(key)
                    .or_insert_with(|| Arc::new(painted_image(px)));
                // Shrunk pictures (over MAX_IMAGE_SIDE) aren't patched.
                if (data.width, data.height) == (px.width, px.height) {
                    painted_ids.insert(image.id, data.key);
                }
                return Some(data.clone());
            }
            let data = files.entry(image.path.clone()).or_insert_with(|| {
                *reads += 1;
                Arc::new(load_image(std::path::Path::new(&image.path)))
            });
            Some(data.clone())
        });
        self.set_materials(MaterialLooks { all_flat, ..looks });
        self.look_inputs = Some(inputs);
    }

    /// the active item, `None` for others. `smooth` is its evaluated
    /// subdivision surface, when it has smooth polygons. Tests use this;
    /// the viewport uses `item_placed`.
    #[cfg(test)]
    pub fn item(
        &mut self,
        item: &Item,
        matrix: DMat4,
        selection: Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
        smooth: Option<&crate::subdivision_view::Evaluated>,
        smoothing: &SmoothingSettings,
    ) -> (Arc<ItemGeometry>, u64, Option<Patch>) {
        let (geometry, generation, patch, _) =
            self.item_placed(item, matrix, selection, smooth, smoothing, false);
        (geometry, generation, patch)
    }

    /// `item`, plus the matrix to draw the geometry through (`delta`).
    /// While `live` (a drag is under way), a placement change alone reuses
    /// the geometry built at the old placement. Tests use this; the
    /// viewport uses `item_patched`.
    #[cfg(test)]
    pub fn item_placed(
        &mut self,
        item: &Item,
        matrix: DMat4,
        selection: Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
        smooth: Option<&crate::subdivision_view::Evaluated>,
        smoothing: &SmoothingSettings,
        live: bool,
    ) -> (Arc<ItemGeometry>, u64, Option<Patch>, glam::Mat4) {
        self.item_patched(item, matrix, selection, smooth, None, smoothing, live)
    }

    /// `item_placed`, with the item's Subdivision polygons drawn as
    /// `patches` (their CPU surface `smooth` is then `None`).
    // One argument per input the geometry depends on, as `build` takes them.
    #[allow(clippy::too_many_arguments)]
    pub fn item_patched(
        &mut self,
        item: &Item,
        matrix: DMat4,
        selection: Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
        smooth: Option<&crate::subdivision_view::Evaluated>,
        patches: Option<&Arc<operandi_core::subdivision::patches::PatchNet>>,
        smoothing: &SmoothingSettings,
        live: bool,
    ) -> (Arc<ItemGeometry>, u64, Option<Patch>, glam::Mat4) {
        let key = Key {
            smoothing: smoothing.clone(),
            mesh: MeshRef(item.mesh.shared().clone()),
            selected: selection,
            subdivision: (smooth.is_some() || patches.is_some())
                .then(|| item.subdivision.clone().unwrap_or_default()),
            palette: crate::theme::generation(),
            materials: self.materials.clone(),
            elements: item.elements.clone(),
            show_locked: show_locked(),
            patched: patches.is_some(),
            tints: tints(),
        };
        if let Some(entry) = self.items.get(&item.id)
            && entry.key == key
            && let Some(d) = delta(matrix, entry.matrix, live)
        {
            return (entry.geometry.clone(), entry.generation, None, d);
        }
        // Points moved and/or the polygon selection changed, nothing else:
        // rewrite only the polygons near the moved points and those whose
        // selection state changed (a paint-select drag adds one at a time).
        // Patches: moved points come with the updated net (its changed
        // chunks go up); a selection change rewrites the changed polygons'
        // looks.
        if let Some(entry) = self.items.get_mut(&item.id)
            && entry.key.patched == key.patched
            && entry.matrix.to_cols_array() == matrix.to_cols_array()
            && entry.key.subdivision == key.subdivision
            && entry.key.smoothing == key.smoothing
            && entry.key.palette == key.palette
            && entry.key.materials == key.materials
            && entry.key.elements == key.elements
            && entry.key.show_locked == key.show_locked
            && entry.key.tints == key.tints
            // Weight edits don't move points: the weight tint rebuilds.
            && key.tints.weights.as_ref().is_none_or(|name| {
                entry.key.mesh.attributes.point_weights.get(name)
                    == item.mesh.attributes.point_weights.get(name)
            })
            // UV distortion is against the whole map's ratio, so any mesh
            // change can recolour every polygon: rebuild.
            && (!key.tints.distortion || entry.key.mesh == key.mesh)
            && let Some(mut dirty) =
                operandi_core::subdivision::dirty_faces(&entry.key.mesh, &item.mesh)
            && let Some(uv_dirty) = uv_dirty_faces(&entry.key.mesh, &item.mesh)
            && let Some(changed) =
                selection_changes(&entry.key.selected, &key.selected, item.mesh.faces.len())
        {
            if !changed.is_empty() || !uv_dirty.is_empty() {
                dirty.extend(uv_dirty);
                dirty.extend(changed);
                dirty.sort_unstable();
                dirty.dedup();
            }
            // A subdivision surface's smooth normals reach one polygon
            // further: pieces next to the moved ones share their points.
            if smooth.is_some() && !dirty.is_empty() {
                let around = item.mesh.point_faces();
                let mut wider = dirty.clone();
                for &f in &dirty {
                    for &v in &item.mesh.faces[f].vertices {
                        wider.extend(around.of(v).iter().map(|&g| g as usize));
                    }
                }
                wider.sort_unstable();
                wider.dedup();
                dirty = wider;
            }
            let from = entry.generation;
            let same_selection = entry.key.selected == key.selected;
            let geometry = Arc::make_mut(&mut entry.geometry);
            if let Some(work) = patch(
                geometry,
                item,
                matrix,
                &key.selected,
                smooth,
                patches,
                smoothing,
                &key.materials,
                &dirty,
                same_selection,
            ) {
                self.patched_pieces += work.pieces;
                self.patch_piece_normals += work.piece_normals;
                let ranges = Patch {
                    from,
                    fill: dirty
                        .iter()
                        .map(|&f| geometry.fill_ranges[f].clone())
                        .collect(),
                    edges: dirty
                        .iter()
                        .map(|&f| geometry.edge_ranges[f].clone())
                        .collect(),
                    highlight: work.highlight,
                };
                self.next_generation += 1;
                self.patches += 1;
                self.patched_polygons += dirty.len() as u64;
                entry.generation = self.next_generation;
                entry.key = key;
                return (
                    entry.geometry.clone(),
                    entry.generation,
                    Some(ranges),
                    glam::Mat4::IDENTITY,
                );
            }
        }
        let geometry = Arc::new(build(
            item,
            matrix,
            &key.selected,
            smooth,
            patches,
            smoothing,
            &key.materials,
        ));
        self.full_builds += 1;
        self.next_generation += 1;
        let generation = self.next_generation;
        self.items.insert(
            item.id,
            Entry {
                key,
                matrix,
                geometry: geometry.clone(),
                generation,
            },
        );
        (geometry, generation, None, glam::Mat4::IDENTITY)
    }

    /// How many times geometry has been fully built (speed guards and
    /// Window > Frame Time).
    pub fn builds(&self) -> u64 {
        self.full_builds
    }

    /// Triangles in `item`'s cached fill: what the views draw for it
    /// (subdivision included). `None` when the item isn't held here, or the
    /// fill was built for another mesh, subdivision level or hidden
    /// geometry (a step the views haven't drawn yet: the memory watch
    /// can read before they do). Constant time: a pointer and a
    /// few small compares, and the fill is a plain triangle list.
    pub fn drawn_triangles(&self, item: &Item) -> Option<u64> {
        let e = self.items.get(&item.id)?;
        let current = Shared::same(&e.key.mesh.0, item.mesh.shared())
            && e.key.elements == item.elements
            && e.key
                .subdivision
                .as_ref()
                .is_none_or(|s| *s == item.subdivision.clone().unwrap_or_default());
        current.then_some(
            (e.geometry.fill.len() / 3) as u64
                + e.geometry.patches.as_ref().map_or(0, |p| p.net.triangles()),
        )
    }

    /// Cage polygons rewritten by patches so far (Frame Time).
    pub fn patched_polygons(&self) -> u64 {
        self.patched_polygons
    }

    /// How many times moved polygons were patched (tests).
    #[cfg(test)]
    pub fn patches(&self) -> u64 {
        self.patches
    }

    /// The item's cached geometry and the selection it was drawn with
    /// (tests compare it with a fresh build).
    #[cfg(test)]
    #[allow(clippy::type_complexity)]
    pub fn cached(
        &self,
        id: u64,
    ) -> Option<(
        Arc<ItemGeometry>,
        Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
    )> {
        self.items
            .get(&id)
            .map(|e| (e.geometry.clone(), e.key.selected.clone()))
    }

    /// Smooth pieces rewritten by patches, and piece normals computed for
    /// them (speed guards).
    #[cfg(test)]
    pub fn patch_piece_work(&self) -> (u64, u64) {
        (self.patched_pieces, self.patch_piece_normals)
    }

    /// Forget items no longer drawn.
    pub fn retain(&mut self, live: &BTreeSet<u64>) {
        // Nothing to forget (every frame of a steady scene): no tree lookups.
        let kept = |len: usize| len <= live.len();
        if kept(self.items.len())
            && kept(self.dots.len())
            && kept(self.seams.len())
            && self
                .items
                .keys()
                .chain(self.dots.keys())
                .chain(self.seams.keys())
                .all(|id| live.contains(id))
        {
            return;
        }
        self.items.retain(|id, _| live.contains(id));
        self.dots.retain(|id, _| live.contains(id));
        self.seams.retain(|id, _| live.contains(id));
    }

    /// The item's UV seam strokes (Show UV Seam), rebuilt when its mesh or
    /// placement changes. Only views with the toggle on ask, so the cost
    /// (a pass over the edges) is opt-in.
    /// Returns the strokes, their generation and the matrix to draw them
    /// through (`delta`).
    pub fn seams(
        &mut self,
        item: &Item,
        matrix: DMat4,
        live: bool,
    ) -> (Arc<Vec<EdgeVertex>>, u64, glam::Mat4) {
        let key = SeamKey {
            mesh: MeshRef(item.mesh.shared().clone()),
            palette: crate::theme::generation(),
        };
        if let Some(entry) = self.seams.get(&item.id)
            && entry.key == key
            && let Some(d) = delta(matrix, entry.matrix, live)
        {
            return (entry.seams.clone(), entry.generation, d);
        }
        let mut out = Vec::new();
        // UV breaks, and the edges marked in seam maps (Cut Map, UVT-101):
        // both are where the map is or will be cut.
        let mut edges: std::collections::BTreeSet<(u32, u32)> =
            uv_seams(&item.mesh).into_iter().collect();
        for name in operandi_core::uv_seams::seam_maps(&item.mesh) {
            edges.extend(operandi_core::uv_seams::seam_edges(&item.mesh, &name));
        }
        for (a, b) in edges {
            let (pa, pb) = (
                matrix.transform_point3(item.mesh.vertices[a as usize]),
                matrix.transform_point3(item.mesh.vertices[b as usize]),
            );
            stroke_width(&mut out, pa, pb, 2., palette::uv_seam());
        }
        self.next_generation += 1;
        let generation = self.next_generation;
        let seams = Arc::new(out);
        self.seams.insert(
            item.id,
            SeamEntry {
                key,
                matrix,
                seams: seams.clone(),
                generation,
            },
        );
        (seams, generation, glam::Mat4::IDENTITY)
    }

    /// The active item's point dots (MV-C03 rules), rebuilt only when the
    /// mesh, placement, mode or point selection changes. Occlusion is the
    /// depth test against the drawn surface, on the graphics card. Also
    /// returns the matrix to draw them through (`delta`).
    #[allow(clippy::too_many_arguments)]
    pub fn dots(
        &mut self,
        item: &Item,
        matrix: DMat4,
        vertex_mode: bool,
        all: bool,
        locked_points: bool,
        selected: &[u32],
        mirror: &BTreeSet<u32>,
        radii: [f32; 2],
        live: bool,
        // Where the points show when not on the cage (a smooth surface,
        // `Operandi::shown_points`); asked only on a rebuild.
        shown: &dyn Fn() -> Option<Vec<DVec3>>,
        topology: Option<TopologyDots>,
    ) -> (Arc<Vec<DotVertex>>, u64, glam::Mat4) {
        let key = DotKey {
            mesh: MeshRef(item.mesh.shared().clone()),
            vertex_mode,
            all,
            selected: selected.to_vec(),
            mirror: mirror.clone(),
            palette: crate::theme::generation(),
            radii: radii.map(f32::to_bits),
            elements: item.elements.clone(),
            locked: locked_points && show_locked(),
            smooth: operandi_core::subdivision::has_smooth(&item.mesh)
                .then(|| item.subdivision.clone().unwrap_or_default()),
            topology,
        };
        // HID-03 hidden points draw nothing; LCK-23 locked points draw in
        // the locked colour.
        let protection = operandi_core::Protection::of(&item.mesh, &item.elements);
        let hidden = |i: usize| protection.as_ref().is_some_and(|p| p.vertex_hidden[i]);
        let locked =
            |i: usize| key.locked && protection.as_ref().is_some_and(|p| p.vertex_locked[i]);
        if let Some(entry) = self.dots.get_mut(&item.id)
            && (entry.key == key || dots_still_match(entry, &key))
            && let Some(d) = delta(matrix, entry.matrix, live)
        {
            entry.key = key;
            return (entry.dots.clone(), entry.generation, d);
        }
        let mesh = &item.mesh;
        let shown = shown();
        let points: &[DVec3] = shown.as_deref().unwrap_or(&mesh.vertices);
        let mut drawn = Vec::new();
        let mut chosen = vec![false; mesh.vertices.len()];
        for &v in selected {
            if let Some(c) = chosen.get_mut(v as usize) {
                *c = true;
            }
        }
        let used = (!vertex_mode && !all).then(|| {
            let mut used = vec![false; mesh.vertices.len()];
            for face in &mesh.faces {
                for &v in &face.vertices {
                    used[v as usize] = true;
                }
            }
            used
        });
        // Topology Mode: a point whose polygons all face away is hidden.
        let back = topology.and_then(|t| t.hide_back).map(|(at, perspective)| {
            let mut normal = vec![DVec3::ZERO; mesh.vertices.len()];
            for f in 0..mesh.faces.len() {
                let n = matrix.transform_vector3(mesh.normal(f));
                for &v in &mesh.faces[f].vertices {
                    if let Some(s) = normal.get_mut(v as usize) {
                        *s += n;
                    }
                }
            }
            (0..mesh.vertices.len())
                .map(|i| {
                    let to_eye = if perspective {
                        at - matrix.transform_point3(mesh.vertices[i])
                    } else {
                        -at
                    };
                    normal[i] != DVec3::ZERO && normal[i].dot(to_eye) < 0.
                })
                .collect::<Vec<bool>>()
        });
        let mut out = Vec::new();
        // Plain points first, selected ones on top.
        for pass in [false, true] {
            for (i, &v) in points.iter().enumerate() {
                if chosen[i] != pass
                    || hidden(i)
                    || back.as_ref().is_some_and(|b| b.get(i) == Some(&true))
                    || (!locked(i)
                        && (used.as_ref().is_some_and(|u| u[i])
                            || (vertex_mode && !all && !chosen[i])))
                {
                    continue;
                }
                let color = if chosen[i] && mirror.contains(&(i as u32)) {
                    palette::mirror_selection()
                } else if chosen[i] {
                    palette::selection()
                } else if locked(i) {
                    palette::locked()
                } else if let Some(t) = topology {
                    t.color
                } else {
                    palette::wireframe()
                };
                drawn.push(i as u32);
                dot(
                    &mut out,
                    matrix.transform_point3(v),
                    if chosen[i] { radii[1] } else { radii[0] },
                    color,
                );
            }
        }
        self.next_generation += 1;
        let generation = self.next_generation;
        let dots = Arc::new(out);
        self.dots.insert(
            item.id,
            DotEntry {
                key,
                matrix,
                dots: dots.clone(),
                drawn,
                generation,
            },
        );
        (dots, generation, glam::Mat4::IDENTITY)
    }
}

/// The edges where the current UV map (the first one, provisional) breaks:
/// borders of its UV islands. An edge is one when only one of its polygon
/// sides has coordinates in the map, or its sides disagree. Operandi keeps
/// no separate seam data, so this is the seam.
pub fn uv_seams(mesh: &Mesh) -> Vec<(u32, u32)> {
    let map = mesh.uv_maps.values().next();
    let uv = |f: usize, c: usize| -> Option<glam::DVec2> {
        match map {
            Some(rows) => rows.get(f).and_then(|r| r.get(c).copied().flatten()),
            None => mesh.faces[f].uv.as_ref().and_then(|u| u.get(c).copied()),
        }
    };
    // Per edge (low, high): the sides' coordinates at (low, high).
    type Side = Option<(glam::DVec2, glam::DVec2)>;
    let mut sides: operandi_core::fxhash::FxHashMap<(u32, u32), Vec<Side>> = Default::default();
    for f in 0..mesh.faces.len() {
        if mesh.is_polyline(f) {
            continue;
        }
        let v = &mesh.faces[f].vertices;
        let n = v.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let (a, b) = (v[i], v[j]);
            let side = uv(f, i).zip(uv(f, j));
            let side = if a <= b {
                side
            } else {
                side.map(|(x, y)| (y, x))
            };
            sides.entry((a.min(b), a.max(b))).or_default().push(side);
        }
    }
    let close = |x: glam::DVec2, y: glam::DVec2| x.distance_squared(y) < 1e-18;
    let mut out: Vec<(u32, u32)> = sides
        .into_iter()
        .filter(|(_, s)| {
            let mapped: Vec<_> = s.iter().flatten().collect();
            match mapped.split_first() {
                None => false,
                Some((first, rest)) => {
                    mapped.len() != s.len()
                        || s.len() == 1
                        || rest
                            .iter()
                            .any(|o| !close(first.0, o.0) || !close(first.1, o.1))
                }
            }
        })
        .map(|(e, _)| e)
        .collect();
    out.sort_unstable();
    out
}

/// Who draws a polygon side's stroke (`Source::shared_side`).
enum SideStroke {
    /// This polygon, for itself.
    Alone,
    /// This polygon, for itself and the polygon across the edge.
    Here,
    /// The polygon across the edge.
    There,
}

/// Everything one build needs about an item.
/// A polygon context: material tag and part.
type LookKey<'a> = (&'a str, Option<&'a str>);

/// One polygon's UVs in the map a material program reads.
enum UvRows<'a> {
    Own(&'a [glam::DVec2]),
    Map(&'a [Option<glam::DVec2>]),
    /// Implicit UV: made from the polygon's corner count (TL-Q32).
    Implicit(usize),
}
/// The UVs a program reads on polygon `f`, as the renderer picks the map:
/// the named one, else the polygon's own, else the first map.
fn uv_rows<'m>(program: Option<&Instance>, m: &'m Mesh, f: usize) -> Option<UvRows<'m>> {
    let p = program?;
    Some(match &p.uv_map {
        Some(name) if name == operandi_core::texture_locator::IMPLICIT_UV_MAP => {
            UvRows::Implicit(m.faces.get(f)?.vertices.len())
        }
        Some(name) => UvRows::Map(m.uv_maps.get(name)?.get(f)?),
        None => match &m.faces[f].uv {
            Some(own) => UvRows::Own(own),
            None => UvRows::Map(m.uv_maps.values().next()?.get(f)?),
        },
    })
}
impl UvRows<'_> {
    fn at(&self, c: usize) -> [f64; 2] {
        match self {
            UvRows::Own(uv) => uv.get(c).copied(),
            UvRows::Map(uv) => uv.get(c).copied().flatten(),
            UvRows::Implicit(n) => Some(operandi_core::texture_locator::implicit_uv(*n, c)),
        }
        .map_or([0.; 2], |u| u.to_array())
    }
}

struct Source<'a> {
    item: &'a Item,
    matrix: DMat4,
    /// Turns local normals into world normals (inverse transpose).
    normal_matrix: glam::DMat3,
    /// Every point placed in the world, for a full build; a patch places
    /// only the points its polygons read (`world`).
    world: Option<Vec<DVec3>>,
    /// A patch's cage polygon normals, each computed once (a smooth-shaded
    /// corner reads every polygon around its point).
    face_memo: std::cell::RefCell<operandi_core::fxhash::FxHashMap<usize, DVec3>>,
    selection: &'a Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
    smooth: Option<&'a crate::subdivision_view::Evaluated>,
    /// Subdivision polygons drawn as patches: they write no fill or plain
    /// wire, only their look (`patch_looks`) and, as a highlight, their
    /// sides traced along the patches.
    patches: Option<&'a operandi_core::subdivision::patches::PatchNet>,
    patch_looks: std::cell::RefCell<Vec<(usize, [u32; PATCH_LOOK])>>,
    /// Patch polygons whose material has a program: their corners' UVs.
    patch_uvs: std::cell::RefCell<Vec<(usize, Vec<[f32; 2]>)>>,
    smoothing: &'a SmoothingSettings,
    materials: &'a MaterialLooks,
    point_faces: std::rc::Rc<operandi_core::topology_cache::PointFaces>,
    /// The polygon across each side (`plain_sides`).
    mates: std::rc::Rc<operandi_core::topology_cache::SideMates>,
    map: VertexMap<'a>,
    /// Colours already folded, by (material tag, part).
    looked: std::cell::RefCell<HashMap<(&'a str, Option<&'a str>), Looked>>,
    /// Procedural surfaces' colour expressions, by (material tag, part).
    patterned: std::cell::RefCell<HashMap<LookKey<'a>, Option<operandi_core::shader_eval::Expr>>>,
    /// Material programs (images drawn per pixel), by (material tag, part).
    programs: std::cell::RefCell<HashMap<LookKey<'a>, Option<Arc<Instance>>>>,
    /// A full build's world polygon normals (cage, then smooth pieces);
    /// a patch computes the few it needs instead.
    face_normals: Option<Vec<DVec3>>,
    piece_normals: Option<Vec<DVec3>>,
    /// A patch's piece normals and smooth point normals, each computed once
    /// (every refined point is shared by several pieces' corners).
    piece_memo: std::cell::RefCell<operandi_core::fxhash::FxHashMap<usize, DVec3>>,
    point_memo: std::cell::RefCell<operandi_core::fxhash::FxHashMap<u32, DVec3>>,
    /// Piece normals computed (not looked up), for the speed guards.
    piece_normals_computed: std::cell::Cell<u64>,
    /// Locked and hidden geometry (element_state.rs), when there is any.
    protection: Option<operandi_core::Protection>,
    /// LCK-25 Show Locked Mesh Items.
    show_locked: bool,
    /// The mesh's selected UV map (UVE-1 section 1): polygons without a
    /// material program carry its UVs for the Checker (UVT-114).
    checker_map: Option<String>,
    /// HT-5: the selected weight map's values, when shown and the mesh has
    /// it.
    weights: Option<&'a std::collections::BTreeMap<u32, f64>>,
    /// HT-6: each polygon's distortion tint, worked out on first use when
    /// shown.
    distortion: Option<std::cell::OnceCell<Vec<[u8; 4]>>>,
}
thread_local! {
    /// HT-5/HT-6: what the views' tints show, set each frame.
    static TINTS: std::cell::RefCell<Tints> = std::cell::RefCell::new(Tints::default());
}
pub fn set_tints(t: Tints) {
    TINTS.with(|c| {
        if *c.borrow() != t {
            *c.borrow_mut() = t;
        }
    });
}
fn tints() -> Tints {
    TINTS.with(|c| c.borrow().clone())
}
thread_local! {
    /// LCK-25: the viewport's Show Locked Mesh Items, set each frame.
    static SHOW_LOCKED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}
#[cfg(test)]
thread_local! {
    /// Tests: draw the way the viewport did before shared edges were drawn
    /// once and whole selected items took the selection as a uniform
    /// (`drawing_equivalence_tests.rs` compares the two pictures).
    static REFERENCE_DRAWING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
#[cfg(test)]
pub fn set_reference_drawing(on: bool) {
    REFERENCE_DRAWING.with(|c| c.set(on));
}
/// Whether tests asked for the reference drawing (always false in the app).
pub fn reference_drawing() -> bool {
    #[cfg(test)]
    return REFERENCE_DRAWING.with(std::cell::Cell::get);
    #[cfg(not(test))]
    false
}
pub fn set_show_locked(on: bool) {
    SHOW_LOCKED.with(|c| c.set(on));
}
fn show_locked() -> bool {
    SHOW_LOCKED.with(std::cell::Cell::get)
}
impl<'a> Source<'a> {
    /// Polygon `f`'s tints at its point `point` (`None`: the polygon's
    /// average weight, for subdivision pieces and patches). Points missing
    /// from the weight map count as 0 (HT-5).
    fn tint(&self, f: usize, point: Option<u32>) -> Tint {
        let mesh = &self.item.mesh;
        let weight = self.weights.map_or([0; 4], |map| {
            let at = |v: u32| map.get(&v).copied().unwrap_or(0.);
            Tint::weight(match point {
                Some(v) => at(v),
                None => {
                    let v = &mesh.faces[f].vertices;
                    v.iter().map(|&p| at(p)).sum::<f64>() / v.len().max(1) as f64
                }
            })
        });
        let distortion = self.distortion.as_ref().map_or([0; 4], |cell| {
            let all = cell.get_or_init(|| {
                self.checker_map.as_deref().map_or_else(Vec::new, |map| {
                    crate::uv_view::polygon_distortion(mesh, map)
                        .into_iter()
                        .map(|c| c.map_or([0; 4], Tint::colour))
                        .collect()
                })
            });
            all.get(f).copied().unwrap_or([0; 4])
        });
        Tint { weight, distortion }
    }
    /// Corner `c`'s UVs in the selected map on `m`'s polygon `f`, for the
    /// Checker; `NO_UV` without.
    fn checker_uv(&self, m: &Mesh, f: usize, c: usize) -> [f32; 2] {
        self.checker_map
            .as_deref()
            .and_then(|name| m.uv_maps.get(name))
            .and_then(|rows| rows.get(f))
            .and_then(|r| r.get(c).copied().flatten())
            .map_or(crate::uv_checker::NO_UV, |u| u.as_vec2().to_array())
    }
    /// HID-03: hidden polygons draw nothing.
    fn hidden(&self, f: usize) -> bool {
        self.protection
            .as_ref()
            .is_some_and(|p| p.face_hidden.get(f).copied().unwrap_or(false))
    }
    /// LCK-23: a locked polygon in the locked colour (unless Show Locked
    /// Mesh Items is off).
    fn locked(&self, f: usize) -> bool {
        self.show_locked
            && self
                .protection
                .as_ref()
                .is_some_and(|p| p.face_locked.get(f).copied().unwrap_or(false))
    }
    fn wire(&self, f: usize) -> Color32 {
        if self.locked(f) {
            palette::locked()
        } else {
            palette::wireframe()
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn patched(
        item: &'a Item,
        matrix: DMat4,
        selection: &'a Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
        smooth: Option<&'a crate::subdivision_view::Evaluated>,
        patches: Option<&'a operandi_core::subdivision::patches::PatchNet>,
        smoothing: &'a SmoothingSettings,
        materials: &'a MaterialLooks,
        full: bool,
    ) -> Self {
        let mut source = Source {
            item,
            matrix,
            normal_matrix: glam::DMat3::from_mat4(matrix).inverse().transpose(),
            world: full.then(|| {
                item.mesh
                    .vertices
                    .iter()
                    .map(|&v| matrix.transform_point3(v))
                    .collect()
            }),
            face_memo: Default::default(),
            selection,
            smooth,
            patches,
            patch_looks: Default::default(),
            patch_uvs: Default::default(),
            smoothing,
            materials,
            point_faces: item.mesh.point_faces(),
            mates: item.mesh.side_mates(),
            map: VertexMap::of(&item.mesh),
            looked: Default::default(),
            patterned: Default::default(),
            programs: Default::default(),
            face_normals: None,
            piece_normals: None,
            piece_memo: Default::default(),
            point_memo: Default::default(),
            piece_normals_computed: Default::default(),
            protection: operandi_core::Protection::of(&item.mesh, &item.elements),
            show_locked: show_locked(),
            checker_map: operandi_core::uv_maps::current_map(&item.mesh),
            weights: None,
            distortion: None,
        };
        let shown = tints();
        source.weights = shown
            .weights
            .as_ref()
            .and_then(|name| item.mesh.attributes.point_weights.get(name));
        source.distortion = shown.distortion.then(std::cell::OnceCell::new);
        if full {
            let faces = (0..item.mesh.faces.len())
                .map(|f| source.face_normal(f))
                .collect();
            let pieces = smooth.map(|s| {
                (0..s.mesh.faces.len())
                    .map(|p| source.piece_normal(p))
                    .collect()
            });
            source.face_normals = Some(faces);
            source.piece_normals = pieces;
        }
        source
    }

    /// Point `v` in the world.
    fn world(&self, v: u32) -> DVec3 {
        match &self.world {
            Some(world) => world[v as usize],
            None => self
                .matrix
                .transform_point3(self.item.mesh.vertices[v as usize]),
        }
    }

    fn face_normal(&self, f: usize) -> DVec3 {
        if let Some(n) = &self.face_normals {
            return n[f];
        }
        if let Some(&n) = self.face_memo.borrow().get(&f) {
            return n;
        }
        let v = &self.item.mesh.faces[f].vertices;
        let n = if v.len() < 3 {
            DVec3::ZERO
        } else {
            newell_by(v.len(), |k| self.world(v[k]))
        };
        if self.world.is_none() {
            self.face_memo.borrow_mut().insert(f, n);
        }
        n
    }

    fn piece_normal(&self, p: usize) -> DVec3 {
        if let Some(n) = &self.piece_normals {
            return n[p];
        }
        if let Some(&n) = self.piece_memo.borrow().get(&p) {
            return n;
        }
        let Some(surface) = self.smooth else {
            return DVec3::ZERO;
        };
        self.piece_normals_computed
            .set(self.piece_normals_computed.get() + 1);
        let v = &surface.mesh.faces[p].vertices;
        let n = if v.len() < 3 {
            DVec3::ZERO
        } else {
            let local = newell_by(v.len(), |k| surface.mesh.vertices[v[k] as usize]);
            (self.normal_matrix * local).normalize_or_zero()
        };
        // A full build fills `piece_normals` with this function first.
        if self.piece_normals.is_none() {
            self.piece_memo.borrow_mut().insert(p, n);
        }
        n
    }

    /// Shading normal of a cage polygon's corner: the default material's
    /// angle rule and amount (SS-02, `operandi_core::shading`).
    /// A material with Smoothing off gives a zero normal, which the shader
    /// reads as flat.
    fn corner_normal(&self, f: usize, v: u32, flat: DVec3) -> DVec3 {
        let mesh = &self.item.mesh;
        if self.materials.flat(&mesh.faces[f].material) {
            return DVec3::ZERO;
        }
        // G9 VN-1: the mesh's vertex normal map wins where it sets a corner.
        if !mesh.attributes.corner_normals.is_empty()
            && let Some(c) = mesh.faces[f].vertices.iter().position(|&x| x == v)
            && let Some(n) = operandi_core::vertex_normals::custom_corner_normal(mesh, f, c)
        {
            return (self.normal_matrix * n).normalize_or_zero();
        }
        operandi_core::shading::corner_normal(
            mesh,
            self.smoothing,
            f,
            self.point_faces.of(v),
            flat,
            |g| {
                if mesh.is_polyline(g) {
                    DVec3::ZERO
                } else {
                    self.face_normal(g)
                }
            },
        )
    }

    /// A subdivision surface is smooth everywhere: the average of the
    /// pieces around the refined point.
    fn piece_point_normal(
        &self,
        surface: &crate::subdivision_view::Evaluated,
        i: u32,
        flat: DVec3,
    ) -> DVec3 {
        let memo = self.point_memo.borrow().get(&i).copied();
        let n = memo.unwrap_or_else(|| {
            let n = surface
                .point_pieces
                .of(i)
                .iter()
                .map(|&p| self.piece_normal(p as usize))
                .sum::<DVec3>()
                .normalize_or_zero();
            self.point_memo.borrow_mut().insert(i, n);
            n
        });
        if n == DVec3::ZERO { flat } else { n }
    }

    fn state(&self, face: usize) -> (bool, bool) {
        match self.selection {
            Some((whole, faces, mirror)) => {
                let f = face as u32;
                let selected = *whole || faces.contains(&f);
                (selected, selected && mirror.contains(&f))
            }
            None => (false, false),
        }
    }

    /// One cage polygon's filled triangles and its wire sides (the cage
    /// edges, traced over the surface for a smooth polygon), in `color` /
    /// `wire`. Each polygon writes its own sides, so shared edges are
    /// stroked twice; that keeps every polygon's data in one block, which
    /// a drag can rewrite in place.
    fn face(
        &self,
        f: usize,
        lit_base: Option<Color32>,
        wire: Color32,
        highlighted: bool,
        fill: &mut Vec<WorldVertex>,
        edges: &mut Vec<EdgeVertex>,
    ) {
        let mesh = &self.item.mesh;
        if mesh.is_polyline(f) || self.hidden(f) {
            return;
        }
        let flip = self.matrix.determinant() < 0.;
        // Each corner: position, normal, vertex-map color.
        // A surface whose colour reads images is drawn per pixel by a
        // material program; a procedural one takes its colour at each
        // corner (stage 5).
        let program = lit_base.and_then(|_| self.program(f));
        let pattern = match &program {
            Some(p) if p.coloured => p.slot.clone(),
            _ => lit_base.and_then(|_| self.pattern(f)),
        };
        let coloured = program.as_ref().is_some_and(|p| p.coloured);
        let program_id = program.as_ref().map_or(0, |p| p.id);
        // The item's own point and normal (`frames` are in item space).
        let to_local_normal = glam::DMat3::from_mat4(self.matrix).transpose();
        let corner_colour =
            |local: DVec3, world_normal: DVec3, uv: [f64; 2], base: Color32| match &pattern {
                Some(e) => {
                    let at = operandi_core::shader_eval::ShadingPoint {
                        position: local.to_array(),
                        uv,
                        normal: (to_local_normal * world_normal).to_array(),
                    };
                    if coloured {
                        // The program's pattern value, stored linearly.
                        let v = e.eval_at(&at, &|_, _| [0.; 4]);
                        let byte = |c: f64| (c.clamp(0., 1.) * 255.).round() as u8;
                        Color32::from_rgb(byte(v[0]), byte(v[1]), byte(v[2]))
                    } else {
                        self.materials.pattern_colour(e, &at)
                    }
                }
                None => base,
            };
        let push_triangle =
            |fill: &mut Vec<WorldVertex>,
             p: [(DVec3, DVec3, Color32, Color32, [f64; 2], Tint); 3]| {
                let order = if flip { [0, 2, 1] } else { [0, 1, 2] };
                for k in order {
                    fill.push(WorldVertex {
                        position: p[k].0.as_vec3().to_array(),
                        normal: p[k].1.as_vec3().to_array(),
                        color: p[k].3,
                        map: p[k].2,
                        uv: p[k].4.map(|c| c as f32),
                        program: program_id,
                        tint: p[k].5,
                    });
                }
            };
        // Selected polygons tint their vertex-map colors (Vertex Map style
        // has no other fill to show the selection). A polygon whose fill is
        // a highlight colour is marked for the shader (`marked`).
        let selected = self.state(f).0;
        let highlight = self.highlighted(f);
        let tint = |map: Color32| {
            let map = match lit_base {
                Some(base) if selected && !self.whole_selected() => map.lerp_to_gamma(base, 0.45),
                _ => map,
            };
            marked(map, highlight)
        };
        if let Some(net) = self.patches
            && !net.blocks[f].is_empty()
        {
            match lit_base {
                // The look the patch shader reads: colours, flags and the
                // material program drawing the polygon per pixel (images),
                // with its corners' UVs. Provisional: a procedural pattern
                // draws its base colour.
                Some(base) => {
                    let faceted = self.materials.flat(&mesh.faces[f].material);
                    let shade = self.tint(f, None);
                    self.patch_looks.borrow_mut().push((
                        f,
                        [
                            u32::from_le_bytes(base.to_array()),
                            u32::from_le_bytes(tint(self.average_map(f)).to_array()),
                            if faceted { PATCH_FACETED } else { 0 },
                            program_id,
                            u32::from_le_bytes(shade.weight),
                            u32::from_le_bytes(shade.distortion),
                        ],
                    ));
                    // UVs whenever the material has a program, so selecting
                    // and deselecting needs no new ones.
                    if let Some(rows) = self
                        .material_program(f)
                        .and_then(|p| uv_rows(Some(&p), mesh, f))
                    {
                        let n = mesh.faces[f].vertices.len();
                        let uv = (0..n).map(|c| rows.at(c).map(|x| x as f32)).collect();
                        self.patch_uvs.borrow_mut().push((f, uv));
                    } else if self.checker_map.is_some() {
                        // The Checker reads the selected map (UVT-114).
                        let n = mesh.faces[f].vertices.len();
                        let uv = (0..n).map(|c| self.checker_uv(mesh, f, c)).collect();
                        self.patch_uvs.borrow_mut().push((f, uv));
                    }
                }
                // A highlight: the polygon's sides along the patches.
                None => {
                    let points = self.patch_sides(net, f);
                    for [a, b] in points.as_chunks::<2>().0 {
                        stroke(edges, *a, *b, highlighted, wire);
                    }
                }
            }
            return;
        }
        if operandi_core::subdivision::is_smooth_polygon(mesh, f)
            && let Some(surface) = self.smooth
        {
            // Pieces show their cage polygon's average map color.
            let map = tint(self.average_map(f));
            // Pieces take their cage polygon's average weight (provisional).
            let shade = self.tint(f, None);
            // One buffer for every piece (a drag rewrites thousands).
            let mut points: Vec<DVec3> = Vec::with_capacity(4);
            for piece in surface.blocks[f].clone() {
                let v = &surface.mesh.faces[piece].vertices;
                let n = v.len();
                points.clear();
                points.extend(v.iter().map(|&i| {
                    self.matrix
                        .transform_point3(surface.mesh.vertices[i as usize])
                }));
                if let Some(base) = lit_base {
                    let flat = self.piece_normal(piece);
                    let faceted = self.materials.flat(&mesh.faces[f].material);
                    let uvs = uv_rows(program.as_deref(), &surface.mesh, piece);
                    let corner = |k: usize| {
                        let n = if faceted {
                            DVec3::ZERO
                        } else {
                            self.piece_point_normal(surface, v[k], flat)
                        };
                        let shading = if faceted { flat } else { n };
                        let local = surface.mesh.vertices[v[k] as usize];
                        let uv = uvs.as_ref().map_or([0.; 2], |u| u.at(k));
                        // Without a program the vertex carries the selected
                        // map's UVs for the Checker (UVT-114).
                        let vertex_uv = match &uvs {
                            Some(_) => uv,
                            None => self.checker_uv(&surface.mesh, piece, k).map(f64::from),
                        };
                        (
                            points[k],
                            n,
                            map,
                            corner_colour(local, shading, uv, base),
                            vertex_uv,
                            shade,
                        )
                    };
                    for i in 1..n.saturating_sub(1) {
                        push_triangle(fill, [corner(0), corner(i), corner(i + 1)]);
                    }
                }
                let always = Feature {
                    always: true,
                    ..Feature::default()
                };
                for i in (0..n).filter(|&i| surface.traced[piece][i]) {
                    let width = if highlighted { 2. } else { 0. };
                    stroke_uses(
                        edges,
                        points[i],
                        points[(i + 1) % n],
                        width,
                        wire,
                        1.,
                        always,
                    );
                }
            }
            return;
        }
        let points: Vec<DVec3> = mesh.faces[f]
            .vertices
            .iter()
            .map(|&v| self.world(v))
            .collect();
        if let Some(base) = lit_base {
            let flat = self.face_normal(f);
            let vertices = &mesh.faces[f].vertices;
            let uvs = mesh.faces[f].uv.as_ref();
            let program_uvs = uv_rows(program.as_deref(), mesh, f);
            let corner = |c: usize| {
                let n = self.corner_normal(f, vertices[c], flat);
                let shading = if n == DVec3::ZERO { flat } else { n };
                let uv = match &program_uvs {
                    Some(rows) => rows.at(c),
                    None => uvs.and_then(|u| u.get(c)).map_or([0.; 2], |u| u.to_array()),
                };
                // Without a program the vertex carries the selected map's
                // UVs for the Checker (UVT-114).
                let vertex_uv = match &program_uvs {
                    Some(_) => uv,
                    None => self.checker_uv(mesh, f, c).map(f64::from),
                };
                (
                    points[c],
                    n,
                    tint(self.map.color(mesh, f, c)),
                    corner_colour(mesh.vertices[vertices[c] as usize], shading, uv, base),
                    vertex_uv,
                    self.tint(f, Some(vertices[c])),
                )
            };
            for t in corners(mesh, f) {
                push_triangle(fill, t.map(corner));
            }
        }
        let n = points.len();
        for i in 0..n {
            let uses = match self.shared_side(f, i, highlighted) {
                SideStroke::Alone => 1.,
                SideStroke::Here => 2.,
                SideStroke::There => continue,
            };
            let width = if highlighted { 2. } else { 0. };
            let across = self
                .mates
                .of(f, i)
                .filter(|&(g, _)| !mesh.is_polyline(g))
                .map_or(DVec3::ZERO, |(g, _)| self.face_normal(g));
            let feature = Feature {
                sides: [self.face_normal(f), across].map(|n| n.as_vec3().to_array()),
                always: false,
            };
            stroke_uses(
                edges,
                points[i],
                points[(i + 1) % n],
                width,
                wire,
                uses,
                feature,
            );
        }
    }

    /// Whether polygon `f`'s side `i` shares its stroke with the polygon
    /// across the edge: when that polygon strokes the same edge the same
    /// way (a plain polygon with the same wire colour, or also in the
    /// selection highlight), the lower-numbered of the two draws it once
    /// for both and the other draws nothing. Depends only on the topology,
    /// polygon kinds, hidden/locked state and (for the highlight) the
    /// selection, so a drag patch keeps every polygon's block the same size.
    fn shared_side(&self, f: usize, i: usize, highlighted: bool) -> SideStroke {
        let Some((g, _)) = self.mates.of(f, i).filter(|_| !reference_drawing()) else {
            return SideStroke::Alone;
        };
        let mesh = &self.item.mesh;
        let plain = |h: usize| {
            !mesh.is_polyline(h)
                && !self.hidden(h)
                && !((self.smooth.is_some() || self.patches.is_some())
                    && operandi_core::subdivision::is_smooth_polygon(mesh, h))
        };
        let same = if highlighted {
            self.state(g).0 && self.state(f).0
        } else {
            self.wire(g) == self.wire(f)
        };
        if g == f || !same || !plain(g) {
            SideStroke::Alone
        } else if f < g {
            SideStroke::Here
        } else {
            SideStroke::There
        }
    }

    /// Polygon `f`'s sides traced along its patches, in the world, as
    /// segment end pairs: side i is patch i's side 0 then patch i+1's
    /// side 3 (each a quadratic Bézier, L segments).
    fn patch_sides(
        &self,
        net: &operandi_core::subdivision::patches::PatchNet,
        f: usize,
    ) -> Vec<DVec3> {
        let block = net.blocks[f].clone();
        let n = block.len();
        let l = usize::from(net.level.max(1));
        let mut out = Vec::with_capacity(n * 4 * l);
        for k in 0..n {
            let here = block.start as usize + k;
            let next = block.start as usize + (k + 1) % n;
            for (patch, along) in [(here, true), (next, false)] {
                for s in 0..l {
                    let (t0, t1) = (s as f64 / l as f64, (s + 1) as f64 / l as f64);
                    // Side 0 is v = 0; side 3 runs u = 0 from v = 1 to 0.
                    let at = |t: f64| {
                        let p = if along {
                            net.evaluate(patch, t, 0.)
                        } else {
                            net.evaluate(patch, 0., 1. - t)
                        };
                        self.matrix.transform_point3(p)
                    };
                    out.push(at(t0));
                    out.push(at(t1));
                }
            }
        }
        out
    }

    fn whole_selected(&self) -> bool {
        matches!(self.selection, Some((true, _, _)))
    }

    fn average_map(&self, f: usize) -> Color32 {
        let mesh = &self.item.mesh;
        let n = mesh.faces[f].vertices.len().max(1);
        let sum = (0..n).fold([0u32; 3], |acc, c| {
            let m = self.map.color(mesh, f, c);
            [
                acc[0] + m.r() as u32,
                acc[1] + m.g() as u32,
                acc[2] + m.b() as u32,
            ]
        });
        Color32::from_rgb(
            (sum[0] / n as u32) as u8,
            (sum[1] / n as u32) as u8,
            (sum[2] / n as u32) as u8,
        )
    }

    /// Whether polygon `f` fills with a highlight colour (selection,
    /// symmetry selection or locked; see `base`).
    fn highlighted(&self, f: usize) -> bool {
        let (selected, mirrored) = self.state(f);
        selected || mirrored || self.locked(f)
    }

    fn base(&self, f: usize) -> Color32 {
        match self.state(f) {
            (_, true) => palette::mirror_selection(),
            (true, _) => palette::selection_fill(),
            _ if self.locked(f) => palette::locked(),
            _ => self.look(f).unwrap_or_else(palette::solid),
        }
    }

    /// A polygon's procedural colour expression, unless it is selected
    /// (the selection fill wins) or has none.
    fn pattern(&self, f: usize) -> Option<operandi_core::shader_eval::Expr> {
        let (selected, mirrored) = self.state(f);
        if selected || mirrored {
            return None;
        }
        let face = &self.item.mesh.faces[f];
        let key = (face.material.as_str(), face.part.as_deref());
        self.patterned
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| self.materials.pattern(self.item.id, key.0, key.1))
            .clone()
    }

    /// A polygon's material program, unless it is selected or locked (the
    /// selection and locked fills win) or its colour reads no image.
    fn program(&self, f: usize) -> Option<Arc<Instance>> {
        let (selected, mirrored) = self.state(f);
        if selected || mirrored || self.locked(f) {
            return None;
        }
        self.material_program(f)
    }

    /// The program of polygon `f`'s material, whatever its state.
    fn material_program(&self, f: usize) -> Option<Arc<Instance>> {
        let face = &self.item.mesh.faces[f];
        let key = (face.material.as_str(), face.part.as_deref());
        self.programs
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| self.materials.program(self.item.id, key.0, key.1))
            .clone()
    }

    /// Whether some polygon's material is see-through, whatever its
    /// selection state (`ItemGeometry::clear`).
    fn any_clear(&self) -> bool {
        (0..self.item.mesh.faces.len()).any(|f| self.material_program(f).is_some_and(|p| p.clear))
    }
    /// Whether some polygon's material has cut-outs (`ItemGeometry::cut`).
    fn any_cut(&self) -> bool {
        (0..self.item.mesh.faces.len()).any(|f| self.material_program(f).is_some_and(|p| p.cut))
    }

    fn look(&self, f: usize) -> Looked {
        let face = &self.item.mesh.faces[f];
        let key = (face.material.as_str(), face.part.as_deref());
        *self
            .looked
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| self.materials.look(self.item.id, key.0, key.1))
    }

    /// The selected polygons' wire in the selection color, drawn on top.
    fn highlight(&self) -> Vec<EdgeVertex> {
        let mut out = Vec::new();
        let mut unused = Vec::new();
        // Only the selected polygons, without visiting the rest of the mesh.
        let faces = self.item.mesh.faces.len();
        let chosen: Vec<usize> = match self.selection {
            Some((true, _, _)) => (0..faces).collect(),
            Some((false, selected, _)) => selected
                .iter()
                .map(|&f| f as usize)
                .filter(|&f| f < faces)
                .collect(),
            None => Vec::new(),
        };
        for f in chosen {
            self.face(f, None, palette::selection(), true, &mut unused, &mut out);
        }
        out
    }
}

/// The polygons whose selection coloring differs between two selection
/// keys (see `Resident::item`). `None` when it can't be told cheaply (the
/// whole item's selection toggled); the caller rebuilds instead.
fn selection_changes(
    old: &Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
    new: &Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
    faces: usize,
) -> Option<Vec<usize>> {
    let empty = (false, BTreeSet::new(), BTreeSet::new());
    let (a, b) = (
        old.as_ref().unwrap_or(&empty),
        new.as_ref().unwrap_or(&empty),
    );
    if a.0 || b.0 {
        return (a == b).then(Vec::new);
    }
    // A polygon's color depends on whether it is selected and mirrored.
    let state = |s: &(bool, BTreeSet<u32>, BTreeSet<u32>), f: u32| {
        (s.1.contains(&f), s.1.contains(&f) && s.2.contains(&f))
    };
    let mut changed: Vec<usize> =
        a.1.symmetric_difference(&b.1)
            .chain(a.2.symmetric_difference(&b.2))
            .filter(|&&f| state(a, f) != state(b, f))
            .map(|&f| f as usize)
            .filter(|&f| f < faces)
            .collect();
    changed.sort_unstable();
    changed.dedup();
    Some(changed)
}

/// The polygons whose UVs changed when only UV values changed (the same
/// maps, each the same shape, the same selected map): a UV tool's drag
/// rewrites just those (AGENTS.md "Fast drags"). `None` (rebuild) when
/// maps were added, removed, reshaped or another map was selected.
pub fn uv_dirty_faces(old: &Mesh, new: &Mesh) -> Option<Vec<usize>> {
    if old.attributes.current_uv_map != new.attributes.current_uv_map {
        return None;
    }
    if operandi_core::Shared::same(&old.uv_maps, &new.uv_maps) {
        return Some(Vec::new());
    }
    if old.uv_maps.len() != new.uv_maps.len() {
        return None;
    }
    let mut dirty = Vec::new();
    for ((ka, a), (kb, b)) in old.uv_maps.iter().zip(new.uv_maps.iter()) {
        if ka != kb || a.len() != b.len() {
            return None;
        }
        if a != b {
            dirty.extend((0..a.len()).filter(|&f| a[f] != b[f]));
        }
    }
    dirty.sort_unstable();
    dirty.dedup();
    Some(dirty)
}

/// Tests: the fill corners a plain build of `item` gives, with the tints
/// the views ask for now (`set_tints`).
#[cfg(test)]
pub fn fill_for_tests(item: &Item) -> Vec<WorldVertex> {
    let smoothing = SmoothingSettings::default();
    build(
        item,
        DMat4::IDENTITY,
        &None,
        None,
        None,
        &smoothing,
        &MaterialLooks::default(),
    )
    .fill
}

/// Tests: the fill vertices' UVs of a full build of `item` with no
/// materials (so every polygon is plain and carries the Checker's UVs).
#[cfg(test)]
pub fn checker_uvs_for_tests(item: &Item) -> Vec<[f32; 2]> {
    let smoothing = SmoothingSettings::default();
    build(
        item,
        DMat4::IDENTITY,
        &None,
        None,
        None,
        &smoothing,
        &MaterialLooks::default(),
    )
    .fill
    .iter()
    .map(|v| v.uv)
    .collect()
}

fn build(
    item: &Item,
    matrix: DMat4,
    selection: &Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
    smooth: Option<&crate::subdivision_view::Evaluated>,
    patches: Option<&Arc<operandi_core::subdivision::patches::PatchNet>>,
    smoothing: &SmoothingSettings,
    materials: &MaterialLooks,
) -> ItemGeometry {
    let source = Source::patched(
        item,
        matrix,
        selection,
        smooth,
        patches.map(|p| &**p),
        smoothing,
        materials,
        true,
    );
    let mut out = ItemGeometry {
        built: matrix,
        ..Default::default()
    };
    for f in 0..item.mesh.faces.len() {
        let (fill_start, edge_start) = (out.fill.len(), out.edges.len());
        source.face(
            f,
            Some(source.base(f)),
            source.wire(f),
            false,
            &mut out.fill,
            &mut out.edges,
        );
        out.fill_ranges.push(fill_start..out.fill.len());
        out.edge_ranges.push(edge_start..out.edges.len());
    }
    out.highlight = source.highlight();
    out.clear = source.any_clear();
    out.cut = source.any_cut();
    if let Some(net) = patches {
        // Polygons that wrote no look (hidden) draw nothing.
        let mut looks = vec![[0, 0, PATCH_HIDDEN, 0, 0, 0]; item.mesh.faces.len()];
        for (f, look) in source.patch_looks.take() {
            looks[f] = look;
        }
        let corner_uvs = source.patch_uvs.take();
        let uvs = (!corner_uvs.is_empty()).then(|| {
            let mut uvs = vec![crate::uv_checker::NO_UV; net.patches.len()];
            for (f, uv) in corner_uvs {
                let block = net.blocks[f].clone();
                for (p, uv) in block.zip(uv) {
                    uvs[p as usize] = uv;
                }
            }
            Arc::new(uvs)
        });
        out.patches = Some(Arc::new(PatchDraw {
            net: net.clone(),
            looks,
            uvs,
            wire: palette::wireframe(),
        }));
    }
    out
}

/// What one patch did, for the speed guards.
struct PatchWork {
    /// Smooth pieces rewritten.
    pieces: u64,
    /// Piece normals computed for them (each once).
    piece_normals: u64,
    /// Whether the selection highlight was rebuilt.
    highlight: bool,
}

/// Rewrite the blocks of `dirty` polygons in place (same topology, points
/// moved). Returns `None` when a block changed size (rebuild instead).
// One argument per input the geometry depends on, as `build` takes them.
#[allow(clippy::too_many_arguments)]
fn patch(
    geometry: &mut ItemGeometry,
    item: &Item,
    matrix: DMat4,
    selection: &Option<(bool, BTreeSet<u32>, BTreeSet<u32>)>,
    smooth: Option<&crate::subdivision_view::Evaluated>,
    patches: Option<&Arc<operandi_core::subdivision::patches::PatchNet>>,
    smoothing: &SmoothingSettings,
    materials: &MaterialLooks,
    dirty: &[usize],
    same_selection: bool,
) -> Option<PatchWork> {
    let source = Source::patched(
        item,
        matrix,
        selection,
        smooth,
        patches.map(|p| &**p),
        smoothing,
        materials,
        false,
    );
    let mut pieces = 0;
    if let Some(surface) = smooth {
        pieces = dirty
            .iter()
            .filter(|&&f| operandi_core::subdivision::is_smooth_polygon(&item.mesh, f))
            .map(|&f| surface.blocks[f].len())
            .sum();
        // Room for the normals the rewritten pieces (and their neighbours)
        // read, so the memo doesn't regrow mid-patch.
        source.piece_memo.borrow_mut().reserve(pieces * 3 / 2);
        source.point_memo.borrow_mut().reserve(pieces * 3 / 2);
    }
    let (mut fill, mut edges) = (Vec::new(), Vec::new());
    for &f in dirty {
        fill.clear();
        edges.clear();
        source.face(
            f,
            Some(source.base(f)),
            source.wire(f),
            false,
            &mut fill,
            &mut edges,
        );
        let (fr, er) = (
            geometry.fill_ranges[f].clone(),
            geometry.edge_ranges[f].clone(),
        );
        if fr.len() != fill.len() || er.len() != edges.len() {
            return None;
        }
        geometry.fill[fr].copy_from_slice(&fill);
        geometry.edges[er].copy_from_slice(&edges);
    }
    // The highlight draws only selected polygons: unchanged when the
    // selection is and none of them moved (a dirty polygon is every one
    // whose drawing changed, smooth neighbours included).
    let highlight = !same_selection || dirty.iter().any(|&f| source.state(f).0);
    if highlight {
        geometry.highlight = source.highlight();
    }
    // Patch polygons: the updated net and their new looks (a dirty one now
    // hidden draws nothing).
    if let Some(draw) = geometry.patches.as_mut() {
        let draw = Arc::make_mut(draw);
        if let Some(net) = patches {
            draw.net = net.clone();
        }
        let looks = &mut draw.looks;
        for &f in dirty {
            looks[f] = [0, 0, PATCH_HIDDEN, 0, 0, 0];
        }
        for (f, look) in source.patch_looks.take() {
            looks[f] = look;
        }
        let corner_uvs = source.patch_uvs.take();
        if !corner_uvs.is_empty()
            && let Some(uvs) = draw.uvs.as_mut()
        {
            let uvs = Arc::make_mut(uvs);
            for (f, uv) in corner_uvs {
                for (p, uv) in draw.net.blocks[f].clone().zip(uv) {
                    uvs[p as usize] = uv;
                }
            }
        }
    }
    Some(PatchWork {
        pieces: pieces as u64,
        piece_normals: source.piece_normals_computed.get(),
        highlight,
    })
}

/// The rollover overlay for polygon `face` of an item: its fill (or its
/// smooth pieces) in the rollover color and its edges, highlighted.
pub fn rollover(
    item: &Item,
    matrix: DMat4,
    face: usize,
    smooth: Option<&crate::subdivision_view::Evaluated>,
    out: &mut ItemGeometry,
) {
    let start = out.fill.len();
    rollover_pieces(item, matrix, face, smooth, out);
    // Mirroring placement: keep the overlay's triangles facing outward.
    if matrix.determinant() < 0. {
        for t in out.fill[start..].as_chunks_mut::<3>().0 {
            t.swap(1, 2);
        }
    }
}

/// A vertex-map colour with its alpha as the shader's highlight mark: 0
/// when the polygon's colour is a highlight (selection, locked, rollover)
/// that goes on top of the display style (display_shading.rs
/// `shade_marked`), else opaque.
fn marked(map: Color32, highlight: bool) -> Color32 {
    let [r, g, b, _] = map.to_array();
    Color32::from_rgba_premultiplied(r, g, b, if highlight { 0 } else { 255 })
}

/// A rollover corner: flat-shaded (no normal of its own), and the rollover
/// color on top of every style.
fn rollover_corner(p: DVec3, color: Color32) -> WorldVertex {
    WorldVertex {
        position: p.as_vec3().to_array(),
        normal: [0.; 3],
        color,
        map: marked(color, true),
        uv: crate::uv_checker::NO_UV,
        program: 0,
        tint: Tint::default(),
    }
}

fn rollover_pieces(
    item: &Item,
    matrix: DMat4,
    face: usize,
    smooth: Option<&crate::subdivision_view::Evaluated>,
    out: &mut ItemGeometry,
) {
    let mesh = &item.mesh;
    let Some(f) = mesh.faces.get(face) else {
        return;
    };
    if mesh.is_polyline(face) {
        return;
    }
    // The rollover colour for this polygon's kind (Face, Subdivision or
    // Catmull-Clark).
    let color = palette::polygon_rollover(mesh, face);
    if operandi_core::subdivision::is_smooth_polygon(mesh, face)
        && let Some(surface) = smooth
    {
        for (sf, piece) in surface.mesh.faces.iter().enumerate() {
            if surface.sources[sf] != face {
                continue;
            }
            let v = &piece.vertices;
            let points: Vec<DVec3> = v
                .iter()
                .map(|&i| matrix.transform_point3(surface.mesh.vertices[i as usize]))
                .collect();
            for i in 1..points.len().saturating_sub(1) {
                for j in [0, i, i + 1] {
                    out.fill.push(rollover_corner(points[j], color));
                }
            }
            let n = v.len();
            for i in (0..n).filter(|&i| surface.traced[sf][i]) {
                stroke(&mut out.edges, points[i], points[(i + 1) % n], true, color);
            }
        }
        return;
    }
    let points: Vec<DVec3> = f
        .vertices
        .iter()
        .map(|&v| matrix.transform_point3(mesh.vertices[v as usize]))
        .collect();
    for t in corners(mesh, face) {
        for c in t {
            out.fill.push(rollover_corner(points[c], color));
        }
    }
    let n = points.len();
    for i in 0..n {
        stroke(&mut out.edges, points[i], points[(i + 1) % n], true, color);
    }
}

/// N13 (Jason, 2026-11-16: wireframe lines were very thick): the plain
/// wire width in points, sized in device pixels so a Retina display draws
/// one pixel (1.5 in wireframe views), not two (three). On a standard
/// display this is the old 1 point (1.5). Below one pixel per point (an
/// Interface Scale under 100% on a standard display) it stays one device
/// pixel rather than thinning to a faint fraction of one.
pub fn plain_wire_width(wire: bool, pixels_per_point: f32) -> f32 {
    let pixels = if wire { 1.5 } else { 1. };
    pixels / pixels_per_point.max(0.25)
}

/// The camera as the shader's uniform: eye (w: the orthographic depth
/// offset, 0 in perspective), right, up, forward (w: the view distance
/// over the orthographic depth range) (vec4 each),
/// then (sx, sy, perspective, near clip in perspective or depth divisor in
/// orthographic) and (viewport
/// width, height in points, plain edge width, 0).
pub fn camera_uniform(camera: &Camera, r: Rect, wire: bool) -> [f32; 24] {
    let (eye, right, up, forward) = camera.basis();
    let (w, h) = (r.width() as f64, r.height() as f64);
    let perspective = camera.view == View::Perspective;
    let lens = camera.lens;
    let (sx, sy) = if perspective {
        (2. * h * lens / w, 2. * lens)
    } else {
        (
            2. * h * lens / (camera.distance * w),
            2. * lens / camera.distance,
        )
    };
    let v = |d: DVec3| [d.x as f32, d.y as f32, d.z as f32, 0.];
    let mut out = [0f32; 24];
    let (offset, range) = camera.ortho_depth();
    out[0..4].copy_from_slice(&v(eye));
    out[3] = offset as f32;
    // forward.w: the view distance in orthographic depth units, so the
    // wire's pull toward the eye stays a fraction of the zoom, as before.
    out[15] = if perspective {
        0.
    } else {
        (camera.distance / range) as f32
    };
    out[4..8].copy_from_slice(&v(right));
    out[8..12].copy_from_slice(&v(up));
    out[12..16].copy_from_slice(&v(forward));
    out[16..20].copy_from_slice(&[
        sx as f32,
        sy as f32,
        if perspective { 1. } else { 0. },
        if perspective { camera.near() } else { range } as f32,
    ]);
    out[20..24].copy_from_slice(&[w as f32, h as f32, if wire { 1.5 } else { 1. }, 0.]);
    out
}

/// A graph layer's coordinates on a mesh whose points span `lo`..`hi`,
/// placed by `world` (`graph_view::GraphSpace`).
fn graph_space(lo: DVec3, hi: DVec3, world: DMat4) -> operandi_core::graph_view::GraphSpace {
    let rows = |m: DMat4| {
        let c = m.to_cols_array_2d();
        let mut out = [0.; 12];
        for r in 0..3 {
            for k in 0..4 {
                out[r * 4 + k] = c[k][r];
            }
        }
        out
    };
    let n = glam::DMat3::from_mat4(world).inverse().transpose();
    let n = n.to_cols_array_2d();
    let mut normal = [0.; 9];
    for r in 0..3 {
        for k in 0..3 {
            normal[r * 3 + k] = n[k][r];
        }
    }
    operandi_core::graph_view::GraphSpace {
        generated: operandi_core::graph_view::GraphSpace::generated_for(lo.into(), hi.into()),
        to_world: rows(world),
        normal_to_world: normal,
    }
}

/// NM-40 (provisional, NM-Q07): when a graph's surface can't be drawn from
/// its nodes (`MaterialLooks::with_graph`), a Node Material shows one
/// colour (its plain base colour, else grey) over the layers by its
/// coverage, as Diffuse Color at full Amount.
fn node_stand_in(
    tree: &operandi_core::shader_tree::ShaderTree,
    mut surface: operandi_core::shader_eval::Surface,
) -> operandi_core::shader_eval::Surface {
    use operandi_core::shader_eval::{Ch, Expr, Mix};
    let Some(n) = surface.node.clone() else {
        return surface;
    };
    let colour = tree.node_viewport_colour(&n);
    for (ch, value) in [(Ch::DiffuseColor, colour), (Ch::DiffuseAmount, [1.; 3])] {
        let i = ch as usize;
        let below = std::mem::replace(&mut surface.channels[i], Expr::scalar(0.));
        surface.channels[i] = Expr::mix(Mix::Blend, n.coverage.clone(), below, Expr::Const(value));
    }
    surface
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    /// N13: one device pixel on Retina (two pixels before), unchanged on a
    /// standard display.
    #[test]
    fn plain_wire_is_one_device_pixel() {
        assert_eq!(plain_wire_width(false, 2.), 0.5);
        assert_eq!(plain_wire_width(true, 2.), 0.75);
        assert_eq!(plain_wire_width(false, 1.), 1.);
        assert_eq!(plain_wire_width(true, 1.), 1.5);
        // Windows 125% and a 75% Interface Scale: still one device pixel.
        assert!((plain_wire_width(false, 1.25) * 1.25 - 1.).abs() < 1e-6);
        assert!((plain_wire_width(false, 0.75) * 0.75 - 1.).abs() < 1e-6);
    }

    /// The shader's projection, written out on the CPU.
    fn shader_project(u: &[f32; 24], p: DVec3) -> Option<[f64; 3]> {
        let get = |i: usize| DVec3::new(u[i] as f64, u[i + 1] as f64, u[i + 2] as f64);
        let d = p - get(0);
        let z = d.dot(get(12));
        let x = d.dot(get(4)) * u[16] as f64;
        let y = d.dot(get(8)) * u[17] as f64;
        if u[18] > 0.5 {
            (z >= u[19] as f64).then(|| [x / z, y / z, (z - u[19] as f64) / z])
        } else {
            Some([x, y, (z + u[3] as f64) / u[19] as f64])
        }
    }

    #[test]
    fn shader_projection_matches_the_cpu_projection() {
        let r = Rect::from_min_size(pos2(100., 50.), vec2(1917., 1264.));
        for view in [View::Perspective, View::Front, View::Top] {
            let camera = Camera {
                view,
                yaw: 0.7,
                pitch: 0.3,
                distance: 6.,
                ..Default::default()
            };
            let u = camera_uniform(&camera, r, false);
            for p in [
                DVec3::new(0.3, -0.2, 0.5),
                DVec3::new(-1., 1.5, 2.),
                DVec3::ZERO,
            ] {
                let (Some(cpu), Some(gpu)) = (camera.project(p, r), shader_project(&u, p)) else {
                    panic!("{view:?} {p}")
                };
                let ndc = crate::outline::vertex(&camera, r, cpu, Color32::WHITE).position;
                for k in 0..3 {
                    assert!(
                        (ndc[k] as f64 - gpu[k]).abs() < 1e-4,
                        "{view:?} {p} {k}: {ndc:?} {gpu:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn geometry_is_reused_until_the_item_changes() {
        let mut scene = operandi_core::Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        let mut resident = Resident::default();
        let item = scene.items[0].clone();
        let (g, first, _) = resident.item(
            &item,
            DMat4::IDENTITY,
            None,
            None,
            &SmoothingSettings::default(),
        );
        // 6 quads -> 12 triangles; each of the 12 edges two quads share is
        // stroked once, for both.
        assert_eq!((g.fill.len(), g.edges.len()), (36, 12 * 6));
        assert_eq!(
            resident
                .item(
                    &item,
                    DMat4::IDENTITY,
                    None,
                    None,
                    &SmoothingSettings::default()
                )
                .1,
            first
        );
        let selected = Some((false, BTreeSet::from([0]), BTreeSet::new()));
        let (g, second, _) = resident.item(
            &item,
            DMat4::IDENTITY,
            selected.clone(),
            None,
            &SmoothingSettings::default(),
        );
        assert_ne!(second, first);
        // The selected quad's 4 sides are drawn again, highlighted, on top.
        assert_eq!(g.highlight.len(), 24);
        assert!(g.highlight.iter().all(|e| e.width == 2.));
        // Moving one point patches only the polygons near it, to the same
        // result as a full build.
        let mut moved = item.clone();
        let mut mesh = (*moved.mesh).clone();
        mesh.vertices[0].x -= 0.25;
        moved.mesh = mesh.into();
        let (g, third, patch) = resident.item(
            &moved,
            DMat4::IDENTITY,
            selected.clone(),
            None,
            &SmoothingSettings::default(),
        );
        let patch = patch.expect("a patch, not a rebuild");
        assert_eq!(patch.from, second);
        assert!(third > second);
        let fresh = Resident::default()
            .item(
                &moved,
                DMat4::IDENTITY,
                selected,
                None,
                &SmoothingSettings::default(),
            )
            .0;
        assert_eq!(g.fill, fresh.fill);
        assert_eq!(g.edges, fresh.edges);
        assert_eq!(g.highlight, fresh.highlight);
        // A new placement rebuilds.
        assert!(
            resident
                .item(
                    &moved,
                    DMat4::from_scale(DVec3::splat(2.)),
                    None,
                    None,
                    &SmoothingSettings::default()
                )
                .2
                .is_none()
        );
    }

    /// A whole-item drag reuses the geometry through a matrix that lands
    /// it where a rebuild would; outside a drag, a mirroring change, or a
    /// mesh edit at a new placement, it rebuilds.
    #[test]
    fn placement_drags_reuse_geometry_through_a_delta() {
        let mut scene = operandi_core::Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 2).unwrap().into();
        let item = scene.items[0].clone();
        let s = SmoothingSettings::default();
        let m0 = DMat4::from_translation(DVec3::new(1., 0., 0.));
        let mut resident = Resident::default();
        let (built, generation, _, d) = resident.item_placed(&item, m0, None, None, &s, true);
        assert_eq!(d, glam::Mat4::IDENTITY);
        let m1 = DMat4::from_scale_rotation_translation(
            DVec3::new(1.5, 0.5, 2.),
            glam::DQuat::from_rotation_y(0.7),
            DVec3::new(-2., 3., 0.5),
        );
        let builds = resident.builds();
        let (reused, same, patch, d) = resident.item_placed(&item, m1, None, None, &s, true);
        assert_eq!((resident.builds(), same), (builds, generation));
        assert!(patch.is_none() && Arc::ptr_eq(&built, &reused));
        let fresh = Resident::default().item(&item, m1, None, None, &s).0;
        assert_eq!(fresh.fill.len(), reused.fill.len());
        for (a, b) in reused.fill.iter().zip(&fresh.fill) {
            let p = d.transform_point3(glam::Vec3::from(a.position));
            assert!(
                (p - glam::Vec3::from(b.position)).length() < 1e-4,
                "{p} {b:?}"
            );
            // Normals through the inverse transpose, as the shader does.
            let normal = glam::Mat3::from_mat4(d).inverse().transpose();
            let n = (normal * glam::Vec3::from(a.normal)).normalize_or_zero();
            assert!(
                (n - glam::Vec3::from(b.normal)).length() < 1e-4,
                "{n} {b:?}"
            );
        }
        for (a, b) in reused.edges.iter().zip(&fresh.edges) {
            let p = d.transform_point3(glam::Vec3::from(a.a));
            assert!((p - glam::Vec3::from(b.a)).length() < 1e-4);
        }
        // The drag ends: the settled placement is built exactly.
        let (_, _, _, d) = resident.item_placed(&item, m1, None, None, &s, false);
        assert_eq!((resident.builds(), d), (builds + 1, glam::Mat4::IDENTITY));
        // Mirroring flips the winding: rebuild.
        let mirrored = m1 * DMat4::from_scale(DVec3::new(-1., 1., 1.));
        resident.item_placed(&item, mirrored, None, None, &s, true);
        assert_eq!(resident.builds(), builds + 2);
        // Points moved at a new placement: rebuild rather than patch.
        let mut moved = item.clone();
        let mut mesh = (*moved.mesh).clone();
        mesh.vertices[0].x -= 0.25;
        moved.mesh = mesh.into();
        let (_, _, patch, d) = resident.item_placed(&moved, m0, None, None, &s, true);
        assert!(patch.is_none());
        assert_eq!((resident.builds(), d), (builds + 3, glam::Mat4::IDENTITY));
        // Dots follow the same rule.
        let (_, dots, d) = resident.dots(
            &item,
            m0,
            true,
            true,
            true,
            &[],
            &BTreeSet::new(),
            [3., 4.],
            true,
            &|| None,
            None,
        );
        assert_eq!(d, glam::Mat4::IDENTITY);
        let (_, again, d) = resident.dots(
            &item,
            m1,
            true,
            true,
            true,
            &[],
            &BTreeSet::new(),
            [3., 4.],
            true,
            &|| None,
            None,
        );
        assert_eq!(again, dots);
        assert_ne!(d, glam::Mat4::IDENTITY);
    }

    #[test]
    fn selection_changes_patch_only_the_changed_polygons() {
        let mut scene = operandi_core::Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        let item = scene.items[0].clone();
        let mut resident = Resident::default();
        let pick = |faces: &[u32], mirror: &[u32]| {
            Some((
                false,
                faces.iter().copied().collect::<BTreeSet<_>>(),
                mirror.iter().copied().collect::<BTreeSet<_>>(),
            ))
        };
        resident.item(
            &item,
            DMat4::IDENTITY,
            None,
            None,
            &SmoothingSettings::default(),
        );
        // Painting adds polygons one at a time, then a mirror and a removal:
        // each is a patch of just the changed polygons, matching a full build.
        for (selection, changed) in [
            (pick(&[2], &[]), vec![2]),
            (pick(&[2, 4], &[]), vec![4]),
            (pick(&[2, 4], &[4]), vec![4]),
            (pick(&[4], &[4]), vec![2]),
            (None, vec![4]),
        ] {
            let builds = resident.builds();
            let (g, _, patch) = resident.item(
                &item,
                DMat4::IDENTITY,
                selection.clone(),
                None,
                &SmoothingSettings::default(),
            );
            let patch = patch.expect("a patch, not a rebuild");
            assert_eq!(resident.builds(), builds);
            let geometry = &g;
            assert_eq!(
                patch.fill,
                changed
                    .iter()
                    .map(|&f| geometry.fill_ranges[f].clone())
                    .collect::<Vec<_>>()
            );
            let fresh = Resident::default()
                .item(
                    &item,
                    DMat4::IDENTITY,
                    selection,
                    None,
                    &SmoothingSettings::default(),
                )
                .0;
            assert_eq!(g.fill, fresh.fill);
            assert_eq!(g.edges, fresh.edges);
            assert_eq!(g.highlight, fresh.highlight);
        }
        // Selecting the whole item rebuilds.
        let builds = resident.builds();
        resident.item(
            &item,
            DMat4::IDENTITY,
            Some((true, BTreeSet::new(), BTreeSet::new())),
            None,
            &SmoothingSettings::default(),
        );
        assert_eq!(resident.builds(), builds + 1);
    }

    /// A one-point polygon (Make Polygon from one point, MP06) draws
    /// nothing of its own and doesn't trip the build, the selected build
    /// or its rollover.
    #[test]
    fn a_one_point_polygon_builds_and_rolls_over() {
        let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let point = mesh.vertices.len() as u32;
        mesh.vertices.push(DVec3::new(2., 0., 0.));
        let (made, face) = operandi_core::make_polygon_from_points(&mesh, &[point]).unwrap();
        let mut scene = operandi_core::Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].mesh = made.into();
        let item = scene.items[0].clone();
        let mut resident = Resident::default();
        for selection in [
            None,
            Some((false, BTreeSet::from([point]), BTreeSet::from([face]))),
            Some((true, BTreeSet::from([point]), BTreeSet::from([face]))),
        ] {
            let (g, _, _) = resident.item(
                &item,
                DMat4::IDENTITY,
                selection,
                None,
                &SmoothingSettings::default(),
            );
            let mut out = (*g).clone();
            rollover(&item, DMat4::IDENTITY, face as usize, None, &mut out);
        }
    }

    #[test]
    fn stored_materials_color_the_fill_and_can_shade_flat() {
        let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 2).unwrap();
        mesh.faces[0].material = "Red".into();
        let mut scene = operandi_core::Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].mesh = mesh.into();
        let item = scene.items[0].clone();
        let wide = SmoothingSettings {
            angle: 100.,
            ..Default::default()
        };
        let build = |resident: &mut Resident, scene: &operandi_core::Scene| {
            resident.set_materials(MaterialLooks::of(scene));
            resident.item(&item, DMat4::IDENTITY, None, None, &wide).0
        };
        let mut resident = Resident::default();
        // No stored materials: every polygon keeps the theme's plain fill.
        let g = build(&mut resident, &scene);
        assert!(g.fill.iter().all(|v| v.color == palette::solid()));
        let red = operandi_core::Material {
            color: [1., 0., 0.],
            smoothing: false,
            ..Default::default()
        };
        scene.materials.insert("Red".into(), red);
        let g = build(&mut resident, &scene);
        let face0 = &g.fill[g.fill_ranges[0].clone()];
        // Diffuse Color × Diffuse Amount (80 %), as renders take it.
        assert!(face0.iter().all(|v| v.color == linear([0.8, 0., 0.])));
        // Smoothing off: zero normals, which the shader draws flat.
        assert!(face0.iter().all(|v| v.normal == [0.; 3]));
        let face1 = &g.fill[g.fill_ranges[1].clone()];
        assert!(face1.iter().all(|v| v.color == palette::solid()));
        assert!(face1.iter().all(|v| v.normal != [0.; 3]));
        // A stored Default colors untagged polygons.
        scene.materials.insert(
            "Default".into(),
            operandi_core::Material {
                color: [0., 0., 1.],
                ..Default::default()
            },
        );
        // A tag with no stored material keeps the plain fill, as renders
        // give it default values rather than the stored Default.
        let mut item = item.clone();
        let mut mesh = (*item.mesh).clone();
        mesh.faces[2].material = "Unstored".into();
        item.mesh = mesh.into();
        resident.set_materials(MaterialLooks::of(&scene));
        let g = resident.item(&item, DMat4::IDENTITY, None, None, &wide).0;
        let face1 = &g.fill[g.fill_ranges[1].clone()];
        assert!(face1.iter().all(|v| v.color == linear([0., 0., 0.8])));
        let face2 = &g.fill[g.fill_ranges[2].clone()];
        assert!(face2.iter().all(|v| v.color == palette::solid()));
    }

    /// Selected polygons fill with the selection colour and carry the
    /// highlight mark (map alpha 0) so the shader puts it on top of the
    /// display style; the others carry none.
    #[test]
    fn selected_polygons_are_marked_for_the_shader() {
        let scene = tagged_scene();
        let mut resident = Resident::default();
        resident.set_materials(MaterialLooks::of(&scene));
        let selection = Some((false, BTreeSet::from([1u32]), BTreeSet::new()));
        let g = resident
            .item(
                &scene.items[0],
                DMat4::IDENTITY,
                selection,
                None,
                &SmoothingSettings::default(),
            )
            .0;
        let face = |f: usize| &g.fill[g.fill_ranges[f].clone()];
        assert!(
            face(1)
                .iter()
                .all(|v| v.color == palette::selection_fill() && v.map.a() == 0)
        );
        assert!(face(2).iter().all(|v| v.map.a() == 255));
        // The rollover overlay is a highlight too.
        let mut out = ItemGeometry::default();
        rollover(&scene.items[0], DMat4::IDENTITY, 0, None, &mut out);
        assert!(!out.fill.is_empty() && out.fill.iter().all(|v| v.map.a() == 0));
    }

    /// A linear colour as the gamma-encoded vertex colour.
    fn linear([r, g, b]: [f32; 3]) -> Color32 {
        Color32::from(eframe::egui::Rgba::from_rgb(r, g, b))
    }

    /// One mesh item (a 2×2-per-side box) whose polygon 0 is tagged `Red`.
    fn tagged_scene() -> operandi_core::Scene {
        let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 2).unwrap();
        mesh.faces[0].material = "Red".into();
        let mut scene = operandi_core::Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].mesh = mesh.into();
        scene
    }
    fn fill_of(scene: &operandi_core::Scene, looks: MaterialLooks) -> Arc<ItemGeometry> {
        let mut resident = Resident::default();
        resident.set_materials(looks);
        resident
            .item(
                &scene.items[0],
                DMat4::IDENTITY,
                None,
                None,
                &SmoothingSettings::default(),
            )
            .0
    }
    fn face_color(g: &ItemGeometry, f: usize) -> Color32 {
        let vs = &g.fill[g.fill_ranges[f].clone()];
        assert!(vs.iter().all(|v| v.color == vs[0].color));
        vs[0].color
    }
    fn set_node(
        tree: &mut operandi_core::shader_tree::ShaderTree,
        id: operandi_core::shader_tree::NodeId,
        f: impl FnOnce(&mut operandi_core::shader_tree::Node),
    ) {
        let mut node = tree.get(id).unwrap().clone();
        f(&mut node);
        tree.replace(node).unwrap();
    }
    fn base_material(tree: &operandi_core::shader_tree::ShaderTree) -> u64 {
        tree.material_for_tag("Default").unwrap()
    }

    #[test]
    fn procedural_layers_colour_the_viewport_at_mesh_points() {
        use operandi_core::procedural::{Pattern, ProceduralKind};
        use operandi_core::shader_tree::{NewLayer, NodeKind, ShaderTreeEdit, TextureSource};
        let mut e = operandi_core::Editor {
            scene: tagged_scene(),
            ..Default::default()
        };
        e.dispatch(operandi_core::Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::Procedural(ProceduralKind::Checker),
            selected: None,
        }))
        .unwrap();
        // (The first edit maps the scene's material tags in, so ids move.)
        let mut node = e
            .scene
            .shader_tree
            .nodes
            .iter()
            .find(|n| matches!(&n.kind, NodeKind::Texture(_)))
            .unwrap()
            .clone();
        let NodeKind::Texture(t) = &mut node.kind else {
            panic!()
        };
        if let TextureSource::Procedural(p) = &mut t.source
            && let Pattern::Checker { transition, .. } = &mut p.pattern
        {
            *transition = 0.;
        }
        let locator = t.locator.unwrap();
        e.dispatch(operandi_core::Command::ShaderTree(
            ShaderTreeEdit::Replace {
                node,
                continuing: false,
            },
        ))
        .unwrap();
        let item = e.scene.item(locator).unwrap();
        let mut settings = item.texture_locator().unwrap().clone();
        settings.axis = 2;
        // Size ⅔ m: the cube's corners (±0.5) fall mid-square, alternating.
        let mut transform = item.transform.clone();
        transform.scale = DVec3::splat(2. / 3.);
        e.dispatch(operandi_core::Command::ShaderTree(
            ShaderTreeEdit::SetLocator {
                locator,
                settings: Box::new(settings),
                transform,
                continuing: false,
            },
        ))
        .unwrap();
        let mesh_item = e.scene.items.iter().position(|i| i.is_mesh()).unwrap();
        let mut resident = Resident::default();
        resident.set_materials(MaterialLooks::of(&e.scene));
        let g = resident
            .item(
                &e.scene.items[mesh_item],
                DMat4::IDENTITY,
                None,
                None,
                &SmoothingSettings::default(),
            )
            .0;
        // Black and white squares (× Diffuse Amount 80 %): a polygon's
        // corners differ.
        let (black, white) = (linear([0., 0., 0.]), linear([0.8, 0.8, 0.8]));
        let mixed = g.fill_ranges.iter().any(|r| {
            let vs = &g.fill[r.clone()];
            vs.iter().any(|v| v.color == black) && vs.iter().any(|v| v.color == white)
        });
        assert!(mixed, "no polygon shows both squares");
    }

    #[test]
    fn edited_shader_tree_colours_the_viewport_without_stored_materials() {
        use operandi_core::shader_tree::NodeKind;
        let mut scene = tagged_scene();
        let red = operandi_core::Material {
            color: [1., 0., 0.],
            diffuse: 0.5,
            ..Default::default()
        };
        // A Red group made in the Shading panel: nothing in the material
        // table, only the tree.
        scene.shader_tree.ensure_material("Red", &red).unwrap();
        let id = scene.shader_tree.material_for_tag("Red").unwrap();
        set_node(&mut scene.shader_tree, id, |n| {
            if let NodeKind::Material(m) = &mut n.kind {
                m.copy_from(&red);
            }
        });
        assert!(scene.materials.is_empty());
        let g = fill_of(&scene, MaterialLooks::of(&scene));
        assert_eq!(face_color(&g, 0), linear([0.5, 0., 0.]));
        assert_eq!(face_color(&g, 1), palette::solid());
        // A Base Material edit colours every other polygon.
        let base = base_material(&scene.shader_tree);
        set_node(&mut scene.shader_tree, base, |n| {
            if let NodeKind::Material(m) = &mut n.kind {
                m.diffuse_color = [0., 1., 0.];
                m.diffuse_amount = 1.;
            }
        });
        let g = fill_of(&scene, MaterialLooks::of(&scene));
        assert_eq!(face_color(&g, 0), linear([0.5, 0., 0.]));
        assert_eq!(face_color(&g, 1), linear([0., 1., 0.]));
    }

    /// Viewport materials (VM-1): a material's principled values reach the
    /// fill shader as an instance with its own rows, with no program when
    /// its colour reads no image; plain polygons keep instance 0, and a
    /// scene without materials has no instance table at all.
    #[test]
    fn principled_values_reach_the_advanced_style() {
        use operandi_core::shader_tree::NodeKind;
        let mut scene = tagged_scene();
        let mut resident = Resident::default();
        resident.set_materials(MaterialLooks::of(&scene));
        let g = resident
            .item(
                &scene.items[0],
                DMat4::IDENTITY,
                None,
                None,
                &SmoothingSettings::default(),
            )
            .0;
        assert!(g.fill.iter().all(|v| v.program == 0));
        assert!(resident.gpu_materials().is_none());
        let red = operandi_core::Material {
            color: [1., 0., 0.],
            diffuse: 0.5,
            ..Default::default()
        };
        scene.shader_tree.ensure_material("Red", &red).unwrap();
        let id = scene.shader_tree.material_for_tag("Red").unwrap();
        set_node(&mut scene.shader_tree, id, |n| {
            if let NodeKind::Material(m) = &mut n.kind {
                m.copy_from(&red);
                m.metallic = 1.;
                m.roughness = 0.2;
            }
        });
        let mut resident = Resident::default();
        resident.set_materials(MaterialLooks::of(&scene));
        let g = resident
            .item(
                &scene.items[0],
                DMat4::IDENTITY,
                None,
                None,
                &SmoothingSettings::default(),
            )
            .0;
        let face0 = &g.fill[g.fill_ranges[0].clone()];
        assert!(face0.iter().all(|v| v.program != 0));
        // The colour is still Diffuse Color × Amount for the other styles.
        assert_eq!(face_color(&g, 0), linear([0.5, 0., 0.]));
        assert!(
            g.fill[g.fill_ranges[1].clone()]
                .iter()
                .all(|v| v.program == 0)
        );
        let m = resident.gpu_materials().expect("an instance");
        assert!(!m.source.contains("fn op_m1("));
        let entry = m.instances[face0[0].program as usize];
        assert_eq!((entry[0], entry[3]), (0, 1));
        let rows =
            &m.params[entry[2] as usize..][..operandi_core::blender_principled::SURFACE_ROWS];
        assert_eq!(rows[0][3], 1., "metallic");
        assert!((rows[1][0] - 0.2).abs() < 1e-6, "roughness");
        // Advanced takes the Amount back out: the render's base colour is
        // Diffuse Color alone.
        assert!((rows[2][2] - 2.).abs() < 1e-6, "base scale");
    }

    /// VM-Q10: a material with cut-outs (a Stencil layer) is marked for
    /// the lighter-shadow pass, its shadow row letting the cut share
    /// through, without drawing in the glass pass.
    #[test]
    fn cut_outs_get_a_lighter_shadow_row() {
        use operandi_core::shader_tree::{NewLayer, NodeKind, TextureEffect, TextureSource};
        let mut scene = tagged_scene();
        scene
            .shader_tree
            .ensure_material("Red", &Default::default())
            .unwrap();
        let layer = scene.shader_tree.add(NewLayer::Constant, None).unwrap();
        set_node(&mut scene.shader_tree, layer, |n| {
            if let NodeKind::Texture(t) = &mut n.kind {
                t.effect = TextureEffect::Stencil;
                t.source = TextureSource::Constant {
                    color: [1.; 3],
                    value: 0.7,
                };
            }
        });
        let mut resident = Resident::default();
        let looks = MaterialLooks::of(&scene);
        let instance = looks
            .program(scene.items[0].id, "Red", None)
            .expect("an instance");
        resident.set_materials(looks);
        let g = resident
            .item(
                &scene.items[0],
                DMat4::IDENTITY,
                None,
                None,
                &SmoothingSettings::default(),
            )
            .0;
        assert!(instance.cut && !instance.clear);
        assert!(g.cut && !g.clear);
        let m = resident.gpu_materials().expect("tables");
        let row = m.shadow[instance.id as usize];
        assert!((row[3] - 0.7).abs() < 1e-5, "{row:?}");
        assert_eq!(m.clear[instance.id as usize], OPAQUE);
    }

    /// Viewport transparency (VT-01): a see-through material (Transparent
    /// Amount 98 %, Diffuse 0) makes an instance marked `clear`, with its
    /// tint and amount in the see-through table and the item flagged for
    /// the glass pass; Transparent Amount 0 adds no see-through row.
    #[test]
    fn see_through_materials_get_a_clear_row() {
        use operandi_core::shader_tree::NodeKind;
        let glass = |amount: f64, tint: [f64; 3]| {
            let mut scene = tagged_scene();
            let red = operandi_core::Material::default();
            scene.shader_tree.ensure_material("Red", &red).unwrap();
            let id = scene.shader_tree.material_for_tag("Red").unwrap();
            set_node(&mut scene.shader_tree, id, |n| {
                if let NodeKind::Material(m) = &mut n.kind {
                    m.diffuse_amount = 0.;
                    m.transparent_amount = amount;
                    m.transparent_color = tint;
                }
            });
            let mut resident = Resident::default();
            let looks = MaterialLooks::of(&scene);
            let instance = looks.program(scene.items[0].id, "Red", None);
            resident.set_materials(looks);
            let g = resident
                .item(
                    &scene.items[0],
                    DMat4::IDENTITY,
                    None,
                    None,
                    &SmoothingSettings::default(),
                )
                .0;
            let program = g.fill[g.fill_ranges[0].clone()][0].program;
            (instance, g, program, resident.gpu_materials())
        };
        let (instance, g, program, m) = glass(0.98, [1., 1., 1.]);
        assert!(instance.expect("an instance").clear);
        assert!(g.clear, "the item draws in the glass pass");
        let m = m.expect("a see-through table");
        let row = m.clear[program as usize];
        assert!(row[..3].iter().all(|c| (c - 1.).abs() < 1e-6), "{row:?}");
        assert!((row[3] - 0.98).abs() < 1e-6, "{row:?}");
        assert_eq!(m.clear[0], OPAQUE, "row 0 stays opaque");
        // An amber tint reaches the table gamma-encoded, like vertex colours.
        let amber = [1., 0.5, 0.1];
        let (_, _, program, m) = glass(0.98, amber);
        let row = m.unwrap().clear[program as usize];
        let want = linear(amber.map(|c| c as f32)).to_array();
        for i in 0..3 {
            assert!((row[i] - f32::from(want[i]) / 255.).abs() < 1e-6, "{row:?}");
        }
        // Opaque: no see-through row, the item stays out of the glass pass.
        let (instance, g, _, m) = glass(0., [1., 1., 1.]);
        assert!(instance.is_none_or(|i| !i.clear));
        assert!(!g.clear);
        assert!(m.is_none_or(|m| m.clear.iter().all(|r| *r == OPAQUE)));
    }

    #[test]
    fn item_masks_and_image_maps_reach_the_viewport() {
        use operandi_core::shader_tree::{
            GroupMask, ItemMask, NewLayer, NodeKind, TagMask, TextureEffect, TextureSource,
        };
        let mut scene = tagged_scene();
        let item = scene.items[0].id;
        scene.items[0].parent = Some(item + 100);
        let tree = &mut scene.shader_tree;
        let base = base_material(tree);
        let group = tree.add(NewLayer::Group, Some(base)).unwrap();
        let constant = tree.add(NewLayer::Constant, Some(group)).unwrap();
        set_node(tree, constant, |n| {
            if let NodeKind::Texture(t) = &mut n.kind {
                t.effect = TextureEffect::DiffuseColor;
                t.source = TextureSource::Constant {
                    color: [0., 0., 1.],
                    value: 1.,
                };
            }
        });
        let mask = |scene: &mut operandi_core::Scene, m: ItemMask| {
            set_node(&mut scene.shader_tree, group, |n| {
                n.kind = NodeKind::Group(GroupMask {
                    item: m,
                    tag: TagMask::All,
                    ..Default::default()
                });
            });
        };
        let amount = operandi_core::Material::default().diffuse as f32;
        // Masked to another item: this one keeps the plain fill.
        mask(&mut scene, ItemMask::Item(item + 1));
        let g = fill_of(&scene, MaterialLooks::of(&scene));
        assert_eq!(face_color(&g, 1), palette::solid());
        // Masked to the item group above it: blue.
        mask(&mut scene, ItemMask::ItemGroup(item + 100));
        let g = fill_of(&scene, MaterialLooks::of(&scene));
        assert_eq!(face_color(&g, 1), linear([0., 0., amount]));

        // An image map stands in as the picture's average colour.
        set_node(&mut scene.shader_tree, constant, |n| {
            if let NodeKind::Texture(t) = &mut n.kind {
                t.source = TextureSource::ImageMap {
                    image: Some(5),
                    settings: Default::default(),
                };
            }
        });
        scene.images.push(operandi_core::backdrop::SceneImage {
            id: 5,
            name: "green".into(),
            path: "green.png".into(),
            width: 1,
            height: 1,
        });
        let green_image = Arc::new(ImageData::new(1, 1, vec![0, 255, 0, 255]));
        let green = |_: &operandi_core::backdrop::SceneImage| Some(green_image.clone());
        let g = fill_of(&scene, MaterialLooks::with_images(&scene, green));
        assert_eq!(face_color(&g, 1), linear([0., amount, 0.]));
        // GL Display off on the shader: no textures in the viewport.
        let shader = scene
            .shader_tree
            .children(None)
            .find(|n| matches!(n.kind, NodeKind::Shader(_)))
            .unwrap()
            .id;
        set_node(&mut scene.shader_tree, shader, |n| {
            if let NodeKind::Shader(s) = &mut n.kind {
                s.gl_display = false;
            }
        });
        let g = fill_of(&scene, MaterialLooks::with_images(&scene, green));
        assert_eq!(face_color(&g, 1), palette::solid());
    }

    /// A scene whose Base Material reads `path` as its Diffuse Color
    /// image, with UVs on polygon 1.
    fn image_scene(path: &str) -> operandi_core::Scene {
        use operandi_core::shader_tree::{NewLayer, NodeKind, TextureEffect, TextureSource};
        let mut scene = tagged_scene();
        let mut mesh = (*scene.items[0].mesh).clone();
        let n = mesh.faces[1].vertices.len();
        mesh.faces[1].uv = Some(
            (0..n)
                .map(|k| glam::DVec2::new(k as f64 * 0.25, 1. - k as f64 * 0.125))
                .collect(),
        );
        scene.items[0].mesh = mesh.into();
        let base = base_material(&scene.shader_tree);
        let layer = scene
            .shader_tree
            .add(NewLayer::Constant, Some(base))
            .unwrap();
        set_node(&mut scene.shader_tree, layer, |n| {
            if let NodeKind::Texture(t) = &mut n.kind {
                t.effect = TextureEffect::DiffuseColor;
                t.source = TextureSource::ImageMap {
                    image: Some(5),
                    settings: Default::default(),
                };
            }
        });
        scene.images.push(operandi_core::backdrop::SceneImage {
            id: 5,
            name: "picture".into(),
            path: path.into(),
            width: 2,
            height: 2,
        });
        scene
    }

    /// Viewport materials stages 2–4: images on Roughness, Bump and Stencil
    /// become one list per pixel for the Advanced style (no colour program:
    /// the colour reads no image), its UVs on the vertices, and the fill
    /// shader with it validates.
    #[test]
    fn textured_principled_inputs_draw_per_pixel() {
        use operandi_core::shader_tree::{NodeKind, TextureEffect};
        let dir = std::env::temp_dir().join(format!("operandi-vm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rough.png");
        let pixels: [u8; 12] = [255, 0, 0, 0, 255, 0, 255, 255, 255, 255, 0, 255];
        image::save_buffer(&path, &pixels, 2, 2, image::ExtendedColorType::Rgb8).unwrap();
        let mut scene = image_scene(path.to_str().unwrap());
        let layer = scene
            .shader_tree
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Texture(_)))
            .unwrap()
            .id;
        set_node(&mut scene.shader_tree, layer, |n| {
            if let NodeKind::Texture(t) = &mut n.kind {
                t.effect = TextureEffect::Roughness;
            }
        });
        for effect in [TextureEffect::Bump, TextureEffect::Stencil] {
            let copy = scene
                .shader_tree
                .nodes
                .iter()
                .find(|n| n.id == layer)
                .unwrap()
                .clone();
            let base = base_material(&scene.shader_tree);
            let id = scene
                .shader_tree
                .add(operandi_core::shader_tree::NewLayer::Constant, Some(base))
                .unwrap();
            set_node(&mut scene.shader_tree, id, |n| {
                n.kind = copy.kind.clone();
                if let NodeKind::Texture(t) = &mut n.kind {
                    t.effect = effect;
                }
            });
        }
        let mut resident = Resident::default();
        resident.sync_materials(&scene, false);
        let g = resident
            .item(
                &scene.items[0],
                DMat4::IDENTITY,
                None,
                None,
                &SmoothingSettings::default(),
            )
            .0;
        let face1 = &g.fill[g.fill_ranges[1].clone()];
        assert!(face1.iter().all(|v| v.program != 0));
        assert!(
            face1.iter().any(|v| v.uv == [0.25, 0.875]),
            "the list's UVs"
        );
        let m = resident.gpu_materials().expect("an instance");
        assert!(!m.source.contains("fn op_m1("), "the colour reads no image");
        assert!(m.source.contains("fn op_l1("));
        let entry = m.instances[face1[0].program as usize];
        assert_eq!(entry[0], 0);
        assert_eq!(entry[3] & 1, 1);
        assert_eq!(entry[3] >> 1, 1, "list code");
        let shader = crate::gpu::world_shader_for_tests(&m.source);
        crate::material_gpu::valid(&shader).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every Voronoi feature and dimension count builds a valid shader
    /// (graph layers' Voronoi texture, drawn per pixel).
    #[test]
    fn voronoi_programs_are_valid_wgsl() {
        use operandi_core::cycles_voronoi::{
            VoronoiFeature as F, VoronoiMetric as M, VoronoiOut as O,
        };
        use operandi_core::shader_eval::Expr;
        use operandi_core::shader_func::Func;
        let mut bodies = Vec::new();
        for (dims, feature, metric, out) in [
            (1, F::SmoothF1, M::Euclidean, O::W),
            (2, F::F2, M::Manhattan, O::Color),
            (3, F::F1, M::Minkowski, O::Position),
            (3, F::DistanceToEdge, M::Euclidean, O::Distance),
            (4, F::NSphereRadius, M::Chebychev, O::Radius),
        ] {
            let mut args = vec![Expr::Locator(Box::new([
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.,
            ]))];
            args.extend([0., 5., 2., 0.5, 2., 1., 0.5, 1.].map(Expr::scalar));
            let f = Func::Voronoi {
                dims,
                feature,
                metric,
                normalize: true,
                out,
            };
            let p =
                operandi_core::shader_wgsl::translate(&Expr::func(f, args), &mut |_| None).unwrap();
            assert!(p.per_pixel() && p.body.contains("op_g_voronoi"));
            bodies.push(p.body);
        }
        let shader =
            crate::gpu::world_shader_for_tests(&operandi_core::shader_wgsl::module(&bodies));
        crate::material_gpu::valid(&shader).unwrap();
    }

    /// MaterialX stage 5: a graph layer (noise on Base Color, a data
    /// picture on Roughness) draws per pixel from its nodes, not as one
    /// flat colour, and its shader is valid WGSL.
    #[test]
    fn graph_layers_draw_from_their_nodes() {
        let dir = std::env::temp_dir().join(format!("operandi-mxv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pixels: [u8; 12] = [255, 0, 0, 0, 255, 0, 255, 255, 255, 255, 0, 255];
        image::save_buffer(
            dir.join("rough.png"),
            &pixels,
            2,
            2,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
        let text = r#"<materialx version="1.39">
  <noise3d name="grain" type="color3" />
  <image name="rough" type="float"><input name="file" type="filename" value="rough.png" /></image>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodename="grain" />
    <input name="specular_roughness" type="float" nodename="rough" />
  </open_pbr_surface>
  <surfacematerial name="Grain" type="material"><input name="surfaceshader" type="surfaceshader" nodename="s"/></surfacematerial>
</materialx>"#;
        let doc = operandi_core::materialx::doc::parse(text).unwrap();
        let (preset, _) =
            operandi_core::materialx::import::preset(&doc, None, &dir, &|_| Some((2, 2))).unwrap();
        let mut e = operandi_core::Editor {
            scene: tagged_scene(),
            ..Default::default()
        };
        e.dispatch(operandi_core::Command::InsertMaterialPreset {
            preset: Box::new(preset),
            beside: None,
        })
        .unwrap();
        let scene = e.scene;
        assert_eq!(scene.shader_tree.node_graphs.graphs.len(), 1);
        let mut resident = Resident::default();
        resident.sync_materials(&scene, false);
        // The picture is loaded once, and kept by the next refold.
        assert_eq!(resident.image_reads(), 1);
        let g = resident
            .item(
                &scene.items[0],
                DMat4::IDENTITY,
                None,
                None,
                &SmoothingSettings::default(),
            )
            .0;
        let face = &g.fill[g.fill_ranges[2].clone()];
        assert!(face.iter().all(|v| v.program != 0), "drawn by a program");
        let m = resident.gpu_materials().expect("a program");
        assert!(m.source.contains("op_g_noise"), "the noise runs per pixel");
        assert_eq!(m.images.len(), 1, "the graph's picture");
        let shader = crate::gpu::world_shader_for_tests(&m.source);
        crate::material_gpu::valid(&shader).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn image_maps_draw_per_pixel_through_material_programs() {
        let dir = std::env::temp_dir().join(format!("operandi-vit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("quads.png");
        let pixels: [u8; 12] = [255, 0, 0, 0, 255, 0, 255, 255, 255, 255, 0, 255];
        image::save_buffer(&path, &pixels, 2, 2, image::ExtendedColorType::Rgb8).unwrap();
        let mut scene = image_scene(path.to_str().unwrap());
        let item = scene.items[0].clone();
        let smoothing = SmoothingSettings::default();
        let mut resident = Resident::default();
        resident.sync_materials(&scene, false);
        let g = resident
            .item(&item, DMat4::IDENTITY, None, None, &smoothing)
            .0;
        let face1 = &g.fill[g.fill_ranges[1].clone()];
        assert!(face1.iter().all(|v| v.program != 0));
        // The polygon's own UVs reach the vertices.
        assert!(face1.iter().any(|v| v.uv == [0.25, 0.875]));
        let m = resident.gpu_materials().expect("a program");
        assert_eq!(m.images.len(), 1);
        assert_eq!((m.images[0].width, m.images[0].height), (2, 2));
        assert!(m.source.contains("fn op_m1("));
        // Nothing changed: the same snapshot, so no upload or compile.
        resident.sync_materials(&scene, false);
        let g2 = resident
            .item(&item, DMat4::IDENTITY, None, None, &smoothing)
            .0;
        assert!(Arc::ptr_eq(&g, &g2));
        assert_eq!(resident.gpu_materials().unwrap().generation, m.generation);
        // An item placement drag reuses the geometry and the materials.
        let moved = DMat4::from_translation(DVec3::new(0.5, 0., 0.));
        let (_, _, _, delta) = resident.item_placed(&item, moved, None, None, &smoothing, true);
        assert_ne!(delta, glam::Mat4::IDENTITY);
        assert_eq!(resident.gpu_materials().unwrap().generation, m.generation);
        assert_eq!(resident.image_reads(), 1);
        // A selected polygon shows the selection fill, not the image.
        let picked = Some((false, BTreeSet::from([1u32]), BTreeSet::new()));
        let g = resident
            .item(&item, DMat4::IDENTITY, picked, None, &smoothing)
            .0;
        assert!(
            g.fill[g.fill_ranges[1].clone()]
                .iter()
                .all(|v| v.program == 0)
        );
        // Reload Image reads the file again and uploads a new texture.
        resident.reload_image(path.to_str().unwrap());
        resident.sync_materials(&scene, false);
        resident.item(&item, DMat4::IDENTITY, None, None, &smoothing);
        let reloaded = resident.gpu_materials().unwrap();
        assert_ne!(reloaded.images[0].key, m.images[0].key);
        assert_eq!(resident.image_reads(), 2);
        // A missing file: mid grey, still per pixel.
        scene.images[0].path = dir.join("missing.png").to_str().unwrap().into();
        resident.sync_materials(&scene, false);
        resident.item(&item, DMat4::IDENTITY, None, None, &smoothing);
        let missing = resident.gpu_materials().unwrap();
        assert!(missing.images[0].average.is_none());
        // GL Display off: image layers cover nothing, so no program draws
        // the colour (the Advanced style's principled values may remain).
        let shader = scene
            .shader_tree
            .children(None)
            .find(|n| matches!(n.kind, operandi_core::shader_tree::NodeKind::Shader(_)))
            .unwrap()
            .id;
        set_node(&mut scene.shader_tree, shader, |n| {
            if let operandi_core::shader_tree::NodeKind::Shader(s) = &mut n.kind {
                s.gl_display = false;
            }
        });
        resident.sync_materials(&scene, false);
        let g = resident
            .item(&item, DMat4::IDENTITY, None, None, &smoothing)
            .0;
        assert!(!g.fill.is_empty());
        if let Some(m) = resident.gpu_materials() {
            assert!(m.instances.iter().all(|i| i[0] == 0));
            assert!(!m.source.contains("fn op_m1("));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sync_materials_folds_again_only_on_change() {
        let mut scene = tagged_scene();
        scene.materials.insert(
            "Red".into(),
            operandi_core::Material {
                color: [1., 0., 0.],
                ..Default::default()
            },
        );
        let mut resident = Resident::default();
        resident.sync_materials(&scene, false);
        let first = resident.materials.0.clone();
        resident.sync_materials(&scene, false);
        assert!(Arc::ptr_eq(&first, &resident.materials.0));
        scene.materials.get_mut("Red").unwrap().color = [0., 1., 0.];
        resident.sync_materials(&scene, false);
        assert!(!Arc::ptr_eq(&first, &resident.materials.0));
        // Moving the item under a group refolds too (item-group masks).
        let second = resident.materials.0.clone();
        scene.items[0].parent = Some(42);
        resident.sync_materials(&scene, false);
        assert!(!Arc::ptr_eq(&second, &resident.materials.0));
    }

    #[test]
    fn smoothing_angle_decides_corner_normals_and_patches_match_builds() {
        let mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 2).unwrap();
        let mut scene = operandi_core::Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].mesh = mesh.clone().into();
        let item = scene.items[0].clone();
        let at_corner = |g: &ItemGeometry| -> BTreeSet<[u32; 3]> {
            g.fill
                .iter()
                .filter(|v| v.position == [0.5, 0.5, 0.5])
                .map(|v| v.normal.map(f32::to_bits))
                .collect()
        };
        // SS-A01: at the default 40° a cube's 90° sides stay crisp.
        let g = Resident::default()
            .item(
                &item,
                DMat4::IDENTITY,
                None,
                None,
                &SmoothingSettings::default(),
            )
            .0;
        assert_eq!(at_corner(&g).len(), 3);
        // SS-A02: at 100° every corner at a point shares the diagonal.
        let wide = SmoothingSettings {
            angle: 100.,
            ..Default::default()
        };
        let mut resident = Resident::default();
        let (g, _, _) = resident.item(&item, DMat4::IDENTITY, None, None, &wide);
        let normals = at_corner(&g);
        assert_eq!(normals.len(), 1);
        let n = glam::Vec3::from(normals.first().unwrap().map(f32::from_bits));
        assert!(
            (n - glam::Vec3::splat(1.).normalize()).length() < 1e-5,
            "{n}"
        );
        // Changing the settings rebuilds rather than reusing stale normals.
        let (_, _, patch) = resident.item(
            &item,
            DMat4::IDENTITY,
            None,
            None,
            &SmoothingSettings::default(),
        );
        assert!(patch.is_none());
        resident.item(&item, DMat4::IDENTITY, None, None, &wide);
        // A drag's patch gives what a full build gives, neighbours included.
        let mut moved = item.clone();
        let mut m = mesh.clone();
        m.vertices[0] += DVec3::new(0.1, -0.2, 0.05);
        moved.mesh = m.into();
        let (g, _, patch) = resident.item(&moved, DMat4::IDENTITY, None, None, &wide);
        assert!(patch.is_some());
        let fresh = Resident::default()
            .item(&moved, DMat4::IDENTITY, None, None, &wide)
            .0;
        assert_eq!(g.fill, fresh.fill);
    }

    #[test]
    fn weight_colors() {
        assert_eq!(weight_color(0.), rgb(0.25, 0.7, 0.25));
        assert_eq!(weight_color(1.), rgb(0.9, 0.15, 0.1));
        assert_eq!(weight_color(-2.), rgb(0.15, 0.3, 0.95));
    }

    #[test]
    fn vertex_map_prefers_color_maps_and_falls_back_to_neutral() {
        let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        fn corners<T: Clone>(m: &Mesh) -> Vec<Vec<Option<T>>> {
            m.faces
                .iter()
                .map(|f| vec![None; f.vertices.len()])
                .collect()
        }
        assert_eq!(
            VertexMap::of(&mesh).color(&mesh, 0, 0),
            palette::vertex_map_none()
        );
        let mut weights: Vec<Vec<Option<f64>>> = corners(&mesh);
        weights[0][0] = Some(1.);
        mesh.attributes.corner_weights.insert("w".into(), weights);
        assert_eq!(VertexMap::of(&mesh).color(&mesh, 0, 0), weight_color(1.));
        // Unassigned weights read as zero.
        assert_eq!(VertexMap::of(&mesh).color(&mesh, 0, 1), weight_color(0.));
        let mut colors: Vec<Vec<Option<glam::DVec4>>> = corners(&mesh);
        colors[0][0] = Some(glam::DVec4::new(0., 0., 1., 1.));
        mesh.attributes.corner_colors.insert("c".into(), colors);
        assert_eq!(
            VertexMap::of(&mesh).color(&mesh, 0, 0),
            Color32::from_rgb(0, 0, 255)
        );
    }

    #[test]
    fn uv_seams_are_uv_island_borders() {
        // Two quads sharing an edge, mapped continuously: only the outline.
        let mut mesh = Mesh {
            vertices: vec![
                DVec3::new(0., 0., 0.),
                DVec3::new(1., 0., 0.),
                DVec3::new(2., 0., 0.),
                DVec3::new(0., 1., 0.),
                DVec3::new(1., 1., 0.),
                DVec3::new(2., 1., 0.),
            ],
            faces: [vec![0, 1, 4, 3], vec![1, 2, 5, 4]]
                .into_iter()
                .map(|vertices| operandi_core::Face {
                    vertices,
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                })
                .collect(),
            ..Default::default()
        };
        assert!(uv_seams(&mesh).is_empty(), "no UVs, no seams");
        let uv = |v: &[u32], shift: f64| {
            v.iter()
                .map(|&i| {
                    let p = mesh.vertices[i as usize];
                    Some(glam::DVec2::new(p.x + shift, p.y))
                })
                .collect::<Vec<_>>()
        };
        let rows = vec![uv(&[0, 1, 4, 3], 0.), uv(&[1, 2, 5, 4], 0.)];
        mesh.uv_maps.insert("Texture".into(), rows);
        let seams = uv_seams(&mesh);
        assert_eq!(seams.len(), 6);
        assert!(!seams.contains(&(1, 4)));
        // The second quad cut off in UV space: the shared edge is a seam.
        let rows = vec![uv(&[0, 1, 4, 3], 0.), uv(&[1, 2, 5, 4], 3.)];
        mesh.uv_maps.insert("Texture".into(), rows);
        assert!(uv_seams(&mesh).contains(&(1, 4)));
    }
}
