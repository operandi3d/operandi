//! The viewport's image layers against the one evaluator of the shader
//! tree: every `Expr` variant, written out as WGSL by
//! `operandi_core::shader_wgsl`, must validate inside the fill shader and,
//! run on a real (headless) GPU, give what `Expr::eval_at` gives.
use crate::gpu_scene::ImageData;
use egui_wgpu::wgpu::{self, util::DeviceExt};
use operandi_core::shader_eval::{
    self as se, Expr, FaceKind, GeometryInput, ImageLookup, Mix, ShadingPoint,
};
use operandi_core::shader_tree::{BlendMode, Filtering};
use operandi_core::shader_wgsl;
use operandi_core::texture_locator::{FalloffType, Tiling};
use std::sync::Arc;

/// Test pictures: a 16 × 16 ramp (red across, green down the rows, alpha
/// across and down) and a 4 × 4 one of distinct colours.
fn images() -> Vec<ImageData> {
    let ramp: Vec<u8> = (0..16u32)
        .flat_map(|y| {
            (0..16u32).flat_map(move |x| {
                [
                    (x * 17) as u8,
                    (y * 17) as u8,
                    128,
                    (40 + (x + y) * 7) as u8,
                ]
            })
        })
        .collect();
    let small: Vec<u8> = (0..16u32)
        .flat_map(|i| {
            [
                (i * 16) as u8,
                (255 - i * 13) as u8,
                ((i * 71) % 256) as u8,
                255,
            ]
        })
        .collect();
    vec![ImageData::new(16, 16, ramp), ImageData::new(4, 4, small)]
}

fn uv_lookup(image: u64, extension: Tiling, vector: Expr) -> Box<ImageLookup> {
    Box::new(ImageLookup {
        image,
        filtering: Filtering::Nearest,
        vector,
        extension,
    })
}
fn color(image: u64, extension: Tiling, vector: Expr) -> Expr {
    Expr::ImageColor(uv_lookup(image, extension, vector))
}
fn alpha(image: u64, extension: Tiling, vector: Expr) -> Expr {
    Expr::ImageAlpha(uv_lookup(image, extension, vector))
}
fn b(e: Expr) -> Box<Expr> {
    Box::new(e)
}
fn locator() -> Expr {
    Expr::Locator(Box::new([
        0.8, 0.1, -0.2, 0.3, //
        -0.1, 0.9, 0.15, -0.2, //
        0.2, -0.3, 1.1, 0.05,
    ]))
}
fn normal() -> Expr {
    Expr::Normal(Box::new([0.9, 0.1, 0., -0.1, 1.0, 0.2, 0.05, 0., 1.1]))
}

