//! Camera and light items (CL-1, spec v0, provisional).
//!
//! Like a Backdrop, a camera or light is an ordinary scene item (name,
//! transform, parent, visibility) with an empty mesh and its own settings:
//! `Item::camera` or `Item::light`. Both look down their local -Z axis with
//! +Y up (provisional, CL-Q08), the convention Cycles uses too.
//!
//! Every default lives in `camera_light_defaults.rs`.
use crate::Result;
use crate::camera_light_defaults as defaults;
use crate::scene::{Item, Scene, Transform};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};

/// Camera Projection (CP-V13), in the panel's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Projection {
    #[default]
    Perspective,
    Orthographic,
    /// 360° around, no poles (CAM-Q01, CAM-Q02).
    Cylindrical,
    CylindricalVr,
    /// Full 360° equirectangular.
    Spherical,
    SphericalVr,
}

impl Projection {
    pub const ALL: [Projection; 6] = [
        Projection::Perspective,
        Projection::Orthographic,
        Projection::Cylindrical,
        Projection::CylindricalVr,
        Projection::Spherical,
        Projection::SphericalVr,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Projection::Perspective => "Perspective",
            Projection::Orthographic => "Orthographic",
            Projection::Cylindrical => "Cylindrical",
            Projection::CylindricalVr => "Cylindrical VR",
            Projection::Spherical => "Spherical",
            Projection::SphericalVr => "Spherical VR",
        }
    }
    /// Spherical and Cylindrical (all four): renders only; views show the
    /// camera as a perspective one.
    pub fn is_panoramic(self) -> bool {
        !matches!(self, Projection::Perspective | Projection::Orthographic)
    }
}

/// How the film back is matched to a picture of another shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilmFit {
    /// The film covers the whole picture (some film is cropped).
    Fill,
    /// The whole film shows inside the picture (with a margin).
    Overscan,
    /// The film's width spans the picture's width.
    #[default]
    Horizontal,
    /// The film's height spans the picture's height.
    Vertical,
}

impl FilmFit {
    pub const ALL: [FilmFit; 4] = [
        FilmFit::Fill,
        FilmFit::Overscan,
        FilmFit::Horizontal,
        FilmFit::Vertical,
    ];
    pub fn label(self) -> &'static str {
        match self {
            FilmFit::Fill => "Fill",
            FilmFit::Overscan => "Overscan",
            FilmFit::Horizontal => "Horizontal",
            FilmFit::Vertical => "Vertical",
        }
    }
}

/// A camera's own channels. Lengths are metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraSettings {
    pub projection: Projection,
    pub focal_length: f64,
    pub film_width: f64,
    pub film_height: f64,
    pub film_offset: [f64; 2],
    pub film_fit: FilmFit,
    pub target_distance: f64,
    /// Focus Distance, metres (spec v4).
    #[serde(default = "default_focus_distance")]
    pub focus_distance: f64,
    pub clipping: bool,
    pub clip_near: f64,
    pub clip_far: f64,
    /// Channels kept and shown but not used yet (observed, spec v1).
    #[serde(default)]
    pub stored: CameraStored,
    /// CP-V16 Lens Distortion: stored only (CAM-Q03).
    #[serde(default)]
    pub lens_distortion: f64,
    /// CP-E01..E07 Depth of Field (renders only, CP-Q6).
    #[serde(default)]
    pub dof: crate::camera_props::DepthOfField,
    /// CP-V17..V19 Resolution Override.
    #[serde(default)]
    pub resolution: crate::camera_props::ResolutionOverride,
    /// CP-V25..V27 Background Image.
    #[serde(default)]
    pub background: crate::camera_props::CameraBackground,
    /// CP-O Camera Overlay, present while Show Camera Overlay is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay: Option<crate::camera_props::CameraOverlay>,
    /// CP-V10 Set Focus Distance: Focus Distance follows the target.
    #[serde(default)]
    pub focus_from_target: bool,
}

/// Camera channels Operandi keeps and shows but doesn't use yet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CameraStored {
    pub lens_squeeze: f64,
    pub film_roll: f64,
    pub display_size: f64,
    pub overscan: f64,
}

impl Default for CameraStored {
    fn default() -> Self {
        Self {
            lens_squeeze: defaults::LENS_SQUEEZE,
            film_roll: defaults::FILM_ROLL,
            display_size: defaults::CAMERA_DISPLAY_SIZE,
            overscan: defaults::OVERSCAN,
        }
    }
}

/// Light channels Operandi keeps and shows but doesn't use yet (observed,
/// spec v1). The light material's channels live on the light itself, so
/// the material belongs to its light and goes when the light is deleted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LightStored {
    pub visible_to_reflection: bool,
    pub visible_to_refraction: bool,
    pub samples: u32,
    pub importance: f64,
    pub display_size: f64,
    pub shadow_map_size: f64,
    pub shadow_color: [f64; 3],
    pub affect_caustics: f64,
    pub affect_subsurface: f64,
    pub material_enabled: bool,
    pub opacity: f64,
}

impl Default for LightStored {
    fn default() -> Self {
        Self {
            visible_to_reflection: defaults::LIGHT_VISIBLE_TO_REFLECTION,
            visible_to_refraction: defaults::LIGHT_VISIBLE_TO_REFRACTION,
            samples: defaults::LIGHT_SAMPLES,
            importance: defaults::LIGHT_IMPORTANCE,
            display_size: defaults::LIGHT_DISPLAY_SIZE,
            shadow_map_size: defaults::SHADOW_MAP_SIZE,
            shadow_color: defaults::LIGHT_SHADOW_COLOR,
            affect_caustics: defaults::LIGHT_AFFECT_CAUSTICS,
            affect_subsurface: defaults::LIGHT_AFFECT_SUBSURFACE,
            material_enabled: defaults::LIGHT_MATERIAL_ENABLED,
            opacity: defaults::LIGHT_MATERIAL_OPACITY,
        }
    }
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            projection: Projection::Perspective,
            focal_length: defaults::FOCAL_LENGTH,
            film_width: defaults::FILM_WIDTH,
            film_height: defaults::FILM_HEIGHT,
            film_offset: defaults::FILM_OFFSET,
            film_fit: defaults::FILM_FIT,
            target_distance: defaults::TARGET_DISTANCE,
            focus_distance: defaults::FOCUS_DISTANCE,
            clipping: defaults::CLIPPING,
            clip_near: defaults::CLIP_NEAR,
            clip_far: defaults::CLIP_FAR,
            stored: Default::default(),
            lens_distortion: 0.0,
            dof: Default::default(),
            resolution: Default::default(),
            background: Default::default(),
            overlay: None,
            focus_from_target: false,
        }
    }
}

fn default_focus_distance() -> f64 {
    defaults::FOCUS_DISTANCE
}

/// Angle of View across the film width: 2·atan(Film Width / (2·Focal Length)).
pub fn angle_of_view(focal_length: f64, film_width: f64) -> f64 {
    2.0 * (film_width / (2.0 * focal_length)).atan()
}

impl CameraSettings {
    /// The derived Angle of View, radians.
    pub fn angle_of_view(&self) -> f64 {
        angle_of_view(self.focal_length, self.film_width)
    }

    /// Whether `fit` matches the film's width to the picture's for a
    /// picture of `aspect` (width / height).
    fn fits_width(&self, aspect: f64) -> bool {
        let film_aspect = self.film_width / self.film_height;
        match self.film_fit {
            FilmFit::Horizontal => true,
            FilmFit::Vertical => false,
            // A picture wider than the film: matching widths covers it.
            FilmFit::Fill => aspect >= film_aspect,
            FilmFit::Overscan => aspect < film_aspect,
        }
    }

    /// Half the picture's width and height at unit distance in front of
    /// the camera (the tangents of half the horizontal and vertical field
    /// of view) for a picture of `aspect` (width / height), after Film Fit.
    pub fn half_tangents(&self, aspect: f64) -> (f64, f64) {
        let aspect = aspect.max(1e-6);
        if self.fits_width(aspect) {
            let w = self.film_width / (2.0 * self.focal_length);
            (w, w / aspect)
        } else {
            let h = self.film_height / (2.0 * self.focal_length);
            (h * aspect, h)
        }
    }

    /// Half the picture's width and height in metres for an orthographic
    /// camera. Provisional (CL-Q02): the frame a perspective camera with
    /// the same lens would see at the Target Distance.
    pub fn orthographic_half_size(&self, aspect: f64) -> (f64, f64) {
        let (w, h) = self.half_tangents(aspect);
        (w * self.target_distance, h * self.target_distance)
    }

    /// Film Offset as a shift of the picture, in units of half the
    /// picture's width and height. Provisional: the offset is measured on
    /// the film after Film Fit.
    pub fn film_shift(&self, aspect: f64) -> [f64; 2] {
        let (w, h) = self.half_tangents(aspect);
        // Film size that spans the picture after the fit, at focal distance.
        let (film_w, film_h) = (2.0 * w * self.focal_length, 2.0 * h * self.focal_length);
        [
            2.0 * self.film_offset[0] / film_w.max(1e-12),
            2.0 * self.film_offset[1] / film_h.max(1e-12),
        ]
    }

