use glam::{DVec2, DVec3, DVec4};
use operandi_core::*;
fn fixture() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.active_mut().unwrap();
    let face = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: "mat".into(),
        smooth: false,
        part: Some("part".into()),
        material_label: None,
    };
    item.mesh = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(0., 1., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(2., 1., 0.),
            DVec3::new(4., 0., 0.),
            DVec3::new(5., 0., 0.),
            DVec3::new(5., 1., 0.),
            DVec3::new(4., 1., 0.),
        ],
        faces: vec![
            face(vec![0, 1, 2, 3]),
            face(vec![1, 4, 5, 2]),
            face(vec![6, 7, 8, 9]),
        ]
        .into(),
        ..Default::default()
    }
    .into();
    item.mesh.uv_maps.insert(
        "Texture".into(),
        vec![
            vec![
                Some(DVec2::new(0., 0.)),
                None,
                Some(DVec2::new(1., 1.)),
                Some(DVec2::new(0., 1.)),
            ],
            vec![None; 4],
            vec![None; 4],
        ],
    );
    item.mesh.attributes.uv_records.insert(
        "Texture".into(),
        vec![
            vec![Some(0), None, Some(1), Some(2)],
            vec![None; 4],
            vec![None; 4],
        ],
    );
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e
}
#[test]
fn flip_double_history_and_roundtrip() {
    let mut e = fixture();
    e.scene.validate().unwrap();
    let before = e.scene.clone();
    let normal = before.active().unwrap().mesh.normal(0);
    e.dispatch(Command::FlipPolygons).unwrap();
    let after = e.scene.clone();
    assert!((after.active().unwrap().mesh.normal(0) + normal).length() < 1e-9);
    assert_eq!(
        after.active().unwrap().mesh.faces[1],
        before.active().unwrap().mesh.faces[1]
    );
    assert!(
        !after
            .active()
            .unwrap()
            .mesh
            .attributes
            .extrude_edge_uses
            .is_empty()
    );
    assert_eq!(
        persistence::decode(&persistence::encode(&after).unwrap()).unwrap(),
        after
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
    e.dispatch(Command::FlipPolygons).unwrap();
    assert_eq!(e.scene, before);
}
#[test]
fn vertex_mode_and_authored_normal_refuse_atomically() {
    // Vertex mode refuses (poly-flip-2026-11-05, FL07); with nothing
    // selected in Polygon mode Flip takes every polygon (FL05).
    let mut e = fixture();
    e.scene.selection.mode = Mode::Vertex;
    let before = e.scene.clone();
    assert_eq!(
        e.dispatch(Command::FlipPolygons).unwrap_err(),
        "Need to directly select one or more polygons."
    );
    assert_eq!(e.scene, before);
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    let m = &mut e.scene.active_mut().unwrap().mesh;
    m.attributes.corner_normals.insert(
        "n".into(),
        vec![vec![Some(DVec3::Z); 4], vec![None; 4], vec![None; 4]],
    );
    let before = e.scene.clone();
    assert!(e.dispatch(Command::FlipPolygons).is_err());
    assert_eq!(e.scene, before);
}
#[test]
fn flip_keeps_corner_data_attached_to_vertices() {
    let mut e = fixture();
    let m = &mut e.scene.active_mut().unwrap().mesh;
    m.faces[0].uv = Some(vec![DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y]);
    m.attributes.corner_weights.insert(
        "weights".into(),
        vec![
            vec![Some(0.1), None, Some(0.7), Some(0.9)],
            vec![None; 4],
            vec![None; 4],
        ],
    );
    m.attributes.corner_colors.insert(
        "colors".into(),
        vec![
            vec![Some(DVec4::X), None, Some(DVec4::Y), Some(DVec4::Z)],
            vec![None; 4],
            vec![None; 4],
        ],
    );
    m.attributes.corner_normals.insert(
        "normals".into(),
        vec![vec![None; 4], vec![Some(DVec3::Z); 4], vec![None; 4]],
    );
    m.attributes.creases.push(MeshCrease {
        a: 0,
        b: 1,
        weight: 0.5,
    });
    let before = e.scene.clone();
    e.dispatch(Command::FlipPolygons).unwrap();
    let after = e.scene.active().unwrap().mesh.clone();
    let mut expected = before.active().unwrap().mesh.clone();
    // The first corner stays first; the rest are reversed (poly-flip-2026-11-05).
    expected.faces[0].vertices[1..].reverse();
    expected.faces[0].uv.as_mut().unwrap()[1..].reverse();
    expected.uv_maps.get_mut("Texture").unwrap()[0][1..].reverse();
    expected.attributes.uv_records.get_mut("Texture").unwrap()[0][1..].reverse();
    expected
        .attributes
        .corner_weights
        .get_mut("weights")
        .unwrap()[0][1..]
        .reverse();
    expected.attributes.corner_colors.get_mut("colors").unwrap()[0][1..].reverse();
    expected.attributes.extrude_edge_uses = after.attributes.extrude_edge_uses.clone();
    assert_eq!(after, expected);
    e.dispatch(Command::FlipPolygons).unwrap();
    assert_eq!(e.scene, before);
}
#[test]
fn spike_disjoint_island_remaps_untouched_ledger() {
    let mut e = fixture();
    e.dispatch(Command::FlipPolygons).unwrap();
    let source = e.scene.active().unwrap().mesh.clone();
    let out = spike_geometry(&source, &[2], 0.25).unwrap();
    out.mesh.validate().unwrap();
    assert_eq!(
        out.mesh.attributes.extrude_edge_uses,
        source.attributes.extrude_edge_uses
    );
    assert!(spike_geometry(&source, &[1], 0.25).is_err());
}
#[test]
fn spike_before_exceptional_island_remaps_face_ids() {
    let mut e = fixture();
    let m = &mut e.scene.active_mut().unwrap().mesh;
    m.faces.swap(0, 2);
    for rows in m.uv_maps.values_mut() {
        rows.swap(0, 2);
    }
    for rows in m.attributes.uv_records.values_mut() {
        rows.swap(0, 2);
    }
    e.scene.selection.faces = vec![2];
    e.dispatch(Command::FlipPolygons).unwrap();
    let source = e.scene.active().unwrap().mesh.clone();
    let out = spike_geometry(&source, &[0], 0.25).unwrap();
    out.mesh.validate().unwrap();
    for (key, old) in &source.attributes.extrude_edge_uses {
        let new = &out.mesh.attributes.extrude_edge_uses[key];
        assert_eq!(old.len(), new.len());
        for (a, b) in old.iter().zip(new) {
            assert_eq!(b[0], a[0] + 3);
            assert_eq!(&a[1..], &b[1..]);
        }
    }
}
#[test]
fn flip_preserves_session_ids_and_corner_identity_through_history() {
    let mut e = fixture();
    e.synchronize_identities().unwrap();
    let original = e.identities.current.clone();
    let old = original.meshes[&1].clone();
    e.dispatch(Command::FlipPolygons).unwrap();
    let flipped = e.identities.current.clone();
    let ids = &flipped.meshes[&1];
    assert_eq!(ids.scope, old.scope);
    assert_eq!(ids.vertices, old.vertices);
    assert_eq!(ids.faces, old.faces);
    assert_eq!(ids.edges, old.edges);
    let mut reversed = old.corners[0].clone();
    reversed[1..].reverse();
    assert_eq!(ids.corners[0], reversed);
    assert_eq!(ids.corners[1], old.corners[1]);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.identities.current, original);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.identities.current, flipped);
}
#[test]
fn disjoint_smooth_shift_and_extrude_preserve_exception_island() {
    for selected_before in [false, true] {
        let mut e = fixture();
        if selected_before {
            let m = &mut e.scene.active_mut().unwrap().mesh;
            m.faces.swap(0, 2);
            for rows in m.uv_maps.values_mut() {
                rows.swap(0, 2);
            }
            for rows in m.attributes.uv_records.values_mut() {
                rows.swap(0, 2);
            }
            e.scene.selection.faces = vec![2];
        }
        e.dispatch(Command::FlipPolygons).unwrap();
        let source = e.scene.active().unwrap().mesh.clone();
        let selected = if selected_before { 0 } else { 2 };
        let boundary = if selected_before { 2 } else { 0 };
        let p = SmoothShiftParameters {
            offset: 0.2,
            ..Default::default()
        };
        let shifted = smooth_shift_geometry(&source, &[selected], p, None).unwrap();
        shifted.mesh.validate().unwrap();
        assert!(smooth_shift_geometry(&source, &[boundary], p, None).is_err());
        let bevel = bevel_geometry(
            &source,
            &[selected],
            0.2,
            0.1,
            true,
            &BevelOptions::default(),
            None,
        )
        .unwrap();
        bevel.mesh.validate().unwrap();
        assert!(
            bevel_geometry(
                &source,
                &[boundary],
                0.2,
                0.1,
                true,
                &BevelOptions::default(),
                None
            )
            .is_err()
        );
        let result = extrude_geometry(
            &source,
            &[selected],
            DVec3::Z * 0.2,
            1,
            ExtrudeMode::Shift,
            ExtrudeUvs::None,
            None,
        )
        .unwrap();
        result.mesh.validate().unwrap();
        let touching = extrude_geometry(
            &source,
            &[boundary],
            DVec3::Z * 0.2,
            1,
            ExtrudeMode::Shift,
            ExtrudeUvs::None,
            None,
        )
        .unwrap();
        touching.mesh.validate().unwrap();
    }
}
#[test]
fn malformed_ledger_and_nonmanifold_refuse_without_publication() {
    let mut e = fixture();
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .extrude_edge_uses
        .insert("0:1".into(), vec![[0, 0, 1]]);
    let before = e.scene.clone();
    assert!(e.dispatch(Command::FlipPolygons).is_err());
    assert_eq!(e.scene, before);
    let mut e = fixture();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    let mut f = mesh.faces[0].clone();
    f.vertices = vec![1, 2, 5, 4];
    mesh.faces.push(f);
    for rows in mesh.uv_maps.values_mut() {
        rows.push(vec![None; 4]);
    }
    for rows in mesh.attributes.uv_records.values_mut() {
        rows.push(vec![None; 4]);
    }
    mesh.attributes
        .extrude_edge_uses
        .insert("1:2".into(), vec![[0, 1, 2], [1, 2, 1], [3, 1, 2]]);
    mesh.validate().unwrap();
    let before = e.scene.clone();
    assert!(e.dispatch(Command::FlipPolygons).is_err());
    assert_eq!(e.scene, before);
}

