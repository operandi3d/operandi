//! Owned offscreen viewport resources. A scene snapshot is projected to clip
//! vertices by the UI adapter; wgpu owns rasterization, occlusion and depth.
//! Resources are replaced on physical resize; queue submission precedes egui's
//! sampling submission. wgpu retains submitted resources until GPU completion.
use crate::gpu_arena::{Arena, Runs, Store};
use crate::gpu_chunks::VertexChunks;
use eframe::egui::{Color32, TextureId};
use egui_wgpu::wgpu;
pub struct Vertex {
    pub position: [f32; 3],
    pub color: Color32,
}
struct Target {
    size: [u32; 2],
    color: wgpu::Texture,
    depth: wgpu::Texture,
    multisampled: Option<wgpu::Texture>,
    id: TextureId,
}
/// How one group of items draws in a view (view_display.rs): its style,
/// overlay and smoothing, as the shader's `Look` uniform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub style: crate::view_display::Style,
    pub overlay: crate::view_display::Overlay,
    /// 0–1.
    pub opacity: f32,
    pub smoothing: bool,
    /// The plain (Uniform overlay) wire draws in this colour instead of the
    /// baked Wireframe: the group's own `wire_color`, or for inactive
    /// meshes the theme's Inactive Wireframe
    /// (ViewDisplay::plain_wire_color). `None`: Wireframe as baked.
    pub plain_wire: Option<Color32>,
    /// The Colored overlay's wire: the group's own `wire_color`, else the
    /// theme's Dark Lines Overlay (black by default, VS-6).
    pub colored: Color32,
    /// The Advanced style's ambient light (Advanced Options > Ambient
    /// Intensity × Ambient Color), 0–1 per channel.
    pub ambient: [f32; 3],
    /// Transparency Overrides (VP-80): the group's polygons see-through.
    pub transparency: crate::view_display::Transparency,
    /// The view's matcap (the Matcap style, matcap.rs).
    pub matcap: crate::matcap::Choice,
    /// HDRI stage 3: the environment picture lighting the Advanced style.
    pub env: EnvLook,
    /// Fade Wireframe on Small Polygons (Preferences > Display): the plain
    /// wire of edges only a few points long fades out (`wire_fade`).
    pub fade_small_wire: bool,
    /// UVT-114 Checker (the view's Display menu; an approved Operandi
    /// extra, uv_checker.rs): fills show the numbered check through their
    /// UVs.
    pub checker: bool,
    /// HT-5 Show Weightmaps and HT-6 Show UV Distortion: the fills' baked
    /// tints show (gpu_scene.rs `Tint`; weights win where both are).
    pub weights: bool,
    pub distortion: bool,
    /// Show Material Transparency (Preferences > Display): see-through
    /// materials let what is behind them show (`glass_on`).
    pub material_transparency: bool,
}
/// Fade Wireframe on Small Polygons: an edge drawn shorter than
/// `WIRE_FADE_GONE` points has no wire, one longer than `WIRE_FADE_FULL`
/// points its full wire, and the opacity rises evenly in between. Where a
/// mesh's polygons are smaller than that on screen, the wire would cover
/// them completely (and cost most of the frame).
pub const WIRE_FADE_GONE: f32 = 2.;
pub const WIRE_FADE_FULL: f32 = 6.;
/// Edges where the polygons turn more than this many degrees are sharp and
/// keep their wire (Operandi's choice, provisional).
pub const WIRE_FADE_SHARP: f32 = 40.;
/// The environment picture's light for the Advanced style (HDRI stage 3):
/// its irradiance harmonics (environment_gpu.rs), the world-to-picture
/// rotation (rows) and Intensity. Off: Advanced keeps its flat ambient.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnvLook {
    pub on: bool,
    pub sh: [[f32; 3]; 9],
    pub to_image: [f32; 9],
    pub intensity: f32,
}
impl EnvLook {
    fn rows(&self) -> impl Iterator<Item = f32> {
        let sh = self.sh.into_iter().flat_map(|c| [c[0], c[1], c[2], 0.]);
        let r = self.to_image;
        sh.chain([
            r[0], r[1], r[2], 0., r[3], r[4], r[5], 0., r[6], r[7], r[8], 0.,
        ])
        .chain([if self.on { 1. } else { 0. }, self.intensity, 0., 0.])
    }
}
/// The scene group binding of the environment picture's blurred copy
/// (HDRI stage 3), after the matcaps.
pub const ENV_BINDING: u32 = crate::matcap::BINDING + 1;
/// The scene group binding of the see-through table (viewport
/// transparency, material_gpu.rs `MaterialGpu::clear_entry`): per material
/// instance, Transparent Color as a tint and how much shows through.
pub const CLEAR_BINDING: u32 = ENV_BINDING + 1;
/// The scene group binding of the picture behind see-through surfaces
/// (`BehindGpu`): refraction shifts what they show of it.
pub const BEHIND_BINDING: u32 = CLEAR_BINDING + 1;
impl Look {
    pub fn of(group: crate::view_display::Group) -> Self {
        Self {
            style: group.style,
            overlay: group.overlay,
            opacity: f32::from(group.opacity.min(100)) / 100.,
            smoothing: group.smoothing,
            plain_wire: None,
            colored: crate::theme::viewport::colored_wire(),
            ambient: crate::view_display::Drawing::default().ambient(),
            transparency: group.transparency,
            matcap: crate::matcap::Choice::default(),
            env: EnvLook::default(),
            fade_small_wire: false,
            checker: false,
            weights: false,
            distortion: false,
            material_transparency: false,
        }
    }
    /// Whether see-through materials show through in this look (viewport
    /// transparency, VT-01..05): Show Material Transparency on, in the
    /// Advanced style only (Jason, 2026-11-14, VT-Q01), and not already a
    /// Transparency Override group (VP-80), which draws its own way. Other
    /// styles draw see-through materials solid.
    pub fn glass_on(&self) -> bool {
        self.material_transparency
            && !self.see_through()
            && self.style == crate::view_display::Style::Advanced
    }
    /// Whether the group's polygons draw see-through (VP-80).
    pub fn see_through(&self) -> bool {
        self.fills() && self.transparency.on()
    }
    /// The active ([0]) and inactive ([1]) groups' looks in a view, in
    /// selection mode `mode`.
    pub fn pair(
        display: &crate::view_display::ViewDisplay,
        mode: operandi_core::Mode,
    ) -> [Self; 2] {
        [true, false].map(|active| {
            let group = display.group(active).unwrap_or(display.settings.inactive);
            Self {
                plain_wire: display.plain_wire_color(active),
                colored: group
                    .wire_color
                    .map_or_else(crate::theme::viewport::colored_wire, |[r, g, b]| {
                        Color32::from_rgb(r, g, b)
                    }),
                ambient: display.settings.drawing.ambient(),
                matcap: display.settings.drawing.matcap,
                ..Self::of(display.effective(group, mode))
            }
        })
    }
    fn fills(&self) -> bool {
        self.style != crate::view_display::Style::Wireframe
    }
    fn wire(&self) -> bool {
        self.overlay != crate::view_display::Overlay::None && self.opacity > 0.
    }
    /// The uniform's size: mode, wire, display_shading's two Gooch rows,
    /// the inactive wire's two rows, Advanced's ambient (w: the matcap
    /// layer), then the three transparency rows.
    /// Then the environment light's 13 rows (`EnvLook`), then the wire fade
    /// (on, gone, full, -), then the glass row (`glass_on`, -, -, -).
    const SIZE: u64 = (2 + 2 + 2 + 1 + 3 + 13 + 1 + 1) * 16;
    /// mode (style, smoothing, opacity, overlay), the colored wire, the
    /// live display-style colours and tuning (display_shading.rs), then
    /// `plain` (the Wireframe colour baked into edge vertices; w = 1 to
    /// recolour it) and `inactive` (the colour it becomes).
    fn bytes(&self) -> Vec<u8> {
        let wire = self.colored.to_array();
        let rgb = |c: Color32| {
            let a = c.to_array();
            [a[0], a[1], a[2]].map(|v| f32::from(v) / 255.)
        };
        let [pr, pg, pb] = rgb(crate::theme::viewport::wireframe());
        let [ir, ig, ib] = rgb(self.plain_wire.unwrap_or(Color32::BLACK));
        let plain = [pr, pg, pb, if self.plain_wire.is_some() { 1. } else { 0. }];
        let inactive = [ir, ig, ib, 1.];
        let overlay = match self.overlay {
            crate::view_display::Overlay::None => 0.,
            crate::view_display::Overlay::Uniform => 1.,
            crate::view_display::Overlay::Colored => 2.,
        };
        [
            self.style.code() as f32,
            if self.smoothing { 1. } else { 0. },
            self.opacity,
            overlay,
            f32::from(wire[0]) / 255.,
            f32::from(wire[1]) / 255.,
            f32::from(wire[2]) / 255.,
            1.,
        ]
        .into_iter()
        .chain(
            crate::display_shading::StyleColors::live()
                .uniform_rows()
                .into_iter()
                .flatten(),
        )
        .chain(plain)
        .chain(inactive)
        .chain(self.ambient)
        .chain([self.matcap.layer() as f32])
        .chain(transparency_rows(&self.transparency).into_iter().flatten())
        .chain(self.env.rows())
        .chain([
            // Wireframe-only groups never fade: a dense mesh would vanish.
            if self.fade_small_wire && self.fills() {
                1.
            } else {
                0.
            },
            WIRE_FADE_GONE,
            WIRE_FADE_FULL,
            WIRE_FADE_SHARP.to_radians().cos(),
        ])
        .chain([
            if self.glass_on() { 1. } else { 0. },
            if self.checker { 1. } else { 0. },
            if self.weights { 1. } else { 0. },
            if self.distortion { 1. } else { 0. },
        ])
        .flat_map(|f| f.to_le_bytes())
        .collect()
    }
}
/// The Look uniform's transparency rows (VP-80): front colour and opacity,
/// back colour and opacity, then (mode: 0 Normal, 1 Ghost, 2 X-Ray; Flat
/// Shaded; Overlay; 0).
pub fn transparency_rows(t: &crate::view_display::Transparency) -> [[f32; 4]; 3] {
    use crate::view_display::TransparencyMode;
    let row = |c: [u8; 3], opacity: u8| {
        let [r, g, b] = c.map(|v| f32::from(v) / 255.);
        [r, g, b, f32::from(opacity.min(100)) / 100.]
    };
    let mode = match t.mode {
        TransparencyMode::Normal => 0.,
        TransparencyMode::Ghost => 1.,
        TransparencyMode::XRay => 2.,
    };
    let flag = |on: bool| if on { 1. } else { 0. };
    [
        row(t.front_color, t.front_opacity),
        row(t.back_color, t.back_opacity),
        [mode, flag(t.flat), flag(t.overlay), 0.],
    ]
}
/// Which look an item draws with in a view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookGroup {
    Active = 0,
    Inactive = 1,
    /// Make Inactive Invisible: kept on the graphics card (other views
    /// may show it) but not drawn here.
    Hidden = 2,
}
/// One item to draw: (item, generation, geometry, blocks changed since the
/// last upload, its look, the matrix its cached geometry is drawn through:
/// identity unless a drag moved the item since the geometry was built).
pub type WorldItem = (
    u64,
    u64,
    std::sync::Arc<crate::gpu_scene::ItemGeometry>,
    Option<crate::gpu_scene::Patch>,
    LookGroup,
    glam::Mat4,
);
/// What one view draws from the meshes kept on the graphics card
/// (gpu_scene.rs): the camera, each item's cached geometry (by generation)
/// and this frame's small overlay (rollover).
/// One mesh's point dots: (item, generation, dots, matrix, first selected
/// dot vertex for the x-ray).
pub type WorldDots = (
    u64,
    u64,
    std::sync::Arc<Vec<crate::gpu_scene::DotVertex>>,
    glam::Mat4,
    u32,
);
pub struct WorldDraw {
    pub camera: [f32; 24],
    /// The active and inactive groups' looks.
    pub looks: [Look; 2],
    pub items: Vec<WorldItem>,
    pub overlay: crate::gpu_scene::ItemGeometry,
    /// The look of the item under the rollover.
    pub overlay_look: LookGroup,
    /// Each active mesh's point dots: (item, generation, dots, matrix,
    /// first dot vertex that is a selected point for the x-ray; selected
    /// dots come last, `u32::MAX` for none).
    pub dots: Vec<WorldDots>,
    /// Show UV Seam: (item, generation, strokes, matrix) per shown item.
    pub seams: Vec<(
        u64,
        u64,
        std::sync::Arc<Vec<crate::gpu_scene::EdgeVertex>>,
        glam::Mat4,
    )>,
    /// The light the Advanced style is lit by, and whether this view draws
    /// its shadows (an Advanced group shown, Show Shadows on, the light
    /// casts).
    pub key_light: KeyLight,
    pub shadows: bool,
    /// Selection x-ray (Preferences > Selection > Transparency: Selection):
    /// the items (the active meshes) whose hidden selection shows through
    /// surfaces. The opacity is `camera[23]`; 0 draws no x-ray.
    pub xray_items: Vec<u64>,
    /// Selected edges' strokes for the x-ray (world space).
    pub xray_edges: Vec<crate::gpu_scene::EdgeVertex>,
    /// Show Selected Outline (Viewport Properties > Visibility): the
    /// selected polygons' wire in the selection colour draws.
    pub outline: bool,
    /// Items drawn whole selected (Item mode) from geometry built
    /// unselected (`Selected`).
    pub whole_selected: std::collections::BTreeSet<u64>,
    /// Items pre-highlighted whole (Item mode rollover), drawn as a whole
    /// selected item is but in the rollover colours; a selected item
    /// shows selected.
    pub whole_rollover: std::collections::BTreeSet<u64>,
    /// Image layers drawn per pixel (gpu_scene.rs `GpuMaterials`).
    pub materials: Option<std::sync::Arc<crate::gpu_scene::GpuMaterials>>,
    /// HDRI stage 3: the picture whose blurred copy the Advanced style's
    /// reflections see (the looks' `env` say whether they use it).
    pub env_picture: Option<std::sync::Arc<crate::environment_gpu::SkyPicture>>,
    /// MT7 Topology Mode (topology_mode.rs): the active group's wire and
    /// the active meshes' points draw over everything (no depth test).
    pub topology: bool,
}
/// The Advanced style's light: the view's own light (camera-fixed), or
/// with Show Scene Lighting the first rendering scene light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyLight {
    /// Unit vector toward the light (directional and area lights, and the
    /// view light).
    Toward([f32; 3]),
    /// A light at this world position (point and spot lights): it shines
    /// toward the drawn meshes' centre.
    At([f32; 3]),
}
impl Default for KeyLight {
    fn default() -> Self {
        // The view light of a Front view (view space is world space).
        KeyLight::Toward(crate::display_shading::HEADLIGHT)
    }
}
/// wgpu's instance flags: its build defaults and WGPU_* variables, but on
/// Windows without `DEBUG` unless `WGPU_DEBUG=1` asks for it. `DEBUG` makes
/// DirectX 12 compile shaders unoptimised, and FXC then aborts on the
/// procedural pattern shaders ("internal error: compilation aborted
/// unexpectedly"). It only changes development builds; releases leave
/// `DEBUG` off anyway.
pub fn instance_flags() -> wgpu::InstanceFlags {
    let mut flags = wgpu::InstanceFlags::from_build_config();
    if cfg!(windows) {
        flags.remove(wgpu::InstanceFlags::DEBUG);
    }
    flags.with_env()
}
/// The graphics setup the app asks for: egui's, with the adapter's own
/// buffer limits instead of WebGPU's 256 MiB defaults, so a subdivision
/// surface's patches (subd_gpu.rs, ~1.8 GB for 145M triangles) fit in
/// single storage buffers. Only raises limits the adapter has.
pub fn wgpu_configuration() -> egui_wgpu::WgpuConfiguration {
    let mut config = egui_wgpu::WgpuConfiguration::default();
    if let egui_wgpu::WgpuSetup::CreateNew(create) = &mut config.wgpu_setup {
        create.instance_descriptor.flags = instance_flags();
        let base = create.device_descriptor.clone();
        create.device_descriptor = std::sync::Arc::new(move |adapter| {
            let mut descriptor = base(adapter);
            let have = adapter.limits();
            let want = &mut descriptor.required_limits;
            want.max_buffer_size = want.max_buffer_size.max(have.max_buffer_size);
            want.max_storage_buffer_binding_size = want
                .max_storage_buffer_binding_size
                .max(have.max_storage_buffer_binding_size);
            // The patch shaders read a dozen lists in the vertex stage.
            want.max_storage_buffers_per_shader_stage = want
                .max_storage_buffers_per_shader_stage
                .max(have.max_storage_buffers_per_shader_stage);
            descriptor
        });
        create.native_adapter_selector = Some(std::sync::Arc::new(crate::gpu_backend::select));
    }
    config
}
/// Shadow map resolution (square). One map shared by every view.
const SHADOW_SIZE: u32 = 2048;
/// The Advanced style's shadow map: a depth image of the shown meshes seen
/// from the key light, orthographic, fitted round their bounds.
struct ShadowMap {
    size: u32,
    texture: wgpu::Texture,
    /// The uniform the fill shader (scene group 2) and the shadow pass
    /// read (light matrix only).
    uniform: wgpu::Buffer,
    pass_group: wgpu::BindGroup,
}
/// The shadow uniform: light matrix, key (toward the light, w = shadows
/// on), params (texel size in uv, normal offset in world units, depth bias).
const SHADOW_UNIFORM_SIZE: u64 = 64 + 16 + 16;
/// World bounds of a list of fill corners.
fn bounds_of(vertices: &[crate::gpu_scene::WorldVertex]) -> Option<([f32; 3], [f32; 3])> {
    let mut it = vertices.iter().map(|v| v.position);
    let first = it.next()?;
    Some(it.fold((first, first), |(lo, hi), p| {
        (
            [lo[0].min(p[0]), lo[1].min(p[1]), lo[2].min(p[2])],
            [hi[0].max(p[0]), hi[1].max(p[1]), hi[2].max(p[2])],
        )
    }))
}
/// Bounds moved through `m`: the box round its eight moved corners.
fn moved_bounds(
    bounds: Option<([f32; 3], [f32; 3])>,
    m: glam::Mat4,
) -> Option<([f32; 3], [f32; 3])> {
    let (lo, hi) = bounds?;
    if m == glam::Mat4::IDENTITY {
        return bounds;
    }
    (0..8)
        .map(|k| {
            let pick = |axis: usize| {
                if k & (1 << axis) == 0 {
                    lo[axis]
                } else {
                    hi[axis]
                }
            };
            let p = m.transform_point3(glam::Vec3::new(pick(0), pick(1), pick(2)));
            Some((p.to_array(), p.to_array()))
        })
        .fold(None, union)
}
/// The per-item uniform (group 3 of the resident shader, group 1 of the
/// shadow pass): the matrix cached geometry is drawn through and its
/// inverse transpose for normals, then for material programs the inverse
/// of the placement the geometry was built at (stored point → the item's
/// own point) and the transpose of the whole placement (drawn normal →
/// the item's own normal).
struct Placement {
    buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
    matrix: glam::Mat4,
    built: glam::DMat4,
    /// What the shader draws in place of the cached colours
    /// (`Selected`); written after the matrices.
    selected: Selected,
}
/// A whole selected item, drawn from geometry built unselected: the fill
/// takes the selection fill (as `gpu_scene::Source::base` gives every
/// polygon of a selected item) and, through the highlight placement, the
/// wire draws as the selection highlight. Switching into Item mode or
/// selecting an item then changes a few uniform bytes instead of rebuilding
/// and re-uploading the whole mesh.
#[derive(Clone, Copy, PartialEq)]
struct Selected {
    /// Fill as selected.
    fill: bool,
    /// Strokes as the selection highlight.
    wire: bool,
    /// Mesh wire, which Fade Wireframe on Small Polygons may fade (not the
    /// views' overlays, which share the edge shader).
    mesh: bool,
    fill_colour: Color32,
    wire_colour: Color32,
}
impl Selected {
    const NONE: Selected = Selected {
        fill: false,
        wire: false,
        mesh: false,
        fill_colour: Color32::TRANSPARENT,
        wire_colour: Color32::TRANSPARENT,
    };
    fn bytes(&self) -> Vec<u8> {
        let unit = |c: Color32| c.to_array().map(|v| f32::from(v) / 255.);
        [
            f32::from(u8::from(self.fill)),
            f32::from(u8::from(self.wire)),
            f32::from(u8::from(self.mesh)),
            0.,
        ]
        .into_iter()
        .chain(unit(self.fill_colour))
        .chain(unit(self.wire_colour))
        .flat_map(f32::to_le_bytes)
        .collect()
    }
}
impl Placement {
    /// Four matrices, then `Selected`'s three rows.
    const SIZE: u64 = 256 + 48;
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Operandi item placement"),
            size: Self::SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Operandi item placement"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        let placement = Self {
            buffer,
            group,
            matrix: glam::Mat4::NAN,
            built: glam::DMat4::IDENTITY,
            selected: Selected::NONE,
        };
        let mut placement = placement;
        placement.set(queue, glam::Mat4::IDENTITY, glam::DMat4::IDENTITY);
        queue.write_buffer(&placement.buffer, 256, &Selected::NONE.bytes());
        placement
    }
    /// Write `selected` when it changed.
    fn select(&mut self, queue: &wgpu::Queue, selected: Selected) {
        if self.selected != selected {
            self.selected = selected;
            queue.write_buffer(&self.buffer, 256, &selected.bytes());
        }
    }
    /// Write `m` (and the build placement `built`) when either changed.
    fn set(&mut self, queue: &wgpu::Queue, m: glam::Mat4, built: glam::DMat4) {
        if self.matrix.to_cols_array().map(f32::to_bits) == m.to_cols_array().map(f32::to_bits)
            && self.built.to_cols_array() == built.to_cols_array()
        {
            return;
        }
        self.matrix = m;
        self.built = built;
        let normal = glam::Mat4::from_mat3(glam::Mat3::from_mat4(m).inverse().transpose());
        let local = if built.determinant().abs() > 1e-300 {
            built.inverse().as_mat4()
        } else {
            glam::Mat4::IDENTITY
        };
        let whole = m.as_dmat4() * built;
        let to_local_normal =
            glam::Mat4::from_mat3(glam::DMat3::from_mat4(whole).transpose().as_mat3());
        let bytes: Vec<u8> = m
            .to_cols_array()
            .into_iter()
            .chain(normal.to_cols_array())
            .chain(local.to_cols_array())
            .chain(to_local_normal.to_cols_array())
            .flat_map(f32::to_le_bytes)
            .collect();
        queue.write_buffer(&self.buffer, 0, &bytes);
    }
}
/// How the items using a shared placement draw (`ViewportGpu::shared`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Shared {
    /// Plain mesh fill and wire.
    Mesh,
    /// A whole selected item's fill (`Selected::fill`).
    WholeFill,
    /// A whole selected item's wire as its outline (`Selected::wire`).
    WholeOutline,
    /// A whole pre-highlighted item's fill (Item mode rollover).
    RolloverFill,
    /// A whole pre-highlighted item's wire as its outline.
    RolloverOutline,
}
/// A matrix as a map key, bit for bit.
fn matrix_key(m: glam::Mat4) -> [u32; 16] {
    m.to_cols_array().map(f32::to_bits)
}
/// The placement uniform's bind group layout.
fn placement_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Operandi item placement"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}
fn union(
    a: Option<([f32; 3], [f32; 3])>,
    b: Option<([f32; 3], [f32; 3])>,
) -> Option<([f32; 3], [f32; 3])> {
    match (a, b) {
        (Some((alo, ahi)), Some((blo, bhi))) => Some((
            [alo[0].min(blo[0]), alo[1].min(blo[1]), alo[2].min(blo[2])],
            [ahi[0].max(bhi[0]), ahi[1].max(bhi[1]), ahi[2].max(bhi[2])],
        )),
        (a, None) => a,
        (None, b) => b,
    }
}
/// The light's view-projection for the shadow map and the unit vector
/// toward the light: orthographic, looking at the bounds' centre from
/// outside their bounding sphere, covering the sphere.
pub fn shadow_matrix(key: KeyLight, lo: [f32; 3], hi: [f32; 3]) -> (glam::Mat4, glam::Vec3) {
    use glam::Vec3;
    let (lo, hi) = (Vec3::from(lo), Vec3::from(hi));
    let center = (lo + hi) * 0.5;
    let radius = ((hi - lo).length() * 0.5).max(1e-3);
    let toward = match key {
        KeyLight::Toward(d) => Vec3::from(d),
        KeyLight::At(p) => Vec3::from(p) - center,
    }
    .try_normalize()
    .unwrap_or(Vec3::Y);
    let eye = center + toward * radius * 2.;
    let up = if toward.y.abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let view = glam::Mat4::look_at_rh(eye, center, up);
    let projection =
        glam::Mat4::orthographic_rh(-radius, radius, -radius, radius, radius * 0.5, radius * 3.5);
    (projection * view, toward)
}
/// An item's geometry on the graphics card.
impl ResidentBuffers {
    /// The shared placement a whole item's fill draws through.
    fn fill_kind(&self) -> Shared {
        if self.rollover {
            Shared::RolloverFill
        } else {
            Shared::WholeFill
        }
    }
    /// The shared placement a whole item's outline draws through.
    fn outline_kind(&self) -> Shared {
        if self.rollover {
            Shared::RolloverOutline
        } else {
            Shared::WholeOutline
        }
    }
}
struct ResidentBuffers {
    generation: u64,
    /// The fill and wire, in the shared buffers for a small item
    /// (gpu_arena.rs), so many items draw in a few calls.
    fill: Store,
    fill_count: u32,
    edges: Store,
    edge_count: u32,
    /// Some fill corner is drawn by a material program, which reads the
    /// item's own placement (`Placement::local`).
    programs: bool,
    /// Some polygon is see-through (`ItemGeometry::clear`): the glass pass
    /// draws the item too.
    clear: bool,
    /// Some polygon has cut-outs (`ItemGeometry::cut`): lighter shadows.
    cut: bool,
    highlight: Option<VertexChunks>,
    highlight_count: u32,
    /// The fill's bounds as built (grown, never shrunk, by drag patches);
    /// `placement` moves them.
    bounds: Option<([f32; 3], [f32; 3])>,
    /// Its own placement, made when first needed: a whole selected item's
    /// fill or one drawn by a material program reads more than the matrix.
    placement: Option<Placement>,
    /// The item is drawn whole selected (or whole pre-highlighted) this
    /// frame.
    whole: bool,
    /// Whole pre-highlighted rather than selected: the rollover colours.
    rollover: bool,
    /// Where the cached geometry draws this frame (identity unless a drag
    /// moved the item since it was built). Items that need nothing else of
    /// their placement draw through a shared one for this matrix
    /// (`ViewportGpu::shared`), so a drag of thousands of items writes one
    /// uniform, not thousands, and their draws join.
    matrix: glam::Mat4,
    /// The placement the geometry was built at (`ItemGeometry::built`).
    built: glam::DMat4,
    /// Subdivision polygons drawn as patches (subd_gpu.rs).
    patches: Option<crate::subd_gpu::PatchGpu>,
}
/// Point dots or UV seam strokes of one item on the graphics card.
struct Strokes {
    generation: u64,
    buffer: VertexChunks,
    count: u32,
    placement: Placement,
}
pub struct ViewportGpu {
    pub(crate) state: egui_wgpu::RenderState,
    /// The outline pass may bind the depth buffer read-only. Not on
    /// DirectX 12: there that pass's command list fails to close
    /// (E_INVALIDARG) and the device is lost on the first frame (Windows
    /// VM, 2026-11-23), so it loads and stores depth instead. Its
    /// pipelines never write depth, so the picture is the same.
    read_only_depth: bool,
    triangles: wgpu::RenderPipeline,
    outlines: wgpu::RenderPipeline,
    world_fill: wgpu::RenderPipeline,
    /// Transparency Overrides' see-through fill, and its Overlay variant
    /// that draws over everything (VP-80).
    world_clear: wgpu::RenderPipeline,
    world_clear_over: wgpu::RenderPipeline,
    /// Viewport transparency's passes for the plain shader (`glass_pipelines`).
    world_glass: [wgpu::RenderPipeline; GLASS_PASSES],
    world_overlay: wgpu::RenderPipeline,
    world_edges: wgpu::RenderPipeline,
    world_dots: wgpu::RenderPipeline,
    /// Topology Mode's wire and points, drawn over everything (MT7).
    topology_edges: wgpu::RenderPipeline,
    topology_dots: wgpu::RenderPipeline,
    /// Selection x-ray: the hidden parts of selected strokes and points.
    xray_edges: wgpu::RenderPipeline,
    xray_dots: wgpu::RenderPipeline,
    shadow_pass: wgpu::RenderPipeline,
    /// See-through casters' lighter shadows (`shadow_clear_pipeline`).
    shadow_clear_pass: wgpu::RenderPipeline,
    shadow_clear_layout: wgpu::BindGroupLayout,
    shadow_sample_layout: wgpu::BindGroupLayout,
    shadow_pass_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    shadow: ShadowMap,
    /// Image layers' textures, parameters and fill pipelines
    /// (material_gpu.rs).
    materials: crate::material_gpu::MaterialGpu,
    /// Group 2 of the resident shader: the shadow map and the materials.
    /// The environment picture's blurred copy (HDRI stage 3).
    env_light: EnvLightGpu,
    /// What lies behind see-through surfaces, copied before they draw
    /// (refraction, VM-Q06).
    behind: BehindGpu,
    scene_group: wgpu::BindGroup,
    /// The same, through `patch_scene_layout`, for the patch pipelines.
    patch_scene_layout: wgpu::BindGroupLayout,
    patch_scene_group: wgpu::BindGroup,
    /// The Matcap style's matcaps (built-in and loaded), one array layer each.
    matcaps: MatcapGpu,
    /// The resident shader's pipeline layout (material fill pipelines).
    world_layout: wgpu::PipelineLayout,
    /// Point dots per item.
    dot_buffers: operandi_core::fxhash::FxHashMap<u64, Strokes>,
    /// Small items' fill and wire blocks (`ResidentBuffers::fill`, `edges`).
    fill_arena: Arena,
    edge_arena: Arena,
    /// UV seam strokes per item, the same way.
    seam_buffers: operandi_core::fxhash::FxHashMap<u64, Strokes>,
    placement_layout: wgpu::BindGroupLayout,
    /// The identity placement, for this frame's overlays (built at the
    /// item's current placement).
    identity: Placement,
    /// Shared placements by matrix (`ResidentBuffers::matrix`) for mesh
    /// fill and wire that need nothing else of their own placement.
    shared: std::collections::BTreeMap<([u32; 16], Shared), Placement>,
    camera_layout: wgpu::BindGroupLayout,
    look_layout: wgpu::BindGroupLayout,
    /// Camera uniform per view.
    cameras: std::collections::BTreeMap<u32, (wgpu::Buffer, wgpu::BindGroup)>,
    /// The active and inactive looks per view.
    looks: std::collections::BTreeMap<u32, [(wgpu::Buffer, wgpu::BindGroup); 2]>,
    /// Geometry per item, shared by every view.
    resident: operandi_core::fxhash::FxHashMap<u64, ResidentBuffers>,
    samples: u32,
    grid: crate::grid::Renderer,
    backdrops: crate::backdrop_gpu::Renderer,
    sky: crate::environment_gpu::Renderer,
    /// One render target per view (views.rs), by view key.
    targets: std::collections::BTreeMap<u32, Target>,
    /// Bytes of cached mesh geometry sent to the graphics card so far
    /// (Window > Frame Time).
    uploaded: u64,
    /// Subdivision patches' pipelines (subd_gpu.rs).
    /// `None` on a device below what patches need (`subd_gpu::supported`):
    /// subdivision then draws the ordinary way.
    patch_pipelines: Option<crate::subd_gpu::PatchPipelines>,
}
impl ViewportGpu {
    pub fn new(state: egui_wgpu::RenderState) -> Self {
        let device = &state.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Operandi viewport shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct VertexOut { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32> };
@vertex fn vs(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> VertexOut {
 var out: VertexOut; out.position=vec4<f32>(position,1.0); out.color=color; return out;
}
@fragment fn fs(in:VertexOut)->@location(0) vec4<f32>{return in.color;}
"#
                .into(),
            ),
        });
        let attributes = wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x4];
        let msaa = wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4;
        let samples = if [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Depth32Float,
        ]
        .iter()
        .all(|&f| {
            state
                .adapter
                .get_texture_format_features(f)
                .flags
                .contains(msaa)
        }) {
            4
        } else {
            1
        };
        let make = |depth_write_enabled| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Operandi depth-tested viewport"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 28,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &attributes,
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: if depth_write_enabled {
                        Default::default()
                    } else {
                        wgpu::DepthBiasState {
                            constant: -2,
                            slope_scale: -1.,
                            clamp: 0.,
                        }
                    },
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                multiview: None,
                cache: None,
            })
        };
        let triangles = make(true);
        let outlines = make(false);
        let grid = crate::grid::Renderer::new(device, samples);
        let backdrops = crate::backdrop_gpu::Renderer::new(device, samples);
        let sky = crate::environment_gpu::Renderer::new(device, samples);
        let placement_layout = placement_layout(device);
        let (shadow_sample_layout, patch_scene_layout, shadow_pass_layout, shadow_pass) =
            shadow_pipeline(device, &placement_layout);
        let (shadow_clear_layout, shadow_clear_pass) =
            shadow_clear_pipeline(device, &shadow_pass_layout, &placement_layout);
        let WorldPipelines {
            camera_layout,
            look_layout,
            fill: world_fill,
            overlay: world_overlay,
            edges: world_edges,
            dots: world_dots,
            topology_edges,
            topology_dots,
            xray_edges,
            xray_dots,
            clear: world_clear,
            clear_over: world_clear_over,
            glass: world_glass,
            layout: world_layout,
        } = world_pipelines(device, samples, &shadow_sample_layout, &placement_layout);
        let identity = Placement::new(device, &state.queue, &placement_layout);

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Operandi shadow compare"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        // A 1×1 map until a view first draws shadows.
        let shadow = ShadowMap::new(device, 1, &shadow_pass_layout);
        let materials = crate::material_gpu::MaterialGpu::new(device, &state.queue);
        let matcaps = MatcapGpu::new(device, &state.queue);
        let env_light = EnvLightGpu::new(device, &state.queue);
        let behind = BehindGpu::new(device, [1, 1]);
        let patch_scene_group = scene_group(
            device,
            &patch_scene_layout,
            &shadow,
            &shadow_sampler,
            &materials,
            &matcaps,
            (&env_light, &behind),
        );
        let scene_group = scene_group(
            device,
            &shadow_sample_layout,
            &shadow,
            &shadow_sampler,
            &materials,
            &matcaps,
            (&env_light, &behind),
        );
        let patch_pipelines = crate::subd_gpu::supported(&device.limits()).then(|| {
            crate::subd_gpu::PatchPipelines::new(
                device,
                samples,
                &plain_world_shader(),
                [&camera_layout, &look_layout, &patch_scene_layout],
            )
        });
        Self {
            read_only_depth: state.adapter.get_info().backend != wgpu::Backend::Dx12,
            patch_pipelines,
            shadow_pass,
            shadow_clear_pass,
            shadow_clear_layout,
            shadow_sample_layout,
            shadow_pass_layout,
            shadow_sampler,
            shadow,
            materials,
            matcaps,
            env_light,
            behind,
            scene_group,
            patch_scene_layout,
            patch_scene_group,
            world_layout,
            grid,
            backdrops,
            sky,
            state,
            triangles,
            outlines,
            world_fill,
            world_overlay,
            world_edges,
            world_dots,
            topology_edges,
            topology_dots,
            xray_edges,
            xray_dots,
            world_clear,
            world_clear_over,
            world_glass,
            dot_buffers: Default::default(),
            fill_arena: Arena::new("Operandi shared fill", FILL_STRIDE),
            edge_arena: Arena::new("Operandi shared wire", EDGE_STRIDE),
            seam_buffers: Default::default(),
            placement_layout,
            identity,
            shared: Default::default(),
            camera_layout,
            look_layout,
            cameras: Default::default(),
            looks: Default::default(),
            resident: Default::default(),
            samples,
            targets: Default::default(),
            uploaded: 0,
        }
    }
    /// Writes a stroke's changed rectangle into painted picture `key`'s
    /// texture (`MaterialGpu::patch`); `false` when it isn't on the card.
    #[cfg(feature = "painting")]
    pub fn patch_painted(&mut self, key: u64, rect: (u32, u32, u32, u32), rgba: &[u8]) -> bool {
        self.materials.patch(&self.state.queue, key, rect, rgba)
    }
    /// Uploads and rectangle writes of image layers so far (speed guards).
    #[cfg(all(test, feature = "painting"))]
    pub fn image_traffic(&self) -> (u64, u64) {
        (self.materials.uploads, self.materials.patches)
    }
    /// Items drawing patches (tests).
    #[cfg(test)]
    pub fn patched_items(&self) -> usize {
        self.resident
            .values()
            .filter(|b| b.patches.is_some())
            .count()
    }
    /// Whether `net`'s patches fit this device's buffer limits.
    pub fn patches_fit(&self, net: &operandi_core::subdivision::patches::PatchNet) -> bool {
        self.patch_pipelines.is_some() && crate::subd_gpu::fits(&self.state.device, net)
    }
    /// The patch pipelines; only asked for where patches exist, which
    /// needs them (`patches_fit`).
    fn patch_pipelines(&self) -> &crate::subd_gpu::PatchPipelines {
        self.patch_pipelines
            .as_ref()
            .expect("patches are only made where their pipelines exist")
    }
    /// The views' last pictures, RGBA8 rows, by view (tests compare them).
    #[cfg(test)]
    pub fn read_views(&self) -> Vec<(u32, [u32; 2], Vec<u8>)> {
        let device = &self.state.device;
        let mut out = Vec::new();
        for (&view, target) in &self.targets {
            let [w, h] = target.size;
            let padded = (w * 4).div_ceil(256) * 256;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Operandi view readback"),
                size: u64::from(padded) * u64::from(h),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_texture_to_buffer(
                target.color.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            self.state.queue.submit([encoder.finish()]);
            buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            let mapped = buffer.slice(..).get_mapped_range();
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for row in mapped.chunks(padded as usize) {
                rgba.extend_from_slice(&row[..(w * 4) as usize]);
            }
            drop(mapped);
            out.push((view, [w, h], rgba));
        }
        out
    }
    /// Bytes of cached mesh geometry uploaded so far (Frame Time).
    pub fn uploaded_bytes(&self) -> u64 {
        self.uploaded
    }
    /// Upload items whose geometry changed and drop ones no longer drawn.
    fn sync_resident(&mut self, world: &WorldDraw) {
        self.sync_materials(world);
        for look in &world.looks {
            self.matcaps.ensure(&self.state.queue, look.matcap);
        }
        let device = &self.state.device;
        let queue = &self.state.queue;
        let layout = &self.placement_layout;
        let uploaded = std::cell::Cell::new(0u64);
        // Upload strokes on a new generation; set their placement always.
        let strokes = |map: &mut operandi_core::fxhash::FxHashMap<u64, Strokes>,
                       id: u64,
                       generation: u64,
                       count: usize,
                       matrix: glam::Mat4,
                       stride: u64,
                       bytes: &dyn Fn() -> Vec<u8>,
                       label: &str| {
            if map.get(&id).is_none_or(|b| b.generation != generation) {
                let bytes = bytes();
                uploaded.set(uploaded.get() + bytes.len() as u64);
                let buffer =
                    VertexChunks::new(device, label, &bytes, stride, wgpu::BufferUsages::VERTEX);
                let placement = map
                    .remove(&id)
                    .map(|b| b.placement)
                    .unwrap_or_else(|| Placement::new(device, queue, layout));
                map.insert(
                    id,
                    Strokes {
                        generation,
                        buffer,
                        count: count as u32,
                        placement,
                    },
                );
            }
            if let Some(b) = map.get_mut(&id) {
                b.placement.set(queue, matrix, glam::DMat4::IDENTITY);
            }
        };
        for (id, generation, dots, matrix, _) in &world.dots {
            strokes(
                &mut self.dot_buffers,
                *id,
                *generation,
                dots.len(),
                *matrix,
                DOT_STRIDE,
                &|| dot_bytes(dots),
                "Operandi resident points",
            );
        }
        for (id, generation, seams, matrix) in &world.seams {
            strokes(
                &mut self.seam_buffers,
                *id,
                *generation,
                seams.len(),
                *matrix,
                EDGE_STRIDE,
                &|| edge_bytes(seams),
                "Operandi resident UV seams",
            );
        }
        for (id, generation, geometry, patch, _, matrix) in &world.items {
            let current = self.resident.get(id).map(|b| b.generation);
            if current == Some(*generation) {
                if let Some(b) = self.resident.get_mut(id) {
                    b.matrix = *matrix;
                }
                continue;
            }
            let new_highlight = || {
                let highlight_bytes = edge_bytes(&geometry.highlight);
                uploaded.set(uploaded.get() + highlight_bytes.len() as u64);
                (!highlight_bytes.is_empty()).then(|| {
                    VertexChunks::new(
                        device,
                        "Operandi resident highlight",
                        &highlight_bytes,
                        EDGE_STRIDE,
                        // COPY_DST: a drag rewrites moved polygons' sides.
                        wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    )
                })
            };
            // A drag: rewrite only the changed polygons' blocks in place.
            if let Some(patch) = patch
                && current == Some(patch.from)
                && let Some(b) = self.resident.get_mut(id)
            {
                // Adjacent blocks go up as one write: a large patch was
                // thousands of tiny writes a frame, which slowed long drags.
                let fill_ranges = merged(&patch.fill);
                for range in &fill_ranges {
                    b.bounds = union(b.bounds, bounds_of(&geometry.fill[range.clone()]));
                    b.programs |= geometry.fill[range.clone()].iter().any(|v| v.program != 0);
                }
                let sent = upload_ranges(
                    &b.fill,
                    &self.fill_arena,
                    queue,
                    &geometry.fill,
                    &fill_ranges,
                    FILL_STRIDE,
                    fill_vertex,
                );
                uploaded.set(uploaded.get() + sent);
                b.clear = geometry.clear;
                b.cut = geometry.cut;
                let sent = upload_ranges(
                    &b.edges,
                    &self.edge_arena,
                    queue,
                    &geometry.edges,
                    &merged(&patch.edges),
                    EDGE_STRIDE,
                    edge_vertex,
                );
                uploaded.set(uploaded.get() + sent);
                if patch.highlight {
                    b.highlight = new_highlight();
                    b.highlight_count = geometry.highlight.len() as u32;
                } else if let Some(buffer) = &b.highlight {
                    // Only the moved selected polygons' sides.
                    for range in merged(&patch.highlight_ranges) {
                        let bytes = edge_bytes(&geometry.highlight[range.clone()]);
                        uploaded.set(uploaded.get() + bytes.len() as u64);
                        buffer.write(queue, range.start as u32, &bytes);
                    }
                }
                // Patches the update can't take (a crease changed their
                // topology, UVs appeared) are made again before the new
                // generation counts as uploaded (Astra P6-001: they stayed
                // stale while marked current).
                match (&mut b.patches, &geometry.patches) {
                    (Some(p), Some(draw)) => match p.update(queue, draw, b.built) {
                        Some(sent) => uploaded.set(uploaded.get() + sent),
                        None => {
                            let pipelines = self
                                .patch_pipelines
                                .as_ref()
                                .expect("patches are only made where their pipelines exist");
                            let (gpu, sent) =
                                crate::subd_gpu::PatchGpu::new(device, pipelines, draw, b.built);
                            uploaded.set(uploaded.get() + sent);
                            b.patches = Some(gpu);
                        }
                    },
                    (None, Some(draw)) => {
                        let pipelines = self
                            .patch_pipelines
                            .as_ref()
                            .expect("patches are only made where their pipelines exist");
                        let (gpu, sent) =
                            crate::subd_gpu::PatchGpu::new(device, pipelines, draw, b.built);
                        uploaded.set(uploaded.get() + sent);
                        b.patches = Some(gpu);
                    }
                    (Some(_), None) => b.patches = None,
                    (None, None) => {}
                }
                b.generation = *generation;
                b.matrix = *matrix;
                continue;
            }
            let highlight = new_highlight();
            let fill = fill_bytes(&geometry.fill);
            let edges = edge_bytes(&geometry.edges);
            uploaded.set(uploaded.get() + (fill.len() + edges.len()) as u64);
            let (placement, old_patches) = match self.resident.remove(id) {
                Some(old) => {
                    old.fill.release(&mut self.fill_arena);
                    old.edges.release(&mut self.edge_arena);
                    (old.placement, old.patches)
                }
                None => (None, None),
            };
            // Patches: the same net only rewrites its looks and placement.
            let patches = geometry.patches.as_ref().map(|draw| {
                if let Some(mut old) = old_patches
                    && let Some(sent) = old.update(queue, draw, geometry.built)
                {
                    uploaded.set(uploaded.get() + sent);
                    return old;
                }
                let (gpu, sent) = crate::subd_gpu::PatchGpu::new(
                    device,
                    self.patch_pipelines(),
                    draw,
                    geometry.built,
                );
                uploaded.set(uploaded.get() + sent);
                gpu
            });
            self.resident.insert(
                *id,
                ResidentBuffers {
                    generation: *generation,
                    fill: Store::upload(
                        &mut self.fill_arena,
                        device,
                        queue,
                        "Operandi resident fill",
                        &fill,
                    ),
                    fill_count: geometry.fill.len() as u32,
                    edges: Store::upload(
                        &mut self.edge_arena,
                        device,
                        queue,
                        "Operandi resident edges",
                        &edges,
                    ),
                    edge_count: geometry.edges.len() as u32,
                    programs: geometry.fill.iter().any(|v| v.program != 0),
                    clear: geometry.clear,
                    cut: geometry.cut,
                    highlight,
                    highlight_count: geometry.highlight.len() as u32,
                    bounds: bounds_of(&geometry.fill),
                    placement,
                    whole: false,
                    rollover: false,
                    matrix: *matrix,
                    built: geometry.built,
                    patches,
                },
            );
        }
        // Whole selected items: the selection is a uniform, not geometry.
        let fill_colour = crate::theme::viewport::selection_fill();
        let wire_colour = crate::theme::viewport::selection();
        let selected_as = |fill: bool, wire: bool| Selected {
            fill,
            wire,
            mesh: true,
            fill_colour,
            wire_colour,
        };
        // Item mode's rollover: the same look in the rollover colours.
        let rollover_as = |fill: bool, wire: bool| Selected {
            fill,
            wire,
            mesh: true,
            fill_colour: crate::theme::viewport::rollover_face(),
            wire_colour: crate::theme::viewport::rollover(),
        };
        for (id, ..) in &world.items {
            let Some(b) = self.resident.get_mut(id) else {
                continue;
            };
            let selected = world.whole_selected.contains(id);
            b.rollover = !selected && world.whole_rollover.contains(id);
            b.whole = selected || b.rollover;
            // The item's own placement: only items whose material programs
            // read it (`Placement::local`).
            if b.programs {
                let p = b
                    .placement
                    .get_or_insert_with(|| Placement::new(device, queue, layout));
                p.set(queue, b.matrix, b.built);
                p.select(
                    queue,
                    if b.rollover {
                        rollover_as(true, false)
                    } else if b.whole {
                        selected_as(true, false)
                    } else {
                        Selected {
                            mesh: true,
                            ..Selected::NONE
                        }
                    },
                );
            }
        }
        // One shared placement per matrix and way of drawing the items use
        // (`Shared`): selecting every item, or dragging them all, writes a
        // few uniforms instead of one per item.
        let mut used = std::collections::BTreeSet::new();
        for (id, ..) in &world.items {
            if let Some(b) = self.resident.get(id) {
                let m = matrix_key(b.matrix);
                used.insert((m, Shared::Mesh));
                if b.whole {
                    used.insert((m, b.fill_kind()));
                    used.insert((m, b.outline_kind()));
                }
            }
        }
        self.shared.retain(|key, _| used.contains(key));
        for key in used {
            let p = self
                .shared
                .entry(key)
                .or_insert_with(|| Placement::new(device, queue, layout));
            p.set(
                queue,
                glam::Mat4::from_cols_array(&key.0.map(f32::from_bits)),
                glam::DMat4::IDENTITY,
            );
            p.select(
                queue,
                match key.1 {
                    Shared::Mesh => Selected {
                        mesh: true,
                        ..Selected::NONE
                    },
                    Shared::WholeFill => selected_as(true, false),
                    Shared::WholeOutline => selected_as(true, true),
                    Shared::RolloverFill => rollover_as(true, false),
                    Shared::RolloverOutline => rollover_as(true, true),
                },
            );
        }
        // Patches draw through the same shared placements (subd_gpu.rs).
        for (id, ..) in &world.items {
            let Some(b) = self.resident.get_mut(id) else {
                continue;
            };
            let kinds = [Shared::Mesh, b.fill_kind(), b.outline_kind()];
            let Some(p) = &mut b.patches else { continue };
            let m = matrix_key(b.matrix);
            for (slot, kind) in kinds.into_iter().enumerate() {
                if let Some(shared) = self.shared.get(&(m, kind)) {
                    p.bind_placement(
                        device,
                        &self
                            .patch_pipelines
                            .as_ref()
                            .expect("patches are only made where their pipelines exist")
                            .layout,
                        slot,
                        &shared.buffer,
                    );
                }
            }
        }
        self.uploaded += uploaded.get();
    }
    /// The placement an item's fill draws through: the shared identity one
    /// when it is where it was built, not drawn whole selected and has no
    /// material program reading its own placement, so neighbouring items
    /// can join one draw call; else its own.
    fn fill_placement<'a>(&'a self, b: &'a ResidentBuffers) -> &'a wgpu::BindGroup {
        match &b.placement {
            Some(own) if b.programs => &own.group,
            _ if b.whole => self.shared_placement(b, b.fill_kind()),
            _ => self.wire_placement(b),
        }
    }
    /// The shared placement for `b`'s matrix drawn as `kind`.
    fn shared_placement<'a>(&'a self, b: &'a ResidentBuffers, kind: Shared) -> &'a wgpu::BindGroup {
        // Every drawn item's matrix has them (`sync_resident`).
        self.shared
            .get(&(matrix_key(b.matrix), kind))
            .map_or(&self.identity.group, |p| &p.group)
    }
    /// The placement an item's plain wire (and shadow) draws through: the
    /// edge and shadow shaders read only the matrix.
    fn wire_placement<'a>(&'a self, b: &'a ResidentBuffers) -> &'a wgpu::BindGroup {
        self.shared_placement(b, Shared::Mesh)
    }
    /// Keep only these items' buffers (the scene's visible items).
    pub fn retain_resident(&mut self, items: &std::collections::BTreeSet<u64>) {
        let gone: Vec<u64> = self
            .resident
            .keys()
            .filter(|id| !items.contains(id))
            .copied()
            .collect();
        if gone.is_empty() && self.dot_buffers.len() + self.seam_buffers.len() == 0 {
            return;
        }
        for id in gone {
            if let Some(b) = self.resident.remove(&id) {
                b.fill.release(&mut self.fill_arena);
                b.edges.release(&mut self.edge_arena);
            }
        }
        self.dot_buffers.retain(|id, _| items.contains(id));
        self.seam_buffers.retain(|id, _| items.contains(id));
    }
    /// Keep only these Backdrop pictures' textures (the loaded ones).
    /// Keep only the environment pictures still drawn (by key).
    pub fn retain_skies(&mut self, keys: &std::collections::BTreeSet<u64>) {
        self.sky.retain(keys);
    }
    pub fn retain_backdrops(&mut self, keys: &std::collections::BTreeSet<u64>) {
        self.backdrops.retain(keys);
    }
    /// Free a closed view's render target.
    pub fn forget(&mut self, view: u32) {
        if let Some(old) = self.targets.remove(&view) {
            self.state.renderer.write().free_texture(&old.id);
        }
        self.cameras.remove(&view);
        self.looks.remove(&view);
    }
    pub fn sample_count(&self) -> u32 {
        self.samples
    }
    // One argument per layer of the view, drawn in this order.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        view: u32,
        size: [u32; 2],
        triangles: &[Vertex],
        edges: &[Vertex],
        grids: &[crate::grid::PlaneGrid],
        backdrops: &crate::backdrop_gpu::Frame,
        sky: Option<&crate::environment_gpu::SkyDraw>,
        world: Option<&WorldDraw>,
    ) -> TextureId {
        let size = size.map(|x| x.clamp(1, 8192));
        if let Some(world) = world {
            let key = world.env_picture.as_ref().map_or(0, |p| p.key);
            if key != self.env_light.key {
                self.env_light = match &world.env_picture {
                    Some(p) => EnvLightGpu::of(&self.state.device, &self.state.queue, p),
                    None => EnvLightGpu::new(&self.state.device, &self.state.queue),
                };
                self.rebuild_scene_group();
            }
            self.sync_resident(world);
            let bytes: Vec<u8> = world.camera.iter().flat_map(|f| f.to_le_bytes()).collect();
            if !self.cameras.contains_key(&view) {
                let buffer = self.state.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Operandi view camera"),
                    size: bytes.len() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let group = self
                    .state
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Operandi view camera"),
                        layout: &self.camera_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: buffer.as_entire_binding(),
                        }],
                    });
                self.cameras.insert(view, (buffer, group));
            }
            self.state
                .queue
                .write_buffer(&self.cameras[&view].0, 0, &bytes);
            if !self.looks.contains_key(&view) {
                let make = || {
                    let buffer = self.state.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Operandi view look"),
                        size: Look::SIZE,
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    let group = self
                        .state
                        .device
                        .create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("Operandi view look"),
                            layout: &self.look_layout,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: buffer.as_entire_binding(),
                            }],
                        });
                    (buffer, group)
                };
                self.looks.insert(view, [make(), make()]);
            }
            for (k, look) in world.looks.iter().enumerate() {
                self.state
                    .queue
                    .write_buffer(&self.looks[&view][k].0, 0, &look.bytes());
            }
        }
        let overlay_fill = world
            .map(|w| fill_bytes(&w.overlay.fill))
            .unwrap_or_default();
        let overlay_edges = world
            .map(|w| edge_bytes(&w.overlay.edges))
            .unwrap_or_default();
        let overlay_buffer = |label, bytes: &[u8], stride| {
            (!bytes.is_empty()).then(|| {
                VertexChunks::new(
                    &self.state.device,
                    label,
                    bytes,
                    stride,
                    wgpu::BufferUsages::VERTEX,
                )
            })
        };
        let overlay_fill_buffer =
            overlay_buffer("Operandi overlay fill", &overlay_fill, FILL_STRIDE);
        let overlay_edge_buffer =
            overlay_buffer("Operandi overlay edges", &overlay_edges, EDGE_STRIDE);
        let xray_edges = world
            .filter(|w| w.camera[23] > 0.)
            .map(|w| edge_bytes(&w.xray_edges))
            .unwrap_or_default();
        let xray_edge_buffer = overlay_buffer("Operandi x-ray edges", &xray_edges, EDGE_STRIDE);
        if self.targets.get(&view).is_none_or(|t| t.size != size) {
            if let Some(old) = self.targets.remove(&view) {
                self.state.renderer.write().free_texture(&old.id);
            }
            let make = |format, usage, sample_count| {
                self.state.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Operandi physical viewport target"),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
            };
            let color = make(
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                1,
            );
            let depth = make(
                wgpu::TextureFormat::Depth32Float,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                self.samples,
            );
            let multisampled = (self.samples > 1).then(|| {
                make(
                    wgpu::TextureFormat::Rgba8Unorm,
                    wgpu::TextureUsages::RENDER_ATTACHMENT,
                    self.samples,
                )
            });
            let id = self.state.renderer.write().register_native_texture(
                &self.state.device,
                &color.create_view(&Default::default()),
                wgpu::FilterMode::Linear,
            );
            self.targets.insert(
                view,
                Target {
                    size,
                    color,
                    depth,
                    multisampled,
                    id,
                },
            );
        }
        let buffer = |vertices: &[Vertex]| {
            let mut bytes = Vec::with_capacity(vertices.len() * 28);
            for v in vertices {
                for f in v.position {
                    bytes.extend_from_slice(&f.to_le_bytes());
                }
                // egui samples native textures as gamma-encoded RGBA, not sRGB-aware views.
                for f in v.color.to_array().map(|c| f32::from(c) / 255.) {
                    bytes.extend_from_slice(&f.to_le_bytes());
                }
            }
            VertexChunks::new(
                &self.state.device,
                "Operandi immutable render snapshot",
                &bytes,
                28,
                wgpu::BufferUsages::VERTEX,
            )
        };
        let grids: Vec<_> = grids
            .iter()
            .map(|g| self.grid.prepare(&self.state.device, g))
            .collect();
        let backdrops_camera = backdrops.camera;
        let backdrops = self
            .backdrops
            .prepare(&self.state.device, &self.state.queue, backdrops);
        // HDRI stage 2: the environment picture behind everything.
        let sky = sky.map(|s| {
            self.sky
                .prepare(&self.state.device, &self.state.queue, &backdrops_camera, s)
        });
        let tb = buffer(triangles);
        let eb = buffer(edges);
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&Default::default());
        if let Some(world) = world {
            self.shadow_pass(world, &mut encoder);
            // Subdivision patches: cull the clusters facing away from this
            // view before its passes draw the rest (subd_gpu.rs).
            let patched: Vec<_> = world
                .items
                .iter()
                .filter_map(|(id, ..)| self.resident.get(id))
                .filter_map(|b| b.patches.as_ref().map(|p| (b, p)))
                .collect();
            if !patched.is_empty() {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Operandi patch culling"),
                    timestamp_writes: None,
                });
                for (b, p) in patched {
                    let whole = b.matrix.as_dmat4() * b.built;
                    let to_item = if whole.determinant().abs() > 1e-300 {
                        whole.inverse()
                    } else {
                        glam::DMat4::IDENTITY
                    };
                    p.cull(
                        &self.state.queue,
                        self.patch_pipelines(),
                        &mut pass,
                        to_item,
                        &world.camera,
                    );
                }
            }
        }
        // Refraction (VM-Q09): see-through surfaces need the picture behind
        // them, copied once the solid ones have drawn.
        let refract = world.is_some_and(|world| {
            world.items.iter().any(|(id, _, _, _, group, _)| {
                let b = &self.resident[id];
                shown(*group)
                    .is_some_and(|k| b.clear && b.fill_count > 0 && world.looks[k].glass_on())
            })
        });
        let size = self.targets[&view].size;
        if refract && (self.behind.size[0] < size[0] || self.behind.size[1] < size[1]) {
            let grown = [
                self.behind.size[0].max(size[0]),
                self.behind.size[1].max(size[1]),
            ];
            self.behind = BehindGpu::new(&self.state.device, grown);
            self.rebuild_scene_group();
        }
        let target = self.targets.get(&view).expect("target allocated above");
        let cv = target.color.create_view(&Default::default());
        let dv = target.depth.create_view(&Default::default());
        let mv = target
            .multisampled
            .as_ref()
            .map(|t| t.create_view(&Default::default()));
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Operandi modeling viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: mv.as_ref().unwrap_or(&cv),
                    depth_slice: None,
                    // Refraction copies the resolved picture before the
                    // see-through surfaces draw.
                    resolve_target: mv.as_ref().filter(|_| refract).map(|_| &cv),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &dv,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(sky) = &sky {
                self.sky.draw(&mut pass, sky);
            }
            match &backdrops {
                // Original-compatible A5 (Perspective): the pictures first,
                // with their depth; the grid only where it is in front of
                // them; then their pixels claimed at the nearest depth, so
                // the meshes drawn next can't cover them.
                Some(backdrops) if backdrops.over_meshes => {
                    self.backdrops.draw_with_depth(&mut pass, backdrops);
                    self.grid.draw_with(&mut pass, &grids, true);
                    self.backdrops.claim(&mut pass, backdrops);
                }
                // Backdrops cover the grid; every mesh covers them (BD-10).
                Some(backdrops) => {
                    self.grid.draw(&mut pass, &grids);
                    self.backdrops.draw(&mut pass, backdrops);
                }
                // No backdrops: the grid draws after the solid meshes,
                // depth-tested, so it shows in front of geometry that lies
                // behind or below its plane.
                None => {}
            }
            let grid_after_solids = backdrops.is_none();
            pass.set_pipeline(&self.triangles);
            tb.draw(&mut pass, 0..triangles.len() as u32);
            if grid_after_solids && world.is_none() {
                self.grid.draw_with(&mut pass, &grids, true);
            }
            if let Some(world) = world {
                let looks = &self.looks[&view];
                pass.set_bind_group(0, &self.cameras[&view].1, &[]);
                pass.set_bind_group(2, &self.scene_group, &[]);
                let fill_pipeline = self
                    .materials
                    .pipeline()
                    .map_or(&self.world_fill, |set| &set.fill);
                pass.set_pipeline(fill_pipeline);
                let mut runs = Runs::default();
                for (id, _, _, _, group, _) in &world.items {
                    let b = &self.resident[id];
                    let Some(k) = shown(*group) else { continue };
                    if b.fill_count > 0 && world.looks[k].fills() && !world.looks[k].see_through() {
                        runs.push(
                            &self.fill_arena,
                            &b.fill,
                            0..b.fill_count,
                            Some(&looks[k].1),
                            self.fill_placement(b),
                        );
                    }
                }
                runs.draw(&mut pass, 1, 3);
                // Subdivision patches (subd_gpu.rs). Provisional: only in
                // solid groups (no see-through override or shadows yet).
                pass.set_pipeline(self.patch_pipelines().fill());
                pass.set_bind_group(2, &self.patch_scene_group, &[]);
                for (id, _, _, _, group, _) in &world.items {
                    let b = &self.resident[id];
                    let Some(k) = shown(*group) else { continue };
                    if let Some(p) = &b.patches
                        && world.looks[k].fills()
                        && !world.looks[k].see_through()
                    {
                        pass.set_bind_group(1, &looks[k].1, &[]);
                        p.draw_fill(&mut pass, if b.whole { 1 } else { 0 });
                    }
                }
                // The grid, now that every solid polygon has its depth:
                // only where the plane is nearer than what is drawn. It
                // writes no depth, so wires and see-through surfaces
                // still draw as before; its bind group 0 replaces the
                // camera's, so that goes back.
                if grid_after_solids {
                    self.grid.draw_with(&mut pass, &grids, true);
                    pass.set_bind_group(0, &self.cameras[&view].1, &[]);
                }
                pass.set_bind_group(2, &self.scene_group, &[]);
                pass.set_pipeline(fill_pipeline);
                // Transparency Overrides (VP-80): see-through groups after
                // every solid polygon, so what lies behind them is drawn.
                for (id, _, _, _, group, _) in &world.items {
                    let b = &self.resident[id];
                    let Some(k) = shown(*group) else { continue };
                    let look = &world.looks[k];
                    if b.fill_count > 0 && look.see_through() {
                        pass.set_pipeline(if look.transparency.overlay {
                            &self.world_clear_over
                        } else {
                            &self.world_clear
                        });
                        let mut runs = Runs::default();
                        runs.push(
                            &self.fill_arena,
                            &b.fill,
                            0..b.fill_count,
                            Some(&looks[k].1),
                            self.fill_placement(b),
                        );
                        runs.draw(&mut pass, 1, 3);
                    }
                }
                // Viewport transparency (VT-01..05): see-through polygons
                // after every solid one, the farthest item first and each
                // item's far side before its near side; each side dims and
                // tints what is behind it, then adds its own light. No
                // depth is written, so what lies behind keeps its wire and
                // selection (drawn in the next pass).
                if refract {
                    // End the pass (resolving it), keep the picture so far,
                    // and go on drawing over it.
                    drop(pass);
                    encoder.copy_texture_to_texture(
                        target.color.as_image_copy(),
                        self.behind.texture.as_image_copy(),
                        wgpu::Extent3d {
                            width: size[0],
                            height: size[1],
                            depth_or_array_layers: 1,
                        },
                    );
                    pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Operandi see-through surfaces"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: mv.as_ref().unwrap_or(&cv),
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &dv,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_bind_group(0, &self.cameras[&view].1, &[]);
                    pass.set_bind_group(2, &self.scene_group, &[]);
                }
                let glass = self
                    .materials
                    .pipeline()
                    .map_or(&self.world_glass, |set| &set.glass);
                let eye = glam::Vec3::from_slice(&world.camera[0..3]);
                let forward = glam::Vec3::from_slice(&world.camera[12..15]);
                let mut see_through: Vec<(f32, &ResidentBuffers, usize)> = world
                    .items
                    .iter()
                    .filter_map(|(id, _, _, _, group, _)| {
                        let b = &self.resident[id];
                        let k = shown(*group)?;
                        if !(b.clear && b.fill_count > 0 && world.looks[k].glass_on()) {
                            return None;
                        }
                        let centre = b.bounds.map_or(glam::Vec3::ZERO, |(lo, hi)| {
                            (glam::Vec3::from(lo) + glam::Vec3::from(hi)) * 0.5
                        });
                        let depth = (b.matrix.transform_point3(centre) - eye).dot(forward);
                        Some((depth, b, k))
                    })
                    .collect();
                see_through.sort_by(|a, b| b.0.total_cmp(&a.0));
                for (_, b, k) in see_through {
                    for pipeline in glass {
                        pass.set_pipeline(pipeline);
                        let mut runs = Runs::default();
                        runs.push(
                            &self.fill_arena,
                            &b.fill,
                            0..b.fill_count,
                            Some(&looks[k].1),
                            self.fill_placement(b),
                        );
                        runs.draw(&mut pass, 1, 3);
                    }
                }
                if let Some(buffer) = &overlay_fill_buffer
                    && let Some(k) = shown(world.overlay_look)
                    && world.looks[k].fills()
                {
                    pass.set_bind_group(1, &looks[k].1, &[]);
                    pass.set_bind_group(3, &self.identity.group, &[]);
                    pass.set_pipeline(&self.world_overlay);
                    buffer.draw(&mut pass, 0..world.overlay.fill.len() as u32);
                }
            }
        }
        {
            // A separate pass also marks the depth attachment read-only. The
            // surface buffer is the sole visibility oracle for every outline.
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Operandi read-only depth outlines"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: mv.as_ref().unwrap_or(&cv),
                    depth_slice: None,
                    resolve_target: mv.as_ref().map(|_| &cv),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &dv,
                    depth_ops: (!self.read_only_depth).then_some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // Cached wireframes first, so this frame's component highlights
            // and the rollover land on top.
            if let Some(world) = world {
                let looks = &self.looks[&view];
                pass.set_bind_group(0, &self.cameras[&view].1, &[]);
                pass.set_bind_group(2, &self.scene_group, &[]);
                pass.set_pipeline(&self.world_edges);
                // Topology Mode (MT7): the active group's wire draws over
                // everything, after the rest.
                for over in [false, true] {
                    if over {
                        if !world.topology {
                            break;
                        }
                        pass.set_pipeline(&self.topology_edges);
                    }
                    let mut runs = Runs::default();
                    for (id, _, _, _, group, _) in &world.items {
                        let b = &self.resident[id];
                        let Some(k) = shown(*group) else { continue };
                        if (world.topology && k == 0) != over {
                            continue;
                        }
                        if b.edge_count > 0 && world.looks[k].wire() {
                            runs.push(
                                &self.edge_arena,
                                &b.edges,
                                0..b.edge_count,
                                Some(&looks[k].1),
                                self.wire_placement(b),
                            );
                        }
                    }
                    runs.draw(&mut pass, 1, 3);
                }
                pass.set_pipeline(&self.world_edges);
                pass.set_pipeline(&self.patch_pipelines().edges);
                pass.set_bind_group(2, &self.patch_scene_group, &[]);
                for (id, _, _, _, group, _) in &world.items {
                    let b = &self.resident[id];
                    let Some(k) = shown(*group) else { continue };
                    if let Some(p) = &b.patches
                        && world.looks[k].wire()
                    {
                        pass.set_bind_group(1, &looks[k].1, &[]);
                        p.draw_wire(&mut pass, 0);
                    }
                }
                pass.set_bind_group(2, &self.scene_group, &[]);
                pass.set_pipeline(&self.world_edges);
                // Selected polygons' wire on top of the plain wire; a whole
                // selected item's is its wire through the highlight placement.
                pass.set_bind_group(1, &looks[0].1, &[]);
                let mut runs = Runs::default();
                for (id, _, _, _, group, _) in &world.items {
                    let b = &self.resident[id];
                    if !world.outline || shown(*group).is_none() {
                        continue;
                    }
                    if let Some(buffer) = &b.highlight {
                        // In order: the outlines gathered so far first.
                        std::mem::take(&mut runs).draw(&mut pass, 1, 3);
                        pass.set_bind_group(3, self.wire_placement(b), &[]);
                        buffer.draw(&mut pass, 0..b.highlight_count);
                    }
                    if b.whole {
                        runs.push(
                            &self.edge_arena,
                            &b.edges,
                            0..b.edge_count,
                            None,
                            self.shared_placement(b, b.outline_kind()),
                        );
                    }
                }
                runs.draw(&mut pass, 1, 3);
                // A whole selected patch surface's wire as its outline.
                pass.set_bind_group(1, &looks[0].1, &[]);
                for (id, _, _, _, group, _) in &world.items {
                    let b = &self.resident[id];
                    if world.outline
                        && b.whole
                        && shown(*group).is_some()
                        && let Some(p) = &b.patches
                    {
                        pass.set_pipeline(&self.patch_pipelines().edges);
                        pass.set_bind_group(2, &self.patch_scene_group, &[]);
                        p.draw_wire(&mut pass, 2);
                        pass.set_bind_group(2, &self.scene_group, &[]);
                        pass.set_pipeline(&self.world_edges);
                    }
                }
                for (id, ..) in &world.seams {
                    if let Some(b) = self.seam_buffers.get(id)
                        && b.count > 0
                    {
                        pass.set_bind_group(3, &b.placement.group, &[]);
                        b.buffer.draw(&mut pass, 0..b.count);
                    }
                }
            }
            pass.set_pipeline(&self.outlines);
            eb.draw(&mut pass, 0..edges.len() as u32);
            if let (Some(world), Some(buffer)) = (world, &overlay_edge_buffer) {
                pass.set_bind_group(0, &self.cameras[&view].1, &[]);
                pass.set_bind_group(1, &self.looks[&view][0].1, &[]);
                pass.set_bind_group(2, &self.scene_group, &[]);
                pass.set_bind_group(3, &self.identity.group, &[]);
                pass.set_pipeline(&self.world_edges);
                buffer.draw(&mut pass, 0..world.overlay.edges.len() as u32);
            }
            for (id, ..) in world.map_or(&[][..], |w| &w.dots[..]) {
                let Some(b) = self.dot_buffers.get(id).filter(|b| b.count > 0) else {
                    continue;
                };
                pass.set_bind_group(0, &self.cameras[&view].1, &[]);
                pass.set_bind_group(1, &self.looks[&view][0].1, &[]);
                pass.set_bind_group(2, &self.scene_group, &[]);
                pass.set_bind_group(3, &b.placement.group, &[]);
                // Topology Mode (MT7): the points over everything.
                pass.set_pipeline(if world.is_some_and(|w| w.topology) {
                    &self.topology_dots
                } else {
                    &self.world_dots
                });
                b.buffer.draw(&mut pass, 0..b.count);
            }
            // Selection x-ray: where a surface hides the selection, it
            // shows through at the preference's opacity.
            if let Some(world) = world
                && world.camera[23] > 0.
            {
                pass.set_bind_group(0, &self.cameras[&view].1, &[]);
                pass.set_bind_group(1, &self.looks[&view][0].1, &[]);
                pass.set_bind_group(2, &self.scene_group, &[]);
                for xray in &world.xray_items {
                    let Some(b) = self.resident.get(xray).filter(|_| world.outline) else {
                        continue;
                    };
                    if let Some(buffer) = &b.highlight {
                        pass.set_bind_group(3, self.wire_placement(b), &[]);
                        pass.set_pipeline(&self.xray_edges);
                        buffer.draw(&mut pass, 0..b.highlight_count);
                    }
                    if b.whole {
                        pass.set_pipeline(&self.xray_edges);
                        let mut runs = Runs::default();
                        runs.push(
                            &self.edge_arena,
                            &b.edges,
                            0..b.edge_count,
                            None,
                            self.shared_placement(b, b.outline_kind()),
                        );
                        runs.draw(&mut pass, 1, 3);
                    }
                }
                // The selected edges' strokes are in world space already,
                // every active mesh's together.
                if let Some(buffer) = &xray_edge_buffer {
                    pass.set_bind_group(3, &self.identity.group, &[]);
                    pass.set_pipeline(&self.xray_edges);
                    buffer.draw(&mut pass, 0..world.xray_edges.len() as u32);
                }
                for (id, _, _, _, from) in &world.dots {
                    if world.xray_items.contains(id)
                        && let Some(d) = self.dot_buffers.get(id)
                        && *from < d.count
                    {
                        pass.set_bind_group(3, &d.placement.group, &[]);
                        pass.set_pipeline(&self.xray_dots);
                        d.buffer.draw(&mut pass, *from..d.count);
                    }
                }
            }
        }
        self.state.queue.submit([encoder.finish()]);
        target.id
    }
}

