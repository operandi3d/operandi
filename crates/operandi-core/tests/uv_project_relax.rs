//! The UV tab's live tools (BEHAVIOR-UV-TOOLS §2–4, UVS-1 §1, §1.5, §3):
//! Project (UVT-10..18, acceptance UVT-A01..A03), Project from View
//! (UVT-19), UV Unwrap as a tool, UV Relax and Unwrap & Relax. They write
//! UVs only (UVT-06) and are one undo step per session (UVT-07).
use glam::{DMat4, DVec2, DVec3};
use operandi_core::uv_project::{ProjectionKind, ViewFrame};
use operandi_core::uv_relax::{RelaxBoundary, RelaxMode};
use operandi_core::uv_tools::Axis3;
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

fn editor_with(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn uvs(e: &Editor, map: &str) -> Vec<Vec<DVec2>> {
    mesh(e).uv_maps[map]
        .iter()
        .map(|r| r.iter().map(|p| p.unwrap()).collect())
        .collect()
}

fn near(a: DVec2, b: DVec2) -> bool {
    (a - b).length() < 1e-9
}

/// Start `tool`, change its parameters with `edit`, preview, drop.
fn run(e: &mut Editor, tool: Tool, edit: impl FnOnce(&mut Parameters)) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    edit(&mut p);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
}

/// A flat grid in XZ, `nx` × `nz` quads of size `w` × `d`.
fn grid(nx: u32, nz: u32, w: f64, d: f64) -> Mesh {
    let mut m = Mesh::default();
    for j in 0..=nz {
        for i in 0..=nx {
            m.vertices.push(DVec3::new(
                w * i as f64 / nx as f64,
                0.,
                d * j as f64 / nz as f64,
            ));
        }
    }
    for j in 0..nz {
        for i in 0..nx {
            let a = j * (nx + 1) + i;
            m.faces.push(face(&[a, a + nx + 1, a + nx + 2, a + 1]));
        }
    }
    m
}

/// A 12-sided open tube about Y, 1 high.
fn tube() -> Mesh {
    let n = 12u32;
    let mut m = Mesh::default();
    for y in 0..2 {
        for i in 0..n {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            m.vertices.push(DVec3::new(a.cos(), y as f64, a.sin()));
        }
    }
    for i in 0..n {
        let j = (i + 1) % n;
        m.faces.push(face(&[i, j, j + n, i + n]));
    }
    m
}

#[test]
fn uvt_a01_planar_y_automatic_spans_the_tile_on_the_longer_side() {
    let mut e = editor_with(grid(2, 1, 2., 1.), Mode::Polygon);
    let points = mesh(&e).vertices.clone();
    run(&mut e, Tool::UvProject, |p| {
        p.uv_project.kind = ProjectionKind::Planar;
        p.uv_project.axis = Axis3::Y;
    });
    let all: Vec<DVec2> = uvs(&e, "Texture").into_iter().flatten().collect();
    let lo = all.iter().fold(DVec2::MAX, |a, &b| a.min(b));
    let hi = all.iter().fold(DVec2::MIN, |a, &b| a.max(b));
    assert!(near(lo, DVec2::ZERO), "{lo}");
    assert!(near(hi, DVec2::new(1., 0.5)), "aspect kept: {hi}");
    assert_eq!(mesh(&e).vertices, points, "UVT-06");
    assert_eq!(
        mesh(&e).attributes.current_uv_map.as_deref(),
        Some("Texture"),
        "UVT-01: the map is made and selected"
    );
}

#[test]
fn uvt_a02_barycentric_gives_every_cube_face_the_whole_tile() {
    let mut cube = primitive::segmented_cube(DVec3::ZERO, DVec3::splat(2.), [1; 3]).unwrap();
    cube.uv_maps.clear();
    let mut e = editor_with(cube, Mode::Polygon);
    run(&mut e, Tool::UvProject, |p| {
        p.uv_project.kind = ProjectionKind::Barycentric
    });
    for row in uvs(&e, "Texture") {
        let mut corners: Vec<(i64, i64)> = row
            .iter()
            .map(|p| ((p.x * 10.).round() as i64, (p.y * 10.).round() as i64))
            .collect();
        corners.sort();
        assert_eq!(corners, vec![(0, 0), (0, 10), (10, 0), (10, 10)]);
    }
}

