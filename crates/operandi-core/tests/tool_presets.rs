//! Tool presets stage 1 (specifications/tool-presets-2026-11-15, v0.2):
//! the activate / drop rules (TP-10..15) and the new rows Flare, Flip It
//! and Flip It Origin (section 6b). The migrated rows' before-and-after
//! check is tool_preset_golden.rs. Original test data.
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
fn mine() -> Falloff {
    RadialFalloff {
        center: DVec3::new(5., 0., 0.),
        size: DVec3::splat(0.25),
        shape: FalloffShape::EaseIn,
        world: false,
    }
    .into()
}
fn preset(e: &mut Editor, id: &str) {
    e.dispatch(Command::ActivatePreset(id.into())).unwrap();
}

/// TP-10/12: the user's falloff comes back on drop, and so does "none".
#[test]
fn tp10_12_soft_move_gives_back_the_users_falloff_or_none() {
    for user in [Some(mine()), None] {
        let mut e = column();
        e.dispatch(Command::SpatialFalloff(user)).unwrap();
        preset(&mut e, "xfrm.magnet");
        assert!(matches!(e.spatial_falloff, Some(Falloff::Radial(_))));
        assert_ne!(e.spatial_falloff, user);
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.spatial_falloff, user);
        assert!(e.session.preset.is_none());
    }
}

/// TP-10: slots the row doesn't name keep the user's layer. Soft Move names
/// no action centre, so the user's stays, open and after drop.
#[test]
fn tp10_unnamed_slots_keep_the_users_layer() {
    let mut e = column();
    let user = ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::Workplane(1),
    };
    e.action_context = user;
    preset(&mut e, "xfrm.magnet");
    assert_eq!(e.action_context, user);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.action_context, user);
}

/// TP-12: every slot a row changes goes back, the action centre and axis
/// as well as the falloff (Flex names all three).
#[test]
fn tp12_flex_gives_back_centre_axis_and_falloff() {
    let mut e = column();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![1, 2];
    let user = ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    };
    e.action_context = user;
    e.dispatch(Command::SpatialFalloff(Some(mine()))).unwrap();
    preset(&mut e, "Flex");
    assert_eq!(e.action_context.center, CenterMode::SelectionBorder);
    assert_eq!(e.action_context.axis, AxisOrientation::Selection);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.action_context, user);
    assert_eq!(e.spatial_falloff, Some(mine()));
}

/// TP-13: from one row straight to another, the first row's layers go back
/// before the second takes them; after the drop only the user's are left.
#[test]
fn tp13_soft_move_to_shear_leaves_nothing_behind() {
    let mut e = column();
    e.dispatch(Command::SpatialFalloff(Some(mine()))).unwrap();
    preset(&mut e, "xfrm.magnet");
    preset(&mut e, "xfrm.shear");
    assert!(matches!(e.spatial_falloff, Some(Falloff::Linear(_))));
    assert_eq!(e.live_tool_label(), Some("Shear"));
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.spatial_falloff, Some(mine()));
    // And with no falloff to begin with, none is left.
    let mut e = column();
    preset(&mut e, "Flex");
    preset(&mut e, "xfrm.twist");
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.spatial_falloff, None);
}

/// TP-14 (Jason 11-15: restore on drop): a slot the user changes while a
/// row is open applies at once, and the pre-row layer still comes back.
#[test]
fn tp14_a_change_while_open_applies_and_is_undone_on_drop() {
    let mut e = column();
    preset(&mut e, "xfrm.magnet");
    let changed = Falloff::Linear(LinearFalloff::default());
    e.dispatch(Command::SpatialFalloff(Some(changed))).unwrap();
    assert_eq!(e.spatial_falloff, Some(changed));
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.spatial_falloff, None);
}

/// TP-15: Reset under Thicken gives Thicken's values, not Smooth Shift's.
#[test]
fn tp15_reset_under_thicken_gives_thickens_values() {
    let mut e = column();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![1];
    e.dispatch(Command::Activate(Tool::Thicken)).unwrap();
    e.dispatch(Command::ConfigureSmoothShift(SmoothShiftParameters {
        offset: 0.3,
        ..live(&e).smooth_shift
    }))
    .unwrap();
    e.dispatch(Command::ResetSmoothShift).unwrap();
    let p = live(&e).smooth_shift;
    assert!(p.thicken);
    assert_eq!(p.angle, 90.1);
    assert_eq!(p.offset, 0.);
    assert_eq!(p, tool_preset::thicken_values());
}