impl ShadowMap {
    fn new(device: &wgpu::Device, size: u32, pass_layout: &wgpu::BindGroupLayout) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Operandi shadow map"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Operandi shadow light"),
            size: SHADOW_UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pass_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Operandi shadow pass"),
            layout: pass_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Self {
            size,
            texture,
            uniform,
            pass_group,
        }
    }
}

impl ViewportGpu {
    /// Write this view's shadow uniform and, when it draws shadows, render
    /// the shown meshes' depth from the key light into the shadow map.
    fn shadow_pass(&mut self, world: &WorldDraw, encoder: &mut wgpu::CommandEncoder) {
        // Each caster, and whether its see-through polygons cast lighter
        // shadows (viewport transparency on in its look: Advanced, Show
        // Material Transparency on; Jason, 2026-11-14, VT-Q07).
        let casters: Vec<(&ResidentBuffers, bool)> = world
            .items
            .iter()
            .filter_map(|(id, _, _, _, group, _)| {
                let look = &world.looks[shown(*group)?];
                let b = self.resident.get(id)?;
                // VM-Q10: cut-outs (drawn in Advanced only) cast a shadow
                // lighter by the share they cut away.
                let lighter = (b.clear && look.glass_on())
                    || (b.cut && look.style == crate::view_display::Style::Advanced);
                (look.fills() && b.fill_count > 0).then_some((b, lighter))
            })
            .collect();
        let bounds = casters.iter().fold(None, |acc, (b, _)| {
            union(acc, moved_bounds(b.bounds, b.matrix))
        });
        let (lo, hi) = bounds.unwrap_or(([-1.; 3], [1.; 3]));
        let (matrix, toward) = shadow_matrix(world.key_light, lo, hi);
        let on = world.shadows && bounds.is_some();
        let radius = (glam::Vec3::from(hi) - glam::Vec3::from(lo)).length() * 0.5;
        let texel = 1. / SHADOW_SIZE as f32;
        let values: Vec<f32> = matrix
            .to_cols_array()
            .into_iter()
            .chain([toward.x, toward.y, toward.z, if on { 1. } else { 0. }])
            // Push the lookup off the surface by about two shadow texels.
            .chain([texel, radius * 2. * texel * 2., 0.0015, 0.])
            .collect();
        let bytes: Vec<u8> = values.iter().flat_map(|f| f.to_le_bytes()).collect();
        if on && self.shadow.size != SHADOW_SIZE {
            self.shadow = ShadowMap::new(&self.state.device, SHADOW_SIZE, &self.shadow_pass_layout);
            // Field by field: `casters` still borrows the resident buffers.
            self.scene_group = scene_group(
                &self.state.device,
                &self.shadow_sample_layout,
                &self.shadow,
                &self.shadow_sampler,
                &self.materials,
                &self.matcaps,
                (&self.env_light, &self.behind),
            );
            self.patch_scene_group = scene_group(
                &self.state.device,
                &self.patch_scene_layout,
                &self.shadow,
                &self.shadow_sampler,
                &self.materials,
                &self.matcaps,
                (&self.env_light, &self.behind),
            );
        }
        self.state
            .queue
            .write_buffer(&self.shadow.uniform, 0, &bytes);
        if !on {
            return;
        }
        let view = self.shadow.texture.create_view(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Operandi shadow map"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.shadow_pass);
        pass.set_bind_group(0, &self.shadow.pass_group, &[]);
        let mut runs = Runs::default();
        for (b, _) in casters.iter().filter(|(_, clear)| !clear) {
            runs.push(
                &self.fill_arena,
                &b.fill,
                0..b.fill_count,
                None,
                self.wire_placement(b),
            );
        }
        runs.draw(&mut pass, 2, 1);
        // Items with see-through polygons: a dithered depth pass that
        // leaves out a share of texels equal to how much shows through, so
        // the lookup's 3×3 filter sees a lighter shadow. Scenes without
        // them never get here.
        if casters.iter().any(|(_, clear)| *clear) {
            let table = self
                .state
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Operandi see-through shadows"),
                    layout: &self.shadow_clear_layout,
                    entries: &[self.materials.shadow_entry(0)],
                });
            pass.set_pipeline(&self.shadow_clear_pass);
            pass.set_bind_group(2, &table, &[]);
            let mut runs = Runs::default();
            for (b, _) in casters.iter().filter(|(_, clear)| *clear) {
                runs.push(
                    &self.fill_arena,
                    &b.fill,
                    0..b.fill_count,
                    None,
                    self.wire_placement(b),
                );
            }
            runs.draw(&mut pass, 3, 1);
        }
    }
}

