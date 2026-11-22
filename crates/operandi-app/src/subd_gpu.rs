//! Subdivision polygons drawn as patches on the graphics card
//! (docs/SUBD_GPU_DESIGN.md). The CPU keeps the patches' controls
//! (`subdivision::patches::PatchNet`); a vertex shader samples each patch
//! on its L × L grid every frame and hands the resident fill shader
//! (`fs_fill`) what a stored corner would give it, and the wire shader
//! strokes the cage edges along the patches' sides. About 12 bytes per
//! drawn triangle instead of the stored path's ~1 KB.
use crate::gpu_scene::PatchDraw;
use egui_wgpu::wgpu::{self, util::DeviceExt};
use operandi_core::subdivision::patches::PatchNet;
use std::sync::Arc;

/// The patch shaders, appended to the plain resident shader (gpu.rs
/// `WORLD_SHADER`): its `FillOut`, `Out`, `placed`, `project`, `edge_out`,
/// `highlight_*` and `twice` are reused.
// The patch shader reads `PATCH_LOOK` u32s per cage polygon.
const _: () = assert!(crate::gpu_scene::PATCH_LOOK == 6);
pub const PATCH_SHADER: &str = r#"
// subd_gpu.rs PatchInfo: the placement the patches are drawn at (item
// space → the space the resident geometry is built in) and its normal
// matrix; params = (level L, patches per instance, patches, first centre
// control); wire = the plain wire colour; more = (cage edges, flip, -,
// cage edges per instance).
struct PatchInfo { model: mat4x4<f32>, normal: mat4x4<f32>, params: vec4<u32>, wire: vec4<f32>, more: vec4<u32> };
@group(3) @binding(1) var<storage, read> p_controls: array<f32>;
@group(3) @binding(2) var<storage, read> p_ids: array<u32>;
@group(3) @binding(3) var<storage, read> p_owner: array<u32>;
@group(3) @binding(4) var<storage, read> p_looks: array<u32>;
@group(3) @binding(5) var<storage, read> p_wire: array<u32>;
@group(3) @binding(6) var<uniform> p_info: PatchInfo;
// The clusters left after culling (cull_fill / cull_wire), one per instance.
@group(3) @binding(7) var<storage, read> p_fill_shown: array<u32>;
@group(3) @binding(8) var<storage, read> p_wire_shown: array<u32>;
// Per wire record, the surface's normal on each side of its edge
// (subdivision::patches::PatchNet::wire_sides).
@group(3) @binding(9) var<storage, read> p_sides: array<u32>;
// Material programs: each patch's corner UV, and each cage polygon's
// patches (start, end) to find its neighbours and the polygon's middle.
@group(3) @binding(10) var<storage, read> p_uvs: array<vec2<f32>>;
@group(3) @binding(11) var<storage, read> p_blocks: array<u32>;
fn p_ctrl(i: u32) -> vec3<f32> {
  let k = 3u * i;
  return vec3<f32>(p_controls[k], p_controls[k + 1u], p_controls[k + 2u]);
}
fn p_bern(t: f32) -> vec3<f32> { let s = 1.0 - t; return vec3<f32>(s * s, 2.0 * t * s, t * t); }
fn p_dbern(t: f32) -> vec3<f32> { return vec3<f32>(-2.0 * (1.0 - t), 2.0 - 4.0 * t, 2.0 * t); }
fn p_unpack(c: u32) -> vec4<f32> { return unpack4x8unorm(c); }
// Sample (i, j) of a patch: vertex = patch-in-instance × (L+1)² + i (L+1) + j.
@vertex fn vs_patch_fill(@builtin(vertex_index) vi: u32, @builtin(instance_index) inst: u32) -> FillOut {
  var out: FillOut;
  let l = p_info.params.x;
  let side = l + 1u;
  let per = side * side;
  let p = p_fill_shown[inst] * p_info.params.y + vi / per;
  let culled = vec4<f32>(0.0, 0.0, 2.0, 1.0);
  if (p >= p_info.params.z) { out.position = culled; return out; }
  let look = p_owner[p] * 6u;
  let flags = p_looks[look + 2u];
  if ((flags & 1u) != 0u) { out.position = culled; return out; }
  let g = vi % per;
  let u = f32(g / side) / f32(l);
  let v = f32(g % side) / f32(l);
  // Controls c[a][b]: a along side 0 (corner 0 → 1), b toward corner 3.
  let b = p * 8u;
  let c00 = p_ctrl(p_ids[b]);
  let c20 = p_ctrl(p_ids[b + 1u]);
  let c22 = p_ctrl(p_ids[b + 2u]);
  let c02 = p_ctrl(p_ids[b + 3u]);
  let c10 = p_ctrl(p_ids[b + 4u]);
  let c21 = p_ctrl(p_ids[b + 5u]);
  let c12 = p_ctrl(p_ids[b + 6u]);
  let c01 = p_ctrl(p_ids[b + 7u]);
  let c11 = p_ctrl(p_info.params.w + p);
  let bu = p_bern(u); let bv = p_bern(v);
  let r0 = c00 * bv.x + c01 * bv.y + c02 * bv.z;
  let r1 = c10 * bv.x + c11 * bv.y + c12 * bv.z;
  let r2 = c20 * bv.x + c21 * bv.y + c22 * bv.z;
  let local = r0 * bu.x + r1 * bu.y + r2 * bu.z;
  var n = vec3<f32>(0.0);
  if ((flags & 2u) == 0u) {
    let du = p_dbern(u); let dv = p_dbern(v);
    let tu = r0 * du.x + r1 * du.y + r2 * du.z;
    let s0 = c00 * bu.x + c10 * bu.y + c20 * bu.z;
    let s1 = c01 * bu.x + c11 * bu.y + c21 * bu.z;
    let s2 = c02 * bu.x + c12 * bu.y + c22 * bu.z;
    let tv = s0 * dv.x + s1 * dv.y + s2 * dv.z;
    n = (p_info.normal * vec4<f32>(cross(tu, tv), 0.0)).xyz;
    if (length(n) > 0.0) { n = normalize(n); }
  }
  let w = placed((p_info.model * vec4<f32>(local, 1.0)).xyz);
  out.position = project(w);
  out.world = w;
  out.normal = (place.n * vec4<f32>(n, 0.0)).xyz;
  out.color = p_unpack(p_looks[look]);
  out.map = p_unpack(p_looks[look + 1u]);
  out.local = local;
  out.uv = vec2<f32>(0.0);
  out.program = p_looks[look + 3u];
  // HT-5/HT-6 tints (gpu_scene.rs `Tint`): flat per cage polygon here.
  out.tint_w = p_unpack(p_looks[look + 4u]);
  out.tint_d = p_unpack(p_looks[look + 5u]);
  if (out.program != 0u) {
    // SDV-15: bilinear over the corner's UV, its sides' middles and the
    // polygon's middle.
    let f = p_owner[p];
    let s = p_blocks[2u * f];
    let n = p_blocks[2u * f + 1u] - s;
    let k = p - s;
    let here = p_uvs[p];
    let next = p_uvs[s + (k + 1u) % n];
    let prev = p_uvs[s + (k + n - 1u) % n];
    var middle = vec2<f32>(0.0);
    for (var i = 0u; i < n; i++) { middle += p_uvs[s + i]; }
    middle = middle / f32(n);
    let q1 = (here + next) * 0.5;
    let q3 = (prev + here) * 0.5;
    out.uv = here * (1.0 - u) * (1.0 - v) + q1 * u * (1.0 - v) + middle * u * v + q3 * (1.0 - u) * v;
  }
  if (place.sel.x > 0.5) { out.color = place.sel_fill; out.map = vec4<f32>(out.map.rgb, 0.0); out.program = 0u; }
  return out;
}
fn p_curve(a: vec3<f32>, m: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
  let k = p_bern(t);
  return a * k.x + m * k.y + b * k.z;
}
// A packed side normal into the space the resident geometry is built in
// (zero stays zero: no polygon on that side).
fn p_side(packed: u32) -> vec4<f32> {
  let s = unpack4x8snorm(packed);
  var n = (p_info.normal * vec4<f32>(s.xyz, 0.0)).xyz;
  if (length(n) > 0.0) { n = normalize(n); }
  return vec4<f32>(n, s.w);
}
fn p_slope(a: vec3<f32>, m: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
  return (m - a) * (1.0 - t) + (b - m) * t;
}
// A cage edge's wire as one strip of 2L segments: point q of 2L+1 (two
// stroke corners each, one per side), the low half for q ≤ L. The stroke
// widens across the curve's direction on screen, as `edge_out` widens a
// straight stroke; it sits 0.2% nearer than its surface, the same way.
@vertex fn vs_patch_edge(@builtin(vertex_index) vi: u32, @builtin(instance_index) inst: u32) -> Out {
  var out: Out;
  let l = p_info.params.x;
  let per = 2u * (2u * l + 1u);
  let curve = p_wire_shown[inst] * p_info.more.w + vi / per;
  let culled = vec4<f32>(0.0, 0.0, 2.0, 1.0);
  if (curve >= p_info.more.x) { out.position = culled; return out; }
  let g = vi % per;
  let q = g / 2u;
  let side = select(-1.0, 1.0, (g & 1u) == 1u);
  let w = curve * 6u;
  let lo = p_ctrl(p_wire[w]);
  let h0 = p_ctrl(p_wire[w + 1u]);
  let mid = p_ctrl(p_wire[w + 2u]);
  let h1 = p_ctrl(p_wire[w + 3u]);
  let hi = p_ctrl(p_wire[w + 4u]);
  var p: vec3<f32>;
  var d: vec3<f32>;
  if (q < l) {
    let t = f32(q) / f32(l);
    p = p_curve(lo, h0, mid, t); d = p_slope(lo, h0, mid, t);
  } else if (q == l) {
    p = mid; d = (mid - h0) + (h1 - mid);
  } else {
    let t = f32(q - l) / f32(l);
    p = p_curve(mid, h1, hi, t); d = p_slope(mid, h1, hi, t);
  }
  let k = highlight_k(vec3<f32>(0.0, side, 0.0));
  var c = highlight_c(p_info.wire);
  out.color = c;
  if (k.z == 0.0) {
    if (look.mode.w > 1.5) { out.color = vec4<f32>(look.wire.rgb, c.a); }
    else if (look.plain.w > 0.5 && all(abs(c.rgb - look.plain.rgb) < vec3<f32>(0.006))) {
      out.color = vec4<f32>(look.inactive.rgb, c.a);
    }
    out.color.a = out.color.a * look.mode.z;
  }
  out.color.a = twice(out.color.a, f32(p_wire[w + 5u]));
  // Fade Wireframe on Small Polygons (gpu.rs edge_out_fading): by the
  // cage edge's length on screen (the stored path never fades a traced
  // cage edge, whose pieces are far shorter than it), keeping outline,
  // open and sharp edges (`kept`, from the normals on each side).
  if (place.sel.z > 0.5 && look.wire_fade.x > 0.5) {
    let wlo = (p_info.model * vec4<f32>(lo, 1.0)).xyz;
    let whi = (p_info.model * vec4<f32>(hi, 1.0)).xyz;
    let s1 = p_side(p_sides[2u * curve]);
    let s2 = p_side(p_sides[2u * curve + 1u]);
    if (!kept(wlo, whi, s1, s2)) {
      let ea = project(placed(wlo));
      let eb = project(placed(whi));
      if (ea.w > 0.0 && eb.w > 0.0) {
        let len = length((eb.xy / eb.w - ea.xy / ea.w) * cam.view.xy * 0.5);
        let f = clamp((len - look.wire_fade.y) / (look.wire_fade.z - look.wire_fade.y), 0.0, 1.0);
        if (f <= 0.0) { out.position = culled; return out; }
        out.color.a = out.color.a * f;
      }
    }
  }
  let wp = placed((p_info.model * vec4<f32>(p, 1.0)).xyz);
  let wd = (place.m * (p_info.model * vec4<f32>(d, 0.0))).xyz;
  let cp = project(wp);
  if (cam.params.z > 0.5 && cp.w < cam.params.w) { out.position = culled; return out; }
  // The direction on screen: a short step along the curve, projected.
  let span = max(length(wd), 1e-20);
  let cq = project(wp + wd * (0.001 * max(cp.w, 1e-6) / span));
  let half = cam.view.xy * 0.5;
  let pa = cp.xy / cp.w * half;
  let pb = cq.xy / cq.w * half;
  var dir = pb - pa;
  if (length(dir) < 1e-12) { dir = vec2<f32>(1.0, 0.0); }
  dir = normalize(dir);
  let normal = vec2<f32>(-dir.y, dir.x);
  let width = select(cam.view.z, k.z, k.z > 0.0);
  let offset = normal * side * width * 0.5 / half;
  var clip = vec4<f32>(cp.xy + offset * cp.w, cp.z, cp.w);
  if (cam.params.z > 0.5) { clip.z = clip.z - cam.params.w * 0.002; } else { clip.z = clip.z - cam.forward.w * 0.002; }
  out.position = clip;
  return out;
}
"#;

