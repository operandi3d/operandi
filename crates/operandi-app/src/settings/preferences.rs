//! Application preferences (Cmd+,): mouse navigation presets and display
//! options. Stored per user beside the input map, never in scenes or history.
use crate::input::NavigationKind;
use crate::prefs_file::PrefsFile;
use crate::unavailable::Gate;
use crate::{color_theme, option_row, theme};
use eframe::egui::{self, Color32, Id, Modifiers, PointerButton};
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

/// Which mouse chords orbit, pan and zoom the viewport. The scroll wheel zooms
/// in every preset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigationPreset {
    /// Operandi's own Alt + left-button chords.
    #[default]
    Operandi,
    /// Middle button navigates without Alt.
    Blender,
    /// Alt with the left, middle or right button.
    Maya,
}
impl NavigationPreset {
    pub const ALL: [Self; 3] = [Self::Operandi, Self::Blender, Self::Maya];
    pub fn label(self) -> &'static str {
        match self {
            Self::Operandi => "Operandi (native)",
            Self::Blender => "Blender style",
            Self::Maya => "Maya style",
        }
    }
    /// Gesture rows for the Preferences window: (action, chord).
    pub fn chords(self) -> [(&'static str, &'static str); 3] {
        match self {
            Self::Operandi => [
                ("Orbit", "Alt + Left drag"),
                ("Pan", "Alt + Shift + Left drag"),
                ("Zoom", "Alt + Ctrl + Left drag"),
            ],
            Self::Blender => [
                ("Orbit", "Middle drag"),
                ("Pan", "Shift + Middle drag"),
                ("Zoom", "Ctrl + Middle drag"),
            ],
            Self::Maya => [
                ("Orbit", "Alt + Left drag"),
                ("Pan", "Alt + Middle drag"),
                ("Zoom", "Alt + Right drag"),
            ],
        }
    }
    /// One line naming this preset's gestures (View > Navigation Help).
    pub fn help(self) -> String {
        let mut parts: Vec<String> = self
            .chords()
            .iter()
            .map(|(action, chord)| format!("{action} {chord}"))
            .collect();
        if self == Self::Operandi {
            parts.extend(
                [
                    "Roll Alt + Middle drag",
                    "Look Around Alt + Right drag",
                    "Pan Up/Down Alt + Shift + Right drag",
                    "Box Zoom Alt + Ctrl + Right drag",
                ]
                .map(String::from),
            );
        }
        parts.push("Wheel Zoom".into());
        parts.join(" · ")
    }
    /// The navigation a viewport press starts, or None when the press belongs
    /// to selection or the active tool.
    pub fn navigation(self, button: PointerButton, mods: Modifiers) -> Option<NavigationKind> {
        match self {
            // NAV-01..07: left orbits/pans/zooms; middle rolls; right
            // looks around, pans up/down or box-zooms (Cmd counts as Ctrl on Mac).
            Self::Operandi => match button {
                PointerButton::Primary => crate::input::navigation(mods),
                PointerButton::Middle if mods.alt && !mods.shift && !mods.ctrl => {
                    Some(NavigationKind::Roll)
                }
                PointerButton::Secondary if mods.alt => Some(if mods.shift {
                    NavigationKind::PanVertical
                } else if mods.ctrl || mods.mac_cmd {
                    NavigationKind::BoxZoom
                } else {
                    NavigationKind::Look
                }),
                _ => None,
            },
            Self::Blender => {
                (button == PointerButton::Middle && !mods.alt).then_some(if mods.shift {
                    NavigationKind::Pan
                } else if mods.ctrl || mods.mac_cmd {
                    NavigationKind::Zoom
                } else {
                    NavigationKind::Orbit
                })
            }
            Self::Maya if mods.alt => match button {
                PointerButton::Primary => Some(NavigationKind::Orbit),
                PointerButton::Middle => Some(NavigationKind::Pan),
                PointerButton::Secondary => Some(NavigationKind::Zoom),
                _ => None,
            },
            Self::Maya => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputPreferences {
    version: u8,
    pub navigation: NavigationPreset,
    /// Input > Remapping; missing in files saved before it existed.
    #[serde(default)]
    pub options: crate::navigation_preferences::NavigationOptions,
    /// Input > Pen Tablet (pen tablet plan): the pressure curve, applied
    /// once where pen samples arrive (`tablet::set_curve`).
    #[serde(default)]
    pub pen_pressure_curve: crate::tablet::PressureCurve,
    /// What the pen's eraser end does in a sculpt stroke.
    #[serde(default)]
    pub pen_eraser_action: crate::tablet::EraserAction,
    /// SC-45: pressure also sets the brush size (strength always follows
    /// it). Off by default.
    #[serde(default)]
    pub pen_pressure_size: bool,
    /// MN-02.8: hold a tool's key to use it briefly (key toggles). On by
    /// default; files from before it load it on.
    #[serde(default = "key_toggles_default")]
    pub key_toggles: bool,
    /// The one-time "Try Trackball Rotation?" offer has been shown
    /// (trackball_offer.rs). Files from before it load it unshown.
    #[serde(default)]
    pub trackball_offered: bool,
}
fn key_toggles_default() -> bool {
    true
}
impl Default for InputPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            navigation: NavigationPreset::default(),
            options: Default::default(),
            pen_pressure_curve: Default::default(),
            pen_eraser_action: Default::default(),
            pen_pressure_size: false,
            key_toggles: true,
            trackball_offered: false,
        }
    }
}
impl crate::prefs_file::PrefsFile for InputPreferences {
    const INVALID: &'static str = "Invalid input preference";
    fn valid(&self) -> bool {
        self.version == 1 && self.options.valid()
    }
    /// Saved as given: input preferences have never been checked on save.
    fn save(&self, path: &Path) -> io::Result<()> {
        crate::prefs_file::write_json(self, path)
    }
}

