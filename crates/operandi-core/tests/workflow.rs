use glam::{DVec2, DVec3};
use operandi_core::*;
fn counts(m: &Mesh) -> (usize, usize, usize) {
    (m.vertices.len(), m.edges().len(), m.faces.len())
}
fn box_editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.scene.selection.faces = vec![5];
    e
}
#[test]
fn vs_001_exact_history_roundtrip() {
    let mut e = box_editor();
    assert_eq!(counts(&e.scene.active().unwrap().mesh), (8, 12, 6));
    let original = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    for inset in [0.25, 0.125, 0.25] {
        e.dispatch(Command::Preview(Parameters {
            inset,
            ..Default::default()
        }))
        .unwrap();
        let m = &e.scene.active().unwrap().mesh;
        assert_eq!(counts(m), (12, 20, 10));
        for v in &m.vertices[8..] {
            assert!((v.x.abs() - (1. - inset)).abs() < 1e-9);
            assert_eq!(v.y, 1.);
            assert!((v.z.abs() - (1. - inset)).abs() < 1e-9);
        }
        assert!(m.is_closed());
    }
    e.dispatch(Command::Drop).unwrap();
    let beveled = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    for height in [0.5, 0.25, 0.5] {
        e.dispatch(Command::Preview(Parameters {
            extent: DVec3::Y * height,
            ..Default::default()
        }))
        .unwrap();
        let m = &e.scene.active().unwrap().mesh;
        assert_eq!(counts(m), (16, 28, 14));
        assert!(m.is_closed());
        m.validate().unwrap();
        for v in &m.vertices[12..] {
            assert_eq!(v.y, 1. + height);
            assert_eq!(v.x.abs(), 0.75);
            assert_eq!(v.z.abs(), 0.75);
        }
    }
    e.dispatch(Command::Drop).unwrap();
    let final_scene = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, beveled);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, original);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, beveled);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, final_scene);
    assert!(e.session.live.is_none());
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
    // Connected closed manifold, outward signed volume and Euler characteristic.
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(m.vertices.len() + m.faces.len() - m.edges().len(), 2);
    let volume: f64 = m
        .faces
        .iter()
        .map(|f| {
            let a = m.vertices[f.vertices[0] as usize];
            (1..f.vertices.len() - 1)
                .map(|j| {
                    a.dot(
                        m.vertices[f.vertices[j] as usize]
                            .cross(m.vertices[f.vertices[j + 1] as usize]),
                    ) / 6.
                })
                .sum::<f64>()
        })
        .sum();
    assert!((volume - 9.125).abs() < 1e-9);
}
#[test]
fn seams_are_corner_local() {
    let mut m = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0).unwrap();
    for (i, f) in m.faces.iter_mut().enumerate() {
        f.uv = Some(vec![DVec2::splat(i as f64); 4]);
        f.material = format!("material-{i}");
        f.smooth = i % 2 == 0;
    }
    let edited = m
        .cap_edit(5, 0.25, DVec3::ZERO)
        .unwrap()
        .cap_edit(5, 0., DVec3::Y * 0.5)
        .unwrap();
    for i in 0..6 {
        assert_eq!(edited.faces[i].uv, m.faces[i].uv);
        assert_eq!(edited.faces[i].material, m.faces[i].material);
        assert_eq!(edited.faces[i].smooth, m.faces[i].smooth);
    }
    for i in 0..5 {
        assert_eq!(edited.faces[i], m.faces[i]);
    }
    let mut s = Scene::default();
    s.new_mesh().unwrap();
    s.active_mut().unwrap().mesh = edited.into();
    assert_eq!(
        s,
        persistence::decode(&persistence::encode(&s).unwrap()).unwrap()
    );
    // Original adjacent quads with a deliberate shared-edge UV discontinuity.
    let seam = Mesh {
        attributes: Default::default(),
        uv_maps: Default::default(),
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 0., 1.),
            DVec3::new(0., 0., 1.),
            DVec3::new(2., 0., 0.),
            DVec3::new(2., 0., 1.),
        ],
        faces: vec![
            Face {
                vertices: vec![0, 1, 2, 3],
                uv: Some(vec![DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y]),
                material: "a".into(),
                smooth: false,
                part: None,
                material_label: None,
            },
            Face {
                vertices: vec![1, 4, 5, 2],
                uv: Some(vec![DVec2::splat(4.); 4]),
                material: "b".into(),
                smooth: false,
                part: None,
                material_label: None,
            },
        ]
        .into(),
    };
    let out = seam.cap_edit(0, 0.1, DVec3::ZERO).unwrap();
    assert_eq!(out.faces[1], seam.faces[1]);
    assert_eq!(out.faces[0].uv, seam.faces[0].uv);
    out.validate().unwrap();
}
#[test]
fn failures_and_cancel_are_atomic() {
    let mut e = box_editor();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let before = e.scene.clone();
    let history = e.history.undo.len();
    // A non-finite Inset refuses (an Inset past the centre no longer does,
    // PB-C-01).
    assert!(
        e.dispatch(Command::Preview(Parameters {
            inset: f64::NAN,
            ..Default::default()
        }))
        .is_err()
    );
    assert_eq!(e.scene, before);
    assert_eq!(e.history.undo.len(), history);
    let invalid = e.clone();
    assert!(e.dispatch(Command::Activate(Tool::Box)).is_err());
    assert!(e == invalid);
    e.dispatch(Command::Preview(Parameters {
        inset: 0.25,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, before);
    assert_eq!(e.history.undo.len(), history);
}
#[test]
fn items_empty_visibility_and_transform_distinction() {
    let mut e = box_editor();
    let mesh = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::X,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(mesh, e.scene.active().unwrap().mesh);
    assert_eq!(e.scene.active().unwrap().transform.translation, DVec3::X);
    e.dispatch(Command::Rename("VS_Box".into())).unwrap();
    let id = e.scene.selection.item.unwrap();
    e.dispatch(Command::Visibility(id)).unwrap();
    assert!(!e.scene.active().unwrap().visible);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::DeleteItem).unwrap();
    assert!(e.scene.items.is_empty());
    e.dispatch(Command::NewMesh).unwrap();
    assert!(e.scene.active().is_some());
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(counts(&e.scene.active().unwrap().mesh), (8, 12, 6));
}
#[test]
fn parser_and_bad_files() {
    for text in ["1 m/4", "250 mm", "(1 + 1)/8 m"] {
        assert!((numeric::parse(text).unwrap() - 0.25).abs() < 1e-12);
    }
    for text in ["NaN", "1/0", "x", "3 kg", "("] {
        assert!(numeric::parse(text).is_err());
    }
    let e = box_editor();
    let bytes = persistence::encode(&e.scene).unwrap();
    assert!(persistence::decode(&bytes[..bytes.len() / 2]).is_err());
    let text = String::from_utf8(bytes)
        .unwrap()
        .replace("\"version\": 4", "\"version\": 99");
    assert!(persistence::decode(text.as_bytes()).is_err());
}
#[test]
fn save_atomic_and_existing_file_survives_invalid_scene() {
    let mut e = box_editor();
    let p = std::env::temp_dir().join(format!(
        "operandi-original-test-{}.opscene",
        std::process::id()
    ));
    persistence::save(&p, &e.scene).unwrap();
    let original = std::fs::read(&p).unwrap();
    e.scene.active_mut().unwrap().mesh.vertices[0].x = f64::NAN;
    assert!(persistence::save(&p, &e.scene).is_err());
    assert_eq!(std::fs::read(&p).unwrap(), original);
    assert!(persistence::load(&p).is_ok());
    std::fs::remove_file(p).unwrap();
}
#[test]
fn selection_memory_order_and_bad_hits() {
    let mut e = box_editor();
    let id = e.scene.selection.item.unwrap();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.dispatch(Command::Select {
        item: id,
        hit: Component::Vertex(2),
        replace: true,
        remove: false,
    })
    .unwrap();
    e.dispatch(Command::Select {
        item: id,
        hit: Component::Vertex(3),
        replace: false,
        remove: false,
    })
    .unwrap();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    assert_eq!(e.scene.selection.vertices, vec![2, 3]);
    let before = e.scene.clone();
    assert!(
        e.dispatch(Command::Select {
            item: id,
            hit: Component::Polygon(99),
            replace: true,
            remove: false
        })
        .is_err()
    );
    assert_eq!(e.scene, before);
    e.dispatch(Command::SelectItem(None)).unwrap();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    assert!(e.dispatch(Command::Activate(Tool::Move)).is_err());
    assert_eq!(e.scene.items.len(), 1);
}
#[test]
fn plane_construction_transform_cancel_and_units() {
    for plane in 0..3 {
        let m = Mesh::box_mesh(DVec3::new(1., 2., 3.), DVec3::new(2., 4., 6.), plane).unwrap();
        m.validate().unwrap();
        assert!(m.is_closed());
        let mean = m.vertices.iter().copied().sum::<DVec3>() / 8.;
        assert_eq!(mean, DVec3::new(1., 2., 3.));
    }
    let mut e = box_editor();
    e.scene.selection.faces = vec![5];
    let source = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Y,
        ..Default::default()
    }))
    .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(m.vertices[0], source.active().unwrap().mesh.vertices[0]);
    assert_eq!(m.vertices[3].y, 2.);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(source, e.scene);
    assert_eq!(
        numeric::parse_quantity("90 deg", numeric::Quantity::Angle).unwrap(),
        90.
    );
    assert!(numeric::parse_quantity("2 mm", numeric::Quantity::Scalar).is_err());
    assert!(numeric::parse_quantity("90 deg", numeric::Quantity::Distance).is_err());
}
#[test]
fn save_does_not_remove_preexisting_temporary() {
    let e = box_editor();
    let path =
        std::env::temp_dir().join(format!("operandi-collision-{}.opscene", std::process::id()));
    let tmp = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    std::fs::write(&tmp, b"original temporary").unwrap();
    // Temporary names are unique now (operandi_files), so another file of
    // the old name is neither in the way nor touched.
    persistence::save(&path, &e.scene).unwrap();
    assert_eq!(std::fs::read(&tmp).unwrap(), b"original temporary");
    // A save that fails leaves it too.
    persistence::refuse_next_save_on_this_thread();
    assert!(persistence::save(&path, &e.scene).is_err());
    assert_eq!(std::fs::read(&tmp).unwrap(), b"original temporary");
    std::fs::remove_file(tmp).unwrap();
    std::fs::remove_file(path).unwrap();
}
#[test]
fn creation_reapply_is_single_task_and_invalid_preview_recovers() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    assert!(
        e.dispatch(Command::Preview(Parameters {
            size: DVec3::splat(f64::NAN),
            ..Default::default()
        }))
        .is_err()
    );
    assert!(e.scene.active().unwrap().mesh.faces.is_empty());
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..Default::default()
    }))
    .unwrap();
    assert_eq!(counts(&e.scene.active().unwrap().mesh), (8, 12, 6));
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), 2);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh.faces.is_empty());
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(counts(&e.scene.active().unwrap().mesh), (8, 12, 6));
}

