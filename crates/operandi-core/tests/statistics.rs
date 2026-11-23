//! Statistics panel counts and its +/− and row menu commands (spec
//! development/specs/statistics-panel/BEHAVIOR-STATISTICS.md §11).
#![allow(clippy::field_reassign_with_default)] // meshes built field by field
use glam::DVec3;
use operandi_core::statistics::{
    self, Border, Check, Kind, MORE, Need, PolyType, Report, Row, Tally,
};
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

/// §11: one quad, two triangles, one five-sided polygon and four loose
/// points (15 points, 4 polygons, 13 edges). The quad shares an edge with
/// one triangle, the pentagon with the other; everything lies flat.
fn acceptance_mesh() -> Mesh {
    let mut m = Mesh::default();
    let p = |x: f64, y: f64| DVec3::new(x, y, 0.);
    m.vertices = vec![
        p(0., 0.),
        p(1., 0.),
        p(1., 1.),
        p(0., 1.),
        p(2., 0.5),
        p(5., 0.),
        p(6., 0.),
        p(6., 1.),
        p(7., 0.),
        p(7.5, 0.5),
        p(7., 1.),
        p(10., 0.),
        p(11., 0.),
        p(12., 0.),
        p(13., 0.),
    ];
    let mut quad = face(vec![0, 1, 2, 3]);
    quad.material = "M".into();
    quad.part = Some("Wheel".into());
    let tri_a = face(vec![1, 4, 2]);
    let tri_b = face(vec![5, 6, 7]);
    let pentagon = face(vec![6, 8, 9, 10, 7]);
    m.faces = vec![quad, tri_a, tri_b, pentagon].into();
    m
}

fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = mode;
    e
}

fn report(e: &Editor) -> Report {
    statistics::compute(&statistics::snapshot(&e.scene, Need::ALL))
}

fn count(r: &Report, row: Row) -> u64 {
    r.get(&row).unwrap_or_default().count
}

fn run(e: &mut Editor, line: &str) {
    match command_line::run_line(e, line) {
        Some(Ok(_)) => {}
        Some(Err(err)) => panic!("{line}: {err}"),
        None => panic!("{line}: not a headless command"),
    }
}

#[test]
fn acceptance_counts() {
    let e = editor(acceptance_mesh(), Mode::Polygon);
    let r = report(&e);
    // 1.
    assert_eq!(count(&r, Row::All(Kind::Vertex)), 15);
    assert_eq!(count(&r, Row::All(Kind::Edge)), 13);
    assert_eq!(count(&r, Row::All(Kind::Polygon)), 4);
    assert_eq!(count(&r, Row::PolygonVertices(3)), 2);
    assert_eq!(count(&r, Row::PolygonVertices(4)), 1);
    assert_eq!(count(&r, Row::PolygonVertices(MORE)), 1);
    // 2.
    assert_eq!(count(&r, Row::VertexEdges(0)), 4);
    assert_eq!(count(&r, Row::VertexPolygons(0)), 4);
    // 3.
    assert_eq!(count(&r, Row::EdgePolygons(1)), 11);
    assert_eq!(count(&r, Row::EdgePolygons(2)), 2);
    assert_eq!(count(&r, Row::EdgeBorder(Border::Geometry)), 11);
    assert_eq!(count(&r, Row::EdgeBorder(Border::Flat)), 2);
    // The quad (Wheel, M) borders the untagged triangle.
    assert_eq!(count(&r, Row::EdgeBorder(Border::Part)), 1);
    assert_eq!(count(&r, Row::EdgeBorder(Border::Material)), 1);
    assert_eq!(count(&r, Row::PolygonType(PolyType::Face)), 4);
    assert_eq!(count(&r, Row::Part("Wheel".into())), 1);
    assert_eq!(count(&r, Row::Material("M".into())), 1);
    assert_eq!(count(&r, Row::Material("Default".into())), 3);
    assert_eq!(count(&r, Row::PolygonCheck(Check::FlatNeighbour)), 4);
    assert_eq!(count(&r, Row::PolygonCheck(Check::NotFlat)), 0);
    assert_eq!(count(&r, Row::PolygonCheck(Check::Concave)), 0);
    assert_eq!(r.names.parts, ["Wheel"]);
}

