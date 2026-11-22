//! Texture bakes from a mesh's own geometry into its UV layout, and UV
//! padding for any texture: Curvature (edges light, cavities dark, flat
//! 50 % grey) and Thickness (thin parts dark, thick parts light), written
//! as 16-bit greyscale PNGs. These feed edge-wear and dirt masks in the
//! shader tree; Cycles already bakes AO and normals.
//!
//! The curvature method, the thickness rays and the padding come from
//! TextureCraft (MIT OR Apache-2.0, see `curvature.rs` and `padding.rs`);
//! the rasteriser, ray tracer and the commands are Operandi's own.
//! Operandi-only commands (docs/NAMES-NEEDED.md): `operandi.bake.curvature`,
//! `operandi.bake.thickness`, `operandi.bake.padImage`.
use crate::{Mesh, Result, Scene};
use glam::{Vec2, Vec3};
use serde_json::{Map, Value};
use std::path::Path;

pub mod bvh;
mod curvature;
pub mod padding;

use bvh::{Bvh, Hit};
use padding::DilationMap;

/// Which map a bake makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BakeMap {
    Curvature,
    Thickness,
}

impl BakeMap {
    pub fn label(self) -> &'static str {
        match self {
            BakeMap::Curvature => "Curvature",
            BakeMap::Thickness => "Thickness",
        }
    }
    pub fn command(self) -> &'static str {
        match self {
            BakeMap::Curvature => "operandi.bake.curvature",
            BakeMap::Thickness => "operandi.bake.thickness",
        }
    }
}

/// A bake's settings. Distances are fractions of the mesh's largest side.
#[derive(Clone, Debug, PartialEq)]
pub struct BakeOptions {
    pub map: BakeMap,
    /// Width and height in pixels.
    pub size: u32,
    /// Pixels of padding around each UV island.
    pub padding: u32,
    /// Fill the rest of the image with a smooth average (no background).
    pub fill: bool,
    /// The UV map baked into; `None` takes the mesh's usual one.
    pub uv_map: Option<String>,
    /// Rays per pixel.
    pub rays: u32,
    /// Curvature: how far around a point the surface is sampled.
    pub radius: f32,
    /// Thickness: the longest distance measured.
    pub distance: f32,
}

impl BakeOptions {
    pub fn new(map: BakeMap) -> BakeOptions {
        BakeOptions {
            map,
            size: 1024,
            padding: 16,
            fill: true,
            uv_map: None,
            rays: match map {
                BakeMap::Curvature => 32,
                BakeMap::Thickness => 64,
            },
            radius: 0.001,
            distance: 0.1,
        }
    }
}

pub const MIN_SIZE: u32 = 16;
pub const MAX_SIZE: u32 = 8192;
const MAX_RAYS: u32 = 1024;

/// A finished bake: values 0..1, rows top first.
#[derive(Clone, Debug)]
pub struct Baked {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<f32>,
    /// Pixels a polygon covers (before padding).
    pub covered: usize,
}

/// The mesh as triangles with their corner normals and UVs.
pub(crate) struct BakeMesh {
    pub bvh: Bvh,
    pub normals: Vec<[Vec3; 3]>,
    pub uvs: Vec<Option<[Vec2; 3]>>,
    /// Largest side of the bounding box (at least a tiny number).
    pub scale: f32,
}

