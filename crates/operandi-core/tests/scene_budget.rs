//! Scenes far past the old 10,000-item cap (scene-budget, 2026-11-20):
//! 100,000 items build, validate, save, reopen, undo and redo, and adding
//! one more costs about the same as in a small scene.
use glam::DVec3;
use operandi_core::{Command, Editor, Face, MAX_SCENE_ITEMS, Mesh, Scene, Shared, persistence};
use std::time::Instant;

const ITEMS: usize = 100_000;

/// One quad, shared by every item as imported instances' meshes are.
fn quad() -> Shared<Mesh> {
    Shared::new(Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.), DVec3::Y],
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    })
}

/// `n` mesh items, every hundredth one the parent of the 99 after it.
fn big_scene(n: usize) -> Scene {
    let mesh = quad();
    let mut scene = Scene::default();
    let mut parent = None;
    for k in 0..n {
        scene.new_mesh_named(Some(format!("Item {k}"))).unwrap();
        let item = scene.items.last_mut().unwrap();
        item.mesh = operandi_core::MeshSlot::new(mesh.clone());
        item.transform.translation = DVec3::new(k as f64 * 0.01, 0., 0.);
        if k % 100 == 0 {
            parent = Some(item.id);
        } else {
            item.parent = parent;
        }
    }
    scene
}

fn secs(t: Instant) -> f64 {
    t.elapsed().as_secs_f64()
}

#[test]
fn a_hundred_thousand_items_save_reopen_undo_and_redo() {
    const { assert!(MAX_SCENE_ITEMS >= 1_000_000) };
    let t = Instant::now();
    let scene = big_scene(ITEMS);
    let built = secs(t);
    let t = Instant::now();
    scene.validate().unwrap();
    let validated = secs(t);

    // Save and reopen: the same scene comes back.
    let t = Instant::now();
    let bytes = persistence::encode(&scene).unwrap();
    let saved = secs(t);
    let t = Instant::now();
    let back = persistence::decode(&bytes).unwrap();
    let loaded = secs(t);
    assert_eq!(back.items.len(), ITEMS);
    assert_eq!(back, scene);

    // Undo and redo a new mesh at the end of the list, and a selection.
    let mut e = Editor {
        scene,
        ..Default::default()
    };
    let last = e.scene.items.last().unwrap().id;
    e.dispatch(Command::SelectItem(Some(last))).unwrap();
    assert_eq!(e.scene.selection.item, Some(last));
    let t = Instant::now();
    e.dispatch(Command::NewMesh).unwrap();
    let new_mesh = secs(t);
    assert_eq!(e.scene.items.len(), ITEMS + 1);
    // The next free "Mesh" name, found in one pass over the list.
    assert_eq!(e.scene.items.last().unwrap().name, "Mesh");
    let t = Instant::now();
    e.dispatch(Command::Undo).unwrap();
    let undone = secs(t);
    assert_eq!(e.scene.items.len(), ITEMS);
    assert_eq!(e.scene, back);
    let t = Instant::now();
    e.dispatch(Command::Redo).unwrap();
    let redone = secs(t);
    assert_eq!(e.scene.items.len(), ITEMS + 1);
    // Every lookup by id still answers after the list grew.
    assert_eq!(e.scene.item_index(last), Some(ITEMS - 1));
    eprintln!(
        "{ITEMS} items: build {built:.2} s, validate {validated:.2} s, \
         save {saved:.2} s ({:.1} MB), load {loaded:.2} s, new mesh {new_mesh:.3} s, \
         undo {undone:.3} s, redo {redone:.3} s; Item is {} bytes",
        bytes.len() as f64 / 1e6,
        std::mem::size_of::<operandi_core::Item>()
    );
}

#[test]
fn free_names_skip_the_taken_numbers_in_one_pass() {
    let mut scene = Scene::default();
    for _ in 0..3 {
        scene.new_mesh().unwrap();
    }
    let names: Vec<&str> = scene.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["Mesh", "Mesh (2)", "Mesh (3)"]);
    // A gap is filled first; look-alikes ("Mesh (02)", "Mesh (x)") don't
    // count as numbers.
    scene.items[1].name = "Mesh (02)".into();
    scene.new_mesh_named(Some("Mesh (x)".into())).unwrap();
    scene.new_mesh().unwrap();
    assert_eq!(scene.items.last().unwrap().name, "Mesh (2)");
    scene.new_mesh().unwrap();
    assert_eq!(scene.items.last().unwrap().name, "Mesh (4)");
}

#[test]
fn undo_keeps_the_newest_steps_that_fit_its_memory_budget() {
    let mut e = Editor {
        scene: big_scene(50),
        ..Default::default()
    };
    for _ in 0..6 {
        e.dispatch(Command::NewMesh).unwrap();
    }
    assert_eq!(e.undo.len(), 6);
    // Each step keeps two copies of about 55 items.
    let step = 2 * 55 * std::mem::size_of::<operandi_core::Item>();
    e.trim_undo_memory_to(3 * step);
    assert!((2..=3).contains(&e.undo.len()), "{}", e.undo.len());
    assert_eq!(e.undo.last().unwrap().after.items.len(), 56);
    // The newest step always stays.
    e.trim_undo_memory_to(0);
    assert_eq!(e.undo.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items.len(), 55);
    const { assert!(operandi_core::UNDO_MEMORY_BUDGET >= 1 << 30) };
}
