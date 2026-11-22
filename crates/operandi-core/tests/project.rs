//! Project system phase 1 (docs/PROJECT_SYSTEM.md, Jason's decisions
//! 2026-11-01): create, open, nesting, moving and manifest validation.
use operandi_core::project::{FOLDERS, MANIFEST};
use operandi_core::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

/// A fresh, empty temporary folder for one test.
fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "operandi-project-test-{}-{}-{name}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn create_makes_the_folders_and_a_readable_manifest() {
    let root = scratch("create").join("My Project");
    let project = Project::create(&root, " My Project ").unwrap();
    assert_eq!(project.name(), "My Project");
    for kind in FOLDERS {
        assert!(root.join(kind).is_dir(), "{kind}");
        assert_eq!(project.folder(kind), Some(root.join(kind)));
    }
    assert_eq!(project.scenes(), root.join("scenes"));
    assert!(root.join(MANIFEST).is_file());
    assert_eq!(Project::open(&root).unwrap(), project);
}

#[test]
fn create_on_a_folder_with_files_keeps_them() {
    let root = scratch("existing");
    std::fs::write(root.join("old.opscene"), b"keep me").unwrap();
    std::fs::create_dir(root.join("scenes")).unwrap();
    std::fs::write(root.join("scenes/a.opscene"), b"and me").unwrap();
    Project::create(&root, "Loose").unwrap();
    assert_eq!(std::fs::read(root.join("old.opscene")).unwrap(), b"keep me");
    assert_eq!(
        std::fs::read(root.join("scenes/a.opscene")).unwrap(),
        b"and me"
    );
    assert!(root.join("renders").is_dir());
}

#[test]
fn create_refuses_existing_nested_and_relative_projects() {
    let root = scratch("refuse");
    Project::create(&root, "Outer").unwrap();
    let manifest = std::fs::read(root.join(MANIFEST)).unwrap();
    let again = Project::create(&root, "Again").unwrap_err();
    assert!(again.contains("already a project"), "{again}");
    assert_eq!(
        std::fs::read(root.join(MANIFEST)).unwrap(),
        manifest,
        "untouched"
    );
    let inner = root.join("assets").join("Inner");
    let nested = Project::create(&inner, "Inner").unwrap_err();
    assert!(nested.contains("nested"), "{nested}");
    assert!(!inner.exists(), "nothing created for a refused project");
    assert!(Project::create(&PathBuf::from("relative/folder"), "R").is_err());
    assert!(Project::create(&scratch("noname"), "  ").is_err());
}

#[test]
fn a_moved_project_still_works() {
    let base = scratch("move");
    let (from, to) = (base.join("Before"), base.join("After"));
    Project::create(&from, "Travels").unwrap();
    std::fs::rename(&from, &to).unwrap();
    let project = Project::open(&to).unwrap();
    assert_eq!(project.root(), to);
    assert_eq!(project.scenes(), to.join("scenes"));
    assert_eq!(Project::enclosing(&to.join("scenes")), Some(to.clone()));
    assert_eq!(Project::enclosing(&base), None);
}

#[test]
fn manifest_folders_can_be_renamed_but_must_stay_inside() {
    let root = scratch("manifest");
    let write = |text: &str| std::fs::write(root.join(MANIFEST), text).unwrap();
    write(
        "[project]\nname = \"Studio\"\nformat = 1\n\n[paths]\nscenes = \"shots/scenes\"\ntextures = \"tex\"\n",
    );
    let project = Project::open(&root).unwrap();
    assert_eq!(project.scenes(), root.join("shots/scenes"));
    assert_eq!(project.folder("textures"), Some(root.join("tex")));
    assert_eq!(
        project.folder("renders"),
        Some(root.join("renders")),
        "default"
    );
    for bad in ["../outside", "/absolute", "", "a/../b"] {
        write(&format!(
            "[project]\nname = \"S\"\nformat = 1\n\n[paths]\nscenes = \"{bad}\"\n"
        ));
        assert!(Project::open(&root).is_err(), "{bad:?}");
    }
    write("[project]\nname = \"S\"\nformat = 2\n");
    assert!(Project::open(&root).unwrap_err().contains("format 2"));
    write("[project]\nname = \"S\"\nformat = 1\nsurprise = true\n");
    assert!(Project::open(&root).is_err(), "unknown fields refused");
    std::fs::remove_file(root.join(MANIFEST)).unwrap();
    assert!(
        Project::open(&root)
            .unwrap_err()
            .contains("isn't a project")
    );
}

#[test]
fn set_project_on_an_older_project_adds_the_new_standard_folders() {
    // A project made before materials/ and scripts/ were standard.
    let root = scratch("older");
    std::fs::write(
        root.join(MANIFEST),
        "[project]\nname = \"Old\"\nformat = 1\n\n[paths]\nscenes = \"scenes\"\n",
    )
    .unwrap();
    let project = Project::open(&root).unwrap();
    assert_eq!(project.folder("materials"), Some(root.join("materials")));
    assert_eq!(project.folder("scripts"), Some(root.join("scripts")));
    project.add_missing_folders().unwrap();
    for kind in FOLDERS {
        assert!(root.join(kind).is_dir(), "{kind}");
    }
}