/// The shadow map's depth-only pipeline and the two bind group layouts:
/// the fill shader's (uniform, map, compare sampler) and the pass's
/// (uniform only).
fn shadow_pipeline(
    device: &wgpu::Device,
    placement_layout: &wgpu::BindGroupLayout,
) -> (
    wgpu::BindGroupLayout,
    wgpu::BindGroupLayout,
    wgpu::BindGroupLayout,
    wgpu::RenderPipeline,
) {
    let uniform_entry = wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let mut scene_entries = vec![
        // The fill shaders read the shadow uniform only per pixel. Keeping
        // it out of the vertex stage leaves room under Metal's 16 vertex
        // stage buffers for the see-through table below, which the patch
        // pipelines' vertex stage (12 buffers of its own) also counts.
        wgpu::BindGroupLayoutEntry {
            visibility: wgpu::ShaderStages::FRAGMENT,
            ..uniform_entry
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
            count: None,
        },
    ];
    // Image layers' resources share the group (wgpu's default limit is
    // four bind groups), and so do the Reflection matcaps.
    scene_entries.extend(crate::material_gpu::layout_entries(true));
    scene_entries.push(wgpu::BindGroupLayoutEntry {
        binding: crate::matcap::BINDING,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2Array,
            multisampled: false,
        },
        count: None,
    });
    // Viewport transparency's table: the vertex stage drops polygons from
    // the pass that doesn't draw them.
    scene_entries.push(wgpu::BindGroupLayoutEntry {
        binding: CLEAR_BINDING,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    });
    scene_entries.push(wgpu::BindGroupLayoutEntry {
        binding: ENV_BINDING,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    });
    scene_entries.push(wgpu::BindGroupLayoutEntry {
        binding: BEHIND_BINDING,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    });
    let sample_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Operandi scene (shadow map, image layers)"),
        entries: &scene_entries,
    });
    let pass_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Operandi shadow pass"),
        entries: &[uniform_entry],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Operandi shadow pass"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_PASS_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Operandi shadow pass"),
        bind_group_layouts: &[&pass_layout, placement_layout],
        push_constant_ranges: &[],
    });
    let attributes = FILL_ATTRIBUTES;
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Operandi shadow pass"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_shadow"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: FILL_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            }],
        },
        fragment: None,
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            // Open meshes (a ground plane) cast from either side.
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.,
                clamp: 0.,
            },
        }),
        multisample: Default::default(),
        multiview: None,
        cache: None,
    });
    // The patch pipelines' copy: the material tables for the fragment
    // stage only (VM-Q11; Metal's vertex stage buffer limit).
    let fragment_only = crate::material_gpu::layout_entries(false);
    let patch_entries: Vec<wgpu::BindGroupLayoutEntry> = scene_entries
        .iter()
        .map(|e| {
            fragment_only
                .iter()
                .find(|f| f.binding == e.binding)
                .copied()
                .unwrap_or(*e)
        })
        .collect();
    let patch_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Operandi scene for patches"),
        entries: &patch_entries,
    });
    (sample_layout, patch_layout, pass_layout, pipeline)
}
const SHADOW_PASS_SHADER: &str = r#"
struct Shadow { light: mat4x4<f32>, key: vec4<f32>, params: vec4<f32> };
@group(0) @binding(0) var<uniform> sh: Shadow;
struct Placement { m: mat4x4<f32>, n: mat4x4<f32> };
@group(1) @binding(0) var<uniform> place: Placement;
@vertex fn vs_shadow(@location(0) p: vec3<f32>) -> @builtin(position) vec4<f32> {
  return sh.light * place.m * vec4<f32>(p, 1.0);
}
// See-through casters (viewport transparency, VT-Q07): per material
// instance, (tint, share that shows through), as the fill shader's op_clear.
@group(2) @binding(0) var<storage, read> op_clear: array<vec4<f32>>;
struct ClearOut { @builtin(position) position: vec4<f32>, @location(0) @interpolate(flat) program: u32 };
@vertex fn vs_shadow_clear(@location(0) p: vec3<f32>, @location(5) program: u32) -> ClearOut {
  var out: ClearOut;
  out.position = sh.light * place.m * vec4<f32>(p, 1.0);
  out.program = program;
  return out;
}
// Leave out the share of texels that shows through, spread evenly
// (interleaved gradient noise on the shadow map's pixels), so the
// lookup's 3×3 filter averages to a shadow that light by how clear the
// surface is. Opaque polygons (share 0) keep every texel.
@fragment fn fs_shadow_clear(in: ClearOut) {
  let through = op_clear[in.program].w;
  let noise = fract(52.9829189 * fract(dot(floor(in.position.xy), vec2<f32>(0.06711056, 0.00583715))));
  if (noise < through) { discard; }
}
"#;