#[test]
fn exact_incidence_typed_corners_and_unrelated_item_survive_flip() {
    use std::collections::BTreeMap;
    let mut e = fixture();
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .typed_maps
        .insert(
            "vectors".into(),
            TypedMap {
                kind: "vector3".into(),
                dimension: 3,
                zero_default: false,
                points: BTreeMap::new(),
                corners: vec![
                    vec![
                        Some(vec![1., 2., 3.]),
                        None,
                        Some(vec![4., 5., 6.]),
                        Some(vec![7., 8., 9.]),
                    ],
                    vec![None; 4],
                    vec![None; 4],
                ],
            },
        );
    let selection = e.scene.selection.clone();
    e.scene.new_mesh().unwrap();
    e.scene.items.last_mut().unwrap().mesh = Mesh::box_mesh(DVec3::new(20., 0., 0.), DVec3::ONE, 1)
        .unwrap()
        .into();
    e.scene.selection = selection;
    let before = e.scene.clone();
    e.dispatch(Command::FlipPolygons).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    assert_eq!(
        mesh.attributes.extrude_edge_uses,
        BTreeMap::from([("1:2".to_string(), vec![[0, 2, 1], [1, 2, 1]])])
    );
    let mut expected =
        before.active().unwrap().mesh.attributes.typed_maps["vectors"].corners[0].clone();
    expected[1..].reverse();
    assert_eq!(mesh.attributes.typed_maps["vectors"].corners[0], expected);
    assert_eq!(e.scene.items[1], before.items[1]);
    e.dispatch(Command::FlipPolygons).unwrap();
    assert_eq!(e.scene, before);
}

