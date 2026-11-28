//! The Render item's Properties (render item spec v1.2, specifications/
//! render-item-2026-11-11): clicking Render at the top of the Shader Tree
//! selects it (RI-02) and the Properties panel shows its Frame, Settings
//! and Preview tabs (RI-10). Every change is one "Render Settings" undo
//! step (RI-04); a drag on a number is one step for the whole drag.
//!
//! Built: Render Camera, Frame Width / Height, Maximum Quality, Noise
//! Threshold, the three ray depths, Maximum Radiance, the Global
//! Illumination tab's Caustics section (Direct Caustics spec v1),
//! Convergence Target and Max Render Time (Preview only). Left off until
//! built (RI-14, RI-B05/B06): User Channels and Tags tabs, Apply Preset,
//! Ray Tracing Engine, Resolution Unit / DPI, Pixel Aspect Ratio, Render
//! Region, Antialiasing Filter, the Geometry and Displacement rows, and
//! the frame size's value presets and link (UIC-55/56, which the form
//! doesn't have yet).
use super::*;
use crate::form::Unit;
use crate::form::rows::{check_d, choice_d, number_d};
use crate::unavailable::Gate;
use operandi_core::render_settings::{
    Caustics, MAX_DEPTH, MAX_FRAME, MAX_LOCAL_PHOTONS, MAX_TOTAL_PHOTONS, PANEL_MAX_SAMPLES,
    RenderSettings,
};

/// The Render item's tabs (RI-10), top to bottom.
pub(crate) const FRAME_TAB: &str = "Frame";
pub(crate) const SETTINGS_TAB: &str = "Settings";
pub(crate) const GI_TAB: &str = "Global Illumination";
pub(crate) const PREVIEW_TAB: &str = "Preview";
pub(crate) const TABS: [&str; 4] = [FRAME_TAB, SETTINGS_TAB, GI_TAB, PREVIEW_TAB];

impl Operandi {
    /// Whether the Render row is the Shader Tree's selection.
    pub(crate) fn render_item_selected(&self) -> bool {
        self.shading.render_selected
            && self.shading.selected.is_empty()
            && self.shading.environment.is_none()
    }

    /// RI-02: select the Render item (and nothing else in the tree).
    pub(crate) fn select_render_item(&mut self) {
        self.shading.selected.clear();
        self.shading.environment = None;
        self.shading.render_selected = true;
    }

    /// The rows of the Render item's `tab`.
    pub(super) fn render_item_rows(&mut self, ui: &mut Ui, tab: &str) {
        let idle = self.editor.session.live.is_none();
        let before = self.editor.scene.render.clone();
        let mut s = before.clone();
        let defaults = RenderSettings::new_scene();
        let mut changed = None::<Response>;
        let mut note = |r: Response| {
            if r.changed() {
                changed = Some(match changed.take() {
                    Some(c) => c | r,
                    None => r,
                });
            }
        };
        let mut camera_pick = None;
        ui.spacing_mut().item_spacing.y = 2.;
        // RI-11: the Name row, greyed (the Render item isn't renamed).
        ui.gated_ui(
            crate::unavailable::unless(false, "The Render item can't be renamed"),
            |ui| {
                let mut name = "Render".to_owned();
                crate::form::rows::text_d(ui, "Name", &mut name, "Render", "The Render item");
            },
        );
        ui.gated_ui(
            crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
            |ui| {
                if tab == SETTINGS_TAB {
                    settings_tab(ui, &mut s, &defaults, &mut note);
                } else if tab == GI_TAB {
                    global_illumination_tab(ui, &mut s, &defaults, &mut note);
                } else if tab == PREVIEW_TAB {
                    preview_tab(ui, &mut s, &defaults, &mut note);
                } else {
                    camera_pick = self.frame_tab(ui, &mut s, &defaults, &mut note);
                }
            },
        );
        if let Some(id) = camera_pick {
            self.run(Command::SetRenderCamera(id));
        }
        let Some(response) = changed else {
            if !ui.input(|i| i.pointer.any_down()) {
                self.shading.render_gesture = None;
            }
            return;
        };
        if s == before {
            return;
        }
        // One undo step per drag (RI-04); a typed or picked value is one.
        let dragging = response.dragged() || ui.input(|i| i.pointer.any_down());
        let gesture = match self.shading.render_gesture {
            Some(g) if dragging => g,
            _ => crate::colour_field::next_gesture(),
        };
        self.shading.render_gesture = dragging.then_some(gesture);
        let previous = self.editor.history.undo_fold.replace(gesture);
        self.run(Command::SetRenderSettings(s));
        self.editor.history.undo_fold = previous;
    }