/// An old Operandi name and the reference id start the same row.
#[test]
fn tp01_old_names_and_ids_start_the_same_row() {
    let mut a = column();
    let mut b = column();
    a.dispatch(Command::ActivatePreset("soft move".into()))
        .unwrap();
    b.dispatch(Command::ActivateDeform(DeformPreset::SoftMove))
        .unwrap();
    assert_eq!(a.session.preset, b.session.preset);
    assert_eq!(a.spatial_falloff, b.spatial_falloff);
    assert!(
        Editor::default()
            .dispatch(Command::ActivatePreset("wobble".into()))
            .is_err()
    );
}

/// Flare: Push with a Linear falloff fitted to the selection. The column's
/// top (the falloff's Start) gets the full push and its foot none; drop
/// gives the user's falloff and action centre back.
#[test]
fn flare_pushes_the_top_fully_and_the_foot_not_at_all() {
    let mut e = column();
    let user = e.action_context;
    let before = points(&e);
    preset(&mut e, "Flare");
    assert_eq!(e.session.live.as_ref().unwrap().tool, Tool::Push);
    assert_eq!(e.live_tool_label(), Some("Flare"));
    assert_eq!(e.action_context.center, CenterMode::Automatic);
    let Some(Falloff::Linear(f)) = e.spatial_falloff else {
        panic!("Flare brings a Linear falloff");
    };
    assert!((f.start.y - 2.).abs() < 1e-9 && f.end.y.abs() < 1e-9);
    e.dispatch(Command::Preview(Parameters {
        shift: 0.1,
        ..live(&e)
    }))
    .unwrap();
    let after = points(&e);
    let radial = |p: DVec3| (p.x * p.x + p.z * p.z).sqrt();
    for (a, b) in before.iter().zip(&after) {
        let grew = radial(*b) - radial(*a);
        if a.y.abs() < 1e-9 {
            assert!(grew.abs() < 1e-9, "the foot stays: {a} -> {b}");
        } else {
            assert!(grew > 0., "{a} pushes out");
        }
    }
    // Each point moves Distance × its weight along its normal: half way up
    // (weight ½ with the Linear shape) moves half as far as the top.
    let top = before.iter().position(|p| (p.y - 2.).abs() < 1e-9).unwrap();
    let mid = before.iter().position(|p| (p.y - 1.).abs() < 1e-9).unwrap();
    let moved = |v: usize| (after[v] - before[v]).length();
    assert!((moved(top) - 0.1).abs() < 1e-9);
    assert!((moved(mid) - 0.05).abs() < 1e-9, "Linear shape");
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.spatial_falloff, None);
    assert_eq!(e.action_context, user);
    assert_eq!(e.history.undo.last().unwrap().label, "Flare");
}

/// A closed cube centred at (3, 0, 0) in Polygon mode, nothing selected.
fn cube_at_three() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh =
        primitive::segmented_cube(DVec3::new(3., 0., 0.), DVec3::splat(1.), [1, 1, 1])
            .unwrap()
            .into();
    e.scene.selection.mode = Mode::Polygon;
    e.history.undo.clear();
    e
}
/// Every polygon's normal points away from `centre`.
fn faces_out(e: &Editor, centre: DVec3) -> bool {
    let mesh = &e.scene.active().unwrap().mesh;
    (0..mesh.faces.len()).all(|f| {
        let mid = mesh.faces[f]
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .sum::<DVec3>()
            / mesh.faces[f].vertices.len() as f64;
        mesh.normal(f).dot(mid - centre) > 0.
    })
}