#[test]
fn typed_normal_selected_refuses_without_history_changes() {
    use std::collections::BTreeMap;
    let mut e = fixture();
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .typed_maps
        .insert(
            "normals".into(),
            TypedMap {
                kind: "normal3".into(),
                dimension: 3,
                zero_default: false,
                points: BTreeMap::from([(0, vec![0., 0., 1.])]),
                corners: vec![],
            },
        );
    let before = e.scene.clone();
    let undo = e.history.undo.clone();
    let redo = e.history.redo.clone();
    assert!(e.dispatch(Command::FlipPolygons).is_err());
    assert_eq!(e.scene, before);
    assert!(e.history.undo == undo);
    assert!(e.history.redo == redo);
}

#[test]
fn native_file_roundtrip_with_flip() {
    let mut e = fixture();
    e.dispatch(Command::FlipPolygons).unwrap();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-flip-20261028/roundtrip");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("flip-{}.opscene", std::process::id()));
    assert!(!path.exists());
    persistence::save(&path, &e.scene).unwrap();
    assert_eq!(persistence::load(&path).unwrap(), e.scene);
    std::fs::remove_file(path).unwrap();
}

// Correspondence is derived independently from unique source positions and the
// original oriented face cycles, not from any consumer's generated ledger.
fn assert_exception_island(source: &Mesh, output: &Mesh, faces: [usize; 2]) {
    use std::collections::BTreeMap;
    let mut vertices = BTreeMap::new();
    for &f in &faces {
        for &v in &source.faces[f].vertices {
            let matches: Vec<_> = output
                .vertices
                .iter()
                .enumerate()
                .filter_map(|(i, p)| (*p == source.vertices[v as usize]).then_some(i as u32))
                .collect();
            assert_eq!(matches.len(), 1, "unique untouched vertex");
            vertices.insert(v, matches[0]);
        }
    }
    let mut face_ids = BTreeMap::new();
    for &f in &faces {
        let mut expected = source.faces[f].clone();
        expected.vertices = expected.vertices.iter().map(|v| vertices[v]).collect();
        let matches: Vec<_> = output
            .faces
            .iter()
            .enumerate()
            .filter_map(|(i, face)| (*face == expected).then_some(i))
            .collect();
        assert_eq!(matches.len(), 1, "unchanged face and tags");
        let of = matches[0];
        face_ids.insert(f as u32, of as u32);
        for (name, rows) in &source.uv_maps {
            assert_eq!(output.uv_maps[name][of], rows[f]);
        }
        for (name, rows) in &source.attributes.uv_records {
            assert_eq!(output.attributes.uv_records[name][of], rows[f]);
        }
        for (name, rows) in &source.attributes.corner_weights {
            assert_eq!(output.attributes.corner_weights[name][of], rows[f]);
        }
        for (name, rows) in &source.attributes.corner_colors {
            assert_eq!(output.attributes.corner_colors[name][of], rows[f]);
        }
        for (name, rows) in &source.attributes.corner_normals {
            assert_eq!(output.attributes.corner_normals[name][of], rows[f]);
        }
        for (name, map) in &source.attributes.typed_maps {
            if !map.corners.is_empty() {
                assert_eq!(
                    output.attributes.typed_maps[name].corners[of],
                    map.corners[f]
                );
            }
        }
        for (name, set) in &source.attributes.polygon_sets {
            assert_eq!(
                output.attributes.polygon_sets[name].contains(&(of as u32)),
                set.contains(&(f as u32))
            );
        }
        if !source.attributes.smoothing_labels.is_empty() {
            assert_eq!(
                output.attributes.smoothing_labels[of],
                source.attributes.smoothing_labels[f]
            );
        }
    }
    let mut ordered_faces = faces;
    ordered_faces.sort_unstable();
    let expected_records: Vec<_> = ordered_faces
        .iter()
        .map(|&f| [face_ids[&(f as u32)], vertices[&2], vertices[&1]])
        .collect();
    let a = vertices[&1];
    let b = vertices[&2];
    let key = format!("{}:{}", a.min(b), a.max(b));
    assert_eq!(output.attributes.extrude_edge_uses[&key], expected_records);
    for (name, map) in &source.attributes.point_uvs {
        for (&v, &mapped) in &vertices {
            assert_eq!(output.attributes.point_uvs[name].get(&mapped), map.get(&v));
        }
    }
    for (name, map) in &source.attributes.point_weights {
        for (&v, &mapped) in &vertices {
            assert_eq!(
                output.attributes.point_weights[name].get(&mapped),
                map.get(&v)
            );
        }
    }
    for (name, map) in &source.attributes.point_colors {
        for (&v, &mapped) in &vertices {
            assert_eq!(
                output.attributes.point_colors[name].get(&mapped),
                map.get(&v)
            );
        }
    }
    for (name, map) in &source.attributes.relative_morphs {
        for (&v, &mapped) in &vertices {
            assert_eq!(
                output.attributes.relative_morphs[name].get(&mapped),
                map.get(&v)
            );
        }
    }
    for (name, map) in &source.attributes.typed_maps {
        for (&v, &mapped) in &vertices {
            assert_eq!(
                output.attributes.typed_maps[name].points.get(&mapped),
                map.points.get(&v)
            );
        }
    }
    for (name, set) in &source.attributes.vertex_sets {
        for (&v, &mapped) in &vertices {
            assert_eq!(
                output.attributes.vertex_sets[name].contains(&mapped),
                set.contains(&v)
            );
        }
    }
    for crease in &source.attributes.creases {
        if vertices.contains_key(&crease.a) && vertices.contains_key(&crease.b) {
            let a = vertices[&crease.a];
            let b = vertices[&crease.b];
            assert!(output.attributes.creases.iter().any(|c| (
                c.a.min(c.b),
                c.a.max(c.b),
                c.weight
            ) == (
                a.min(b),
                a.max(b),
                crease.weight
            )));
        }
    }
}

