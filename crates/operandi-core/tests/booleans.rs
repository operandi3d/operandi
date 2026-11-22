//! Boolean (booleans/BOOLEANS.md stage 1, BOOL-01..25). Reference scene:
//! target cube A from -1 to 1 (volume 8), driver cube B from 0 to 2.
use glam::DVec3;
use operandi_core::*;
use std::collections::HashMap;

/// A cube from `lo` to `hi` on every axis, all polygons in `material`.
fn cube_mesh(lo: DVec3, hi: DVec3, material: &str) -> Mesh {
    let mut m = Mesh::box_mesh((lo + hi) * 0.5, hi - lo, 0).unwrap();
    for f in &mut m.faces {
        f.material = material.into();
    }
    m
}

/// An editor with one mesh item per cube; the first is selected (the target).
fn scene(cubes: &[(DVec3, DVec3)]) -> (Editor, Vec<u64>) {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for (i, &(lo, hi)) in cubes.iter().enumerate() {
        e.dispatch(Command::NewMesh).unwrap();
        let item = e.scene.active_mut().unwrap();
        item.mesh = cube_mesh(lo, hi, if i == 0 { "A" } else { "B" }).into();
        ids.push(item.id);
    }
    e.dispatch(Command::SelectItem(Some(ids[0]))).unwrap();
    e.synchronize_identities().unwrap();
    e.undo.clear();
    (e, ids)
}

fn reference() -> (Editor, Vec<u64>) {
    scene(&[
        (DVec3::splat(-1.), DVec3::ONE),
        (DVec3::ZERO, DVec3::splat(2.)),
    ])
}

fn mesh(e: &Editor, id: u64) -> &Mesh {
    &e.scene.items.iter().find(|i| i.id == id).unwrap().mesh
}

/// Signed volume of a polygon mesh (fan per polygon; exact for flat ones).
fn volume(m: &Mesh) -> f64 {
    let mut v = 0.;
    for f in &m.faces {
        let p0 = m.vertices[f.vertices[0] as usize];
        for i in 1..f.vertices.len() - 1 {
            let a = m.vertices[f.vertices[i] as usize];
            let b = m.vertices[f.vertices[i + 1] as usize];
            v += p0.dot(a.cross(b));
        }
    }
    v / 6.
}

/// Polygons on each edge, as a histogram: uses → edges.
fn edge_uses(m: &Mesh) -> HashMap<usize, usize> {
    let mut uses: HashMap<(u32, u32), usize> = HashMap::new();
    for f in &m.faces {
        for i in 0..f.vertices.len() {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % f.vertices.len()]);
            *uses.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    let mut out = HashMap::new();
    for n in uses.values() {
        *out.entry(*n).or_default() += 1;
    }
    out
}

fn watertight(m: &Mesh) -> bool {
    edge_uses(m).keys().all(|&n| n == 2)
}

