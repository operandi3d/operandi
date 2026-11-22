//! The UV tab's one-shot buttons (uv_ops.rs; BEHAVIOR-UV-TOOLS §7.5–7.9,
//! §9, §11, §12; acceptance UVT-A05..A09). Each is one undo step and
//! never moves points (UVT-06/07).
use glam::{DVec2, DVec3};
use operandi_core::uv_ops::*;
use operandi_core::*;

fn face(v: &[u32]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn p(u: f64, v: f64) -> Option<DVec2> {
    Some(DVec2::new(u, v))
}

/// Two 1 m quads in XZ sharing edge 1-4; `uvs` gives each quad's corners.
fn strip(uvs: [[(f64, f64); 4]; 2]) -> Mesh {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(0., 0., 1.),
            DVec3::new(1., 0., 1.),
            DVec3::new(2., 0., 1.),
        ],
        faces: vec![face(&[0, 1, 4, 3]), face(&[1, 2, 5, 4])].into(),
        ..Default::default()
    };
    m.uv_maps.insert(
        "Texture".into(),
        uvs.iter()
            .map(|q| q.iter().map(|&(u, v)| p(u, v)).collect())
            .collect(),
    );
    m.attributes.current_uv_map = Some("Texture".into());
    m
}

/// Two separate islands: left at U 0.1..0.2, right at U 0.3..0.4.
fn two_islands() -> Mesh {
    strip([
        [(0.1, 0.), (0.2, 0.), (0.2, 0.1), (0.1, 0.1)],
        [(0.3, 0.2), (0.4, 0.2), (0.4, 0.3), (0.3, 0.3)],
    ])
}

fn editor_with(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}

fn rows(e: &Editor) -> Vec<Vec<Option<DVec2>>> {
    e.scene.active().unwrap().mesh.uv_maps["Texture"].clone()
}

fn uv(e: &Editor, f: usize, c: usize) -> DVec2 {
    rows(e)[f][c].unwrap()
}

fn near(a: DVec2, b: DVec2) -> bool {
    (a - b).length() < 1e-9
}

fn run(e: &mut Editor, op: UvOp) {
    let points = e.scene.active().unwrap().mesh.vertices.clone();
    e.dispatch(Command::UvOp(op)).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices, points, "UVT-06");
}

#[test]
fn uvt_a05_island_align_min_lines_up_left_sides_and_keeps_v() {
    let mut e = editor_with(two_islands(), Mode::Polygon);
    run(
        &mut e,
        UvOp::IslandAlign {
            u: uv_ops::Side::Min,
            v: uv_ops::Side::Unchanged,
        },
    );
    assert!(near(uv(&e, 0, 0), DVec2::new(0.1, 0.)));
    assert!(near(uv(&e, 1, 0), DVec2::new(0.1, 0.2)), "{:?}", rows(&e));
    e.dispatch(Command::Undo).unwrap();
    assert!(near(uv(&e, 1, 0), DVec2::new(0.3, 0.2)), "one undo step");
}

#[test]
fn uvt_a06_sew_average_joins_the_seam_halfway() {
    // The shared edge 1-4: left side at U 0.40, right side at U 0.44.
    let m = strip([
        [(0.3, 0.), (0.4, 0.), (0.4, 0.1), (0.3, 0.1)],
        [(0.44, 0.), (0.54, 0.), (0.54, 0.1), (0.44, 0.1)],
    ]);
    let mut e = editor_with(m, Mode::Edge);
    e.scene.selection.edges = vec![(1, 4)];
    run(
        &mut e,
        UvOp::Sew {
            connect: ConnectTo::Average,
            selected_only: false,
        },
    );
    assert!(near(uv(&e, 0, 1), DVec2::new(0.42, 0.)), "{:?}", rows(&e));
    assert!(near(uv(&e, 1, 0), uv(&e, 0, 1)), "one UV position");
    assert!(near(uv(&e, 1, 3), uv(&e, 0, 2)));
    // The far sides stay.
    assert!(near(uv(&e, 1, 1), DVec2::new(0.54, 0.)));
}

