//! Image layers drawn per pixel in the viewport: the GPU side of the
//! material programs (`gpu_scene::GpuMaterials`, generated from the folded
//! shader tree by `operandi_core::shader_wgsl`). Each image file is uploaded
//! once (with mipmaps) and freed when no program reads it; the parameters
//! and instance table are rewritten when they change; fill pipelines are
//! cached by their generated source, so a shader compiles only when the
//! tree's shape changes, never during a drag.
use crate::gpu_scene::{GpuMaterials, ImageData};
use egui_wgpu::wgpu::{self, util::DeviceExt};
use operandi_core::blender_principled::LUT_ROWS;
use operandi_core::shader_wgsl::MAX_IMAGES;
use std::collections::HashMap;
use std::sync::Arc;

/// Scene bind group (group 2) bindings after the shadow map's three.
pub const PARAMS: u32 = 3;
pub const INSTANCES: u32 = 4;
pub const FIRST_SAMPLER: u32 = 5;
/// Nearest then linear, each with repeat, mirror-repeat and clamp.
pub const SAMPLERS: [&str; 6] = ["n_r", "n_m", "n_c", "l_r", "l_m", "l_c"];
pub const FIRST_IMAGE: u32 = FIRST_SAMPLER + SAMPLERS.len() as u32;

/// A resident shader module's fill pipelines: the solid fill, then the
/// see-through passes of viewport transparency (gpu.rs `glass_pipelines`:
/// far side tint, far side light, near side tint, near side light).
pub struct FillSet {
    pub fill: wgpu::RenderPipeline,
    pub glass: [wgpu::RenderPipeline; crate::gpu::GLASS_PASSES],
}

/// Fill pipelines kept by generated source (a few, for undo and redo).
const KEEP_PIPELINES: usize = 6;

/// The WGSL declarations of the material resources in group 2.
pub fn bindings() -> String {
    let mut out = format!(
        "@group(2) @binding({PARAMS}) var<storage, read> op_params: array<vec4<f32>>;\n\
         @group(2) @binding({INSTANCES}) var<storage, read> op_instances: array<vec4<u32>>;\n"
    );
    for (k, name) in SAMPLERS.iter().enumerate() {
        out += &format!(
            "@group(2) @binding({}) var op_smp_{name}: sampler;\n",
            FIRST_SAMPLER + k as u32
        );
    }
    for k in 0..MAX_IMAGES {
        out += &format!(
            "@group(2) @binding({}) var op_img{k}: texture_2d<f32>;\n",
            FIRST_IMAGE + k as u32
        );
    }
    out
}

/// The scene bind group layout's material entries.
/// `vertex`: the vertex stage reads them too (the fill's displacement,
/// VM-Q11); the patch pipelines leave them to the fragment stage, which
/// keeps their vertex stage under Metal's 16 buffers.
pub fn layout_entries(vertex: bool) -> Vec<wgpu::BindGroupLayoutEntry> {
    let stages = if vertex {
        wgpu::ShaderStages::VERTEX_FRAGMENT
    } else {
        wgpu::ShaderStages::FRAGMENT
    };
    let storage = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: stages,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let mut out = vec![storage(PARAMS), storage(INSTANCES)];
    out.extend(
        (0..SAMPLERS.len() as u32).map(|k| wgpu::BindGroupLayoutEntry {
            binding: FIRST_SAMPLER + k,
            visibility: stages,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }),
    );
    out.extend((0..MAX_IMAGES as u32).map(|k| wgpu::BindGroupLayoutEntry {
        binding: FIRST_IMAGE + k,
        visibility: stages,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }));
    out
}

