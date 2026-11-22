//! Teapot and Teapot Item against specifications/missing-tools-g2
//! (G2-PLANE-TEAPOT v1, MT6-01..09). The shape is the public Utah Teapot,
//! so proportions are checked against its data within the tessellation's
//! tolerance; Operandi's own density is not compared with the reference.
use glam::DVec3;
use operandi_core::teapot::{TeapotParameters, teapot_geometry};
use operandi_core::*;

fn bounds(m: &Mesh) -> (DVec3, DVec3) {
    m.vertices.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(lo, hi), v| (lo.min(*v), hi.max(*v)),
    )
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// The vertex furthest along `dir`.
fn extreme(m: &Mesh, dir: DVec3) -> (u32, DVec3) {
    m.vertices
        .iter()
        .enumerate()
        .map(|(i, v)| (i as u32, *v))
        .max_by(|a, b| a.1.dot(dir).total_cmp(&b.1.dot(dir)))
        .unwrap()
}

/// The summed normal of the polygons round point `v`.
fn normal_at(m: &Mesh, v: u32) -> DVec3 {
    (0..m.faces.len())
        .filter(|&f| m.faces[f].vertices.contains(&v))
        .map(|f| m.normal(f))
        .sum()
}

#[test]
fn mt6_01_proportions_with_axis_y_and_size_1() {
    let m = teapot_geometry(&TeapotParameters::default()).unwrap();
    let (lo, hi) = bounds(&m);
    // Spout tip to handle is the longest side: exactly 1 along Z.
    assert!(
        near(lo.z, -0.5, 1e-12) && near(hi.z, 0.5, 1e-12),
        "{lo} {hi}"
    );
    // 4.0 and 3.15 to 6.43 (the data's width, height and length).
    assert!(
        near(hi.x, 0.311061, 2e-3) && near(lo.x, -hi.x, 1e-9),
        "{hi}"
    );
    assert!(
        near(hi.y, 0.244961, 2e-3) && near(lo.y, -hi.y, 1e-9),
        "{hi}"
    );
}

#[test]
fn mt6_02_up_is_y_the_spout_points_minus_z_and_the_handle_plus_z() {
    let m = teapot_geometry(&TeapotParameters::default()).unwrap();
    let (lo, hi) = bounds(&m);
    // The knob's top and the bottom pole sit on the pot's centre line,
    // 0.0334 toward the handle.
    let (_, top) = extreme(&m, DVec3::Y);
    let (_, bottom) = extreme(&m, -DVec3::Y);
    for (p, y) in [(top, hi.y), (bottom, lo.y)] {
        assert!(near(p.x, 0., 1e-9) && near(p.y, y, 1e-12), "{p}");
        assert!(near(p.z, 0.0334, 2e-3), "{p}");
    }
    // The spout's tip is the most −Z point, high up; the handle +Z.
    let (_, spout) = extreme(&m, -DVec3::Z);
    assert!(spout.y > 0.05, "spout tip {spout}");
    let (_, handle) = extreme(&m, DVec3::Z);
    assert!(handle.y > -0.1 && handle.y < 0.15, "handle {handle}");
    // Symmetric across X = 0.
    for v in &m.vertices {
        let mirror = DVec3::new(-v.x, v.y, v.z);
        assert!(
            m.vertices.iter().any(|w| (*w - mirror).length() < 1e-9),
            "{v} has no mirror"
        );
    }
}

#[test]
fn mt6_03_04_closed_rounded_bottom_lid_quads_and_pole_triangles() {
    let m = teapot_geometry(&TeapotParameters::default()).unwrap();
    let (lo, hi) = bounds(&m);
    // One bottom pole point and one knob-top point.
    let at = |y: f64| m.vertices.iter().filter(|v| near(v.y, y, 1e-12)).count();
    assert_eq!(at(lo.y), 1, "a single bottom pole point");
    assert_eq!(at(hi.y), 1, "a single knob-top point");
    let (top, _) = extreme(&m, DVec3::Y);
    let (bottom, _) = extreme(&m, -DVec3::Y);
    // Quads, with triangles only round the poles.
    for f in &m.faces {
        match f.vertices.len() {
            4 => {}
            3 => assert!(
                f.vertices.contains(&top) || f.vertices.contains(&bottom),
                "a triangle away from a pole"
            ),
            n => panic!("a {n}-sided polygon"),
        }
    }
    // No duplicate points: patch edges are welded.
    for (i, a) in m.vertices.iter().enumerate() {
        for b in &m.vertices[i + 1..] {
            assert!((*a - *b).length() > 1e-9, "duplicate point {a}");
        }
    }
    // Every polygon faces out: up at the knob, down at the bottom.
    assert!(normal_at(&m, top).y > 0.);
    assert!(normal_at(&m, bottom).y < 0.);
    // All of it in one mesh, Face polygons.
    assert!(!m.has_polylines());
}

