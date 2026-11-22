//! Backdrop items: a flat reference picture shown for one view direction,
//! drawn behind meshes in the viewports and never rendered
//! (specifications/backdrop-2026-11-04, rules BD-01..BD-52).
//!
//! A Backdrop is an ordinary scene item (name, transform, parent,
//! visibility) whose `backdrop` settings are set and whose mesh stays
//! empty. The picture it shows is a scene image entry (`SceneImage`), kept
//! by file path; several Backdrops may share one entry (BD-03). Pixel data
//! is the app's business: the core only needs the image's pixel size.
use crate::{Result, Scene};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};

/// The view a Backdrop faces (Projection Type).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Projection {
    #[default]
    Front,
    Back,
    Top,
    Bottom,
    Left,
    Right,
    /// Shown only when looking through a camera, filling its render frame
    /// (BD-12; the app draws it, `drawn_in` covers the other views).
    Camera,
}

impl Projection {
    pub const ALL: [Projection; 7] = [
        Projection::Front,
        Projection::Back,
        Projection::Top,
        Projection::Bottom,
        Projection::Left,
        Projection::Right,
        Projection::Camera,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Projection::Front => "Front",
            Projection::Back => "Back",
            Projection::Top => "Top",
            Projection::Bottom => "Bottom",
            Projection::Left => "Left",
            Projection::Right => "Right",
            Projection::Camera => "Camera",
        }
    }

    /// The picture's plane in item space (BD-05): the direction of image
    /// right, image up, and the axis index (0 X, 1 Y, 2 Z) of each, plus the
    /// viewing axis. Paired projections share one placement (BD-06).
    /// `None` for Camera, which has no plane in the scene.
    pub fn plane(self) -> Option<Plane> {
        Some(match self {
            Projection::Front | Projection::Back => Plane {
                right: DVec3::X,
                up: DVec3::Y,
                axes: [0, 1],
                normal_axis: 2,
            },
            Projection::Top | Projection::Bottom => Plane {
                right: DVec3::X,
                up: -DVec3::Z,
                axes: [0, 2],
                normal_axis: 1,
            },
            Projection::Left | Projection::Right => Plane {
                right: DVec3::Z,
                up: DVec3::Y,
                axes: [2, 1],
                normal_axis: 0,
            },
            Projection::Camera => return None,
        })
    }

    /// The side of a box the matching view sees first (BD-31): +1 for the
    /// largest coordinate along the viewing axis (Front, Top, Right), -1 for
    /// the smallest (Back, Bottom, Left; BD-O-64..66).
    fn near_side(self) -> f64 {
        match self {
            Projection::Front | Projection::Top | Projection::Right => 1.,
            _ => -1.,
        }
    }
}

/// See `Projection::plane`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub right: DVec3,
    pub up: DVec3,
    /// Scale components that stretch the picture horizontally and vertically.
    pub axes: [usize; 2],
    pub normal_axis: usize,
}

/// A view a Backdrop may be drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewKind {
    /// A fixed orthographic view looking along one axis.
    Axis(Projection),
    /// The perspective view (and, provisionally, the free orthographic
    /// view: BD-Q18).
    Free,
}

/// Defaults of a new Backdrop (BD-02, BD-O-01).
pub const DEFAULT_PIXEL_SIZE: f64 = 0.01;

/// A Backdrop's own settings (BD-02). Percentages are stored as fractions:
/// Contrast and Brightness -1..1, Transparency 0..1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backdrop {
    /// A scene image id (`Scene::images`), or none: draws nothing.
    #[serde(default)]
    pub image: Option<u64>,
    pub projection: Projection,
    pub show_in_perspective: bool,
    pub keep_aspect: bool,
    pub flip: bool,
    pub invert: bool,
    pub blend: bool,
    pub contrast: f64,
    pub brightness: f64,
    pub transparency: f64,
    pub uniform_size: bool,
    /// Metres per image pixel; only used with `uniform_size` off.
    pub pixel_size: f64,
    /// BD-25: values for settings with no panel control (Gamma, Blur,
    /// Render Item in the reference) are kept if a file brings them.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub extra: std::collections::BTreeMap<String, f64>,
}

