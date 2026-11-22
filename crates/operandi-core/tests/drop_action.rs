//! Drop Action (spec drop-action-2026-11-06), test cases DA-T1..T9 with the
//! spec's measured numbers, plus refusals, multi-source and Place.

use glam::{DMat4, DQuat, DVec3};
use operandi_core::drop_action::Ray;
use operandi_core::editor::{Command, Editor};
use operandi_core::scene::Transform;
use operandi_core::{DropAction, HistoryAction};

fn deg(x: f64, y: f64, z: f64) -> DVec3 {
    DVec3::new(x, y, z).map(f64::to_radians)
}

fn t(pos: DVec3, rot_deg: DVec3, scale: f64) -> Transform {
    Transform {
        translation: pos,
        rotation: deg(rot_deg.x, rot_deg.y, rot_deg.z),
        scale: DVec3::splat(scale),
        zxy: false,
        order: None,
    }
}

struct Rig {
    e: Editor,
    p: u64,
    d: u64,
    dchild: u64,
    q: u64,
    s: u64,
}

fn new_item(e: &mut Editor, transform: Transform, parent: Option<u64>) -> u64 {
    e.dispatch(Command::NewMesh).unwrap();
    let id = e.scene.items.last().unwrap().id;
    e.dispatch(Command::ItemTransform {
        item: id,
        transform,
    })
    .unwrap();
    if parent.is_some() {
        e.dispatch(Command::SetParent {
            item: id,
            parent,
            keep_position: false,
        })
        .unwrap();
    }
    id
}

/// The spec's rig: P (2,0,0) Y90 ×2; D under P (1,0,0) X30; DChild under D
/// (0,1,0); Q (−3,0,0); S under Q (0,3,0) Z45 ×0.5.
fn rig() -> Rig {
    let mut e = Editor::default();
    let p = new_item(
        &mut e,
        t(DVec3::new(2., 0., 0.), DVec3::new(0., 90., 0.), 2.),
        None,
    );
    let d = new_item(&mut e, t(DVec3::X, DVec3::new(30., 0., 0.), 1.), Some(p));
    let dchild = new_item(&mut e, t(DVec3::Y, DVec3::ZERO, 1.), Some(d));
    let q = new_item(&mut e, t(DVec3::new(-3., 0., 0.), DVec3::ZERO, 1.), None);
    let s = new_item(
        &mut e,
        t(DVec3::new(0., 3., 0.), DVec3::new(0., 0., 45.), 0.5),
        Some(q),
    );
    Rig {
        e,
        p,
        d,
        dchild,
        q,
        s,
    }
}

fn drop(e: &mut Editor, action: DropAction, sources: &[u64], destination: u64) {
    e.dispatch(Command::DropAction {
        action,
        sources: sources.to_vec(),
        destination,
        ray: None,
    })
    .unwrap();
}

fn item(e: &Editor, id: u64) -> &operandi_core::scene::Item {
    e.scene.items.iter().find(|i| i.id == id).unwrap()
}

fn world(e: &Editor, id: u64) -> DMat4 {
    e.scene.world_matrix(id).unwrap()
}

fn close(a: DVec3, b: DVec3) {
    assert!((a - b).length() < 1e-3, "{a:?} != {b:?}");
}

fn close_m(a: DMat4, b: DMat4) {
    assert!(a.abs_diff_eq(b, 1e-9), "{a:?} != {b:?}");
}

fn rotation(m: DMat4) -> DQuat {
    m.to_scale_rotation_translation().1
}

fn same_rotation(a: DQuat, b: DQuat) {
    assert!(a.dot(b).abs() > 1. - 1e-9, "{a:?} != {b:?}");
}

fn children(e: &Editor, parent: u64) -> Vec<u64> {
    e.scene
        .items
        .iter()
        .filter(|i| i.parent == Some(parent))
        .map(|i| i.id)
        .collect()
}

