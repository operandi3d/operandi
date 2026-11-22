//! The shared snap service (architecture review 2026-11-14, pick B): the
//! screen grid finds what a full search finds, hidden points never snap,
//! big meshes snap, the grid is built once per view, and a query on a
//! million-point mesh stays fast.
use super::*;
use crate::snap_service::{Role, SnapService};
use operandi_core::{Face, Mesh};

fn quad(vertices: [u32; 4]) -> Face {
    Face {
        vertices: vertices.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// An app with one mesh, seen from the Front at `px` points per metre
/// around `target`, in a 1600 × 900 view.
fn front_view(mesh: Mesh, target: DVec3, px: f64) -> (Operandi, Rect) {
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.editor.scene.active_mut().unwrap().mesh = mesh.into();
    let rect = Rect::from_min_size(pos2(0., 0.), vec2(1600., 900.));
    app.camera.view = View::Front;
    app.camera.target = target;
    app.camera.distance = rect.height() as f64 * app.camera.lens / px;
    (app, rect)
}

/// `n` × `n` points one centimetre apart in the XY plane, from the
/// origin, with a quad on every square when `faces` is set.
fn sheet(n: u32, faces: bool) -> Mesh {
    let mut vertices = Vec::with_capacity((n * n) as usize);
    for y in 0..n {
        for x in 0..n {
            vertices.push(DVec3::new(x as f64 * 0.01, y as f64 * 0.01, 0.));
        }
    }
    let mut polygons = Vec::new();
    if faces {
        for y in 0..n - 1 {
            for x in 0..n - 1 {
                let a = y * n + x;
                polygons.push(quad([a, a + 1, a + n + 1, a + n]));
            }
        }
    }
    Mesh {
        vertices,
        faces: polygons.into(),
        ..Default::default()
    }
}

fn screen(app: &Operandi, p: DVec3, rect: Rect) -> Pos2 {
    app.camera.project(p, rect).unwrap().0
}

/// A cheap repeatable number in 0..1.
fn noise(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

#[test]
fn the_grid_finds_what_a_full_search_finds() {
    let mut seed = 7;
    // Points spread past the view on every side, so some sit outside the
    // grid's margin, plus a few piled onto one spot.
    let mut vertices: Vec<DVec3> = (0..20_000)
        .map(|_| DVec3::new(noise(&mut seed) * 16. - 8., noise(&mut seed) * 10. - 5., 0.))
        .collect();
    vertices.extend([DVec3::new(0.3, 0.2, 0.); 4]);
    let mesh = Mesh {
        vertices,
        ..Default::default()
    };
    let (app, rect) = front_view(mesh, DVec3::ZERO, 200.);
    let service = SnapService::default();
    let world = service.world(&app.editor.scene);
    let projector = app.camera.projector(rect);
    let grid = service.vertex_grid(&world, projector);
    let points = &app.editor.scene.active().unwrap().mesh.vertices;
    let mut checked = 0;
    for _ in 0..400 {
        // Pointer positions inside the view, at its edges and well past.
        let at = pos2(
            (noise(&mut seed) * 2400. - 400.) as f32,
            (noise(&mut seed) * 1700. - 400.) as f32,
        );
        let radius = (noise(&mut seed) * 60.) as f32;
        let got: Vec<usize> = grid.near(at, radius).iter().map(|c| c.index).collect();
        let mut want: Vec<(f32, usize)> = points
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| {
                let d = projector.project(p)?.0.distance(at);
                (d <= radius).then_some((d, i))
            })
            .collect();
        want.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let want: Vec<usize> = want.into_iter().map(|w| w.1).collect();
        assert_eq!(got, want, "at {at:?} radius {radius}");
        checked += got.len();
    }
    assert!(checked > 100, "the queries found something: {checked}");
    let piled = screen(&app, DVec3::new(0.3, 0.2, 0.), rect);
    let first = grid.near(piled, 1.)[0].index;
    assert_eq!(first, 20_000, "equal distances keep scene order");
}

#[test]
fn a_point_behind_a_surface_never_snaps_for_any_role() {
    // A quad between the camera and the point at the origin hides it; the
    // point at (1, 0, 0) is in the open.
    let mut app = Operandi::empty();
    app.camera.view = View::Front;
    let z = 0.5 * app.camera.basis().0.z.signum();
    let mesh = Mesh {
        vertices: vec![
            DVec3::ZERO,
            DVec3::X,
            DVec3::new(-0.3, -0.3, z),
            DVec3::new(0.3, -0.3, z),
            DVec3::new(0.3, 0.3, z),
            DVec3::new(-0.3, 0.3, z),
        ],
        faces: vec![quad([2, 3, 4, 5])].into(),
        ..Default::default()
    };
    let (app, rect) = front_view(mesh, DVec3::ZERO, 200.);
    let item = app.editor.scene.active().unwrap().id;
    let hidden = screen(&app, DVec3::ZERO, rect);
    let open = screen(&app, DVec3::X, rect);
    let nothing = |_: &crate::snap_service::Owner| false;
    for role in [Role::JoinPoint, Role::RulerEnd, Role::Weld] {
        assert_eq!(
            app.snap_scene_point(hidden + vec2(2., 0.), role, rect, &nothing),
            None,
            "{role:?} snapped to a hidden point"
        );
        let found = app
            .snap_scene_point(open + vec2(2., 0.), role, rect, &nothing)
            .expect("the open point snaps");
        assert_eq!(
            (found.hit.point, found.owner),
            (DVec3::X, crate::snap_service::Owner::Point { item, v: 1 }),
            "{role:?}"
        );
    }
}

#[test]
fn meshes_over_200k_points_snap() {
    // Draw and the Ruler used to skip any mesh over 200,000 points.
    let (app, rect) = front_view(sheet(500, false), DVec3::new(1.2, 1.2, 0.), 300.);
    assert!(app.editor.scene.active().unwrap().mesh.vertices.len() > 200_000);
    let target = DVec3::new(1.23, 1.17, 0.);
    let at = screen(&app, target, rect) + vec2(0.4, 0.);
    let hit = app
        .snap_scene_point(at, Role::JoinPoint, rect, &|_| false)
        .expect("a big mesh snaps")
        .hit;
    assert!((hit.point - target).length() < 1e-9, "{}", hit.point);
}

#[test]
fn the_grid_is_built_once_per_view_and_scene() {
    let (mut app, rect) = front_view(sheet(50, true), DVec3::new(0.25, 0.25, 0.), 1000.);
    let at = screen(&app, DVec3::new(0.2, 0.2, 0.), rect);
    for i in 0..30 {
        let pointer = at + vec2(i as f32 * 0.5, 0.);
        assert!(
            app.snap_scene_point(pointer, Role::JoinPoint, rect, &|_| false)
                .is_some()
        );
    }
    assert_eq!(app.snap_service.grid_builds(), 1, "pointer moves reuse it");
    app.camera.target.x += 0.01;
    app.snap_scene_point(at, Role::JoinPoint, rect, &|_| false);
    assert_eq!(
        app.snap_service.grid_builds(),
        2,
        "a new view builds a new one"
    );
    app.editor.scene.active_mut().unwrap().mesh.vertices[0].z = 0.5;
    app.snap_scene_point(at, Role::JoinPoint, rect, &|_| false);
    assert_eq!(app.snap_service.grid_builds(), 3, "an edited mesh too");
}

#[test]
fn each_role_keeps_its_radius() {
    // Unchanged from the seven copies this service replaced.
    let options = crate::snapping::SnappingPreferences::default();
    assert_eq!(Role::Handle.radius(&options), options.outer());
    assert_eq!(Role::DrawPoint.radius(&options), options.outer());
    assert_eq!(Role::JoinPoint.radius(&options), 8.);
    assert_eq!(Role::RulerEnd.radius(&options), 10.);
    assert_eq!(Role::Weld.radius(&options), 15.);
    assert_eq!(Role::SymmetrySeam.radius(&options), 8.);
}

/// Opt-in speed probe (release): a snap query on a 1M-point mesh, with
/// its hidden-point check, stays under 2 ms per pointer move once the
/// grid is built for the view.
/// `cargo test --release -p operandi-app snap_query_on_a_million_points -- --ignored --nocapture`
#[test]
#[ignore]
fn snap_query_on_a_million_points_is_under_2_ms() {
    let (app, rect) = front_view(sheet(1000, true), DVec3::new(5., 5., 0.), 120.);
    let started = std::time::Instant::now();
    let at = screen(&app, DVec3::new(5., 5., 0.), rect);
    assert!(
        app.snap_scene_point(at, Role::DrawPoint, rect, &|_| false)
            .is_some()
    );
    let first = started.elapsed();
    let moves = 500;
    let started = std::time::Instant::now();
    let mut found = 0;
    for i in 0..moves {
        let pointer = at + vec2((i % 50) as f32 * 7. - 175., (i / 50) as f32 * 9. - 45.);
        found += usize::from(
            app.snap_scene_point(pointer, Role::DrawPoint, rect, &|_| false)
                .is_some(),
        );
    }
    let per = started.elapsed() / moves;
    // After the view moves only the grid is built again.
    let mut app = app;
    app.camera.target.x += 0.001;
    let started = std::time::Instant::now();
    app.snap_scene_point(at, Role::DrawPoint, rect, &|_| false);
    let new_view = started.elapsed();
    println!(
        "snap on 1M points: first query (gather, grid, surface) {first:?}, then {per:?} per move ({found}/{moves} snapped), {new_view:?} after a view change"
    );
    assert_eq!(found, moves as usize);
    assert!(
        per < std::time::Duration::from_millis(2),
        "{per:?} per move"
    );
    // Every point and edge type at once (spec v2.2), once their grids exist.
    let mut all = Targets::NONE;
    for kind in [
        Kind::Vertex,
        Kind::Edge,
        Kind::EdgeCenter,
        Kind::PolygonCenter,
        Kind::Intersection,
        Kind::Box,
        Kind::Pivot,
    ] {
        *all.flag(kind) = true;
    }
    let started = std::time::Instant::now();
    assert!(find(&app, at, rect, all).is_some());
    let first_all = started.elapsed();
    let started = std::time::Instant::now();
    for i in 0..moves {
        let pointer = at + vec2((i % 50) as f32 * 7. - 175., (i / 50) as f32 * 9. - 45.);
        assert!(find(&app, pointer, rect, all).is_some());
    }
    let per_all = started.elapsed() / moves;
    println!("all point and edge types: first {first_all:?}, then {per_all:?} per move");
    for kind in [
        Kind::Vertex,
        Kind::Edge,
        Kind::EdgeCenter,
        Kind::PolygonCenter,
        Kind::Intersection,
    ] {
        let t = only(kind);
        find(&app, at, rect, t);
        let started = std::time::Instant::now();
        for i in 0..moves {
            let pointer = at + vec2((i % 50) as f32 * 7. - 175., (i / 50) as f32 * 9. - 45.);
            find(&app, pointer, rect, t);
        }
        println!("{kind:?}: {:?} per move", started.elapsed() / moves);
    }
    assert!(
        per_all < std::time::Duration::from_millis(2),
        "{per_all:?} per move"
    );
}

// ---- Snapping spec v2.1 target types (snapping-2026-11-14) ----

use crate::snap_kinds::Kind;
use crate::snap_service::{Owner, Query, SnapHit};
use crate::snapping::Targets;

/// A unit quad in the XY plane at the origin (points 0..3 counter-clockwise
/// from the origin), seen from the Front at 200 points per metre.
fn quad_scene() -> (Operandi, Rect) {
    let mesh = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.), DVec3::Y],
        faces: vec![quad([0, 1, 2, 3])].into(),
        ..Default::default()
    };
    front_view(mesh, DVec3::new(0.5, 0.5, 0.), 200.)
}

