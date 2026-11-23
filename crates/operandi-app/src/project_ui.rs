//! Project system phase 1 in the app (docs/PROJECT_SYSTEM.md): the active
//! project, File > New Project… / Set Project… / Recent Projects, and the
//! remembered last/recent projects (per user, in the app data folder).
//! Setting a project never closes or changes the open scene.
use super::*;
use operandi_core::Project;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Most recent projects kept in File > Recent Projects.
const RECENT_LIMIT: usize = 10;

#[derive(Default, Serialize, Deserialize)]
struct Remembered {
    version: u32,
    /// Set when the user last set a project; reopened on launch.
    last: Option<PathBuf>,
    /// Most recent first.
    recent: Vec<PathBuf>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectDialogKind {
    New,
    Set,
}

pub(super) struct ProjectDialog {
    kind: ProjectDialogKind,
    form: form::Dialog,
    /// Until the user types a name, it follows the folder's name.
    name_edited: bool,
    /// The folder last seen, to notice edits.
    last_folder: String,
}
impl ProjectDialog {
    pub(super) fn folder(&self) -> String {
        self.form.values.text("folder").to_owned()
    }
    pub(super) fn set_folder(&mut self, folder: &str) {
        self.form
            .values
            .set("folder", form::Value::Text(folder.into()));
    }
    pub(super) fn name(&self) -> String {
        self.form.values.text("name").to_owned()
    }
    #[cfg(test)]
    pub(super) fn set_name(&mut self, name: &str) {
        self.form.values.set("name", form::Value::Text(name.into()));
        self.name_edited = true;
    }
    #[cfg(test)]
    pub(super) fn error(&self) -> Option<&String> {
        self.form.error.as_ref()
    }
    /// The name follows the folder until the user types one.
    fn follow_folder(&mut self) {
        let folder = self.folder();
        if folder != self.last_folder {
            if !self.name_edited {
                self.form
                    .values
                    .set("name", form::Value::Text(folder_name(&folder)));
            }
            self.last_folder = folder;
        }
    }
}
/// The dialog's rows (UIC-98). Paths start fresh each time.
fn rows(kind: ProjectDialogKind, folder: &str, has_panel: bool) -> Vec<form::Row> {
    use form::{Row, When};
    let new = kind == ProjectDialogKind::New;
    let mut rows = vec![
        Row::text("folder", "Folder", folder).fresh().tip(if new {
            "Project folder (absolute path). Missing standard folders are added; existing files are kept."
        } else {
            "Project folder (absolute path) containing operandi.project.toml."
        }),
        Row::button("choose", "Choose…")
            .when(if has_panel { When::Always } else { When::Never })
            .tip("Pick the folder in a folder panel"),
    ];
    if new {
        rows.push(
            Row::text("name", "Name", &folder_name(folder))
                .fresh()
                .tip("The project's name; it follows the folder until you type one"),
        );
    }
    rows
}

/// The last part of a folder path, as a default project name.
fn folder_name(folder: &str) -> String {
    Path::new(folder.trim())
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl Operandi {
    /// Launch: reopen the last project if its folder is still a project,
    /// and drop recent entries whose folder is gone.
    pub(super) fn load_projects(&mut self) {
        let path = self.environment.paths().projects();
        let remembered: Remembered = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        self.recent_projects = remembered
            .recent
            .into_iter()
            .filter(|root| root.join(operandi_core::project::MANIFEST).is_file())
            .take(RECENT_LIMIT)
            .collect();
        self.project = remembered.last.and_then(|root| Project::open(&root).ok());
    }

    fn save_projects(&self) {
        let remembered = Remembered {
            version: 1,
            last: self.project.as_ref().map(|p| p.root().to_path_buf()),
            recent: self.recent_projects.clone(),
        };
        let path = self.environment.paths().projects();
        let temp = path.with_extension(format!("tmp-{}", std::process::id()));
        if let Ok(bytes) = serde_json::to_vec_pretty(&remembered) {
            // Losing the recent list is harmless; never block modeling on it.
            let _ = resource_write::write_bytes(&path, &temp, &bytes);
        }
    }

    /// Make `project` active and first in Recent Projects.
    pub(super) fn set_project(&mut self, project: Project) {
        let root = project.root().to_path_buf();
        self.recent_projects.retain(|r| *r != root);
        self.recent_projects.insert(0, root);
        self.recent_projects.truncate(RECENT_LIMIT);
        self.status = format!("Project set: {}", project.name());
        if let Err(e) = project.add_missing_folders() {
            self.status = format!("{} · {e}", self.status);
        }
        self.project = Some(project);
        self.save_projects();
    }

    pub(super) fn open_recent_project(&mut self, index: usize) {
        let Some(root) = self.recent_projects.get(index).cloned() else {
            return;
        };
        match Project::open(&root) {
            Ok(project) => self.set_project(project),
            Err(error) => {
                self.recent_projects.remove(index);
                self.save_projects();
                self.status = format!("{error}; removed it from Recent Projects");
            }
        }
    }

    pub(super) fn open_project_dialog(&mut self, kind: ProjectDialogKind) {
        // Suggest a folder next to the current project, or the home folder.
        let start = self
            .project
            .as_ref()
            .and_then(|p| p.root().parent().map(Path::to_path_buf))
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .unwrap_or_default();
        let folder = match kind {
            ProjectDialogKind::New => start.join("New Project"),
            ProjectDialogKind::Set => self
                .project
                .as_ref()
                .map(|p| p.root().to_path_buf())
                .unwrap_or(start),
        };
        // Set Project is just a folder choice; go straight to the panel.
        if kind == ProjectDialogKind::Set
            && self.choose(
                file_chooser::Choose::Folder {
                    folder: folder.clone(),
                    title: "Set Project",
                },
                file_chooser::Purpose::SetProject,
            )
        {
            return;
        }
        let folder = folder.display().to_string();
        let has_panel = self.chooser.is_some() || self.native_panels;
        let title = match kind {
            ProjectDialogKind::New => "New Project",
            ProjectDialogKind::Set => "Set Project",
        };
        self.project_dialog = Some(ProjectDialog {
            kind,
            form: self
                .form_dialog(title, rows(kind, &folder, has_panel))
                .focus("folder"),
            name_edited: false,
            last_folder: folder,
        });
    }

    /// The folder panel's answer for the open project dialog.
    pub(super) fn project_dialog_folder(&mut self, root: &Path) {
        if let Some(dialog) = &mut self.project_dialog {
            dialog.set_folder(&root.display().to_string());
            dialog.follow_folder();
        }
    }

    /// Text-path dialog, like Open/Save, for New Project and Set Project.
    pub(super) fn project_dialog_window(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.project_dialog.take() else {
            return;
        };
        let name_before = dialog.name();
        let outcome = self.show_form(ctx, &mut dialog.form);
        if dialog.kind == ProjectDialogKind::New && dialog.name() != name_before {
            dialog.name_edited = true;
        }
        dialog.follow_folder();
        match outcome {
            form::Outcome::Cancel => {
                ctx.request_repaint();
                return;
            }
            form::Outcome::Ok => {
                let root = PathBuf::from(dialog.folder().trim());
                let result = if !root.is_absolute() {
                    Err("Enter an absolute folder path".into())
                } else if dialog.kind == ProjectDialogKind::New {
                    Project::create(&root, &dialog.name())
                } else {
                    Project::open(&root)
                };
                match result {
                    Ok(project) => {
                        self.set_project(project);
                        return;
                    }
                    Err(error) => {
                        self.status = error.clone();
                        dialog.form.error = Some(error);
                    }
                }
            }
            _ => {}
        }
        // The Choose… row reports its click as a change of "choose".
        let choose = dialog.form.take_clicked("choose");
        self.project_dialog = Some(dialog);
        // After the dialog is back in place, so the answer can fill it in.
        if choose {
            let start = PathBuf::from(self.project_dialog.as_ref().unwrap().folder().trim());
            let folder = start.parent().map(Path::to_path_buf).unwrap_or(start);
            let title = "Choose or create the project folder";
            self.choose(
                file_chooser::Choose::Folder { folder, title },
                file_chooser::Purpose::ProjectFolder,
            );
        }
    }

    /// Default Open path: the project's scenes folder when one is set.
    pub(super) fn open_dialog_default(&self) -> String {
        match &self.project {
            Some(p) => format!("{}/", p.scenes().display()),
            None => self.environment.paths().open_dialog().to_owned(),
        }
    }

    /// Default Save As path: the scene's own file, else the project's scenes
    /// folder (so an untitled scene saves into the active project).
    pub(super) fn save_dialog_default(&self) -> String {
        if let Some(path) = &self.path {
            return path.display().to_string();
        }
        match &self.project {
            Some(p) => p.scenes().join("Untitled.opscene").display().to_string(),
            None => self.environment.paths().save_dialog().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{draw, key};

    /// A fresh temporary folder (not yet a project).
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "operandi-app-project-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A second app sharing `a`'s data folder, as after a relaunch.
    fn relaunch(a: &Operandi) -> Operandi {
        let mut b = Operandi::with_environment(a.environment.clone());
        b.initialize_new_document();
        b.load_projects();
        b
    }

    #[test]
    fn defaults_follow_the_active_project() {
        let mut a = Operandi::empty();
        assert_eq!(a.open_dialog_default(), a.environment.paths().open_dialog());
        assert_eq!(a.save_dialog_default(), a.environment.paths().save_dialog());
        let root = scratch("defaults");
        a.set_project(Project::create(&root, "Shots").unwrap());
        assert_eq!(
            a.open_dialog_default(),
            format!("{}/", root.join("scenes").display())
        );
        assert_eq!(
            a.save_dialog_default(),
            root.join("scenes")
                .join("Untitled.opscene")
                .display()
                .to_string()
        );
        // A scene that already has a file keeps saving there.
        a.path = Some(PathBuf::from("/elsewhere/kept.opscene"));
        assert_eq!(a.save_dialog_default(), "/elsewhere/kept.opscene");
    }

    #[test]
    fn setting_a_project_keeps_the_open_scene() {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        a.run(Command::NewMesh);
        let (scene, saved, dirty) = (a.editor.scene.clone(), a.saved.clone(), a.dirty());
        assert!(dirty);
        a.set_project(Project::create(&scratch("keep"), "Keep").unwrap());
        assert_eq!(a.editor.scene, scene);
        assert_eq!(a.saved, saved);
        assert!(a.dirty(), "still unsaved");
    }

    #[test]
    fn last_and_recent_projects_survive_a_relaunch() {
        let mut a = Operandi::empty();
        let roots: Vec<PathBuf> = (0..12).map(|i| scratch(&format!("recent{i}"))).collect();
        for (i, root) in roots.iter().enumerate() {
            a.set_project(Project::create(root, &format!("P{i}")).unwrap());
        }
        // Setting an older project again moves it to the top, no duplicate.
        a.set_project(Project::open(&roots[5]).unwrap());
        assert_eq!(a.recent_projects.len(), RECENT_LIMIT);
        assert_eq!(a.recent_projects[0], roots[5]);
        assert_eq!(a.recent_projects[1], roots[11]);
        let b = relaunch(&a);
        assert_eq!(
            b.project.as_ref().map(|p| p.root()),
            Some(roots[5].as_path())
        );
        assert_eq!(b.recent_projects, a.recent_projects);
        assert!(b.path.is_none(), "no scene is opened at launch");
    }

    #[test]
    fn missing_project_folders_are_dropped() {
        let mut a = Operandi::empty();
        let (kept, gone) = (scratch("kept"), scratch("gone"));
        a.set_project(Project::create(&kept, "Kept").unwrap());
        a.set_project(Project::create(&gone, "Gone").unwrap());
        std::fs::remove_dir_all(&gone).unwrap();
        let b = relaunch(&a);
        assert!(b.project.is_none(), "the last project's folder is gone");
        assert_eq!(b.recent_projects, vec![kept.clone()]);
        // A recent entry whose folder vanished while running is removed on use.
        let mut a = a;
        assert_eq!(a.recent_projects[0], gone);
        a.open_recent_project(0);
        assert_eq!(a.recent_projects, vec![kept]);
        assert!(a.status.contains("Recent Projects"), "{}", a.status);
    }

    #[test]
    fn file_menu_lists_project_actions() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        let titles = |a: &Operandi| -> Vec<(String, bool)> {
            a.menu_model(&ctx)[0]
                .leaves()
                .into_iter()
                .map(|i| (i.title.clone(), i.enabled))
                .collect()
        };
        let before = titles(&a);
        assert!(before.contains(&("New Project…".into(), true)));
        assert!(before.contains(&("Set Project…".into(), true)));
        assert!(before.contains(&("No Recent Projects".into(), false)));
        let root = scratch("menu");
        a.set_project(Project::create(&root, "Menu").unwrap());
        assert!(titles(&a).contains(&(root.display().to_string(), true)));
    }

    #[test]
    fn new_and_set_project_dialogs() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        let size = vec2(1440., 900.);
        let root = scratch("dialog").join("From Dialog");
        a.open_project_dialog(ProjectDialogKind::New);
        let dialog = a.project_dialog.as_mut().unwrap();
        dialog.set_folder(&root.display().to_string());
        dialog.set_name("Dialog Made");
        draw(&mut a, &ctx, size, vec![]);
        // UIC-85: the Folder field has focus, so the first Return commits
        // it and the next one is OK.
        draw(&mut a, &ctx, size, vec![key(Key::Enter)]);
        draw(&mut a, &ctx, size, vec![key(Key::Enter)]);
        assert!(a.project_dialog.is_none(), "closed after Create");
        assert_eq!(a.project.as_ref().unwrap().name(), "Dialog Made");
        assert!(root.join("scenes").is_dir());

        // Set Project on a folder that isn't a project shows an error and stays open.
        a.open_project_dialog(ProjectDialogKind::Set);
        a.project_dialog
            .as_mut()
            .unwrap()
            .set_folder(&scratch("plain").display().to_string());
        draw(&mut a, &ctx, size, vec![]);
        draw(&mut a, &ctx, size, vec![key(Key::Enter)]);
        draw(&mut a, &ctx, size, vec![key(Key::Enter)]);
        let dialog = a.project_dialog.as_ref().expect("still open");
        assert!(dialog.error().unwrap().contains("isn't a project"));
        assert_eq!(a.project.as_ref().unwrap().root(), root, "unchanged");
        draw(&mut a, &ctx, size, vec![key(Key::Escape)]);
        assert!(a.project_dialog.is_none());
    }

    #[test]
    fn project_name_follows_the_typed_folder_until_edited() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        let size = vec2(1440., 900.);
        a.open_project_dialog(ProjectDialogKind::New);
        draw(&mut a, &ctx, size, vec![]);
        draw(&mut a, &ctx, size, vec![Event::Text("X".into())]);
        let d = a.project_dialog.as_ref().unwrap();
        assert!(
            d.folder().contains('X'),
            "typed into the folder field: {}",
            d.folder()
        );
        assert_eq!(
            d.name(),
            folder_name(&d.folder()),
            "name follows the folder"
        );
        // Once a name has been typed, folder edits leave it alone.
        a.project_dialog.as_mut().unwrap().set_name("Custom");
        draw(&mut a, &ctx, size, vec![Event::Text("Y".into())]);
        let d = a.project_dialog.as_ref().unwrap();
        assert!(d.folder().contains('Y'));
        assert_eq!(d.name(), "Custom");
    }
}
