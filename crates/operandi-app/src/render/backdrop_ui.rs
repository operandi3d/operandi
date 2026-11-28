//! Backdrop items in the app (specifications/backdrop-2026-11-04): reading
//! picture files, the Backdrop's Properties controls, which Backdrops each
//! view draws, and the selection outline. Settings, placement and undo live
//! in the core (operandi_core::backdrop); drawing in backdrop_gpu.rs.
//!
//! Provisional choices the spec leaves open, kept easy to change:
//! - BD-Q01: a click in a viewport never picks a Backdrop; select it in the
//!   Items list. Clicks pass through to meshes.
//! - BD-Q08: Backdrop Size takes a width and height from two fields on the
//!   panel; Backdrop Size Keep Aspect takes the width.
//! - BD-Q12: every control change, image load and Auto Fit is one undo step.
//! - BD-Q21: a dropped image's Backdrop faces the view it was dropped into
//!   (Perspective, free Orthographic and Camera views give Front), and the
//!   drop is one undo step. Only the main window's views count as 3D views.
//! - BD-Q06: a Camera Backdrop fills the render frame of the camera being
//!   looked through (BD-12); Flip and the colour options apply as in the
//!   other views. With no picture it shows nothing (no placeholder).
//! - BD-Q13: a missing picture file draws nothing (the selected outline still
//!   shows where it would be); the panel and status bar say so once.
use super::*;
use crate::backdrop_gpu::Pixels;
use crate::camera::View;
use crate::unavailable::Gate;
use operandi_core::backdrop::{Backdrop, BackdropEdit, PIXEL_SIZE_REFUSAL, Projection, ViewKind};
use operandi_core::command_line::attr_edit;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

/// Pictures larger than this on a side are scaled down for display (the
/// graphics card's usual texture limit). Sizes still use the file's pixels.
const MAX_SIDE: u32 = 8192;

enum Entry {
    Ready(Arc<Pixels>),
    Problem(String),
}

/// Decoded pictures by file path, and the panel's in-progress values.
#[derive(Default)]
pub(crate) struct Images {
    entries: HashMap<String, Entry>,
    next_key: u64,
    /// A number field being dragged: (item, field, value). Committed as one
    /// undo step when the drag or edit ends.
    draft: Option<(u64, &'static str, f64)>,
    /// Backdrop Size fields: (item, width, height) in metres.
    size_fields: Option<(u64, f64, f64)>,
    /// Clip thumbnails for the Image field and its chooser, by picture key.
    thumbs: HashMap<u64, egui::TextureHandle>,
    /// The Clips chooser (BC-05) and the Backdrop it is choosing for.
    pub(crate) chooser: crate::clip_picker::Chooser,
    pub(crate) chooser_item: Option<u64>,
}

/// Longest side of a clip thumbnail, in pixels.
const THUMB_SIDE: u32 = 128;

impl Images {
    /// The thumbnail of the picture at `path`, made once per picture.
    pub(crate) fn thumb(
        &mut self,
        ctx: &egui::Context,
        path: &str,
    ) -> Option<crate::clip_picker::Thumb> {
        let mut problem = None;
        let pixels = self.get(path, &mut problem)?;
        let texture = self.thumbs.entry(pixels.key).or_insert_with(|| {
            let full = image::ImageBuffer::<image::Rgba<u8>, &[u8]>::from_raw(
                pixels.width,
                pixels.height,
                &pixels.rgba[..],
            );
            let (w, h) = (pixels.width.max(1), pixels.height.max(1));
            let scale = (THUMB_SIDE as f32 / w.max(h) as f32).min(1.);
            let (tw, th) = (
                ((w as f32 * scale).round() as u32).max(1),
                ((h as f32 * scale).round() as u32).max(1),
            );
            let small = match full {
                Some(full) => image::imageops::thumbnail(&full, tw, th),
                None => image::RgbaImage::new(tw, th),
            };
            let colour = egui::ColorImage::from_rgba_unmultiplied(
                [tw as usize, th as usize],
                small.as_raw(),
            );
            ctx.load_texture(
                format!("clip-thumb-{}", pixels.key),
                colour,
                egui::TextureOptions::LINEAR,
            )
        });
        Some(crate::clip_picker::Thumb {
            texture: texture.id(),
            size: texture.size_vec2(),
        })
    }
}

impl Images {
    /// The picture at `path`, reading it the first time it's asked for.
    /// A file that can't be read is reported once through `problem`.
    pub(crate) fn get(&mut self, path: &str, problem: &mut Option<String>) -> Option<Arc<Pixels>> {
        if !self.entries.contains_key(path) {
            let entry = match decode(std::path::Path::new(path)) {
                Ok((pixels, _)) => Entry::Ready(self.keyed(pixels)),
                Err(error) => {
                    *problem = Some(error.clone());
                    Entry::Problem(error)
                }
            };
            self.entries.insert(path.to_owned(), entry);
        }
        match &self.entries[path] {
            Entry::Ready(p) => Some(p.clone()),
            Entry::Problem(_) => None,
        }
    }
    fn keyed(&mut self, mut pixels: Pixels) -> Arc<Pixels> {
        self.next_key += 1;
        pixels.key = self.next_key;
        Arc::new(pixels)
    }
    /// Keep a freshly decoded picture for `path` (camera Background Image).
    pub(crate) fn insert_ready(&mut self, path: &str, pixels: Pixels) {
        let pixels = self.keyed(pixels);
        self.entries.insert(path.to_owned(), Entry::Ready(pixels));
    }
    pub(crate) fn problem(&self, path: &str) -> Option<&str> {
        match self.entries.get(path) {
            Some(Entry::Problem(e)) => Some(e),
            _ => None,
        }
    }
    /// Texture keys of every loaded picture.
    pub fn keys(&self) -> BTreeSet<u64> {
        self.entries
            .values()
            .filter_map(|e| match e {
                Entry::Ready(p) => Some(p.key),
                Entry::Problem(_) => None,
            })
            .collect()
    }
}

/// Read a PNG or JPEG file: the picture for display (scaled down past
/// `MAX_SIDE`) and the file's own pixel size.
pub(crate) fn decode(path: &std::path::Path) -> std::result::Result<(Pixels, (u32, u32)), String> {
    let name = path.display();
    if !path.exists() {
        return Err(format!("Backdrop picture not found: {name}"));
    }
    let image = crate::image_decode::reader(path)
        .map_err(|e| format!("Can't read {name}: {e}"))?
        .decode()
        .map_err(|e| format!("Can't read {name} as a PNG or JPEG picture: {e}"))?;
    let size = (image.width(), image.height());
    if size.0 == 0 || size.1 == 0 {
        return Err(format!("{name} has no pixels"));
    }
    let mut rgba = image.to_rgba8();
    if size.0.max(size.1) > MAX_SIDE {
        let k = f64::from(MAX_SIDE) / f64::from(size.0.max(size.1));
        let (w, h) = (
            ((f64::from(size.0) * k).round() as u32).max(1),
            ((f64::from(size.1) * k).round() as u32).max(1),
        );
        rgba = image::imageops::resize(&rgba, w, h, image::imageops::FilterType::Triangle);
    }
    Ok((
        Pixels {
            key: 0,
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        },
        size,
    ))
}

/// Which Backdrops a view of this kind draws (BD-07, BD-11).
pub(crate) fn view_kind(view: View) -> ViewKind {
    match view {
        View::Perspective | View::Orthographic => ViewKind::Free,
        View::Front => ViewKind::Axis(Projection::Front),
        View::Back => ViewKind::Axis(Projection::Back),
        View::Top => ViewKind::Axis(Projection::Top),
        View::Bottom => ViewKind::Axis(Projection::Bottom),
        View::Left => ViewKind::Axis(Projection::Left),
        View::Right => ViewKind::Axis(Projection::Right),
    }
}

/// The Backdrop projection matching a view (BD-60): the axis views give
/// their own; Perspective, the free Orthographic view and a Camera view
/// give the default, Front.
pub(crate) fn view_projection(camera: &crate::camera::Camera) -> Projection {
    match view_kind(camera.view) {
        ViewKind::Axis(p) if !camera.through => p,
        _ => Projection::Front,
    }
}

/// Still-picture files a drop treats as images (the formats `decode` reads).
pub(crate) fn is_still_image(path: &std::path::Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "tif" | "tiff" | "exr" | "hdr" | "bmp" | "tga"
        )
    })
}

