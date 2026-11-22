//! Environment presets in the app (EP, HDRI stage 6): Save Environment
//! Preset… with its options dialog (the shared dialog, UIC-80/86/87/96/98),
//! and applying a preset as Replace or Add. The file is a cut-down scene
//! file (`.openv`, operandi-core preset.rs); Operandi reads no other
//! app's preset format.
//!
//! Commands (registry thread, 2026-11-12): `preset.saveAs item:ENV
//! filename:PATH lights:BOOL meshes:BOOL` (lights/meshes Operandi-only,
//! default false; the dialog passes them) and `preset.do preset:PATH
//! mode:replace|add` (mode Operandi-only, default replace).
use super::*;
use operandi_core::preset::{ApplyMode, ENVIRONMENT_PRESET_EXTENSION, EnvironmentPresetOptions};

/// The open Save Environment Preset dialog.
pub(super) struct EnvPresetDialog {
    pub(super) form: form::Dialog,
}

/// The dialog's title (UIC-81: the command's name without "…").
pub(super) const SAVE_ENVIRONMENT_PRESET: &str = "Save Environment Preset";

impl EnvPresetDialog {
    /// EP §Save rows: Include Lights (on), Include Selected Meshes (on,
    /// greyed with no mesh selected, UIC-117), Include Cameras (off, v0.2),
    /// Copy Images Next To Preset (on). Each with its tooltip (UIC-82).
    fn rows(any_mesh: bool) -> Vec<form::Row> {
        use form::{Row, When};
        let meshes = Row::check("meshes", "Include Selected Meshes", true);
        let meshes = if any_mesh {
            meshes.tip("Save the selected meshes with the environment (a ground, a backdrop)")
        } else {
            meshes
                .when(When::Never)
                .tip("Select one or more meshes to include them")
        };
        vec![
            Row::check("lights", "Include Lights", true)
                .tip("Save every light with the environments"),
            meshes,
            Row::check("cameras", "Include Cameras", false)
                .tip("Save every camera with the environments (render settings never are)"),
            Row::check("copy", "Copy Images Next To Preset", true).tip(
                "Copy the environment pictures beside the preset file, so the preset still \
                 finds them when its folder moves",
            ),
        ]
    }
}

impl Operandi {
    /// Whether any mesh is selected (for Include Selected Meshes).
    fn any_mesh_selected(&self) -> bool {
        let scene = &self.editor.scene;
        scene.selected_items().into_iter().any(|id| {
            scene.items.iter().any(|i| {
                i.id == id
                    && i.is_mesh()
                    && i.in_item_list()
                    && i.environment.is_none()
                    && i.light.is_none()
            })
        })
    }

