use super::*;
use std::path::PathBuf;

/// A fresh scratch folder per test.
fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "operandi-interchange-{name}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// One box item with a material on one face, smooth flags and Texture UVs.
fn box_scene() -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.name = "Box".into();
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    mesh.faces[0].material = "Red".into();
    mesh.faces[1].smooth = true;
    let uvs = mesh
        .faces
        .iter()
        .map(|f| {
            (0..f.vertices.len())
                .map(|c| Some(DVec2::new(c as f64 * 0.25, 0.5)))
                .collect()
        })
        .collect();
    mesh.uv_maps.insert(UV_MAP.into(), uvs);
    item.mesh = mesh.into();
    scene
}

fn round_trip(format: ExportFormat, scene: &Scene) -> Imported {
    let dir = scratch(&format!("{format:?}"));
    let path = dir.join(format!("out.{}", format.extension()));
    export(&path, scene, format).unwrap();
    import(&path).unwrap()
}

fn only_mesh(imported: &Imported) -> &Mesh {
    assert_eq!(imported.scene.items.len(), 1);
    &imported.scene.items[0].mesh
}

#[test]
fn obj_round_trip_keeps_polygons_uvs_and_materials() {
    let scene = box_scene();
    let back = round_trip(ExportFormat::Obj, &scene);
    let mesh = only_mesh(&back);
    let original = &scene.items[0].mesh;
    assert_eq!(back.scene.items[0].name, "Box");
    assert_eq!(mesh.vertices, original.vertices);
    let corners = |m: &Mesh| {
        m.faces
            .iter()
            .map(|f| f.vertices.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(corners(mesh), corners(original));
    assert_eq!(mesh.faces[0].material, "Red");
    assert!(mesh.faces[1].smooth && !mesh.faces[0].smooth);
    assert_eq!(mesh.uv_maps[UV_MAP], original.uv_maps[UV_MAP]);
    assert!(mesh.is_closed());
}

#[test]
fn obj_reader_handles_common_forms_and_groups() {
    let dir = scratch("obj-forms");
    let path = dir.join("forms.obj");
    fs::write(
        &path,
        "# two quads, the second with negative and v//vn indices\n\
         g first\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvn 0 0 1\nf 1 2 3 4\n\
         g second\nv 0 0 1\nv 1 0 1\nv 1 1 1\nv 0 1 1\nf -4//1 -3//1 -2//1 -1//1\nl 1 2\n",
    )
    .unwrap();
    let imported = import(&path).unwrap();
    let names: Vec<_> = imported
        .scene
        .items
        .iter()
        .map(|i| i.name.as_str())
        .collect();
    assert_eq!(names, ["first", "second"]);
    assert_eq!(
        imported.scene.items[1].mesh.vertices[0],
        DVec3::new(0., 0., 1.)
    );
    assert!(imported.notes.iter().any(|n| n.contains("line")));
    assert_eq!(
        imported.scene.selection.item,
        Some(imported.scene.items[0].id)
    );
}

#[test]
fn invalid_files_are_refused_with_a_reason() {
    let dir = scratch("invalid");
    let cases = [
        ("range.obj", "v 0 0 0\nf 1 2 3\n", "out of range"),
        ("empty.obj", "v 0 0 0\n", "no polygons"),
        ("bad.stl", "not an stl", "STL"),
        ("bad.gltf", "{\"asset\":{}}", "glTF"),
        ("other.geo", "3DG3\n0\n", "3DG1"),
    ];
    for (name, text, reason) in cases {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        let err = import(&path).unwrap_err();
        assert!(err.contains(reason), "{name}: {err}");
    }
    assert!(import(&dir.join("x.fbx")).is_err());
}

#[test]
fn stl_binary_and_ascii_round_trip_as_a_closed_triangle_mesh() {
    let scene = box_scene();
    for format in [ExportFormat::StlBinary, ExportFormat::StlAscii] {
        let back = round_trip(format, &scene);
        let mesh = only_mesh(&back);
        assert_eq!(mesh.vertices.len(), 8, "{format:?}");
        assert_eq!(mesh.faces.len(), 12, "{format:?}");
        assert!(mesh.is_closed(), "{format:?}");
        assert!(mesh.attributes.extrude_edge_uses.is_empty());
        // Outward winding survives: some triangle of the +Y face points up.
        assert!((0..12).any(|f| mesh.normal(f).normalize().dot(DVec3::Y) > 0.99));
    }
}

#[test]
fn gltf_and_glb_round_trip_with_welded_points_and_uvs() {
    let scene = box_scene();
    for format in [ExportFormat::Gltf, ExportFormat::Glb] {
        let back = round_trip(format, &scene);
        let mesh = only_mesh(&back);
        assert_eq!(mesh.vertices.len(), 8, "{format:?}");
        assert_eq!(mesh.faces.len(), 12, "{format:?}");
        assert!(mesh.is_closed(), "{format:?}");
        assert!(mesh.faces.iter().any(|f| f.material == "Red"));
        // V is flipped on the way out and back in, so it returns unchanged.
        let uvs = &mesh.uv_maps[UV_MAP];
        assert!(
            uvs.iter()
                .flatten()
                .all(|u| u.is_some_and(|u| (u.y - 0.5).abs() < 1e-6))
        );
    }
}

#[test]
fn geo_round_trip_keeps_colors_and_skips_lines() {
    let dir = scratch("geo");
    let path = dir.join("in.geo");
    fs::write(
        &path,
        "3DG1\n4\n0 0 0\n1 0 0\n1 1 0\n0 1 0\n4 0 1 2 3 12\n2 0 2 5\n",
    )
    .unwrap();
    let imported = import(&path).unwrap();
    let mesh = only_mesh(&imported);
    assert_eq!(mesh.faces.len(), 1);
    assert_eq!(mesh.faces[0].material, "GEO color 12");
    assert!(imported.notes.iter().any(|n| n.contains("fewer than 3")));
    let out = dir.join("out.geo");
    export(&out, &imported.scene, ExportFormat::Geo).unwrap();
    assert_eq!(
        fs::read_to_string(&out).unwrap(),
        "3DG1\n4\n0 0 0\n1 0 0\n1 1 0\n0 1 0\n4 0 1 2 3 12\n"
    );
}

#[test]
fn svg_draws_every_polygon() {
    let dir = scratch("svg");
    let path = dir.join("front.svg");
    let report = export(&path, &box_scene(), ExportFormat::Svg).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("<svg"));
    assert_eq!(text.matches("<polygon").count(), 6);
    assert_eq!(report.polygons, 6);
}

#[test]
fn export_bakes_transforms_and_keeps_mirrored_polygons_outward() {
    let mut scene = box_scene();
    scene.items[0].transform.translation = DVec3::new(10., 0., 0.);
    scene.items[0].transform.scale = DVec3::new(-1., 1., 1.);
    let back = round_trip(ExportFormat::Obj, &scene);
    let mesh = only_mesh(&back);
    assert!(
        mesh.vertices
            .iter()
            .all(|p| (p.x - 10.).abs() <= 0.5 + 1e-9)
    );
    // Every polygon still faces away from the box center.
    let center = DVec3::new(10., 0., 0.);
    for f in 0..mesh.faces.len() {
        assert!(
            mesh.normal(f).dot(mesh.center(f) - center) > 0.0,
            "face {f}"
        );
    }
}

#[test]
fn export_skips_hidden_items_and_refuses_empty_scenes_and_scene_paths() {
    let dir = scratch("refuse");
    let mut scene = box_scene();
    scene.items[0].visible = false;
    let err = export(&dir.join("a.obj"), &scene, ExportFormat::Obj).unwrap_err();
    assert!(err.contains("Nothing to export"));
    assert!(!dir.join("a.obj").exists());
    let mut two = box_scene();
    two.new_mesh().unwrap();
    two.items[1].mesh = Mesh::box_mesh(DVec3::ONE, DVec3::ONE, 1).unwrap().into();
    two.items[1].visible = false;
    let report = export(&dir.join("b.obj"), &two, ExportFormat::Obj).unwrap();
    assert_eq!(report.items, 1);
    assert!(report.notes.iter().any(|n| n.contains("hidden")));
    let scene_file = dir.join("c.opscene");
    fs::write(&scene_file, "keep").unwrap();
    assert!(export(&scene_file, &box_scene(), ExportFormat::Obj).is_err());
    assert_eq!(fs::read_to_string(&scene_file).unwrap(), "keep");
}

#[test]
fn non_manifold_and_degenerate_input_imports_as_protected_geometry() {
    let dir = scratch("nonmanifold");
    let path = dir.join("fin.obj");
    // Three triangles share edge 1-2 (0-based); the fourth is zero-area.
    fs::write(
        &path,
        "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 2 0 0\nv 1 0 1\nv 3 0 0\n\
         f 1 2 3\nf 2 4 3\nf 2 3 5\nf 1 4 6\n",
    )
    .unwrap();
    let imported = import(&path).unwrap();
    let mesh = only_mesh(&imported);
    assert_eq!(mesh.faces.len(), 4);
    assert!(mesh.attributes.extrude_edge_uses.contains_key("1:2"));
    assert!(mesh.attributes.degenerate_faces.contains(&3));
    assert!(imported.notes.iter().any(|n| n.contains("non-manifold")));
    // The imported scene saves natively and reopens unchanged.
    let saved = dir.join("fin.opscene");
    crate::persistence::save(&saved, &imported.scene).unwrap();
    assert_eq!(crate::persistence::load(&saved).unwrap(), imported.scene);
}

#[test]
fn obj_writes_polylines_as_lines() {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    let mut mesh = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::Y],
        faces: vec![Face {
            vertices: vec![0, 1, 2],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    };
    mesh.attributes.representations = vec![crate::PolygonRepresentation::Polyline {
        closed: false,
        curve: Default::default(),
    }];
    mesh.validate().unwrap();
    scene.items[0].mesh = mesh.into();
    let path = scratch("lines").join("x.obj");
    let report = export(&path, &scene, ExportFormat::Obj).unwrap();
    assert!(fs::read_to_string(&path).unwrap().contains("\nl 1 2 3\n"));
    assert!(report.notes.iter().any(|n| n.contains("polyline")));
}

#[test]
fn lxo_export_writes_a_form_file_keeps_hidden_items_and_protects_the_source() {
    let dir = scratch("lxo");
    let mut scene = box_scene();
    scene.items[0].visible = false;
    let path = dir.join("scene.lxo");
    let report = export(&path, &scene, ExportFormat::Lxo).unwrap();
    assert_eq!((report.items, report.polygons), (1, 6));
    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[..4], b"FORM");
    assert_eq!(&bytes[8..12], b"LXOB");
    scene.import_source = Some(crate::ImportSource {
        path: path.display().to_string(),
        producer: "Modo".into(),
        device: None,
        inode: None,
    });
    let err = export(&path, &scene, ExportFormat::Lxo).unwrap_err();
    assert!(err.contains("read-only"));
}

