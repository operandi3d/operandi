//! Original VAC-F01..03 fixtures from the reviewed coordinate prescriptions.
//! Locally generated test data, no reference scene/export or distribution grant.
use glam::{DVec2, DVec3};
use operandi_core::*;
fn fixture() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut mesh = Mesh::box_mesh(DVec3::new(3., 1., 0.), DVec3::splat(2.), 0).unwrap();
    for (i, f) in mesh.faces.iter_mut().enumerate() {
        f.uv = Some(vec![
            DVec2::new(i as f64, 0.),
            DVec2::new(i as f64 + 1., 0.),
            DVec2::new(i as f64 + 1., 1.),
            DVec2::new(i as f64, 1.),
        ]);
        f.smooth = i == 5;
        f.material = if i == 5 { "cap" } else { "wall" }.into();
    }
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.faces = vec![5];
    e
}
fn close(a: DVec3, b: DVec3) {
    assert!((a - b).length() < 1e-6, "{a:?} != {b:?}");
}
#[test]
fn vac_t08_13_geometry_and_independent_axes() {
    for center in [CenterMode::None, CenterMode::Selection, CenterMode::Origin] {
        for tool in [Tool::Move, Tool::Rotate, Tool::Scale, Tool::Transform] {
            let mut e = fixture();
            e.dispatch(Command::ActionContext(ActionContext {
                center,
                axis: AxisOrientation::World,
            }))
            .unwrap();
            let before = e.scene.clone();
            e.dispatch(Command::Activate(tool)).unwrap();
            assert_eq!(e.scene, before);
            let c = if center == CenterMode::Origin {
                DVec3::ZERO
            } else {
                DVec3::new(3., 2., 0.)
            };
            close(
                e.session
                    .live
                    .as_ref()
                    .unwrap()
                    .action_frame
                    .unwrap()
                    .center,
                c,
            );
            let p = Parameters {
                delta: match tool {
                    Tool::Scale => DVec3::splat(2.),
                    Tool::Move => DVec3::X,
                    _ => DVec3::new(0., 0., 90.),
                },
                ..Default::default()
            };
            let p = if tool == Tool::Transform {
                Parameters {
                    delta: DVec3::X,
                    rotation_delta: DVec3::new(0., 0., 90.),
                    scale_delta: DVec3::splat(2.),
                    ..Default::default()
                }
            } else {
                p
            };
            e.dispatch(Command::Preview(p.clone())).unwrap();
            let selection = &before.active().unwrap().mesh.faces[5].vertices;
            for (i, &v) in before.active().unwrap().mesh.vertices.iter().enumerate() {
                let d = v - c;
                let expected = if !selection.contains(&(i as u32)) {
                    v
                } else {
                    match tool {
                        Tool::Move => v + DVec3::X,
                        Tool::Rotate => c + DVec3::new(-d.y, d.x, d.z),
                        Tool::Scale => c + d * 2.,
                        _ => c + DVec3::new(-d.y, d.x, d.z) * 2. + DVec3::X,
                    }
                };
                close(e.scene.active().unwrap().mesh.vertices[i], expected);
            }
            assert_eq!(
                e.scene.active().unwrap().mesh.faces,
                before.active().unwrap().mesh.faces
            );
            let changed = e.scene.clone();
            e.dispatch(Command::Preview(p)).unwrap();
            assert_eq!(e.scene, changed);
            let n = e.history.undo.len();
            e.dispatch(Command::Drop).unwrap();
            assert_eq!(e.history.undo.len(), n + 1);
            e.dispatch(Command::Undo).unwrap();
            assert_eq!(e.scene, before);
            e.dispatch(Command::Redo).unwrap();
            assert_eq!(e.scene, changed);
            assert_eq!(
                persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap(),
                changed
            );
        }
    }
    for center in [CenterMode::Selection, CenterMode::Origin] {
        let mut e = fixture();
        e.dispatch(Command::ActionContext(ActionContext {
            center,
            axis: AxisOrientation::World,
        }))
        .unwrap();
        e.dispatch(Command::Activate(Tool::Move)).unwrap();
        let before = e.scene.clone();
        let c = e
            .session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center;
        e.dispatch(Command::ActionContext(ActionContext {
            center,
            axis: AxisOrientation::Workplane(1),
        }))
        .unwrap();
        close(
            e.session
                .live
                .as_ref()
                .unwrap()
                .action_frame
                .unwrap()
                .center,
            c,
        );
        e.dispatch(Command::Preview(Parameters {
            delta: DVec3::X,
            ..Default::default()
        }))
        .unwrap();
        let i = before.active().unwrap().mesh.faces[5].vertices[0] as usize;
        close(
            e.scene.active().unwrap().mesh.vertices[i],
            before.active().unwrap().mesh.vertices[i] - DVec3::Y,
        );
        e.dispatch(Command::ActionContext(ActionContext::default()))
            .unwrap();
        close(
            e.scene.active().unwrap().mesh.vertices[i],
            before.active().unwrap().mesh.vertices[i] + DVec3::X,
        );
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.scene, before);
    }
}
#[test]
fn vac_t12_asymmetric_average_t16_empty_scope_and_t22_refusal() {
    let mut e = fixture();
    e.scene.active_mut().unwrap().mesh = Mesh {
        attributes: Default::default(),
        vertices: vec![
            DVec3::new(2., 0., 0.),
            DVec3::new(8., 0., 0.),
            DVec3::new(2., 3., 0.),
        ],
        faces: vec![Face {
            vertices: vec![0, 1, 2],
            uv: None,
            material: "triangle".into(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    }
    .into();
    e.scene.selection.faces = vec![0];
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    // Changed for the ACS rulebook (2026-11-19, table row None: "Like
    // Automatic"): None takes the bounding-box centre, not the vertex
    // average (4, 1, 0) VAC-T12 had.
    close(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        DVec3::new(5., 1.5, 0.),
    );
    e.dispatch(Command::Cancel).unwrap();
    e.scene.selection.faces.clear();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    // Nothing selected: the whole mesh's box centre (ACS table).
    close(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        DVec3::new(5., 1.5, 0.),
    );
    e.dispatch(Command::Cancel).unwrap();
    // A mirrored item is edited in world space since the fix pass.
    e.scene.active_mut().unwrap().transform.scale.x = -1.;
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    e.scene.selection.mode = Mode::Item;
    e.scene.selection.item = None;
    assert!(e.dispatch(Command::Activate(Tool::Move)).is_err());
    assert!(e.session.live.is_none());
}
#[test]
fn vac_t19_21_bevel_replacement_cancel_history_attributes_and_context_survival() {
    let mut e = fixture();
    let original = e.scene.clone();
    let context = ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    };
    e.dispatch(Command::ActionContext(context)).unwrap();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    // M18 captures polygon frames as well; geometry/history regression unchanged.
    close(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        DVec3::ZERO,
    );
    for inset in [0.125, 0.25, 0.125] {
        e.dispatch(Command::Preview(Parameters {
            shift: 0.25,
            inset,
            ..Default::default()
        }))
        .unwrap();
        let mesh = &e.scene.active().unwrap().mesh;
        assert_eq!((mesh.vertices.len(), mesh.faces.len()), (12, 10));
        let cap = e.scene.selection.faces[0] as usize;
        close(mesh.center(cap), DVec3::new(3., 2.25, 0.));
        assert_eq!(
            mesh.faces[cap].uv,
            original.active().unwrap().mesh.faces[5].uv
        );
    }
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, original);
    assert_eq!(e.action_context, context);
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    close(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        DVec3::ZERO,
    );
}

#[test]
fn acs_availability_matches_dispatch_without_mutating_stage_or_history() {
    let selection = ActionContext {
        center: CenterMode::Selection,
        axis: AxisOrientation::Selection,
    };
    let choices = [
        ActionContext::default(),
        selection,
        ActionContext {
            center: CenterMode::Origin,
            axis: AxisOrientation::World,
        },
        ActionContext {
            center: CenterMode::Origin,
            axis: AxisOrientation::Selection,
        },
        ActionContext {
            center: CenterMode::Selection,
            axis: AxisOrientation::Workplane(1),
        },
    ];
    for tool in [Tool::Move, Tool::Bevel, Tool::Extrude, Tool::Box] {
        let mut e = fixture();
        e.scene.selection.faces = vec![3, 5];
        let initial = e.clone();
        for choice in choices {
            e.validate_action_context(choice).unwrap();
            assert!(e == initial, "availability must be read-only");
        }
        e.dispatch(Command::Activate(tool)).unwrap();
        if tool != Tool::Box {
            e.validate_action_context(selection).unwrap();
            let geometry = e.scene.clone();
            e.dispatch(Command::ActionContext(selection)).unwrap();
            assert_eq!(e.scene, geometry);
            // A-SEL (AC-RULES v6): the cap and the side next to it make an
            // L. It is thinnest along their shared normal direction (Z)
            // and longest across the L (X); Y is what is left.
            let basis = e.session.live.as_ref().unwrap().action_frame.unwrap().basis;
            close(basis.x_axis, DVec3::new(1., -1., 0.).normalize());
            close(basis.y_axis, -DVec3::Z);
            close(basis.z_axis, DVec3::new(1., 1., 0.).normalize());
            e.dispatch(Command::Preview(Parameters {
                delta: DVec3::Y * 0.01,
                shift: 0.01,
                extent: DVec3::splat(0.01),
                ..Default::default()
            }))
            .unwrap();
        }
        let before = e.clone();
        for choice in choices {
            let current = e.clone();
            if tool == Tool::Box {
                let error = e.validate_action_context(choice).unwrap_err();
                assert!(error.contains("Drop or Cancel"));
                assert_eq!(
                    e.dispatch(Command::ActionContext(choice)).unwrap_err(),
                    error
                );
                assert!(e == current);
            } else {
                e.validate_action_context(choice).unwrap();
                assert!(e == current);
                e.dispatch(Command::ActionContext(choice)).unwrap();
                assert_eq!(
                    e.session.live.as_ref().unwrap().base,
                    before.session.live.as_ref().unwrap().base
                );
                assert_eq!(
                    e.session.live.as_ref().unwrap().params,
                    before.session.live.as_ref().unwrap().params
                );
                assert_eq!(e.history.undo.len(), before.history.undo.len());
                if tool != Tool::Move {
                    assert_eq!(e.scene, before.scene);
                }
            }
        }
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.scene, initial.scene);
        e.validate_action_context(selection).unwrap();
    }
    let mut e = fixture();
    // AC-20: opposing normals cancel, so the axes fall back to World
    // instead of refusing (which greyed the menu items out).
    e.scene.selection.faces = vec![0, 1];
    e.validate_action_context(selection).unwrap();
    e.dispatch(Command::ActionContext(selection)).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let frame = e.session.live.as_ref().unwrap().action_frame.unwrap();
    assert!((frame.basis - glam::DMat3::IDENTITY).abs_diff_eq(glam::DMat3::ZERO, 1e-12));
}