/// Culling (run before the view's passes): every cluster of patches, and
/// of wire records, whose normals all face away from the eye is left out
/// of the draws. The cluster's sphere holds its controls and its cone every
/// normal (subdivision::patches::Cluster), so the test is exact: back
/// faces are what the fill pipeline culls anyway. The wire is culled only
/// on a closed surface seen from outside its bounds (cull.flags.y), where
/// whatever faces away is hidden behind the near side.
pub const CULL_SHADER: &str = r#"
// eye: the eye in the item's own space (w = 1), or for an orthographic
// view the view direction (w = 0). counts = (fill clusters, wire clusters,
// fill indices, wire indices); flags = (cull fill, cull wire, -, -).
struct Cull { eye: vec4<f32>, counts: vec4<u32>, flags: vec4<u32> };
struct Cluster { centre: vec4<f32>, cone: vec4<f32> };
@group(0) @binding(0) var<uniform> cull: Cull;
@group(0) @binding(1) var<storage, read> fill_clusters: array<Cluster>;
@group(0) @binding(2) var<storage, read> wire_clusters: array<Cluster>;
@group(0) @binding(3) var<storage, read_write> fill_shown: array<u32>;
@group(0) @binding(4) var<storage, read_write> wire_shown: array<u32>;
// DrawIndexedIndirect: index count, instance count, first index, base
// vertex, first instance.
@group(0) @binding(5) var<storage, read_write> fill_args: array<atomic<u32>, 5>;
@group(0) @binding(6) var<storage, read_write> wire_args: array<atomic<u32>, 5>;
// Every point of the sphere sees every normal of the cone from behind:
// the angle from the axis to the eye ray plus the cone's half angle stays
// under 90°, with room for the radius.
fn faces_away(k: Cluster) -> bool {
  let cos_t = k.cone.w;
  if (cos_t <= 0.0) { return false; }
  var d = cull.eye.xyz;
  if (cull.eye.w > 0.5) { d = k.centre.xyz - cull.eye.xyz; }
  let len = length(d);
  if (len <= 0.0) { return false; }
  let cos_a = dot(k.cone.xyz, d) / len;
  if (cos_a <= 0.0) { return false; }
  let sin_a = sqrt(max(0.0, 1.0 - cos_a * cos_a));
  let sin_t = sqrt(max(0.0, 1.0 - cos_t * cos_t));
  let cos_sum = cos_a * cos_t - sin_a * sin_t;
  if (cos_sum <= 0.0) { return false; }
  if (cull.eye.w > 0.5) { return len * cos_sum > k.centre.w; }
  return true;
}
fn index_of(id: vec3<u32>, groups: vec3<u32>) -> u32 { return id.x + id.y * groups.x * 64u; }
@compute @workgroup_size(64) fn cull_fill(@builtin(global_invocation_id) id: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {
  let i = index_of(id, groups);
  if (i >= cull.counts.x) { return; }
  if (cull.flags.x == 0u || !faces_away(fill_clusters[i])) {
    fill_shown[atomicAdd(&fill_args[1], 1u)] = i;
  }
}
@compute @workgroup_size(64) fn cull_wire(@builtin(global_invocation_id) id: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {
  let i = index_of(id, groups);
  if (i >= cull.counts.y) { return; }
  if (cull.flags.y == 0u || !faces_away(wire_clusters[i])) {
    wire_shown[atomicAdd(&wire_args[1], 1u)] = i;
  }
}
"#;

thread_local! {
    /// Tests: draw Subdivision polygons the stored way instead of as
    /// patches, to compare pictures.
    static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
#[cfg(test)]
pub fn set_off(off: bool) {
    OFF.with(|c| c.set(off));
}
/// Whether patches are in use (always in the app).
pub fn on() -> bool {
    !OFF.with(std::cell::Cell::get)
}

thread_local! {
    /// Tests: draw even small surfaces as patches.
    static MIN_PIECES: std::cell::Cell<u64> =
        const { std::cell::Cell::new(operandi_core::subdivision::PATCH_MIN_PIECES) };
}
#[cfg(test)]
pub fn set_min_pieces(n: u64) {
    MIN_PIECES.with(|c| c.set(n));
}
/// Smooth pieces from which a surface draws as patches.
pub fn min_pieces() -> u64 {
    MIN_PIECES.with(std::cell::Cell::get)
}

/// Patches drawn per instance: one cluster, about 192 grid samples, so
/// the vertex cache reuses each shared sample.
fn per_instance(level: u8) -> u32 {
    operandi_core::subdivision::patches::fill_per_cluster(level)
}

/// Cage edges drawn per instance: one wire cluster.
fn wires_per_instance(level: u8) -> u32 {
    operandi_core::subdivision::patches::wire_per_cluster(level)
}

/// Whether culling is on (tests compare pictures without it).
static CULLING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
#[cfg(test)]
pub fn set_culling(on: bool) {
    CULLING.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// The wire's index list for one instance: each edge's 2L segments as two
/// triangles over its strip of stroke corners.
fn wire_pattern(level: u8) -> Vec<u32> {
    let points = 2 * u32::from(level) + 1;
    let per = 2 * points;
    let mut out = Vec::new();
    for e in 0..wires_per_instance(level) {
        let base = e * per;
        for s in 0..points - 1 {
            let (a, b, c, d) = (
                base + 2 * s,
                base + 2 * s + 1,
                base + 2 * s + 2,
                base + 2 * s + 3,
            );
            out.extend([a, b, d, a, d, c]);
        }
    }
    out
}

/// The fill's index list for one instance: every patch's L × L quads as
/// two triangles each, wound as the stored fill winds them.
fn index_pattern(level: u8, flip: bool) -> Vec<u32> {
    let l = u32::from(level);
    let side = l + 1;
    let per = side * side;
    let mut out = Vec::new();
    for q in 0..per_instance(level) {
        let base = q * per;
        for i in 0..l {
            for j in 0..l {
                let g = |a: u32, b: u32| base + a * side + b;
                let quad = [g(i, j), g(i + 1, j), g(i + 1, j + 1), g(i, j + 1)];
                for t in [[0, 1, 2], [0, 2, 3]] {
                    let t = if flip { [t[0], t[2], t[1]] } else { t };
                    out.extend(t.map(|k| quad[k]));
                }
            }
        }
    }
    out
}

/// The patch bind group layout (group 3 of the patch pipelines).
pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let uniform = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let storage = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Operandi patches"),
        entries: &[
            uniform(0),
            storage(1),
            storage(2),
            storage(3),
            storage(4),
            storage(5),
            uniform(6),
            storage(7),
            storage(8),
            storage(9),
            storage(10),
            storage(11),
        ],
    })
}

/// The culling bind group layout (group 0 of the cull pipelines).
fn cull_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let read = wgpu::BufferBindingType::Storage { read_only: true };
    let write = wgpu::BufferBindingType::Storage { read_only: false };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Operandi patch culling"),
        entries: &[
            entry(0, wgpu::BufferBindingType::Uniform),
            entry(1, read),
            entry(2, read),
            entry(3, write),
            entry(4, write),
            entry(5, write),
            entry(6, write),
        ],
    })
}