#[test]
fn uvt_a07_rotate_polygons_shifts_each_corner_along_one() {
    let m = strip([
        [(0., 0.), (1., 0.), (1., 1.), (0., 1.)],
        [(2., 0.), (3., 0.), (3., 1.), (2., 1.)],
    ]);
    let mut e = editor_with(m, Mode::Polygon);
    e.scene.selection.faces = vec![0];
    run(&mut e, UvOp::RotatePolygons);
    let r = rows(&e);
    assert_eq!(r[0], vec![p(1., 0.), p(1., 1.), p(0., 1.), p(0., 0.)]);
    assert_eq!(r[1][0], p(2., 0.), "the unselected polygon stays");
}

#[test]
fn uvt_a08_texel_density_is_image_size_times_root_area_ratio() {
    // One 1 m quad with UV area 0.25: 1024 × sqrt(0.25) = 512 px/m.
    let m = strip([
        [(0., 0.), (0.5, 0.), (0.5, 0.5), (0., 0.5)],
        [(0.6, 0.), (1.1, 0.), (1.1, 0.5), (0.6, 0.5)],
    ]);
    let rows0: Vec<Vec<Option<DVec2>>> = m.uv_maps["Texture"].clone();
    let d = density(&m, &rows0, &[0], 1024., glam::DMat4::IDENTITY);
    assert!((d - 512.).abs() < 1e-9, "{d}");
    // Apply 1024: the UV area becomes 1.0 (each island scaled about its
    // own centre, UVT-58).
    let mut e = editor_with(m, Mode::Polygon);
    e.scene.selection.faces = vec![0];
    run(
        &mut e,
        UvOp::Texel(
            TexelAction::Individually,
            TexelOptions {
                density: 1024.,
                ..Default::default()
            },
        ),
    );
    let r = rows(&e);
    let q: Vec<DVec2> = r[0].iter().flatten().copied().collect();
    let area = (0..4).map(|i| q[i].perp_dot(q[(i + 1) % 4])).sum::<f64>() / 2.;
    assert!((area - 1.).abs() < 1e-9, "{area}");
    assert!(
        near((q[0] + q[2]) * 0.5, DVec2::splat(0.25)),
        "about its centre"
    );
    assert_eq!(r[1], rows0[1], "the other island stays");
}

#[test]
fn uvt_a09_select_unmapped_after_delete_finds_that_face_only() {
    let mut e = editor_with(two_islands(), Mode::Polygon);
    e.scene.selection.faces = vec![1];
    run(&mut e, UvOp::Delete);
    assert!(rows(&e)[1].iter().all(Option::is_none));
    e.scene.selection.faces.clear();
    run(&mut e, UvOp::SelectUnmapped { invert: false });
    assert_eq!(e.scene.selection.faces, vec![1]);
    run(&mut e, UvOp::SelectUnmapped { invert: true });
    assert_eq!(e.scene.selection.faces, vec![0]);
}

#[test]
fn quick_align_turns_the_whole_island_so_the_edge_lies_along_u() {
    // One island, its bottom edge 0-1 tilted by 10°.
    let a = 10f64.to_radians();
    let r = |x: f64, y: f64| (x * a.cos() - y * a.sin(), x * a.sin() + y * a.cos());
    let m = strip([
        [r(0., 0.), r(0.1, 0.), r(0.1, 0.1), r(0., 0.1)],
        [r(0.1, 0.), r(0.2, 0.), r(0.2, 0.1), r(0.1, 0.1)],
    ]);
    let mut e = editor_with(m, Mode::Edge);
    e.scene.selection.edges = vec![(0, 1)];
    run(&mut e, UvOp::QuickAlign(AxisChoice::Auto));
    let d = uv(&e, 0, 1) - uv(&e, 0, 0);
    assert!(d.y.abs() < 1e-9, "{d}");
    // The other polygon of the island turned with it.
    let d = uv(&e, 1, 1) - uv(&e, 1, 0);
    assert!(d.y.abs() < 1e-9, "{d}");
}

