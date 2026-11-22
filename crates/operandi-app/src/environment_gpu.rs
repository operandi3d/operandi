//! The environment's picture in the 3D view (HDRI stages 2 and 3).
//!
//! When a view's Background is Environment and the environment the camera
//! sees has an enabled Image Map layer, the picture is drawn on the
//! graphics card behind everything: one full-screen triangle whose pixels
//! turn into world ray directions with the view's camera, looked up in the
//! latitude-longitude picture with the formula Cycles uses
//! (`operandi_core::environment::environment_image_uv`, the same pixel for
//! the same direction and Rotation). It is multiplied by Intensity, laid
//! over the CPU gradient underneath by the layer's Opacity, clamped to
//! 0..1 and stored gamma-encoded like the rest of the view.
//!
//! A Light Probe picture (a mirror-ball photo) is unwrapped to
//! latitude-longitude when it is decoded
//! (`operandi_core::environment::environment_probe_uv`, which Cycles'
//! Mirror Ball texture matches), so the rest draws it like any other.
//!
//! Pictures are decoded off the UI thread, shrunk to at most
//! [`MAX_WIDTH`] pixels wide, kept as half floats (Rgba16Float), and cached
//! by file path. Each one also carries its 9-coefficient spherical
//! harmonic irradiance (stage 3: the shaded view's ambient light) and a
//! small blurred copy for glossy reflections.
//!
//! [chosen] Only the top enabled image layer draws; its Blend Mode draws
//! as Normal. Orthographic views look along one direction, so they show
//! one colour of the sky, as a render through an orthographic camera
//! would.
use egui_wgpu::wgpu::{self, util::DeviceExt};
use std::collections::HashMap;
use std::sync::{Arc, mpsc};

/// Widest picture kept for the view [brief: at most 4096 px].
pub const MAX_WIDTH: u32 = 4096;
/// The blurred copy used for glossy reflections: this wide, half as tall.
pub const BLUR_WIDTH: u32 = 128;

/// A decoded environment picture, ready to upload.
pub struct SkyPicture {
    /// Unique per decode, so a texture is uploaded once.
    pub key: u64,
    pub width: u32,
    pub height: u32,
    /// RGBA half floats (`half::f16` bits), rows top to bottom, linear.
    pub texels: Vec<u16>,
    /// The blurred copy: `BLUR_WIDTH` × `BLUR_WIDTH / 2`, same layout.
    pub blurred: Vec<u16>,
    /// Irradiance spherical harmonics in the picture's own frame (+Y the
    /// pole, +Z the picture's centre), 9 RGB coefficients with the cosine
    /// lobe folded in: `irradiance(n) = Σ sh[k] · Y_k(n)`, divided by π so a
    /// white sky of radiance 1 gives 1 [Ramamoorthi & Hanrahan 2001].
    pub sh: [[f32; 3]; 9],
}

/// What one view draws behind everything this frame.
pub struct SkyDraw {
    pub picture: Arc<SkyPicture>,
    /// World direction to the picture's frame, row-major (the image layer's
    /// Texture Locator rotation undone).
    pub to_image: [f32; 9],
    /// Environment Intensity.
    pub intensity: f32,
    /// The image layer's Opacity over the gradient underneath.
    pub opacity: f32,
    pub invert: bool,
}

/// Decoded pictures by path, and the decodes still running.
#[derive(Default)]
pub struct SkyCache {
    ready: HashMap<String, Result<Arc<SkyPicture>, String>>,
    loading: HashMap<String, mpsc::Receiver<Result<SkyPicture, String>>>,
    next_key: u64,
}

/// The cache key of picture file `path` read as latitude-longitude, or as
/// a Light Probe (mirror ball) to unwrap. A NUL never appears in a path.
pub fn sky_key(path: &str, light_probe: bool) -> String {
    if light_probe {
        format!("{path}{PROBE_SUFFIX}")
    } else {
        path.to_owned()
    }
}
const PROBE_SUFFIX: &str = "\0light-probe";
/// Cache keys of generated pictures (`SkyCache::generated`); no path
/// starts with a NUL.
const GENERATED_PREFIX: &str = "\0generated:";

