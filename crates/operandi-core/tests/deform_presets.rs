//! Deform tab Tier 1B (specifications/deform-presets-2026-11-06): the
//! transform presets, Push and Smooth. Original test data.
use glam::DVec3;
use operandi_core::*;

/// A 0.5 × 2 × 0.5 column standing on the ground, four rows high.
fn column() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh =
        primitive::segmented_cube(DVec3::new(0., 1., 0.), DVec3::new(0.5, 2., 0.5), [1, 4, 1])
            .unwrap()
            .into();
    e.scene.selection.mode = Mode::Vertex;
    e.history.undo.clear();
    e
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
fn live(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}
/// One drag of the preset's transform: a preview, then the release.
fn drag(e: &mut Editor, delta: DVec3) {
    e.dispatch(Command::Preview(Parameters { delta, ..live(e) }))
        .unwrap();
    e.dispatch(Command::CommitTransformStep).unwrap();
}

#[test]
fn twist_turns_each_point_by_weight_times_angle() {
    let mut e = column();
    let before = points(&e);
    e.dispatch(Command::ActivateDeform(DeformPreset::Twist))
        .unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().tool, Tool::Rotate);
    assert_eq!(e.live_tool_label(), Some("Twist"));
    // Fitted: Start (full effect) at the top, End at the bottom.
    let Some(Falloff::Linear(f)) = e.spatial_falloff else {
        panic!("Twist uses a Linear falloff");
    };
    assert!((f.start - DVec3::new(0., 2., 0.)).length() < 1e-9);
    assert!((f.end - DVec3::ZERO).length() < 1e-9);
    // Nothing changes until the drag.
    assert_eq!(points(&e), before);
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(0., 90., 0.),
        ..live(&e)
    }))
    .unwrap();
    for (a, b) in before.iter().zip(points(&e)) {
        // TW-Q1: a turn, not a blend, so no point moves toward the axis.
        let ra = (a.x * a.x + a.z * a.z).sqrt();
        let rb = (b.x * b.x + b.z * b.z).sqrt();
        assert!((ra - rb).abs() < 1e-9, "{a} -> {b}");
        assert!((a.y - b.y).abs() < 1e-9);
        let turned = (a.z.atan2(a.x) - b.z.atan2(b.x))
            .to_degrees()
            .rem_euclid(360.);
        let turned = if turned > 180. { 360. - turned } else { turned };
        // Weight = height / 2 with the Linear shape.
        assert!((turned - 90. * a.y / 2.).abs() < 1e-6, "{a}: {turned}");
    }
}

#[test]
fn drop_names_the_step_after_the_preset_and_gives_back_the_users_falloff() {
    let mut e = column();
    let mine = RadialFalloff {
        center: DVec3::new(5., 0., 0.),
        size: DVec3::splat(0.25),
        shape: FalloffShape::EaseIn,
        world: false,
    };
    e.dispatch(Command::SpatialFalloff(Some(mine.into())))
        .unwrap();
    let linear_before = e.linear_falloff_preset;
    e.dispatch(Command::ActivateDeform(DeformPreset::Taper))
        .unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().tool, Tool::Scale);
    assert!(matches!(e.spatial_falloff, Some(Falloff::Linear(_))));
    drag(&mut e, DVec3::new(0.5, 1., 0.5));
    drag(&mut e, DVec3::new(0.5, 1., 0.5));
    // Two drags are tweaks of the session; Undo takes back one and keeps
    // the tool.
    let after_two = points(&e);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.live_tool_label(), Some("Taper"));
    assert_ne!(points(&e), after_two);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(points(&e), after_two);
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.live.is_none());
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Taper");
    assert_eq!(e.spatial_falloff, Some(mine.into()));
    assert_eq!(e.linear_falloff_preset, linear_before);
    assert!(e.session.preset.is_none());
    // One undo removes the whole action.
    let start = column();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points(&e), points(&start));
}