#[test]
fn acs_invalid_recomputed_transform_preserves_entire_editor() {
    let mut e = fixture();
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Selection,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::splat(3e8),
        ..Default::default()
    }))
    .unwrap();
    let before = e.clone();
    // The same scale about Origin exceeds the coordinate budget. Frame
    // resolution succeeds, but candidate geometry must fail atomically.
    let origin = ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    };
    e.validate_action_context(origin).unwrap();
    assert!(e.dispatch(Command::ActionContext(origin)).is_err());
    assert!(e == before);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, before.session.live.unwrap().base);
}
#[test]
fn act_auto_01_02_automatic_uses_the_bounding_box_centre() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut mesh = Mesh::box_mesh(DVec3::new(2., 1., 0.), DVec3::ONE, 0).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.clone().into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Automatic,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    close(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        DVec3::new(2., 1., 0.),
    );
    e.dispatch(Command::Cancel).unwrap();
    // ACT-AUTO-02: the bounds centre, not the vertex average (2.125,1,0).
    let corner = mesh
        .vertices
        .iter()
        .position(|p| *p == DVec3::new(2.5, 1.5, 0.5))
        .unwrap();
    mesh.vertices[corner].x = 3.5;
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    close(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        DVec3::new(2.5, 1., 0.),
    );
    assert!(e.scene.selection.faces.is_empty());
}

