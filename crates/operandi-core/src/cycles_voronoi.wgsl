// SPDX-FileCopyrightText: 2013 Inigo Quilez
// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-FileCopyrightText: 2026 Operandi contributors
//
// SPDX-License-Identifier: MIT AND Apache-2.0
//
// WGSL twin of cycles_voronoi.rs: Cycles' Voronoi texture (Cycles commit
// 3eb35f85996baa6c1b2e19bf297b22745c85e321, src/kernel/svm/voronoi.h and the
// PCG hashes of src/util/hash.h). See that file for what is adapted from
// where. Every material module has it (the shader tree's Cellular and
// Poisson Cells patterns use it too), so it also holds the Jenkins hashes
// shader_func.wgsl uses (src/util/hash.h, Apache-2.0).

fn op_g_rot(x: u32, k: u32) -> u32 { return (x << k) | (x >> (32u - k)); }
fn op_g_hash(k: vec4<u32>, n: u32) -> u32 {
  let start = 0xdeadbeefu + (n << 2u) + 13u;
  var a = start;
  var b = start;
  var c = start;
  if (n == 1u) {
    a += k.x;
  } else if (n == 2u) {
    b += k.y;
    a += k.x;
  } else if (n == 3u) {
    c += k.z;
    b += k.y;
    a += k.x;
  } else {
    a += k.x;
    b += k.y;
    c += k.z;
    a -= c; a ^= op_g_rot(c, 4u); c += b;
    b -= a; b ^= op_g_rot(a, 6u); a += c;
    c -= b; c ^= op_g_rot(b, 8u); b += a;
    a -= c; a ^= op_g_rot(c, 16u); c += b;
    b -= a; b ^= op_g_rot(a, 19u); a += c;
    c -= b; c ^= op_g_rot(b, 4u); b += a;
    a += k.w;
  }
  c ^= b; c -= op_g_rot(b, 14u);
  a ^= c; a -= op_g_rot(c, 11u);
  b ^= a; b -= op_g_rot(a, 25u);
  c ^= b; c -= op_g_rot(b, 16u);
  a ^= c; a -= op_g_rot(c, 4u);
  b ^= a; b -= op_g_rot(a, 14u);
  c ^= b; c -= op_g_rot(b, 24u);
  return c;
}
fn op_g_hashf(k: vec4<f32>, n: u32) -> f32 {
  return f32(op_g_hash(bitcast<vec4<u32>>(k), n)) * (1.0 / 4294967296.0);
}

struct OpVParams {
  dims: u32, feature: u32, metric: u32, normalize: bool,
  scale: f32, detail: f32, rough: f32, lac: f32, smo: f32, expo: f32, rnd: f32,
};
struct OpVOctave { d: f32, c: vec3<f32>, p: vec4<f32> };