fn only(kind: Kind) -> Targets {
    let mut t = Targets::NONE;
    *t.flag(kind) = true;
    t
}

/// The best target at `at` for `targets`, as a tool handle would find it
/// (24 px Inner Range, 40 px Outer Range).
fn find(app: &Operandi, at: Pos2, rect: Rect, targets: Targets) -> Option<SnapHit> {
    find_with(app, at, rect, targets, &|_| false)
}

fn find_with(
    app: &Operandi,
    at: Pos2,
    rect: Rect,
    targets: Targets,
    skip: &dyn Fn(&Owner) -> bool,
) -> Option<SnapHit> {
    let scene = &app.editor.scene;
    let world = app.snap_service.world(scene);
    let surface = app.surface_cache.get(scene, &app.subdivision_cache);
    app.snap_service.nearest(
        &world,
        &Query {
            at,
            radius: 40.,
            inner: 24.,
            pivot_inner: 15.,
            targets,
            skip,
            extra: &[],
            max_depth: None,
            surface: &surface,
            camera: &app.camera,
            rect,
        },
    )
}

#[test]
fn edge_snaps_to_the_nearest_point_along_an_edge() {
    let (app, rect) = quad_scene();
    // Just below the bottom edge, a quarter of the way along.
    let at = screen(&app, DVec3::new(0.25, 0., 0.), rect) + vec2(0., 5.);
    let hit = find(&app, at, rect, only(Kind::Edge)).expect("an edge is in range");
    assert_eq!(hit.kind, Kind::Edge);
    assert!(
        (hit.hit.point - DVec3::new(0.25, 0., 0.)).length() < 1e-6,
        "{}",
        hit.hit.point
    );
    assert!(matches!(hit.owner, Owner::Edge { a: 0, b: 1, .. }));
}