#[test]
fn taper_scales_the_top_fully_and_the_bottom_not_at_all() {
    let mut e = column();
    let before = points(&e);
    e.dispatch(Command::ActivateDeform(DeformPreset::Taper))
        .unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(0.5, 1., 0.5),
        ..live(&e)
    }))
    .unwrap();
    for (a, b) in before.iter().zip(points(&e)) {
        // Weight = height / 2: the top halves, the bottom stays.
        let factor = 1. - 0.5 * a.y / 2.;
        assert!((b.x - a.x * factor).abs() < 1e-9, "{a} -> {b}");
        assert!((b.z - a.z * factor).abs() < 1e-9, "{a} -> {b}");
    }
}

#[test]
fn shear_soft_move_swirl_and_bulge_use_their_falloffs() {
    for (preset, tool, linear, shape) in [
        (
            DeformPreset::SoftMove,
            Tool::Move,
            false,
            FalloffShape::Linear,
        ),
        (DeformPreset::Shear, Tool::Move, true, FalloffShape::Linear),
        (
            DeformPreset::Swirl,
            Tool::Rotate,
            false,
            FalloffShape::Linear,
        ),
        (
            DeformPreset::Bulge,
            Tool::Scale,
            false,
            FalloffShape::Smooth,
        ),
    ] {
        let mut e = column();
        e.dispatch(Command::ActivateDeform(preset)).unwrap();
        assert_eq!(e.session.live.as_ref().unwrap().tool, tool, "{preset:?}");
        match e.spatial_falloff.unwrap() {
            Falloff::Linear(f) => {
                assert!(linear, "{preset:?}");
                assert_eq!(f.shape, shape);
            }
            Falloff::Radial(f) => {
                assert!(!linear, "{preset:?}");
                assert_eq!(f.shape, shape);
                assert!((f.center - DVec3::new(0., 1., 0.)).length() < 1e-9);
                assert!((f.size - DVec3::new(0.25, 1., 0.25)).length() < 1e-9);
            }
            other => panic!("{preset:?}: unexpected falloff {other:?}"),
        }
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.spatial_falloff, None, "{preset:?}");
        assert!(e.history.undo.is_empty());
    }
    // Shear: the top moves fully, the bottom stays.
    let mut e = column();
    let before = points(&e);
    e.dispatch(Command::ActivateDeform(DeformPreset::Shear))
        .unwrap();
    drag(&mut e, DVec3::X);
    for (a, b) in before.iter().zip(points(&e)) {
        assert!((b.x - a.x - a.y / 2.).abs() < 1e-9, "{a} -> {b}");
    }
}

#[test]
fn presets_refuse_item_mode_and_toggle_off_from_their_own_button() {
    let mut e = column();
    e.scene.selection.mode = Mode::Item;
    let before = e.clone();
    assert!(
        e.dispatch(Command::ActivateDeform(DeformPreset::Twist))
            .is_err()
    );
    assert!(e.session.live.is_none() && e.session.preset.is_none());
    assert_eq!(e.scene, before.scene);
    let mut e = column();
    e.dispatch(Command::ActivateDeform(DeformPreset::Twist))
        .unwrap();
    // Another preset on the same transform replaces it.
    e.dispatch(Command::ActivateDeform(DeformPreset::Swirl))
        .unwrap();
    assert_eq!(e.live_tool_label(), Some("Swirl"));
    assert!(matches!(e.spatial_falloff, Some(Falloff::Radial(_))));
    // Its own button again drops it; the falloff goes with it.
    e.dispatch(Command::ActivateDeform(DeformPreset::Swirl))
        .unwrap();
    assert!(e.session.live.is_none() && e.spatial_falloff.is_none());
    // Plain Rotate after a preset has no falloff.
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    assert!(e.spatial_falloff.is_none() && e.session.preset.is_none());
    // Switching from a preset to plain Rotate drops the preset first.
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::ActivateDeform(DeformPreset::Twist))
        .unwrap();
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    assert_eq!(e.live_tool_label(), Some("Rotate"));
    assert!(e.spatial_falloff.is_none());
}

#[test]
fn undo_past_the_presets_start_closes_it_and_clears_its_falloff() {
    let mut e = column();
    e.dispatch(Command::ActivateDeform(DeformPreset::Bulge))
        .unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(e.session.live.is_none());
    assert!(e.spatial_falloff.is_none() && e.session.preset.is_none());
}

