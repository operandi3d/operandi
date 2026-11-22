//! Behaviour parity fixes PF-01..PF-07 (selection), with the exact cases
//! of BEHAVIOR-PARITY-FIXES.
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

fn editor(vertices: Vec<DVec3>, faces: Vec<Vec<u32>>) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    mesh.vertices = vertices;
    mesh.faces = faces.into_iter().map(face).collect();
    e.synchronize_identities().unwrap();
    e
}

/// G4: 25 points (i, j, 0), index 5j + i; quad 4j + i = [a, a+1, a+6, a+5].
fn g4() -> Editor {
    let points = (0..25)
        .map(|k| DVec3::new((k % 5) as f64, (k / 5) as f64, 0.))
        .collect();
    let mut quads = Vec::new();
    for j in 0..4u32 {
        for i in 0..4u32 {
            let a = 5 * j + i;
            quads.push(vec![a, a + 1, a + 6, a + 5]);
        }
    }
    editor(points, quads)
}

/// T3: open tube of 8 sides and 3 rows; index 8y + i.
fn t3() -> Editor {
    let round = |v: f64| (v * 1e6).round() / 1e6;
    let mut points = Vec::new();
    for y in 0..4 {
        for i in 0..8 {
            let a = std::f64::consts::FRAC_PI_4 * i as f64;
            points.push(DVec3::new(round(a.cos()), y as f64, round(a.sin())));
        }
    }
    let mut quads = Vec::new();
    for y in 0..3u32 {
        for i in 0..8u32 {
            let n = (i + 1) % 8;
            quads.push(vec![8 * y + i, 8 * y + n, 8 * (y + 1) + n, 8 * (y + 1) + i]);
        }
    }
    editor(points, quads)
}

fn op(e: &mut Editor, o: SelectionOperation) -> std::result::Result<(), String> {
    e.dispatch(Command::Selection(o)).map(|_| ())
}

fn sorted<T: Ord + Clone>(v: &[T]) -> Vec<T> {
    let mut v = v.to_vec();
    v.sort();
    v
}

fn edge(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

fn faces(e: &mut Editor, mode: Mode, faces: &[u32]) {
    e.scene.selection.mode = mode;
    e.scene.selection.faces = faces.to_vec();
}

// ---- PF-01 Grow

#[test]
fn pf01_grow_adds_every_polygon_sharing_a_point() {
    let mut e = g4();
    faces(&mut e, Mode::Polygon, &[6]);
    op(&mut e, SelectionOperation::Grow).unwrap();
    assert_eq!(
        sorted(&e.scene.selection.faces),
        [1, 2, 3, 5, 6, 7, 9, 10, 11]
    );
    let mut e = g4();
    faces(&mut e, Mode::Polygon, &[0]);
    op(&mut e, SelectionOperation::Grow).unwrap();
    op(&mut e, SelectionOperation::Grow).unwrap();
    assert_eq!(
        sorted(&e.scene.selection.faces),
        [0, 1, 2, 4, 5, 6, 8, 9, 10]
    );
}

#[test]
fn pf01_vertex_grow_is_unchanged() {
    let mut e = g4();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![12];
    op(&mut e, SelectionOperation::Grow).unwrap();
    assert_eq!(sorted(&e.scene.selection.vertices), [7, 11, 12, 13, 17]);
}

// ---- PF-02 Shrink

#[test]
fn pf02_shrink_keeps_the_open_border() {
    let mut e = g4();
    faces(&mut e, Mode::Polygon, &[0, 1, 2, 4, 5, 6, 8, 9, 10]);
    op(&mut e, SelectionOperation::Shrink).unwrap();
    assert_eq!(sorted(&e.scene.selection.faces), [0, 1, 4, 5]);

    let mut e = g4();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![0, 1, 2, 3, 5, 6, 7, 8, 10, 11, 12, 13, 15, 16, 17, 18];
    op(&mut e, SelectionOperation::Shrink).unwrap();
    assert_eq!(
        sorted(&e.scene.selection.vertices),
        [0, 1, 2, 5, 6, 7, 10, 11, 12]
    );
}

// ---- PF-03 Loop from a border edge

#[test]
fn pf03_border_loop_follows_the_whole_open_border() {
    let mut e = g4();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![edge(0, 1)];
    op(&mut e, SelectionOperation::Loop).unwrap();
    let want: Vec<_> = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (4, 9),
        (9, 14),
        (14, 19),
        (19, 24),
        (23, 24),
        (22, 23),
        (21, 22),
        (20, 21),
        (15, 20),
        (10, 15),
        (5, 10),
        (0, 5),
    ]
    .into_iter()
    .map(|(a, b)| edge(a, b))
    .collect();
    assert_eq!(sorted(&e.scene.selection.edges), sorted(&want));
}