#[test]
fn plus_and_minus_select_rows_and_every_sel_cell_follows() {
    let mut e = editor(acceptance_mesh(), Mode::Polygon);
    // 4.
    run(&mut e, "operandi.statistics.select row:polygons.vertices:3");
    let mut faces = e.scene.selection.faces.clone();
    faces.sort();
    assert_eq!(faces, [1, 2]);
    let r = report(&e);
    let sel = |row| r.get(&row).unwrap_or_default().selected;
    assert_eq!(sel(Row::PolygonVertices(3)), 2);
    // Every row's Sel, not only the clicked one.
    assert_eq!(sel(Row::All(Kind::Polygon)), 2);
    assert_eq!(sel(Row::Material("Default".into())), 2);
    run(&mut e, "operandi.statistics.select row:polygons.vertices:4");
    assert_eq!(e.scene.selection.faces.len(), 3);
    run(
        &mut e,
        "operandi.statistics.select row:polygons.vertices:3 action:remove",
    );
    assert_eq!(e.scene.selection.faces, [0]);
    // Each click is one undo step.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection.faces.len(), 3);
    // Select only replaces; a row of 0 does nothing.
    run(
        &mut e,
        "operandi.statistics.select row:polygons.vertices:more action:only",
    );
    assert_eq!(e.scene.selection.faces, [3]);
    let undo = e.undo.len();
    run(&mut e, "operandi.statistics.select row:polygons.vertices:1");
    assert_eq!(e.undo.len(), undo, "an empty row records nothing");
    assert_eq!(e.scene.selection.faces, [3]);
}

#[test]
fn plus_on_a_vertex_row_switches_to_vertex_mode() {
    // 5.
    let mut e = editor(acceptance_mesh(), Mode::Polygon);
    let undo = e.undo.len();
    run(&mut e, "operandi.statistics.select row:vertices.edges:0");
    assert_eq!(e.scene.selection.mode, Mode::Vertex);
    let mut v = e.scene.selection.vertices.clone();
    v.sort();
    assert_eq!(v, [11, 12, 13, 14]);
    assert_eq!(
        e.undo.len(),
        undo + 1,
        "mode switch and selection: one step"
    );
    // Minus in another mode edits that mode's list and stays put.
    e.scene.selection.faces = vec![0, 1];
    run(
        &mut e,
        "operandi.statistics.select row:polygons.vertices:4 action:remove",
    );
    assert_eq!(e.scene.selection.mode, Mode::Vertex);
    assert_eq!(e.scene.selection.faces, [1]);
}

#[test]
fn renaming_a_part_is_one_undo_step() {
    // 6.
    let mut e = editor(acceptance_mesh(), Mode::Polygon);
    let undo = e.undo.len();
    run(
        &mut e,
        "operandi.statistics.rename row:polygons.part:Wheel name:{Front Wheel}",
    );
    let part = |e: &Editor| e.scene.active().unwrap().mesh.faces[0].part.clone();
    assert_eq!(part(&e).as_deref(), Some("Front Wheel"));
    assert_eq!(e.undo.len(), undo + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(part(&e).as_deref(), Some("Wheel"));
    // Delete removes the tag; the polygon stays.
    run(&mut e, "operandi.statistics.delete row:polygons.part:Wheel");
    assert_eq!(part(&e), None);
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 4);
}

#[test]
fn hidden_elements_are_not_counted_and_row_hide_unhide_round_trips() {
    let mut e = editor(acceptance_mesh(), Mode::Polygon);
    run(&mut e, "operandi.statistics.hide row:polygons.vertices:3");
    let r = report(&e);
    assert_eq!(count(&r, Row::All(Kind::Polygon)), 2);
    assert_eq!(count(&r, Row::PolygonVertices(3)), 0);
    // Points only the triangles used (4 and 5) are hidden with them.
    assert_eq!(count(&r, Row::All(Kind::Vertex)), 13);
    run(
        &mut e,
        "operandi.statistics.hide row:polygons.vertices:3 action:unhide",
    );
    assert_eq!(count(&report(&e), Row::All(Kind::Polygon)), 4);
}

