//! Painting stage 2: brush strokes on a mesh's image (PNT-40..73). Only
//! built with the `painting` feature.
//!
//! - **Projection (PNT-45).** Dabs are circles on the screen. Every texel
//!   of the image is mapped once per stroke to its surface point (through
//!   the mesh's UVs) and that point to the screen; a texel is painted when
//!   its point is inside a dab, faces the viewer and is the nearest
//!   surface there (a ray against the mesh's BVH). Its weight is the tip's
//!   value at that screen distance, so the dab keeps its on-screen shape
//!   whatever the UVs do, and texels on both sides of a UV seam that land
//!   on the same spot are painted alike. Texels whose centre is just
//!   outside a UV triangle but whose square touches it are mapped too
//!   (clamped onto the triangle), so island borders get paint and filtering
//!   shows no seam.
//! - **Stroke mask (PNT-58..60, 70..72).** Each stroke keeps a per-texel
//!   mask 0..1. Airbrush adds each dab into it (m + (1 − m) × dab), so
//!   going over a spot builds up; Paintbrush and Eraser take the largest
//!   dab (max), so overlaps don't darken. A dab's strength is tip weight ×
//!   Density × pressure × the attenuation ramp × the falloff. The image is
//!   always recomputed from its pixels at the press with mask × Opacity,
//!   so one stroke never passes Opacity.
//! - **Blend (PNT-57).** The shader tree's blend formulas (operandi-paint
//!   `blend::full`), plus Erase, which takes alpha away. The Eraser tool
//!   erases alpha, or paints the BG colour on images without alpha (PNT-72).
//! - The stroke edits its own copy of the pixels; the app shows it while
//!   the stroke goes on and records it as one undo step on release
//!   (`PaintEdit::Stroke`, PNT-50).
use crate::brush_engine::{Tip, TipKind, TipShape};
use crate::{Result, Scene, Symmetry};
use glam::{DMat4, DVec2, DVec3, Vec3};
use operandi_paint::{Blend, PaintImage};
use std::collections::HashMap;
use std::sync::Arc;

/// The paint tools: stage 2's three (PNT-70..72) and stage 3's
/// (PNT-90..111).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PaintTool {
    Airbrush,
    Paintbrush,
    Eraser,
    Smudge,
    Blur,
    Sharpen,
    Clone,
    Line,
    /// Image-space tools: they ignore the tip (PNT-105).
    Fill,
    LinearGradient,
    RadialGradient,
}

impl PaintTool {
    pub const ALL: [PaintTool; 11] = [
        PaintTool::Airbrush,
        PaintTool::Paintbrush,
        PaintTool::Eraser,
        PaintTool::Smudge,
        PaintTool::Blur,
        PaintTool::Sharpen,
        PaintTool::Clone,
        PaintTool::Line,
        PaintTool::Fill,
        PaintTool::LinearGradient,
        PaintTool::RadialGradient,
    ];
    pub fn label(self) -> &'static str {
        match self {
            PaintTool::Airbrush => "Airbrush",
            PaintTool::Paintbrush => "Paintbrush",
            PaintTool::Eraser => "Eraser",
            PaintTool::Smudge => "Smudge",
            PaintTool::Blur => "Blur",
            PaintTool::Sharpen => "Sharpen",
            PaintTool::Clone => "Clone",
            PaintTool::Line => "Line",
            PaintTool::Fill => "Fill",
            PaintTool::LinearGradient => "Linear Gradient",
            PaintTool::RadialGradient => "Radial Gradient",
        }
    }
    /// The toolbox button's label (PNT-92: Sharpen shows as "Sharp").
    pub fn button_label(self) -> &'static str {
        match self {
            PaintTool::Sharpen => "Sharp",
            other => other.label(),
        }
    }
    /// The tool's command (Operandi's own name, PNT-Q01).
    pub fn command(self) -> &'static str {
        match self {
            PaintTool::Airbrush => "operandi.paint.airbrush",
            PaintTool::Paintbrush => "operandi.paint.paintbrush",
            PaintTool::Eraser => "operandi.paint.eraser",
            PaintTool::Smudge => "operandi.paint.smudge",
            PaintTool::Blur => "operandi.paint.blur",
            PaintTool::Sharpen => "operandi.paint.sharpen",
            PaintTool::Clone => "operandi.paint.clone",
            PaintTool::Line => "operandi.paint.line",
            PaintTool::Fill => "operandi.paint.fill",
            PaintTool::LinearGradient => "operandi.paint.linearGradient",
            PaintTool::RadialGradient => "operandi.paint.radialGradient",
        }
    }
    /// Within one stroke, does going over a spot again add more (PNT-70)?
    pub fn builds_up(self) -> bool {
        self == PaintTool::Airbrush
    }
    /// Works on the image itself, not through projected dabs (PNT-105).
    pub fn image_space(self) -> bool {
        matches!(
            self,
            PaintTool::Fill | PaintTool::LinearGradient | PaintTool::RadialGradient
        )
    }
    /// Moves pixels that are already there rather than laying colour
    /// (PNT-90..92).
    pub fn moves_pixels(self) -> bool {
        matches!(
            self,
            PaintTool::Smudge | PaintTool::Blur | PaintTool::Sharpen
        )
    }
    /// The undo step's name (PNT-50).
    pub fn step_label(self) -> String {
        if self.image_space() {
            self.label().to_owned()
        } else {
            format!("{} Stroke", self.label())
        }
    }
}

/// Line's Shape row (PNT-101).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineShape {
    #[default]
    Line,
    Circle,
    Rectangle,
}

impl LineShape {
    pub const ALL: [LineShape; 3] = [LineShape::Line, LineShape::Circle, LineShape::Rectangle];
    pub fn label(self) -> &'static str {
        match self {
            LineShape::Line => "Line",
            LineShape::Circle => "Circle",
            LineShape::Rectangle => "Rectangle",
        }
    }
    /// The outline from press `a` to release `b` on screen: Line a→b,
    /// Circle centred on `a` through `b`, Rectangle with `a` and `b` as
    /// opposite corners (closed shapes repeat their first point).
    pub fn outline(self, a: DVec2, b: DVec2) -> Vec<DVec2> {
        match self {
            LineShape::Line => vec![a, b],
            LineShape::Rectangle => vec![a, DVec2::new(b.x, a.y), b, DVec2::new(a.x, b.y), a],
            LineShape::Circle => {
                let r = a.distance(b);
                let n = ((r * std::f64::consts::TAU / 2.).ceil() as usize).clamp(16, 2048);
                (0..=n)
                    .map(|k| {
                        let t = k as f64 / n as f64 * std::f64::consts::TAU;
                        a + DVec2::new(t.cos(), t.sin()) * r
                    })
                    .collect()
            }
        }
    }
}

/// Blend Mode (PNT-57), in the form's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PaintBlend {
    #[default]
    Normal,
    Erase,
    Add,
    Subtract,
    Difference,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    HardLight,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
}

impl PaintBlend {
    pub const ALL: [PaintBlend; 14] = [
        PaintBlend::Normal,
        PaintBlend::Erase,
        PaintBlend::Add,
        PaintBlend::Subtract,
        PaintBlend::Difference,
        PaintBlend::Multiply,
        PaintBlend::Screen,
        PaintBlend::Overlay,
        PaintBlend::SoftLight,
        PaintBlend::HardLight,
        PaintBlend::Darken,
        PaintBlend::Lighten,
        PaintBlend::ColorDodge,
        PaintBlend::ColorBurn,
    ];
    pub fn label(self) -> &'static str {
        match self {
            PaintBlend::Normal => "Normal",
            PaintBlend::Erase => "Erase",
            PaintBlend::Add => "Add",
            PaintBlend::Subtract => "Subtract",
            PaintBlend::Difference => "Difference",
            PaintBlend::Multiply => "Multiply",
            PaintBlend::Screen => "Screen",
            PaintBlend::Overlay => "Overlay",
            PaintBlend::SoftLight => "Soft Light",
            PaintBlend::HardLight => "Hard Light",
            PaintBlend::Darken => "Darken",
            PaintBlend::Lighten => "Lighten",
            PaintBlend::ColorDodge => "Color Dodge",
            PaintBlend::ColorBurn => "Color Burn",
        }
    }
    pub fn named(word: &str) -> Option<PaintBlend> {
        let w: String = word.chars().filter(|c| !c.is_whitespace()).collect();
        PaintBlend::ALL.into_iter().find(|b| {
            b.label()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .eq_ignore_ascii_case(&w)
        })
    }
    /// The compositor's mode (Erase has none: it takes alpha away).
    fn blend(self) -> Option<Blend> {
        Some(match self {
            PaintBlend::Normal => Blend::Normal,
            PaintBlend::Erase => return None,
            PaintBlend::Add => Blend::Add,
            PaintBlend::Subtract => Blend::Subtract,
            PaintBlend::Difference => Blend::Difference,
            PaintBlend::Multiply => Blend::Multiply,
            PaintBlend::Screen => Blend::Screen,
            PaintBlend::Overlay => Blend::Overlay,
            PaintBlend::SoftLight => Blend::SoftLight,
            PaintBlend::HardLight => Blend::HardLight,
            PaintBlend::Darken => Blend::Darken,
            PaintBlend::Lighten => Blend::Lighten,
            PaintBlend::ColorDodge => Blend::ColorDodge,
            PaintBlend::ColorBurn => Blend::ColorBurn,
        })
    }
}