    /// The Frame tab (RI-F01, RI-F03). Returns a newly picked render
    /// camera.
    fn frame_tab(
        &self,
        ui: &mut Ui,
        s: &mut RenderSettings,
        defaults: &RenderSettings,
        note: &mut impl FnMut(Response),
    ) -> Option<u64> {
        let scene = &self.editor.scene;
        let cameras: Vec<(u64, String)> = scene
            .items
            .iter()
            .filter(|i| i.camera.is_some())
            .map(|i| (i.id, i.name.clone()))
            .collect();
        let mut picked = None;
        crate::form::rows::section(ui, "Camera", false, |ui| {
            ui.gated_ui(
                crate::unavailable::unless(!cameras.is_empty(), "The scene has no camera"),
                |ui| {
                    let current = scene.render_camera();
                    let mut chosen = current;
                    let options: Vec<(Option<u64>, &str)> = cameras
                        .iter()
                        .map(|(id, name)| (Some(*id), name.as_str()))
                        .collect();
                    crate::form::rows::status(false);
                    let r = crate::option_row::choice(
                        ui,
                        "render-item-camera",
                        "Render Camera",
                        &mut chosen,
                        &options,
                    );
                    r.response.on_hover_text(
                        "The camera F9 renders through, and the Preview's (Render Camera). \
                         Same as Set as Render Camera in the item list",
                    );
                    if r.inner && chosen != current {
                        picked = chosen;
                    }
                },
            );
        });
        crate::form::rows::section(ui, "Frame Options", false, |ui| {
            let override_on = scene
                .render_camera()
                .and_then(|id| scene.item(id))
                .and_then(|i| i.camera.as_ref())
                .is_some_and(|c| c.resolution.enable);
            let tip = if override_on {
                " The render camera's Override Render Resolution is on, so it wins."
            } else {
                ""
            };
            note(number_d(
                ui,
                "Frame Width",
                &mut s.frame[0],
                defaults.frame[0],
                Unit::Suffix(" px"),
                1..=MAX_FRAME,
                &format!("Width of the rendered picture in pixels.{tip}"),
            ));
            note(number_d(
                ui,
                "Frame Height",
                &mut s.frame[1],
                defaults.frame[1],
                Unit::Suffix(" px"),
                1..=MAX_FRAME,
                &format!("Height of the rendered picture in pixels.{tip}"),
            ));
        });
        picked
    }
}

/// The Settings tab (RI-S03, S04, S06..S08, S10, S11).
fn settings_tab(
    ui: &mut Ui,
    s: &mut RenderSettings,
    defaults: &RenderSettings,
    note: &mut impl FnMut(Response),
) {
    crate::form::rows::section(ui, "Sampling", false, |ui| {
        // Older scenes and scripts may hold more than the panel's range.
        let most = PANEL_MAX_SAMPLES.max(s.samples);
        note(number_d(
            ui,
            "Maximum Quality",
            &mut s.samples,
            defaults.samples,
            Unit::Count,
            1..=most,
            "Most samples each pixel gets. Low is fast and noisy, high is clean and slow",
        ));
        let mut percent = s.adaptive_threshold * 100.;
        let r = number_d(
            ui,
            "Noise Threshold",
            &mut percent,
            defaults.adaptive_threshold * 100.,
            Unit::Percent,
            0.0..=100.0,
            "A pixel stops sampling once its noise is below this. Higher is faster and \
             noisier; 0 % samples every pixel to Maximum Quality",
        );
        if r.changed() {
            s.adaptive_threshold = (percent / 100.).clamp(0., 1.);
        }
        note(r);
    });
    crate::form::rows::section(ui, "Ray Depth", false, |ui| {
        note(number_d(
            ui,
            "Diffuse Depth",
            &mut s.diffuse_depth,
            defaults.diffuse_depth,
            Unit::Count,
            0..=MAX_DEPTH,
            "Most times light bounces off matte surfaces. 0 is direct light only",
        ));
        note(number_d(
            ui,
            "Reflection Depth",
            &mut s.reflection_depth,
            defaults.reflection_depth,
            Unit::Count,
            0..=MAX_DEPTH,
            "Most mirror and glossy bounces",
        ));
        note(number_d(
            ui,
            "Refraction Depth",
            &mut s.refraction_depth,
            defaults.refraction_depth,
            Unit::Count,
            0..=MAX_DEPTH,
            "Most bounces through glass and liquids. Too low turns stacked glass black",
        ));
    });
    // RI-12: the rows the spec keeps under More start folded.
    crate::form::rows::section(ui, "More", true, |ui| {
        note(number_d(
            ui,
            "Maximum Radiance",
            &mut s.max_radiance,
            defaults.max_radiance,
            Unit::Scalar,
            0.0..=1e6,
            "Bounced light brighter than this is clamped, to stop fireflies. 0 is off. \
             What the camera sees directly is never clamped; keep it above 1",
        ));
    });
}