/// Whether the device can hold `net`'s buffers (each under its binding
/// limit).
pub fn fits(device: &wgpu::Device, net: &PatchNet) -> bool {
    let limit = u64::from(device.limits().max_storage_buffer_binding_size)
        .min(device.limits().max_buffer_size);
    [
        net.controls.len() as u64 * 12,
        net.patches.len() as u64 * 32,
        net.owner.len() as u64 * 4,
        net.blocks.len() as u64 * 16,
        net.wire.len() as u64 * 24,
    ]
    .iter()
    .all(|&b| b <= limit)
}

/// One item's patches on the graphics card.
pub struct PatchGpu {
    net: Arc<PatchNet>,
    controls: wgpu::Buffer,
    ids: wgpu::Buffer,
    owner: wgpu::Buffer,
    looks: wgpu::Buffer,
    looks_data: Vec<[u32; crate::gpu_scene::PATCH_LOOK]>,
    wire: wgpu::Buffer,
    info: wgpu::Buffer,
    info_bytes: Vec<u8>,
    index: wgpu::Buffer,
    wire_index: wgpu::Buffer,
    pub wire_index_count: u32,
    flip: bool,
    pub index_count: u32,
    /// Culling: the clusters, the ones left this frame, the indirect draw
    /// arguments, the eye uniform and their bind group.
    fill_clusters: u32,
    wire_clusters: u32,
    fill_shown: wgpu::Buffer,
    wire_shown: wgpu::Buffer,
    fill_args: wgpu::Buffer,
    wire_args: wgpu::Buffer,
    cull: wgpu::Buffer,
    cull_group: wgpu::BindGroup,
    fill_cluster_buffer: wgpu::Buffer,
    wire_cluster_buffer: wgpu::Buffer,
    sides: wgpu::Buffer,
    uvs: wgpu::Buffer,
    uvs_data: Option<Arc<Vec<[f32; 2]>>>,
    blocks: wgpu::Buffer,
    /// One bind group per placement uniform the item draws through this
    /// frame (gpu.rs `Shared`: plain, whole selected fill, whole selected
    /// outline), with the buffer it was made for.
    groups: [Option<(wgpu::Buffer, wgpu::BindGroup)>; 3],
}