impl SkyCache {
    /// The picture of cache key `path` (`sky_key`): `Some` once decoded,
    /// `None` while it decodes (started now if it wasn't) or when it can't
    /// be read. `Err` paths are remembered so a broken file isn't read
    /// every frame.
    pub fn get(&mut self, path: &str) -> Option<Arc<SkyPicture>> {
        if let Some(found) = self.ready.get(path) {
            return found.as_ref().ok().cloned();
        }
        if let Some(rx) = self.loading.get(path) {
            match rx.try_recv() {
                Ok(result) => {
                    self.loading.remove(path);
                    let result = result.map(|mut p| {
                        self.next_key += 1;
                        p.key = self.next_key;
                        Arc::new(p)
                    });
                    let out = result.as_ref().ok().cloned();
                    self.ready.insert(path.to_owned(), result);
                    return out;
                }
                Err(mpsc::TryRecvError::Empty) => return None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.loading.remove(path);
                    self.ready
                        .insert(path.to_owned(), Err("The picture's decode stopped".into()));
                    return None;
                }
            }
        }
        let (tx, rx) = mpsc::channel();
        let owned = path.to_owned();
        let started = std::thread::Builder::new()
            .name("environment image".into())
            .spawn(move || {
                let picture = match owned.strip_suffix(PROBE_SUFFIX) {
                    Some(file) => decode_as(std::path::Path::new(file), true),
                    None => decode(std::path::Path::new(&owned)),
                };
                let _ = tx.send(picture);
            });
        if started.is_ok() {
            self.loading.insert(path.to_owned(), rx);
        }
        None
    }

    /// A picture made from the environment's own colours under cache key
    /// `key` (say, a hash of its settings): made now the first time; the
    /// previous generated picture is dropped, so editing a gradient keeps
    /// one copy.
    pub fn generated(&mut self, key: &str, make: impl FnOnce() -> SkyPicture) -> Arc<SkyPicture> {
        let full = format!("{GENERATED_PREFIX}{key}");
        if let Some(Ok(found)) = self.ready.get(&full) {
            return found.clone();
        }
        self.ready.retain(|k, _| !k.starts_with(GENERATED_PREFIX));
        let mut picture = make();
        self.next_key += 1;
        picture.key = self.next_key;
        let picture = Arc::new(picture);
        self.ready.insert(full, Ok(picture.clone()));
        picture
    }

    /// Whether any picture is still decoding (the view asks for frames
    /// until it is done).
    pub fn busy(&self) -> bool {
        !self.loading.is_empty()
    }

    /// Why picture file `path` couldn't be shown, once its decode failed
    /// (as either reading).
    pub fn error(&self, path: &str) -> Option<&str> {
        [sky_key(path, false), sky_key(path, true)]
            .iter()
            .find_map(|k| self.ready.get(k)?.as_ref().err().map(String::as_str))
    }

    /// Keys of every decoded picture (textures of others are dropped).
    pub fn keys(&self) -> std::collections::BTreeSet<u64> {
        self.ready
            .values()
            .filter_map(|r| r.as_ref().ok().map(|p| p.key))
            .collect()
    }

    /// Forget a path (Reload Image, or the file changed).
    pub fn forget(&mut self, path: &str) {
        for key in [sky_key(path, false), sky_key(path, true)] {
            self.ready.remove(&key);
            self.loading.remove(&key);
        }
    }

    /// Wait for `path` (tests and offscreen captures).
    #[cfg_attr(not(any(test, feature = "qa")), allow(dead_code))]
    pub fn wait(&mut self, path: &str) -> Option<Arc<SkyPicture>> {
        for _ in 0..2000 {
            if let Some(p) = self.get(path) {
                return Some(p);
            }
            if self.ready.contains_key(path) {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        None
    }
}

/// Read `path` as linear RGBA floats, shrink it, and build the blurred
/// copy and the irradiance harmonics. PNG, JPEG and TIFF are sRGB-encoded
/// and are linearised, as Cycles reads them; HDR and EXR are already
/// linear.
pub fn decode(path: &std::path::Path) -> Result<SkyPicture, String> {
    decode_as(path, false)
}

/// `decode`, reading the file as a Light Probe (mirror ball) and
/// unwrapping it to latitude-longitude first when `light_probe` is set.
pub fn decode_as(path: &std::path::Path, light_probe: bool) -> Result<SkyPicture, String> {
    let name = path.display();
    let picture = image::ImageReader::open(path)
        .map_err(|e| format!("Can't open {name}: {e}"))?
        .with_guessed_format()
        .map_err(|e| format!("Can't read {name}: {e}"))?;
    let mut reader = picture;
    reader.no_limits();
    let decoded = reader
        .decode()
        .map_err(|e| format!("Can't read {name}: {e}"))?;
    let linear_already = matches!(
        decoded,
        image::DynamicImage::ImageRgb32F(_) | image::DynamicImage::ImageRgba32F(_)
    );
    let mut rgba = decoded.into_rgba32f();
    if !linear_already {
        for p in rgba.pixels_mut() {
            for c in &mut p.0[..3] {
                *c = srgb_to_linear(*c);
            }
        }
    }
    if rgba.width() == 0 || rgba.height() == 0 {
        return Err(format!("{name} has no pixels"));
    }
    if light_probe {
        rgba = unwrap_probe(&rgba);
    }
    Ok(picture_of(rgba))
}

/// A latitude-longitude picture (linear RGBA) ready to draw and light
/// with: capped at `MAX_WIDTH`, with its blurred copy and harmonics.
fn picture_of(mut rgba: image::Rgba32FImage) -> SkyPicture {
    if rgba.width() > MAX_WIDTH {
        let h = ((rgba.height() as u64 * MAX_WIDTH as u64) / rgba.width() as u64).max(1) as u32;
        rgba = image::imageops::resize(&rgba, MAX_WIDTH, h, image::imageops::FilterType::Triangle);
    }
    let small = image::imageops::resize(
        &rgba,
        BLUR_WIDTH,
        BLUR_WIDTH / 2,
        image::imageops::FilterType::Triangle,
    );
    let blurred = image::imageops::blur(&small, 3.0);
    let to_half = |img: &image::Rgba32FImage| -> Vec<u16> {
        img.as_raw()
            .iter()
            .map(|&v| half::f16::from_f32(if v.is_finite() { v } else { 0.0 }).to_bits())
            .collect()
    };
    let sh = irradiance_sh(&small);
    SkyPicture {
        key: 0,
        width: rgba.width(),
        height: rgba.height(),
        texels: to_half(&rgba),
        blurred: to_half(&blurred),
        sh,
    }
}

/// Width of a picture made from an environment's own colours
/// (`from_radiance`): gradients are smooth, so a small one is enough.
pub const GENERATED_WIDTH: u32 = 256;

/// A latitude-longitude picture of `radiance` (picture-frame direction ->
/// linear colour): what a gradient-only environment looks like all round,
/// so the Advanced style's reflections and light see it as they see an
/// environment image (VM-Q05).
pub fn from_radiance(radiance: impl Fn(glam::DVec3) -> [f64; 3]) -> SkyPicture {
    let (w, h) = (GENERATED_WIDTH, GENERATED_WIDTH / 2);
    let img = image::Rgba32FImage::from_fn(w, h, |x, y| {
        let d = image_direction(
            (f64::from(x) + 0.5) / f64::from(w),
            (f64::from(y) + 0.5) / f64::from(h),
        );
        let c = radiance(d);
        image::Rgba([c[0] as f32, c[1] as f32, c[2] as f32, 1.0])
    });
    picture_of(img)
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// A Light Probe picture (`environment_probe_uv`) unwrapped to
/// latitude-longitude: twice as wide as the ball's square is tall (at most
/// `MAX_WIDTH`), half as tall, each pixel read bilinearly from the ball
/// along its direction. A picture that isn't square is read as filling it
/// (the ball spans the whole width and height).
pub fn unwrap_probe(probe: &image::Rgba32FImage) -> image::Rgba32FImage {
    let (pw, ph) = (probe.width(), probe.height());
    let width = (2 * pw.max(ph)).clamp(2, MAX_WIDTH);
    let height = (width / 2).max(1);
    let read = |x: i64, y: i64| -> [f32; 4] {
        let x = x.clamp(0, pw as i64 - 1) as u32;
        let y = y.clamp(0, ph as i64 - 1) as u32;
        probe.get_pixel(x, y).0
    };
    image::Rgba32FImage::from_fn(width, height, |i, j| {
        let u = (i as f64 + 0.5) / width as f64;
        let v = (j as f64 + 0.5) / height as f64;
        let [pu, pv] = operandi_core::environment::environment_probe_uv(
            glam::DMat3::IDENTITY,
            image_direction(u, v),
        );
        // Bilinear, pixel centres at half steps.
        let fx = pu * pw as f64 - 0.5;
        let fy = pv * ph as f64 - 0.5;
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = ((fx - x0) as f32, (fy - y0) as f32);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let [a, b, c, d] = [
            read(x0, y0),
            read(x0 + 1, y0),
            read(x0, y0 + 1),
            read(x0 + 1, y0 + 1),
        ];
        let mut out = [0f32; 4];
        for k in 0..4 {
            let top = a[k] + (b[k] - a[k]) * tx;
            let bottom = c[k] + (d[k] - c[k]) * tx;
            out[k] = top + (bottom - top) * ty;
        }
        image::Rgba(out)
    })
}

/// The direction (picture frame) of latitude-longitude pixel centre
/// (u, v), v from the top: the inverse of `environment_image_uv`.
pub fn image_direction(u: f64, v: f64) -> glam::DVec3 {
    let phi = (0.5 - u) * std::f64::consts::TAU;
    let theta = (0.5 - v) * std::f64::consts::PI;
    glam::DVec3::new(
        theta.cos() * phi.sin(),
        theta.sin(),
        theta.cos() * phi.cos(),
    )
}

/// The 9 real spherical harmonic basis functions at unit `d`.
pub fn sh_basis(d: glam::DVec3) -> [f64; 9] {
    let (x, y, z) = (d.x, d.y, d.z);
    [
        0.282_095,
        0.488_603 * y,
        0.488_603 * z,
        0.488_603 * x,
        1.092_548 * x * y,
        1.092_548 * y * z,
        0.315_392 * (3.0 * z * z - 1.0),
        1.092_548 * x * z,
        0.546_274 * (x * x - y * y),
    ]
}

/// Irradiance harmonics of a latitude-longitude picture (see
/// `SkyPicture::sh`): radiance projected on the basis, each pixel weighted
/// by its solid angle, then each band scaled by the cosine lobe's
/// convolution (π, 2π/3, π/4) and divided by π.
pub fn irradiance_sh(img: &image::Rgba32FImage) -> [[f32; 3]; 9] {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut sum = [[0f64; 3]; 9];
    let d_phi = std::f64::consts::TAU / w as f64;
    let d_theta = std::f64::consts::PI / h as f64;
    for j in 0..h {
        let v = (j as f64 + 0.5) / h as f64;
        let elevation = (0.5 - v) * std::f64::consts::PI;
        let area = elevation.cos() * d_phi * d_theta;
        for i in 0..w {
            let u = (i as f64 + 0.5) / w as f64;
            let basis = sh_basis(image_direction(u, v));
            let p = img.get_pixel(i as u32, j as u32).0;
            for (k, b) in basis.iter().enumerate() {
                for c in 0..3 {
                    let value = if p[c].is_finite() { p[c] as f64 } else { 0.0 };
                    sum[k][c] += value * b * area;
                }
            }
        }
    }
    let band = [
        std::f64::consts::PI,
        2.0 * std::f64::consts::PI / 3.0,
        2.0 * std::f64::consts::PI / 3.0,
        2.0 * std::f64::consts::PI / 3.0,
        std::f64::consts::PI / 4.0,
        std::f64::consts::PI / 4.0,
        std::f64::consts::PI / 4.0,
        std::f64::consts::PI / 4.0,
        std::f64::consts::PI / 4.0,
    ];
    let mut out = [[0f32; 3]; 9];
    for k in 0..9 {
        for c in 0..3 {
            out[k][c] = (sum[k][c] * band[k] / std::f64::consts::PI) as f32;
        }
    }
    out
}

/// Irradiance (÷π) along unit picture-frame normal `n` from `sh`.
#[allow(dead_code)]
pub fn sh_irradiance(sh: &[[f32; 3]; 9], n: glam::DVec3) -> [f64; 3] {
    let basis = sh_basis(n);
    let mut out = [0f64; 3];
    for (k, b) in basis.iter().enumerate() {
        for c in 0..3 {
            out[c] += sh[k][c] as f64 * b;
        }
    }
    out.map(|v| v.max(0.0))
}

const SHADER: &str = r#"
struct Camera { eye: vec4<f32>, right: vec4<f32>, up: vec4<f32>, forward: vec4<f32>, params: vec4<f32>, view: vec4<f32> };
@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var sky: texture_2d<f32>;
@group(1) @binding(1) var sky_sampler: sampler;
// Rows of the world-to-picture rotation (w unused), then (intensity,
// opacity, invert, 0).
struct Options { r0: vec4<f32>, r1: vec4<f32>, r2: vec4<f32>, look: vec4<f32> };
@group(2) @binding(0) var<uniform> options: Options;
struct Out { @builtin(position) position: vec4<f32>, @location(0) ndc: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32) -> Out {
  // One triangle covering the view.
  let p = vec2<f32>(f32((i << 1u) & 2u) * 2.0 - 1.0, f32(i & 2u) * 2.0 - 1.0);
  var out: Out;
  out.position = vec4<f32>(p, 1.0, 1.0);
  out.ndc = p;
  return out;
}
const TAU: f32 = 6.283185307;
const PI: f32 = 3.141592654;
fn to_gamma(c: f32) -> f32 {
  if (c <= 0.0031308) { return c * 12.92; }
  return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}
@fragment fn fs(in: Out) -> @location(0) vec4<f32> {
  // The pixel's world ray (gpu_scene::camera_uniform's projection undone).
  var d = cam.forward.xyz;
  if (cam.params.z > 0.5) {
    d = cam.forward.xyz + cam.right.xyz * (in.ndc.x / cam.params.x) + cam.up.xyz * (in.ndc.y / cam.params.y);
  }
  let world = normalize(d);
  let i = normalize(vec3<f32>(dot(options.r0.xyz, world), dot(options.r1.xyz, world), dot(options.r2.xyz, world)));
  // environment_image_uv: u from +Z toward -X, v from the top row.
  let u = 0.5 - atan2(i.x, i.z) / TAU;
  let v = 0.5 - asin(clamp(i.y, -1.0, 1.0)) / PI;
  var c = textureSampleLevel(sky, sky_sampler, vec2<f32>(u, v), 0.0).rgb;
  if (options.look.z > 0.5) { c = vec3<f32>(1.0) - c; }
  c = clamp(c * options.look.x, vec3<f32>(0.0), vec3<f32>(1.0));
  let g = vec3<f32>(to_gamma(c.r), to_gamma(c.g), to_gamma(c.b));
  let a = clamp(options.look.y, 0.0, 1.0);
  return vec4<f32>(g * a, a);
}
"#;

/// The GPU side: one pipeline, a texture per picture.
pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    camera_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    options_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: HashMap<u64, (wgpu::Texture, wgpu::BindGroup)>,
}

/// Bindings for one view's sky this frame.
pub struct Prepared {
    camera: wgpu::BindGroup,
    texture: u64,
    options: wgpu::BindGroup,
}

/// An Rgba16Float texture of `texels` (`width` × `height`).
pub fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    width: u32,
    height: u32,
    texels: &[u16],
) -> wgpu::Texture {
    let bytes: Vec<u8> = texels.iter().flat_map(|t| t.to_le_bytes()).collect();
    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &bytes,
    )
}

