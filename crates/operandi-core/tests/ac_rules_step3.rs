//! Action Center rule sheet v6, step 3: the pivot service is the one
//! answer. For Move, Rotate, Scale, Stretch (Scale with unequal factors)
//! and Transform, on five fixtures, the service's groups, the handle frame
//! and the points' final positions are each compared with a result worked
//! out here by hand from the rule sheet; the preview equals the dropped
//! result; undo and redo round-trip; a refused live switch restores the
//! whole live state, the placed-centre marker included.
use glam::{DMat3, DQuat, DVec3, EulerRot, dvec3};
use operandi_core::pivot_service::{GroupId, Role};
use operandi_core::*;
use std::sync::Arc;

fn quad(v: [u32; 4]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// Two separate quads. A (polygon 0, points 0..4) lies flat on the ground,
/// 2 along X by 1 along Z, facing +Y. B (polygon 1, points 4..8) is 1 wide
/// along X and runs 1.5 up a slope along (0, 0.6, 0.8), facing
/// (0, 0.8, -0.6).
fn two_clusters() -> Mesh {
    Mesh {
        vertices: vec![
            dvec3(0., 0., 0.),
            dvec3(0., 0., 1.),
            dvec3(2., 0., 1.),
            dvec3(2., 0., 0.),
            dvec3(5., 0., 0.),
            dvec3(5., 0.9, 1.2),
            dvec3(6., 0.9, 1.2),
            dvec3(6., 0., 0.),
        ],
        faces: vec![quad([0, 1, 2, 3]), quad([4, 5, 6, 7])].into(),
        ..Default::default()
    }
}
const SLOPE: DVec3 = dvec3(0., 0.6, 0.8);
const B_NORMAL: DVec3 = dvec3(0., 0.8, -0.6);
const A_CENTRE: DVec3 = dvec3(1., 0., 0.5);
const B_CENTRE: DVec3 = dvec3(5.5, 0.45, 0.6);

/// Local's own frame for cluster A (A-LOCAL): flat on the ground, so Z is
/// up (its normal), X the item's X (its longer side) and Y = Z × X.
fn a_frame() -> ActionFrame {
    ActionFrame {
        center: A_CENTRE,
        basis: DMat3::from_cols(DVec3::X, -DVec3::Z, DVec3::Y),
    }
}
/// Local's own frame for cluster B: Z on its normal, X the in-plane
/// direction nearest item X (its width), Y = Z × X (down its slope).
fn b_frame() -> ActionFrame {
    ActionFrame {
        center: B_CENTRE,
        basis: DMat3::from_cols(DVec3::X, -SLOPE, B_NORMAL),
    }
}

/// A tilted quad on the + side of the X plane (points 0..4) and its mirror
/// image (points 4..8): 1.118 along (1, 0.5, 0) by 1 along Z.
fn mirrored_pair() -> Mesh {
    Mesh {
        vertices: vec![
            dvec3(1., 0., 0.),
            dvec3(2., 0.5, 0.),
            dvec3(2., 0.5, 1.),
            dvec3(1., 0., 1.),
            dvec3(-1., 0., 0.),
            dvec3(-2., 0.5, 0.),
            dvec3(-2., 0.5, 1.),
            dvec3(-1., 0., 1.),
        ],
        faces: vec![quad([0, 3, 2, 1]), quad([4, 5, 6, 7])].into(),
        ..Default::default()
    }
}

fn editor(mesh: Mesh, faces: Vec<u32>, symmetry: Symmetry, context: ActionContext) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = faces;
    e.dispatch(Command::Symmetry(symmetry)).unwrap();
    e.dispatch(Command::ActionContext(context)).unwrap();
    e
}

