//! MP-C11 COL-B01..03 core acceptance for basic Polygon Collapse.
//!
//! Source geometry comes from the admitted original fixture
//! `COLL-F01` (`COLLAPSE_ORIGINAL_FIXTURE.json`); its expected result is read
//! from the same file and compared by oriented position loops and source
//! association, independent of output slot order. Every other expectation
//! (transformed variants, rich data, the closed box, identities) is authored
//! literally here from the fixture mathematics. No expectation is produced by
//! calling the operation under test.
use glam::DVec3;
use operandi_core::*;
use std::collections::BTreeMap;

/// Editor with the collapse mesh active plus an unrelated tagged box.
fn editor(mesh: Mesh, selected: &[u32]) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let target = e.scene.selection.item.unwrap();
    e.scene.new_mesh().unwrap();
    let mut other = Mesh::box_mesh(DVec3::new(9., 0., 0.), DVec3::ONE, 1).unwrap();
    for (i, f) in other.faces.iter_mut().enumerate() {
        f.part = (i % 2 == 0).then(|| "Unrelated".into());
    }
    other
        .attributes
        .point_weights
        .insert("Other".into(), BTreeMap::from([(2, 0.3)]));
    for item in &mut e.scene.items {
        if item.id == target {
            item.name = "Plate".into();
            item.mesh = mesh.clone().into();
            item.transform.translation = DVec3::new(0., 0., 2.);
        } else {
            item.name = "Other".into();
            item.mesh = other.clone().into();
            item.transform.rotation = DVec3::new(0., 0.5, 0.);
        }
    }
    e.scene.select_item(Some(target));
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = selected.to_vec();
    e.scene.validate().unwrap();
    e.synchronize_identities().unwrap();
    e.undo.clear();
    e
}

fn collapse(e: &mut Editor) {
    e.dispatch(Command::CollapsePolygon)
        .unwrap_or_else(|err| panic!("collapse refused: {err}"));
}

/// Closed box: Mesh::box_mesh's literal layout (points 4..7 form the +Z
/// face, slot 1). Collapsing it leaves a closed square pyramid.
#[test]
fn col_b01_closed_box_top_becomes_pyramid() {
    let source = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0).unwrap();
    assert_eq!(source.faces[1].vertices, [4, 5, 6, 7]);
    let mut e = editor(source.clone(), &[1]);
    collapse(&mut e);
    let out = &e.scene.active().unwrap().mesh;
    let mut vertices = source.vertices[..4].to_vec();
    vertices.push(DVec3::new(0., 0., 1.));
    assert_eq!(out.vertices, vertices);
    let cycles: Vec<Vec<u32>> = out.faces.iter().map(|f| f.vertices.clone()).collect();
    assert_eq!(
        cycles,
        [
            vec![0, 3, 2, 1],
            vec![0, 4, 3],
            vec![1, 2, 4],
            vec![0, 1, 4],
            vec![3, 4, 2]
        ]
    );
    out.validate().unwrap();
}

// A 3 by 3 grid with the center face at index 4.
fn collapse_grid() -> Mesh {
    let vertices = (0..4)
        .flat_map(|y| (0..4).map(move |x| DVec3::new(x as f64, y as f64, 0.)))
        .collect();
    let faces = (0..3)
        .flat_map(|y| {
            (0..3).map(move |x| {
                let a = y * 4 + x;
                Face {
                    vertices: vec![a, a + 1, a + 5, a + 4],
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                }
            })
        })
        .collect();
    let mesh = Mesh {
        vertices,
        faces,
        ..Default::default()
    };
    mesh.validate().unwrap();
    mesh
}

fn assert_collapse_size(out: &CollapseOutcome, points: usize, faces: usize) {
    assert_eq!(out.mesh.vertices.len(), points);
    assert_eq!(out.mesh.faces.len(), faces);
    out.mesh.validate().unwrap();
}

#[test]
fn collapse_quad_in_grid() {
    let source = collapse_grid();
    let out = collapse_face(&source, 4).unwrap();
    assert_collapse_size(&out, 13, 8);
    assert_eq!(
        out.mesh.vertices[out.points[0] as usize],
        DVec3::new(1.5, 1.5, 0.)
    );
    assert_eq!(out.collapsed_faces, 1);
}