/// A Backdrop shown without its picture: no image yet, or a picture file
/// that can't be read.
pub(super) struct Placeholder {
    pub corners: [DVec3; 4],
    pub no_image: bool,
}

/// Original-compatible A4: an empty Backdrop shows a dotted square.
const EMPTY_DOTS: &str = "A4";

/// Where Backdrop `id` shows in the viewport, for drawing, the selection
/// outline and view fitting: its picture's rectangle, or with no image
/// today's placeholder square; with A4 on, an empty Backdrop is a 1 m
/// square centred on the item whatever its Pixel Size.
pub(super) fn shown_corners(scene: &Scene, id: u64) -> Option<[DVec3; 4]> {
    if crate::original_compatible::is_original(EMPTY_DOTS)
        && let Some(square) = operandi_core::backdrop::empty_square_corners(scene, id, 1.)
    {
        return Some(square);
    }
    operandi_core::backdrop::outline_corners(scene, id)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaceholderLook {
    /// Operandi's orange outline round a see-through grid.
    Outline,
    /// A4: a square of fine light dots.
    Dots,
}

/// Original-compatible A5: in Perspective, backdrop pictures cover meshes.
const PERSPECTIVE_OVER: &str = "A5";

/// Whether this view draws its pictures over meshes: the Perspective view
/// (not an axis view, not through a camera) with A5 on.
fn pictures_over_meshes(kind: ViewKind, through_camera: bool, original: bool) -> bool {
    original && kind == ViewKind::Free && !through_camera
}

/// How a placeholder draws: dots only for a Backdrop with no image while
/// the original-compatible switch is on (an unreadable picture keeps the
/// outline).
fn placeholder_look(no_image: bool, original: bool) -> PlaceholderLook {
    if no_image && original {
        PlaceholderLook::Dots
    } else {
        PlaceholderLook::Outline
    }
}

/// The dots of A4's square: a 24 by 24 grid over `corners` (tl, tr, br,
/// bl), inset half a step from the edges.
fn placeholder_dots([tl, tr, _, bl]: [DVec3; 4]) -> Vec<DVec3> {
    const N: usize = 24;
    let (across, down) = (tr - tl, bl - tl);
    (0..N * N)
        .map(|k| {
            let (i, j) = (k % N, k / N);
            let (u, v) = ((i as f64 + 0.5) / N as f64, (j as f64 + 0.5) / N as f64);
            tl + across * u + down * v
        })
        .collect()
}

impl Operandi {
    /// The Backdrops the current view draws, back to front: the first
    /// created is drawn last, on top where they overlap (BD-11b).
    /// Also returns the corners of Backdrops shown as placeholders: no
    /// image yet, or a picture file that can't be read.
    pub(super) fn backdrop_frame(
        &mut self,
        r: Rect,
    ) -> (crate::backdrop_gpu::Frame, Vec<Placeholder>) {
        let kind = view_kind(self.camera.view);
        let mut frame = crate::backdrop_gpu::Frame {
            camera: crate::gpu_scene::camera_uniform(&self.camera, r, false),
            items: Vec::new(),
            over_meshes: pictures_over_meshes(
                kind,
                self.camera.through,
                crate::original_compatible::is_original(PERSPECTIVE_OVER),
            ),
        };
        let mut placeholders = Vec::new();
        // CP-V26: a camera's Background Image goes under every Backdrop.
        if let Some(background) = self.camera_background_draw(r) {
            frame.items.push(background);
        }
        if !self.display.settings.drawing.backdrops {
            return (frame, placeholders);
        }
        // BD-12: in a camera view, Camera Backdrops fill the render frame
        // exactly, stretched to its shape; their position, scale and Pixel
        // Size don't apply. `None` outside a camera view.
        let camera_frame = self.camera_frame_corners(r, 1., 1.);
        let scene = &self.editor.scene;
        let mut problem = None;
        let mut items: Vec<_> = scene.items.iter().filter(|i| scene.shows(i)).collect();
        items.sort_by_key(|i| std::cmp::Reverse(i.id));
        for item in items {
            let camera = item
                .backdrop
                .as_ref()
                .is_some_and(|b| b.projection == Projection::Camera);
            let Some(b) = item
                .backdrop
                .as_ref()
                .filter(|b| b.drawn_in(kind) || camera && camera_frame.is_some())
            else {
                continue;
            };
            let pixels = b
                .image
                .and_then(|i| scene.image(i))
                .and_then(|image| self.backdrop_images.get(&image.path, &mut problem));
            let corners = if camera {
                camera_frame.filter(|_| b.image.is_some())
            } else {
                operandi_core::backdrop::world_corners(scene, item.id)
            };
            let (Some(pixels), Some(corners)) = (pixels, corners) else {
                placeholders.extend(shown_corners(scene, item.id).map(|corners| Placeholder {
                    corners,
                    no_image: b.image.is_none(),
                }));
                continue;
            };
            frame.items.push(crate::backdrop_gpu::Draw {
                corners,
                pixels,
                flip: b.flip,
                invert: b.invert,
                blend: b.blend,
                contrast: b.contrast as f32,
                brightness: b.brightness as f32,
                transparency: b.transparency as f32,
            });
        }
        if let Some(problem) = problem {
            self.status = problem;
        }
        (frame, placeholders)
    }

    /// A Backdrop with nothing to show yet still marks where it sits: an
    /// orange outline round a see-through grid (Jason, 2026-11-04). Drawn
    /// under the graphics-card layer, so meshes and the grid cover it.
    pub(super) fn draw_backdrop_placeholders(
        &self,
        painter: &egui::Painter,
        r: Rect,
        placeholders: &[Placeholder],
    ) {
        const DIVISIONS: usize = 10;
        let color = theme::viewport::backdrop_placeholder();
        let original = crate::original_compatible::is_original(EMPTY_DOTS);
        for placeholder in placeholders {
            let [tl, tr, br, bl] = placeholder.corners;
            let project = |p: DVec3| self.camera.project(p, r).map(|p| p.0);
            if placeholder_look(placeholder.no_image, original) == PlaceholderLook::Dots {
                // A4: the square filled with fine light dots, no outline.
                for p in placeholder_dots(placeholder.corners) {
                    if let Some(q) = project(p) {
                        painter.circle_filled(q, 1.0, Color32::from_white_alpha(150));
                    }
                }
                continue;
            }
            let line = |a: DVec3, b: DVec3, stroke: Stroke| {
                if let (Some(a), Some(b)) = (project(a), project(b)) {
                    painter.line_segment([a, b], stroke);
                }
            };
            let thin = Stroke::new(1.0f32, color.gamma_multiply(0.4));
            for k in 1..DIVISIONS {
                let t = k as f64 / DIVISIONS as f64;
                line(tl.lerp(tr, t), bl.lerp(br, t), thin);
                line(tl.lerp(bl, t), tr.lerp(br, t), thin);
            }
            let edge = Stroke::new(2.0f32, color);
            for (a, b) in [(tl, tr), (tr, br), (br, bl), (bl, tl)] {
                line(a, b, edge);
            }
        }
    }

    /// BD-40: a one-pixel outline round the selected Backdrop's picture.
    pub(super) fn draw_backdrop_outline(&self, painter: &egui::Painter, r: Rect) {
        let scene = &self.editor.scene;
        let Some(item) = scene.active().filter(|i| scene.shows(i)) else {
            return;
        };
        if !self.display.settings.drawing.backdrops
            || !item
                .backdrop
                .as_ref()
                .is_some_and(|b| b.drawn_in(view_kind(self.camera.view)))
        {
            return;
        }
        let Some(corners) = shown_corners(scene, item.id) else {
            return;
        };
        let points: Option<Vec<Pos2>> = corners
            .iter()
            .map(|&p| self.camera.project(p, r).map(|p| p.0))
            .collect();
        if let Some(mut points) = points {
            points.push(points[0]);
            painter.add(egui::Shape::line(
                points,
                Stroke::new(1.0f32, theme::viewport::selection()),
            ));
        }
    }

    /// A new Backdrop (backdrop.add).
    pub(super) fn create_backdrop(&mut self) {
        if self.run_ok(Command::NewBackdrop) {
            self.status = "New Backdrop: choose its Image in Properties".into();
        }
    }

    fn choose_backdrop_image(&mut self, item: u64) {
        let folder = self
            .editor
            .scene
            .images
            .last()
            .and_then(|i| {
                std::path::Path::new(&i.path)
                    .parent()
                    .map(|p| p.to_path_buf())
            })
            .or_else(dirs_home)
            .unwrap_or_default();
        if !self.choose(
            file_chooser::Choose::Image { folder },
            file_chooser::Purpose::BackdropImage(item),
        ) {
            self.status = "No file panel is available to choose a picture".into();
        }
    }

    /// A picture file chosen for Backdrop `item`.
    pub(super) fn backdrop_image_chosen(&mut self, item: u64, path: std::path::PathBuf) {
        match decode(&path) {
            Ok((pixels, (width, height))) => {
                let key = path.to_string_lossy().into_owned();
                let pixels = self.backdrop_images.keyed(pixels);
                self.backdrop_images
                    .entries
                    .insert(key.clone(), Entry::Ready(pixels));
                if self.run_ok(Command::BackdropLoadImage {
                    item,
                    path: key,
                    width,
                    height,
                }) {
                    self.status = format!("Backdrop picture {width} × {height} px");
                }
            }
            Err(error) => self.status = error,
        }
    }

    /// Read a picture file and keep its pixels for drawing: its path key
    /// and pixel size, or the reason it can't be read (status set).
    fn read_picture(&mut self, path: &std::path::Path) -> Option<(String, u32, u32)> {
        match decode(path) {
            Ok((pixels, (width, height))) => {
                let key = path.to_string_lossy().into_owned();
                let pixels = self.backdrop_images.keyed(pixels);
                self.backdrop_images
                    .entries
                    .insert(key.clone(), Entry::Ready(pixels));
                Some((key, width, height))
            }
            Err(error) => {
                self.status = error;
                None
            }
        }
    }

    /// BD-60 / `backdrop.add`: a new Backdrop facing `projection`, named
    /// `name` (else "Backdrop Item"), showing the picture at `path` if
    /// given; one undo step.
    pub(super) fn add_backdrop(
        &mut self,
        name: Option<String>,
        path: Option<&std::path::Path>,
        projection: Projection,
    ) {
        let image = match path {
            Some(path) => match self.read_picture(path) {
                Some(image) => Some(image),
                None => return,
            },
            None => None,
        };
        let size = image.as_ref().map(|(_, w, h)| (*w, *h));
        if self.run_ok(Command::AddBackdrop {
            name,
            image,
            projection,
        }) {
            self.status = match size {
                Some((w, h)) => {
                    format!("New {} Backdrop, picture {w} × {h} px", projection.label())
                }
                None => format!("New {} Backdrop", projection.label()),
            };
        }
    }

    /// BD-60: a dropped picture as a new Backdrop facing `projection`.
    pub(super) fn backdrop_from_file(&mut self, path: &std::path::Path, projection: Projection) {
        self.add_backdrop(None, Some(path), projection);
    }

    /// BD-60: the picture at `path` as a scene image entry only.
    pub(super) fn load_image_file(&mut self, path: &std::path::Path) {
        let Some((path, width, height)) = self.read_picture(path) else {
            return;
        };
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if self.run_ok(Command::LoadImage {
            path,
            width,
            height,
        }) {
            self.status = format!("Loaded image {name} ({width} × {height} px)");
        }
    }

    /// The projection a picture dropped at `at` (main-window points) gets
    /// (BD-60), or `None` when `at` isn't over a 3D view.
    pub(super) fn drop_projection(&self, at: Option<Pos2>) -> Option<Projection> {
        let at = at?;
        let (&view, _) = self.corner.areas.iter().find(|(_, a)| a.contains(at))?;
        let camera = if view == self.active_view {
            &self.camera
        } else {
            self.cameras.get(&view)?
        };
        Some(view_projection(camera))
    }

    /// A file dropped on the window (BD-60): a still picture over a 3D view
    /// becomes a Backdrop facing that view; elsewhere it only loads as an
    /// image entry. Anything else opens as a scene, as before.
    /// BD-60: pictures become Backdrops or image entries by where they
    /// land; scene files open as before. MN-07.2: every file of a drop, at
    /// the same drop point; with several, scene files queue up and open one
    /// by one, each as its own scene.
    pub(super) fn files_dropped(&mut self, paths: Vec<std::path::PathBuf>, at: Option<Pos2>) {
        let several = paths.len() > 1;
        for path in paths {
            let path = self.adopt_dropped(path);
            if several && !path.is_dir() && !is_still_image(&path) {
                self.document_opens.extend(vec![path]);
            } else {
                self.file_dropped(path, at);
            }
        }
    }

    pub(super) fn file_dropped(&mut self, path: std::path::PathBuf, at: Option<Pos2>) {
        // MC-01/02: a folder dropped on the Preset Browser asks whether to
        // add it as a location or copy it into the library.
        if path.is_dir() {
            if !self.browser_folder_dropped(path, at, false) {
                self.status = "Drop a folder on the Preset Browser to browse it there".into();
            }
            return;
        }
        if !is_still_image(&path) {
            self.request_open_path(path);
            return;
        }
        if self.has_modal() {
            return;
        }
        if self.hdr_dropped_as_environment(&path, at) {
            return;
        }
        // MN-07.1: on a mesh in a 3D view, the picture joins the material
        // of the polygon under the pointer as a colour texture layer.
        if let Some(at) = at
            && self.drop_projection(Some(at)).is_some()
            && let Some((item, faces)) = self.drop_geometry(at)
        {
            self.image_dropped_on_surface(&path, item, &faces);
            return;
        }
        match self.drop_projection(at) {
            Some(projection) => self.backdrop_from_file(&path, projection),
            None => self.load_image_file(&path),
        }
    }

    /// MN-07.1: picture `path` as a new Image Map layer at the top of the
    /// material group of polygons `faces` of `item` (they share one tag).
    pub(super) fn image_dropped_on_surface(
        &mut self,
        path: &std::path::Path,
        item: u64,
        faces: &[usize],
    ) {
        let Some(tag) = self
            .editor
            .scene
            .item(item)
            .and_then(|i| faces.first().and_then(|&f| i.mesh.faces.get(f)))
            .map(|f| f.material.clone())
        else {
            return;
        };
        let Some((path, width, height)) = self.read_picture(path) else {
            return;
        };
        if self.run_ok(Command::ShaderTree(
            operandi_core::ShaderTreeEdit::AddImageForTag {
                tag: tag.clone(),
                image: (path, width, height),
            },
        )) {
            self.status = format!("Image Map added to material {tag}");
        }
    }

    /// HDRI stage 4 (UIC-74 deviation, Jason OK 2026-11-12): an .hdr or
    /// .exr dropped on a 3D view's empty space (no mesh under the pointer)
    /// becomes the top environment's image (Command::SetEnvironmentImage,
    /// one undo step) instead of a Backdrop. Dropped on a mesh, or any
    /// other picture, it is a Backdrop as before. Returns whether it took
    /// the drop. To revert the deviation, make this return false.
    pub(super) fn hdr_dropped_as_environment(
        &mut self,
        path: &std::path::Path,
        at: Option<Pos2>,
    ) -> bool {
        let hdr = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "hdr" | "exr"));
        let Some(at) = at else {
            return false;
        };
        if !hdr || self.drop_projection(Some(at)).is_none() || self.drop_geometry(at).is_some() {
            return false;
        }
        self.environment_image_chosen(None, path);
        true
    }

    fn edit_backdrop(&mut self, item: u64, edit: BackdropEdit) {
        self.run(Command::EditBackdrop { item, edit });
    }

    /// A number control: value shown in `scale` units (100 for percent,
    /// 1000 for millimetres). One undo step when the drag or typing ends.
    #[allow(clippy::too_many_arguments)]
    fn backdrop_number(
        &mut self,
        ui: &mut Ui,
        item: u64,
        key: &'static str,
        label: &str,
        value: f64,
        range: std::ops::RangeInclusive<f64>,
        suffix: &str,
        scale: f64,
        why: Option<&str>,
        make: fn(f64) -> BackdropEdit,
    ) -> Response {
        let mut shown = match self.backdrop_images.draft {
            Some((i, k, v)) if i == item && k == key => v,
            _ => value * scale,
        };
        let response = ui
            .horizontal(|ui| {
                crate::option_row::label(ui, label);
                crate::option_row::fill(ui, |ui| {
                    ui.gated(
                        why,
                        crate::numeric_tab::DragValue::new(&mut shown)
                            .range(range)
                            .speed(if scale == 1000. { 0.05 } else { 0.5 })
                            .max_decimals(3)
                            .suffix(suffix),
                    )
                })
            })
            .inner;
        if response.changed() {
            self.backdrop_images.draft = Some((item, key, shown));
        }
        let finished = response.drag_stopped()
            || response.lost_focus()
            || (response.changed() && !response.dragged() && !response.has_focus());
        if finished
            && let Some((i, k, v)) = self.backdrop_images.draft
            && i == item
            && k == key
        {
            self.backdrop_images.draft = None;
            if (v / scale - value).abs() > 1e-12 {
                self.edit_backdrop(item, make(v / scale));
            }
        }
        response
    }

    /// The Backdrop's controls in the spec's order (BD-52), under the
    /// name and transform.
    pub(super) fn backdrop_properties(&mut self, ui: &mut Ui, item: &operandi_core::Item) {
        let Some(b) = item.backdrop.clone() else {
            return;
        };
        let id = item.id;
        let idle = self.editor.session.live.is_none();
        crate::form::rows::section(ui, "Backdrop", false, |ui| {
            ui.gated_ui(crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE), |ui| {
                ui.spacing_mut().item_spacing.y = 2.;
                self.backdrop_image_row(ui, id, &b);
                let mut toggle =
                    |ui: &mut Ui, label: &str, on: bool, make: fn(bool) -> BackdropEdit| {
                        let mut v = on;
                        if crate::option_row::check(ui, label, &mut v).changed() {
                            self.edit_backdrop(id, make(v));
                        }
                    };
                toggle(
                    ui,
                    "Show In Perspective",
                    b.show_in_perspective,
                    BackdropEdit::ShowInPerspective,
                );
                ui.horizontal(|ui| {
                    crate::option_row::label(ui, "Projection Type");
                    crate::option_row::fill(ui, |ui| {
                        let mut p = b.projection;
                        egui::ComboBox::from_id_salt(("backdrop-projection", id))
                            .selected_text(p.label())
                            .show_ui(ui, |ui| {
                                for option in Projection::ALL {
                                    ui.selectable_value(&mut p, option, option.label());
                                }
                            });
                        if p != b.projection {
                            self.edit_backdrop(id, BackdropEdit::Projection(p));
                        }
                    })
                });
                let mut toggle =
                    |ui: &mut Ui, label: &str, on: bool, make: fn(bool) -> BackdropEdit| {
                        let mut v = on;
                        if crate::option_row::check(ui, label, &mut v).changed() {
                            self.edit_backdrop(id, make(v));
                        }
                    };
                toggle(ui, "Keep Aspect", b.keep_aspect, BackdropEdit::KeepAspect);
                toggle(ui, "Flip", b.flip, BackdropEdit::Flip);
                toggle(ui, "Invert", b.invert, BackdropEdit::Invert);
                toggle(ui, "Blend", b.blend, BackdropEdit::Blend);
                self.backdrop_number(
                    ui,
                    id,
                    "contrast",
                    "Contrast",
                    b.contrast,
                    -100. ..=100.,
                    " %",
                    100.,
                    None,
                    BackdropEdit::Contrast,
                );
                self.backdrop_number(
                    ui,
                    id,
                    "brightness",
                    "Brightness",
                    b.brightness,
                    -100. ..=100.,
                    " %",
                    100.,
                    None,
                    BackdropEdit::Brightness,
                );
                self.backdrop_number(
                    ui,
                    id,
                    "transparency",
                    "Transparency",
                    b.transparency,
                    0. ..=100.,
                    " %",
                    100.,
                    None,
                    BackdropEdit::Transparency,
                );
                let mut uniform = b.uniform_size;
                if crate::option_row::check(ui, "Use Uniform Size", &mut uniform).changed() {
                    self.edit_backdrop(id, BackdropEdit::UniformSize(uniform));
                }
                self.backdrop_number(
                    ui,
                    id,
                    "pixel",
                    "Pixel Size",
                    b.pixel_size,
                    0.001..=1e6,
                    " mm",
                    1000.,
                    crate::unavailable::unless(!b.uniform_size, PIXEL_SIZE_REFUSAL),
                    BackdropEdit::PixelSize,
                );
                ui.add_space(4.);
                if ui
                    .gated(crate::unavailable::unless(b.uniform_size, operandi_core::backdrop::AUTO_FIT_REFUSAL), Button::new("Auto Fit"))
                    .on_hover_text(
                        "Fit to the visible meshes' box, on the side facing this Backdrop's view",
                    )
                    .clicked()
                {
                    self.run(Command::BackdropAutoFit(id));
                }
                self.backdrop_size_rows(ui, item, &b);
            });
        });
    }

    /// BC-01: the Image field is a thumbnail, the clip's name and an arrow;
    /// clicking it opens the Clips chooser (BC-05, clip_picker.rs).
    fn backdrop_image_row(&mut self, ui: &mut Ui, id: u64, b: &Backdrop) {
        let scene = &self.editor.scene;
        let current = b.image.and_then(|i| scene.image(i));
        let clips = crate::clip_picker::clips(scene);
        let text = crate::clip_picker::shown(b.image, &clips);
        let missing =
            current.and_then(|i| self.backdrop_images.problem(&i.path).map(str::to_owned));
        let path = current.map(|i| i.path.clone());
        let thumb = path
            .as_deref()
            .and_then(|p| self.backdrop_images.thumb(ui.ctx(), p));
        let mut clicked = false;
        ui.horizontal(|ui| {
            crate::option_row::label(ui, "Image");
            crate::option_row::fill(ui, |ui| {
                // Long clip names cut with "…" (UIC-111).
                let field = crate::clip_picker::field(ui, thumb, &text);
                clicked = field.clicked();
                if let Some(path) = &path {
                    field.on_hover_text(path);
                }
            })
        });
        if let Some(missing) = missing {
            ui.colored_label(theme::error(), missing);
        }
        if clicked {
            self.open_backdrop_clips(id);
        }
    }

    /// Open the Clips chooser for Backdrop `item` (BC-05).
    pub(super) fn open_backdrop_clips(&mut self, item: u64) {
        self.backdrop_images.chooser_item = Some(item);
        self.backdrop_images.chooser.open();
    }

    /// Draw the open Clips chooser (BC-05) and apply its pick. It closes by
    /// itself when its Backdrop is gone.
    pub(super) fn backdrop_clip_chooser(&mut self, ctx: &egui::Context) {
        if !self.backdrop_images.chooser.is_open() {
            return;
        }
        let Some(item) = self.backdrop_images.chooser_item else {
            self.backdrop_images.chooser.close();
            return;
        };
        let Some(current) = self
            .editor
            .scene
            .item(item)
            .and_then(|i| i.backdrop.as_ref())
            .map(|b| b.image)
        else {
            self.backdrop_images.chooser.close();
            return;
        };
        let tiles: Vec<_> = crate::clip_picker::clips(&self.editor.scene)
            .into_iter()
            .map(|clip| {
                let thumb = self.backdrop_images.thumb(ctx, &clip.path);
                (clip, thumb)
            })
            .collect();
        let mut chooser = self.backdrop_images.chooser;
        let picked = chooser.show(ctx, egui::Id::new("backdrop-clips"), current, &tiles);
        self.backdrop_images.chooser = chooser;
        if let Some(pick) = picked {
            self.pick_backdrop_clip(item, pick);
        }
    }

    /// A Clips picker choice for Backdrop `item`: an existing clip or
    /// "(none)" is one Backdrop Setting step (the clip is not read again);
    /// "Add Clip…" opens the file panel, and the chosen file becomes one
    /// new clip shown on the Backdrop in one step (BD-03, BD-Q12).
    pub(super) fn pick_backdrop_clip(&mut self, item: u64, pick: crate::clip_picker::ClipPick) {
        use crate::clip_picker::ClipPick;
        let current = self
            .editor
            .scene
            .item(item)
            .and_then(|i| i.backdrop.as_ref())
            .and_then(|b| b.image);
        match crate::clip_picker::change(current, pick) {
            None => {}
            Some(ClipPick::Clear) => self.edit_backdrop(item, BackdropEdit::Image(None)),
            Some(ClipPick::Existing(clip)) => {
                self.edit_backdrop(item, BackdropEdit::Image(Some(clip)))
            }
            Some(ClipPick::Add) => self.choose_backdrop_image(item),
        }
    }

    /// Backdrop Size and Backdrop Size Keep Aspect (BD-19, provisional
    /// BD-Q08): the fields start at the picture's current size.
    fn backdrop_size_rows(&mut self, ui: &mut Ui, item: &operandi_core::Item, b: &Backdrop) {
        let id = item.id;
        let current = b
            .image
            .and_then(|i| self.editor.scene.image(i))
            .and_then(|i| b.size(i.width, i.height, item.transform.scale));
        let Some((w, h)) = current else {
            ui.weak("Backdrop Size needs an image and a view projection");
            return;
        };
        let (mut fw, mut fh) = match self.backdrop_images.size_fields {
            Some((i, fw, fh)) if i == id => (fw, fh),
            _ => (w, h),
        };
        for (label, value) in [("Width", &mut fw), ("Height", &mut fh)] {
            ui.horizontal(|ui| {
                crate::option_row::label(ui, label);
                crate::option_row::fill(ui, |ui| {
                    ui.add(
                        crate::numeric_tab::DragValue::new(value)
                            .range(0.001..=1e6)
                            .speed(0.01)
                            .max_decimals(4)
                            .distance(),
                    );
                })
            });
        }
        self.backdrop_images.size_fields = Some((id, fw, fh));
        ui.horizontal(|ui| {
            if ui
                .button("Backdrop Size")
                .on_hover_text("Make the picture exactly Width × Height (turns Keep Aspect off)")
                .clicked()
            {
                self.backdrop_images.size_fields = None;
                self.run(Command::BackdropSize {
                    item: id,
                    width: fw,
                    height: Some(fh),
                });
            }
            if ui
                .button("Backdrop Size Keep Aspect")
                .on_hover_text("Make the picture Width wide, keeping its shape")
                .clicked()
            {
                self.backdrop_images.size_fields = None;
                self.run(Command::BackdropSize {
                    item: id,
                    width: fw,
                    height: None,
                });
            }
        });
    }
}

