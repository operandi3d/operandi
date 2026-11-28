//! SA-T01..12 / PT-T-SPI-01..03 numeric-domain acceptance.
use glam::{DVec2, DVec3};
use operandi_core::*;
use serde_json::Value;
fn point(v: &Value) -> DVec3 {
    DVec3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}
fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        part: None,
        material_label: None,
        smooth: false,
    }
}
fn mesh(input: &Value) -> Mesh {
    let mut m = Mesh {
        vertices: input["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .map(point)
            .collect(),
        faces: input["faces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                face(
                    f.as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap() as u32)
                        .collect(),
                )
            })
            .collect(),
        ..Default::default()
    };
    for i in 0..m.faces.len() {
        if m.normal(i) == DVec3::ZERO {
            m.attributes.degenerate_faces.insert(i as u32);
        }
    }
    m
}
fn square() -> Mesh {
    mesh(&serde_json::json!({"vertices":[[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]],"faces":[[0,1,2,3]]}))
}
fn editor(mesh: Mesh) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.history.undo.clear();
    e
}
#[test]
fn sa_t07_every_sparse_uv_corner_count_and_asymmetric_chart() {
    for n in [3, 4, 5] {
        let mut source = if n == 4 {
            square()
        } else {
            let vertices = (0..n)
                .map(|i| {
                    let a = i as f64 / n as f64 * std::f64::consts::TAU;
                    DVec3::new(a.cos(), a.sin(), 0.)
                })
                .collect();
            Mesh {
                vertices,
                faces: vec![face((0..n).collect())].into(),
                ..Default::default()
            }
        };
        for assigned in 0..=n {
            let uv: Vec<_> = (0..n)
                .map(|i| {
                    if i < assigned {
                        Some(DVec2::new(0.1 * (i * i) as f64, 0.2 * (i + 1) as f64))
                    } else {
                        None
                    }
                })
                .collect();
            source.uv_maps.insert("Sparse".into(), vec![uv.clone()]);
            let r = spike_geometry(&source, &[], 0.37).unwrap();
            for (i, f) in r.mesh.uv_maps["Sparse"].iter().enumerate() {
                assert_eq!(f[0], uv[i]);
                assert_eq!(f[1], uv[(i + 1) % n as usize]);
                assert_eq!(f[2].is_some(), assigned == n);
                if assigned == n {
                    assert!(
                        f[2].unwrap()
                            .distance(uv.iter().flatten().copied().sum::<DVec2>() / n as f64)
                            < 1e-12
                    );
                }
            }
        }
    }
}
#[test]
fn sa_t09_t12_discrete_apply_history_invalid_recovery_and_exact_roundtrip() {
    let mut e = editor(square());
    let id = e.scene.active().unwrap().id;
    e.scene.active_mut().unwrap().transform.scale = DVec3::new(2., 3., 4.);
    e.scene.workplane.center = DVec3::new(4., 5., 6.);
    let before = e.scene.clone();
    e.dispatch(Command::ArmSpike).unwrap();
    e.dispatch(Command::ConfigureSpike(0.5)).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Apply).unwrap();
    let first = e.scene.clone();
    assert_eq!(first.active().unwrap().mesh.vertices[4], DVec3::Z);
    assert_eq!(
        first.active().unwrap().transform.scale,
        before.active().unwrap().transform.scale
    );
    // UIC-78 (Jason, 2026-11-25): the first Apply stays live, so a new
    // Strength reshapes it in the same undo step (the spec's discrete
    // Apply left it alone).
    e.dispatch(Command::ConfigureSpike(0.25)).unwrap();
    let first = e.scene.clone();
    assert_eq!(first.active().unwrap().mesh.vertices[4], DVec3::Z * 0.5);
    assert_eq!(e.history.undo.len(), 1);
    e.dispatch(Command::Apply).unwrap();
    let second = e.scene.clone();
    assert_eq!(second.active().unwrap().mesh.vertices.len(), 9);
    assert_eq!(second.active().unwrap().mesh.faces.len(), 12);
    assert_eq!(e.history.undo.len(), 2);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, first);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, second);
    assert!(e.session.spike.is_none());
    e.dispatch(Command::ArmSpike).unwrap();
    let checkpoints = e.history.undo.len();
    assert!(e.dispatch(Command::ConfigureSpike(f64::NAN)).is_err());
    assert_eq!(e.session.spike.as_ref().unwrap().requested, 0.25);
    e.dispatch(Command::ConfigureSpike(f64::MAX)).unwrap();
    assert!(e.dispatch(Command::Apply).is_err());
    assert_eq!(e.scene, second);
    assert_eq!(e.history.undo.len(), checkpoints);
    e.dispatch(Command::ConfigureSpike(-0.1)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, second);
    e.dispatch(Command::ArmSpike).unwrap();
    let path = std::env::temp_dir().join(format!("operandi-spike-{}.opscene", std::process::id()));
    e.save_committed(&path).unwrap();
    assert!(e.session.spike.is_none());
    assert_eq!(persistence::load(&path).unwrap(), second);
    std::fs::remove_file(path).unwrap();
    e.scene.selection.item = None;
    // No mesh selected since the scene opened, so none stays active (VP-04).
    e.scene.last_mesh = Default::default();
    assert!(e.dispatch(Command::ArmSpike).is_err());
    assert_eq!(e.scene.items[0].id, id);
}