/// A sampler that wraps around in u (longitude) and clamps in v.
pub fn sky_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Environment picture sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}

impl Renderer {
    pub fn new(device: &wgpu::Device, samples: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Operandi environment picture shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let uniform = |visibility| wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Environment picture camera"),
            entries: &[uniform(wgpu::ShaderStages::FRAGMENT)],
        });
        let options_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Environment picture options"),
            entries: &[uniform(wgpu::ShaderStages::FRAGMENT)],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Environment picture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Environment picture pipeline layout"),
            bind_group_layouts: &[&camera_layout, &texture_layout, &options_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Operandi environment picture"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            // Behind everything: no depth test, no depth write.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });
        Self {
            pipeline,
            camera_layout,
            texture_layout,
            options_layout,
            sampler: sky_sampler(device),
            textures: HashMap::new(),
        }
    }

    /// Upload the picture if new and build this frame's bindings.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera: &[f32; 24],
        sky: &SkyDraw,
    ) -> Prepared {
        let p = &sky.picture;
        if !self.textures.contains_key(&p.key) {
            let texture = upload(
                device,
                queue,
                "Environment picture",
                p.width,
                p.height,
                &p.texels,
            );
            let view = texture.create_view(&Default::default());
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Environment picture"),
                layout: &self.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.textures.insert(p.key, (texture, group));
        }
        let bytes =
            |floats: &[f32]| -> Vec<u8> { floats.iter().flat_map(|f| f.to_le_bytes()).collect() };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Environment picture camera"),
            contents: &bytes(camera),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let r = &sky.to_image;
        let options = [
            r[0],
            r[1],
            r[2],
            0.,
            r[3],
            r[4],
            r[5],
            0.,
            r[6],
            r[7],
            r[8],
            0.,
            sky.intensity,
            sky.opacity,
            if sky.invert { 1. } else { 0. },
            0.,
        ];
        let options_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Environment picture options"),
            contents: &bytes(&options),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = |layout, buffer: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Environment picture uniforms"),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            })
        };
        Prepared {
            camera: group(&self.camera_layout, &camera_buffer),
            texture: p.key,
            options: group(&self.options_layout, &options_buffer),
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, prepared: &Prepared) {
        let Some((_, texture)) = self.textures.get(&prepared.texture) else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &prepared.camera, &[]);
        pass.set_bind_group(1, texture, &[]);
        pass.set_bind_group(2, &prepared.options, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Drop textures of pictures no longer drawn.
    pub fn retain(&mut self, keys: &std::collections::BTreeSet<u64>) {
        self.textures.retain(|k, _| keys.contains(k));
    }
}

