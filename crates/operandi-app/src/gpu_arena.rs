//! Small items' geometry packed into shared buffers (perf task 2,
//! 2026-11-13), so a scene of thousands of separate objects draws with a
//! few draw calls instead of one per item.
//!
//! Each item still owns its block of vertices (a `Slot`): it is uploaded,
//! patched by drags and freed on its own, exactly as when every item had
//! buffers of its own. What changes is where the block lives: items smaller
//! than a quarter of a shared buffer go into the next free stretch of one,
//! so items built one after another sit side by side, and the draw code
//! (`Runs`) joins neighbouring blocks drawn the same way into one call.
//! Bigger items keep buffers of their own (`Store::Own`); they cost one call
//! each anyway.
use crate::gpu_chunks::VertexChunks;
use egui_wgpu::wgpu;
use std::ops::Range;

/// Bytes per shared buffer.
const CHUNK_BYTES: u64 = 64 << 20;

/// Where an item's block is: shared buffer `chunk`, vertices
/// `first..first + count`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub chunk: u32,
    pub first: u32,
    pub count: u32,
}

/// Shared vertex buffers of one layout, `stride` bytes a vertex.
pub struct Arena {
    label: &'static str,
    stride: u64,
    chunks: Vec<(wgpu::Buffer, u32)>,
    /// Free stretches: (chunk, first, count).
    free: Vec<Slot>,
    /// How far each chunk has been handed out.
    used: Vec<u32>,
}

impl Arena {
    pub fn new(label: &'static str, stride: u64) -> Self {
        Self {
            label,
            stride,
            chunks: Vec::new(),
            free: Vec::new(),
            used: Vec::new(),
        }
    }
    /// Vertices a shared buffer holds: whole triangles and strokes.
    fn capacity(&self, device: &wgpu::Device) -> u32 {
        crate::gpu_chunks::vertices_per_chunk(
            self.stride,
            CHUNK_BYTES.min(device.limits().max_buffer_size),
        )
    }
    /// A block of `count` vertices, or `None` when the item is too big to
    /// share (a quarter of a buffer or more).
    fn alloc(&mut self, device: &wgpu::Device, count: u32) -> Option<Slot> {
        let capacity = self.capacity(device);
        if count == 0 || count >= capacity / 4 {
            return None;
        }
        // The same size freed earlier first (a rebuilt item), then any
        // stretch big enough, then the end of the last buffer.
        let fit = self
            .free
            .iter()
            .position(|s| s.count == count)
            .or_else(|| self.free.iter().position(|s| s.count > count));
        if let Some(i) = fit {
            let s = self.free[i];
            if s.count == count {
                self.free.swap_remove(i);
            } else {
                self.free[i] = Slot {
                    chunk: s.chunk,
                    first: s.first + count,
                    count: s.count - count,
                };
            }
            return Some(Slot { count, ..s });
        }
        if self.used.last().is_none_or(|&u| capacity - u < count) {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: u64::from(capacity) * self.stride,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.chunks.push((buffer, capacity));
            self.used.push(0);
        }
        let chunk = self.chunks.len() - 1;
        let first = self.used[chunk];
        self.used[chunk] += count;
        Some(Slot {
            chunk: chunk as u32,
            first,
            count,
        })
    }
    fn release(&mut self, slot: Slot) {
        if slot.count > 0 {
            self.free.push(slot);
        }
    }
    fn write(&self, queue: &wgpu::Queue, slot: Slot, first: u32, bytes: &[u8]) {
        let count = (bytes.len() as u64 / self.stride) as u32;
        debug_assert!(
            first + count <= slot.count,
            "a write outside the item's block"
        );
        if let Some((buffer, _)) = self.chunks.get(slot.chunk as usize) {
            queue.write_buffer(buffer, u64::from(slot.first + first) * self.stride, bytes);
        }
    }
}

/// An item's vertices: a block in shared buffers, or buffers of its own.
pub enum Store {
    Shared(Slot),
    Own(VertexChunks),
}

impl Store {
    /// Upload `bytes` (`arena.stride` bytes a vertex).
    pub fn upload(
        arena: &mut Arena,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        bytes: &[u8],
    ) -> Self {
        let count = (bytes.len() as u64 / arena.stride) as u32;
        match arena.alloc(device, count) {
            Some(slot) => {
                arena.write(queue, slot, 0, bytes);
                Store::Shared(slot)
            }
            None => Store::Own(VertexChunks::new(
                device,
                label,
                bytes,
                arena.stride,
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            )),
        }
    }
    /// Overwrite vertices from `first` on (a drag patch).
    pub fn write(&self, arena: &Arena, queue: &wgpu::Queue, first: u32, bytes: &[u8]) {
        match self {
            Store::Shared(slot) => arena.write(queue, *slot, first, bytes),
            Store::Own(chunks) => chunks.write(queue, first, bytes),
        }
    }
    /// Give a shared block back (the item was rebuilt or left the view).
    pub fn release(self, arena: &mut Arena) {
        if let Store::Shared(slot) = self {
            arena.release(slot);
        }
    }
}

