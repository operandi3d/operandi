//! Local-only numerical acceptance from the admitted BV revision-2 handoff.
use glam::DVec3;
use operandi_core::*;
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/support/bevel_oracle_caps.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/support/bevel_correspondence.rs"
));

fn attributed_chamfer_source() -> Mesh {
    let points = [
        [0., 0.2, 0.],
        [0.2, 0., 0.],
        [3.8, 0., 0.],
        [4., 0.2, 0.],
        [4., 3.8, 0.],
        [3.8, 4., 0.],
        [0.2, 4., 0.],
        [0., 3.8, 0.],
    ];
    let mut source = Mesh {
        vertices: points.into_iter().map(DVec3::from_array).collect(),
        faces: vec![Face {
            vertices: (0..8).collect(),
            uv: None,
            material: "Source QA".into(),
            material_label: Some("Source QA".into()),
            part: Some("Chamfer QA".into()),
            smooth: true,
        }]
        .into(),
        ..Default::default()
    };
    source.uv_maps.insert(
        "UV".into(),
        vec![
            source
                .vertices
                .iter()
                .map(|v| Some(glam::DVec2::new(0.1 + v.x * 0.2, 0.1 + v.y * 0.2)))
                .collect(),
        ],
    );
    source
        .attributes
        .uv_records
        .insert("UV".into(), vec![(0..8).map(Some).collect()]);
    source.attributes.corner_weights.insert(
        "Corner Weight".into(),
        vec![(0..8).map(|v| Some(v as f64 / 7.)).collect()],
    );
    source.attributes.point_weights.insert(
        "Point Weight".into(),
        (0..8).map(|v| (v, v as f64 / 7.)).collect(),
    );
    source.attributes.point_colors.insert(
        "Point Color".into(),
        (0..8)
            .map(|v| (v, glam::DVec4::new(v as f64 / 7., 0.2, 0.8, 1.)))
            .collect(),
    );
    source.attributes.corner_colors.insert(
        "Corner Color".into(),
        vec![
            (0..8)
                .map(|v| Some(glam::DVec4::new(v as f64 / 7., 0.2, 0.8, 1.)))
                .collect(),
        ],
    );
    source
        .attributes
        .corner_normals
        .insert("Normals".into(), vec![vec![Some(DVec3::Z); 8]]);
    source.attributes.normal_map_order = vec!["Normals".into()];
    source.attributes.smoothing_labels = vec![Some("Source smoothing".into())];
    source.attributes.creases.push(MeshCrease {
        a: 1,
        b: 2,
        weight: 0.7,
    });
    source
}
#[test]
fn bv_valid_planar_repair_survivors_history_context_and_native_roundtrip() {
    let source = attributed_chamfer_source();
    source.validate().unwrap();
    let options = BevelOptions {
        auto_weld: true,
        weld_range: 0.,
        use_material: true,
        material: "Repaired bands".into(),
        ..Default::default()
    };
    let welded = bevel_geometry(&source, &[0], 0.2, 0.5, true, &options, None).unwrap();
    assert_eq!(
        (welded.mesh.vertices.len(), welded.mesh.faces.len()),
        (12, 9)
    );
    assert_eq!(welded.mesh.faces[0].vertices, vec![8, 9, 10, 11]);
    // The repaired cap stays the selected descendant and keeps source winding.
    assert_eq!(welded.selected, vec![0]);
    let cap: Vec<_> = welded.mesh.faces[0]
        .vertices
        .iter()
        .map(|&v| welded.mesh.vertices[v as usize])
        .collect();
    let twice_area: f64 = (0..cap.len())
        .map(|i| cap[i].x * cap[(i + 1) % 4].y - cap[(i + 1) % 4].x * cap[i].y)
        .sum();
    assert!(twice_area > 0.);
    // Complete source maps stay complete and keep their identities (BV-R21).
    assert_eq!(
        welded.mesh.uv_maps.keys().collect::<Vec<_>>(),
        source.uv_maps.keys().collect::<Vec<_>>()
    );
    let a = &welded.mesh.attributes;
    assert_eq!(
        a.corner_weights
            .keys()
            .chain(a.corner_colors.keys())
            .chain(a.corner_normals.keys())
            .collect::<Vec<_>>(),
        ["Corner Weight", "Corner Color", "Normals"]
    );
    for (f, face) in welded.mesh.faces.iter().enumerate() {
        let n = face.vertices.len();
        assert!(
            welded.mesh.uv_maps["UV"][f].len() == n
                && welded.mesh.uv_maps["UV"][f].iter().all(Option::is_some)
        );
        assert!(
            a.corner_weights["Corner Weight"][f].len() == n
                && a.corner_weights["Corner Weight"][f]
                    .iter()
                    .all(Option::is_some)
        );
        assert!(
            a.corner_colors["Corner Color"][f].len() == n
                && a.corner_colors["Corner Color"][f]
                    .iter()
                    .all(Option::is_some)
        );
        assert!(
            a.corner_normals["Normals"][f].len() == n
                && a.corner_normals["Normals"][f].iter().all(Option::is_some)
        );
    }
    assert_eq!(
        welded
            .mesh
            .faces
            .iter()
            .filter(|f| f.vertices.len() == 3)
            .count(),
        4
    );
    // Adjacent ray contact t clamps the chamfer at 0.2/(1-(sqrt(2)-1)).
    let offset = 0.2 / (2. - 2_f64.sqrt());
    for (actual, (x, y)) in welded.mesh.vertices[8..].iter().zip([
        (offset, offset),
        (4. - offset, offset),
        (4. - offset, 4. - offset),
        (offset, 4. - offset),
    ]) {
        assert!(actual.distance(DVec3::new(x, y, 0.2)) < 1e-10);
    }
    for (corner, source_corner) in [0_usize, 2, 4, 6].into_iter().enumerate() {
        assert_eq!(
            welded.mesh.attributes.corner_weights["Corner Weight"][0][corner],
            Some(source_corner as f64 / 7.)
        );
        assert_eq!(
            welded.mesh.attributes.point_weights["Point Weight"][&(8 + corner as u32)],
            source_corner as f64 / 7.
        );
        assert_eq!(
            welded.mesh.attributes.corner_colors["Corner Color"][0][corner],
            source.attributes.corner_colors["Corner Color"][0][source_corner]
        );
        assert_eq!(
            welded.mesh.attributes.point_colors["Point Color"][&(8 + corner as u32)],
            source.attributes.point_colors["Point Color"][&(source_corner as u32)]
        );
    }
    assert!(
        welded
            .mesh
            .attributes
            .creases
            .iter()
            .any(|c| c.a == 8 && c.b == 9 && c.weight == 0.7)
    );
    assert_eq!(
        welded.mesh.attributes.normal_map_order,
        source.attributes.normal_map_order
    );
    assert_eq!(welded.mesh.faces[0].material, "Source QA");
    assert!(
        welded.mesh.faces[1..]
            .iter()
            .all(|f| f.material == "Repaired bands" && f.part.as_deref() == Some("Chamfer QA"))
    );
    for map in welded.mesh.attributes.uv_records.values() {
        for (f, row) in map.iter().enumerate() {
            for (c, id) in row.iter().enumerate() {
                if welded.mesh.faces[f].vertices[c] >= 8 {
                    assert!(id.is_some_and(|id| id >= 8));
                }
            }
        }
    }
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = source.clone().into();
    e.scene.selection.faces = vec![0];
    let original = e.scene.clone();
    let p = Parameters {
        shift: 0.2,
        inset: 0.5,
        bevel: options.clone(),
        ..Default::default()
    };
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    // TA-01 (tool-activation v1): the panel edit bevels at once; it used to
    // store the values until the first viewport click.
    e.dispatch(Command::ConfigureBevel(p.clone())).unwrap();
    assert_ne!(e.scene, original);
    assert_eq!(e.scene.active().unwrap().mesh, welded.mesh);
    e.dispatch(Command::Preview(p.clone())).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, welded.mesh);
    assert_eq!(e.scene.selection.faces, welded.selected);
    let stage = e.scene.clone();
    let mut smaller = p.clone();
    smaller.inset = 0.2;
    e.dispatch(Command::Preview(smaller)).unwrap();
    e.dispatch(Command::Preview(p.clone())).unwrap();
    assert_eq!(e.scene, stage);
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    assert_eq!(e.scene, stage);
    assert!(e.history.undo.is_empty());
    e.dispatch(Command::Continue).unwrap();
    let mut next = e.session.live.as_ref().unwrap().params.clone();
    next.shift = 0.05;
    next.inset = 0.01;
    e.dispatch(Command::Preview(next)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, stage);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, original);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, stage);
    assert_eq!(
        persistence::decode(&persistence::encode(&stage).unwrap()).unwrap(),
        stage
    );
    // Range cannot merge disconnected source regions, even when they coincide.
    let mut islands = source.clone();
    let mut other = source.clone();
    for v in &mut other.vertices {
        v.x += 0.00001;
    }
    append_geometry(&mut islands, other).unwrap();
    let island_options = BevelOptions {
        weld_range: 0.01,
        ..options.clone()
    };
    let result = bevel_geometry(&islands, &[0, 1], 0.2, 0.5, true, &island_options, None).unwrap();
    assert_eq!(
        (result.mesh.vertices.len(), result.mesh.faces.len()),
        (24, 18)
    );
    // A partially authored survivor map must refuse, retaining the exact stage.
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    let mut sparse = source.clone();
    sparse
        .attributes
        .corner_weights
        .get_mut("Corner Weight")
        .unwrap()[0][1] = None;
    assert!(
        bevel_geometry(&sparse, &[0], 0.2, 0.5, true, &options, None)
            .unwrap_err()
            .contains("Bevel can't")
    );
    if let Some(directory) = std::env::var_os("OPERANDI_BV_WORKFLOWS") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        persistence::save(&directory.join("Weld_Source.opscene"), &original).unwrap();
        persistence::save(&directory.join("Weld_Result.opscene"), &stage).unwrap();
    }
}