/// The Backdrop commands, named `<domain>.<verb>` like poly.flip and
/// edge.addLoop (BD-Q19: command names are Operandi's choice). Each acts on
/// the selected Backdrop and goes through the same operation as its control.
pub(crate) const COMMANDS: [(&str, &str); 5] = [
    (
        "backdrop.add",
        "Help: backdrop.add [name:<text>] [image:<file path>] [project:front|back|top|bottom|left|right|camera] (Add Item > Backdrop Item) adds a Backdrop at the origin, alone selected; the arguments may also be given in that order without names. With an image it shows that picture (what dropping a picture on a view does, BD-60); project defaults to Front. One undo step. Old spelling: backdrop.create.",
    ),
    (
        "backdrop.image",
        "Help: backdrop.image <file path|none> shows a PNG or JPEG picture on the selected Backdrop (the same file is one image entry), or none. Bare backdrop.image opens the file panel.",
    ),
    (
        "backdrop.edit",
        "Help: backdrop.edit project:<view> aspect:<bool> flip:<bool> invert:<bool> blend:<bool> contrast:<0–1> brightness:<0–1> transparency:<0–1> image:<path|none> (map names; percentages 0–1 or 50%), or the old backdrop.attr form backdrop.edit <name> <value>, which sets one option of the selected Backdrop: projection front|back|top|bottom|left|right|camera; showInPerspective, keepAspect, flip, invert, blend, uniformSize true|false; contrast, brightness -100..100 %, transparency 0..100 %; pixelSize <distance> (only with uniformSize false). Percentages: 50 or 50%.",
    ),
    (
        "backdrop.fit",
        "Help: backdrop.fit (old spelling backdrop.autoFit) fits the selected Backdrop to the active meshes' combined box (BD-61; hidden active meshes count) (Use Uniform Size must be on). No arguments.",
    ),
    (
        "backdrop.size",
        "Help: backdrop.size <width> [<height>] makes the selected Backdrop's picture that size: with a height it turns Keep Aspect off (Backdrop Size); with only a width it keeps the picture's shape (Backdrop Size Keep Aspect).",
    ),
];

