//! Drawing Backdrop pictures on the graphics card (backdrop.rs in the core;
//! specifications/backdrop-2026-11-04). Each picture is one textured
//! rectangle, drawn after the grid and before every mesh with no depth test
//! and no depth write, so it covers the background and grid and every mesh
//! covers it, even one behind its plane (BD-10). Perspective works the same
//! way: a deliberate Operandi choice (Jason, 2026-11-04), where the reference
//! draws the picture over meshes (BD-11a). Colour options run in the
//! shader in the spec's order (BD-20); Transparency mixes with what is
//! behind (BD-22); Blend picks smooth or nearest-pixel sampling (BD-23).
use egui_wgpu::wgpu::{self, util::DeviceExt};
use glam::DVec3;
use std::collections::HashMap;
use std::sync::Arc;

/// A decoded picture ready to upload. `key` is unique per decode, so a
/// texture is uploaded once and reused while the picture stays loaded.
pub struct Pixels {
    pub key: u64,
    pub width: u32,
    pub height: u32,
    /// RGBA, 8 bits per channel, rows top to bottom.
    pub rgba: Vec<u8>,
}

/// One Backdrop to draw in a view.
pub struct Draw {
    /// World corners: image top-left, top-right, bottom-right, bottom-left.
    pub corners: [DVec3; 4],
    pub pixels: Arc<Pixels>,
    pub flip: bool,
    pub invert: bool,
    pub blend: bool,
    pub contrast: f32,
    pub brightness: f32,
    pub transparency: f32,
}

/// The Backdrops one view draws this frame, with that view's camera
/// (`gpu_scene::camera_uniform`).
#[derive(Default)]
pub struct Frame {
    pub camera: [f32; 24],
    pub items: Vec<Draw>,
    /// Original-compatible A5 (Perspective only): the pictures cover
    /// meshes, and only the grid in front of a picture draws over it.
    pub over_meshes: bool,
}

struct Texture {
    nearest: wgpu::BindGroup,
    smooth: wgpu::BindGroup,
}

/// Bindings for one draw, made in `prepare` and used in the render pass.
pub struct Prepared {
    pub over_meshes: bool,
    camera: wgpu::BindGroup,
    draws: Vec<(u64, bool, wgpu::Buffer, wgpu::BindGroup)>,
}

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    /// A5: the picture with its depth written, before the grid.
    with_depth: wgpu::RenderPipeline,
    /// A5: depth 0 over the picture (no colour), so meshes can't cover it.
    claim: wgpu::RenderPipeline,
    camera_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    options_layout: wgpu::BindGroupLayout,
    nearest: wgpu::Sampler,
    smooth: wgpu::Sampler,
    textures: HashMap<u64, Texture>,
}

const SHADER: &str = r#"
struct Camera { eye: vec4<f32>, right: vec4<f32>, up: vec4<f32>, forward: vec4<f32>, params: vec4<f32>, view: vec4<f32> };
@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var picture: texture_2d<f32>;
@group(1) @binding(1) var picture_sampler: sampler;
// (invert, contrast, brightness, transparency)
struct Options { adjust: vec4<f32> };
@group(2) @binding(0) var<uniform> options: Options;
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
fn project(p: vec3<f32>) -> vec4<f32> {
  let d = p - cam.eye.xyz;
  let z = dot(d, cam.forward.xyz);
  let x = dot(d, cam.right.xyz) * cam.params.x;
  let y = dot(d, cam.up.xyz) * cam.params.y;
  if (cam.params.z > 0.5) { return vec4<f32>(x, y, z - cam.params.w, z); }
  return vec4<f32>(x, y, clamp((z + cam.eye.w) / cam.params.w, 0.0, 1.0), 1.0);
}
@vertex fn vs(@location(0) p: vec3<f32>, @location(1) uv: vec2<f32>) -> Out {
  var out: Out; out.position = project(p); out.uv = uv; return out;
}
@fragment fn fs(in: Out) -> @location(0) vec4<f32> {
  let texel = textureSample(picture, picture_sampler, in.uv);
  var v = texel.rgb;
  if (options.adjust.x > 0.5) { v = vec3<f32>(1.0) - v; }
  v = clamp(v * (1.0 + options.adjust.y) + vec3<f32>(options.adjust.z), vec3<f32>(0.0), vec3<f32>(1.0));
  // Premultiplied: the picture's own alpha (BD-Q16, provisional) and
  // Transparency both let what is behind show through.
  let a = texel.a * (1.0 - options.adjust.w);
  return vec4<f32>(v * a, a);
}
// Original-compatible A5: where the picture shows, the nearest depth, so
// meshes drawn after it can't cover it. Clear parts of it don't claim.
struct Claim { @builtin(frag_depth) depth: f32 };
@fragment fn fs_claim(in: Out) -> Claim {
  let a = textureSample(picture, picture_sampler, in.uv).a * (1.0 - options.adjust.w);
  if (a <= 0.0) { discard; }
  var out: Claim; out.depth = 0.0; return out;
}
"#;

