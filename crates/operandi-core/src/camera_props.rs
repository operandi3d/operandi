//! Camera Properties panels (spec camera-properties-2026-11-06, Tiers 1
//! and 2): depth of field, Resolution Override, the camera Background
//! Image, the Camera Overlay, Film Back and lens presets, Transform Reset,
//! Autofocus, Sync to View and Set Target's Set Focus Distance. The settings
//! live on `CameraSettings` (camera_light.rs); this file holds their types
//! and the maths.
//!
//! Provisional readings (docs/GUI_QA_HANDOFF_M1_M2_M3.md, CAM-Q*):
//! - CAM-Q01: Spherical and Cylindrical put the camera's back at the
//!   picture's centre ("flipped"); the VR variants put the view direction
//!   there.
//! - CAM-Q02: Cylindrical sees 360° around and as much up and down as keeps
//!   pixels square (picture height ÷ width × 2π on the unit cylinder).
//! - CAM-Q03: Lens Distortion and Edge Weighting are stored and shown but
//!   don't change renders (Cycles has no matching control).
//! - CAM-Q04: with no render item, Film Back Preset sets only the film and
//!   Set Resolution from Clip turns this camera's Resolution Override on at
//!   the picture's size.
//! - CAM-Q05: Camera Overlay colours and the other CP-Q4 defaults are the
//!   spec's picks; colours are ours.
//! - CAM-Q06: Autofocus measures along the view axis (Cycles focuses on a
//!   plane) to the first visible mesh polygon through the picture's centre.
use crate::Result;
use crate::camera_light::{CameraSettings, Frame, item_frame};
use crate::scene::{Scene, Transform};
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// CP-E01..E07 Depth of Field. Focus Distance stays on `CameraSettings`
/// (the frustum draws it whether or not depth of field is on).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DepthOfField {
    pub enable: bool,
    /// Minimum 1.0; never changes exposure.
    pub f_stop: f64,
    /// 0 = round bokeh, else 3..=16 sides.
    pub iris_blades: u32,
    /// Radians.
    pub iris_rotation: f64,
    /// 0..1; stored only (CAM-Q03).
    pub edge_weighting: f64,
}

impl Default for DepthOfField {
    fn default() -> Self {
        Self {
            enable: false,
            f_stop: F_STOP,
            iris_blades: 0,
            iris_rotation: 0.0,
            edge_weighting: 0.5,
        }
    }
}

/// F-Stop default and minimum (CP-E04, observed).
pub const F_STOP: f64 = 4.0;
pub const F_STOP_MIN: f64 = 1.0;
/// Most iris blades (CP-E05 pick).
pub const IRIS_BLADES_MAX: u32 = 16;
/// CP-V14 lens presets, millimetres (spec pick).
pub const LENS_PRESETS_MM: [f64; 8] = [18.0, 24.0, 28.0, 35.0, 50.0, 85.0, 135.0, 200.0];
/// CP-E04 F-Stop presets (spec pick).
pub const F_STOP_PRESETS: [f64; 9] = [1.4, 2.0, 2.8, 4.0, 5.6, 8.0, 11.0, 16.0, 22.0];

/// Whether `n` is a valid Iris Blades count: 0 (round) or 3..=16.
pub fn valid_iris_blades(n: u32) -> bool {
    n == 0 || (3..=IRIS_BLADES_MAX).contains(&n)
}

impl DepthOfField {
    /// The Cycles aperture radius, metres: Focal Length ÷ (2 × F-Stop).
    pub fn aperture_radius(&self, focal_length: f64) -> f64 {
        focal_length / (2.0 * self.f_stop.max(F_STOP_MIN))
    }

    pub fn validate(&self) -> Result<()> {
        if !(self.f_stop.is_finite() && (F_STOP_MIN..1e4).contains(&self.f_stop))
            || !valid_iris_blades(self.iris_blades)
            || !self.iris_rotation.is_finite()
            || !(0.0..=1.0).contains(&self.edge_weighting)
        {
            return Err("Invalid depth of field settings".into());
        }
        Ok(())
    }
}

/// CP-V17..V19 Resolution Override.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResolutionOverride {
    pub enable: bool,
    pub width: u32,
    pub height: u32,
}