impl Default for Backdrop {
    fn default() -> Self {
        Self {
            image: None,
            projection: Projection::Front,
            show_in_perspective: true,
            keep_aspect: true,
            flip: false,
            invert: false,
            blend: false,
            contrast: 0.,
            brightness: 0.,
            transparency: 0.,
            uniform_size: false,
            pixel_size: DEFAULT_PIXEL_SIZE,
            extra: Default::default(),
        }
    }
}

/// A loaded still image, referred to by its file path (BD-03, BD-50).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneImage {
    pub id: u64,
    pub name: String,
    pub path: String,
    /// Pixel size, read when the image was loaded and kept so sizes stay
    /// right while the file is missing (BD-Q13).
    pub width: u32,
    pub height: u32,
}

/// Panel edits, one per control, so each is its own undo step.
#[derive(Clone, Debug, PartialEq)]
pub enum BackdropEdit {
    Image(Option<u64>),
    Projection(Projection),
    ShowInPerspective(bool),
    KeepAspect(bool),
    Flip(bool),
    Invert(bool),
    Blend(bool),
    Contrast(f64),
    Brightness(f64),
    Transparency(f64),
    UniformSize(bool),
    PixelSize(f64),
}

pub const PIXEL_SIZE_REFUSAL: &str = "Pixel Size only applies when Use Uniform Size is off";
/// BD-61: no active mesh with geometry.
pub const AUTO_FIT_NO_MESH: &str = "Auto Fit needs an active mesh with geometry";
pub const AUTO_FIT_REFUSAL: &str = "Auto Fit only applies with Use Uniform Size on";

impl Backdrop {
    /// A new Backdrop's settings: the factory ones, sized as the user's
    /// Display > OpenGL Backdrop preferences say.
    pub fn new_default() -> Self {
        let d = crate::user_defaults::current();
        Self {
            uniform_size: d.backdrop_uniform_size,
            pixel_size: d.backdrop_pixel_size,
            ..Self::default()
        }
    }
    /// Apply one panel edit (BD-18: Pixel Size is refused while Use Uniform
    /// Size is on). Values outside their range are clamped.
    pub fn apply(&mut self, edit: BackdropEdit) -> Result<()> {
        let percent = |v: f64, lo: f64| {
            if v.is_finite() {
                Ok(v.clamp(lo, 1.))
            } else {
                Err::<f64, String>("Enter a number".into())
            }
        };
        match edit {
            BackdropEdit::Image(image) => self.image = image,
            BackdropEdit::Projection(p) => self.projection = p,
            BackdropEdit::ShowInPerspective(on) => self.show_in_perspective = on,
            BackdropEdit::KeepAspect(on) => self.keep_aspect = on,
            BackdropEdit::Flip(on) => self.flip = on,
            BackdropEdit::Invert(on) => self.invert = on,
            BackdropEdit::Blend(on) => self.blend = on,
            BackdropEdit::Contrast(v) => self.contrast = percent(v, -1.)?,
            BackdropEdit::Brightness(v) => self.brightness = percent(v, -1.)?,
            BackdropEdit::Transparency(v) => self.transparency = percent(v, 0.)?,
            BackdropEdit::UniformSize(on) => self.uniform_size = on,
            BackdropEdit::PixelSize(v) => {
                if self.uniform_size {
                    return Err(PIXEL_SIZE_REFUSAL.into());
                }
                if !(v.is_finite() && v > 0. && v <= 1e3) {
                    return Err("Pixel Size must be above 0".into());
                }
                self.pixel_size = v;
            }
        }
        Ok(())
    }

