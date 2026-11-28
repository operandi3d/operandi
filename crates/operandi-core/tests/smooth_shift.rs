use glam::DVec3;
use operandi_core::*;

fn editor(tool: Tool) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: vec![
            DVec3::new(-1., -1., 0.),
            DVec3::new(1., -1., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(-1., 1., 0.),
        ],
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: "Original".into(),
            smooth: false,
            part: Some("OriginalPart".into()),
            material_label: None,
        }]
        .into(),
        ..Default::default()
    }
    .into();
    e.history.undo.clear();
    e.dispatch(Command::Activate(tool)).unwrap();
    e
}
/// The shifted copy of the fixture's lone quad: the polygon made only of new
/// points (the fixture has points 0..4). Found by geometry because with
/// nothing selected the cap doesn't end up selected (Q-SS-10).
fn cap_face(m: &Mesh) -> usize {
    let caps: Vec<usize> = (0..m.faces.len())
        .filter(|&f| m.faces[f].vertices.len() == 4 && m.faces[f].vertices.iter().all(|&v| v >= 4))
        .collect();
    assert_eq!(caps.len(), 1, "expected one cap polygon");
    caps[0]
}

#[test]
fn sst_both_identities_live_replacement_cancel_apply_continuation_and_history() {
    for tool in [Tool::SmoothShift, Tool::Thicken] {
        let mut e = editor(tool);
        let source = e.scene.clone();
        let mut p = e.session.live.as_ref().unwrap().params.smooth_shift;
        assert_eq!(p.thicken, tool == Tool::Thicken);
        assert_eq!(e.scene, source);
        p.offset = 0.2;
        e.dispatch(Command::ConfigureSmoothShift(p)).unwrap();
        assert_eq!(e.scene, source);
        e.dispatch(Command::Apply).unwrap();
        assert!(!e.session.live.as_ref().unwrap().applied);
        assert_eq!(e.history.undo.len(), 1);
        assert_eq!(e.history.undo[0].label, tool.label());
        let first = e.scene.clone();
        p.offset = 0.1;
        e.dispatch(Command::ConfigureSmoothShift(p)).unwrap();
        e.dispatch(Command::Apply).unwrap();
        let second = e.scene.clone();
        assert_ne!(first, second);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, first);
        assert!(e.session.live.is_none());
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, second);
        e = editor(tool);
        p = e.session.live.as_ref().unwrap().params.smooth_shift;
        p.offset = 0.1;
        e.dispatch(Command::ConfigureSmoothShift(p)).unwrap();
        let second = e.scene.clone();
        let l = e.session.live.as_ref().unwrap().params.clone();
        e.dispatch(Command::Preview(l)).unwrap();
        let live = e.scene.clone();
        p.offset = 0.3;
        e.dispatch(Command::ConfigureSmoothShift(p)).unwrap();
        p.offset = 0.1;
        e.dispatch(Command::ConfigureSmoothShift(p)).unwrap();
        assert_eq!(e.scene, live);
        assert!(e.dispatch(Command::Apply).is_err());
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.scene, second);
        e.dispatch(Command::Activate(tool)).unwrap();
        let l = e.session.live.as_ref().unwrap().params.clone();
        e.dispatch(Command::Preview(l)).unwrap();
        let stage = e.scene.clone();
        e.dispatch(Command::Continue).unwrap();
        assert_eq!(e.scene, stage);
        assert_eq!(
            e.session.live.as_ref().unwrap().params.smooth_shift.offset,
            0.
        );
        assert_eq!(
            e.session.live.as_ref().unwrap().params.smooth_shift.scale,
            1.
        );
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.scene, stage);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, second);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, stage);
        let bytes = persistence::encode(&e.scene).unwrap();
        assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
    }
}
#[test]
fn sst_states_namespaces_reset_retention_and_invalid_recovery() {
    assert_eq!(
        numeric::parse_quantity("50% + 25%", numeric::Quantity::Percent).unwrap(),
        75.
    );
    assert!(numeric::parse_quantity("50%", numeric::Quantity::Scalar).is_err());
    let mut e = editor(Tool::Thicken);
    let mut p = SmoothShiftParameters {
        offset: 0.4,
        scale: -0.5,
        sharp: true,
        angle_weight: true,
        both_sides: true,
        ..Default::default()
    };
    e.dispatch(Command::ConfigureSmoothShift(p)).unwrap();
    e.dispatch(Command::SurfaceStateSave("A".into())).unwrap();
    e.dispatch(Command::SurfaceStateRecall(None)).unwrap();
    e.dispatch(Command::ResetSmoothShift).unwrap();
    // Under Thicken, Reset with no state gives Thicken's own values (tool
    // presets TP-15, 2026-11-15; before, Smooth Shift's).
    assert_eq!(
        e.session.live.as_ref().unwrap().params.smooth_shift,
        tool_preset::thicken_values()
    );
    e.dispatch(Command::SurfaceStateRecall(Some("A".into())))
        .unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().params.smooth_shift, p);
    e.dispatch(Command::SurfaceStateRename("B".into())).unwrap();
    e.dispatch(Command::SurfaceStateDelete).unwrap();
    assert!(e.surface_state.is_empty());
    p.offset = f64::INFINITY;
    assert!(e.dispatch(Command::ConfigureSmoothShift(p)).is_err());
    let before = e.scene.clone();
    assert!(e.dispatch(Command::Apply).is_err());
    assert_eq!(e.scene, before);
    e.dispatch(Command::ResetSmoothShift).unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Activate(Tool::SmoothShift)).unwrap();
    assert_eq!(
        e.session.live.as_ref().unwrap().params.smooth_shift,
        SmoothShiftParameters::default()
    );
    assert!(!e.surface_states.contains_key("Smooth Shift"));
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Activate(Tool::Thicken)).unwrap();
    // Thicken starts from where its last Reset left it: its own values
    // (TP-15).
    assert_eq!(
        e.session.live.as_ref().unwrap().params.smooth_shift,
        tool_preset::thicken_values()
    );
}
#[test]
fn sst_typed_storage_limits_append_native_version_and_unrelated_preservation() {
    use std::collections::BTreeMap;
    let mut e = editor(Tool::SmoothShift);
    let source = e.scene.active_mut().unwrap();
    source.mesh.attributes.typed_maps.insert(
        "Target".into(),
        TypedMap {
            kind: "absolute_position3".into(),
            dimension: 3,
            zero_default: false,
            points: BTreeMap::from([(0, vec![8., 9., 10.])]),
            corners: vec![vec![None; 4]],
        },
    );
    source
        .mesh
        .attributes
        .vertex_sets
        .insert("V".into(), [0].into());
    source
        .mesh
        .attributes
        .polygon_sets
        .insert("P".into(), [0].into());
    // Activate captured the old scene; recapture after adding original attributes.
    e.session.live = None;
    e.dispatch(Command::Activate(Tool::SmoothShift)).unwrap();
    let source = e.scene.clone();
    let mut extra = source.items[0].clone();
    extra.id = 2;
    extra.name = "Unrelated".into();
    e.scene.items.push(extra.clone());
    e.scene.next_id = 3;
    e.session.live.as_mut().unwrap().base = e.scene.clone();
    e.dispatch(Command::ConfigureSmoothShift(SmoothShiftParameters {
        offset: 0.2,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(e.scene.items[1], extra);
    let m = &e.scene.items[0].mesh;
    assert_eq!(
        m.attributes.typed_maps["Target"].points[&4],
        vec![8., 9., 10.]
    );
    assert_eq!(m.attributes.vertex_sets["V"], [0, 4].into());
    assert_eq!(m.attributes.polygon_sets["P"].len(), 5);
    let bytes = persistence::encode(&e.scene).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 7"));
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
    let old = String::from_utf8(bytes)
        .unwrap()
        .replace("\"version\": 7", "\"version\": 6");
    assert!(persistence::decode(old.as_bytes()).is_err());
    let mut appended = m.clone();
    append_mesh(&mut appended, m.to_owned_value()).unwrap();
    appended.validate().unwrap();
    assert_eq!(appended.attributes.typed_maps["Target"].points.len(), 4);
    let mut bad = m.clone();
    bad.attributes
        .typed_maps
        .get_mut("Target")
        .unwrap()
        .dimension = 6;
    assert!(smooth_shift_geometry(&bad, &[], Default::default(), None).is_err());
}
#[test]
fn sst_local_context_falloff_only_offsets_and_source_integrity() {
    let mut e = editor(Tool::SmoothShift);
    e.dispatch(Command::Cancel).unwrap();
    let m = &mut e.scene.active_mut().unwrap().mesh;
    for v in &mut m.vertices {
        v.x = if v.x < 0. { 2. } else { 6. };
    }
    let source = m.clone();
    let p = SmoothShiftParameters {
        offset: 0.4,
        scale: 0.5,
        ..Default::default()
    };
    let r = smooth_shift_geometry(&source, &[], p, Some(&[1., 0., 0., 1.])).unwrap();
    assert_eq!(r.mesh.vertices[4], DVec3::new(1., -0.5, 0.2));
    assert_eq!(r.mesh.vertices[5], DVec3::new(3., -0.5, 0.));
    assert_eq!(source.vertices[0], DVec3::new(2., -1., 0.));
    e.scene.active_mut().unwrap().transform = scene::Transform {
        translation: DVec3::new(4., -2., 1.),
        rotation: DVec3::new(0.1, 0.2, 0.3),
        scale: DVec3::new(-2., 3., 0.5),
        ..Default::default()
    };
    let transform = e.scene.active().unwrap().transform.clone();
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    e.dispatch(Command::Activate(Tool::SmoothShift)).unwrap();
    e.dispatch(Command::ConfigureSmoothShift(p)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(e.scene.active().unwrap().transform, transform);
    assert_eq!(
        e.scene.active().unwrap().mesh.vertices[4],
        DVec3::new(1., -0.5, 0.2)
    );
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::SelectItem(None)).unwrap();
    // No mesh selected since the scene opened, so none stays active (VP-04).
    e.scene.last_mesh = Default::default();
    let before = e.scene.clone();
    assert!(e.dispatch(Command::Activate(Tool::Thicken)).is_err());
    assert_eq!(e.scene, before);
}

#[test]
fn sst_legacy_sparse_uv_identity_and_crease_descendants_survive() {
    use glam::DVec2;
    let mut e = editor(Tool::Thicken);
    e.session.live = None;
    let m = &mut e.scene.active_mut().unwrap().mesh;
    m.uv_maps.insert(
        "UV".into(),
        vec![vec![
            Some(DVec2::ZERO),
            Some(DVec2::X),
            None,
            Some(DVec2::Y),
        ]],
    );
    m.attributes
        .uv_records
        .insert("UV".into(), vec![vec![Some(10), Some(11), None, Some(12)]]);
    m.attributes.creases.push(MeshCrease {
        a: 0,
        b: 1,
        weight: 0.75,
    });
    let source = m.clone();
    let result = smooth_shift_geometry(
        &source,
        &[],
        SmoothShiftParameters {
            offset: 0.2,
            thicken: true,
            ..Default::default()
        },
        None,
    )
    .unwrap();
    result.mesh.validate().unwrap();
    assert_eq!(result.mesh.attributes.creases.len(), 2);
    assert_eq!(result.mesh.uv_maps["UV"][0][2], None);
    let base = result
        .mesh
        .faces
        .iter()
        .position(|f| f.vertices == vec![0, 3, 2, 1])
        .unwrap();
    assert_eq!(
        result.mesh.attributes.uv_records["UV"][base],
        vec![Some(10), Some(12), None, Some(11)]
    );
    let cap_id = result.mesh.attributes.uv_records["UV"][0][0].unwrap();
    assert!(cap_id > 12);
    assert_ne!(cap_id, 10);
}

#[test]
fn sst_both_identities_all_checkbox_combinations_signed_zero_apply_and_roundtrip() {
    for tool in [Tool::SmoothShift, Tool::Thicken] {
        for flags in 0..16 {
            for offset in [-0.2, 0., 0.2] {
                for scale in [-1., 0., 1.5] {
                    let mut e = editor(tool);
                    // Origin-centered Scale, as in the command oracles.
                    e.action_context.center = CenterMode::Origin;
                    let source = e.scene.clone();
                    let p = SmoothShiftParameters {
                        offset,
                        scale,
                        angle: 90.1,
                        thicken: flags & 1 != 0,
                        sharp: flags & 2 != 0,
                        angle_weight: flags & 4 != 0,
                        both_sides: flags & 8 != 0,
                    };
                    e.dispatch(Command::ConfigureSmoothShift(p)).unwrap();
                    assert_eq!(e.scene, source);
                    e.dispatch(Command::Apply).unwrap();
                    let m = &e.scene.active().unwrap().mesh;
                    assert_eq!(m.faces.len(), if p.thicken { 6 } else { 5 });
                    // Nothing was selected, so only bridge quads end up
                    // selected, and a lone quad has none (Q-SS-10).
                    assert!(e.scene.selection.faces.is_empty());
                    let half = if p.both_sides { 0.5 } else { 1. };
                    let cap = cap_face(m);
                    assert!(
                        m.vertices[m.faces[cap].vertices[0] as usize].distance(
                            scale * (source.items[0].mesh.vertices[0] + DVec3::Z * half * offset)
                        ) < 1e-12
                    );
                    if p.both_sides {
                        assert_eq!(
                            m.vertices[0],
                            scale * (source.items[0].mesh.vertices[0] - DVec3::Z * 0.5 * offset)
                        );
                    } else {
                        assert_eq!(m.vertices[0], source.items[0].mesh.vertices[0]);
                    }
                    e.dispatch(Command::Drop).unwrap();
                    let result = e.scene.clone();
                    e.dispatch(Command::Undo).unwrap();
                    assert_eq!(e.scene, source);
                    e.dispatch(Command::Redo).unwrap();
                    assert_eq!(e.scene, result);
                    assert_eq!(
                        persistence::decode(&persistence::encode(&result).unwrap()).unwrap(),
                        result
                    );
                }
            }
        }
    }
}

#[test]
fn sst_default_action_center_scales_about_the_selection() {
    // User correction 2026-10-30: with the default action center, Scale
    // alone keeps a lifted cap in its plane and Offset alone keeps its size.
    // Q-SS-04 (2026-11-18): Selection gives the same centre as no action
    // centre.
    for (tool, center) in [Tool::SmoothShift, Tool::Thicken]
        .into_iter()
        .flat_map(|t| [(t, CenterMode::None), (t, CenterMode::Selection)])
    {
        for (offset, scale) in [(0., 0.7), (0.4, 1.), (0.4, 0.5)] {
            let mut e = editor(tool);
            e.action_context.center = center;
            e.dispatch(Command::Cancel).unwrap();
            for v in &mut e.scene.active_mut().unwrap().mesh.vertices {
                v.z = 3.;
            }
            e.dispatch(Command::Activate(tool)).unwrap();
            e.dispatch(Command::ConfigureSmoothShift(SmoothShiftParameters {
                offset,
                scale,
                ..e.session.live.as_ref().unwrap().params.smooth_shift
            }))
            .unwrap();
            e.dispatch(Command::Apply).unwrap();
            let m = &e.scene.active().unwrap().mesh;
            let cap = &m.faces[cap_face(m)];
            for &v in &cap.vertices {
                let q = m.vertices[v as usize];
                assert!((q.z - (3. + offset)).abs() < 1e-9, "{tool:?} {q}");
                assert!((q.x.abs() - scale).abs() < 1e-9, "{tool:?} {q}");
            }
        }
    }
}

/// Two unit quads facing +Z, centred at x = -3 and x = +3, both selected.
fn two_quads(tool: Tool, center: CenterMode, symmetry: Symmetry) -> Editor {
    let face = |v: [u32; 4]| Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Original".into(),
        smooth: false,
        part: None,
        material_label: None,
    };
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut vertices = Vec::new();
    for cx in [-3., 3.] {
        for (x, y) in [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)] {
            vertices.push(DVec3::new(cx + x, y, 0.));
        }
    }
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices,
        faces: vec![face([0, 1, 2, 3]), face([4, 5, 6, 7])].into(),
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0, 1];
    e.symmetry = symmetry;
    e.action_context.center = center;
    e.history.undo.clear();
    e.dispatch(Command::Activate(tool)).unwrap();
    let p = SmoothShiftParameters {
        offset: 0.,
        scale: 0.5,
        ..e.session.live.as_ref().unwrap().params.smooth_shift
    };
    e.dispatch(Command::ConfigureSmoothShift(p)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e
}

/// The selected caps' corner x values, sorted.
fn cap_xs(e: &Editor) -> Vec<f64> {
    let m = &e.scene.active().unwrap().mesh;
    let mut xs: Vec<f64> = e
        .scene
        .selection
        .faces
        .iter()
        .flat_map(|&f| m.faces[f as usize].vertices.clone())
        .map(|v| (m.vertices[v as usize].x * 1e9).round() / 1e9)
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    xs
}

#[test]
fn sst_scale_follows_the_action_center() {
    // Q-SS-04 (Jason, 2026-11-18): Scale pivots on the selected action
    // centre (an intentional difference from the reference).
    for tool in [Tool::SmoothShift, Tool::Thicken] {
        let off = Symmetry::OFF;
        // None and Selection: the two caps' shared box middle, x = 0.
        for center in [CenterMode::None, CenterMode::Selection] {
            let e = two_quads(tool, center, off);
            assert_eq!(cap_xs(&e), [-2., -1., 1., 2.], "{tool:?} {center:?}");
        }
        // Local: each cap about its own middle.
        let e = two_quads(tool, CenterMode::Local, off);
        assert_eq!(cap_xs(&e), [-3.5, -2.5, 2.5, 3.5], "{tool:?}");
        // Element: both caps toward the picked point.
        let at = PickedPoint::new(DVec3::new(3., 0., 0.));
        let e = two_quads(tool, CenterMode::Element(at), off);
        assert_eq!(cap_xs(&e), [-0.5, 0.5, 2.5, 3.5], "{tool:?}");
    }
}

#[test]
fn sst_origin_action_center_scales_about_the_origin() {
    for tool in [Tool::SmoothShift, Tool::Thicken] {
        let mut e = editor(tool);
        e.dispatch(Command::Cancel).unwrap();
        for v in &mut e.scene.active_mut().unwrap().mesh.vertices {
            *v += DVec3::new(5., 0., 3.);
        }
        e.action_context.center = CenterMode::Origin;
        e.dispatch(Command::Activate(tool)).unwrap();
        e.dispatch(Command::ConfigureSmoothShift(SmoothShiftParameters {
            offset: 0.4,
            scale: 0.5,
            ..e.session.live.as_ref().unwrap().params.smooth_shift
        }))
        .unwrap();
        e.dispatch(Command::Apply).unwrap();
        let m = &e.scene.active().unwrap().mesh;
        let cap = &m.faces[cap_face(m)];
        for &v in &cap.vertices {
            let q = m.vertices[v as usize];
            assert!((q.z - 0.5 * 3.4).abs() < 1e-9, "{tool:?} {q}");
            assert!(
                (q.x - 2.).abs() < 1e-9 || (q.x - 3.).abs() < 1e-9,
                "{tool:?} {q}"
            );
        }
    }
}

#[test]
fn sst_action_center_scale_keeps_symmetry() {
    // With Symmetry X, a point centre on one side is mirrored for the
    // other side's points, so the result stays symmetric.
    let at = PickedPoint::new(DVec3::new(3., 0., 0.));
    for tool in [Tool::SmoothShift, Tool::Thicken] {
        for center in [
            CenterMode::None,
            CenterMode::Selection,
            CenterMode::Origin,
            CenterMode::Local,
            CenterMode::Element(at),
        ] {
            let e = two_quads(tool, center, Symmetry::x());
            let xs = cap_xs(&e);
            let mirrored: Vec<f64> = xs.iter().rev().map(|x| -x).collect();
            assert_eq!(xs, mirrored, "{tool:?} {center:?}");
            let m = &e.scene.active().unwrap().mesh;
            for v in &m.vertices {
                let partner = DVec3::new(-v.x, v.y, v.z);
                assert!(
                    m.vertices.iter().any(|w| w.distance(partner) < 1e-9),
                    "{tool:?} {center:?} {v}"
                );
            }
        }
        let e = two_quads(tool, CenterMode::Element(at), Symmetry::x());
        assert_eq!(cap_xs(&e), [-3.5, -2.5, 2.5, 3.5], "{tool:?}");
    }
}