/// Largest override side, pixels.
pub const MAX_RESOLUTION: u32 = 32_768;

impl Default for ResolutionOverride {
    fn default() -> Self {
        let [width, height] = crate::camera_light_defaults::RENDER_FRAME;
        Self {
            enable: false,
            width,
            height,
        }
    }
}

/// CP-V25..V27 Background Image. Overscan is `CameraStored::overscan`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CameraBackground {
    pub enable: bool,
    /// A scene image id (`Scene::images`), or none.
    pub image: Option<u64>,
}

impl Default for CameraBackground {
    fn default() -> Self {
        Self {
            enable: true,
            image: None,
        }
    }
}

/// CP-O08 Grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayGrid {
    RuleOfThirds,
    CenterLines,
    GoldenRatio,
    Tenths,
    Square,
    #[default]
    Off,
}

impl OverlayGrid {
    pub const ALL: [OverlayGrid; 6] = [
        OverlayGrid::RuleOfThirds,
        OverlayGrid::CenterLines,
        OverlayGrid::GoldenRatio,
        OverlayGrid::Tenths,
        OverlayGrid::Square,
        OverlayGrid::Off,
    ];
    pub fn label(self) -> &'static str {
        match self {
            OverlayGrid::RuleOfThirds => "Rule of Thirds",
            OverlayGrid::CenterLines => "Center Lines",
            OverlayGrid::GoldenRatio => "Golden Ratio",
            OverlayGrid::Tenths => "Tenths",
            OverlayGrid::Square => "Square",
            OverlayGrid::Off => "Off",
        }
    }
}

/// CP-O01..O14 Camera Overlay (Tier 2 rows), drawn only in a view looking
/// through this camera and never rendered. Present while Display > Show
/// Camera Overlay is on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CameraOverlay {
    /// 0..1.
    pub opacity: f64,
    pub action_safe: bool,
    /// Inset per side as a fraction of the frame width and height.
    pub action_margin: [f64; 2],
    pub title_safe: bool,
    pub title_margin: [f64; 2],
    pub safe_color: [f64; 3],
    pub grid: OverlayGrid,
    pub grid_color: [f64; 3],
    pub right_diagonal: bool,
    pub left_diagonal: bool,
    pub chevron_up: bool,
    pub chevron_down: bool,
    pub horizon: bool,
    pub line_color: [f64; 3],
}

impl Default for CameraOverlay {
    fn default() -> Self {
        Self {
            opacity: 0.5,
            action_safe: true,
            action_margin: [0.035, 0.035],
            title_safe: true,
            title_margin: [0.10, 0.05],
            safe_color: [1.0, 0.8, 0.2],
            grid: OverlayGrid::Off,
            grid_color: [0.6, 0.8, 1.0],
            right_diagonal: false,
            left_diagonal: false,
            chevron_up: false,
            chevron_down: false,
            horizon: false,
            line_color: [1.0, 0.45, 0.45],
        }
    }
}

impl CameraOverlay {
    pub fn validate(&self) -> Result<()> {
        let unit = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
        let margin = |v: f64| v.is_finite() && (0.0..0.5).contains(&v);
        if !unit(self.opacity)
            || !self
                .action_margin
                .iter()
                .chain(&self.title_margin)
                .all(|v| margin(*v))
            || !self
                .safe_color
                .iter()
                .chain(&self.grid_color)
                .chain(&self.line_color)
                .all(|v| unit(*v))
        {
            return Err("Invalid camera overlay settings".into());
        }
        Ok(())
    }
}

/// Which colour an overlay line takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayRole {
    Safe,
    Grid,
    Guide,
}

/// One overlay line in frame coordinates: 0..1 across, 0..1 down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayLine {
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub role: OverlayRole,
}

/// 1 ÷ golden ratio.
const GOLDEN: f64 = 0.618_033_988_749_895;