impl Renderer {
    pub fn new(device: &wgpu::Device, samples: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Operandi backdrop shader"),
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
            label: Some("Backdrop camera"),
            entries: &[uniform(wgpu::ShaderStages::VERTEX)],
        });
        let options_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Backdrop options"),
            entries: &[uniform(wgpu::ShaderStages::FRAGMENT)],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Backdrop picture"),
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
            label: Some("Backdrop pipeline layout"),
            bind_group_layouts: &[&camera_layout, &texture_layout, &options_layout],
            push_constant_ranges: &[],
        });
        let make = |entry: &str, write_depth: bool, colour: wgpu::ColorWrites| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Operandi backdrop pictures"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 20,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: colour,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: write_depth,
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
            })
        };
        let pipeline = make("fs", false, wgpu::ColorWrites::ALL);
        let with_depth = make("fs", true, wgpu::ColorWrites::ALL);
        let claim = make("fs_claim", true, wgpu::ColorWrites::empty());
        let sampler = |filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Backdrop sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            })
        };
        Self {
            pipeline,
            with_depth,
            claim,
            camera_layout,
            texture_layout,
            options_layout,
            nearest: sampler(wgpu::FilterMode::Nearest),
            smooth: sampler(wgpu::FilterMode::Linear),
            textures: HashMap::new(),
        }
    }

    /// Upload new pictures, forget ones no view drew, and build this
    /// frame's buffers. `None` when there is nothing to draw.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &Frame,
    ) -> Option<Prepared> {
        if frame.items.is_empty() {
            return None;
        }
        for draw in &frame.items {
            let p = &draw.pixels;
            if self.textures.contains_key(&p.key) {
                continue;
            }
            let texture = device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("Backdrop picture"),
                    size: wgpu::Extent3d {
                        width: p.width,
                        height: p.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    // Not sRGB: the shader works on the stored 8-bit values,
                    // as the colour maths was measured (BD-20).
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &p.rgba,
            );
            let view = texture.create_view(&Default::default());
            let group = |sampler: &wgpu::Sampler| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Backdrop picture"),
                    layout: &self.texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                })
            };
            self.textures.insert(
                p.key,
                Texture {
                    nearest: group(&self.nearest),
                    smooth: group(&self.smooth),
                },
            );
        }
        let camera_bytes: Vec<u8> = frame.camera.iter().flat_map(|f| f.to_le_bytes()).collect();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Backdrop camera"),
            contents: &camera_bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let camera = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Backdrop camera"),
            layout: &self.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let draws = frame
            .items
            .iter()
            .map(|d| {
                let vertices = quad(d);
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Backdrop rectangle"),
                    contents: &vertices,
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let options: Vec<u8> = [
                    if d.invert { 1. } else { 0. },
                    d.contrast,
                    d.brightness,
                    d.transparency,
                ]
                .iter()
                .flat_map(|f: &f32| f.to_le_bytes())
                .collect();
                let options = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Backdrop options"),
                    contents: &options,
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let options = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Backdrop options"),
                    layout: &self.options_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: options.as_entire_binding(),
                    }],
                });
                (d.pixels.key, d.blend, buffer, options)
            })
            .collect();
        Some(Prepared {
            over_meshes: frame.over_meshes,
            camera,
            draws,
        })
    }

    /// Drop textures for pictures no longer loaded.
    pub fn retain(&mut self, keys: &std::collections::BTreeSet<u64>) {
        self.textures.retain(|k, _| keys.contains(k));
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, prepared: &Prepared) {
        self.draw_with(pass, prepared, &self.pipeline);
    }
    /// A5: the pictures with their depth written (before the grid).
    pub fn draw_with_depth(&self, pass: &mut wgpu::RenderPass<'_>, prepared: &Prepared) {
        self.draw_with(pass, prepared, &self.with_depth);
    }
    /// A5: claim the pictures' pixels at the nearest depth (after the grid).
    pub fn claim(&self, pass: &mut wgpu::RenderPass<'_>, prepared: &Prepared) {
        self.draw_with(pass, prepared, &self.claim);
    }
    fn draw_with(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        prepared: &Prepared,
        pipeline: &wgpu::RenderPipeline,
    ) {
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &prepared.camera, &[]);
        for (key, blend, buffer, options) in &prepared.draws {
            let Some(texture) = self.textures.get(key) else {
                continue;
            };
            pass.set_bind_group(
                1,
                if *blend {
                    &texture.smooth
                } else {
                    &texture.nearest
                },
                &[],
            );
            pass.set_bind_group(2, options, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..6, 0..1);
        }
    }
}

