//! Deleting an edge loop with forward Delete (`delete`) merges the polygons
//! on either side and takes out the points the loop leaves behind between
//! two edges; Backspace (Edge Remove) keeps them (Jason, 2026-11-10;
//! provisional ER-Q01).
use glam::DVec3;
use operandi_core::*;
use serde_json::Map;
use std::collections::BTreeMap;

const SIDES: u32 = 12;
const SEGMENTS: u32 = 8;

fn globe() -> Mesh {
    globe_geometry(
        DVec3::ZERO,
        RadialParameters {
            radius: DVec3::ONE,
            sides: SIDES,
            segments: SEGMENTS,
        },
    )
    .unwrap()
}

fn loop_edges(points: &[u32], closed: bool) -> Vec<(u32, u32)> {
    let mut edges: Vec<(u32, u32)> = points.windows(2).map(|w| (w[0], w[1])).collect();
    if closed {
        edges.push((points[points.len() - 1], points[0]));
    }
    edges
}

/// Valid, closed (every edge on exactly two polygons) and with no point
/// left between just two edges.
fn assert_clean_closed(m: &Mesh) {
    m.validate().unwrap();
    let mut uses: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for f in &m.faces {
        assert!(f.vertices.len() >= 3);
        let n = f.vertices.len();
        for i in 0..n {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % n]);
            *uses.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    assert!(uses.values().all(|&u| u == 2), "a hole or a stray edge");
    let mut valence = vec![0; m.vertices.len()];
    for &(a, b) in uses.keys() {
        valence[a as usize] += 1;
        valence[b as usize] += 1;
    }
    assert!(
        valence.iter().all(|&v| v >= 3),
        "a point is left between two edges or unused: {valence:?}"
    );
}

#[test]
fn removing_a_latitude_loop_leaves_quads_and_no_loop_points() {
    let m = globe();
    let first = 1 + 3 * SIDES;
    let ring: Vec<u32> = (first..first + SIDES).collect();
    assert_eq!(
        tweak::edge_loop_points(&m, ring[0], ring[1]).len(),
        ring.len()
    );
    let out = edge_delete_geometry(&m, &loop_edges(&ring, true)).unwrap();
    assert_clean_closed(&out);
    assert_eq!(out.vertices.len(), m.vertices.len() - SIDES as usize);
    assert_eq!(out.faces.len(), m.faces.len() - SIDES as usize);
    assert!(out.faces.iter().all(|f| f.vertices.len() <= 4));
    // The loop's points are gone; the rest keep their places.
    for p in &ring {
        let gone = m.vertices[*p as usize];
        assert!(out.vertices.iter().all(|v| v.distance(gone) > 1e-9));
    }
}

#[test]
fn removing_a_pole_to_pole_loop_keeps_the_poles() {
    let m = globe();
    let points = tweak::edge_loop_points(&m, 1, 1 + SIDES);
    assert_eq!(points.len(), SEGMENTS as usize + 1, "{points:?}");
    let out = edge_delete_geometry(&m, &loop_edges(&points, false)).unwrap();
    assert_clean_closed(&out);
    // Only the points between the poles go.
    assert_eq!(
        out.vertices.len(),
        m.vertices.len() - (SEGMENTS as usize - 1)
    );
    assert_eq!(out.faces.len(), m.faces.len() - SEGMENTS as usize);
    for pole in [m.vertices[0], m.vertices[m.vertices.len() - 1]] {
        assert!(out.vertices.iter().any(|v| v.distance(pole) < 1e-9));
    }
}

#[test]
fn removing_the_loop_next_to_a_pole_leaves_triangles_there() {
    let m = globe();
    let ring: Vec<u32> = (1..1 + SIDES).collect();
    let out = edge_delete_geometry(&m, &loop_edges(&ring, true)).unwrap();
    assert_clean_closed(&out);
    assert_eq!(out.vertices.len(), m.vertices.len() - SIDES as usize);
    assert_eq!(
        out.faces.iter().filter(|f| f.vertices.len() == 3).count(),
        2 * SIDES as usize
    );
}

