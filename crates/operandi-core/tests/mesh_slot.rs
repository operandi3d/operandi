//! Packed mesh storage, stage 2: an item's `MeshSlot` behaves like the
//! `Shared<Mesh>` it replaces, whether it is packed or open.
//!
//! `read_opens` is a process-wide counter and tests run in parallel, so
//! tests that check it run in one test function.
use glam::DVec3;
use operandi_core::mesh_slot::read_opens;
use operandi_core::*;

fn grid(n: u32) -> Mesh {
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices
                .push(DVec3::new(i as f64, j as f64, ((i * j) % 5) as f64 * 0.1));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: "Default".into(),
                smooth: true,
                part: None,
                material_label: None,
            });
        }
    }
    mesh
}

#[test]
fn packing_keeps_contents_and_stamp_and_counts_hidden_opens() {
    let mesh = grid(40);
    let mut slot = MeshSlot::new(mesh.clone());
    let stamp = slot.stamp();
    assert!(slot.pack(), "1,600 polygons pack");
    assert!(slot.is_packed());
    assert_eq!(slot.stamp(), stamp, "packing keeps the stamp");

    // read() looks without opening.
    let opens = read_opens();
    match slot.read() {
        MeshView::Packed(p) => assert_eq!(p.polygon_count(), 1600),
        MeshView::Open(_) => panic!("slot should be packed"),
    }
    assert_eq!(slot.read().as_read().point_count(), mesh.vertices.len());
    assert_eq!(read_opens(), opens);

    // Reading through Deref opens it, counted, and keeps the packed form.
    assert_eq!(slot.faces.len(), 1600);
    assert_eq!(read_opens(), opens + 1);
    assert!(!slot.is_packed() && slot.has_packed());
    assert_eq!(slot.stamp(), stamp);
    assert_eq!(*slot, mesh);
    // A second read reuses the open copy.
    let _ = slot.vertices.len();
    assert_eq!(read_opens(), opens + 1);

    // open() is the deliberate open for the mesh being edited: not counted.
    let mut slot = MeshSlot::new(mesh.clone());
    slot.pack();
    assert_eq!(slot.open().faces.len(), 1600);
    assert_eq!(read_opens(), opens + 1);

    // Writing opens without counting, drops the packed form, new stamp.
    let mut slot = MeshSlot::new(mesh.clone());
    slot.pack();
    slot.vertices[0].z = 9.0;
    assert_eq!(read_opens(), opens + 1);
    assert!(!slot.has_packed());
    assert_ne!(slot.stamp(), stamp);
    assert_eq!(slot.vertices[0].z, 9.0);
    assert_eq!(slot.vertices[1], mesh.vertices[1]);
}

#[test]
fn small_meshes_stay_open() {
    let mut slot = MeshSlot::new(Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap());
    assert!(!slot.pack());
    assert!(!slot.is_packed());
}

#[test]
fn a_copy_shares_until_one_side_changes() {
    let mut a = MeshSlot::new(grid(8));
    let b = a.clone();
    assert!(MeshSlot::same(&a, &b));
    assert!(
        Shared::same(a.shared(), b.shared()),
        "no copy made by cloning"
    );
    a.vertices[0].x = -1.0;
    assert!(!MeshSlot::same(&a, &b));
    assert_eq!(b.vertices[0].x, 0.0, "the other copy is unchanged");
    assert_ne!(a, b);
}

/// The I2 test scene plus a second item holding a 1,600-quad grid.
fn editor_with_big_untouched_item() -> Editor {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/i2/expectations/graphs/G0.json");
    let mut scene: Scene = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut other = scene.items[0].clone();
    other.id = scene.next_id;
    other.name = "Big".into();
    other.mesh = grid(40).into();
    other.elements = Default::default();
    scene.next_id += 1;
    scene.items.push(other);
    let mut e = Editor {
        scene,
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    e
}

#[test]
fn a_packed_item_survives_editing_another_with_undo_and_redo() {
    let mut e = editor_with_big_untouched_item();
    let original = e.scene.items[1].mesh.to_owned_value();
    assert!(e.scene.items[1].mesh.pack());
    let stamp = e.scene.items[1].mesh.stamp();
    let p = |z: f64| Parameters {
        extent: DVec3::Z * z,
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    };
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(p(0.))).unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    e.dispatch(Command::Preview(p(0.5))).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let edited = e.scene.items[0].mesh.to_owned_value();
    e.dispatch(Command::Undo).unwrap();
    assert_ne!(e.scene.items[0].mesh, edited);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.items[0].mesh, edited);
    let big = &e.scene.items[1].mesh;
    assert_eq!(big.stamp(), stamp, "the untouched item never changed");
    assert!(big.has_packed(), "nothing wrote to it");
    assert_eq!(*big, original);
}

#[test]
fn packed_slots_save_exactly_like_open_ones() {
    let mut scene = editor_with_big_untouched_item().scene;
    let open = persistence::encode(&scene).unwrap();
    assert!(scene.items[1].mesh.pack());
    assert!(scene.items[1].mesh.is_packed());
    let packed = persistence::encode(&scene).unwrap();
    assert_eq!(open, packed, "the file doesn't change");
    let reopened = persistence::decode(&packed).unwrap();
    assert_eq!(reopened.items[1].mesh, scene.items[1].mesh);
    assert!(reopened.items[1].mesh.validate().is_ok());
}