fn run(e: &mut Editor, op: BooleanOp) -> Result<()> {
    e.dispatch(Command::Boolean(BooleanOptions {
        op,
        ..Default::default()
    }))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn reference_operations_match_the_spec() {
    // BOOL-01..03: polygon counts and volumes; BOOL-08 driver untouched.
    for (op, polygons, vol) in [
        (BooleanOp::Union, 12, 15.),
        (BooleanOp::Intersect, 6, 1.),
        (BooleanOp::Subtract, 9, 7.),
    ] {
        let (mut e, ids) = reference();
        let driver_before = e.scene.items[1].clone();
        run(&mut e, op).unwrap();
        let m = mesh(&e, ids[0]);
        assert_eq!(m.faces.len(), polygons, "{op:?}");
        assert!(close(volume(m), vol), "{op:?} volume {}", volume(m));
        assert!(watertight(m), "{op:?}");
        assert_eq!(e.scene.items[1], driver_before, "{op:?} driver changed");
        assert_eq!(e.scene.items.len(), 2);
        assert!(e.scene.selection.faces.is_empty(), "BOOL-14");
        assert_eq!(e.boolean_preset.op, op);
    }
}

#[test]
fn union_keeps_untouched_quads_and_makes_l_shaped_ngons() {
    // BOOL-09: three faces of A become 6-sided L shapes, nothing triangulated.
    let (mut e, ids) = reference();
    run(&mut e, BooleanOp::Union).unwrap();
    let m = mesh(&e, ids[0]);
    let mut sides: Vec<usize> = m.faces.iter().map(|f| f.vertices.len()).collect();
    sides.sort_unstable();
    assert_eq!(sides, [4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6]);
}

#[test]
fn intersect_takes_faces_from_both_and_subtract_caps_take_the_driver_material() {
    // BOOL-02, BOOL-12.
    let (mut e, ids) = reference();
    run(&mut e, BooleanOp::Intersect).unwrap();
    let count = |m: &Mesh, mat: &str| m.faces.iter().filter(|f| f.material == mat).count();
    assert_eq!(count(mesh(&e, ids[0]), "A"), 3);
    assert_eq!(count(mesh(&e, ids[0]), "B"), 3);
    let (mut e, ids) = reference();
    run(&mut e, BooleanOp::Subtract).unwrap();
    assert_eq!(count(mesh(&e, ids[0]), "B"), 3, "cap faces");
    assert_eq!(count(mesh(&e, ids[0]), "A"), 6);
}

#[test]
fn add_keeps_both_shells_stitched_along_the_crossing() {
    // BOOL-04: 18 polygons, volume 16, the crossing edges used four times.
    let (mut e, ids) = reference();
    run(&mut e, BooleanOp::Add).unwrap();
    let m = mesh(&e, ids[0]);
    assert_eq!(m.faces.len(), 18);
    assert!(close(volume(m), 16.), "{}", volume(m));
    let uses = edge_uses(m);
    assert_eq!(uses.get(&4), Some(&6), "{uses:?}");
    assert!(uses.keys().all(|&n| n == 2 || n == 4));
    m.validate().unwrap();
}

#[test]
fn undo_and_redo_are_one_step() {
    // BOOL-24.
    let (mut e, ids) = reference();
    let before = e.scene.clone();
    run(&mut e, BooleanOp::Subtract).unwrap();
    let after = e.scene.clone();
    assert_eq!(e.undo.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items, before.items);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(mesh(&e, ids[0]), mesh(&after_editor(after), ids[0]));
}

fn after_editor(scene: Scene) -> Editor {
    Editor {
        scene,
        ..Default::default()
    }
}

#[test]
fn coplanar_and_touching_cases_are_clean() {
    // BOOL-10/11.
    let (mut e, ids) = scene(&[
        (DVec3::splat(-1.), DVec3::ONE),
        (DVec3::new(1., -1., -1.), DVec3::new(3., 1., 1.)),
    ]);
    run(&mut e, BooleanOp::Union).unwrap();
    let m = mesh(&e, ids[0]);
    assert_eq!(m.faces.len(), 10, "touching union");
    assert!(close(volume(m), 16.));
    assert!(watertight(m));

    let (mut e, ids) = scene(&[
        (DVec3::splat(-1.), DVec3::ONE),
        (DVec3::new(0., -1., -1.), DVec3::new(2., 2., 2.)),
    ]);
    run(&mut e, BooleanOp::Subtract).unwrap();
    let m = mesh(&e, ids[0]);
    assert_eq!(m.faces.len(), 6, "coplanar subtract");
    assert!(close(volume(m), 4.));
    assert!(m.faces.iter().all(|f| f.vertices.len() == 4));
}

#[test]
fn driver_inside_the_target() {
    // BOOL-16.
    let inside = || {
        scene(&[
            (DVec3::splat(-1.), DVec3::ONE),
            (DVec3::splat(-0.5), DVec3::splat(0.5)),
        ])
    };
    let (mut e, ids) = inside();
    run(&mut e, BooleanOp::Subtract).unwrap();
    let m = mesh(&e, ids[0]);
    assert_eq!(m.faces.len(), 12);
    assert!(close(volume(m), 7.));
    assert!(watertight(m));

    let (mut e, ids) = inside();
    let before = mesh(&e, ids[0]).clone();
    run(&mut e, BooleanOp::Union).unwrap();
    assert_eq!(mesh(&e, ids[0]).faces, before.faces, "A unchanged");
    assert_eq!(mesh(&e, ids[0]).vertices, before.vertices);

    let (mut e, ids) = inside();
    run(&mut e, BooleanOp::Intersect).unwrap();
    let m = mesh(&e, ids[0]);
    assert_eq!(m.faces.len(), 6);
    assert!(close(volume(m), 1.));
    assert!(m.faces.iter().all(|f| f.material == "B"));
}

#[test]
fn no_overlap() {
    // BOOL-17.
    let apart = || {
        scene(&[
            (DVec3::splat(-1.), DVec3::ONE),
            (DVec3::splat(3.), DVec3::splat(5.)),
        ])
    };
    let (mut e, ids) = apart();
    let before = mesh(&e, ids[0]).clone();
    run(&mut e, BooleanOp::Subtract).unwrap();
    assert_eq!(mesh(&e, ids[0]).faces, before.faces);

    let (mut e, ids) = apart();
    run(&mut e, BooleanOp::Union).unwrap();
    assert_eq!(mesh(&e, ids[0]).faces.len(), 12);
    assert!(close(volume(mesh(&e, ids[0])), 16.));

    let (mut e, ids) = apart();
    run(&mut e, BooleanOp::Intersect).unwrap();
    assert!(mesh(&e, ids[0]).faces.is_empty());
    assert!(mesh(&e, ids[0]).vertices.is_empty());
    assert!(e.boolean_note.as_deref().unwrap().contains("don't overlap"));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e, ids[0]).faces.len(), 6);
}

