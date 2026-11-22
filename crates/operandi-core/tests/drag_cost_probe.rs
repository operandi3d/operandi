//! Opt-in probe: time per Move preview frame on a large mesh (core only).
use glam::DVec3;
use operandi_core::*;

#[test]
#[ignore = "opt-in timing probe"]
fn move_preview_frame_cost() {
    let n = 128u32;
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices
                .push(DVec3::new(i as f64, j as f64, ((i + j) % 3) as f64 * 0.1));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![n * (n + 1) / 2];
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let t = std::time::Instant::now();
    let frames: usize = std::env::var("PROBE_FRAMES")
        .ok()
        .and_then(|f| f.parse().ok())
        .unwrap_or(20);
    for k in 0..frames {
        let p = Parameters {
            delta: DVec3::new(0., 0.01 * k as f64, 0.),
            ..Default::default()
        };
        e.dispatch(Command::Preview(p)).unwrap();
    }
    println!(
        "PREVIEW ms/frame {:.2}",
        t.elapsed().as_secs_f64() * 1000. / frames as f64
    );
    let m = e.scene.active().unwrap().mesh.clone();
    let t = std::time::Instant::now();
    for _ in 0..frames {
        m.validate().unwrap();
    }
    println!(
        "VALIDATE ms {:.2}",
        t.elapsed().as_secs_f64() * 1000. / frames as f64
    );
    let t = std::time::Instant::now();
    for _ in 0..frames {
        std::hint::black_box(m.edges());
    }
    println!(
        "EDGES ms {:.2}",
        t.elapsed().as_secs_f64() * 1000. / frames as f64
    );
}