#[test]
fn disjoint_consumers_preserve_the_exception_island_in_both_face_orders() {
    for selected_before in [false, true] {
        let mut e = fixture();
        if selected_before {
            let m = &mut e.scene.active_mut().unwrap().mesh;
            m.faces.swap(0, 2);
            for rows in m.uv_maps.values_mut() {
                rows.swap(0, 2);
            }
            for rows in m.attributes.uv_records.values_mut() {
                rows.swap(0, 2);
            }
            e.scene.selection.faces = vec![2];
        }
        e.dispatch(Command::FlipPolygons).unwrap();
        let source = e.scene.active().unwrap().mesh.clone();
        let selected = if selected_before { 0 } else { 2 };
        let island = if selected_before { [2, 1] } else { [0, 1] };
        let shift = smooth_shift_geometry(
            &source,
            &[selected],
            SmoothShiftParameters {
                offset: 0.2,
                ..Default::default()
            },
            None,
        )
        .unwrap();
        assert_exception_island(&source, &shift.mesh, island);
        let extrude = extrude_geometry(
            &source,
            &[selected],
            DVec3::Z * 0.2,
            1,
            ExtrudeMode::Shift,
            ExtrudeUvs::None,
            None,
        )
        .unwrap();
        assert_exception_island(&source, &extrude.mesh, island);
        let bevel = bevel_geometry(
            &source,
            &[selected],
            0.2,
            0.1,
            true,
            &BevelOptions::default(),
            None,
        )
        .unwrap();
        assert_exception_island(&source, &bevel.mesh, island);
        let auto = BevelOptions {
            auto_weld: true,
            ..Default::default()
        };
        let bevel = bevel_geometry(&source, &[selected], 0.2, 0.1, true, &auto, None).unwrap();
        assert_exception_island(&source, &bevel.mesh, island);
        let spike = spike_geometry(&source, &[selected], 0.25).unwrap();
        assert_exception_island(&source, &spike.mesh, island);
        // The legacy operation cannot remap UV record identities in generated
        // walls. Use the previously supported non-record profile for this route.
        let mut legacy = source.clone();
        legacy.attributes.uv_records.clear();
        let cap = legacy
            .cap_edit(selected as usize, 0., DVec3::Z * 0.2)
            .unwrap();
        assert_exception_island(&legacy, &cap, island);
    }
}