/// A tool with the values typed into it.
#[derive(Clone, Copy, Debug)]
struct Use {
    name: &'static str,
    tool: Tool,
    moved: DVec3,
    turned: DVec3,
    scaled: DVec3,
}
fn uses() -> [Use; 5] {
    let (moved, turned, stretch) = (
        dvec3(0.3, -0.2, 0.5),
        dvec3(20., -35., 50.),
        dvec3(2., 0.5, 1.25),
    );
    let blank = Use {
        name: "",
        tool: Tool::Move,
        moved: DVec3::ZERO,
        turned: DVec3::ZERO,
        scaled: DVec3::ONE,
    };
    [
        Use {
            name: "Move",
            moved,
            ..blank
        },
        Use {
            name: "Rotate",
            tool: Tool::Rotate,
            turned,
            ..blank
        },
        Use {
            name: "Scale",
            tool: Tool::Scale,
            scaled: DVec3::splat(1.5),
            ..blank
        },
        Use {
            name: "Stretch",
            tool: Tool::Scale,
            scaled: stretch,
            ..blank
        },
        Use {
            name: "Transform",
            tool: Tool::Transform,
            moved,
            turned,
            scaled: stretch,
        },
    ]
}
impl Use {
    fn params(&self, e: &Editor) -> Parameters {
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        match self.tool {
            Tool::Move => p.delta = self.moved,
            Tool::Rotate => p.delta = self.turned,
            Tool::Scale => p.delta = self.scaled,
            _ => {
                p.delta = self.moved;
                p.rotation_delta = self.turned;
                p.scale_delta = self.scaled;
            }
        }
        p
    }
    /// The rule sheet's transform of world point `v` about `frame`: scale
    /// along the frame's axes, turn about them (X, then Y, then Z), move
    /// along them.
    fn apply(&self, frame: ActionFrame, v: DVec3) -> DVec3 {
        let turn = DQuat::from_euler(
            EulerRot::XYZ,
            self.turned.x.to_radians(),
            self.turned.y.to_radians(),
            self.turned.z.to_radians(),
        );
        let local = frame.basis.transpose() * (v - frame.center);
        frame.center + frame.basis * (turn * (local * self.scaled) + self.moved)
    }
}

/// What a fixture should resolve to: the handle and each group's identity,
/// frame and points (those taking the values as entered, and those on the
/// other side of the Symmetry plane).
struct Want {
    handle: ActionFrame,
    groups: Vec<(GroupId, ActionFrame, Vec<u32>, Vec<u32>)>,
}

fn close(a: DVec3, b: DVec3, what: &str) {
    assert!(a.distance(b) < 1e-9, "{what}: {a} != {b}");
}
fn same_frame(got: ActionFrame, want: ActionFrame, what: &str) {
    close(got.center, want.center, &format!("{what}: centre"));
    for axis in 0..3 {
        close(
            got.basis.col(axis),
            want.basis.col(axis),
            &format!("{what}: axis {axis}"),
        );
    }
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.to_vec()
}
fn mirror(p: DVec3) -> DVec3 {
    dvec3(-p.x, p.y, p.z)
}