#[test]
fn centres_box_and_pivot_are_point_targets() {
    let (app, rect) = quad_scene();
    let near = |p: DVec3| screen(&app, p, rect) + vec2(3., 0.);
    let edge_mid = DVec3::new(0.5, 0., 0.);
    let hit = find(&app, near(edge_mid), rect, only(Kind::EdgeCenter)).unwrap();
    assert!((hit.hit.point - edge_mid).length() < 1e-9);
    let centre = DVec3::new(0.5, 0.5, 0.);
    let hit = find(&app, near(centre), rect, only(Kind::PolygonCenter)).unwrap();
    assert!((hit.hit.point - centre).length() < 1e-9);
    assert!(matches!(hit.owner, Owner::Polygon { face: 0, .. }));
    // Box: the corners of the item's box (SN2-32); not its sides.
    let corner = DVec3::new(1., 1., 0.);
    let hit = find(&app, near(corner), rect, only(Kind::Box)).unwrap();
    assert!((hit.hit.point - corner).length() < 1e-9);
    assert!(find(&app, near(DVec3::new(1., 0.5, 0.)), rect, only(Kind::Box)).is_none());
    // Pivot: the item's position (the origin here).
    let hit = find(&app, near(DVec3::ZERO), rect, only(Kind::Pivot)).unwrap();
    assert!((hit.hit.point).length() < 1e-9);
    assert!(matches!(hit.owner, Owner::Item(_)));
}