impl BakeMesh {
    pub fn new(
        mesh: &Mesh,
        smoothing: &crate::shading::SmoothingSettings,
        uv_map: Option<&str>,
    ) -> Result<BakeMesh> {
        let map_name = crate::uv_tools::target_uv_map(mesh, uv_map);
        if let Some(name) = uv_map
            && !mesh.uv_maps.contains_key(name)
        {
            return Err(format!("The mesh has no UV map named {name}"));
        }
        let map = mesh.uv_maps.get(&map_name);
        let normals = crate::shading::mesh_corner_normals(mesh, smoothing);
        let v3 = |p: glam::DVec3| Vec3::new(p.x as f32, p.y as f32, p.z as f32);
        let (mut tris, mut tri_normals, mut tri_uvs) = (Vec::new(), Vec::new(), Vec::new());
        for (f, face) in mesh.faces.iter().enumerate() {
            if mesh.is_polyline(f) {
                continue;
            }
            let Ok(triangles) = mesh.surface_triangles(f) else {
                continue;
            };
            let uv = |c: usize| {
                map.and_then(|m| m.get(f))
                    .and_then(|u| u.get(c).copied().flatten())
                    .or_else(|| face.uv.as_ref().and_then(|u| u.get(c).copied()))
                    .map(|u| Vec2::new(u.x as f32, u.y as f32))
            };
            for t in triangles {
                let p = t.map(|c| v3(mesh.vertices[face.vertices[c] as usize]));
                let n = t.map(|c| {
                    normals
                        .get(f)
                        .and_then(|n| n.get(c))
                        .map(|n| v3(*n))
                        .unwrap_or(Vec3::ZERO)
                });
                tris.push(p);
                tri_normals.push(n);
                tri_uvs.push(match t.map(uv) {
                    [Some(a), Some(b), Some(c)] => Some([a, b, c]),
                    _ => None,
                });
            }
        }
        if tri_uvs.iter().all(Option::is_none) {
            return Err(
                "The mesh has no UVs to bake into: unwrap it first (UV Unwrap or Atlas Unwrap)"
                    .into(),
            );
        }
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for t in &tris {
            for p in t {
                min = min.min(*p);
                max = max.max(*p);
            }
        }
        let scale = (max - min).max_element().max(1e-6);
        Ok(BakeMesh {
            bvh: Bvh::new(tris),
            normals: tri_normals,
            uvs: tri_uvs,
            scale,
        })
    }

    /// The smooth (shading) normal at a point of triangle `tri`.
    pub fn shading_normal(&self, tri: u32, u: f32, v: f32) -> Option<Vec3> {
        let n = self.normals.get(tri as usize)?;
        (n[0] * (1.0 - u - v) + n[1] * u + n[2] * v).try_normalize()
    }

    /// The flat normal of triangle `tri`.
    pub fn face_normal(&self, tri: u32) -> Option<Vec3> {
        let t = self.bvh.triangle(tri);
        (t[1] - t[0]).cross(t[2] - t[0]).try_normalize()
    }

    pub fn closest(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<Hit> {
        self.bvh.closest(origin, dir, max)
    }
}

/// A point on the surface a pixel shows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Surface {
    pub tri: u32,
    pub pos: Vec3,
    /// Smooth normal.
    pub nrm: Vec3,
    /// Flat normal, on the smooth normal's side.
    pub geo: Vec3,
}

/// For each pixel (rows top first, v up in UV space) the surface point at
/// its centre, if a triangle covers it. A later triangle wins where UVs
/// overlap.
pub(crate) fn rasterise(mesh: &BakeMesh, width: u32, height: u32) -> Vec<Option<Surface>> {
    let mut out = vec![None; width as usize * height as usize];
    let (w, h) = (width as f32, height as f32);
    for (tri, uv) in mesh.uvs.iter().enumerate() {
        let Some(uv) = uv else { continue };
        // Pixel space: x right, y down.
        let p = uv.map(|t| Vec2::new(t.x * w, (1.0 - t.y) * h));
        let area = (p[1] - p[0]).perp_dot(p[2] - p[0]);
        if area.abs() < 1e-12 {
            continue;
        }
        let lo = p[0].min(p[1]).min(p[2]);
        let hi = p[0].max(p[1]).max(p[2]);
        let x0 = (lo.x - 0.5).ceil().max(0.0) as u32;
        let y0 = (lo.y - 0.5).ceil().max(0.0) as u32;
        let x1 = ((hi.x - 0.5).floor() as i64).min(width as i64 - 1);
        let y1 = ((hi.y - 0.5).floor() as i64).min(height as i64 - 1);
        if x1 < 0 || y1 < 0 {
            continue;
        }
        let corners = mesh.bvh.triangle(tri as u32);
        let normals = &mesh.normals[tri];
        let flat = (corners[1] - corners[0])
            .cross(corners[2] - corners[0])
            .normalize_or_zero();
        for y in y0..=y1 as u32 {
            for x in x0..=x1 as u32 {
                let c = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let b1 = (c - p[0]).perp_dot(p[2] - p[0]) / area;
                let b2 = (p[1] - p[0]).perp_dot(c - p[0]) / area;
                let b0 = 1.0 - b1 - b2;
                const E: f32 = -1e-5;
                if b0 < E || b1 < E || b2 < E {
                    continue;
                }
                let pos = corners[0] * b0 + corners[1] * b1 + corners[2] * b2;
                let nrm = (normals[0] * b0 + normals[1] * b1 + normals[2] * b2)
                    .try_normalize()
                    .unwrap_or(flat);
                let geo = if flat.dot(nrm) < 0.0 { -flat } else { flat };
                out[(y * width + x) as usize] = Some(Surface {
                    tri: tri as u32,
                    pos,
                    nrm,
                    geo,
                });
            }
        }
    }
    out
}