    pub fn validate(&self) -> Result<()> {
        let positive = |v: f64| v.is_finite() && v > 0.0 && v < 1e6;
        if !positive(self.focal_length)
            || !positive(self.film_width)
            || !positive(self.film_height)
            || !positive(self.target_distance)
            || !positive(self.focus_distance)
            || !self
                .film_offset
                .iter()
                .all(|v| v.is_finite() && v.abs() < 1e6)
            || !positive(self.clip_near)
            || !positive(self.clip_far)
            || self.clip_far <= self.clip_near
            || !(self.lens_distortion.is_finite() && self.lens_distortion.abs() <= 10.0)
            || !(self.stored.lens_squeeze.is_finite()
                && self.stored.lens_squeeze > 0.0
                && self.stored.lens_squeeze < 1e3)
            || !self.stored.film_roll.is_finite()
            || !(self.stored.display_size.is_finite()
                && (0.0..1e6).contains(&self.stored.display_size))
            || !(self.stored.overscan.is_finite()
                && self.stored.overscan > 0.0
                && self.stored.overscan < 1e3)
            || !(1..=crate::camera_props::MAX_RESOLUTION).contains(&self.resolution.width)
            || !(1..=crate::camera_props::MAX_RESOLUTION).contains(&self.resolution.height)
        {
            return Err("Invalid camera settings".into());
        }
        self.dof.validate()?;
        if let Some(o) = &self.overlay {
            o.validate()?;
        }
        Ok(())
    }
}

/// Kinds of light (CL-1 directional; CL-2 point, spot and area).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightKind {
    #[default]
    Directional,
    Point,
    Spot,
    Area,
    /// LP-PO: a window helper that makes no light of its own; it tells the
    /// renderer where sky light comes in (a Cycles area-light portal).
    Portal,
    /// LP-CY: a glowing tube along local X, ends dark (LPQ-02).
    Cylinder,
    /// LP-MS: a mesh item's shape glows.
    Mesh,
    /// LP-PH: a real fixture's light from an IES file.
    Photometric,
    /// LP-DM-01: even light from a huge sphere round the scene.
    Dome,
}

impl LightKind {
    pub const ALL: [LightKind; 9] = [
        LightKind::Directional,
        LightKind::Point,
        LightKind::Spot,
        LightKind::Area,
        LightKind::Cylinder,
        LightKind::Photometric,
        LightKind::Mesh,
        LightKind::Dome,
        LightKind::Portal,
    ];
    /// Short name, as in the Create Light menu.
    pub fn label(self) -> &'static str {
        match self {
            LightKind::Directional => "Directional",
            LightKind::Point => "Point",
            LightKind::Spot => "Spot",
            LightKind::Area => "Area",
            LightKind::Portal => "Portal",
            LightKind::Cylinder => "Cylinder",
            LightKind::Mesh => "Mesh",
            LightKind::Photometric => "Photometric",
            LightKind::Dome => "Dome",
        }
    }
    /// A new item's name.
    pub fn item_name(self) -> &'static str {
        match self {
            LightKind::Directional => defaults::DIRECTIONAL_LIGHT_NAME,
            LightKind::Point => defaults::POINT_LIGHT_NAME,
            LightKind::Spot => defaults::SPOT_LIGHT_NAME,
            LightKind::Area => defaults::AREA_LIGHT_NAME,
            LightKind::Portal => defaults::PORTAL_NAME,
            LightKind::Cylinder => defaults::CYLINDER_LIGHT_NAME,
            LightKind::Mesh => defaults::MESH_LIGHT_NAME,
            LightKind::Photometric => defaults::PHOTOMETRIC_LIGHT_NAME,
            LightKind::Dome => defaults::DOME_LIGHT_NAME,
        }
    }
    /// What the Intensity channel measures, for its label (LP-12). A
    /// portal has no intensity row; its stored value keeps the area label.
    pub fn intensity_label(self) -> &'static str {
        match self {
            LightKind::Directional => "Radiant Exitance",
            LightKind::Point | LightKind::Spot | LightKind::Photometric => "Radiant Intensity",
            LightKind::Area
            | LightKind::Portal
            | LightKind::Cylinder
            | LightKind::Mesh
            | LightKind::Dome => "Radiance",
        }
    }
    /// LP-04: kinds with Targeting rows (not Point or Mesh, which have no
    /// facing).
    pub fn has_targeting(self) -> bool {
        !matches!(self, LightKind::Point | LightKind::Mesh | LightKind::Dome)
    }
    /// LP-40: kinds with a Volumetrics section.
    pub fn has_volumetrics(self) -> bool {
        matches!(
            self,
            LightKind::Directional | LightKind::Point | LightKind::Spot | LightKind::Photometric
        )
    }
    /// Kinds with the Visible rows and LPE Label (not Dome or Portal).
    pub fn has_visible_rows(self) -> bool {
        !matches!(self, LightKind::Dome | LightKind::Portal)
    }
    /// The Properties tab's name (LP-01), e.g. "Directional Light".
    pub fn tab_name(self) -> &'static str {
        match self {
            LightKind::Portal => "Portal",
            _ => self.item_name(),
        }
    }
    /// LP-PO: a portal makes no light; it has no intensity, colour,
    /// falloff, shadow, Dissolve, Visible or light material rows.
    pub fn emits(self) -> bool {
        self != LightKind::Portal
    }
    /// Kinds with a Width × Height outline (area light, portal).
    pub fn has_area(self) -> bool {
        matches!(self, LightKind::Area | LightKind::Portal)
    }
    /// The Intensity field's unit: "W/srm²" for every kind, as observed
    /// (spec v3), even for a directional light whose Radiant Exitance is
    /// really W/m² (logged as odd, CL-Q04).
    pub fn intensity_unit(self) -> &'static str {
        " W/srm²"
    }
}

/// How a point, spot or area light dims with distance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightFalloff {
    None,
    InverseDistance,
    #[default]
    InverseDistanceSquared,
}

impl LightFalloff {
    pub const ALL: [LightFalloff; 3] = [
        LightFalloff::None,
        LightFalloff::InverseDistance,
        LightFalloff::InverseDistanceSquared,
    ];
    pub fn label(self) -> &'static str {
        match self {
            LightFalloff::None => "None",
            LightFalloff::InverseDistance => "Inverse Distance",
            LightFalloff::InverseDistanceSquared => "Inverse Distance Squared",
        }
    }
}

/// An area light's outline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AreaShape {
    #[default]
    Rectangular,
    Elliptical,
}

impl AreaShape {
    pub const ALL: [AreaShape; 2] = [AreaShape::Rectangular, AreaShape::Elliptical];
    /// LP-AR-01 / LP-PO-02 labels.
    pub fn label(self) -> &'static str {
        match self {
            AreaShape::Rectangular => "Rectangle",
            AreaShape::Elliptical => "Ellipse",
        }
    }
}

/// The light's Render channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightRender {
    /// Renders when the item is visible.
    #[default]
    Default,
    On,
    Off,
}

impl LightRender {
    pub const ALL: [LightRender; 3] = [LightRender::Default, LightRender::On, LightRender::Off];
    /// LP-10 labels: Default, Yes, No.
    pub fn label(self) -> &'static str {
        match self {
            LightRender::Default => "Default",
            LightRender::On => "Yes",
            LightRender::Off => "No",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShadowType {
    None,
    #[default]
    RayTraced,
}

impl ShadowType {
    pub const ALL: [ShadowType; 2] = [ShadowType::None, ShadowType::RayTraced];
    pub fn label(self) -> &'static str {
        match self {
            ShadowType::None => "None",
            ShadowType::RayTraced => "Ray Traced",
        }
    }
}

/// A light's own channels. Percentages are fractions; angles radians.
/// Colour and Affect Diffuse/Specular belong to the light's material in a
/// shader tree; until Operandi has one they live here (CL-1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LightSettings {
    pub kind: LightKind,
    pub render: LightRender,
    pub dissolve: f64,
    /// Radiant exitance, W/m².
    pub intensity: f64,
    pub shadow: ShadowType,
    pub visible_to_camera: bool,
    pub spread_angle: f64,
    pub color: [f64; 3],
    pub affect_diffuse: f64,
    pub affect_specular: f64,
    /// Point and Spot: the emitting sphere's radius, metres.
    #[serde(default)]
    pub radius: f64,
    /// Point, Spot and Area.
    #[serde(default)]
    pub falloff: LightFalloff,
    /// Spot: the full cone angle, radians.
    #[serde(default = "default_cone_angle")]
    pub cone_angle: f64,
    /// Spot: the fading band inside the cone's edge, radians.
    #[serde(default = "default_soft_edge")]
    pub soft_edge: f64,
    /// Area: outline and size in metres (the item's scale multiplies them).
    #[serde(default)]
    pub shape: AreaShape,
    #[serde(default = "default_area_width")]
    pub width: f64,
    #[serde(default = "default_area_height")]
    pub height: f64,
    /// Portal: LP-PO-04 Environment Only. Stored and shown; Cycles portals
    /// only ever gather environment light, so on and off render alike.
    #[serde(default)]
    pub environment_only: bool,
    /// Cylinder: tube length along local X, metres (its Radius is
    /// `radius`).
    #[serde(default = "default_cylinder_length")]
    pub length: f64,
    /// Mesh light: the mesh item whose shape glows (LP-MS-01); none emits
    /// nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prototype: Option<u64>,
    /// Mesh light: on = the glow sits where the mesh is; off = the mesh's
    /// shape placed by the light's own transform (LP-MS-02).
    #[serde(default = "default_true")]
    pub use_prototype_transform: bool,
    /// Photometric: the IES file, kept as a path like an image (LP-PH-01).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub photometry_file: String,
    /// Directional: the Physical Sun section (LP-SUN).
    #[serde(default, skip_serializing_if = "is_default_sun")]
    pub sun: crate::physical_sun::PhysicalSun,
    /// LP-40..44 and LM-30..33.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumetrics: Option<Volumetrics>,
    /// LP-22: names this light's own render pass (a Cycles light group).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lpe_label: String,
    /// LM-03 Blend Mode of the light material.
    #[serde(default, skip_serializing_if = "is_normal_blend")]
    pub blend_mode: LightBlend,
    /// Image layers above the light material (gobos), bottom first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<LightLayer>,
    /// LP-30..32 Light Linking; `None` is the defaults (off).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linking: Option<LightLinking>,
    /// Channels kept and shown but not used yet (observed, spec v1).
    #[serde(default)]
    pub stored: LightStored,
}