/// The see-through casters' shadow pipeline (`fs_shadow_clear`) and its
/// table's bind group layout (group 2: the material see-through table).
fn shadow_clear_pipeline(
    device: &wgpu::Device,
    pass_layout: &wgpu::BindGroupLayout,
    placement_layout: &wgpu::BindGroupLayout,
) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
    let table_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Operandi see-through shadows"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Operandi see-through shadows"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_PASS_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Operandi see-through shadows"),
        bind_group_layouts: &[pass_layout, placement_layout, &table_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Operandi see-through shadows"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_shadow_clear"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: FILL_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &FILL_ATTRIBUTES,
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_shadow_clear"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        // As the opaque shadow pass.
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.,
                clamp: 0.,
            },
        }),
        multisample: Default::default(),
        multiview: None,
        cache: None,
    });
    (table_layout, pipeline)
}

/// `ranges` with touching or overlapping ones joined, in order.
fn merged(ranges: &[std::ops::Range<usize>]) -> Vec<std::ops::Range<usize>> {
    let mut sorted: Vec<_> = ranges.iter().filter(|r| !r.is_empty()).cloned().collect();
    sorted.sort_unstable_by_key(|r| r.start);
    let mut out: Vec<std::ops::Range<usize>> = Vec::with_capacity(sorted.len());
    for r in sorted {
        match out.last_mut() {
            Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
            _ => out.push(r),
        }
    }
    out
}