#[test]
fn uvt_a03_cylindrical_y_goes_once_round_with_one_seam() {
    let mut e = editor_with(tube(), Mode::Polygon);
    run(&mut e, Tool::UvProject, |p| {
        p.uv_project.kind = ProjectionKind::Cylindrical;
        p.uv_project.axis = Axis3::Y;
    });
    let rows = uvs(&e, "Texture");
    let mut widest: f64 = 0.;
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for row in &rows {
        let (a, b) = row
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
        widest = widest.max(b - a);
        lo = lo.min(a);
        hi = hi.max(b);
    }
    assert!(
        widest <= 1. / 12. + 1e-9,
        "no polygon spans the seam: {widest}"
    );
    assert!(
        lo >= -1e-9 && (hi - lo - 1.).abs() < 1e-9,
        "once round: {lo}..{hi}"
    );
    // V runs along the axis: 0 at the bottom ring, 1 at the top.
    for row in &rows {
        for (c, p) in row.iter().enumerate() {
            let want = if c < 2 { 0. } else { 1. };
            assert!((p.y - want).abs() < 1e-9, "{row:?}");
        }
    }
}

#[test]
fn uvt10_only_the_selected_polygons_get_new_uvs_and_undo_restores() {
    let mut m = grid(2, 1, 2., 1.);
    let old = vec![
        vec![Some(DVec2::new(0.1, 0.1)); 4],
        vec![Some(DVec2::new(0.9, 0.9)); 4],
    ];
    m.uv_maps.insert("Texture".into(), old.clone());
    let mut e = editor_with(m, Mode::Polygon);
    e.scene.selection.faces = vec![0];
    run(&mut e, Tool::UvProject, |p| {
        p.uv_project.kind = ProjectionKind::Barycentric
    });
    let after = &mesh(&e).uv_maps["Texture"];
    assert_ne!(after[0], old[0]);
    assert_eq!(after[1], old[1], "the unselected polygon keeps its UVs");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e).uv_maps["Texture"], old, "one undo step");
}

#[test]
fn uvt14_cubic_lays_each_side_flat_without_mirroring() {
    let mut cube = primitive::segmented_cube(DVec3::ZERO, DVec3::splat(2.), [1; 3]).unwrap();
    cube.uv_maps.clear();
    let mut e = editor_with(cube, Mode::Polygon);
    run(&mut e, Tool::UvProject, |p| {
        p.uv_project.kind = ProjectionKind::Cubic
    });
    let m = mesh(&e);
    for (f, row) in uvs(&e, "Texture").iter().enumerate() {
        // Every side is a full square and none is mirrored: counter-clockwise
        // seen from outside stays counter-clockwise in UV.
        let area: f64 = (0..row.len())
            .map(|i| row[i].perp_dot(row[(i + 1) % row.len()]))
            .sum::<f64>()
            / 2.;
        assert!(
            (area - 1.).abs() < 1e-9,
            "face {f} ({:?}): area {area}",
            m.normal(f)
        );
    }
}

#[test]
fn manual_planar_maps_size_onto_the_tile_about_center() {
    let mut e = editor_with(grid(1, 1, 1., 1.), Mode::Polygon);
    run(&mut e, Tool::UvProject, |p| {
        p.uv_project.manual = true;
        p.uv_project.size = DVec3::splat(2.);
        p.uv_project.center = DVec3::ZERO;
    });
    // Point (1, 0, 1): Top view U = x, V = -z, a 2 m frame about 0.
    let rows = uvs(&e, "Texture");
    assert!(
        rows[0].iter().any(|p| near(*p, DVec2::new(1., 0.))),
        "{rows:?}"
    );
    assert!(rows[0].iter().any(|p| near(*p, DVec2::new(0.5, 0.5))));
}

