//! Opt-in memory probe for packed mesh storage. Measures real heap use with
//! a counting allocator, for the 16k-quad grid and a 2M-quad mesh, each with
//! one UV map and Catmull-Clark representations:
//!
//! `cargo test --release -p operandi-core --test packed_mesh_probe -- --ignored --nocapture`
use glam::{DVec2, DVec3};
use operandi_core::packed_mesh::full_heap_bytes;
use operandi_core::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};

/// Counts live heap bytes. Allocation goes straight to the system
/// allocator; the counter is the only addition.
struct Counting;
static LIVE: AtomicIsize = AtomicIsize::new(0);
// SAFETY: every method forwards to `System` with the caller's arguments
// unchanged, so `System`'s guarantees carry over; the counter has no effect
// on the memory handed out.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        // SAFETY: same layout the caller passed, which it guarantees valid.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        // SAFETY: `ptr` came from `alloc` above with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_add(
            new_size as isize - layout.size() as isize,
            Ordering::Relaxed,
        );
        // SAFETY: forwarded unchanged; the caller upholds `realloc`'s rules.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
#[global_allocator]
static ALLOC: Counting = Counting;

fn live() -> isize {
    LIVE.load(Ordering::Relaxed)
}

/// An `n` x `n` quad grid with one full UV map. Built straight into its
/// final Vecs so measured capacity matches what a loaded mesh holds.
fn grid(n: u32) -> Mesh {
    let side = n + 1;
    let mut mesh = Mesh {
        vertices: Vec::with_capacity((side * side) as usize),
        faces: Vec::with_capacity((n * n) as usize).into(),
        ..Mesh::default()
    };
    for j in 0..side {
        for i in 0..side {
            mesh.vertices.push(DVec3::new(
                i as f64 * 0.01,
                j as f64 * 0.01,
                ((i + j) % 3) as f64 * 0.001,
            ));
        }
    }
    let mut uvs = Vec::with_capacity((n * n) as usize);
    for j in 0..n {
        for i in 0..n {
            let a = j * side + i;
            let corners = vec![a, a + 1, a + side + 1, a + side];
            uvs.push(
                corners
                    .iter()
                    .map(|&p| {
                        Some(DVec2::new(
                            (p % side) as f64 / n as f64,
                            (p / side) as f64 / n as f64,
                        ))
                    })
                    .collect(),
            );
            mesh.faces.push(Face {
                vertices: corners,
                uv: None,
                material: "Default".into(),
                smooth: true,
                part: None,
                material_label: None,
            });
        }
    }
    mesh.uv_maps.insert("Texture".into(), uvs);
    mesh.attributes.representations = vec![PolygonRepresentation::CatmullClark; mesh.faces.len()];
    mesh
}

fn mb(bytes: isize) -> String {
    format!("{:.1} MB", bytes as f64 / 1e6)
}

fn probe(name: &str, n: u32) {
    let before = live();
    let mesh = grid(n);
    let full = live() - before;
    let quads = mesh.faces.len() as isize;
    let attributes = (mesh.attributes.representations.capacity()
        * std::mem::size_of::<PolygonRepresentation>()) as isize;

    let before = live();
    let packed = PackedMesh::from_mesh(&mesh).unwrap();
    let packed_bytes = live() - before;

    // An undo step that only moves points: the full form copies the whole
    // mesh, the packed form only a new points array.
    let before = live();
    let mut moved_full = mesh.clone();
    moved_full.vertices[0].z += 1.0;
    let full_step = live() - before;
    let before = live();
    let moved_packed = packed.with_points(packed.points().to_vec()).unwrap();
    let packed_step = live() - before;
    assert!(PackedMesh::shares_polygons(&packed, &moved_packed));

    let t = std::time::Instant::now();
    let reopened = packed.to_mesh();
    let unpack_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = std::time::Instant::now();
    let repacked = PackedMesh::from_mesh(&reopened).unwrap();
    let pack_ms = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(reopened, mesh);

    println!("{name}: {quads} quads, {} points", mesh.vertices.len());
    println!(
        "  full mesh, measured        {:>10}  ({} B/quad)",
        mb(full),
        full / quads
    );
    println!(
        "    of which attributes      {:>10}  (kept as is in both forms)",
        mb(attributes)
    );
    println!(
        "    estimate (struct sizes)  {:>10}",
        mb(full_heap_bytes(&mesh) as isize + attributes)
    );
    println!(
        "  packed, measured           {:>10}  ({} B/quad, {:.1}x smaller)",
        mb(packed_bytes),
        packed_bytes / quads,
        full as f64 / packed_bytes as f64
    );
    println!(
        "    estimate (heap_bytes)    {:>10}",
        mb(packed.heap_bytes() as isize + attributes)
    );
    println!(
        "  points-only undo step      {:>10} full, {} packed ({:.1}x)",
        mb(full_step),
        mb(packed_step),
        full_step as f64 / packed_step as f64
    );
    println!("  unpack {unpack_ms:.1} ms, pack {pack_ms:.1} ms");
    drop((moved_full, moved_packed, repacked));
}

#[test]
#[ignore = "opt-in memory probe"]
fn packed_mesh_memory() {
    probe("16k grid", 128);
    probe("2M-quad mesh", 1415);
}