pub struct MaterialGpu {
    /// The `GpuMaterials` generation applied (0: none).
    generation: u64,
    params: wgpu::Buffer,
    instances: wgpu::Buffer,
    /// Per instance, see-through tint and amount (`GpuMaterials::clear`),
    /// bound at gpu.rs `CLEAR_BINDING`; row 0 opaque.
    clear: wgpu::Buffer,
    /// Per instance, how much light its shadow lets through
    /// (`GpuMaterials::shadow`), for the lighter-shadow pass.
    shadow: wgpu::Buffer,
    samplers: Vec<wgpu::Sampler>,
    /// Empty slots read this.
    blank: wgpu::TextureView,
    /// Uploaded images by `ImageData::key`.
    textures: HashMap<u64, wgpu::TextureView>,
    /// The image key in each slot.
    slots: Vec<u64>,
    /// Compiled fill pipelines by generated source, most recent last.
    pipelines: Vec<(String, Arc<FillSet>)>,
    /// The pipeline the current materials draw with (`None`: the plain
    /// fill pipeline, which reads no programs).
    current: Option<Arc<FillSet>>,
    /// Shader compiles so far (speed guards: a drag compiles nothing).
    pub compiles: u64,
    /// Image uploads so far.
    pub uploads: u64,
    /// Painted images (`GpuMaterials::painted`): their texture and every
    /// mip level's pixels, kept so a stroke rewrites only what it changed.
    painted: HashMap<u64, Painted>,
    /// Changed rectangles written into painted textures so far.
    #[cfg_attr(not(feature = "painting"), allow(dead_code))]
    pub patches: u64,
}

/// A painted image on the card, with its mip levels' pixels (level 0
/// first) as uploaded. Painting builds only make them.
#[cfg_attr(not(feature = "painting"), allow(dead_code))]
struct Painted {
    texture: wgpu::Texture,
    levels: Vec<(u32, u32, Vec<u8>)>,
}