#[test]
fn polygon_lands_under_the_handle_only_when_nothing_is_in_inner_range() {
    let (app, rect) = quad_scene();
    let mut both = only(Kind::Polygon);
    both.vertex = true;
    // Far from every corner: the surface under the handle.
    let middle = screen(&app, DVec3::new(0.4, 0.6, 0.), rect);
    let hit = find(&app, middle, rect, both).unwrap();
    assert_eq!(hit.kind, Kind::Polygon);
    assert!((hit.hit.point - DVec3::new(0.4, 0.6, 0.)).length() < 1e-6);
    // 10 px from a corner: the vertex is inside Inner Range and wins.
    let corner = screen(&app, DVec3::new(1., 1., 0.), rect) - vec2(10., -2.);
    let hit = find(&app, corner, rect, both).unwrap();
    assert_eq!(hit.kind, Kind::Vertex);
}

#[test]
fn ties_within_2_px_go_to_the_more_specific_type() {
    let (app, rect) = quad_scene();
    let mut t = only(Kind::Edge);
    t.vertex = true;
    t.edge_center = true;
    // 1.5 px nearer the edge than the corner: still the vertex (SN2-15).
    let corner = screen(&app, DVec3::X, rect);
    let at = corner + vec2(-1.5, 3.);
    let hit = find(&app, at, rect, t).unwrap();
    assert_eq!(hit.kind, Kind::Vertex, "{hit:?}");
    // Well along the edge: the edge itself.
    let along = screen(&app, DVec3::new(0.3, 0., 0.), rect) + vec2(0., 3.);
    assert_eq!(find(&app, along, rect, t).unwrap().kind, Kind::Edge);
}

