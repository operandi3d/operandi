//! Painting stage 5: inks and the Curve Constraint (PNT-170..200).
//! Painting builds only.
//!
//! Inks change where a dab's colour (or strength) comes from as it lands.
//! Several can be in the stack at once; each takes the colour and
//! strength the one before it gave, in stack order ([`InkRun::apply`]).
//! - **Image Ink (PNT-170..173):** a picture floating over the view like a
//!   sheet of glass; a texel takes the picture's colour where its surface
//!   point shows on screen.
//! - **Parametric Ink (PNT-180..182):** FG to BG (or less paint) by slope,
//!   altitude, incidence angle or pen pressure.
//! - **Random Ink (PNT-185):** each dab's colour varies at random.
//! - **Projection Ink (PNT-190..192):** a camera's background image
//!   projected from that camera.
//!
//! The Curve Constraint (PNT-200) snaps dabs to background curves:
//! [`CurveGuide`].
use crate::brush_engine::TipShape;
use crate::paint_stroke::PaintView;
use glam::{DMat4, DVec2, DVec3};
use operandi_paint::PaintImage;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InkKind {
    Image,
    Parametric,
    Random,
    Projection,
}

impl InkKind {
    pub const ALL: [InkKind; 4] = [
        InkKind::Image,
        InkKind::Parametric,
        InkKind::Random,
        InkKind::Projection,
    ];
    pub fn label(self) -> &'static str {
        match self {
            InkKind::Image => "Image Ink",
            InkKind::Parametric => "Parametric Ink",
            InkKind::Random => "Random Ink",
            InkKind::Projection => "Projection Ink",
        }
    }
    pub fn word(self) -> &'static str {
        match self {
            InkKind::Image => "image",
            InkKind::Parametric => "parametric",
            InkKind::Random => "random",
            InkKind::Projection => "projection",
        }
    }
    pub fn named(word: &str) -> Option<InkKind> {
        InkKind::ALL
            .into_iter()
            .find(|k| k.word().eq_ignore_ascii_case(word.trim()))
    }
}

/// Image Ink's rows (PNT-172).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageInk {
    pub image: Option<u64>,
    pub flip_h: bool,
    pub flip_v: bool,
    pub stamp: bool,
    pub pixel_blending: bool,
    pub repeat: bool,
    pub auto_scale: bool,
    pub fit_view: bool,
    pub soft_border: f64,
    pub scale: f64,
    /// The widget's centre in view pixels (0,0 top-left); `None` until
    /// shown, then the view's centre.
    pub pos: Option<DVec2>,
    /// Radians.
    pub rotation: f64,
    pub use_color_as_mask: bool,
    pub uv_mask: Option<u64>,
    pub mask_blur: f64,
    pub invert_mask: bool,
}

impl Default for ImageInk {
    fn default() -> Self {
        ImageInk {
            image: None,
            flip_h: false,
            flip_v: false,
            stamp: false,
            pixel_blending: true,
            repeat: true,
            auto_scale: false,
            fit_view: false,
            soft_border: 0.05,
            scale: 1.,
            pos: None,
            rotation: 0.,
            use_color_as_mask: false,
            uv_mask: None,
            mask_blur: 1.,
            invert_mask: false,
        }
    }
}

/// Parametric Ink's Parameter (PNT-180).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Parameter {
    Displacement,
    Slope,
    Altitude,
    #[default]
    IncidenceAngle,
    Pressure,
    TangentPressure,
}