/// One expression per variant (and per mode of each), each reading an
/// image so it is drawn on the GPU.
fn cases() -> Vec<(String, Expr)> {
    let uv = Expr::mesh_uv(None, [1.5, 2.5], 30.);
    let img = |ext| color(0, ext, uv.clone());
    let a0 = alpha(0, Tiling::Repeat, uv.clone());
    let mut out: Vec<(String, Expr)> = Vec::new();
    for ext in [Tiling::Repeat, Tiling::Mirror, Tiling::Edge, Tiling::Reset] {
        out.push((format!("image {ext:?}"), img(ext)));
    }
    out.push(("image alpha".into(), a0.clone()));
    out.push((
        "second image, plain uv".into(),
        color(1, Tiling::Repeat, Expr::mesh_uv(None, [1., 1.], 0.)),
    ));
    for axis in 0..3u8 {
        for mirror in 0..4u8 {
            let planar = Expr::Planar {
                of: b(locator()),
                axis,
                centre: 0.5,
                repeat: [1.2, 0.8],
                mirror,
            };
            out.push((
                format!("planar {axis} {mirror}"),
                color(0, Tiling::Mirror, planar),
            ));
        }
        for spherical in [false, true] {
            let wrap = Expr::Wrap {
                of: b(locator()),
                axis,
                spherical,
                centre: 0.5,
                repeat: [2., 1.],
            };
            out.push((
                format!("wrap {axis} {spherical}"),
                color(0, Tiling::Repeat, wrap),
            ));
        }
        // Light Probe (TL-Q31).
        let probe = Expr::Probe {
            of: b(locator()),
            axis,
            repeat: [2., 1.],
        };
        out.push((format!("probe {axis}"), color(0, Tiling::Repeat, probe)));
    }
    // Front (TL-Q30): (u·w, v·w, w) / w through Planar Z.
    {
        let h = locator();
        let uvw = Expr::mix(
            Mix::Divide,
            Expr::scalar(1.),
            h.clone(),
            Expr::channel(h, 2),
        );
        let front = Expr::Planar {
            of: b(uvw),
            axis: 2,
            centre: 0.,
            repeat: [1., 1.],
            mirror: 0,
        };
        out.push(("front".into(), color(0, Tiling::Mirror, front)));
    }
    for t in Tiling::ALL {
        let tiled = Expr::Tile(b(uv.clone()), [*t, Tiling::Mirror]);
        out.push((format!("tile {t:?}"), color(1, Tiling::Edge, tiled)));
    }
    for clip in [[true, false], [false, true], [true, true]] {
        let e = Expr::mix(
            Mix::Blend,
            Expr::Inside(b(uv.clone()), clip),
            Expr::Const([0.1, 0.2, 0.3]),
            img(Tiling::Repeat),
        );
        out.push((format!("inside {clip:?}"), e));
    }
    let weights = [
        (0..6).map(|f| (f, FaceKind::Signed)).collect::<Vec<_>>(),
        (0..3).map(|f| (f, FaceKind::Dominant)).collect(),
        (0..3).map(|f| (f, FaceKind::Power(3.))).collect(),
    ];
    for (face, kind) in weights.into_iter().flatten() {
        let w = Expr::FaceWeight {
            normal: b(normal()),
            face,
            kind,
        };
        out.push((
            format!("face {face} {kind:?}"),
            Expr::mix(
                Mix::Blend,
                w,
                Expr::Const([0.9, 0.1, 0.]),
                img(Tiling::Repeat),
            ),
        ));
    }
    for axis in 0..3 {
        out.push((
            format!("facing {axis}"),
            Expr::mix(
                Mix::Blend,
                Expr::Facing(b(normal()), axis),
                img(Tiling::Repeat),
                Expr::Const([0.2, 0.9, 0.4]),
            ),
        ));
    }
    for kind in FalloffType::ALL {
        let f = Expr::Falloff {
            of: b(locator()),
            kind: *kind,
            amount: [0.4, 0.7, 0.5],
        };
        out.push((
            format!("falloff {kind:?}"),
            Expr::mix(Mix::Blend, f, Expr::Const([0.; 3]), img(Tiling::Repeat)),
        ));
    }
    let fac = Expr::luma(color(1, Tiling::Repeat, Expr::mesh_uv(None, [3., 3.], 0.)));
    for mode in [
        Mix::Blend,
        Mix::Add,
        Mix::Multiply,
        Mix::Subtract,
        Mix::Screen,
        Mix::Divide,
        Mix::Difference,
        Mix::Darken,
        Mix::Lighten,
        Mix::Overlay,
        Mix::Dodge,
        Mix::Burn,
        Mix::SoftLight,
    ] {
        out.push((
            format!("mix {mode:?}"),
            Expr::mix(
                mode,
                fac.clone(),
                img(Tiling::Repeat),
                Expr::Const([0.6, 0.25, 0.9]),
            ),
        ));
    }
    for mode in BlendMode::ALL {
        out.push((
            format!("blend {mode:?}"),
            se::blend(
                *mode,
                Expr::Const([0.3, 0.55, 0.8]),
                img(Tiling::Repeat),
                a0.clone(),
            ),
        ));
    }
    out.push(("luma".into(), Expr::luma(img(Tiling::Repeat))));
    // VM-Q11: every procedural pattern per pixel (pattern.wgsl), and the
    // per-cell randoms of the patterns with cells.
    {
        use operandi_core::procedural::{
            CellularType, DotsType, GridType, NoiseType, Pattern, Procedural, ProceduralKind,
        };
        use operandi_core::shader_eval::PatternLookup;
        let mut patterns: Vec<(String, Pattern)> = ProceduralKind::ALL
            .iter()
            .map(|k| (k.label().to_owned(), Procedural::new(*k).pattern))
            .collect();
        for (name, mut p) in patterns.clone() {
            match &mut p {
                Pattern::Grid { kind, .. } => *kind = GridType::Hexagon,
                Pattern::Dots { kind, .. } => *kind = DotsType::Hexagon,
                Pattern::Cellular { kind, .. } => *kind = CellularType::Round,
                Pattern::Noise(n) => {
                    n.kind = NoiseType::Turbulence;
                    n.smooth_tops = 0.5;
                }
                _ => continue,
            }
            patterns.push((format!("{name} (other type)"), p));
        }
        for (name, pattern) in patterns {
            let lookup = PatternLookup {
                pattern,
                axis: 1,
                vector: Expr::times(locator(), Expr::scalar(1.7)),
            };
            let cells = matches!(
                lookup.pattern,
                Pattern::Checker { .. }
                    | Pattern::Grid { .. }
                    | Pattern::Dots { .. }
                    | Pattern::Cellular { .. }
                    | Pattern::Poisson { .. }
            );
            out.push((
                format!("pattern {name}"),
                Expr::Pattern(Box::new(lookup.clone())),
            ));
            if cells {
                out.push((
                    format!("cell random {name}"),
                    Expr::CellRandom(Box::new(lookup)),
                ));
            }
        }
    }
    for k in 0..3 {
        out.push((
            format!("channel {k}"),
            Expr::channel(img(Tiling::Repeat), k),
        ));
    }
    out.push(("gamma".into(), Expr::gamma(img(Tiling::Repeat), 2.2)));
    out.push((
        "bias gain".into(),
        Expr::BiasGain(b(Expr::luma(img(Tiling::Repeat))), 0.3, 0.7),
    ));
    out.push((
        "hsv vary".into(),
        Expr::HsvVary {
            color: b(img(Tiling::Repeat)),
            random: b(color(1, Tiling::Repeat, uv.clone())),
            amounts: [0.5, 0.3, 0.4],
        },
    ));
    out.push((
        "hsv".into(),
        Expr::Hsv {
            of: b(img(Tiling::Repeat)),
            hue_scale: 1.3,
            hue_shift: 40.,
            saturation_scale: 0.7,
        },
    ));
    out.push((
        "distance".into(),
        Expr::mix(
            Mix::Multiply,
            Expr::scalar(1.),
            img(Tiling::Repeat),
            Expr::DistanceTo([0.2, -0.1, 0.4]),
        ),
    ));
    let stops = Arc::new(vec![
        [0., 0., 0.2, 1.],
        [1., 0.3, 0., 0.5],
        [0.2, 1., 0.4, 0.],
        [0.9, 0.9, 0.9, 1.],
    ]);
    for alpha_out in [false, true] {
        out.push((
            format!("ramp {alpha_out}"),
            Expr::Ramp {
                input: b(Expr::luma(img(Tiling::Repeat))),
                lo: 0.1,
                hi: 0.8,
                stops: stops.clone(),
                alpha: alpha_out,
            },
        ));
    }
    for (name, e) in [
        ("geometry", Expr::Geometry(GeometryInput::Incidence)),
        (
            "attribute",
            Expr::Attribute {
                name: "Color".into(),
                alpha: false,
            },
        ),
        (
            "occlusion",
            Expr::Occlusion {
                rays: 8,
                distance: 1.,
                same_surface: false,
                inside: false,
                toward: Default::default(),
            },
        ),
    ] {
        out.push((
            name.into(),
            Expr::mix(Mix::Add, Expr::scalar(1.), img(Tiling::Repeat), e),
        ));
    }
    // A procedural pattern rides in per corner (the slot).
    let pattern = Expr::Pattern(Box::new(se::PatternLookup {
        pattern: operandi_core::procedural::Procedural::new(
            operandi_core::procedural::ProceduralKind::Checker,
        )
        .pattern,
        axis: 2,
        vector: locator(),
    }));
    out.push((
        "pattern slot".into(),
        Expr::mix(
            Mix::Blend,
            pattern,
            img(Tiling::Repeat),
            Expr::Const([0.; 3]),
        ),
    ));
    out
}