fn default_cone_angle() -> f64 {
    defaults::SPOT_CONE_ANGLE_DEGREES.to_radians()
}
fn default_soft_edge() -> f64 {
    defaults::SPOT_SOFT_EDGE_DEGREES.to_radians()
}
fn default_area_width() -> f64 {
    defaults::AREA_WIDTH
}
fn default_area_height() -> f64 {
    defaults::AREA_HEIGHT
}
fn default_cylinder_length() -> f64 {
    defaults::CYLINDER_LENGTH
}
fn default_true() -> bool {
    true
}
fn is_default_sun(sun: &crate::physical_sun::PhysicalSun) -> bool {
    *sun == Default::default()
}
fn is_normal_blend(b: &LightBlend) -> bool {
    *b == LightBlend::Normal
}

/// LM-03 / layer blend modes (the ones Cycles' Mix Color has).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightBlend {
    #[default]
    Normal,
    Add,
    Subtract,
    Multiply,
    Screen,
    Overlay,
}

impl LightBlend {
    pub const ALL: [LightBlend; 6] = [
        LightBlend::Normal,
        LightBlend::Add,
        LightBlend::Subtract,
        LightBlend::Multiply,
        LightBlend::Screen,
        LightBlend::Overlay,
    ];
    pub fn label(self) -> &'static str {
        match self {
            LightBlend::Normal => "Normal",
            LightBlend::Add => "Add",
            LightBlend::Subtract => "Subtract",
            LightBlend::Multiply => "Multiply",
            LightBlend::Screen => "Screen",
            LightBlend::Overlay => "Overlay",
        }
    }
    /// The colour `top` blended over `base` at `opacity` (0..1), as
    /// Cycles' Mix Color of the same mode computes it.
    pub fn mix(self, base: [f64; 3], top: [f64; 3], opacity: f64) -> [f64; 3] {
        let t = opacity.clamp(0.0, 1.0);
        let blended = |b: f64, c: f64| match self {
            LightBlend::Normal => c,
            LightBlend::Add => b + c,
            LightBlend::Subtract => b - c,
            LightBlend::Multiply => b * c,
            LightBlend::Screen => 1.0 - (1.0 - b) * (1.0 - c),
            LightBlend::Overlay => {
                if b < 0.5 {
                    2.0 * b * c
                } else {
                    1.0 - 2.0 * (1.0 - b) * (1.0 - c)
                }
            }
        };
        [0, 1, 2].map(|i| base[i] + (blended(base[i], top[i]) - base[i]) * t)
    }
}

/// An image layer on the light material: a gobo / slide projector picture
/// the light shines through.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LightLayer {
    pub image: String,
    pub enabled: bool,
    pub blend: LightBlend,
    /// 0..1.
    pub opacity: f64,
}

impl Default for LightLayer {
    fn default() -> Self {
        Self {
            image: String::new(),
            enabled: true,
            blend: LightBlend::Multiply,
            opacity: 1.0,
        }
    }
}

/// LP-40..44 and LM-30..33: the visible beam. Lengths metres, percentages
/// fractions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Volumetrics {
    pub enabled: bool,
    /// LP-41, stored only (Cycles has no per-volume shadow samples).
    pub samples: u32,
    /// LP-42 (directional, spot).
    pub height: f64,
    /// LP-43: directional Radius / spot Base.
    pub radius: f64,
    /// LP-44 (point, photometric).
    pub sphere_radius: f64,
    /// LM-30..33.
    pub color: [f64; 3],
    pub density: f64,
    pub attenuation: f64,
    /// −1..1.
    pub shift: f64,
}

impl Volumetrics {
    /// A new light's Volumetrics (observed defaults, LP-4x / LM-3x).
    pub fn new(kind: LightKind) -> Self {
        Self {
            enabled: false,
            samples: 40,
            height: 10.0,
            radius: if kind == LightKind::Directional {
                0.5
            } else {
                0.0
            },
            sphere_radius: 1.0,
            color: [1.0; 3],
            density: 0.5,
            attenuation: 0.1,
            shift: 0.0,
        }
    }
    pub fn validate(&self) -> Result<()> {
        let len = |v: f64| v.is_finite() && (0.0..1e6).contains(&v);
        if !(len(self.height) && self.height > 0.0)
            || !len(self.radius)
            || !len(self.sphere_radius)
            || !self
                .color
                .iter()
                .all(|c| c.is_finite() && (0.0..1e3).contains(c))
            || !(self.density.is_finite() && (0.0..=1e3).contains(&self.density))
            || !(self.attenuation.is_finite() && (0.0..=1.0).contains(&self.attenuation))
            || !(self.shift.is_finite() && (-1.0..=1.0).contains(&self.shift))
            || self.samples == 0
        {
            return Err("Invalid Volumetrics settings".into());
        }
        Ok(())
    }
}

/// One Volumetrics row.
#[derive(Clone, Debug, PartialEq)]
pub enum VolumetricEdit {
    Enabled(bool),
    Samples(u32),
    Height(f64),
    Radius(f64),
    SphereRadius(f64),
    Color([f64; 3]),
    Density(f64),
    Attenuation(f64),
    Shift(f64),
}

impl VolumetricEdit {
    pub fn label(&self) -> &'static str {
        match self {
            VolumetricEdit::Enabled(_) => "Volumetrics",
            VolumetricEdit::Samples(_) => "Volumetric Samples",
            VolumetricEdit::Height(_) => "Volumetric Height",
            VolumetricEdit::Radius(_) => "Volumetric Radius",
            VolumetricEdit::SphereRadius(_) => "Sphere Radius",
            VolumetricEdit::Color(_) => "Scattering Color",
            VolumetricEdit::Density(_) => "Density",
            VolumetricEdit::Attenuation(_) => "Light Attenuation",
            VolumetricEdit::Shift(_) => "Light Shift",
        }
    }
}

/// LP-32 Mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkMode {
    /// The light affects only the group.
    Include,
    /// The light affects everything but the group.
    #[default]
    Exclude,
}

impl LinkMode {
    pub const ALL: [LinkMode; 2] = [LinkMode::Include, LinkMode::Exclude];
    pub fn label(self) -> &'static str {
        match self {
            LinkMode::Include => "Include",
            LinkMode::Exclude => "Exclude",
        }
    }
}

/// LP-30..32 Light Linking.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LightLinking {
    /// LP-30 Control Light Linking.
    pub control: bool,
    /// LP-31 Item Group (a group id); none = no group.
    pub group: Option<u64>,
    /// LP-32.
    pub mode: LinkMode,
}

impl LightLinking {
    /// Whether this light lights item `id`, given its group's members
    /// (`None`: no group, or the group is gone). Include with no group or
    /// an empty one lights nothing; Exclude with none lights everything.
    pub fn lights(&self, id: u64, members: Option<&[u64]>) -> bool {
        if !self.control {
            return true;
        }
        let inside = members.is_some_and(|m| m.contains(&id));
        match self.mode {
            LinkMode::Include => inside,
            LinkMode::Exclude => !inside,
        }
    }
}

/// One light-material layer row.
#[derive(Clone, Debug, PartialEq)]
pub enum LayerEdit {
    Add(String),
    Remove(usize),
    Image(usize, String),
    Enabled(usize, bool),
    Blend(usize, LightBlend),
    Opacity(usize, f64),
}

impl Default for LightSettings {
    fn default() -> Self {
        Self::new(LightKind::Directional)
    }
}

impl LightSettings {
    /// A new light of `kind` with its defaults.
    pub fn new(kind: LightKind) -> Self {
        Self {
            kind,
            render: LightRender::Default,
            dissolve: defaults::LIGHT_DISSOLVE,
            intensity: match kind {
                LightKind::Directional => defaults::LIGHT_INTENSITY,
                LightKind::Point => defaults::POINT_LIGHT_INTENSITY,
                LightKind::Spot => defaults::SPOT_LIGHT_INTENSITY,
                _ => defaults::AREA_LIGHT_INTENSITY,
            },
            shadow: ShadowType::RayTraced,
            visible_to_camera: defaults::LIGHT_VISIBLE_TO_CAMERA,
            spread_angle: defaults::LIGHT_SPREAD_ANGLE,
            color: defaults::LIGHT_COLOR,
            affect_diffuse: defaults::LIGHT_AFFECT_DIFFUSE,
            affect_specular: defaults::LIGHT_AFFECT_SPECULAR,
            radius: match kind {
                LightKind::Cylinder => defaults::CYLINDER_RADIUS,
                _ => defaults::LIGHT_RADIUS,
            },
            falloff: defaults::LIGHT_FALLOFF,
            // LP-PH-04: a photometric light starts with no cone limit;
            // LP-PH-03: no size until a file sets it.
            cone_angle: match kind {
                LightKind::Photometric => std::f64::consts::TAU,
                _ => default_cone_angle(),
            },
            soft_edge: match kind {
                LightKind::Photometric => 0.0,
                _ => default_soft_edge(),
            },
            shape: defaults::AREA_SHAPE,
            width: match kind {
                LightKind::Photometric => 0.0,
                _ => defaults::AREA_WIDTH,
            },
            height: match kind {
                LightKind::Photometric => 0.0,
                _ => defaults::AREA_HEIGHT,
            },
            environment_only: defaults::PORTAL_ENVIRONMENT_ONLY,
            length: defaults::CYLINDER_LENGTH,
            prototype: None,
            use_prototype_transform: true,
            photometry_file: String::new(),
            sun: Default::default(),
            volumetrics: None,
            linking: None,
            lpe_label: String::new(),
            blend_mode: LightBlend::Normal,
            layers: Vec::new(),
            stored: Default::default(),
        }
    }