/// A colour row: colour, its alpha and a multiplier (PNT-62, 63).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintColor {
    pub color: [f64; 3],
    pub alpha: f64,
    /// Above 1 gives brighter than white paint (float images only).
    pub value: f64,
}

impl PaintColor {
    pub fn rgb(&self) -> [f32; 3] {
        self.color.map(|c| (c * self.value) as f32)
    }
}

/// One paint tool's settings (PNT-55..63) with its tip (PNT-80, 81).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintSettings {
    /// Interpolation Step, a share of the tip size (PNT-47).
    pub step: f64,
    pub blend: PaintBlend,
    pub opacity: f64,
    pub density: f64,
    /// Attenuation Steps: 0 off, else the stroke fades out over that many
    /// dabs (PNT-60).
    pub attenuation: u32,
    pub use_falloff: bool,
    pub fg: PaintColor,
    pub bg: PaintColor,
    pub tip: Tip,
    /// Fill Tolerance, 0..1 per RGBA channel (PNT-106).
    pub fill_tolerance: f64,
    /// Line's Shape (PNT-101).
    pub shape: LineShape,
    /// Curve Constraint (PNT-56, 200).
    pub curve: crate::paint_inks::CurveConstraint,
}

/// The paint tools' starting tip: Smooth shape, 10 px (PNT-80, PNT-Q17).
pub fn default_tip() -> Tip {
    Tip {
        kind: TipKind::Smooth,
        shape: TipShape::Smooth,
        ..Tip::default()
    }
}

impl Default for PaintSettings {
    fn default() -> Self {
        Self {
            step: 0.05,
            blend: PaintBlend::Normal,
            opacity: 1.,
            density: 1.,
            attenuation: 0,
            use_falloff: false,
            fg: PaintColor {
                color: [1.; 3],
                alpha: 1.,
                value: 1.,
            },
            // PNT-63: black (provisional, PNT-Q14).
            bg: PaintColor {
                color: [0.; 3],
                alpha: 1.,
                value: 1.,
            },
            tip: default_tip(),
            fill_tolerance: 0.1,
            shape: LineShape::Line,
            curve: crate::paint_inks::CurveConstraint::None,
        }
    }
}

impl PaintSettings {
    pub fn validate(&self) -> Result<()> {
        let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
        if !ok(self.step, 0.01, 10.) {
            return Err("Interpolation Step must be from 1 % to 1000 %".into());
        }
        if !ok(self.opacity, 0., 1.) || !ok(self.density, 0., 1.) {
            return Err("Opacity and Density are 0 % to 100 %".into());
        }
        if !ok(self.fg.alpha, 0., 1.) || !ok(self.bg.alpha, 0., 1.) {
            return Err("Alpha is 0 % to 100 %".into());
        }
        if !ok(self.fill_tolerance, 0., 1.) {
            return Err("Fill Tolerance is 0 % to 100 %".into());
        }
        if !ok(self.fg.value, 0., 100.) || !ok(self.bg.value, 0., 100.) {
            return Err("Value is 0 % to 10000 %".into());
        }
        if !ok(
            self.tip.size,
            crate::brush_engine::SIZE_RANGE.0,
            crate::brush_engine::SIZE_RANGE.1,
        ) {
            return Err("Size must be from 1 to 2000 pixels".into());
        }
        Ok(())
    }
}

/// The view a stroke is painted through: the app's camera, as numbers
/// (screen points, y down; the same projection as the viewport's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintView {
    pub eye: DVec3,
    pub right: DVec3,
    pub up: DVec3,
    pub forward: DVec3,
    pub perspective: bool,
    /// Orthographic views: the view distance that sets the scale.
    pub distance: f64,
    pub near: f64,
    pub lens: f64,
    /// The viewport's centre and size in screen points.
    pub center: DVec2,
    pub size: DVec2,
}

impl PaintView {
    /// Screen point and depth of world point `p`; `None` behind the eye.
    pub fn project(&self, p: DVec3) -> Option<(DVec2, f64)> {
        let d = p - self.eye;
        let z = d.dot(self.forward);
        if z < self.near {
            return None;
        }
        let scale = self.size.y * self.lens / if self.perspective { z } else { self.distance };
        Some((
            DVec2::new(
                self.center.x + d.dot(self.right) * scale,
                self.center.y - d.dot(self.up) * scale,
            ),
            z,
        ))
    }
    /// Where a ray toward `p` starts (the eye, or the view plane).
    fn ray_origin(&self, p: DVec3) -> DVec3 {
        if self.perspective {
            self.eye
        } else {
            p - self.forward * (p - self.eye).dot(self.forward)
        }
    }
    /// Unit vector from `p` toward the viewer.
    fn toward_viewer(&self, p: DVec3) -> DVec3 {
        if self.perspective {
            (self.eye - p).normalize_or(-self.forward)
        } else {
            -self.forward
        }
    }
    /// A view looking at `target` from `eye` (tests and scripts).
    pub fn looking(eye: DVec3, target: DVec3, size: DVec2) -> PaintView {
        let forward = (target - eye).normalize();
        let world_up = if forward.cross(DVec3::Y).length() < 1e-6 {
            DVec3::Z
        } else {
            DVec3::Y
        };
        let right = forward.cross(world_up).normalize();
        let up = right.cross(forward);
        PaintView {
            eye,
            right,
            up,
            forward,
            perspective: true,
            distance: (target - eye).length(),
            near: 1e-4,
            lens: 1.,
            center: size / 2.,
            size,
        }
    }
}

/// One texel the stroke can reach.
#[derive(Clone, Copy, Debug)]
struct Texel {
    /// Pixel index in the image.
    pixel: u32,
    /// The surface point in the mesh's own space (for visibility rays; the
    /// world point is `PaintStroke::world_of`).
    local: Vec3,
    screen: DVec2,
    /// The active falloff's weight here (1 without one).
    falloff: f32,
    /// The polygon it lies on (Projection Ink's Stamp; its world normal
    /// for Parametric Ink is `PaintStroke::normals`).
    face: u32,
}

/// Screen bins for finding the texels under a dab.
const CELL: f64 = 16.;
/// How far (points) Smudge and Clone look for the texel under a screen
/// point: half a cell, so texels up to about 11 points apart (a small
/// image seen close up) still have one in reach. Provisional (PNT-Q74).
const SAMPLE_REACH: f64 = CELL / 2.;

/// A paint stroke in progress (PNT-45..60).
pub struct PaintStroke {
    pub tool: PaintTool,
    pub settings: PaintSettings,
    pub image: u64,
    view: PaintView,
    base: Arc<PaintImage>,
    /// The pixels painted so far.
    current: PaintImage,
    /// Per pixel: the stroke mask, 0..1.
    mask: Vec<f32>,
    texels: Vec<Texel>,
    cells: HashMap<(i32, i32), Vec<u32>>,
    /// Per texel: 0 not yet tested, 1 visible, 2 hidden.
    visible: Vec<u8>,
    /// The surface's texels not yet placed in the view (`None`: all are).
    lazy: Option<Box<Lazy>>,
    bvh: Arc<crate::bake::bvh::Bvh>,
    /// Per polygon, its world normal.
    normals: Vec<DVec3>,
    world: DMat4,
    symmetry: Symmetry,
    dabs: u32,
    /// Pixels changed since the last `take_dirty`: (x0, y0, x1, y1).
    dirty: Option<(u32, u32, u32, u32)>,
    /// Any pixel changed in the whole stroke.
    pub changed: bool,
    /// Clone (PNT-96): the screen offset from the brush to the source.
    pub clone_offset: Option<DVec2>,
    /// Clone: each painted pixel's source colour.
    source: HashMap<u32, [f32; 4]>,
    /// Smudge: the last dab's centre (the drag direction, PNT-90).
    last_center: Option<DVec2>,
    /// Smudge: where the colour was last pulled from, and how far apart
    /// the texels sit on screen (measured at the first dab). See `smudge`.
    smudge_from: Option<DVec2>,
    texel_spacing: Option<f64>,
    /// The tip beyond its basic profile (paint_tips.rs, stage 4).
    pub extra: crate::paint_tips::TipExtra,
    /// How far along the stroke the dabs have come, in tip radii (Roller).
    travel: f64,
    /// The inks in use (stage 5, paint_inks.rs).
    pub inks: crate::paint_inks::InkRun,
    /// The current dab's pen pressure (Parametric Ink).
    pressure: f64,
    /// Background 3D (PNT-200): only surface within this world distance of
    /// this point takes the dab.
    pub near: Option<(DVec3, f64)>,
    /// A UV-view stroke (PNT-215): how the view maps UVs to the screen,
    /// and each reached pixel's texel.
    uv: Option<UvScreen>,
    uv_texel: HashMap<u32, u32>,
}

