//! Packed mesh storage, stage 1: packing is exact for every mesh shape the
//! app makes, and `MeshRead` answers the same for both forms.
use glam::{DVec2, DVec3};
use operandi_core::packed_mesh::{MeshRead, PackedMesh, UvMapRef, worth_packing};
use operandi_core::*;
use std::path::{Path, PathBuf};

/// Field-by-field comparison with floats compared by their bits, so signed
/// zeros and NaN payloads must survive too.
fn assert_bit_identical(a: &Mesh, b: &Mesh, what: &str) {
    let bits3 = |v: &DVec3| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()];
    let bits2 = |v: &DVec2| [v.x.to_bits(), v.y.to_bits()];
    assert_eq!(a.vertices.len(), b.vertices.len(), "{what}: point count");
    assert!(
        a.vertices
            .iter()
            .zip(&b.vertices)
            .all(|(x, y)| bits3(x) == bits3(y)),
        "{what}: points differ"
    );
    assert_eq!(a.faces.len(), b.faces.len(), "{what}: polygon count");
    for (i, (x, y)) in a.faces.iter().zip(&b.faces).enumerate() {
        assert_eq!(x.vertices, y.vertices, "{what}: polygon {i} corners");
        assert_eq!(x.material, y.material, "{what}: polygon {i} material");
        assert_eq!(x.smooth, y.smooth, "{what}: polygon {i} smooth");
        assert_eq!(x.part, y.part, "{what}: polygon {i} part");
        assert_eq!(
            x.material_label, y.material_label,
            "{what}: polygon {i} label"
        );
        assert_eq!(
            x.uv.as_ref()
                .map(|u| u.iter().map(bits2).collect::<Vec<_>>()),
            y.uv.as_ref()
                .map(|u| u.iter().map(bits2).collect::<Vec<_>>()),
            "{what}: polygon {i} face UVs"
        );
    }
    assert_eq!(
        a.uv_maps.keys().collect::<Vec<_>>(),
        b.uv_maps.keys().collect::<Vec<_>>(),
        "{what}: UV map names"
    );
    for (name, polygons) in &a.uv_maps {
        let other = &b.uv_maps[name];
        let flat = |p: &Vec<Vec<Option<DVec2>>>| -> Vec<Vec<Option<[u64; 2]>>> {
            p.iter()
                .map(|c| c.iter().map(|v| v.as_ref().map(bits2)).collect())
                .collect()
        };
        assert_eq!(flat(polygons), flat(other), "{what}: UV map {name}");
    }
    assert_eq!(a.attributes, b.attributes, "{what}: attributes");
}

/// Every reader answer is the same for the full and the packed form.
fn assert_reads_agree(full: &Mesh, packed: &PackedMesh, what: &str) {
    assert_eq!(full.point_count(), packed.point_count(), "{what}");
    // By bits, so a NaN point compares equal to itself.
    let bits = |p: &[DVec3]| {
        p.iter()
            .flat_map(|v| v.to_array().map(f64::to_bits))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        bits(MeshRead::points(full)),
        bits(packed.points()),
        "{what}"
    );
    assert_eq!(full.polygon_count(), packed.polygon_count(), "{what}");
    assert_eq!(full.corner_count(), packed.corner_count(), "{what}");
    assert_eq!(full.uv_map_names(), packed.uv_map_names(), "{what}");
    for f in 0..full.polygon_count() {
        assert_eq!(full.polygon(f), packed.polygon(f), "{what}: polygon {f}");
        assert_eq!(full.material(f), packed.material(f), "{what}: polygon {f}");
        assert_eq!(full.part(f), packed.part(f), "{what}: polygon {f}");
        assert_eq!(full.material_label(f), packed.material_label(f), "{what}");
        assert_eq!(full.smooth(f), packed.smooth(f), "{what}: polygon {f}");
        assert_eq!(full.has_face_uv(f), packed.has_face_uv(f), "{what}");
        // One past the last corner must read as absent in both forms.
        for c in 0..=full.polygon(f).len() {
            assert_eq!(full.face_uv(f, c), packed.face_uv(f, c), "{what}");
            for name in full.uv_map_names() {
                let (a, b) = (full.uv_map(name).unwrap(), packed.uv_map(name).unwrap());
                assert_eq!(a.get(f, c), b.get(f, c), "{what}: map {name} {f}/{c}");
            }
        }
    }
    assert!(packed.uv_map("no such map").is_none());
    assert_eq!(full.attributes(), packed.attributes(), "{what}");
}

