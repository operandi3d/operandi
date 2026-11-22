//! Project-relative file references (docs/PROJECT_SYSTEM.md phase 2): a
//! scene saved inside a project writes the files it uses that are inside
//! the project as `project:<rel>`, and a copied project finds them again.
use operandi_core::camera_light::{LightKind, LightSettings};
use operandi_core::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "operandi-project-paths-{}-{}-{name}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn touch(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, b"x").unwrap();
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// A scene using `inside` as a backdrop picture and IES file, a node-graph
/// image and a render output, plus `outside` as a second picture.
fn scene_using(inside: &Path, ies: &Path, outside: &Path, render: &Path) -> Scene {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.image_for_path(&s(inside), 4, 4).unwrap();
    e.scene.image_for_path(&s(outside), 4, 4).unwrap();
    let mut light = LightSettings::new(LightKind::Photometric);
    light.photometry_file = s(ies);
    e.scene
        .add_light(light, Default::default())
        .expect("photometric light");
    let graph: node_graph::StoredGraph = serde_json::from_value(serde_json::json!({
        "id": 1,
        "material": {
            "name": "Wood",
            "nodes": [
                {"name": "Material Output", "type": "ShaderNodeOutputMaterial"},
                {"name": "Image", "type": "ShaderNodeTexImage",
                 "image": {"path": s(inside)}}
            ]
        }
    }))
    .unwrap();
    e.scene.shader_tree.node_graphs.graphs.push(graph);
    // A Render Output's file name, when the scene has one.
    for output in e.scene.shader_tree.outputs.values_mut() {
        output.filename = s(render);
    }
    e.scene
}

fn graph_image(scene: &Scene) -> &str {
    &scene.shader_tree.node_graphs.graphs[0].material.nodes[1]
        .image
        .as_ref()
        .unwrap()
        .path
}

#[test]
fn copied_project_finds_its_files_and_outside_files_stay_absolute() {
    let base = scratch("copy");
    let a = base.join("A");
    Project::create(&a, "A").unwrap();
    let oak = a.join("sourceimages/wood/oak.png");
    let ies = a.join("sourceimages/down.ies");
    let render = a.join("renders/shot.exr");
    let outside = base.join("elsewhere/ref.png");
    for f in [&oak, &ies, &outside] {
        touch(f);
    }
    let scene = scene_using(&oak, &ies, &outside, &render);
    let file = a.join("scenes/shot.opscene");
    persistence::save(&file, &scene).unwrap();

    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"version\": 48"), "marked v48");
    assert!(text.contains("\"project:sourceimages/wood/oak.png\""));
    assert!(text.contains("\"project:sourceimages/down.ies\""));
    assert!(
        !text.contains(&s(&oak)),
        "no absolute path into the project"
    );
    assert!(text.contains(&serde_json::to_string(&s(&outside)).unwrap()));

    // Loading where it was saved gives back the scene exactly.
    assert_eq!(persistence::load(&file).unwrap(), scene);

    // Copy the project; the copy's scene points into the copy.
    let b = base.join("moved/B");
    copy_dir(&a, &b);
    std::fs::remove_dir_all(&a).unwrap();
    let back = persistence::load(&b.join("scenes/shot.opscene")).unwrap();
    assert_eq!(back.images[0].path, s(&b.join("sourceimages/wood/oak.png")));
    assert_eq!(back.images[1].path, s(&outside));
    assert_eq!(graph_image(&back), s(&b.join("sourceimages/wood/oak.png")));
    let photometry = back
        .items
        .iter()
        .find_map(|i| i.light.as_ref().filter(|l| !l.photometry_file.is_empty()))
        .unwrap();
    assert_eq!(
        photometry.photometry_file,
        s(&b.join("sourceimages/down.ies"))
    );
    for output in back.shader_tree.outputs.values() {
        if !output.filename.is_empty() {
            assert_eq!(output.filename, s(&b.join("renders/shot.exr")));
        }
    }
}

