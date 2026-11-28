//! Selection falloff and Flex (tutorial-gaps-2026-11-13,
//! SPEC-SELECTION-FALLOFF-FLEX). Original test data; the weights follow the
//! spec's provisional reading (SF-Q1..Q7).
use glam::DVec3;
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// SF-01's 1 × 6 strip of quads along X: bottom points 0..=6 at x = 0..6,
/// top points 7..=13 above them; polygon i spans x = i..i+1.
fn strip() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut vertices = Vec::new();
    for y in 0..2 {
        for x in 0..=6 {
            vertices.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    let faces = (0..6u32)
        .map(|i| face(vec![i, i + 1, i + 8, i + 7]))
        .collect();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices,
        faces,
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = (1..6).collect();
    e.history.undo.clear();
    e
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
fn live(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}
fn selection(steps: u32, shape: FalloffShape) -> Falloff {
    Falloff::Selection(SelectionFalloff { steps, shape })
}
fn weights(e: &Editor, f: Falloff) -> Vec<f64> {
    let id = e.scene.active().unwrap().id;
    ActiveFalloff::new(f).weights(&e.scene, id).unwrap()
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// SF-01: border points at x = 1 weigh 0, x = 2 weighs 0.5, x ≥ 3 weigh 1;
/// x = 0 is outside the selection and weighs 0. The open end at x = 6 is
/// the mesh's border, not the selection's.
#[test]
fn sf01_weights_rise_from_the_border_over_steps_rings() {
    let e = strip();
    let w = weights(&e, selection(2, FalloffShape::Linear));
    for row in [0, 7] {
        let expected = [0., 0., 0.5, 1., 1., 1., 1.];
        for (x, want) in expected.into_iter().enumerate() {
            assert!(
                close(w[row + x], want),
                "x = {x}: {} not {want}",
                w[row + x]
            );
        }
    }
}

/// SF-02: a Move of (0, 1, 0) moves each point by its weight.
#[test]
fn sf02_move_follows_the_weights_and_cancel_restores() {
    let mut e = strip();
    let before = points(&e);
    e.dispatch(Command::SpatialFalloff(Some(selection(
        2,
        FalloffShape::Linear,
    ))))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Y,
        ..live(&e)
    }))
    .unwrap();
    let after = points(&e);
    for x in 0..=6 {
        let lift = after[x].y - before[x].y;
        let want = [0., 0., 0.5, 1., 1., 1., 1.][x];
        assert!(close(lift, want), "x = {x}: moved {lift}");
    }
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(points(&e), before);
}

/// Steps 1 is nearly hard; more Steps soften it; the shape curves apply
/// (Ease-Out from the inside: (r / Steps)²).
#[test]
fn steps_and_shape_change_the_ramp() {
    let e = strip();
    let hard = weights(&e, selection(1, FalloffShape::Linear));
    assert_eq!(&hard[0..7], &[0., 0., 1., 1., 1., 1., 1.]);
    let soft = weights(&e, selection(4, FalloffShape::Linear));
    assert_eq!(&soft[0..7], &[0., 0., 0.25, 0.5, 0.75, 1., 1.]);
    let ease = weights(&e, selection(4, FalloffShape::EaseOut));
    assert!(close(ease[2], 0.0625) && close(ease[3], 0.25) && close(ease[4], 0.5625));
    // The registry's default: Steps 2, Smooth.
    assert_eq!(
        SelectionFalloff::default(),
        SelectionFalloff {
            steps: 2,
            shape: FalloffShape::Smooth
        }
    );
}

/// Q3: a selection with no border (everything selected, or nothing, which
/// works on the whole mesh) weighs 1 everywhere.
#[test]
fn a_selection_without_a_border_weighs_one() {
    let mut e = strip();
    e.scene.selection.faces = (0..6).collect();
    assert!(
        weights(&e, selection(3, FalloffShape::Linear))
            .iter()
            .all(|&w| w == 1.)
    );
    e.scene.selection.faces.clear();
    assert!(
        weights(&e, selection(3, FalloffShape::Linear))
            .iter()
            .all(|&w| w == 1.)
    );
}

/// Q2: Vertex mode: the border is the selected points with an unselected
/// neighbour; rings run along edges between selected points.
#[test]
fn vertex_selection_counts_rings_along_edges() {
    let mut e = strip();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = (2..=6).chain(9..=13).collect();
    let w = weights(&e, selection(2, FalloffShape::Linear));
    assert_eq!(&w[0..7], &[0., 0., 0., 0.5, 1., 1., 1.]);
    // Edge mode: the points of the selected edges, same rule.
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = (2..6).map(|x| (x, x + 1)).collect();
    let w = weights(&e, selection(2, FalloffShape::Linear));
    // Every selected bottom point has an unselected neighbour above it.
    assert!(w.iter().all(|&w| w == 0.));
}