/// The look index of a shown group.
impl WorldDraw {
    /// The transparency override and the shadow map both read stored fill.
    pub fn needs_stored_fill(&self, group: LookGroup) -> bool {
        shown(group).is_some_and(|index| {
            let look = &self.looks[index];
            look.see_through() || (self.shadows && look.fills())
        })
    }
}

fn shown(group: LookGroup) -> Option<usize> {
    match group {
        LookGroup::Active => Some(0),
        LookGroup::Inactive => Some(1),
        LookGroup::Hidden => None,
    }
}

/// Bytes per fill corner (`fill_bytes`).
const FILL_STRIDE: u64 = 52;
/// Bytes per stroke corner (`edge_bytes`) and per dot corner (`dot_bytes`).
const EDGE_STRIDE: u64 = 60;
const DOT_STRIDE: u64 = 28;
/// A fill corner's attributes: position, normal, colour, vertex-map colour,
/// the UVs a material program reads, its instance, then the two tints.
const FILL_ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
    0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Unorm8x4, 4 => Float32x2, 5 => Uint32,
    6 => Unorm8x4, 7 => Unorm8x4
];
/// Vertices packed `stride` bytes each by `write` (`pack_into`).
fn pack<T: Sync>(vertices: &[T], stride: u64, write: fn(&T, &mut Bytes)) -> Vec<u8> {
    let mut bytes = vec![0u8; vertices.len() * stride as usize];
    pack_into(&mut bytes, vertices, stride, write);
    bytes
}

/// Fewer vertices than this are packed on one core: spreading them costs
/// more than it saves.
const PARALLEL_PACK: usize = 1 << 14;

/// Packs `vertices` into `out`, `stride` bytes each, with `write` filling
/// one vertex's bytes in order (`Bytes`). Big lists (a whole
/// million-polygon mesh, or a drag that moves most of it) are packed on
/// every core: each vertex's bytes depend only on that vertex, so the
/// result is the same.
fn pack_into<T: Sync>(out: &mut [u8], vertices: &[T], stride: u64, write: fn(&T, &mut Bytes)) {
    use rayon::prelude::*;
    const RUN: usize = 4096;
    let stride = stride as usize;
    debug_assert_eq!(out.len(), vertices.len() * stride);
    let run = |out: &mut [u8], vertices: &[T]| {
        for (out, v) in out.chunks_exact_mut(stride).zip(vertices) {
            let mut b = Bytes { out, at: 0 };
            write(v, &mut b);
            debug_assert_eq!(b.at, stride, "a vertex filled its stride");
        }
    };
    if vertices.len() < PARALLEL_PACK {
        run(out, vertices);
    } else {
        out.par_chunks_mut(RUN * stride)
            .zip(vertices.par_chunks(RUN))
            .for_each(|(out, vertices)| run(out, vertices));
    }
}

/// Uploads the vertices in each of `ranges` (indices into `vertices` and
/// into `store`'s list alike), packing them straight into the queue's
/// staging memory on every core: a drag that moves most of a big mesh
/// sends hundreds of megabytes a frame, and building them in a list of
/// our own first cost a fill and a copy of all of it. Returns the bytes
/// sent.
fn upload_ranges<T: Sync>(
    store: &crate::gpu_arena::Store,
    arena: &crate::gpu_arena::Arena,
    queue: &wgpu::Queue,
    vertices: &[T],
    ranges: &[std::ops::Range<usize>],
    stride: u64,
    write: fn(&T, &mut Bytes),
) -> u64 {
    let mut views = Vec::new();
    let mut sent = 0;
    for r in ranges {
        let before = views.len();
        if store
            .views(arena, queue, r.start as u32, r.len() as u32, &mut views)
            .is_none()
        {
            // No staging memory: the old way for this range.
            views.truncate(before);
            store.write(
                arena,
                queue,
                r.start as u32,
                &pack(&vertices[r.clone()], stride, write),
            );
        } else {
            for (_, v) in &mut views[before..] {
                *v = (r.start as u32 + v.start)..(r.start as u32 + v.end);
            }
        }
        sent += r.len() as u64 * stride;
    }
    pack_views(&mut views, vertices, stride, write);
    sent
}

/// Packs each staging view's vertices (`.1`, indices into `vertices`) into
/// it, the views shared out over every core when there are many vertices
/// in all (each view is small: a mesh row's worth of moved polygons).
fn pack_views<T: Sync>(
    views: &mut [crate::gpu_arena::View],
    vertices: &[T],
    stride: u64,
    write: fn(&T, &mut Bytes),
) {
    use rayon::prelude::*;
    let total: usize = views.iter().map(|(_, r)| r.len()).sum();
    let mut jobs: Vec<(&mut [u8], &[T])> = views
        .iter_mut()
        .map(|(view, r)| (view.as_mut(), &vertices[r.start as usize..r.end as usize]))
        .collect();
    if total < PARALLEL_PACK {
        for (out, vs) in jobs {
            pack_into(out, vs, stride, write);
        }
    } else {
        jobs.par_iter_mut()
            .for_each(|(out, vs)| pack_into(out, vs, stride, write));
    }
}

/// One vertex's bytes, written front to back.
struct Bytes<'a> {
    out: &'a mut [u8],
    at: usize,
}
impl Bytes<'_> {
    fn put(&mut self, bytes: &[u8]) {
        self.out[self.at..self.at + bytes.len()].copy_from_slice(bytes);
        self.at += bytes.len();
    }
    fn f32s(&mut self, values: &[f32]) {
        for f in values {
            self.put(&f.to_le_bytes());
        }
    }
}

/// 52 bytes per fill corner: position, normal, then the color and the
/// vertex-map color as RGBA8 (gamma-encoded, as the projected path's),
/// then the UVs, the material program instance and the HT-5/HT-6 tints
/// (gpu_scene.rs `Tint`).
fn fill_bytes(vertices: &[crate::gpu_scene::WorldVertex]) -> Vec<u8> {
    pack(vertices, FILL_STRIDE, fill_vertex)
}
fn fill_vertex(v: &crate::gpu_scene::WorldVertex, b: &mut Bytes) {
    b.f32s(&v.position);
    b.f32s(&v.normal);
    b.put(&v.color.to_array());
    b.put(&v.map.to_array());
    b.f32s(&v.uv);
    b.put(&v.program.to_le_bytes());
    b.put(&v.tint.weight);
    b.put(&v.tint.distortion);
}

/// 60 bytes per stroke corner: both ends, (end, side, width), RGBA8, uses,
/// then the two sides' normals as 16-bit signed normalized numbers (the
/// first with the never-fade flag as its fourth).
fn edge_bytes(vertices: &[crate::gpu_scene::EdgeVertex]) -> Vec<u8> {
    pack(vertices, EDGE_STRIDE, edge_vertex)
}
fn edge_vertex(v: &crate::gpu_scene::EdgeVertex, b: &mut Bytes) {
    let snorm = |x: f32| ((x.clamp(-1., 1.) * 32767.).round() as i16).to_le_bytes();
    b.f32s(&v.a);
    b.f32s(&v.b);
    b.f32s(&[v.end, v.side, v.width]);
    b.put(&v.color.to_array());
    b.put(&v.uses.to_le_bytes());
    let [n1, n2] = v.sides;
    for x in n1
        .into_iter()
        .chain([if v.always { 1. } else { 0. }])
        .chain(n2)
        .chain([0.])
    {
        b.put(&snorm(x));
    }
}

/// 28 bytes per dot corner: center, corner, radius, RGBA8.
fn dot_bytes(vertices: &[crate::gpu_scene::DotVertex]) -> Vec<u8> {
    pack(vertices, DOT_STRIDE, |v, b| {
        b.f32s(&v.center);
        b.f32s(&v.corner);
        b.f32s(&[v.radius]);
        b.put(&v.color.to_array());
    })
}