fn translate(e: &Expr) -> shader_wgsl::Program {
    shader_wgsl::translate(e, &mut |id| Some(id as u32)).expect("translates")
}

/// Every case's function, together, inside the real fill shader.
#[test]
fn generated_fill_shaders_validate() {
    let bodies: Vec<String> = cases().iter().map(|(_, e)| translate(e).body).collect();
    let source = crate::gpu::world_shader_for_tests(&shader_wgsl::module(&bodies));
    if let Err(e) = crate::material_gpu::valid(&source) {
        panic!("{e}");
    }
}

/// `eval_at`'s image sampler for the test pictures: the nearest texel at
/// the extended coordinates (v up), linear colour; flags lookups within a
/// hair of a texel edge, where f32 and f64 may pick different texels.
fn cpu_sample(
    images: &[ImageData],
    near_edge: &std::cell::Cell<bool>,
    l: &ImageLookup,
    raw: [f64; 2],
) -> [f64; 4] {
    let Some([u, v]) = se::extended(raw, l.extension) else {
        return [0.; 4];
    };
    let img = &images[l.image as usize];
    let (w, h) = (f64::from(img.width), f64::from(img.height));
    let (x, y) = (u * w, (1. - v) * h);
    // Inside edges only: a clamped lookup on the picture's own border
    // reads the border texel either way.
    let edge =
        |t: f64, size: f64| (t - t.round()).abs() < 2e-3 && t.round() > 0. && t.round() < size;
    if edge(x, w) || edge(y, h) {
        near_edge.set(true);
    }
    let xi = (x.floor() as i64).clamp(0, img.width as i64 - 1) as u32;
    let yi = (y.floor() as i64).clamp(0, img.height as i64 - 1) as u32;
    let i = ((yi * img.width + xi) * 4) as usize;
    let c = |k: usize| {
        f64::from(eframe::egui::ecolor::linear_f32_from_gamma_u8(
            img.rgba[i + k],
        ))
    };
    [c(0), c(1), c(2), f64::from(img.rgba[i + 3]) / 255.]
}

/// Sample points: positions, UVs and unit normals spread pseudo-randomly.
fn points() -> Vec<ShadingPoint> {
    let mut seed = 12345u64;
    let mut next = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 33) as f64) / f64::from(1u32 << 31)
    };
    (0..96)
        .map(|_| {
            let position = [0, 1, 2].map(|_| next() * 3. - 1.5);
            let uv = [next() * 4. - 1.5, next() * 4. - 1.5];
            let n = [0, 1, 2].map(|_| next() * 2. - 1.);
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-3);
            ShadingPoint {
                position,
                uv,
                normal: n.map(|c| c / len),
            }
        })
        .collect()
}

