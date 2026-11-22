//! Copy added files into the active project (docs/PROJECT_SYSTEM.md,
//! phase 2): a picture, IES file or imported material's images chosen from
//! outside the project are copied into the project's folder for that kind,
//! and the scene refers to the copy. Every file panel result that a scene
//! will refer to goes through [`Operandi::adopt_file`].
//!
//! Preferences > File I/O > Copy Added Files into the Project turns this
//! off. Without an active project, or for a file already inside it,
//! nothing is copied. The original is never moved, renamed or deleted, and
//! the copy isn't part of undo: undo takes back the scene change only.
use operandi_core::project_paths::{self, Adopted};
use std::path::{Path, PathBuf};

/// Project folder kinds for added files.
pub(super) const REFERENCES: &str = "references";
pub(super) const SOURCE_IMAGES: &str = "sourceimages";
/// Project folder kinds file panels start in.
pub(super) const RENDERS: &str = "renders";
pub(super) const EXPORTS: &str = "exports";
pub(super) const MATERIALS: &str = "materials";

impl super::Operandi {
    /// The path the scene should use for `path`, a file being added to it:
    /// a copy in the active project's `kind` folder, or `path` itself
    /// (preference off, no project, already inside the project, missing
    /// file, or a copy that failed, which leaves a status note).
    pub(super) fn adopt_file(&mut self, kind: &str, path: &Path) -> PathBuf {
        if !self.file_io_preferences.copy_into_project || !path.is_file() {
            return path.to_path_buf();
        }
        let Some(project) = &self.project else {
            return path.to_path_buf();
        };
        if project_paths::relative_to(project.root(), path).is_some() {
            return path.to_path_buf();
        }
        let folder = project
            .folder(kind)
            .unwrap_or_else(|| project.root().join(kind));
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match project_paths::copy_into(&folder, path) {
            Ok(adopted) => {
                if let Adopted::Copied(to) = &adopted {
                    let shown = to.file_name().map(|n| n.to_string_lossy().into_owned());
                    self.copy_notes.push(format!(
                        "Copied {} into the project's {kind} folder",
                        shown.unwrap_or(name)
                    ));
                }
                adopted.path().to_path_buf()
            }
            Err(e) => {
                self.copy_notes.push(format!(
                    "Couldn't copy {name} into the project ({e}); using the original"
                ));
                path.to_path_buf()
            }
        }
    }

    /// [`Self::adopt_file`] for every file an imported material or preset
    /// scene refers to (pictures, node-graph images, IES files). Pictures a
    /// Backdrop or camera Background Image shows go to `references`, the
    /// rest to `sourceimages`, as when they are chosen one by one.
    pub(super) fn adopt_scene_files(&mut self, scene: &mut operandi_core::Scene) {
        if self.project.is_none() || !self.file_io_preferences.copy_into_project {
            return;
        }
        let referenced: std::collections::BTreeSet<u64> = scene
            .items
            .iter()
            .flat_map(|i| {
                let backdrop = i.backdrop.as_ref().and_then(|b| b.image);
                let camera = i.camera.as_ref().and_then(|c| c.background.image);
                [backdrop, camera]
            })
            .flatten()
            .collect();
        // Kinds in `for_each_path_mut` order: the scene's pictures first.
        let mut kinds: Vec<&'static str> = scene
            .images
            .iter()
            .map(|i| {
                if referenced.contains(&i.id) {
                    REFERENCES
                } else {
                    SOURCE_IMAGES
                }
            })
            .collect();
        let mut paths = Vec::new();
        project_paths::for_each_path_mut(scene, |p| paths.push(p.clone()));
        kinds.resize(paths.len(), SOURCE_IMAGES);
        let adopted: Vec<String> = paths
            .iter()
            .zip(kinds)
            .map(|(p, kind)| {
                if p.is_empty() {
                    p.clone()
                } else {
                    self.adopt_file(kind, Path::new(p))
                        .to_string_lossy()
                        .into_owned()
                }
            })
            .collect();
        let mut next = adopted.into_iter();
        project_paths::for_each_path_mut(scene, |p| {
            if let Some(a) = next.next() {
                *p = a;
            }
        });
    }

