//! MaterialX in the app (docs/specs/openpbr-materialx, Stage B): File >
//! Import MaterialX… and File > Export Material as MaterialX…, the
//! commands behind them, and the Preset Browser's option to write a
//! .mtlx beside a saved material preset (MX-05, `preset.saveAs mtlx:true`).
//! Importing reads the file as the browser's loader does, then adds its
//! layers as one undo step: directly above `layer:`, or at the top of the
//! shader tree without one (the menu passes the selected layer).
//!
//! The command names are Operandi's own (registry ruling 2026-11-12,
//! docs/COMMAND_REGISTRY.md "Operandi-only today").
use super::*;
use operandi_core::preset::PresetSource;
use operandi_core::shader_tree::{NodeId, NodeKind};
use std::path::{Path, PathBuf};

pub(crate) const COMMANDS: [(&str, &str); 2] = [
    (
        "operandi.materialx.import",
        "Help: operandi.materialx.import [filename:<file.mtlx>] [layer:<id>] imports a MaterialX material (open_pbr_surface, standard_surface, gltf_pbr or UsdPreviewSurface) as a Material and Image Map layers, directly above layer <id> or at the top of the shader tree without one; a material that needs Node Material is refused whole. One undo step. Without a filename it opens the file panel.",
    ),
    (
        "operandi.materialx.export",
        "Help: operandi.materialx.export [filename:<file.mtlx>] [item:<material>] writes material or material group <material> (its id or name; without one, the one selected in the Shading panel) as an OpenPBR MaterialX material and lists what it couldn't write. Not undoable. Without a filename it opens the save panel.",
    ),
];

impl Operandi {
    /// The MaterialX commands typed or sent.
    pub(super) fn materialx_canonical(&mut self, args: &[String]) -> bool {
        let Some(first) = args.first().map(|a| a.to_ascii_lowercase()) else {
            return false;
        };
        if first == "help" && args.len() == 2 {
            let topic = args[1].to_ascii_lowercase();
            if let Some((_, help)) = COMMANDS.iter().find(|(c, _)| *c == topic) {
                self.status = (*help).into();
                return true;
            }
            return false;
        }
        let import = first == COMMANDS[0].0;
        if !import && first != COMMANDS[1].0 {
            return false;
        }
        let names: &[&str] = if import {
            &["filename", "layer"]
        } else {
            &["filename", "item"]
        };
        let bound = match operandi_core::command_args::bind(&first, &args[1..], names) {
            Ok(b) => b,
            Err(e) => {
                self.status = e;
                return true;
            }
        };
        let path = bound[0].map(PathBuf::from);
        if import {
            let beside = match bound[1] {
                // The file panel's route (no filename:) puts it above the
                // selected shader tree layer, as File > Import MaterialX…
                // did (menus as forms).
                None if path.is_none() => self.shading.selected.last().copied(),
                None => None,
                Some(v) => match v.trim().parse::<NodeId>() {
                    Ok(id) if self.editor.scene.shader_tree.get(id).is_some() => Some(id),
                    _ => {
                        self.status = format!(
                            "operandi.materialx.import: no shader tree layer {v} (layer: takes a layer id)"
                        );
                        return true;
                    }
                },
            };
            match path {
                Some(p) => self.import_materialx_from(&p, beside),
                None => self.request_import_materialx(beside),
            }
        } else {
            let node = match bound[1] {
                Some(v) => match self.material_row_named(v) {
                    Some(id) => id,
                    None => {
                        self.status = format!(
                            "operandi.materialx.export: {v} isn't a material or material group in the shader tree"
                        );
                        return true;
                    }
                },
                None => match self.selected_material_row() {
                    Some(id) => id,
                    None => {
                        self.status =
                            "Select a material or material group in the Shading panel to export"
                                .into();
                        return true;
                    }
                },
            };
            match path {
                Some(p) => self.export_materialx_to(p, node),
                None => self.request_export_materialx(node),
            }
        }
        true
    }

    /// The project's materials folder, else the Presets folder.
    fn materialx_folder(&self) -> PathBuf {
        if let Some(materials) = self.project_folder(crate::project_files::MATERIALS) {
            return materials;
        }
        let presets = self.environment.paths().presets();
        if presets.is_dir() {
            presets
        } else {
            self.environment.paths().data_root().to_path_buf()
        }
    }