/// How a UV view shows UV space: its centre on screen, the UV point there
/// and screen points per UV unit (v up).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvScreen {
    pub screen_center: DVec2,
    pub uv_center: DVec2,
    pub zoom: f64,
    /// The view's size (Image Ink fits its height).
    pub size: DVec2,
}

/// Everything a UV-view stroke needs at the press.
pub struct UvStrokeSetup {
    pub image: u64,
    pub pixels: Arc<PaintImage>,
    pub uv: UvScreen,
    pub tool: PaintTool,
    pub settings: PaintSettings,
}

/// Everything a stroke needs at the press.
pub struct StrokeSetup<'a> {
    pub scene: &'a Scene,
    pub item: u64,
    pub image: u64,
    pub pixels: Arc<PaintImage>,
    /// The UV map the target layer reads (else the mesh's own).
    pub uv_map: Option<&'a str>,
    pub view: PaintView,
    pub tool: PaintTool,
    pub settings: PaintSettings,
    pub symmetry: Symmetry,
    pub falloff: Option<&'a crate::ActiveFalloff>,
    /// The Lasso falloff (PNT-115..120), used with Use Falloff.
    pub lasso: Option<&'a crate::paint_lasso::Lasso>,
}

impl PaintStroke {
    pub fn begin(setup: StrokeSetup<'_>) -> Result<PaintStroke> {
        setup.settings.validate()?;
        let scene = setup.scene;
        let item = scene.item(setup.item).ok_or("No such item")?;
        if !item.is_mesh() {
            return Err(format!("{} isn't a mesh", item.name));
        }
        let mesh = &item.mesh;
        if mesh.uv_maps.is_empty() {
            return Err(crate::paint::NO_UV.into());
        }
        let px = setup.pixels;
        if px.is_empty() {
            return Err(crate::paint::EMPTY_IMAGE.into());
        }
        let world = scene.world_matrix(setup.item)?;
        let normal_m = glam::DMat3::from_mat4(world).inverse().transpose();
        let map_name = crate::uv_tools::target_uv_map(mesh, setup.uv_map);
        let falloff = match (setup.settings.use_falloff, setup.falloff) {
            (true, Some(f)) => Some(f.weights(scene, setup.item)?),
            _ => None,
        };
        // The view-independent part (which pixel lies where on the
        // surface) is the same stroke after stroke: it is kept per mesh,
        // level, UV map and image size (`surface_texels`), so a press only
        // places it in this view. A falloff's weights change it, so a
        // stroke with one rasterises afresh.
        let surface = match &falloff {
            None => cached_surface(item, &map_name, px.width, px.height),
            Some(f) => Arc::new(surface_texels(
                item,
                &map_name,
                px.width,
                px.height,
                Some(f),
            )),
        };
        let (w, h) = (px.width, px.height);
        let view = setup.view;
        let normals: Vec<DVec3> = surface
            .normals
            .iter()
            .map(|n| (normal_m * *n).normalize_or_zero())
            .collect();
        // Texels are placed in the view a screen tile at a time, as dabs
        // first reach it (`Lazy`): a press only places each polygon's box.
        let lazy = Lazy::new(
            surface.clone(),
            &view,
            world,
            setup
                .settings
                .use_falloff
                .then_some(setup.lasso)
                .flatten()
                .cloned(),
            falloff.is_some(),
        );
        let bvh = surface.bvh.clone();
        Ok(PaintStroke {
            tool: setup.tool,
            settings: setup.settings,
            image: setup.image,
            view,
            current: (*px).clone(),
            mask: vec![0.; w as usize * h as usize],
            base: px,
            texels: Vec::new(),
            cells: HashMap::new(),
            visible: Vec::new(),
            lazy: Some(Box::new(lazy)),
            bvh,
            normals,
            world,
            symmetry: setup.symmetry,
            dabs: 0,
            dirty: None,
            changed: false,
            clone_offset: None,
            source: HashMap::new(),
            last_center: None,
            smudge_from: None,
            texel_spacing: None,
            extra: crate::paint_tips::TipExtra::Basic,
            travel: 0.,
            inks: Default::default(),
            pressure: 1.,
            near: None,
            uv: None,
            uv_texel: HashMap::new(),
        })
    }

    /// A stroke straight into the image as the UV view shows it (PNT-215):
    /// dab sizes are UV-view screen points, no projection and no
    /// hidden-surface test. Every tool works: the texels are the image's
    /// pixels, made as dabs reach them.
    pub fn begin_uv(setup: UvStrokeSetup) -> Result<PaintStroke> {
        setup.settings.validate()?;
        let px = setup.pixels;
        if px.is_empty() {
            return Err(crate::paint::EMPTY_IMAGE.into());
        }
        let (w, h) = (px.width, px.height);
        Ok(PaintStroke {
            tool: setup.tool,
            settings: setup.settings,
            image: setup.image,
            view: PaintView::looking(DVec3::Z, DVec3::ZERO, setup.uv.size),
            current: (*px).clone(),
            mask: vec![0.; w as usize * h as usize],
            base: px,
            texels: Vec::new(),
            cells: HashMap::new(),
            lazy: None,
            visible: Vec::new(),
            bvh: Arc::new(crate::bake::bvh::Bvh::new(Vec::new())),
            normals: vec![DVec3::Z],
            world: DMat4::IDENTITY,
            symmetry: Symmetry::default(),
            dabs: 0,
            dirty: None,
            changed: false,
            clone_offset: None,
            source: HashMap::new(),
            last_center: None,
            smudge_from: None,
            texel_spacing: None,
            extra: crate::paint_tips::TipExtra::Basic,
            travel: 0.,
            inks: Default::default(),
            pressure: 1.,
            near: None,
            uv: Some(setup.uv),
            uv_texel: HashMap::new(),
        })
    }

    /// The UV view's screen point for pixel `p`.
    fn uv_screen(&self, p: u32) -> DVec2 {
        let v = self.uv.expect("a UV-view stroke");
        let uv = self.pixel_uv(p);
        DVec2::new(
            v.screen_center.x + (uv.x - v.uv_center.x) * v.zoom,
            v.screen_center.y - (uv.y - v.uv_center.y) * v.zoom,
        )
    }

    /// The UV point under UV-view screen point `q`.
    pub fn uv_at(&self, q: DVec2) -> Option<DVec2> {
        let v = self.uv?;
        Some(DVec2::new(
            v.uv_center.x + (q.x - v.screen_center.x) / v.zoom,
            v.uv_center.y - (q.y - v.screen_center.y) / v.zoom,
        ))
    }

    /// The pixel under UV-view screen point `q`, if on the image.
    fn uv_pixel(&self, q: DVec2) -> Option<u32> {
        let uv = self.uv_at(q)?;
        let (w, h) = (f64::from(self.base.width), f64::from(self.base.height));
        let (x, y) = ((uv.x * w).floor(), ((1. - uv.y) * h).floor());
        (x >= 0. && y >= 0. && x < w && y < h).then(|| y as u32 * self.base.width + x as u32)
    }

    /// `uv_pixel` with the point clamped onto the image (for bounds).
    fn uv_pixel_clamped(&self, q: DVec2) -> Option<u32> {
        let uv = self.uv_at(q)?;
        let (w, h) = (f64::from(self.base.width), f64::from(self.base.height));
        let x = (uv.x * w).floor().clamp(0., w - 1.);
        let y = ((1. - uv.y) * h).floor().clamp(0., h - 1.);
        Some(y as u32 * self.base.width + x as u32)
    }

    /// Pixel `p`'s texel in a UV-view stroke, made when first reached.
    fn uv_texel_of(&mut self, p: u32) -> u32 {
        if let Some(&i) = self.uv_texel.get(&p) {
            return i;
        }
        let i = self.texels.len() as u32;
        let screen = self.uv_screen(p);
        let uv = self.pixel_uv(p);
        self.texels.push(Texel {
            pixel: p,
            local: Vec3::new(uv.x as f32, uv.y as f32, 0.),
            screen,
            falloff: 1.,
            face: 0,
        });
        self.visible.push(1);
        self.uv_texel.insert(p, i);
        i
    }

