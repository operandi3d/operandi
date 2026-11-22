//! LOOP-NGON-01 (provisional, to be confirmed): an edge loop on the rim of
//! an n-gon cap goes around the cap instead of stopping at the rim's
//! three-edge points. Plain quad loops, poles and open borders behave as
//! before.
use glam::DVec3;
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Caps {
    Ngon,
    Fan,
    Open,
}

const SIDES: u32 = 8;

/// An 8-sided cylinder, `rows` quads tall. Ring `r`'s point `i` is
/// `r * 8 + i` (ring 0 at the bottom). Fan caps add a centre point per cap
/// after the rings (bottom first).
fn cylinder(rows: u32, caps: Caps) -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    for r in 0..=rows {
        for i in 0..SIDES {
            let a = i as f64 * std::f64::consts::TAU / SIDES as f64;
            mesh.vertices.push(DVec3::new(a.cos(), r as f64, a.sin()));
        }
    }
    for r in 0..rows {
        for i in 0..SIDES {
            let j = (i + 1) % SIDES;
            mesh.faces.push(face(vec![
                r * SIDES + i,
                (r + 1) * SIDES + i,
                (r + 1) * SIDES + j,
                r * SIDES + j,
            ]));
        }
    }
    let top = rows * SIDES;
    match caps {
        Caps::Ngon => {
            mesh.faces.push(face((0..SIDES).collect()));
            mesh.faces
                .push(face((0..SIDES).rev().map(|i| top + i).collect()));
        }
        Caps::Fan => {
            let (bottom_c, top_c) = (top + SIDES, top + SIDES + 1);
            mesh.vertices.push(DVec3::ZERO);
            mesh.vertices.push(DVec3::new(0., rows as f64, 0.));
            for i in 0..SIDES {
                let j = (i + 1) % SIDES;
                mesh.faces.push(face(vec![bottom_c, i, j]));
                mesh.faces.push(face(vec![top_c, top + j, top + i]));
            }
        }
        Caps::Open => {}
    }
    e.scene.validate().unwrap();
    e.scene.selection.mode = Mode::Edge;
    let item = e.scene.selection.item.unwrap();
    (e, item)
}

fn ring(r: u32) -> Vec<(u32, u32)> {
    let mut v: Vec<(u32, u32)> = (0..SIDES)
        .map(|i| {
            let (a, b) = (r * SIDES + i, r * SIDES + (i + 1) % SIDES);
            (a.min(b), a.max(b))
        })
        .collect();
    v.sort();
    v
}

fn sorted(edges: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut v = edges.to_vec();
    v.sort();
    v
}

#[test]
fn loop_ngon_01_double_click_on_the_top_rim_selects_all_eight_edges() {
    let (mut e, item) = cylinder(1, Caps::Ngon);
    e.dispatch(Command::SelectEdgeLoop {
        item,
        a: 9,
        b: 10,
        add: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.edges[0], (9, 10), "clicked edge first");
    assert_eq!(sorted(&e.scene.selection.edges), ring(1));
}

#[test]
fn loop_ngon_01_select_loop_on_either_rim() {
    for (seed, r) in [((9, 10), 1), ((2, 3), 0)] {
        let (mut e, _) = cylinder(1, Caps::Ngon);
        e.scene.selection.edges = vec![seed];
        e.dispatch(Command::Selection(SelectionOperation::Loop))
            .unwrap();
        assert_eq!(sorted(&e.scene.selection.edges), ring(r), "rim {r}");
    }
}

#[test]
fn loop_ngon_01_several_seeds_join_both_rims() {
    let (mut e, _) = cylinder(2, Caps::Ngon);
    e.scene.selection.edges = vec![(17, 18), (0, 1)];
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    let mut want = ring(2);
    want.extend(ring(0));
    want.sort();
    assert_eq!(sorted(&e.scene.selection.edges), want);
}

#[test]
fn middle_ring_is_a_plain_quad_loop_as_before() {
    let (mut e, _) = cylinder(2, Caps::Ngon);
    e.scene.selection.edges = vec![(8, 9)];
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    assert_eq!(sorted(&e.scene.selection.edges), ring(1));
}

#[test]
fn a_side_edge_loops_up_the_side_and_stops_at_both_caps() {
    // The vertical loop runs rim to rim; at the three-edge rim points the
    // edge has quads on both sides, so LOOP-NGON-01 doesn't apply.
    let (mut e, _) = cylinder(2, Caps::Ngon);
    e.scene.selection.edges = vec![(0, 8)];
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    assert_eq!(sorted(&e.scene.selection.edges), vec![(0, 8), (8, 16)]);
}

#[test]
fn triangle_fan_cap_rim_already_loops() {
    // The rim points of a fan cap have four edges, so the straight walk
    // goes around on its own (as before this rule).
    let (mut e, _) = cylinder(1, Caps::Fan);
    e.scene.selection.edges = vec![(9, 10)];
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    assert_eq!(sorted(&e.scene.selection.edges), ring(1));
}

#[test]
fn open_rim_border_loops_as_before() {
    let (mut e, _) = cylinder(1, Caps::Open);
    e.scene.selection.edges = vec![(9, 10)];
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    assert_eq!(sorted(&e.scene.selection.edges), ring(1));
}

#[test]
fn a_lone_ngon_border_loops_its_rim() {
    // PF-03: every rim point has exactly two border edges, so the border
    // loop turns at each and runs all round.
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    for i in 0..6 {
        let a = i as f64 * std::f64::consts::TAU / 6.;
        mesh.vertices.push(DVec3::new(a.cos(), a.sin(), 0.));
    }
    mesh.faces.push(face((0..6).collect()));
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![(0, 1)];
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    assert_eq!(e.scene.selection.edges.len(), 6);
}

#[test]
fn next_loop_reaches_the_rim_then_stops_at_the_cap() {
    // From the middle ring the next loop is a rim (it now loops around the
    // cap); one more step stops at the n-gon, as at any pole.
    let (mut e, _) = cylinder(2, Caps::Ngon);
    e.scene.selection.edges = ring(1);
    e.dispatch(Command::Selection(SelectionOperation::NextLoop))
        .unwrap();
    let rim = sorted(&e.scene.selection.edges);
    assert!(rim == ring(0) || rim == ring(2), "{rim:?}");
    let before = e.scene.clone();
    let err = e
        .dispatch(Command::Selection(SelectionOperation::NextLoop))
        .unwrap_err();
    assert!(err.contains("pole"), "{err}");
    assert_eq!(e.scene, before);
}
