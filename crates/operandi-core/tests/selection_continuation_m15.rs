//! SEL-M1..M6: Select More/Less, Select Between and Polygon Selection Fill.
use glam::DVec3;
use operandi_core::*;

const W: u32 = 5;
const H: u32 = 4;

/// Open W×H quad grid. Face (x, y) is `y * W + x`; vertex (x, y) is `y * (W + 1) + x`.
fn grid() -> Editor {
    let mut editor = Editor::default();
    editor.dispatch(Command::NewMesh).unwrap();
    let mesh = &mut editor.scene.active_mut().unwrap().mesh;
    for y in 0..=H {
        for x in 0..=W {
            mesh.vertices.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    for y in 0..H {
        for x in 0..W {
            let v = |x: u32, y: u32| y * (W + 1) + x;
            mesh.faces.push(Face {
                vertices: vec![v(x, y), v(x + 1, y), v(x + 1, y + 1), v(x, y + 1)],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    editor.scene.selection.mode = Mode::Polygon;
    editor
}
fn face(x: u32, y: u32) -> u32 {
    y * W + x
}
fn vertex(x: u32, y: u32) -> u32 {
    y * (W + 1) + x
}
fn edge(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}
fn run(e: &mut Editor, op: SelectionOperation) -> Result<()> {
    e.dispatch(Command::Selection(op))
}

#[test]
fn more_repeats_the_stride_of_the_last_two_picks() {
    let mut e = grid();
    let mesh = e.scene.active().unwrap().mesh.clone();
    e.scene.selection.faces = vec![face(0, 0), face(1, 0)];
    run(&mut e, SelectionOperation::More).unwrap();
    assert_eq!(e.scene.selection.faces, vec![0, 1, 2]);
    // A skipped polygon keeps being skipped.
    e.scene.selection.faces = vec![face(0, 0), face(2, 0)];
    run(&mut e, SelectionOperation::More).unwrap();
    assert_eq!(e.scene.selection.faces, vec![0, 2, 4]);
    // Vertical direction follows the picks, not a fixed axis.
    e.scene.selection.faces = vec![face(1, 0), face(1, 1)];
    run(&mut e, SelectionOperation::More).unwrap();
    assert_eq!(*e.scene.selection.faces.last().unwrap(), face(1, 2));
    assert_eq!(e.scene.active().unwrap().mesh, mesh);
    // Edge loop and edge ring.
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![
        edge(vertex(0, 0), vertex(1, 0)),
        edge(vertex(1, 0), vertex(2, 0)),
    ];
    run(&mut e, SelectionOperation::More).unwrap();
    assert_eq!(
        *e.scene.selection.edges.last().unwrap(),
        edge(vertex(2, 0), vertex(3, 0))
    );
    e.scene.selection.edges = vec![
        edge(vertex(0, 0), vertex(0, 1)),
        edge(vertex(1, 0), vertex(1, 1)),
    ];
    run(&mut e, SelectionOperation::More).unwrap();
    assert_eq!(
        *e.scene.selection.edges.last().unwrap(),
        edge(vertex(2, 0), vertex(2, 1))
    );
    // Vertex loop with a skip.
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![vertex(0, 1), vertex(2, 1)];
    run(&mut e, SelectionOperation::More).unwrap();
    assert_eq!(
        e.scene.selection.vertices,
        vec![vertex(0, 1), vertex(2, 1), vertex(4, 1)]
    );
}

#[test]
fn less_removes_latest_pick_down_to_nothing() {
    let mut e = grid();
    e.scene.selection.faces = vec![7, 3, 9];
    run(&mut e, SelectionOperation::Less).unwrap();
    assert_eq!(e.scene.selection.faces, vec![7, 3]);
    run(&mut e, SelectionOperation::Less).unwrap();
    run(&mut e, SelectionOperation::Less).unwrap();
    assert!(e.scene.selection.faces.is_empty());
    let before = e.scene.clone();
    assert!(run(&mut e, SelectionOperation::Less).is_err());
    assert_eq!(e.scene, before);
}

#[test]
fn between_selects_one_row_or_the_rectangle() {
    let mut e = grid();
    e.scene.selection.faces = vec![face(0, 0), face(3, 0)];
    run(&mut e, SelectionOperation::Between).unwrap();
    assert_eq!(e.scene.selection.faces, vec![0, 1, 2, 3]);
    e.scene.selection.faces = vec![face(3, 2), face(1, 0)];
    run(&mut e, SelectionOperation::Between).unwrap();
    // SEL-BT-02: the two picks first, then the rest of the rectangle.
    let mut got = e.scene.selection.faces.clone();
    assert_eq!(got[..2], [face(3, 2), face(1, 0)]);
    got.sort_unstable();
    let mut want: Vec<u32> = (0..=2)
        .flat_map(|y| (1..=3).map(move |x| face(x, y)))
        .collect();
    want.sort_unstable();
    assert_eq!(got, want);
    // Parallel edges across rows and columns form a rectangle of the same orientation.
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![
        edge(vertex(0, 0), vertex(1, 0)),
        edge(vertex(2, 2), vertex(3, 2)),
    ];
    run(&mut e, SelectionOperation::Between).unwrap();
    assert_eq!(e.scene.selection.edges.len(), 9);
    assert!(e.scene.selection.edges.iter().all(|&(a, b)| b == a + 1));
}

#[test]
fn fill_adds_the_enclosed_area_and_keeps_the_perimeter() {
    let mut e = grid();
    let border: Vec<u32> = (0..H)
        .flat_map(|y| (0..W).map(move |x| (x, y)))
        .filter(|&(x, y)| x == 0 || y == 0 || x == W - 1 || y == H - 1)
        .map(|(x, y)| face(x, y))
        .collect();
    e.scene.selection.faces = border.clone();
    let item = e.scene.active().unwrap().id;
    let mesh = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::SelectionFill {
        item,
        face: face(2, 1),
    })
    .unwrap();
    assert_eq!(&e.scene.selection.faces[..border.len()], &border[..]);
    assert_eq!(e.scene.selection.faces.len(), (W * H) as usize);
    assert_eq!(e.scene.active().unwrap().mesh, mesh);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection.faces, border);

    // Closed mesh: a middle band encloses the top cap.
    e.scene.active_mut().unwrap().mesh =
        primitive::segmented_cube(DVec3::ZERO, DVec3::new(2., 3., 2.), [1, 3, 1])
            .unwrap()
            .into();
    let cube = e.scene.active().unwrap().mesh.clone();
    let band: Vec<u32> = (0..cube.faces.len())
        .filter(|&f| cube.normal(f).y.abs() < 0.5 && cube.center(f).y.abs() < 1e-9)
        .map(|f| f as u32)
        .collect();
    assert_eq!(band.len(), 4);
    let top = (0..cube.faces.len())
        .find(|&f| cube.normal(f).y > 0.9)
        .unwrap() as u32;
    e.scene.selection.faces = band.clone();
    e.dispatch(Command::SelectionFill { item, face: top })
        .unwrap();
    assert!(
        e.scene.selection.faces[band.len()..]
            .iter()
            .all(|&f| cube.center(f as usize).y > 0.)
    );
    assert_eq!(e.scene.selection.faces.len(), 4 + 4 + 1);
}

#[test]
fn refusals_are_atomic() {
    let mut e = grid();
    let item = e.scene.active().unwrap().id;
    let refuse = |e: &mut Editor, faces: Vec<u32>, cmd: Command| {
        e.scene.selection.faces = faces;
        let before = e.scene.clone();
        assert!(e.dispatch(cmd).is_err());
        assert_eq!(e.scene, before);
    };
    let op = |o| Command::Selection(o);
    refuse(&mut e, vec![3], op(SelectionOperation::More));
    // Not on one loop.
    refuse(
        &mut e,
        vec![face(0, 0), face(1, 1)],
        op(SelectionOperation::More),
    );
    // Open loop ends before the next element.
    refuse(
        &mut e,
        vec![face(2, 0), face(4, 0)],
        op(SelectionOperation::More),
    );
    // Three picks are allowed since SEL-BT-03 (only the last pair joins).
    // Fill seed on the perimeter, and an area leaking to the open border.
    refuse(
        &mut e,
        vec![face(1, 1)],
        Command::SelectionFill {
            item,
            face: face(1, 1),
        },
    );
    refuse(
        &mut e,
        vec![face(1, 1), face(2, 1)],
        Command::SelectionFill {
            item,
            face: face(3, 3),
        },
    );
    // Between rejects crossed edge orientations. (Points off one row add
    // nothing rather than refusing, SEL-BT-04.)
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![
        edge(vertex(0, 0), vertex(1, 0)),
        edge(vertex(3, 1), vertex(3, 2)),
    ];
    let before = e.scene.clone();
    assert!(run(&mut e, SelectionOperation::Between).is_err());
    assert_eq!(e.scene, before);
    // Closed mesh with a non-separating selection.
    e.scene.selection.mode = Mode::Polygon;
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    refuse(&mut e, vec![0], Command::SelectionFill { item, face: 1 });
}

#[test]
fn continuation_is_one_undo_step_and_survives_reopen() {
    let mut e = grid();
    e.scene.selection.faces = vec![face(0, 0), face(2, 2)];
    let before = e.scene.clone();
    run(&mut e, SelectionOperation::Between).unwrap();
    let after = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
    let path = std::env::temp_dir().join(format!(
        "operandi-m15-continuation-{}.opscene",
        std::process::id()
    ));
    persistence::save(&path, &after).unwrap();
    assert_eq!(persistence::load(&path).unwrap(), after);
    std::fs::remove_file(path).unwrap();
}