/// The whole comparison for one fixture: for each of the five uses, the
/// service's answer, the editor's kept copy of it, the handle frame, the
/// previewed positions, the dropped result, and undo / redo.
fn check(make: impl Fn() -> Editor, want: &Want) {
    for u in uses() {
        let what = u.name;
        let mut e = make();
        e.dispatch(Command::Activate(u.tool)).unwrap();
        let before = points(&e);

        // The service's answer, asked directly.
        let live = e.session.live.as_ref().unwrap();
        let resolved = e.pivot_request(&live.base, u.tool).resolve().unwrap();
        same_frame(resolved.handle, want.handle, &format!("{what}: handle"));
        assert_eq!(resolved.items.len(), 1, "{what}");
        let groups = &resolved.items[0].groups;
        assert_eq!(groups.len(), want.groups.len(), "{what}: groups");
        for (group, (id, frame, entered, other)) in groups.iter().zip(&want.groups) {
            assert_eq!(group.id, *id, "{what}");
            same_frame(group.frame, *frame, &format!("{what}: {id:?}"));
            let with = |role: Role| -> Vec<u32> {
                group
                    .targets
                    .iter()
                    .filter(|t| t.role == role)
                    .map(|t| t.point)
                    .collect()
            };
            assert_eq!(with(Role::Primary), *entered, "{what}: {id:?} as entered");
            assert_eq!(with(Role::Other), *other, "{what}: {id:?} mirrored");
        }

        // The live tool holds that same answer, resolved once: the handle
        // reads it, and so does every preview.
        let held = e.live_pivots().unwrap().unwrap();
        assert_eq!(*held, resolved, "{what}: the live tool's pivots");
        same_frame(e.handle_frame().unwrap(), want.handle, what);
        e.dispatch(Command::Preview(u.params(&e))).unwrap();
        e.dispatch(Command::Preview(u.params(&e))).unwrap();
        let again = e.live_pivots().unwrap().unwrap();
        assert!(Arc::ptr_eq(&held, &again), "{what}: resolved once per base");

        // Final positions, group by group.
        let after = points(&e);
        let mut expected = before.clone();
        for (_, frame, entered, other) in &want.groups {
            for &v in entered {
                expected[v as usize] = u.apply(*frame, before[v as usize]);
            }
            for &v in other {
                expected[v as usize] = mirror(u.apply(*frame, mirror(before[v as usize])));
            }
        }
        for (v, (got, want)) in after.iter().zip(&expected).enumerate() {
            close(*got, *want, &format!("{what}: point {v}"));
        }
        assert_ne!(after, before, "{what} did something");

        // The dropped result is the preview; undo and redo round-trip.
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(points(&e), after, "{what}: drop keeps the preview");
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(points(&e), before, "{what}: undo");
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(points(&e), after, "{what}: redo");
    }
}

/// Local on two tilted clusters: each cluster about its own centre with
/// its own axes; the handle on the cluster picked last.
#[test]
fn local_two_tilted_clusters() {
    let groups = || {
        vec![
            (GroupId::Cluster(0), a_frame(), vec![0, 1, 2, 3], vec![]),
            (GroupId::Cluster(1), b_frame(), vec![4, 5, 6, 7], vec![]),
        ]
    };
    for (order, handle) in [(vec![0, 1], b_frame()), (vec![1, 0], a_frame())] {
        check(
            || {
                editor(
                    two_clusters(),
                    order.clone(),
                    Symmetry::OFF,
                    ActionContext::LOCAL,
                )
            },
            &Want {
                handle,
                groups: groups(),
            },
        );
    }
}

/// The mixing rule, Local's centre with one frame (here Origin's World
/// axes): each cluster keeps its own centre, and all take that one frame.
#[test]
fn local_centre_with_origin_axes() {
    let world = |center| ActionFrame {
        center,
        basis: DMat3::IDENTITY,
    };
    check(
        || {
            editor(
                two_clusters(),
                vec![0, 1],
                Symmetry::OFF,
                ActionContext {
                    center: CenterMode::Local,
                    axis: AxisOrientation::World,
                },
            )
        },
        &Want {
            handle: world(B_CENTRE),
            groups: vec![
                (
                    GroupId::Cluster(0),
                    world(A_CENTRE),
                    vec![0, 1, 2, 3],
                    vec![],
                ),
                (
                    GroupId::Cluster(1),
                    world(B_CENTRE),
                    vec![4, 5, 6, 7],
                    vec![],
                ),
            ],
        },
    );
}

/// The mixing rule, one centre (here the Origin) with Local's axes: the
/// frame of the cluster holding the element picked last, for everything.
#[test]
fn origin_centre_with_local_axes() {
    for (order, basis) in [(vec![0, 1], b_frame().basis), (vec![1, 0], a_frame().basis)] {
        let frame = ActionFrame {
            center: DVec3::ZERO,
            basis,
        };
        check(
            || {
                editor(
                    two_clusters(),
                    order.clone(),
                    Symmetry::OFF,
                    ActionContext {
                        center: CenterMode::Origin,
                        axis: AxisOrientation::Local,
                    },
                )
            },
            &Want {
                handle: frame,
                groups: vec![(GroupId::Whole, frame, (0..8).collect(), vec![])],
            },
        );
    }
}