#[test]
fn uvt19_project_from_view_waits_for_a_click_then_writes_screen_positions() {
    let mut e = editor_with(grid(1, 1, 1., 1.), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::UvViewProject)).unwrap();
    assert!(mesh(&e).uv_maps.is_empty(), "nothing before the click");
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    // Looking straight down -Y from above (orthographic): screen x = world
    // x, screen y = world -z.
    let rows = [
        glam::DVec4::new(1., 0., 0., 0.),
        glam::DVec4::new(0., 0., -1., 0.),
        glam::DVec4::new(0., -1., 0., 0.),
        glam::DVec4::new(0., 0., 0., 1.),
    ];
    p.uv_view_project.view = Some(ViewFrame {
        matrix: DMat4::from_cols(rows[0], rows[1], rows[2], rows[3]).transpose(),
        eye: None,
        forward: -DVec3::Y,
    });
    p.uv_view_project.auto_scale = false;
    e.dispatch(Command::Preview(p.clone())).unwrap();
    let rows = uvs(&e, "Texture");
    // World (1, 0, 1) is screen (1, -1): UV (1, 0).
    assert!(
        rows[0].iter().any(|q| near(*q, DVec2::new(1., 0.))),
        "{rows:?}"
    );
    // Hidden Face: the quad faces -Y (wound away from the eye): dropped.
    p.uv_view_project.hidden_face = true;
    let normal = mesh(&e).normal(0);
    let r = e.dispatch(Command::Preview(p));
    if normal.y > 0. {
        assert!(r.is_ok(), "it faces the eye above");
    } else {
        assert!(r.is_err(), "every polygon faces away");
    }
}

/// A 4 × 4 grid with UVs (x, z) / 4 and one middle UV pulled out of place.
fn crumpled() -> Mesh {
    let mut m = grid(4, 4, 4., 4.);
    let rows = m
        .faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&v| {
                    let p = m.vertices[v as usize];
                    let mut q = DVec2::new(p.x / 4., p.z / 4.);
                    if v == 12 {
                        q += DVec2::new(0.2, 0.15);
                    }
                    Some(q)
                })
                .collect()
        })
        .collect();
    m.uv_maps.insert("Texture".into(), rows);
    m
}

fn uv_of(e: &Editor, point: u32) -> DVec2 {
    let m = mesh(e);
    for (f, face) in m.faces.iter().enumerate() {
        if let Some(c) = face.vertices.iter().position(|&v| v == point) {
            return m.uv_maps["Texture"][f][c].unwrap();
        }
    }
    panic!("point {point} not used")
}

#[test]
fn relax_iterations_zero_changes_nothing() {
    let mut e = editor_with(crumpled(), Mode::Polygon);
    let before = uvs(&e, "Texture");
    run(&mut e, Tool::UvRelax, |p| p.uv_relax.iterations = 0);
    assert_eq!(uvs(&e, "Texture"), before);
}

#[test]
fn every_relax_mode_pulls_a_crumpled_uv_back_towards_its_place() {
    for mode in RelaxMode::ALL {
        let mut e = editor_with(crumpled(), Mode::Polygon);
        let points = mesh(&e).vertices.clone();
        let target = DVec2::new(0.5, 0.5);
        let start = uv_of(&e, 12).distance(target);
        run(&mut e, Tool::UvRelax, |p| {
            p.uv_relax.mode = mode;
            p.uv_relax.iterations = 30;
            p.uv_relax.boundary = RelaxBoundary::Locked;
        });
        let end = uv_of(&e, 12).distance(target);
        assert!(end < start * 0.5, "{mode:?}: {start} -> {end}");
        // Locked: the border stays exactly where it was (Spring has no
        // Boundary choice: it relaxes borders too).
        if mode != RelaxMode::Spring {
            assert!(near(uv_of(&e, 0), DVec2::ZERO), "{mode:?}");
            assert!(near(uv_of(&e, 24), DVec2::ONE), "{mode:?}");
        }
        assert_eq!(mesh(&e).vertices, points, "UVT-06");
    }
}

#[test]
fn relax_with_edge_selection_not_ignored_holds_that_island() {
    let mut e = editor_with(crumpled(), Mode::Edge);
    e.scene.selection.edges = vec![(0, 1)];
    let before = uvs(&e, "Texture");
    run(&mut e, Tool::UvRelax, |p| {
        p.uv_relax.iterations = 30;
        p.uv_relax.ignore_edge_selection = false;
    });
    assert_eq!(
        uvs(&e, "Texture"),
        before,
        "the only island holds a selected edge"
    );
}