/// The resident-mesh shader, with the display styles' shading spliced in
/// and the material programs of `materials` (`shader_wgsl::module`; no
/// programs for the plain pipelines).
fn world_shader(materials: &str) -> String {
    use operandi_core::blender_principled::{SURFACE_ROWS, list};
    let fill = [
        ("{list_len}", operandi_core::shader_wgsl::LIST_LEN),
        ("{surface_rows}", SURFACE_ROWS),
        ("{flags}", list::FLAGS),
        ("{cut}", list::CUT),
        ("{h}", list::HEIGHT),
        ("{hx}", list::HEIGHT_RIGHT),
        ("{hy}", list::HEIGHT_DOWN),
        ("{normal}", list::NORMAL),
        ("{displacement}", list::DISPLACEMENT),
        ("{displaced}", list::DISPLACED),
        ("{clear_binding}", CLEAR_BINDING as usize),
        ("{behind_binding}", BEHIND_BINDING as usize),
    ]
    .iter()
    .fold(WORLD_SHADER.to_string(), |s, (k, v)| {
        s.replace(k, &v.to_string())
    });
    let fill = [
        ("{solid_glass}", GLASS_SOLID),
        ("{glass_face}", GLASS_FACE),
        ("{glass_rim}", GLASS_RIM),
        ("{glass_rim_grey}", GLASS_RIM_GREY),
        ("{glass_shine}", GLASS_SHINE),
        ("{glass_spec}", GLASS_SPEC),
        ("{refraction}", REFRACTION),
    ]
    .iter()
    .fold(fill, |s, (k, v)| s.replace(k, &format!("{v:?}")));
    fill.replace(
        "//{display_shading}",
        &(crate::display_shading::wgsl()
            + &crate::uv_checker::wgsl()
            + &crate::gpu_scene::tint_wgsl()),
    )
    .replace(
        "//{materials}",
        &(crate::material_gpu::bindings() + materials),
    )
}
#[cfg(test)]
pub fn world_shader_for_tests(materials: &str) -> String {
    world_shader(materials)
}
fn plain_world_shader() -> String {
    world_shader(&operandi_core::shader_wgsl::module(&[]))
}
/// Passes over each see-through item: far side tint and light, near side
/// tint and light, then refraction's add and take-away halves.
pub const GLASS_PASSES: usize = 6;
/// Refraction's shift of what shows through glass, as a share of the
/// view's height per unit of (IOR − 1) (VM-Q09, provisional, by eye).
pub const REFRACTION: f32 = 0.06;
/// Viewport transparency's look, by eye (provisional, VT-Q02): a
/// see-through surface reflects `GLASS_FACE` of the light facing the eye,
/// rising to `GLASS_RIM` at its silhouette, in grey `GLASS_RIM_GREY`, with a
/// key light highlight (`GLASS_SHINE` sharpness, `GLASS_SPEC` strength).
/// Shown solid, it leans toward its Transparent Color times `GLASS_SOLID`.
const GLASS_FACE: f32 = 0.06;
const GLASS_RIM: f32 = 0.6;
const GLASS_RIM_GREY: f32 = 0.85;
const GLASS_SHINE: f32 = 60.;
const GLASS_SPEC: f32 = 0.5;
const GLASS_SOLID: f32 = 0.75;
const WORLD_SHADER: &str = r#"
struct Camera { eye: vec4<f32>, right: vec4<f32>, up: vec4<f32>, forward: vec4<f32>, params: vec4<f32>, view: vec4<f32> };
@group(0) @binding(0) var<uniform> cam: Camera;
//{display_shading}
// Advanced's shadow map (gpu.rs ShadowMap): key = toward the light, w = on;
// params = (texel, normal offset, depth bias, -).
struct Shadow { light: mat4x4<f32>, key: vec4<f32>, params: vec4<f32> };
@group(2) @binding(0) var<uniform> sh: Shadow;
@group(2) @binding(1) var shadow_map: texture_depth_2d;
@group(2) @binding(2) var shadow_cmp: sampler_comparison;
// Where the item's cached geometry is drawn (gpu.rs Placement): identity
// unless a drag moved the whole item since it was built.
// sel (gpu.rs Selected): x = fill as a whole selected item, y = strokes as
// its selection highlight, z = mesh wire (the wire fade applies); sel_fill
// and sel_wire are those colours.
struct Placement { m: mat4x4<f32>, n: mat4x4<f32>, local: mat4x4<f32>, ln: mat4x4<f32>, sel: vec4<f32>, sel_fill: vec4<f32>, sel_wire: vec4<f32> };
@group(3) @binding(0) var<uniform> place: Placement;
//{materials}
// Viewport transparency (gpu.rs CLEAR_BINDING): per material instance,
// (Transparent Color as a tint, how much shows through); row 0 opaque.
@group(2) @binding({clear_binding}) var<storage, read> op_clear: array<vec4<f32>>;
// The view as it was before see-through surfaces drew (BehindGpu).
@group(2) @binding({behind_binding}) var op_behind: texture_2d<f32>;
fn placed(p: vec3<f32>) -> vec3<f32> { return (place.m * vec4<f32>(p, 1.0)).xyz; }
// 1 lit, 0 in shadow: a 3×3 filtered lookup.
fn visibility(p: vec3<f32>, n: vec3<f32>) -> f32 {
  if (sh.key.w < 0.5) { return 1.0; }
  let q = sh.light * vec4<f32>(p + n * sh.params.y, 1.0);
  let uv = vec2<f32>(q.x * 0.5 + 0.5, 0.5 - q.y * 0.5);
  if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || q.z > 1.0) { return 1.0; }
  var sum = 0.0;
  for (var i = -1; i <= 1; i++) {
    for (var j = -1; j <= 1; j++) {
      let o = vec2<f32>(f32(i), f32(j)) * sh.params.x;
      sum += textureSampleCompareLevel(shadow_map, shadow_cmp, uv + o, q.z - sh.params.z);
    }
  }
  return sum / 9.0;
}
struct Out { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32> };
struct FillOut {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) world: vec3<f32>,
  @location(2) normal: vec3<f32>,
  @location(3) map: vec4<f32>,
  // Material programs (material_gpu.rs): the item's own point, the UVs
  // and the instance (0: the vertex colour).
  @location(4) local: vec3<f32>,
  @location(5) uv: vec2<f32>,
  @location(6) @interpolate(flat) program: u32,
  // HT-5/HT-6 tints (gpu_scene.rs `Tint`): weight, UV distortion.
  @location(7) tint_w: vec4<f32>,
  @location(8) tint_d: vec4<f32>,
};
// Where an edge running behind the near clip is cut, as a multiple of the
// near distance (camera.rs NEAR_KEEP).
const NEAR_KEEP: f32 = 1.01;
// GitHub #30: how far edges and points are pulled toward the eye so they
// win over their own polygon. The pull was a share of the view depth
// (0.2% for edges), which on thin walls seen from a few metres let edges
// behind a front face show past its rim. Now it is at most `max_pull` in
// depth-buffer units, a few dozen float steps: enough to clear the
// polygon's own depth, far less than a wall's thickness up close. Far out
// (where the buffer is coarser) the old share is the smaller and stays.
fn pulled(clip: vec4<f32>, share: f32, max_pull: f32) -> vec4<f32> {
  var c = clip;
  if (cam.params.z > 0.5) {
    c.z = c.z - min(cam.params.w * share, max_pull * c.w);
  } else {
    c.z = c.z - min(cam.forward.w * share, max_pull);
  }
  return c;
}
const EDGE_PULL: f32 = 1e-6;
const DOT_PULL: f32 = 2e-6;
fn project(p: vec3<f32>) -> vec4<f32> {
  let d = p - cam.eye.xyz;
  let z = dot(d, cam.forward.xyz);
  let x = dot(d, cam.right.xyz) * cam.params.x;
  let y = dot(d, cam.up.xyz) * cam.params.y;
  if (cam.params.z > 0.5) { return vec4<f32>(x, y, z - cam.params.w, z); }
  return vec4<f32>(x, y, (z + cam.eye.w) / cam.params.w, 1.0);
}
// Viewport transparency: whether a corner's polygon draws see-through in
// this look (Show Material Transparency on, a style that shows materials).
fn glassy(program: u32) -> bool {
  return look.glass.x > 0.5 && op_clear[program].w > 0.0;
}
// The solid pass: see-through polygons are left to the glass pass.
@vertex fn vs_fill(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) c: vec4<f32>, @location(3) m: vec4<f32>, @location(4) uv: vec2<f32>, @location(5) program: u32, @location(6) tw: vec4<f32>, @location(7) td: vec4<f32>) -> FillOut {
  var out = fill_vertex(p, n, c, m, uv, program);
  out.tint_w = tw; out.tint_d = td;
  if (glassy(out.program)) { out.position = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
  return out;
}
// The glass pass: only the see-through polygons.
@vertex fn vs_glass(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) c: vec4<f32>, @location(3) m: vec4<f32>, @location(4) uv: vec2<f32>, @location(5) program: u32, @location(6) tw: vec4<f32>, @location(7) td: vec4<f32>) -> FillOut {
  var out = fill_vertex(p, n, c, m, uv, program);
  out.tint_w = tw; out.tint_d = td;
  if (!glassy(out.program)) { out.position = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
  return out;
}
// VM-Q11: the Advanced style moves a corner along its normal by its
// material's displacement (scene units, the item's own space), evaluated
// here from the per-pixel list, so it follows every drag. Corners without
// a smooth normal (faceted polygons) stay put.
fn displaced(p0: vec3<f32>, n: vec3<f32>, c: vec4<f32>, uv: vec2<f32>, program: u32) -> vec3<f32> {
  if (i32(look.mode.x + 0.5) != 9 || program == 0u || place.sel.x > 0.5 || length(n) <= 0.0) { return p0; }
  let ins = op_instances[program];
  let code = ins.w >> 1u;
  if (code == 0u) { return p0; }
  let local = (place.local * vec4<f32>(p0, 1.0)).xyz;
  let ln = normalize((place.ln * vec4<f32>(normalize(n), 0.0)).xyz);
  let s = OpSp(local, uv, ln, c.rgb, 0.0);
  let l = op_list(code, s, s, s, ins.z + {surface_rows}u);
  if (l[{displaced}].x < 0.5) { return p0; }
  return p0 + normalize(n) * l[{displacement}].x;
}
fn fill_vertex(p0: vec3<f32>, n: vec3<f32>, c: vec4<f32>, m: vec4<f32>, uv: vec2<f32>, program: u32) -> FillOut {
  let p = displaced(p0, n, c, uv, program);
  let w = placed(p);
  var out: FillOut; out.position = project(w); out.color = c; out.world = w;
  out.normal = (place.n * vec4<f32>(n, 0.0)).xyz; out.map = m;
  out.local = (place.local * vec4<f32>(p, 1.0)).xyz; out.uv = uv; out.program = program;
  // A whole selected item: every polygon in the selection fill, marked as
  // a highlight, without its material program (gpu_scene.rs Source::base).
  if (place.sel.x > 0.5) { out.color = place.sel_fill; out.map = vec4<f32>(m.rgb, 0.0); out.program = 0u; }
  return out;
}
// sRGB encoding (egui's), so a program's linear colour lights like the
// gamma-encoded vertex colours.
fn op_encode(c: vec3<f32>) -> vec3<f32> {
  return select(1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055, 12.92 * c, c <= vec3<f32>(0.0031308));
}
// A material program's colour (image layers per pixel), gamma-encoded.
// `flat` is the polygon's eye-facing normal; d* are the inputs' changes
// across one pixel, for the image lookups' mip levels.
fn material_base(in: FillOut, flat: vec3<f32>, dlx: vec3<f32>, dly: vec3<f32>, dux: vec2<f32>, duy: vec2<f32>, dsx: vec3<f32>, dsy: vec3<f32>) -> vec3<f32> {
  let ins = op_instances[in.program];
  if (ins.x == 0u) { return in.color.rgb; }
  let s = op_points(in, flat, dlx, dly, dux, duy, dsx, dsy);
  let lin = op_material(ins.x, s[0], s[1], s[2], ins.y);
  return op_encode(clamp(lin, vec3<f32>(0.0), vec3<f32>(1.0)));
}
// The three shading points a program reads: the pixel's and those one
// pixel right and down. The shading normal is the one the CPU fold uses:
// the corner normals when the material is smooth, else the polygon's; in
// the item's own space.
fn op_points(in: FillOut, flat: vec3<f32>, dlx: vec3<f32>, dly: vec3<f32>, dux: vec2<f32>, duy: vec2<f32>, dsx: vec3<f32>, dsy: vec3<f32>) -> array<OpSp, 3> {
  var sn = flat;
  if (length(in.normal) > 0.0) {
    var s = normalize(in.normal);
    if (dot(s, flat) < 0.0) { s = -s; }
    sn = s;
  }
  let ln = (place.ln * vec4<f32>(sn, 0.0)).xyz;
  // Incidence (degrees): 0 facing the eye, 90 edge-on, as the renderer.
  var to_eye = -cam.forward.xyz;
  if (cam.params.z > 0.5) { to_eye = normalize(cam.eye.xyz - in.world); }
  let inc = degrees(acos(clamp(dot(sn, to_eye), 0.0, 1.0)));
  return array<OpSp, 3>(
    OpSp(in.local, in.uv, ln, in.color.rgb, inc),
    OpSp(in.local + dlx, in.uv + dux, ln, in.color.rgb + dsx, inc),
    OpSp(in.local + dly, in.uv + duy, ln, in.color.rgb + dsy, inc),
  );
}
@fragment fn fs_fill(in: FillOut) -> @location(0) vec4<f32> {
  return vec4<f32>(fill_rgb(in, true), 1.0);
}
// The styled colour of a fill pixel. `solid`: drawn opaque, so a
// see-through surface (shown solid: the setting off, a style that doesn't
// show materials, a subdivision patch) leans toward its Transparent Color
// instead of going black at Diffuse Amount 0 (provisional, VT-Q04).
fn fill_rgb(in: FillOut, solid: bool) -> vec3<f32> {
  // Derivatives first, while control flow is uniform.
  let dwx = dpdx(in.world); let dwy = dpdy(in.world);
  let dlx = dpdx(in.local); let dly = dpdy(in.local);
  let dux = dpdx(in.uv); let duy = dpdy(in.uv);
  let dsx = dpdx(in.color.rgb); let dsy = dpdy(in.color.rgb);
  var to_eye = -cam.forward.xyz;
  if (cam.params.z > 0.5) { to_eye = normalize(cam.eye.xyz - in.world); }
  // The polygon's own normal from the surface's slope, facing the eye
  // (back faces are culled).
  var n = cross(dwx, dwy);
  if (length(n) > 0.0) { n = normalize(n); } else { n = to_eye; }
  if (dot(n, to_eye) < 0.0) { n = -n; }
  var base = in.color.rgb;
  if (in.program != 0u) { base = material_base(in, n, dlx, dly, dux, duy, dsx, dsy); }
  // UVT-114 Checker (approved Operandi extra): the numbered check through
  // the polygon's UVs replaces its colour; polygons without UVs keep it.
  if (look.glass.y > 0.5 && in.uv.x > -1.0e8) { base = op_checker(in.uv); }
  // HT-5/HT-6: the weight map's or UV distortion's tint, lit like the
  // colour it covers (gpu_scene.rs tint_wgsl).
  base = op_tint(base, in.tint_w, in.tint_d);
  let clear = op_clear[in.program];
  if (solid && clear.w > 0.0) { base = mix(base, clear.rgb * {solid_glass}, clear.w); }
  let style = i32(look.mode.x + 0.5);
  // The polygon's principled values for Advanced (blender_principled.rs),
  // else the plain fill's; with a list, the ones that read images per
  // pixel, its bump, normal map and cut-out.
  var surface = op_default_surface();
  var list: array<vec3<f32>, {list_len}>;
  var listed = false;
  if (in.program != 0u) {
    let ins = op_instances[in.program];
    if ((ins.w & 1u) != 0u) { surface = op_surface(ins.z); }
    let code = ins.w >> 1u;
    if (code != 0u && style == 9) {
      let s = op_points(in, n, dlx, dly, dux, duy, dsx, dsy);
      list = op_list(code, s[0], s[1], s[2], ins.z + {surface_rows}u);
      surface = op_listed_surface(surface, list);
      listed = true;
      if (list[{flags}].z > 0.5 && list[{cut}].x > 0.5) { discard; }
    }
  }
  // Smoothing: the corners' averaged normals, on the same side.
  if (look.mode.y > 0.5 && length(in.normal) > 0.0) {
    var s = normalize(in.normal);
    if (dot(s, n) < 0.0) { s = -s; }
    n = s;
  }
  if (listed) {
    let n_geo = n;
    if (list[{flags}].x > 0.5) { n = op_bump(n, dwx, dwy, list[{h}].x, list[{hx}].x, list[{hy}].x); }
    if (list[{flags}].y > 0.5) { n = op_normal_map(n, dwx, dwy, dux, duy, list[{normal}]); }
    // Clearcoat Bump: the coat follows the relief by that share.
    if (surface.coat > 0.0) {
      let cn = mix(n_geo, n, clamp(surface.coat_bump, 0.0, 1.0));
      if (length(cn) > 0.0) { surface.coat_n = normalize(cn); }
    }
  }
  var lit = 1.0;
  if (style == 9) { lit = visibility(in.world, n); }
  // Highlighted polygons (selection, locked, rollover) carry map alpha 0
  // (gpu_scene.rs `marked`): the style first, the highlight on top.
  return shade_marked(style, base, in.map.rgb, n, to_eye, sh.key.xyz, lit, in.map.a < 0.5, surface);
}
// Viewport transparency's edge terms for a see-through pixel: (how much the
// surface reflects at this angle, its key light highlight). Facing the eye
// it reflects a little, toward its silhouette much more, so clear glass
// keeps its shape (VT-01's visible edges) without refraction.
fn glass_terms(in: FillOut) -> vec2<f32> {
  var to_eye = -cam.forward.xyz;
  if (cam.params.z > 0.5) { to_eye = normalize(cam.eye.xyz - in.world); }
  var n = cross(dpdx(in.world), dpdy(in.world));
  if (length(n) > 0.0) { n = normalize(n); } else { n = to_eye; }
  if (dot(n, to_eye) < 0.0) { n = -n; }
  if (look.mode.y > 0.5 && length(in.normal) > 0.0) {
    var s = normalize(in.normal);
    if (dot(s, n) < 0.0) { s = -s; }
    n = s;
  }
  let facing = clamp(dot(n, to_eye), 0.0, 1.0);
  let rim = mix({glass_face}, {glass_rim}, pow(1.0 - facing, 3.0));
  let h = normalize(sh.key.xyz + to_eye);
  let spec = pow(max(dot(n, h), 0.0), {glass_shine}) * {glass_spec};
  return vec2<f32>(rim, spec);
}
// First pass of a see-through side: what is behind keeps the share that
// shows through, tinted, less what the surface reflects (multiplied in).
@fragment fn fs_glass_tint(in: FillOut) -> @location(0) vec4<f32> {
  let c = op_clear[in.program];
  let g = glass_terms(in);
  return vec4<f32>(c.rgb * c.w * (1.0 - g.x), 1.0);
}
// Second pass: the surface's own styled colour for the share that doesn't
// show through, plus its reflection and highlight (added). Alpha counts
// as covered, so the picture composites over an empty background.
@fragment fn fs_glass_light(in: FillOut) -> @location(0) vec4<f32> {
  let c = op_clear[in.program];
  let body = fill_rgb(in, false);
  let g = glass_terms(in);
  let rgb = body * (1.0 - c.w) + vec3<f32>(g.x * {glass_rim_grey} + g.y);
  return vec4<f32>(rgb, 1.0 - c.w * (1.0 - g.x));
}
// Refraction (VM-Q09, provisional): the near side shows what is behind it
// shifted against its normal, by (IOR - 1) × {refraction} of the view's
// height. The tint pass already dimmed the straight-through picture; this
// adds the difference to the shifted one, split into a part to add and a
// part to take away (render targets clamp negative colours).
fn glass_refracted(in: FillOut) -> vec3<f32> {
  let c = op_clear[in.program];
  var to_eye = -cam.forward.xyz;
  if (cam.params.z > 0.5) { to_eye = normalize(cam.eye.xyz - in.world); }
  var n = cross(dpdx(in.world), dpdy(in.world));
  if (length(n) > 0.0) { n = normalize(n); } else { n = to_eye; }
  if (dot(n, to_eye) < 0.0) { n = -n; }
  if (look.mode.y > 0.5 && length(in.normal) > 0.0) {
    var s = normalize(in.normal);
    if (dot(s, n) < 0.0) { s = -s; }
    n = s;
  }
  var ior = 1.5;
  let ins = op_instances[in.program];
  if ((ins.w & 1u) != 0u) { ior = op_surface(ins.z).ior; }
  let size = vec2<f32>(textureDimensions(op_behind));
  // Screen y runs down.
  let shift = vec2<f32>(-dot(n, cam.right.xyz), dot(n, cam.up.xyz)) * (ior - 1.0) * {refraction} * size.y;
  let here = in.position.xy / size;
  let there = clamp((in.position.xy + shift) / size, vec2<f32>(0.0), vec2<f32>(1.0));
  let through = c.rgb * c.w * (1.0 - glass_terms(in).x);
  let a = textureSampleLevel(op_behind, op_smp_l_c, here, 0.0).rgb;
  let b = textureSampleLevel(op_behind, op_smp_l_c, there, 0.0).rgb;
  return (b - a) * through;
}
@fragment fn fs_glass_refract_add(in: FillOut) -> @location(0) vec4<f32> {
  return vec4<f32>(max(glass_refracted(in), vec3<f32>(0.0)), 0.0);
}
@fragment fn fs_glass_refract_sub(in: FillOut) -> @location(0) vec4<f32> {
  return vec4<f32>(max(-glass_refracted(in), vec3<f32>(0.0)), 0.0);
}
// Transparency Overrides (VP-80): Ghost tints the polygons facing the eye
// with the front colour at its opacity and draws those facing away with
// the back colour; X-Ray drops the front ones. Flat Shaded skips lighting.
@fragment fn fs_fill_clear(in: FillOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
  let mode = i32(look.t_mode.x + 0.5);
  if (mode == 2 && front) { discard; }
  var c = look.t_back;
  if (front) { c = look.t_front; }
  if (c.a <= 0.0) { discard; }
  var rgb = c.rgb;
  if (look.t_mode.y < 0.5) {
    var to_eye = -cam.forward.xyz;
    if (cam.params.z > 0.5) { to_eye = normalize(cam.eye.xyz - in.world); }
    var n = cross(dpdx(in.world), dpdy(in.world));
    if (length(n) > 0.0) { n = normalize(n); } else { n = to_eye; }
    if (dot(n, to_eye) < 0.0) { n = -n; }
    rgb = shade(i32(look.mode.x + 0.5), c.rgb, in.map.rgb, n, to_eye, sh.key.xyz, 1.0);
  }
  return vec4<f32>(rgb, c.a);
}
// A whole selected item's wire drawn as its selection highlight (gpu.rs
// Selected): the selection colour at the highlight width.
fn highlight_k(k: vec3<f32>) -> vec3<f32> {
  if (place.sel.y > 0.5) { return vec3<f32>(k.x, k.y, 2.0); }
  return k;
}
fn highlight_c(c: vec4<f32>) -> vec4<f32> {
  if (place.sel.y > 0.5) { return place.sel_wire; }
  return c;
}
@vertex fn vs_edge(@location(0) a: vec3<f32>, @location(1) b: vec3<f32>, @location(2) k_in: vec3<f32>, @location(3) c_in: vec4<f32>, @location(4) uses: f32, @location(5) s1: vec4<f32>, @location(6) s2: vec4<f32>) -> Out {
  let k = highlight_k(k_in); let c = highlight_c(c_in);
  var out = edge_out_fading(a, b, k, c, k_in.z == 0.0 && !kept(a, b, s1, s2));
  out.color.a = twice(out.color.a, uses);
  return out;
}
// Topology Mode (missing-tools-g3 MT7-02): the plain wire in the look's
// Colored overlay colour (Topo Edge), or in the inactive row's colour
// (Topo Back Edge) where every polygon beside it faces away; drawn
// without the depth test, so over the background meshes.
@vertex fn vs_edge_topo(@location(0) a: vec3<f32>, @location(1) b: vec3<f32>, @location(2) k_in: vec3<f32>, @location(3) c_in: vec4<f32>, @location(4) uses: f32, @location(5) s1: vec4<f32>, @location(6) s2: vec4<f32>) -> Out {
  let k = highlight_k(k_in); let c = highlight_c(c_in);
  var out = edge_out_fading(a, b, k, c, false);
  if (k.z == 0.0) {
    var to_eye = -cam.forward.xyz;
    if (cam.params.z > 0.5) { to_eye = cam.eye.xyz - placed((a + b) * 0.5); }
    var front = false;
    var any = false;
    if (dot(s1.xyz, s1.xyz) >= 0.25) {
      any = true;
      front = front || dot((place.n * vec4<f32>(s1.xyz, 0.0)).xyz, to_eye) > 0.0;
    }
    if (dot(s2.xyz, s2.xyz) >= 0.25) {
      any = true;
      front = front || dot((place.n * vec4<f32>(s2.xyz, 0.0)).xyz, to_eye) > 0.0;
    }
    if (any && !front) { out.color = vec4<f32>(look.inactive.rgb, out.color.a); }
  }
  out.color.a = twice(out.color.a, uses);
  return out;
}
// A stroke standing for two polygons' strokes of one edge (gpu_scene.rs
// EdgeVertex::uses) blends once as the two would: 1 − (1 − α)².
fn twice(alpha: f32, uses: f32) -> f32 {
  if (uses > 1.5) { return 1.0 - (1.0 - alpha) * (1.0 - alpha); }
  return alpha;
}
fn edge_out(a: vec3<f32>, b: vec3<f32>, k: vec3<f32>, c: vec4<f32>) -> Out {
  return edge_out_fading(a, b, k, c, false);
}
// Edges Fade Wireframe on Small Polygons keeps however short, so a dense
// mesh still reads as its shape: strokes marked never to fade (s1.w), open
// borders (no polygon across), sharp edges (the two polygons turn more
// than the fade's angle, look.wire_fade.w its cosine) and the outline
// (one polygon faces the eye, the other away).
fn kept(a: vec3<f32>, b: vec3<f32>, s1: vec4<f32>, s2: vec4<f32>) -> bool {
  if (s1.w > 0.5 || dot(s1.xyz, s1.xyz) < 0.25 || dot(s2.xyz, s2.xyz) < 0.25) { return true; }
  let n1 = normalize((place.n * vec4<f32>(s1.xyz, 0.0)).xyz);
  let n2 = normalize((place.n * vec4<f32>(s2.xyz, 0.0)).xyz);
  if (dot(n1, n2) < look.wire_fade.w) { return true; }
  var to_eye = -cam.forward.xyz;
  if (cam.params.z > 0.5) { to_eye = cam.eye.xyz - placed((a + b) * 0.5); }
  return dot(n1, to_eye) * dot(n2, to_eye) <= 0.0;
}
// `fades`: Fade Wireframe on Small Polygons applies (gpu.rs WIRE_FADE_*):
// the plain wire and a whole selected item's outline, never the
// highlights of selected polygons or edges.
fn edge_out_fading(a: vec3<f32>, b: vec3<f32>, k: vec3<f32>, c: vec4<f32>, fades: bool) -> Out {
  var out: Out; out.color = c;
  // The plain wire takes the group's overlay color and opacity; selected,
  // rollover, crease and seam strokes (k.z > 0) keep theirs.
  if (k.z == 0.0) {
    if (look.mode.w > 1.5) { out.color = vec4<f32>(look.wire.rgb, c.a); }
    // Inactive meshes: the plain wire (baked as the Wireframe colour)
    // takes the Inactive Wireframe colour; locked edges keep theirs.
    else if (look.plain.w > 0.5 && all(abs(c.rgb - look.plain.rgb) < vec3<f32>(0.006))) {
      out.color = vec4<f32>(look.inactive.rgb, c.a);
    }
    out.color.a = out.color.a * look.mode.z;
  }
  var ca = project(placed(a)); var cb = project(placed(b));
  let culled = vec4<f32>(0.0, 0.0, 2.0, 1.0);
  // Perspective: an edge with one end behind the near clip is cut where
  // it crosses it (a hair in front, so the pull below stays inside
  // the depth range) rather than dropped, so a long edge running past
  // the eye keeps the part in view as its polygon does (camera.rs
  // `clip_to_near`, the same cut on the CPU). Gone only when both ends
  // are behind.
  if (cam.params.z > 0.5) {
    let keep = cam.params.w * NEAR_KEEP;
    if (ca.w < keep && cb.w < keep) { out.position = culled; return out; }
    // `project` is affine in the point, so clip coordinates interpolate.
    if (ca.w < keep) { ca = mix(ca, cb, (keep - ca.w) / (cb.w - ca.w)); }
    if (cb.w < keep) { cb = mix(cb, ca, (keep - cb.w) / (ca.w - cb.w)); }
  }
  let half = cam.view.xy * 0.5;
  let pa = ca.xy / ca.w * half;
  let pb = cb.xy / cb.w * half;
  let len = length(pb - pa);
  if (len < 1e-5) { out.position = culled; return out; }
  if (fades && place.sel.z > 0.5 && look.wire_fade.x > 0.5) {
    let f = clamp((len - look.wire_fade.y) / (look.wire_fade.z - look.wire_fade.y), 0.0, 1.0);
    // Gone: not drawn at all, which is what makes dense meshes cheap.
    if (f <= 0.0) { out.position = culled; return out; }
    out.color.a = out.color.a * f;
  }
  let dir = (pb - pa) / len;
  let normal = vec2<f32>(-dir.y, dir.x);
  // k.z: 0 for the plain edge width, otherwise the width in points.
  let width = select(cam.view.z, k.z, k.z > 0.0);
  var clip = select(ca, cb, k.x > 0.5);
  let offset = normal * k.y * width * 0.5 / half;
  clip = vec4<f32>(clip.xy + offset * clip.w, clip.z, clip.w);
  // Sit just in front of the edge's own surface (`pulled`).
  out.position = pulled(clip, 0.002, EDGE_PULL); return out;
}
@fragment fn fs(in: Out) -> @location(0) vec4<f32> { return in.color; }
// Selection x-ray: only highlighted strokes (selection, k.z > 0), faded by
// the x-ray opacity in cam.view.w; drawn where the depth test fails.
@vertex fn vs_edge_xray(@location(0) a: vec3<f32>, @location(1) b: vec3<f32>, @location(2) k_in: vec3<f32>, @location(3) c_in: vec4<f32>, @location(4) uses: f32, @location(5) s1: vec4<f32>, @location(6) s2: vec4<f32>) -> Out {
  let k = highlight_k(k_in); let c = highlight_c(c_in);
  var out = edge_out_fading(a, b, k, c, k_in.z == 0.0 && !kept(a, b, s1, s2));
  if (k.z == 0.0) { out.position = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
  out.color.a = twice(out.color.a * cam.view.w, uses);
  return out;
}
struct DotOut { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32>, @location(1) corner: vec2<f32> };
@vertex fn vs_dot(@location(0) p: vec3<f32>, @location(1) corner: vec2<f32>, @location(2) radius: f32, @location(3) c: vec4<f32>) -> DotOut {
  var out: DotOut; out.color = c; out.corner = corner;
  var clip = project(placed(p));
  if (cam.params.z > 0.5 && clip.w < cam.params.w) { out.position = vec4<f32>(0.0, 0.0, 2.0, 1.0); return out; }
  let half = cam.view.xy * 0.5;
  clip = vec4<f32>(clip.xy + corner * radius / half * clip.w, clip.z, clip.w);
  // Points sit on their surface: a little closer than the edges' pull.
  out.position = pulled(clip, 0.004, DOT_PULL); return out;
}
@fragment fn fs_dot(in: DotOut) -> @location(0) vec4<f32> {
  if (dot(in.corner, in.corner) > 1.0) { discard; }
  return in.color;
}
@fragment fn fs_dot_xray(in: DotOut) -> @location(0) vec4<f32> {
  if (dot(in.corner, in.corner) > 1.0) { discard; }
  return vec4<f32>(in.color.rgb, in.color.a * cam.view.w);
}
"#;

/// Pipelines for geometry kept on the graphics card; the shader projects
/// with the view's camera uniform (see gpu_scene.rs for the math).
fn world_pipelines(
    device: &wgpu::Device,
    samples: u32,
    shadow_layout: &wgpu::BindGroupLayout,
    placement_layout: &wgpu::BindGroupLayout,
) -> WorldPipelines {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Operandi resident mesh shader"),
        source: wgpu::ShaderSource::Wgsl(plain_world_shader().into()),
    });
    let uniform_layout = |label| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    };
    let look_layout = uniform_layout("Operandi view look");
    let camera_layout = uniform_layout("Operandi view camera");
    // Fills read Advanced's shadow map (group 2); every pipeline shares
    // the layout so the item placement is group 3 for all of them.
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Operandi resident mesh"),
        bind_group_layouts: &[
            &camera_layout,
            &look_layout,
            shadow_layout,
            placement_layout,
        ],
        push_constant_ranges: &[],
    });
    let fill_attributes = FILL_ATTRIBUTES;
    let edge_attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Unorm8x4, 4 => Float32, 5 => Snorm16x4, 6 => Snorm16x4];
    let dot_attributes =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32, 3 => Unorm8x4];
    let make_with = |entry: &str,
                     stride: u64,
                     attributes: &[wgpu::VertexAttribute],
                     cull: bool,
                     depth_write: bool,
                     xray: bool,
                     always: bool| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Operandi resident mesh"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: stride,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(match entry {
                    "vs_dot" if xray => "fs_dot_xray",
                    "vs_dot" => "fs_dot",
                    "vs_fill" => "fs_fill",
                    _ => "fs",
                }),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    // The overlay's opacity blends the wire over the fill.
                    blend: (entry == "vs_edge" || xray || always)
                        .then_some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                // Shaded view hides polygons facing away, as the projected
                // path's facing test does.
                cull_mode: cull.then_some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: depth_write,
                // X-ray draws only what the surface hides.
                // Topology Mode draws over everything (MT7-01).
                depth_compare: if always {
                    wgpu::CompareFunction::Always
                } else if xray {
                    wgpu::CompareFunction::Greater
                } else {
                    wgpu::CompareFunction::LessEqual
                },
                stencil: Default::default(),
                bias: if depth_write || xray || always {
                    Default::default()
                } else {
                    wgpu::DepthBiasState {
                        constant: -2,
                        slope_scale: -1.,
                        clamp: 0.,
                    }
                },
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        })
    };
    let make = |entry, stride, attributes, cull, depth_write| {
        make_with(entry, stride, attributes, cull, depth_write, false, false)
    };
    let fill = fill_pipeline(device, &layout, samples, &shader);
    let overlay = make("vs_fill", FILL_STRIDE, &fill_attributes, true, false);
    let edges = make("vs_edge", EDGE_STRIDE, &edge_attributes, false, false);
    let dots = make("vs_dot", 28, &dot_attributes, false, false);
    let xray_edges = make_with(
        "vs_edge_xray",
        EDGE_STRIDE,
        &edge_attributes,
        false,
        false,
        true,
        false,
    );
    let xray_dots = make_with("vs_dot", 28, &dot_attributes, false, false, true, false);
    let topology_edges = make_with(
        "vs_edge_topo",
        EDGE_STRIDE,
        &edge_attributes,
        false,
        false,
        false,
        true,
    );
    let topology_dots = make_with("vs_dot", 28, &dot_attributes, false, false, false, true);
    // Transparency Overrides (VP-80): both sides, blended, no depth written
    // so what lies behind still draws; the Overlay variant ignores depth.
    let clear = |over: bool| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Operandi see-through fill"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fill"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: FILL_STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &fill_attributes,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_fill_clear"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: if over {
                    wgpu::CompareFunction::Always
                } else {
                    wgpu::CompareFunction::LessEqual
                },
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        })
    };
    WorldPipelines {
        camera_layout,
        look_layout,
        fill,
        overlay,
        edges,
        dots,
        xray_edges,
        xray_dots,
        topology_edges,
        topology_dots,
        clear: clear(false),
        clear_over: clear(true),
        glass: glass_pipelines(device, &layout, samples, &shader),
        layout,
    }
}