fn undo_label(e: &Editor) -> Option<String> {
    e.history_eligibility(HistoryAction::Undo)
        .label
        .map(|l| l.to_string())
}

#[test]
fn rig_matches_the_spec() {
    let r = rig();
    close(
        r.e.scene.world_point(r.d, DVec3::ZERO),
        DVec3::new(2., 0., -2.),
    );
    close(
        r.e.scene.world_point(r.s, DVec3::ZERO),
        DVec3::new(-3., 3., 0.),
    );
}

/// DA-T1: S under D after DChild, local fields unchanged; one undo step
/// "Parent" that restores the scene.
#[test]
fn da_t1_parent() {
    let mut r = rig();
    let before = r.e.scene.clone();
    let local = item(&r.e, r.s).transform.clone();
    drop(&mut r.e, DropAction::Parent, &[r.s], r.d);
    assert_eq!(item(&r.e, r.s).parent, Some(r.d));
    assert_eq!(children(&r.e, r.d), vec![r.dchild, r.s]);
    assert_eq!(item(&r.e, r.s).transform, local);
    assert_eq!(undo_label(&r.e).as_deref(), Some("Parent"));
    r.e.dispatch(Command::Undo).unwrap();
    assert_eq!(r.e.scene, before);
}

/// DA-T2: S under D, world kept; local (−1, 0.049, −2.915), scale 0.25.
#[test]
fn da_t2_parent_in_place() {
    let mut r = rig();
    let w = world(&r.e, r.s);
    drop(&mut r.e, DropAction::ParentInPlace, &[r.s], r.d);
    let s = item(&r.e, r.s);
    assert_eq!(s.parent, Some(r.d));
    close(s.transform.translation, DVec3::new(-1., 0.049, -2.915));
    close(s.transform.scale, DVec3::splat(0.25));
    close_m(world(&r.e, r.s), w);
    assert_eq!(undo_label(&r.e).as_deref(), Some("Parent in Place"));
}

/// DA-T3: S stays under Q; local (5,0,−2), world (2,0,−2); rotation and
/// scale fields unchanged.
#[test]
fn da_t3_match_position() {
    let mut r = rig();
    let old = item(&r.e, r.s).transform.clone();
    drop(&mut r.e, DropAction::MatchPosition, &[r.s], r.d);
    let s = item(&r.e, r.s);
    assert_eq!(s.parent, Some(r.q));
    close(s.transform.translation, DVec3::new(5., 0., -2.));
    close(s.transform.rotation, old.rotation);
    close(s.transform.scale, old.scale);
    close(
        r.e.scene.world_point(r.s, DVec3::ZERO),
        DVec3::new(2., 0., -2.),
    );
    assert_eq!(undo_label(&r.e).as_deref(), Some("Match Position"));
}

/// DA-T4: S's world rotation becomes D's; position and scale unchanged,
/// also when Q is itself rotated Z 90°.
#[test]
fn da_t4_match_rotation() {
    for q_rot in [0., 90.] {
        let mut r = rig();
        r.e.dispatch(Command::ItemTransform {
            item: r.q,
            transform: t(DVec3::new(-3., 0., 0.), DVec3::new(0., 0., q_rot), 1.),
        })
        .unwrap();
        let (scale, _, pos) = world(&r.e, r.s).to_scale_rotation_translation();
        drop(&mut r.e, DropAction::MatchRotation, &[r.s], r.d);
        let (s2, rot, p2) = world(&r.e, r.s).to_scale_rotation_translation();
        same_rotation(rot, rotation(world(&r.e, r.d)));
        close(p2, pos);
        close(s2, scale);
        assert_eq!(item(&r.e, r.s).parent, Some(r.q));
    }
}

