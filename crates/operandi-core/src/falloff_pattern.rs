//! Map- and pattern-driven falloffs (falloff spec 2026-11-15 batch 5):
//! Noise (FO-50), Vertex Map colour and morph maps (FO-49) and Image
//! (FO-54). Original code: the noise is a plain gradient noise with a fixed
//! seed, written here.
use crate::Mesh;
use crate::falloff::{VertexMapFalloff, VertexMapMode};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// FO-50 Noise: smooth 3D noise of position / Scale, 0..1.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NoiseFalloff {
    /// Size of the noise blobs (scene units).
    pub scale: f64,
    /// FO-35, as for the geometric falloffs.
    pub world: bool,
}
impl Default for NoiseFalloff {
    /// FO-50 (measured): Scale 1 m, World off.
    fn default() -> Self {
        Self {
            scale: 1.,
            world: false,
        }
    }
}
impl NoiseFalloff {
    pub fn validate(self) -> crate::Result<()> {
        if !(self.scale.is_finite() && self.scale != 0.) {
            return Err("Noise Scale must be a number other than 0".into());
        }
        Ok(())
    }
    /// Gradient noise of `p / Scale`, mapped from −1..1 to 0..1 (FO-50,
    /// proposed; fixed seed, so it repeats).
    pub fn weight(self, p: DVec3) -> f64 {
        (0.5 + 0.5 * gradient_noise(p / self.scale)).clamp(0., 1.)
    }
}

/// A repeatable hash of a lattice corner.
fn hash(x: i64, y: i64, z: i64) -> u64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ (z as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h
}

/// One of twelve cube-edge directions for a lattice corner.
fn gradient(x: i64, y: i64, z: i64) -> DVec3 {
    const DIRS: [[f64; 3]; 12] = [
        [1., 1., 0.],
        [-1., 1., 0.],
        [1., -1., 0.],
        [-1., -1., 0.],
        [1., 0., 1.],
        [-1., 0., 1.],
        [1., 0., -1.],
        [-1., 0., -1.],
        [0., 1., 1.],
        [0., -1., 1.],
        [0., 1., -1.],
        [0., -1., -1.],
    ];
    DVec3::from(DIRS[(hash(x, y, z) % 12) as usize])
}

/// Gradient noise in about −1..1: dot products with corner gradients,
/// blended with the quintic fade. 0 at every lattice point.
pub fn gradient_noise(p: DVec3) -> f64 {
    let base = p.floor();
    let f = p - base;
    let (x, y, z) = (base.x as i64, base.y as i64, base.z as i64);
    let fade = |t: f64| t * t * t * (t * (t * 6. - 15.) + 10.);
    let (u, v, w) = (fade(f.x), fade(f.y), fade(f.z));
    let corner = |dx: i64, dy: i64, dz: i64| {
        gradient(x + dx, y + dy, z + dz).dot(f - DVec3::new(dx as f64, dy as f64, dz as f64))
    };
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let x00 = lerp(corner(0, 0, 0), corner(1, 0, 0), u);
    let x10 = lerp(corner(0, 1, 0), corner(1, 1, 0), u);
    let x01 = lerp(corner(0, 0, 1), corner(1, 0, 1), u);
    let x11 = lerp(corner(0, 1, 1), corner(1, 1, 1), u);
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w)
}

/// What kind of vertex map a name selects (FO-49), in the order they are
/// looked up when names clash (provisional).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapKind {
    Weight,
    Morph,
    Colour,
}

/// The kind of map `name` names on `mesh`.
pub fn map_kind(mesh: &Mesh, name: &str) -> Option<MapKind> {
    let a = &mesh.attributes;
    if a.point_weights.contains_key(name) {
        Some(MapKind::Weight)
    } else if a.relative_morphs.contains_key(name) {
        Some(MapKind::Morph)
    } else if a.corner_colors.contains_key(name) {
        Some(MapKind::Colour)
    } else {
        None
    }
}