#[test]
fn m11_history_hidden_items_identity_and_scene_state() {
    let mut e = box_editor();
    let id = e.scene.selection.item.unwrap();
    let history = e.history.undo.len();
    for tool in [Tool::Move, Tool::Rotate, Tool::Scale] {
        e.dispatch(Command::Activate(tool)).unwrap();
        // Undo/Redo work with a transform tool live and keep it (Jason,
        // 2026-11-03).
        // (The fixture's selection was set outside history, so compare meshes.)
        let mesh = e.scene.active().unwrap().mesh.clone();
        // The first Undo undoes the activation: the tool closes (MU-MID-03).
        e.dispatch(Command::Undo).unwrap();
        assert!(e.session.live.is_none());
        assert_eq!(e.scene.active().unwrap().mesh, mesh);
        e.dispatch(Command::Undo).unwrap();
        assert!(e.scene.active().unwrap().mesh.vertices.is_empty());
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene.active().unwrap().mesh, mesh);
        e.dispatch(Command::Activate(tool)).unwrap();
        e.dispatch(Command::Apply).unwrap();
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.history.undo.len(), history);
    }
    e.dispatch(Command::Visibility(id)).unwrap();
    assert!(
        e.dispatch(Command::Activate(Tool::Move))
            .unwrap_err()
            .contains("Show Mesh")
    );
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let mesh = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::X * 0.25,
        ..Default::default()
    }))
    .unwrap();
    // UIC-307: the eye toggle settles the move first (showing the mesh
    // again), then Move starts again; toggle it back hidden.
    e.dispatch(Command::Visibility(id)).unwrap();
    assert!(e.scene.active().unwrap().visible);
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Visibility(id)).unwrap();
    assert_eq!(mesh, e.scene.active().unwrap().mesh);
    assert!(!e.scene.active().unwrap().visible);
    e.dispatch(Command::Workplane(Workplane {
        fixed: true,
        plane: 2,
        center: DVec3::ZERO,
    }))
    .unwrap();
    let scene = e.scene.clone();
    assert_eq!(
        scene,
        persistence::decode(&persistence::encode(&scene).unwrap()).unwrap()
    );
    e.dispatch(Command::NewMesh).unwrap();
    let second = e.scene.selection.item.unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::NewMesh).unwrap();
    assert!(e.scene.selection.item.unwrap() > second);
}
#[test]
fn m11_failed_staged_save_preserves_exact_live_ownership_and_existing_destination() {
    let mut e = box_editor();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        inset: 0.25,
        ..Default::default()
    }))
    .unwrap();
    let before = e.clone();
    let path =
        std::env::temp_dir().join(format!("operandi-m11-stage-{}.opscene", std::process::id()));
    std::fs::write(&path, b"existing saved original").unwrap();
    let temp = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    std::fs::write(&temp, b"injected exclusive-write failure").unwrap();
    persistence::refuse_next_save_on_this_thread();
    assert!(e.save_committed(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"existing saved original");
    assert_eq!(e.scene, before.scene);
    assert_eq!(e.revision, before.revision);
    assert_eq!(
        e.session.live.as_ref().unwrap().base,
        before.session.live.as_ref().unwrap().base
    );
    assert_eq!(
        e.session.live.as_ref().unwrap().params,
        before.session.live.as_ref().unwrap().params
    );
    assert_eq!(e.history.undo.len(), before.history.undo.len());
    std::fs::remove_file(temp).unwrap();
    e.save_committed(&path).unwrap();
    assert!(e.session.live.is_none());
    assert_eq!(e.history.undo.len(), before.history.undo.len() + 1);
    assert_eq!(persistence::load(&path).unwrap(), e.scene);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn m11_empty_mesh_requires_deliberate_creation_and_trimmed_names() {
    let mut e = Editor::default();
    assert!(e.dispatch(Command::Activate(Tool::Box)).is_err());
    assert!(e.scene.items.is_empty());
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Rename("  Named Mesh  ".into()))
        .unwrap();
    assert_eq!(e.scene.active().unwrap().name, "Named Mesh");
    let before = e.scene.clone();
    assert!(e.dispatch(Command::Rename("  ".into())).is_err());
    assert_eq!(e.scene, before);
}
#[test]
fn m11_item_budget_and_invalid_identity_fail_without_partial_mutation() {
    let mut e = Editor::default();
    e.scene.next_id = u64::MAX - 1;
    let before = e.scene.clone();
    assert!(e.dispatch(Command::NewMesh).is_err());
    assert_eq!(e.scene, before);
    let mut e = box_editor();
    let before = e.scene.clone();
    assert!(e.dispatch(Command::SelectItem(Some(999))).is_err());
    assert_eq!(e.scene, before);
}