impl Operandi {
    /// Run a Backdrop command line, or its help topic. False when `text`
    /// isn't one.
    pub(super) fn backdrop_canonical(&mut self, text: &str) -> bool {
        let Ok(args) = command::tokens(text) else {
            return false;
        };
        let Some(first) = args.first().map(|a| a.to_ascii_lowercase()) else {
            return false;
        };
        if first == "help" && args.len() == 2 {
            let topic = args[1].to_ascii_lowercase();
            if let Some((_, help)) = COMMANDS
                .iter()
                .find(|(c, _)| c.eq_ignore_ascii_case(&topic))
            {
                self.status = (*help).into();
                return true;
            }
            return false;
        }
        if !COMMANDS.iter().any(|(c, _)| c.eq_ignore_ascii_case(&first)) {
            return false;
        }
        let usage = |first: &str| {
            COMMANDS
                .iter()
                .find(|(c, _)| c.eq_ignore_ascii_case(first))
                .map_or("", |(_, h)| h)
                .trim_start_matches("Help: ")
                .to_owned()
        };
        if first == "backdrop.add" {
            if args.len() == 1 {
                self.create_backdrop();
                return true;
            }
            match add_arguments(&args[1..]) {
                Ok((name, image, projection)) => self.add_backdrop(
                    name,
                    image.as_deref().map(std::path::Path::new),
                    projection.unwrap_or_default(),
                ),
                Err(e) => self.status = format!("{e}. Usage: {}", usage(&first)),
            }
            return true;
        }
        let Some(id) = self
            .editor
            .scene
            .active()
            .filter(|i| i.backdrop.is_some())
            .map(|i| i.id)
        else {
            self.status = "Select a Backdrop first".into();
            return true;
        };
        match (first.as_str(), args.len()) {
            ("backdrop.image", 1) => self.choose_backdrop_image(id),
            ("backdrop.image", 2) if args[1].eq_ignore_ascii_case("none") => {
                self.edit_backdrop(id, BackdropEdit::Image(None))
            }
            ("backdrop.image", 2) => {
                self.backdrop_image_chosen(id, std::path::PathBuf::from(&args[1]))
            }
            ("backdrop.fit", 1) => self.run(Command::BackdropAutoFit(id)),
            ("backdrop.size", 2 | 3) => {
                let parse = |s: &str| numeric::parse_quantity(s, numeric::Quantity::Distance);
                match (parse(&args[1]), args.get(2).map(|h| parse(h)).transpose()) {
                    (Ok(width), Ok(height)) => self.run(Command::BackdropSize {
                        item: id,
                        width,
                        height,
                    }),
                    (Err(e), _) | (_, Err(e)) => self.status = e,
                }
            }
            // The old backdrop.attr form: one option and its value.
            ("backdrop.edit", 3) if !args[1].contains(':') => match attr_edit(&args[1], &args[2]) {
                Ok(edit) => self.edit_backdrop(id, edit),
                Err(e) => self.status = e,
            },
            // The map's named options, each one edit (provisional: one undo
            // step per option).
            ("backdrop.edit", n) if n > 1 => {
                for arg in &args[1..] {
                    let Some((key, value)) = arg.split_once(':') else {
                        self.status = usage(&first);
                        return true;
                    };
                    let key = key.to_ascii_lowercase();
                    if key == "image" {
                        if value.eq_ignore_ascii_case("none") {
                            self.edit_backdrop(id, BackdropEdit::Image(None));
                        } else {
                            self.backdrop_image_chosen(id, std::path::PathBuf::from(value));
                        }
                        continue;
                    }
                    let (name, value) = match key.as_str() {
                        "project" => ("projection", value.to_owned()),
                        "aspect" => ("keepAspect", value.to_owned()),
                        // Map percentages are 0–1 (or 50%).
                        "contrast" | "brightness" | "transparency" => {
                            match operandi_core::command_names::script_percent(value) {
                                Ok(p) => (key.as_str(), format!("{p}%")),
                                Err(e) => {
                                    self.status = e;
                                    return true;
                                }
                            }
                        }
                        _ => (key.as_str(), value.to_owned()),
                    };
                    match attr_edit(name, &value) {
                        Ok(edit) => self.edit_backdrop(id, edit),
                        Err(e) => {
                            self.status = e;
                            return true;
                        }
                    }
                }
            }
            _ => self.status = usage(&first),
        }
        true
    }
}