/// The overlay's lines for a frame of `aspect` (width ÷ height), with the
/// horizon tilted by `horizon_tilt` radians (anticlockwise on screen).
pub fn overlay_lines(o: &CameraOverlay, aspect: f64, horizon_tilt: f64) -> Vec<OverlayLine> {
    let mut lines = Vec::new();
    let mut push = |a: [f64; 2], b: [f64; 2], role| lines.push(OverlayLine { a, b, role });
    let rect = |m: [f64; 2], push: &mut dyn FnMut([f64; 2], [f64; 2], OverlayRole)| {
        let (x0, y0, x1, y1) = (m[0], m[1], 1.0 - m[0], 1.0 - m[1]);
        push([x0, y0], [x1, y0], OverlayRole::Safe);
        push([x1, y0], [x1, y1], OverlayRole::Safe);
        push([x1, y1], [x0, y1], OverlayRole::Safe);
        push([x0, y1], [x0, y0], OverlayRole::Safe);
    };
    if o.action_safe {
        rect(o.action_margin, &mut push);
    }
    if o.title_safe {
        rect(o.title_margin, &mut push);
    }
    let mut grid = |xs: &[f64], ys: &[f64]| {
        for &x in xs {
            push([x, 0.0], [x, 1.0], OverlayRole::Grid);
        }
        for &y in ys {
            push([0.0, y], [1.0, y], OverlayRole::Grid);
        }
    };
    match o.grid {
        OverlayGrid::Off => {}
        OverlayGrid::RuleOfThirds => grid(&[1. / 3., 2. / 3.], &[1. / 3., 2. / 3.]),
        OverlayGrid::CenterLines => grid(&[0.5], &[0.5]),
        OverlayGrid::GoldenRatio => grid(&[1. - GOLDEN, GOLDEN], &[1. - GOLDEN, GOLDEN]),
        OverlayGrid::Tenths => {
            let t: Vec<f64> = (1..10).map(|k| f64::from(k) / 10.0).collect();
            grid(&t, &t);
        }
        OverlayGrid::Square => {
            // Squares a tenth of the frame height, from the centre out.
            let ys: Vec<f64> = (1..10).map(|k| f64::from(k) / 10.0).collect();
            let step = 0.1 / aspect.max(1e-6);
            let mut xs = vec![0.5];
            let mut k = 1.0;
            while 0.5 - k * step > 1e-9 && xs.len() < 2000 {
                xs.push(0.5 - k * step);
                xs.push(0.5 + k * step);
                k += 1.0;
            }
            grid(&xs, &ys);
        }
    }
    if o.right_diagonal {
        push([0.0, 1.0], [1.0, 0.0], OverlayRole::Guide);
    }
    if o.left_diagonal {
        push([0.0, 0.0], [1.0, 1.0], OverlayRole::Guide);
    }
    if o.chevron_up {
        push([0.0, 1.0], [0.5, 0.0], OverlayRole::Guide);
        push([1.0, 1.0], [0.5, 0.0], OverlayRole::Guide);
    }
    if o.chevron_down {
        push([0.0, 0.0], [0.5, 1.0], OverlayRole::Guide);
        push([1.0, 0.0], [0.5, 1.0], OverlayRole::Guide);
    }
    if o.horizon {
        // Through the centre at the tilt, to the frame's edge. In frame
        // units a screen direction (cos, sin) is (cos / aspect, −sin).
        let d = [horizon_tilt.cos() / aspect.max(1e-6), -horizon_tilt.sin()];
        let reach = [d[0].abs(), d[1].abs()]
            .iter()
            .filter(|v| **v > 1e-12)
            .map(|v| 0.5 / v)
            .fold(f64::INFINITY, f64::min);
        push(
            [0.5 - d[0] * reach, 0.5 - d[1] * reach],
            [0.5 + d[0] * reach, 0.5 + d[1] * reach],
            OverlayRole::Guide,
        );
    }
    lines
}

/// How far the world horizon is turned on a camera's picture, radians
/// (anticlockwise positive): 0 when the camera's right axis is level.
pub fn horizon_tilt(frame: &Frame) -> f64 {
    (-frame.right.y).atan2(frame.up.y)
}

/// A named film back (CP-V36), metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilmBackPreset {
    pub name: String,
    pub width: f64,
    pub height: f64,
}

/// Built-in film backs, millimetres (common sensor and gate sizes).
pub const FILM_BACKS_MM: [(&str, f64, f64); 7] = [
    ("Default (36 × 24 mm)", 36.0, 24.0),
    ("Super 35 (24.89 × 18.66 mm)", 24.89, 18.66),
    ("APS-C (23.6 × 15.6 mm)", 23.6, 15.6),
    ("Micro Four Thirds (17.3 × 13 mm)", 17.3, 13.0),
    ("1 inch (13.2 × 8.8 mm)", 13.2, 8.8),
    ("Medium Format (44 × 33 mm)", 44.0, 33.0),
    ("6 × 7 (70 × 56 mm)", 70.0, 56.0),
];

