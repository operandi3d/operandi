//! Native file panels, with a stand-in chooser (no window in tests).
use super::file_chooser::Choose;
use super::*;
use operandi_core::Project;
use std::cell::RefCell;

thread_local! {
    static ANSWER: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static ASKED: RefCell<Vec<Choose>> = const { RefCell::new(Vec::new()) };
}

/// Records the request and returns the queued answer (None = Cancel).
fn stand_in(request: Choose) -> Option<PathBuf> {
    ASKED.with(|a| a.borrow_mut().push(request));
    ANSWER.with(|a| a.borrow_mut().take())
}

fn answer(path: Option<PathBuf>) {
    ANSWER.with(|a| *a.borrow_mut() = path);
    ASKED.with(|a| a.borrow_mut().clear());
}

fn asked() -> Vec<Choose> {
    ASKED.with(|a| a.borrow().clone())
}

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a.chooser = Some(stand_in);
    a
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "operandi-chooser-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn save_as_and_open_use_the_panels_in_the_project_scenes_folder() {
    let mut a = app();
    let root = scratch("save");
    a.set_project(Project::create(&root, "Panels").unwrap());
    a.run(Command::NewMesh);
    let file = root.join("scenes/Shot.opscene");
    answer(Some(file.clone()));
    a.request_file(FileAction::SaveAs);
    assert_eq!(
        asked(),
        vec![Choose::Save {
            folder: root.join("scenes"),
            name: "Untitled.opscene".into()
        }]
    );
    assert!(file.is_file() && a.path.as_ref() == Some(&file) && !a.dirty());
    assert!(a.file_dialog.is_none(), "no typed dialog");
    let saved = a.editor.scene.clone();

    // Open from a fresh scene: the panel starts in scenes/.
    a.initialize_new_document();
    answer(Some(file.clone()));
    a.request_file(FileAction::Open);
    assert_eq!(
        asked(),
        vec![Choose::Open {
            folder: root.join("scenes/")
        }]
    );
    assert_eq!(a.editor.scene.items.len(), saved.items.len());
    assert_eq!(a.path.as_ref(), Some(&file));
}

#[test]
fn cancelling_a_panel_changes_nothing() {
    let mut a = app();
    a.run(Command::NewMesh);
    let (scene, path) = (a.editor.scene.clone(), a.path.clone());
    answer(None);
    a.request_file(FileAction::SaveAs);
    answer(None);
    a.do_file(FileAction::Open);
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.path, path);
    assert!(a.dirty() && a.file_dialog.is_none() && a.pending.is_none());
}

#[test]
fn unsaved_prompt_save_uses_the_panel_then_continues() {
    let mut a = app();
    a.run(Command::NewMesh);
    a.new_scene_tab();
    a.run(Command::NewMesh);
    a.request_file(FileAction::CloseScene);
    assert!(
        matches!(a.pending, Some(FileAction::CloseScene)),
        "asks first"
    );
    let file = scratch("prompt").join("Kept.opscene");
    answer(Some(file.clone()));
    a.prompt_save(FileAction::CloseScene);
    assert!(file.is_file(), "saved through the panel");
    assert!(a.pending.is_none(), "then carried on");
    assert_eq!(a.scenes.tabs.len(), 1, "Close Scene ran");

    // Cancelling the panel keeps the prompt.
    a.run(Command::NewMesh);
    a.request_file(FileAction::CloseScene);
    answer(None);
    a.prompt_save(FileAction::CloseScene);
    assert!(matches!(a.pending, Some(FileAction::CloseScene)));
}

#[test]
fn set_project_goes_straight_to_a_folder_panel() {
    let mut a = app();
    let root = scratch("setproj");
    Project::create(&root, "Chosen").unwrap();
    answer(Some(root.clone()));
    a.open_project_dialog(project_ui::ProjectDialogKind::Set);
    assert!(matches!(asked()[..], [Choose::Folder { .. }]));
    assert!(a.project_dialog.is_none());
    assert_eq!(a.project.as_ref().unwrap().name(), "Chosen");

    answer(Some(scratch("plain")));
    a.open_project_dialog(project_ui::ProjectDialogKind::Set);
    assert!(a.status.contains("isn't a project"), "{}", a.status);
    assert_eq!(a.project.as_ref().unwrap().name(), "Chosen", "unchanged");
}