impl MaterialGpu {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let samplers = SAMPLERS
            .iter()
            .map(|name| {
                let linear = name.starts_with('l');
                let address = match &name[2..] {
                    "r" => wgpu::AddressMode::Repeat,
                    "m" => wgpu::AddressMode::MirrorRepeat,
                    _ => wgpu::AddressMode::ClampToEdge,
                };
                let filter = if linear {
                    wgpu::FilterMode::Linear
                } else {
                    wgpu::FilterMode::Nearest
                };
                device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("Operandi image layer"),
                    address_mode_u: address,
                    address_mode_v: address,
                    address_mode_w: address,
                    mag_filter: filter,
                    min_filter: filter,
                    // Mipmaps either way, so distant surfaces don't shimmer.
                    mipmap_filter: filter,
                    anisotropy_clamp: if linear { 8 } else { 1 },
                    ..Default::default()
                })
            })
            .collect();
        let blank = upload(device, queue, &ImageData::unreadable());
        Self {
            generation: 0,
            params: storage(device, "Operandi material parameters", &param_bytes(&[])),
            instances: storage(device, "Operandi material instances", &[0; 16]),
            clear: storage(
                device,
                "Operandi material see-through",
                &clear_bytes(&[crate::gpu_scene::OPAQUE]),
            ),
            shadow: storage(
                device,
                "Operandi material shadow share",
                &clear_bytes(&[crate::gpu_scene::OPAQUE]),
            ),
            samplers,
            blank,
            textures: HashMap::new(),
            slots: Vec::new(),
            pipelines: Vec::new(),
            current: None,
            compiles: 0,
            uploads: 0,
            painted: HashMap::new(),
            patches: 0,
        }
    }

    /// Writes the changed rectangle `rect` (x0, y0, x1, y1, inclusive) of
    /// the painted image uploaded as `key`, its 8-bit sRGB pixels `rgba`
    /// row by row, into its texture, and the mip levels' pixels above it.
    /// `false` when `key` isn't a painted image on the card (the caller
    /// then uploads a new picture).
    #[cfg(feature = "painting")]
    pub fn patch(
        &mut self,
        queue: &wgpu::Queue,
        key: u64,
        rect: (u32, u32, u32, u32),
        rgba: &[u8],
    ) -> bool {
        let Some(p) = self.painted.get_mut(&key) else {
            return false;
        };
        let (x0, y0, x1, y1) = rect;
        let (w, h) = (p.levels[0].0, p.levels[0].1);
        let (rw, rh) = (x1.saturating_sub(x0) + 1, y1.saturating_sub(y0) + 1);
        if x1 >= w || y1 >= h || x0 > x1 || y0 > y1 || rgba.len() != (rw * rh * 4) as usize {
            return false;
        }
        let write = |level: u32, (x, y): (u32, u32), (rw, rh): (u32, u32), bytes: &[u8]| {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &p.texture,
                    mip_level: level,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * rw),
                    rows_per_image: Some(rh),
                },
                wgpu::Extent3d {
                    width: rw,
                    height: rh,
                    depth_or_array_layers: 1,
                },
            );
        };
        {
            let level0 = &mut p.levels[0].2;
            for row in 0..rh {
                let to = (((y0 + row) * w + x0) * 4) as usize;
                let from = (row * rw * 4) as usize;
                level0[to..to + (rw * 4) as usize]
                    .copy_from_slice(&rgba[from..from + (rw * 4) as usize]);
            }
        }
        write(0, (x0, y0), (rw, rh), rgba);
        // Each level below: the pixels over the changed ones, averaged
        // again from the level above (as `mip_chain` makes them).
        let (mut a, mut b) = ((x0, y0), (x1, y1));
        for k in 1..p.levels.len() {
            let (above, rest) = p.levels.split_at_mut(k);
            let (aw, ah, above) = (&above[k - 1].0, &above[k - 1].1, &above[k - 1].2);
            let (nw, nh, next) = (&rest[0].0, &rest[0].1, &mut rest[0].2);
            a = (a.0 / 2, a.1 / 2);
            b = ((b.0 / 2).min(nw - 1), (b.1 / 2).min(nh - 1));
            let (rw, rh) = (b.0 - a.0 + 1, b.1 - a.1 + 1);
            let mut bytes = Vec::with_capacity((rw * rh * 4) as usize);
            for y in a.1..=b.1 {
                for x in a.0..=b.0 {
                    let px = mip_pixel(above, *aw, *ah, x, y);
                    let o = ((y * nw + x) * 4) as usize;
                    next[o..o + 4].copy_from_slice(&px);
                    bytes.extend_from_slice(&px);
                }
            }
            write(k as u32, a, (rw, rh), &bytes);
        }
        self.patches += 1;
        true
    }

    /// The fill pipeline for the current materials, if they need their own.
    pub fn pipeline(&self) -> Option<&FillSet> {
        self.current.as_deref()
    }

    /// The see-through table's scene group entry (gpu.rs `CLEAR_BINDING`).
    pub fn clear_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: self.clear.as_entire_binding(),
        }
    }

    /// The lighter-shadow table (gpu.rs shadow pass), shaped as the
    /// see-through one with the shadow's share in `w`.
    pub fn shadow_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: self.shadow.as_entire_binding(),
        }
    }

    /// Bring the GPU side up to `materials`. Returns true when the scene
    /// bind group must be rebuilt (buffers or textures changed). `compile`
    /// makes the fill pipeline for a generated module.
    pub fn sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        materials: Option<&GpuMaterials>,
        compile: &dyn Fn(&str) -> Option<FillSet>,
    ) -> bool {
        let generation = materials.map_or(0, |m| m.generation);
        if generation == self.generation {
            return false;
        }
        self.generation = generation;
        let Some(m) = materials else {
            self.current = None;
            self.slots.clear();
            self.textures.clear();
            // The plain pipeline still reads the reflection table.
            write_storage(
                device,
                queue,
                &mut self.params,
                "Operandi material parameters",
                &param_bytes(&[]),
            );
            write_storage(
                device,
                queue,
                &mut self.clear,
                "Operandi material see-through",
                &clear_bytes(&[crate::gpu_scene::OPAQUE]),
            );
            write_storage(
                device,
                queue,
                &mut self.shadow,
                "Operandi material shadow share",
                &clear_bytes(&[crate::gpu_scene::OPAQUE]),
            );
            return true;
        };
        // The pipeline: cached by source, compiled once.
        let pipeline = match self.pipelines.iter().position(|(s, _)| *s == m.source) {
            Some(k) => {
                let entry = self.pipelines.remove(k);
                self.pipelines.push(entry);
                self.pipelines.last().map(|(_, p)| p.clone())
            }
            None => {
                self.compiles += 1;
                let made = compile(&m.source).map(Arc::new);
                if let Some(p) = &made {
                    self.pipelines.push((m.source.clone(), p.clone()));
                    if self.pipelines.len() > KEEP_PIPELINES {
                        self.pipelines.remove(0);
                    }
                }
                made
            }
        };
        // A module that didn't compile draws every program's polygons in
        // their vertex colour (code 0), never black. The parameters sit
        // after the reflection table (`param_bytes`).
        let shift = LUT_ROWS as u32;
        let instances: Vec<[u32; 4]> = m
            .instances
            .iter()
            .map(|&[code, b, s, has]| {
                let code = if pipeline.is_some() { code } else { 0 };
                // Without the module the lists go too (bit 0 stays).
                let has = if pipeline.is_some() { has } else { has & 1 };
                [code, b + shift, if has != 0 { s + shift } else { 0 }, has]
            })
            .collect();
        self.current = pipeline;
        write_storage(
            device,
            queue,
            &mut self.params,
            "Operandi material parameters",
            &param_bytes(&m.params),
        );
        let bytes: Vec<u8> = instances
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        write_storage(
            device,
            queue,
            &mut self.instances,
            "Operandi material instances",
            &bytes,
        );
        // See-through rows don't need the module: without it the glass
        // passes run the plain pipelines.
        write_storage(
            device,
            queue,
            &mut self.clear,
            "Operandi material see-through",
            &clear_bytes(&m.clear),
        );
        write_storage(
            device,
            queue,
            &mut self.shadow,
            "Operandi material shadow share",
            &clear_bytes(&m.shadow),
        );
        // Textures: upload new reads, free what nothing reads. Painted
        // images keep their levels' pixels for `patch`.
        for image in &m.images {
            if !self.textures.contains_key(&image.key) {
                self.uploads += 1;
                let painted = m.painted.iter().any(|&(_, k)| k == image.key);
                let (view, kept) = upload_keeping(device, queue, image, painted);
                self.textures.insert(image.key, view);
                if let Some(kept) = kept {
                    self.painted.insert(image.key, kept);
                }
            }
        }
        self.slots = m.images.iter().map(|i| i.key).collect();
        let used = &self.slots;
        self.textures.retain(|k, _| used.contains(k));
        self.painted.retain(|k, _| used.contains(k));
        true
    }

    /// The material entries of the scene bind group.
    pub fn entries(&self) -> Vec<wgpu::BindGroupEntry<'_>> {
        let mut out = vec![
            wgpu::BindGroupEntry {
                binding: PARAMS,
                resource: self.params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: INSTANCES,
                resource: self.instances.as_entire_binding(),
            },
        ];
        out.extend(
            self.samplers
                .iter()
                .enumerate()
                .map(|(k, s)| wgpu::BindGroupEntry {
                    binding: FIRST_SAMPLER + k as u32,
                    resource: wgpu::BindingResource::Sampler(s),
                }),
        );
        out.extend((0..MAX_IMAGES).map(|k| {
            let view = self
                .slots
                .get(k)
                .and_then(|key| self.textures.get(key))
                .unwrap_or(&self.blank);
            wgpu::BindGroupEntry {
                binding: FIRST_IMAGE + k as u32,
                resource: wgpu::BindingResource::TextureView(view),
            }
        }));
        out
    }
}