#[cfg(test)]
mod tests {
    /// VM-Q05: a gradient environment becomes a picture the Advanced
    /// style lights and reflects with: a white sky irradiates 1, a sky lit
    /// only above lights tops more than bottoms, and the cache keeps one
    /// generated picture per key.
    #[test]
    fn generated_pictures_light_like_images() {
        let white = from_radiance(|_| [1.; 3]);
        let up = sh_irradiance(&white.sh, glam::DVec3::Y);
        assert!((up[0] - 1.).abs() < 0.03, "{up:?}");
        let top = from_radiance(|d| if d.y > 0. { [1.; 3] } else { [0.; 3] });
        let (a, b) = (
            sh_irradiance(&top.sh, glam::DVec3::Y)[0],
            sh_irradiance(&top.sh, -glam::DVec3::Y)[0],
        );
        assert!(a > 0.8 && b < 0.2, "{a} {b}");
        let mut cache = SkyCache::default();
        let mut made = 0;
        let first = cache.generated("a", || {
            made += 1;
            from_radiance(|_| [0.5; 3])
        });
        let again = cache.generated("a", || {
            made += 1;
            from_radiance(|_| [0.5; 3])
        });
        assert_eq!((made, first.key), (1, again.key));
        let other = cache.generated("b", || from_radiance(|_| [0.2; 3]));
        assert_ne!(other.key, first.key);
        // Only the newest generated picture is kept.
        assert_eq!(cache.keys().len(), 1);
    }