/// FO-49 for colour and morph maps: one value per point. Magnitude: a
/// colour's brightness (the mean of R, G, B) or a morph offset's length;
/// Component: the Index channel (R, G, B, A or X, Y, Z). A point with no
/// value weighs 0 (VMF-04). `None` for weight maps (read as before) and
/// unknown names.
pub fn vertex_map_values(mesh: &Mesh, name: &str, f: VertexMapFalloff) -> Option<Vec<f64>> {
    let n = mesh.vertices.len();
    let component = |v: &[f64]| -> f64 {
        match f.mode {
            VertexMapMode::Component => v.get(f.index as usize).copied().unwrap_or(0.),
            VertexMapMode::Magnitude => 0.,
        }
    };
    match map_kind(mesh, name)? {
        MapKind::Weight => None,
        MapKind::Morph => {
            let map = &mesh.attributes.relative_morphs[name];
            let mut out = vec![0.; n];
            for (&v, d) in map {
                if let Some(o) = out.get_mut(v as usize) {
                    *o = match f.mode {
                        VertexMapMode::Magnitude => d.length(),
                        VertexMapMode::Component => component(&d.to_array()),
                    };
                }
            }
            Some(out)
        }
        MapKind::Colour => {
            // Each point averages the colours of its polygon corners.
            let rows = &mesh.attributes.corner_colors[name];
            let mut sum = vec![[0.; 4]; n];
            let mut count = vec![0u32; n];
            for (face, row) in mesh.faces.iter().zip(rows) {
                for (&v, c) in face.vertices.iter().zip(row) {
                    if let (Some(c), Some(s)) = (c, sum.get_mut(v as usize)) {
                        for (k, x) in c.to_array().into_iter().enumerate() {
                            s[k] += x;
                        }
                        count[v as usize] += 1;
                    }
                }
            }
            Some(
                sum.iter()
                    .zip(&count)
                    .map(|(s, &c)| {
                        if c == 0 {
                            return 0.;
                        }
                        let mean = s.map(|x| x / c as f64);
                        match f.mode {
                            VertexMapMode::Magnitude => (mean[0] + mean[1] + mean[2]) / 3.,
                            VertexMapMode::Component => component(&mean),
                        }
                    })
                    .collect(),
            )
        }
    }
}

/// FO-54 Image: the brightness of a picture laid over the screen, or
/// looked up through the selected UV map.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageFalloff {
    /// Tile the picture; off, outside it weighs 0.
    pub repeat: bool,
    /// Per cent of the picture's half-size over which its edges fade.
    pub soft_border: f64,
    pub use_uv: bool,
    /// Per cent.
    pub scale: f64,
    /// Offset in picture widths and heights (screen mode) or UV units.
    pub pos: DVec2,
    /// Degrees.
    pub rotation: f64,
    pub world: bool,
}
impl Default for ImageFalloff {
    /// FO-54 (measured): Repeat on, Soft Border 5 %, Use Selected UV Map
    /// off, Scale 100 %, Pos 0, 0, Rotation 0°, World off.
    fn default() -> Self {
        Self {
            repeat: true,
            soft_border: 5.,
            use_uv: false,
            scale: 100.,
            pos: DVec2::ZERO,
            rotation: 0.,
            world: false,
        }
    }
}
impl ImageFalloff {
    pub fn validate(self) -> crate::Result<()> {
        if !(self.scale.is_finite() && self.scale > 0.) {
            return Err("Image Scale must be above 0 %".into());
        }
        if !(self.soft_border.is_finite() && (0. ..=100.).contains(&self.soft_border)) {
            return Err("Image Soft Border must be from 0 % to 100 %".into());
        }
        if !self.pos.is_finite() || !self.rotation.is_finite() {
            return Err("Image Pos and Rotation must be numbers".into());
        }
        Ok(())
    }
    /// The weight at picture coordinates `uv` (0..1 across the untransformed
    /// picture, v down): moved by Pos, scaled about the centre, turned by
    /// Rotation, tiled with Repeat, faded by Soft Border, then the picture's
    /// brightness.
    pub fn weight_at(self, image: &GrayImage, uv: DVec2) -> f64 {
        let (s, c) = self.rotation.to_radians().sin_cos();
        let d = (uv - DVec2::splat(0.5) - self.pos) / (self.scale / 100.);
        let mut q = DVec2::new(c * d.x + s * d.y, -s * d.x + c * d.y) + DVec2::splat(0.5);
        if self.repeat {
            q = q - q.floor();
        } else if q.x < 0. || q.y < 0. || q.x > 1. || q.y > 1. {
            return 0.;
        }
        let edge = q.x.min(1. - q.x).min(q.y).min(1. - q.y);
        let border = self.soft_border / 100. * 0.5;
        let fade = if border > 0. && edge < border {
            crate::FalloffShape::Smooth.weight(1. - edge / border)
        } else {
            1.
        };
        image.sample(q) * fade
    }
}

