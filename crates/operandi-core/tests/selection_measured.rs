//! Selection measured cases (specifications/selection-2026-11-02/ORACLES.json).
//! The fixtures are the spec's Ring8, Grid4 and Box, rebuilt here.
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;
use std::collections::BTreeSet;

fn oracle(id: &str) -> Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specifications/selection-2026-11-02/ORACLES.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["oracles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == id)
        .unwrap_or_else(|| panic!("missing oracle {id}"))
        .clone()
}
fn list(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect()
}
fn edges(v: &Value) -> BTreeSet<(u32, u32)> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let e = list(e);
            (e[0].min(e[1]), e[0].max(e[1]))
        })
        .collect()
}
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
fn editor(vertices: Vec<DVec3>, faces: Vec<Vec<u32>>) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices,
        faces: faces.into_iter().map(face).collect(),
        ..Default::default()
    }
    .into();
    e
}
/// Ring8: 8 quads around Y; point i at 45°·i, rows y = 0 and 1.
fn ring8() -> Editor {
    let mut v = Vec::new();
    for y in [0., 1.] {
        for i in 0..8 {
            let a = std::f64::consts::FRAC_PI_4 * i as f64;
            v.push(DVec3::new(a.cos(), y, a.sin()));
        }
    }
    let f = (0..8)
        .map(|i| vec![i, (i + 1) % 8, (i + 1) % 8 + 8, i + 8])
        .collect();
    editor(v, f)
}
/// Grid4: 4×4 quads in XY; point = row × 5 + column.
fn grid4() -> Editor {
    let mut v = Vec::new();
    for r in 0..5 {
        for c in 0..5 {
            v.push(DVec3::new(c as f64, r as f64, 0.));
        }
    }
    let mut f = Vec::new();
    for r in 0..4 {
        for c in 0..4 {
            let a = r * 5 + c;
            f.push(vec![a, a + 1, a + 6, a + 5]);
        }
    }
    editor(v, f)
}
fn box_fixture() -> Editor {
    let v = [
        (-1., -1., -1.),
        (1., -1., -1.),
        (1., -1., 1.),
        (-1., -1., 1.),
        (-1., 1., -1.),
        (1., 1., -1.),
        (1., 1., 1.),
        (-1., 1., 1.),
    ]
    .map(|(x, y, z)| DVec3::new(x, y, z))
    .to_vec();
    let f = vec![
        vec![0, 1, 2, 3],
        vec![7, 6, 5, 4],
        vec![0, 4, 5, 1],
        vec![1, 5, 6, 2],
        vec![2, 6, 7, 3],
        vec![3, 7, 4, 0],
    ];
    editor(v, f)
}
fn polygons(e: &mut Editor, picks: &[u32]) {
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = picks.to_vec();
}
fn points(e: &mut Editor, picks: &[u32]) {
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = picks.to_vec();
}
fn run(e: &mut Editor, op: SelectionOperation) {
    e.dispatch(Command::Selection(op)).unwrap();
}

/// SEL-BT-05 (SEL-Q-M1): a tie selects one route, chosen by pick order.
#[test]
fn sel_bt05_ring_ties_pick_one_route() {
    for (a, b) in [(0, 4), (4, 0), (1, 5), (5, 1), (2, 6), (3, 7)] {
        let o = oracle(&format!("SEL-O-ring_poly_{a}_{b}"));
        let mut e = ring8();
        polygons(&mut e, &[a, b]);
        run(&mut e, SelectionOperation::Between);
        assert_eq!(
            e.scene.selection.faces,
            list(&o["expected"]["selected_polygons"]),
            "{a},{b}"
        );
    }
    for (a, b) in [(0, 4), (4, 0), (2, 6)] {
        let o = oracle(&format!("SEL-O-ring_vert_{a}_{b}"));
        let mut e = ring8();
        points(&mut e, &[a, b]);
        run(&mut e, SelectionOperation::Between);
        assert_eq!(
            e.scene.selection.vertices,
            list(&o["expected"]["selected_vertices"]),
            "{a},{b}"
        );
    }
}

