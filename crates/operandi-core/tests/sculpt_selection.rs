//! Sculpt brushes respect the component selection (spec sculpt-selection,
//! SCS-01..09).
use glam::DVec3;
use operandi_core::sculpt::{Dab, Stroke, StrokeSetup};
use operandi_core::*;

const N: u32 = 20;

fn grid() -> Mesh {
    let mut mesh = Mesh::default();
    for j in 0..=N {
        for i in 0..=N {
            mesh.vertices.push(DVec3::new(
                i as f64 / N as f64 - 0.5,
                j as f64 / N as f64 - 0.5,
                0.,
            ));
        }
    }
    for j in 0..N {
        for i in 0..N {
            let a = j * (N + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + N + 2, a + N + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    mesh
}

/// The point at column `i`, row `j` (x = i/20 − 0.5).
fn at(i: u32, j: u32) -> u32 {
    j * (N + 1) + i
}

fn editor() -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = grid().into();
    let id = e.scene.active().unwrap().id;
    (e, id)
}

fn setup(brush: SculptBrush) -> StrokeSetup {
    StrokeSetup {
        brush,
        settings: brush.defaults(),
        ctrl: false,
        shift: false,
        symmetry: Symmetry::OFF,
        falloff: None,
        lock_borders: false,
        lock_corners: true,
        radius: 0.2,
        weight_map: None,
    }
}

fn dab(x: f64) -> Dab {
    Dab {
        center: DVec3::new(x, 0., 0.),
        radius: 0.2,
        view: DVec3::Z,
        normal: DVec3::Z,
        pressure: 1.,
    }
}

fn z(scene: &Scene, id: u64, v: u32) -> f64 {
    scene.item(id).unwrap().mesh.vertices[v as usize].z
}

/// A stroke of `dabs` dabs at x, left in `scene` (not committed).
fn stroke(scene: &mut Scene, items: &[u64], st: StrokeSetup, x: f64, dabs: usize) -> Stroke {
    let mut s = Stroke::begin(scene, items, st).unwrap().unwrap();
    for _ in 0..dabs {
        s.dab(dab(x));
    }
    s.flush(scene);
    s
}

fn select_vertices(e: &mut Editor, vertices: &[u32]) {
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vertices.to_vec();
}

/// SCS-01, SCS-04, SCS-09: only selected vertices move, at the brush's
/// full weight; the stroke is one undo step and undo restores it.
#[test]
fn vertex_selection_limits_the_brush() {
    let (mut e, id) = editor();
    let (mid, right, left) = (at(10, 10), at(11, 10), at(9, 10));
    select_vertices(&mut e, &[mid, right]);
    let mut scene = e.scene.clone();
    let s = stroke(&mut scene, &[id], setup(SculptBrush::Push), 0., 20);
    // Same height as without a selection: no fade at the border (SCS-04).
    let (free, free_id) = editor();
    let mut free_scene = free.scene.clone();
    stroke(
        &mut free_scene,
        &[free_id],
        setup(SculptBrush::Push),
        0.,
        20,
    );
    assert!(z(&scene, id, mid) > 0.001);
    assert_eq!(z(&scene, id, mid), z(&free_scene, free_id, mid));
    assert!(z(&scene, id, right) > 0.001);
    assert_eq!(z(&scene, id, left), 0.);
    assert_eq!(z(&scene, id, at(10, 11)), 0.);
    let edit = s.edit();
    assert_eq!(edit.items[0].points.len(), 2);
    let steps = e.undo.len();
    e.dispatch(Command::SculptStroke(Box::new(edit))).unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert!(z(&e.scene, id, mid) > 0.001);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(z(&e.scene, id, mid), 0.);
    assert_eq!(e.scene.selection.vertices, vec![mid, right]);
}

/// SCS-01: both ends of each selected edge (a row loop) move.
#[test]
fn edge_selection_moves_the_loop() {
    let (mut e, id) = editor();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = (0..N).map(|i| (at(i, 10), at(i + 1, 10))).collect();
    let mut scene = e.scene.clone();
    stroke(&mut scene, &[id], setup(SculptBrush::Push), 0., 20);
    for i in 8..=12 {
        assert!(z(&scene, id, at(i, 10)) > 0.001, "row point {i}");
        assert_eq!(z(&scene, id, at(i, 11)), 0., "row above {i}");
        assert_eq!(z(&scene, id, at(i, 9)), 0., "row below {i}");
    }
}

/// SCS-01: every corner of a selected polygon moves, nothing else.
#[test]
fn polygon_selection_moves_its_corners() {
    let (mut e, id) = editor();
    e.scene.selection.mode = Mode::Polygon;
    // The quad with corners (10,10), (11,10), (11,11), (10,11).
    e.scene.selection.faces = vec![10 * N + 10];
    let mut scene = e.scene.clone();
    stroke(&mut scene, &[id], setup(SculptBrush::Push), 0., 20);
    for v in [at(10, 10), at(11, 10), at(11, 11), at(10, 11)] {
        assert!(z(&scene, id, v) > 0.001, "corner {v}");
    }
    for v in [at(9, 10), at(12, 10), at(10, 9), at(10, 12)] {
        assert_eq!(z(&scene, id, v), 0., "outside {v}");
    }
}

/// SCS-02: nothing selected in the current mode, or Item mode, sculpts the
/// whole mesh.
#[test]
fn no_selection_or_item_mode_sculpts_everything() {
    let (mut e, id) = editor();
    // Polygon mode with only (remembered) vertices selected.
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.vertices = vec![at(10, 10)];
    let mut scene = e.scene.clone();
    stroke(&mut scene, &[id], setup(SculptBrush::Push), 0., 20);
    assert!(z(&scene, id, at(9, 10)) > 0.001);
    e.scene.selection.mode = Mode::Item;
    let mut scene = e.scene.clone();
    stroke(&mut scene, &[id], setup(SculptBrush::Push), 0., 20);
    assert!(z(&scene, id, at(9, 10)) > 0.001);
}

/// SCS-03: with two active meshes, one with a selection, the other is left
/// alone; with neither selected both are sculpted.
#[test]
fn a_mesh_without_a_selection_is_left_alone() {
    let (mut e, a) = editor();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = grid().into();
    let b = e.scene.active().unwrap().id;
    let mid = at(10, 10);
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices.clear();
    let mut scene = e.scene.clone();
    stroke(&mut scene, &[a, b], setup(SculptBrush::Push), 0., 20);
    assert!(z(&scene, a, mid) > 0.001 && z(&scene, b, mid) > 0.001);
    // `b` is the primary; `a` has no components selected.
    e.scene.selection.vertices = vec![mid];
    let mut scene = e.scene.clone();
    stroke(&mut scene, &[a, b], setup(SculptBrush::Push), 0., 20);
    assert!(z(&scene, b, mid) > 0.001);
    assert_eq!(z(&scene, b, at(9, 10)), 0.);
    assert_eq!(z(&scene, a, mid), 0.);
}

/// SCS-05, SCS-06: Shift-smooth moves only selected points, and they still
/// average against unselected neighbours (a spike they surround pulls them
/// up while the spike itself stays put).
#[test]
fn smooth_uses_unselected_neighbours_but_moves_only_the_selection() {
    let (mut e, id) = editor();
    let mid = at(10, 10);
    e.scene.active_mut().unwrap().mesh.vertices[mid as usize].z = 0.2;
    select_vertices(&mut e, &[at(11, 10)]);
    let mut scene = e.scene.clone();
    let st = StrokeSetup {
        shift: true,
        ..setup(SculptBrush::Push)
    };
    stroke(&mut scene, &[id], st, 0., 10);
    assert_eq!(z(&scene, id, mid), 0.2);
    assert!(z(&scene, id, at(11, 10)) > 0.001);
    assert_eq!(z(&scene, id, at(9, 10)), 0.);
}

/// SCS-05: Move grabs only selected points.
#[test]
fn move_grabs_only_the_selection() {
    let (mut e, id) = editor();
    let mid = at(10, 10);
    select_vertices(&mut e, &[mid]);
    let mut scene = e.scene.clone();
    let mut s = Stroke::begin(&scene, &[id], setup(SculptBrush::Move))
        .unwrap()
        .unwrap();
    s.dab(dab(0.));
    s.grab_to(DVec3::new(0., 0., 0.2));
    s.flush(&mut scene);
    assert!((z(&scene, id, mid) - 0.2).abs() < 1e-9);
    assert_eq!(z(&scene, id, at(11, 10)), 0.);
}

/// SCS-07: a mirrored dab leaves an unselected mirror partner alone.
#[test]
fn symmetry_leaves_an_unselected_mirror_alone() {
    let (mut e, id) = editor();
    let (right, left) = (at(16, 10), at(4, 10));
    select_vertices(&mut e, &[right]);
    let mut scene = e.scene.clone();
    let st = StrokeSetup {
        symmetry: Symmetry::x(),
        ..setup(SculptBrush::Push)
    };
    stroke(&mut scene, &[id], st.clone(), 0.3, 20);
    assert!(z(&scene, id, right) > 0.001);
    assert_eq!(z(&scene, id, left), 0.);
    // The mirrored dab still moves a selected point on the other side.
    select_vertices(&mut e, &[right, left]);
    let mut scene = e.scene.clone();
    stroke(&mut scene, &[id], st, 0.3, 20);
    assert!((z(&scene, id, right) - z(&scene, id, left)).abs() < 1e-9);
    assert!(z(&scene, id, left) > 0.001);
}

/// SCS-08: a locked point stays frozen even when selected; a selection of
/// nothing but frozen points leaves nothing to sculpt.
#[test]
fn locked_points_stay_frozen_inside_the_selection() {
    let (mut e, id) = editor();
    let (mid, right) = (at(10, 10), at(11, 10));
    e.scene
        .active_mut()
        .unwrap()
        .elements
        .locked
        .vertices
        .insert(mid);
    select_vertices(&mut e, &[mid, right]);
    let mut scene = e.scene.clone();
    stroke(&mut scene, &[id], setup(SculptBrush::Push), 0., 20);
    assert_eq!(z(&scene, id, mid), 0.);
    assert!(z(&scene, id, right) > 0.001);
    select_vertices(&mut e, &[mid]);
    assert!(
        Stroke::begin(&e.scene, &[id], setup(SculptBrush::Push))
            .unwrap()
            .is_none()
    );
}

/// SCS-05: the Vertex Map brushes paint only the selected points.
#[test]
fn vertex_map_brushes_paint_only_the_selection() {
    let (mut e, id) = editor();
    e.dispatch(Command::NewWeightMap {
        name: "Mask".into(),
        value: None,
    })
    .unwrap();
    let mid = at(10, 10);
    select_vertices(&mut e, &[mid]);
    let st = StrokeSetup {
        weight_map: e.selected_weight_map.clone(),
        ..setup(SculptBrush::VmPaint)
    };
    let mut s = Stroke::begin(&e.scene, &[id], st).unwrap().unwrap();
    for _ in 0..10 {
        s.dab(dab(0.));
    }
    let edit = s.weight_edit().expect("a weight stroke");
    s.restore(&mut e.scene);
    e.dispatch(Command::WeightStroke(Box::new(edit))).unwrap();
    let item = e.scene.item(id).unwrap();
    let map = &item.mesh.attributes.point_weights["Mask"];
    assert!(map.get(&mid).is_some_and(|&w| w > 0.));
    assert_eq!(map.len(), 1, "{map:?}");
}