/// Viewport transparency's four passes over an item's see-through polygons
/// (`vs_glass` drops the rest): its far side, then its near side, each
/// first dimming and tinting what is behind (`fs_glass_tint`, multiplied
/// in), then adding its own light (`fs_glass_light`). Depth is tested, not
/// written.
fn glass_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    samples: u32,
    shader: &wgpu::ShaderModule,
) -> [wgpu::RenderPipeline; GLASS_PASSES] {
    let tint = wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::Src,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    };
    let light = wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    };
    let make = |cull: wgpu::Face, entry: &str, blend: wgpu::BlendState| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Operandi see-through material"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_glass"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: FILL_STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &FILL_ATTRIBUTES,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(cull),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        })
    };
    // Refraction's two halves change colour only.
    let keep_alpha = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    };
    let refract_add = wgpu::BlendState {
        color: light.color,
        alpha: keep_alpha,
    };
    let refract_sub = wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::ReverseSubtract,
        },
        alpha: keep_alpha,
    };
    // Culling the near faces draws the far side.
    [
        make(wgpu::Face::Front, "fs_glass_tint", tint),
        make(wgpu::Face::Front, "fs_glass_light", light),
        make(wgpu::Face::Back, "fs_glass_tint", tint),
        make(wgpu::Face::Back, "fs_glass_light", light),
        make(wgpu::Face::Back, "fs_glass_refract_add", refract_add),
        make(wgpu::Face::Back, "fs_glass_refract_sub", refract_sub),
    ]
}

/// The opaque fill pipeline of a resident shader module (the plain one, or
/// one with material programs).
fn fill_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    samples: u32,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Operandi resident fill"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_fill"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: FILL_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &FILL_ATTRIBUTES,
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_fill"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview: None,
        cache: None,
    })
}

/// The Reflection matcaps on the graphics card: one array layer per
/// built-in, then `matcap::MAX_CUSTOM` slots filled when a view first
/// chooses a loaded image (an unreadable one draws the default).
struct MatcapGpu {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    /// Which custom image each slot holds on the card (matcap::slot).
    loaded: [Option<u32>; crate::matcap::MAX_CUSTOM],
}
impl MatcapGpu {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        use crate::matcap::{BUILT_IN, Choice, LAYERS, SIZE};
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Operandi matcaps"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: LAYERS,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Gamma-encoded values sampled as they are: the shading works
            // on gamma-encoded colours (display_shading.rs).
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let out = Self {
            texture,
            view,
            loaded: [None; crate::matcap::MAX_CUSTOM],
        };
        for i in 0..BUILT_IN.len() {
            out.write(queue, i as u32, Choice::BuiltIn(i as u8));
        }
        // Empty custom slots show the default until filled.
        for slot in 0..crate::matcap::MAX_CUSTOM {
            out.write(queue, (BUILT_IN.len() + slot) as u32, Choice::default());
        }
        out
    }
    fn write(&self, queue: &wgpu::Queue, layer: u32, choice: crate::matcap::Choice) {
        let size = crate::matcap::SIZE;
        let image = crate::matcap::image_or_default(choice);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &image.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size * 4),
                rows_per_image: Some(size),
            },
            wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
        );
    }
    /// Upload a loaded image when a view uses it and its slot doesn't
    /// hold it yet (a missing one draws Clay Studio's layer instead).
    fn ensure(&mut self, queue: &wgpu::Queue, choice: crate::matcap::Choice) {
        if let crate::matcap::Choice::Custom(id) = choice
            && !choice.missing()
        {
            let slot = crate::matcap::slot(id);
            if self.loaded[slot] != Some(id) {
                self.loaded[slot] = Some(id);
                self.write(queue, choice.layer(), choice);
            }
        }
    }
}

/// The environment picture's blurred copy for the Advanced style's
/// reflections (HDRI stage 3); a black pixel when there is none.
/// The resolved view picture as it was before the see-through surfaces
/// drew (refraction, VM-Q06). One texture serves every view: each view
/// copies into its top-left corner just before its own glass passes, and
/// grows it when it is too small.
struct BehindGpu {
    size: [u32; 2],
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}
impl BehindGpu {
    fn new(device: &wgpu::Device, size: [u32; 2]) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Operandi picture behind see-through surfaces"),
            size: wgpu::Extent3d {
                width: size[0].max(1),
                height: size[1].max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self {
            size: [size[0].max(1), size[1].max(1)],
            texture,
            view,
        }
    }
}

struct EnvLightGpu {
    /// The picture's key (`SkyPicture::key`), 0 for none.
    key: u64,
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}
impl EnvLightGpu {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let texture =
            crate::environment_gpu::upload(device, queue, "No environment light", 1, 1, &[0; 4]);
        let view = texture.create_view(&Default::default());
        Self {
            key: 0,
            _texture: texture,
            view,
        }
    }
    fn of(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        p: &crate::environment_gpu::SkyPicture,
    ) -> Self {
        use crate::environment_gpu::BLUR_WIDTH;
        let texture = crate::environment_gpu::upload(
            device,
            queue,
            "Environment light",
            BLUR_WIDTH,
            BLUR_WIDTH / 2,
            &p.blurred,
        );
        let view = texture.create_view(&Default::default());
        Self {
            key: p.key,
            _texture: texture,
            view,
        }
    }
}

/// The scene bind group (group 2): the shadow map, the image layers and
/// the matcaps.
fn scene_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shadow: &ShadowMap,
    compare: &wgpu::Sampler,
    materials: &crate::material_gpu::MaterialGpu,
    matcaps: &MatcapGpu,
    // The environment light's picture and the picture behind glass.
    (env, behind): (&EnvLightGpu, &BehindGpu),
) -> wgpu::BindGroup {
    let view = shadow.texture.create_view(&Default::default());
    let mut entries = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: shadow.uniform.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::TextureView(&view),
        },
        wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::Sampler(compare),
        },
    ];
    entries.extend(materials.entries());
    entries.push(wgpu::BindGroupEntry {
        binding: crate::matcap::BINDING,
        resource: wgpu::BindingResource::TextureView(&matcaps.view),
    });
    entries.push(wgpu::BindGroupEntry {
        binding: ENV_BINDING,
        resource: wgpu::BindingResource::TextureView(&env.view),
    });
    entries.push(materials.clear_entry(CLEAR_BINDING));
    entries.push(wgpu::BindGroupEntry {
        binding: BEHIND_BINDING,
        resource: wgpu::BindingResource::TextureView(&behind.view),
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Operandi scene (shadow map, image layers)"),
        layout,
        entries: &entries,
    })
}

impl ViewportGpu {
    fn rebuild_scene_group(&mut self) {
        self.scene_group = scene_group(
            &self.state.device,
            &self.shadow_sample_layout,
            &self.shadow,
            &self.shadow_sampler,
            &self.materials,
            &self.matcaps,
            (&self.env_light, &self.behind),
        );
        self.patch_scene_group = scene_group(
            &self.state.device,
            &self.patch_scene_layout,
            &self.shadow,
            &self.shadow_sampler,
            &self.materials,
            &self.matcaps,
            (&self.env_light, &self.behind),
        );
    }
    /// Bring the image layers' textures, parameters and fill pipeline up to
    /// this frame's materials (nothing to do while they are unchanged).
    fn sync_materials(&mut self, world: &WorldDraw) {
        let device = &self.state.device;
        let layout = &self.world_layout;
        let samples = self.samples;
        let compile = |module: &str| {
            let source = world_shader(module);
            if let Err(e) = crate::material_gpu::valid(&source) {
                crate::log_ring::log_line!(
                    "Image layers can't be drawn in the viewport (shader error); showing average colours. {e}"
                );
                return None;
            }
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Operandi resident mesh shader (image layers)"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            Some(crate::material_gpu::FillSet {
                fill: fill_pipeline(device, layout, samples, &shader),
                glass: glass_pipelines(device, layout, samples, &shader),
            })
        };
        if self.materials.sync(
            device,
            &self.state.queue,
            world.materials.as_deref(),
            &compile,
        ) {
            self.rebuild_scene_group();
        }
        // Patches draw the same programs (subd_gpu.rs).
        let module = self
            .materials
            .pipeline()
            .and(world.materials.as_deref())
            .map(|m| m.source.as_str());
        if let Some(p) = self.patch_pipelines.as_mut() {
            p.sync_materials(&self.state.device, module, world_shader);
        }
    }
}

