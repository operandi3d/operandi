//! Every default for camera and light items in one place (CL-1, spec v0).
//!
//! Values marked *observed* come from spec v1 (observed defaults,
//! 2026-11-04); the rest are still guesses. Each guess names its question in docs/SPECIFICATION_QUESTIONS.md ("Cameras and
//! lights (CL-1)"). Change a value here and everything follows: new scenes,
//! new items, the panel's Reset and the tests that compare against these.
//!
//! Units are Operandi's own: metres, radians, and fractions for percentages.

/// Name of a new camera item (duplicates get the usual " (2)" suffix).
pub const CAMERA_NAME: &str = "Camera";
/// Names of new light items, by kind.
pub const DIRECTIONAL_LIGHT_NAME: &str = "Directional Light";
pub const POINT_LIGHT_NAME: &str = "Point Light";
pub const SPOT_LIGHT_NAME: &str = "Spot Light";
pub const AREA_LIGHT_NAME: &str = "Area Light";
/// LP-PO: a portal is named "Portal" (its kind name, like the Properties
/// tab, LP-01).
pub const PORTAL_NAME: &str = "Portal";
pub const CYLINDER_LIGHT_NAME: &str = "Cylinder Light";
pub const MESH_LIGHT_NAME: &str = "Mesh Light";
pub const PHOTOMETRIC_LIGHT_NAME: &str = "Photometric Light";
pub const DOME_LIGHT_NAME: &str = "Dome Light";

// --- New scene contents (CL-Q01) -------------------------------------------

/// The new scene's camera position (metres) and rotation (degrees, X Y Z).
/// Observed (spec v1).
pub const NEW_SCENE_CAMERA_POSITION: [f64; 3] = [0.0, 0.75, 4.0];
pub const NEW_SCENE_CAMERA_ROTATION_DEGREES: [f64; 3] = [-5.0, 0.0, 0.0];
/// The new scene's directional light position (metres), rotation (degrees,
/// X Y Z) and rotation order ZXY. Observed (spec v1).
pub const NEW_SCENE_LIGHT_POSITION: [f64; 3] = [-2.0, 2.0, 2.0];
pub const NEW_SCENE_LIGHT_ROTATION_DEGREES: [f64; 3] = [-30.0, -45.0, 0.0];
pub const NEW_SCENE_LIGHT_ZXY: bool = true;

// --- Camera channels (CL-Q02, CL-Q07) ---------------------------------------

/// Focal Length, metres (50 mm). Observed (spec v1).
pub const FOCAL_LENGTH: f64 = 0.050;
/// Film Width and Height, metres (36 × 24 mm, from the spec).
pub const FILM_WIDTH: f64 = 0.036;
pub const FILM_HEIGHT: f64 = 0.024;
/// Film Offset X/Y, metres.
pub const FILM_OFFSET: [f64; 2] = [0.0, 0.0];
/// Target Distance, metres: the distance the camera orbits about.
pub const TARGET_DISTANCE: f64 = 4.0;
/// Focus Distance, metres: 1 m observed (spec v4). Only the selected
/// camera's frustum and handle use it so far (depth of field is deferred).
pub const FOCUS_DISTANCE: f64 = 1.0;
/// The new scene's camera focuses at 4 m (CP-E02, observed), like its
/// Target Distance; a camera made later keeps 1 m.
pub const NEW_SCENE_FOCUS_DISTANCE: f64 = 4.0;
/// The new scene's camera uses rotation order ZXY (CP-V05, observed).
pub const NEW_SCENE_CAMERA_ZXY: bool = true;
/// Film Fit default. Observed (spec v1): Fill.
pub const FILM_FIT: crate::camera_light::FilmFit = crate::camera_light::FilmFit::Fill;
/// Clipping is off by default (spec).
pub const CLIPPING: bool = false;
/// Clipping Distance, metres: 4 m observed (spec v1). Operandi reads it as
/// the near clip plane when Clipping is on (guess, CL-Q02); the far plane
/// is a guess too.
pub const CLIP_NEAR: f64 = 4.0;
pub const CLIP_FAR: f64 = 1.0e5;
/// Observed (spec v1), stored and shown but not used yet: Lens Squeeze,
/// Film Roll (radians), Display Size, Overscan (fraction).
pub const LENS_SQUEEZE: f64 = 1.0;
pub const FILM_ROLL: f64 = 0.0;
pub const CAMERA_DISPLAY_SIZE: f64 = 0.0;
pub const OVERSCAN: f64 = 1.0;
/// Render resolution in pixels. Observed (spec v1): 1920 × 1080. The
/// Cycles render from the scene camera uses it as the picture size.
pub const RENDER_FRAME: [u32; 2] = [1920, 1080];

// --- Light channels (CL-Q02, CL-Q04) ----------------------------------------

/// Intensity of every light kind: 3.0 in its own unit (W/m² directional,
/// W/sr point and spot, W/sr/m² area). Observed (spec v1).
pub const LIGHT_INTENSITY: f64 = 3.0;
pub const POINT_LIGHT_INTENSITY: f64 = 3.0;
pub const SPOT_LIGHT_INTENSITY: f64 = 3.0;
pub const AREA_LIGHT_INTENSITY: f64 = 3.0;
/// Point/Spot Radius, metres. Guess (CL-Q02): a true point.
pub const LIGHT_RADIUS: f64 = 0.0;
/// Falloff default for Point, Spot and Area lights. Observed (spec v1).
pub const LIGHT_FALLOFF: crate::camera_light::LightFalloff =
    crate::camera_light::LightFalloff::InverseDistanceSquared;