#[test]
fn bridge_disjoint_ordered_faces_preserves_exception_island() {
    for selected_before in [false, true] {
        let mut e = fixture();
        let m = &mut e.scene.active_mut().unwrap().mesh;
        m.vertices.extend([
            DVec3::new(4., 0., 2.),
            DVec3::new(5., 0., 2.),
            DVec3::new(5., 1., 2.),
            DVec3::new(4., 1., 2.),
        ]);
        let mut face = m.faces[2].clone();
        face.vertices = vec![13, 12, 11, 10];
        m.faces.push(face);
        for rows in m.uv_maps.values_mut() {
            rows.push(vec![None; 4]);
        }
        for rows in m.attributes.uv_records.values_mut() {
            rows.push(vec![None; 4]);
        }
        if selected_before {
            m.faces.swap(0, 2);
            m.faces.swap(1, 3);
            for rows in m.uv_maps.values_mut() {
                rows.swap(0, 2);
                rows.swap(1, 3);
            }
            for rows in m.attributes.uv_records.values_mut() {
                rows.swap(0, 2);
                rows.swap(1, 3);
            }
            e.scene.selection.faces = vec![2];
        }
        e.dispatch(Command::FlipPolygons).unwrap();
        let (selected, island) = if selected_before {
            ([0, 1], [2, 3])
        } else {
            ([2, 3], [0, 1])
        };
        let mesh = &mut e.scene.active_mut().unwrap().mesh;
        let mut weights = vec![vec![None; 4]; 4];
        weights[island[0]] = vec![Some(0.1), None, Some(0.7), Some(0.9)];
        mesh.attributes
            .corner_weights
            .insert("island weights".into(), weights);
        let mut normals = vec![vec![None; 4]; 4];
        normals[island[1]] = vec![Some(DVec3::Z); 4];
        mesh.attributes
            .corner_normals
            .insert("island normals".into(), normals);
        mesh.attributes
            .point_weights
            .insert("island points".into(), [(0, 0.25), (2, 0.75)].into());
        mesh.attributes.creases.push(MeshCrease {
            a: 1,
            b: 2,
            weight: 0.5,
        });
        let mut corners = vec![vec![None; 4]; 4];
        corners[island[0]][0] = Some(vec![1., 2., 3.]);
        mesh.attributes.typed_maps.insert(
            "island vectors".into(),
            TypedMap {
                kind: "vector3".into(),
                dimension: 3,
                zero_default: false,
                points: [(0, vec![4., 5., 6.])].into(),
                corners,
            },
        );
        mesh.attributes
            .vertex_sets
            .insert("island vertices".into(), [0, 2].into());
        mesh.attributes
            .polygon_sets
            .insert("island faces".into(), [island[0] as u32].into());
        let source = mesh.clone();
        let result = bridge_geometry(&source, &selected, BridgeParameters::default()).unwrap();
        assert_exception_island(&source, &result.mesh, island);
    }
}