/// Run each case's generated function on the GPU at the sample points
/// (a compute pass, zero pixel footprint: mip 0) and compare with
/// `eval_at`. Skips without a graphics device.
#[test]
fn generated_shaders_match_eval_at_on_the_gpu() {
    let Some(state) = crate::performance_tests::headless_gpu() else {
        println!("no graphics device; skipped");
        return;
    };
    let (device, queue) = (&state.device, &state.queue);
    let images = images();
    let textures: Vec<wgpu::TextureView> = images
        .iter()
        .map(|i| crate::material_gpu::upload(device, queue, i))
        .collect();
    let samplers: Vec<wgpu::Sampler> = crate::material_gpu::SAMPLERS
        .iter()
        .map(|name| {
            let address = match &name[2..] {
                "r" => wgpu::AddressMode::Repeat,
                "m" => wgpu::AddressMode::MirrorRepeat,
                _ => wgpu::AddressMode::ClampToEdge,
            };
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: address,
                address_mode_v: address,
                ..Default::default()
            })
        })
        .collect();
    let pts = points();
    let mut failures = Vec::new();
    let mut lists_checked = 0;
    for (name, e) in cases() {
        let program = translate(&e);
        // The same value as the third item of a list (no pattern slot:
        // lists can't take one), its parameters after the program's.
        let list = shader_wgsl::translate_list(
            &[
                (Expr::scalar(0.25), 0),
                (Expr::Const([0.1, 0.2, 0.3]), 1),
                (e.clone(), 0),
            ],
            &mut |id| Some(id as u32),
        );
        // Inputs: p, uv, n, slot per point (vec4 each).
        let near_edge = std::cell::Cell::new(false);
        let mut skip = vec![false; pts.len()];
        let mut want = Vec::new();
        let mut input: Vec<f32> = Vec::new();
        for (k, at) in pts.iter().enumerate() {
            near_edge.set(false);
            let slot = program
                .slot
                .as_ref()
                .map_or([0.; 3], |s| s.eval_at(at, &|_, _| [0.; 4]));
            let cpu = e.eval_at(at, &|l, raw| cpu_sample(&images, &near_edge, l, raw));
            skip[k] = near_edge.get();
            want.push(cpu);
            for v in [
                [at.position[0], at.position[1], at.position[2], 0.],
                [at.uv[0], at.uv[1], 0., 0.],
                [at.normal[0], at.normal[1], at.normal[2], 0.],
                [slot[0], slot[1], slot[2], 0.],
            ] {
                input.extend(v.map(|c| c as f32));
            }
        }
        let source = format!(
            "{bindings}{module}
struct Pt {{ p: vec4<f32>, uv: vec4<f32>, n: vec4<f32>, slot: vec4<f32> }};
@group(0) @binding(40) var<storage, read> pts: array<Pt>;
@group(0) @binding(41) var<storage, read_write> outs: array<vec4<f32>>;
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id: vec3<u32>) {{
  let q = pts[id.x];
  // eval_at has no view: incidence 0 (facing).
  let s = OpSp(q.p.xyz, q.uv.xy, q.n.xyz, q.slot.xyz, 0.0);
  outs[id.x * 2u] = vec4<f32>(op_material(1u, s, s, s, 0u), 0.0);
  let l = op_list({list_code}u, s, s, s, {list_base}u);
  outs[id.x * 2u + 1u] = vec4<f32>(l[2], l[0].x);
}}",
            bindings = crate::material_gpu::bindings().replace("@group(2)", "@group(0)"),
            module = shader_wgsl::module_with_lists(
                std::slice::from_ref(&program.body),
                list.as_ref()
                    .map(|l| std::slice::from_ref(&l.body))
                    .unwrap_or(&[]),
            ),
            list_code = u32::from(list.is_some()),
            list_base = program.params.len(),
        );
        crate::material_gpu::valid(&source).unwrap_or_else(|e| panic!("{name}: {e}"));
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&name),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        // Every material binding, as the fill shader's scene group has
        // them, plus the points and results.
        let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let mut layout_entries: Vec<wgpu::BindGroupLayoutEntry> =
            crate::material_gpu::layout_entries(false)
                .into_iter()
                .map(|e| wgpu::BindGroupLayoutEntry {
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ..e
                })
                .collect();
        layout_entries.extend([storage(40, true), storage(41, false)]);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &layout_entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let buffer = |data: &[f32], usage| {
            let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &bytes,
                usage,
            })
        };
        let mut params: Vec<f32> = program
            .params
            .iter()
            .chain(list.iter().flat_map(|l| &l.params))
            .flatten()
            .copied()
            .collect();
        params.resize(params.len().max(4), 0.);
        let params = buffer(&params, wgpu::BufferUsages::STORAGE);
        let instances = buffer(&[0.; 4], wgpu::BufferUsages::STORAGE);
        let pts_buffer = buffer(&input, wgpu::BufferUsages::STORAGE);
        let size = (pts.len() * 32) as u64;
        let outs = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 40,
                resource: pts_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 41,
                resource: outs.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: crate::material_gpu::PARAMS,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: crate::material_gpu::INSTANCES,
                resource: instances.as_entire_binding(),
            },
        ];
        for (k, s) in samplers.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: crate::material_gpu::FIRST_SAMPLER + k as u32,
                resource: wgpu::BindingResource::Sampler(s),
            });
        }
        for k in 0..shader_wgsl::MAX_IMAGES {
            entries.push(wgpu::BindGroupEntry {
                binding: crate::material_gpu::FIRST_IMAGE + k as u32,
                resource: wgpu::BindingResource::TextureView(&textures[k.min(textures.len() - 1)]),
            });
        }
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &entries,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(pts.len() as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&outs, 0, &read, 0, size);
        queue.submit([encoder.finish()]);
        read.slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.unwrap());
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let got: Vec<f32> = read
            .slice(..)
            .get_mapped_range()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        let mut checked = 0;
        for (k, cpu) in want.iter().enumerate() {
            if skip[k] {
                continue;
            }
            checked += 1;
            let close = |gpu: &[f32]| {
                (0..3).all(|c| {
                    let d = (f64::from(gpu[c]) - cpu[c]).abs();
                    d <= 2e-3 + 2e-3 * cpu[c].abs()
                })
            };
            let gpu = &got[k * 8..k * 8 + 3];
            if !close(gpu) {
                failures.push(format!("{name} at {:?}: gpu {gpu:?}, cpu {cpu:?}", pts[k]));
                break;
            }
            if list.is_some() {
                let listed = &got[k * 8 + 4..k * 8 + 8];
                if !close(listed) || (listed[3] - 0.25).abs() > 1e-6 {
                    failures.push(format!(
                        "{name} list at {:?}: gpu {listed:?}, cpu {cpu:?}",
                        pts[k]
                    ));
                    break;
                }
                lists_checked += 1;
            }
        }
        // A mismatch stops the case early; that is reported below with
        // the rest, not as "too few points".
        let failed = failures
            .last()
            .is_some_and(|f| f.starts_with(&format!("{name} ")));
        assert!(
            failed || checked > pts.len() / 2,
            "{name}: too few points checked"
        );
    }
    assert!(
        failures.is_empty(),
        "GPU ({:?}) ≠ eval_at:\n{}",
        state.adapter.get_info(),
        failures.join("\n")
    );
    assert!(lists_checked > 0, "no list was checked");
}