/// Preferences window pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Input,
    /// Input > Remapping.
    Remapping,
    /// Input > Selection.
    Selection,
    /// Input > Distance Units.
    DistanceUnits,
    /// Input > Grid and Work Plane.
    Grid,
    /// Input > 3D Mouse.
    SpaceMouse,
    Display,
    Colors,
    /// Display > Tool Handles.
    ToolHandles,
    /// Display > 3D Information Overlays.
    Overlays,
    /// Defaults > Tool Settings.
    ToolSettings,
    /// Defaults > Scene Save/Export.
    FileInfo,
    /// Defaults > Application (undo).
    Application,
    /// Defaults > Auto-Save.
    AutoSave,
    /// Defaults > Mesh Items.
    MeshItems,
    /// Defaults > UI.
    Ui,
    /// Rendering.
    Rendering,
    /// File I/O.
    FileIo,
    /// Images & Painting (painting builds, PNT-214).
    #[cfg(feature = "painting")]
    ImagesPainting,
    /// Defaults > Default Tools (painting builds, PNT-214).
    #[cfg(feature = "painting")]
    DefaultTools,
}
impl Page {
    #[cfg(feature = "painting")]
    pub const ALL: [Self; 20] = [
        Self::Input,
        Self::Remapping,
        Self::Selection,
        Self::DistanceUnits,
        Self::Grid,
        Self::SpaceMouse,
        Self::Display,
        Self::Colors,
        Self::ToolHandles,
        Self::Overlays,
        Self::ToolSettings,
        Self::FileInfo,
        Self::Application,
        Self::AutoSave,
        Self::MeshItems,
        Self::Ui,
        Self::Rendering,
        Self::FileIo,
        Self::ImagesPainting,
        Self::DefaultTools,
    ];
    #[cfg(not(feature = "painting"))]
    pub const ALL: [Self; 18] = [
        Self::Input,
        Self::Remapping,
        Self::Selection,
        Self::DistanceUnits,
        Self::Grid,
        Self::SpaceMouse,
        Self::Display,
        Self::Colors,
        Self::ToolHandles,
        Self::Overlays,
        Self::ToolSettings,
        Self::FileInfo,
        Self::Application,
        Self::AutoSave,
        Self::MeshItems,
        Self::Ui,
        Self::Rendering,
        Self::FileIo,
    ];
}

/// The Preferences window's own state (not saved).
#[derive(Default)]
pub struct PreferencesUi {
    pub page: Page,
    /// The scheme last loaded or saved (also kept in the colour file).
    pub scheme: Option<color_theme::Scheme>,
    /// That scheme's colours, to tell whether the user has edited since.
    scheme_colors: Option<color_theme::Overrides>,
    /// Saved scheme names, read when the Colors page opens or a scheme saves.
    schemes: Option<Vec<String>>,
    /// The name being typed for Save to Scheme, while that row is open.
    save_name: Option<String>,
    color_status: Option<String>,
    /// Reset Preferences… is asking for confirmation.
    pub confirm_reset: bool,
}