    /// This light switched to `kind` (the Type dropdown, MCP `light_set
    /// type`; provisional, MCP-Q02): the shared settings stay (intensity
    /// number, colour, shadow, render, visibility, light material, linking),
    /// settings both types use stay, and the new type's own settings start
    /// at its defaults. Intensity is converted so the light is about as
    /// bright as before (`converted_intensity`; Jason, 2026-11-07).
    pub fn with_kind(&self, kind: LightKind) -> LightSettings {
        use LightKind as K;
        if kind == self.kind {
            return self.clone();
        }
        let fresh = LightSettings::new(kind);
        let mut next = self.clone();
        next.kind = kind;
        let both = |kinds: &[K]| kinds.contains(&self.kind) && kinds.contains(&kind);
        if !both(&[K::Point, K::Spot, K::Cylinder]) {
            next.radius = fresh.radius;
        }
        if !both(&[K::Point, K::Spot, K::Area]) {
            next.falloff = fresh.falloff;
        }
        if !both(&[K::Spot]) {
            next.cone_angle = fresh.cone_angle;
            next.soft_edge = fresh.soft_edge;
        }
        if !both(&[K::Area, K::Portal]) {
            next.shape = fresh.shape;
            next.width = fresh.width;
            next.height = fresh.height;
        }
        if !both(&[K::Portal]) {
            next.environment_only = fresh.environment_only;
        }
        if !both(&[K::Cylinder]) {
            next.length = fresh.length;
        }
        if !both(&[K::Mesh]) {
            next.prototype = fresh.prototype;
            next.use_prototype_transform = fresh.use_prototype_transform;
        }
        if !both(&[K::Photometric]) {
            next.photometry_file = fresh.photometry_file.clone();
        }
        if !both(&[K::Directional]) {
            next.sun = fresh.sun.clone();
        }
        if !kind.has_volumetrics() {
            next.volumetrics = None;
        }
        if let Some(intensity) = self.converted_intensity(&next) {
            next.intensity = intensity;
        }
        next
    }

    /// Irradiance (W/m²) one unit of Intensity gives at the reference point:
    /// 1 m in front of the light, on its axis, facing it (LT-Q01). Lights
    /// that are small next to 1 m act as a point there: a surface emitter of
    /// radiance L and area A is a point of radiant intensity L·A (far-field
    /// rule, used for Area and Cylinder whatever their size). `None` when
    /// the light's brightness can't be compared this way (Mesh, whose
    /// glowing area is another item's; Portal, which emits nothing).
    pub fn irradiance_per_intensity(&self) -> Option<f64> {
        const REFERENCE_DISTANCE: f64 = 1.0;
        let d2 = REFERENCE_DISTANCE * REFERENCE_DISTANCE;
        match self.kind {
            // Sun strength: irradiance at any distance.
            LightKind::Directional => Some(defaults::CYCLES_SUN_STRENGTH_PER_INTENSITY),
            // Radiant intensity I: E = I / d².
            LightKind::Point | LightKind::Spot | LightKind::Photometric => Some(1.0 / d2),
            // Radiance L over the outline: I = L·A on the axis.
            LightKind::Area => {
                let area = match self.shape {
                    AreaShape::Rectangular => self.width * self.height,
                    AreaShape::Elliptical => std::f64::consts::FRAC_PI_4 * self.width * self.height,
                };
                Some(area / d2)
            }
            // A glowing tube seen side-on: projected area 2r × length.
            LightKind::Cylinder => Some(2.0 * self.radius * self.length / d2),
            // A uniform sky of radiance L lights an upward surface with πL.
            LightKind::Dome => Some(std::f64::consts::PI),
            LightKind::Mesh | LightKind::Portal => None,
        }
    }

    /// The Intensity `next` (this light as another type) needs to give the
    /// same irradiance at the reference point; `None` keeps the number
    /// (either type has no comparable brightness, or the new one is
    /// degenerate).
    pub fn converted_intensity(&self, next: &LightSettings) -> Option<f64> {
        let from = self.irradiance_per_intensity()?;
        let to = next.irradiance_per_intensity()?;
        if !(to.is_finite() && to > 1e-12 && from.is_finite()) {
            return None;
        }
        let v = self.intensity * from / to;
        v.is_finite().then(|| v.clamp(0.0, 1e8))
    }

    /// Spot: the angle from the axis within which the light is at full
    /// strength (half the cone less the soft edge), radians.
    pub fn spot_full_angle(&self) -> f64 {
        (self.cone_angle / 2.0 - self.soft_edge).max(0.0)
    }

    /// Spot: Cycles' spot blend that puts full strength at the edge of the
    /// Soft Edge band (spec v2): (cos(h − e) − cos h) / (1 − cos h), with
    /// h half the cone and e the Soft Edge. 0 is a hard cut.
    pub fn spot_blend(&self) -> f64 {
        let cos_h = (self.cone_angle / 2.0).cos();
        if 1.0 - cos_h < 1e-12 {
            return 0.0;
        }
        ((self.spot_full_angle().cos() - cos_h) / (1.0 - cos_h)).clamp(0.0, 1.0)
    }

    /// Spot: strength at `angle` radians off the axis, as Cycles computes it
    /// with `spot_blend` (smoothstep in cos from the cone's edge to the
    /// inside of the Soft Edge).
    pub fn spot_attenuation(&self, angle: f64) -> f64 {
        let cos_h = (self.cone_angle / 2.0).cos();
        let blend = self.spot_blend();
        if blend <= 0.0 {
            return if angle.cos() > cos_h { 1.0 } else { 0.0 };
        }
        let x = ((angle.cos() - cos_h) / ((1.0 - cos_h) * blend)).clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    }
}

impl LightSettings {
    /// Whether the light takes part in a render, given its item's
    /// visibility: Render Off never does, Default follows visibility.
    pub fn renders(&self, item_visible: bool) -> bool {
        match self.render {
            LightRender::Off => false,
            LightRender::On => true,
            LightRender::Default => item_visible,
        }
    }

    pub fn validate(&self) -> Result<()> {
        let fraction = |v: f64| (0.0..=1.0).contains(&v);
        if !fraction(self.dissolve)
            || !(self.intensity.is_finite() && (0.0..1e9).contains(&self.intensity))
            || !(0.0..=std::f64::consts::PI).contains(&self.spread_angle)
            || !self
                .color
                .iter()
                .all(|c| c.is_finite() && (0.0..1e6).contains(c))
            || !(self.affect_diffuse.is_finite() && (0.0..1e3).contains(&self.affect_diffuse))
            || !(self.affect_specular.is_finite() && (0.0..1e3).contains(&self.affect_specular))
            || !(self.radius.is_finite() && (0.0..1e6).contains(&self.radius))
            // Up to 360°: a full cone is like a point light (LP-SP-02).
            || !(self.cone_angle.is_finite()
                && self.cone_angle > 0.0
                && self.cone_angle <= std::f64::consts::TAU + 1e-9)
            || !(self.soft_edge.is_finite() && self.soft_edge >= 0.0)
            || !(self.width.is_finite() && self.width >= 0.0 && self.width < 1e6)
            || !(self.height.is_finite() && self.height >= 0.0 && self.height < 1e6)
            // Area and portal outlines need a size; a photometric light's
            // Width/Height may be 0 (LP-PH-03).
            || (self.kind.has_area() && (self.width <= 0.0 || self.height <= 0.0))
            || !(self.length.is_finite() && self.length > 0.0 && self.length < 1e6)
            || self.photometry_file.len() > 4096
            || self.lpe_label.len() > 256
            || self.layers.len() > 16
            || !self
                .layers
                .iter()
                .all(|l| l.opacity.is_finite() && (0.0..=1.0).contains(&l.opacity) && l.image.len() <= 4096)
            || !(self.stored.importance.is_finite() && (0.0..1e3).contains(&self.stored.importance))
            || !(self.stored.opacity.is_finite() && (0.0..=1.0).contains(&self.stored.opacity))
            || !(self.stored.affect_subsurface.is_finite()
                && (0.0..1e3).contains(&self.stored.affect_subsurface))
            || !(self.stored.affect_caustics.is_finite()
                && (0.0..=1.0).contains(&self.stored.affect_caustics))
        {
            return Err("Invalid light settings".into());
        }
        if let Some(v) = &self.volumetrics {
            v.validate()?;
        }
        self.sun.validate()
    }
}