/// Jason, 2026-11-28: Local axes with a single centre keep the frame of the
/// cluster holding the last-selected element. A selection with no pick
/// order (Select All, or a list a script set in one go) still has a last
/// entry, so the choice is fixed and repeatable: the cluster of the last
/// polygon in the list, here the highest-numbered one, whichever cluster
/// was clicked before.
#[test]
fn local_axes_without_a_pick_order_are_repeatable() {
    let origin_local = ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::Local,
    };
    let want = ActionFrame {
        center: DVec3::ZERO,
        basis: b_frame().basis,
    };
    for clicked_first in [vec![0], vec![1], vec![]] {
        for round in 0..2 {
            let mut e = editor(
                two_clusters(),
                clicked_first.clone(),
                Symmetry::OFF,
                origin_local,
            );
            e.dispatch(Command::Selection(SelectionOperation::All))
                .unwrap();
            assert_eq!(e.scene.selection.faces, [0, 1]);
            e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
            let pivots = e.live_pivots().unwrap().unwrap();
            same_frame(
                pivots.handle,
                want,
                &format!("Select All after {clicked_first:?}, round {round}"),
            );
            same_frame(e.handle_frame().unwrap(), want, "the view's handle");
        }
    }
    // A script's list counts in the order it gave.
    let mut e = editor(two_clusters(), vec![1, 0], Symmetry::OFF, origin_local);
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    let pivots = e.live_pivots().unwrap().unwrap();
    close(
        pivots.handle.basis.z_axis,
        DVec3::Y,
        "polygon 0 listed last",
    );
}

fn selection_and_its_axes() -> ActionContext {
    ActionContext {
        center: CenterMode::Selection,
        axis: AxisOrientation::Selection,
    }
}

/// Selection with Symmetry: one group about the + side's centre with the +
/// side's axes (X along the quad's longer side, Z its normal); the + points
/// take the values as entered and the − points the mirror image.
#[test]
fn selection_with_symmetry() {
    let long = dvec3(1., 0.5, 0.).normalize();
    let normal = dvec3(-0.5, 1., 0.).normalize();
    let frame = ActionFrame {
        center: dvec3(1.5, 0.25, 0.5),
        basis: DMat3::from_cols(long, -DVec3::Z, normal),
    };
    let make = || {
        editor(
            mirrored_pair(),
            vec![0, 1],
            Symmetry::x(),
            selection_and_its_axes(),
        )
    };
    check(
        make,
        &Want {
            handle: frame,
            groups: vec![(GroupId::Whole, frame, vec![0, 1, 2, 3], vec![4, 5, 6, 7])],
        },
    );
    let e = make();
    let resolved = e.pivot_request(&e.scene, Tool::Move).resolve().unwrap();
    assert_eq!(resolved.primary, Some(Side::Positive));
    // Every target carries the side it starts on.
    for target in &resolved.items[0].groups[0].targets {
        let side = if target.point < 4 {
            Side::Positive
        } else {
            Side::Negative
        };
        assert_eq!(target.side, Some(side), "point {}", target.point);
    }
}

/// An Element pick (cluster B's polygon): one group about the polygon's
/// middle, Z along its normal and X world X laid square to it.
#[test]
fn element_pick() {
    let frame = ActionFrame {
        center: B_CENTRE,
        basis: DMat3::from_cols(DVec3::X, B_NORMAL.cross(DVec3::X), B_NORMAL),
    };
    check(
        || {
            let mut e = editor(
                two_clusters(),
                vec![0, 1],
                Symmetry::OFF,
                ActionContext::ELEMENT_UNPICKED,
            );
            let item = e.scene.active().unwrap().id;
            let pick = element_frame(&e.scene, item, TweakElement::Polygon(1)).unwrap();
            let context = e.action_context.picked(pick);
            e.dispatch(Command::ActionContext(context)).unwrap();
            e
        },
        &Want {
            handle: frame,
            groups: vec![(GroupId::Whole, frame, (0..8).collect(), vec![])],
        },
    );
}