    use super::*;
    use glam::DVec3;

    /// A Light Probe unwraps to latitude-longitude: the ball's centre is
    /// what the picture's centre (+Z) shows, its top half the top rows,
    /// and its right edge +X (which lat-long has left of centre).
    #[test]
    fn light_probe_unwraps_to_latitude_longitude() {
        let ball = image::Rgba32FImage::from_fn(64, 64, |x, y| {
            let (s, t) = (x as f32 / 32.0 - 1.0, 1.0 - y as f32 / 32.0);
            image::Rgba([(s > 0.0) as u8 as f32, (t > 0.0) as u8 as f32, 0.5, 1.0])
        });
        let flat = unwrap_probe(&ball);
        assert_eq!((flat.width(), flat.height()), (128, 64));
        let at = |d: DVec3| {
            let [u, v] = operandi_core::environment::environment_image_uv(glam::DMat3::IDENTITY, d);
            let x = ((u * 128.0) as u32).min(127);
            let y = ((v * 64.0) as u32).min(63);
            flat.get_pixel(x, y).0
        };
        assert_eq!(at(DVec3::new(0.0, 0.6, 0.8))[1], 1.0);
        assert_eq!(at(DVec3::new(0.0, -0.6, 0.8))[1], 0.0);
        assert_eq!(at(DVec3::new(0.6, 0.0, 0.8))[0], 1.0);
        assert_eq!(at(DVec3::new(-0.6, 0.0, 0.8))[0], 0.0);
    }