/// SEL-BT-06: opposite and side faces of a box.
#[test]
fn sel_bt06_box_faces() {
    for (id, picks) in [("SEL-O-box-bottom-top", [0, 1]), ("SEL-O-box-2-4", [2, 4])] {
        let mut e = box_fixture();
        polygons(&mut e, &picks);
        run(&mut e, SelectionOperation::Between);
        assert_eq!(
            e.scene.selection.faces,
            list(&oracle(id)["expected"]["selected_polygons"]),
            "{id}"
        );
    }
}

/// SEL-BT-01/02: one row, or the whole rectangle (picks first). The order
/// of the added rectangle polygons isn't specified as a rule, so the
/// rectangle is compared as a set.
#[test]
fn sel_bt01_02_row_and_rectangle() {
    let mut e = grid4();
    polygons(&mut e, &[0, 3]);
    run(&mut e, SelectionOperation::Between);
    assert_eq!(
        e.scene.selection.faces,
        list(&oracle("SEL-O-grid_0_3_row")["expected"]["selected_polygons"])
    );

    for (id, picks) in [
        ("SEL-O-grid_0_10", [0, 10]),
        ("SEL-O-grid_10_0_reversed_order", [10, 0]),
    ] {
        let mut e = grid4();
        polygons(&mut e, &picks);
        run(&mut e, SelectionOperation::Between);
        let want = list(&oracle(id)["expected"]["selected_polygons"]);
        assert_eq!(e.scene.selection.faces[..2], want[..2], "{id}: picks first");
        let got: BTreeSet<_> = e.scene.selection.faces.iter().copied().collect();
        assert_eq!(got, want.iter().copied().collect(), "{id}");
        assert_eq!(e.scene.selection.faces.len(), want.len());
    }

    let mut e = grid4();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = vec![(0, 1), (3, 4)];
    run(&mut e, SelectionOperation::Between);
    let got: BTreeSet<_> = e.scene.selection.edges.iter().copied().collect();
    assert_eq!(
        got,
        edges(&oracle("SEL-O-grid_edges_row")["expected"]["selected_edges"])
    );
}

/// SEL-BT-03: with more than two picks only the last pair is connected.
#[test]
fn sel_bt03_three_picks_connect_the_last_pair() {
    let mut e = grid4();
    polygons(&mut e, &[0, 3, 15]);
    run(&mut e, SelectionOperation::Between);
    assert_eq!(
        e.scene.selection.faces,
        list(&oracle("SEL-O-grid_three_picks")["expected"]["selected_polygons"])
    );
}

/// SEL-BT-04: points not in one row add nothing.
#[test]
fn sel_bt04_points_off_a_row_add_nothing() {
    let mut e = grid4();
    points(&mut e, &[0, 12]);
    let undo = e.undo.len();
    run(&mut e, SelectionOperation::Between);
    assert_eq!(
        e.scene.selection.vertices,
        list(&oracle("SEL-O-grid_verts_0_12")["expected"]["selected_vertices"])
    );
    assert_eq!(e.undo.len(), undo, "an unchanged selection records no step");
}

/// Select Between is one undo step.
#[test]
fn select_between_undo_redo() {
    let mut e = ring8();
    polygons(&mut e, &[0, 4]);
    let before = e.scene.clone();
    run(&mut e, SelectionOperation::Between);
    let after = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
}

/// SEL-LP-01: Loop from one polygon and from two.
#[test]
fn sel_lp01_polygon_loops() {
    let mut e = grid4();
    polygons(&mut e, &[5]);
    run(&mut e, SelectionOperation::Loop);
    let got: BTreeSet<_> = e.scene.selection.faces.iter().copied().collect();
    assert_eq!(
        got,
        list(&oracle("SEL-O-poly_loop_base_from_one")["expected"]["selection"])
            .into_iter()
            .collect()
    );
    polygons(&mut e, &[5, 6]);
    run(&mut e, SelectionOperation::Loop);
    let got: BTreeSet<_> = e.scene.selection.faces.iter().copied().collect();
    assert_eq!(
        got,
        list(&oracle("SEL-O-poly_loop_base")["expected"]["selection"])
            .into_iter()
            .collect()
    );
}