impl super::Operandi {
    pub(super) fn open_preferences(&mut self) {
        self.show_preferences = true;
        self.preferences_ui.schemes = None;
        self.preferences_ui.color_status = None;
    }
    /// Settings apply and save as they change; Close or Escape dismisses.
    pub(super) fn preferences_window(&mut self, ctx: &egui::Context) {
        if !self.show_preferences {
            return;
        }
        self.frame_ctx = Some(ctx.clone());
        let mut open_input_map = false;
        let mut close = false;
        crate::dialog_place::modal(ctx, Id::new("Preferences")).show(ctx, |ui| {
            ui.set_width(640.);
            ui.heading("Preferences");
            ui.separator();
            // A fixed-size body: the window must never grow past the app
            // window (a vertical divider or a scroll list would otherwise
            // stretch to fill all the height the modal offers).
            let height = (ctx.content_rect().height() - 160.).clamp(200., 460.);
            let top_down = egui::Layout::top_down(egui::Align::Min);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(egui::vec2(120., height), top_down, |ui| {
                    ui.set_min_size(egui::vec2(120., height));
                    let page = &mut self.preferences_ui.page;
                    ui.selectable_value(page, Page::Input, "Input");
                    ui.indent("preferences-input", |ui| {
                        ui.selectable_value(page, Page::Remapping, "Remapping");
                        ui.selectable_value(page, Page::Selection, "Selection");
                        ui.selectable_value(page, Page::DistanceUnits, "Distance Units");
                        ui.selectable_value(page, Page::Grid, "Grid and Work Plane");
                        ui.selectable_value(page, Page::SpaceMouse, "3D Mouse");
                    });
                    ui.selectable_value(page, Page::Display, "Display");
                    ui.indent("preferences-display", |ui| {
                        ui.selectable_value(page, Page::Colors, "Colors");
                        ui.selectable_value(page, Page::ToolHandles, "Tool Handles");
                        ui.selectable_value(page, Page::Overlays, "3D Information Overlays");
                    });
                    ui.label("Defaults");
                    ui.indent("preferences-defaults", |ui| {
                        ui.selectable_value(page, Page::Application, "Application");
                        ui.selectable_value(page, Page::AutoSave, "Auto-Save");
                        ui.selectable_value(page, Page::MeshItems, "Mesh Items");
                        ui.selectable_value(page, Page::Ui, "UI");
                        ui.selectable_value(page, Page::ToolSettings, "Tool Settings");
                        ui.selectable_value(page, Page::FileInfo, "Scene Save/Export");
                        #[cfg(feature = "painting")]
                        ui.selectable_value(page, Page::DefaultTools, "Default Tools");
                    });
                    ui.selectable_value(page, Page::FileIo, "File I/O");
                    ui.selectable_value(page, Page::Rendering, "Rendering");
                    #[cfg(feature = "painting")]
                    ui.selectable_value(page, Page::ImagesPainting, "Images & Painting");
                });
                let (divider, _) =
                    ui.allocate_exact_size(egui::vec2(9., height), egui::Sense::hover());
                ui.painter().vline(
                    divider.center().x,
                    divider.y_range(),
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
                ui.allocate_ui_with_layout(egui::vec2(500., height), top_down, |ui| {
                    ui.set_min_size(egui::vec2(500., height));
                    ui.set_max_size(egui::vec2(500., height));
                    let page_name = format!("preferences/{:?}", self.preferences_ui.page);
                    // A page's rows share one label edge (UIC-111).
                    // Row edits run pref.value where the page has entries.
                    let page = self.preferences_ui.page;
                    let before = self.pref_snapshot(page);
                    crate::form::rows::scope(ui, &page_name, |ui| match self.preferences_ui.page {
                        Page::Input => open_input_map = self.input_page(ui),
                        Page::Remapping => self.remapping_page(ui),
                        Page::Selection => self.selection_page(ui),
                        Page::DistanceUnits => self.distance_units_page(ui),
                        Page::Grid => self.grid_page(ui),
                        Page::SpaceMouse => self.space_mouse_page(ui),
                        Page::Display => self.display_page(ui),
                        Page::Colors => self.colors_page(ui),
                        Page::ToolHandles => self.tool_handles_page(ui),
                        Page::Overlays => self.overlays_page(ui),
                        Page::ToolSettings => self.tool_settings_page(ui),
                        Page::FileInfo => self.file_info_page(ui),
                        Page::Application => self.application_page(ui),
                        Page::AutoSave => self.autosave_page(ui),
                        Page::MeshItems => self.mesh_items_page(ui),
                        Page::Ui => self.ui_page(ui),
                        Page::Rendering => self.rendering_page(ui),
                        Page::FileIo => self.file_io_page(ui),
                        #[cfg(feature = "painting")]
                        Page::ImagesPainting => self.images_painting_page(ui),
                        #[cfg(feature = "painting")]
                        Page::DefaultTools => self.default_tools_page(ui),
                    });
                    self.pref_rows_through_commands(before);
                });
            });
            ui.separator();
            let mut reset = false;
            ui.horizontal(|ui| {
                close = ui.button("Close").clicked()
                    || (self.preferences_ui.save_name.is_none()
                        && !std::mem::take(&mut self.space_mouse.keys_taken)
                        && ui.input(|i| i.key_pressed(egui::Key::Escape)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    reset = ui
                        .button("Reset Panel")
                        .on_hover_text("This page's settings back to their factory values")
                        .clicked();
                });
            });
            if reset {
                self.reset_page(self.preferences_ui.page);
            }
        });
        if open_input_map {
            self.show_preferences = false;
            self.open_input_map();
        } else if close {
            self.show_preferences = false;
            self.preferences_ui.save_name = None;
        }
    }
    /// Returns true when Input Map… was pressed.
    fn input_page(&mut self, ui: &mut egui::Ui) -> bool {
        use crate::form::rows;
        let before = self.input_preferences.navigation;
        let mut open_map = false;
        rows::section(ui, "Input", false, |ui| {
            let presets: Vec<_> = NavigationPreset::ALL
                .iter()
                .map(|&p| (p, p.label()))
                .collect();
            rows::choice_d(
                ui,
                "Mouse Navigation",
                &mut self.input_preferences.navigation,
                &NavigationPreset::default(),
                &presets,
                "Which mouse chords orbit, pan and zoom the views",
            );
            // The chosen preset's chords, read-only.
            for (action, chord) in self.input_preferences.navigation.chords() {
                rows::value(ui, action, chord);
            }
            rows::value(ui, "Zoom", "Scroll wheel");
            open_map = rows::button(ui, "Input Map…")
                .on_hover_text("Edit the keyboard shortcuts")
                .clicked();
        });
        let pen_before = self.input_preferences;
        rows::section(ui, "Pen Tablet", false, |ui| {
            let curves: Vec<_> = crate::tablet::PressureCurve::ALL
                .iter()
                .map(|&c| (c, c.label()))
                .collect();
            rows::choice_d(
                ui,
                "Pressure Curve",
                &mut self.input_preferences.pen_pressure_curve,
                &crate::tablet::PressureCurve::default(),
                &curves,
                "How pen pressure maps to effect: Soft needs a lighter touch, Firm a harder one",
            );
            let erasers: Vec<_> = crate::tablet::EraserAction::ALL
                .iter()
                .map(|&e| (e, e.label()))
                .collect();
            rows::choice_d(
                ui,
                "Eraser End",
                &mut self.input_preferences.pen_eraser_action,
                &crate::tablet::EraserAction::default(),
                &erasers,
                "What a sculpt stroke with the pen's eraser end does: Invert is like holding Ctrl, Smooth like holding Shift",
            );
            rows::check_d(
                ui,
                "Pressure Sets Brush Size",
                &mut self.input_preferences.pen_pressure_size,
                false,
                "Sculpt brushes get smaller as you press lighter. Pressure always sets strength",
            );
        });
        rows::section(ui, "Keys", false, |ui| {
            rows::check_d(
                ui,
                "Key Toggles",
                &mut self.input_preferences.key_toggles,
                true,
                "Hold a tool's key and drag to use it briefly; letting go of the key drops it. A quick tap still turns the tool on",
            );
        });
        if (self.input_preferences.navigation != before || self.input_preferences != pen_before)
            && let Err(error) = self
                .input_preferences
                .save(&self.environment.paths().input_preferences())
        {
            self.status = format!("Navigation changed; could not save preference: {error}");
        }
        open_map
    }
    /// Display > Colors: every themable colour, grouped, editable live.
    fn colors_page(&mut self, ui: &mut egui::Ui) {
        let paths = self.environment.paths();
        let dir = paths.color_schemes();
        let state = &mut self.preferences_ui;
        let schemes = state
            .schemes
            .get_or_insert_with(|| color_theme::schemes(&dir))
            .clone();
        let current = color_theme::current();
        let mut changed = false;
        let mut status = None;

        use color_theme::Scheme;
        use theme::presets::Preset;

        let selected = state.scheme.as_ref();
        let shown = selected.map_or(Preset::Steel.name(), Scheme::name);
        let edited = match selected {
            None => !current.is_empty(),
            Some(_) => state
                .scheme_colors
                .as_ref()
                .is_some_and(|colors| *colors != current),
        };
        let shown = if edited {
            format!("{shown} (edited)")
        } else {
            shown.to_owned()
        };
        let picked = option_row::dropdown(ui, "color-scheme", "Scheme", &shown, |ui| {
            let mut picked = None;
            let mut add = |ui: &mut egui::Ui, scheme: Scheme| {
                let active = selected == Some(&scheme)
                    || (selected.is_none()
                        && current.is_empty()
                        && scheme == Scheme::BuiltIn(Preset::Steel));
                let name = egui::RichText::new(scheme.name());
                let name = if active {
                    name.color(theme::on_orange())
                } else {
                    name
                };
                if ui.selectable_label(active, name).clicked() {
                    picked = Some(scheme);
                    ui.close();
                }
            };
            for preset in Preset::ALL {
                add(ui, Scheme::BuiltIn(preset));
            }
            if !schemes.is_empty() {
                ui.separator();
            }
            for name in &schemes {
                add(ui, Scheme::Saved(name.clone()));
            }
            picked
        })
        .inner;
        if let Some(scheme) = picked {
            match scheme.colors(&dir) {
                Some(colors) => {
                    state.select_scheme(scheme, colors);
                    changed = true;
                }
                None => status = Some(format!("Could not read the scheme \"{}\".", scheme.name())),
            }
        }
        option_row::row(ui, "", |ui| {
            // UIC-03, UIC-04, UIC-22: label-only actions with tooltips.
            if ui
                .button("Save to Scheme…")
                .on_hover_text("Save to Scheme…\nSave the current colors under a name")
                .clicked()
            {
                state.save_name = Some(match &state.scheme {
                    Some(Scheme::Saved(name)) => name.clone(),
                    _ => String::new(),
                });
            }
            if ui
                .gated(
                    crate::unavailable::unless(
                        state.scheme.is_some() || !current.is_empty(),
                        "Every colour is already the default",
                    ),
                    egui::Button::new("Reset All"),
                )
                .on_hover_text("Every colour back to the built-in default")
                .clicked()
            {
                theme::set_overrides(&color_theme::Overrides::new());
                state.scheme = None;
                state.scheme_colors = None;
                changed = true;
            }
        });
        // UIC-116: each file action fills its own field-column row.
        let import = option_row::button(ui, "Import…")
            .on_hover_text("Import…\nLoad a color scheme from a JSON file")
            .clicked();
        let export = option_row::button(ui, "Export…")
            .on_hover_text("Export…\nWrite all current colors to a JSON file")
            .clicked();
        if let Some(name) = &mut state.save_name {
            let mut save = false;
            let mut cancel = false;
            option_row::row(ui, "Scheme Name", |ui| {
                let field = ui.add(egui::TextEdit::singleline(name).desired_width(180.));
                crate::numeric_tab::register(ui, &field, Some(name.as_str()));
                if !field.has_focus() && name.is_empty() {
                    field.request_focus();
                }
                let clean = color_theme::clean_name(name);
                let exists = clean.as_ref().is_some_and(|n| schemes.contains(n));
                save = ui
                    .gated(
                        crate::unavailable::unless(clean.is_some(), "Type a scheme name first"),
                        egui::Button::new(if exists { "Replace" } else { "Save" }),
                    )
                    .clicked()
                    || (clean.is_some()
                        && field.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                cancel =
                    ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape));
            });
            if save && let Some(clean) = color_theme::clean_name(name) {
                let path = color_theme::scheme_path(&dir, &clean);
                let colors = color_theme::current();
                match color_theme::ColorFile::new(&colors, None).save(&path) {
                    Ok(()) => {
                        state.select_scheme(Scheme::Saved(clean.clone()), colors);
                        state.schemes = None;
                        changed = true;
                        status = Some(format!("Saved the colour scheme \"{clean}\"."));
                    }
                    Err(error) => status = Some(format!("Could not save the scheme: {error}")),
                }
                state.save_name = None;
            } else if cancel {
                state.save_name = None;
            }
        }
        // UIC-113: a dropdown row. Every rendered icon has a dark-outline
        // and a light-outline set (icon studio); Automatic follows Panel.
        let panel = theme::panel();
        let choices: Vec<(crate::rendered_icons::OutlineChoice, String)> =
            crate::rendered_icons::OutlineChoice::ALL
                .iter()
                .map(|&c| (c, c.label(panel)))
                .collect();
        let choices: Vec<_> = choices.iter().map(|(c, l)| (*c, l.as_str())).collect();
        let outline_before = self.ui_preferences.rendered_icon_outline;
        crate::form::rows::choice_d(
            ui,
            "Rendered Icon Outline",
            &mut self.ui_preferences.rendered_icon_outline,
            &Default::default(),
            &choices,
            "Rendered icons come with a dark outline for light colours and a light outline for dark ones. Automatic picks by the Panel colour",
        );
        let state = &mut self.preferences_ui;
        if self.ui_preferences.rendered_icon_outline != outline_before {
            crate::rendered_icons::set_choice(self.ui_preferences.rendered_icon_outline);
            if let Err(error) = self.ui_preferences.save(&paths.ui_preferences()) {
                status = Some(format!("Could not save the icon outline: {error}"));
            }
        }
        if let Some(message) = status.as_ref().or(state.color_status.as_ref()) {
            ui.label(message);
        }
        ui.separator();

        let current = color_theme::current();
        egui::ScrollArea::vertical()
            .id_salt("preferences-colors")
            .max_height(ui.available_height())
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // One section per colour group (UIC-105).
                let mut groups: Vec<&str> = Vec::new();
                for entry in theme::COLORS {
                    if !groups.contains(&entry.group) {
                        groups.push(entry.group);
                    }
                }
                for group in groups {
                    crate::form::rows::section(ui, group, false, |ui| {
                        // Reset to Defaults for this section only.
                        let edited = theme::COLORS
                            .iter()
                            .any(|e| e.group == group && current.contains_key(&e.id));
                        option_row::row(ui, "", |ui| {
                            if ui
                                .gated(
                                    crate::unavailable::unless(
                                        edited,
                                        "This section is already the default",
                                    ),
                                    egui::Button::new("Reset Section"),
                                )
                                .on_hover_text("This section's colours back to the default")
                                .clicked()
                            {
                                for entry in theme::COLORS.iter().filter(|e| e.group == group) {
                                    theme::set_override(entry.id, None);
                                }
                                changed = true;
                            }
                        });
                        for entry in theme::COLORS.iter().filter(|e| e.group == group) {
                            changed |= color_row(ui, entry);
                        }
                    });
                }
            });

        if changed {
            let file = color_theme::ColorFile::for_preferences(state.scheme.as_ref());
            if let Err(error) = file.save(&paths.color_theme()) {
                status = Some(format!("Colours changed; could not save them: {error}"));
            }
            ui.ctx().request_repaint();
        }
        let folder = (import || export).then(|| crate::file_chooser::split(paths.open_dialog()).0);
        if let Some(status) = status {
            self.color_status(status);
        }

        // Use the app's shared native Open/Save sheets (UIC-80, UIC-98).
        use crate::file_chooser::{Choose, Purpose};
        if let Some(folder) = folder {
            if import {
                self.choose(
                    Choose::LoadColorScheme { folder },
                    Purpose::ImportColorScheme,
                );
                return;
            }
            let name = format!(
                "{}.json",
                self.preferences_ui
                    .scheme
                    .as_ref()
                    .map_or("Colors", Scheme::name)
            );
            self.choose(
                Choose::SaveColorScheme { folder, name },
                Purpose::ExportColorScheme,
            );
        }
    }

    fn color_status(&mut self, message: String) {
        self.preferences_ui.color_status = Some(message.clone());
        self.status = message;
    }

    pub(super) fn import_color_scheme(&mut self, source: &std::path::Path) {
        let paths = self.environment.paths();
        let message = match color_theme::import_scheme(source, &paths.color_schemes()) {
            Ok((scheme, colors, count)) => {
                let noun = if count == 1 { "color" } else { "colors" };
                let message = format!("Loaded {count} {noun} into \"{}\".", scheme.name());
                self.preferences_ui.select_scheme(scheme, colors);
                self.preferences_ui.schemes = None;
                self.preferences_ui.save_name = None;
                let file =
                    color_theme::ColorFile::for_preferences(self.preferences_ui.scheme.as_ref());
                match file.save(&paths.color_theme()) {
                    Ok(()) => message,
                    Err(error) => format!("{message} Could not save the active colors: {error}"),
                }
            }
            Err(error) => format!("Could not import the color scheme: {error}"),
        };
        self.color_status(message);
    }

    pub(super) fn export_color_scheme(&mut self, mut path: std::path::PathBuf) {
        if path.extension().is_none() {
            path.set_extension("json");
        }
        let message = match color_theme::export_scheme(&path) {
            Ok(()) => "Exported the current colors.".to_owned(),
            Err(error) => format!("Could not export the color scheme: {error}"),
        };
        self.color_status(message);
    }
}