#[test]
fn intersection_is_where_an_edge_passes_through_a_polygon() {
    // A quad in the XY plane and a separate stick along Z through
    // (0.3, 0.4): Front view looks down the stick, so test from Top.
    let mut mesh = Mesh {
        vertices: vec![
            DVec3::ZERO,
            DVec3::X,
            DVec3::new(1., 1., 0.),
            DVec3::Y,
            DVec3::new(0.3, 0.4, -0.5),
            DVec3::new(0.3, 0.4, 0.5),
            DVec3::new(0.9, 0.4, 0.5),
        ],
        faces: vec![quad([0, 1, 2, 3])].into(),
        ..Default::default()
    };
    // The stick as a thin triangle so its edge 4-5 exists.
    let mut faces: Vec<Face> = mesh.faces.to_vec();
    faces.push(Face {
        vertices: vec![4, 5, 6],
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    });
    mesh.faces = faces.into();
    let (mut app, rect) = front_view(mesh, DVec3::new(0.5, 0.5, 0.), 200.);
    app.camera.view = View::Perspective;
    app.camera.yaw = 0.9;
    app.camera.pitch = 0.4;
    let through = DVec3::new(0.3, 0.4, 0.);
    let at = screen(&app, through, rect) + vec2(2., 1.);
    let hit = find(&app, at, rect, only(Kind::Intersection)).expect("the crossing");
    assert!(
        (hit.hit.point - through).length() < 1e-6,
        "{}",
        hit.hit.point
    );
}

#[test]
fn layers_only_selected_and_depth_leave_targets_out() {
    let (mut app, rect) = quad_scene();
    let item = app.editor.scene.active().unwrap().id;
    let at = screen(&app, DVec3::X, rect) + vec2(3., 0.);
    // Background: the selected mesh gives nothing.
    app.snapping.layers = crate::snapping::Layers::Background;
    app.editor.scene.selection.item = Some(item);
    let scene = app.editor.scene.clone();
    let world = app.snap_service.world(&scene);
    let skip = |o: &Owner| app.geometry_skip_for_tests(&scene, &world, o);
    assert!(find_with(&app, at, rect, only(Kind::Vertex), &skip).is_none());
    // Only Selected: points of the selected polygons only.
    app.snapping.layers = crate::snapping::Layers::Both;
    app.snapping.only_selected = true;
    app.editor.scene.selection.faces.clear();
    let scene = app.editor.scene.clone();
    let skip = |o: &Owner| app.geometry_skip_for_tests(&scene, &world, o);
    assert!(find_with(&app, at, rect, only(Kind::Vertex), &skip).is_none());
    app.editor.scene.selection.faces = vec![0];
    let scene = app.editor.scene.clone();
    let skip = |o: &Owner| app.geometry_skip_for_tests(&scene, &world, o);
    assert!(find_with(&app, at, rect, only(Kind::Vertex), &skip).is_some());
}

#[test]
fn presets_are_per_mode_and_hold_ticks() {
    let mut s = crate::snapping::SnappingPreferences::default();
    assert_eq!(s.presets[0].current, "Default");
    s.component.edge = true;
    s.new_preset(1, "Edges").unwrap();
    assert!(s.new_preset(1, "Edges").is_err(), "names are unique");
    s.component.vertex = true;
    s.store_ticks_in_preset(1);
    s.select_preset(1, "Default");
    assert_eq!(s.component, Targets::NONE, "Default's ticks are back");
    s.select_preset(1, "Edges");
    assert!(s.component.edge && s.component.vertex);
    // Global's presets are untouched.
    assert_eq!(s.presets[0].list.len(), 1);
    s.remove_preset(1, "Default");
    s.remove_preset(1, "Edges");
    assert_eq!(s.presets[1].current, "Default");
    assert_eq!(s.presets[1].list.len(), 1, "Default can't be removed");
}

#[test]
fn f11_opens_the_popover_in_the_app_and_again_closes_it() {
    let mut app = Operandi::empty();
    app.open_snapping_options();
    assert!(app.snapping_options.is_some());
    app.open_snapping_options();
    assert!(
        app.snapping_options.is_none(),
        "SN2-04: F11 again closes it"
    );
}

#[test]
fn tool_cards_name_their_snap_role() {
    use operandi_core::Tool;
    use operandi_core::tool_card::Snap;
    assert_eq!(Role::of_card(Snap::None), None);
    assert_eq!(Role::of_card(Snap::Handle), Some(Role::Handle));
    assert_eq!(Role::of_card(Snap::Grab), Some(Role::Handle));
    assert_eq!(Role::of_card(Snap::Placement), Some(Role::DrawPoint));
    assert_eq!(Role::of_tool(Tool::Move), Some(Role::Handle));
    assert_eq!(Role::of_tool(Tool::Bevel), None);
}
