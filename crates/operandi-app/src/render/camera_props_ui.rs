//! The camera's Properties tabs (spec camera-properties-2026-11-06):
//! Camera View, Camera Effects and, while Display > Show Camera Overlay is
//! on, Camera Overlay; plus the camera rows on the shared Display tab and
//! what Camera view draws for them (Background Image, overlay lines).
//! Settings, maths and undo live in the core (operandi_core::camera_props).
//!
//! Every control edits every selected camera (CP-X3) as one undo step
//! named after the property (CP-X1); a number the cameras disagree on
//! shows blank. Rows whose feature is off grey but keep their values
//! (CP-05). Provisional picks are CAM-Q01.. in
//! docs/GUI_QA_HANDOFF_M1_M2_M3.md.
use super::*;
use crate::unavailable::Gate;
use operandi_core::camera_light::{
    CameraEdit, CameraSettings, FilmFit, OverlayEdit, Projection, item_frame, local_transform_for,
};
use operandi_core::camera_props::{
    self as props, F_STOP_PRESETS, FilmBackPreset, LENS_PRESETS_MM, OverlayGrid, OverlayRole,
};

pub(super) const VIEW_TAB: &str = "Camera View";
pub(super) const EFFECTS_TAB: &str = "Camera Effects";
pub(super) const OVERLAY_TAB: &str = "Camera Overlay";

/// Film Back presets the user saved with "(new preset)", kept in
/// `film-back-presets.json` in the data folder.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct FilmBackPresets {
    pub presets: Vec<FilmBackPreset>,
}

impl FilmBackPresets {
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .map(|mut p| {
                p.presets
                    .retain(|f| f.width > 0.0 && f.height > 0.0 && !f.name.trim().is_empty());
                p
            })
            .unwrap_or_default()
    }
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
}