/// `backdrop.add` arguments (name, image, project), each named
/// (`name:<v>`) or in that order without a name. The reference map's other
/// arguments (aspect, flip, autoSize, invert, blend, contrast, brightness,
/// transparency, cenX, cenY …) aren't supported yet: set them afterwards
/// with `backdrop.edit`.
type AddArguments = (Option<String>, Option<String>, Option<Projection>);
fn add_arguments(args: &[String]) -> std::result::Result<AddArguments, String> {
    const NAMES: [&str; 3] = ["name", "image", "project"];
    let mut values: [Option<String>; 3] = Default::default();
    let mut next = 0;
    for arg in args {
        // Only a known argument name before ':' names it, so a path with a
        // colon in it still works in order.
        let named = arg.split_once(':').and_then(|(key, value)| {
            NAMES
                .iter()
                .position(|n| n.eq_ignore_ascii_case(key))
                .map(|i| (i, value))
        });
        let (slot, value) = match named {
            Some((slot, value)) => (slot, value.to_owned()),
            None if next < NAMES.len() => (next, arg.clone()),
            None => return Err(format!("Unexpected argument {arg}")),
        };
        if values[slot].is_some() {
            return Err(format!("{} is given twice", NAMES[slot]));
        }
        values[slot] = Some(value);
        next = next.max(slot + 1);
    }
    let [name, image, project] = values;
    let projection = project
        .as_deref()
        .map(parse_projection)
        .transpose()
        .map_err(|e| e.replace("projection", "project"))?;
    Ok((name, image, projection))
}

