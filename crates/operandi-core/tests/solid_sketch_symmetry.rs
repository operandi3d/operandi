//! Solid Sketch with Symmetry (symmetry-2026-11-13 section 6, SS-SYM-01..09).
//!
//! Acceptance criteria:
//! 1. With Symmetry X, a node off the plane gets a twin; a node on the plane
//!    is a seam node with no twin (SS-SYM-01/02).
//! 2. The cage is one closed, welded, exactly symmetric mesh: a torso on the
//!    seam with an arm each side is one shell, every point has a mirror
//!    partner within 1e-9 m, no doubled points (SS-SYM-05).
//! 3. Editing either twin edits both, mirrored: move, scale, rotate, twist,
//!    panel values (SS-SYM-01, SS-SYM-07).
//! 4. A seam node stays on the plane and only turns about the plane's axis
//!    (SS-SYM-02, SYM3-13).
//! 5. Deleting a node deletes its twin; each edit is one in-tool undo step
//!    for both (SS-SYM-06).
//! 6. Symmetry Off builds the same as before (no twins).
//! 7. Saved with the sketch: picked up with Symmetry off, twins stay linked
//!    and the status says so; new nodes then get no twin (SS-SYM-08, Q8).
//!    Symmetry turned on for a sketch without twins only twins new nodes
//!    (SS-SYM-09).
//! 8. A link crossing the plane is allowed, with the warning (SS-SYM-04).
//! 9. The plane follows a moved mesh item.
use glam::{DQuat, DVec3, dvec3};
use operandi_core::solid_sketch::{SketchOptions, SubdivMode};
use operandi_core::*;
use std::collections::BTreeMap;