#[test]
fn m11_remove_absent_cross_mesh_hit_is_idempotent_and_item_remove_shared() {
    let mut e = box_editor();
    let a = e.scene.selection.item.unwrap();
    e.dispatch(Command::NewMesh).unwrap();
    let b = e.scene.selection.item.unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    e.dispatch(Command::SelectItem(Some(a))).unwrap();
    e.scene.selection.faces = vec![5];
    let before = e.scene.clone();
    e.dispatch(Command::Select {
        item: b,
        hit: Component::Polygon(0),
        replace: false,
        remove: true,
    })
    .unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::Select {
        item: a,
        hit: Component::Item,
        replace: false,
        remove: true,
    })
    .unwrap();
    assert!(e.scene.selection.item.is_none());
}

#[test]
fn m11_oversized_item_transform_and_duplicate_selection_are_rejected_atomically() {
    for tool in [Tool::Move, Tool::Rotate, Tool::Scale] {
        let mut e = box_editor();
        e.dispatch(Command::Mode(Mode::Item)).unwrap();
        e.dispatch(Command::Activate(tool)).unwrap();
        let before = e.scene.clone();
        let params = e.session.live.as_ref().unwrap().params.clone();
        assert!(
            e.dispatch(Command::Preview(Parameters {
                delta: DVec3::splat(1e308),
                ..Default::default()
            }))
            .is_err()
        );
        assert_eq!(e.scene, before);
        assert_eq!(e.session.live.as_ref().unwrap().params, params);
    }
    let mut e = box_editor();
    e.scene.selection.faces = vec![5, 5];
    assert!(persistence::encode(&e.scene).is_err());
}

