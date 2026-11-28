//! Vertex buffers split into pieces the graphics card accepts.
//!
//! A wgpu buffer can't be bigger than the device's `max_buffer_size`
//! (256 MiB with the default limits). A sphere subdivided five times
//! (283k quads) needs 271 MB of wireframe strokes, and creating that one
//! buffer made wgpu panic, which crashed the app. Every vertex list the
//! viewport draws is a list of whole triangles, so it can go up in several
//! buffers and be drawn with one draw call per buffer.
use egui_wgpu::wgpu::{self, util::DeviceExt};
use std::ops::Range;

/// Largest piece, whatever the device allows: big enough that ordinary
/// meshes stay in one buffer, small enough to allocate easily.
const MAX_CHUNK_BYTES: u64 = 128 << 20;

/// Vertices per piece for `stride`-byte vertices under `max_bytes`: a
/// whole number of triangles (and of six-corner strokes and dots), so no
/// primitive is split across two buffers.
pub fn vertices_per_chunk(stride: u64, max_bytes: u64) -> u32 {
    let per = (max_bytes.min(MAX_CHUNK_BYTES) / stride.max(1)).min(u32::MAX as u64) as u32;
    (per / 6 * 6).max(6)
}

/// The vertex ranges of each piece of `count` vertices, `per` at a time.
pub fn chunk_ranges(count: u32, per: u32) -> Vec<Range<u32>> {
    (0..count.div_ceil(per))
        .map(|k| k * per..((k + 1) * per).min(count))
        .collect()
}

/// One vertex list on the graphics card, in one or more buffers.
pub struct VertexChunks {
    /// Each buffer and the vertices it holds.
    parts: Vec<(wgpu::Buffer, Range<u32>)>,
    stride: u64,
}

impl VertexChunks {
    /// Upload `bytes`, `stride` bytes per vertex. An empty list still gets a
    /// small buffer (wgpu can't make zero-sized ones) and draws nothing.
    pub fn new(
        device: &wgpu::Device,
        label: &str,
        bytes: &[u8],
        stride: u64,
        usage: wgpu::BufferUsages,
    ) -> Self {
        let per = vertices_per_chunk(stride, device.limits().max_buffer_size);
        Self::with_chunk_size(device, label, bytes, stride, usage, per)
    }