    /// Base height before scale (BD-15).
    fn base_height(&self, height_px: u32) -> f64 {
        if self.uniform_size {
            1.
        } else {
            f64::from(height_px) * self.pixel_size
        }
    }

    /// The picture's width and height in metres for an image of
    /// `width_px` × `height_px` and the item's own scale (BD-15..17).
    /// `None` for Camera.
    pub fn size(&self, width_px: u32, height_px: u32, scale: DVec3) -> Option<(f64, f64)> {
        let plane = self.projection.plane()?;
        let (sa, sb) = (scale[plane.axes[0]].abs(), scale[plane.axes[1]].abs());
        let h0 = self.base_height(height_px.max(1));
        Some(if self.keep_aspect {
            let w0 = h0 * f64::from(width_px) / f64::from(height_px.max(1));
            let s = sa.min(sb);
            (w0 * s, h0 * s)
        } else {
            (h0 * sa, h0 * sb)
        })
    }

    /// Whether the picture is drawn in `view` (BD-07, BD-11, BD-12). The
    /// item's visibility and the view's Show Backdrop switch are checked by
    /// the caller.
    pub fn drawn_in(&self, view: ViewKind) -> bool {
        match view {
            ViewKind::Axis(p) => self.projection == p && p != Projection::Camera,
            ViewKind::Free => self.show_in_perspective && self.projection != Projection::Camera,
        }
    }
}

/// One pixel channel after Invert, Contrast and Brightness (BD-20), 0..1.
pub fn adjust_channel(v: f64, invert: bool, contrast: f64, brightness: f64) -> f64 {
    let v = if invert { 1. - v } else { v };
    (v * (1. + contrast) + brightness).clamp(0., 1.)
}

/// The world-space corners of a Backdrop's picture: top-left, top-right,
/// bottom-right, bottom-left as the image is stored (before Flip, which
/// only mirrors the picture inside them, BD-21). `None` for an item that is
/// not a Backdrop, a Backdrop with no image, or a Camera Backdrop.
pub fn world_corners(scene: &Scene, id: u64) -> Option<[DVec3; 4]> {
    let item = scene.item(id)?;
    let image = scene.image(item.backdrop.as_ref()?.image?)?;
    corners_for(scene, id, image.width, image.height)
}

/// A Backdrop picture's drawn rectangle as a UV frame (BD-62): U runs 0..1
/// from the picture's left edge to its right, V 0..1 from bottom to top,
/// as the picture shows after Flip (BD-21 mirrors it left to right).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PictureFrame {
    /// The bottom-left corner as stored (before Flip).
    pub origin: DVec3,
    /// Bottom-left to bottom-right.
    pub right: DVec3,
    /// Bottom-left to top-left.
    pub up: DVec3,
    pub flip: bool,
}

impl PictureFrame {
    /// The UV of the picture point `p` sits over, looking along the
    /// Backdrop's view axis. Points outside the rectangle get UVs below 0
    /// or above 1; nothing is clamped (BD-62).
    pub fn uv(&self, p: DVec3) -> glam::DVec2 {
        let d = p - self.origin;
        let along = |axis: DVec3| {
            let l = axis.length_squared();
            if l > 0. { d.dot(axis) / l } else { 0. }
        };
        let u = along(self.right);
        glam::DVec2::new(if self.flip { 1. - u } else { u }, along(self.up))
    }
}

/// The UV frame of Backdrop `id`'s picture (see `world_corners`).
pub fn picture_frame(scene: &Scene, id: u64) -> Option<PictureFrame> {
    let item = scene.item(id)?;
    let [tl, _, br, bl] = world_corners(scene, id)?;
    Some(PictureFrame {
        origin: bl,
        right: br - bl,
        up: tl - bl,
        flip: item.backdrop.as_ref()?.flip,
    })
}