fn round_trip(mesh: &Mesh, what: &str) -> PackedMesh {
    let packed = PackedMesh::from_mesh(mesh).unwrap_or_else(|e| panic!("{what}: {e}"));
    assert_bit_identical(mesh, &packed.to_mesh(), what);
    assert_reads_agree(mesh, &packed, what);
    // Packing the unpacked copy again gives the same contents.
    assert_bit_identical(
        &packed.to_mesh(),
        &PackedMesh::from_mesh(&packed.to_mesh()).unwrap().to_mesh(),
        what,
    );
    packed
}

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: true,
        part: None,
        material_label: None,
    }
}

fn grid(n: u32, uv_map: bool) -> Mesh {
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(
                i as f64 * 0.1,
                j as f64 * 0.1,
                ((i + j) % 3) as f64 * 0.01,
            ));
        }
    }
    let mut uvs = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            let corners = [a, a + 1, a + n + 2, a + n + 1];
            mesh.faces.push(face(corners.to_vec()));
            uvs.push(
                corners
                    .iter()
                    .map(|&p| {
                        Some(DVec2::new(
                            (p % (n + 1)) as f64 / n as f64,
                            (p / (n + 1)) as f64 / n as f64,
                        ))
                    })
                    .collect(),
            );
        }
    }
    if uv_map {
        mesh.uv_maps.insert("Texture".into(), uvs);
    }
    mesh.attributes.representations = vec![PolygonRepresentation::CatmullClark; mesh.faces.len()];
    mesh
}

fn scene_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            scene_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "opscene") {
            out.push(path);
        }
    }
}

#[test]
fn every_fixture_mesh_round_trips_bit_for_bit() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let mut files = Vec::new();
    scene_files(&fixtures, &mut files);
    assert!(!files.is_empty(), "no fixture scenes found");
    let mut meshes = 0;
    for file in &files {
        let scene = persistence::load(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        for item in &scene.items {
            round_trip(&item.mesh, &format!("{} item {}", file.display(), item.id));
            meshes += 1;
        }
    }
    let obj = interchange::import(&fixtures.join("uv-plane.obj")).unwrap();
    for item in &obj.scene.items {
        round_trip(&item.mesh, "uv-plane.obj");
        meshes += 1;
    }
    assert!(meshes > 0);
}

#[test]
fn primitives_round_trip() {
    round_trip(&Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap(), "box");
    round_trip(&cone::cone_geometry(&Default::default()).unwrap(), "cone");
    round_trip(
        &ellipsoid::ellipsoid_geometry(&Default::default()).unwrap(),
        "sphere",
    );
    round_trip(&grid(128, true), "16k grid with UVs");
    round_trip(&Mesh::default(), "empty mesh");
}

#[test]
fn mixed_polygons_tags_and_sparse_uvs_round_trip() {
    let mut mesh = Mesh {
        vertices: (0..8)
            .map(|i| DVec3::new(i as f64, (i * i) as f64, 0.5))
            .collect(),
        ..Mesh::default()
    };
    // Signed zero and a NaN with a payload: both must come back unchanged.
    mesh.vertices[0] = DVec3::new(-0.0, 0.0, f64::from_bits(0x7ff8_0000_dead_beef));
    let mut tri = face(vec![0, 1, 2]);
    tri.uv = Some(vec![DVec2::new(0.0, -0.0), DVec2::X, DVec2::Y]);
    tri.part = Some("Lid".into());
    let mut quad = face(vec![2, 3, 4, 5]);
    quad.material = String::new();
    quad.smooth = false;
    quad.material_label = Some("Painted".into());
    let mut ngon = face(vec![0, 2, 4, 6, 7]);
    ngon.material = "Lid".into(); // same text as a part name: shares one table entry
    ngon.part = Some(String::new());
    let line = face(vec![6, 7]);
    mesh.faces = vec![tri, quad, ngon, line].into();
    let sparse = vec![
        vec![Some(DVec2::ZERO), None, Some(DVec2::ONE)],
        vec![None; 4],
        vec![Some(DVec2::splat(0.25)); 5],
        vec![None, Some(DVec2::NEG_ONE)],
    ];
    mesh.uv_maps.insert("Sparse".into(), sparse);
    mesh.uv_maps.insert(
        "Full".into(),
        mesh.faces
            .iter()
            .map(|f| {
                f.vertices
                    .iter()
                    .map(|&v| Some(DVec2::splat(v as f64)))
                    .collect()
            })
            .collect(),
    );
    mesh.attributes.creases.push(MeshCrease {
        a: 0,
        b: 1,
        weight: 0.5,
    });
    mesh.attributes
        .vertex_sets
        .insert("Top".into(), [1, 2, 3].into_iter().collect());
    round_trip(&mesh, "mixed mesh");
}