/// The Global Illumination tab's Caustics section (Direct Caustics spec
/// v1). Indirect Caustics moved here from Settings ▸ More (DC-Q05). The
/// photon rows grey out while Direct Caustics is off (UIC-92); Operandi
/// keeps them but doesn't count photons (DC-Q02, provisional).
fn global_illumination_tab(
    ui: &mut Ui,
    s: &mut RenderSettings,
    defaults: &RenderSettings,
    note: &mut impl FnMut(Response),
) {
    crate::form::rows::section(ui, "Caustics", false, |ui| {
        let d = &defaults.direct_caustics;
        let dc = &mut s.direct_caustics;
        // UIC-87: a checkbox row has no left label.
        note(check_d(
            ui,
            "Direct Caustics",
            &mut dc.enabled,
            d.enabled,
            "Lights focused by glass and liquids throw bright patterns on what they \
             land on. Each light's Affect Caustics sets its share; glass needs smooth \
             shading",
        ));
        ui.gated_ui(
            crate::unavailable::unless(dc.enabled, "Direct Caustics is off"),
            |ui| {
                note(number_d(
                    ui,
                    "Total Photons",
                    &mut dc.total_photons,
                    d.total_photons,
                    Unit::Count,
                    1..=MAX_TOTAL_PHOTONS,
                    "Kept with the scene. Operandi traces caustics without photons: \
                     raise Maximum Quality for cleaner caustics",
                ));
                note(number_d(
                    ui,
                    "Local Photons",
                    &mut dc.local_photons,
                    d.local_photons,
                    Unit::Count,
                    1..=MAX_LOCAL_PHOTONS,
                    "Kept with the scene. Operandi traces caustics without photons, so \
                     this doesn't change the render yet",
                ));
                note(check_d(
                    ui,
                    "Walkthrough Mode",
                    &mut dc.walkthrough,
                    d.walkthrough,
                    "Kept with the scene. Operandi works caustics out again every \
                     frame, so this doesn't change the render",
                ));
            },
        );
        let options: Vec<(Caustics, &str)> =
            Caustics::ALL.iter().map(|&c| (c, c.label())).collect();
        note(choice_d(
            ui,
            "Indirect Caustics",
            &mut s.caustics,
            &defaults.caustics,
            &options,
            "Whether bounced light focused by mirrors and glass is traced, such as the \
             sky through a glass. None is faster and cleaner, less real",
        ));
    });
}

/// The Preview tab (RI-P01, RI-P02): when the Preview window stops. F9
/// final renders don't use either row.
fn preview_tab(
    ui: &mut Ui,
    s: &mut RenderSettings,
    defaults: &RenderSettings,
    note: &mut impl FnMut(Response),
) {
    crate::form::rows::section(ui, "Preview", false, |ui| {
        let mut percent = s.preview_convergence * 100.;
        let r = number_d(
            ui,
            "Convergence Target",
            &mut percent,
            defaults.preview_convergence * 100.,
            Unit::Percent,
            0.0..=100.0,
            "The Preview says Finished and stops at this share of its Samples \
             (Preview Settings). Final renders (F9) don't use it",
        );
        if r.changed() {
            s.preview_convergence = (percent / 100.).clamp(0., 1.);
        }
        note(r);
        note(number_d(
            ui,
            "Max Render Time",
            &mut s.preview_time_limit,
            defaults.preview_time_limit,
            Unit::Suffix(" min"),
            0.0..=operandi_core::render_settings::MAX_PREVIEW_MINUTES,
            "The Preview stops after this many minutes and keeps what it has. 0 is no \
             limit. Final renders (F9) don't use it",
        ));
    });
}
