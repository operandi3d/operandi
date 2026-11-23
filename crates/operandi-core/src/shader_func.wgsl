// SPDX-FileCopyrightText: 2009-2010 Sony Pictures Imageworks Inc., et al.
// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-FileCopyrightText: 2026 Operandi contributors
//
// SPDX-License-Identifier: Apache-2.0 AND BSD-3-Clause
//
// WGSL twin of shader_func.rs and cycles_noise.rs: Cycles' node sums and
// procedural textures (Cycles commit 3eb35f85996baa6c1b2e19bf297b22745c85e321;
// Perlin noise BSD-3-Clause via Open Shading Language, the rest Apache-2.0).
// See those files for what each part is adapted from.

fn op_g_safe_div(a: f32, b: f32) -> f32 { if (b != 0.0) { return a / b; } return 0.0; }
fn op_g_sign(a: f32) -> f32 { if (a == 0.0) { return 0.0; } if (a < 0.0) { return -1.0; } return 1.0; }
fn op_g_safe_pow(a: f32, b: f32) -> f32 {
  if (a < 0.0 && b != trunc(b)) { return 0.0; }
  if (b == 0.0) { return 1.0; }
  if (a == 0.0) { return 0.0; }
  if (a < 0.0) {
    let p = pow(-a, b);
    if ((-b) % 2.0 == 0.0) { return p; }
    return -p;
  }
  return pow(a, b);
}
fn op_g_smooth_min(a: f32, b: f32, k: f32) -> f32 {
  if (k != 0.0) {
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * h * k * (1.0 / 6.0);
  }
  return min(a, b);
}
fn op_g_wrap(a: f32, b: f32, c: f32) -> f32 {
  let r = b - c;
  if (r != 0.0) { return a - r * floor((a - c) / r); }
  return c;
}
fn op_g_math1(op: u32, a: f32, b: f32, c: f32) -> f32 {
  switch op {
    case 0u: { return a + b; }
    case 1u: { return a - b; }
    case 2u: { return a * b; }
    case 3u: { return op_g_safe_div(a, b); }
    case 4u: { return a * b + c; }
    case 5u: { return op_g_safe_pow(a, b); }
    case 6u: {
      if (a <= 0.0 || b <= 0.0) { return 0.0; }
      return op_g_safe_div(log(a), log(b));
    }
    case 7u: { return sqrt(max(a, 0.0)); }
    case 8u: { if (a > 0.0) { return 1.0 / sqrt(a); } return 0.0; }
    case 9u: { return abs(a); }
    case 10u: { return exp(a); }
    case 11u: { return min(a, b); }
    case 12u: { return max(a, b); }
    case 13u: { return select(0.0, 1.0, a < b); }
    case 14u: { return select(0.0, 1.0, a > b); }
    case 15u: { return op_g_sign(a); }
    case 16u: { return select(0.0, 1.0, a == b || abs(a - b) <= max(c, 1.1920929e-7)); }
    case 17u: { return op_g_smooth_min(a, b, c); }
    case 18u: { return -op_g_smooth_min(-a, -b, c); }
    case 19u: { return floor(a + 0.5); }
    case 20u: { return floor(a); }
    case 21u: { return ceil(a); }
    case 22u: { return trunc(a); }
    case 23u: { return a - floor(a); }
    case 24u: { if (b != 0.0) { return a % b; } return 0.0; }
    case 25u: { if (b != 0.0) { return a - floor(a / b) * b; } return 0.0; }
    case 26u: { return op_g_wrap(a, b, c); }
    case 27u: { return floor(op_g_safe_div(a, b)) * b; }
    case 28u: {
      if (b != 0.0) { let f = (a - b) / (b * 2.0); return abs((f - floor(f)) * b * 2.0 - b); }
      return 0.0;
    }
    case 29u: { return sin(a); }
    case 30u: { return cos(a); }
    case 31u: { return tan(a); }
    case 32u: { return asin(clamp(a, -1.0, 1.0)); }
    case 33u: { return acos(clamp(a, -1.0, 1.0)); }
    case 34u: { return atan(a); }
    case 35u: { if (a == 0.0 && b == 0.0) { return 0.0; } return atan2(a, b); }
    case 36u: { return sinh(a); }
    case 37u: { return cosh(a); }
    case 38u: { return tanh(a); }
    case 39u: { return a * (3.141592653589793 / 180.0); }
    case 40u: { return a * (180.0 / 3.141592653589793); }
    default: { return 0.0; }
  }
}
fn op_g_math(op: u32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, sat: bool) -> vec3<f32> {
  var v = vec3<f32>(op_g_math1(op, a.x, b.x, c.x), op_g_math1(op, a.y, b.y, c.y), op_g_math1(op, a.z, b.z, c.z));
  if (sat) { v = clamp(v, vec3<f32>(0.0), vec3<f32>(1.0)); }
  return v;
}
fn op_g_safe_normalize(v: vec3<f32>) -> vec3<f32> {
  let l = length(v);
  if (l != 0.0) { return v / l; }
  return vec3<f32>(0.0);
}
fn op_g_vmath(op: u32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, sv: vec3<f32>) -> vec3<f32> {
  let s = sv.x;
  switch op {
    case 0u: { return a + b; }
    case 1u: { return a - b; }
    case 2u: { return a * b; }
    case 3u: { return vec3<f32>(op_g_safe_div(a.x, b.x), op_g_safe_div(a.y, b.y), op_g_safe_div(a.z, b.z)); }
    case 4u: { return cross(a, b); }
    case 5u: {
      let l2 = dot(b, b);
      if (l2 != 0.0) { return (dot(a, b) / l2) * b; }
      return vec3<f32>(0.0);
    }
    case 6u: { let n = op_g_safe_normalize(b); return a - 2.0 * dot(n, a) * n; }
    case 7u: {
      let n = op_g_safe_normalize(b);
      let ni = dot(n, a);
      let k = 1.0 - s * s * (1.0 - ni * ni);
      if (k < 0.0) { return vec3<f32>(0.0); }
      return s * a - (s * ni + sqrt(k)) * n;
    }
    case 8u: { if (dot(c, b) < 0.0) { return a; } return -a; }
    case 9u: { return a * b + c; }
    case 10u: { return vec3<f32>(dot(a, b)); }
    case 11u: { return vec3<f32>(distance(a, b)); }
    case 12u: { return vec3<f32>(length(a)); }
    case 13u: { return a * s; }
    case 14u: { return op_g_safe_normalize(a); }
    case 15u: { return vec3<f32>(floor(op_g_safe_div(a.x, b.x)) * b.x, floor(op_g_safe_div(a.y, b.y)) * b.y, floor(op_g_safe_div(a.z, b.z)) * b.z); }
    case 16u: { return floor(a + 0.5); }
    case 17u: { return floor(a); }
    case 18u: { return ceil(a); }
    case 19u: { return vec3<f32>(op_g_math1(24u, a.x, b.x, 0.0), op_g_math1(24u, a.y, b.y, 0.0), op_g_math1(24u, a.z, b.z, 0.0)); }
    case 20u: { return vec3<f32>(op_g_wrap(a.x, b.x, c.x), op_g_wrap(a.y, b.y, c.y), op_g_wrap(a.z, b.z, c.z)); }
    case 21u: { return a - floor(a); }
    case 22u: { return abs(a); }
    case 23u: { return vec3<f32>(op_g_safe_pow(a.x, b.x), op_g_safe_pow(a.y, b.y), op_g_safe_pow(a.z, b.z)); }
    case 24u: { return vec3<f32>(op_g_sign(a.x), op_g_sign(a.y), op_g_sign(a.z)); }
    case 25u: { return min(a, b); }
    case 26u: { return max(a, b); }
    case 27u: { return sin(a); }
    case 28u: { return cos(a); }
    case 29u: { return tan(a); }
    default: { return vec3<f32>(0.0); }
  }
}
fn op_g_mix_more(m: u32, fv: vec3<f32>, a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
  let t = fv.x;
  switch m {
    case 0u: {
      let h2 = op_rgb_to_hsv(b);
      if (h2.y != 0.0) { var h = op_rgb_to_hsv(a); h.x = h2.x; return mix(a, op_hsv_to_rgb(h), t); }
      return a;
    }
    case 1u: {
      var h = op_rgb_to_hsv(a);
      if (h.y != 0.0) { let h2 = op_rgb_to_hsv(b); h.y = (1.0 - t) * h.y + t * h2.y; return op_hsv_to_rgb(h); }
      return a;
    }
    case 2u: {
      var h = op_rgb_to_hsv(a);
      let h2 = op_rgb_to_hsv(b);
      h.z = (1.0 - t) * h.z + t * h2.z;
      return op_hsv_to_rgb(h);
    }
    case 3u: {
      let h2 = op_rgb_to_hsv(b);
      if (h2.y != 0.0) { var h = op_rgb_to_hsv(a); h.x = h2.x; h.y = h2.y; return mix(a, op_hsv_to_rgb(h), t); }
      return a;
    }
    case 4u: { return a + t * (2.0 * b - vec3<f32>(1.0)); }
    default: { return max(mix(a, a + b - 2.0 * a * b, t), vec3<f32>(0.0)); }
  }
}
fn op_g_hsv_node(hue: vec3<f32>, sat: vec3<f32>, val: vec3<f32>, fac: vec3<f32>, c: vec3<f32>) -> vec3<f32> {
  var h = op_rgb_to_hsv(c);
  let x = h.x + hue.x + 0.5;
  h.x = x - floor(x);
  h.y = clamp(h.y * sat.x, 0.0, 1.0);
  h.z = h.z * val.x;
  let o = op_hsv_to_rgb(h);
  return max(fac.x * o + (1.0 - fac.x) * c, vec3<f32>(0.0));
}
fn op_g_rgb_to_hsl(c: vec3<f32>) -> vec3<f32> {
  let cmax = max(c.x, max(c.y, c.z));
  let cmin = min(c.x, min(c.y, c.z));
  let l = min(1.0, (cmax + cmin) / 2.0);
  var h = 0.0;
  var s = 0.0;
  if (cmax != cmin) {
    let d = cmax - cmin;
    if (l > 0.5) { s = d / (2.0 - cmax - cmin); } else { s = d / (cmax + cmin); }
    if (cmax == c.x) {
      h = (c.y - c.z) / d + select(0.0, 6.0, c.y < c.z);
    } else if (cmax == c.y) {
      h = (c.z - c.x) / d + 2.0;
    } else {
      h = (c.x - c.y) / d + 4.0;
    }
  }
  return vec3<f32>(h / 6.0, s, l);
}
fn op_g_hsl_to_rgb(c: vec3<f32>) -> vec3<f32> {
  let nr = clamp(abs(c.x * 6.0 - 3.0) - 1.0, 0.0, 1.0);
  let ng = clamp(2.0 - abs(c.x * 6.0 - 2.0), 0.0, 1.0);
  let nb = clamp(2.0 - abs(c.x * 6.0 - 4.0), 0.0, 1.0);
  let chroma = (1.0 - abs(2.0 * c.z - 1.0)) * c.y;
  return (vec3<f32>(nr, ng, nb) - 0.5) * chroma + c.z;
}
fn op_g_smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
  if (x < e0) { return 0.0; }
  if (x >= e1) { return 1.0; }
  let t = (x - e0) / (e1 - e0);
  return (3.0 - 2.0 * t) * t * t;
}
fn op_g_smootherstep(e0: f32, e1: f32, x: f32) -> f32 {
  let t = clamp(op_g_safe_div(x - e0, e1 - e0), 0.0, 1.0);
  return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}