#[test]
fn no_driver_changes_nothing_and_adds_no_undo_step() {
    // BOOL-18: the driver is hidden, so nothing qualifies.
    let (mut e, ids) = reference();
    e.scene.items[1].visible = false;
    let before = e.scene.clone();
    let err = run(&mut e, BooleanOp::Union).unwrap_err();
    assert!(err.contains("no driver"), "{err}");
    assert_eq!(e.scene, before);
    assert!(e.undo.is_empty());
    let _ = ids;
}

#[test]
fn open_input_is_refused_with_its_problem_edges() {
    // BOOL-19: the target without its top face.
    let (mut e, ids) = reference();
    let item = e.scene.active_mut().unwrap();
    let mut m: Mesh = (*item.mesh).clone();
    m.faces.remove(1);
    item.mesh = m.into();
    item.name = "Open Cube".into();
    let before = e.scene.clone();
    let err = run(&mut e, BooleanOp::Subtract).unwrap_err();
    assert!(err.contains("Open Cube has 4 open edges"), "{err}");
    assert_eq!(e.scene, before);
    let problem = e
        .boolean_problem(&BooleanOptions {
            op: BooleanOp::Subtract,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(problem.item, ids[0]);
    assert_eq!(problem.edges.len(), 4);
}

#[test]
fn self_intersecting_driver_is_refused() {
    // BOOL-21: two overlapping cubes in one driver item.
    let (mut e, ids) = reference();
    let item = e.scene.items.iter_mut().find(|i| i.id == ids[1]).unwrap();
    let mut m = cube_mesh(DVec3::ZERO, DVec3::splat(2.), "B");
    append_mesh(&mut m, cube_mesh(DVec3::splat(1.), DVec3::splat(3.), "B")).unwrap();
    item.mesh = m.into();
    let err = run(&mut e, BooleanOp::Union).unwrap_err();
    assert!(err.contains("passes through itself"), "{err}");
    assert_eq!(
        e.boolean_problem(&BooleanOptions::default()).unwrap().item,
        ids[1]
    );
}

#[test]
fn several_drivers_act_as_one() {
    // BOOL-07: two corner cubes subtracted at once.
    let (mut e, ids) = scene(&[
        (DVec3::splat(-1.), DVec3::ONE),
        (DVec3::ZERO, DVec3::splat(2.)),
        (DVec3::splat(-2.), DVec3::ZERO),
    ]);
    run(&mut e, BooleanOp::Subtract).unwrap();
    let m = mesh(&e, ids[0]);
    assert!(close(volume(m), 6.), "{}", volume(m));
    assert!(watertight(m));
}

#[test]
fn selected_items_are_not_drivers_and_a_specific_item_is() {
    // BOOL-06: Background skips selected meshes; Specific Item uses a hidden one.
    let (mut e, ids) = reference();
    e.dispatch(Command::ToggleItemSelection(ids[1])).unwrap();
    assert!(run(&mut e, BooleanOp::Union).is_err());
    let (mut e, ids) = reference();
    e.scene.items[1].visible = false;
    e.dispatch(Command::Boolean(BooleanOptions {
        op: BooleanOp::Subtract,
        drive: DriveMesh::Item,
        item: Some(ids[1]),
        children: false,
    }))
    .unwrap();
    assert!(close(volume(mesh(&e, ids[0])), 7.));
}

#[test]
fn last_selected_pieces_cut_the_rest() {
    // BOOL-06 Last Selected (Q1 reading): the selected piece is the driver.
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut m = cube_mesh(DVec3::splat(-1.), DVec3::ONE, "A");
    append_mesh(&mut m, cube_mesh(DVec3::ZERO, DVec3::splat(2.), "B")).unwrap();
    let id = e.scene.active().unwrap().id;
    e.scene.active_mut().unwrap().mesh = m.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![7];
    e.dispatch(Command::Boolean(BooleanOptions {
        op: BooleanOp::Subtract,
        drive: DriveMesh::LastSelected,
        ..Default::default()
    }))
    .unwrap();
    let m = mesh(&e, id);
    assert_eq!(m.faces.len(), 9);
    assert!(close(volume(m), 7.));
}

#[test]
fn moved_items_work_in_world_space() {
    // BOOL-08: the result lands in the target's own space.
    let (mut e, ids) = scene(&[
        (DVec3::splat(-1.), DVec3::ONE),
        (DVec3::splat(-1.), DVec3::ONE),
    ]);
    e.scene.items[0].transform.translation = DVec3::new(10., 0., 0.);
    e.scene.items[1].transform.translation = DVec3::new(11., 1., 1.);
    run(&mut e, BooleanOp::Subtract).unwrap();
    let m = mesh(&e, ids[0]);
    assert!(close(volume(m), 7.));
    // Target-local: the removed corner is at +x +y +z.
    assert!(
        m.vertices
            .iter()
            .all(|v| v.abs().max_element() <= 1. + 1e-9)
    );
}

#[test]
fn small_parts_far_from_the_origin() {
    // Recentring: a 1 cm boolean 100 km out.
    let far = DVec3::splat(1e5);
    let (mut e, ids) = scene(&[
        (far - DVec3::splat(0.01), far + DVec3::splat(0.01)),
        (far, far + DVec3::splat(0.02)),
    ]);
    run(&mut e, BooleanOp::Subtract).unwrap();
    let m = mesh(&e, ids[0]);
    assert_eq!(m.faces.len(), 9);
    assert!(watertight(m));
}

#[test]
fn uv_maps_follow_and_new_corners_interpolate() {
    // BOOL-13: an untouched corner keeps its UV; cut corners blend.
    let (mut e, ids) = reference();
    for id in &ids {
        let item = e.scene.items.iter_mut().find(|i| i.id == *id).unwrap();
        let mut m: Mesh = (*item.mesh).clone();
        let rows = m
            .faces
            .iter()
            .map(|f| {
                f.vertices
                    .iter()
                    .map(|&v| {
                        let p = m.vertices[v as usize];
                        Some(glam::DVec2::new(p.x, p.y))
                    })
                    .collect()
            })
            .collect();
        m.uv_maps.insert("Texture".into(), rows);
        item.mesh = m.into();
    }
    run(&mut e, BooleanOp::Union).unwrap();
    let m = mesh(&e, ids[0]);
    let rows = &m.uv_maps["Texture"];
    for (f, face) in m.faces.iter().enumerate() {
        for (c, &v) in face.vertices.iter().enumerate() {
            let p = m.vertices[v as usize];
            let uv = rows[f][c].expect("every corner has a UV");
            assert!((uv - glam::DVec2::new(p.x, p.y)).length() < 1e-9);
        }
    }
}

#[test]
fn command_line_parses_numbers_and_words() {
    assert_eq!(BooleanOp::parse("1").unwrap(), BooleanOp::Intersect);
    assert_eq!(BooleanOp::parse("Subtract").unwrap(), BooleanOp::Subtract);
    assert_eq!(
        DriveMesh::parse("background").unwrap(),
        DriveMesh::Background
    );
}

/// A cube from -s to s with `n` x `n` quads on each side, welded.
fn dense_cube(n: u32, s: f64, offset: DVec3) -> Mesh {
    let mut m = Mesh::default();
    let mut index = HashMap::new();
    let mut point = |p: DVec3, m: &mut Mesh| -> u32 {
        let k = (p * 1e6).round().as_i64vec3().to_array();
        *index.entry(k).or_insert_with(|| {
            m.vertices.push(p + offset);
            m.vertices.len() as u32 - 1
        })
    };
    for axis in 0..3 {
        for side in [-1., 1.] {
            let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
            for i in 0..n {
                for j in 0..n {
                    let corner = |a: u32, b: u32| {
                        let mut p = DVec3::ZERO;
                        p[axis] = side * s;
                        p[u] = -s + 2. * s * a as f64 / n as f64;
                        p[v] = -s + 2. * s * b as f64 / n as f64;
                        p
                    };
                    let mut q = vec![
                        point(corner(i, j), &mut m),
                        point(corner(i + 1, j), &mut m),
                        point(corner(i + 1, j + 1), &mut m),
                        point(corner(i, j + 1), &mut m),
                    ];
                    if side < 0. {
                        q.reverse();
                    }
                    m.faces.push(Face {
                        vertices: q,
                        uv: None,
                        material: "A".into(),
                        smooth: false,
                        part: None,
                        material_label: None,
                    });
                }
            }
        }
    }
    m
}

/// Opt-in timing: `cargo test --release -p operandi-core --test all booleans::
/// -- --ignored --nocapture`.
#[test]
#[ignore]
fn dense_subtract_timing() {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for offset in [DVec3::ZERO, DVec3::splat(0.37)] {
        e.dispatch(Command::NewMesh).unwrap();
        let item = e.scene.active_mut().unwrap();
        item.mesh = dense_cube(52, 1., offset).into();
        ids.push(item.id);
    }
    e.dispatch(Command::SelectItem(Some(ids[0]))).unwrap();
    let polygons = mesh(&e, ids[0]).faces.len();
    let start = std::time::Instant::now();
    run(&mut e, BooleanOp::Subtract).unwrap();
    println!(
        "Subtract {polygons} + {polygons} polygons: {:.0} ms, {} result polygons",
        start.elapsed().as_secs_f64() * 1e3,
        mesh(&e, ids[0]).faces.len()
    );
    assert!(watertight(mesh(&e, ids[0])));
}