fn storage(device: &wgpu::Device, label: &str, bytes: &[u8]) -> wgpu::Buffer {
    // A zero-length storage binding isn't allowed.
    let bytes = if bytes.is_empty() {
        &[0u8; 16][..]
    } else {
        bytes
    };
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    })
}

fn flat<'a, T: Copy + 'a, const N: usize>(
    rows: impl IntoIterator<Item = &'a [T; N], IntoIter: ExactSizeIterator>,
    each: impl Fn(T) -> [u8; 4],
) -> Vec<u8> {
    let rows = rows.into_iter();
    let mut out = Vec::with_capacity(rows.len() * N * 4);
    for row in rows {
        for &v in row {
            out.extend_from_slice(&each(v));
        }
    }
    out
}

fn info_bytes(draw: &PatchDraw, model: glam::DMat4, flip: bool) -> Vec<u8> {
    let net = &draw.net;
    let normal = glam::DMat4::from_mat3(glam::DMat3::from_mat4(model).inverse().transpose());
    let mut out = Vec::with_capacity(176);
    for f in model
        .as_mat4()
        .to_cols_array()
        .into_iter()
        .chain(normal.as_mat4().to_cols_array())
    {
        out.extend_from_slice(&f.to_le_bytes());
    }
    for u in [
        u32::from(net.level),
        per_instance(net.level),
        net.patches.len() as u32,
        net.centre_base,
    ] {
        out.extend_from_slice(&u.to_le_bytes());
    }
    for c in draw.wire.to_array() {
        out.extend_from_slice(&(f32::from(c) / 255.).to_le_bytes());
    }
    for u in [
        net.wire.len() as u32,
        u32::from(flip),
        0,
        wires_per_instance(net.level),
    ] {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}

/// Write the chunks of `now` that aren't the very ones of `was` (what the
/// card holds) into `buffer`, `stride` bytes per element. Returns the bytes
/// sent.
fn send_chunks<T: Clone>(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    was: &operandi_core::subdivision::patches::Chunks<T>,
    now: &operandi_core::subdivision::patches::Chunks<T>,
    stride: u64,
    bytes: impl Fn(&Vec<T>) -> Vec<u8>,
) -> u64 {
    let mut sent = 0;
    for (k, chunk) in now.chunks().iter().enumerate() {
        if was.same_chunk(now, k) {
            continue;
        }
        let b = bytes(chunk);
        let at = (k * operandi_core::subdivision::patches::CHUNK) as u64 * stride;
        queue.write_buffer(buffer, at, &b);
        sent += b.len() as u64;
    }
    sent
}

/// The storage and info buffers of one item's patches.
struct Parts<'a> {
    controls: &'a wgpu::Buffer,
    ids: &'a wgpu::Buffer,
    owner: &'a wgpu::Buffer,
    looks: &'a wgpu::Buffer,
    wire: &'a wgpu::Buffer,
    info: &'a wgpu::Buffer,
    fill_shown: &'a wgpu::Buffer,
    wire_shown: &'a wgpu::Buffer,
    sides: &'a wgpu::Buffer,
    uvs: &'a wgpu::Buffer,
    blocks: &'a wgpu::Buffer,
}