fn op_v_pcg(k: vec4<i32>, n: u32) -> vec4<i32> {
  let m = 1664525;
  var v = k * m + 1013904223;
  for (var s = 0; s < 2; s++) {
    if (n == 2u) {
      v.x += v.y * m;
      v.y += v.x * m;
    } else if (n == 3u) {
      v.x += v.y * v.z;
      v.y += v.z * v.x;
      v.z += v.x * v.y;
    } else {
      v.x += v.y * v.w;
      v.y += v.z * v.x;
      v.z += v.x * v.y;
      v.w += v.y * v.z;
    }
    if (s == 0) {
      var t = v ^ (v >> vec4<u32>(16u));
      if (n < 4u) { t.w = v.w; }
      if (n < 3u) { t.z = v.z; }
      v = t;
    }
  }
  return v & vec4<i32>(0x7FFFFFFF);
}
fn op_v_unit(k: vec4<i32>, n: u32) -> vec4<f32> {
  return vec4<f32>(op_v_pcg(k, n)) * (1.0 / 2147483648.0);
}
fn op_v_point(c: vec4<i32>, n: u32) -> vec4<f32> {
  var h = op_v_unit(c, n);
  if (n < 4u) { h.w = 0.0; }
  if (n < 3u) { h.z = 0.0; }
  return h;
}
fn op_v_color(c: vec4<i32>, n: u32) -> vec3<f32> {
  if (n == 2u) { return op_v_unit(vec4<i32>(c.x, c.y, 0, 0), 3u).xyz; }
  return op_v_unit(c, n).xyz;
}
fn op_v_mask(v: vec4<f32>, n: u32) -> vec4<f32> {
  var o = v;
  if (n < 4u) { o.w = 0.0; }
  if (n < 3u) { o.z = 0.0; }
  if (n < 2u) { o.y = 0.0; }
  return o;
}
fn op_v_distance(pr: OpVParams, a: vec4<f32>, b: vec4<f32>, n: u32) -> f32 {
  let d = abs(op_v_mask(a - b, n));
  switch pr.metric {
    case 0u: { return length(d); }
    case 1u: { return d.x + d.y + d.z + d.w; }
    case 2u: { return max(max(d.x, d.y), max(d.z, d.w)); }
    default: {
      var s = 0.0;
      for (var k = 0u; k < n; k++) { s += pow(d[k], pr.expo); }
      return pow(s, 1.0 / pr.expo);
    }
  }
}
fn op_v_bound(pr: OpVParams, a: vec4<f32>, b: vec4<f32>, n: u32) -> f32 {
  let d = abs(op_v_mask(a - b, n));
  if (pr.metric == 0u) { return dot(d, d); }
  if (pr.metric == 3u) {
    var s = 0.0;
    for (var k = 0u; k < n; k++) { s += pow(d[k], pr.expo); }
    return s;
  }
  return op_v_distance(pr, a, b, n);
}
// Cell offset number i of a (2r+1)^n search, x fastest.
fn op_v_offset(i0: u32, n: u32, r: i32) -> vec4<i32> {
  let side = u32(2 * r + 1);
  var i = i0;
  var o = vec4<i32>(0);
  for (var k = 0u; k < n; k++) {
    // Component k by a lane mask, not o[k]: FXC (DirectX 12) can't write
    // a vector component picked by a loop counter.
    o = select(o, vec4<i32>(i32(i % side) - r), vec4<u32>(0u, 1u, 2u, 3u) == vec4<u32>(k));
    i = i / side;
  }
  return o;
}
fn op_v_count(n: u32, r: i32) -> u32 {
  let side = u32(2 * r + 1);
  var c = 1u;
  for (var k = 0u; k < n; k++) { c *= side; }
  return c;
}
fn op_v_smoothstep(x: f32) -> f32 {
  if (x < 0.0) { return 0.0; }
  if (x >= 1.0) { return 1.0; }
  return (3.0 - 2.0 * x) * x * x;
}
fn op_v_hash1(x: f32) -> f32 { return op_g_hashf(vec4<f32>(x, 0.0, 0.0, 0.0), 1u); }
fn op_v_hash1_3(x: f32) -> vec3<f32> {
  return vec3<f32>(op_v_hash1(x), op_g_hashf(vec4<f32>(x, 1.0, 0.0, 0.0), 2u), op_g_hashf(vec4<f32>(x, 2.0, 0.0, 0.0), 2u));
}
fn op_v_octave1(pr: OpVParams, x: f32, feature: u32) -> OpVOctave {
  let cell = floor(x);
  let local = x - cell;
  var o: OpVOctave;
  if (feature == 2u) {
    var d = 0.0; var col = vec3<f32>(0.0); var pos = 0.0; var h = -1.0;
    for (var i = -2; i <= 2; i++) {
      let off = f32(i);
      let p = off + op_v_hash1(cell + off) * pr.rnd;
      let dp = abs(local - p);
      if (h == -1.0) { h = 1.0; } else { h = op_v_smoothstep(0.5 + 0.5 * (d - dp) / pr.smo); }
      var corr = pr.smo * h * (1.0 - h);
      d = mix(d, dp, h) - corr;
      corr = corr / (1.0 + 3.0 * pr.smo);
      col = mix(col, op_v_hash1_3(cell + off), vec3<f32>(h)) - corr;
      pos = mix(pos, p, h) - corr;
    }
    o.d = d; o.c = col; o.p = vec4<f32>(0.0, 0.0, 0.0, cell + pos);
    return o;
  }
  var d1 = 3.4e38; var d2 = 3.4e38; var o1 = 0.0; var o2 = 0.0; var p1 = 0.0; var p2 = 0.0;
  for (var i = -1; i <= 1; i++) {
    let off = f32(i);
    let p = off + op_v_hash1(cell + off) * pr.rnd;
    let dp = abs(local - p);
    if (dp < d1) { d2 = d1; d1 = dp; o2 = o1; o1 = off; p2 = p1; p1 = p; }
    else if (dp < d2) { d2 = dp; o2 = off; p2 = p; }
  }
  if (feature == 1u) {
    o.d = d2; o.c = op_v_hash1_3(cell + o2); o.p = vec4<f32>(0.0, 0.0, 0.0, p2 + cell);
  } else {
    o.d = d1; o.c = op_v_hash1_3(cell + o1); o.p = vec4<f32>(0.0, 0.0, 0.0, p1 + cell);
  }
  return o;
}
fn op_v_octave(pr: OpVParams, coord: vec4<f32>, n: u32, feature: u32) -> OpVOctave {
  if (n == 1u) { return op_v_octave1(pr, coord.w, feature); }
  let cell_f = floor(coord);
  let local = coord - cell_f;
  let cell = vec4<i32>(cell_f);
  var o: OpVOctave;
  if (feature == 2u) {
    var d = 0.0; var col = vec3<f32>(0.0); var pos = vec4<f32>(0.0); var h = -1.0;
    let count = op_v_count(n, 2);
    for (var i = 0u; i < count; i++) {
      let off = op_v_offset(i, n, 2);
      let c = cell + off;
      let p = vec4<f32>(off) + op_v_point(c, n) * pr.rnd;
      let dp = op_v_distance(pr, p, local, n);
      if (h == -1.0) { h = 1.0; } else { h = op_v_smoothstep(0.5 + 0.5 * (d - dp) / pr.smo); }
      var corr = pr.smo * h * (1.0 - h);
      d = mix(d, dp, h) - corr;
      corr = corr / (1.0 + 3.0 * pr.smo);
      col = mix(col, op_v_color(c, n), vec3<f32>(h)) - corr;
      pos = mix(pos, p, vec4<f32>(h)) - corr;
    }
    o.d = d; o.c = col; o.p = op_v_mask(pos + cell_f, n);
    return o;
  }
  var d1 = 3.4e38; var d2 = 3.4e38;
  var c1 = cell; var c2 = cell;
  var p1 = vec4<f32>(0.0); var p2 = vec4<f32>(0.0);
  for (var i = 0u; i < op_v_count(n, 1); i++) {
    let off = op_v_offset(i, n, 1);
    let c = cell + off;
    let p = vec4<f32>(off) + op_v_point(c, n) * pr.rnd;
    if (feature == 1u) {
      let dp = op_v_distance(pr, p, local, n);
      if (dp < d1) { d2 = d1; d1 = dp; c2 = c1; c1 = c; p2 = p1; p1 = p; }
      else if (dp < d2) { d2 = dp; c2 = c; p2 = p; }
    } else {
      let dp = op_v_bound(pr, p, local, n);
      if (dp < d1) { d1 = dp; c1 = c; p1 = p; }
    }
  }
  if (feature == 1u) {
    o.d = d2; o.c = op_v_color(c2, n); o.p = op_v_mask(p2 + cell_f, n);
  } else {
    o.d = op_v_distance(pr, p1, local, n); o.c = op_v_color(c1, n); o.p = op_v_mask(p1 + cell_f, n);
  }
  return o;
}
fn op_v_edge(pr: OpVParams, coord: vec4<f32>, n: u32) -> f32 {
  if (n == 1u) {
    let x = coord.w;
    let cell = floor(x);
    let local = x - cell;
    let mid = op_v_hash1(cell) * pr.rnd;
    let left = -1.0 + op_v_hash1(cell - 1.0) * pr.rnd;
    let right = 1.0 + op_v_hash1(cell + 1.0) * pr.rnd;
    return min(abs((mid + left) / 2.0 - local), abs((mid + right) / 2.0 - local));
  }
  let cell_f = floor(coord);
  let local = coord - cell_f;
  let cell = vec4<i32>(cell_f);
  let count = op_v_count(n, 1);
  var best = 3.4e38;
  var closest = vec4<f32>(0.0);
  for (var i = 0u; i < count; i++) {
    let off = op_v_offset(i, n, 1);
    let v = op_v_mask(vec4<f32>(off) + op_v_point(cell + off, n) * pr.rnd - local, n);
    let d = dot(v, v);
    if (d < best) { best = d; closest = v; }
  }
  var edge = 3.4e38;
  for (var i = 0u; i < count; i++) {
    let off = op_v_offset(i, n, 1);
    let v = op_v_mask(vec4<f32>(off) + op_v_point(cell + off, n) * pr.rnd - local, n);
    let perp = v - closest;
    let l2 = dot(perp, perp);
    if (l2 > 0.0001) { edge = min(edge, dot((closest + v) / 2.0, perp / sqrt(l2))); }
  }
  return edge;
}
fn op_v_n_sphere(pr: OpVParams, coord: vec4<f32>, n: u32) -> f32 {
  if (n == 1u) {
    let x = coord.w;
    let cell = floor(x);
    let local = x - cell;
    var best = 3.4e38; var cp = 0.0; var co = 0.0;
    for (var i = -1; i <= 1; i++) {
      let off = f32(i);
      let p = off + op_v_hash1(cell + off) * pr.rnd;
      let d = abs(p - local);
      if (d < best) { best = d; cp = p; co = off; }
    }
    best = 3.4e38;
    var cc = 0.0;
    for (var i = -1; i <= 1; i += 2) {
      let off = f32(i) + co;
      let p = off + op_v_hash1(cell + off) * pr.rnd;
      let d = abs(cp - p);
      if (d < best) { best = d; cc = p; }
    }
    return abs(cc - cp) / 2.0;
  }
  let cell_f = floor(coord);
  let local = coord - cell_f;
  let cell = vec4<i32>(cell_f);
  let count = op_v_count(n, 1);
  var best = 3.4e38;
  var cp = vec4<f32>(0.0);
  var co = vec4<i32>(0);
  for (var i = 0u; i < count; i++) {
    let off = op_v_offset(i, n, 1);
    let p = vec4<f32>(off) + op_v_point(cell + off, n) * pr.rnd;
    let d = op_v_mask(p - local, n);
    if (dot(d, d) < best) { best = dot(d, d); cp = p; co = off; }
  }
  best = 3.4e38;
  var cc = vec4<f32>(0.0);
  for (var i = 0u; i < count; i++) {
    let off = op_v_offset(i, n, 1);
    if (all(off == vec4<i32>(0))) { continue; }
    let o2 = off + co;
    let p = vec4<f32>(o2) + op_v_point(cell + o2, n) * pr.rnd;
    let d = op_v_mask(cp - p, n);
    if (dot(d, d) < best) { best = dot(d, d); cc = p; }
  }
  return length(op_v_mask(cc - cp, n)) / 2.0;
}
fn op_v_fractal(pr: OpVParams, coord: vec4<f32>, n: u32, max_distance: f32) -> OpVOctave {
  var amp = 1.0; var max_amp = 0.0; var scale = 1.0;
  var o: OpVOctave;
  o.d = 0.0; o.c = vec3<f32>(0.0); o.p = vec4<f32>(0.0);
  let zero_input = pr.detail == 0.0 || pr.rough == 0.0;
  var feature = pr.feature;
  if (feature == 2u && pr.smo == 0.0) { feature = 0u; }
  for (var i = 0; f32(i) <= ceil(pr.detail); i++) {
    let oct = op_v_octave(pr, coord * scale, n, feature);
    if (zero_input) { max_amp = 1.0; o = oct; break; }
    if (f32(i) <= pr.detail) {
      max_amp += amp;
      o.d += oct.d * amp;
      o.c += oct.c * amp;
      o.p = mix(o.p, oct.p / scale, vec4<f32>(amp));
      scale *= pr.lac;
      amp *= pr.rough;
    } else {
      let rem = pr.detail - floor(pr.detail);
      if (rem != 0.0) {
        max_amp = mix(max_amp, max_amp + amp, rem);
        o.d = mix(o.d, o.d + oct.d * amp, rem);
        o.c = mix(o.c, o.c + oct.c * amp, vec3<f32>(rem));
        o.p = mix(o.p, mix(o.p, oct.p / scale, vec4<f32>(amp)), vec4<f32>(rem));
      }
    }
  }
  if (pr.normalize) {
    o.d = o.d / (max_amp * max_distance);
    o.c = o.c / max_amp;
  }
  if (pr.scale != 0.0) { o.p = o.p / pr.scale; } else { o.p = vec4<f32>(0.0); }
  return o;
}
fn op_v_fractal_edge(pr: OpVParams, coord: vec4<f32>, n: u32, max_distance: f32) -> f32 {
  var amp = 1.0; var max_amp = max_distance; var scale = 1.0; var dist = 8.0;
  let zero_input = pr.detail == 0.0 || pr.rough == 0.0;
  for (var i = 0; f32(i) <= ceil(pr.detail); i++) {
    let od = op_v_edge(pr, coord * scale, n);
    if (zero_input) { dist = od; break; }
    if (f32(i) <= pr.detail) {
      max_amp = mix(max_amp, max_distance / scale, amp);
      dist = mix(dist, min(dist, od / scale), amp);
      scale *= pr.lac;
      amp *= pr.rough;
    } else {
      let rem = pr.detail - floor(pr.detail);
      if (rem != 0.0) {
        let la = mix(max_amp, max_distance / scale, amp);
        max_amp = mix(max_amp, la, rem);
        let ld = mix(dist, min(dist, od / scale), amp);
        dist = mix(dist, min(dist, ld), rem);
      }
    }
  }
  if (pr.normalize) { dist = dist / max_amp; }
  return dist;
}
// The node: one output (0 Distance, 1 Color, 2 Position, 3 W, 4 Radius).
fn op_g_voronoi(dims: u32, feature: u32, metric: u32, normalize: bool, which: u32,
                vv: vec3<f32>, wv: vec3<f32>, scale: vec3<f32>, detail: vec3<f32>, rough: vec3<f32>,
                lac: vec3<f32>, smo: vec3<f32>, expo: vec3<f32>, rnd: vec3<f32>) -> vec3<f32> {
  let n = clamp(dims, 1u, 4u);
  let pr = OpVParams(n, feature, metric, normalize, scale.x, clamp(detail.x, 0.0, 15.0),
                     clamp(rough.x, 0.0, 1.0), lac.x, clamp(smo.x / 2.0, 0.0, 0.5), expo.x,
                     clamp(rnd.x, 0.0, 1.0));
  let s = scale.x;
  var coord = vec4<f32>(vv * s, wv.x * s);
  if (n == 1u) { coord = vec4<f32>(0.0, 0.0, 0.0, wv.x * s); }
  if (n == 2u) { coord.z = 0.0; coord.w = 0.0; }
  if (n == 3u) { coord.w = 0.0; }
  let reach = 0.5 + 0.5 * pr.rnd;
  if (feature == 3u) {
    if (which != 0u) { return vec3<f32>(0.0); }
    return vec3<f32>(op_v_fractal_edge(pr, coord, n, reach));
  }
  if (feature == 4u) {
    if (which != 4u) { return vec3<f32>(0.0); }
    return vec3<f32>(op_v_n_sphere(pr, coord, n));
  }
  if (which == 4u) { return vec3<f32>(0.0); }
  var k = 1.0;
  if (feature == 1u) { k = 2.0; }
  var max_distance = reach * k;
  if (n > 1u) { max_distance = op_v_distance(pr, vec4<f32>(0.0), vec4<f32>(reach), n) * k; }
  let o = op_v_fractal(pr, coord, n, max_distance);
  switch which {
    case 0u: { return vec3<f32>(o.d); }
    case 1u: { return o.c; }
    case 2u: { return o.p.xyz; }
    default: { return vec3<f32>(o.p.w); }
  }
}