/// Spot Cone Angle, degrees: 45 observed (spec v1). Read as the FULL
/// cone angle (still a guess, CL-Q05).
pub const SPOT_CONE_ANGLE_DEGREES: f64 = 45.0;
/// Spot Soft Edge, degrees: 5 observed (spec v1). Read as a band inside the
/// cone's edge (guess, CL-Q05): a 45° cone with a 5° edge is fully lit
/// within a 35° inner cone and fades over the outer 5° on each side.
pub const SPOT_SOFT_EDGE_DEGREES: f64 = 5.0;
/// Area light Shape, Width and Height (metres, before item scale). Observed
/// (spec v1): 1 × 1 m Rectangular.
pub const AREA_SHAPE: crate::camera_light::AreaShape = crate::camera_light::AreaShape::Rectangular;
pub const AREA_WIDTH: f64 = 1.0;
pub const AREA_HEIGHT: f64 = 1.0;
/// Portal: Shape and Width × Height as the area light (LP-PO-02/03,
/// observed); Environment Only off (LP-PO-04, observed).
pub const PORTAL_ENVIRONMENT_ONLY: bool = false;
/// Cylinder light Radius and Length, metres (LP-CY-01, observed).
pub const CYLINDER_RADIUS: f64 = 0.2;
pub const CYLINDER_LENGTH: f64 = 1.0;
/// Dissolve, fraction (0 %).
pub const LIGHT_DISSOLVE: f64 = 0.0;
/// Spread Angle, radians (0°).
pub const LIGHT_SPREAD_ANGLE: f64 = 0.0;
/// Visible to Camera default.
pub const LIGHT_VISIBLE_TO_CAMERA: bool = false;
/// Observed (spec v1), stored and shown but not used yet: Visible to
/// Reflection / Refraction, Samples, Importance (fraction), Display Size,
/// directional Shadow Map Size (metres), light material Shadow Color,
/// Affect Caustics / Subsurface (fractions), material Enabled and Opacity.
pub const LIGHT_VISIBLE_TO_REFLECTION: bool = false;
pub const LIGHT_VISIBLE_TO_REFRACTION: bool = false;
pub const LIGHT_SAMPLES: u32 = 64;
pub const LIGHT_IMPORTANCE: f64 = 1.0;
pub const LIGHT_DISPLAY_SIZE: f64 = 1.0;
pub const SHADOW_MAP_SIZE: f64 = 2.0;
pub const LIGHT_SHADOW_COLOR: [f64; 3] = [0.0, 0.0, 0.0];
pub const LIGHT_AFFECT_CAUSTICS: f64 = 1.0;
pub const LIGHT_AFFECT_SUBSURFACE: f64 = 1.0;
pub const LIGHT_MATERIAL_ENABLED: bool = true;
pub const LIGHT_MATERIAL_OPACITY: f64 = 1.0;
/// Light material: colour (linear RGB) and Affect Diffuse / Specular
/// (fractions). Kept on the light until a shader tree exists (CL-1 defers
/// the shader-tree light material).
pub const LIGHT_COLOR: [f64; 3] = [1.0, 1.0, 1.0];
pub const LIGHT_AFFECT_DIFFUSE: f64 = 1.0;
pub const LIGHT_AFFECT_SPECULAR: f64 = 1.0;

// --- Cycles mapping (CL-Q08, CL-Q14) ----------------------------------------

/// Cycles sun strength per unit of Intensity. Guess (CL-Q14): 1:1, both in
/// W/m².
pub const CYCLES_SUN_STRENGTH_PER_INTENSITY: f64 = 1.0;

// --- Viewport drawing (CL-Q09, CL-Q16) --------------------------------------

/// Viewport glyphs (CL-13, spec v3 shapes; sizes and line counts are
/// guesses). The camera glyph keeps a constant size on screen (its Display
/// Size is 0): about this many points across.
pub const CAMERA_GLYPH_POINTS: f64 = 48.0;
/// A Group Locator's viewport glyph (GL-10, provisional): a cube shown by
/// its corner brackets, a constant size on screen about this many points
/// across (times Item Drawing size).
pub const GROUP_LOCATOR_GLYPH_POINTS: f64 = 24.0;
/// Directional light: a cylinder along local Z this wide (metres at Display
/// Size 1) and this long, then an arrow of this length along −Z.
pub const DIRECTIONAL_GLYPH_DIAMETER: f64 = 1.0;
pub const DIRECTIONAL_GLYPH_LENGTH: f64 = 1.0;
pub const DIRECTIONAL_ARROW_LENGTH: f64 = 1.0;
/// Point light: a star this wide (metres at Display Size 1).
pub const POINT_GLYPH_SIZE: f64 = 1.0;
/// Spot light: lamp housing length (metres at Display Size 1) and, when
/// selected, the dotted cone's length.
pub const SPOT_HOUSING_LENGTH: f64 = 0.5;
pub const SPOT_CONE_LENGTH: f64 = 2.0;
/// Area light: the arrow along −Z, as a share of the smaller side.
pub const AREA_ARROW_SHARE: f64 = 0.5;
/// Light/camera handles (CL-14, spec v4): spot handle circles sit this many
/// points along the axis on screen (the distance follows the view; guess).
pub const SPOT_HANDLE_POINTS: f64 = 90.0;
/// Grab crosses and area squares: half size in points, and how close a
/// press must be to take one.
pub const GRAB_HALF_POINTS: f32 = 5.0;
pub const GRAB_HIT_POINTS: f32 = 9.0;
/// Filled arrowheads: length as a share of the arrow.
pub const ARROWHEAD_SHARE: f64 = 0.25;
/// Each view's Show Cameras / Show Lights start off: cameras and lights
/// are not drawn in modeling views, selected or not, until a view turns
/// them on (Jason's decision, 2026-11-05, CL-Q16).
pub const SHOW_CAMERAS: bool = false;
pub const SHOW_LIGHTS: bool = false;