fn bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    placement: &wgpu::Buffer,
    b: Parts<'_>,
) -> wgpu::BindGroup {
    fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: buffer.as_entire_binding(),
        }
    }
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Operandi patches"),
        layout,
        entries: &[
            entry(0, placement),
            entry(1, b.controls),
            entry(2, b.ids),
            entry(3, b.owner),
            entry(4, b.looks),
            entry(5, b.wire),
            entry(6, b.info),
            entry(7, b.fill_shown),
            entry(8, b.wire_shown),
            entry(9, b.sides),
            entry(10, b.uvs),
            entry(11, b.blocks),
        ],
    })
}

fn clusters_bytes<'a>(
    clusters: impl ExactSizeIterator<Item = &'a operandi_core::subdivision::patches::Cluster>,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(clusters.len() * 32);
    for k in clusters {
        for f in k
            .centre
            .iter()
            .chain([&k.radius])
            .chain(&k.axis)
            .chain([&k.cos])
        {
            out.extend_from_slice(&f.to_le_bytes());
        }
    }
    out
}

impl PatchGpu {
    /// Upload `draw`, built at `model`. Returns the bytes sent too. Bind
    /// it to its placements with `bind_placement` before drawing.
    pub fn new(
        device: &wgpu::Device,
        pipelines: &PatchPipelines,
        draw: &PatchDraw,
        model: glam::DMat4,
    ) -> (Self, u64) {
        let net = &draw.net;
        let flip = model.determinant() < 0.;
        // Chunk by chunk, so no list of references to every control is made.
        let chunked = |parts: &[Arc<Vec<_>>], each: &dyn Fn(&Vec<_>) -> Vec<u8>| -> Vec<u8> {
            parts.iter().flat_map(|c| each(c)).collect()
        };
        let controls = chunked(net.controls.chunks(), &|c: &Vec<[f32; 3]>| {
            flat(c.iter(), f32::to_le_bytes)
        });
        let sides: Vec<u8> = net
            .wire_sides
            .chunks()
            .iter()
            .flat_map(|c| flat(c.iter(), u32::to_le_bytes))
            .collect();
        let ids = flat(net.patches.iter(), u32::to_le_bytes);
        let owner: Vec<u8> = net.owner.iter().flat_map(|v| v.to_le_bytes()).collect();
        let looks = flat(draw.looks.iter(), u32::to_le_bytes);
        let wire = flat(net.wire.iter(), u32::to_le_bytes);
        let indices = index_pattern(net.level, flip);
        let wire_indices = wire_pattern(net.level);
        let info = info_bytes(draw, model, flip);
        let sent = (controls.len() + ids.len() + owner.len() + looks.len() + wire.len()) as u64;
        let index_bytes: Vec<u8> = indices.iter().flat_map(|v| v.to_le_bytes()).collect();
        let info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Operandi patch info"),
            contents: &info,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let controls = storage(device, "Operandi patch controls", &controls);
        let ids = storage(device, "Operandi patch ids", &ids);
        let owner = storage(device, "Operandi patch owners", &owner);
        let looks = storage(device, "Operandi patch looks", &looks);
        let wire = storage(device, "Operandi patch wire", &wire);
        let sides = storage(device, "Operandi patch wire sides", &sides);
        let uvs = storage(
            device,
            "Operandi patch UVs",
            &draw
                .uvs
                .as_ref()
                .map(|u| flat(u.iter(), f32::to_le_bytes))
                .unwrap_or_default(),
        );
        let blocks = storage(
            device,
            "Operandi patch blocks",
            &net.blocks
                .iter()
                .flat_map(|r| [r.start, r.end])
                .flat_map(u32::to_le_bytes)
                .collect::<Vec<u8>>(),
        );
        let fill_clusters = storage(
            device,
            "Operandi patch clusters",
            &net.fill_clusters
                .chunks()
                .iter()
                .flat_map(|c| clusters_bytes(c.iter()))
                .collect::<Vec<u8>>(),
        );
        let wire_clusters = storage(
            device,
            "Operandi patch wire clusters",
            &net.wire_clusters
                .chunks()
                .iter()
                .flat_map(|c| clusters_bytes(c.iter()))
                .collect::<Vec<u8>>(),
        );
        let shown = |label, n: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (n.max(4) * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        let fill_shown = shown("Operandi patches shown", net.fill_clusters.len());
        let wire_shown = shown("Operandi patch wire shown", net.wire_clusters.len());
        let args = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: 20,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::INDIRECT
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let fill_args = args("Operandi patch draw");
        let wire_args = args("Operandi patch wire draw");
        let cull = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Operandi patch culling"),
            size: 48,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cull_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Operandi patch culling"),
            layout: &pipelines.cull_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cull.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: fill_clusters.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wire_clusters.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: fill_shown.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wire_shown.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: fill_args.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wire_args.as_entire_binding(),
                },
            ],
        });
        let b = Self {
            net: net.clone(),
            controls,
            ids,
            owner,
            looks,
            looks_data: draw.looks.clone(),
            wire,
            info: info_buffer,
            info_bytes: info,
            index: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Operandi patch indices"),
                contents: &index_bytes,
                usage: wgpu::BufferUsages::INDEX,
            }),
            wire_index: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Operandi patch wire indices"),
                contents: &wire_indices
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<u8>>(),
                usage: wgpu::BufferUsages::INDEX,
            }),
            wire_index_count: wire_indices.len() as u32,
            flip,
            index_count: indices.len() as u32,
            fill_clusters: net.fill_clusters.len() as u32,
            wire_clusters: net.wire_clusters.len() as u32,
            fill_shown,
            wire_shown,
            fill_args,
            wire_args,
            cull,
            cull_group,
            fill_cluster_buffer: fill_clusters,
            wire_cluster_buffer: wire_clusters,
            sides,
            uvs,
            uvs_data: draw.uvs.clone(),
            blocks,
            groups: [None, None, None],
        };
        (b, sent)
    }

    /// Bring this upload up to `draw` (built at `model`) when only the
    /// looks, placement or (a drag) the controls changed: same patches,
    /// same winding. Sends only the chunks of controls, clusters and sides
    /// that differ from what the card holds. Returns the bytes sent, or
    /// `None` when it needs a new upload.
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        draw: &PatchDraw,
        model: glam::DMat4,
    ) -> Option<u64> {
        let flip = model.determinant() < 0.;
        if self.net.topology != draw.net.topology || flip != self.flip {
            return None;
        }
        // UVs: the same list, or (a selection's dirty polygons) the same
        // length rewritten whole; appearing or disappearing needs new ones.
        match (&self.uvs_data, &draw.uvs) {
            (None, None) => {}
            (Some(a), Some(b)) if Arc::ptr_eq(a, b) => {}
            (Some(a), Some(b)) if a.len() == b.len() => {
                queue.write_buffer(&self.uvs, 0, &flat(b.iter(), f32::to_le_bytes));
                self.uvs_data = Some(b.clone());
            }
            _ => return None,
        }
        let mut sent = 0;
        sent += send_chunks(
            queue,
            &self.controls,
            &self.net.controls,
            &draw.net.controls,
            12,
            |c| flat(c.iter(), f32::to_le_bytes),
        );
        sent += send_chunks(
            queue,
            &self.fill_cluster_buffer,
            &self.net.fill_clusters,
            &draw.net.fill_clusters,
            32,
            |c| clusters_bytes(c.iter()),
        );
        sent += send_chunks(
            queue,
            &self.wire_cluster_buffer,
            &self.net.wire_clusters,
            &draw.net.wire_clusters,
            32,
            |c| clusters_bytes(c.iter()),
        );
        sent += send_chunks(
            queue,
            &self.sides,
            &self.net.wire_sides,
            &draw.net.wire_sides,
            8,
            |c| flat(c.iter(), u32::to_le_bytes),
        );
        self.net = draw.net.clone();
        if self.looks_data != draw.looks {
            // Only the rows that changed (a selection changes a few).
            let mut f = 0;
            while f < draw.looks.len() {
                if self.looks_data[f] == draw.looks[f] {
                    f += 1;
                    continue;
                }
                let start = f;
                while f < draw.looks.len() && self.looks_data[f] != draw.looks[f] {
                    f += 1;
                }
                let bytes = flat(draw.looks[start..f].iter(), u32::to_le_bytes);
                queue.write_buffer(
                    &self.looks,
                    (start * crate::gpu_scene::PATCH_LOOK * 4) as u64,
                    &bytes,
                );
                sent += bytes.len() as u64;
            }
            self.looks_data.clone_from(&draw.looks);
        }
        let info = info_bytes(draw, model, flip);
        if info != self.info_bytes {
            queue.write_buffer(&self.info, 0, &info);
            self.info_bytes = info;
        }
        Some(sent)
    }

    /// Make sure placement slot `slot` (0 plain, 1 whole selected fill,
    /// 2 whole selected outline) is bound through `buffer`.
    pub fn bind_placement(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        slot: usize,
        buffer: &wgpu::Buffer,
    ) {
        if self.groups[slot].as_ref().is_none_or(|(b, _)| b != buffer) {
            let group = bind(device, layout, buffer, self.parts());
            self.groups[slot] = Some((buffer.clone(), group));
        }
    }

    fn parts(&self) -> Parts<'_> {
        Parts {
            controls: &self.controls,
            ids: &self.ids,
            owner: &self.owner,
            looks: &self.looks,
            wire: &self.wire,
            info: &self.info,
            fill_shown: &self.fill_shown,
            wire_shown: &self.wire_shown,
            sides: &self.sides,
            uvs: &self.uvs,
            blocks: &self.blocks,
        }
    }

    /// Cull for one view: `to_item` takes the view's world into the
    /// item's own space (the placement and the build matrix, inverted);
    /// `camera` is the view's camera uniform (gpu.rs: eye, right, up,
    /// forward, params, view). Writes this frame's arguments, then encodes
    /// the culling in `pass`.
    pub fn cull(
        &self,
        queue: &wgpu::Queue,
        pipelines: &PatchPipelines,
        pass: &mut wgpu::ComputePass<'_>,
        to_item: glam::DMat4,
        camera: &[f32; 24],
    ) {
        let perspective = camera[18] > 0.5;
        let eye = if perspective {
            to_item
                .transform_point3(glam::DVec3::new(
                    camera[0] as f64,
                    camera[1] as f64,
                    camera[2] as f64,
                ))
                .extend(1.)
        } else {
            to_item
                .transform_vector3(glam::DVec3::new(
                    camera[12] as f64,
                    camera[13] as f64,
                    camera[14] as f64,
                ))
                .extend(0.)
        };
        let on = CULLING.load(std::sync::atomic::Ordering::Relaxed);
        // A mirroring placement turns the faces over; don't guess.
        let fill = on && to_item.determinant() > 0.;
        let [lo, hi] = self.net.bounds;
        let inside =
            perspective && (0..3).all(|i| eye[i] as f32 >= lo[i] && eye[i] as f32 <= hi[i]);
        let wire = fill && self.net.closed && !inside;
        let mut bytes = Vec::with_capacity(48);
        for f in eye.as_vec4().to_array() {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        for u in [
            self.fill_clusters,
            self.wire_clusters,
            self.index_count,
            self.wire_index_count,
            u32::from(fill),
            u32::from(wire),
            0,
            0,
        ] {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        queue.write_buffer(&self.cull, 0, &bytes);
        let args = |count: u32| -> Vec<u8> {
            [count, 0, 0, 0, 0]
                .iter()
                .flat_map(|v: &u32| v.to_le_bytes())
                .collect()
        };
        queue.write_buffer(&self.fill_args, 0, &args(self.index_count));
        queue.write_buffer(&self.wire_args, 0, &args(self.wire_index_count));
        pass.set_bind_group(0, &self.cull_group, &[]);
        for (pipeline, n) in [
            (&pipelines.cull_fill, self.fill_clusters),
            (&pipelines.cull_wire, self.wire_clusters),
        ] {
            let groups = n.div_ceil(64);
            if groups == 0 {
                continue;
            }
            pass.set_pipeline(pipeline);
            let x = groups.min(65535);
            pass.dispatch_workgroups(x, groups.div_ceil(x), 1);
        }
    }

    /// The clusters culling left (`cull` this frame), through placement
    /// slot `slot` (`bind_placement`; nothing when it isn't bound).
    pub fn draw_fill(&self, pass: &mut wgpu::RenderPass<'_>, slot: usize) {
        let Some((_, group)) = &self.groups[slot] else {
            return;
        };
        pass.set_bind_group(3, group, &[]);
        pass.set_index_buffer(self.index.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed_indirect(&self.fill_args, 0);
    }

    pub fn draw_wire(&self, pass: &mut wgpu::RenderPass<'_>, slot: usize) {
        let Some((_, group)) = &self.groups[slot] else {
            return;
        };
        pass.set_bind_group(3, group, &[]);
        pass.set_index_buffer(self.wire_index.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed_indirect(&self.wire_args, 0);
    }
}

/// A patch pipeline: the fill (as gpu.rs `fill_pipeline`) or the wire (as
/// the resident edges).
fn make_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    samples: u32,
    shader: &wgpu::ShaderModule,
    vertex: &str,
    fragment: &str,
    fill: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Operandi patches"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: (!fill).then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: fill.then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: fill,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: if fill {
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
}

/// The patch pipelines: the fill (as `fill_pipeline`) and the wire (as
/// the resident edges).
pub struct PatchPipelines {
    pub layout: wgpu::BindGroupLayout,
    pub fill: wgpu::RenderPipeline,
    /// The fill with the current material programs (image layers drawn
    /// per pixel), by the generated module it was made from.
    pub material_fill: Option<(String, wgpu::RenderPipeline)>,
    pipeline_layout: wgpu::PipelineLayout,
    samples: u32,
    pub edges: wgpu::RenderPipeline,
    cull_layout: wgpu::BindGroupLayout,
    cull_fill: wgpu::ComputePipeline,
    cull_wire: wgpu::ComputePipeline,
}

impl PatchPipelines {
    pub fn new(
        device: &wgpu::Device,
        samples: u32,
        world_shader: &str,
        groups: [&wgpu::BindGroupLayout; 3],
    ) -> Self {
        let layout = layout(device);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Operandi patch shader"),
            source: wgpu::ShaderSource::Wgsl(format!("{world_shader}\n{PATCH_SHADER}").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Operandi patches"),
            bind_group_layouts: &[groups[0], groups[1], groups[2], &layout],
            push_constant_ranges: &[],
        });
        let make = |vertex: &str, fragment: &str, fill: bool| {
            make_pipeline(
                device,
                &pipeline_layout,
                samples,
                &shader,
                vertex,
                fragment,
                fill,
            )
        };
        let cull_layout = cull_layout(device);
        let cull_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Operandi patch culling"),
            source: wgpu::ShaderSource::Wgsl(CULL_SHADER.into()),
        });
        let cull_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Operandi patch culling"),
            bind_group_layouts: &[&cull_layout],
            push_constant_ranges: &[],
        });
        let compute = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Operandi patch culling"),
                layout: Some(&cull_pipeline_layout),
                module: &cull_shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        Self {
            fill: make("vs_patch_fill", "fs_fill", true),
            edges: make("vs_patch_edge", "fs", false),
            material_fill: None,
            pipeline_layout,
            samples,
            layout,
            cull_fill: compute("cull_fill"),
            cull_wire: compute("cull_wire"),
            cull_layout,
        }
    }
}