    /// File > Save Environment Preset… and the environment rows' menus:
    /// the options dialog, then the save panel.
    pub(super) fn open_save_environment_preset(&mut self) {
        if self.env_preset_dialog.is_some() || self.choosing.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before Save Environment Preset".into();
            return;
        }
        if let Err(e) = self.editor.require_idle() {
            self.status = e;
            return;
        }
        // EP v0.2: saving starts from an environment (the clicked one, or
        // the top one from the File menu).
        if self.editor.scene.environments().next().is_none() {
            self.status =
                "The scene has no environment to save; use Item > Create Environment".into();
            return;
        }
        let rows = EnvPresetDialog::rows(self.any_mesh_selected());
        // EP: the options start ticked every time.
        let rows = rows.into_iter().map(form::Row::fresh).collect();
        self.env_preset_dialog = Some(EnvPresetDialog {
            form: form::Dialog::with_defaults(SAVE_ENVIRONMENT_PRESET, rows),
        });
    }

    /// The dialog, once per frame. OK opens the save panel with its
    /// options (UIC-96: nothing is written until a file is chosen).
    pub(super) fn environment_preset_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.env_preset_dialog.take() else {
            return;
        };
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Ok => {
                let v = &dialog.form.values;
                let options = EnvironmentPresetOptions {
                    lights: v.bool("lights"),
                    meshes: v.bool("meshes") && self.any_mesh_selected(),
                    cameras: v.bool("cameras"),
                    copy_images: v.bool("copy"),
                };
                self.request_save_environment_preset(options);
            }
            form::Outcome::Open => self.env_preset_dialog = Some(dialog),
            _ => self.status = "Save Environment Preset canceled; nothing saved".into(),
        }
    }

    /// The save panel for an environment preset, starting in the Presets
    /// folder with the top environment's name.
    pub(super) fn request_save_environment_preset(&mut self, options: EnvironmentPresetOptions) {
        let folder = self.environment.paths().presets();
        if let Err(e) = std::fs::create_dir_all(&folder) {
            self.status = format!("Can't make the Presets folder {}: {e}", folder.display());
            return;
        }
        let name = self
            .editor
            .scene
            .environments()
            .next()
            .map_or_else(|| "Environment".to_owned(), |i| i.name.clone())
            .replace(['/', '\\', ':'], "-");
        let request = file_chooser::Choose::SaveEnvironmentPreset {
            folder,
            name: format!("{}.{ENVIRONMENT_PRESET_EXTENSION}", name.trim()),
        };
        if !self.choose(
            request,
            file_chooser::Purpose::SaveEnvironmentPreset(options),
        ) {
            self.status = "No file panel is available; use preset.saveAs with a filename".into();
        }
    }

    /// Write the environment preset to `path` (`.openv` added when
    /// missing). Returns whether it was written.
    pub(super) fn save_environment_preset_to(
        &mut self,
        path: PathBuf,
        options: EnvironmentPresetOptions,
    ) -> bool {
        let path = if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(operandi_core::preset::is_preset_extension)
        {
            path
        } else {
            let mut name = path.into_os_string();
            name.push(format!(".{ENVIRONMENT_PRESET_EXTENSION}"));
            PathBuf::from(name)
        };
        let file = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        if !self.confirm_fields() {
            return false;
        }
        match self.editor.save_environment_preset(&path, options) {
            Ok(notes) => {
                self.status = std::iter::once(format!("Saved environment preset {file}"))
                    .chain(notes)
                    .collect::<Vec<_>>()
                    .join(" · ");
                // The browser shows it, with its ball thumbnail to follow.
                self.preset_browser.thumbs.forget(&path, true);
                if let Some(parent) = path.parent() {
                    self.preset_browser.refresh(Some(parent));
                }
                self.preset_browser
                    .thumbs
                    .render_into_file(&path, operandi_core::preset::PresetKind::Environment);
                true
            }
            Err(e) => {
                self.status = format!("Save environment preset {file}: {e}");
                false
            }
        }
    }

    /// Apply the environment preset at `path` as `mode` (EP §Apply), one
    /// undo step. The new top environment is selected in the Shader Tree;
    /// missing pictures are listed once; with Global Illumination off a
    /// one-time hint says to turn it on. Returns whether it applied.
    pub(super) fn apply_environment_preset_from(
        &mut self,
        path: &std::path::Path,
        mode: ApplyMode,
        copy_files: bool,
    ) -> bool {
        let file = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let mut preset = match operandi_core::persistence::load_preset(path) {
            Ok(p) => p,
            Err(e) => {
                self.status = format!("Load preset {file}: {e}");
                return false;
            }
        };
        // From the Preset Browser, the HDRI is copied into the project
        // (project_files.rs); `preset.do` lines don't copy.
        if copy_files {
            self.adopt_scene_files(&mut preset.scene);
        }
        if preset.header.kind != operandi_core::preset::PresetKind::Environment {
            self.status = format!("{file} isn't an environment preset");
            return false;
        }
        if !self.confirm_fields() {
            return false;
        }
        let missing = operandi_core::preset::missing_images(&preset);
        let name = path
            .file_stem()
            .map_or_else(|| file.clone(), |s| s.to_string_lossy().into_owned());
        if !self.run_ok(Command::ApplyEnvironmentPreset {
            preset: Box::new(preset),
            mode,
            name,
        }) {
            return false;
        }
        if let Some(top) = self.editor.scene.environments().next().map(|i| i.id) {
            self.shading.selected.clear();
            self.shading.environment = Some(crate::environment_ui::EnvironmentRow::Item(top));
            self.shading.environments_open(top);
        }
        let verb = match mode {
            ApplyMode::Replace => "Replaced the environment with",
            ApplyMode::Add => "Added environment preset",
        };
        let mut status = format!("{verb} {file}");
        if !missing.is_empty() {
            // EP: one message listing every missing picture.
            status = format!(
                "{status} · Missing pictures (shown magenta): {}",
                missing.join(", ")
            );
        }
        if !self.preview.options.light_paths.diffuse && !self.gi_hint_shown {
            self.gi_hint_shown = true;
            status = format!("{status} · {GI_HINT}");
        }
        self.status = status;
        true
    }
}

impl Operandi {
    /// The small Replace / Add menu where an environment preset was
    /// dropped on a 3D view (EP §Apply). Escape or a click elsewhere
    /// cancels; nothing changes until a choice.
    pub(super) fn environment_drop_menu(&mut self, ctx: &Context) {
        let Some((path, at)) = self.env_drop_menu.clone() else {
            return;
        };
        self.menu_owned_input = true;
        let mut choice = None;
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let area = egui::Area::new(Id::new("environment-preset-drop"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    if ui
                        .button("Replace")
                        .on_hover_text("Remove every environment and the lights and meshes an earlier preset brought, then add this one")
                        .clicked()
                    {
                        choice = Some(ApplyMode::Replace);
                    }
                    if ui
                        .button("Add")
                        .on_hover_text("Add this preset's environments on top (they win), with its lights and meshes")
                        .clicked()
                    {
                        choice = Some(ApplyMode::Add);
                    }
                    ui.separator();
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        if ctx.input(|i| i.pointer.any_pressed())
            && !area
                .response
                .rect
                .contains(ctx.input(|i| i.pointer.interact_pos()).unwrap_or(at))
        {
            close = true;
        }
        if let Some(mode) = choice {
            self.env_drop_menu = None;
            self.apply_environment_preset_from(&path, mode, true);
        } else if close {
            self.env_drop_menu = None;
        }
    }
}

/// The one-time hint after applying an environment with GI off (EP).
pub(super) const GI_HINT: &str = "Turn on Global Illumination so the environment lights the scene";