/// The Backdrop whose picture is on top in an axis view (BD-62, BD-11b):
/// of the shown Backdrops with a picture drawn in `view`, the first
/// created. `None` for the free views (provisional, BD-Q22: only an axis
/// view's own projection counts) or when none is drawn there.
pub fn topmost_backdrop_in(scene: &Scene, view: ViewKind) -> Option<u64> {
    if view == ViewKind::Free {
        return None;
    }
    scene
        .items
        .iter()
        .filter(|i| scene.shows(i))
        .filter(|i| i.backdrop.as_ref().is_some_and(|b| b.drawn_in(view)))
        .filter(|i| world_corners(scene, i.id).is_some())
        .map(|i| i.id)
        .min()
}

/// Pixel size of the placeholder square a Backdrop shows until it has a
/// picture: 1 m at the default 10 mm Pixel Size.
pub const PLACEHOLDER_PIXELS: u32 = 100;

/// Where a Backdrop sits even without a picture: its picture's rectangle,
/// or with no image a square of `PLACEHOLDER_PIXELS` under the same size
/// rules, so a new Backdrop can be seen and moved (Jason, 2026-11-04).
/// `None` for a Camera Backdrop or an item that isn't a Backdrop.
pub fn outline_corners(scene: &Scene, id: u64) -> Option<[DVec3; 4]> {
    let item = scene.item(id)?;
    match item.backdrop.as_ref()?.image.and_then(|i| scene.image(i)) {
        Some(image) => corners_for(scene, id, image.width, image.height),
        None => corners_for(scene, id, PLACEHOLDER_PIXELS, PLACEHOLDER_PIXELS),
    }
}

/// A square `side` metres across, centred on a Backdrop with no image, in
/// its projection plane and placed by its world matrix (so the item's own
/// scale stretches it like any object). Pixel Size doesn't apply: there are
/// no pixels. `None` for a Backdrop with an image, a Camera Backdrop or an
/// item that isn't a Backdrop. Used by the original-compatible empty look.
pub fn empty_square_corners(scene: &Scene, id: u64, side: f64) -> Option<[DVec3; 4]> {
    let item = scene.item(id)?;
    let backdrop = item.backdrop.as_ref()?;
    if backdrop.image.is_some() {
        return None;
    }
    let plane = backdrop.projection.plane()?;
    let world = scene.world_matrix(id).ok()?;
    let (r, u) = (plane.right * (side / 2.), plane.up * (side / 2.));
    Some([-r + u, r + u, r - u, -r - u].map(|p| world.transform_point3(p)))
}

fn corners_for(scene: &Scene, id: u64, width: u32, height: u32) -> Option<[DVec3; 4]> {
    let item = scene.item(id)?;
    let backdrop = item.backdrop.as_ref()?;
    let plane = backdrop.projection.plane()?;
    let (w, h) = backdrop.size(width, height, item.transform.scale)?;
    // The item's own scale is in the size already; place the rectangle with
    // the rest of the world matrix.
    let world = scene.world_matrix(id).ok()?;
    let unscale = DMat4::from_scale(
        item.transform
            .scale
            .map(|s| if s == 0. { 1. } else { 1. / s }),
    );
    let frame = world * unscale;
    let (r, u) = (plane.right * (w / 2.), plane.up * (h / 2.));
    Some([-r + u, r + u, r - u, -r - u].map(|p| frame.transform_point3(p)))
}

impl Scene {
    pub fn image(&self, id: u64) -> Option<&SceneImage> {
        self.images.iter().find(|i| i.id == id)
    }