/// A greyscale picture, 0..1 per pixel, row by row from the top.
#[derive(Clone, Debug, PartialEq)]
pub struct GrayImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<f32>,
}
impl GrayImage {
    /// Bilinear brightness at `q` (0..1 across, v down); 0 when empty.
    pub fn sample(&self, q: DVec2) -> f64 {
        if self.width == 0 || self.height == 0 || self.pixels.len() < self.width * self.height {
            return 0.;
        }
        let x = (q.x * self.width as f64 - 0.5).clamp(0., (self.width - 1) as f64);
        let y = (q.y * self.height as f64 - 0.5).clamp(0., (self.height - 1) as f64);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (fx, fy) = (x - x0 as f64, y - y0 as f64);
        let at = |x: usize, y: usize| self.pixels[y * self.width + x] as f64;
        let top = at(x0, y0) * (1. - fx) + at(x1, y0) * fx;
        let bottom = at(x0, y1) * (1. - fx) + at(x1, y1) * fx;
        top * (1. - fy) + bottom * fy
    }
    /// Width over height.
    pub fn aspect(&self) -> f64 {
        if self.height == 0 {
            1.
        } else {
            self.width as f64 / self.height as f64
        }
    }
}

/// FO-54 with Use Selected UV Map: each point's weight from its UV in
/// `map` (the mean of its corners' UVs; v flipped to run down the
/// picture). Points with no UV weigh 0.
pub fn image_uv_weights(mesh: &Mesh, map: &str, image: &GrayImage, f: ImageFalloff) -> Vec<f64> {
    let n = mesh.vertices.len();
    let Some(rows) = mesh.uv_maps.get(map) else {
        return vec![0.; n];
    };
    let mut sum = vec![DVec2::ZERO; n];
    let mut count = vec![0u32; n];
    for (face, row) in mesh.faces.iter().zip(rows) {
        for (&v, uv) in face.vertices.iter().zip(row) {
            if let (Some(uv), Some(s)) = (uv, sum.get_mut(v as usize)) {
                *s += *uv;
                count[v as usize] += 1;
            }
        }
    }
    sum.iter()
        .zip(&count)
        .map(|(&s, &c)| {
            if c == 0 {
                return 0.;
            }
            let uv = s / c as f64;
            f.weight_at(image, DVec2::new(uv.x, 1. - uv.y))
        })
        .collect()
}

/// FO-54 laid over the screen: the picture spans the view's height,
/// centred; `q` is a point in view pixels, `view` the view's size.
pub fn image_screen_uv(image: &GrayImage, q: DVec2, view: DVec2) -> DVec2 {
    let h = view.y.max(1.);
    DVec2::new((q.x - view.x / 2.) / (h * image.aspect()) + 0.5, q.y / h)
}