#[test]
fn flip_refusal_keeps_editor_and_redo() {
    let mut e = fixture();
    e.dispatch(Command::FlipPolygons).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.scene.selection.mode = Mode::Vertex;
    let before = e.clone();
    assert!(e.dispatch(Command::FlipPolygons).is_err());
    assert!(e == before);
}

#[test]
fn attributed_island_is_preserved_or_existing_profile_refuses() {
    for selected_before in [false, true] {
        let mut e = fixture();
        let m = &mut e.scene.active_mut().unwrap().mesh;
        m.attributes.corner_weights.insert(
            "w".into(),
            vec![
                vec![Some(0.2), None, Some(0.7), None],
                vec![Some(0.4); 4],
                vec![Some(0.5); 4],
            ],
        );
        m.attributes.corner_colors.insert(
            "c".into(),
            vec![
                vec![Some(DVec4::X), None, Some(DVec4::Y), None],
                vec![Some(DVec4::ONE); 4],
                vec![Some(DVec4::ONE); 4],
            ],
        );
        m.attributes.corner_normals.insert(
            "n".into(),
            vec![
                vec![None; 4],
                vec![Some(DVec3::Z); 4],
                vec![Some(DVec3::Z); 4],
            ],
        );
        m.attributes
            .point_weights
            .insert("p".into(), (0..10).map(|i| (i, i as f64 / 10.)).collect());
        m.attributes.creases.push(MeshCrease {
            a: 1,
            b: 2,
            weight: 0.5,
        });
        if selected_before {
            m.faces.swap(0, 2);
            for rows in m.uv_maps.values_mut() {
                rows.swap(0, 2);
            }
            for rows in m.attributes.uv_records.values_mut() {
                rows.swap(0, 2);
            }
            for rows in m.attributes.corner_weights.values_mut() {
                rows.swap(0, 2);
            }
            for rows in m.attributes.corner_colors.values_mut() {
                rows.swap(0, 2);
            }
            for rows in m.attributes.corner_normals.values_mut() {
                rows.swap(0, 2);
            }
            e.scene.selection.faces = vec![2];
        }
        let baseline = e.scene.active().unwrap().mesh.clone();
        e.dispatch(Command::FlipPolygons).unwrap();
        let source = e.scene.active().unwrap().mesh.clone();
        let selected = if selected_before { 0 } else { 2 };
        let island = if selected_before { [2, 1] } else { [0, 1] };
        type Operation = Box<dyn Fn(&Mesh) -> operandi_core::Result<Mesh>>;
        let operations: Vec<Operation> = vec![
            Box::new(move |m| {
                smooth_shift_geometry(
                    m,
                    &[selected],
                    SmoothShiftParameters {
                        offset: 0.2,
                        ..Default::default()
                    },
                    None,
                )
                .map(|r| r.mesh)
            }),
            Box::new(move |m| {
                extrude_geometry(
                    m,
                    &[selected],
                    DVec3::Z * 0.2,
                    1,
                    ExtrudeMode::Shift,
                    ExtrudeUvs::None,
                    None,
                )
                .map(|r| r.mesh)
            }),
            Box::new(move |m| {
                bevel_geometry(
                    m,
                    &[selected],
                    0.2,
                    0.1,
                    true,
                    &BevelOptions::default(),
                    None,
                )
                .map(|r| r.mesh)
            }),
            Box::new(move |m| {
                bevel_geometry(
                    m,
                    &[selected],
                    0.2,
                    0.1,
                    true,
                    &BevelOptions {
                        auto_weld: true,
                        ..Default::default()
                    },
                    None,
                )
                .map(|r| r.mesh)
            }),
            Box::new(move |m| spike_geometry(m, &[selected], 0.25).map(|r| r.mesh)),
            Box::new(move |m| m.cap_edit(selected as usize, 0., DVec3::Z * 0.2)),
        ];
        for (index, operation) in operations.into_iter().enumerate() {
            let baseline_result = operation(&baseline);
            let actual = operation(&source);
            eprintln!(
                "attributed profile order={selected_before} operation={} baseline={:?}",
                [
                    "Smooth Shift",
                    "Extrude",
                    "Bevel",
                    "Bevel Auto Weld",
                    "Spike",
                    "legacy cap edit"
                ][index],
                baseline_result.as_ref().err()
            );
            if baseline_result.is_ok() {
                assert_exception_island(
                    &source,
                    &actual.expect("a disconnected Flip must preserve supported profiles"),
                    island,
                );
            } else {
                // Existing unsupported attribute profiles remain a refusal;
                // no new interpolation rule is inferred here.
                assert!(actual.is_err());
            }
        }
    }
}