#[test]
fn collapse_triangle_in_grid_removes_degenerate_neighbour() {
    let mut source = collapse_grid();
    source.faces[4].vertices = vec![5, 6, 10];
    let mut neighbour = source.faces[4].clone();
    neighbour.vertices = vec![5, 10, 9];
    source.faces.insert(5, neighbour);
    source.validate().unwrap();
    let out = collapse_face(&source, 4).unwrap();
    assert_collapse_size(&out, 14, 8);
    assert!(!out.correspondence.face_sources.contains(&5));
    assert_eq!(
        out.mesh.vertices[out.points[0] as usize],
        DVec3::new(5. / 3., 4. / 3., 0.)
    );
}

#[test]
fn collapse_polygon_on_open_border() {
    let out = collapse_face(&collapse_grid(), 1).unwrap();
    assert_collapse_size(&out, 13, 8);
    assert_eq!(
        out.mesh.vertices[out.points[0] as usize],
        DVec3::new(1.5, 0.5, 0.)
    );
}

#[test]
fn collapse_two_adjacent_polygons_as_one_piece() {
    let out = collapse_faces(&collapse_grid(), &[4, 5]).unwrap();
    assert_collapse_size(&out, 11, 7);
    assert_eq!(out.pieces, [vec![5, 6, 7, 9, 10, 11]]);
    assert_eq!(
        out.mesh.vertices[out.points[0] as usize],
        DVec3::new(2., 1.5, 0.)
    );
    assert_eq!(out.collapsed_faces, 2);
}

#[test]
fn collapse_invalid_face_index_leaves_source_unchanged() {
    let source = collapse_grid();
    let before = source.clone();
    assert!(collapse_face(&source, 9).is_err());
    assert!(collapse_faces(&source, &[4, 9]).is_err());
    assert_eq!(source, before);
    assert_eq!(source.vertices.len(), 16);
    assert_eq!(source.faces.len(), 9);
    source.validate().unwrap();
}

#[test]
fn collapse_preserves_surviving_uv_maps_weights_and_polygon_tags() {
    use glam::DVec2;
    let mut source = collapse_grid();
    for (f, face) in source.faces.iter_mut().enumerate() {
        face.uv = Some(
            (0..face.vertices.len())
                .map(|c| DVec2::new(f as f64, c as f64))
                .collect(),
        );
        face.part = Some(format!("part-{f}"));
        face.material = format!("material-{f}");
        face.material_label = Some(format!("label-{f}"));
    }
    let uv_rows = source
        .faces
        .iter()
        .map(|face| {
            face.uv
                .as_ref()
                .unwrap()
                .iter()
                .copied()
                .map(Some)
                .collect()
        })
        .collect();
    source.uv_maps.insert("UV".into(), uv_rows);
    source.attributes.point_weights.insert(
        "weight".into(),
        BTreeMap::from([(0, 0.25), (5, 0.75), (6, 0.75), (9, 0.75), (10, 0.75)]),
    );
    source
        .attributes
        .polygon_sets
        .insert("tagged".into(), [0, 4, 5].into_iter().collect());
    source.validate().unwrap();
    let out = collapse_face(&source, 4).unwrap();
    assert_collapse_size(&out, 13, 8);
    for (i, (&f, corners)) in out
        .correspondence
        .face_sources
        .iter()
        .zip(&out.correspondence.corner_sources)
        .enumerate()
    {
        let original = &source.faces[f as usize];
        let result = &out.mesh.faces[i];
        assert_eq!(result.part, original.part);
        assert_eq!(result.material, original.material);
        assert_eq!(result.material_label, original.material_label);
        assert_eq!(
            result.uv.as_ref().unwrap(),
            &corners
                .iter()
                .map(|&c| original.uv.as_ref().unwrap()[c])
                .collect::<Vec<_>>()
        );
        assert_eq!(
            out.mesh.uv_maps["UV"][i],
            corners
                .iter()
                .map(|&c| source.uv_maps["UV"][f as usize][c])
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(
        out.mesh.attributes.point_weights["weight"].get(&0),
        Some(&0.25)
    );
    assert_eq!(
        out.mesh.attributes.point_weights["weight"].get(&out.points[0]),
        Some(&0.75)
    );
    assert_eq!(
        out.mesh.attributes.polygon_sets["tagged"],
        [0, 4].into_iter().collect()
    );
}