    /// The texels of the pixels within `reach` of `center` (UV view).
    fn uv_texels_near(&mut self, center: DVec2, reach: f64) -> Vec<u32> {
        let (Some(a), Some(b)) = (
            self.uv_pixel_clamped(center - DVec2::splat(reach)),
            self.uv_pixel_clamped(center + DVec2::splat(reach)),
        ) else {
            return Vec::new();
        };
        let wd = self.base.width;
        let (x0, x1) = ((a % wd).min(b % wd), (a % wd).max(b % wd));
        let (y0, y1) = ((a / wd).min(b / wd), (a / wd).max(b / wd));
        let mut out = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                out.push(self.uv_texel_of(y * wd + x));
            }
        }
        out
    }

    /// How many texels of the image the stroke can reach from this view.
    pub fn reachable(&mut self) -> usize {
        self.place_all();
        self.texels.len()
    }

    /// The pixels painted so far.
    pub fn pixels(&self) -> &PaintImage {
        &self.current
    }

    /// The pixels at the press.
    pub fn base(&self) -> &Arc<PaintImage> {
        &self.base
    }

    /// One dab at screen point `center` (screen points, y down) of
    /// `radius` screen points, at world point `surface` (for symmetry), with
    /// pen `pressure` (PNT-49).
    pub fn dab(&mut self, center: DVec2, radius: f64, surface: Option<DVec3>, pressure: f64) {
        self.dab_turned(center, radius, surface, pressure, 0.);
    }

    /// `dab` with the tip turned by `angle` radians on screen (the
    /// nozzle's Rotation, PNT-141; Stamp's dragged angle, PNT-132).
    pub fn dab_turned(
        &mut self,
        center: DVec2,
        radius: f64,
        surface: Option<DVec3>,
        pressure: f64,
        angle: f64,
    ) {
        // PNT-60: the stroke runs out of paint after Attenuation Steps dabs.
        let fade = match self.settings.attenuation {
            0 => 1.,
            n => (1. - f64::from(self.dabs) / f64::from(n)).max(0.),
        };
        self.dabs += 1;
        if fade <= 0. {
            return;
        }
        let strength = (self.settings.density * pressure.clamp(0., 1.) * fade) as f32;
        self.pressure = pressure.clamp(0., 1.);
        if let Some(last) = self.last_center
            && radius > 0.
        {
            self.travel += last.distance(center) / radius;
        }
        self.stamp(center, radius, strength, angle);
        // PNT-54: the mirror dab, projected from the mirrored surface point.
        if self.symmetry.axis.is_some()
            && let Some(p) = surface
        {
            let inv = self.world.inverse();
            let mirrored = self
                .world
                .transform_point3(self.symmetry.mirror(inv.transform_point3(p)));
            if let Some((q, _)) = self.view.project(mirrored)
                && q.distance(center) > 0.5
            {
                self.stamp(q, radius, strength, -angle);
            }
        }
    }

    fn stamp(&mut self, center: DVec2, radius: f64, strength: f32, angle: f64) {
        if !(radius > 0. && strength > 0.) {
            return;
        }
        let reach = radius * self.extra.reach();
        let r2 = reach * reach;
        self.place_near(center, reach);
        let (cx0, cy0) = (
            ((center.x - reach) / CELL).floor() as i32,
            ((center.y - reach) / CELL).floor() as i32,
        );
        let (cx1, cy1) = (
            ((center.x + reach) / CELL).floor() as i32,
            ((center.y + reach) / CELL).floor() as i32,
        );
        let tip = self.settings.tip;
        let travel = self.travel;
        let mut hits: Vec<(u32, f32, Option<[f32; 3]>)> = Vec::new();
        // The image-space stroke (UV view) has one cell: the pixels under
        // the dab.
        let uv_list = self
            .uv
            .is_some()
            .then(|| self.uv_texels_near(center, reach));
        let cell_keys: Vec<(i32, i32)> = if uv_list.is_some() {
            vec![(0, 0)]
        } else {
            (cy0..=cy1)
                .flat_map(|cy| (cx0..=cx1).map(move |cx| (cx, cy)))
                .collect()
        };
        {
            for (cx, cy) in cell_keys {
                let list: &[u32] = match &uv_list {
                    Some(l) => l,
                    None => match self.cells.get(&(cx, cy)) {
                        Some(l) => l,
                        None => continue,
                    },
                };
                for &i in list {
                    let t = &self.texels[i as usize];
                    let d2 = t.screen.distance_squared(center);
                    if d2 > r2 {
                        continue;
                    }
                    if let Some((p, r)) = self.near
                        && self.world_of(t).distance_squared(p) > r * r
                    {
                        continue;
                    }
                    let d = (t.screen - center) / radius;
                    let uv = {
                        let (w, h) = (f64::from(self.base.width), f64::from(self.base.height));
                        let (x, y) = (
                            f64::from(t.pixel % self.base.width),
                            f64::from(t.pixel / self.base.width),
                        );
                        DVec2::new((x + 0.5) / w, 1. - (y + 0.5) / h)
                    };
                    if let Some((w, colour)) = self.extra.eval(&tip, d, angle, uv, travel) {
                        hits.push((i, w, colour));
                    }
                }
            }
        }
        let mut weighted = Vec::with_capacity(hits.len());
        let mut colours = Vec::new();
        for (i, w, colour) in hits {
            if !self.is_visible(i as usize) {
                continue;
            }
            let t = self.texels[i as usize];
            let (colour, ink) = self.ink(&t, colour);
            let dab = (w * strength * t.falloff * ink).clamp(0., 1.);
            if dab > 0. {
                weighted.push((i, dab));
                colours.push(colour);
            }
        }
        match self.tool {
            PaintTool::Smudge => self.smudge(center, &weighted),
            PaintTool::Blur | PaintTool::Sharpen => self.blur(&weighted),
            _ => self.lay(center, &weighted, &colours),
        }
        self.last_center = Some(center);
    }

    /// The inks' colour and strength at texel `t` (PNT-170..192): the
    /// colour stays `tip`'s without inks or on tools that lay no colour of
    /// their own.
    fn ink(&self, t: &Texel, tip: Option<[f32; 3]>) -> (Option<[f32; 3]>, f32) {
        if self.inks.is_empty() {
            return (tip, 1.);
        }
        let s = &self.settings;
        let texel = crate::paint_inks::InkTexel {
            screen: t.screen,
            world: self.world_of(t),
            normal: self.normals[t.face as usize],
            toward_viewer: self.view.toward_viewer(self.world_of(t)),
            uv: self.pixel_uv(t.pixel),
        };
        let start = tip.unwrap_or_else(|| s.fg.rgb());
        let dab = crate::paint_inks::InkDab {
            dab: self.dabs,
            pressure: self.pressure,
            fg: s.fg.rgb(),
            bg: s.bg.rgb(),
        };
        let (c, k) = self.inks.apply(&texel, &dab, start, 1.);
        let lays_colour = !matches!(
            self.tool,
            PaintTool::Clone
                | PaintTool::Eraser
                | PaintTool::Smudge
                | PaintTool::Blur
                | PaintTool::Sharpen
        );
        (if lays_colour { Some(c) } else { tip }, k)
    }

    /// Background 3D (PNT-200): a dab at curve point `on_curve` that only
    /// reaches surface within `world_radius` of it.
    pub fn dab_near(
        &mut self,
        center: DVec2,
        radius: f64,
        on_curve: DVec3,
        world_radius: f64,
        pressure: f64,
    ) {
        self.near = Some((on_curve, world_radius));
        self.dab(center, radius, Some(on_curve), pressure);
        self.near = None;
    }

    /// Image or Projection Ink's Stamp (PNT-172, 191): the inks laid over
    /// every visible texel at once, or only those on `faces` when given.
    pub fn ink_stamp(&mut self, faces: Option<&[u32]>) {
        self.dabs += 1;
        let strength = self.settings.density as f32;
        let mut weighted = Vec::new();
        let mut colours = Vec::new();
        let faces: Option<std::collections::HashSet<u32>> =
            faces.map(|f| f.iter().copied().collect());
        self.place_all();
        for i in 0..self.texels.len() {
            let t = self.texels[i];
            if faces.as_ref().is_some_and(|f| !f.contains(&t.face)) || !self.is_visible(i) {
                continue;
            }
            let (colour, ink) = self.ink(&t, None);
            let dab = (strength * t.falloff * ink).clamp(0., 1.);
            if dab > 0. {
                weighted.push((i as u32, dab));
                colours.push(colour);
            }
        }
        self.lay(DVec2::ZERO, &weighted, &colours);
    }

    /// Colour-laying tools: the stroke mask (PNT-58..60, 70..72); Clone
    /// takes each pixel's colour from the source (PNT-96).
    fn lay(&mut self, center: DVec2, hits: &[(u32, f32)], colours: &[Option<[f32; 3]>]) {
        let offset = self.clone_offset;
        // Clone's source texels, binned once for the whole dab.
        let bins = match offset {
            Some(offset) if self.tool == PaintTool::Clone => {
                let reach = self.hits_reach(center, hits) + SAMPLE_REACH;
                self.screen_bins(center + offset, reach)
            }
            _ => HashMap::new(),
        };
        for (k, &(i, dab)) in hits.iter().enumerate() {
            let t = self.texels[i as usize];
            // A tip with its own colour (Sphere, Image) lays that colour.
            if self.tool != PaintTool::Clone
                && self.tool != PaintTool::Eraser
                && let Some(Some(c)) = colours.get(k)
            {
                let m = self.mask[t.pixel as usize];
                if dab > m {
                    self.source
                        .insert(t.pixel, [c[0], c[1], c[2], self.settings.fg.alpha as f32]);
                }
            }
            if self.tool == PaintTool::Clone {
                // PNT-95: no source, no paint.
                let Some(offset) = offset else {
                    return;
                };
                let Some(src) = self.sample(&bins, &self.base.clone(), t.screen + offset) else {
                    continue;
                };
                self.source.insert(t.pixel, src);
            }
            let m = &mut self.mask[t.pixel as usize];
            let next = if self.tool.builds_up() {
                *m + (1. - *m) * dab
            } else {
                m.max(dab)
            };
            if next <= *m {
                continue;
            }
            *m = next;
            self.repaint(t.pixel);
        }
        let _ = center;
    }

    /// PNT-90: each texel takes the colour that was a step back along the
    /// drag, mixed by the dab.
    fn smudge(&mut self, center: DVec2, hits: &[(u32, f32)]) {
        let Some(from) = self.smudge_from.or(self.last_center) else {
            return;
        };
        let delta = center - from;
        if delta.length_squared() < 1e-12 {
            return;
        }
        // Dabs come every couple of points, but a small image seen close up
        // has its texels further apart than that, and pulling colour from
        // under the dab itself moves nothing (test drive 2026-11-26). So the
        // pull waits until the drag has gone at least one texel; with dense
        // texels that is every dab, as before.
        let spacing = match self.texel_spacing {
            Some(s) => s,
            None => {
                let s = self.measure_texel_spacing(hits);
                self.texel_spacing = Some(s);
                s
            }
        };
        if delta.length() < spacing {
            self.smudge_from = Some(from);
            return;
        }
        self.smudge_from = Some(center);
        let opacity = self.settings.opacity as f32;
        let before = self.current.clone();
        let reach = self.hits_reach(center, hits) + SAMPLE_REACH;
        let bins = self.screen_bins(center - delta, reach);
        for &(i, dab) in hits {
            let t = self.texels[i as usize];
            let Some(src) = self.sample(&bins, &before, t.screen - delta) else {
                continue;
            };
            let cur = before.get(t.pixel as usize);
            let k = (dab * opacity).clamp(0., 1.);
            let out: [f32; 4] = std::array::from_fn(|c| cur[c] + (src[c] - cur[c]) * k);
            self.write(t.pixel, out);
        }
    }

    /// How far apart neighbouring texels sit on screen near the dab's
    /// texels `hits` (median of each one's nearest other texel, up to 32 of
    /// them); never under one point.
    fn measure_texel_spacing(&self, hits: &[(u32, f32)]) -> f64 {
        let mut nearest: Vec<f64> = hits
            .iter()
            .take(32)
            .filter_map(|&(i, _)| {
                let p = self.texels[i as usize].screen;
                let (cx, cy) = ((p.x / CELL).floor() as i32, (p.y / CELL).floor() as i32);
                let mut best = f64::INFINITY;
                for y in cy - 1..=cy + 1 {
                    for x in cx - 1..=cx + 1 {
                        for &j in self.cells.get(&(x, y)).map(Vec::as_slice).unwrap_or(&[]) {
                            if j != i {
                                best = best.min(self.texels[j as usize].screen.distance(p));
                            }
                        }
                    }
                }
                best.is_finite().then_some(best)
            })
            .collect();
        if nearest.is_empty() {
            return 1.;
        }
        nearest.sort_by(f64::total_cmp);
        nearest[nearest.len() / 2].max(1.)
    }

    /// PNT-91, 92: a small image-space Gaussian (radius 2 texels × Density)
    /// mixed in by the dab; Sharpen pushes away from it (unsharp mask).
    fn blur(&mut self, hits: &[(u32, f32)]) {
        let radius = (2. * self.settings.density).max(0.5);
        let opacity = self.settings.opacity as f32;
        let sharpen = self.tool == PaintTool::Sharpen;
        let (w, h) = (self.current.width as i64, self.current.height as i64);
        let reach = radius.ceil() as i64;
        let sigma = radius.max(0.5) / 1.5;
        let mut out = Vec::with_capacity(hits.len());
        for &(i, dab) in hits {
            let p = self.texels[i as usize].pixel as i64;
            let (x, y) = (p % w, p / w);
            let mut sum = [0f32; 4];
            let mut total = 0f32;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let (nx, ny) = ((x + dx).clamp(0, w - 1), (y + dy).clamp(0, h - 1));
                    let g = (-((dx * dx + dy * dy) as f64) / (2. * sigma * sigma)).exp() as f32;
                    let c = self.current.get((ny * w + nx) as usize);
                    for k in 0..4 {
                        sum[k] += c[k] * g;
                    }
                    total += g;
                }
            }
            let avg = sum.map(|v| v / total);
            let cur = self.current.get(p as usize);
            let k = (dab * opacity).clamp(0., 1.);
            let px: [f32; 4] = std::array::from_fn(|c| {
                if sharpen {
                    cur[c] + (cur[c] - avg[c]) * k
                } else {
                    cur[c] + (avg[c] - cur[c]) * k
                }
            });
            out.push((p as u32, px));
        }
        for (p, px) in out {
            self.write(p, px);
        }
    }

    /// How far the dab's texels `hits` reach from its `center`.
    fn hits_reach(&self, center: DVec2, hits: &[(u32, f32)]) -> f64 {
        hits.iter()
            .map(|&(i, _)| self.texels[i as usize].screen.distance(center))
            .fold(0., f64::max)
    }

    /// Screen bins one point wide over the texels within `reach` of
    /// `center`, each holding the texel nearest its middle, so [`sample`]
    /// finds the texel under a point in a few lookups however densely the
    /// texels lie. Smudge and Clone ask once per texel under the dab; a
    /// scan of the 16-point cells for each froze the app with a zoomed-out
    /// 1024 image (test drive 2026-11-27). Empty in the UV view, which
    /// looks pixels up directly.
    ///
    /// [`sample`]: Self::sample
    fn screen_bins(&mut self, center: DVec2, reach: f64) -> HashMap<(i32, i32), u32> {
        let mut bins: HashMap<(i32, i32), (f64, u32)> = HashMap::new();
        if self.uv.is_some() {
            return HashMap::new();
        }
        self.place_near(center, reach);
        let r2 = reach * reach;
        let (cx0, cy0) = (
            ((center.x - reach) / CELL).floor() as i32,
            ((center.y - reach) / CELL).floor() as i32,
        );
        let (cx1, cy1) = (
            ((center.x + reach) / CELL).floor() as i32,
            ((center.y + reach) / CELL).floor() as i32,
        );
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                for &i in self.cells.get(&(cx, cy)).map(Vec::as_slice).unwrap_or(&[]) {
                    let p = self.texels[i as usize].screen;
                    if p.distance_squared(center) > r2 {
                        continue;
                    }
                    let key = (p.x.floor() as i32, p.y.floor() as i32);
                    let middle = DVec2::new(f64::from(key.0) + 0.5, f64::from(key.1) + 0.5);
                    let d = p.distance_squared(middle);
                    let slot = bins.entry(key).or_insert((d, i));
                    if d < slot.0 {
                        *slot = (d, i);
                    }
                }
            }
        }
        bins.into_iter().map(|(k, (_, i))| (k, i)).collect()
    }

    /// The colour of `image` where the view shows screen point `q`: the
    /// nearest texel in `bins` within `SAMPLE_REACH` points, when it is
    /// visible. Zoomed in on a small image the texels sit several points
    /// apart, so a two-point reach found none and Smudge and Clone painted
    /// nothing (test drive 2026-11-26).
    fn sample(
        &mut self,
        bins: &HashMap<(i32, i32), u32>,
        image: &PaintImage,
        q: DVec2,
    ) -> Option<[f32; 4]> {
        if self.uv.is_some() {
            return self.uv_pixel(q).map(|p| image.get(p as usize));
        }
        let (qx, qy) = (q.x.floor() as i32, q.y.floor() as i32);
        let mut best: Option<(f64, u32)> = None;
        // Rings of bins outward; a texel in ring r is at least r - 1 points
        // away, so once one that near is found, no further ring can beat it.
        for ring in 0..=SAMPLE_REACH.ceil() as i32 {
            if best.is_some_and(|(d, _)| d.sqrt() <= f64::from(ring - 1)) {
                break;
            }
            for y in qy - ring..=qy + ring {
                for x in qx - ring..=qx + ring {
                    if (x - qx).abs() != ring && (y - qy).abs() != ring {
                        continue;
                    }
                    let Some(&i) = bins.get(&(x, y)) else {
                        continue;
                    };
                    let d = self.texels[i as usize].screen.distance_squared(q);
                    if d <= SAMPLE_REACH * SAMPLE_REACH && best.is_none_or(|(b, _)| d < b) {
                        best = Some((d, i));
                    }
                }
            }
        }
        let (_, i) = best?;
        if !self.is_visible(i as usize) {
            return None;
        }
        Some(image.get(self.texels[i as usize].pixel as usize))
    }

    fn write(&mut self, p: u32, px: [f32; 4]) {
        self.current.set(p as usize, px);
        self.changed = true;
        self.mark(p);
    }

    fn mark(&mut self, p: u32) {
        let (x, y) = (p % self.current.width, p / self.current.width);
        self.dirty = Some(match self.dirty {
            None => (x, y, x, y),
            Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
        });
    }

    /// The pixel the view shows at screen point `q` (a click for Fill or a
    /// gradient end, PNT-106, 108): the nearest reachable visible texel.
    pub fn pixel_at(&mut self, q: DVec2) -> Option<u32> {
        if self.uv.is_some() {
            return self.uv_pixel(q);
        }
        self.place_near(q, 2. * CELL);
        let (cx, cy) = ((q.x / CELL).floor() as i32, (q.y / CELL).floor() as i32);
        let mut best: Option<(f64, u32)> = None;
        for y in cy - 1..=cy + 1 {
            for x in cx - 1..=cx + 1 {
                for &i in self.cells.get(&(x, y)).map(Vec::as_slice).unwrap_or(&[]) {
                    let d = self.texels[i as usize].screen.distance_squared(q);
                    if d <= 9. && best.is_none_or(|(b, _)| d < b) {
                        best = Some((d, i));
                    }
                }
            }
        }
        let (_, i) = best?;
        self.is_visible(i as usize)
            .then(|| self.texels[i as usize].pixel)
    }

    /// A pixel's centre in UV space (v up).
    pub fn pixel_uv(&self, p: u32) -> DVec2 {
        let (w, h) = (f64::from(self.base.width), f64::from(self.base.height));
        let (x, y) = (
            f64::from(p % self.base.width),
            f64::from(p / self.base.width),
        );
        DVec2::new((x + 0.5) / w, 1. - (y + 0.5) / h)
    }

    /// Use Falloff for the image-space tools: each pixel's weight, from
    /// the texels this view reaches (others 0; PNT-Q31). `None` with Use
    /// Falloff off.
    fn pixel_weights(&mut self) -> Option<Vec<f32>> {
        if !self.settings.use_falloff || self.uv.is_some() {
            return None;
        }
        let mut w = vec![0f32; self.mask.len()];
        self.place_all();
        for i in 0..self.texels.len() {
            if self.is_visible(i) {
                let t = self.texels[i];
                w[t.pixel as usize] = t.falloff;
            }
        }
        Some(w)
    }

    /// PNT-106 Fill: every pixel of the whole image within Fill Tolerance
    /// of the seed pixel's colour (each RGBA channel), not only the
    /// connected region, takes the FG colour.
    pub fn fill(&mut self, seed: u32) {
        let s = self.settings;
        let seed_c = self.base.get(seed as usize);
        let tol = s.fill_tolerance as f32 + 1e-6;
        let weights = self.pixel_weights();
        let has_alpha = self.base.format.has_alpha();
        let amount = (s.opacity * s.density * s.fg.alpha) as f32;
        for p in 0..self.base.pixel_count() {
            let c = self.base.get(p);
            if (0..4).any(|k| (c[k] - seed_c[k]).abs() > tol) {
                continue;
            }
            let w = weights.as_ref().map_or(1., |w| w[p]);
            if w <= 0. {
                continue;
            }
            let out = mix_color(s.blend, c, s.fg.rgb(), amount * w, has_alpha);
            self.write(p as u32, out);
        }
    }

    /// PNT-108, 109: the whole image flooded with a ramp from FG at `a` to
    /// BG at `b` (UV space): straight across for Linear, outward from `a`
    /// for Radial; FG before the start, BG past the end.
    pub fn gradient(&mut self, a: DVec2, b: DVec2, radial: bool) {
        let s = self.settings;
        let weights = self.pixel_weights();
        let has_alpha = self.base.format.has_alpha();
        let (fg, bg) = (s.fg.rgb(), s.bg.rgb());
        let d = b - a;
        let len2 = d.length_squared().max(1e-12);
        for p in 0..self.base.pixel_count() {
            let uv = self.pixel_uv(p as u32);
            let t = if radial {
                (uv.distance(a) / len2.sqrt()).clamp(0., 1.)
            } else {
                ((uv - a).dot(d) / len2).clamp(0., 1.)
            } as f32;
            let w = weights.as_ref().map_or(1., |w| w[p]);
            if w <= 0. {
                continue;
            }
            let c: [f32; 3] = std::array::from_fn(|k| fg[k] + (bg[k] - fg[k]) * t);
            let alpha = s.fg.alpha as f32 + (s.bg.alpha as f32 - s.fg.alpha as f32) * t;
            let below = self.base.get(p);
            let out = mix_color(s.blend, below, c, s.opacity as f32 * alpha * w, has_alpha);
            self.write(p as u32, out);
        }
    }

    /// PNT-100..102: Line, Circle or Rectangle from `a` to `b` on screen,
    /// stroked with the tip at Interpolation Step.
    pub fn shape(&mut self, a: DVec2, b: DVec2, radius: f64) {
        let pts = self.settings.shape.outline(a, b);
        let gap = (self.settings.step * radius).max(1.);
        self.dab(pts[0], radius, None, 1.);
        let mut carry = 0.;
        for seg in pts.windows(2) {
            let (p, q) = (seg[0], seg[1]);
            let len = p.distance(q);
            let mut s = gap - carry;
            while s <= len {
                self.dab(p.lerp(q, s / len), radius, None, 1.);
                s += gap;
            }
            carry = len - (s - gap);
        }
    }

    /// Places the texels of the screen tiles within `reach` of `center`.
    fn place_near(&mut self, center: DVec2, reach: f64) {
        let Some(mut lazy) = self.lazy.take() else {
            return;
        };
        let faces = lazy.faces_near(center, reach);
        self.place_faces(&mut lazy, &faces);
        self.lazy = Some(lazy);
    }

    /// Places every texel still waiting (Fill, the falloff's weights, an
    /// ink Stamp over the whole view).
    fn place_all(&mut self) {
        let Some(mut lazy) = self.lazy.take() else {
            return;
        };
        let faces: Vec<u32> = (0..lazy.placed.len() as u32)
            .filter(|&f| lazy.wanted[f as usize] && !lazy.placed[f as usize])
            .collect();
        self.place_faces(&mut lazy, &faces);
    }

    /// The texels of `faces` (not placed yet) that face the viewer and
    /// show in the view, placed and binned.
    fn place_faces(&mut self, lazy: &mut Lazy, faces: &[u32]) {
        let surface = lazy.surface.clone();
        for &f in faces {
            if std::mem::replace(&mut lazy.placed[f as usize], true) {
                continue;
            }
            let normal = self.normals[f as usize];
            let run = surface.face_start[f as usize] as usize
                ..surface.face_start[f as usize + 1] as usize;
            for b in &surface.texels[run] {
                let local = DVec3::new(
                    f64::from(b.local.x),
                    f64::from(b.local.y),
                    f64::from(b.local.z),
                );
                let world_p = self.world.transform_point3(local);
                if normal.dot(self.view.toward_viewer(world_p)) <= 0. {
                    continue;
                }
                let Some((screen, _)) = self.view.project(world_p) else {
                    continue;
                };
                if !lazy.on_screen(screen) {
                    continue;
                }
                let mut falloff = b.falloff;
                // PNT-118: the lasso mixes with the other falloffs.
                if let Some(l) = &lazy.lasso {
                    let lw = l.weight_for(screen, world_p);
                    falloff = if lazy.has_falloff {
                        l.mix.mix(f64::from(falloff), lw) as f32
                    } else {
                        lw as f32
                    };
                }
                let i = self.texels.len() as u32;
                self.texels.push(Texel {
                    pixel: b.pixel,
                    local: b.local,
                    screen,
                    falloff,
                    face: b.face,
                });
                self.visible.push(0);
                self.cells
                    .entry((
                        (screen.x / CELL).floor() as i32,
                        (screen.y / CELL).floor() as i32,
                    ))
                    .or_default()
                    .push(i);
            }
        }
    }

    /// Texel `t`'s surface point in the world.
    fn world_of(&self, t: &Texel) -> DVec3 {
        let l = t.local;
        self.world
            .transform_point3(DVec3::new(f64::from(l.x), f64::from(l.y), f64::from(l.z)))
    }

    /// PNT-45 (b): the texel's point is the nearest surface along the view.
    fn is_visible(&mut self, i: usize) -> bool {
        match self.visible[i] {
            1 => return true,
            2 => return false,
            _ => {}
        }
        let t = self.texels[i];
        let inv = self.world.inverse();
        let origin = inv.transform_point3(self.view.ray_origin(self.world_of(&t)));
        let o = Vec3::new(origin.x as f32, origin.y as f32, origin.z as f32);
        let to = t.local - o;
        let dist = to.length();
        let ok = if dist <= 1e-9 {
            true
        } else {
            let dir = to / dist;
            match self.bvh.closest(o, dir, dist * (1. + 1e-3)) {
                Some(hit) => hit.t >= dist * (1. - 1e-3) - 1e-5,
                None => true,
            }
        };
        self.visible[i] = if ok { 1 } else { 2 };
        ok
    }

    /// Recomputes pixel `p` from its pixel at the press and the mask.
    fn repaint(&mut self, p: u32) {
        let amount = self.mask[p as usize] * self.settings.opacity as f32;
        let below = self.base.get(p as usize);
        let has_alpha = self.base.format.has_alpha();
        let out = match self.source.get(&p) {
            // Clone lays the source's colour and alpha.
            Some(src) => mix_color(
                self.settings.blend,
                below,
                [src[0], src[1], src[2]],
                amount * src[3],
                has_alpha,
            ),
            None => mix(self.tool, &self.settings, below, amount, has_alpha),
        };
        self.write(p, out);
    }

    /// The pixel rectangle changed since the last call (inclusive), for
    /// uploading only what changed (PNT-51).
    pub fn take_dirty(&mut self) -> Option<(u32, u32, u32, u32)> {
        self.dirty.take()
    }

    /// The finished stroke's label (PNT-50).
    pub fn label(&self) -> String {
        self.tool.step_label()
    }
}