#[test]
fn par_combined_transform_replacement_cancel_attributes_and_native_roundtrip() {
    let mut e = box_editor();
    e.scene.active_mut().unwrap().mesh.faces[5].uv =
        Some(vec![DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y]);
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    let original = e.scene.clone();
    let n = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Transform)).unwrap();
    let mut params = Parameters {
        delta: DVec3::new(3., 4., 5.),
        rotation_delta: DVec3::new(0., 0., 90.),
        scale_delta: DVec3::new(2., 3., 4.),
        ..Default::default()
    };
    e.dispatch(Command::Preview(params.clone())).unwrap();
    let scene = e.scene.clone();
    for &index in &original.active().unwrap().mesh.faces[5].vertices {
        let p = original.active().unwrap().mesh.vertices[index as usize] * params.scale_delta;
        let expected = DVec3::new(-p.y, p.x, p.z) + params.delta;
        assert!(
            (e.scene.active().unwrap().mesh.vertices[index as usize] - expected).length() < 1e-12
        );
    }
    assert_eq!(
        e.scene.active().unwrap().mesh.faces,
        original.active().unwrap().mesh.faces
    );
    e.dispatch(Command::Preview(params.clone())).unwrap();
    assert_eq!(e.scene, scene);
    // A negative component scale is refused (0 is allowed, #24).
    params.scale_delta.x = -1.;
    assert!(e.dispatch(Command::Preview(params)).is_err());
    assert_eq!(e.scene, scene);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, original);
    assert_eq!(e.history.undo.len(), n);
    e.dispatch(Command::ActionContext(ActionContext::default()))
        .unwrap();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.scene.active_mut().unwrap().transform.zxy = true;
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Transform)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(1., 2., 3.),
        rotation_delta: DVec3::new(10., 20., 30.),
        scale_delta: DVec3::new(2., 3., 4.),
        ..Default::default()
    }))
    .unwrap();
    assert!(e.scene.active().unwrap().transform.zxy);
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    let changed = e.scene.clone();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), n + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, changed);
    let path = std::env::temp_dir().join("operandi-par-combined-roundtrip.opscene");
    persistence::save(&path, &changed).unwrap();
    assert_eq!(persistence::load(&path).unwrap(), changed);
    let _ = std::fs::remove_file(path);
}
