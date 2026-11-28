//! Presets, Stage 0 (operandi_core::preset, PRESET-BROWSER.md PB-40..52):
//! the File menu entries, the Item List and Shading panel right-click
//! entries, and the `preset.save` / `preset.load` commands. Every route
//! goes through the same two operations: `Editor::save_preset` (changes
//! nothing) and `Command::LoadPreset` (one undo step).
//!
//! Labels that open a file panel end in "…" (UIC-03).
use super::*;
use operandi_core::preset::{PresetKind, PresetSource};
use operandi_core::shader_tree::{NodeId, NodeKind};

/// The preset commands, named `<domain>.<verb>` like the others. The name
/// map has no preset commands, so these are Operandi's names.
pub(crate) const COMMANDS: [(&str, &str); 2] = [
    (
        "preset.saveas",
        "Help: preset.saveAs kind:<item|assembly|environment|renderSettings|material> [filename:<file.oppreset>] [mtlx:<true|false>] (old spelling preset.save, path:) saves a preset: the selected items, the selected items with everything below them, every light, the render outputs and render camera, or the material or material group selected in the Shading panel. Without a path it opens the save panel in your Presets folder. mtlx:true (an Operandi extra, default off; the Preset Browser gear toggle sets it) also writes a .mtlx of a material preset beside it.",
    ),
    (
        "preset.do",
        "Help: preset.do [preset:<file.oppreset>] (old spelling preset.load, path:) loads a preset as one undo step: items are added, an environment replaces every light, render settings replace the render outputs, and a material goes into the material group selected in the Shading panel. Also loads what the Preset Browser shows (texture sets, HDRs, pictures, scenes); handler: is accepted and ignored; target:<material group> (an Operandi extra) names the group a material goes into. Without a path it opens the file panel.",
    ),
];

impl Operandi {
    /// The material group a material preset loads into: the most recently
    /// selected Shading row that is a Material Group.
    pub(super) fn preset_target_group(&self) -> Option<NodeId> {
        let tree = &self.editor.scene.shader_tree;
        self.shading
            .selected
            .iter()
            .rev()
            .copied()
            .find(|&id| matches!(tree.get(id).map(|n| &n.kind), Some(NodeKind::Group(_))))
    }

    /// The material or material group selected in the Shading panel, if
    /// any (the most recently selected).
    pub(super) fn selected_material_row(&self) -> Option<NodeId> {
        let tree = &self.editor.scene.shader_tree;
        self.shading.selected.iter().rev().copied().find(|&id| {
            matches!(
                tree.get(id).map(|n| &n.kind),
                Some(NodeKind::Group(_) | NodeKind::Material(_))
            )
        })
    }

    /// What File > Save Item Preset… saves (PB-40): a material preset when
    /// the Shading tab is in front with a material or material group
    /// selected there, else the selected items. Provisional (PB-Q4): the
    /// shader tree keeps its selection while hidden, so the tab decides.
    pub(super) fn item_preset_source(&self) -> PresetSource {
        if self.shading_tab()
            && let Some(id) = self.selected_material_row()
        {
            return PresetSource::Material(id);
        }
        PresetSource::Items
    }

    /// A file name to suggest for a preset of `source`.
    pub(super) fn preset_file_name(&self, source: PresetSource) -> String {
        let scene = &self.editor.scene;
        let name = match source {
            PresetSource::Material(id) => scene.shader_tree.get(id).map(|n| n.name.clone()),
            PresetSource::Items | PresetSource::Assembly => scene.active().map(|i| i.name.clone()),
            PresetSource::Environment { .. } => Some("Environment".into()),
            PresetSource::RenderSettings => Some("Render Settings".into()),
        }
        .unwrap_or_else(|| "Preset".into());
        // Path separators would make a folder of the name.
        let name: String = name
            .chars()
            .map(|c| {
                if matches!(c, '/' | '\\' | ':') {
                    '-'
                } else {
                    c
                }
            })
            .collect();
        format!(
            "{}.{}",
            name.trim(),
            operandi_core::preset::extension_for(source.kind())
        )
    }

