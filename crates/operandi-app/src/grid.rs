//! VAC-G01..05. Analytic, two-sided plane grids have no finite world-origin
//! patch. All intersections use the same camera rays and major planes as input.
//! Density/fade are Operandi rendering choices, not measured reference defaults.
use crate::{Camera, View, theme::viewport as palette};
use eframe::egui::Rect;
use egui_wgpu::wgpu::{self, util::DeviceExt};
use glam::DVec3;

fn focus_distance(camera: &Camera, rect: Rect, plane: u8, center: DVec3) -> f64 {
    if camera.view != View::Perspective {
        return camera.distance;
    }
    camera
        .plane_point(rect.center(), rect, plane, center)
        .map(|p| {
            (p - camera.basis().0)
                .dot(camera.basis().3)
                .max(camera.distance)
        })
        .unwrap_or(camera.distance)
        .clamp(0.0001, 1e12)
}
thread_local! {
    /// Fixed grid square size (Preferences > Grid and Work Plane), if any.
    static FIXED: std::cell::Cell<Option<f64>> = const { std::cell::Cell::new(None) };
}
/// Use a fixed grid square size, or `None` for dynamic spacing.
pub fn set_fixed(size: Option<f64>) {
    FIXED.with(|f| f.set(size.filter(|s| s.is_finite() && *s > 0.)));
}
/// Plane and Grid Increments: which spacings the dynamic grid steps
/// through as you zoom. Provisional (GRD-Q1): the reference's choices
/// aren't listed; these are Operandi's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Increments {
    /// 1, 2, 5, 10…: the grid so far.
    #[default]
    OneTwoFive,
    /// 1, 5, 10, 50…
    OneFive,
    /// Powers of ten only.
    Tens,
    /// Powers of two (…0.5, 1, 2, 4…).
    Twos,
}
impl Increments {
    pub const ALL: [Self; 4] = [Self::OneTwoFive, Self::OneFive, Self::Tens, Self::Twos];
    pub fn label(self) -> &'static str {
        match self {
            Self::OneTwoFive => "1, 2, 5",
            Self::OneFive => "1, 5",
            Self::Tens => "Powers of 10",
            Self::Twos => "Powers of 2",
        }
    }
    /// The smallest step in this series at least `desired`.
    pub fn step(self, desired: f64) -> f64 {
        if self == Self::Twos {
            return 2_f64.powf(desired.log2().ceil());
        }
        let decade = 10_f64.powf(desired.log10().floor());
        let fraction = desired / decade;
        let steps: &[f64] = match self {
            Self::OneTwoFive => &[1., 2., 5., 10.],
            Self::OneFive => &[1., 5., 10.],
            _ => &[1., 10.],
        };
        decade
            * steps
                .iter()
                .copied()
                .find(|&s| fraction <= s)
                .unwrap_or(10.)
    }
}
thread_local! {
    static INCREMENTS: std::cell::Cell<Increments> = const { std::cell::Cell::new(Increments::OneTwoFive) };
}
pub fn set_increments(increments: Increments) {
    INCREMENTS.with(|c| c.set(increments));
}
thread_local! {
    /// Coordinate Rounding's Fixed Increment: (size, forced). Fixed keeps
    /// the dynamic grid from getting finer than `size`; Forced Fixed holds
    /// it at `size` at every zoom.
    static FLOOR: std::cell::Cell<Option<(f64, bool)>> = const { std::cell::Cell::new(None) };
}
pub fn set_floor(floor: Option<(f64, bool)>) {
    FLOOR.with(|c| c.set(floor.filter(|(s, _)| s.is_finite() && *s > 0.)));
}
pub fn spacing(camera: &Camera, rect: Rect, plane: u8, center: DVec3) -> f64 {
    let desired = focus_distance(camera, rect, plane, center) * 28.
        / (rect.height().max(1.) as f64 * camera.lens);
    // A fixed grid keeps its size until its lines would crowd (closer than
    // the dynamic grid's spacing), then spaces dynamically.
    if let Some(size) = FIXED.with(std::cell::Cell::get)
        && size >= desired
        && !matches!(FLOOR.with(std::cell::Cell::get), Some((_, true)))
    {
        return size;
    }
    let step = INCREMENTS.with(std::cell::Cell::get).step(desired);
    match FLOOR.with(std::cell::Cell::get) {
        Some((size, true)) => size,
        Some((size, false)) => step.max(size),
        None => step,
    }
}
/// The major plane an axis-aligned view (Top, Front, Side…) looks straight
/// at, or `None` for Perspective and free Orthographic.
pub fn view_zero_plane(camera: &Camera) -> Option<u8> {
    if matches!(camera.view, View::Perspective | View::Orthographic) {
        return None;
    }
    let f = camera.basis().3.abs();
    Some(if f.y >= f.x && f.y >= f.z {
        0
    } else if f.x >= f.z {
        1
    } else {
        2
    })
}
pub struct Display {
    pub plane: u8,
    pub center: DVec3,
    pub axes: bool,
    pub workplane: bool,
    pub visible: bool,
    pub opacity: f32,
    /// Draw only the plane's two zero lines, solid and dark, across the
    /// whole viewport (the Top/Front/Side views' centre lines).
    pub zero_axes: bool,
    /// Draw Faded Grid: lines fade out with distance from the eye.
    pub fade: bool,
}
pub struct PlaneGrid {
    pub data: Vec<f32>,
}
impl PlaneGrid {
    pub fn new(camera: &Camera, rect: Rect, display: Display, scale: f32) -> Self {
        let Display {
            plane,
            center,
            axes,
            workplane,
            visible,
            opacity,
            zero_axes,
            fade,
        } = display;
        let (eye, right, up, forward) = camera.basis();
        let (u, v) = match plane {
            1 => (DVec3::Y, DVec3::Z),
            2 => (DVec3::X, DVec3::Y),
            _ => (DVec3::X, DVec3::Z),
        };
        let step = spacing(camera, rect, plane, center);
        // Recenter in double precision on a major lattice intersection before
        // uploading floats. Distant mesh positions do not lose fine divisions.
        let target = camera
            .plane_point(rect.center(), rect, plane, center)
            .unwrap_or(camera.target)
            - center;
        let origin = center
            + u * (target.dot(u) / (step * 5.)).floor() * step * 5.
            + v * (target.dot(v) / (step * 5.)).floor() * step * 5.;
        let mut data = Vec::new();
        let mut push =
            |v: DVec3, w: f64| data.extend([v.x as f32, v.y as f32, v.z as f32, w as f32]);
        push(eye - origin, step);
        push(right, rect.width() as f64 / rect.height().max(1.) as f64);
        push(up, focus_distance(camera, rect, plane, center));
        push(
            forward,
            if camera.view == View::Perspective {
                1.
            } else {
                0.
            },
        );
        push(u, (rect.width() * scale) as f64);
        push(v, (rect.height() * scale) as f64);
        let c = if workplane {
            palette::work_plane()
        } else {
            palette::grid()
        };
        push(
            DVec3::new(c.r() as f64, c.g() as f64, c.b() as f64) / 255.,
            if axes || zero_axes { 1. } else { 0. },
        );
        for a in [u, v] {
            let index = if a.x != 0. {
                0
            } else if a.y != 0. {
                1
            } else {
                2
            };
            let c = if zero_axes {
                palette::zero_axes()
            } else {
                palette::grid_axes()[index]
            };
            push(
                DVec3::new(c.r() as f64, c.g() as f64, c.b() as f64) / 255.,
                origin.dot(if a == u { v } else { u }) - center.dot(if a == u { v } else { u }),
            );
        }
        data.extend([
            if visible { 1. } else { 0. },
            scale,
            if opacity.is_finite() {
                opacity.clamp(0., 1.)
            } else {
                0.
            },
            if zero_axes { 1. } else { 0. },
        ]);
        // The view's lens: the grid's rays must match the mesh's projection,
        // which follows `camera.lens` (Maximise/Restore and Flatness of
        // Perspective change it; Camera view sets it).
        // view.z: the perspective near clip, so the grid's depth matches.
        data.extend([
            camera.lens as f32,
            if fade { 1. } else { 0. },
            camera.near() as f32,
            0.,
        ]);
        Self { data }
    }
}
pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    /// Depth-tested against what is drawn (original-compatible A5).
    tested: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}