/// Runs `f` over every pixel on all cores, in row bands.
fn per_pixel(samples: &[Option<Surface>], f: impl Fn(usize, &Surface) -> f32 + Sync) -> Vec<f32> {
    let mut out = vec![f32::NAN; samples.len()];
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .max(1);
    let chunk = samples.len().div_ceil(threads * 8).max(1);
    let bands: Vec<(usize, &mut [f32])> = out.chunks_mut(chunk).enumerate().collect();
    let bands = std::sync::Mutex::new(bands);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                while let Some((band, values)) = bands.lock().map(|mut b| b.pop()).ok().flatten() {
                    let start = band * chunk;
                    for (i, v) in values.iter_mut().enumerate() {
                        if let Some(surface) = &samples[start + i] {
                            *v = f(start + i, surface);
                        }
                    }
                }
            });
        }
    });
    out
}

/// Two numbers in 0..1 from a pixel index, to turn each pixel's rays.
fn rotation(i: usize) -> [f32; 2] {
    let mut x = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    [
        (x & 0xFFFF_FFFF) as f32 / 4_294_967_296.0,
        (x >> 32) as f32 / 4_294_967_296.0,
    ]
}

/// Average distance through the mesh along cosine-spread rays into the
/// surface (TextureCraft's thickness rays).
fn thickness(mesh: &BakeMesh, s: &Surface, rays: u32, max: f32, eps: f32, turn: [f32; 2]) -> f32 {
    let axis = -s.nrm;
    let geo = -s.geo;
    let origin = s.pos + geo * eps;
    let (tangent, bitangent) = axis.any_orthonormal_pair();
    let (mut sum, mut count) = (0.0, 0);
    for r in 0..rays {
        let a = (r as f32 / rays as f32 + turn[0]).fract();
        let e = (r.reverse_bits() as f32 / 4_294_967_296.0 + turn[1]).fract();
        let radius = e.sqrt();
        let height = (1.0 - radius * radius).max(0.0).sqrt();
        let angle = std::f32::consts::TAU * a;
        let dir =
            tangent * (radius * angle.cos()) + bitangent * (radius * angle.sin()) + axis * height;
        if dir.dot(geo) <= 0.0 {
            continue;
        }
        sum += mesh.closest(origin, dir, max).map_or(max, |h| h.t);
        count += 1;
    }
    if count == 0 { max } else { sum / count as f32 }
}

/// Bakes `options.map` for mesh item `item` of `scene` (its own, local
/// shape; the item's transform doesn't change the result).
pub fn bake(scene: &Scene, item: u64, options: &BakeOptions) -> Result<Baked> {
    let item = scene.item(item).ok_or("No such item")?;
    if !item.is_mesh() || item.mesh.faces.is_empty() {
        return Err(format!("{} has no polygons to bake", item.name));
    }
    if !(MIN_SIZE..=MAX_SIZE).contains(&options.size) {
        return Err(format!("size is {MIN_SIZE} to {MAX_SIZE} pixels"));
    }
    if options.rays == 0 || options.rays > MAX_RAYS {
        return Err(format!("rays is 1 to {MAX_RAYS}"));
    }
    let mesh = BakeMesh::new(&item.mesh, &scene.smoothing, options.uv_map.as_deref())?;
    let (w, h) = (options.size, options.size);
    let samples = rasterise(&mesh, w, h);
    let covered: Vec<bool> = samples.iter().map(Option::is_some).collect();
    let count = covered.iter().filter(|c| **c).count();
    if count == 0 {
        return Err("No polygon's UVs fall inside the 0-1 square; nothing to bake".into());
    }
    let eps = 1e-5 * mesh.scale;
    let mut pixels = match options.map {
        BakeMap::Thickness => {
            if !(options.distance > 0.0 && options.distance <= 10.0) {
                return Err("distance is above 0 and at most 10 (times the mesh's size)".into());
            }
            let max = options.distance * mesh.scale;
            let raw = per_pixel(&samples, |i, s| {
                thickness(&mesh, s, options.rays, max, eps, rotation(i))
            });
            normalise(raw, &covered, |lo, hi| (lo, hi), 1.0)
        }
        BakeMap::Curvature => {
            if !(options.radius > 0.0 && options.radius <= 10.0) {
                return Err("radius is above 0 and at most 10 (times the mesh's size)".into());
            }
            let c = curvature::Curvature::new(&mesh, options.rays as usize, options.radius, w, h);
            let raw = per_pixel(&samples, |_, s| c.value(&mesh, s));
            // Flat stays at 50 %: the range is symmetric about zero.
            normalise(
                raw,
                &covered,
                |lo, hi| {
                    let m = lo.abs().max(hi.abs());
                    (-m, m)
                },
                0.5,
            )
        }
    };
    let dilation = DilationMap::new(&covered, w, h, Some(options.padding));
    dilation.apply(&mut pixels, 1);
    if options.fill {
        padding::fill_rest(&mut pixels, 1, &dilation);
    }
    for v in &mut pixels {
        if !v.is_finite() {
            *v = 0.0;
        }
    }
    Ok(Baked {
        width: w,
        height: h,
        pixels,
        covered: count,
    })
}