impl PreferencesUi {
    fn select_scheme(&mut self, scheme: color_theme::Scheme, colors: color_theme::Overrides) {
        theme::set_overrides(&colors);
        self.scheme = Some(scheme);
        self.scheme_colors = Some(colors);
    }
}

impl super::Operandi {
    /// Put one page's preferences back to factory values, apply and save
    /// them. Pages that share a file keep the other pages' values.
    pub(super) fn reset_page(&mut self, page: Page) {
        let paths = self.environment.paths().clone();
        let mut errors = Vec::new();
        let mut note = |r: std::io::Result<()>| {
            if let Err(e) = r {
                errors.push(e.to_string());
            }
        };
        match page {
            Page::Input => {
                self.input_preferences.navigation = NavigationPreset::default();
                note(self.input_preferences.save(&paths.input_preferences()));
            }
            Page::Remapping => {
                self.input_preferences.options = Default::default();
                note(self.input_preferences.save(&paths.input_preferences()));
            }
            Page::Selection => {
                self.selection_preferences = Default::default();
                self.lasso_partial = self.selection_preferences.lasso_partial;
                note(
                    self.selection_preferences
                        .save(&paths.selection_preferences()),
                );
            }
            Page::DistanceUnits => {
                let factory = crate::units_preferences::UnitsPreferences::default();
                let u = &mut self.units_preferences;
                u.distance = factory.distance;
                u.rounding = factory.rounding;
                u.fixed_increment = factory.fixed_increment;
                u.color_units = factory.color_units;
                u.factor = factory.factor;
                u.up_axis = factory.up_axis;
                note(self.units_preferences.save(&paths.units_preferences()));
            }
            Page::Grid => {
                let kept = self.units_preferences;
                let u = &mut self.units_preferences;
                *u = Default::default();
                u.distance = kept.distance;
                u.rounding = kept.rounding;
                u.fixed_increment = kept.fixed_increment;
                u.color_units = kept.color_units;
                u.factor = kept.factor;
                u.up_axis = kept.up_axis;
                note(self.units_preferences.save(&paths.units_preferences()));
            }
            Page::Display => {
                let flatness = self.viewport_preferences.flatness;
                self.viewport_preferences = Default::default();
                self.reflatten_views(flatness);
                note(self.viewport_preferences.save(&paths.viewport_display()));
            }
            Page::Colors => {
                self.ui_preferences.rendered_icon_outline = Default::default();
                self.ui_preferences.apply();
                note(self.ui_preferences.save(&paths.ui_preferences()));
                theme::set_overrides(&color_theme::Overrides::new());
                self.preferences_ui.scheme = None;
                self.preferences_ui.scheme_colors = None;
                note(
                    color_theme::ColorFile::new(&color_theme::current(), None)
                        .save(&paths.color_theme()),
                );
            }
            Page::ToolHandles => {
                self.handle_preferences = Default::default();
                let factory = crate::snapping::SnappingPreferences::default();
                self.snapping.inner_range = factory.inner_range;
                self.snapping.outer_range = factory.outer_range;
                note(self.handle_preferences.save(&paths.handle_preferences()));
                note(self.snapping.save(&paths.snapping_preferences()));
            }
            Page::Overlays => {
                self.defaults_preferences.overlays = Default::default();
                note(
                    self.defaults_preferences
                        .save(&paths.defaults_preferences()),
                );
            }
            Page::ToolSettings => {
                let factory = crate::defaults_preferences::DefaultsPreferences::default();
                let d = &mut self.defaults_preferences;
                d.component_center = factory.component_center;
                d.component_axis = factory.component_axis;
                d.item_center = factory.item_center;
                d.item_axis = factory.item_axis;
                d.default_part = factory.default_part;
                note(
                    self.defaults_preferences
                        .save(&paths.defaults_preferences()),
                );
            }
            Page::FileInfo => {
                self.defaults_preferences.author.clear();
                self.defaults_preferences.copyright.clear();
                note(
                    self.defaults_preferences
                        .save(&paths.defaults_preferences()),
                );
            }
            Page::Application => {
                let factory = crate::undo_autosave::UndoAutosavePreferences::default();
                let u = &mut self.undo_autosave;
                u.max_undo_level = factory.max_undo_level;
                u.group_selection = factory.group_selection;
                u.max_sub_commands = factory.max_sub_commands;
                let factory = crate::defaults_preferences::DefaultsPreferences::default();
                let d = &mut self.defaults_preferences;
                d.default_material = factory.default_material;
                d.texture_map = factory.texture_map;
                d.index_style = factory.index_style;
                d.default_scene = factory.default_scene;
                d.os_color_picker = factory.os_color_picker;
                note(self.undo_autosave.save(&paths.undo_autosave()));
                note(
                    self.defaults_preferences
                        .save(&paths.defaults_preferences()),
                );
                note(self.reset_update_preferences());
                note(self.reset_online_preferences());
            }
            Page::AutoSave => {
                let factory = crate::undo_autosave::UndoAutosavePreferences::default();
                let u = &mut self.undo_autosave;
                u.autosave = factory.autosave;
                u.interval_minutes = factory.interval_minutes;
                u.backup_dir = factory.backup_dir;
                u.revisions = factory.revisions;
                note(self.undo_autosave.save(&paths.undo_autosave()));
            }
            Page::MeshItems => {
                self.mesh_items_preferences = Default::default();
                note(
                    self.mesh_items_preferences
                        .save(&paths.mesh_items_preferences()),
                );
            }
            Page::SpaceMouse => {
                self.space_mouse.prefs = Default::default();
                note(
                    self.space_mouse
                        .prefs
                        .save(&paths.space_mouse_preferences()),
                );
            }
            #[cfg(feature = "painting")]
            Page::ImagesPainting | Page::DefaultTools => self.reset_painting_prefs(page),
            Page::FileIo => {
                self.file_io_preferences = Default::default();
                self.file_io_preferences.apply();
                note(self.file_io_preferences.save(&paths.file_io_preferences()));
            }
            Page::Rendering => {
                self.render_preferences = Default::default();
                note(self.render_preferences.save(&paths.render_preferences()));
                let folder_before = self.render.prefs.recent_frames_folder.clone();
                self.render.prefs = Default::default();
                note(self.render.prefs.save(&paths.image_viewer_preferences()));
                if folder_before.is_some() {
                    self.load_render_history();
                }
            }
            Page::Ui => {
                self.ui_preferences = Default::default();
                self.ui_preferences.apply();
                note(self.ui_preferences.save(&paths.ui_preferences()));
            }
        }
        self.apply_preferences();
        self.status = if errors.is_empty() {
            "Preferences page reset".into()
        } else {
            format!("Reset; could not save: {}", errors.join("; "))
        };
    }
    /// Every page back to factory values (Edit > Reset Preferences…). The
    /// input map, saved colour schemes and viewport presets are kept.
    pub(super) fn reset_all_preferences(&mut self) {
        for page in Page::ALL {
            self.reset_page(page);
        }
        let factory_snapping = crate::snapping::SnappingPreferences::default();
        self.snapping = factory_snapping;
        let _ = self
            .snapping
            .save(&self.environment.paths().snapping_preferences());
        self.apply_preferences();
        self.status = "Preferences reset to factory values".into();
    }
    /// Make the live state follow the preferences after a reset.
    fn apply_preferences(&mut self) {
        if let Some(ctx) = &self.frame_ctx {
            self.input_preferences.options.apply_style(ctx);
            ctx.request_repaint();
        }
        crate::dialog_place::set_under_mouse(self.input_preferences.options.dialogs_under_mouse);
        self.units_preferences.apply();
        self.file_io_preferences.apply();
        self.ui_preferences.apply();
        self.apply_user_defaults();
        self.editor.history.settings = self.undo_autosave.history();
        self.clear_fields();
    }
    /// Reset Preferences… confirmation.
    pub(super) fn reset_preferences_dialog(&mut self, ctx: &egui::Context) {
        if !self.preferences_ui.confirm_reset {
            return;
        }
        self.frame_ctx = Some(ctx.clone());
        // UIC-97: a confirmation in the shared dialog frame.
        let answer = match self.show_confirm(
            ctx,
            "Reset Preferences",
            "Every preference goes back to its factory value. Your keyboard \
             shortcuts, saved colour schemes and viewport presets are kept.",
            &[],
        ) {
            crate::form::Outcome::Ok => Some(true),
            crate::form::Outcome::Cancel => Some(false),
            _ => None,
        };
        if let Some(reset) = answer {
            self.preferences_ui.confirm_reset = false;
            if reset {
                self.reset_all_preferences();
            }
        }
    }
}