#[test]
fn orient_lays_a_long_island_along_u_and_to_origin_moves_it_home() {
    let m = strip([
        [(0.5, 0.5), (0.5, 0.6), (0.4, 0.6), (0.4, 0.5)],
        [(0.5, 0.6), (0.5, 0.7), (0.4, 0.7), (0.4, 0.6)],
    ]);
    let mut e = editor_with(m, Mode::Polygon);
    run(&mut e, UvOp::Orient(AxisChoice::Horizontal));
    let all: Vec<DVec2> = rows(&e).into_iter().flatten().flatten().collect();
    let lo = all.iter().fold(DVec2::MAX, |a, &b| a.min(b));
    let hi = all.iter().fold(DVec2::MIN, |a, &b| a.max(b));
    assert!((hi - lo).x > (hi - lo).y, "{lo} {hi}");
    run(&mut e, UvOp::ToOrigin { udim: 1002 });
    let all: Vec<DVec2> = rows(&e).into_iter().flatten().flatten().collect();
    let lo = all.iter().fold(DVec2::MAX, |a, &b| a.min(b));
    assert!(near(lo, DVec2::new(1., 0.)), "tile 1002's corner: {lo}");
}

#[test]
fn split_pulls_a_shared_edge_apart_into_two_islands() {
    let m = strip([
        [(0., 0.), (0.5, 0.), (0.5, 0.5), (0., 0.5)],
        [(0.5, 0.), (1., 0.), (1., 0.5), (0.5, 0.5)],
    ]);
    let mut e = editor_with(m, Mode::Edge);
    e.scene.selection.edges = vec![(1, 4)];
    run(
        &mut e,
        UvOp::Split {
            distance: SPLIT_LARGE,
        },
    );
    let gap = uv(&e, 1, 0).x - uv(&e, 0, 1).x;
    assert!((gap - SPLIT_LARGE).abs() < 1e-9, "{gap}");
    // A plain split is invisible but makes them two positions.
    let m = strip([
        [(0., 0.), (0.5, 0.), (0.5, 0.5), (0., 0.5)],
        [(0.5, 0.), (1., 0.), (1., 0.5), (0.5, 0.5)],
    ]);
    let mut e = editor_with(m, Mode::Edge);
    e.scene.selection.edges = vec![(1, 4)];
    run(&mut e, UvOp::Split { distance: 0. });
    let gap = uv(&e, 1, 0).x - uv(&e, 0, 1).x;
    assert!(gap > 1e-6 && gap < 1e-4, "{gap}");
}

#[test]
fn move_and_sew_brings_the_other_island_to_the_seam() {
    // The right quad's island sits 0.3 up and turned; Connect to Selected
    // brings it to the left one.
    let m = strip([
        [(0., 0.), (0.5, 0.), (0.5, 0.5), (0., 0.5)],
        [(0.5, 0.8), (1., 0.8), (1., 1.3), (0.5, 1.3)],
    ]);
    let mut e = editor_with(m, Mode::Polygon);
    e.scene.selection.faces = vec![0];
    run(
        &mut e,
        UvOp::MoveSew {
            connect: ConnectTo::Selected,
            scale: true,
        },
    );
    assert!(near(uv(&e, 1, 0), DVec2::new(0.5, 0.)), "{:?}", rows(&e));
    assert!(near(uv(&e, 1, 3), DVec2::new(0.5, 0.5)));
    assert!(near(uv(&e, 1, 1), DVec2::new(1., 0.)));
    assert!(near(uv(&e, 0, 0), DVec2::ZERO), "the selected island stays");
}

#[test]
fn select_border_picks_every_island_outline() {
    let mut e = editor_with(two_islands(), Mode::Polygon);
    run(&mut e, UvOp::SelectBorder);
    assert_eq!(e.scene.selection.mode, Mode::Edge);
    // Two separate quads: all 7 mesh edges are on a UV outline.
    assert_eq!(e.scene.selection.edges.len(), 7);
    let joined = strip([
        [(0., 0.), (0.5, 0.), (0.5, 0.5), (0., 0.5)],
        [(0.5, 0.), (1., 0.), (1., 0.5), (0.5, 0.5)],
    ]);
    let mut e = editor_with(joined, Mode::Polygon);
    run(&mut e, UvOp::SelectBorder);
    assert_eq!(e.scene.selection.edges.len(), 6, "the shared edge isn't");
}

