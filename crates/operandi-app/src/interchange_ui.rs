//! File > Import… and File > Export for the interchange formats (OBJ, STL,
//! glTF/GLB, VideoScape GEO, SVG, USD...). Reading and writing live in
//! `operandi_core::interchange`; this file routes menu intent, panels and
//! the status line.
use super::*;
use operandi_core::interchange::{self, ExportFormat};

impl Operandi {
    /// Replace the document with an imported file as a new unsaved scene.
    /// Callers have already passed unsaved-work protection (Open/Import).
    pub(super) fn import_interchange(&mut self, path: PathBuf) {
        self.import_interchange_with(path, &Default::default());
    }

    /// [`import_interchange`](Self::import_interchange) with a USD file's
    /// import options (the USD import dialog's).
    pub(super) fn import_interchange_with(
        &mut self,
        path: PathBuf,
        usd: &interchange::UsdImportOptions,
    ) {
        match interchange::import_with(&path, usd) {
            Ok(mut imported) => {
                // Interchange files carry no camera or light: give the scene
                // the new-scene ones so Camera view is available.
                if let Err(e) = imported.scene.ensure_camera_and_light() {
                    self.status = format!("Import {}: {e}", path.display());
                    self.import_message = Some(self.status.clone());
                    return;
                }
                // RI-05: new-scene render settings.
                imported.scene.use_new_scene_render_settings();
                super::import::item_mode_after_import(&mut imported.scene);
                self.make_room_for_document();
                self.editor = Editor {
                    bevel_states: self.editor.bevel_states.clone(),
                    bevel_state: self.editor.bevel_state.clone(),
                    bevel_profile: self.editor.bevel_profile,
                    scene: imported.scene,
                    ..Default::default()
                };
                // Any imported file (OBJ, FBX, USD…) is a new, unsaved
                // document until saved as .opscene, like LXO (2026-11-19).
                self.saved = Scene::default();
                self.path = None;
                self.clear_fields();
                self.name = self
                    .editor
                    .scene
                    .active()
                    .map(|i| i.name.clone())
                    .unwrap_or_default();
                let file = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.status = format!("Imported {file} as an unsaved scene");
                if !imported.notes.is_empty() {
                    self.import_message =
                        Some(format!("{}\n\n{}", self.status, imported.notes.join("\n")));
                }
                self.frame(false);
            }
            Err(e) => {
                self.status = format!("Import {}: {e}", path.display());
                self.import_message = Some(self.status.clone());
            }
        }
    }

    /// Suggested export path: the scene's file (or the Save As default) with
    /// the format's extension.
    /// With a project set it is in the project's exports folder.
    pub(super) fn export_dialog_default(&self, format: ExportFormat) -> String {
        let base = self.save_dialog_default();
        if let Some(exports) = self.project_folder(crate::project_files::EXPORTS) {
            let stem = std::path::Path::new(&base)
                .file_stem()
                .filter(|_| !base.ends_with('/'))
                .map_or_else(|| "Untitled".into(), |s| s.to_string_lossy().into_owned());
            return exports
                .join(format!("{stem}.{}", format.extension()))
                .display()
                .to_string();
        }
        if base.ends_with('/') {
            return format!("{base}Untitled.{}", format.extension());
        }
        std::path::Path::new(&base)
            .with_extension(format.extension())
            .display()
            .to_string()
    }

    /// File > Export > format. Export never changes the document, so there is
    /// no unsaved-work prompt.
    pub(super) fn request_export(&mut self, format: ExportFormat) {
        if self.has_modal() {
            return;
        }
        let (folder, name) = file_chooser::split(&self.export_dialog_default(format));
        if self.choose(
            file_chooser::Choose::Export {
                folder,
                name,
                format,
            },
            file_chooser::Purpose::Export(format),
        ) {
            return;
        }
        self.file_dialog = Some(FileDialog {
            action: FileAction::Export(format),
            path: self.export_dialog_default(format),
            overwrite: false,
        });
    }

    /// Write the committed scene (live tool previews are not exported).
    pub(super) fn export_to(&mut self, path: PathBuf, format: ExportFormat) -> bool {
        interchange::set_file_info(self.defaults_preferences.file_info());
        match interchange::export(
            &path,
            &self
                .editor
                .evaluated(operandi_core::evaluate::Purpose::Final),
            format,
        ) {
            Ok(report) => {
                let file = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.status = format!(
                    "Exported {} item(s), {} polygon(s) to {file}",
                    report.items, report.polygons
                );
                if !report.notes.is_empty() {
                    self.status = format!("{} · {}", self.status, report.notes.join(" · "));
                }
                true
            }
            Err(e) => {
                self.status = format!("Export {}: {e}", path.display());
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "operandi-interchange-ui-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A new document whose mesh is a unit box.
    fn app() -> Operandi {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        a.editor.scene.items[0].mesh = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1)
            .unwrap()
            .into();
        a.saved = a.editor.scene.clone();
        a
    }

    #[test]
    fn export_then_open_brings_the_mesh_back_as_an_unsaved_scene() {
        let dir = scratch("round-trip");
        let mut app = app();
        let path = dir.join("box.obj");
        assert!(app.export_to(path.clone(), ExportFormat::Obj));
        assert!(app.status.starts_with("Exported 1 item(s), 6 polygon(s)"));
        let before = app.editor.scene.items[0].mesh.vertices.clone();
        app.request_file(FileAction::New);
        app.open(path);
        assert_eq!(app.path, None);
        // An imported file is unsaved ("Untitled *") until saved as
        // .opscene, like LXO (2026-11-19; BIG_SCENE_SAVE_GUI_QA step 1).
        assert!(app.dirty());
        assert_eq!(app.editor.scene.items[0].mesh.vertices, before);
        assert!(app.status.starts_with("Imported box.obj"));
        // IMP-Q01: an import opens in Item mode with its mesh selected, so
        // Move moves the item rather than every point of the mesh.
        let selection = &app.editor.scene.selection;
        assert_eq!(selection.mode, Mode::Item);
        assert_eq!(selection.item, Some(app.editor.scene.items[0].id));
    }

    #[test]
    fn a_refused_import_keeps_the_current_document() {
        let dir = scratch("refused");
        let mut app = app();
        let before = app.editor.scene.clone();
        let path = dir.join("broken.stl");
        std::fs::write(&path, "not an stl").unwrap();
        app.open(path);
        assert_eq!(app.editor.scene, before);
        assert!(app.import_message.is_some());
    }

    #[test]
    fn export_menu_uses_the_chosen_format_and_path() {
        let dir = scratch("menu");
        let target = dir.join("drawing.svg");
        static TARGET: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
        *TARGET.lock().unwrap() = Some(target.clone());
        fn chooser(request: file_chooser::Choose) -> Option<PathBuf> {
            match request {
                file_chooser::Choose::Export { format, name, .. } => {
                    assert_eq!(format, ExportFormat::Svg);
                    assert!(name.ends_with(".svg"), "{name}");
                    TARGET.lock().unwrap().clone()
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        let mut app = app();
        app.chooser = Some(chooser);
        app.request_export(ExportFormat::Svg);
        assert!(
            std::fs::read_to_string(&target)
                .unwrap()
                .starts_with("<svg")
        );
    }
}