fn editor(symmetry: Symmetry) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Symmetry(symmetry)).unwrap();
    e
}
fn start(e: &mut Editor) {
    e.dispatch(Command::SolidSketchStart {
        options: SketchOptions {
            subdiv: SubdivMode::None,
            ..SketchOptions::default()
        },
        resume: true,
    })
    .unwrap();
}
fn edit(e: &mut Editor, edit: SketchEdit) {
    e.dispatch(Command::SolidSketch {
        edit,
        continuing: false,
    })
    .unwrap();
}
fn add(e: &mut Editor, at: DVec3) {
    edit(
        e,
        SketchEdit::Add {
            at,
            size: None,
            up: None,
        },
    );
}
fn select(e: &mut Editor, i: usize) {
    edit(e, SketchEdit::Select(Some(i)));
}
fn session(e: &Editor) -> &SketchSession {
    e.session.solid_sketch.as_ref().unwrap()
}
fn nodes(e: &Editor) -> Vec<solid_sketch::SketchNode> {
    session(e).sketch().nodes.clone()
}
fn mesh(e: &Editor) -> Mesh {
    Mesh::clone(&e.scene.active().unwrap().mesh)
}
/// Torso (seam) at the origin, a neck/head node above it on the seam, an
/// arm of two nodes to +X: the cartoon kid's top half.
fn kid(e: &mut Editor) {
    start(e);
    add(e, dvec3(0., 0., 0.));
    add(e, dvec3(0., 0.5, 0.));
    select(e, 0);
    add(e, dvec3(0.5, 0.1, 0.));
    add(e, dvec3(1.0, 0.1, 0.));
}
fn closed(mesh: &Mesh) -> bool {
    let mut uses: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for f in &mesh.faces {
        for i in 0..f.vertices.len() {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % f.vertices.len()]);
            *uses.entry((a, b)).or_default() += 1;
        }
    }
    !uses.is_empty()
        && uses
            .iter()
            .all(|(&(a, b), &n)| n == 1 && uses.get(&(b, a)) == Some(&1))
}
fn shells(mesh: &Mesh) -> usize {
    let mut parent: Vec<usize> = (0..mesh.vertices.len()).collect();
    fn find(p: &mut [usize], mut v: usize) -> usize {
        while p[v] != v {
            p[v] = p[p[v]];
            v = p[v];
        }
        v
    }
    for f in &mesh.faces {
        for w in f.vertices.windows(2) {
            let (a, b) = (
                find(&mut parent, w[0] as usize),
                find(&mut parent, w[1] as usize),
            );
            parent[a] = b;
        }
    }
    (0..mesh.vertices.len())
        .filter(|&v| find(&mut parent, v) == v)
        .count()
}
fn welded_and_symmetric(m: &Mesh, s: Symmetry) {
    assert!(closed(m), "closed and consistently wound");
    assert_eq!(shells(m), 1, "one connected surface");
    let bad = symmetry::unpaired_points(m, s, 1e-9);
    assert!(bad.is_empty(), "unpaired {bad:?}");
    for (i, a) in m.vertices.iter().enumerate() {
        for b in &m.vertices[i + 1..] {
            assert!(a.distance(*b) > 1e-9, "doubled point {a:?}");
        }
    }
    // Symmetric polygons too, not only points: every face's mirror image
    // (its partners, in reverse order) is a face of the mesh.
    let plane = solid_sketch_symmetry::SketchMirror::of(s).unwrap();
    let partner = |v: u32| {
        let target = plane.point(m.vertices[v as usize]);
        m.vertices
            .iter()
            .position(|p| p.distance(target) <= 1e-9)
            .unwrap() as u32
    };
    let key = |vs: &[u32]| {
        let k = (0..vs.len()).min_by_key(|&i| vs[i]).unwrap();
        (0..vs.len())
            .map(|i| vs[(k + i) % vs.len()])
            .collect::<Vec<_>>()
    };
    let faces: std::collections::BTreeSet<Vec<u32>> =
        m.faces.iter().map(|f| key(&f.vertices)).collect();
    for f in &m.faces {
        let mirrored: Vec<u32> = f.vertices.iter().rev().map(|&v| partner(v)).collect();
        assert!(
            faces.contains(&key(&mirrored)),
            "face {:?} has no mirror image",
            f.vertices
                .iter()
                .map(|&v| m.vertices[v as usize])
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn twins_and_seam_nodes_build_one_welded_symmetric_cage() {
    let mut e = editor(Symmetry::x());
    kid(&mut e);
    let n = nodes(&e);
    assert_eq!(n.len(), 6, "two seam nodes, two arm nodes and their twins");
    assert!(n[0].seam && n[1].seam);
    assert_eq!(n[0].twin, None);
    // The arm's nodes pair up with mirrored twins under mirrored parents.
    let (arm, twin) = (2, n[2].twin.unwrap());
    assert_eq!(n[twin].twin, Some(arm));
    assert_eq!(n[twin].center, dvec3(-0.5, 0.1, 0.));
    assert_eq!(n[twin].parent, Some(0));
    let elbow_twin = n[4].twin.unwrap();
    assert_eq!(n[elbow_twin].parent, Some(twin));
    // The user's node stays current.
    assert_eq!(session(&e).current(), Some(4));
    welded_and_symmetric(&mesh(&e), Symmetry::x());
    assert!(session(&e).symmetry_notice().is_none());
}

#[test]
fn editing_either_twin_edits_both() {
    let mut e = editor(Symmetry::x());
    kid(&mut e);
    let twin = nodes(&e)[4].twin.unwrap();
    // Move the twin (picked on the other side): the leader follows.
    edit(
        &mut e,
        SketchEdit::Translate {
            index: twin,
            delta: dvec3(-0.2, 0.3, 0.1),
            branch: false,
        },
    );
    let n = nodes(&e);
    assert!((n[4].center - dvec3(1.2, 0.4, 0.1)).length() < 1e-12);
    welded_and_symmetric(&mesh(&e), Symmetry::x());
    // Scale, rotate and twist the leader: the twin mirrors.
    edit(
        &mut e,
        SketchEdit::Scale {
            index: 2,
            factor: 1.5,
            branch: true,
        },
    );
    edit(
        &mut e,
        SketchEdit::Rotate {
            index: 2,
            turn: DQuat::from_rotation_z(0.4) * DQuat::from_rotation_y(0.3),
        },
    );
    select(&mut e, 4);
    let c = nodes(&e)[4].center;
    edit(
        &mut e,
        SketchEdit::SetNode {
            center: c,
            size: dvec3(0.1, 0.2, 0.15),
            twist: 0.7,
        },
    );
    let n = nodes(&e);
    assert_eq!(n[twin].size, n[4].size);
    assert_eq!(n[twin].twist, -0.7);
    welded_and_symmetric(&mesh(&e), Symmetry::x());
}

#[test]
fn seam_nodes_stay_on_the_plane_and_turn_only_about_its_axis() {
    let mut e = editor(Symmetry::x());
    kid(&mut e);
    edit(
        &mut e,
        SketchEdit::Translate {
            index: 1,
            delta: dvec3(0.3, 0.2, 0.),
            branch: false,
        },
    );
    assert_eq!(nodes(&e)[1].center, dvec3(0., 0.7, 0.));
    // A turn about Z (which would tip the seam box off-symmetric) is
    // dropped; a turn about X (the plane's axis) is kept.
    edit(
        &mut e,
        SketchEdit::Rotate {
            index: 1,
            turn: DQuat::from_rotation_z(0.5),
        },
    );
    assert!(nodes(&e)[1].rotation.angle_between(DQuat::IDENTITY) < 1e-12);
    edit(
        &mut e,
        SketchEdit::Rotate {
            index: 1,
            turn: DQuat::from_rotation_x(0.5),
        },
    );
    assert!((nodes(&e)[1].rotation.angle_between(DQuat::IDENTITY) - 0.5).abs() < 1e-12);
    // Moving the torso's whole branch keeps the arms mirrored.
    edit(
        &mut e,
        SketchEdit::Translate {
            index: 0,
            delta: dvec3(0.5, 0.25, 0.),
            branch: true,
        },
    );
    welded_and_symmetric(&mesh(&e), Symmetry::x());
}

#[test]
fn deleting_a_node_deletes_its_twin_and_undo_restores_both() {
    let mut e = editor(Symmetry::x());
    kid(&mut e);
    let twin = nodes(&e)[4].twin.unwrap();
    edit(
        &mut e,
        SketchEdit::Delete {
            index: twin,
            branch: false,
        },
    );
    assert_eq!(nodes(&e).len(), 4);
    welded_and_symmetric(&mesh(&e), Symmetry::x());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(nodes(&e).len(), 6);
    // Branch delete of the arm takes both arms.
    edit(
        &mut e,
        SketchEdit::Delete {
            index: 2,
            branch: true,
        },
    );
    assert_eq!(nodes(&e).len(), 2);
    assert_eq!(session(&e).current(), Some(0));
    welded_and_symmetric(&mesh(&e), Symmetry::x());
}

#[test]
fn symmetry_off_makes_no_twins() {
    let mut e = editor(Symmetry::OFF);
    kid(&mut e);
    let n = nodes(&e);
    assert_eq!(n.len(), 4);
    assert!(n.iter().all(|n| n.twin.is_none() && !n.seam));
    assert_eq!(session(&e).sketch().mirror, None);
}

#[test]
fn picked_up_with_symmetry_off_keeps_twins_and_says_so() {
    let mut e = editor(Symmetry::x());
    kid(&mut e);
    e.dispatch(Command::Drop).unwrap();
    // Saved with the mesh: the stored sketch round-trips through JSON.
    let stored = solid_sketch_store::stored(&mesh(&e)).unwrap();
    let text = serde_json::to_string(&stored).unwrap();
    let back: solid_sketch_store::StoredSketch = serde_json::from_str(&text).unwrap();
    assert_eq!(back, stored);

    e.dispatch(Command::Symmetry(Symmetry::OFF)).unwrap();
    start(&mut e);
    assert_eq!(nodes(&e).len(), 6);
    assert!(!session(&e).mirrors_new_nodes());
    assert_eq!(
        session(&e).symmetry_notice().as_deref(),
        Some(
            "Sketch is symmetric about X; its twins stay linked. Turn Symmetry X on to mirror new nodes"
        )
    );
    // Linked: editing one still edits both.
    edit(
        &mut e,
        SketchEdit::Translate {
            index: 3,
            delta: dvec3(0., 0.2, 0.),
            branch: false,
        },
    );
    welded_and_symmetric(&mesh(&e), Symmetry::x());
    // New nodes get no twin.
    add(&mut e, dvec3(1.5, 0.1, 0.));
    assert_eq!(nodes(&e).len(), 7);
}

#[test]
fn symmetry_turned_on_mid_sketch_twins_only_new_nodes() {
    let mut e = editor(Symmetry::OFF);
    start(&mut e);
    add(&mut e, dvec3(0., 0., 0.));
    add(&mut e, dvec3(0.5, 0., 0.));
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    start(&mut e);
    assert!(session(&e).mirrors_new_nodes());
    let n = nodes(&e);
    assert!(n.iter().all(|n| n.twin.is_none()));
    add(&mut e, dvec3(1.0, 0., 0.));
    let n = nodes(&e);
    assert_eq!(n.len(), 4);
    assert!(n[2].twin.is_some());
}

/// Tool chain (Jason, 2026-11-14): Symmetry toggled with the tool still
/// open applies to the next node, without dropping the sketch.
#[test]
fn symmetry_toggled_with_the_sketch_open_applies_to_new_nodes() {
    let mut e = editor(Symmetry::OFF);
    start(&mut e);
    add(&mut e, dvec3(0., 0., 0.));
    add(&mut e, dvec3(0.5, 0., 0.));
    e.dispatch(Command::ToggleSymmetry).unwrap();
    assert!(e.session.solid_sketch.is_some(), "still open");
    assert!(session(&e).mirrors_new_nodes());
    add(&mut e, dvec3(1.0, 0., 0.));
    let n = nodes(&e);
    assert_eq!(n.len(), 4, "the new node and its twin");
    assert!(n[2].twin.is_some());
    // Off again: the twins made stay linked, new nodes get none.
    e.dispatch(Command::ToggleSymmetry).unwrap();
    assert!(!session(&e).mirrors_new_nodes());
    add(&mut e, dvec3(1.5, 0.3, 0.));
    assert_eq!(nodes(&e).len(), 5);
}

#[test]
fn the_first_node_lands_on_the_plane_wherever_it_is_clicked() {
    // Spec v3.1 section 11 (SYM-B3-10): the root goes onto the plane.
    let mut e = editor(Symmetry::x());
    start(&mut e);
    add(&mut e, dvec3(0.5, 0.2, 0.));
    let n = nodes(&e);
    assert_eq!(n.len(), 1);
    assert!(n[0].seam);
    assert_eq!(n[0].center, dvec3(0., 0.2, 0.));
    // Later nodes off the plane get twins as usual.
    add(&mut e, dvec3(0.5, 0.2, 0.));
    assert_eq!(nodes(&e).len(), 3);
    welded_and_symmetric(&mesh(&e), Symmetry::x());
}

#[test]
fn a_crossing_link_is_allowed_with_a_warning() {
    // A link drawn across the plane: the head side under a node on the
    // other side.
    let mut e = editor(Symmetry::x());
    start(&mut e);
    add(&mut e, dvec3(0., 0., 0.));
    add(&mut e, dvec3(0.5, 0., 0.));
    let left = nodes(&e)[1].twin.unwrap();
    select(&mut e, left);
    add(&mut e, dvec3(0.5, 0.5, 0.));
    assert_eq!(
        session(&e).symmetry_notice().as_deref(),
        Some("This link crosses the symmetry plane")
    );
    let m = mesh(&e);
    assert!(symmetry::unpaired_points(&m, Symmetry::x(), 1e-9).is_empty());
}

#[test]
fn the_plane_follows_a_moved_item_and_other_axes() {
    let mut e = editor(Symmetry {
        axis: Some(2),
        offset: 0.25,
        ..Symmetry::OFF
    });
    let id = e.scene.active().unwrap().id;
    e.scene.item_mut(id).unwrap().transform.translation = dvec3(3., 0., 0.);
    start(&mut e);
    // World points; the plane is z = 0.25 in the item's own space.
    add(&mut e, dvec3(3., 0., 0.25));
    add(&mut e, dvec3(3., 0., 0.75));
    let n = nodes(&e);
    assert!(n[0].seam);
    assert_eq!(n[n[1].twin.unwrap()].center, dvec3(0., 0., -0.25));
    welded_and_symmetric(
        &mesh(&e),
        Symmetry {
            axis: Some(2),
            offset: 0.25,
            ..Symmetry::OFF
        },
    );
}

/// Regression (Jason, 2026-11-14, cartoon kid tutorial): legs branched off
/// a seam hip node and running mostly downward fused into one lopsided
/// blob. Both legs wanted the hip box's bottom face; the first leg took it
/// and its twin fell back to a side face, so the cage wasn't symmetric.
/// Each leg (and its twin) now leaves through the hip's side facing it.
#[test]
fn legs_branched_off_a_seam_hip_stay_two_mirrored_tubes() {
    // Leg directions: down and a little out, straight down and forward,
    // out sideways, down-out-back.
    let legs = [
        (dvec3(0.15, -1.0, 0.), dvec3(0.2, -1.5, 0.05)),
        (dvec3(0.1, -1.0, 0.2), dvec3(0.1, -1.5, 0.3)),
        (dvec3(0.6, -0.6, 0.), dvec3(1.1, -0.7, 0.)),
        (dvec3(0.3, -0.9, -0.3), dvec3(0.4, -1.4, -0.4)),
    ];
    for ((auto_merge, auto_align), (knee, foot)) in [(true, true), (false, true), (true, false)]
        .into_iter()
        .flat_map(|o| legs.map(|l| (o, l)))
    {
        let mut e = editor(Symmetry::x());
        e.dispatch(Command::SolidSketchStart {
            options: SketchOptions {
                subdiv: SubdivMode::None,
                auto_merge,
                auto_align,
                ..SketchOptions::default()
            },
            resume: true,
        })
        .unwrap();
        // Chest and hip on the seam, then a leg down and a little out.
        add(&mut e, dvec3(0., 0., 0.));
        add(&mut e, dvec3(0., -0.5, 0.));
        add(&mut e, knee);
        add(&mut e, foot);
        let n = nodes(&e);
        assert_eq!(n.len(), 6, "two seam nodes, two leg nodes, two twins");
        assert!(n[1].seam);
        let twin = n[2].twin.unwrap();
        assert_eq!(n[twin].parent, Some(1), "both legs hang off the hip");
        let m = mesh(&e);
        welded_and_symmetric(&m, Symmetry::x());
    }
}