/// One pixel: `below` with the stroke laid over it at `amount` (mask ×
/// Opacity).
pub fn mix(
    tool: PaintTool,
    s: &PaintSettings,
    below: [f32; 4],
    amount: f32,
    has_alpha: bool,
) -> [f32; 4] {
    if amount <= 0. {
        return below;
    }
    // Eraser: alpha down where there is alpha; otherwise the BG colour
    // painted with the blend mode (PNT-72).
    let erase = (tool == PaintTool::Eraser && has_alpha) || s.blend == PaintBlend::Erase;
    if erase {
        let mut out = below;
        out[3] = below[3] * (1. - amount.clamp(0., 1.));
        return out;
    }
    let paint = if tool == PaintTool::Eraser {
        s.bg
    } else {
        s.fg
    };
    mix_color(
        s.blend,
        below,
        paint.rgb(),
        amount * paint.alpha as f32,
        has_alpha,
    )
}

/// `color` laid over `below` with blend `mode` at `cover` 0..1.
pub fn mix_color(
    mode: PaintBlend,
    below: [f32; 4],
    c: [f32; 3],
    cover: f32,
    has_alpha: bool,
) -> [f32; 4] {
    let cover = cover.clamp(0., 1.);
    if cover <= 0. {
        return below;
    }
    if mode == PaintBlend::Erase {
        let mut out = below;
        out[3] = below[3] * (1. - cover);
        return out;
    }
    let mode = mode.blend().unwrap_or(Blend::Normal);
    let below_a = if has_alpha { below[3] } else { 1. };
    let a = cover + below_a * (1. - cover);
    let mut out = [0.; 4];
    for k in 0..3 {
        let b = below[k];
        let f = operandi_paint::blend::full(mode, b, c[k]);
        let blended = f * below_a + c[k] * (1. - below_a);
        out[k] = if a > 0. {
            (blended * cover + b * below_a * (1. - cover)) / a
        } else {
            0.
        };
    }
    out[3] = if has_alpha { a } else { 1. };
    out
}