#[test]
fn select_error_uvs_finds_flipped_stacked_and_overlapping() {
    let flipped = strip([
        [(0., 0.), (0., 0.5), (0.5, 0.5), (0.5, 0.)],
        [(0.6, 0.), (1., 0.), (1., 0.4), (0.6, 0.4)],
    ]);
    let mut e = editor_with(flipped, Mode::Polygon);
    run(
        &mut e,
        UvOp::SelectErrors(ErrorTests {
            cross: false,
            stacked: false,
            ..Default::default()
        }),
    );
    assert_eq!(e.scene.selection.faces, vec![0]);
    let overlapping = strip([
        [(0., 0.), (0.5, 0.), (0.5, 0.5), (0., 0.5)],
        [(0.25, 0.25), (0.75, 0.25), (0.75, 0.75), (0.25, 0.75)],
    ]);
    let mut e = editor_with(overlapping, Mode::Polygon);
    run(&mut e, UvOp::SelectErrors(ErrorTests::default()));
    assert_eq!(e.scene.selection.faces, vec![0, 1]);
}

#[test]
fn copy_and_paste_by_selection_and_by_connectivity() {
    let mut e = editor_with(two_islands(), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    e.dispatch(Command::UvOp(UvOp::Copy)).unwrap();
    e.scene.selection.faces = vec![1];
    run(&mut e, UvOp::Paste(PasteMode::Selection));
    assert_eq!(rows(&e)[1], rows(&e)[0]);
    // Connectivity: copy both, paste onto both from the other end; the
    // walk lines the corners up by the shared edge.
    let mut e = editor_with(two_islands(), Mode::Polygon);
    let before = rows(&e);
    e.scene.selection.faces = vec![0, 1];
    e.dispatch(Command::UvOp(UvOp::Copy)).unwrap();
    run(&mut e, UvOp::Cut);
    assert!(
        rows(&e).iter().flatten().all(Option::is_none),
        "Cut empties them"
    );
    run(&mut e, UvOp::Paste(PasteMode::Connectivity));
    assert_eq!(rows(&e), before);
}

#[test]
fn set_value_writes_u_or_v_of_the_selected_points() {
    let mut e = editor_with(two_islands(), Mode::Vertex);
    e.scene.selection.vertices = vec![0];
    run(
        &mut e,
        UvOp::SetValue {
            u: Some(0.75),
            v: None,
        },
    );
    assert!(near(uv(&e, 0, 0), DVec2::new(0.75, 0.)));
    assert!(near(uv(&e, 0, 1), DVec2::new(0.2, 0.)), "others stay");
}

#[test]
fn merge_uv_maps_fills_the_first_map_and_drops_the_rest() {
    let mut m = two_islands();
    let mut first = m.uv_maps["Texture"].clone();
    first[1] = vec![None; 4];
    m.uv_maps.insert("Texture".into(), first);
    let mut other = m.uv_maps["Texture"].clone();
    other[0] = vec![p(9., 9.); 4];
    other[1] = vec![p(5., 5.); 4];
    m.uv_maps.insert("Other".into(), other);
    let mut e = editor_with(m, Mode::Polygon);
    run(
        &mut e,
        UvOp::MergeMaps(vec!["Texture".into(), "Other".into()]),
    );
    let mesh = &e.scene.active().unwrap().mesh;
    assert!(!mesh.uv_maps.contains_key("Other"));
    let r = &mesh.uv_maps["Texture"];
    assert_eq!(r[0][0], p(0.1, 0.), "the first map wins where it has UVs");
    assert_eq!(r[1][0], p(5., 5.));
}

#[test]
fn real_world_size_makes_one_tile_cover_the_given_length() {
    let m = strip([
        [(0., 0.), (0.25, 0.), (0.25, 0.25), (0., 0.25)],
        [(0.25, 0.), (0.5, 0.), (0.5, 0.25), (0.25, 0.25)],
    ]);
    let mut e = editor_with(m, Mode::Polygon);
    run(
        &mut e,
        UvOp::RealWorldSize {
            metres: 2.,
            world_space: true,
        },
    );
    // 2 m of mesh (the strip's length) now spans 1 UV.
    let span = uv(&e, 1, 1).x - uv(&e, 0, 0).x;
    assert!((span - 1.).abs() < 1e-9, "{span}");
}