/// One colour: right-aligned name, a swatch that opens a picker, R G B from
/// 0 to 1, and a reset to the default. Returns true when it changed.
fn color_row(ui: &mut egui::Ui, entry: &theme::ColorEntry) -> bool {
    let id = entry.id;
    let overridden = theme::override_of(id).is_some();
    let before = theme::color(id);
    let to_rgb = |c: Color32| [c.r(), c.g(), c.b()].map(|v| f64::from(v) / 255.);
    let mut rgb = to_rgb(before);
    let (edit, reset) = ui
        .push_id(entry.key, |ui| {
            // UIC-110: filled for an explicit override.
            crate::form::rows::status(overridden);
            option_row::row(ui, entry.label, |ui| {
                // Theme colours are sRGB already: the shared field shows them
                // as they are, in the Color Unit System.
                let default = to_rgb(id.default_color());
                let width = (ui.available_width() - 24.).max(60.);
                let field = ui
                    .allocate_ui(egui::vec2(width, ui.spacing().interact_size.y), |ui| {
                        crate::colour_field::show(
                            ui,
                            &mut rgb,
                            crate::colour_field::Options::theme().with_default(default, overridden),
                        )
                    })
                    .inner;
                let reset = ui
                    .gated(
                        crate::unavailable::unless(overridden, "Already the default colour"),
                        egui::Button::new("↺").small(),
                    )
                    .on_hover_text("Reset to default")
                    .clicked();
                (field.changed(), reset || field.reset)
            })
            .inner
        })
        .inner;
    if reset {
        theme::set_override(id, None);
        return true;
    }
    let [r, g, b] = rgb.map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
    let color = Color32::from_rgb(r, g, b);
    if [r, g, b] != [before.r(), before.g(), before.b()]
        || (edit && !overridden && id.follows_accent_by_default())
    {
        theme::set_override(id, Some(color));
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NavigationKind::*;
    use NavigationPreset::*;
    use PointerButton::*;
    use std::fs;
    fn mods(alt: bool, shift: bool, ctrl: bool) -> Modifiers {
        Modifiers {
            alt,
            shift,
            ctrl,
            ..Modifiers::NONE
        }
    }
    #[test]
    fn operandi_preset_keeps_the_alt_left_button_chords() {
        assert_eq!(
            Operandi.navigation(Primary, mods(true, false, false)),
            Some(Orbit)
        );
        assert_eq!(
            Operandi.navigation(Primary, mods(true, true, false)),
            Some(Pan)
        );
        assert_eq!(
            Operandi.navigation(Primary, mods(true, false, true)),
            Some(Zoom)
        );
        assert_eq!(Operandi.navigation(Primary, Modifiers::NONE), None);
        // Middle and right buttons stay with lasso selection; with Alt they
        // roll and look around (NAV-02/03).
        assert_eq!(Operandi.navigation(Middle, Modifiers::NONE), None);
        assert_eq!(Operandi.navigation(Secondary, Modifiers::NONE), None);
        assert_eq!(
            Operandi.navigation(Secondary, mods(true, false, false)),
            Some(NavigationKind::Look)
        );
    }
    #[test]
    fn blender_preset_navigates_with_the_middle_button() {
        assert_eq!(Blender.navigation(Middle, Modifiers::NONE), Some(Orbit));
        assert_eq!(
            Blender.navigation(Middle, mods(false, true, false)),
            Some(Pan)
        );
        assert_eq!(
            Blender.navigation(Middle, mods(false, false, true)),
            Some(Zoom)
        );
        assert_eq!(Blender.navigation(Middle, Modifiers::MAC_CMD), Some(Zoom));
        // Alt + left is free for tools; the right button stays lasso.
        assert_eq!(Blender.navigation(Primary, mods(true, false, false)), None);
        assert_eq!(Blender.navigation(Secondary, Modifiers::NONE), None);
    }
    #[test]
    fn maya_preset_uses_alt_with_each_button() {
        let alt = mods(true, false, false);
        assert_eq!(Maya.navigation(Primary, alt), Some(Orbit));
        assert_eq!(Maya.navigation(Middle, alt), Some(Pan));
        assert_eq!(Maya.navigation(Secondary, alt), Some(Zoom));
        assert_eq!(Maya.navigation(Primary, Modifiers::NONE), None);
        assert_eq!(Maya.navigation(Middle, Modifiers::NONE), None);
    }
    #[test]
    fn input_preferences_roundtrip_and_fall_back_to_defaults() {
        let root = std::env::temp_dir().join(format!(
            "operandi-input-prefs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("input-preferences.json");
        assert_eq!(InputPreferences::load(&path), InputPreferences::default());
        for navigation in NavigationPreset::ALL {
            let p = InputPreferences {
                navigation,
                ..Default::default()
            };
            p.save(&path).unwrap();
            assert_eq!(InputPreferences::load(&path), p);
        }
        // Pen Tablet settings save, and files from before them load with
        // the defaults.
        let pen = InputPreferences {
            pen_pressure_curve: crate::tablet::PressureCurve::Firm,
            pen_eraser_action: crate::tablet::EraserAction::Smooth,
            pen_pressure_size: true,
            ..Default::default()
        };
        pen.save(&path).unwrap();
        assert_eq!(InputPreferences::load(&path), pen);
        fs::write(&path, r#"{"version":1,"navigation":"Maya"}"#).unwrap();
        let old = InputPreferences::load(&path);
        assert_eq!(old.navigation, NavigationPreset::Maya);
        assert_eq!(old.pen_pressure_curve, crate::tablet::PressureCurve::Linear);
        assert_eq!(old.pen_eraser_action, crate::tablet::EraserAction::Invert);
        assert!(!old.pen_pressure_size);
        // MN-02.8: files from before Key Toggles load it on.
        assert!(old.key_toggles);
        for invalid in [
            r#"{"version":2,"navigation":"Maya"}"#,
            r#"{"version":1,"navigation":"Houdini"}"#,
            "invalid",
        ] {
            fs::write(&path, invalid).unwrap();
            assert_eq!(InputPreferences::load(&path), InputPreferences::default());
        }
        fs::remove_dir_all(root).unwrap();
    }

    // App routes: synthetic egui events through the real input handling.
    use crate::tests::draw;
    use crate::{Command, Operandi};
    use eframe::egui::{Context, Event, Key, Pos2, vec2};
    /// A full key press, so the next press of the same key is not a repeat.
    fn key(key: Key, modifiers: Modifiers) -> Vec<Event> {
        [true, false]
            .map(|pressed| Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            })
            .to_vec()
    }
    const CMD: Modifiers = crate::tests::PLATFORM_CMD;
    #[test]
    fn cmd_z_undoes_and_shift_cmd_z_redoes() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        let before = a.editor.scene.clone();
        a.run(Command::NewMesh);
        let after = a.editor.scene.clone();
        assert_ne!(before, after);
        draw(&mut a, &ctx, vec2(1440., 900.), key(Key::Z, CMD));
        assert_eq!(a.editor.scene, before);
        let shift_cmd = Modifiers { shift: true, ..CMD };
        draw(&mut a, &ctx, vec2(1440., 900.), key(Key::Z, shift_cmd));
        assert_eq!(a.editor.scene, after);
        // Plain Z stays Edge Extend's chord, not Undo.
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            key(Key::Z, Modifiers::NONE),
        );
        assert_eq!(a.editor.scene, after);
    }
    #[test]
    fn cmd_comma_opens_preferences_and_escape_closes_them() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        draw(&mut a, &ctx, vec2(1440., 900.), key(Key::Comma, CMD));
        assert!(a.show_preferences);
        // Shortcuts are held while the window is open.
        let before = a.editor.scene.clone();
        a.run(Command::NewMesh);
        a.show_preferences = true;
        draw(&mut a, &ctx, vec2(1440., 900.), key(Key::Z, CMD));
        assert_ne!(a.editor.scene, before);
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            key(Key::Escape, Modifiers::NONE),
        );
        assert!(!a.show_preferences);
    }
    #[test]
    fn saved_maps_from_before_the_undo_chords_gain_them_unless_taken() {
        let root = std::env::temp_dir().join(format!(
            "operandi-undo-map-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("input-map.json");
        let load = |json: &str| {
            fs::write(&path, json).unwrap();
            crate::input::load_bindings(&path)
        };
        let old = load(r#"{"Undo":"","Redo":"","Move":"W"}"#);
        assert_eq!(old["Undo"], "Cmd+Z");
        assert_eq!(old["Redo"], "Shift+Cmd+Z");
        assert_eq!(old["Preferences"], "Cmd+,");
        // A map saved after this change keeps an explicitly cleared Undo.
        let cleared = load(r#"{"Undo":"","Redo":"","Preferences":"Cmd+,"}"#);
        assert_eq!(cleared["Undo"], "");
        // An old map that already put Cmd+Z elsewhere keeps it there.
        let taken = load(r#"{"Undo":"","Move":"Cmd+Z","Flip":"Cmd+,"}"#);
        assert_eq!(taken["Move"], "Cmd+Z");
        assert_eq!(taken["Undo"], "");
        assert_eq!(taken["Redo"], "Shift+Cmd+Z");
        assert_eq!(taken["Preferences"], "");
        assert!(crate::input::bindings_valid(
            &crate::input::default_bindings()
        ));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn preferences_window_fits_inside_the_app_window_on_every_page() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        a.open_preferences();
        for size in [vec2(1440., 900.), vec2(1000., 600.)] {
            for page in [Page::Input, Page::Display, Page::Colors] {
                a.preferences_ui.page = page;
                for _ in 0..3 {
                    draw(&mut a, &ctx, size, vec![]);
                }
                let rect = ctx
                    .memory(|m| m.area_rect(Id::new("Preferences")))
                    .expect("Preferences drawn");
                let screen = eframe::egui::Rect::from_min_size(Pos2::ZERO, size);
                assert!(
                    screen.contains_rect(rect),
                    "{page:?} at {size:?}: {rect:?} overflows {screen:?}"
                );
            }
        }
    }
    #[test]
    fn colors_page_applies_live_and_saves_schemes() {
        use crate::theme::{self, ColorId};
        use eframe::egui::Color32;
        let mut a = Operandi::empty();
        let ctx = Context::default();
        theme::install(&ctx);
        theme::set_overrides(&Default::default());
        a.open_preferences();
        a.preferences_ui.page = Page::Colors;
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        // A changed colour reaches the widgets on the next frame.
        let red = Color32::from_rgb(200, 10, 10);
        theme::set_override(ColorId::Panel, Some(red));
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        assert_eq!(ctx.style().visuals.panel_fill, red);
        // Save to Scheme: type a name, press Enter.
        a.preferences_ui.save_name = Some(String::new());
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![Event::Text("Night".into())],
        );
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            key(Key::Enter, Modifiers::NONE),
        );
        assert_eq!(a.preferences_ui.save_name, None);
        assert_eq!(
            a.preferences_ui
                .scheme
                .as_ref()
                .map(color_theme::Scheme::name),
            Some("Night")
        );
        let dir = a.environment.paths().color_schemes();
        assert_eq!(crate::color_theme::schemes(&dir), ["Night"]);
        // Reset back to defaults, then reload the scheme from its file.
        theme::set_overrides(&Default::default());
        assert_eq!(theme::override_of(ColorId::Panel), None);
        let file =
            crate::color_theme::ColorFile::read(&crate::color_theme::scheme_path(&dir, "Night"))
                .unwrap();
        assert_eq!(file.overrides().get(&ColorId::Panel), Some(&red));
        // Escape closes the window again.
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            key(Key::Escape, Modifiers::NONE),
        );
        assert!(!a.show_preferences);
        theme::set_overrides(&Default::default());
    }
    fn middle_drag(a: &mut Operandi, ctx: &Context, mods: Modifiers) {
        let start = a.viewport_rect.center();
        let end = start + vec2(60., 20.);
        let press = |pos: Pos2, pressed| Event::PointerButton {
            pos,
            button: Middle,
            pressed,
            modifiers: mods,
        };
        draw(
            a,
            ctx,
            vec2(1440., 900.),
            vec![
                Event::PointerMoved(start),
                press(start, true),
                Event::PointerMoved(end),
                press(end, false),
            ],
        );
    }
    #[test]
    fn middle_drag_orbits_only_under_the_blender_preset() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let camera = (a.camera.yaw, a.camera.pitch);
        middle_drag(&mut a, &ctx, Modifiers::NONE);
        assert_eq!(
            (a.camera.yaw, a.camera.pitch),
            camera,
            "Operandi: middle is lasso"
        );
        a.input_preferences.navigation = Blender;
        let checkpoints = a.editor.history.undo.len();
        middle_drag(&mut a, &ctx, Modifiers::NONE);
        assert_ne!((a.camera.yaw, a.camera.pitch), camera);
        assert!(a.navigation.is_none());
        assert_eq!(
            a.editor.history.undo.len(),
            checkpoints,
            "navigation records no history"
        );
        let target = a.camera.target;
        middle_drag(&mut a, &ctx, mods(false, true, false));
        assert_ne!(a.camera.target, target, "Shift + middle pans");
        a.input_preferences.navigation = Maya;
        let target = a.camera.target;
        middle_drag(&mut a, &ctx, mods(true, false, false));
        assert_ne!(a.camera.target, target, "Maya: Alt + middle pans");
    }
}