#[test]
fn push_moves_the_selection_along_its_normals_and_is_one_step() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
        .unwrap()
        .into();
    e.scene.selection.mode = Mode::Polygon;
    e.history.undo.clear();
    let before = points(&e);
    e.dispatch(Command::Activate(Tool::Push)).unwrap();
    assert_eq!(live(&e).shift, 0.);
    assert_eq!(live(&e).push_normals, PushNormals::Averaged);
    e.dispatch(Command::Preview(Parameters {
        shift: 0.5,
        ..live(&e)
    }))
    .unwrap();
    // A box corner's averaged normal points out along the diagonal.
    for (a, b) in before.iter().zip(points(&e)) {
        let moved = b - *a;
        assert!((moved.length() - 0.5).abs() < 1e-9);
        assert!(moved.normalize().dot(a.normalize()) > 1. - 1e-9);
    }
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Push");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points(&e), before);
}

#[test]
fn push_selected_normals_pushes_a_patch_out_as_a_slab() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
        .unwrap()
        .into();
    e.scene.selection.mode = Mode::Polygon;
    let top = (0..6)
        .find(|&f| e.scene.active().unwrap().mesh.normal(f).y > 0.9)
        .unwrap() as u32;
    e.scene.selection.faces = vec![top];
    let before = points(&e);
    e.dispatch(Command::Activate(Tool::Push)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        shift: 0.5,
        push_normals: PushNormals::Selected,
        ..live(&e)
    }))
    .unwrap();
    for (a, b) in before.iter().zip(points(&e)) {
        let expected = if a.y > 0. { *a + DVec3::Y * 0.5 } else { *a };
        assert!((b - expected).length() < 1e-9, "{a} -> {b}");
    }
}

#[test]
fn smooth_works_on_activation_and_its_fields_rework_one_result() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
        .unwrap()
        .into();
    e.scene.selection.mode = Mode::Vertex;
    e.history.undo.clear();
    let before = points(&e);
    e.dispatch(Command::Activate(Tool::Smooth)).unwrap();
    let p = live(&e);
    assert_eq!(p.smooth, SmoothOptions::default());
    assert_eq!((p.smooth.strength, p.smooth.iterations), (1., 1));
    assert!(p.smooth.lock_corner && !p.smooth.lock_boundary);
    assert_eq!(p.smooth.sharp_angle, 60.);
    // A closed box has no borders: every point moves toward its neighbours,
    // so the box shrinks.
    let once = points(&e);
    assert!(
        once.iter()
            .zip(&before)
            .all(|(a, b)| a.length() < b.length())
    );
    // A field edit reworks the result from the start, it doesn't stack.
    e.dispatch(Command::Preview(Parameters {
        smooth: SmoothOptions {
            strength: 0.,
            ..p.smooth
        },
        ..live(&e)
    }))
    .unwrap();
    assert_eq!(points(&e), before);
    e.dispatch(Command::Preview(Parameters {
        smooth: p.smooth,
        ..live(&e)
    }))
    .unwrap();
    assert_eq!(points(&e), once);
    // Mid-tool undo: the field edit comes back off, the tool stays.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().tool, Tool::Smooth);
    assert_eq!(points(&e), before);
    e.dispatch(Command::Redo).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Smooth");
    // Cancel puts the mesh back.
    e.dispatch(Command::Activate(Tool::Smooth)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(points(&e), once);
}

#[test]
fn push_and_smooth_refuse_item_mode() {
    let mut e = column();
    e.scene.selection.mode = Mode::Item;
    for tool in [Tool::Push, Tool::Smooth] {
        assert!(e.dispatch(Command::Activate(tool)).is_err());
        assert!(e.session.live.is_none());
    }
}

#[test]
fn push_and_smooth_follow_an_active_falloff() {
    let mut e = column();
    e.dispatch(Command::SpatialFalloff(Some(
        LinearFalloff {
            start: DVec3::new(0., 2., 0.),
            end: DVec3::ZERO,
            ..Default::default()
        }
        .into(),
    )))
    .unwrap();
    let before = points(&e);
    e.dispatch(Command::Activate(Tool::Push)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        shift: 0.1,
        ..live(&e)
    }))
    .unwrap();
    for (a, b) in before.iter().zip(points(&e)) {
        if a.y.abs() < 1e-9 {
            assert_eq!(*a, b, "weight 0 at the bottom");
        } else {
            assert!(*a != b);
        }
    }
}