/// Flip It Origin on polygons right of X = 0: they end up mirrored on the
/// left, still facing out, in one undo step that Undo takes back.
#[test]
fn flip_it_origin_mirrors_to_the_other_side_facing_out() {
    let mut e = cube_at_three();
    let before = points(&e);
    assert!(faces_out(&e, DVec3::new(3., 0., 0.)));
    e.dispatch(Command::FlipIt {
        preset: "FlipItOrigin".into(),
        axis: 0,
    })
    .unwrap();
    for (a, b) in before.iter().zip(points(&e)) {
        assert!((b - DVec3::new(-a.x, a.y, a.z)).length() < 1e-9);
    }
    assert!(faces_out(&e, DVec3::new(-3., 0., 0.)));
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(e.history.undo[0].label, "Flip It Origin");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points(&e), before);
    assert!(faces_out(&e, DVec3::new(3., 0., 0.)));
}

/// Flip It mirrors through the selection's own centre: the cube stays where
/// it is, turned inside out positionally but still facing out. Symmetry is
/// ignored.
#[test]
fn flip_it_mirrors_through_the_selection_centre_and_ignores_symmetry() {
    let mut e = cube_at_three();
    e.dispatch(Command::Symmetry(Symmetry {
        axis: Some(0),
        ..e.symmetry
    }))
    .unwrap();
    let before = points(&e);
    e.dispatch(Command::FlipIt {
        preset: "FlipIt".into(),
        axis: 1,
    })
    .unwrap();
    for (a, b) in before.iter().zip(points(&e)) {
        assert!((b - DVec3::new(a.x, -a.y, a.z)).length() < 1e-9);
    }
    assert!(faces_out(&e, DVec3::new(3., 0., 0.)));
    assert_eq!(e.history.undo.last().unwrap().label, "Flip It");
}

/// Only the selected polygons flip: with two separate cubes in one mesh,
/// flipping the first cube's polygons leaves the second alone. The result
/// survives save and reopen.
#[test]
fn flip_it_on_a_selection_saves_and_reopens() {
    let mut e = cube_at_three();
    let other =
        primitive::segmented_cube(DVec3::new(-6., 0., 5.), DVec3::splat(1.), [1, 1, 1]).unwrap();
    {
        let mesh = &mut e.scene.active_mut().unwrap().mesh;
        let offset = mesh.vertices.len() as u32;
        mesh.vertices.extend(other.vertices.iter().copied());
        for mut face in other.faces.iter().cloned() {
            face.vertices.iter_mut().for_each(|v| *v += offset);
            mesh.faces.push(face);
        }
    }
    e.scene.selection.faces = (0..6).collect();
    let before = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::FlipIt {
        preset: "FlipItOrigin".into(),
        axis: 2,
    })
    .unwrap();
    let after = e.scene.active().unwrap().mesh.clone();
    assert_ne!(after.faces[0].vertices, before.faces[0].vertices);
    assert_eq!(after.faces[6..], before.faces[6..]);
    assert_eq!(after.vertices[8..], before.vertices[8..]);
    for v in 0..8 {
        let (a, b) = (before.vertices[v], after.vertices[v]);
        assert!((b - DVec3::new(a.x, a.y, -a.z)).length() < 1e-9);
    }
    let bytes = persistence::encode(&e.scene).unwrap();
    let reopened = persistence::decode(&bytes).unwrap();
    assert_eq!(reopened.active().unwrap().mesh.vertices, points(&e));
    assert_eq!(
        reopened.active().unwrap().mesh.faces[0].vertices,
        after.faces[0].vertices
    );
}

/// Refusals change nothing: Vertex mode, a bad axis, a row that isn't Flip
/// It, an open tool.
#[test]
fn flip_it_refusals_change_nothing() {
    let mut e = cube_at_three();
    let scene = e.scene.clone();
    let mut tries = vec![
        Command::FlipIt {
            preset: "FlipIt".into(),
            axis: 3,
        },
        Command::FlipIt {
            preset: "xfrm.magnet".into(),
            axis: 0,
        },
    ];
    for cmd in tries.drain(..) {
        assert!(e.dispatch(cmd).is_err());
        assert_eq!(e.scene, scene);
    }
    e.scene.selection.mode = Mode::Vertex;
    assert!(
        e.dispatch(Command::FlipIt {
            preset: "FlipIt".into(),
            axis: 0
        })
        .is_err()
    );
    e.scene.selection.mode = Mode::Polygon;
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(
        e.dispatch(Command::FlipIt {
            preset: "FlipIt".into(),
            axis: 0
        })
        .is_err()
    );
    assert!(e.history.undo.is_empty());
}