/// Covered values mapped to 0..1 over `range(min, max)`; a flat map gives
/// `flat`.
fn normalise(
    mut px: Vec<f32>,
    covered: &[bool],
    range: impl Fn(f32, f32) -> (f32, f32),
    flat: f32,
) -> Vec<f32> {
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for (v, c) in px.iter().zip(covered) {
        if *c && v.is_finite() {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }
    }
    if !lo.is_finite() {
        return px;
    }
    let (lo, hi) = range(lo, hi);
    for (v, c) in px.iter_mut().zip(covered) {
        if *c {
            *v = if hi - lo > f32::EPSILON {
                ((*v - lo) / (hi - lo)).clamp(0.0, 1.0)
            } else {
                flat
            };
        }
    }
    px
}

/// Writes values 0..1 as a 16-bit greyscale PNG (through a temporary, so
/// a failure never leaves half a file).
pub fn write_png16(path: &Path, baked: &Baked) -> Result<()> {
    let mut data = Vec::with_capacity(baked.pixels.len() * 2);
    for v in &baked.pixels {
        data.extend(((v.clamp(0.0, 1.0) * 65535.0).round() as u16).to_be_bytes());
    }
    write_png(
        path,
        baked.width,
        baked.height,
        png::ColorType::Grayscale,
        png::BitDepth::Sixteen,
        &data,
    )
}

fn write_png(
    path: &Path,
    w: u32,
    h: u32,
    color: png::ColorType,
    depth: png::BitDepth,
    data: &[u8],
) -> Result<()> {
    crate::interchange::refuse_scene_file(path)?;
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, w, h);
        encoder.set_color(color);
        encoder.set_depth(depth);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(data).map_err(|e| e.to_string())?;
    }
    let name = path
        .file_name()
        .ok_or("Choose a file name")?
        .to_string_lossy();
    let temp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&temp, &bytes).map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    std::fs::rename(&temp, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        format!("Cannot write {}: {e}", path.display())
    })
}

/// Pads a texture (PNG, any colour type and depth) around mesh item
/// `item`'s UV islands, so seams don't bleed background when filtered.
/// Writes `output` (the same file when `None`). Returns how many pixels
/// changed.
pub fn pad_image(
    scene: &Scene,
    item: u64,
    input: &Path,
    output: Option<&Path>,
    padding_px: u32,
    uv_map: Option<&str>,
) -> Result<usize> {
    let item = scene.item(item).ok_or("No such item")?;
    if !item.is_mesh() {
        return Err(format!("{} isn't a mesh", item.name));
    }
    let file =
        std::fs::File::open(input).map_err(|e| format!("Cannot read {}: {e}", input.display()))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("{}: {e}", input.display()))?;
    let size = reader
        .output_buffer_size()
        .ok_or("The image is too large")?;
    let mut buf = vec![0; size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{}: {e}", input.display()))?;
    buf.truncate(info.buffer_size());
    let (w, h) = (info.width, info.height);
    if w > MAX_SIZE * 2 || h > MAX_SIZE * 2 {
        return Err("The image is larger than 16384 pixels".into());
    }
    let bpp = info.line_size / w.max(1) as usize;
    let mesh = BakeMesh::new(&item.mesh, &scene.smoothing, uv_map)?;
    let covered: Vec<bool> = rasterise(&mesh, w, h).iter().map(Option::is_some).collect();
    if !covered.iter().any(|c| *c) {
        return Err("No polygon's UVs fall inside the 0-1 square; nothing to pad".into());
    }
    let dilation = DilationMap::new(&covered, w, h, Some(padding_px));
    let changed = dilation
        .src
        .iter()
        .enumerate()
        .filter(|(i, s)| **s != padding::NONE && **s as usize != *i)
        .count();
    // Rows have no gaps once expanded, so whole pixels copy as bytes.
    dilation.apply(&mut buf, bpp);
    write_png(
        output.unwrap_or(input),
        w,
        h,
        info.color_type,
        info.bit_depth,
        &buf,
    )?;
    Ok(changed)
}