fn op_g_map_range(kind: u32, clamped: bool, v: vec3<f32>, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>, st: vec3<f32>) -> vec3<f32> {
  let x = v.x;
  let fmin = a.x;
  let fmax = b.x;
  let tmin = c.x;
  let tmax = d.x;
  let steps = st.x;
  if (fmax == fmin) { return vec3<f32>(0.0); }
  var f = 0.0;
  switch kind {
    case 0u: { f = (x - fmin) / (fmax - fmin); }
    case 1u: {
      f = (x - fmin) / (fmax - fmin);
      if (steps > 0.0) { f = floor(f * (steps + 1.0)) / steps; } else { f = 0.0; }
    }
    case 2u: {
      if (fmin > fmax) { f = 1.0 - op_g_smoothstep(fmax, fmin, x); } else { f = op_g_smoothstep(fmin, fmax, x); }
    }
    default: {
      if (fmin > fmax) { f = 1.0 - op_g_smootherstep(fmax, fmin, x); } else { f = op_g_smootherstep(fmin, fmax, x); }
    }
  }
  var o = tmin + f * (tmax - tmin);
  if (clamped && kind < 2u) { o = clamp(o, min(tmin, tmax), max(tmin, tmax)); }
  return vec3<f32>(o);
}
fn op_g_mapping(kind: u32, v: vec3<f32>, loc: vec3<f32>, rot: vec3<f32>, sc: vec3<f32>) -> vec3<f32> {
  let sx = sin(rot.x); let cx = cos(rot.x);
  let sy = sin(rot.y); let cy = cos(rot.y);
  let sz = sin(rot.z); let cz = cos(rot.z);
  let r0 = vec3<f32>(cy * cz, sy * sx * cz - cx * sz, sy * cx * cz + sx * sz);
  let r1 = vec3<f32>(cy * sz, sy * sx * sz + cx * cz, sy * cx * sz - sx * cz);
  let r2 = vec3<f32>(-sy, cy * sx, cy * cx);
  switch kind {
    case 0u: { let p = v * sc; return vec3<f32>(dot(r0, p), dot(r1, p), dot(r2, p)) + loc; }
    case 1u: {
      let p = v - loc;
      let q = r0 * p.x + r1 * p.y + r2 * p.z;
      return vec3<f32>(op_g_safe_div(q.x, sc.x), op_g_safe_div(q.y, sc.y), op_g_safe_div(q.z, sc.z));
    }
    case 2u: { let p = v * sc; return vec3<f32>(dot(r0, p), dot(r1, p), dot(r2, p)); }
    default: {
      let p = vec3<f32>(op_g_safe_div(v.x, sc.x), op_g_safe_div(v.y, sc.y), op_g_safe_div(v.z, sc.z));
      return op_g_safe_normalize(vec3<f32>(dot(r0, p), dot(r1, p), dot(r2, p)));
    }
  }
}
fn op_g_fresnel_cos(cosi: f32, eta: f32) -> f32 {
  let c = abs(cosi);
  var g = eta * eta - 1.0 + c * c;
  if (g > 0.0) {
    g = sqrt(g);
    let a = (g - c) / (g + c);
    let b = (c * (g + c) - 1.0) / (c * (g - c) + 1.0);
    return 0.5 * a * a * (1.0 + b * b);
  }
  return 1.0;
}
fn op_g_fresnel(ior: vec3<f32>, inc: vec3<f32>) -> vec3<f32> {
  return vec3<f32>(op_g_fresnel_cos(cos(radians(inc.x)), max(ior.x, 1e-5)));
}
fn op_g_layer_weight(facing: bool, blend: vec3<f32>, inc: vec3<f32>) -> vec3<f32> {
  let c = cos(radians(inc.x));
  if (!facing) {
    let eta = max(1.0 - blend.x, 1e-5);
    return vec3<f32>(op_g_fresnel_cos(c, 1.0 / eta));
  }
  var f = abs(c);
  if (blend.x != 0.5) {
    var b = clamp(blend.x, 0.0, 1.0 - 1e-5);
    if (b < 0.5) { b = 2.0 * b; } else { b = 0.5 / (1.0 - b); }
    f = pow(f, b);
  }
  return vec3<f32>(1.0 - f);
}
fn op_g_srgb1(c: f32) -> f32 {
  if (c < 0.0031308) { if (c < 0.0) { return 0.0; } return c * 12.92; }
  return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}