/// One pixel's place on the surface, the same in every view.
#[derive(Clone, Copy, Debug)]
struct SurfaceTexel {
    pixel: u32,
    local: Vec3,
    face: u32,
    /// The active falloff's weight (1 without one).
    falloff: f32,
}

/// A mesh's texels for one image size and UV map, with its polygons'
/// own-space normals and the visibility-ray tree.
struct SurfaceTexels {
    /// By polygon: polygon `f`'s run is `face_start[f]..face_start[f + 1]`.
    texels: Vec<SurfaceTexel>,
    face_start: Vec<u32>,
    /// Per polygon, its texels' box in the mesh's own space.
    boxes: Vec<(Vec3, Vec3)>,
    normals: Vec<DVec3>,
    bvh: Arc<crate::bake::bvh::Bvh>,
}

/// Which pixel lies where on `item`'s surface. Sub-D (Tab): the smooth
/// surface the view draws, with its interpolated UVs, not the cage;
/// otherwise away from the middle of a polygon the paint lands where the
/// cage is, not under the brush. Falloff weights are per cage point, so
/// with a falloff the cage is painted (provisional, PNT-Q62).
fn surface_texels(
    item: &crate::Item,
    map_name: &str,
    w: u32,
    h: u32,
    falloff: Option<&Vec<f64>>,
) -> SurfaceTexels {
    let mesh = &item.mesh;
    let refined;
    let mesh: &crate::Mesh = if falloff.is_none()
        && (0..mesh.faces.len()).any(|f| crate::subdivision::is_smooth_polygon(mesh, f))
    {
        match crate::subdivision::freeze(mesh, &item.subdivision.clone().unwrap_or_default()) {
            Ok(m) => {
                refined = m;
                &refined
            }
            Err(_) => mesh,
        }
    } else {
        mesh
    };
    let map = mesh.uv_maps.get(map_name);
    let mut tris: Vec<[Vec3; 3]> = Vec::new();
    let mut normals = Vec::with_capacity(mesh.faces.len());
    // Pixel -> its texel as found; the first triangle covering a pixel's
    // centre wins, else one touching it (rank 2 / 1).
    let mut found: Vec<(u8, SurfaceTexel)> = vec![
        (
            0,
            SurfaceTexel {
                pixel: 0,
                local: Vec3::ZERO,
                face: 0,
                falloff: 0.,
            }
        );
        w as usize * h as usize
    ];
    let v3 = |p: DVec3| Vec3::new(p.x as f32, p.y as f32, p.z as f32);
    for (f, face) in mesh.faces.iter().enumerate() {
        normals.push(mesh.normal(f));
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
        };
        for t in triangles {
            let pts = t.map(|c| mesh.vertices[face.vertices[c] as usize]);
            tris.push(pts.map(v3));
            let [Some(a), Some(b), Some(c)] = t.map(uv) else {
                continue;
            };
            let weights = t.map(|c| {
                falloff
                    .and_then(|fw| fw.get(face.vertices[c] as usize))
                    .map_or(1., |v| v.clamp(0., 1.))
            });
            raster_triangle([a, b, c], w, h, |pixel, bary, inside| {
                let slot = &mut found[pixel as usize];
                let rank = if inside { 2 } else { 1 };
                if slot.0 >= rank {
                    return;
                }
                let local = pts[0] * bary[0] + pts[1] * bary[1] + pts[2] * bary[2];
                *slot = (
                    rank,
                    SurfaceTexel {
                        pixel,
                        local: v3(local),
                        face: f as u32,
                        falloff: (weights[0] * bary[0]
                            + weights[1] * bary[1]
                            + weights[2] * bary[2]) as f32,
                    },
                );
            });
        }
    }
    let mut texels: Vec<SurfaceTexel> = found
        .into_iter()
        .filter(|(rank, _)| *rank > 0)
        .map(|(_, t)| t)
        .collect();
    texels.sort_by_key(|t| t.face);
    let faces = normals.len();
    let mut face_start = vec![0u32; faces + 1];
    let mut boxes = vec![(Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)); faces];
    for t in &texels {
        face_start[t.face as usize + 1] += 1;
        let b = &mut boxes[t.face as usize];
        *b = (b.0.min(t.local), b.1.max(t.local));
    }
    for f in 0..faces {
        face_start[f + 1] += face_start[f];
    }
    SurfaceTexels {
        texels,
        face_start,
        boxes,
        normals,
        bvh: Arc::new(crate::bake::bvh::Bvh::new(tris)),
    }
}

