// Procedural patterns per pixel in the viewport (shader-tree-complete,
// VM-Q11): a line-for-line port of procedural.rs `Pattern::value` and
// `Pattern::cell_random`, reading the numbers `Pattern::packed` gives
// (mode, then a, b, c = parameters 0..3, 4..7, 8..11). Keep the two in step;
// material_gpu_tests compares them.

fn op_pt_smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
  if (e1 <= e0) { return select(1.0, 0.0, x <= e0); }
  let t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
  return t * t * (3.0 - 2.0 * t);
}
fn op_pt_plane(axis: u32, p: vec3<f32>) -> vec2<f32> {
  if (axis == 0u) { return vec2<f32>(-p.z, p.y); }
  if (axis == 1u) { return vec2<f32>(p.x, -p.z); }
  return p.xy;
}
fn op_pt_parity(x: f32, width: f32) -> f32 {
  let p = 2.0 * x;
  let fl = floor(p);
  let half = fl * 0.5;
  let sign = 1.0 - 4.0 * (half - floor(half));
  let f = p - fl;
  let d = min(f, 1.0 - f);
  return sign * op_pt_smoothstep(0.0, width * 0.5, d);
}
fn op_pt_cover(dist: f32, half: f32, transition: f32) -> f32 {
  return 1.0 - op_pt_smoothstep(half - transition * 0.5, half + transition * 0.5, dist);
}
fn op_pt_line(x: f32) -> f32 { return abs((x - floor(x)) - 0.5); }
fn op_pt_mod(a: f32, b: f32) -> f32 { return a - b * floor(a / b); }
fn op_pt_hex_g(u: f32, v: f32) -> vec2<f32> {
  let r1 = 1.7320508;
  let h1 = 0.8660254;
  let a = vec2<f32>(op_pt_mod(u, 1.0) - 0.5, op_pt_mod(v, r1) - h1);
  let b = vec2<f32>(op_pt_mod(u - 0.5, 1.0) - 0.5, op_pt_mod(v - h1, r1) - h1);
  return select(b, a, dot(a, a) < dot(b, b));
}
fn op_pt_hex_edge(u: f32, v: f32) -> f32 {
  let g = abs(op_pt_hex_g(u, v));
  return 0.5 - max(g.x, 0.5 * g.x + 0.8660254 * g.y);
}
fn op_pt_hash(x: i32, y: i32, z: i32) -> u32 {
  var h = (bitcast<u32>(x) * 0x8da6b343u) ^ (bitcast<u32>(y) * 0xd8163841u) ^ (bitcast<u32>(z) * 0xcb1ab31fu);
  h = h ^ (h >> 15u);
  h = h * 0x2c1b3c6du;
  h = h ^ (h >> 12u);
  h = h * 0x297a2d39u;
  return h ^ (h >> 15u);
}
fn op_pt_random3(c: vec3<f32>) -> vec3<f32> {
  let x = i32(c.x);
  let y = i32(c.y);
  let z = i32(c.z);
  var out = vec3<f32>(0.0);
  for (var k = 0; k < 3; k++) {
    let h = op_pt_hash(x + k * 7919, y, z);
    out[k] = f32(h >> 8u) / 16777216.0;
  }
  return out;
}
fn op_pt_ripple_hash(x0: u32) -> u32 {
  var x = x0;
  x = x ^ (x >> 16u);
  x = x * 0x7feb352du;
  x = x ^ (x >> 15u);
  x = x * 0x846ca68bu;
  return x ^ (x >> 16u);
}
const OP_PT_G = array<vec3<f32>, 12>(
  vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(-1.0, 1.0, 0.0), vec3<f32>(1.0, -1.0, 0.0), vec3<f32>(-1.0, -1.0, 0.0),
  vec3<f32>(1.0, 0.0, 1.0), vec3<f32>(-1.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, -1.0), vec3<f32>(-1.0, 0.0, -1.0),
  vec3<f32>(0.0, 1.0, 1.0), vec3<f32>(0.0, -1.0, 1.0), vec3<f32>(0.0, 1.0, -1.0), vec3<f32>(0.0, -1.0, -1.0));
