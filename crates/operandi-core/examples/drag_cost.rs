//! Headless drag-sample cost: one `Command::Preview` dispatch during a legacy
//! Extrude preview and during an I2 Begin/Move stage, over the `G0` fixture
//! with synthetic history and a generated flat grid mesh. Original generated
//! geometry only; never user scenes.
//!
//! Run in release: `cargo run --release -p operandi-core --example drag_cost`
//! - default: the grid is a second, untouched item (cost should not depend on it)
//! - `-- edited`: the grid is the extruded mesh itself (cost follows that mesh)
//!
//! Single-machine wall-clock means over a few samples; compare runs on the same
//! hardware only.
use glam::DVec3;
use operandi_core::*;
use std::time::Instant;

fn params(z: f64) -> Parameters {
    Parameters {
        extent: DVec3::Z * z,
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }
}
/// `n` x `n` planar quads facing +Z, offset clear of the G0 cube.
fn grid(n: usize) -> Mesh {
    let mut mesh = Mesh::default();
    for y in 0..=n {
        for x in 0..=n {
            mesh.vertices.push(DVec3::new(x as f64 + 10., y as f64, 0.));
        }
    }
    let w = (n + 1) as u32;
    for y in 0..n as u32 {
        for x in 0..n as u32 {
            let a = y * w + x;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + 1 + w, a + w],
                uv: None,
                material: "Default".into(),
                smooth: true,
                part: None,
                material_label: None,
            });
        }
    }
    mesh.uv_maps.insert(
        "Texture".into(),
        mesh.faces
            .iter()
            .map(|f| vec![None; f.vertices.len()])
            .collect(),
    );
    mesh
}
fn editor(grid_size: usize, history: usize, edit_grid: bool) -> Editor {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = std::fs::read(root.join("docs/i2/expectations/graphs/G0.json")).unwrap();
    let mut scene: Scene = serde_json::from_slice(&fixture).unwrap();
    if edit_grid {
        scene.items[0].mesh = grid(grid_size).into();
        scene.selection.faces = vec![0];
        scene.selection.vertices.clear();
        scene.selection.edges.clear();
    } else if grid_size > 0 {
        let mut other = scene.items[0].clone();
        other.id = scene.next_id;
        other.name = "Grid".into();
        other.mesh = grid(grid_size).into();
        scene.next_id += 1;
        scene.items.push(other);
    }
    let mut e = Editor {
        scene,
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    for i in 0..history {
        let s = e.scene.clone();
        e.undo.push(std::sync::Arc::new(Checkpoint {
            identity_before: None,
            identity_after: None,
            label: format!("h{i}"),
            before: s.clone(),
            after: s,
            fold: None,
            records: Default::default(),
        }));
    }
    // One ordinary command assigns the synthetic checkpoints their identities.
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(params(0.))).unwrap();
    e
}
fn mean_ms(iterations: usize, mut sample: impl FnMut(usize)) -> f64 {
    let start = Instant::now();
    for i in 0..iterations {
        sample(i);
    }
    start.elapsed().as_secs_f64() * 1e3 / iterations as f64
}
fn main() {
    let edit_grid = std::env::args().any(|a| a == "edited");
    let label = if edit_grid {
        "edited_mesh_faces"
    } else {
        "untouched_mesh_faces"
    };
    println!("{label},history,legacy_preview_ms,i2_move_ms,editor_clone_ms");
    for grid_size in [0usize, 100, 300] {
        if edit_grid && grid_size == 0 {
            continue;
        }
        for history in [0usize, 20, 90] {
            let iterations = if grid_size >= 300 { 5 } else { 30 };
            let mut legacy = editor(grid_size, history, edit_grid);
            let legacy_ms = mean_ms(iterations, |i| {
                legacy
                    .dispatch(Command::Preview(params(0.1 + i as f64 * 0.01)))
                    .unwrap()
            });
            let mut staged = editor(grid_size, history, edit_grid);
            staged.dispatch(Command::BeginExtrude).unwrap();
            assert!(staged.i2_active());
            let i2_ms = mean_ms(iterations, |i| {
                staged
                    .dispatch(Command::Preview(params(0.1 + i as f64 * 0.01)))
                    .unwrap()
            });
            let clone_ms = mean_ms(iterations, |_| {
                std::hint::black_box(staged.clone());
            });
            println!(
                "{},{history},{legacy_ms:.3},{i2_ms:.3},{clone_ms:.3}",
                grid_size * grid_size
            );
        }
    }
}