#[test]
fn pf03_interior_loop_is_unchanged() {
    let mut e = g4();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![edge(6, 7)];
    op(&mut e, SelectionOperation::Loop).unwrap();
    assert_eq!(
        sorted(&e.scene.selection.edges),
        [edge(5, 6), edge(6, 7), edge(7, 8), edge(8, 9)]
    );
}

// ---- PF-04 Boundary in Edge mode

#[test]
fn pf04_edge_mode_boundary_with_nothing_selected() {
    let mut e = t3();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges.clear();
    op(&mut e, SelectionOperation::Boundary).unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Edge);
    let mut want = Vec::new();
    for ring in [0u32, 24] {
        for i in 0..8 {
            want.push(edge(ring + i, ring + (i + 1) % 8));
        }
    }
    assert_eq!(sorted(&e.scene.selection.edges), sorted(&want));
}

#[test]
fn pf04_polygon_boundary_is_unchanged() {
    let mut e = g4();
    faces(&mut e, Mode::Polygon, &[5, 6, 9, 10]);
    op(&mut e, SelectionOperation::Boundary).unwrap();
    assert_eq!(e.scene.selection.edges.len(), 8);
    let mut e = g4();
    faces(&mut e, Mode::Polygon, &[]);
    op(&mut e, SelectionOperation::Boundary).unwrap();
    assert_eq!(e.scene.selection.edges.len(), 16);
}

// ---- PF-05 Close Loop with polygons

#[test]
fn pf05_close_loop_completes_the_ring() {
    let mut e = t3();
    faces(&mut e, Mode::Polygon, &[8, 9, 10]);
    op(&mut e, SelectionOperation::CloseLoop).unwrap();
    assert_eq!(
        sorted(&e.scene.selection.faces),
        (8..16).collect::<Vec<u32>>()
    );
}

// ---- PF-06 Next / Previous Loop in Vertex mode

fn points(e: &mut Editor, start: &[u32], next: bool, steps: usize) -> Vec<Vec<u32>> {
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = start.to_vec();
    let o = if next {
        SelectionOperation::NextLoop
    } else {
        SelectionOperation::PreviousLoop
    };
    (0..steps)
        .map(|_| {
            op(e, o).unwrap();
            sorted(&e.scene.selection.vertices)
        })
        .collect()
}

fn border_points() -> Vec<u32> {
    sorted(&[0, 1, 2, 3, 4, 9, 14, 19, 24, 23, 22, 21, 20, 15, 10, 5])
}

#[test]
fn pf06_next_and_previous_loop_on_points() {
    let mut e = g4();
    assert_eq!(
        points(&mut e, &[2, 7, 12, 17, 22], true, 2),
        [vec![3, 8, 13, 18, 23], border_points()]
    );
    let mut e = g4();
    assert_eq!(
        points(&mut e, &[2, 7, 12, 17, 22], false, 2),
        [vec![1, 6, 11, 16, 21], border_points()]
    );
}

// ---- PF-07 Close Loop must not stall

#[test]
fn pf07_close_loop_on_an_open_edge_run_answers_at_once() {
    let mut e = g4();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![edge(6, 7), edge(7, 8)];
    let start = std::time::Instant::now();
    let _ = op(&mut e, SelectionOperation::CloseLoop);
    assert!(start.elapsed().as_secs_f64() < 1.0, "{:?}", start.elapsed());
}