/// SEL-LP-03 (base) and SEL-LP-04 (base): edge and point loops.
#[test]
fn sel_lp03_04_edge_and_point_loops() {
    let mut e = grid4();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = vec![(1, 6)];
    run(&mut e, SelectionOperation::Loop);
    let got: BTreeSet<_> = e.scene.selection.edges.iter().copied().collect();
    assert_eq!(
        got,
        edges(&oracle("SEL-O-edge_loop_base")["expected"]["selected_edges"])
    );

    let mut e = grid4();
    points(&mut e, &[6, 7]);
    run(&mut e, SelectionOperation::Loop);
    let got: BTreeSet<_> = e.scene.selection.vertices.iter().copied().collect();
    assert_eq!(
        got,
        list(&oracle("SEL-O-vert_loop_base")["expected"]["selection"])
            .into_iter()
            .collect()
    );
}

fn vec3(v: &Value) -> DVec3 {
    let a: Vec<f64> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    DVec3::new(a[0], a[1], a[2])
}

/// SEL-IT-01..03 (SEL-O-parent): Parent with and without keeping the
/// world position, then Unparent.
#[test]
fn sel_it01_03_parent_and_unparent() {
    let o = oracle("SEL-O-parent");
    for (key, keep) in [("in_place_off", false), ("in_place_on", true)] {
        let want = &o["expected"][key];
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.dispatch(Command::NewMesh).unwrap();
        let (parent, child) = (e.scene.items[0].id, e.scene.items[1].id);
        for (id, transform) in [
            (
                parent,
                scene::Transform {
                    translation: DVec3::X,
                    rotation: DVec3::new(0., 90f64.to_radians(), 0.),
                    ..Default::default()
                },
            ),
            (
                child,
                scene::Transform {
                    translation: DVec3::new(0., 0., 2.),
                    ..Default::default()
                },
            ),
        ] {
            e.dispatch(Command::ItemTransform {
                item: id,
                transform,
            })
            .unwrap();
        }
        let before = e.scene.clone();
        e.dispatch(Command::SetParent {
            item: child,
            parent: Some(parent),
            keep_position: keep,
        })
        .unwrap();
        let c = e.scene.items.iter().find(|i| i.id == child).unwrap();
        assert_eq!(c.parent, Some(parent));
        let close = |a: DVec3, b: DVec3| assert!((a - b).length() < 1e-9, "{key}: {a:?} != {b:?}");
        close(c.transform.translation, vec3(&want["local_pos"]));
        close(
            c.transform.rotation.map(f64::to_degrees),
            vec3(&want["local_rot_deg"]),
        );
        close(
            e.scene.world_point(child, DVec3::ZERO),
            vec3(&want["world_pos"]),
        );
        // SEL-IT-03: unparent with the same setting leaves the child at
        // (0,0,2) in local and world terms.
        let mut u = e.clone();
        u.dispatch(Command::SetParent {
            item: child,
            parent: None,
            keep_position: keep,
        })
        .unwrap();
        let c = u.scene.items.iter().find(|i| i.id == child).unwrap();
        assert_eq!(c.parent, None);
        close(c.transform.translation, DVec3::new(0., 0., 2.));
        close(
            u.scene.world_point(child, DVec3::ZERO),
            DVec3::new(0., 0., 2.),
        );
        // One undo step.
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before);
        // A parent can't be the item itself or its child.
        e.dispatch(Command::SetParent {
            item: child,
            parent: Some(parent),
            keep_position: keep,
        })
        .unwrap();
        let snapshot = e.scene.clone();
        for (item, p) in [(parent, child), (parent, parent)] {
            assert!(
                e.dispatch(Command::SetParent {
                    item,
                    parent: Some(p),
                    keep_position: keep,
                })
                .is_err()
            );
            assert_eq!(e.scene, snapshot);
        }
    }
}

/// Jason-specified: hiding an item with its children (Ctrl-click on the eye)
/// gives every descendant the parent's new visibility; one undo step.
#[test]
fn visibility_with_children_follows_the_parent() {
    let mut e = Editor::default();
    for _ in 0..3 {
        e.dispatch(Command::NewMesh).unwrap();
    }
    let ids: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    for (child, parent) in [(ids[1], ids[0]), (ids[2], ids[1])] {
        e.dispatch(Command::SetParent {
            item: child,
            parent: Some(parent),
            keep_position: false,
        })
        .unwrap();
    }
    let before = e.scene.clone();
    e.dispatch(Command::VisibilityWithChildren(ids[0])).unwrap();
    assert!(e.scene.items.iter().all(|i| !i.visible));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}