/// The fixture's folder, relative to the repository root (the scene stores
/// its image paths relative to it; `fixture_scene` makes them absolute).
const FIXTURE_DIR: &str = "fixtures/viewport-image-textures";

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Our own test pictures, drawn here: (file name, width, height, RGBA).
fn fixture_pictures() -> Vec<(&'static str, u32, u32, Vec<u8>)> {
    let draw = |w: u32, h: u32, f: &dyn Fn(u32, u32) -> [u8; 4]| -> Vec<u8> {
        (0..h)
            .flat_map(|y| (0..w).flat_map(move |x| f(x, y)))
            .collect()
    };
    // Quadrants (top-left red, top-right green, bottom-left blue,
    // bottom-right yellow) with a white grid every 32 px and a dark
    // arrow pointing up (+V): orientation reads at a glance.
    let uv_grid = draw(256, 256, &|x, y| {
        let arrow = x.abs_diff(128) < 6 && (40..200).contains(&y)
            || (40..80).contains(&y) && x.abs_diff(128) < (y - 40);
        if arrow {
            return [20, 20, 20, 255];
        }
        if x % 32 == 0 || y % 32 == 0 {
            return [255, 255, 255, 255];
        }
        match (x < 128, y < 128) {
            (true, true) => [220, 60, 50, 255],
            (false, true) => [60, 190, 80, 255],
            (true, false) => [60, 90, 220, 255],
            (false, false) => [235, 205, 60, 255],
        }
    });
    let stripes = draw(128, 128, &|x, y| {
        if (x + y) / 16 % 2 == 0 {
            [240, 240, 235, 255]
        } else {
            [40, 120, 160, 255]
        }
    });
    // A target badge: rings, opaque.
    let badge = draw(128, 128, &|x, y| {
        let d = ((x as f64 - 63.5).hypot(y as f64 - 63.5) / 8.) as u32;
        if d.is_multiple_of(2) {
            [250, 250, 250, 255]
        } else {
            [200, 30, 40, 255]
        }
    });
    // A soft green disc on transparent: alpha 1 in the middle to 0 at the
    // rim.
    let disc = draw(128, 128, &|x, y| {
        let r = (x as f64 - 63.5).hypot(y as f64 - 63.5) / 60.;
        let a = ((1. - r) * 2.).clamp(0., 1.);
        [40, 200, 70, (a * 255.).round() as u8]
    });
    vec![
        ("uv-grid.png", 256, 256, uv_grid),
        ("planar-stripes.png", 128, 128, stripes),
        ("reset-badge.png", 128, 128, badge),
        ("alpha-disc.png", 128, 128, disc),
    ]
}