/// Steps must be a whole number from 1; the command line and stack take it.
#[test]
fn steps_are_validated_and_the_command_adds_the_falloff() {
    assert!(selection(0, FalloffShape::Linear).validate().is_err());
    assert!(selection(1001, FalloffShape::Linear).validate().is_err());
    assert!(selection(1000, FalloffShape::Linear).validate().is_ok());
    let line = command_line::parse_sculpt("falloff.selection steps:5 shape:easeOut");
    let Some(command_line::SculptLine::Falloff { falloff, .. }) = line else {
        panic!("not parsed: {line:?}");
    };
    assert_eq!(falloff, selection(5, FalloffShape::EaseOut));
    assert!(matches!(
        command_line::parse_sculpt("falloff.selection steps:0"),
        Some(command_line::SculptLine::Usage(_))
    ));
    let Some(command_line::SculptLine::Falloff { falloff, .. }) =
        command_line::parse_sculpt("falloff.selection shape:custom p0:0.25 p1:0.5")
    else {
        panic!("custom not parsed");
    };
    assert_eq!(
        falloff,
        selection(
            2,
            FalloffShape::Custom {
                ease_in: 0.25,
                ease_out: 0.5
            }
        )
    );
}

/// Undo of a dropped Move under the falloff gives back the strip; Redo the
/// edit; save/reopen keeps the result.
#[test]
fn undo_redo_and_reopen_agree_with_the_preview() {
    let mut e = strip();
    let before = points(&e);
    e.dispatch(Command::SpatialFalloff(Some(selection(
        2,
        FalloffShape::Linear,
    ))))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Y,
        ..live(&e)
    }))
    .unwrap();
    let previewed = points(&e);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(points(&e), previewed);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points(&e), before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(points(&e), previewed);
    let bytes = persistence::encode(&e.scene).unwrap();
    let reopened = persistence::decode(&bytes).unwrap();
    assert_eq!(reopened.active().unwrap().mesh.vertices, previewed);
}

/// FX-01: Flex is Transform with a Selection falloff about the Selection
/// Border. Rotating 90° bends the strip; points outside the selection, and
/// on its border (weight 0), don't move.
#[test]
fn fx01_flex_bends_the_selection_and_drops_back_to_the_users_setup() {
    let mut e = strip();
    let before = points(&e);
    let user_action = e.action_context;
    e.dispatch(Command::ActivateDeform(DeformPreset::Flex))
        .unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().tool, Tool::Transform);
    assert_eq!(e.live_tool_label(), Some("Flex"));
    assert_eq!(
        e.spatial_falloff,
        Some(Falloff::Selection(SelectionFalloff::default()))
    );
    assert_eq!(e.action_context.center, CenterMode::SelectionBorder);
    assert_eq!(e.action_context.axis, AxisOrientation::Selection);
    e.dispatch(Command::Preview(Parameters {
        rotation_delta: DVec3::new(0., 0., 90.),
        ..live(&e)
    }))
    .unwrap();
    let after = points(&e);
    for v in [0, 1, 7, 8] {
        assert!((after[v] - before[v]).length() < 1e-9, "point {v} moved");
    }
    assert!(
        after
            .iter()
            .zip(&before)
            .skip(2)
            .take(5)
            .all(|(a, b)| (*a - *b).length() > 1e-6)
    );
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.live.is_none());
    assert_eq!(e.spatial_falloff, None);
    assert_eq!(e.action_context, user_action);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points(&e), before);
}

/// Flex refuses Item mode, and the action centre can still be changed
/// while it is open.
#[test]
fn flex_refuses_item_mode_and_takes_another_axis() {
    let mut e = strip();
    e.scene.selection.mode = Mode::Item;
    assert!(
        e.dispatch(Command::ActivateDeform(DeformPreset::Flex))
            .is_err()
    );
    assert!(e.session.live.is_none());
    let mut e = strip();
    e.dispatch(Command::ActivateDeform(DeformPreset::Flex))
        .unwrap();
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::SelectionBorder,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    assert_eq!(e.live_tool_label(), Some("Flex"));
    assert_eq!(e.action_context.axis, AxisOrientation::World);
}