/// The answer is resolved again when what it reads changes: a placed
/// centre, Symmetry, the Action Center. Each time the handle, the groups
/// and the next preview agree, and a live switch applies the same values
/// again from the original base.
#[test]
fn the_answer_follows_placement_symmetry_and_context() {
    let mut e = editor(
        mirrored_pair(),
        vec![0, 1],
        Symmetry::OFF,
        ActionContext {
            center: CenterMode::Origin,
            axis: AxisOrientation::World,
        },
    );
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    let first = e.live_pivots().unwrap().unwrap();
    assert_eq!(first.handle.center, DVec3::ZERO);
    assert_eq!(first.primary, None);

    // A placed centre.
    let placed = dvec3(1.5, 0.25, 0.5);
    e.dispatch(Command::SetActionCenter(placed)).unwrap();
    let second = e.live_pivots().unwrap().unwrap();
    assert!(!Arc::ptr_eq(&first, &second));
    assert_eq!(second.handle.center, placed);
    assert_eq!(second.items[0].groups[0].frame.center, placed);

    // Symmetry on: the placed centre's side drives, the other mirrors.
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    let third = e.live_pivots().unwrap().unwrap();
    assert_eq!(third.primary, Some(Side::Positive));
    let other: Vec<u32> = third.items[0].groups[0]
        .targets
        .iter()
        .filter(|t| t.role == Role::Other)
        .map(|t| t.point)
        .collect();
    assert_eq!(other, [4, 5, 6, 7]);

    // The Action Center, while the tool holds a turn: the same values are
    // applied again about the new frames, from the original base.
    let before = points(&e);
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = dvec3(0., 0., 90.);
    e.dispatch(Command::Preview(p.clone())).unwrap();
    let undo_steps = e.history.undo.len();
    e.dispatch(Command::ActionContext(ActionContext::LOCAL))
        .unwrap();
    let fourth = e.live_pivots().unwrap().unwrap();
    assert_eq!(fourth.items[0].groups.len(), 2, "one group per cluster");
    assert_eq!(e.handle_frame().unwrap(), fourth.handle);
    let live = e.session.live.as_ref().unwrap();
    assert_eq!(live.base.active().unwrap().mesh.vertices.to_vec(), before);
    assert_eq!(live.params, p, "the values are kept");
    assert_eq!(e.history.undo.len(), undo_steps, "no extra undo step");
    // Each quad turned about its own middle, so its middle stays put.
    let after = points(&e);
    let middle =
        |p: &[DVec3], range: std::ops::Range<usize>| p[range].iter().copied().sum::<DVec3>() / 4.;
    close(
        middle(&after, 0..4),
        middle(&before, 0..4),
        "+ quad's middle",
    );
    close(
        middle(&after, 4..8),
        middle(&before, 4..8),
        "− quad's middle",
    );
    assert_ne!(after, before);
}

/// A live switch that can't be applied (the same scale about a far point
/// leaves the coordinate budget) restores the whole live state: the
/// Action Center, the frame, the scene and the placed-centre marker.
#[test]
fn a_refused_live_switch_restores_the_placed_centre() {
    let mut e = editor(
        two_clusters(),
        vec![0],
        Symmetry::OFF,
        ActionContext {
            center: CenterMode::Automatic,
            axis: AxisOrientation::World,
        },
    );
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    let placed = dvec3(1.2, 0., 0.4);
    e.dispatch(Command::SetActionCenter(placed)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::splat(3e8);
    e.dispatch(Command::Preview(p)).unwrap();
    let before = e.clone();
    assert_eq!(
        before.session.live.as_ref().unwrap().placed_centre,
        Some(placed)
    );

    // About a point 40 m away instead: frame resolution succeeds, the
    // candidate geometry doesn't.
    let far = ActionContext {
        center: CenterMode::Element(PickedPoint::new(dvec3(40., 0., 0.))),
        axis: AxisOrientation::World,
    };
    e.validate_action_context(far).unwrap();
    assert!(e.dispatch(Command::ActionContext(far)).is_err());
    assert!(e == before, "the refused switch changed the editor");
    let live = e.session.live.as_ref().unwrap();
    assert_eq!(live.placed_centre, Some(placed));
    assert_eq!(live.action_frame.unwrap().center, placed);
    assert_eq!(e.handle_frame().unwrap().center, placed);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, before.session.live.unwrap().base);
}