    /// Ask where to save a preset of `source`. A selection that can't be
    /// saved is reported now, before any panel opens.
    /// A `preset.saveAs` line from the menu, with `mtlx:true` when the
    /// Preset Browser gear toggle is on.
    #[cfg(test)] // bare preset.saveAs follows the gear itself now
    pub(super) fn preset_save_line(&self, line: &str) -> String {
        if self.preset_browser.write_materialx {
            format!("{line} mtlx:true")
        } else {
            line.to_owned()
        }
    }

    /// Panel buttons: the gear toggle decides the .mtlx beside it.
    pub(super) fn request_save_preset(&mut self, source: PresetSource) {
        self.request_save_preset_mtlx(source, self.preset_browser.write_materialx);
    }

    /// `preset.saveAs` without a filename; `mtlx` is its `mtlx:` argument.
    pub(super) fn request_save_preset_mtlx(&mut self, source: PresetSource, mtlx: bool) {
        if self.has_modal() || self.choosing.is_some() {
            return;
        }
        if !self.confirm_fields() {
            return;
        }
        if let Err(e) = self
            .editor
            .require_idle()
            .and_then(|_| source.build(&self.editor.scene).map(|_| ()))
        {
            self.status = e;
            return;
        }
        // PB-Q2 (decided): the Presets folder in the data folder, made on
        // first save.
        // A material preset starts in the project's materials folder.
        let project_materials = (source.kind() == PresetKind::Material)
            .then(|| self.project_folder(crate::project_files::MATERIALS))
            .flatten();
        let folder = project_materials.unwrap_or_else(|| self.environment.paths().presets());
        if let Err(e) = std::fs::create_dir_all(&folder) {
            self.status = format!("Can't make the Presets folder {}: {e}", folder.display());
            return;
        }
        let request = file_chooser::Choose::SavePreset {
            folder,
            name: self.preset_file_name(source),
            title: format!("Save {} Preset", source.kind().label()),
        };
        if !self.choose(request, file_chooser::Purpose::SavePreset(source, mtlx)) {
            self.status = "No file panel is available; use preset.saveAs with a filename".into();
        }
    }

    /// Write the preset chosen in the save panel.
    /// `mtlx` also writes a .mtlx beside a material preset (MX-05).
    pub(super) fn save_preset_to(&mut self, path: PathBuf, source: PresetSource, mtlx: bool) {
        // The panel normally adds the extension; a typed name may not have it.
        let ext = operandi_core::preset::extension_for(source.kind());
        let path = if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(operandi_core::preset::is_preset_extension)
        {
            path
        } else {
            let mut name = path.into_os_string();
            name.push(format!(".{ext}"));
            PathBuf::from(name)
        };
        let file = file_label(&path);
        let center = std::mem::take(&mut self.preset_browser.save_centered);
        self.status = match self.editor.save_preset_placed(&path, source, center) {
            Ok(notes) => std::iter::once(format!("Saved {} preset {file}", source.kind().label()))
                .chain(notes)
                .chain(self.materialx_sidecar(&path, source, mtlx))
                .collect::<Vec<_>>()
                .join(" · "),
            Err(e) => format!("Save preset {file}: {e}"),
        };
    }

    /// File > Load Preset… (PB-50) and the material group's Presets >
    /// Load… (PB-51, with `group`).
    pub(super) fn request_load_preset(&mut self, group: Option<NodeId>) {
        if self.has_modal() || self.choosing.is_some() {
            return;
        }
        let presets = self.environment.paths().presets();
        let folder = if presets.is_dir() {
            presets
        } else {
            self.environment.paths().data_root().to_path_buf()
        };
        let request = file_chooser::Choose::LoadPreset { folder };
        if !self.choose(request, file_chooser::Purpose::LoadPreset(group)) {
            self.status = "No file panel is available; use preset.do with a preset path".into();
        }
    }

    /// Load the preset at `path`: one undo step, or nothing with a message.
    /// `copy_files`: the preset's pictures are copied into the active
    /// project first (project_files.rs); `preset.do` lines don't.
    pub(super) fn load_preset_from(
        &mut self,
        path: PathBuf,
        group: Option<NodeId>,
        copy_files: bool,
    ) {
        let file = file_label(&path);
        let mut preset = match operandi_core::persistence::load_preset(&path) {
            Ok(preset) => preset,
            Err(e) => {
                self.status = format!("Load preset {file}: {e}");
                return;
            }
        };
        if copy_files {
            self.adopt_scene_files(&mut preset.scene);
        }
        let kind = preset.header.kind;
        if !self.confirm_fields() {
            return;
        }
        if self.run_ok(Command::LoadPreset {
            preset: Box::new(preset),
            group,
        }) {
            self.status = format!("Loaded {} preset {file}", kind.label());
        }
    }