    /// The image entry for `path`, adding one named after the file (without its extension, BD-Q20) if the
    /// scene has none yet (BD-03).
    pub fn image_for_path(&mut self, path: &str, width: u32, height: u32) -> Result<u64> {
        if width == 0 || height == 0 {
            return Err("The image has no pixels".into());
        }
        if let Some(existing) = self.images.iter().find(|i| i.path == path) {
            return Ok(existing.id);
        }
        let id = self.images.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        let base = std::path::Path::new(path)
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_owned());
        // BD-03: a different file with a name already taken gets " (2)",
        // " (3)" and so on.
        let taken = |name: &str| self.images.iter().any(|i| i.name == name);
        let name = if taken(&base) {
            (2..)
                .map(|n| format!("{base} ({n})"))
                .find(|n| !taken(n))
                .unwrap_or(base)
        } else {
            base
        };
        self.images.push(SceneImage {
            id,
            name,
            path: path.to_owned(),
            width,
            height,
        });
        Ok(id)
    }

    /// Add a Backdrop at the origin with no parent, alone selected, named
    /// "Backdrop Item", then "Backdrop Item (2)" and so on (BD-01).
    pub fn new_backdrop(&mut self, settings: Backdrop) -> Result<u64> {
        if self.next_id >= u64::MAX - 1 || self.items.len() >= crate::MAX_SCENE_ITEMS {
            return Err("Scene item/identity budget exhausted".into());
        }
        let name = self.free_name("Backdrop Item");
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(crate::scene::Item {
            id,
            name,
            visible: true,
            transform: Default::default(),
            mesh: crate::Mesh::default().into(),
            parent: None,
            source_id: None,
            source_display_name: None,
            subdivision: None,
            backdrop: Some(settings),
            camera: None,
            light: None,
            texture: None,
            locator: None,
            environment: None,
            common: Default::default(),
            elements: Default::default(),
            hides_children: false,
            mesh_ops: None,
            kit: None,
            instance_of: None,
            replicator: None,
        });
        self.selection = crate::scene::Selection {
            item: Some(id),
            mode: self.selection.mode,
            ..Default::default()
        };
        Ok(id)
    }

    /// The meshes Auto Fit fits to (BD-61): the active mesh items, the mesh
    /// layers being edited. That is every selected mesh, hidden ones
    /// included (provisional, BD-Q04), or with no mesh selected (a Backdrop
    /// selected on its own) the mesh that stays active (VP-04).
    pub fn auto_fit_meshes(&self) -> Vec<u64> {
        let selected = self.selected_meshes();
        if selected.is_empty() {
            self.remembered_mesh().into_iter().collect()
        } else {
            selected
        }
    }

    /// The combined world bounding box of `ids`' geometry: the box Auto
    /// Fit uses (BD-61). `None` when none of them has a point.
    pub fn mesh_bounds(&self, ids: &[u64]) -> Option<(DVec3, DVec3)> {
        let mut bounds: Option<(DVec3, DVec3)> = None;
        for item in self
            .items
            .iter()
            .filter(|i| ids.contains(&i.id) && i.backdrop.is_none())
        {
            let Ok(world) = self.world_matrix(item.id) else {
                continue;
            };
            for &p in &item.mesh.vertices {
                let p = world.transform_point3(p);
                bounds = Some(match bounds {
                    Some((lo, hi)) => (lo.min(p), hi.max(p)),
                    None => (p, p),
                });
            }
        }
        bounds
    }

    /// Auto Fit (BD-30..33): move the Backdrop to the box centre in its
    /// plane and onto the box face nearest its view, scale it so the
    /// picture is as tall as the box (and, with Keep Aspect off, as wide),
    /// and reset the out-of-plane scale to 1.
    pub fn auto_fit_backdrop(&mut self, id: u64, bounds: (DVec3, DVec3)) -> Result<()> {
        let item = self.item(id).ok_or("Select a Backdrop")?;
        let backdrop = item.backdrop.as_ref().ok_or("Select a Backdrop")?;
        if !backdrop.uniform_size {
            return Err(AUTO_FIT_REFUSAL.into());
        }
        let plane = backdrop
            .projection
            .plane()
            .ok_or("Auto Fit needs a view projection, not Camera")?;
        let (lo, hi) = bounds;
        let extent = hi - lo;
        let mut target = (lo + hi) / 2.;
        let n = plane.normal_axis;
        target[n] = if backdrop.projection.near_side() > 0. {
            hi[n]
        } else {
            lo[n]
        };
        let height = extent[plane.axes[1]];
        let width = if backdrop.keep_aspect {
            height
        } else {
            extent[plane.axes[0]]
        };
        if !(height > 1e-8 && width > 1e-8) {
            return Err("Auto Fit needs a mesh with height in the Backdrop's plane".into());
        }
        let parent = item.parent;
        let local = match parent {
            Some(p) => self.world_matrix(p)?.inverse().transform_point3(target),
            None => target,
        };
        let item = self.item_mut(id).ok_or("Select a Backdrop")?;
        item.transform.translation = local;
        item.transform.scale[plane.axes[0]] = width;
        item.transform.scale[plane.axes[1]] = height;
        // BD-33: the out-of-plane scale goes back to 1.
        item.transform.scale[plane.normal_axis] = 1.;
        Ok(())
    }

    /// Backdrop Size (BD-19, provisional BD-Q08): make the picture exactly
    /// `width` × `height` metres by turning Keep Aspect off and setting the
    /// in-plane scale. With `height` `None` (Backdrop Size Keep Aspect),
    /// turn Keep Aspect on and scale both in-plane axes for that width.
    pub fn size_backdrop(&mut self, id: u64, width: f64, height: Option<f64>) -> Result<()> {
        let valid = |v: f64| v.is_finite() && v > 1e-6 && v < 1e8;
        if !valid(width) || height.is_some_and(|h| !valid(h)) {
            return Err("Sizes must be above 0".into());
        }
        let (w_px, h_px) = {
            let item = self.item(id).ok_or("Select a Backdrop")?;
            let backdrop = item.backdrop.as_ref().ok_or("Select a Backdrop")?;
            let image = backdrop
                .image
                .and_then(|i| self.image(i))
                .ok_or("Choose the Backdrop's image first")?;
            (image.width, image.height)
        };
        let item = self.item_mut(id).ok_or("Select a Backdrop")?;
        let backdrop = item.backdrop.as_mut().ok_or("Select a Backdrop")?;
        let plane = backdrop
            .projection
            .plane()
            .ok_or("Backdrop Size needs a view projection, not Camera")?;
        let h0 = backdrop.base_height(h_px);
        match height {
            Some(height) => {
                backdrop.keep_aspect = false;
                item.transform.scale[plane.axes[0]] = width / h0;
                item.transform.scale[plane.axes[1]] = height / h0;
            }
            None => {
                backdrop.keep_aspect = true;
                let w0 = h0 * f64::from(w_px) / f64::from(h_px);
                item.transform.scale[plane.axes[0]] = width / w0;
                item.transform.scale[plane.axes[1]] = width / w0;
            }
        }
        Ok(())
    }

    pub(crate) fn validate_backdrops(&self) -> Result<()> {
        let mut ids = std::collections::BTreeSet::new();
        for image in &self.images {
            if image.id == 0
                || !ids.insert(image.id)
                || image.width == 0
                || image.height == 0
                || image.path.len() > 65_536
                || image.name.len() > 65_536
            {
                return Err("Invalid scene image".into());
            }
        }
        for item in &self.items {
            let Some(b) = &item.backdrop else { continue };
            if !item.mesh.vertices.is_empty() {
                return Err("Backdrop items hold no geometry; select a mesh".into());
            }
            if b.image.is_some_and(|i| !ids.contains(&i)) {
                return Err("Backdrop refers to a missing image".into());
            }
            let finite = [b.contrast, b.brightness, b.transparency, b.pixel_size]
                .iter()
                .all(|v| v.is_finite());
            if !finite
                || !(-1. ..=1.).contains(&b.contrast)
                || !(-1. ..=1.).contains(&b.brightness)
                || !(0. ..=1.).contains(&b.transparency)
                || b.pixel_size <= 0.
            {
                return Err("Invalid Backdrop settings".into());
            }
        }
        Ok(())
    }
}