/// A projection by name, any case.
fn parse_projection(value: &str) -> std::result::Result<Projection, &'static str> {
    Projection::ALL
        .iter()
        .copied()
        .find(|p| p.label().eq_ignore_ascii_case(value))
        .ok_or("projection takes front, back, top, bottom, left, right or camera")
}

pub(super) fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Original-compatible A4: only an empty Backdrop with the switch on
    /// shows dots; off (or an unreadable picture) keeps today's outline.
    /// Original-compatible A5: only the Perspective view with the switch
    /// on draws pictures over meshes.
    #[test]
    fn a5_pictures_cover_meshes_only_in_perspective_when_original_compatible() {
        assert!(!pictures_over_meshes(ViewKind::Free, false, false));
        assert!(pictures_over_meshes(ViewKind::Free, false, true));
        assert!(!pictures_over_meshes(ViewKind::Free, true, true));
        assert!(!pictures_over_meshes(
            ViewKind::Axis(Projection::Top),
            false,
            true
        ));
    }

    #[test]
    fn a4_empty_backdrops_show_dots_only_when_original_compatible() {
        assert_eq!(placeholder_look(true, false), PlaceholderLook::Outline);
        assert_eq!(placeholder_look(true, true), PlaceholderLook::Dots);
        assert_eq!(placeholder_look(false, true), PlaceholderLook::Outline);
        // The dots fill the same square the outline marks, inside it.
        let corners = [
            DVec3::new(-0.5, 0.5, 0.),
            DVec3::new(0.5, 0.5, 0.),
            DVec3::new(0.5, -0.5, 0.),
            DVec3::new(-0.5, -0.5, 0.),
        ];
        let dots = placeholder_dots(corners);
        assert_eq!(dots.len(), 24 * 24);
        assert!(
            dots.iter()
                .all(|p| p.x.abs() < 0.5 && p.y.abs() < 0.5 && p.z == 0.)
        );
        let centre = dots.iter().copied().sum::<DVec3>() / dots.len() as f64;
        assert!(centre.length() < 1e-9);
    }

    fn write_png(path: &std::path::Path, w: u32, h: u32) {
        let mut img = image::RgbaImage::new(w, h);
        for (x, _, p) in img.enumerate_pixels_mut() {
            *p = if x < w / 2 {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 0, 255, 255])
            };
        }
        img.save(path).unwrap();
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("operandi-backdrop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn loading_a_picture_sets_the_image_and_draws_it_only_in_its_view() {
        let path = temp("front.png");
        write_png(&path, 512, 256);
        let mut a = Operandi::empty();
        a.create_backdrop();
        let id = a.editor.scene.selection.item.unwrap();
        let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
        // Before a picture: a placeholder in its view, nothing for the card.
        a.camera.view = View::Front;
        let (frame, placeholders) = a.backdrop_frame(r);
        assert!(frame.items.is_empty());
        assert_eq!(placeholders.len(), 1);
        a.backdrop_image_chosen(id, path.clone());
        assert!(a.backdrop_frame(r).1.is_empty());
        let image = &a.editor.scene.images[0];
        assert_eq!((image.width, image.height), (512, 256));
        assert_eq!(image.name, "front");
        a.camera.view = View::Front;
        let frame = a.backdrop_frame(r).0;
        assert_eq!(frame.items.len(), 1);
        assert_eq!(frame.items[0].pixels.width, 512);
        // Same picture again: no second decode, same texture.
        assert_eq!(
            a.backdrop_frame(r).0.items[0].pixels.key,
            frame.items[0].pixels.key
        );
        a.camera.view = View::Top;
        assert!(a.backdrop_frame(r).0.items.is_empty());
        a.camera.view = View::Perspective;
        assert_eq!(a.backdrop_frame(r).0.items.len(), 1);
        // BD-13: Show Backdrop off hides it in this view.
        a.display.settings.drawing.backdrops = false;
        assert!(a.backdrop_frame(r).0.items.is_empty());
        a.display.settings.drawing.backdrops = true;
        // BD-14: the item's eye hides it everywhere.
        a.run(Command::Visibility(id));
        assert!(a.backdrop_frame(r).0.items.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_missing_picture_draws_nothing_and_says_so() {
        let mut a = Operandi::empty();
        a.create_backdrop();
        let id = a.editor.scene.selection.item.unwrap();
        a.run(Command::BackdropLoadImage {
            item: id,
            path: "/no/such/picture.png".into(),
            width: 10,
            height: 10,
        });
        a.camera.view = View::Front;
        let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
        let (frame, placeholders) = a.backdrop_frame(r);
        assert!(frame.items.is_empty());
        assert_eq!(placeholders.len(), 1, "drawn as a placeholder instead");
        assert!(a.status.contains("not found"), "{}", a.status);
        assert!(a.backdrop_images.problem("/no/such/picture.png").is_some());
    }

    #[test]
    fn unreadable_files_are_refused_without_changing_the_scene() {
        let path = temp("not-a-picture.png");
        std::fs::write(&path, b"hello").unwrap();
        let mut a = Operandi::empty();
        a.create_backdrop();
        let id = a.editor.scene.selection.item.unwrap();
        let before = a.editor.scene.clone();
        a.backdrop_image_chosen(id, path.clone());
        assert_eq!(a.editor.scene, before);
        assert!(a.status.contains("Can't read"), "{}", a.status);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn commands_run_the_same_operations_as_the_panel() {
        let path = temp("command.png");
        write_png(&path, 100, 50);
        let mut a = Operandi::empty();
        assert!(a.backdrop_canonical("backdrop.edit flip true"));
        assert_eq!(a.status, "Select a Backdrop first");
        assert!(a.backdrop_canonical("backdrop.add"));
        assert_eq!(a.editor.scene.items[0].name, "Backdrop Item");
        let line = format!("backdrop.image {{{}}}", path.display());
        assert!(a.backdrop_canonical(&line));
        assert_eq!(a.editor.scene.images.len(), 1);
        for line in [
            "backdrop.edit projection Top",
            "backdrop.edit flip true",
            "backdrop.edit contrast 50%",
            "backdrop.edit transparency 25",
            "backdrop.edit pixelSize 4mm",
            "backdrop.edit uniformSize true",
        ] {
            assert!(a.backdrop_canonical(line), "{line}");
        }
        let b = a.editor.scene.items[0].backdrop.clone().unwrap();
        assert_eq!(b.projection, Projection::Top);
        assert!(b.flip && b.uniform_size);
        assert_eq!((b.contrast, b.transparency), (0.5, 0.25));
        assert!((b.pixel_size - 0.004).abs() < 1e-12);
        // Refused like the greyed field.
        a.backdrop_canonical("backdrop.edit pixelSize 5mm");
        assert_eq!(a.status, PIXEL_SIZE_REFUSAL);
        a.run(Command::NewMesh);
        a.editor.scene.items[1].mesh = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
            .unwrap()
            .into();
        a.run(Command::SelectItem(Some(a.editor.scene.items[0].id)));
        assert!(a.backdrop_canonical("backdrop.fit"));
        assert_eq!(a.editor.scene.items[0].transform.translation.y, 0.5);
        assert!(a.backdrop_canonical("backdrop.size 3 2"));
        assert!(
            !a.editor.scene.items[0]
                .backdrop
                .as_ref()
                .unwrap()
                .keep_aspect
        );
        assert!(a.backdrop_canonical("help backdrop.edit"));
        assert!(a.status.starts_with("Help: backdrop.edit"));
        assert!(!a.backdrop_canonical("poly.flip"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn transform_tools_take_a_backdrop_in_any_mode() {
        let mut a = Operandi::empty();
        a.create_backdrop();
        for (mode, tool) in [
            (Mode::Polygon, Tool::Move),
            (Mode::Edge, Tool::Rotate),
            (Mode::Vertex, Tool::Scale),
            (Mode::Polygon, Tool::Transform),
        ] {
            a.run(Command::Mode(mode));
            a.activate(tool);
            assert_eq!(
                a.editor.session.live.as_ref().map(|l| l.tool),
                Some(tool),
                "{}",
                a.status
            );
            assert_eq!(a.editor.scene.selection.mode, Mode::Item);
            a.run(Command::Drop);
        }
    }

    #[test]
    fn bd_o_61_the_first_created_backdrop_draws_on_top() {
        let path = temp("order.png");
        write_png(&path, 64, 32);
        let mut a = Operandi::empty();
        let mut ids = Vec::new();
        for _ in 0..3 {
            a.create_backdrop();
            let id = a.editor.scene.selection.item.unwrap();
            a.backdrop_image_chosen(id, path.clone());
            ids.push(id);
        }
        // Give each its own size so the draw list shows which is which.
        for (k, &id) in ids.iter().enumerate() {
            let item = a.editor.scene.item_mut(id).unwrap();
            item.transform.scale = DVec3::splat(k as f64 + 1.);
        }
        a.camera.view = View::Front;
        let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
        let widths: Vec<f64> = a
            .backdrop_frame(r)
            .0
            .items
            .iter()
            .map(|d| (d.corners[1] - d.corners[0]).length())
            .collect();
        // Drawn last = on top: the first created (smallest).
        assert!(widths[0] > widths[1] && widths[1] > widths[2], "{widths:?}");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn frame_all_includes_backdrops_drawn_in_the_view() {
        let path = temp("frame.png");
        write_png(&path, 400, 200);
        let mut a = Operandi::empty();
        a.create_backdrop();
        let id = a.editor.scene.selection.item.unwrap();
        a.backdrop_image_chosen(id, path.clone());
        a.editor.scene.items[0].transform.translation = DVec3::new(10., 0., 0.);
        a.camera.view = View::Front;
        a.frame(false);
        assert!(
            (a.camera.target.x - 10.).abs() < 1e-9,
            "{}",
            a.camera.target
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bd_60_a_view_gives_its_projection_free_and_camera_views_front() {
        let mut camera = crate::camera::Camera::default();
        for (view, want) in [
            (View::Front, Projection::Front),
            (View::Back, Projection::Back),
            (View::Top, Projection::Top),
            (View::Bottom, Projection::Bottom),
            (View::Left, Projection::Left),
            (View::Right, Projection::Right),
            (View::Perspective, Projection::Front),
            (View::Orthographic, Projection::Front),
        ] {
            camera.view = view;
            assert_eq!(view_projection(&camera), want, "{view:?}");
        }
        camera.view = View::Top;
        camera.through = true;
        assert_eq!(view_projection(&camera), Projection::Front, "camera view");
    }

    #[test]
    fn bd_60_dropped_pictures_go_by_where_they_land() {
        let path = temp("dropped.png");
        write_png(&path, 120, 60);
        let mut a = Operandi::empty();
        // Two views side by side: the active one (Top) and another (Right).
        let left = Rect::from_min_size(Pos2::ZERO, vec2(400., 300.));
        let right = Rect::from_min_size(pos2(400., 0.), vec2(400., 300.));
        let other = a.active_view + 1;
        a.corner.areas.insert(a.active_view, left);
        a.corner.areas.insert(other, right);
        a.camera.view = View::Top;
        let right_camera = crate::camera::Camera {
            view: View::Right,
            ..Default::default()
        };
        a.cameras.insert(other, right_camera);
        // Into the Top view: a Top Backdrop showing it, alone selected.
        a.file_dropped(path.clone(), Some(left.center()));
        let item = a.editor.scene.items.last().unwrap();
        let b = item.backdrop.as_ref().expect("a Backdrop");
        assert_eq!(b.projection, Projection::Top);
        assert_eq!(a.editor.scene.selection.item, Some(item.id));
        assert_eq!(a.editor.scene.image(b.image.unwrap()).unwrap().width, 120);
        // It draws at once: the pixels were kept for the view.
        let (frame, placeholders) = a.backdrop_frame(left);
        assert_eq!((frame.items.len(), placeholders.len()), (1, 0));
        // One undo step takes it all back.
        a.run(Command::Undo);
        assert!(a.editor.scene.items.is_empty() && a.editor.scene.images.is_empty());
        // Into the other view: that view's projection.
        a.file_dropped(path.clone(), Some(right.center()));
        let b = a.editor.scene.items[0].backdrop.as_ref().unwrap();
        assert_eq!(b.projection, Projection::Right);
        // Not over a 3D view (a panel, or no pointer): an image entry only.
        a.run(Command::Undo);
        a.file_dropped(path.clone(), Some(pos2(900., 100.)));
        assert!(a.editor.scene.items.is_empty());
        assert_eq!(a.editor.scene.images.len(), 1);
        a.run(Command::Undo);
        a.file_dropped(path.clone(), None);
        assert!(a.editor.scene.items.is_empty());
        assert_eq!(a.editor.scene.images.len(), 1);
        // A scene file still goes to Open: no Backdrop, no image entry.
        let mut a = Operandi::empty();
        a.corner.areas.insert(a.active_view, left);
        a.file_dropped(temp("scene.opscene"), Some(left.center()));
        assert!(a.editor.scene.items.is_empty() && a.editor.scene.images.is_empty());
        // The missing file's Open error is a dialog; OK it.
        assert_eq!(a.file_error.as_ref().map(|e| e.0.as_str()), Some("Open"));
        a.file_error = None;
        // A broken picture is refused and changes nothing.
        let broken = temp("broken.png");
        std::fs::write(&broken, b"no").unwrap();
        a.file_dropped(broken.clone(), Some(left.center()));
        assert!(a.editor.scene.items.is_empty() && a.editor.scene.images.is_empty());
        assert!(a.status.contains("Can't read"), "{}", a.status);
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(broken);
    }

    #[test]
    fn bd_60_backdrop_add_and_clip_add_still_commands() {
        let path = temp("typed.png");
        write_png(&path, 80, 40);
        let mut a = Operandi::empty();
        a.camera.view = View::Left;
        // In order: name, image; project defaults to Front (BD-02).
        a.execute(&format!("backdrop.add Plan {{{}}}", path.display()));
        let item = a.editor.scene.items[0].clone();
        assert_eq!(item.name, "Plan");
        let b = item.backdrop.unwrap();
        assert_eq!(b.projection, Projection::Front);
        assert!(b.image.is_some());
        // Named, any order and case.
        a.execute(&format!(
            "backdrop.add project:Bottom {{image:{}}}",
            path.display()
        ));
        let item = a.editor.scene.items[1].clone();
        assert_eq!(item.name, "Backdrop Item");
        assert_eq!(item.backdrop.unwrap().projection, Projection::Bottom);
        assert_eq!(a.editor.scene.images.len(), 1, "one entry per file");
        // One undo step each.
        a.run(Command::Undo);
        assert_eq!(a.editor.scene.items.len(), 1);
        // Refusals change nothing.
        for line in [
            format!("backdrop.add project:sideways {{image:{}}}", path.display()),
            "backdrop.add name:A name:B".to_owned(),
            "backdrop.add {image:/no/such/picture.png}".to_owned(),
        ] {
            a.execute(&line);
            assert_eq!(a.editor.scene.items.len(), 1, "{line}: {}", a.status);
        }
        // The old spelling still adds a plain Backdrop.
        a.execute("backdrop.create");
        assert_eq!(a.editor.scene.items.len(), 2);
        let mut a = Operandi::empty();
        a.execute(&format!("clip.addStill {{filename:{}}}", path.display()));
        assert!(a.editor.scene.items.is_empty());
        assert_eq!(a.editor.scene.images[0].name, "typed");
        a.run(Command::Undo);
        a.execute(&format!("clip.addstill {{{}}}", path.display()));
        assert_eq!(a.editor.scene.images.len(), 1);
        let _ = std::fs::remove_file(path);
    }

    thread_local! {
        static CLIP_ANSWER: std::cell::RefCell<Option<std::path::PathBuf>> =
            const { std::cell::RefCell::new(None) };
        static CLIP_ASKED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    /// A stand-in file panel: counts requests, answers the queued path.
    fn clip_panel(_: file_chooser::Choose) -> Option<std::path::PathBuf> {
        CLIP_ASKED.with(|n| n.set(n.get() + 1));
        CLIP_ANSWER.with(|a| a.borrow_mut().take())
    }

    fn backdrop_image(a: &Operandi, id: u64) -> Option<u64> {
        a.editor
            .scene
            .item(id)
            .and_then(|i| i.backdrop.as_ref())
            .and_then(|b| b.image)
    }

    #[test]
    fn clips_picker_reuses_existing_clips_adds_new_ones_and_clears() {
        use crate::clip_picker::{ClipPick, clips};
        let first = temp("clip-first.png");
        let second = temp("clip-second.png");
        write_png(&first, 16, 8);
        write_png(&second, 10, 10);
        let mut a = Operandi::empty();
        a.chooser = Some(clip_panel);
        CLIP_ASKED.with(|n| n.set(0));
        a.load_image_file(&first);
        a.create_backdrop();
        let id = a.editor.scene.selection.item.unwrap();
        let listed = clips(&a.editor.scene);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "clip-first");

        // Pick an existing clip: no file panel, no new clip, one step.
        let steps = a.editor.history.undo.len();
        a.pick_backdrop_clip(id, ClipPick::Existing(listed[0].id));
        assert_eq!(backdrop_image(&a, id), Some(listed[0].id));
        assert_eq!(a.editor.scene.images.len(), 1);
        assert_eq!(CLIP_ASKED.with(|n| n.get()), 0);
        assert_eq!(a.editor.history.undo.len(), steps + 1);
        // Picking it again changes nothing and adds no step.
        a.pick_backdrop_clip(id, ClipPick::Existing(listed[0].id));
        assert_eq!(a.editor.history.undo.len(), steps + 1);

        // Add Clip…: the file panel opens; its file is one new clip, shown.
        CLIP_ANSWER.with(|p| *p.borrow_mut() = Some(second.clone()));
        a.pick_backdrop_clip(id, ClipPick::Add);
        assert_eq!(CLIP_ASKED.with(|n| n.get()), 1);
        assert_eq!(a.editor.scene.images.len(), 2);
        let added = a.editor.scene.images[1].id;
        assert_eq!(backdrop_image(&a, id), Some(added));
        assert_eq!(a.editor.history.undo.len(), steps + 2);
        // Add Clip… with a file the scene has reuses its entry.
        CLIP_ANSWER.with(|p| *p.borrow_mut() = Some(first.clone()));
        a.pick_backdrop_clip(id, ClipPick::Add);
        assert_eq!(a.editor.scene.images.len(), 2);
        assert_eq!(backdrop_image(&a, id), Some(listed[0].id));
        // Cancelling the panel changes nothing.
        let before = a.editor.history.undo.len();
        a.pick_backdrop_clip(id, ClipPick::Add);
        assert_eq!(a.editor.history.undo.len(), before);
        assert_eq!(backdrop_image(&a, id), Some(listed[0].id));

        // (none) clears; the clips stay in the scene.
        a.pick_backdrop_clip(id, ClipPick::Clear);
        assert_eq!(backdrop_image(&a, id), None);
        assert_eq!(a.editor.scene.images.len(), 2);
        // (none) again is no step.
        let cleared = a.editor.history.undo.len();
        a.pick_backdrop_clip(id, ClipPick::Clear);
        assert_eq!(a.editor.history.undo.len(), cleared);

        // Each choice undoes on its own.
        a.run(Command::Undo);
        assert_eq!(backdrop_image(&a, id), Some(listed[0].id));
        a.run(Command::Undo);
        assert_eq!(backdrop_image(&a, id), Some(added));
        a.run(Command::Undo);
        assert_eq!(backdrop_image(&a, id), Some(listed[0].id));
        assert_eq!(a.editor.scene.images.len(), 1, "Add Clip's clip undone");
        a.run(Command::Redo);
        assert_eq!(backdrop_image(&a, id), Some(added));
        assert_eq!(a.editor.scene.images.len(), 2);
        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    /// BC-05 / BC-04: the field opens the chooser; Esc closes it with no
    /// change; it closes by itself when its Backdrop is gone.
    #[test]
    fn bc_05_chooser_opens_from_the_field_and_esc_changes_nothing() {
        let path = temp("chooser.png");
        write_png(&path, 8, 8);
        let mut a = Operandi::empty();
        a.create_backdrop();
        let id = a.editor.scene.selection.item.unwrap();
        a.backdrop_image_chosen(id, path.clone());
        let image = backdrop_image(&a, id);
        let steps = a.editor.history.undo.len();
        let ctx = egui::Context::default();
        let run = |a: &mut Operandi, events: Vec<egui::Event>| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))),
                    events,
                    ..Default::default()
                },
                |ctx| a.backdrop_clip_chooser(ctx),
            );
        };
        a.open_backdrop_clips(id);
        run(&mut a, vec![]);
        assert!(a.backdrop_images.chooser.is_open());
        assert!(
            !a.backdrop_images.thumbs.is_empty(),
            "the clip has a thumbnail"
        );
        run(
            &mut a,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(!a.backdrop_images.chooser.is_open());
        assert_eq!(backdrop_image(&a, id), image);
        assert_eq!(a.editor.history.undo.len(), steps);
        // Its Backdrop deleted: the chooser closes.
        a.open_backdrop_clips(id);
        a.backdrop_images.chooser_item = Some(u64::MAX);
        run(&mut a, vec![]);
        assert!(!a.backdrop_images.chooser.is_open());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bd_62_pen_maps_from_the_axis_view_it_draws_in() {
        use operandi_core::backdrop::ViewKind;
        let mut a = Operandi::empty();
        a.camera.view = View::Front;
        assert_eq!(a.pen_uv_view(), Some(ViewKind::Axis(Projection::Front)));
        a.camera.view = View::Perspective;
        assert_eq!(a.pen_uv_view(), None);
        a.camera.view = View::Top;
        a.display.settings.drawing.backdrops = false;
        assert_eq!(a.pen_uv_view(), None, "Show Backdrop off");
        a.display.settings.drawing.backdrops = true;
        a.camera.through = true;
        assert_eq!(a.pen_uv_view(), None, "camera view");
    }
}
