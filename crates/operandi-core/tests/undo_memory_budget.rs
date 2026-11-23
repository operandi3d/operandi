//! Hidden-limits sweep A1: the undo history counts the mesh data its steps
//! keep, so edits of a big mesh trim old steps instead of filling memory.
use glam::DVec3;
use operandi_core::*;

/// A flat grid of `n` x `n` quads as the only mesh, in Polygon mode.
fn grid(n: u32) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut mesh = Mesh::default();
    for y in 0..=n {
        for x in 0..=n {
            mesh.vertices.push(DVec3::new(x as f64, 0., y as f64));
        }
    }
    let row = n + 1;
    let faces: Vec<Face> = (0..n)
        .flat_map(|y| (0..n).map(move |x| (x, y)))
        .map(|(x, y)| Face {
            vertices: vec![
                y * row + x,
                y * row + x + 1,
                (y + 1) * row + x + 1,
                (y + 1) * row + x,
            ],
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        })
        .collect();
    mesh.faces = faces.into();
    e.scene.active_mut().unwrap().mesh = MeshSlot::new(mesh);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.undo.clear();
    e
}

#[test]
fn undo_history_counts_mesh_data_and_trims_to_its_budget() {
    let mut e = grid(100); // 10,000 polygons, about a MB per changed copy
    e.dispatch(Command::FlipPolygons).unwrap();
    let one_step = e.undo_memory_bytes();
    assert!(
        one_step > 10_000 * std::mem::size_of::<Face>(),
        "a step that replaced the polygons is charged for them: {one_step}"
    );
    e.undo_memory_budget = Some(one_step * 5 + one_step / 2);
    for _ in 0..20 {
        e.dispatch(Command::FlipPolygons).unwrap();
    }
    assert!(
        e.undo.len() < 10,
        "old steps were trimmed: {}",
        e.undo.len()
    );
    assert!(
        e.undo.len() >= 4,
        "but not more than needed: {}",
        e.undo.len()
    );
    assert!(e.undo_memory_bytes() <= e.undo_memory_budget.unwrap());
    // The kept steps still undo.
    let n = e.undo.len();
    for _ in 0..n {
        e.dispatch(Command::Undo).unwrap();
    }
    assert!(e.undo.is_empty());
}

#[test]
fn shared_data_is_counted_once() {
    // Item edits that leave the mesh alone keep sharing it: the history
    // then holds almost nothing beyond the scene.
    let mut e = grid(100);
    for k in 0..10 {
        e.dispatch(Command::Rename(format!("Grid {k}"))).unwrap();
    }
    assert!(e.undo.len() >= 10);
    assert!(
        e.undo_memory_bytes() < 1 << 20,
        "renames share the mesh: {}",
        e.undo_memory_bytes()
    );
}

#[test]
fn free_memory_caps_the_history() {
    let mut e = grid(100);
    e.dispatch(Command::FlipPolygons).unwrap();
    let one_step = e.undo_memory_bytes();
    // Little memory free: the history may take half of what it and the
    // free memory add up to, keeping at least the newest step.
    e.free_memory = FreeMemory(Some(|| 1));
    for _ in 0..10 {
        e.dispatch(Command::FlipPolygons).unwrap();
    }
    assert!(
        e.undo.len() <= 2,
        "{} steps of {one_step} bytes",
        e.undo.len()
    );
}