fn op_g_srgb_encode(c: vec3<f32>) -> vec3<f32> {
  return vec3<f32>(op_g_srgb1(c.x), op_g_srgb1(c.y), op_g_srgb1(c.z));
}
fn op_g_gamma(c: vec3<f32>, g: vec3<f32>) -> vec3<f32> {
  if (g.x == 0.0) { return vec3<f32>(1.0); }
  return vec3<f32>(
    select(c.x, pow(c.x, g.x), c.x > 0.0),
    select(c.y, pow(c.y, g.x), c.y > 0.0),
    select(c.z, pow(c.z, g.x), c.z > 0.0));
}

// ---- Hashes and noise (cycles_noise.rs) ----

// (The hashes, op_g_hash / op_g_hashf, are in cycles_voronoi.wgsl, which
// every module has.)
fn op_g_fade(t: f32) -> f32 { return t * t * t * (t * (t * 6.0 - 15.0) + 10.0); }
fn op_g_neg(v: f32, c: u32) -> f32 { if (c != 0u) { return -v; } return v; }
fn op_g_grad(dims: u32, hash: u32, f: vec4<f32>) -> f32 {
  if (dims == 1u) {
    let h = hash & 15u;
    return op_g_neg(1.0 + f32(h & 7u), h & 8u) * f.x;
  }
  if (dims == 2u) {
    let h = hash & 7u;
    let u = select(f.y, f.x, h < 4u);
    let v = 2.0 * select(f.x, f.y, h < 4u);
    return op_g_neg(u, h & 1u) + op_g_neg(v, h & 2u);
  }
  if (dims == 3u) {
    let h = hash & 15u;
    let u = select(f.y, f.x, h < 8u);
    let vt = select(f.z, f.x, h == 12u || h == 14u);
    let v = select(vt, f.y, h < 4u);
    return op_g_neg(u, h & 1u) + op_g_neg(v, h & 2u);
  }
  let h = hash & 31u;
  let u = select(f.y, f.x, h < 24u);
  let v = select(f.z, f.y, h < 16u);
  let s = select(f.w, f.z, h < 8u);
  return op_g_neg(u, h & 1u) + op_g_neg(v, h & 2u) + op_g_neg(s, h & 4u);
}
fn op_g_perlin(dims: u32, p: vec4<f32>) -> f32 {
  let fl = floor(p);
  let cell = vec4<i32>(fl);
  let f = p - fl;
  let corners = 1u << dims;
  var v: array<f32, 16>;
  for (var i = 0u; i < corners; i++) {
    let bits = vec4<u32>(i & 1u, (i >> 1u) & 1u, (i >> 2u) & 1u, (i >> 3u) & 1u);
    let key = bitcast<vec4<u32>>(cell + vec4<i32>(bits));
    let d = f - vec4<f32>(bits);
    v[i] = op_g_grad(dims, op_g_hash(key, dims), d);
  }
  var n = corners;
  for (var k = 0u; k < dims; k++) {
    let t = op_g_fade(f[k]);
    n = n / 2u;
    for (var i = 0u; i < n; i++) {
      let a = v[2u * i];
      let b = v[2u * i + 1u];
      if (dims == 1u || (dims == 4u && k == 3u)) {
        v[i] = a + t * (b - a);
      } else {
        v[i] = (1.0 - t) * a + t * b;
      }
    }
  }
  return v[0];
}
fn op_g_snoise(dims: u32, p: vec4<f32>) -> f32 {
  let corr = select(vec4<f32>(0.0), vec4<f32>(0.5), abs(p) >= vec4<f32>(1000000.0));
  var q = p % vec4<f32>(100000.0) + corr;
  if (dims < 4u) { q.w = 0.0; }
  if (dims < 3u) { q.z = 0.0; }
  if (dims < 2u) { q.y = 0.0; }
  var scale = 0.8344;
  if (dims == 1u) { scale = 0.25; } else if (dims == 2u) { scale = 0.6616; } else if (dims == 3u) { scale = 0.9820; }
  return scale * op_g_perlin(dims, q);
}
struct OpGFractal { detail: f32, rough: f32, lac: f32, offset: f32, gain: f32 };
fn op_g_fbm(dims: u32, p: vec4<f32>, fr: OpGFractal, normalize: bool) -> f32 {
  var fscale = 1.0;
  var amp = 1.0;
  var maxamp = 0.0;
  var sum = 0.0;
  for (var i = 0; i <= i32(fr.detail); i++) {
    let t = op_g_snoise(dims, p * fscale);
    sum += t * amp;
    maxamp += amp;
    amp *= fr.rough;
    fscale *= fr.lac;
  }
  let rmd = fr.detail - floor(fr.detail);
  if (rmd != 0.0) {
    let t = op_g_snoise(dims, p * fscale);
    let sum2 = sum + t * amp;
    if (normalize) {
      let a = 0.5 * sum / maxamp + 0.5;
      let b = 0.5 * sum2 / (maxamp + amp) + 0.5;
      return a + rmd * (b - a);
    }
    return sum + rmd * (sum2 - sum);
  }
  if (normalize) { return 0.5 * sum / maxamp + 0.5; }
  return sum;
}
fn op_g_multi_fractal(dims: u32, p0: vec4<f32>, fr: OpGFractal) -> f32 {
  var p = p0;
  var value = 1.0;
  var pwr = 1.0;
  for (var i = 0; i <= i32(fr.detail); i++) {
    value *= pwr * op_g_snoise(dims, p) + 1.0;
    pwr *= fr.rough;
    p *= fr.lac;
  }
  let rmd = fr.detail - floor(fr.detail);
  if (rmd != 0.0) { value *= rmd * pwr * op_g_snoise(dims, p) + 1.0; }
  return value;
}
fn op_g_hetero_terrain(dims: u32, p0: vec4<f32>, fr: OpGFractal) -> f32 {
  var p = p0;
  var pwr = fr.rough;
  var value = fr.offset + op_g_snoise(dims, p);
  p *= fr.lac;
  for (var i = 1; i <= i32(fr.detail); i++) {
    let inc = (op_g_snoise(dims, p) + fr.offset) * pwr * value;
    value += inc;
    pwr *= fr.rough;
    p *= fr.lac;
  }
  let rmd = fr.detail - floor(fr.detail);
  if (rmd != 0.0) {
    let inc = (op_g_snoise(dims, p) + fr.offset) * pwr * value;
    value += rmd * inc;
  }
  return value;
}
fn op_g_hybrid_multi_fractal(dims: u32, p0: vec4<f32>, fr: OpGFractal) -> f32 {
  var p = p0;
  var pwr = 1.0;
  var value = 0.0;
  var weight = 1.0;
  for (var i = 0; weight > 0.001 && i <= i32(fr.detail); i++) {
    weight = min(weight, 1.0);
    let signal = (op_g_snoise(dims, p) + fr.offset) * pwr;
    pwr *= fr.rough;
    value += weight * signal;
    weight *= fr.gain * signal;
    p *= fr.lac;
  }
  let rmd = fr.detail - floor(fr.detail);
  if (rmd != 0.0 && weight > 0.001) {
    weight = min(weight, 1.0);
    let signal = (op_g_snoise(dims, p) + fr.offset) * pwr;
    value += rmd * weight * signal;
  }
  return value;
}
fn op_g_ridged_multi_fractal(dims: u32, p0: vec4<f32>, fr: OpGFractal) -> f32 {
  var p = p0;
  var pwr = fr.rough;
  var signal = fr.offset - abs(op_g_snoise(dims, p));
  signal *= signal;
  var value = signal;
  for (var i = 1; i <= i32(fr.detail); i++) {
    p *= fr.lac;
    let weight = clamp(signal * fr.gain, 0.0, 1.0);
    signal = fr.offset - abs(op_g_snoise(dims, p));
    signal *= signal;
    signal *= weight;
    value += signal * pwr;
    pwr *= fr.rough;
  }
  return value;
}
fn op_g_noise_select(dims: u32, kind: u32, p: vec4<f32>, fr: OpGFractal, normalize: bool) -> f32 {
  switch kind {
    case 0u: { return op_g_multi_fractal(dims, p, fr); }
    case 1u: { return op_g_fbm(dims, p, fr, normalize); }
    case 2u: { return op_g_hybrid_multi_fractal(dims, p, fr); }
    case 3u: { return op_g_ridged_multi_fractal(dims, p, fr); }
    default: { return op_g_hetero_terrain(dims, p, fr); }
  }
}
fn op_g_random_offset(dims: u32, seed: f32) -> vec4<f32> {
  if (dims == 1u) { return vec4<f32>(100.0 + op_g_hashf(vec4<f32>(seed, 0.0, 0.0, 0.0), 1u) * 100.0, 0.0, 0.0, 0.0); }
  var o = vec4<f32>(0.0);
  for (var k = 0u; k < dims; k++) {
    // Component k by a lane mask, not o[k]: FXC (DirectX 12) can't write
    // a vector component picked by a loop counter.
    let value = 100.0 + op_g_hashf(vec4<f32>(seed, f32(k), 0.0, 0.0), 2u) * 100.0;
    o = select(o, vec4<f32>(value), vec4<u32>(0u, 1u, 2u, 3u) == vec4<u32>(k));
  }
  return o;
}
fn op_g_noise(dims: u32, kind: u32, normalize: bool, color: bool, vv: vec3<f32>, wv: vec3<f32>, scale: vec3<f32>,
              detail: vec3<f32>, rough: vec3<f32>, lac: vec3<f32>, offset: vec3<f32>, gain: vec3<f32>, distortion: vec3<f32>) -> vec3<f32> {
  let fr = OpGFractal(clamp(detail.x, 0.0, 15.0), max(rough.x, 0.0), lac.x, offset.x, gain.x);
  let v = vv * scale.x;
  let w = wv.x * scale.x;
  var p = vec4<f32>(v, w);
  if (dims == 1u) { p = vec4<f32>(w, 0.0, 0.0, 0.0); }
  if (dims == 2u) { p.z = 0.0; p.w = 0.0; }
  if (dims == 3u) { p.w = 0.0; }
  if (distortion.x != 0.0) {
    var d = vec4<f32>(0.0);
    for (var k = 0u; k < dims; k++) {
      var seed = f32(k);
      if (dims == 1u) { seed = 0.0; }
      let value = op_g_snoise(dims, p + op_g_random_offset(dims, seed)) * distortion.x;
      d = select(d, vec4<f32>(value), vec4<u32>(0u, 1u, 2u, 3u) == vec4<u32>(k));
    }
    p += d;
  }
  let value = op_g_noise_select(dims, kind, p, fr, normalize);
  if (!color) { return vec3<f32>(value); }
  let first = f32(dims);
  return vec3<f32>(value,
    op_g_noise_select(dims, kind, p + op_g_random_offset(dims, first), fr, normalize),
    op_g_noise_select(dims, kind, p + op_g_random_offset(dims, first + 1.0), fr, normalize));
}
fn op_g_white_noise(dims: u32, color: bool, v: vec3<f32>, wv: vec3<f32>) -> vec3<f32> {
  let w = wv.x;
  if (dims == 1u) {
    let a = op_g_hashf(vec4<f32>(w, 0.0, 0.0, 0.0), 1u);
    if (!color) { return vec3<f32>(a); }
    return vec3<f32>(a, op_g_hashf(vec4<f32>(w, 1.0, 0.0, 0.0), 2u), op_g_hashf(vec4<f32>(w, 2.0, 0.0, 0.0), 2u));
  }
  if (dims == 2u) {
    let a = op_g_hashf(vec4<f32>(v.x, v.y, 0.0, 0.0), 2u);
    if (!color) { return vec3<f32>(a); }
    return vec3<f32>(a, op_g_hashf(vec4<f32>(v.x, v.y, 1.0, 0.0), 3u), op_g_hashf(vec4<f32>(v.x, v.y, 2.0, 0.0), 3u));
  }
  if (dims == 3u) {
    let a = op_g_hashf(vec4<f32>(v, 0.0), 3u);
    if (!color) { return vec3<f32>(a); }
    return vec3<f32>(a, op_g_hashf(vec4<f32>(v, 1.0), 4u), op_g_hashf(vec4<f32>(v, 2.0), 4u));
  }
  let a = op_g_hashf(vec4<f32>(v, w), 4u);
  if (!color) { return vec3<f32>(a); }
  return vec3<f32>(a, op_g_hashf(vec4<f32>(v.z, v.x, w, v.y), 4u), op_g_hashf(vec4<f32>(w, v.z, v.y, v.x), 4u));
}
fn op_g_checker(color: bool, v: vec3<f32>, c1: vec3<f32>, c2: vec3<f32>, scale: vec3<f32>) -> vec3<f32> {
  let p = (v * scale.x + 0.000001) * 0.999999;
  let i = abs(vec3<i32>(floor(p))) % vec3<i32>(2);
  let f = select(0.0, 1.0, (i.x == i.y) == (i.z == 1));
  if (!color) { return vec3<f32>(f); }
  if (f == 1.0) { return c1; }
  return c2;
}
fn op_g_gradient(kind: u32, p: vec3<f32>) -> f32 {
  var f = 0.0;
  switch kind {
    case 0u: { f = p.x; }
    case 1u: { let r = max(p.x, 0.0); f = r * r; }
    case 2u: { let r = clamp(p.x, 0.0, 1.0); let t = r * r; f = 3.0 * t - 2.0 * t * r; }
    case 3u: { f = (p.x + p.y) * 0.5; }
    case 4u: { f = atan2(p.y, p.x) / 6.283185307179586 + 0.5; }
    default: {
      let r = max(0.999999 - length(p), 0.0);
      if (kind == 6u) { f = r; } else { f = r * r; }
    }
  }
  return clamp(f, 0.0, 1.0);
}
fn op_g_wave(rings: bool, dir: u32, profile: u32, v: vec3<f32>, scale: vec3<f32>, distortion: vec3<f32>,
             detail: vec3<f32>, dscale: vec3<f32>, drough: vec3<f32>, phase: vec3<f32>) -> f32 {
  let p = (v * scale.x + 0.000001) * 0.999999;
  var n = 0.0;
  if (!rings) {
    if (dir == 0u) { n = p.x * 20.0; } else if (dir == 1u) { n = p.y * 20.0; } else if (dir == 2u) { n = p.z * 20.0; } else { n = (p.x + p.y + p.z) * 10.0; }
  } else {
    var keep = vec3<f32>(1.0);
    if (dir == 0u) { keep = vec3<f32>(0.0, 1.0, 1.0); } else if (dir == 1u) { keep = vec3<f32>(1.0, 0.0, 1.0); } else if (dir == 2u) { keep = vec3<f32>(1.0, 1.0, 0.0); }
    n = length(p * keep) * 20.0;
  }
  n += phase.x;
  if (distortion.x != 0.0) {
    let fr = OpGFractal(clamp(detail.x, 0.0, 15.0), drough.x, 2.0, 0.0, 1.0);
    n += distortion.x * (op_g_fbm(3u, vec4<f32>(p * dscale.x, 0.0), fr, true) * 2.0 - 1.0);
  }
  if (profile == 0u) { return 0.5 + 0.5 * sin(n - 1.5707963267948966); }
  let m = n / 6.283185307179586;
  if (profile == 1u) { return m - floor(m); }
  return abs(m - floor(m + 0.5)) * 2.0;
}