#[test]
fn gltf_round_trip_keeps_material_settings() {
    let mut scene = box_scene();
    let red = crate::Material {
        color: [0.9, 0.1, 0.05],
        diffuse: 0.6,
        specular: 0.25,
        smoothing: false,
        make_default: true,
        ..Default::default()
    };
    scene.materials.insert("Red".into(), red.clone());
    for format in [ExportFormat::Gltf, ExportFormat::Glb] {
        let back = round_trip(format, &scene);
        // "Red" comes back exactly; the untagged polygons' placeholder
        // material doesn't become a stored Default.
        assert_eq!(back.scene.materials.len(), 1, "{format:?}");
        assert_eq!(back.scene.material("Red"), Some(&red), "{format:?}");
    }
    // The base color a viewer sees is Color times Diffuse.
    let dir = scratch("gltf-material-json");
    let path = dir.join("out.gltf");
    export(&path, &scene, ExportFormat::Gltf).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let red_json = json["materials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["name"] == "Red")
        .unwrap();
    let base = &red_json["pbrMetallicRoughness"]["baseColorFactor"];
    assert!((base[0].as_f64().unwrap() - 0.54).abs() < 1e-6);
    // Specular 25 % is reflectance 0.02 on the Specular curve.
    let root = 0.02f64.sqrt();
    let ior = red_json["extensions"]["KHR_materials_ior"]["ior"]
        .as_f64()
        .unwrap();
    assert!((ior - (1. + root) / (1. - root)).abs() < 1e-9, "{ior}");
    // An export from before the curve (no "operandiSpecular" marker)
    // stored the reflectance: 0.25 reads back as the matching amount.
    let mut old = json.clone();
    for m in old["materials"].as_array_mut().unwrap() {
        if let Some(extras) = m["extras"].as_object_mut() {
            extras.remove("operandiSpecular");
        }
    }
    fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let back = import(&path).unwrap();
    let specular = back.scene.material("Red").unwrap().specular;
    assert!((crate::material::specular_reflectance(specular) - 0.25).abs() < 1e-9);
}

#[test]
fn gltf_material_without_operandi_extras_uses_its_base_color() {
    let scene = box_scene();
    let dir = scratch("gltf-foreign-material");
    let path = dir.join("out.gltf");
    export(&path, &scene, ExportFormat::Gltf).unwrap();
    // Make it look like another app's file: a colored material, no extras.
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for m in json["materials"].as_array_mut().unwrap() {
        m.as_object_mut().unwrap().remove("extras");
        m["pbrMetallicRoughness"]["baseColorFactor"] = serde_json::json!([0.2, 0.4, 0.6, 1.0]);
    }
    fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
    let back = import(&path).unwrap();
    let red = back.scene.material("Red").unwrap();
    assert!((red.color[1] - 0.4).abs() < 1e-6);
    assert_eq!(red.diffuse, 1.0);
}

#[test]
fn ply_binary_and_ascii_round_trip_keep_polygons_and_uvs() {
    let scene = box_scene();
    for format in [ExportFormat::PlyBinary, ExportFormat::PlyAscii] {
        let back = round_trip(format, &scene);
        let mesh = only_mesh(&back);
        let original = &scene.items[0].mesh;
        assert_eq!(mesh.vertices, original.vertices, "{format:?}");
        let corners = |m: &Mesh| {
            m.faces
                .iter()
                .map(|f| f.vertices.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(corners(mesh), corners(original), "{format:?}");
        assert!(mesh.is_closed(), "{format:?}");
        // Per-corner texcoord lists keep every UV, seams included.
        assert_eq!(mesh.uv_maps[UV_MAP], original.uv_maps[UV_MAP], "{format:?}");
    }
}

#[test]
fn ply_reader_takes_big_endian_vertex_uvs_and_skips_other_elements() {
    let dir = scratch("ply-forms");
    let path = dir.join("in.ply");
    let mut data = b"ply\r\nformat binary_big_endian 1.0\r\ncomment other writer\r\n\
element vertex 4\r\nproperty double x\r\nproperty double y\r\nproperty double z\r\n\
property float s\r\nproperty float t\r\nproperty uchar red\r\n\
element face 1\r\nproperty uchar flags\r\nproperty list uchar uint vertex_index\r\n\
element edge 1\r\nproperty int vertex1\r\nproperty int vertex2\r\nend_header\r\n"
        .to_vec();
    for (i, [x, y]) in [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
        .into_iter()
        .enumerate()
    {
        for c in [x, y, 0.0f64] {
            data.extend(c.to_be_bytes());
        }
        data.extend((x as f32).to_be_bytes());
        data.extend((y as f32).to_be_bytes());
        data.push(i as u8);
    }
    data.push(7);
    data.push(4);
    for v in 0u32..4 {
        data.extend(v.to_be_bytes());
    }
    data.extend(0i32.to_be_bytes());
    data.extend(2i32.to_be_bytes());
    fs::write(&path, &data).unwrap();
    let imported = import(&path).unwrap();
    let mesh = only_mesh(&imported);
    assert_eq!(mesh.faces.len(), 1);
    assert_eq!(mesh.vertices[2], DVec3::new(1.0, 1.0, 0.0));
    assert_eq!(mesh.uv_maps[UV_MAP][0][2], Some(DVec2::new(1.0, 1.0)));
    assert!(imported.notes.iter().any(|n| n.contains("1 edge")));
}

#[test]
fn ply_refuses_broken_files_and_writes_items_as_one_mesh() {
    let dir = scratch("ply-bad");
    for (name, text, reason) in [
        ("nomagic.ply", "plx\n", "magic"),
        ("noend.ply", "ply\nformat ascii 1.0\n", "end_header"),
        (
            "short.ply",
            "ply\nformat ascii 1.0\nelement vertex 2\nproperty float x\nproperty float y\nproperty float z\nend_header\n0 0 0\n",
            "ends early",
        ),
        (
            "badindex.ply",
            "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 9\n",
            "missing point",
        ),
    ] {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        let err = import(&path).unwrap_err();
        assert!(err.contains(reason), "{name}: {err}");
    }
    let mut two = box_scene();
    two.new_mesh().unwrap();
    two.items.last_mut().unwrap().mesh = Mesh::box_mesh(DVec3::X * 3.0, DVec3::ONE, 1)
        .unwrap()
        .into();
    let path = dir.join("two.ply");
    let report = export(&path, &two, ExportFormat::PlyAscii).unwrap();
    assert!(report.notes.iter().any(|n| n.contains("one PLY mesh")));
    assert_eq!(only_mesh(&import(&path).unwrap()).faces.len(), 12);
}

/// A small original ASCII FBX: one quad in centimetres, moved 2 m along X,
/// with one red material.
const QUAD_FBX: &str = r#"; FBX 7.4.0 project file
FBXHeaderExtension:  {
	FBXHeaderVersion: 1003
	FBXVersion: 7400
}
GlobalSettings:  {
	Version: 1000
	Properties70:  {
		P: "UpAxis", "int", "Integer", "",1
		P: "UpAxisSign", "int", "Integer", "",1
		P: "FrontAxis", "int", "Integer", "",2
		P: "FrontAxisSign", "int", "Integer", "",1
		P: "CoordAxis", "int", "Integer", "",0
		P: "CoordAxisSign", "int", "Integer", "",1
		P: "UnitScaleFactor", "double", "Number", "",1
	}
}
Objects:  {
	Geometry: 1000, "Geometry::Quad", "Mesh" {
		Vertices: *12 {
			a: 0,0,0,100,0,0,100,100,0,0,100,0
		}
		PolygonVertexIndex: *4 {
			a: 0,1,2,-4
		}
		GeometryVersion: 124
		LayerElementUV: 0 {
			Version: 101
			Name: "map1"
			MappingInformationType: "ByPolygonVertex"
			ReferenceInformationType: "IndexToDirect"
			UV: *8 {
				a: 0,0,1,0,1,1,0,1
			}
			UVIndex: *4 {
				a: 0,1,2,3
			}
		}
		LayerElementMaterial: 0 {
			Version: 101
			Name: ""
			MappingInformationType: "AllSame"
			ReferenceInformationType: "IndexToDirect"
			Materials: *1 {
				a: 0
			}
		}
		Layer: 0 {
			Version: 100
			LayerElement:  {
				Type: "LayerElementUV"
				TypedIndex: 0
			}
			LayerElement:  {
				Type: "LayerElementMaterial"
				TypedIndex: 0
			}
		}
	}
	Model: 2000, "Model::Quad", "Mesh" {
		Version: 232
		Properties70:  {
			P: "Lcl Translation", "Lcl Translation", "", "A",200,0,0
		}
	}
	Material: 3000, "Material::Red", "" {
		ShadingModel: "phong"
		Properties70:  {
			P: "DiffuseColor", "Color", "", "A",1,0,0
		}
	}
}
Connections:  {
	C: "OO",2000,0
	C: "OO",1000,2000
	C: "OO",3000,2000
}
"#;

#[test]
fn fbx_import_bakes_units_transform_uvs_and_material() {
    let dir = scratch("fbx");
    let path = dir.join("quad.fbx");
    fs::write(&path, QUAD_FBX).unwrap();
    let imported = import(&path).unwrap();
    assert_eq!(imported.scene.items[0].name, "Quad");
    let mesh = only_mesh(&imported);
    assert_eq!(mesh.faces.len(), 1);
    // 100 cm quad moved 200 cm: metres, transform baked in.
    let expect = [[2.0, 0.0], [3.0, 0.0], [3.0, 1.0], [2.0, 1.0]];
    for (i, [x, y]) in expect.into_iter().enumerate() {
        let p = mesh.vertices[mesh.faces[0].vertices[i] as usize];
        assert!((p - DVec3::new(x, y, 0.0)).length() < 1e-9, "{p}");
    }
    assert_eq!(mesh.faces[0].material, "Red");
    assert_eq!(
        imported.scene.material("Red").unwrap().color,
        [1.0, 0.0, 0.0]
    );
    assert_eq!(mesh.uv_maps[UV_MAP][0][2], Some(DVec2::new(1.0, 1.0)));
    // Facing +Z as written.
    assert!(mesh.normal(0).normalize().dot(DVec3::Z) > 0.99);
}

#[test]
fn fbx_refuses_a_broken_file() {
    let dir = scratch("fbx-bad");
    let path = dir.join("bad.fbx");
    fs::write(&path, "Kaydara FBX Binary  \0garbage").unwrap();
    let err = import(&path).unwrap_err();
    assert!(err.contains("FBX"), "{err}");
}

#[test]
fn alembic_round_trip_keeps_polygons_winding_uvs_and_items() {
    let mut scene = box_scene();
    // A second item with the same name and a moved, mirrored transform.
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.name = "Box".into();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    item.transform.translation = DVec3::X * 3.0;
    item.transform.scale = DVec3::new(-1.0, 1.0, 1.0);
    let back = round_trip(ExportFormat::Alembic, &scene);
    assert_eq!(back.scene.items.len(), 2);
    let names: Vec<&str> = back.scene.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["Box", "Box 2"]);
    let first = &back.scene.items[0].mesh;
    let original = &scene.items[0].mesh;
    assert_eq!(first.vertices, original.vertices);
    let corners = |m: &Mesh| {
        m.faces
            .iter()
            .map(|f| f.vertices.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(corners(first), corners(original));
    assert_eq!(first.uv_maps[UV_MAP], original.uv_maps[UV_MAP]);
    // The mirrored copy still faces outward: every normal points away
    // from its centre.
    let second = &back.scene.items[1].mesh;
    assert!(second.is_closed());
    let centre = DVec3::X * 3.0;
    for f in 0..second.faces.len() {
        let p = second.vertices[second.faces[f].vertices[0] as usize];
        assert!(second.normal(f).dot(p - centre) > 0.0, "face {f}");
    }
}

#[test]
fn alembic_refuses_a_broken_file_and_leaves_no_temporary_on_failure() {
    let dir = scratch("abc-bad");
    let path = dir.join("bad.abc");
    fs::write(&path, b"not an archive").unwrap();
    let err = import(&path).unwrap_err();
    assert!(err.contains("Alembic"), "{err}");
    // Exporting into a folder that doesn't exist fails cleanly.
    let missing = dir.join("nope/out.abc");
    assert!(export(&missing, &box_scene(), ExportFormat::Alembic).is_err());
    let left: Vec<_> = fs::read_dir(&dir).unwrap().collect();
    assert_eq!(left.len(), 1, "only bad.abc remains");
}

/// Points equal to f32 precision (FBX and 3MF numbers).
fn close_points(a: &[DVec3], b: &[DVec3]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| p.distance(*q) < 1e-5)
}

#[test]
fn fbx_export_reads_back_with_polygons_uvs_smoothing_and_materials() {
    let mut scene = box_scene();
    scene.materials.insert(
        "Red".into(),
        crate::Material {
            color: [0.8, 0.1, 0.05],
            ..Default::default()
        },
    );
    let back = round_trip(ExportFormat::Fbx, &scene);
    let mesh = only_mesh(&back);
    let original = &scene.items[0].mesh;
    assert_eq!(back.scene.items[0].name, "Box");
    assert!(close_points(&mesh.vertices, &original.vertices));
    let corners = |m: &Mesh| {
        m.faces
            .iter()
            .map(|f| f.vertices.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(corners(mesh), corners(original));
    assert!(mesh.is_closed());
    assert_eq!(mesh.faces[0].material, "Red");
    assert!(mesh.faces[1].smooth && !mesh.faces[2].smooth);
    let uv = |m: &Mesh| m.uv_maps[UV_MAP].clone();
    for (a, b) in uv(mesh).iter().flatten().zip(uv(original).iter().flatten()) {
        assert!(a.unwrap().distance(b.unwrap()) < 1e-6);
    }
    let red = back.scene.materials["Red"].color;
    assert!((red[0] - 0.8).abs() < 1e-6 && (red[2] - 0.05).abs() < 1e-6);
}

#[test]
fn fbx_export_bakes_transforms_and_keeps_every_item() {
    let mut scene = box_scene();
    scene.items[0].transform.translation = DVec3::new(10., 0., 0.);
    scene.items[0].transform.scale = DVec3::new(-1., 1., 1.);
    scene.new_mesh().unwrap();
    let second = scene.items.last_mut().unwrap();
    second.name = "Second".into();
    second.mesh = Mesh::box_mesh(DVec3::new(0., 5., 0.), DVec3::ONE, 1)
        .unwrap()
        .into();
    let back = round_trip(ExportFormat::Fbx, &scene);
    assert_eq!(back.scene.items.len(), 2);
    let first = &back.scene.items[0].mesh;
    let center = DVec3::new(10., 0., 0.);
    for f in 0..first.faces.len() {
        assert!(
            first.normal(f).dot(first.center(f) - center) > 0.0,
            "face {f}"
        );
    }
    assert!(back.scene.items[1].mesh.vertices.iter().all(|p| p.y > 4.0));
}

#[test]
fn threemf_round_trip_keeps_shape_items_and_material_colours() {
    let mut scene = box_scene();
    scene.materials.insert(
        "Red".into(),
        crate::Material {
            color: [0.8, 0.1, 0.05],
            ..Default::default()
        },
    );
    let back = round_trip(ExportFormat::ThreeMf, &scene);
    let mesh = only_mesh(&back);
    let original = &scene.items[0].mesh;
    assert_eq!(back.scene.items[0].name, "Box");
    // Triangles, so the same points and twice the polygons of a box.
    assert!(close_points(&mesh.vertices, &original.vertices));
    assert_eq!(mesh.faces.len(), 12);
    assert!(mesh.is_closed());
    assert_eq!(mesh.faces.iter().filter(|f| f.material == "Red").count(), 2);
    // Colour goes through 8-bit sRGB.
    let red = back.scene.materials["Red"].color;
    assert!((red[0] - 0.8).abs() < 0.01 && (red[1] - 0.1).abs() < 0.01);
    // Every triangle still faces out.
    for f in 0..mesh.faces.len() {
        assert!(mesh.normal(f).dot(mesh.center(f)) > 0.0, "face {f}");
    }
}

#[test]
fn threemf_reader_places_components_in_units_and_refuses_broken_files() {
    let model = r#"<?xml version="1.0"?>
<model unit="centimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
 <resources>
  <object id="1" type="model"><mesh>
   <vertices><vertex x="0" y="0" z="0"/><vertex x="1" y="0" z="0"/><vertex x="0" y="1" z="0"/><vertex x="0" y="0" z="1"/></vertices>
   <triangles><triangle v1="0" v2="2" v3="1"/><triangle v1="0" v2="1" v3="3"/><triangle v1="0" v2="3" v3="2"/><triangle v1="1" v2="2" v3="3"/><triangle v1="0" v2="1" v3="9"/></triangles>
  </mesh></object>
  <object id="2" name="Pair" type="model"><components>
   <component objectid="1"/>
   <component objectid="1" transform="1 0 0 0 1 0 0 0 1 0 0 10"/>
  </components></object>
 </resources>
 <build><item objectid="2"/></build>
</model>"#;
    let package = zip::write(&[("3D/3dmodel.model", model.as_bytes())]).unwrap();
    let dir = scratch("3mf-read");
    let path = dir.join("pair.3mf");
    fs::write(&path, &package).unwrap();
    let back = import(&path).unwrap();
    assert_eq!(back.scene.items.len(), 2);
    assert!(back.scene.items.iter().all(|i| i.name == "Pair"));
    // 3MF is Z-up: the second copy, 10 cm up in the file, is 0.1 m up in Y.
    let top = back.scene.items[1]
        .mesh
        .vertices
        .iter()
        .map(|p| p.y)
        .fold(f64::MIN, f64::max);
    assert!((top - 0.11).abs() < 1e-9, "{top}");
    assert!(back.notes.iter().any(|n| n.contains("missing vertices")));
    let broken = dir.join("broken.3mf");
    fs::write(&broken, b"PK\x03\x04not really").unwrap();
    assert!(import(&broken).is_err());
}