/// Two triangles with texture coordinates; Flip mirrors the picture left
/// to right inside the same rectangle (BD-21).
fn quad(d: &Draw) -> Vec<u8> {
    let [tl, tr, br, bl] = d.corners;
    let (l, r) = if d.flip { (1., 0.) } else { (0., 1.) };
    let corners = [
        (tl, [l, 0.]),
        (tr, [r, 0.]),
        (br, [r, 1.]),
        (tl, [l, 0.]),
        (br, [r, 1.]),
        (bl, [l, 1.]),
    ];
    let mut bytes = Vec::with_capacity(6 * 20);
    for (p, uv) in corners {
        for f in [p.x as f32, p.y as f32, p.z as f32, uv[0], uv[1]] {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_swaps_the_horizontal_texture_coordinate_only() {
        let draw = |flip| Draw {
            corners: [DVec3::ZERO, DVec3::X, DVec3::X - DVec3::Y, -DVec3::Y],
            pixels: Arc::new(Pixels {
                key: 1,
                width: 1,
                height: 1,
                rgba: vec![0; 4],
            }),
            flip,
            invert: false,
            blend: false,
            contrast: 0.,
            brightness: 0.,
            transparency: 0.,
        };
        let uv = |bytes: &[u8], k: usize| {
            let f = |o| f32::from_le_bytes(bytes[k * 20 + o..k * 20 + o + 4].try_into().unwrap());
            (f(12), f(16))
        };
        let (plain, flipped) = (quad(&draw(false)), quad(&draw(true)));
        assert_eq!(plain.len(), flipped.len());
        for k in 0..6 {
            let (a, b) = (uv(&plain, k), uv(&flipped, k));
            assert_eq!(a.0, 1. - b.0);
            assert_eq!(a.1, b.1);
        }
        assert_eq!(
            uv(&plain, 0),
            (0., 0.),
            "image top-left at the first corner"
        );
    }
}

#[cfg(test)]
mod shader_tests {
    use egui_wgpu::wgpu::naga;

    fn validate(source: &str) -> naga::Module {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}"));
        module
    }

    /// The backdrop and grid shaders, with the A5 depth entries, validate,
    /// so a typo can't leave the viewport blank.
    #[test]
    fn backdrop_and_grid_shaders_validate() {
        for (source, entries) in [
            (super::SHADER, &["fs", "fs_claim"][..]),
            (include_str!("grid.wgsl"), &["fs", "fs_depth"][..]),
        ] {
            let module = validate(source);
            for entry in entries {
                assert!(
                    module.entry_points.iter().any(|e| e.name == *entry),
                    "{entry}"
                );
            }
        }
    }
}