    /// `preset.save` and `preset.load` typed or sent as commands.
    pub(super) fn preset_canonical(&mut self, text: &str) -> bool {
        let Ok(args) = command::tokens(text) else {
            return false;
        };
        let Some(first) = args.first().map(|a| a.to_ascii_lowercase()) else {
            return false;
        };
        // The preset browser's commands (preset_browser.rs).
        if self.preset_browser_canonical(text) {
            return true;
        }
        // MaterialX import and export (materialx_ui.rs).
        if self.materialx_canonical(&args) {
            return true;
        }
        // USD import with options (usd_import_dialog.rs).
        if self.usd_canonical(&args) {
            return true;
        }
        // Load Image… on an environment (environment_ui.rs).
        if self.environment_canonical(&args) {
            return true;
        }
        // Browse… on an Image Map layer (shading_panel.rs).
        if self.texture_load_image_canonical(&args) {
            return true;
        }
        if first == "help" && args.len() == 2 {
            let topic = args[1].to_ascii_lowercase();
            if let Some((_, help)) = COMMANDS.iter().find(|(c, _)| *c == topic) {
                self.status = (*help).into();
                return true;
            }
            return false;
        }
        match first.as_str() {
            "preset.saveas" => {
                let bound = match operandi_core::command_args::bind(
                    "preset.saveAs",
                    &args[1..],
                    &[
                        "kind",
                        "filename",
                        "mtlx",
                        "item",
                        "lights",
                        "meshes",
                        "cameras",
                        "centerOnGround",
                    ],
                ) {
                    Ok(b) => b,
                    Err(e) => {
                        self.status = e;
                        return true;
                    }
                };
                // EP §Save: an environment preset when kind:environment or
                // item: names an environment. lights:/meshes: are Operandi
                // extras (default false); bare, the options dialog opens.
                let env_item = bound[3]
                    .and_then(|v| v.trim().parse::<u64>().ok())
                    .filter(|id| {
                        self.editor
                            .scene
                            .items
                            .iter()
                            .any(|i| i.id == *id && i.environment.is_some())
                    });
                if bound[3].is_some() && env_item.is_none() {
                    self.status = "preset.saveAs: item: takes an environment's id".into();
                    return true;
                }
                let flag = |v: Option<&str>| -> std::result::Result<Option<bool>, String> {
                    match v.map(|v| v.trim().to_ascii_lowercase()) {
                        None => Ok(None),
                        Some(v) if matches!(v.as_str(), "true" | "1" | "on" | "yes") => {
                            Ok(Some(true))
                        }
                        Some(v) if matches!(v.as_str(), "false" | "0" | "off" | "no") => {
                            Ok(Some(false))
                        }
                        Some(v) => Err(format!("preset.saveAs: takes true or false, not {v}")),
                    }
                };
                let (lights, meshes, cameras) =
                    match (flag(bound[4]), flag(bound[5]), flag(bound[6])) {
                        (Ok(l), Ok(m), Ok(c)) => (l, m, c),
                        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                            self.status = e;
                            return true;
                        }
                    };
                if env_item.is_some()
                    || bound[0].map(PresetKind::from_script_name)
                        == Some(Some(PresetKind::Environment))
                {
                    if self.editor.scene.environments().next().is_none() {
                        self.status =
                            "The scene has no environment to save; use Item > Create Environment"
                                .into();
                        return true;
                    }
                    let options = operandi_core::preset::EnvironmentPresetOptions {
                        lights: lights.unwrap_or(false),
                        meshes: meshes.unwrap_or(false),
                        cameras: cameras.unwrap_or(false),
                        copy_images: false,
                    };
                    match bound[1] {
                        Some(path) => {
                            self.save_environment_preset_to(PathBuf::from(path), options);
                        }
                        None if bound[4..7].iter().all(Option::is_none) => {
                            self.open_save_environment_preset()
                        }
                        None => self.request_save_environment_preset(
                            operandi_core::preset::EnvironmentPresetOptions {
                                copy_images: true,
                                ..options
                            },
                        ),
                    }
                    return true;
                }
                let source = match bound[0].map(PresetKind::from_script_name) {
                    None => self.item_preset_source(),
                    Some(Some(PresetKind::Item)) => PresetSource::Items,
                    Some(Some(PresetKind::Assembly)) => PresetSource::Assembly,
                    Some(Some(PresetKind::Environment)) => unreachable!("handled above"),
                    Some(Some(PresetKind::RenderSettings)) => PresetSource::RenderSettings,
                    Some(Some(PresetKind::Material)) => match self.selected_material_row() {
                        Some(id) => PresetSource::Material(id),
                        None => {
                            self.status =
                                "Select a material or material group in the Shading panel".into();
                            return true;
                        }
                    },
                    Some(None) => {
                        self.status = COMMANDS[0].1.trim_start_matches("Help: ").into();
                        return true;
                    }
                };
                // mtlx: (an Operandi extra, default off) also writes a
                // .mtlx beside a material preset; the gear toggle sets it.
                // Without mtlx: the save panel's route follows the gear
                // toggle, as File > Save Item Preset… did (menus as forms).
                let mtlx = match bound[2].map(|v| v.trim().to_ascii_lowercase()) {
                    None => bound[1].is_none() && self.preset_browser.write_materialx,
                    Some(v) if matches!(v.as_str(), "true" | "1" | "on" | "yes") => true,
                    Some(v) if matches!(v.as_str(), "false" | "0" | "off" | "no") => false,
                    Some(v) => {
                        self.status = format!("preset.saveAs: mtlx: takes true or false, not {v}");
                        return true;
                    }
                };
                // centerOnGround: (an Operandi extra, PG-04, default off).
                let center = match bound[7] {
                    None => false,
                    Some(v) => match operandi_core::command_args::boolean("centerOnGround", v) {
                        Ok(b) => b,
                        Err(e) => {
                            self.status = format!("preset.saveAs: {e}");
                            return true;
                        }
                    },
                };
                self.preset_browser.save_centered = center;
                match bound[1] {
                    Some(path) => {
                        if self.confirm_fields() {
                            self.save_preset_to(PathBuf::from(path), source, mtlx);
                        }
                    }
                    None => self.request_save_preset_mtlx(source, mtlx),
                }
                true
            }
            "preset.do" => {
                let bound = match operandi_core::command_args::bind(
                    "preset.do",
                    &args[1..],
                    &["preset", "handler", "target", "mode"],
                ) {
                    Ok(b) => b,
                    Err(e) => {
                        self.status = e;
                        return true;
                    }
                };
                // mode: (an Operandi extra) applies an environment preset
                // as replace (default) or add; 0/1 are aliases (EP §Apply).
                let mode = match bound[3].map(|v| v.trim().to_ascii_lowercase()) {
                    None => operandi_core::preset::ApplyMode::Replace,
                    Some(v) if v == "replace" || v == "0" => {
                        operandi_core::preset::ApplyMode::Replace
                    }
                    Some(v) if v == "add" || v == "1" => operandi_core::preset::ApplyMode::Add,
                    Some(v) => {
                        self.status = format!("preset.do: mode: takes replace or add, not {v}");
                        return true;
                    }
                };
                if let Some(path) = bound[0]
                    && operandi_core::persistence::read_preset_header(std::path::Path::new(path))
                        .is_ok_and(|h| h.kind == PresetKind::Environment)
                {
                    self.apply_environment_preset_from(std::path::Path::new(path), mode, false);
                    return true;
                }
                // Anything that isn't an .oppreset loads as the browser
                // would (texture sets, HDRs, pictures, scenes).
                if let Some(path) = bound[0]
                    && self.preset_do_any(std::path::Path::new(path), bound[2])
                {
                    return true;
                }
                if let Some(group) = bound[2]
                    && !self.select_group_named(group)
                {
                    return true;
                }
                let group = self.preset_target_group();
                match bound[0] {
                    Some(path) => self.load_preset_from(PathBuf::from(path), group, false),
                    None => self.request_load_preset(group),
                }
                true
            }
            _ => false,
        }
    }
}

fn file_label(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