/// The fixture scene: four items in a row, each with its own masked
/// group: a UV-mapped image (with the polygon's own UVs), a Planar
/// locator image on a box, a Reset-clipped image over an orange colour
/// layer, and an image with alpha over a blue colour layer. Image paths
/// are relative to the repository root.
fn build_fixture_scene() -> operandi_core::Scene {
    use glam::DVec3;
    use operandi_core::Command;
    use operandi_core::Mesh;
    use operandi_core::shader_tree::{
        GroupMask, ItemMask, NewLayer, NodeKind, Projection, ShaderTreeEdit, TagMask,
        TextureEffect, TextureSource,
    };
    let mut e = operandi_core::Editor::default();
    let plane = || {
        let mut m = Mesh::box_mesh(DVec3::ZERO, DVec3::new(2.5, 2.5, 0.), 2).unwrap_or_else(|_| {
            Mesh::box_mesh(DVec3::ZERO, DVec3::new(2.5, 2.5, 1e-3), 2).unwrap()
        });
        // UVs from the points: (x, y) over the plane's 2.5 m, 0..1.
        for f in 0..m.faces.len() {
            let uv = m.faces[f]
                .vertices
                .iter()
                .map(|&v| {
                    let p = m.vertices[v as usize];
                    glam::DVec2::new(p.x / 2.5 + 0.5, p.y / 2.5 + 0.5)
                })
                .collect();
            m.faces[f].uv = Some(uv);
        }
        m
    };
    let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0).unwrap();
    let meshes = [plane(), cube, plane(), plane()];
    let mut ids = Vec::new();
    for (k, mesh) in meshes.into_iter().enumerate() {
        e.scene.new_mesh().unwrap();
        let item = e.scene.active_mut().unwrap();
        item.mesh = mesh.into();
        item.name = [
            "UV Image",
            "Planar Image",
            "Reset Over Colour",
            "Alpha Image",
        ][k]
            .into();
        item.transform.translation = DVec3::new(k as f64 * 3. - 4.5, 0., 0.);
        ids.push(item.id);
    }
    let base = e.scene.shader_tree.material_for_tag("Default").unwrap();
    let add = |e: &mut operandi_core::Editor, layer, selected| {
        let id = e.scene.shader_tree.next_id;
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
            layer,
            selected: Some(selected),
        }))
        .unwrap();
        id
    };
    let edit = |e: &mut operandi_core::Editor, id, f: &dyn Fn(&mut NodeKind)| {
        let mut node = e.scene.shader_tree.get(id).unwrap().clone();
        f(&mut node.kind);
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node,
            continuing: false,
        }))
        .unwrap();
    };
    let pictures = fixture_pictures();
    for (k, &item) in ids.iter().enumerate() {
        let group = add(&mut e, NewLayer::Group, base);
        edit(&mut e, group, &|kind| {
            *kind = NodeKind::Group(GroupMask {
                item: ItemMask::Item(item),
                tag: TagMask::All,
                ..Default::default()
            })
        });
        // Cases 3 and 4: a colour layer under the image.
        if k >= 2 {
            let colour = add(&mut e, NewLayer::Constant, group);
            let rgb = if k == 2 {
                [1., 0.45, 0.05]
            } else {
                [0.1, 0.25, 0.9]
            };
            edit(&mut e, colour, &|kind| {
                if let NodeKind::Texture(t) = kind {
                    t.effect = TextureEffect::DiffuseColor;
                    t.source = TextureSource::Constant {
                        color: rgb,
                        value: 1.,
                    };
                }
            });
        }
        let layer = add(&mut e, NewLayer::ImageMap, group);
        let (name, w, h, _) = &pictures[k];
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetImage {
            node: layer,
            image: Some((format!("{FIXTURE_DIR}/{name}"), *w, *h)),
        }))
        .unwrap();
        edit(&mut e, layer, &|kind| {
            if let NodeKind::Texture(t) = kind {
                t.effect = TextureEffect::DiffuseColor;
            }
        });
        let locator = match &e.scene.shader_tree.get(layer).unwrap().kind {
            NodeKind::Texture(t) => t.locator.unwrap(),
            _ => unreachable!(),
        };
        let located = e.scene.item(locator).unwrap();
        let mut settings = located.texture_locator().unwrap().clone();
        // The locator follows its item (World Coordinates off): centred on it.
        let transform = located.transform.clone();
        match k {
            0 | 3 => settings.projection = Projection::UvMap,
            1 => {
                settings.projection = Projection::Planar;
                settings.axis = 2;
                settings.repeat = [2., 2.];
            }
            _ => {
                settings.projection = Projection::Planar;
                settings.axis = 2;
                settings.tiling = [operandi_core::texture_locator::Tiling::Reset; 2];
            }
        }
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetLocator {
            locator,
            settings: Box::new(settings),
            transform,
            continuing: false,
        }))
        .unwrap();
    }
    e.scene
}