    /// `image_direction` undoes `environment_image_uv`.
    #[test]
    fn pixel_directions_round_trip() {
        for (u, v) in [(0.5, 0.5), (0.75, 0.5), (0.1, 0.2), (0.9, 0.8)] {
            let d = image_direction(u, v);
            let uv = operandi_core::environment::environment_image_uv(glam::DMat3::IDENTITY, d);
            assert!(
                (uv[0] - u).abs() < 1e-9 && (uv[1] - v).abs() < 1e-9,
                "{u},{v}: {uv:?}"
            );
        }
    }

    /// A white sky of radiance 1 gives irradiance 1 in every direction; a
    /// sky lit only from above gives more upward than downward.
    #[test]
    fn harmonics_of_white_and_of_a_lit_top() {
        let white = image::Rgba32FImage::from_pixel(64, 32, image::Rgba([1.0, 1.0, 1.0, 1.0]));
        let sh = irradiance_sh(&white);
        for n in [
            DVec3::Y,
            -DVec3::Y,
            DVec3::X,
            DVec3::new(0.3, -0.4, 0.86).normalize(),
        ] {
            let e = sh_irradiance(&sh, n);
            assert!((e[0] - 1.0).abs() < 0.02, "{n}: {e:?}");
        }
        let top = image::Rgba32FImage::from_fn(64, 32, |_, y| {
            let v = if y < 16 { 1.0 } else { 0.0 };
            image::Rgba([v, v, v, 1.0])
        });
        let sh = irradiance_sh(&top);
        let up = sh_irradiance(&sh, DVec3::Y)[0];
        let down = sh_irradiance(&sh, -DVec3::Y)[0];
        let side = sh_irradiance(&sh, DVec3::X)[0];
        assert!(
            up > 0.9 && down < 0.1 && (side - 0.5).abs() < 0.05,
            "{up} {down} {side}"
        );
    }

    /// LDR pictures are linearised and big ones shrunk to MAX_WIDTH.
    #[test]
    fn decode_linearises_and_shrinks() {
        let dir = std::env::temp_dir().join(format!("operandi-sky-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("grey.png");
        image::RgbImage::from_pixel(MAX_WIDTH + 8, 4, image::Rgb([128, 128, 128]))
            .save(&path)
            .unwrap();
        let p = decode(&path).unwrap();
        assert_eq!(p.width, MAX_WIDTH);
        let first = half::f16::from_bits(p.texels[0]).to_f32();
        assert!((first - 0.2158).abs() < 0.01, "{first}");
        assert_eq!(p.blurred.len(), (BLUR_WIDTH * BLUR_WIDTH / 2 * 4) as usize);
        let mut cache = SkyCache::default();
        let path_text = path.to_string_lossy().into_owned();
        let got = cache.wait(&path_text).unwrap();
        assert_eq!(got.width, MAX_WIDTH);
        assert!(cache.wait("/no/such/sky.hdr").is_none());
        assert!(cache.error("/no/such/sky.hdr").is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