/// DA-T5: S local scale 2 (D's world scale, Q scale 1).
#[test]
fn da_t5_match_scale() {
    let mut r = rig();
    let (_, rot, pos) = world(&r.e, r.s).to_scale_rotation_translation();
    drop(&mut r.e, DropAction::MatchScale, &[r.s], r.d);
    close(item(&r.e, r.s).transform.scale, DVec3::splat(2.));
    let (_, rot2, pos2) = world(&r.e, r.s).to_scale_rotation_translation();
    same_rotation(rot, rot2);
    close(pos, pos2);
}

/// DA-28: Match = position, rotation and world scale of D, one undo step.
#[test]
fn match_copies_the_whole_world_transform() {
    let mut r = rig();
    let before = r.e.scene.clone();
    drop(&mut r.e, DropAction::Match, &[r.s], r.d);
    close_m(world(&r.e, r.s), world(&r.e, r.d));
    assert_eq!(item(&r.e, r.s).parent, Some(r.q));
    assert_eq!(undo_label(&r.e).as_deref(), Some("Match"));
    r.e.dispatch(Command::Undo).unwrap();
    assert_eq!(r.e.scene, before);
}

/// DA-T6: S under P in D's old slot; S local (1,0,0)/(30,0,0)/0.25; D under
/// S at (0,0,0)/(0,0,0)/4; D and DChild unchanged in world.
#[test]
fn da_t6_insert() {
    let mut r = rig();
    let (dw, cw) = (world(&r.e, r.d), world(&r.e, r.dchild));
    let slot = r.e.scene.items.iter().position(|i| i.id == r.d).unwrap();
    drop(&mut r.e, DropAction::Insert, &[r.s], r.d);
    let s = item(&r.e, r.s);
    assert_eq!(s.parent, Some(r.p));
    close(s.transform.translation, DVec3::X);
    close(s.transform.rotation, deg(30., 0., 0.));
    close(s.transform.scale, DVec3::splat(0.25));
    // DA-Q5 pick: the source takes the destination's old slot.
    assert_eq!(r.e.scene.items[slot].id, r.s);
    let d = item(&r.e, r.d);
    assert_eq!(d.parent, Some(r.s));
    close(d.transform.translation, DVec3::ZERO);
    close(d.transform.rotation, DVec3::ZERO);
    close(d.transform.scale, DVec3::splat(4.));
    close_m(world(&r.e, r.d), dw);
    close_m(world(&r.e, r.dchild), cw);
    assert_eq!(item(&r.e, r.dchild).parent, Some(r.d));
    assert_eq!(undo_label(&r.e).as_deref(), Some("Insert"));
}

/// DA-T7: S local (0,0,0)/(0,0,0)/0.25 → at P; D under S at
/// (4,0,0)/(30,0,0)/4, world unchanged.
#[test]
fn da_t7_insert_at_parent() {
    let mut r = rig();
    let dw = world(&r.e, r.d);
    drop(&mut r.e, DropAction::InsertAtParent, &[r.s], r.d);
    let s = item(&r.e, r.s);
    assert_eq!(s.parent, Some(r.p));
    close(s.transform.translation, DVec3::ZERO);
    close(s.transform.rotation, DVec3::ZERO);
    close(s.transform.scale, DVec3::splat(0.25));
    close(
        r.e.scene.world_point(r.s, DVec3::ZERO),
        DVec3::new(2., 0., 0.),
    );
    let d = item(&r.e, r.d);
    assert_eq!(d.parent, Some(r.s));
    close(d.transform.translation, DVec3::new(4., 0., 0.));
    close(d.transform.rotation, deg(30., 0., 0.));
    close(d.transform.scale, DVec3::splat(4.));
    close_m(world(&r.e, r.d), dw);
    assert_eq!(undo_label(&r.e).as_deref(), Some("Insert at Parent"));
}