impl Renderer {
    pub fn new(device: &wgpu::Device, samples: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Operandi analytic workplane grid"),
            source: wgpu::ShaderSource::Wgsl(include_str!("grid.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Grid snapshot"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Grid pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let make = |entry: &str, compare| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Infinite major-plane grid"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: false,
                    depth_compare: compare,
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
        let pipeline = make("fs", wgpu::CompareFunction::Always);
        // Original-compatible A5: only the grid in front of what is drawn.
        let tested = make("fs_depth", wgpu::CompareFunction::Less);
        Self {
            pipeline,
            tested,
            layout,
        }
    }
    pub fn prepare(&self, device: &wgpu::Device, grid: &PlaneGrid) -> wgpu::BindGroup {
        let bytes: Vec<_> = grid.data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid immutable camera/plane snapshot"),
            contents: &bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grid bindings"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, bindings: &[wgpu::BindGroup]) {
        self.draw_with(pass, bindings, false);
    }
    /// `tested`: only where the grid is nearer than the depth already
    /// drawn (a Perspective backdrop over meshes, original-compatible A5).
    pub fn draw_with(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        bindings: &[wgpu::BindGroup],
        tested: bool,
    ) {
        pass.set_pipeline(if tested { &self.tested } else { &self.pipeline });
        for binding in bindings {
            pass.set_bind_group(0, binding, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