#[test]
fn mt6_02_axis_x_and_z_turn_up_onto_that_axis() {
    for (axis, up) in [(0usize, DVec3::X), (2, DVec3::Z)] {
        let m = teapot_geometry(&TeapotParameters {
            axis,
            ..Default::default()
        })
        .unwrap();
        let (top, p) = extreme(&m, up);
        assert!(
            normal_at(&m, top).dot(up) > 0.,
            "axis {axis}: knob faces up"
        );
        assert!(near(p.dot(up), 0.244961, 2e-3), "axis {axis}: {p}");
        let (lo, hi) = bounds(&m);
        assert!(near((hi - lo).max_element(), 1., 1e-12));
    }
}

#[test]
fn size_and_position_set_the_box() {
    let p = TeapotParameters {
        position: DVec3::new(1., 2., 3.),
        size: 2.,
        ..Default::default()
    };
    let (lo, hi) = bounds(&teapot_geometry(&p).unwrap());
    assert!(((lo + hi) / 2. - p.position).length() < 1e-9);
    assert!(near(hi.z - lo.z, 2., 1e-12));
    assert!(
        teapot_geometry(&TeapotParameters {
            size: 0.,
            ..Default::default()
        })
        .is_err()
    );
}

#[test]
fn make_uvs_adds_a_texture_map_inside_the_unit_square() {
    let m = teapot_geometry(&TeapotParameters::default()).unwrap();
    let rows = &m.uv_maps["Texture"];
    assert_eq!(rows.len(), m.faces.len());
    for (row, f) in rows.iter().zip(m.faces.iter()) {
        assert_eq!(row.len(), f.vertices.len());
        for uv in row {
            let uv = uv.unwrap();
            assert!((0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y));
        }
    }
    let off = teapot_geometry(&TeapotParameters {
        make_uvs: false,
        ..Default::default()
    })
    .unwrap();
    assert!(off.uv_maps.is_empty());
}

#[test]
fn mt6_05_teapot_item_is_a_new_selected_item_and_one_undo_step() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let first = e.scene.active().unwrap().id;
    let steps = e.undo.len();
    e.dispatch(Command::InsertShapeUnit(
        UnitShape::Teapot,
        UnitDestination::NewMesh,
    ))
    .unwrap();
    let item = e.scene.active().unwrap();
    assert_eq!(item.name, "Teapot");
    assert_ne!(item.id, first);
    assert!(!item.mesh.faces.is_empty());
    assert!(
        e.scene.item(first).unwrap().mesh.faces.is_empty(),
        "unchanged"
    );
    let (lo, hi) = bounds(&item.mesh);
    assert!(((lo + hi) / 2.).length() < 1e-9, "at the origin");
    assert_eq!(e.undo.len(), steps + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().id, first);
}

#[test]
fn the_teapot_tool_previews_and_applies() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::ArmShape(ShapeKind::Teapot)).unwrap();
    let p = TeapotParameters {
        size: 2.,
        ..Default::default()
    };
    e.dispatch(Command::ConfigureShape(ShapeParameters::Teapot(p)))
        .unwrap();
    e.dispatch(Command::Drop).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    let (lo, hi) = bounds(mesh);
    assert!(near(hi.z - lo.z, 2., 1e-9), "{lo} {hi}");
    // Cancel leaves the scene as it was.
    let before = e.scene.clone();
    e.dispatch(Command::ArmShape(ShapeKind::Teapot)).unwrap();
    e.dispatch(Command::ConfigureShape(ShapeParameters::Teapot(
        TeapotParameters::default(),
    )))
    .unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.scene.active().unwrap().mesh == before.active().unwrap().mesh);
}