/// The parameter buffer: Blender's reflection table for the Advanced
/// style's principled surfaces (blender_principled.rs), then `rows`.
fn param_bytes(rows: &[[f32; 4]]) -> Vec<u8> {
    operandi_core::blender_principled::lut()
        .iter()
        .chain(rows)
        .flatten()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

/// The see-through table (never empty: row 0 is opaque).
fn clear_bytes(rows: &[[f32; 4]]) -> Vec<u8> {
    let rows = if rows.is_empty() {
        &[crate::gpu_scene::OPAQUE][..]
    } else {
        rows
    };
    rows.iter()
        .flatten()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

fn storage(device: &wgpu::Device, label: &str, bytes: &[u8]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    })
}

/// Write `bytes` into `buffer`, growing it when they don't fit.
fn write_storage(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut wgpu::Buffer,
    label: &str,
    bytes: &[u8],
) {
    if bytes.is_empty() {
        return;
    }
    if (bytes.len() as u64) > buffer.size() {
        let mut padded = bytes.to_vec();
        // Room to grow without a new buffer every time.
        padded.resize(bytes.len().next_power_of_two().max(16), 0);
        *buffer = storage(device, label, &padded);
    } else {
        queue.write_buffer(buffer, 0, bytes);
    }
}

/// An image as an sRGB texture with its mipmaps.
pub(crate) fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: &ImageData,
) -> wgpu::TextureView {
    upload_keeping(device, queue, image, false).0
}