/// DA-T8: S under P and D under S, both worlds unchanged.
#[test]
fn da_t8_insert_in_place() {
    let mut r = rig();
    let (sw, dw) = (world(&r.e, r.s), world(&r.e, r.d));
    drop(&mut r.e, DropAction::InsertInPlace, &[r.s], r.d);
    assert_eq!(item(&r.e, r.s).parent, Some(r.p));
    assert_eq!(item(&r.e, r.d).parent, Some(r.s));
    close_m(world(&r.e, r.s), sw);
    close_m(world(&r.e, r.d), dw);
}

/// DA-T9: destination at root. Insert and Insert at Parent both give S at
/// root with (4,0,0)/(0,90,0)/0.5 and T under S with local scale 4.
#[test]
fn da_t9_insert_at_root() {
    for action in [DropAction::Insert, DropAction::InsertAtParent] {
        let mut e = Editor::default();
        let target = new_item(
            &mut e,
            t(DVec3::new(4., 0., 0.), DVec3::new(0., 90., 0.), 2.),
            None,
        );
        let s = new_item(&mut e, t(DVec3::new(0., 3., 0.), DVec3::ZERO, 0.5), None);
        let tw = world(&e, target);
        drop(&mut e, action, &[s], target);
        let si = item(&e, s);
        assert_eq!(si.parent, None);
        close(si.transform.translation, DVec3::new(4., 0., 0.));
        same_rotation(rotation(si.transform.matrix()), rotation(tw));
        close(si.transform.scale, DVec3::splat(0.5));
        let ti = item(&e, target);
        assert_eq!(ti.parent, Some(s));
        close(ti.transform.scale, DVec3::splat(4.));
        close_m(world(&e, target), tw);
        // The source takes the destination's slot (DA-Q5).
        assert_eq!(e.scene.items[0].id, s);
    }
}

/// DA-25: the source's own children stay its children.
#[test]
fn insert_keeps_the_sources_children() {
    let mut r = rig();
    let kid = new_item(&mut r.e, Transform::default(), Some(r.s));
    drop(&mut r.e, DropAction::InsertInPlace, &[r.s], r.d);
    assert_eq!(item(&r.e, kid).parent, Some(r.s));
    let mut kids = children(&r.e, r.s);
    kids.sort();
    let mut want = vec![r.d, kid];
    want.sort();
    assert_eq!(kids, want);
}

/// DA-9: loops are refused and leave the scene as it was.
#[test]
fn hierarchy_loops_are_refused() {
    let mut r = rig();
    let before = r.e.scene.clone();
    for (action, source, destination) in [
        (DropAction::Parent, r.p, r.d),
        (DropAction::ParentInPlace, r.p, r.dchild),
        (DropAction::Insert, r.p, r.d),
        (DropAction::InsertInPlace, r.p, r.dchild),
        (DropAction::InsertAtParent, r.d, r.dchild),
    ] {
        let result = r.e.dispatch(Command::DropAction {
            action,
            sources: vec![source],
            destination,
            ray: None,
        });
        assert!(result.is_err(), "{action:?}");
        assert_eq!(r.e.scene, before);
    }
    // Dropping on itself and (none) do nothing either.
    for (action, source) in [(DropAction::Parent, r.d), (DropAction::None, r.s)] {
        assert!(
            r.e.dispatch(Command::DropAction {
                action,
                sources: vec![source],
                destination: r.d,
                ray: None,
            })
            .is_err()
        );
        assert_eq!(r.e.scene, before);
    }
}

/// DA-7 / DA-Q4: every source in selection order, the destination skipped,
/// one undo step. The Insert family uses only the first (pressed) source.
#[test]
fn several_sources() {
    let mut r = rig();
    let before = r.e.scene.clone();
    assert!(
        r.e.dispatch(Command::DropAction {
            action: DropAction::Parent,
            sources: vec![r.s, r.d],
            destination: r.dchild,
            ray: None,
        })
        .is_err()
    );
    assert_eq!(r.e.scene, before);
    // Two valid sources: both last children of DChild, in order.
    drop(
        &mut r.e,
        DropAction::Parent,
        &[r.s, r.q, r.dchild],
        r.dchild,
    );
    assert_eq!(children(&r.e, r.dchild), vec![r.s, r.q]);
    r.e.dispatch(Command::Undo).unwrap();
    assert_eq!(r.e.scene, before);
    // Insert family: only the pressed item moves.
    drop(&mut r.e, DropAction::InsertInPlace, &[r.s, r.q], r.d);
    assert_eq!(item(&r.e, r.s).parent, Some(r.p));
    assert_eq!(item(&r.e, r.q).parent, None);
}