fn op_pt_gradient(x: i32, y: i32, z: i32, phase: f32) -> vec3<f32> {
  let h = op_pt_hash(x, y, z);
  let g = OP_PT_G[h % 12u];
  if (phase == 0.0) { return g; }
  let k = OP_PT_G[(h >> 8u) % 12u] * 0.70710678;
  let angle = 6.2831853 * phase;
  let s = sin(angle);
  let c = cos(angle);
  return g * c + cross(k, g) * s + k * dot(k, g) * (1.0 - c);
}
fn op_pt_noise(p: vec3<f32>, phase: f32) -> f32 {
  let cell = floor(p);
  let f = p - cell;
  let w = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  var out = 0.0;
  for (var corner = 0u; corner < 8u; corner++) {
    let o = vec3<f32>(f32(corner & 1u), f32((corner >> 1u) & 1u), f32((corner >> 2u) & 1u));
    let g = op_pt_gradient(i32(cell.x + o.x), i32(cell.y + o.y), i32(cell.z + o.z), phase);
    let weight = select(1.0 - w, w, o > vec3<f32>(0.0));
    out += weight.x * weight.y * weight.z * dot(g, f - o);
  }
  return out;
}
// (f1, f2) and the nearest cell's three randoms (cells_with): Cycles'
// Voronoi, as renders build Cellular (cycles_voronoi.wgsl).
struct OpPtCells { f1: f32, f2: f32, id: vec3<f32> };
fn op_pt_cells(p: vec3<f32>, randomness: f32, flat: bool) -> OpPtCells {
  let n = select(3u, 2u, flat);
  let pr = OpVParams(n, 0u, 0u, false, 1.0, 0.0, 0.5, 2.0, 0.5, 0.5, clamp(randomness, 0.0, 1.0));
  var coord = vec4<f32>(p, 0.0);
  if (flat) { coord.z = 0.0; }
  let a = op_v_octave(pr, coord, n, 0u);
  let b = op_v_octave(pr, coord, n, 1u);
  return OpPtCells(a.d, b.d, a.c);
}
fn op_pt_weave(u: f32, v: f32, width: f32, roundness: f32) -> f32 {
  let pu = 2.0 * u;
  let pv = 2.0 * v;
  let fu = pu - floor(pu);
  let fv = pv - floor(pv);
  let sum = floor(pu) + floor(pv);
  let top_h = select(0.0, 1.0, (sum - 2.0 * floor(sum * 0.5)) < 0.5);
  let half = max(width * 0.5, 1e-6);
  let xh = abs(fv - 0.5) / half;
  let xv = abs(fu - 0.5) / half;
  let in_h = select(0.0, 1.0, xh < 1.0);
  let in_v = select(0.0, 1.0, xv < 1.0);
  let ph = 1.0 + roundness * (sqrt(max(1.0 - xh * xh, 0.0)) - 1.0);
  let pv2 = 1.0 + roundness * (sqrt(max(1.0 - xv * xv, 0.0)) - 1.0);
  let sel_h = in_h * (top_h + (1.0 - top_h) * (1.0 - in_v));
  let sel_v = in_v * ((1.0 - top_h) + top_h * (1.0 - in_h));
  return sel_h * ph + sel_v * pv2;
}
// procedural.rs gabor, with packed orientation (radians), frequency
// (× GABOR_FREQUENCY), bandwidth, offset and scale.
fn op_pt_gabor(u: f32, v: f32, orientation: f32, f: f32, bandwidth: f32, offset: f32, scale: f32) -> f32 {
  let x = u * scale + offset * 17.31;
  let y = v * scale + offset * 31.71;
  let cx = floor(x);
  let cy = floor(y);
  var sum = 0.0;
  for (var dx = -1; dx <= 1; dx++) {
    for (var dy = -1; dy <= 1; dy++) {
      let cell = vec2<f32>(cx + f32(dx), cy + f32(dy));
      for (var k = 0; k < 8; k++) {
        let r = op_pt_random3(vec3<f32>(cell, f32(k)));
        let d = vec2<f32>(x, y) - (cell + r.xy);
        let d2 = dot(d, d);
        if (d2 > 1.0) { continue; }
        let angle = orientation + clamp(bandwidth, 0.0, 1.0) * (r.z - 0.5) * 3.14159265;
        let along = d.x * cos(angle) + d.y * sin(angle);
        let weight = select(1.0, -1.0, r.z < 0.5);
        sum += weight * exp(-3.14159265 * d2) * cos(6.2831853 * f * along);
      }
    }
  }
  return sum / 4.0;
}
fn op_pt_poisson_point(p: vec3<f32>, axis: u32, solid: bool, scale: f32) -> vec3<f32> {
  if (solid) { return p * scale; }
  return vec3<f32>(op_pt_plane(axis, p) * scale, 0.0);
}
// Pattern::value: t in 0..1 at p.
fn op_pat(mode: u32, a: vec4<f32>, b: vec4<f32>, c: vec4<f32>, p: vec3<f32>) -> f32 {
  let kind = u32(a.x + 0.5);
  let axis = u32(a.y + 0.5);
  let uv = op_pt_plane(axis, p);
  let u = uv.x;
  let v = uv.y;
  switch mode {
    case 0u: {
      let w = a.z;
      var s = op_pt_parity(u, w) * op_pt_parity(v, w);
      if (kind == 1u) { s = op_pt_parity(p.x, w) * op_pt_parity(p.y, w) * op_pt_parity(p.z, w); }
      return 0.5 + 0.5 * s;
    }
    case 1u: {
      let half = a.z * 0.5;
      let tr = a.w;
      var line = max(op_pt_cover(op_pt_line(u), half, tr), op_pt_cover(op_pt_line(v), half, tr));
      if (kind == 1u) { line = op_pt_cover(op_pt_line(u), half, tr); }
      if (kind == 2u) {
        let diagonal = op_pt_line(u - v + 0.5) / 1.41421356;
        line = max(line, op_pt_cover(diagonal, half, tr));
      }
      if (kind == 3u) { line = op_pt_cover(op_pt_hex_edge(u, v), half, tr); }
      if (kind == 4u) {
        line = max(max(op_pt_cover(op_pt_line(p.x), half, tr), op_pt_cover(op_pt_line(p.y), half, tr)), op_pt_cover(op_pt_line(p.z), half, tr));
      }
      return 1.0 - line;
    }
    case 2u: {
      var r = length(uv - floor(uv + 0.5));
      if (kind == 1u) {
        let row = floor(v + 0.5);
        let shift = 0.5 * (row - 2.0 * floor(row * 0.5));
        let us = u - shift;
        r = length(vec2<f32>(us - floor(us + 0.5), v - row));
      }
      if (kind == 2u) { r = length(p - floor(p + 0.5)); }
      return 1.0 - op_pt_cover(r, a.z * 0.5, a.w);
    }
    case 4u: {
      let octaves = clamp(u32(b.x + 0.5), 1u, 8u);
      var amplitude = 1.0;
      var frequency = 1.0;
      var first = 0.0;
      var rest = 0.0;
      var total = 0.0;
      for (var i = 0u; i < octaves; i++) {
        let cl = op_pt_cells(p * frequency, 1.0, false);
        var d = 2.0 * cl.f1;
        if (kind == 0u) { d = 1.0 - clamp(2.0 * (cl.f2 - cl.f1), 0.0, 1.0); }
        let t = 1.0 - op_pt_cover(d, a.z, a.w);
        if (i == 0u) { first = t; } else { rest += amplitude * t; }
        total += amplitude;
        amplitude *= b.z;
        frequency *= b.y;
      }
      let k = 1.0 - b.w * clamp(first, 0.0, 1.0);
      return (first + k * rest) / total;
    }
    case 5u: {
      let wave = op_pt_noise(p / max(b.x, 1e-6), 0.0);
      let r = length(uv) + a.w * a.z * wave;
      let ring = 0.5 + 0.5 * cos(6.2831853 * r / max(a.z, 1e-6));
      let grain = 0.5 + 0.5 * op_pt_noise(p / max(b.z, 1e-6), 0.0);
      let g = b.y * 0.5;
      return ring * (1.0 - g) + grain * g;
    }
    case 6u: {
      let n = clamp(u32(a.z + 0.5), 1u, 32u);
      var sum = 0.0;
      for (var i = 0u; i < n; i++) {
        var centre = vec3<f32>(0.0);
        for (var k = 0u; k < 3u; k++) {
          centre[k] = f32(op_pt_ripple_hash(i * 3u + k) >> 8u) / 16777216.0 - 0.5;
        }
        let d = length(p - centre);
        sum += cos(6.2831853 * (d / max(a.w, 1e-6) + b.x));
      }
      return 0.5 - 0.5 * sum / f32(n);
    }
    case 7u: {
      return 1.0 - op_pt_weave(u, v, a.z, a.w);
    }
    case 8u: {
      let g = op_pt_gabor(u, v, a.z, a.w, b.x, b.y, b.z);
      return 1.0 - op_pt_smoothstep(b.w, c.x, abs(g));
    }
    case 9u: {
      let solid = b.y != 0.0;
      let octaves = clamp(u32(b.z + 0.5), 1u, 8u);
      var amplitude = 1.0;
      var frequency = 1.0;
      var sum = 0.0;
      var total = 0.0;
      for (var i = 0u; i < octaves; i++) {
        let q = op_pt_poisson_point(p, axis, solid, b.x * frequency);
        let cl = op_pt_cells(q, c.z, !solid);
        var d = 2.0 * cl.f1;
        if (kind == 0u) { d = 1.0 - clamp(2.0 * (cl.f2 - cl.f1), 0.0, 1.0); }
        var t = 1.0 - op_pt_cover(d, a.z, a.w);
        if (c.y != 0.0) { t = op_pt_smoothstep(0.0, max(a.z, 1e-6), d); }
        sum += amplitude * t;
        total += amplitude;
        amplitude *= c.x;
        frequency *= b.w;
      }
      return sum / total;
    }
    default: {
      // Noise (mode 3): procedural.rs noise_value.
      let turbulence = kind == 2u;
      let octaves = clamp(u32(a.z + 0.5), 1u, 8u);
      let phase = select(0.0, b.w, b.z != 0.0);
      var amplitude = 1.0;
      var frequency = 1.0;
      var first = 0.0;
      var rest = 0.0;
      var total = 0.0;
      for (var i = 0u; i < octaves; i++) {
        var s = op_pt_noise(p * frequency, phase);
        if (turbulence) { s = abs(s); }
        if (i == 0u) { first = s; } else { rest += amplitude * s; }
        total += amplitude;
        amplitude *= b.x;
        frequency *= a.w;
      }
      let t0 = select(0.5 + 0.5 * first, 2.0 * first, turbulence);
      let k = 1.0 - b.y * clamp(t0, 0.0, 1.0);
      let sum = (first + k * rest) / max(total, 1e-12);
      if (turbulence) { return clamp(2.0 * sum, 0.0, 1.0); }
      return 0.5 + 0.5 * sum;
    }
  }
}
// Pattern::cell_random: three randoms for the cell holding p (0 for
// patterns without cells).
fn op_cell_random(mode: u32, a: vec4<f32>, b: vec4<f32>, c: vec4<f32>, p: vec3<f32>) -> vec3<f32> {
  let kind = u32(a.x + 0.5);
  let axis = u32(a.y + 0.5);
  let uv = op_pt_plane(axis, p);
  let u = uv.x;
  let v = uv.y;
  var id = vec3<f32>(0.0);
  switch mode {
    case 0u: {
      id = vec3<f32>(floor(2.0 * u), floor(2.0 * v), 0.0);
      if (kind == 1u) { id = floor(2.0 * p); }
    }
    case 1u: {
      id = vec3<f32>(floor(u + 0.5), floor(v + 0.5), 0.0);
      if (kind == 1u) { id = vec3<f32>(floor(u + 0.5), 0.0, 0.0); }
      if (kind == 2u) {
        let d = u - v + 0.5;
        id.z = select(0.0, 1.0, d - floor(d) > 0.5);
      }
      if (kind == 3u) {
        let ctr = uv - op_pt_hex_g(u, v);
        id = vec3<f32>(round(ctr.x * 2.0), round(ctr.y * 2.0 / 1.7320508), 0.0);
      }
      if (kind == 4u) { id = floor(p + 0.5); }
    }
    case 2u: {
      id = vec3<f32>(floor(u + 0.5), floor(v + 0.5), 0.0);
      if (kind == 1u) {
        let row = floor(v + 0.5);
        let shift = 0.5 * (row - 2.0 * floor(row * 0.5));
        id = vec3<f32>(floor(u - shift + 0.5), row, 0.0);
      }
      if (kind == 2u) { id = floor(p + 0.5); }
    }
    case 4u: { return op_pt_cells(p, 1.0, false).id; }
    case 9u: {
      let solid = b.y != 0.0;
      return op_pt_cells(op_pt_poisson_point(p, axis, solid, b.x), c.z, !solid).id;
    }
    default: { return vec3<f32>(0.0); }
  }
  return op_pt_random3(id);
}