/// One camera panel control, so each change is its own undo step.
#[derive(Clone, Debug, PartialEq)]
pub enum CameraEdit {
    Projection(Projection),
    FocalLength(f64),
    /// Setting the Angle of View changes the Focal Length (film kept).
    AngleOfView(f64),
    FilmWidth(f64),
    FilmHeight(f64),
    FilmOffsetX(f64),
    FilmOffsetY(f64),
    FilmFit(FilmFit),
    TargetDistance(f64),
    FocusDistance(f64),
    Clipping(bool),
    ClipNear(f64),
    ClipFar(f64),
    LensDistortion(f64),
    LensSqueeze(f64),
    /// Radians.
    FilmRoll(f64),
    /// CP-D04 Display Size (0 = constant screen size).
    DisplaySize(f64),
    /// CP-V36: Film Width and Height together (one undo step).
    FilmBack {
        width: f64,
        height: f64,
    },
    DofEnable(bool),
    FStop(f64),
    IrisBlades(u32),
    /// Radians.
    IrisRotation(f64),
    /// 0..1.
    EdgeWeighting(f64),
    ResolutionEnable(bool),
    ResolutionWidth(u32),
    ResolutionHeight(u32),
    /// Resolution Override on at this size (Set Resolution from Clip).
    ResolutionFromClip {
        width: u32,
        height: u32,
    },
    BackgroundEnable(bool),
    /// A scene image id, or none (checked by the editor).
    BackgroundImage(Option<u64>),
    /// Fraction (1 = 100 %).
    Overscan(f64),
    /// Display > Show Camera Overlay.
    ShowOverlay(bool),
    Overlay(OverlayEdit),
    FocusFromTarget(bool),
}

/// One Camera Overlay row (CP-O01..O14).
#[derive(Clone, Debug, PartialEq)]
pub enum OverlayEdit {
    Opacity(f64),
    ActionSafe(bool),
    /// Axis 0 horizontal, 1 vertical; fraction per side.
    ActionMargin(usize, f64),
    TitleSafe(bool),
    TitleMargin(usize, f64),
    SafeColor([f64; 3]),
    Grid(crate::camera_props::OverlayGrid),
    GridColor([f64; 3]),
    RightDiagonal(bool),
    LeftDiagonal(bool),
    ChevronUp(bool),
    ChevronDown(bool),
    Horizon(bool),
    LineColor([f64; 3]),
}

impl OverlayEdit {
    pub fn label(&self) -> &'static str {
        match self {
            OverlayEdit::Opacity(_) => "Overlay Opacity",
            OverlayEdit::ActionSafe(_) => "Action Safe Visible",
            OverlayEdit::ActionMargin(0, _) => "Action Safe Horizontal Margin",
            OverlayEdit::ActionMargin(..) => "Action Safe Vertical Margin",
            OverlayEdit::TitleSafe(_) => "Title Safe Visible",
            OverlayEdit::TitleMargin(0, _) => "Title Safe Horizontal Margin",
            OverlayEdit::TitleMargin(..) => "Title Safe Vertical Margin",
            OverlayEdit::SafeColor(_) => "Safe Area Color",
            OverlayEdit::Grid(_) => "Grid",
            OverlayEdit::GridColor(_) => "Grid Color",
            OverlayEdit::RightDiagonal(_) => "Right Diagonal",
            OverlayEdit::LeftDiagonal(_) => "Left Diagonal",
            OverlayEdit::ChevronUp(_) => "Chevron Up",
            OverlayEdit::ChevronDown(_) => "Chevron Down",
            OverlayEdit::Horizon(_) => "Horizon",
            OverlayEdit::LineColor(_) => "Line Color",
        }
    }
}

impl CameraEdit {
    /// The undo step's name: the property's (CP-X1).
    pub fn label(&self) -> &'static str {
        match self {
            CameraEdit::Projection(_) => "Projection Type",
            CameraEdit::FocalLength(_) => "Focal Length",
            CameraEdit::AngleOfView(_) => "Angle of View",
            CameraEdit::FilmWidth(_) => "Film Width",
            CameraEdit::FilmHeight(_) => "Film Height",
            CameraEdit::FilmOffsetX(_) => "Film Offset X",
            CameraEdit::FilmOffsetY(_) => "Film Offset Y",
            CameraEdit::FilmFit(_) => "Film Fit",
            CameraEdit::TargetDistance(_) => "Target Distance",
            CameraEdit::FocusDistance(_) => "Focus Distance",
            CameraEdit::Clipping(_) => "Clipping Plane",
            CameraEdit::ClipNear(_) => "Clipping Distance",
            CameraEdit::ClipFar(_) => "Far Clip",
            CameraEdit::LensDistortion(_) => "Lens Distortion",
            CameraEdit::LensSqueeze(_) => "Lens Squeeze",
            CameraEdit::FilmRoll(_) => "Film Roll",
            CameraEdit::DisplaySize(_) => "Size",
            CameraEdit::FilmBack { .. } => "Film Back Preset",
            CameraEdit::DofEnable(_) => "Depth of Field",
            CameraEdit::FStop(_) => "F-Stop",
            CameraEdit::IrisBlades(_) => "Iris Blades",
            CameraEdit::IrisRotation(_) => "Iris Rotation",
            CameraEdit::EdgeWeighting(_) => "Edge Weighting",
            CameraEdit::ResolutionEnable(_) => "Override Render Resolution",
            CameraEdit::ResolutionWidth(_) => "Width",
            CameraEdit::ResolutionHeight(_) => "Height",
            CameraEdit::ResolutionFromClip { .. } => "Set Resolution from Clip",
            CameraEdit::BackgroundEnable(_) => "Background Image Enable",
            CameraEdit::BackgroundImage(_) => "Background Image",
            CameraEdit::Overscan(_) => "Overscan",
            CameraEdit::ShowOverlay(true) => "Show Camera Overlay",
            CameraEdit::ShowOverlay(false) => "Hide Camera Overlay",
            CameraEdit::Overlay(e) => e.label(),
            CameraEdit::FocusFromTarget(_) => "Set Focus Distance",
        }
    }
}

/// A camera or light channel edited from the active Move, Rotate or
/// Transform tool (CL-14): its panel fields and the area light's handles.
#[derive(Clone, Debug, PartialEq)]
pub enum ItemChannelEdit {
    Camera(CameraEdit),
    Light(LightEdit),
    /// A Planar Texture Locator's whole local Transform, from its viewport
    /// handles (planar_texture.rs). The tool's own offset stays on top.
    Transform(crate::scene::Transform),
}

/// The tools whose panel shows the selected camera or light's own
/// channels, and which show the area light's size handles (CL-14).
pub fn tool_shows_item_channels(tool: crate::Tool) -> bool {
    matches!(
        tool,
        crate::Tool::Move | crate::Tool::Rotate | crate::Tool::Transform
    )
}

impl Scene {
    /// Apply `edit` to item `id`'s camera or light settings.
    pub fn apply_item_channel(&mut self, id: u64, edit: &ItemChannelEdit) -> Result<()> {
        let item = self.item_mut(id).ok_or("Missing item")?;
        match edit {
            ItemChannelEdit::Camera(e) => item
                .camera
                .as_mut()
                .ok_or("Select a camera")?
                .apply(e.clone()),
            ItemChannelEdit::Light(e) => item
                .light
                .as_mut()
                .ok_or("Select a light")?
                .apply(e.clone()),
            ItemChannelEdit::Transform(t) => {
                if item.texture_locator().is_none() {
                    return Err("Select a Texture Locator".into());
                }
                if !(t.translation.is_finite() && t.rotation.is_finite() && t.scale.is_finite()) {
                    return Err("Texture Locator transform must be finite".into());
                }
                item.transform = t.clone();
                // Planar spec: Size stops at 1 mm.
                item.transform.scale = crate::texture_locator::clamp_size(t.scale);
                Ok(())
            }
        }
    }
}

/// One light panel control.
#[derive(Clone, Debug, PartialEq)]
pub enum LightEdit {
    /// Change the light's type, keeping its other settings (MCP
    /// `light_set type`; provisional, MCP-Q02: no panel control changes a
    /// light's type).
    Kind(LightKind),
    Render(LightRender),
    Dissolve(f64),
    Intensity(f64),
    Shadow(ShadowType),
    VisibleToCamera(bool),
    SpreadAngle(f64),
    Color([f64; 3]),
    AffectDiffuse(f64),
    AffectSpecular(f64),
    Radius(f64),
    Falloff(LightFalloff),
    ConeAngle(f64),
    SoftEdge(f64),
    Shape(AreaShape),
    Width(f64),
    Height(f64),
    /// LP-20 / LP-21.
    VisibleToReflection(bool),
    VisibleToRefraction(bool),
    /// LM-16 Set Temperature: sets Color from a colour temperature in
    /// kelvins; the temperature itself isn't kept (LPQ-06).
    Temperature(f64),
    /// LP-PO-04.
    EnvironmentOnly(bool),
    /// LP-CY-01.
    Length(f64),
    /// LP-MS-01 / 02.
    Prototype(Option<u64>),
    UsePrototypeTransform(bool),
    /// LP-PH-01/02: a loaded Photometry File and what it sets (the
    /// editor's LoadPhotometry reads the file).
    Photometry {
        path: String,
        intensity: f64,
        width: f64,
        height: f64,
    },
    /// LP-SUN rows.
    Sun(crate::physical_sun::SunEdit),
    /// LP-40..44, LM-30..33.
    Volumetric(VolumetricEdit),
    /// LP-22.
    LpeLabel(String),
    /// LP-18, fraction.
    Importance(f64),
    /// LM-01 Enable.
    MaterialEnabled(bool),
    /// LM-03, LM-04 (fraction).
    BlendMode(LightBlend),
    Opacity(f64),
    /// LM-15, fraction.
    AffectSubsurface(f64),
    /// Direct Caustics spec v1: the light's share of caustics, fraction.
    AffectCaustics(f64),
    /// Light material image layers.
    Layer(LayerEdit),
    /// LP-30..32.
    LinkControl(bool),
    LinkGroup(Option<u64>),
    LinkMode(LinkMode),
}