#[test]
fn touching_exception_refusal_preserves_editor_history_and_redo() {
    for tool in [Tool::Bevel, Tool::SmoothShift] {
        let mut e = fixture();
        e.dispatch(Command::FlipPolygons).unwrap();
        e.dispatch(Command::FlipPolygons).unwrap();
        e.dispatch(Command::Undo).unwrap();
        assert!(!e.history.redo.is_empty());
        e.dispatch(Command::Activate(tool)).unwrap();
        let mut before = e.clone();
        let p = Parameters {
            shift: 0.2,
            inset: 0.1,
            extent: DVec3::Z * 0.2,
            smooth_shift: SmoothShiftParameters {
                offset: 0.2,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(e.dispatch(Command::Preview(p)).is_err());
        // Failed live previews publish only the existing diagnostic field.
        assert!(e.session.live.as_ref().unwrap().preview_error.is_some());
        before.session.live.as_mut().unwrap().preview_error =
            e.session.live.as_ref().unwrap().preview_error.clone();
        assert!(
            e == before,
            "{tool:?} refusal changed editor state beyond its diagnostic"
        );
    }
    for bridge in [false, true] {
        let mut e = fixture();
        e.dispatch(Command::FlipPolygons).unwrap();
        e.dispatch(Command::FlipPolygons).unwrap();
        e.dispatch(Command::Undo).unwrap();
        if bridge {
            e.scene.selection.faces = vec![0, 1];
        }
        e.dispatch(if bridge {
            Command::ArmBridge
        } else {
            Command::ArmSpike
        })
        .unwrap();
        let before = e.clone();
        assert!(e.dispatch(Command::Apply).is_err());
        assert!(e == before);
        assert!(!e.history.redo.is_empty());
    }
}

#[test]
fn extrude_and_spike_preserve_supported_point_and_normal_profiles() {
    for selected_before in [false, true] {
        let mut e = fixture();
        if selected_before {
            let m = &mut e.scene.active_mut().unwrap().mesh;
            m.faces.swap(0, 2);
            for rows in m.uv_maps.values_mut() {
                rows.swap(0, 2);
            }
            for rows in m.attributes.uv_records.values_mut() {
                rows.swap(0, 2);
            }
            e.scene.selection.faces = vec![2];
        }
        e.dispatch(Command::FlipPolygons).unwrap();
        let m = &mut e.scene.active_mut().unwrap().mesh;
        m.attributes.point_weights.insert(
            "weights".into(),
            (0..10).map(|v| (v, v as f64 / 10.)).collect(),
        );
        m.attributes.point_colors.insert(
            "colors".into(),
            (0..10)
                .map(|v| (v, DVec4::new(v as f64 / 10., 0.2, 0.3, 1.)))
                .collect(),
        );
        m.attributes
            .corner_normals
            .insert("normals".into(), vec![vec![Some(DVec3::Z); 4]; 3]);
        m.attributes.creases.push(MeshCrease {
            a: 1,
            b: 2,
            weight: 0.6,
        });
        m.attributes.smoothing_labels = vec![Some("one".into()), None, Some("three".into())];
        let source = m.clone();
        let selected = if selected_before { 0 } else { 2 };
        let island = if selected_before { [2, 1] } else { [0, 1] };
        let spike = spike_geometry(&source, &[selected], 0.25).unwrap();
        assert_exception_island(&source, &spike.mesh, island);
        let extrude = extrude_geometry(
            &source,
            &[selected],
            DVec3::Z * 0.2,
            1,
            ExtrudeMode::Shift,
            ExtrudeUvs::None,
            None,
        )
        .unwrap();
        assert_exception_island(&source, &extrude.mesh, island);
    }
}

#[test]
fn existing_thicken_followups_match_pre_flip_baseline_through_editor() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/flip_thicken_compatibility.json")).unwrap();
    let source: Mesh = serde_json::from_value(fixture["source"].clone()).unwrap();
    source.validate().unwrap();
    assert_eq!(source.attributes.extrude_edge_uses.len(), 4);
    for row in fixture["expected"].as_array().unwrap() {
        let selected: Vec<u32> = serde_json::from_value(row["selected"].clone()).unwrap();
        let expected: Mesh = serde_json::from_value(row["mesh"].clone()).unwrap();
        let mut editor = Editor::default();
        editor.dispatch(Command::NewMesh).unwrap();
        editor.scene.active_mut().unwrap().mesh = source.clone().into();
        editor.scene.selection.mode = Mode::Polygon;
        editor.scene.selection.faces = selected;
        if row["op"] == "bridge" {
            editor.dispatch(Command::ArmBridge).unwrap();
            editor.dispatch(Command::Apply).unwrap();
        } else {
            editor.dispatch(Command::Activate(Tool::Extrude)).unwrap();
            let p = Parameters {
                extent: DVec3::Z * 0.2,
                extrude_mode: ExtrudeMode::Shift,
                extrude_uvs: ExtrudeUvs::None,
                extrude_uv_map: None,
                ..Default::default()
            };
            editor.dispatch(Command::Preview(p)).unwrap();
        }
        let output = &editor.scene.active().unwrap().mesh;
        output.validate().unwrap();
        assert_eq!(**output, expected);
    }
}

/// Sub-D faces (Tab) are ordinary polygons drawn smoothed: Flip turns them
/// like any face, with undo/redo and save/reopen; curves still refuse.
#[test]
fn flip_turns_subd_faces_and_refuses_curves() {
    for rep in [
        PolygonRepresentation::Subdivs,
        PolygonRepresentation::CatmullClark,
    ] {
        let mut e = fixture();
        e.scene
            .active_mut()
            .unwrap()
            .mesh
            .attributes
            .representations = vec![rep; 3];
        e.scene.validate().unwrap();
        let before = e.scene.clone();
        let normal = before.active().unwrap().mesh.normal(0);
        e.dispatch(Command::FlipPolygons).unwrap();
        let after = e.scene.clone();
        let mesh = &after.active().unwrap().mesh;
        assert!((mesh.normal(0) + normal).length() < 1e-9);
        assert_eq!(mesh.attributes.representations, vec![rep; 3]);
        assert_eq!(
            persistence::decode(&persistence::encode(&after).unwrap()).unwrap(),
            after
        );
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, after);
        // Nothing selected flips every Sub-D face.
        e.scene.selection.faces.clear();
        e.dispatch(Command::FlipPolygons).unwrap();
        let all = &e.scene.active().unwrap().mesh;
        assert!((all.normal(0) - normal).length() < 1e-9);
    }
    let mut e = fixture();
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .representations = vec![
        PolygonRepresentation::Face,
        PolygonRepresentation::Face,
        PolygonRepresentation::Polyline {
            closed: true,
            curve: Default::default(),
        },
    ];
    e.scene.selection.faces = vec![2];
    let before = e.scene.clone();
    let err = e.dispatch(Command::FlipPolygons).unwrap_err().to_string();
    assert!(err.contains("not curves"), "{err}");
    assert_eq!(e.scene, before);
}