/// `upload`, also giving back the texture and its levels' pixels when
/// `keep` (painted images).
fn upload_keeping(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: &ImageData,
    keep: bool,
) -> (wgpu::TextureView, Option<Painted>) {
    let levels = mip_chain(image.width, image.height, &image.rgba);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Operandi image layer"),
        size: wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32 + 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let write = |level: u32, w: u32, h: u32, rgba: &[u8]| {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    };
    write(0, image.width, image.height, &image.rgba);
    for (k, (w, h, rgba)) in levels.iter().enumerate() {
        write(k as u32 + 1, *w, *h, rgba);
    }
    let view = texture.create_view(&Default::default());
    let kept = keep.then(|| {
        let mut all = vec![(image.width, image.height, image.rgba.clone())];
        all.extend(levels);
        Painted {
            texture,
            levels: all,
        }
    });
    (view, kept)
}

/// The sRGB decode table and linear encode table the mip levels use.
fn mip_tables() -> &'static (Vec<f32>, Vec<u8>) {
    static TABLES: std::sync::OnceLock<(Vec<f32>, Vec<u8>)> = std::sync::OnceLock::new();
    TABLES.get_or_init(|| {
        let decode = (0..=255u8)
            .map(eframe::egui::ecolor::linear_f32_from_gamma_u8)
            .collect();
        let encode = (0..MIP_STEPS)
            .map(|i| {
                eframe::egui::ecolor::gamma_u8_from_linear_f32(i as f32 / (MIP_STEPS - 1) as f32)
            })
            .collect();
        (decode, encode)
    })
}
const MIP_STEPS: usize = 4096;

/// Pixel (x, y) of the level below `above` (`w` x `h`): its 2 x 2 block
/// averaged in linear light.
fn mip_pixel(above: &[u8], w: u32, h: u32, x: u32, y: u32) -> [u8; 4] {
    let (decode, encode) = mip_tables();
    let mut sum = [0f32; 4];
    let mut count = 0.;
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let (sx, sy) = ((2 * x + dx).min(w - 1), (2 * y + dy).min(h - 1));
        let i = ((sy * w + sx) * 4) as usize;
        for c in 0..3 {
            sum[c] += decode[above[i + c] as usize];
        }
        sum[3] += f32::from(above[i + 3]);
        count += 1.;
    }
    let mut out = [0u8; 4];
    for c in 0..3 {
        let l = (sum[c] / count).clamp(0., 1.);
        out[c] = encode[(l * (MIP_STEPS - 1) as f32).round() as usize];
    }
    out[3] = (sum[3] / count).round() as u8;
    out
}

/// The mip levels below `rgba` (sRGB-encoded RGBA), each half the size
/// (at least 1), averaged in linear light.
pub fn mip_chain(width: u32, height: u32, rgba: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    let mut out: Vec<(u32, u32, Vec<u8>)> = Vec::new();
    let (mut w, mut h) = (width, height);
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let above: &[u8] = out.last().map_or(rgba, |l| &l.2);
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                let o = ((y * nw + x) * 4) as usize;
                next[o..o + 4].copy_from_slice(&mip_pixel(above, w, h, x, y));
            }
        }
        out.push((nw, nh, next));
        (w, h) = (nw, nh);
    }
    out
}

/// Whether a generated module parses and validates, so a translation bug
/// draws the vertex colour instead of failing on the device.
pub fn valid(source: &str) -> Result<(), String> {
    use wgpu::naga;
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map(|_| ())
    .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn mip_chain_halves_to_one_pixel_in_linear_light() {
        // Black and white columns average to linear 0.5, not sRGB 128.
        let rgba: Vec<u8> = (0..4 * 4)
            .flat_map(|i| {
                let v = if i % 2 == 0 { 0 } else { 255 };
                [v, v, v, 255]
            })
            .collect();
        let chain = super::mip_chain(4, 4, &rgba);
        assert_eq!(
            chain.iter().map(|(w, h, _)| (*w, *h)).collect::<Vec<_>>(),
            vec![(2, 2), (1, 1)]
        );
        let half = eframe::egui::ecolor::gamma_u8_from_linear_f32(0.5);
        assert!(chain[1].2[0].abs_diff(half) <= 1, "{}", chain[1].2[0]);
        assert_eq!(chain[1].2[3], 255);
    }
}
