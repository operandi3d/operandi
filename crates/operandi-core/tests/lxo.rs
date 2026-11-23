fn numeric(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-6 + 1e-6 * a.abs().max(b.abs()),
        "{a} != {b}"
    );
}
#[test]
fn native_v2_hierarchy_sparse_maps_labels_provenance_and_source_guard() {
    use glam::{DVec2, DVec3};
    use operandi_core::*;
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    scene.active_mut().unwrap().transform = Transform {
        translation: DVec3::new(3., -2., 5.),
        rotation: DVec3::new(0.2, 0.3, 0.4),
        scale: DVec3::new(-2., 0.5, 3.),
        zxy: true,
        order: None,
    };
    scene.new_mesh().unwrap();
    scene.active_mut().unwrap().parent = Some(1);
    scene.active_mut().unwrap().transform.rotation = DVec3::new(0.3, 0.5, -0.2);
    let mesh = &mut scene.active_mut().unwrap().mesh;
    *mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    mesh.faces[0].part = Some(String::new());
    mesh.faces[0].material_label = Some("Mat_A".into());
    let mut values: Vec<_> = mesh
        .faces
        .iter()
        .map(|f| vec![None; f.vertices.len()])
        .collect();
    values[0][0] = Some(DVec2::new(-1., 4.));
    values[1][1] = Some(DVec2::ZERO);
    mesh.uv_maps.insert("UV_水".into(), values);
    let dir = std::env::temp_dir().join(format!("operandi-lxo-native-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("source.lxo");
    std::fs::write(&source, b"original-source").unwrap();
    let before = std::fs::metadata(&source).unwrap();
    // Only Unix fills in device and inode below.
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut provenance = ImportSource {
        path: source.display().to_string(),
        producer: "synthetic native persistence test".into(),
        device: None,
        inode: None,
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        provenance.device = Some(before.dev());
        provenance.inode = Some(before.ino());
    }
    scene.import_source = Some(provenance);
    let encoded = persistence::encode(&scene).unwrap();
    assert_eq!(scene, persistence::decode(&encoded).unwrap());
    let native = dir.join("separate.opscene");
    persistence::save(&native, &scene).unwrap();
    assert_eq!(scene, persistence::load(&native).unwrap());
    assert!(persistence::save(&source, &scene).is_err());
    #[cfg(unix)]
    {
        let alias = dir.join("alias.opscene");
        let _ = std::fs::remove_file(&alias);
        std::os::unix::fs::symlink(&source, &alias).unwrap();
        assert!(persistence::save(&alias, &scene).is_err());
        std::fs::remove_file(&alias).unwrap();
        std::fs::hard_link(&source, &alias).unwrap();
        assert!(persistence::save(&alias, &scene).is_err());
        std::fs::remove_file(alias).unwrap();
    }
    assert_eq!(std::fs::read(&source).unwrap(), b"original-source");
    assert_eq!(
        std::fs::metadata(&source).unwrap().modified().unwrap(),
        before.modified().unwrap()
    );
    std::fs::remove_file(native).unwrap();
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir(dir).unwrap();
    let mut v1 = serde_json::to_value(&scene).unwrap();
    v1.as_object_mut().unwrap().remove("import_source");
    for i in v1["items"].as_array_mut().unwrap() {
        let o = i.as_object_mut().unwrap();
        o.remove("parent");
        o.remove("source_id");
        o.remove("source_display_name");
        o["transform"].as_object_mut().unwrap().remove("zxy");
        o["transform"]["scale"] = serde_json::json!([1., 1., 1.]);
        o["mesh"].as_object_mut().unwrap().remove("uv_maps");
        for f in o["mesh"]["faces"].as_array_mut().unwrap() {
            f.as_object_mut().unwrap().remove("part");
            f.as_object_mut().unwrap().remove("material_label");
        }
    }
    assert!(
        persistence::decode(
            &serde_json::to_vec(&serde_json::json!({"format":"Operandi","version":1,"scene":v1}))
                .unwrap()
        )
        .is_ok()
    );
}
#[test]
fn concave_triangulation_keeps_original_loops_and_excludes_notch() {
    use glam::DVec3;
    use operandi_core::*;
    let mesh = Mesh {
        attributes: Default::default(),
        vertices: vec![(0., 0.), (3., 0.), (3., 2.), (1., 1.), (0., 2.)]
            .into_iter()
            .map(|(x, y)| DVec3::new(x, y, 0.5))
            .collect(),
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3, 4],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        uv_maps: Default::default(),
    };
    mesh.validate().unwrap();
    let tris = mesh.triangles(0).unwrap();
    assert_eq!(tris.len(), 3);
    let area: f64 = tris
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|c| mesh.vertices[mesh.faces[0].vertices[c] as usize]);
            (b - a).cross(c - a).length() / 2.
        })
        .sum();
    numeric(area, 4.5);
    assert_eq!(mesh.faces[0].vertices, vec![0, 1, 2, 3, 4]);
}