#[test]
fn a_scene_outside_any_project_keeps_absolute_paths() {
    // The crash-recovery journal and auto-save backups live in the app data
    // folder: even pictures inside a project stay absolute there.
    let base = scratch("recovery");
    let a = base.join("A");
    Project::create(&a, "A").unwrap();
    let oak = a.join("sourceimages/oak.png");
    touch(&oak);
    let scene = scene_using(&oak, &oak, &oak, &a.join("renders/r.exr"));
    let journal = base.join("AppData/recovery.opscene");
    std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
    persistence::save(&journal, &scene).unwrap();
    let text = std::fs::read_to_string(&journal).unwrap();
    assert!(!text.contains("project:"));
    assert!(!text.contains("\"version\": 48"));
    assert_eq!(persistence::load(&journal).unwrap(), scene);
}

#[test]
fn an_old_absolute_scene_loads_unchanged_inside_a_project() {
    let a = scratch("old").join("A");
    Project::create(&a, "A").unwrap();
    let oak = a.join("sourceimages/oak.png");
    touch(&oak);
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.image_for_path(&s(&oak), 4, 4).unwrap();
    // What a build before version 48 wrote: absolute, older version.
    let file = a.join("scenes/old.opscene");
    std::fs::write(&file, persistence::encode(&scene).unwrap()).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(!text.contains("\"version\": 48"));
    assert_eq!(persistence::load(&file).unwrap(), scene);
    // The next save inside the project makes it relative.
    persistence::save(&file, &scene).unwrap();
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("project:sourceimages/oak.png")
    );
}

#[test]
fn without_the_manifest_the_scene_folder_ancestors_are_tried() {
    let base = scratch("fallback");
    let a = base.join("A");
    Project::create(&a, "A").unwrap();
    let oak = a.join("sourceimages/oak.png");
    let gone = a.join("sourceimages/gone.png");
    touch(&oak);
    touch(&gone);
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.image_for_path(&s(&oak), 4, 4).unwrap();
    scene.image_for_path(&s(&gone), 4, 4).unwrap();
    let file = a.join("scenes/s.opscene");
    persistence::save(&file, &scene).unwrap();
    std::fs::remove_file(a.join(project::MANIFEST)).unwrap();
    std::fs::remove_file(&gone).unwrap();
    let back = persistence::load(&file).unwrap();
    assert_eq!(back.images[0].path, s(&oak), "found from an ancestor");
    assert_eq!(
        back.images[1].path, "project:sourceimages/gone.png",
        "kept as text for the missing-file message"
    );
}

#[test]
fn a_missing_file_in_the_project_resolves_to_where_it_belongs() {
    let a = scratch("missing").join("A");
    Project::create(&a, "A").unwrap();
    let oak = a.join("sourceimages/oak.png");
    touch(&oak);
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.image_for_path(&s(&oak), 4, 4).unwrap();
    let file = a.join("scenes/s.opscene");
    persistence::save(&file, &scene).unwrap();
    std::fs::remove_file(&oak).unwrap();
    assert_eq!(persistence::load(&file).unwrap().images[0].path, s(&oak));
}

#[test]
fn copy_into_reuses_identical_files_and_numbers_clashes() {
    use operandi_core::project_paths::{Adopted, copy_into};
    let base = scratch("copy-into");
    let folder = base.join("P/sourceimages");
    let a = base.join("one/oak.png");
    let b = base.join("two/oak.png");
    std::fs::create_dir_all(a.parent().unwrap()).unwrap();
    std::fs::create_dir_all(b.parent().unwrap()).unwrap();
    std::fs::write(&a, b"first").unwrap();
    std::fs::write(&b, b"second").unwrap();
    assert_eq!(
        copy_into(&folder, &a).unwrap(),
        Adopted::Copied(folder.join("oak.png")),
        "the folder is created"
    );
    assert_eq!(
        copy_into(&folder, &a).unwrap(),
        Adopted::Reused(folder.join("oak.png"))
    );
    assert_eq!(
        copy_into(&folder, &b).unwrap(),
        Adopted::Copied(folder.join("oak (2).png"))
    );
    assert_eq!(
        copy_into(&folder, &b).unwrap(),
        Adopted::Reused(folder.join("oak (2).png"))
    );
    assert_eq!(
        std::fs::read(folder.join("oak (2).png")).unwrap(),
        b"second"
    );
    // The originals are untouched, and no temporary is left.
    assert_eq!(std::fs::read(&a).unwrap(), b"first");
    assert_eq!(std::fs::read(&b).unwrap(), b"second");
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 2);
    assert!(copy_into(&folder, &base.join("nope.png")).is_err());
}