impl LightEdit {
    /// The undo step's name: the property's label (IC-1 rule, used for the
    /// light rows too).
    pub fn label(&self) -> &'static str {
        match self {
            LightEdit::Kind(_) => "Light Type",
            LightEdit::Render(_) => "Render",
            LightEdit::Dissolve(_) => "Dissolve",
            LightEdit::Intensity(_) => "Light Intensity",
            LightEdit::Shadow(_) => "Shadow Type",
            LightEdit::VisibleToCamera(_) => "Visible to Camera",
            LightEdit::SpreadAngle(_) => "Spread Angle",
            LightEdit::Color(_) => "Light Color",
            LightEdit::AffectDiffuse(_) => "Affect Diffuse",
            LightEdit::AffectSpecular(_) => "Affect Specular",
            LightEdit::Radius(_) => "Radius",
            LightEdit::Falloff(_) => "Falloff Type",
            LightEdit::ConeAngle(_) => "Cone Angle",
            LightEdit::SoftEdge(_) => "Soft Edge",
            LightEdit::Shape(_) => "Shape",
            LightEdit::Width(_) => "Width",
            LightEdit::Height(_) => "Height",
            LightEdit::VisibleToReflection(_) => "Visible to Reflection Rays",
            LightEdit::VisibleToRefraction(_) => "Visible to Refraction Rays",
            LightEdit::Temperature(_) => "Set Temperature",
            LightEdit::EnvironmentOnly(_) => "Environment Only",
            LightEdit::Length(_) => "Length",
            LightEdit::Prototype(_) => "Prototype",
            LightEdit::UsePrototypeTransform(_) => "Use Prototype Transform",
            LightEdit::Photometry { .. } => "Photometry File",
            LightEdit::Sun(e) => e.label(),
            LightEdit::Volumetric(e) => e.label(),
            LightEdit::LpeLabel(_) => "LPE Label",
            LightEdit::Importance(_) => "Importance",
            LightEdit::MaterialEnabled(_) => "Material Enable",
            LightEdit::BlendMode(_) => "Blend Mode",
            LightEdit::Opacity(_) => "Opacity",
            LightEdit::AffectSubsurface(_) => "Affect Subsurface",
            LightEdit::AffectCaustics(_) => "Affect Caustics",
            LightEdit::Layer(LayerEdit::Add(_)) => "Add Image Layer",
            LightEdit::Layer(LayerEdit::Remove(_)) => "Remove Layer",
            LightEdit::Layer(LayerEdit::Image(..)) => "Layer Image",
            LightEdit::Layer(LayerEdit::Enabled(..)) => "Layer Enable",
            LightEdit::Layer(LayerEdit::Blend(..)) => "Layer Blend Mode",
            LightEdit::Layer(LayerEdit::Opacity(..)) => "Layer Opacity",
            LightEdit::LinkControl(_) => "Control Light Linking",
            LightEdit::LinkGroup(_) => "Item Group",
            LightEdit::LinkMode(_) => "Light Linking Mode",
        }
    }
}

/// Lowest and highest temperature Set Temperature takes, kelvins.
pub const KELVIN_RANGE: std::ops::RangeInclusive<f64> = 1000.0..=40000.0;

/// LM-16: a light colour (linear RGB, brightest channel 1) for a black body
/// at `kelvin`. Public approximation of the CIE black-body colours in sRGB
/// (Tanner Helland's curve fit to Mitchell Charity's table), converted to
/// linear and scaled so the light's brightness stays with its intensity.
/// 6600 K is white; lower is warmer, higher bluer. Provisional (LPROP-Q01).
pub fn kelvin_to_rgb(kelvin: f64) -> [f64; 3] {
    let t = kelvin.clamp(*KELVIN_RANGE.start(), *KELVIN_RANGE.end()) / 100.0;
    let red = if t <= 66.0 {
        255.0
    } else {
        329.698_727_446 * (t - 60.0).powf(-0.133_204_759_2)
    };
    let green = if t <= 66.0 {
        99.470_802_586_1 * t.ln() - 161.119_568_166_1
    } else {
        288.122_169_528_3 * (t - 60.0).powf(-0.075_514_849_2)
    };
    let blue = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7
    };
    let linear = |srgb: f64| {
        let c = (srgb / 255.0).clamp(0.0, 1.0);
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let rgb = [linear(red), linear(green), linear(blue)];
    let max = rgb.iter().cloned().fold(0.0, f64::max).max(1e-12);
    rgb.map(|c| c / max)
}

impl CameraSettings {
    /// Apply one panel edit; the result must be valid or nothing changes.
    pub fn apply(&mut self, edit: CameraEdit) -> Result<()> {
        let mut next = self.clone();
        match edit {
            CameraEdit::Projection(p) => next.projection = p,
            CameraEdit::FocalLength(v) => next.focal_length = v,
            CameraEdit::AngleOfView(a) => {
                if !(a.is_finite() && a > 1e-6 && a < std::f64::consts::PI - 1e-6) {
                    return Err("Angle of View must be between 0° and 180°".into());
                }
                next.focal_length = next.film_width / (2.0 * (a / 2.0).tan());
            }
            CameraEdit::FilmWidth(v) => next.film_width = v,
            CameraEdit::FilmHeight(v) => next.film_height = v,
            CameraEdit::FilmOffsetX(v) => next.film_offset[0] = v,
            CameraEdit::FilmOffsetY(v) => next.film_offset[1] = v,
            CameraEdit::FilmFit(f) => next.film_fit = f,
            CameraEdit::TargetDistance(v) => next.target_distance = v,
            CameraEdit::FocusDistance(v) => next.focus_distance = v,
            CameraEdit::Clipping(on) => next.clipping = on,
            CameraEdit::ClipNear(v) => next.clip_near = v,
            CameraEdit::ClipFar(v) => next.clip_far = v,
            CameraEdit::LensDistortion(v) => next.lens_distortion = v,
            CameraEdit::LensSqueeze(v) => next.stored.lens_squeeze = v,
            CameraEdit::FilmRoll(v) => next.stored.film_roll = v,
            CameraEdit::DisplaySize(v) => next.stored.display_size = v,
            CameraEdit::FilmBack { width, height } => {
                next.film_width = width;
                next.film_height = height;
            }
            CameraEdit::DofEnable(on) => next.dof.enable = on,
            CameraEdit::FStop(v) => {
                if !(v.is_finite() && v >= crate::camera_props::F_STOP_MIN) {
                    return Err("F-Stop must be 1.0 or more".into());
                }
                next.dof.f_stop = v
            }
            CameraEdit::IrisBlades(n) => {
                if !crate::camera_props::valid_iris_blades(n) {
                    return Err("Iris Blades must be 0 (round) or 3 to 16".into());
                }
                next.dof.iris_blades = n
            }
            CameraEdit::IrisRotation(v) => next.dof.iris_rotation = v,
            CameraEdit::EdgeWeighting(v) => next.dof.edge_weighting = v,
            CameraEdit::ResolutionEnable(on) => next.resolution.enable = on,
            CameraEdit::ResolutionWidth(v) => next.resolution.width = v,
            CameraEdit::ResolutionHeight(v) => next.resolution.height = v,
            CameraEdit::ResolutionFromClip { width, height } => {
                next.resolution = crate::camera_props::ResolutionOverride {
                    enable: true,
                    width,
                    height,
                }
            }
            CameraEdit::BackgroundEnable(on) => next.background.enable = on,
            CameraEdit::BackgroundImage(image) => next.background.image = image,
            CameraEdit::Overscan(v) => next.stored.overscan = v,
            CameraEdit::ShowOverlay(on) => {
                next.overlay = on.then(|| self.overlay.clone().unwrap_or_default())
            }
            CameraEdit::Overlay(e) => {
                let o = next
                    .overlay
                    .as_mut()
                    .ok_or("Turn on Display > Show Camera Overlay first")?;
                match e {
                    OverlayEdit::Opacity(v) => o.opacity = v,
                    OverlayEdit::ActionSafe(on) => o.action_safe = on,
                    OverlayEdit::ActionMargin(axis, v) => o.action_margin[axis.min(1)] = v,
                    OverlayEdit::TitleSafe(on) => o.title_safe = on,
                    OverlayEdit::TitleMargin(axis, v) => o.title_margin[axis.min(1)] = v,
                    OverlayEdit::SafeColor(c) => o.safe_color = c,
                    OverlayEdit::Grid(g) => o.grid = g,
                    OverlayEdit::GridColor(c) => o.grid_color = c,
                    OverlayEdit::RightDiagonal(on) => o.right_diagonal = on,
                    OverlayEdit::LeftDiagonal(on) => o.left_diagonal = on,
                    OverlayEdit::ChevronUp(on) => o.chevron_up = on,
                    OverlayEdit::ChevronDown(on) => o.chevron_down = on,
                    OverlayEdit::Horizon(on) => o.horizon = on,
                    OverlayEdit::LineColor(c) => o.line_color = c,
                }
            }
            CameraEdit::FocusFromTarget(on) => next.focus_from_target = on,
        }
        next.validate().map_err(|_| {
            "Enter a value in range (above 0; Far Clip beyond Near Clip)".to_string()
        })?;

        *self = next;
        Ok(())
    }
}