/// ACS-EM: Selection (centre and axes) works in Edge and Vertex mode, and
/// on a moved mesh. The axes are A-SEL's (AC-RULES v6): they follow the
/// shape of the selected points, and the polygons round them only say
/// which way Z points.
#[test]
fn acs_selection_in_edge_and_vertex_mode() {
    let selection = ActionContext {
        center: CenterMode::Selection,
        axis: AxisOrientation::Selection,
    };
    // The fixture's top face (5) is the +Y cap of a 2-unit box at (3,1,0).
    let mut e = fixture();
    let top = e.scene.active().unwrap().mesh.faces[5].vertices.clone();
    let mesh = e.scene.active().unwrap().mesh.clone();
    // One top edge: its polygons are the cap (+Y) and one side.
    let (a, b) = (top[0], top[1]);
    let side = (0..mesh.faces.len())
        .find(|&f| {
            f != 5 && mesh.faces[f].vertices.contains(&a) && mesh.faces[f].vertices.contains(&b)
        })
        .unwrap();
    let up = (DVec3::Y + mesh.normal(side)).normalize();
    for mode in [Mode::Edge, Mode::Vertex, Mode::Polygon] {
        let mut e = fixture();
        e.scene.selection.mode = mode;
        e.scene.selection.faces.clear();
        match mode {
            Mode::Edge => e.scene.selection.edges = vec![(a, b)],
            Mode::Vertex => e.scene.selection.vertices = top.clone(),
            _ => e.scene.selection.faces = vec![5],
        }
        e.validate_action_context(selection)
            .unwrap_or_else(|err| panic!("{mode:?}: {err}"));
        e.dispatch(Command::ActionContext(selection)).unwrap();
        e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
        let frame = e.session.live.as_ref().unwrap().action_frame.unwrap();
        match mode {
            Mode::Edge => {
                let pa = mesh.vertices[a as usize];
                let pb = mesh.vertices[b as usize];
                close(frame.center, (pa + pb) / 2.);
                // One edge: X runs along it (pointing the positive way).
                let along = (pb - pa).normalize();
                close(frame.basis.x_axis, along * along.element_sum().signum());
                // Z is on the side of its two polygons.
                assert!(frame.basis.z_axis.dot(up) > 0.5, "{:?}", frame.basis);
                close(
                    frame.basis.y_axis,
                    frame.basis.z_axis.cross(frame.basis.x_axis),
                );
            }
            _ => {
                // The top's four points, or the top polygon: a square, so
                // X and Y tie and X goes to world X; Z is its normal (the
                // sides round the points cancel sideways).
                close(frame.center, DVec3::new(3., 2., 0.));
                close(frame.basis.x_axis, DVec3::X);
                close(frame.basis.y_axis, -DVec3::Z);
                close(frame.basis.z_axis, DVec3::Y);
            }
        }
        e.dispatch(Command::Cancel).unwrap();
    }
    // Nothing selected: the whole mesh, a cube, which has no longer side,
    // so World axes; Auto Axis falls back too.
    for mode in [Mode::Edge, Mode::Vertex, Mode::Polygon] {
        let mut e = fixture();
        e.scene.selection.mode = mode;
        e.scene.selection.faces.clear();
        e.validate_action_context(selection).unwrap();
        e.dispatch(Command::ActionContext(selection)).unwrap();
        e.dispatch(Command::Activate(Tool::Move)).unwrap();
        let frame = e.session.live.as_ref().unwrap().action_frame.unwrap();
        close(frame.center, DVec3::new(3., 1., 0.));
        close(frame.basis.y_axis, DVec3::Y);
        e.dispatch(Command::Cancel).unwrap();
        e.dispatch(Command::ActionContext(ActionContext {
            center: CenterMode::SelectionAutoAxis,
            axis: AxisOrientation::World,
        }))
        .unwrap();
        e.dispatch(Command::Activate(Tool::Move)).unwrap();
        close(
            e.session
                .live
                .as_ref()
                .unwrap()
                .action_frame
                .unwrap()
                .basis
                .y_axis,
            DVec3::Y,
        );
        e.dispatch(Command::Cancel).unwrap();
    }
    // AC-22 (action-centers rulebook 2026-11-21): Auto Axis keeps World
    // axes in Edge mode too, never the edge's normals.
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.faces.clear();
    e.scene.selection.edges = vec![(a, b)];
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::SelectionAutoAxis,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    close(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .basis
            .y_axis,
        DVec3::Y,
    );
    e.dispatch(Command::Cancel).unwrap();
    // A moved mesh keeps Selection available (world-space frame).
    let mut e = fixture();
    e.scene.active_mut().unwrap().transform.translation = DVec3::new(0., 5., 0.);
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.faces.clear();
    e.scene.selection.edges = vec![(a, b)];
    e.validate_action_context(selection).unwrap();
    e.dispatch(Command::ActionContext(selection)).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let frame = e.session.live.as_ref().unwrap().action_frame.unwrap();
    let pa = mesh.vertices[a as usize];
    let pb = mesh.vertices[b as usize];
    close(frame.center, (pa + pb) / 2. + DVec3::Y * 5.);
}

/// ACS-EM: edges with no polygon (a wire) take World axes, not a refusal.
#[test]
fn acs_selection_on_a_wire_edge_uses_world_axes() {
    let mut e = fixture();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::new(2., 0., 0.)],
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.faces.clear();
    e.scene.selection.edges = vec![(0, 1)];
    let selection = ActionContext {
        center: CenterMode::Selection,
        axis: AxisOrientation::Selection,
    };
    let frame = selection.resolve(&e.scene).unwrap();
    close(frame.center, DVec3::X);
    close(frame.basis.y_axis, DVec3::Y);
}