/// The built-in film backs as presets.
pub fn built_in_film_backs() -> Vec<FilmBackPreset> {
    FILM_BACKS_MM
        .iter()
        .map(|(name, w, h)| FilmBackPreset {
            name: (*name).to_owned(),
            width: w / 1000.0,
            height: h / 1000.0,
        })
        .collect()
}

/// The preset whose film matches `c`, if any.
pub fn matching_film_back<'a>(
    c: &CameraSettings,
    presets: impl IntoIterator<Item = &'a FilmBackPreset>,
) -> Option<&'a FilmBackPreset> {
    presets
        .into_iter()
        .find(|p| (p.width - c.film_width).abs() < 1e-7 && (p.height - c.film_height).abs() < 1e-7)
}

/// CP-V07 Reset: which part of the transform goes back to identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformReset {
    Position,
    Rotation,
    Scale,
    All,
}

impl TransformReset {
    pub const ALL: [TransformReset; 4] = [
        TransformReset::Position,
        TransformReset::Rotation,
        TransformReset::Scale,
        TransformReset::All,
    ];
    pub fn label(self) -> &'static str {
        match self {
            TransformReset::Position => "Position",
            TransformReset::Rotation => "Rotation",
            TransformReset::Scale => "Scale",
            TransformReset::All => "All",
        }
    }
    /// `t` with this part set back to identity (rotation order kept).
    pub fn apply(self, t: &Transform) -> Transform {
        let mut t = t.clone();
        if matches!(self, TransformReset::Position | TransformReset::All) {
            t.translation = DVec3::ZERO;
        }
        if matches!(self, TransformReset::Rotation | TransformReset::All) {
            t.rotation = DVec3::ZERO;
        }
        if matches!(self, TransformReset::Scale | TransformReset::All) {
            t.scale = DVec3::ONE;
        }
        t
    }
}

impl Scene {
    /// The Focus Distance renders use for camera `id`: the distance to its
    /// target's centre when Set Focus Distance is on and the target is set
    /// and enabled (CP-V10), else its own Focus Distance.
    pub fn camera_focus_distance(&self, id: u64) -> Option<f64> {
        let item = self.item(id)?;
        let camera = item.camera.as_ref()?;
        if camera.focus_from_target
            && let Some(t) = item.common.target.as_ref().filter(|t| t.enable)
            && let (Ok(from), Ok(to)) = (self.world_matrix(id), self.world_matrix(t.item))
        {
            let d = from.w_axis.truncate().distance(to.w_axis.truncate());
            if d > 1e-6 {
                return Some(d);
            }
        }
        Some(camera.focus_distance)
    }

    /// CP-E03 Autofocus: the distance along camera `id`'s view axis to the
    /// first visible mesh polygon hit by a ray through the centre of its
    /// picture (CAM-Q06). Errors when nothing is hit.
    pub fn autofocus_distance(&self, id: u64) -> Result<f64> {
        let item = self.item(id).ok_or("Missing item")?;
        let camera = item.camera.as_ref().ok_or("Select a camera")?;
        let frame = item_frame(self, id).ok_or("The camera has no usable transform")?;
        let [w, h] = camera.render_size(self.render.frame);
        let aspect = f64::from(w) / f64::from(h.max(1));
        let direction = camera.centre_ray(&frame, aspect);
        let ray = crate::drop_action::Ray {
            origin: frame.position,
            direction,
        };
        let hidden = self.display_hidden();
        self.items
            .iter()
            .filter(|i| i.is_mesh() && i.visible && !hidden.contains(&i.id))
            .filter_map(|i| crate::drop_action::surface_hit(self, i.id, ray))
            .map(|(p, _)| (p - frame.position).dot(frame.forward))
            .filter(|d| *d > 1e-6)
            .min_by(f64::total_cmp)
            .ok_or_else(|| "Autofocus: nothing under the centre of the camera's view".into())
    }
}