impl LightSettings {
    pub fn apply(&mut self, edit: LightEdit) -> Result<()> {
        let mut next = self.clone();
        match edit {
            LightEdit::Kind(k) => next = self.with_kind(k),
            LightEdit::Render(r) => next.render = r,
            LightEdit::Dissolve(v) => next.dissolve = v,
            LightEdit::Intensity(v) => next.intensity = v,
            LightEdit::Shadow(s) => next.shadow = s,
            LightEdit::VisibleToCamera(on) => next.visible_to_camera = on,
            LightEdit::SpreadAngle(v) => next.spread_angle = v,
            LightEdit::Color(c) => next.color = c,
            LightEdit::AffectDiffuse(v) => next.affect_diffuse = v,
            LightEdit::AffectSpecular(v) => next.affect_specular = v,
            LightEdit::Radius(v) => next.radius = v,
            LightEdit::Falloff(f) => next.falloff = f,
            LightEdit::ConeAngle(v) => next.cone_angle = v,
            LightEdit::SoftEdge(v) => next.soft_edge = v,
            LightEdit::Shape(s) => next.shape = s,
            LightEdit::Width(v) => next.width = v,
            LightEdit::Height(v) => next.height = v,
            LightEdit::VisibleToReflection(on) => next.stored.visible_to_reflection = on,
            LightEdit::VisibleToRefraction(on) => next.stored.visible_to_refraction = on,
            LightEdit::Temperature(k) => {
                if !(k.is_finite() && KELVIN_RANGE.contains(&k)) {
                    return Err("Enter a temperature from 1000 K to 40000 K".into());
                }
                next.color = kelvin_to_rgb(k);
            }
            LightEdit::EnvironmentOnly(on) => next.environment_only = on,
            LightEdit::Length(v) => next.length = v,
            LightEdit::Prototype(p) => next.prototype = p,
            LightEdit::UsePrototypeTransform(on) => next.use_prototype_transform = on,
            LightEdit::Photometry {
                path,
                intensity,
                width,
                height,
            } => {
                next.photometry_file = path;
                next.intensity = intensity;
                next.width = width;
                next.height = height;
            }
            LightEdit::Sun(e) => next.sun.apply(&e)?,
            LightEdit::Volumetric(e) => {
                if !next.kind.has_volumetrics() {
                    return Err("This light has no Volumetrics".into());
                }
                let v = next
                    .volumetrics
                    .get_or_insert_with(|| Volumetrics::new(next.kind));
                match e {
                    VolumetricEdit::Enabled(on) => v.enabled = on,
                    VolumetricEdit::Samples(n) => v.samples = n,
                    VolumetricEdit::Height(x) => v.height = x,
                    VolumetricEdit::Radius(x) => v.radius = x,
                    VolumetricEdit::SphereRadius(x) => v.sphere_radius = x,
                    VolumetricEdit::Color(c) => v.color = c,
                    VolumetricEdit::Density(x) => v.density = x,
                    VolumetricEdit::Attenuation(x) => v.attenuation = x,
                    VolumetricEdit::Shift(x) => v.shift = x,
                }
                if *v == Volumetrics::new(next.kind) {
                    next.volumetrics = None;
                }
            }
            LightEdit::LpeLabel(text) => next.lpe_label = text.trim().to_string(),
            LightEdit::Importance(v) => next.stored.importance = v,
            LightEdit::MaterialEnabled(on) => next.stored.material_enabled = on,
            LightEdit::BlendMode(b) => next.blend_mode = b,
            LightEdit::Opacity(v) => next.stored.opacity = v,
            LightEdit::AffectSubsurface(v) => next.stored.affect_subsurface = v,
            LightEdit::AffectCaustics(v) => next.stored.affect_caustics = v,
            LightEdit::LinkControl(on) => {
                next.linking.get_or_insert_with(Default::default).control = on
            }
            LightEdit::LinkGroup(g) => next.linking.get_or_insert_with(Default::default).group = g,
            LightEdit::LinkMode(m) => next.linking.get_or_insert_with(Default::default).mode = m,
            LightEdit::Layer(e) => {
                let missing = || "No such layer".to_string();
                match e {
                    LayerEdit::Add(image) => next.layers.push(LightLayer {
                        image,
                        ..Default::default()
                    }),
                    LayerEdit::Remove(i) => {
                        if i >= next.layers.len() {
                            return Err(missing());
                        }
                        next.layers.remove(i);
                    }
                    LayerEdit::Image(i, path) => {
                        next.layers.get_mut(i).ok_or_else(missing)?.image = path
                    }
                    LayerEdit::Enabled(i, on) => {
                        next.layers.get_mut(i).ok_or_else(missing)?.enabled = on
                    }
                    LayerEdit::Blend(i, b) => next.layers.get_mut(i).ok_or_else(missing)?.blend = b,
                    LayerEdit::Opacity(i, v) => {
                        next.layers.get_mut(i).ok_or_else(missing)?.opacity = v
                    }
                }
            }
        }
        next.validate()
            .map_err(|_| "Value out of range".to_string())?;
        *self = next;
        Ok(())
    }
}

/// Where a camera or light is and which way it faces, in world space, with
/// scale taken out: right (+X), up (+Y) and forward (-Z, the way it looks).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub position: DVec3,
    pub right: DVec3,
    pub up: DVec3,
    pub forward: DVec3,
}

impl Frame {
    pub fn from_matrix(m: DMat4) -> Option<Frame> {
        let right = m.transform_vector3(DVec3::X).try_normalize()?;
        let up_raw = m.transform_vector3(DVec3::Y);
        let forward = (-m.transform_vector3(DVec3::Z)).try_normalize()?;
        // Re-orthogonalise so a sheared parent can't skew the view.
        let up = forward.cross(right).try_normalize().unwrap_or(up_raw);
        let up = if up.dot(up_raw) < 0.0 { -up } else { up };
        // forward × up, not up × forward: with forward = −Z the latter is −X
        // and mirrored every Cycles render left to right.
        let right = forward.cross(up).try_normalize()?;
        Some(Frame {
            position: m.transform_point3(DVec3::ZERO),
            right,
            up,
            forward,
        })
    }
}

impl Item {
    /// A mesh item, as opposed to a Backdrop, camera, light, texture
    /// locator or Group Locator (which hold no geometry).
    pub fn is_mesh(&self) -> bool {
        self.backdrop.is_none()
            && self.camera.is_none()
            && self.light.is_none()
            && self.texture.is_none()
            && self.locator.is_none()
            && self.environment.is_none()
            && self.kit.is_none()
            && self.replicator.is_none()
    }
}

/// The world frame of item `id`.
pub fn item_frame(scene: &Scene, id: u64) -> Option<Frame> {
    Frame::from_matrix(scene.world_matrix(id).ok()?)
}

/// The rotation (XYZ Euler, radians) that makes an item at `eye` look at
/// `target` with world +Y up, looking down its local -Z.
pub fn look_at_rotation(eye: DVec3, target: DVec3) -> DVec3 {
    let forward = (target - eye).normalize_or(-DVec3::Z);
    // Pitch about X, then yaw about Y, as a quaternion, then the XYZ
    // angles giving it.
    let yaw = (-forward.x).atan2(-forward.z);
    let pitch = forward.y.clamp(-1.0, 1.0).asin();
    let q = glam::DQuat::from_rotation_y(yaw) * glam::DQuat::from_rotation_x(pitch);
    crate::scene::RotationOrder::Xyz.angles(q)
}

/// The item transform that puts item `id` at world `position` with world
/// orientation `rotation` (scale taken out), keeping its own scale and
/// rotation order. Used when Camera view navigation moves the camera.
pub fn local_transform_for(
    scene: &Scene,
    id: u64,
    position: DVec3,
    rotation: glam::DQuat,
) -> Result<Transform> {
    let item = scene.item(id).ok_or("Missing item")?;
    let parent = match item.parent {
        Some(p) => scene.world_matrix(p)?,
        None => DMat4::IDENTITY,
    };
    if parent.determinant().abs() < 1e-12 {
        return Err("The camera's parent has no invertible transform".into());
    }
    let local = parent.inverse() * DMat4::from_rotation_translation(rotation, position);
    let (_, r, t) = local.to_scale_rotation_translation();
    let r = r.normalize();
    Ok(Transform {
        translation: t,
        rotation: item.transform.angles_for(r),
        ..item.transform.clone()
    })
}

/// The new scene's camera (observed, spec v1): at (0, 0.75, 4), tilted
/// down 5°.
pub fn new_scene_camera_transform() -> Transform {
    Transform {
        translation: DVec3::from_array(defaults::NEW_SCENE_CAMERA_POSITION),
        rotation: DVec3::from_array(
            defaults::NEW_SCENE_CAMERA_ROTATION_DEGREES.map(f64::to_radians),
        ),
        zxy: defaults::NEW_SCENE_CAMERA_ZXY,
        order: None,
        ..Default::default()
    }
}