#[test]
fn uv_maps_with_mostly_unique_values_round_trip() {
    // Every corner its own value, with a few unset: the per-corner layout.
    let mut mesh = grid(40, false);
    let mut k = 0.0;
    let uvs = mesh
        .faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|_| {
                    k += 1.0;
                    (!(k as u64).is_multiple_of(97)).then(|| DVec2::new(k, -k))
                })
                .collect()
        })
        .collect();
    mesh.uv_maps.insert("Unique".into(), uvs);
    // Signed zeros are different bits, so they must not share a table entry.
    let mut zeros = grid(4, true);
    zeros.uv_maps.get_mut("Texture").unwrap()[0][0] = Some(DVec2::new(-0.0, 0.0));
    round_trip(&mesh, "unique UVs");
    round_trip(&zeros, "signed-zero UVs");
}

#[test]
fn more_than_65535_names_use_wide_indices() {
    let mesh = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::Y],
        faces: (0..70_000)
            .map(|i| {
                let mut f = face(vec![0, 1, 2]);
                f.material = format!("M{i}");
                f.part = (i % 2 == 0).then(|| format!("P{i}"));
                f
            })
            .collect(),
        ..Mesh::default()
    };
    let packed = round_trip(&mesh, "70k names");
    assert_eq!(packed.material(69_999), "M69999");
    assert_eq!(packed.part(69_998), Some("P69998"));
    assert_eq!(packed.part(69_999), None);
}

#[test]
fn moving_points_shares_polygons_and_uvs() {
    let packed = PackedMesh::from_mesh(&grid(40, true)).unwrap();
    let moved_points: Vec<DVec3> = packed.points().iter().map(|p| *p + DVec3::Z).collect();
    let moved = packed.with_points(moved_points).unwrap();
    assert!(PackedMesh::shares_polygons(&packed, &moved));
    assert!(PackedMesh::shares_uvs(&packed, &moved));
    assert_eq!(moved.point(0), packed.point(0) + DVec3::Z);
    // A fresh pack of equal contents shares nothing.
    let repacked = PackedMesh::from_mesh(&packed.to_mesh()).unwrap();
    assert!(!PackedMesh::shares_polygons(&packed, &repacked));
    assert!(
        packed.with_points(vec![DVec3::ZERO]).is_err(),
        "point count must stay"
    );
}

#[test]
fn uv_lists_that_do_not_match_their_polygons_are_refused() {
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    mesh.faces[0].uv = Some(vec![DVec2::ZERO]);
    assert!(PackedMesh::from_mesh(&mesh).is_err());
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    mesh.uv_maps.insert("Short".into(), vec![vec![None; 4]]);
    assert!(PackedMesh::from_mesh(&mesh).is_err());
}

#[test]
fn only_meshes_of_1024_polygons_or_more_pack() {
    assert!(!worth_packing(&grid(31, false)), "961 polygons stay open");
    assert!(worth_packing(&grid(32, false)), "1,024 polygons pack");
}

#[test]
fn uv_reader_ignores_out_of_range_polygons() {
    let mesh = grid(2, true);
    let packed = PackedMesh::from_mesh(&mesh).unwrap();
    for reader in [
        mesh.uv_map("Texture").unwrap(),
        packed.uv_map("Texture").unwrap(),
    ] {
        assert_eq!(reader.get(99, 0), None);
        assert!(matches!(
            reader,
            UvMapRef::Full(_) | UvMapRef::Packed { .. }
        ));
    }
}