/// `scene` with its image paths made absolute under `root`.
fn rooted(mut scene: operandi_core::Scene, root: &std::path::Path) -> operandi_core::Scene {
    for image in &mut scene.images {
        if std::path::Path::new(&image.path).is_relative() {
            image.path = root.join(&image.path).to_string_lossy().into_owned();
        }
    }
    scene
}

/// The committed fixture loads, every item draws through a material
/// program, each case's image sits where it should, and the shader
/// validates.
#[test]
fn image_layer_fixture_draws_per_pixel() {
    let root = repo_root();
    let path = root.join(FIXTURE_DIR).join("image-layers.opscene");
    let scene = rooted(operandi_core::persistence::load(&path).unwrap(), &root);
    assert_eq!(scene.images.len(), 4);
    let mut resident = crate::gpu_scene::Resident::default();
    resident.sync_materials(&scene, false);
    for item in scene.items.iter().filter(|i| i.is_mesh()) {
        let matrix = scene.world_matrix(item.id).unwrap();
        let g = resident.item(item, matrix, None, None, &scene.smoothing).0;
        assert!(
            !g.fill.is_empty() && g.fill.iter().all(|v| v.program != 0),
            "{} isn't drawn per pixel",
            item.name
        );
    }
    let m = resident.gpu_materials().unwrap();
    assert_eq!(m.images.len(), 4);
    assert!(
        m.images.iter().all(|i| i.average.is_some()),
        "a picture didn't load"
    );
    let source = crate::gpu::world_shader_for_tests(&m.source);
    crate::material_gpu::valid(&source).unwrap();
    // The fixture is what `build_fixture_scene` makes (regenerate with
    // OPERANDI_WRITE_FIXTURE=1 cargo test … write_image_layer_fixture).
    let built = build_fixture_scene();
    assert_eq!(scene.images.len(), built.images.len());
}

/// Regenerates the committed fixture (OPERANDI_WRITE_FIXTURE=1) and writes
/// a copy with absolute image paths for GUI QA to
/// development/viewport-image-textures/image-layers.opscene.
#[test]
#[ignore = "writes the fixture and the QA scene"]
fn write_image_layer_fixture() {
    let root = repo_root();
    let dir = root.join(FIXTURE_DIR);
    let scene = build_fixture_scene();
    if std::env::var_os("OPERANDI_WRITE_FIXTURE").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        for (name, w, h, rgba) in fixture_pictures() {
            image::save_buffer(dir.join(name), &rgba, w, h, image::ExtendedColorType::Rgba8)
                .unwrap();
        }
        operandi_core::persistence::save(&dir.join("image-layers.opscene"), &scene).unwrap();
    }
    let qa = root.join("development/viewport-image-textures");
    std::fs::create_dir_all(&qa).unwrap();
    operandi_core::persistence::save(&qa.join("image-layers.opscene"), &rooted(scene, &root))
        .unwrap();
}

/// UIC-160..164: Reload Image is a typeable command, and the scene's
/// images can be listed.
#[test]
fn image_commands_reload_and_list() {
    let root = repo_root();
    let path = root.join(FIXTURE_DIR).join("image-layers.opscene");
    let mut a = crate::Operandi::empty();
    a.editor.scene = rooted(operandi_core::persistence::load(&path).unwrap(), &root);
    a.resident_cache.sync_materials(&a.editor.scene, false);
    assert_eq!(a.resident_cache.image_reads(), 4);
    a.execute("image.list");
    assert!(a.status.contains("uv-grid"), "{}", a.status);
    a.execute("image.reload uv-grid");
    assert!(a.status.starts_with("Reloaded"), "{}", a.status);
    a.resident_cache.sync_materials(&a.editor.scene, false);
    assert_eq!(a.resident_cache.image_reads(), 5);
    a.execute("image.reload all");
    a.resident_cache.sync_materials(&a.editor.scene, false);
    assert_eq!(a.resident_cache.image_reads(), 9);
    a.execute("image.reload nothing-here");
    assert!(a.status.starts_with("No scene image"));
    a.execute("help image.reload");
    assert!(a.status.starts_with("Help: clip.reload"), "{}", a.status);
}