// ------------------------------------------------------------- commands

fn get<'a>(a: &'a Map<String, Value>, k: &str) -> Option<&'a Value> {
    a.get(k).filter(|v| !v.is_null())
}
fn int(a: &Map<String, Value>, k: &str) -> Result<Option<u32>> {
    match get(a, k) {
        None => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .map(Some)
            .ok_or(format!("{k} is a whole number")),
        Some(Value::String(s)) => s
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| format!("{k} is a whole number")),
        _ => Err(format!("{k} is a whole number")),
    }
}
fn float(a: &Map<String, Value>, k: &str) -> Result<Option<f32>> {
    match get(a, k) {
        None => Ok(None),
        Some(Value::Number(n)) => n
            .as_f64()
            .map(|v| Some(v as f32))
            .ok_or(format!("{k} is a number")),
        Some(Value::String(s)) => s
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| format!("{k} is a number")),
        _ => Err(format!("{k} is a number")),
    }
}
fn flag(a: &Map<String, Value>, k: &str) -> Result<Option<bool>> {
    match get(a, k) {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(Value::String(s)) => crate::command_args::boolean(k, s).map(Some),
        _ => Err(format!("{k} is true or false")),
    }
}
fn text<'a>(a: &'a Map<String, Value>, k: &str) -> Result<Option<&'a str>> {
    match get(a, k) {
        None => Ok(None),
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(Some(s.trim())),
        _ => Err(format!("{k} is a text")),
    }
}

/// A bake command's options; unnamed ones keep `base`'s values.
pub fn bake_args(a: &Map<String, Value>, base: &BakeOptions) -> Result<BakeOptions> {
    let mut o = base.clone();
    if let Some(v) = int(a, "size")? {
        o.size = v;
    }
    if let Some(v) = int(a, "padding")? {
        o.padding = v;
    }
    if let Some(v) = flag(a, "fill")? {
        o.fill = v;
    }
    if let Some(v) = text(a, "uvMap")? {
        o.uv_map = Some(v.to_owned());
    }
    if let Some(v) = int(a, "rays")? {
        o.rays = v;
    }
    if let Some(v) = float(a, "radius")? {
        o.radius = v;
    }
    if let Some(v) = float(a, "distance")? {
        o.distance = v;
    }
    Ok(o)
}

/// Runs a bake command on the active mesh: writes `filename:` and returns
/// the status line.
pub fn run_bake(scene: &Scene, a: &Map<String, Value>, options: &BakeOptions) -> Result<String> {
    let path = text(a, "filename")?.ok_or("filename is required (a .png to write)")?;
    let item = scene.selection.item.ok_or("Select a mesh item to bake")?;
    let baked = bake(scene, item, options)?;
    let path = Path::new(path);
    write_png16(path, &baked)?;
    Ok(format!(
        "Baked {} {}×{} to {}",
        options.map.label(),
        baked.width,
        baked.height,
        path.display()
    ))
}

/// `operandi.bake.padImage`: pads `filename:` into `output:` (or itself).
pub fn run_pad(scene: &Scene, a: &Map<String, Value>) -> Result<String> {
    let input = text(a, "filename")?.ok_or("filename is required (the .png to pad)")?;
    let output = text(a, "output")?;
    let padding_px = int(a, "padding")?.unwrap_or(16);
    let item = scene
        .selection
        .item
        .ok_or("Select the mesh item whose UVs the image uses")?;
    let changed = pad_image(
        scene,
        item,
        Path::new(input),
        output.map(Path::new),
        padding_px,
        text(a, "uvMap")?,
    )?;
    Ok(format!(
        "Padded {changed} pixel(s) into {}",
        output.unwrap_or(input)
    ))
}

#[cfg(test)]
mod tests;