#[test]
fn relax_follows_the_falloff_weights() {
    let mut e = editor_with(crumpled(), Mode::Polygon);
    // A linear falloff that is 0 at the crumpled point.
    let start = uv_of(&e, 12);
    // Across X from 0 to 2: one end weighs nothing; put it at point 12
    // (x = 2).
    let falloff = |a: f64, b: f64| {
        Falloff::Linear(LinearFalloff {
            start: DVec3::new(a, 0., 2.),
            end: DVec3::new(b, 0., 2.),
            ..Default::default()
        })
    };
    e.dispatch(Command::SpatialFalloff(Some(falloff(0., 2.))))
        .unwrap();
    let id = e.scene.active().unwrap().id;
    let w = e.active_falloff().unwrap().weights(&e.scene, id).unwrap();
    if w[12] > 0.5 {
        e.dispatch(Command::SpatialFalloff(Some(falloff(2., 0.))))
            .unwrap();
    }
    let w = e.active_falloff().unwrap().weights(&e.scene, id).unwrap();
    assert!(w[12].abs() < 1e-9, "{w:?}");
    run(&mut e, Tool::UvRelax, |p| {
        p.uv_relax.mode = RelaxMode::Spring;
        p.uv_relax.iterations = 30;
    });
    assert!(near(uv_of(&e, 12), start), "weight 0 keeps it");
}

#[test]
fn unwrap_tool_unwraps_live_and_is_one_undo_step() {
    let mut cube = primitive::segmented_cube(DVec3::ZERO, DVec3::splat(2.), [1; 3]).unwrap();
    cube.uv_maps.clear();
    let mut e = editor_with(cube, Mode::Edge);
    e.dispatch(Command::Activate(Tool::UvUnwrap)).unwrap();
    // UVS-1 §1 Interaction: the result shows as soon as the tool is live.
    assert!(mesh(&e).uv_maps.contains_key("Texture"));
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.uv_unwrap.offset_angle = 90.;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let all: Vec<DVec2> = uvs(&e, "Texture").into_iter().flatten().collect();
    assert!(
        all.iter()
            .all(|p| p.x >= -1e-9 && p.x <= 1. + 1e-9 && p.y >= -1e-9 && p.y <= 1. + 1e-9)
    );
    e.dispatch(Command::Undo).unwrap();
    assert!(mesh(&e).uv_maps.is_empty());
}

#[test]
fn unwrap_and_relax_runs_both_halves() {
    let mut cube = primitive::segmented_cube(DVec3::ZERO, DVec3::splat(2.), [2; 3]).unwrap();
    cube.uv_maps.clear();
    let mut e = editor_with(cube, Mode::Edge);
    run(&mut e, Tool::UvPelt, |p| {
        p.uv_pelt.unwrap.iterations = 10;
        p.uv_pelt.relax.iterations = 5;
    });
    let after = uvs(&e, "Texture");
    // Relax off gives the plain unwrap; on, something else.
    let mut cube = primitive::segmented_cube(DVec3::ZERO, DVec3::splat(2.), [2; 3]).unwrap();
    cube.uv_maps.clear();
    let mut plain = editor_with(cube, Mode::Edge);
    run(&mut plain, Tool::UvPelt, |p| {
        p.uv_pelt.unwrap.iterations = 10;
        p.uv_pelt.relax_on = false;
    });
    assert_ne!(after, uvs(&plain, "Texture"));
    assert!(after.iter().flatten().all(|p| p.is_finite()));
}

#[test]
fn relax_pins_hold_their_points_and_pull_the_map() {
    let mut e = editor_with(crumpled(), Mode::Polygon);
    let pins = vec![
        operandi_core::uv_relax::UvPin {
            point: 0,
            at: DVec2::ZERO,
            to: DVec2::new(-0.2, 0.),
        },
        operandi_core::uv_relax::UvPin {
            point: 24,
            at: DVec2::ONE,
            to: DVec2::ONE,
        },
    ];
    run(&mut e, Tool::UvRelax, |p| {
        p.uv_relax.iterations = 10;
        p.uv_relax.interactive = true;
        p.uv_relax.pins = pins;
    });
    assert!(near(uv_of(&e, 0), DVec2::new(-0.2, 0.)), "the dragged pin");
    assert!(near(uv_of(&e, 24), DVec2::ONE), "the other pin stays");
    // The point next to the dragged pin followed it left.
    assert!(uv_of(&e, 1).x < 0.25 - 1e-3, "{}", uv_of(&e, 1));
}