/// What one draw call draws from.
#[derive(Clone, Copy)]
enum Source<'a> {
    /// Vertices of shared buffer `.0`.
    Shared(u32, &'a wgpu::Buffer),
    Own(&'a VertexChunks),
}

/// One draw call: vertices `range` of `source`, with look bind group
/// `look` (group 1) and placement `placement` (group 3).
struct Run<'a> {
    look: Option<&'a wgpu::BindGroup>,
    placement: &'a wgpu::BindGroup,
    source: Source<'a>,
    range: Range<u32>,
}

/// The draw calls of one pass, in drawing order; neighbouring blocks of the
/// same shared buffer drawn with the same bind groups become one call.
#[derive(Default)]
pub struct Runs<'a> {
    runs: Vec<Run<'a>>,
}

impl<'a> Runs<'a> {
    /// Draw vertices `range` of `store`, `range` counted from its start.
    pub fn push(
        &mut self,
        arena: &'a Arena,
        store: &'a Store,
        range: Range<u32>,
        look: Option<&'a wgpu::BindGroup>,
        placement: &'a wgpu::BindGroup,
    ) {
        if range.is_empty() {
            return;
        }
        let (source, range) = match store {
            Store::Shared(slot) => {
                let Some((buffer, _)) = arena.chunks.get(slot.chunk as usize) else {
                    return;
                };
                (
                    Source::Shared(slot.chunk, buffer),
                    slot.first + range.start..slot.first + range.end.min(slot.count),
                )
            }
            Store::Own(chunks) => (Source::Own(chunks), range),
        };
        if let Some(last) = self.runs.last_mut()
            && let (Source::Shared(a, _), Source::Shared(b, _)) = (last.source, source)
            && a == b
            && last.range.end == range.start
            && same(last.look, look)
            && std::ptr::eq(last.placement, placement)
        {
            last.range.end = range.end;
            return;
        }
        self.runs.push(Run {
            look,
            placement,
            source,
            range,
        });
    }
    /// How many draw calls `draw` makes (tests).
    #[cfg(test)]
    pub fn calls(&self) -> usize {
        self.runs.len()
    }
    /// Make the draw calls, with the pipeline and the other groups set.
    pub fn draw(self, pass: &mut wgpu::RenderPass<'_>, look_slot: u32, placement_slot: u32) {
        let mut bound: Option<u32> = None;
        let mut look_bound: Option<&wgpu::BindGroup> = None;
        let mut placement_bound: Option<&wgpu::BindGroup> = None;
        for run in self.runs {
            if let Some(look) = run.look
                && !look_bound.is_some_and(|b| std::ptr::eq(b, look))
            {
                pass.set_bind_group(look_slot, look, &[]);
                look_bound = Some(look);
            }
            if !placement_bound.is_some_and(|b| std::ptr::eq(b, run.placement)) {
                pass.set_bind_group(placement_slot, run.placement, &[]);
                placement_bound = Some(run.placement);
            }
            match run.source {
                Source::Shared(chunk, buffer) => {
                    if bound != Some(chunk) {
                        pass.set_vertex_buffer(0, buffer.slice(..));
                        bound = Some(chunk);
                    }
                    pass.draw(run.range, 0..1);
                }
                Source::Own(chunks) => {
                    chunks.draw(pass, run.range);
                    bound = None;
                }
            }
        }
    }
}

fn same(a: Option<&wgpu::BindGroup>, b: Option<&wgpu::BindGroup>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_are_reused_and_neighbours_draw_together() {
        let Some(state) = crate::performance_tests::headless_gpu() else {
            println!("no graphics device");
            return;
        };
        let (device, queue) = (&state.device, &state.queue);
        let mut arena = Arena::new("test", 44);
        let bytes = |n: usize| vec![1u8; n * 44];
        let a = Store::upload(&mut arena, device, queue, "a", &bytes(30));
        let b = Store::upload(&mut arena, device, queue, "b", &bytes(60));
        let (Store::Shared(sa), Store::Shared(sb)) = (&a, &b) else {
            panic!("small items share a buffer");
        };
        assert_eq!((sa.chunk, sa.first, sb.first), (0, 0, 30));
        // A rebuilt item of the same size takes its old block back.
        let sa = *sa;
        a.release(&mut arena);
        let again = Store::upload(&mut arena, device, queue, "a", &bytes(30));
        assert!(matches!(again, Store::Shared(s) if s == sa));
        // A huge item gets buffers of its own.
        let capacity = arena.capacity(device) as usize;
        let big = Store::upload(&mut arena, device, queue, "big", &bytes(capacity / 2));
        assert!(matches!(big, Store::Own(_)));
        // Neighbours with the same groups: one call; a different placement
        // or an own buffer: separate calls.
        let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[],
        });
        let group = |_: u8| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &group_layout,
                entries: &[],
            })
        };
        let (p, q) = (group(0), group(1));
        let mut runs = Runs::default();
        runs.push(&arena, &again, 0..30, None, &p);
        runs.push(&arena, &b, 0..60, None, &p);
        assert_eq!(runs.calls(), 1);
        runs.push(&arena, &big, 0..6, None, &p);
        runs.push(&arena, &again, 0..30, None, &q);
        assert_eq!(runs.calls(), 3);
    }
}