    /// A picture dropped from the Finder (or another app) is added like a
    /// chosen one: HDR/EXR (usually an environment) to `sourceimages`,
    /// other pictures (Backdrops) to `references`. Scene and model files,
    /// folders, and Preset Browser drags (library content) aren't copied.
    pub(super) fn adopt_dropped(&mut self, path: PathBuf) -> PathBuf {
        if self.has_modal() || !crate::backdrop_ui::is_still_image(&path) {
            return path;
        }
        let hdr = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "hdr" | "exr"));
        self.adopt_file(if hdr { SOURCE_IMAGES } else { REFERENCES }, &path)
    }

    /// The active project's `kind` folder for a file panel to start in,
    /// made if it is missing; `None` without a project (callers keep
    /// their own default).
    pub(super) fn project_folder(&self, kind: &str) -> Option<PathBuf> {
        let project = self.project.as_ref()?;
        let folder = project
            .folder(kind)
            .unwrap_or_else(|| project.root().join(kind));
        std::fs::create_dir_all(&folder).ok()?;
        Some(folder)
    }

    /// Put this frame's copy notes after the status line the action wrote.
    pub(super) fn flush_copy_notes(&mut self) {
        if self.copy_notes.is_empty() {
            return;
        }
        let notes = std::mem::take(&mut self.copy_notes);
        let copied = notes.iter().filter(|n| n.starts_with("Copied ")).count();
        let mut parts: Vec<String> = notes
            .into_iter()
            .filter(|n| copied <= 1 || !n.starts_with("Copied "))
            .collect();
        if copied > 1 {
            parts.insert(0, format!("Copied {copied} files into the project"));
        }
        let joined = parts.join(" · ");
        self.status = if self.status.is_empty() {
            joined
        } else {
            format!("{} · {joined}", self.status)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> crate::Operandi {
        crate::Operandi::empty()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "operandi-project-files-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn adopt_copies_outside_files_only_with_a_project_and_the_preference() {
        let base = scratch("adopt");
        let outside = base.join("Downloads/oak.png");
        std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
        std::fs::write(&outside, b"pixels").unwrap();
        let mut a = app();
        // No project: the original.
        assert_eq!(a.adopt_file(SOURCE_IMAGES, &outside), outside);
        let root = base.join("Proj");
        a.project = Some(operandi_core::Project::create(&root, "Proj").unwrap());
        // Preference off: the original.
        a.file_io_preferences.copy_into_project = false;
        assert_eq!(a.adopt_file(SOURCE_IMAGES, &outside), outside);
        a.file_io_preferences.copy_into_project = true;
        let copy = a.adopt_file(SOURCE_IMAGES, &outside);
        assert_eq!(copy, root.join("sourceimages/oak.png"));
        assert_eq!(std::fs::read(&copy).unwrap(), b"pixels");
        assert!(outside.is_file(), "the original stays");
        // Inside the project already: no second copy.
        assert_eq!(a.adopt_file(SOURCE_IMAGES, &copy), copy);
        a.status = "Loaded".into();
        a.flush_copy_notes();
        assert_eq!(
            a.status,
            "Loaded · Copied oak.png into the project's sourceimages folder"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_failed_copy_uses_the_original_and_says_so() {
        let base = scratch("adopt-fail");
        let outside = base.join("oak.png");
        std::fs::write(&outside, b"pixels").unwrap();
        let root = base.join("Proj");
        let mut a = app();
        a.project = Some(operandi_core::Project::create(&root, "Proj").unwrap());
        // A file where the folder should be makes the copy fail.
        std::fs::remove_dir_all(root.join("references")).unwrap();
        std::fs::write(root.join("references"), b"not a folder").unwrap();
        assert_eq!(a.adopt_file(REFERENCES, &outside), outside);
        a.flush_copy_notes();
        assert!(a.status.contains("Couldn't copy oak.png into the project"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn with_a_project_exports_and_renders_start_in_its_folders() {
        use operandi_core::interchange::ExportFormat;
        let base = scratch("folders");
        let mut a = app();
        assert!(a.project_folder(RENDERS).is_none());
        assert!(
            !a.export_dialog_default(ExportFormat::Obj)
                .contains("exports")
        );
        let root = base.join("Proj");
        let project = operandi_core::Project::create(&root, "Proj").unwrap();
        std::fs::remove_dir_all(root.join("renders")).unwrap();
        a.project = Some(project);
        assert_eq!(a.project_folder(RENDERS), Some(root.join("renders")));
        assert!(root.join("renders").is_dir(), "made when missing");
        assert_eq!(
            std::path::PathBuf::from(a.export_dialog_default(ExportFormat::Obj)),
            root.join("exports/Untitled.obj")
        );
        assert_eq!(a.project_folder(MATERIALS), Some(root.join("materials")));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_preference_has_a_command_line() {
        let mut a = app();
        assert!(a.file_io_preferences.copy_into_project, "on by default");
        a.execute("pref.value fileIo.copy_into_project false");
        assert!(!a.file_io_preferences.copy_into_project, "{}", a.status);
        a.execute("pref.value fileIo.copy_into_project true");
        assert!(a.file_io_preferences.copy_into_project);
    }

    fn browser_entry(
        path: &Path,
        kind: operandi_core::preset::library::EntryKind,
    ) -> operandi_core::preset::library::Entry {
        operandi_core::preset::library::Entry {
            name: path.file_stem().unwrap().to_string_lossy().into_owned(),
            path: path.to_path_buf(),
            kind,
            modified: None,
            size: 0,
            texture_set: None,
            modo_kind: None,
        }
    }

    fn uses(a: &crate::Operandi, path: &Path) -> bool {
        a.editor
            .scene
            .images
            .iter()
            .any(|i| Path::new(&i.path) == path)
    }

    #[test]
    fn presets_from_the_browser_copy_their_pictures_and_preset_do_does_not() {
        use operandi_core::preset::PresetKind;
        use operandi_core::preset::library::EntryKind;
        let base = scratch("presets");
        let library = base.join("Library");
        let sky = library.join("Skies/sky.png");
        std::fs::create_dir_all(sky.parent().unwrap()).unwrap();
        image::RgbaImage::new(8, 4).save(&sky).unwrap();
        // An environment preset whose picture lives in the library.
        let mut source = app();
        source.run(operandi_core::Command::AddItem(
            operandi_core::item_list::AddItemKind::Environment,
        ));
        assert!(
            source.environment_image_chosen(None, &sky),
            "{}",
            source.status
        );
        let preset = library.join("Presets/Sky.openv");
        std::fs::create_dir_all(preset.parent().unwrap()).unwrap();
        source.run_command_line(&format!(
            "preset.saveAs kind:environment filename:{{{}}}",
            preset.display()
        ));
        assert!(preset.exists(), "{}", source.status);

        let root = base.join("Proj");
        let mut a = app();
        a.project = Some(operandi_core::Project::create(&root, "Proj").unwrap());
        // preset.do (a script line) doesn't copy.
        a.run_command_line(&format!("preset.do preset:{{{}}}", preset.display()));
        assert!(uses(&a, &sky), "{}", a.status);
        assert!(!root.join("sourceimages/sky.png").exists());
        // The Preset Browser does.
        let entry = browser_entry(&preset, EntryKind::Preset(PresetKind::Environment));
        assert!(a.browser_load(&entry, crate::preset_browser::Target::Default));
        let copy = root.join("sourceimages/sky.png");
        assert!(uses(&a, &copy), "{}", a.status);
        assert!(sky.is_file(), "the library file stays");

        // A library HDRI double-clicked: copied too (identical, so reused).
        let mut b = app();
        b.project = a.project.clone();
        b.run(operandi_core::Command::AddItem(
            operandi_core::item_list::AddItemKind::Environment,
        ));
        let entry = browser_entry(&sky, EntryKind::Environment);
        assert!(b.browser_load(&entry, crate::preset_browser::Target::Default));
        assert!(uses(&b, &copy), "{}", b.status);
        assert_eq!(
            std::fs::read_dir(root.join("sourceimages"))
                .unwrap()
                .count(),
            1
        );

        // Preference off: the library path.
        let mut c = app();
        c.project = a.project.clone();
        c.file_io_preferences.copy_into_project = false;
        c.run(operandi_core::Command::AddItem(
            operandi_core::item_list::AddItemKind::Environment,
        ));
        assert!(c.browser_load(&entry, crate::preset_browser::Target::Default));
        assert!(uses(&c, &sky));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn scene_files_pick_references_for_backdrop_pictures() {
        let base = scratch("kinds");
        let outside = base.join("Desktop/ref.png");
        std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
        image::RgbaImage::new(4, 4).save(&outside).unwrap();
        let mut src = app();
        src.create_backdrop();
        let item = src.editor.scene.selection.item.unwrap();
        src.backdrop_image_chosen(item, outside.clone());
        let mut scene = src.editor.scene.clone();
        let root = base.join("Proj");
        let mut a = app();
        a.project = Some(operandi_core::Project::create(&root, "Proj").unwrap());
        a.adopt_scene_files(&mut scene);
        assert_eq!(
            Path::new(&scene.images[0].path),
            root.join("references/ref.png")
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