impl Parameter {
    pub const ALL: [Parameter; 6] = [
        Parameter::Displacement,
        Parameter::Slope,
        Parameter::Altitude,
        Parameter::IncidenceAngle,
        Parameter::Pressure,
        Parameter::TangentPressure,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Parameter::Displacement => "Displacement",
            Parameter::Slope => "Slope",
            Parameter::Altitude => "Altitude",
            Parameter::IncidenceAngle => "Incidence Angle",
            Parameter::Pressure => "Pressure",
            Parameter::TangentPressure => "Tangent Pressure",
        }
    }
    pub fn named(word: &str) -> Option<Parameter> {
        let w: String = word.chars().filter(|c| !c.is_whitespace()).collect();
        Parameter::ALL
            .into_iter()
            .find(|p| p.label().replace(' ', "").eq_ignore_ascii_case(&w))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParametricInk {
    pub parameter: Parameter,
    pub use_as_mask: bool,
    pub shape: TipShape,
    pub inn: f64,
    pub out: f64,
}

impl Default for ParametricInk {
    fn default() -> Self {
        ParametricInk {
            parameter: Parameter::IncidenceAngle,
            use_as_mask: true,
            shape: TipShape::Smooth,
            inn: 0.,
            out: 0.,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RandomInk {
    pub hue: f64,
    pub saturation: f64,
    pub brightness: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectionInk {
    pub show_image: bool,
    pub distance: f64,
    pub opacity: f64,
    pub stamp: bool,
    pub exposure: f64,
}

impl Default for ProjectionInk {
    fn default() -> Self {
        ProjectionInk {
            show_image: true,
            distance: 2.,
            opacity: 1.,
            stamp: false,
            exposure: 0.,
        }
    }
}

/// One ink in the stack.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ink {
    Image(ImageInk),
    Parametric(ParametricInk),
    Random(RandomInk),
    Projection(ProjectionInk),
}

impl Ink {
    pub fn kind(&self) -> InkKind {
        match self {
            Ink::Image(_) => InkKind::Image,
            Ink::Parametric(_) => InkKind::Parametric,
            Ink::Random(_) => InkKind::Random,
            Ink::Projection(_) => InkKind::Projection,
        }
    }
    pub fn new(kind: InkKind) -> Ink {
        match kind {
            InkKind::Image => Ink::Image(ImageInk::default()),
            InkKind::Parametric => Ink::Parametric(ParametricInk::default()),
            InkKind::Random => Ink::Random(RandomInk::default()),
            InkKind::Projection => Ink::Projection(ProjectionInk::default()),
        }
    }
}

/// The inks of a tool, in stack order (at most one of each kind).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inks(pub Vec<Ink>);

impl Inks {
    /// Turns `kind` on (added at the end) or off (removed).
    pub fn toggle(&mut self, kind: InkKind) -> bool {
        if let Some(i) = self.0.iter().position(|k| k.kind() == kind) {
            self.0.remove(i);
            false
        } else {
            self.0.push(Ink::new(kind));
            true
        }
    }
    pub fn has(&self, kind: InkKind) -> bool {
        self.0.iter().any(|k| k.kind() == kind)
    }
    pub fn get_mut(&mut self, kind: InkKind) -> Option<&mut Ink> {
        self.0.iter_mut().find(|k| k.kind() == kind)
    }
    pub fn image(&self) -> Option<&ImageInk> {
        self.0.iter().find_map(|k| match k {
            Ink::Image(i) => Some(i),
            _ => None,
        })
    }
}

/// A camera's projection for Projection Ink.
#[derive(Clone, Debug)]
pub struct CameraSource {
    pub frame: crate::camera_light::Frame,
    /// Half the picture's width and height at unit distance.
    pub half: (f64, f64),
    pub image: Arc<PaintImage>,
}

impl CameraSource {
    /// The camera `item`'s background picture, if it has one (PNT-192).
    pub fn of(
        scene: &crate::Scene,
        item: u64,
        image: impl Fn(u64) -> Option<Arc<PaintImage>>,
    ) -> Result<CameraSource, String> {
        let it = scene.item(item).ok_or("No such camera")?;
        let cam = it
            .camera
            .as_ref()
            .ok_or("Select a camera for Projection Ink")?;
        let id = cam.background.image.ok_or(NO_CAMERA_IMAGE)?;
        let px = image(id).ok_or("The camera's background image can't be read")?;
        let frame = crate::camera_light::Frame::from_matrix(scene.world_matrix(item)?)
            .ok_or("The camera's placement is degenerate")?;
        let aspect = f64::from(px.width) / f64::from(px.height.max(1));
        Ok(CameraSource {
            frame,
            half: cam.half_tangents(aspect),
            image: px,
        })
    }
    /// The picture's colour where world point `p` lands on it.
    fn sample(&self, p: DVec3) -> Option<[f32; 4]> {
        let d = p - self.frame.position;
        let z = d.dot(self.frame.forward);
        if z <= 1e-9 {
            return None;
        }
        let x = d.dot(self.frame.right) / z / self.half.0;
        let y = d.dot(self.frame.up) / z / self.half.1;
        if x.abs() > 1. || y.abs() > 1. {
            return None;
        }
        let (u, v) = ((x + 1.) / 2., (1. - y) / 2.);
        let px = &self.image;
        Some(px.sample(u as f32 * px.width as f32, v as f32 * px.height as f32))
    }
}

/// Projection Ink without a camera background image.
pub const NO_CAMERA_IMAGE: &str = "Needs a camera background image";

/// Everything the inks need during one stroke.
#[derive(Clone, Debug, Default)]
pub struct InkRun {
    pub inks: Inks,
    /// Image Ink's picture and UV Mask.
    pub image: Option<Arc<PaintImage>>,
    pub uv_mask: Option<Arc<PaintImage>>,
    pub camera: Option<CameraSource>,
    /// The view's size (Image Ink fits its height) and its top-left
    /// corner in screen points (Pos X/Y count from there).
    pub view: DVec2,
    pub origin: DVec2,
    /// The painted mesh's world bounding box (Altitude).
    pub bounds: (DVec3, DVec3),
    pub seed: u32,
}

/// The dab's side of the inks: its number in the stroke (Random Ink),
/// the pen pressure, and the FG and BG colours (Parametric Ink).
#[derive(Clone, Copy, Debug)]
pub struct InkDab {
    pub dab: u32,
    pub pressure: f64,
    pub fg: [f32; 3],
    pub bg: [f32; 3],
}

/// What a texel offers the inks.
#[derive(Clone, Copy, Debug)]
pub struct InkTexel {
    pub screen: DVec2,
    pub world: DVec3,
    pub normal: DVec3,
    pub toward_viewer: DVec3,
    pub uv: DVec2,
}

impl InkRun {
    pub fn is_empty(&self) -> bool {
        self.inks.0.is_empty()
    }

    /// The colour and strength a texel paints with, from `colour` and
    /// `strength` (the tool's), through every ink in order. `dab` numbers
    /// the dab in the stroke (Random Ink), `pressure` is the pen's.
    pub fn apply(
        &self,
        t: &InkTexel,
        d: &InkDab,
        mut colour: [f32; 3],
        mut strength: f32,
    ) -> ([f32; 3], f32) {
        for ink in &self.inks.0 {
            match ink {
                Ink::Image(ii) => {
                    let Some(px) = &self.image else {
                        continue;
                    };
                    match image_ink_sample(ii, px, self.view, t.screen - self.origin) {
                        Some((c, a)) => {
                            if ii.use_color_as_mask {
                                // A stencil: white lets the tool's colour
                                // through, black holds it back (PNT-Q43).
                                let b = operandi_paint::image::luma(c);
                                strength *= b.clamp(0., 1.) * a;
                            } else {
                                colour = c;
                                strength *= a;
                            }
                        }
                        None => strength = 0.,
                    }
                    if let Some(m) = &self.uv_mask {
                        let mut v = blurred_luma(m, t.uv, ii.mask_blur);
                        if ii.invert_mask {
                            v = 1. - v;
                        }
                        strength *= v;
                    }
                }
                Ink::Parametric(pi) => {
                    let raw = match pi.parameter {
                        Parameter::Slope => {
                            t.normal.dot(DVec3::Y).clamp(-1., 1.).acos()
                                / std::f64::consts::FRAC_PI_2
                        }
                        Parameter::Altitude => {
                            let (lo, hi) = self.bounds;
                            let h = (hi.y - lo.y).max(1e-9);
                            (t.world.y - lo.y) / h
                        }
                        Parameter::IncidenceAngle => {
                            t.normal.dot(t.toward_viewer).clamp(-1., 1.).acos()
                                / std::f64::consts::FRAC_PI_2
                        }
                        Parameter::Pressure => 1. - d.pressure,
                        // No displaced surface or barrel rotation yet.
                        Parameter::Displacement | Parameter::TangentPressure => 0.,
                    };
                    let span = (1. - pi.inn - pi.out).max(1e-6);
                    let tt = ((raw.clamp(0., 1.) - pi.inn) / span).clamp(0., 1.);
                    // The Smooth tip's shapes fall from 1 to 0; the ink rises.
                    let shaped = (1. - pi.shape.weight(tt)) as f32;
                    if pi.use_as_mask {
                        strength *= 1. - shaped;
                    } else {
                        colour = std::array::from_fn(|k| d.fg[k] + (d.bg[k] - d.fg[k]) * shaped);
                    }
                }
                Ink::Random(ri) => {
                    let r = crate::paint_tips::pcg3d(self.seed, d.dab, 0x1A7E);
                    let signed = |v: u32| (f64::from(v) / f64::from(u32::MAX)) * 2. - 1.;
                    let (h, s, v) = rgb_to_hsv(colour);
                    let h = (h + signed(r[0]) * ri.hue * 0.5).rem_euclid(1.);
                    let s = (s * (1. + signed(r[1]) * ri.saturation)).clamp(0., 1.);
                    let v = (v * (1. + signed(r[2]) * ri.brightness)).max(0.);
                    colour = hsv_to_rgb(h, s, v);
                }
                Ink::Projection(pj) => match self.camera.as_ref().and_then(|c| c.sample(t.world)) {
                    Some(c) => {
                        let gain = 2f32.powf(pj.exposure as f32);
                        colour = [c[0] * gain, c[1] * gain, c[2] * gain];
                        strength *= c[3];
                    }
                    None => strength = 0.,
                },
            }
        }
        (colour, strength)
    }
}

/// The widget's rectangle in view pixels: centre, half width and half
/// height (before Rotation).
pub fn image_ink_frame(ii: &ImageInk, image: Option<(u32, u32)>, view: DVec2) -> (DVec2, DVec2) {
    let (w, h) = image.map_or((4., 3.), |(w, h)| {
        (f64::from(w.max(1)), f64::from(h.max(1)))
    });
    let scale = ii.scale.max(1e-6);
    let height = view.y * scale;
    let width = if ii.fit_view {
        view.x * scale
    } else {
        height * w / h
    };
    (ii.pos.unwrap_or(view / 2.), DVec2::new(width, height) / 2.)
}

/// Image Ink's picture at screen point `q`: its colour and coverage, or
/// `None` outside it when Repeat is off.
pub fn image_ink_sample(
    ii: &ImageInk,
    px: &PaintImage,
    view: DVec2,
    q: DVec2,
) -> Option<([f32; 3], f32)> {
    let (w, h) = (f64::from(px.width.max(1)), f64::from(px.height.max(1)));
    let (centre, half) = image_ink_frame(ii, Some((px.width, px.height)), view);
    let (width, height) = (half.x * 2., half.y * 2.);
    let (sin, cos) = (-ii.rotation).sin_cos();
    let d = q - centre;
    let l = DVec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos);
    let mut u = l.x / width + 0.5;
    let mut v = l.y / height + 0.5;
    let inside = (0. ..=1.).contains(&u) && (0. ..=1.).contains(&v);
    if !inside && !ii.repeat {
        return None;
    }
    u = u.rem_euclid(1.);
    v = v.rem_euclid(1.);
    if ii.flip_h {
        u = 1. - u;
    }
    if ii.flip_v {
        v = 1. - v;
    }
    let p = if ii.pixel_blending {
        px.sample(u as f32 * px.width as f32, v as f32 * px.height as f32)
    } else {
        let x = ((u * w) as u32).min(px.width - 1);
        let y = ((v * h) as u32).min(px.height - 1);
        px.get((y * px.width + x) as usize)
    };
    let mut a = p[3];
    if ii.soft_border > 0. && !ii.repeat {
        let edge = u.min(1. - u).min(v.min(1. - v));
        a *= (edge / (ii.soft_border / 2.)).clamp(0., 1.) as f32;
    }
    Some(([p[0], p[1], p[2]], a))
}

fn blurred_luma(px: &PaintImage, uv: DVec2, radius: f64) -> f32 {
    let (w, h) = (px.width as i64, px.height as i64);
    let x = (uv.x * w as f64) as i64;
    let y = ((1. - uv.y) * h as f64) as i64;
    let r = radius.round().clamp(0., 8.) as i64;
    let (mut sum, mut n) = (0f32, 0f32);
    for dy in -r..=r {
        for dx in -r..=r {
            let (nx, ny) = ((x + dx).clamp(0, w - 1), (y + dy).clamp(0, h - 1));
            let c = px.get((ny * w + nx) as usize);
            sum += operandi_paint::image::luma([c[0], c[1], c[2]]);
            n += 1.;
        }
    }
    sum / n.max(1.)
}

fn rgb_to_hsv(c: [f32; 3]) -> (f64, f64, f64) {
    let [r, g, b] = c.map(f64::from);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= 0. {
        0.
    } else if max == r {
        ((g - b) / d).rem_euclid(6.) / 6.
    } else if max == g {
        ((b - r) / d + 2.) / 6.
    } else {
        ((r - g) / d + 4.) / 6.
    };
    let s = if max > 0. { d / max } else { 0. };
    (h, s, max)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> [f32; 3] {
    let i = (h * 6.).floor();
    let f = h * 6. - i;
    let (p, q, t) = (v * (1. - s), v * (1. - f * s), v * (1. - (1. - f) * s));
    let (r, g, b) = match (i as i64).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [r as f32, g as f32, b as f32]
}

/// The Curve Constraint (PNT-200): the background curves a stroke snaps
/// to, as world-space polylines.
#[derive(Clone, Debug, Default)]
pub struct CurveGuide {
    pub curves: Vec<Vec<DVec3>>,
}

impl CurveGuide {
    /// The visible, unselected meshes' curves, as drawn.
    pub fn background(scene: &crate::Scene) -> CurveGuide {
        let mut curves = Vec::new();
        for item in &scene.items {
            if !item.visible || scene.selection.has_item(item.id) {
                continue;
            }
            let Ok(world) = scene.world_matrix(item.id) else {
                continue;
            };
            let mesh = &item.mesh;
            for (f, face) in mesh.faces.iter().enumerate() {
                let Some(crate::PolygonRepresentation::Polyline { closed, curve }) =
                    mesh.attributes.representations.get(f).copied()
                else {
                    continue;
                };
                if face.vertices.len() < 2 {
                    continue;
                }
                let pts: Vec<DVec3> = face
                    .vertices
                    .iter()
                    .map(|&v| world.transform_point3(mesh.vertices[v as usize]))
                    .collect();
                let mut line =
                    crate::curves::sample_adaptive(&pts, curve, closed, 4f64.to_radians());
                if closed && let Some(&first) = line.first() {
                    line.push(first);
                }
                curves.push(line);
            }
        }
        CurveGuide { curves }
    }

    pub fn is_empty(&self) -> bool {
        self.curves.is_empty()
    }

    /// Background 2D: the nearest point of the curves as seen on screen.
    pub fn snap_screen(&self, view: &PaintView, q: DVec2) -> Option<DVec2> {
        let mut best: Option<(f64, DVec2)> = None;
        for c in &self.curves {
            let pts: Vec<Option<DVec2>> =
                c.iter().map(|&p| view.project(p).map(|(s, _)| s)).collect();
            for w in pts.windows(2) {
                let (Some(a), Some(b)) = (w[0], w[1]) else {
                    continue;
                };
                let p = nearest_on_segment(q, a, b);
                let d = p.distance_squared(q);
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, p));
                }
            }
        }
        best.map(|(_, p)| p)
    }

    /// Background 3D: the nearest point of the curves in space.
    pub fn snap_world(&self, p: DVec3) -> Option<DVec3> {
        let mut best: Option<(f64, DVec3)> = None;
        for c in &self.curves {
            for w in c.windows(2) {
                let (a, b) = (w[0], w[1]);
                let d = b - a;
                let t = if d.length_squared() > 0. {
                    ((p - a).dot(d) / d.length_squared()).clamp(0., 1.)
                } else {
                    0.
                };
                let q = a + d * t;
                let dist = q.distance_squared(p);
                if best.is_none_or(|(bd, _)| dist < bd) {
                    best = Some((dist, q));
                }
            }
        }
        best.map(|(_, q)| q)
    }
}

fn nearest_on_segment(q: DVec2, a: DVec2, b: DVec2) -> DVec2 {
    let d = b - a;
    let t = if d.length_squared() > 0. {
        ((q - a).dot(d) / d.length_squared()).clamp(0., 1.)
    } else {
        0.
    };
    a + d * t
}

/// Curve Constraint (PNT-56, 200).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CurveConstraint {
    #[default]
    None,
    Background2D,
    Background3D,
}

impl CurveConstraint {
    pub const ALL: [CurveConstraint; 3] = [
        CurveConstraint::None,
        CurveConstraint::Background2D,
        CurveConstraint::Background3D,
    ];
    pub fn label(self) -> &'static str {
        match self {
            CurveConstraint::None => "None",
            CurveConstraint::Background2D => "Background 2D",
            CurveConstraint::Background3D => "Background 3D",
        }
    }
    pub fn named(word: &str) -> Option<CurveConstraint> {
        let w: String = word.chars().filter(|c| !c.is_whitespace()).collect();
        CurveConstraint::ALL
            .into_iter()
            .find(|c| c.label().replace(' ', "").eq_ignore_ascii_case(&w))
    }
}

/// A mesh's world bounding box.
pub fn world_bounds(mesh: &crate::Mesh, world: DMat4) -> (DVec3, DVec3) {
    let mut lo = DVec3::splat(f64::INFINITY);
    let mut hi = DVec3::splat(f64::NEG_INFINITY);
    for &v in &mesh.vertices {
        let p = world.transform_point3(v);
        lo = lo.min(p);
        hi = hi.max(p);
    }
    (lo, hi)
}