/// Opacity scale of overlay lines (0..1 → alpha).
fn overlay_color(rgb: [f64; 3], opacity: f64) -> Color32 {
    let [r, g, b] = rgb.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
    Color32::from_rgba_unmultiplied(r, g, b, (opacity.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Where the render picture of `aspect` (width ÷ height) sits in a view of
/// `r`: as large as fits, centred.
pub(crate) fn frame_rect_for(r: Rect, aspect: f32) -> Rect {
    let (w, h) = if r.width() / r.height().max(1.) > aspect {
        (r.height() * aspect, r.height())
    } else {
        (r.width(), r.width() / aspect)
    };
    Rect::from_center_size(r.center(), vec2(w, h))
}

impl Operandi {
    /// The selected cameras' settings, the primary first.
    fn selected_cameras(&self) -> Vec<CameraSettings> {
        self.common_targets()
            .iter()
            .filter_map(|id| self.editor.scene.item(*id))
            .filter_map(|i| i.camera.clone())
            .collect()
    }

    /// Core applies a camera edit to every selected camera as one undo
    /// step (CP-X3).
    fn camera_edit(&mut self, id: u64, edit: CameraEdit) {
        self.run(Command::EditCamera { item: id, edit });
    }

    /// A camera number shown in `scale` units, blank when the selected
    /// cameras differ. One undo step when the drag or typing ends.
    #[allow(clippy::too_many_arguments)]
    fn camera_number(
        &mut self,
        ui: &mut Ui,
        id: u64,
        key: &'static str,
        label: &str,
        get: impl Fn(&CameraSettings) -> f64,
        range: std::ops::RangeInclusive<f64>,
        suffix: &str,
        scale: f64,
        decimals: usize,
        make: impl Fn(f64) -> CameraEdit,
    ) -> Response {
        let values: Vec<f64> = self.selected_cameras().iter().map(&get).collect();
        let Some(&value) = values.first() else {
            return ui.label("");
        };
        let mixed = values.iter().any(|v| (v - value).abs() > 1e-12);
        let draft = match self.locator_draft {
            Some((i, k, v)) if i == id && k == key => Some(v),
            _ => None,
        };
        let mut shown = draft.unwrap_or(value * scale);
        // UIC-110: filled when the channel differs from a new camera's.
        let default = get(&CameraSettings::default());
        crate::form::rows::status(mixed || (value - default).abs() > 1e-12);
        // UIC-112: a keyable camera channel gets its key light.
        self.anim_row_for(
            &Command::EditCamera {
                item: id,
                edit: make(value),
            },
            mixed || (value - default).abs() > 1e-12,
        );
        let response = ui
            .horizontal(|ui| {
                crate::option_row::label(ui, label);
                crate::option_row::fill(ui, |ui| {
                    let drag = crate::numeric_tab::DragValue::new(&mut shown)
                        .range(range)
                        .speed(0.1)
                        .max_decimals(decimals)
                        .suffix(suffix);
                    // UIC-118: "(mixed)" until edited.
                    crate::form::rows::mixed(mixed && draft.is_none());
                    ui.add(drag)
                })
            })
            .inner;
        if response.changed() {
            self.locator_draft = Some((id, key, shown));
        }
        let finished = response.drag_stopped()
            || response.lost_focus()
            || (response.changed() && !response.dragged() && !response.has_focus());
        if finished
            && let Some((i, k, v)) = self.locator_draft
            && i == id
            && k == key
        {
            self.locator_draft = None;
            if mixed || (v / scale - value).abs() > 1e-12 {
                self.camera_edit(id, make(v / scale));
            }
        }
        response
    }

    /// A camera checkbox over every selected camera: "(mixed)" where they
    /// differ (UIC-118), filled circle where it isn't a new camera's value.
    fn camera_check(
        &mut self,
        ui: &mut Ui,
        id: u64,
        label: &str,
        get: impl Fn(&CameraSettings) -> bool,
        make: impl Fn(bool) -> CameraEdit,
    ) {
        let values: Vec<bool> = self.selected_cameras().iter().map(&get).collect();
        let Some(&first) = values.first() else {
            return;
        };
        let mixed = values.iter().any(|v| *v != first);
        crate::form::rows::status(first != get(&CameraSettings::default()));
        self.anim_row_for(
            &Command::EditCamera {
                item: id,
                edit: make(first),
            },
            first != get(&CameraSettings::default()),
        );
        crate::form::rows::mixed(mixed);
        let mut v = first;
        if crate::option_row::check(ui, label, &mut v).changed() {
            self.camera_edit(id, make(v));
        }
    }

    /// A camera dropdown over every selected camera (see `camera_check`).
    fn camera_choice<T: Copy + PartialEq>(
        &mut self,
        ui: &mut Ui,
        id: u64,
        label: &str,
        get: impl Fn(&CameraSettings) -> T,
        options: &[(T, &str)],
        make: impl Fn(T) -> CameraEdit,
    ) {
        let values: Vec<T> = self.selected_cameras().iter().map(&get).collect();
        let Some(&first) = values.first() else {
            return;
        };
        let mixed = values.iter().any(|v| *v != first);
        crate::form::rows::status(first != get(&CameraSettings::default()));
        self.anim_row_for(
            &Command::EditCamera {
                item: id,
                edit: make(first),
            },
            first != get(&CameraSettings::default()),
        );
        crate::form::rows::mixed(mixed);
        let mut v = first;
        if crate::option_row::choice(ui, &format!("camera-{label}-{id}"), label, &mut v, options)
            .inner
        {
            self.camera_edit(id, make(v));
        }
    }

    fn camera_section(ui: &mut Ui, title: &str, _id: u64, open: bool, body: impl FnOnce(&mut Ui)) {
        crate::form::rows::section(ui, title, !(open), |ui| {
            ui.spacing_mut().item_spacing.y = 2.;
            body(ui)
        });
    }

    /// Camera View tab rows below Name and Item Transform.
    pub(super) fn camera_view_tab(&mut self, ui: &mut Ui, item: &operandi_core::Item) {
        let Some(c) = item.camera.clone() else {
            return;
        };
        let id = item.id;
        let idle = self.editor.session.live.is_none();
        let locked = item.is_locked();
        let mm = 1000.;
        let deg = 180. / std::f64::consts::PI;
        ui.gated_ui(crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE), |ui| {
            ui.spacing_mut().item_spacing.y = 2.;
            // CP-V07 Reset is the shared Item Transform > Reset
            // (item_transform_ui.rs, transform.reset).
            // CP-V10 Set Target (shared targeting, LP-04) and the camera's
            // Set Focus Distance.
            Self::camera_section(ui, "Targeting", id, item.common.target.is_some(), |ui| {
                self.set_target_from_selection_row(ui, id, locked);
                self.targeting_rows(ui, item);
                if item.common.target.is_some() {
                    self.camera_check(ui, id, "Set Focus Distance", |c| c.focus_from_target, CameraEdit::FocusFromTarget);
                }
            });
            self.camera_number(ui, id, "target", "Target Distance", |c| c.target_distance, 0.001..=1e6, " m", 1., 3, CameraEdit::TargetDistance);
            // CP-V12 Sync to View.
            ui.horizontal(|ui| {
                crate::option_row::label(ui, "");
                let through = self.camera.through
                    && crate::camera_light_ui::view_camera(&self.editor.scene) == Some(id);
                if ui
                    .gated(crate::unavailable::unless(!locked && !through, if locked {
                        "The camera is locked (Assembly > Lock)"
                    } else {
                        "This view already looks through the camera"
                    }), Button::new("Sync to View"))
                    .on_hover_text("Move and turn this camera to match the 3D view")
                    .clicked()
                {
                    self.sync_camera_to_view(id);
                }
            });
            Self::camera_section(ui, "Projection", id, true, |ui| {
                let options = Projection::ALL.map(|p| (p, p.label()));
                self.camera_choice(ui, id, "Projection Type", |c| c.projection, &options, CameraEdit::Projection);
                if c.projection.is_panoramic() {
                    ui.weak("Renders only: views show it as Perspective");
                }
                self.camera_number(ui, id, "focal", "Focal Length", |c| c.focal_length, 1.0..=1e5, " mm", mm, 3, CameraEdit::FocalLength);
                // CP-V14 lens presets.
                let lens = crate::option_row::dropdown(ui, &format!("camera-lens-{id}"), "Lens Preset", "", |ui| {
                    let mut picked = None;
                    for f in LENS_PRESETS_MM {
                        let on = (c.focal_length * mm - f).abs() < 1e-6;
                        if ui.selectable_label(on, format!("{f} mm")).clicked() {
                            picked = Some(f);
                        }
                    }
                    picked
                })
                .inner;
                if let Some(f) = lens {
                    self.camera_edit(id, CameraEdit::FocalLength(f / mm));
                }
                self.camera_number(ui, id, "aov", "Angle of View", |c| c.angle_of_view(), 0.01..=179.9, "°", deg, 3, CameraEdit::AngleOfView);
                ui.gated_ui(crate::unavailable::unless(c.projection == Projection::Perspective, "Only for Perspective projection"), |ui| {
                    self.camera_number(ui, id, "distortion", "Lens Distortion", |c| c.lens_distortion, -1.0..=1.0, "", 1., 3, CameraEdit::LensDistortion)
                        .on_hover_text("Positive barrel, negative pincushion. Stored only: renders ignore it (provisional)");
                });
                if c.projection == Projection::Orthographic {
                    let [w, h] = c.render_size(self.editor.scene.render.frame);
                    let width = c.orthographic_width(f64::from(w) / f64::from(h.max(1)));
                    crate::option_row::value(ui, "Ortho Width", &format!("{width:.3} m"))
                        .on_hover_text("Film Width × Target Distance ÷ Focal Length (CP-Q1)");
                }
            });
            Self::camera_section(ui, "Resolution Override", id, true, |ui| {
                self.camera_check(ui, id, "Override Render Resolution", |c| c.resolution.enable, CameraEdit::ResolutionEnable);
                ui.gated_ui(crate::unavailable::unless(c.resolution.enable, "Turn on Override Render Resolution first"), |ui| {
                    let max = f64::from(props::MAX_RESOLUTION);
                    self.camera_number(ui, id, "res-w", "Width", |c| f64::from(c.resolution.width), 1.0..=max, " px", 1., 0, |v| CameraEdit::ResolutionWidth(v.round() as u32));
                    self.camera_number(ui, id, "res-h", "Height", |c| f64::from(c.resolution.height), 1.0..=max, " px", 1., 0, |v| CameraEdit::ResolutionHeight(v.round() as u32));
                });
            });
            Self::camera_section(ui, "Images", id, false, |ui| {
                self.camera_background_rows(ui, id, &c);
            });
            Self::camera_section(ui, "Film Back", id, true, |ui| {
                self.film_back_rows(ui, id, &c);
                self.camera_number(ui, id, "film-w", "Film Width", |c| c.film_width, 0.1..=1e5, " mm", mm, 3, CameraEdit::FilmWidth);
                self.camera_number(ui, id, "film-h", "Film Height", |c| c.film_height, 0.1..=1e5, " mm", mm, 3, CameraEdit::FilmHeight);
                self.camera_number(ui, id, "offset-x", "Film Offset X", |c| c.film_offset[0], -1e5..=1e5, " mm", mm, 3, CameraEdit::FilmOffsetX);
                self.camera_number(ui, id, "offset-y", "Film Offset Y", |c| c.film_offset[1], -1e5..=1e5, " mm", mm, 3, CameraEdit::FilmOffsetY);
                self.camera_number(ui, id, "squeeze", "Lens Squeeze", |c| c.stored.lens_squeeze, 0.01..=100.0, "", 1., 3, CameraEdit::LensSqueeze)
                    .on_hover_text("Stored; renders ignore it for now");
                self.camera_number(ui, id, "film-roll", "Film Roll", |c| c.stored.film_roll, -36000.0..=36000.0, "°", deg, 3, CameraEdit::FilmRoll)
                    .on_hover_text("Stored; renders ignore it for now");
                let fits = FilmFit::ALL.map(|f| (f, f.label()));
                self.camera_choice(ui, id, "Film Fit", |c| c.film_fit, &fits, CameraEdit::FilmFit);
            });
            ui.add_space(4.);
            let is_render = self.editor.scene.render_camera() == Some(id);
            ui.horizontal(|ui| {
                crate::option_row::label(ui, "");
                if ui
                    .gated(crate::unavailable::unless(!is_render, "This is already the render camera"), Button::new(if is_render { "Render Camera" } else { "Set as Render Camera" }))
                    .on_hover_text("Cycles renders and Camera view look through the render camera")
                    .clicked()
                {
                    self.run(Command::SetRenderCamera(id));
                }
            });
        });
    }

    /// CP-V10: Set Target from the selection: exactly this camera and one
    /// other item selected.
    fn set_target_from_selection_row(&mut self, ui: &mut Ui, id: u64, locked: bool) {
        let ids = self.common_targets();
        let other = (ids.len() == 2 && ids.contains(&id))
            .then(|| ids.iter().copied().find(|i| *i != id))
            .flatten();
        ui.horizontal(|ui| {
            crate::option_row::label(ui, "");
            if ui
                .gated(
                    crate::unavailable::unless(
                        !locked && other.is_some(),
                        "Select the camera, then one other item",
                    ),
                    Button::new("Set Target"),
                )
                .on_hover_text("Aim this camera at the other selected item's centre")
                .clicked()
                && let Some(t) = other
            {
                self.run_common_on(
                    vec![id],
                    operandi_core::item_common::ItemCommonEdit::Target(Some(t)),
                );
            }
        });
    }

    /// CP-V12: move camera `id` to where this 3D view looks from.
    pub(super) fn sync_camera_to_view(&mut self, id: u64) {
        let (eye, right, up, forward) = self.camera.basis();
        let rotation = glam::DQuat::from_mat3(&glam::DMat3::from_cols(right, up, -forward));
        let result = local_transform_for(&self.editor.scene, id, eye, rotation).and_then(|t| {
            self.editor.dispatch(Command::SyncCameraToView {
                item: id,
                transform: t,
                target_distance: self.camera.distance.max(0.001),
            })
        });
        self.status = match result {
            Ok(()) => "Camera synced to the view".into(),
            Err(e) => format!("Sync to View: {e}"),
        };
    }

    /// CP-V25..V28.
    fn camera_background_rows(&mut self, ui: &mut Ui, id: u64, c: &CameraSettings) {
        self.camera_check(
            ui,
            id,
            "Background Image",
            |c| c.background.enable,
            CameraEdit::BackgroundEnable,
        );
        let scene = &self.editor.scene;
        let current = c.background.image.and_then(|i| scene.image(i));
        let text = current.map_or("(none)".to_owned(), |i| i.name.clone());
        let size = current.map(|i| (i.width, i.height));
        let images: Vec<(u64, String)> = scene
            .images
            .iter()
            .map(|i| (i.id, i.name.clone()))
            .collect();
        let missing =
            current.and_then(|i| self.backdrop_images.problem(&i.path).map(str::to_owned));
        let mut chosen = None;
        let mut load = false;
        ui.gated_ui(
            crate::unavailable::unless(c.background.enable, "Turn on Background Image first"),
            |ui| {
                crate::option_row::dropdown(ui, &format!("camera-bg-{id}"), "Image", &text, |ui| {
                    if ui
                        .selectable_label(c.background.image.is_none(), "(none)")
                        .clicked()
                    {
                        chosen = Some(None);
                    }
                    for (image, name) in &images {
                        if ui
                            .selectable_label(c.background.image == Some(*image), name)
                            .clicked()
                        {
                            chosen = Some(Some(*image));
                        }
                    }
                    ui.separator();
                    if ui.button("Load Image…").clicked() {
                        load = true;
                    }
                });
                if let Some(missing) = &missing {
                    ui.colored_label(theme::error(), missing);
                }
                self.camera_number(
                    ui,
                    id,
                    "overscan",
                    "Overscan",
                    |c| c.stored.overscan,
                    1.0..=10000.0,
                    " %",
                    100.,
                    1,
                    CameraEdit::Overscan,
                )
                .on_hover_text("Scales the background image, keeping its shape");
                ui.horizontal(|ui| {
                    crate::option_row::label(ui, "");
                    if ui
                        .gated(
                            crate::unavailable::unless(
                                size.is_some(),
                                "Choose a background image first",
                            ),
                            Button::new("Set Resolution from Clip"),
                        )
                        .on_hover_text(
                            "Turn on Resolution Override at the image's pixel size (provisional)",
                        )
                        .clicked()
                        && let Some((width, height)) = size
                    {
                        self.camera_edit(id, CameraEdit::ResolutionFromClip { width, height });
                    }
                });
            },
        );
        if let Some(image) = chosen
            && image != c.background.image
        {
            self.camera_edit(id, CameraEdit::BackgroundImage(image));
        }
        if load {
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
                .or_else(crate::backdrop_ui::dirs_home)
                .unwrap_or_default();
            if !self.choose(
                file_chooser::Choose::Image { folder },
                file_chooser::Purpose::CameraBackground(id),
            ) {
                self.status = "No file panel is available to choose a picture".into();
            }
        }
    }

    /// A picture file chosen for camera `item`'s Background Image.
    pub(super) fn camera_background_chosen(&mut self, item: u64, path: std::path::PathBuf) {
        match crate::backdrop_ui::decode(&path) {
            Ok((pixels, (width, height))) => {
                let key = path.to_string_lossy().into_owned();
                self.backdrop_images.insert_ready(&key, pixels);
                if self.run_ok(Command::CameraBackgroundLoad {
                    item,
                    path: key,
                    width,
                    height,
                }) {
                    self.status = format!("Background image {width} × {height} px");
                }
            }
            Err(error) => self.status = error,
        }
    }

    /// CP-V36 Film Back Preset, with "(new preset)" to save the current
    /// film under a name.
    fn film_back_rows(&mut self, ui: &mut Ui, id: u64, c: &CameraSettings) {
        let mut all = props::built_in_film_backs();
        all.extend(self.film_backs.presets.iter().cloned());
        let shown =
            props::matching_film_back(c, &all).map_or("(custom)".to_owned(), |p| p.name.clone());
        let picked = crate::option_row::dropdown(
            ui,
            &format!("camera-film-back-{id}"),
            "Film Back Preset",
            &shown,
            |ui| {
                let mut picked = None;
                for p in &all {
                    if ui.selectable_label(p.name == shown, &p.name).clicked() {
                        picked = Some(Some(p.clone()));
                    }
                }
                ui.separator();
                if ui.selectable_label(false, "(new preset)").clicked() {
                    picked = Some(None);
                }
                picked
            },
        )
        .inner;
        match picked {
            Some(Some(p)) => self.camera_edit(
                id,
                CameraEdit::FilmBack {
                    width: p.width,
                    height: p.height,
                },
            ),
            Some(None) => {
                self.film_back_naming = Some(format!(
                    "{:.4} × {:.4} mm",
                    c.film_width * 1000.,
                    c.film_height * 1000.
                ));
            }
            None => {}
        }
        if let Some(mut name) = self.film_back_naming.take() {
            let mut keep = true;
            ui.horizontal(|ui| {
                crate::option_row::label(ui, "Preset Name");
                let r = ui
                    .add(TextEdit::singleline(&mut name).desired_width(ui.available_width() - 90.));
                self.text_editors.insert(r.id);
                crate::numeric_tab::register(ui, &r, Some(&name));
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                if ui.button("Save").clicked() || enter {
                    keep = false;
                    let name = name.trim().to_owned();
                    if name.is_empty() {
                        self.status = "Name the film back preset".into();
                    } else {
                        self.film_backs.presets.retain(|p| p.name != name);
                        self.film_backs.presets.push(FilmBackPreset {
                            name: name.clone(),
                            width: c.film_width,
                            height: c.film_height,
                        });
                        self.status = match self
                            .film_backs
                            .save(&self.environment.paths().film_back_presets())
                        {
                            Ok(()) => format!("Saved film back preset {name}"),
                            Err(e) => format!("Couldn't save the film back preset: {e}"),
                        };
                    }
                }
                if ui.button("Cancel").clicked() {
                    keep = false;
                }
            });
            if keep {
                self.film_back_naming = Some(name);
            }
        }
    }

    /// Camera Effects tab: Depth of Field and Clipping Plane. Motion Blur
    /// waits for animation; Stereo is Tier 3.
    pub(super) fn camera_effects_tab(&mut self, ui: &mut Ui) {
        let Some(item) = self
            .editor
            .scene
            .active()
            .filter(|i| i.camera.is_some())
            .cloned()
        else {
            ui.weak("Select a camera");
            return;
        };
        let Some(c) = item.camera.clone() else {
            return;
        };
        let id = item.id;
        let deg = 180. / std::f64::consts::PI;
        ui.gated_ui(crate::unavailable::unless(self.editor.session.live.is_none(), crate::unavailable::TOOL_ACTIVE), |ui| {
            ui.spacing_mut().item_spacing.y = 2.;
            Self::camera_section(ui, "Depth of Field", id, true, |ui| {
                self.camera_check(ui, id, "Enable", |c| c.dof.enable, CameraEdit::DofEnable);
                let on = c.dof.enable;
                let driven =
                    c.focus_from_target && item.common.target.as_ref().is_some_and(|t| t.enable);
                ui.gated_ui(crate::unavailable::first([crate::unavailable::unless(on, "Turn on Depth of Field first"), crate::unavailable::unless(!driven, "Set Focus Distance follows the target; turn it off to type a distance")]), |ui| {
                    self.camera_number(
                        ui,
                        id,
                        "focus",
                        "Focus Distance",
                        |c| c.focus_distance,
                        0.001..=1e6,
                        " m",
                        1.,
                        3,
                        CameraEdit::FocusDistance,
                    )
                    .on_disabled_hover_text(if driven {
                        "Set Focus Distance: the target's distance drives it"
                    } else {
                        "Turn on Depth of Field"
                    });
                    ui.horizontal(|ui| {
                        crate::option_row::label(ui, "");
                        if ui
                            .button("Autofocus")
                            .on_hover_text(
                                "Focus on the surface under the centre of the camera's view",
                            )
                            .clicked()
                        {
                            self.run(Command::CameraAutofocus(id));
                        }
                    });
                });
                // CP-E04: F-Stop stays live with depth of field off.
                self.camera_number(
                    ui,
                    id,
                    "fstop",
                    "F-Stop",
                    |c| c.dof.f_stop,
                    props::F_STOP_MIN..=1000.0,
                    "",
                    1.,
                    2,
                    CameraEdit::FStop,
                )
                .on_hover_text("Smaller: shallower focus. Doesn't change exposure");
                let preset = crate::option_row::dropdown(
                    ui,
                    &format!("camera-fstop-{id}"),
                    "F-Stop Preset",
                    "",
                    |ui| {
                        let mut picked = None;
                        for f in F_STOP_PRESETS {
                            if ui
                                .selectable_label((c.dof.f_stop - f).abs() < 1e-9, format!("f/{f}"))
                                .clicked()
                            {
                                picked = Some(f);
                            }
                        }
                        picked
                    },
                )
                .inner;
                if let Some(f) = preset {
                    self.camera_edit(id, CameraEdit::FStop(f));
                }
                ui.gated_ui(crate::unavailable::unless(on, "Turn on Depth of Field first"), |ui| {
                    let r = self.camera_number(
                        ui,
                        id,
                        "blades",
                        "Iris Blades",
                        |c| f64::from(c.dof.iris_blades),
                        0.0..=f64::from(props::IRIS_BLADES_MAX),
                        "",
                        1.,
                        0,
                        |v| {
                            // 1 and 2 aren't shapes: round down to a circle.
                            let n = v.round() as u32;
                            CameraEdit::IrisBlades(if n < 3 { 0 } else { n })
                        },
                    );
                    r.on_hover_text("0: round bokeh; 3 to 16: that many sides");
                    self.camera_number(
                        ui,
                        id,
                        "iris-rot",
                        "Iris Rotation",
                        |c| c.dof.iris_rotation,
                        -36000.0..=36000.0,
                        "°",
                        deg,
                        2,
                        CameraEdit::IrisRotation,
                    );
                    self.camera_number(
                        ui,
                        id,
                        "edge-weight",
                        "Edge Weighting",
                        |c| c.dof.edge_weighting,
                        0.0..=100.0,
                        " %",
                        100.,
                        1,
                        CameraEdit::EdgeWeighting,
                    )
                    .on_hover_text("Stored; Cycles has no matching control (provisional)");
                });
                ui.weak("Renders only; views stay sharp");
            });
            Self::camera_section(ui, "Clipping Plane", id, true, |ui| {
                self.camera_check(ui, id, "Enable", |c| c.clipping, CameraEdit::Clipping);
                // CP-E18: not greyed with clipping off.
                self.camera_number(
                    ui,
                    id,
                    "near",
                    "Clipping Distance",
                    |c| c.clip_near,
                    0.0001..=1e6,
                    " m",
                    1.,
                    3,
                    CameraEdit::ClipNear,
                );
                self.camera_number(
                    ui,
                    id,
                    "far",
                    "Far Clip",
                    |c| c.clip_far,
                    0.001..=1e7,
                    " m",
                    1.,
                    3,
                    CameraEdit::ClipFar,
                );
            });
        });
    }

    /// Display tab rows a camera adds: Size (CP-D04), Show Camera Overlay
    /// (CP-D12) and Show Stereo Volume (CP-D13, waits for stereo).
    pub(super) fn camera_display_rows(&mut self, ui: &mut Ui) {
        let Some(item) = self
            .editor
            .scene
            .active()
            .filter(|i| i.camera.is_some())
            .cloned()
        else {
            return;
        };
        let Some(c) = item.camera.clone() else {
            return;
        };
        let id = item.id;
        ui.gated_ui(
            crate::unavailable::unless(
                self.editor.session.live.is_none(),
                crate::unavailable::TOOL_ACTIVE,
            ),
            |ui| {
                ui.spacing_mut().item_spacing.y = 2.;
                ui.separator();
                ui.gated_ui(
                    crate::unavailable::unless(!item.is_locked(), crate::unavailable::LOCKED),
                    |ui| {
                        self.camera_number(
                            ui,
                            id,
                            "display-size",
                            "Size",
                            |c| c.stored.display_size,
                            0.0..=1e6,
                            "",
                            1.,
                            3,
                            CameraEdit::DisplaySize,
                        )
                        .on_hover_text("Glyph size in 3D views; 0 keeps a constant size on screen");
                    },
                );
                let on = c.overlay.is_some();
                ui.horizontal(|ui| {
                    crate::option_row::label(ui, "");
                    if ui
                    .button(if on {
                        "Hide Camera Overlay"
                    } else {
                        "Show Camera Overlay"
                    })
                    .on_hover_text(
                        "Composition guides in a view looking through this camera; never rendered",
                    )
                    .clicked()
                {
                    self.camera_edit(id, CameraEdit::ShowOverlay(!on));
                    if !on {
                        self.properties_tab = OVERLAY_TAB.to_owned();
                    }
                }
                });
                ui.horizontal(|ui| {
                    crate::option_row::label(ui, "");
                    ui.gated(
                        Some("Needs stereo cameras (not built yet)"),
                        Button::new("Show Stereo Volume"),
                    );
                });
            },
        );
    }

    /// Camera Overlay tab (CP-O01..O14, no golden-rectangle rows).
    pub(super) fn camera_overlay_tab(&mut self, ui: &mut Ui) {
        let Some(item) = self
            .editor
            .scene
            .active()
            .filter(|i| i.camera.is_some())
            .cloned()
        else {
            ui.weak("Select a camera");
            return;
        };
        let id = item.id;
        let Some(o) = item.camera.as_ref().and_then(|c| c.overlay.clone()) else {
            ui.weak("Display > Show Camera Overlay adds the overlay");
            return;
        };
        let ov = |e: OverlayEdit| CameraEdit::Overlay(e);
        ui.gated_ui(
            crate::unavailable::unless(
                self.editor.session.live.is_none(),
                crate::unavailable::TOOL_ACTIVE,
            ),
            |ui| {
                ui.spacing_mut().item_spacing.y = 2.;
                ui.weak("Drawn when looking through this camera; never rendered");
                self.camera_number(
                    ui,
                    id,
                    "ov-opacity",
                    "Overlay Opacity",
                    |c| overlay_of(c).opacity,
                    0.0..=100.0,
                    " %",
                    100.,
                    1,
                    move |v| ov(OverlayEdit::Opacity(v)),
                );
                Self::camera_section(ui, "Safe Areas", id, true, |ui| {
                    self.camera_check(
                        ui,
                        id,
                        "Action Safe Visible",
                        |c| overlay_of(c).action_safe,
                        |v| CameraEdit::Overlay(OverlayEdit::ActionSafe(v)),
                    );
                    ui.gated_ui(
                        crate::unavailable::unless(
                            o.action_safe,
                            "Turn on Action Safe Visible first",
                        ),
                        |ui| {
                            self.camera_number(
                                ui,
                                id,
                                "ov-ah",
                                "Horizontal Margin",
                                |c| overlay_of(c).action_margin[0],
                                0.0..=49.9,
                                " %",
                                100.,
                                2,
                                move |v| ov(OverlayEdit::ActionMargin(0, v)),
                            );
                            self.camera_number(
                                ui,
                                id,
                                "ov-av",
                                "Vertical Margin",
                                |c| overlay_of(c).action_margin[1],
                                0.0..=49.9,
                                " %",
                                100.,
                                2,
                                move |v| ov(OverlayEdit::ActionMargin(1, v)),
                            );
                        },
                    );
                    self.camera_check(
                        ui,
                        id,
                        "Title Safe Visible",
                        |c| overlay_of(c).title_safe,
                        |v| CameraEdit::Overlay(OverlayEdit::TitleSafe(v)),
                    );
                    ui.gated_ui(
                        crate::unavailable::unless(
                            o.title_safe,
                            "Turn on Title Safe Visible first",
                        ),
                        |ui| {
                            self.camera_number(
                                ui,
                                id,
                                "ov-th",
                                "Horizontal Margin ",
                                |c| overlay_of(c).title_margin[0],
                                0.0..=49.9,
                                " %",
                                100.,
                                2,
                                move |v| ov(OverlayEdit::TitleMargin(0, v)),
                            );
                            self.camera_number(
                                ui,
                                id,
                                "ov-tv",
                                "Vertical Margin ",
                                |c| overlay_of(c).title_margin[1],
                                0.0..=49.9,
                                " %",
                                100.,
                                2,
                                move |v| ov(OverlayEdit::TitleMargin(1, v)),
                            );
                        },
                    );
                    self.overlay_color_row(ui, id, "Color", o.safe_color, OverlayEdit::SafeColor);
                });
                Self::camera_section(ui, "Grid", id, true, |ui| {
                    let grids = OverlayGrid::ALL.map(|g| (g, g.label()));
                    self.camera_choice(
                        ui,
                        id,
                        "Grid",
                        |c| overlay_of(c).grid,
                        &grids,
                        move |g| ov(OverlayEdit::Grid(g)),
                    );
                    self.overlay_color_row(
                        ui,
                        id,
                        "Grid Color",
                        o.grid_color,
                        OverlayEdit::GridColor,
                    );
                });
                Self::camera_section(ui, "Guides", id, true, |ui| {
                    self.camera_check(
                        ui,
                        id,
                        "Right Diagonal",
                        |c| overlay_of(c).right_diagonal,
                        |v| CameraEdit::Overlay(OverlayEdit::RightDiagonal(v)),
                    );
                    self.camera_check(
                        ui,
                        id,
                        "Left Diagonal",
                        |c| overlay_of(c).left_diagonal,
                        |v| CameraEdit::Overlay(OverlayEdit::LeftDiagonal(v)),
                    );
                    self.camera_check(
                        ui,
                        id,
                        "Chevron Up",
                        |c| overlay_of(c).chevron_up,
                        |v| CameraEdit::Overlay(OverlayEdit::ChevronUp(v)),
                    );
                    self.camera_check(
                        ui,
                        id,
                        "Chevron Down",
                        |c| overlay_of(c).chevron_down,
                        |v| CameraEdit::Overlay(OverlayEdit::ChevronDown(v)),
                    );
                    self.camera_check(
                        ui,
                        id,
                        "Horizon",
                        |c| overlay_of(c).horizon,
                        |v| CameraEdit::Overlay(OverlayEdit::Horizon(v)),
                    );
                    self.overlay_color_row(
                        ui,
                        id,
                        "Line Color",
                        o.line_color,
                        OverlayEdit::LineColor,
                    );
                });
            },
        );
    }

    fn overlay_color_row(
        &mut self,
        ui: &mut Ui,
        id: u64,
        label: &str,
        value: [f64; 3],
        make: fn([f64; 3]) -> OverlayEdit,
    ) {
        ui.horizontal(|ui| {
            crate::option_row::label(ui, label);
            crate::option_row::fill(ui, |ui| {
                let mut rgb = value;
                // One undo step per drag or pick (History::undo_fold).
                if crate::colour_field::show(ui, &mut rgb, crate::colour_field::Options::ui())
                    .changed()
                {
                    self.camera_edit(id, CameraEdit::Overlay(make(rgb)));
                }
            })
        });
    }

    /// Camera view: the Background Image behind everything (drawn first
    /// in the Backdrop pass) for the camera this view looks through.
    pub(super) fn camera_background_draw(&mut self, r: Rect) -> Option<crate::backdrop_gpu::Draw> {
        if !self.camera.through {
            return None;
        }
        let scene = &self.editor.scene;
        let id = crate::camera_light_ui::view_camera(scene)?;
        let item = scene.item(id)?;
        let c = item.camera.as_ref()?;
        if !c.background.enable {
            return None;
        }
        let image = scene.image(c.background.image?)?;
        let mut problem = None;
        let pixels = self.backdrop_images.get(&image.path.clone(), &mut problem);
        if let Some(problem) = problem {
            self.status = problem;
        }
        let pixels = pixels?;
        let [w, h] = c.render_size(scene.render.frame);
        let picture = frame_rect_for(r, w as f32 / h.max(1) as f32);
        // Fit the image in the picture, keeping its shape, then Overscan.
        let image_aspect = f64::from(pixels.width) / f64::from(pixels.height.max(1));
        let frame_aspect = f64::from(picture.width()) / f64::from(picture.height().max(1.));
        let (mut sx, mut sy) = if image_aspect > frame_aspect {
            (1.0, frame_aspect / image_aspect)
        } else {
            (image_aspect / frame_aspect, 1.0)
        };
        sx *= c.stored.overscan;
        sy *= c.stored.overscan;
        let world = self.camera_frame_corners(r, sx, sy)?;
        Some(crate::backdrop_gpu::Draw {
            corners: world,
            pixels,
            flip: false,
            invert: false,
            blend: true,
            contrast: 0.0,
            brightness: 0.0,
            transparency: 0.0,
        })
    }

    /// In a camera view, the world corners (top-left, top-right,
    /// bottom-right, bottom-left) of the view camera's render frame scaled
    /// by `sx` × `sy` about its centre, on a plane in front of the view
    /// camera (nothing depth-tests against it). `None` outside a camera view.
    pub(super) fn camera_frame_corners(&self, r: Rect, sx: f64, sy: f64) -> Option<[DVec3; 4]> {
        if !self.camera.through {
            return None;
        }
        let scene = &self.editor.scene;
        let id = crate::camera_light_ui::view_camera(scene)?;
        let c = scene.item(id)?.camera.as_ref()?;
        let [w, h] = c.render_size(scene.render.frame);
        let picture = frame_rect_for(r, w as f32 / h.max(1) as f32);
        let centre = picture.center();
        let half = vec2(
            picture.width() * 0.5 * sx as f32,
            picture.height() * 0.5 * sy as f32,
        );
        let corners = [
            centre + vec2(-half.x, -half.y),
            centre + vec2(half.x, -half.y),
            centre + vec2(half.x, half.y),
            centre + vec2(-half.x, half.y),
        ];
        let depth = self.camera.distance.max(0.01);
        let forward = self.camera.basis().3;
        let mut world = [DVec3::ZERO; 4];
        for (k, p) in corners.iter().enumerate() {
            let (o, d) = self.camera.ray(*p, r);
            world[k] = o + d * (depth / d.dot(forward).max(1e-6));
        }
        Some(world)
    }

    /// Camera view: the Camera Overlay's lines over the render frame.
    pub(super) fn draw_camera_overlay(&self, painter: &egui::Painter, r: Rect) {
        if !self.camera.through {
            return;
        }
        let scene = &self.editor.scene;
        let Some(id) = crate::camera_light_ui::view_camera(scene) else {
            return;
        };
        let Some(c) = scene.item(id).and_then(|i| i.camera.as_ref()) else {
            return;
        };
        let Some(o) = &c.overlay else {
            return;
        };
        let [w, h] = c.render_size(scene.render.frame);
        let aspect = w as f32 / h.max(1) as f32;
        let f = frame_rect_for(r, aspect);
        let tilt = item_frame(scene, id).map_or(0.0, |fr| props::horizon_tilt(&fr));
        let at = |p: [f64; 2]| {
            pos2(
                f.left() + p[0] as f32 * f.width(),
                f.top() + p[1] as f32 * f.height(),
            )
        };
        let painter = painter.with_clip_rect(f);
        for line in props::overlay_lines(o, f64::from(aspect), tilt) {
            let color = match line.role {
                OverlayRole::Safe => o.safe_color,
                OverlayRole::Grid => o.grid_color,
                OverlayRole::Guide => o.line_color,
            };
            painter.line_segment(
                [at(line.a), at(line.b)],
                Stroke::new(1.0f32, overlay_color(color, o.opacity)),
            );
        }
    }
}

/// Put `bg` behind a premultiplied linear RGBA render of `width` ×
/// `height` (top row first): fitted inside the picture keeping its shape,
/// scaled by `overscan`, sampled nearest. Covered pixels end opaque.
pub(crate) fn composite_background(
    rgba: &mut [f32],
    width: u32,
    height: u32,
    bg: &crate::backdrop_gpu::Pixels,
    overscan: f64,
) {
    if bg.width == 0 || bg.height == 0 || width == 0 || height == 0 {
        return;
    }
    let (w, h) = (f64::from(width), f64::from(height));
    let image_aspect = f64::from(bg.width) / f64::from(bg.height);
    let (mut iw, mut ih) = if image_aspect > w / h {
        (w, w / image_aspect)
    } else {
        (h * image_aspect, h)
    };
    iw *= overscan;
    ih *= overscan;
    let (x0, y0) = ((w - iw) * 0.5, (h - ih) * 0.5);
    let to_linear = |c: u8| {
        let v = f32::from(c) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    for y in 0..height {
        let v = (f64::from(y) + 0.5 - y0) / ih;
        if !(0.0..1.0).contains(&v) {
            continue;
        }
        let sy = ((v * f64::from(bg.height)) as u32).min(bg.height - 1);
        for x in 0..width {
            let u = (f64::from(x) + 0.5 - x0) / iw;
            if !(0.0..1.0).contains(&u) {
                continue;
            }
            let sx = ((u * f64::from(bg.width)) as u32).min(bg.width - 1);
            let s = ((sy * bg.width + sx) * 4) as usize;
            let a_bg = f32::from(bg.rgba[s + 3]) / 255.0;
            let o = ((y * width + x) * 4) as usize;
            let cover = 1.0 - rgba[o + 3].clamp(0.0, 1.0);
            for k in 0..3 {
                rgba[o + k] += to_linear(bg.rgba[s + k]) * a_bg * cover;
            }
            rgba[o + 3] += a_bg * cover;
        }
    }
}

impl Operandi {
    /// The render camera's Background Image for a final render, with its
    /// Overscan, when it's on and the picture is loaded.
    pub(super) fn render_camera_background(
        &mut self,
    ) -> Option<(std::sync::Arc<crate::backdrop_gpu::Pixels>, f64)> {
        let scene = &self.editor.scene;
        let id = scene.render_camera()?;
        let c = scene.item(id)?.camera.as_ref()?;
        if !c.background.enable {
            return None;
        }
        let path = scene.image(c.background.image?)?.path.clone();
        let overscan = c.stored.overscan;
        let mut problem = None;
        let pixels = self.backdrop_images.get(&path, &mut problem);
        if let Some(problem) = problem {
            self.status = problem;
        }
        Some((pixels?, overscan))
    }
}

fn overlay_of(c: &CameraSettings) -> props::CameraOverlay {
    c.overlay.clone().unwrap_or_default()
}
