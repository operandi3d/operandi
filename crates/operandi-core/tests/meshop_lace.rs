//! Lace Geometry (`laceGeom`, MESHOPS-TIDY TB-50..54): a capped tube
//! along a curve in another mesh. The spec's worked examples.
use crate::mesh_stack_tools::{add, ok, with_base};
use glam::DVec3;
use operandi_core::*;
use serde_json::json;

/// Curve 0: a straight Polyline (0,0,0)→(3,0,0) through 4 points; curve 1:
/// the L (0,0,0) (1,0,0) (1,1,0) (1,2,0).
fn guide() -> Mesh {
    let mut m = Mesh {
        vertices: (0..4).map(|i| DVec3::new(i as f64, 0., 0.)).collect(),
        ..Default::default()
    };
    m.vertices.extend([
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(1., 2., 0.),
    ]);
    let m = operandi_core::curve_commands::make_curve(&m, &[0, 1, 2, 3], false)
        .unwrap()
        .0;
    operandi_core::curve_commands::make_curve(&m, &[4, 5, 6, 7], false)
        .unwrap()
        .0
}
/// `base` with a Lace step; the guide mesh is linked when `linked`.
fn setup(base: Mesh, linked: bool) -> (Editor, u64, mesh_stack_editor::OpRef) {
    let (mut e, id) = with_base(base);
    e.scene.new_mesh().unwrap();
    let g = e.scene.items.last().unwrap().id;
    e.scene.item_mut(g).unwrap().mesh = guide().into();
    let values = if linked {
        json!({ "guide": g })
    } else {
        json!({})
    };
    let op = add(&mut e, id, "laceGeom", values);
    (e, id, op)
}
fn set(e: &mut Editor, op: mesh_stack_editor::OpRef, values: serde_json::Value) {
    let values = values.as_object().unwrap().clone();
    e.dispatch(Command::MeshOpSet { op, values }).unwrap();
}
fn plane() -> Mesh {
    // 2x2 quads in XZ: 9 points, 4 polygons.
    let mut m = Mesh::default();
    for z in 0..3 {
        for x in 0..3 {
            m.vertices.push(DVec3::new(x as f64, 0., z as f64));
        }
    }
    for z in 0..2u32 {
        for x in 0..2u32 {
            let a = z * 3 + x;
            m.faces.push(Face {
                vertices: vec![a, a + 3, a + 4, a + 1],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    m.validate().unwrap();
    m
}
/// Largest |y| and |z| of the points from `from` on.
fn extent(m: &Mesh, from: usize) -> (f64, f64) {
    m.vertices[from..].iter().fold((0f64, 0f64), |(y, z), p| {
        (y.max(p.y.abs()), z.max(p.z.abs()))
    })
}

#[test]
fn lace_without_a_guide_or_curve_leaves_the_mesh() {
    let (e, id, _) = setup(plane(), false);
    assert_eq!(ok(&e, id), plane());
    let (mut e, id, op) = setup(Mesh::default(), true);
    set(&mut e, op, json!({ "curveId": 5 }));
    assert!(ok(&e, id).faces.is_empty(), "curve 5 doesn't exist");
}

#[test]
fn lace_adds_a_capped_tube_and_keeps_the_mesh() {
    let (e, id, _) = setup(plane(), true);
    let m = ok(&e, id);
    assert_eq!(m.vertices.len(), 9 + 24, "two 12-point rings");
    assert_eq!(m.faces.len(), 4 + 12 + 2);
    assert_eq!(m.faces[..4], plane().faces[..]);
    assert_eq!(m.faces[4 + 12].vertices.len(), 12, "twelve-sided caps");
    let xs: Vec<f64> = m.vertices[9..].iter().map(|p| p.x).collect();
    assert!(xs.iter().all(|x| x.abs() < 1e-9 || (x - 3.).abs() < 1e-9));
    let (y, z) = extent(&m, 9);
    assert!((y - 0.002).abs() < 1e-9 && (z - 0.002).abs() < 1e-9);
}

#[test]
fn lace_profiles() {
    let (mut e, id, op) = setup(Mesh::default(), true);
    set(&mut e, op, json!({ "radius": 0.1, "sides": 4 }));
    let m = ok(&e, id);
    assert_eq!((m.vertices.len(), m.faces.len()), (8, 6));
    // TB-52: point 0 at (0, 0, −r), point 1 at (0, +r, 0).
    assert!(m.vertices[0].distance(DVec3::new(0., 0., -0.1)) < 1e-9);
    assert!(m.vertices[1].distance(DVec3::new(0., 0.1, 0.)) < 1e-9);
    let close = |(y, z): (f64, f64), want: (f64, f64)| {
        assert!(
            (y - want.0).abs() < 1e-4 && (z - want.1).abs() < 1e-4,
            "{y} {z}"
        );
    };
    close(extent(&m, 0), (0.1, 0.1));
    set(&mut e, op, json!({ "shape": 2 }));
    close(extent(&ok(&e, id), 0), (0.0707, 0.0707));
    set(&mut e, op, json!({ "shape": 1, "aspect": 0.5 }));
    close(extent(&ok(&e, id), 0), (0.05, 0.1));
    set(&mut e, op, json!({ "shape": 3 }));
    close(extent(&ok(&e, id), 0), (0.0383, 0.0924));
}

#[test]
fn lace_start_end_trim_the_tube() {
    let (mut e, id, op) = setup(Mesh::default(), true);
    set(&mut e, op, json!({ "radius": 0.1, "end": 0.5 }));
    let m = ok(&e, id);
    let max_x = m.vertices.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    assert!((max_x - 1.5).abs() < 1e-9, "{max_x}");
    // TB-Q6: a middle piece trims properly too.
    set(&mut e, op, json!({ "start": 0.25, "end": 0.75 }));
    let m = ok(&e, id);
    let (lo, hi) = m
        .vertices
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
    assert!(
        (lo - 0.75).abs() < 1e-9 && (hi - 2.25).abs() < 1e-9,
        "{lo} {hi}"
    );
    set(&mut e, op, json!({ "start": 0.8, "end": 0.2 }));
    assert!(ok(&e, id).faces.is_empty(), "Start past End: no tube");
}

#[test]
fn lace_follows_a_bend_with_extra_rings() {
    let (mut e, id, op) = setup(Mesh::default(), true);
    set(
        &mut e,
        op,
        json!({ "radius": 0.1, "sides": 4, "curveId": 1 }),
    );
    let m = ok(&e, id);
    let rings = m.vertices.len() / 4;
    assert!(rings > 2, "extra rings round the corner: {rings}");
    // It reaches the L's far end.
    assert!(m.vertices.iter().any(|p| (p.y - 2.).abs() < 1e-6));
    // A polyline L has one sharp corner: Refinement Angle can't smooth
    // it, but each ring's step stays one turn.
    assert!(m.validate().is_ok());
}

#[test]
fn lace_writes_uvs_by_pattern_size() {
    let (mut e, id, op) = setup(Mesh::default(), true);
    set(&mut e, op, json!({ "radius": 0.1, "sides": 4 }));
    let m = ok(&e, id);
    let map = &m.uv_maps["Texture"];
    let longest = map
        .iter()
        .flatten()
        .flatten()
        .map(|uv| uv.y)
        .fold(0f64, f64::max);
    assert!((longest - 3. / 0.35).abs() < 1e-6, "{longest}");
    set(&mut e, op, json!({ "uvs": 0 }));
    assert!(!ok(&e, id).uv_maps.contains_key("Texture"));
}