/// The resident-mesh pipelines (`world_pipelines`).
struct WorldPipelines {
    camera_layout: wgpu::BindGroupLayout,
    look_layout: wgpu::BindGroupLayout,
    fill: wgpu::RenderPipeline,
    overlay: wgpu::RenderPipeline,
    edges: wgpu::RenderPipeline,
    dots: wgpu::RenderPipeline,
    xray_edges: wgpu::RenderPipeline,
    xray_dots: wgpu::RenderPipeline,
    topology_edges: wgpu::RenderPipeline,
    topology_dots: wgpu::RenderPipeline,
    clear: wgpu::RenderPipeline,
    clear_over: wgpu::RenderPipeline,
    glass: [wgpu::RenderPipeline; GLASS_PASSES],
    layout: wgpu::PipelineLayout,
}

#[cfg(test)]
mod tests {
    use super::Look;

    /// The packers write the layout they always did, on one core or many
    /// (heavy-mesh-speed): checked against the byte-by-byte packing they
    /// replaced, on lists either side of the parallel threshold.
    #[test]
    fn vertex_packing_is_unchanged_on_every_core() {
        use crate::gpu_scene::{EdgeVertex, Tint, WorldVertex};
        use eframe::egui::Color32;
        let f = |k: usize, i: usize| ((k * 31 + i * 7) % 97) as f32 / 13. - 3.;
        let c = |k: usize| Color32::from_rgba_premultiplied(k as u8, (k >> 3) as u8, 7, 255);
        for n in [5, 40_000] {
            let fill: Vec<WorldVertex> = (0..n)
                .map(|k| WorldVertex {
                    position: [f(k, 0), f(k, 1), f(k, 2)],
                    normal: [f(k, 3), f(k, 4), f(k, 5)],
                    color: c(k),
                    map: c(k + 1),
                    uv: [f(k, 6), f(k, 7)],
                    program: k as u32,
                    tint: Tint {
                        weight: [k as u8, 1, 2, 3],
                        distortion: [4, k as u8, 6, 7],
                    },
                })
                .collect();
            let mut old = Vec::new();
            for v in &fill {
                for x in v.position.iter().chain(&v.normal) {
                    old.extend_from_slice(&x.to_le_bytes());
                }
                old.extend_from_slice(&v.color.to_array());
                old.extend_from_slice(&v.map.to_array());
                for x in v.uv {
                    old.extend_from_slice(&x.to_le_bytes());
                }
                old.extend_from_slice(&v.program.to_le_bytes());
                old.extend_from_slice(&v.tint.weight);
                old.extend_from_slice(&v.tint.distortion);
            }
            assert_eq!(super::fill_bytes(&fill), old);
            let edges: Vec<EdgeVertex> = (0..n)
                .map(|k| EdgeVertex {
                    a: [f(k, 0), f(k, 1), f(k, 2)],
                    b: [f(k, 3), f(k, 4), f(k, 5)],
                    end: (k % 2) as f32,
                    side: f(k, 6),
                    width: f(k, 7),
                    color: c(k),
                    uses: (1 + k % 2) as f32,
                    sides: [[f(k, 8), f(k, 9), 0.5], [f(k, 10), -0.25, f(k, 11)]],
                    always: k % 3 == 0,
                })
                .collect();
            let snorm = |x: f32| ((x.clamp(-1., 1.) * 32767.).round() as i16).to_le_bytes();
            let mut old = Vec::new();
            for v in &edges {
                for x in v.a.iter().chain(&v.b).chain(&[v.end, v.side, v.width]) {
                    old.extend_from_slice(&x.to_le_bytes());
                }
                old.extend_from_slice(&v.color.to_array());
                old.extend_from_slice(&v.uses.to_le_bytes());
                let [n1, n2] = v.sides;
                for x in n1
                    .into_iter()
                    .chain([if v.always { 1. } else { 0. }])
                    .chain(n2)
                    .chain([0.])
                {
                    old.extend_from_slice(&snorm(x));
                }
            }
            assert_eq!(super::edge_bytes(&edges), old);
        }
    }
    use crate::view_display::ViewDisplay;
    use operandi_core::Mode;

    /// The uniform's two rows before `ambient` and the transparency rows:
    /// `plain` (Wireframe, w = recolour flag) and `inactive` (Inactive
    /// Wireframe).
    fn wire_rows(look: &Look) -> [[f32; 4]; 2] {
        let bytes = look.bytes();
        assert_eq!(bytes.len() as u64, Look::SIZE);
        let f: Vec<f32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        // Before the 3 transparency rows, the environment's 13, the wire
        // fade's 1 and the glass row.
        let n = f.len() - 16 - 13 * 4 - 4 - 4;
        [
            [f[n - 8], f[n - 7], f[n - 6], f[n - 5]],
            [f[n - 4], f[n - 3], f[n - 2], f[n - 1]],
        ]
    }
    fn rgb(c: eframe::egui::Color32) -> [f32; 3] {
        let a = c.to_array();
        [a[0], a[1], a[2]].map(|v| f32::from(v) / 255.)
    }

    /// The inactive look carries the flag, the colour to match (the baked
    /// Wireframe) and its Inactive Wireframe replacement; the active look
    /// over a filled style recolours its plain wire to Shaded Wireframe
    /// (black, VS-1).
    #[test]
    fn inactive_look_recolors_the_plain_wire_and_active_takes_shaded_wireframe() {
        let [active, inactive] = Look::pair(&ViewDisplay::default(), Mode::Polygon);
        assert_eq!(
            active.plain_wire,
            Some(crate::theme::viewport::shaded_wireframe())
        );
        assert_eq!(
            inactive.plain_wire,
            Some(crate::theme::viewport::inactive_wireframe())
        );
        let [plain, replacement] = wire_rows(&inactive);
        assert_eq!(plain[3], 1.);
        assert_eq!(plain[..3], rgb(crate::theme::viewport::wireframe()));
        assert_eq!(
            replacement[..3],
            rgb(crate::theme::viewport::inactive_wireframe())
        );
        assert_eq!(wire_rows(&active)[0][3], 1.);
        assert_eq!(wire_rows(&active)[1][..3], [0.; 3]);
    }

    /// VS-T1: the active meshes' plain wire is black in all eight filled
    /// styles; Wireframe (and Vertex Map) keep the light Wireframe colour.
    /// The colour stays themable.
    #[test]
    fn active_wire_is_black_in_every_filled_style() {
        use crate::view_display::Style;
        crate::theme::set_overrides(&Default::default());
        assert_eq!(
            crate::theme::viewport::SHADED_WIREFRAME,
            eframe::egui::Color32::BLACK
        );
        for style in [
            Style::Default,
            Style::Solid,
            Style::Shaded,
            Style::Texture,
            Style::Advanced,
            Style::Gooch,
            Style::Cel,
            Style::Reflection,
        ] {
            let mut d = ViewDisplay::default();
            d.settings.active.style = style;
            let [active, _] = Look::pair(&d, Mode::Polygon);
            assert_eq!(
                active.plain_wire,
                Some(eframe::egui::Color32::BLACK),
                "{style:?}"
            );
        }
        for style in [Style::Wireframe, Style::VertexMap] {
            let mut d = ViewDisplay::default();
            d.settings.active.style = style;
            assert_eq!(Look::pair(&d, Mode::Polygon)[0].plain_wire, None);
        }
        let teal = eframe::egui::Color32::from_rgb(0, 128, 128);
        crate::theme::set_override(crate::theme::ColorId::ShadedWireframe, Some(teal));
        let [active, _] = Look::pair(&ViewDisplay::default(), Mode::Polygon);
        assert_eq!(active.plain_wire, Some(teal));
        crate::theme::set_overrides(&Default::default());
    }

    /// Inactive Same as Active: inactive meshes match the active look,
    /// wire colour included.
    #[test]
    fn inactive_same_as_active_keeps_the_wire_colour() {
        let mut d = ViewDisplay::default();
        d.settings.drawing.inactive_same_as_active = true;
        let [active, inactive] = Look::pair(&d, Mode::Polygon);
        assert_eq!(active, inactive);
        assert_eq!(
            inactive.plain_wire,
            Some(crate::theme::viewport::shaded_wireframe())
        );
    }

    /// HDRI stage 3: the environment light rides at the end of the look
    /// uniform: 9 harmonics rows, 3 rotation rows, then (on, intensity).
    #[test]
    fn environment_light_rows_come_before_the_wire_fade() {
        let mut look = Look::of(ViewDisplay::default().settings.active);
        look.env = super::EnvLook {
            on: true,
            sh: [[0.5, 0.25, 0.125]; 9],
            to_image: [1., 0., 0., 0., 1., 0., 0., 0., 1.],
            intensity: 2.,
        };
        let f: Vec<f32> = look
            .bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        assert_eq!(f.len() as u64 * 4, Look::SIZE);
        // The glass row ends the look (viewport transparency), the wire
        // fade's comes before it, and the environment's before that.
        assert_eq!(&f[f.len() - 4..], &[0.; 4]);
        let f = &f[..f.len() - 4];
        assert_eq!(
            &f[f.len() - 4..],
            &[
                0.,
                super::WIRE_FADE_GONE,
                super::WIRE_FADE_FULL,
                super::WIRE_FADE_SHARP.to_radians().cos()
            ]
        );
        let n = f.len() - 4;
        assert_eq!(&f[n - 4..n], &[1., 2., 0., 0.]);
        assert_eq!(&f[n - 16..n - 12], &[1., 0., 0., 0.]);
        assert_eq!(&f[n - 52..n - 48], &[0.5, 0.25, 0.125, 0.]);
        assert_eq!(super::EnvLook::default().rows().last(), Some(0.));
    }

    /// Viewport transparency: see-through materials show only in the
    /// Advanced style (Jason, 2026-11-14), with Show Material Transparency
    /// on, and not in a Transparency Override group (which draws its own
    /// way). The look's last row carries it to the shader.
    #[test]
    fn glass_shows_only_in_advanced() {
        use crate::view_display::{Style, TransparencyMode};
        let base = Look {
            material_transparency: true,
            ..Look::of(ViewDisplay::default().settings.active)
        };
        let with = |style| Look { style, ..base };
        assert!(with(Style::Advanced).glass_on());
        let off = Look {
            material_transparency: false,
            ..with(Style::Advanced)
        };
        assert!(!off.glass_on(), "the setting off");
        for style in [
            Style::Shaded,
            Style::Texture,
            Style::Default,
            Style::Solid,
            Style::Gooch,
            Style::Cel,
            Style::Reflection,
            Style::Wireframe,
        ] {
            assert!(!with(style).glass_on(), "{style:?}");
        }
        let mut ghost = with(Style::Advanced);
        ghost.transparency.mode = TransparencyMode::Ghost;
        assert!(!ghost.glass_on(), "Transparency Override");
        let last = |l: &Look| {
            let b = l.bytes();
            f32::from_le_bytes(b[b.len() - 16..][..4].try_into().unwrap())
        };
        assert_eq!(last(&with(Style::Advanced)), 1.);
        assert_eq!(last(&with(Style::Default)), 0.);
    }

    /// A view's own inactive wire colour wins over the theme's.
    #[test]
    fn a_views_own_inactive_wire_colour_wins_over_the_theme() {
        let mut d = ViewDisplay::default();
        d.settings.inactive.wire_color = Some([10, 200, 30]);
        let [active, inactive] = Look::pair(&d, Mode::Polygon);
        let own = eframe::egui::Color32::from_rgb(10, 200, 30);
        assert_eq!(d.plain_wire_color(false), Some(own));
        assert_eq!(inactive.plain_wire, Some(own));
        assert_eq!(wire_rows(&inactive)[1][..3], rgb(own));
        assert_eq!(
            active.plain_wire,
            Some(crate::theme::viewport::shaded_wireframe())
        );
    }

    /// The active group's own wire colour recolours the active meshes'
    /// plain wire too; inactive meshes keep the theme's Inactive Wireframe.
    #[test]
    fn an_active_group_wire_colour_recolors_the_active_wire() {
        let mut d = ViewDisplay::default();
        d.settings.active.wire_color = Some([200, 40, 40]);
        let own = eframe::egui::Color32::from_rgb(200, 40, 40);
        let [active, inactive] = Look::pair(&d, Mode::Polygon);
        assert_eq!(active.plain_wire, Some(own));
        assert_eq!(wire_rows(&active)[0][3], 1.);
        assert_eq!(wire_rows(&active)[1][..3], rgb(own));
        assert_eq!(
            inactive.plain_wire,
            Some(crate::theme::viewport::inactive_wireframe())
        );
        // Inactive Same as Active: inactive meshes take the active colour.
        d.settings.drawing.inactive_same_as_active = true;
        assert_eq!(Look::pair(&d, Mode::Polygon)[1].plain_wire, Some(own));
    }

    /// The shader swaps only the plain wire, by the flag.
    #[test]
    fn edge_shader_recolors_by_the_flag() {
        let s = super::plain_world_shader();
        assert!(s.contains("look.plain.w > 0.5"));
        assert!(s.contains("out.color = vec4<f32>(look.inactive.rgb, c.a);"));
    }

    /// The resident shader (with the shared display shading) parses and
    /// validates, so a typo can't leave the viewport blank.
    #[test]
    fn world_shader_validates() {
        use egui_wgpu::wgpu::naga;
        let source = super::plain_world_shader();
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}"));
    }

    /// Forum report (disappearing wireframe): the wire cuts an edge at the
    /// near clip as `camera::clip_to_near` does (same NEAR_KEEP) and culls
    /// only when both ends are behind, never when one is.
    #[test]
    fn edge_shader_cuts_edges_at_the_near_clip() {
        let s = super::plain_world_shader();
        assert!(s.contains(&format!(
            "const NEAR_KEEP: f32 = {:?};",
            crate::camera::NEAR_KEEP as f32
        )));
        assert!(
            s.contains("if (ca.w < keep && cb.w < keep) { out.position = culled; return out; }")
        );
        assert!(!s.contains("ca.w < cam.params.w || cb.w < cam.params.w"));
    }

    #[test]
    fn patch_ranges_merge_when_they_touch() {
        assert_eq!(
            super::merged(&[6..9, 0..3, 3..6, 12..15, 13..14, 20..20]),
            vec![0..9, 12..15]
        );
    }

    #[test]
    fn shadow_pass_shader_validates() {
        use egui_wgpu::wgpu::naga;
        let module = naga::front::wgsl::parse_str(super::SHADOW_PASS_SHADER).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }

    /// The light's matrix puts the whole bounds inside the shadow map, and
    /// nearer the light means smaller depth: a box over a plane, lit from
    /// above, lands in front of the plane below it.
    #[test]
    fn shadow_matrix_fits_the_bounds_and_faces_the_light() {
        use super::{KeyLight, shadow_matrix};
        use glam::Vec3;
        let (lo, hi) = ([-2., 0., -2.], [2., 2., 2.]);
        for key in [
            KeyLight::default(),
            KeyLight::Toward([0., 1., 0.]),
            KeyLight::At([5., 5., 5.]),
        ] {
            let (m, toward) = shadow_matrix(key, lo, hi);
            assert!((toward.length() - 1.).abs() < 1e-5);
            for x in [lo[0], hi[0]] {
                for y in [lo[1], hi[1]] {
                    for z in [lo[2], hi[2]] {
                        let q = m.project_point3(Vec3::new(x, y, z));
                        assert!(q.x.abs() <= 1. && q.y.abs() <= 1., "{key:?} {q}");
                        assert!((0. ..=1.).contains(&q.z), "{key:?} {q}");
                    }
                }
            }
        }
        let (m, _) = shadow_matrix(KeyLight::Toward([0., 1., 0.]), lo, hi);
        let top = m.project_point3(Vec3::new(0., 1., 0.));
        let floor = m.project_point3(Vec3::new(0., 0., 0.));
        assert!(top.z < floor.z);
        // Same spot in the map: the box shadows the floor under it.
        assert!((top.x - floor.x).abs() < 1e-5 && (top.y - floor.y).abs() < 1e-5);
    }

    // ------------------------------------------- VS-6 (VS-T13..T16)
    // The GPU pass draws a mesh's plain wire only when its look's `wire()`
    // holds, in `colored` for the Colored overlay; the selection highlight
    // is its own buffer (gpu_scene `highlight`), drawn whenever Show
    // Selected Outline is on.

    /// The Colored wire colour the shader gets (the uniform's second row).
    fn colored_row(look: &Look) -> [f32; 3] {
        let bytes = look.bytes();
        let f: Vec<f32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        [f[4], f[5], f[6]]
    }

    /// VS-T13: a fresh view, Item mode: the cube draws its plain wire,
    /// Colored and black.
    #[test]
    fn vs_t13_fresh_view_item_mode_draws_a_black_plain_wire() {
        let d = ViewDisplay::default();
        assert!(d.settings.drawing.item_mode_wireframe);
        for look in Look::pair(&d, Mode::Item) {
            assert!(look.wire());
            assert_eq!(look.overlay, crate::view_display::Overlay::Colored);
            assert_eq!(look.opacity, 1.);
            assert_eq!(colored_row(&look), [0.; 3]);
        }
    }

    /// VS-T14: setting off, nothing selected, Item mode: no plain wire on
    /// any mesh, though the overlay is still Colored, a Wireframe Color
    /// override included.
    #[test]
    fn vs_t14_setting_off_hides_every_plain_wire_in_item_mode() {
        let mut d = ViewDisplay::default();
        d.settings.drawing.item_mode_wireframe = false;
        d.settings.active.wire_color = Some([200, 40, 40]);
        d.settings.inactive.wire_color = Some([10, 200, 30]);
        assert_eq!(
            d.settings.active.overlay,
            crate::view_display::Overlay::Colored
        );
        for look in Look::pair(&d, Mode::Item) {
            assert!(!look.wire());
        }
    }

    /// VS-T15: the same view in Polygon (and Vertex, Edge) mode: the black
    /// plain wire is back; the setting doesn't touch component modes.
    #[test]
    fn vs_t15_component_modes_keep_the_black_wire_with_the_setting_off() {
        for on in [false, true] {
            let mut d = ViewDisplay::default();
            d.settings.drawing.item_mode_wireframe = on;
            for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon] {
                let [active, inactive] = Look::pair(&d, mode);
                for look in [active, inactive] {
                    assert!(look.wire(), "{mode:?} {on}");
                    assert_eq!(colored_row(&look), [0.; 3]);
                }
            }
        }
        // A group's Wireframe Color still colours the Colored wire.
        let mut d = ViewDisplay::default();
        d.settings.active.wire_color = Some([255, 0, 0]);
        assert_eq!(colored_row(&Look::pair(&d, Mode::Polygon)[0]), [1., 0., 0.]);
    }

    /// VS-T16: setting off, the cube selected, Item mode: no plain wire,
    /// but the selection highlight (every edge of the selected item, in
    /// the selection colour, on top) still draws.
    #[test]
    fn vs_t16_selected_item_keeps_only_its_selection_highlight() {
        use glam::{DMat4, DVec3};
        use std::collections::BTreeSet;
        let mut d = ViewDisplay::default();
        d.settings.drawing.item_mode_wireframe = false;
        assert!(d.settings.drawing.selected_outline);
        let [active, _] = Look::pair(&d, Mode::Item);
        assert!(!active.wire());
        let mut scene = operandi_core::Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].mesh = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1)
            .unwrap()
            .into();
        let mut resident = crate::gpu_scene::Resident::default();
        let whole = Some((true, BTreeSet::new(), BTreeSet::new()));
        let (g, _, _) = resident.item(
            &scene.items[0],
            DMat4::IDENTITY,
            whole,
            None,
            &operandi_core::shading::SmoothingSettings::default(),
        );
        // A box's 12 edges, each shared by two sides and drawn once.
        assert_eq!(g.highlight.len(), 12 * 6);
        assert!(g.highlight.iter().all(|e| e.uses == 2.));
        assert!(
            g.highlight
                .iter()
                .all(|e| e.color == crate::theme::viewport::selection())
        );
    }
}
