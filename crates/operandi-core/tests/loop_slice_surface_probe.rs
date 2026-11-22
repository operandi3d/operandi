//! Opt-in probe: the subdivision update cost of one Loop Slice drag frame on
//! a 16k-quad Catmull-Clark grid (the whole 128-cell ring moves each frame).
use glam::DVec3;
use operandi_core::subdivision::{
    SubdivisionSettings, display_surface, source_blocks, update_surface,
};
use operandi_core::*;

fn grid(n: u32) -> Mesh {
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (x, y) = (i as f64 / n as f64 - 0.5, j as f64 / n as f64 - 0.5);
            mesh.vertices
                .push(DVec3::new(x, y, 0.1 * (x * 9.).sin() * (y * 7.).cos()));
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
    mesh.attributes.representations = vec![PolygonRepresentation::CatmullClark; mesh.faces.len()];
    mesh
}

fn sliced(base: &Mesh, ring: (u32, u32), t: f64) -> Mesh {
    let o = LoopSlice {
        positions: vec![t],
        rings: vec![ring],
        ..Default::default()
    };
    slice::loop_slice_geometry(base, &o, &[]).unwrap().0
}

#[test]
#[ignore = "opt-in timing probe"]
fn loop_slice_surface_update_cost() {
    let n = 128u32;
    let mid = (n / 2) * (n + 1) + n / 2;
    let base = grid(n);
    let settings = SubdivisionSettings::default();
    let frames: Vec<Mesh> = (0..40)
        .map(|k| sliced(&base, (mid, mid + 1), 0.3 + 0.01 * k as f64))
        .collect();
    let (mut surface, sources) = display_surface(&frames[0], &settings).unwrap();
    let blocks = source_blocks(&sources, frames[0].faces.len());
    let rounds = 200;
    let t = std::time::Instant::now();
    for k in 1..=rounds {
        let (old, new) = (&frames[(k - 1) % 40], &frames[k % 40]);
        std::hint::black_box(update_surface(old, new, &settings, &mut surface, &blocks).unwrap());
    }
    let ms = t.elapsed().as_secs_f64() * 1000. / rounds as f64;
    println!("PROBE loop_slice update_surface: {ms:.2} ms/frame");
    let t = std::time::Instant::now();
    for k in 1..=20 {
        std::hint::black_box(display_surface(&frames[k % 40], &settings).unwrap());
    }
    let ms = t.elapsed().as_secs_f64() * 1000. / 20.;
    println!("PROBE full display_surface: {ms:.2} ms");
}