/// DA-21/22/29/31: Place moves the pivot to the ray hit; Place and Align
/// also turns +Y onto the normal; a miss or a non-mesh does nothing.
#[test]
fn place_and_align() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let ground = e.scene.items[0].id;
    *e.scene.items[0].mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::new(4., 2., 4.), 0).unwrap();
    // Ground box centred at (0,1,0) in world.
    e.dispatch(Command::ItemTransform {
        item: ground,
        transform: t(DVec3::Y, DVec3::ZERO, 1.),
    })
    .unwrap();
    let source = new_item(
        &mut e,
        t(DVec3::new(5., 5., 5.), DVec3::new(0., 30., 0.), 1.),
        None,
    );
    let before_rot = rotation(world(&e, source));
    // Straight down onto the top face at (0.5, 2, 0.5).
    let down = Ray {
        origin: DVec3::new(0.5, 10., 0.5),
        direction: -DVec3::Y,
    };
    e.dispatch(Command::DropAction {
        action: DropAction::Place,
        sources: vec![source],
        destination: ground,
        ray: Some(down),
    })
    .unwrap();
    close(
        e.scene.world_point(source, DVec3::ZERO),
        DVec3::new(0.5, 2., 0.5),
    );
    same_rotation(rotation(world(&e, source)), before_rot);
    assert_eq!(item(&e, source).parent, None);
    assert_eq!(undo_label(&e).as_deref(), Some("Place"));
    // From the +X side onto the side face: +Y turns onto +X and the heading
    // change is the smallest one.
    let side = Ray {
        origin: DVec3::new(10., 1.5, 0.),
        direction: -DVec3::X,
    };
    e.dispatch(Command::DropAction {
        action: DropAction::PlaceAndAlign,
        sources: vec![source],
        destination: ground,
        ray: Some(side),
    })
    .unwrap();
    close(
        e.scene.world_point(source, DVec3::ZERO),
        DVec3::new(2., 1.5, 0.),
    );
    let rot = rotation(world(&e, source));
    close(rot * DVec3::Y, DVec3::X);
    let arc = DQuat::from_rotation_arc(before_rot * DVec3::Y, DVec3::X);
    same_rotation(rot, arc * before_rot);
    assert_eq!(undo_label(&e).as_deref(), Some("Place and Align"));
    // A miss changes nothing.
    let scene = e.scene.clone();
    let miss = Ray {
        origin: DVec3::new(50., 50., 50.),
        direction: DVec3::Y,
    };
    for ray in [Some(miss), None] {
        assert!(
            e.dispatch(Command::DropAction {
                action: DropAction::Place,
                sources: vec![source],
                destination: ground,
                ray,
            })
            .is_err()
        );
        assert_eq!(e.scene, scene);
    }
}

#[test]
fn keys_round_trip_in_menu_order() {
    let labels: Vec<&str> = DropAction::ALL.iter().map(|a| a.label()).collect();
    assert_eq!(
        labels,
        [
            "(none)",
            "Parent",
            "Parent in Place",
            "Match",
            "Match Position",
            "Match Rotation",
            "Match Scale",
            "Insert",
            "Insert at Parent",
            "Insert in Place",
            "Place",
            "Place and Align"
        ]
    );
    for a in DropAction::ALL {
        assert_eq!(DropAction::from_key(a.key()), Some(a));
    }
    assert_eq!(DropAction::default(), DropAction::None);
}