/// The last mesh's texels, kept between strokes: (mesh, Sub-D settings,
/// UV map, width, height) and what they gave.
type SurfaceKey = (
    crate::Shared<crate::Mesh>,
    Option<crate::subdivision::SubdivisionSettings>,
    String,
    u32,
    u32,
);
thread_local! {
    static SURFACE: std::cell::RefCell<Option<(SurfaceKey, Arc<SurfaceTexels>)>> =
        const { std::cell::RefCell::new(None) };
    static SURFACE_BUILDS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn cached_surface(item: &crate::Item, map_name: &str, w: u32, h: u32) -> Arc<SurfaceTexels> {
    let same = |k: &SurfaceKey| {
        crate::Shared::same(&k.0, item.mesh.shared())
            && k.1 == item.subdivision
            && k.2 == map_name
            && (k.3, k.4) == (w, h)
    };
    if let Some(hit) = SURFACE.with(|c| {
        c.borrow()
            .as_ref()
            .filter(|(k, _)| same(k))
            .map(|(_, s)| s.clone())
    }) {
        return hit;
    }
    SURFACE_BUILDS.with(|n| n.set(n.get() + 1));
    let made = Arc::new(surface_texels(item, map_name, w, h, None));
    let key = (
        item.mesh.shared().clone(),
        item.subdivision.clone(),
        map_name.to_owned(),
        w,
        h,
    );
    SURFACE.with(|c| *c.borrow_mut() = Some((key, made.clone())));
    made
}

/// Builds the kept texels for painting `image` on `item` ahead of the
/// press (the app calls it while a paint tool is on), so the first stroke
/// starts at once too. Does nothing when they're already kept.
pub fn warm_surface(scene: &Scene, item: u64, uv_map: Option<&str>, w: u32, h: u32) {
    let Some(it) = scene.item(item).filter(|i| i.is_mesh()) else {
        return;
    };
    if it.mesh.uv_maps.is_empty() || w == 0 || h == 0 {
        return;
    }
    let map_name = crate::uv_tools::target_uv_map(&it.mesh, uv_map);
    cached_surface(it, &map_name, w, h);
}

/// How many times the kept texels were built (speed guards).
pub fn surface_builds() -> u64 {
    SURFACE_BUILDS.with(std::cell::Cell::get)
}

/// Screen tiles (in points) for placing a stroke's texels as dabs reach
/// them.
const TILE: f64 = 64.;

/// The texels a stroke hasn't placed in its view yet: each polygon's
/// screen box, binned by tile; a dab places the polygons of the tiles it
/// reaches, each once.
struct Lazy {
    surface: Arc<SurfaceTexels>,
    /// Per tile, the polygons whose box reaches it.
    tiles: HashMap<(i32, i32), Vec<u32>>,
    done: std::collections::HashSet<(i32, i32)>,
    /// Per polygon: placed already; shows in the view at all.
    placed: Vec<bool>,
    wanted: Vec<bool>,
    /// The view with a margin: texels outside it are never placed.
    lo: DVec2,
    hi: DVec2,
    lasso: Option<crate::paint_lasso::Lasso>,
    has_falloff: bool,
}

impl Lazy {
    fn new(
        surface: Arc<SurfaceTexels>,
        view: &PaintView,
        world: DMat4,
        lasso: Option<crate::paint_lasso::Lasso>,
        has_falloff: bool,
    ) -> Lazy {
        let margin = 64.;
        let lo = view.center - view.size / 2. - DVec2::splat(margin);
        let hi = view.center + view.size / 2. + DVec2::splat(margin);
        let faces = surface.boxes.len();
        let mut tiles: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        let mut wanted = vec![false; faces];
        let tile = |v: f64| (v / TILE).floor() as i32;
        let (tlo, thi) = ((tile(lo.x), tile(lo.y)), (tile(hi.x), tile(hi.y)));
        for (f, &(bmin, bmax)) in surface.boxes.iter().enumerate() {
            if surface.face_start[f] == surface.face_start[f + 1] {
                continue;
            }
            // The box's corners on screen; one behind the eye: every tile.
            let mut smin = DVec2::splat(f64::INFINITY);
            let mut smax = DVec2::splat(f64::NEG_INFINITY);
            let mut behind = false;
            for k in 0..8 {
                let c = Vec3::new(
                    if k & 1 == 0 { bmin.x } else { bmax.x },
                    if k & 2 == 0 { bmin.y } else { bmax.y },
                    if k & 4 == 0 { bmin.z } else { bmax.z },
                );
                let p = world.transform_point3(DVec3::new(
                    f64::from(c.x),
                    f64::from(c.y),
                    f64::from(c.z),
                ));
                match view.project(p) {
                    Some((q, _)) => {
                        smin = smin.min(q);
                        smax = smax.max(q);
                    }
                    None => behind = true,
                }
            }
            let (a, b) = if behind {
                (tlo, thi)
            } else {
                if smax.x < lo.x || smax.y < lo.y || smin.x > hi.x || smin.y > hi.y {
                    continue;
                }
                (
                    (tile(smin.x).max(tlo.0), tile(smin.y).max(tlo.1)),
                    (tile(smax.x).min(thi.0), tile(smax.y).min(thi.1)),
                )
            };
            wanted[f] = true;
            for ty in a.1..=b.1 {
                for tx in a.0..=b.0 {
                    tiles.entry((tx, ty)).or_default().push(f as u32);
                }
            }
        }
        Lazy {
            surface,
            tiles,
            done: Default::default(),
            placed: vec![false; faces],
            wanted,
            lo,
            hi,
            lasso,
            has_falloff,
        }
    }

    fn on_screen(&self, s: DVec2) -> bool {
        s.x >= self.lo.x && s.y >= self.lo.y && s.x <= self.hi.x && s.y <= self.hi.y
    }

    /// The polygons of the tiles within `reach` of `center` not yet
    /// handed out.
    fn faces_near(&mut self, center: DVec2, reach: f64) -> Vec<u32> {
        let tile = |v: f64| (v / TILE).floor() as i32;
        let mut out = Vec::new();
        for ty in tile(center.y - reach)..=tile(center.y + reach) {
            for tx in tile(center.x - reach)..=tile(center.x + reach) {
                if self.done.insert((tx, ty))
                    && let Some(faces) = self.tiles.get(&(tx, ty))
                {
                    out.extend(faces.iter().filter(|&&f| !self.placed[f as usize]));
                }
            }
        }
        out
    }
}

/// Visits the pixels a UV triangle covers: `f(pixel, barycentric,
/// inside)`. Pixels whose centre is outside but whose square touches the
/// triangle come with `inside` false and the barycentric clamped onto it
/// (so island borders get paint, PNT-45). Pixel rows run top first, v up.
fn raster_triangle(
    uv: [DVec2; 3],
    width: u32,
    height: u32,
    mut f: impl FnMut(u32, [f64; 3], bool),
) {
    let (w, h) = (f64::from(width), f64::from(height));
    let p = uv.map(|t| DVec2::new(t.x * w, (1. - t.y) * h));
    let area = (p[1] - p[0]).perp_dot(p[2] - p[0]);
    if area.abs() < 1e-12 {
        return;
    }
    let lo = p[0].min(p[1]).min(p[2]) - DVec2::splat(1.);
    let hi = p[0].max(p[1]).max(p[2]) + DVec2::splat(1.);
    let x0 = lo.x.floor().max(0.) as i64;
    let y0 = lo.y.floor().max(0.) as i64;
    let x1 = (hi.x.ceil() as i64).min(i64::from(width) - 1);
    let y1 = (hi.y.ceil() as i64).min(i64::from(height) - 1);
    let bary = |c: DVec2| {
        let b1 = (c - p[0]).perp_dot(p[2] - p[0]) / area;
        let b2 = (p[1] - p[0]).perp_dot(c - p[0]) / area;
        [1. - b1 - b2, b1, b2]
    };
    for y in y0..=y1 {
        for x in x0..=x1 {
            let c = DVec2::new(x as f64 + 0.5, y as f64 + 0.5);
            let b = bary(c);
            let pixel = (y as u32) * width + x as u32;
            if b.iter().all(|&v| v >= -1e-9) {
                f(pixel, b, true);
                continue;
            }
            // Touching: any corner of the pixel square inside.
            let touches = [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)]
                .iter()
                .any(|&(dx, dy)| bary(c + DVec2::new(dx, dy)).iter().all(|&v| v >= 0.));
            if touches {
                let clamped = b.map(|v| v.max(0.));
                let sum: f64 = clamped.iter().sum();
                if sum > 0. {
                    f(pixel, clamped.map(|v| v / sum), false);
                }
            }
        }
    }
}