/// The new scene's camera settings: the defaults, focused at 4 m.
pub fn new_scene_camera_settings() -> CameraSettings {
    CameraSettings {
        focus_distance: defaults::NEW_SCENE_FOCUS_DISTANCE,
        ..Default::default()
    }
}

/// The new scene's directional light (CL-Q01).
pub fn new_scene_light_transform() -> Transform {
    Transform {
        translation: DVec3::from_array(defaults::NEW_SCENE_LIGHT_POSITION),
        rotation: DVec3::from_array(
            defaults::NEW_SCENE_LIGHT_ROTATION_DEGREES.map(f64::to_radians),
        ),
        zxy: defaults::NEW_SCENE_LIGHT_ZXY,
        order: None,
        ..Default::default()
    }
}

impl Scene {
    /// `base`, or `base` numbered in the user's Item Index Style.
    pub(crate) fn free_name(&self, base: &str) -> String {
        crate::user_defaults::free_name_among(base, self.items.iter().map(|i| i.name.as_str()))
    }

    fn push_locator(
        &mut self,
        name: &str,
        transform: Transform,
        camera: Option<CameraSettings>,
        light: Option<LightSettings>,
    ) -> Result<u64> {
        if self.next_id >= u64::MAX - 1 || self.items.len() >= crate::MAX_SCENE_ITEMS {
            return Err("Scene item/identity budget exhausted".into());
        }
        let name = self.free_name(name);
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(Item {
            id,
            name,
            visible: true,
            transform,
            mesh: crate::Mesh::default().into(),
            parent: None,
            source_id: None,
            source_display_name: None,
            subdivision: None,
            backdrop: None,
            camera,
            light,
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
        Ok(id)
    }

    /// Add a camera named "Camera" ("Camera (2)" and so on). The selection
    /// is left alone.
    pub fn add_camera(&mut self, settings: CameraSettings, transform: Transform) -> Result<u64> {
        self.push_locator(defaults::CAMERA_NAME, transform, Some(settings), None)
    }

    /// Add a light named after its kind. The selection is left alone.
    pub fn add_light(&mut self, settings: LightSettings, transform: Transform) -> Result<u64> {
        self.push_locator(settings.kind.item_name(), transform, None, Some(settings))
    }

    /// What every new scene gets besides its first mesh: one camera and
    /// one directional light at their default places (CL-Q01). The
    /// selection is left alone.
    pub fn add_default_camera_and_light(&mut self) -> Result<()> {
        self.add_camera(new_scene_camera_settings(), new_scene_camera_transform())?;
        self.add_light(LightSettings::default(), new_scene_light_transform())?;
        // ENV §1: and one Environment with its Environment Material.
        self.add_environment()?;
        Ok(())
    }

    /// For a scene made from an imported file: add the default camera if
    /// it has no camera, and the default light if it has no light, so
    /// Camera view and renders work as in a new scene. A file's own
    /// cameras and lights are kept. The selection is left alone.
    pub fn ensure_camera_and_light(&mut self) -> Result<()> {
        if !self.items.iter().any(|i| i.camera.is_some()) {
            self.add_camera(new_scene_camera_settings(), new_scene_camera_transform())?;
        }
        if !self.items.iter().any(|i| i.light.is_some()) {
            self.add_light(LightSettings::default(), new_scene_light_transform())?;
        }
        self.ensure_environment()?;
        Ok(())
    }

    /// The render camera: the one chosen with `set_render_camera`, else
    /// the first camera in the item list. A scene has exactly one while it
    /// has any camera.
    pub fn render_camera(&self) -> Option<u64> {
        self.render_camera
            .filter(|id| self.items.iter().any(|i| i.id == *id && i.camera.is_some()))
            .or_else(|| self.items.iter().find(|i| i.camera.is_some()).map(|i| i.id))
    }

    pub fn set_render_camera(&mut self, id: u64) -> Result<()> {
        if !self.items.iter().any(|i| i.id == id && i.camera.is_some()) {
            return Err("Choose a camera".into());
        }
        self.render_camera = Some(id);
        Ok(())
    }

    pub(crate) fn validate_cameras_lights(&self) -> Result<()> {
        for item in &self.items {
            let kinds = [
                item.backdrop.is_some(),
                item.camera.is_some(),
                item.light.is_some(),
                item.locator.is_some(),
            ];
            if kinds.iter().filter(|k| **k).count() > 1 {
                return Err("An item can only be one kind".into());
            }
            if (item.camera.is_some() || item.light.is_some() || item.locator.is_some())
                && !item.mesh.vertices.is_empty()
            {
                return Err("Cameras, lights and locators hold no geometry".into());
            }
            if let Some(c) = &item.camera {
                c.validate()?;
            }
            if let Some(l) = &item.light {
                l.validate()?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_at_points_minus_z_at_the_target() {
        for eye in [
            DVec3::new(0.0, 1.0, 4.0),
            DVec3::new(3.0, -2.0, 1.0),
            DVec3::new(-1.0, 5.0, -4.0),
        ] {
            let t = Transform {
                translation: eye,
                rotation: look_at_rotation(eye, DVec3::ZERO),
                ..Default::default()
            };
            let f = Frame::from_matrix(t.matrix()).unwrap();
            assert!((f.forward - (-eye).normalize()).length() < 1e-9, "{eye}");
            // No roll: right stays horizontal.
            assert!(f.right.y.abs() < 1e-9);
            assert!(f.up.y > 0.0);
        }
    }

    #[test]
    fn local_transform_round_trips_under_a_parent() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let parent = scene.items[0].id;
        scene.items[0].transform.translation = DVec3::new(1.0, 2.0, 3.0);
        scene.items[0].transform.rotation = DVec3::new(0.0, 0.7, 0.0);
        let cam = scene
            .add_camera(Default::default(), Default::default())
            .unwrap();
        scene.set_parent(cam, Some(parent), false).unwrap();
        for zxy in [false, true] {
            scene.item_mut(cam).unwrap().transform.zxy = zxy;
            let q = glam::DQuat::from_euler(glam::EulerRot::XYZ, -0.3, 1.1, 0.2);
            let pos = DVec3::new(-2.0, 1.5, 4.0);
            let t = local_transform_for(&scene, cam, pos, q).unwrap();
            scene.item_mut(cam).unwrap().transform = t;
            let f = item_frame(&scene, cam).unwrap();
            assert!((f.position - pos).length() < 1e-9);
            assert!((f.forward - q * -DVec3::Z).length() < 1e-9);
            assert!((f.up - q * DVec3::Y).length() < 1e-9);
        }
    }

    /// Regression (mirrored renders, 2026-11-05): an item's frame keeps
    /// its own axes and handedness: right is +X, up +Y, forward −Z.
    #[test]
    fn frame_keeps_the_item_axes() {
        let f = Frame::from_matrix(DMat4::IDENTITY).unwrap();
        assert_eq!((f.right, f.up, f.forward), (DVec3::X, DVec3::Y, -DVec3::Z));
        let t = new_scene_camera_transform();
        let m = t.matrix();
        let f = Frame::from_matrix(m).unwrap();
        assert!((f.right - m.transform_vector3(DVec3::X)).length() < 1e-12);
        assert!((f.up - m.transform_vector3(DVec3::Y)).length() < 1e-12);
        // Right-handed: right × up points backwards (+Z, against forward).
        assert!((f.right.cross(f.up) + f.forward).length() < 1e-12);
    }

    #[test]
    fn film_fit_matches_the_chosen_side() {
        let mut c = CameraSettings::default();
        // Horizontal: the width tangent is film/(2f) whatever the aspect.
        let (w, h) = c.half_tangents(2.0);
        assert!((w - 0.36).abs() < 1e-12 && (h - 0.18).abs() < 1e-12);
        c.film_fit = FilmFit::Vertical;
        let (w, h) = c.half_tangents(2.0);
        assert!((h - 0.24).abs() < 1e-12 && (w - 0.48).abs() < 1e-12);
        // Fill on a picture wider than 3:2 matches widths; Overscan heights.
        c.film_fit = FilmFit::Fill;
        assert!((c.half_tangents(2.0).0 - 0.36).abs() < 1e-12);
        c.film_fit = FilmFit::Overscan;
        assert!((c.half_tangents(2.0).1 - 0.24).abs() < 1e-12);
    }

    #[test]
    fn angle_of_view_edit_sets_focal_length() {
        let mut c = CameraSettings::default();
        let aov = c.angle_of_view();
        c.apply(CameraEdit::FocalLength(0.035)).unwrap();
        assert!(c.angle_of_view() > aov);
        c.apply(CameraEdit::AngleOfView(aov)).unwrap();
        assert!((c.focal_length - defaults::FOCAL_LENGTH).abs() < 1e-12);
        assert!(c.apply(CameraEdit::FocalLength(0.0)).is_err());
        assert!((c.focal_length - defaults::FOCAL_LENGTH).abs() < 1e-12);
    }

    #[test]
    fn light_render_rule() {
        let mut l = LightSettings::default();
        assert!(l.renders(true) && !l.renders(false));
        l.render = LightRender::On;
        assert!(l.renders(false));
        l.render = LightRender::Off;
        assert!(!l.renders(true));
        assert!(l.apply(LightEdit::Dissolve(1.5)).is_err());
    }
}