#[test]
fn delete_loop_is_one_undo_step_and_reopens_the_same() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = globe().into();
    e.synchronize_identities().unwrap();
    let first = 1 + 3 * SIDES;
    let ring: Vec<u32> = (first..first + SIDES).collect();
    e.scene.selection.mode = Mode::Edge;
    // Selected edges are stored low point first.
    e.scene.selection.edges = loop_edges(&ring, true)
        .into_iter()
        .map(|(a, b)| (a.min(b), a.max(b)))
        .collect();
    let before = e.scene.clone();
    // Backspace's Edge Remove merges the polygons but keeps the loop's
    // points (CMD-E-01): one hexagon per pair, nothing removed.
    let mut kept = e.clone();
    kept.dispatch(Command::EdgeCommand(EdgeCommand::Remove))
        .unwrap();
    let kept = kept.scene.active().unwrap().mesh.clone();
    assert_eq!(kept.vertices.len(), globe().vertices.len());
    assert_eq!(
        kept.faces.iter().filter(|f| f.vertices.len() == 6).count(),
        SIDES as usize
    );
    // Forward Delete's command `delete` also takes the loop's points.
    command_exec::run(&mut e, "delete", &Default::default()).unwrap();
    let after = e.scene.active().unwrap().mesh.clone();
    assert_clean_closed(&after);
    assert_eq!(e.undo.last().unwrap().label, "Delete");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, after);
    let path =
        std::env::temp_dir().join(format!("edge-loop-delete-{}.opscene", std::process::id()));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(back.active().unwrap().mesh, after);
}

/// `delete` on a point shared by four quads takes the quads with it: one
/// eight-sided hole edged by quads, no triangles and no stray points
/// (Jason, 2026-11-10).
#[test]
fn deleting_a_point_removes_its_quads_and_leaves_an_eight_sided_hole() {
    let m = globe();
    let point = 1 + 3 * SIDES + 2;
    let around: Vec<usize> = (0..m.faces.len())
        .filter(|&f| m.faces[f].vertices.contains(&point))
        .collect();
    assert_eq!(around.len(), 4);
    let faces: Vec<u32> = around.iter().map(|&f| f as u32).collect();
    let out = delete_geometry(&m, &faces, &[point]).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), m.faces.len() - 4);
    assert_eq!(out.vertices.len(), m.vertices.len() - 1);
    assert_eq!(
        out.faces.iter().filter(|f| f.vertices.len() == 3).count(),
        m.faces.iter().filter(|f| f.vertices.len() == 3).count(),
        "no new triangles"
    );
    // The hole's border: eight edges, each on one polygon.
    let mut uses: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for f in &out.faces {
        let n = f.vertices.len();
        for i in 0..n {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % n]);
            *uses.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    assert_eq!(uses.values().filter(|&&u| u == 1).count(), 8);
    assert!(uses.values().all(|&u| u <= 2));
}

/// `delete` runs in every mode through one command: points and polygons
/// through `DeleteComponents` (polygon Delete also drops points left
/// unused), edges through Edge Remove; an empty selection refuses.
#[test]
fn delete_command_per_mode_is_one_undo_step() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = globe().into();
    e.synchronize_identities().unwrap();
    let none = Map::new();
    for mode in [Mode::Vertex, Mode::Polygon] {
        e.scene.selection.mode = mode;
        e.scene.selection.vertices.clear();
        e.scene.selection.faces.clear();
        let before = e.scene.clone();
        assert!(
            command_exec::run(&mut e, "delete", &none).is_err(),
            "{mode:?}"
        );
        assert_eq!(e.scene, before);
        e.scene.selection.vertices = vec![40];
        e.scene.selection.faces = vec![3];
        let start = e.scene.clone();
        command_exec::run(&mut e, "delete", &none).unwrap();
        assert_eq!(e.undo.last().unwrap().label, "Delete");
        let mesh = &e.scene.active().unwrap().mesh;
        let mut used = vec![false; mesh.vertices.len()];
        for f in &mesh.faces {
            for &v in &f.vertices {
                used[v as usize] = true;
            }
        }
        assert!(used.iter().all(|&u| u), "{mode:?} left an unused point");
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, start);
    }
    // A lone polygon's points go with it; shared ones stay.
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![3];
    command_exec::run(&mut e, "delete", &none).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh.vertices.len(),
        globe().vertices.len()
    );
}