    /// As `new`, with `per` vertices a piece (tests use small pieces).
    pub fn with_chunk_size(
        device: &wgpu::Device,
        label: &str,
        bytes: &[u8],
        stride: u64,
        usage: wgpu::BufferUsages,
        per: u32,
    ) -> Self {
        let count = (bytes.len() as u64 / stride) as u32;
        let mut ranges = chunk_ranges(count, per);
        if ranges.is_empty() {
            ranges.push(0..0);
        }
        let parts = ranges
            .into_iter()
            .map(|r| {
                let span = r.start as usize * stride as usize..r.end as usize * stride as usize;
                let contents = &bytes[span];
                let zeros = vec![0; stride.max(4) as usize];
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: if contents.is_empty() {
                        &zeros
                    } else {
                        contents
                    },
                    usage,
                });
                (buffer, r)
            })
            .collect();
        Self { parts, stride }
    }

    /// Vertices held.
    #[cfg(test)]
    pub fn count(&self) -> u32 {
        self.parts.last().map_or(0, |(_, r)| r.end)
    }

    /// Buffers used.
    #[cfg(test)]
    pub fn pieces(&self) -> usize {
        self.parts.len()
    }

    /// Overwrite vertices from `first` on with `bytes` (a drag patch). The
    /// write must lie inside the list.
    pub fn write(&self, queue: &wgpu::Queue, first: u32, bytes: &[u8]) {
        let count = (bytes.len() as u64 / self.stride) as u32;
        let end = first + count;
        for (buffer, r) in &self.parts {
            let (a, b) = (first.max(r.start), end.min(r.end));
            if a >= b {
                continue;
            }
            let src = (a - first) as usize * self.stride as usize
                ..(b - first) as usize * self.stride as usize;
            queue.write_buffer(buffer, (a - r.start) as u64 * self.stride, &bytes[src]);
        }
    }

    /// Staging writes for `count` vertices from `first` on, one per buffer
    /// the stretch touches (`gpu_arena::Store::views`).
    pub fn views(
        &self,
        queue: &wgpu::Queue,
        first: u32,
        count: u32,
        out: &mut Vec<crate::gpu_arena::View>,
    ) -> Option<()> {
        let end = first + count;
        for (buffer, r) in &self.parts {
            let (a, b) = (first.max(r.start), end.min(r.end));
            if a >= b {
                continue;
            }
            let size = wgpu::BufferSize::new(u64::from(b - a) * self.stride)?;
            let view =
                queue.write_buffer_with(buffer, u64::from(a - r.start) * self.stride, size)?;
            out.push((view, a - first..b - first));
        }
        Some(())
    }

    /// Draw vertices `range` (clamped to the list), with the pipeline and
    /// bind groups already set; the list goes in vertex slot 0.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, range: Range<u32>) {
        for (buffer, r) in &self.parts {
            let (a, b) = (range.start.max(r.start), range.end.min(r.end));
            if a >= b {
                continue;
            }
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(a - r.start..b - r.start, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_hold_whole_triangles_under_the_limit() {
        // The crash: 283k quads' strokes, 40 bytes a corner, 256 MiB limit.
        let per = vertices_per_chunk(40, 256 << 20);
        assert_eq!(per % 6, 0);
        assert!(per as u64 * 40 <= 256 << 20);
        let count = 282_624 * 4 * 6;
        let ranges = chunk_ranges(count, per);
        assert!(ranges.len() > 1);
        assert_eq!(ranges.first().unwrap().start, 0);
        assert_eq!(ranges.last().unwrap().end, count);
        assert!(ranges.windows(2).all(|w| w[0].end == w[1].start));
        assert!(ranges.iter().all(|r| (r.end - r.start) <= per));
        // 44-byte fill vertices: still multiples of three.
        assert_eq!(vertices_per_chunk(44, 256 << 20) % 3, 0);
        // A tiny limit never gives empty pieces.
        assert_eq!(vertices_per_chunk(40, 1), 6);
        assert!(chunk_ranges(0, per).is_empty());
    }

    #[test]
    fn pieces_cover_the_list_on_a_real_device() {
        let Some(state) = crate::performance_tests::headless_gpu() else {
            println!("no graphics device");
            return;
        };
        let usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;
        let bytes = vec![7u8; 40 * 6 * 5];
        let one = VertexChunks::new(&state.device, "test", &bytes, 40, usage);
        assert_eq!((one.pieces(), one.count()), (1, 30));
        let split = VertexChunks::with_chunk_size(&state.device, "test", &bytes, 40, usage, 12);
        assert_eq!((split.pieces(), split.count()), (3, 30));
        // A patch across a piece boundary writes both pieces.
        split.write(&state.queue, 6, &bytes[..40 * 12]);
        let empty = VertexChunks::new(&state.device, "test", &[], 40, usage);
        assert_eq!((empty.pieces(), empty.count()), (1, 0));
    }

    /// Jason's crash (2026-11-12): a default Sphere subdivided five times
    /// needed a 271 MB wireframe buffer, over the 256 MiB limit, and wgpu
    /// aborted the app. Draws every pass on a headless graphics device.
    /// Opt-in (about 20 s in debug; the splitting itself is covered by the
    /// two tests above): `cargo test --locked -p operandi-app --
    /// sphere_subdivided_five_times --ignored`.
    #[test]
    #[ignore = "heavy probe: run with --ignored"]
    fn sphere_subdivided_five_times_draws() {
        use crate::*;
        let Some(state) = crate::performance_tests::headless_gpu() else {
            println!("no graphics device");
            return;
        };
        let device = state.device.clone();
        let mut a = Operandi::empty();
        a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
        a.run(Command::NewMesh);
        a.activate(Tool::Sphere);
        let p = a.editor.session.live.as_ref().unwrap().params.clone();
        a.run(Command::Preview(p));
        a.run(Command::Drop);
        a.camera.view = crate::camera::View::Perspective;
        let ctx = eframe::egui::Context::default();
        theme::install(&ctx);
        let frame = |a: &mut Operandi| {
            crate::tests::draw(a, &ctx, eframe::egui::vec2(1440., 900.), vec![]);
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
        };
        frame(&mut a);
        a.run(Command::Mode(Mode::Polygon));
        for _ in 0..5 {
            let options = a
                .editor
                .presets
                .get::<operandi_core::SubdividePreset>()
                .clone();
            a.run(Command::SubdividePolygons { options, times: 1 });
            frame(&mut a);
        }
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 282_624);
    }
}