    /// A Material or Material Group by id or (case-insensitive) name.
    fn material_row_named(&self, v: &str) -> Option<NodeId> {
        let tree = &self.editor.scene.shader_tree;
        let is_material = |n: &operandi_core::shader_tree::Node| {
            matches!(n.kind, NodeKind::Group(_) | NodeKind::Material(_))
        };
        if let Ok(id) = v.trim().parse::<NodeId>()
            && tree.get(id).is_some_and(is_material)
        {
            return Some(id);
        }
        tree.nodes
            .iter()
            .find(|n| n.name.eq_ignore_ascii_case(v.trim()) && is_material(n))
            .map(|n| n.id)
    }

    pub(super) fn request_import_materialx(&mut self, beside: Option<NodeId>) {
        if self.has_modal() || self.choosing.is_some() {
            return;
        }
        let folder = self.materialx_folder();
        if !self.choose(
            file_chooser::Choose::MaterialX { folder },
            file_chooser::Purpose::ImportMaterialX(beside),
        ) {
            self.status =
                "No file panel is available; use operandi.materialx.import filename:<file.mtlx>"
                    .into();
        }
    }

    /// Import `path` (read as the Preset Browser reads a .mtlx) as new
    /// layers directly above `beside`, or at the top of the shader tree;
    /// one undo step, or nothing with a message.
    pub(super) fn import_materialx_from(&mut self, path: &Path, beside: Option<NodeId>) {
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(operandi_core::materialx::EXTENSION))
        {
            self.status = format!("{} isn't a MaterialX (.mtlx) file", path.display());
            return;
        }
        if !path.exists() {
            self.status = format!("No file at {}", path.display());
            return;
        }
        let entry = self.entry_for_path(path);
        let (preset, notes) = match self.materialx_preset(&entry) {
            Ok(p) => p,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        if !self.confirm_fields() {
            return;
        }
        if self.run_ok(Command::InsertMaterialPreset {
            preset: Box::new(preset),
            beside,
        }) {
            self.status = std::iter::once(format!("Loaded MaterialX {}", entry.name))
                .chain(notes)
                .collect::<Vec<_>>()
                .join(" · ");
        }
    }

    pub(super) fn request_export_materialx(&mut self, node: NodeId) {
        if self.has_modal() || self.choosing.is_some() {
            return;
        }
        let name = self
            .editor
            .scene
            .shader_tree
            .get(node)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "Material".into());
        let folder = self.materialx_folder();
        if !self.choose(
            file_chooser::Choose::SaveMaterialX {
                folder,
                name: format!("{name}.{}", operandi_core::materialx::EXTENSION),
            },
            file_chooser::Purpose::ExportMaterialX(node),
        ) {
            self.status =
                "No file panel is available; use operandi.materialx.export filename:<file.mtlx>"
                    .into();
        }
    }

    /// Write the material holding `node` to `path`; the status lists what
    /// was left out (MX-05's warning).
    pub(super) fn export_materialx_to(&mut self, path: PathBuf, node: NodeId) {
        let path = with_extension(path);
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.status = match operandi_core::materialx::export(&self.editor.scene, node, &path) {
            Ok(losses) if losses.is_empty() => format!("Exported MaterialX {file}"),
            Ok(losses) => format!(
                "Exported MaterialX {file}; left out (the material's value stands in): {}",
                losses.join("; ")
            ),
            Err(e) => format!("Export MaterialX {file}: {e}"),
        };
    }

    /// The Preset Browser option (MX-05): a saved material preset gets a
    /// .mtlx of the same name beside it. Returns status notes.
    /// `mtlx` is `preset.saveAs mtlx:` (the gear toggle sets it).
    pub(super) fn materialx_sidecar(
        &mut self,
        preset: &Path,
        source: PresetSource,
        mtlx: bool,
    ) -> Vec<String> {
        let PresetSource::Material(node) = source else {
            return Vec::new();
        };
        if !mtlx {
            return Vec::new();
        }
        let path = preset.with_extension(operandi_core::materialx::EXTENSION);
        match operandi_core::materialx::export(&self.editor.scene, node, &path) {
            Ok(losses) if losses.is_empty() => vec!["MaterialX written beside it".into()],
            Ok(losses) => vec![format!(
                "MaterialX written beside it; left out: {}",
                losses.join("; ")
            )],
            Err(e) => vec![format!("MaterialX not written: {e}")],
        }
    }
}

fn with_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(operandi_core::materialx::EXTENSION))
    {
        path
    } else {
        let mut name = path.into_os_string();
        name.push(format!(".{}", operandi_core::materialx::EXTENSION));
        PathBuf::from(name)
    }
}