impl PatchPipelines {
    /// Keep the material fill in step with the resident fill: `module` is
    /// the generated material programs (`shader_wgsl`) the resident fill
    /// draws with, `None` for the plain one. `world` makes the resident
    /// shader around a module (gpu.rs `world_shader`).
    pub fn sync_materials(
        &mut self,
        device: &wgpu::Device,
        module: Option<&str>,
        world: impl Fn(&str) -> String,
    ) {
        let Some(module) = module else {
            self.material_fill = None;
            return;
        };
        if self
            .material_fill
            .as_ref()
            .is_some_and(|(m, _)| m == module)
        {
            return;
        }
        let source = format!("{}\n{PATCH_SHADER}", world(module));
        if crate::material_gpu::valid(&source).is_err() {
            self.material_fill = None;
            return;
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Operandi patch shader (image layers)"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let fill = make_pipeline(
            device,
            &self.pipeline_layout,
            self.samples,
            &shader,
            "vs_patch_fill",
            "fs_fill",
            true,
        );
        self.material_fill = Some((module.to_owned(), fill));
    }
    /// The fill to draw with now.
    pub fn fill(&self) -> &wgpu::RenderPipeline {
        self.material_fill.as_ref().map_or(&self.fill, |(_, p)| p)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn index_pattern_covers_each_patch_grid() {
        let indices = super::index_pattern(2, false);
        let per = super::per_instance(2);
        assert_eq!(indices.len() as u32, per * 4 * 6);
        assert!(indices.iter().all(|&i| i < per * 9));
        // The first quad winds as the polygon does: (0,0) (1,0) (1,1).
        assert_eq!(&indices[..3], &[0, 3, 4]);
        let flipped = super::index_pattern(2, true);
        assert_eq!(&flipped[..3], &[0, 4, 3]);
    }

    #[test]
    fn cull_shader_validates() {
        use egui_wgpu::wgpu::naga;
        let module = naga::front::wgsl::parse_str(super::CULL_SHADER).unwrap_or_else(|e| {
            panic!("{}", e.emit_to_string(super::CULL_SHADER));
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }

    #[test]
    fn patch_shader_validates() {
        use egui_wgpu::wgpu::naga;
        let source = format!(
            "{}\n{}",
            crate::gpu::world_shader_for_tests(&operandi_core::shader_wgsl::module(&[])),
            super::PATCH_SHADER
        );
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| {
            panic!("{}", e.emit_to_string(&source));
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}