/// The Advanced style's principled surface (operandi_core::
/// blender_principled): the WGSL run on a headless GPU gives what the CPU
/// `shade` gives, over metals, coats, sheen, emission, levels and angles.
#[test]
fn principled_wgsl_matches_the_cpu() {
    use operandi_core::blender_principled::{self as bp, Lights, Params};
    let Some(state) = crate::performance_tests::headless_gpu() else {
        println!("no graphics device; skipped");
        return;
    };
    let (device, queue) = (&state.device, &state.queue);
    let surfaces = [
        Params::default(),
        Params {
            metallic: 1.,
            roughness: 0.2,
            tint: [0.9, 0.6, 0.3],
            ..Params::default()
        },
        Params {
            coat: 1.,
            coat_roughness: 0.05,
            roughness: 0.7,
            ..Params::default()
        },
        Params {
            sheen: 0.8,
            emission: [0.5, 0.2, 0.],
            level: 0.9,
            ior: 1.33,
            ..Params::default()
        },
        Params {
            level: 0.,
            metallic: 0.4,
            roughness: 0.,
            ..Params::default()
        },
    ];
    let unit = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        v.map(|c| c / l)
    };
    let mut cases = Vec::new();
    for (k, s) in surfaces.iter().enumerate() {
        for (n, v, l) in [
            ([0., 0., 1.], [0., 0., 1.], [0., 0.7, 0.7]),
            ([0., 0.38, 0.92], [0., 0., 1.], [0., 0.7, 0.7]),
            ([0.6, 0., 0.8], [0., 0., 1.], [-0.3, 0.3, 0.9]),
            ([0., -0.9, 0.43], [0., 0., 1.], [0., 0.7, 0.7]),
        ] {
            cases.push((k, *s, [0.6, 0.3, 0.1], unit(n), unit(v), unit(l)));
        }
    }
    let lights = Lights {
        ambient: [0.17, 0.15, 0.2],
        key: 1.07,
    };
    let mut params: Vec<f32> = bp::lut().iter().flatten().copied().collect();
    for s in &surfaces {
        params.extend(s.rows().iter().flatten());
    }
    let mut input: Vec<f32> = Vec::new();
    for (k, _, base, n, v, l) in &cases {
        let row = (bp::LUT_ROWS + k * bp::SURFACE_ROWS) as f32;
        input.extend([base[0], base[1], base[2], row]);
        input.extend([n[0], n[1], n[2], 0.8]);
        input.extend([v[0], v[1], v[2], 0.]);
        input.extend([l[0], l[1], l[2], 0.]);
    }
    let source = format!(
        "@group(0) @binding(0) var<storage, read> op_params: array<vec4<f32>>;
{principled}
@group(0) @binding(1) var<storage, read> ins: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> outs: array<vec4<f32>>;
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id: vec3<u32>) {{
  let b = ins[id.x * 4u]; let n = ins[id.x * 4u + 1u];
  let v = ins[id.x * 4u + 2u]; let l = ins[id.x * 4u + 3u];
  let s = op_surface(u32(b.w));
  outs[id.x * 3u] = vec4<f32>(op_principled(b.xyz, s, n.xyz, v.xyz, l.xyz, n.w, vec3<f32>({a0}, {a1}, {a2}), {key}), 0.0);
  let dx = vec3<f32>(0.01, 0.002, 0.0);
  let dy = vec3<f32>(0.001, -0.01, 0.003);
  outs[id.x * 3u + 1u] = vec4<f32>(op_bump(n.xyz, dx, dy, 0.0, 0.002 * b.x, -0.001), 0.0);
  outs[id.x * 3u + 2u] = vec4<f32>(op_normal_map(n.xyz, dx, dy, vec2<f32>(0.01, 0.002), vec2<f32>(-0.001, 0.01), b.xyz), 0.0);
}}",
        principled = bp::wgsl(),
        a0 = lights.ambient[0],
        a1 = lights.ambient[1],
        a2 = lights.ambient[2],
        key = lights.key,
    );
    crate::material_gpu::valid(&source).unwrap();
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("principled"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let buffer = |data: &[f32]| {
        let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &bytes,
            usage: wgpu::BufferUsages::STORAGE,
        })
    };
    let (params, ins) = (buffer(&params), buffer(&input));
    let size = (cases.len() * 48) as u64;
    let outs = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: ins.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: outs.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(cases.len() as u32, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&outs, 0, &read, 0, size);
    queue.submit([encoder.finish()]);
    read.slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.unwrap());
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let got: Vec<f32> = read
        .slice(..)
        .get_mapped_range()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect();
    for (i, (k, s, base, n, v, l)) in cases.iter().enumerate() {
        let cpu = bp::shade(*base, s, *n, *v, *l, 0.8, &lights);
        let gpu = &got[i * 12..i * 12 + 3];
        for c in 0..3 {
            assert!(
                (gpu[c] - cpu[c]).abs() <= 1e-3 + 1e-2 * cpu[c].abs(),
                "surface {k} case {i}: gpu {gpu:?} cpu {cpu:?}"
            );
        }
        let (dx, dy) = ([0.01, 0.002, 0.], [0.001, -0.01, 0.003]);
        let bumped = bp::bump(*n, dx, dy, 0., 0.002 * base[0], -0.001);
        let mapped = bp::normal_map(*n, dx, dy, [0.01, 0.002], [-0.001, 0.01], *base);
        for (what, cpu, at) in [("bump", bumped, 4), ("normal map", mapped, 8)] {
            let gpu = &got[i * 12 + at..i * 12 + at + 3];
            assert!(
                (0..3).all(|c| (gpu[c] - cpu[c]).abs() < 1e-4),
                "{what} case {i}: gpu {gpu:?} cpu {cpu:?}"
            );
        }
    }
}