impl CameraSettings {
    /// The picture size renders through this camera use: Resolution
    /// Override when on, else `frame`, the scene's Frame Width and Height
    /// (`RenderSettings::frame`, RI-F03).
    pub fn render_size(&self, frame: [u32; 2]) -> [u32; 2] {
        if self.resolution.enable {
            [self.resolution.width, self.resolution.height]
        } else {
            frame
        }
    }

    /// The world direction through the centre of the picture (Film Offset
    /// moves it) for a camera at `frame`.
    pub fn centre_ray(&self, frame: &Frame, aspect: f64) -> DVec3 {
        if self.projection != crate::camera_light::Projection::Perspective {
            return frame.forward;
        }
        let (tw, th) = self.half_tangents(aspect);
        let [sx, sy] = self.film_shift(aspect);
        (frame.forward + frame.right * (sx * tw) + frame.up * (sy * th)).normalize_or(frame.forward)
    }

    /// CP-Q1: an orthographic camera's picture width, metres (Film Width ×
    /// Target Distance ÷ Focal Length, through Film Fit for `aspect`).
    pub fn orthographic_width(&self, aspect: f64) -> f64 {
        2.0 * self.orthographic_half_size(aspect).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_light::Projection;

    #[test]
    fn aperture_is_focal_over_twice_the_f_stop() {
        let dof = DepthOfField::default();
        assert!((dof.aperture_radius(0.05) - 0.00625).abs() < 1e-12);
        let wide = DepthOfField {
            f_stop: 1.0,
            ..dof.clone()
        };
        assert!((wide.aperture_radius(0.05) - 0.025).abs() < 1e-12);
        // Below the minimum reads as the minimum.
        let low = DepthOfField { f_stop: 0.5, ..dof };
        assert!((low.aperture_radius(0.05) - 0.025).abs() < 1e-12);
    }

    #[test]
    fn iris_blades_are_zero_or_three_to_sixteen() {
        assert!(valid_iris_blades(0));
        assert!(!valid_iris_blades(1));
        assert!(!valid_iris_blades(2));
        assert!(valid_iris_blades(3));
        assert!(valid_iris_blades(16));
        assert!(!valid_iris_blades(17));
    }

    #[test]
    fn ortho_width_keeps_the_perspective_framing_at_the_target() {
        let c = CameraSettings {
            film_fit: crate::camera_light::FilmFit::Horizontal,
            ..Default::default()
        };
        // 36 mm × 4 m ÷ 50 mm = 2.88 m.
        assert!((c.orthographic_width(1.5) - 2.88).abs() < 1e-9);
    }

    #[test]
    fn safe_areas_and_thirds_are_where_the_margins_say() {
        let o = CameraOverlay {
            grid: OverlayGrid::RuleOfThirds,
            ..Default::default()
        };
        let lines = overlay_lines(&o, 16.0 / 9.0, 0.0);
        let safe: Vec<_> = lines
            .iter()
            .filter(|l| l.role == OverlayRole::Safe)
            .collect();
        assert_eq!(safe.len(), 8);
        // Action safe top edge 3.5 % in; title safe left edge 10 % in.
        assert_eq!(safe[0].a, [0.035, 0.035]);
        assert_eq!(safe[4].a, [0.10, 0.05]);
        let grid: Vec<_> = lines
            .iter()
            .filter(|l| l.role == OverlayRole::Grid)
            .collect();
        assert_eq!(grid.len(), 4);
        assert!((grid[0].a[0] - 1.0 / 3.0).abs() < 1e-12);
        let off = CameraOverlay {
            action_safe: false,
            title_safe: false,
            ..Default::default()
        };
        assert!(overlay_lines(&off, 1.0, 0.0).is_empty());
    }

    #[test]
    fn square_grid_cells_are_a_tenth_of_the_height() {
        let o = CameraOverlay {
            action_safe: false,
            title_safe: false,
            grid: OverlayGrid::Square,
            ..Default::default()
        };
        let aspect = 2.0;
        let mut xs: Vec<f64> = overlay_lines(&o, aspect, 0.0)
            .iter()
            .filter(|l| l.a[0] == l.b[0])
            .map(|l| l.a[0])
            .collect();
        xs.sort_by(f64::total_cmp);
        // A tenth of the height is a twentieth of the width at 2:1.
        assert!((xs[1] - xs[0] - 0.05).abs() < 1e-9);
        assert!(xs.contains(&0.5));
    }

    #[test]
    fn horizon_follows_the_camera_roll() {
        let o = CameraOverlay {
            action_safe: false,
            title_safe: false,
            horizon: true,
            ..Default::default()
        };
        let level = overlay_lines(&o, 1.0, 0.0)[0];
        assert_eq!((level.a, level.b), ([0.0, 0.5], [1.0, 0.5]));
        let tilted = overlay_lines(&o, 1.0, std::f64::consts::FRAC_PI_4)[0];
        // Anticlockwise: the right end rises (smaller y, frame y down).
        assert!(tilted.b[1] < 0.5 && tilted.b[0] > 0.5);
        assert!((tilted.b[0] - 1.0).abs() < 1e-9 && tilted.b[1].abs() < 1e-9);
        // A camera rolled clockwise sees the horizon turned anticlockwise.
        let roll = 0.3f64;
        let frame = Frame {
            position: DVec3::ZERO,
            right: DVec3::new(roll.cos(), -roll.sin(), 0.0),
            up: DVec3::new(roll.sin(), roll.cos(), 0.0),
            forward: -DVec3::Z,
        };
        assert!((horizon_tilt(&frame) - roll).abs() < 1e-12);
    }

    #[test]
    fn transform_reset_parts() {
        let t = Transform {
            translation: DVec3::ONE,
            rotation: DVec3::splat(0.5),
            scale: DVec3::splat(2.0),
            zxy: true,
            order: None,
        };
        let p = TransformReset::Position.apply(&t);
        assert_eq!(p.translation, DVec3::ZERO);
        assert_eq!(p.rotation, t.rotation);
        let r = TransformReset::Rotation.apply(&t);
        assert_eq!(r.rotation, DVec3::ZERO);
        assert_eq!(r.scale, t.scale);
        let all = TransformReset::All.apply(&t);
        assert_eq!(
            all,
            Transform {
                zxy: true,
                order: None,
                ..Default::default()
            }
        );
    }

    #[test]
    fn autofocus_hits_the_mesh_under_the_view_centre() {
        let mut scene = Scene::default();
        let mesh = crate::Mesh::box_mesh(DVec3::new(0.0, 0.0, -3.0), DVec3::ONE, 0).unwrap();
        scene.items.push(crate::scene::Item {
            id: 1,
            name: "Box".into(),
            visible: true,
            transform: Default::default(),
            mesh: mesh.into(),
            parent: None,
            source_id: None,
            source_display_name: None,
            subdivision: None,
            backdrop: None,
            camera: None,
            light: None,
            texture: None,
            locator: None,
            environment: None,
            common: Default::default(),
            elements: Default::default(),
            hides_children: false,
            mesh_stack: None,
            kit: None,
            instance_of: None,
            replicator: None,
        });
        scene.next_id = 2;
        let cam = scene
            .add_camera(CameraSettings::default(), Transform::default())
            .unwrap();
        // The box's near face is at z = -2.5; the camera at the origin
        // looks down -Z.
        assert!((scene.autofocus_distance(cam).unwrap() - 2.5).abs() < 1e-9);
        scene.items[0].visible = false;
        assert!(scene.autofocus_distance(cam).is_err());
        let _ = Projection::Orthographic;
    }

    #[test]
    fn focus_from_target_measures_to_the_target() {
        let mut scene = Scene::default();
        let cam = scene
            .add_camera(
                CameraSettings {
                    focus_from_target: true,
                    ..Default::default()
                },
                Transform::default(),
            )
            .unwrap();
        let other = scene
            .add_camera(
                CameraSettings::default(),
                Transform {
                    translation: DVec3::new(0.0, 3.0, -4.0),
                    ..Default::default()
                },
            )
            .unwrap();
        // No target yet: its own Focus Distance.
        assert_eq!(scene.camera_focus_distance(cam), Some(1.0));
        scene
            .apply_item_common(
                &[cam],
                &crate::item_common::ItemCommonEdit::Target(Some(other)),
            )
            .unwrap();
        assert!((scene.camera_focus_distance(cam).unwrap() - 5.0).abs() < 1e-9);
    }
}