#[test]
fn sa_t02_t05_closed_box_symmetric_triangle_and_disconnected_orientations() {
    let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0).unwrap();
    for strength in [-0.1, 0., 0.1] {
        let r = spike_geometry(&cube, &[], strength).unwrap();
        assert_eq!(
            (
                r.mesh.vertices.len(),
                r.mesh.edges().len(),
                r.mesh.faces.len()
            ),
            (14, 36, 24)
        );
        assert!(r.mesh.is_closed());
        for fi in 0..6 {
            assert!(
                r.mesh.vertices[8 + fi]
                    .distance(cube.center(fi) + cube.normal(fi) * (2. * strength))
                    < 1e-12
            );
        }
    }
    let tri = Mesh {
        vertices: vec![
            DVec3::X,
            DVec3::new(-0.5, 3_f64.sqrt() / 2., 0.),
            DVec3::new(-0.5, -3_f64.sqrt() / 2., 0.),
        ],
        faces: vec![face(vec![0, 1, 2])].into(),
        ..Default::default()
    };
    for strength in [-0.1, 0., 0.1] {
        let r = spike_geometry(&tri, &[], strength).unwrap();
        assert!(r.mesh.vertices[3].distance(DVec3::Z * (3_f64.sqrt() * strength)) < 1e-12);
    }
    let mut islands = square();
    let mut second = square();
    for v in &mut second.vertices {
        *v = DVec3::new(v.x, 0., v.y) + DVec3::X * 5.;
    }
    append_mesh(&mut islands, second).unwrap();
    let out = spike_geometry(&islands, &[1, 0], 0.2).unwrap();
    assert!(out.mesh.vertices[8].distance(DVec3::Z * 0.4) < 1e-12);
    assert!(out.mesh.vertices[9].distance(DVec3::new(5., -0.4, 0.)) < 1e-12);
}
#[test]
fn sa_t09_t11_target_and_context_refusals_preserve_state() {
    let mut e = editor(square());
    let original = e.scene.clone();
    e.scene.active_mut().unwrap().visible = false;
    assert!(e.dispatch(Command::ArmSpike).is_err());
    e.scene = original.clone();
    e.scene.active_mut().unwrap().transform.scale = DVec3::new(-1., 1., 1.);
    assert!(e.dispatch(Command::ArmSpike).is_err());
    e.scene = original.clone();
    e.action_context.center = CenterMode::Origin;
    assert!(e.dispatch(Command::ArmSpike).is_err());
    e.action_context = Default::default();
    e.scene.active_mut().unwrap().transform.translation = DVec3::new(5., -2., 3.);
    e.scene.active_mut().unwrap().transform.rotation =
        DVec3::new(0., 0., std::f64::consts::FRAC_PI_2);
    e.dispatch(Command::ArmSpike).unwrap();
    let state = e.clone();
    assert!(e.dispatch(Command::SelectItem(None)).is_err());
    assert!(e == state);
    e.dispatch(Command::ConfigureSpike(0.2)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh.vertices[4],
        DVec3::new(0., 0., 0.4)
    );
    let mut malformed = square();
    malformed.faces[0].vertices[1] = 0;
    assert!(spike_geometry(&malformed, &[], 0.1).is_err());
}

#[test]
fn sa_t11_operation_budgets_and_identity_exhaustion_are_atomic() {
    let mut large = Mesh::default();
    for i in 0..33_334 {
        let base = large.vertices.len() as u32;
        let x = i as f64 * 2.;
        large.vertices.extend([
            DVec3::new(x, 0., 0.),
            DVec3::new(x + 1., 0., 0.),
            DVec3::new(x, 1., 0.),
        ]);
        large.faces.push(face(vec![base, base + 1, base + 2]));
    }
    let mut e = editor(large);
    e.dispatch(Command::ArmSpike).unwrap();
    e.dispatch(Command::ConfigureSpike(0.1)).unwrap();
    let original = e.scene.clone();
    assert!(e.dispatch(Command::Apply).unwrap_err().contains("budget"));
    assert_eq!(e.scene, original);
    assert!(e.history.undo.is_empty());
    let mut m = square();
    m.uv_maps
        .insert("Known".into(), vec![vec![Some(DVec2::ZERO); 4]]);
    m.attributes.uv_records.insert(
        "Known".into(),
        vec![vec![Some(u32::MAX), Some(1), Some(2), Some(3)]],
    );
    m.validate().unwrap();
    assert!(
        spike_geometry(&m, &[], 0.1)
            .unwrap_err()
            .contains("identity budget")
    );
}