#[test]
fn checks_find_bent_concave_sliver_and_overlapping_polygons() {
    let mut m = Mesh::default();
    m.vertices = vec![
        // 0..4 a bent quad
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(1., 1., 0.3),
        DVec3::new(0., 1., 0.),
        // 4..8 an arrow (concave at 7)
        DVec3::new(3., 0., 0.),
        DVec3::new(5., 0., 0.),
        DVec3::new(5., 2., 0.),
        DVec3::new(4.5, 0.5, 0.),
        // 8..11 a sliver triangle
        DVec3::new(6., 0., 0.),
        DVec3::new(9., 0., 0.),
        DVec3::new(6., 0.01, 0.),
        // 11..14 the same triangle again on other points
        DVec3::new(6., 0., 0.),
        DVec3::new(9., 0., 0.),
        DVec3::new(6., 0.01, 0.),
    ];
    m.faces = vec![
        face(vec![0, 1, 2, 3]),
        face(vec![4, 5, 6, 7]),
        face(vec![8, 9, 10]),
        face(vec![13, 12, 11]),
    ]
    .into();
    let e = editor(m, Mode::Polygon);
    let r = report(&e);
    assert_eq!(count(&r, Row::PolygonCheck(Check::NotFlat)), 1);
    assert_eq!(count(&r, Row::PolygonCheck(Check::Concave)), 1);
    assert_eq!(count(&r, Row::PolygonCheck(Check::SharpAngle)), 2);
    assert_eq!(count(&r, Row::PolygonCheck(Check::Overlapping)), 2);
    // Collapsed groups count only their top rows; expensive checks wait
    // for the Checks group.
    let top = statistics::compute(&statistics::snapshot(&e.scene, Need::TOP));
    assert_eq!(count(&top, Row::All(Kind::Polygon)), 4);
    assert!(top.get(&Row::PolygonVertices(3)).is_none());
    let no_checks = statistics::compute(&statistics::snapshot(
        &e.scene,
        Need {
            checks: false,
            ..Need::ALL
        },
    ));
    assert!(no_checks.get(&Row::PolygonCheck(Check::NotFlat)).is_none());
    assert_eq!(
        no_checks.get(&Row::PolygonCheck(Check::SinglePoint)),
        Some(Tally::default())
    );
}

#[test]
fn count_query_answers_and_changes_nothing() {
    let mut e = editor(acceptance_mesh(), Mode::Polygon);
    let undo = e.undo.len();
    run(&mut e, "operandi.statistics.count row:edges.polygons:1");
    assert_eq!(e.command_note.as_deref(), Some("11 (0 selected)"));
    assert_eq!(e.undo.len(), undo);
}

#[test]
fn items_rows_select_items_by_type() {
    let mut e = editor(acceptance_mesh(), Mode::Polygon);
    e.scene.new_mesh().unwrap();
    let r = report(&e);
    assert_eq!(count(&r, Row::All(Kind::Item)), 2);
    assert_eq!(r.names.item_types, [statistics::ItemType::Mesh]);
    run(
        &mut e,
        "operandi.statistics.select row:items.type:mesh action:only",
    );
    assert_eq!(e.scene.selection.mode, Mode::Item);
    assert_eq!(e.scene.selected_meshes().len(), 2);
    let r = report(&e);
    assert_eq!(r.get(&Row::All(Kind::Item)).unwrap().selected, 2);
    // Two active meshes: totals across both.
    assert_eq!(
        count(&r, Row::All(Kind::Polygon)),
        4 + e.scene.items[1].mesh.faces.len() as u64
    );
}

/// 5M polygons (spec §9 target: a full refresh under 300 ms). Run with
/// `cargo test --release --locked --workspace -- statistics::five_million --ignored --nocapture`.
#[test]
#[ignore = "probe: about 5M polygons; run in release"]
fn five_million_polygon_refresh() {
    let n = 2240usize; // 2240^2 ≈ 5.0M quads
    let mut m = Mesh::default();
    m.vertices = (0..=n)
        .flat_map(|y| (0..=n).map(move |x| DVec3::new(x as f64, y as f64, 0.)))
        .collect();
    let w = (n + 1) as u32;
    m.faces = (0..n as u32)
        .flat_map(|y| {
            (0..n as u32).map(move |x| {
                let a = y * w + x;
                face(vec![a, a + 1, a + w + 1, a + w])
            })
        })
        .collect::<Vec<_>>()
        .into();
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.scene.selection.faces = (0..1000).collect();
    let input = statistics::snapshot(&e.scene, Need::ALL);
    let t = std::time::Instant::now();
    let first = statistics::compute(&input);
    let cold = t.elapsed();
    // A drag frame: the points move, the polygon list stays.
    e.scene.active_mut().unwrap().mesh.vertices[5].z = 0.5;
    let input = statistics::snapshot(&e.scene, Need::ALL);
    let t = std::time::Instant::now();
    let warm = statistics::compute(&input);
    let all = t.elapsed();
    let input = statistics::snapshot(
        &e.scene,
        Need {
            checks: false,
            ..Need::ALL
        },
    );
    let t = std::time::Instant::now();
    statistics::compute(&input);
    let no_checks = t.elapsed();
    let input = statistics::snapshot(&e.scene, Need::TOP);
    let t = std::time::Instant::now();
    statistics::compute(&input);
    let top = t.elapsed();
    println!(
        "5M quads: first refresh (builds topology) {cold:?}; after a point move: everything open {all:?}, checks closed {no_checks:?}, all collapsed {top:?}"
    );
    assert_eq!(count(&first, Row::All(Kind::Polygon)), (n * n) as u64);
    assert_eq!(count(&warm, Row::PolygonCheck(Check::NotFlat)), 2);
    assert_eq!(warm.get(&Row::All(Kind::Polygon)).unwrap().selected, 1000);
}
