//! Solid Drill and Axis Drill (specifications/drills-2026-11-11, DRILL-01..27,
//! acceptance cases DR-D0..D3, DR-S1..S5, DR-A1..A5, DR-F1..F3).
use glam::{DVec2, DVec3};
use operandi_core::*;
use std::collections::HashMap;

fn face(vertices: Vec<u32>, material: &str) -> Face {
    Face {
        vertices,
        uv: None,
        material: material.into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// An n×n grid of quads over `lo..hi` in two axes (`u`, `v`), at `w` on the
/// third axis, facing +third.
fn grid(n: u32, lo: f64, hi: f64, axes: (usize, usize, usize), w: f64) -> Mesh {
    let (u, v, k) = axes;
    let mut m = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let mut p = DVec3::ZERO;
            p[u] = lo + (hi - lo) * i as f64 / n as f64;
            p[v] = lo + (hi - lo) * j as f64 / n as f64;
            p[k] = w;
            m.vertices.push(p);
        }
    }
    let at = |i: u32, j: u32| j * (n + 1) + i;
    for j in 0..n {
        for i in 0..n {
            m.faces.push(face(
                vec![at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)],
                "matG",
            ));
        }
    }
    m
}

/// DR grid target: 4x4 unit quads, X,Y in -2..2, Z = 0.
fn grid_target() -> Mesh {
    grid(4, -2., 2., (0, 1, 2), 0.)
}

fn cube(c: DVec3, size: DVec3) -> Mesh {
    let mut m = Mesh::box_mesh(c, size, 0).unwrap();
    for f in &mut m.faces {
        f.material = "D".into();
    }
    m
}

fn unit_cube() -> Mesh {
    cube(DVec3::ZERO, DVec3::splat(2.))
}

/// An editor with the target (selected) and drivers as mesh items.
fn scene(target: Mesh, drivers: Vec<Mesh>) -> (Editor, u64) {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for m in std::iter::once(target).chain(drivers) {
        e.dispatch(Command::NewMesh).unwrap();
        let item = e.scene.active_mut().unwrap();
        item.mesh = m.into();
        ids.push(item.id);
    }
    e.dispatch(Command::SelectItem(Some(ids[0]))).unwrap();
    e.synchronize_identities().unwrap();
    e.undo.clear();
    (e, ids[0])
}

fn mesh(e: &Editor, id: u64) -> &Mesh {
    &e.scene.items.iter().find(|i| i.id == id).unwrap().mesh
}

fn opts(op: DrillOp) -> DrillOptions {
    DrillOptions {
        op,
        ..Default::default()
    }
}

fn solid(e: &mut Editor, op: DrillOp) -> Result<()> {
    e.dispatch(Command::SolidDrill(opts(op)))
}

fn axis(e: &mut Editor, op: DrillOp, axis: DrillAxis) -> Result<()> {
    e.dispatch(Command::AxisDrill(DrillOptions { axis, ..opts(op) }))
}

fn polygon_area(m: &Mesh, f: &Face) -> f64 {
    let mut n = DVec3::ZERO;
    for i in 0..f.vertices.len() {
        let a = m.vertices[f.vertices[i] as usize];
        let b = m.vertices[f.vertices[(i + 1) % f.vertices.len()] as usize];
        n += a.cross(b);
    }
    n.length() * 0.5
}

fn area(m: &Mesh) -> f64 {
    m.faces.iter().map(|f| polygon_area(m, f)).sum()
}

fn area_with(m: &Mesh, material: &str) -> f64 {
    m.faces
        .iter()
        .filter(|f| f.material == material)
        .map(|f| polygon_area(m, f))
        .sum()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn edge_uses(m: &Mesh) -> HashMap<(u32, u32), usize> {
    let mut uses = HashMap::new();
    for f in &m.faces {
        for i in 0..f.vertices.len() {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % f.vertices.len()]);
            *uses.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    uses
}

/// Loops of open edges.
fn boundary_loops(m: &Mesh) -> usize {
    let open: Vec<(u32, u32)> = edge_uses(m)
        .into_iter()
        .filter(|(_, n)| *n == 1)
        .map(|(e, _)| e)
        .collect();
    let mut parent: HashMap<u32, u32> = HashMap::new();
    fn find(p: &mut HashMap<u32, u32>, x: u32) -> u32 {
        let up = *p.entry(x).or_insert(x);
        if up == x {
            x
        } else {
            let r = find(p, up);
            p.insert(x, r);
            r
        }
    }
    for &(a, b) in &open {
        let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
        parent.insert(ra, rb);
    }
    let points: Vec<u32> = parent.keys().copied().collect();
    let mut roots: Vec<u32> = points.into_iter().map(|p| find(&mut parent, p)).collect();
    roots.sort_unstable();
    roots.dedup();
    roots.len()
}

fn no_unused_points(m: &Mesh) -> bool {
    m.used_points().iter().all(|u| *u)
}

// ── DR-D0..D3: the observed grid cases ──────────────────────────────────

#[test]
fn dr_d_grid_cases_match_the_observed_counts() {
    for (op, polygons, points, stencil) in [
        (DrillOp::Core, 4, 9, 0),
        (DrillOp::Tunnel, 12, 24, 0),
        (DrillOp::Stencil, 16, 25, 4),
        (DrillOp::Slice, 16, 25, 0),
    ] {
        let (mut e, id) = scene(grid_target(), vec![unit_cube()]);
        let driver_before = e.scene.items[1].clone();
        let result = solid(&mut e, op);
        let m = mesh(&e, id);
        if op == DrillOp::Slice {
            // DRILL-17: the outline is on grid lines, so nothing changes.
            assert!(result.is_err(), "Slice along existing edges is a no-op");
            assert!(e.undo.is_empty());
        } else {
            result.unwrap();
            assert_eq!(e.undo.len(), 1, "{op:?}");
        }
        assert_eq!(m.faces.len(), polygons, "{op:?}");
        assert_eq!(m.vertices.len(), points, "{op:?}");
        assert!(
            m.faces.iter().all(|f| f.vertices.len() == 4),
            "{op:?} all quads"
        );
        assert!(e.scene.selection.faces.is_empty(), "DRILL-22");
        assert_eq!(
            m.faces.iter().filter(|f| f.material == "Stencil").count(),
            stencil,
            "{op:?}"
        );
        assert_eq!(e.scene.items[1], driver_before, "DRILL-05 {op:?}");
    }
}

#[test]
fn dr_d2_stencil_creates_its_material_in_one_undo_step() {
    let (mut e, id) = scene(grid_target(), vec![unit_cube()]);
    let before = e.scene.clone();
    e.dispatch(Command::SolidDrill(DrillOptions {
        op: DrillOp::Stencil,
        material: "stencilMat".into(),
        ..Default::default()
    }))
    .unwrap();
    let m = mesh(&e, id);
    assert_eq!(
        m.faces
            .iter()
            .filter(|f| f.material == "stencilMat")
            .count(),
        4
    );
    assert!(e.scene.shader_tree.material_for_tag("stencilMat").is_some());
    assert_eq!(e.undo.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items, before.items);
    assert!(e.scene.shader_tree.material_for_tag("stencilMat").is_none());
}

// ── DR-S1..S5: a hole inside one polygon, a tunnel through a cube ───────

#[test]
fn dr_s_hole_inside_one_polygon() {
    let quad = grid(1, -2., 2., (0, 1, 2), 0.);
    for (op, polygons, total, inside) in [
        (DrillOp::Core, 1, 4., 0.),
        (DrillOp::Tunnel, 2, 12., 0.),
        (DrillOp::Stencil, 3, 16., 4.),
        (DrillOp::Slice, 3, 16., 0.),
    ] {
        let (mut e, id) = scene(quad.clone(), vec![unit_cube()]);
        solid(&mut e, op).unwrap();
        let m = mesh(&e, id);
        assert_eq!(
            m.faces.len(),
            polygons,
            "{op:?} (Q1: two hole-free polygons)"
        );
        assert!(close(area(m), total), "{op:?} area {}", area(m));
        assert!(close(area_with(m, "Stencil"), inside), "{op:?}");
        assert!(no_unused_points(m), "{op:?}");
        // Hole-free: no polygon repeats a point.
        for f in &m.faces {
            let mut v = f.vertices.clone();
            v.sort_unstable();
            v.dedup();
            assert_eq!(v.len(), f.vertices.len());
        }
        // The rim is shared, not duplicated.
        if op == DrillOp::Slice {
            assert!(edge_uses(m).values().all(|&n| n <= 2));
            assert_eq!(boundary_loops(m), 1, "only the quad's own outline is open");
        }
    }
}

#[test]
fn dr_s5_tunnel_through_a_closed_cube_leaves_holes_without_walls() {
    let driver = cube(DVec3::ZERO, DVec3::new(1., 4., 1.));
    let mut target = unit_cube();
    for f in &mut target.faces {
        f.material = "T".into();
    }
    let (mut e, id) = scene(target, vec![driver]);
    solid(&mut e, DrillOp::Tunnel).unwrap();
    let m = mesh(&e, id);
    assert!(close(area(m), 22.), "area {}", area(m));
    assert_eq!(boundary_loops(m), 2);
    assert!(m.faces.iter().all(|f| f.material != "D"), "no driver faces");
}

// ── DR-A1..A5: Axis Drill ───────────────────────────────────────────────

fn outline_quad(x: (f64, f64), z: (f64, f64), y: f64) -> Mesh {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(x.0, y, z.0),
            DVec3::new(x.1, y, z.0),
            DVec3::new(x.1, y, z.1),
            DVec3::new(x.0, y, z.1),
        ],
        ..Default::default()
    };
    m.faces.push(face(vec![0, 1, 2, 3], "D"));
    m
}

#[test]
fn dr_a_axis_drill_along_y_on_a_cube() {
    for (op, total, inside, polygons) in [
        (DrillOp::Core, 2., 0., Some(2)),
        (DrillOp::Tunnel, 22., 0., None),
        (DrillOp::Stencil, 24., 2., None),
        (DrillOp::Slice, 24., 0., None),
    ] {
        let driver = outline_quad((-0.5, 0.5), (-0.5, 0.5), 5.);
        let (mut e, id) = scene(unit_cube(), vec![driver]);
        axis(&mut e, op, DrillAxis::Y).unwrap();
        let m = mesh(&e, id);
        assert!(close(area(m), total), "{op:?} area {}", area(m));
        assert!(close(area_with(m, "Stencil"), inside), "{op:?}");
        if let Some(n) = polygons {
            assert_eq!(m.faces.len(), n, "{op:?}");
        }
        if op == DrillOp::Tunnel {
            assert_eq!(boundary_loops(m), 2, "the bottom (back) face is cut too");
        }
    }
}

#[test]
fn dr_a5_overlapping_driver_polygons_cut_as_one_outline() {
    let target = grid(1, -2., 2., (2, 0, 1), 0.); // X,Z in -2..2 at Y=0
    let mut driver = outline_quad((-1., 0.5), (-0.5, 0.5), 3.);
    append_mesh(&mut driver, outline_quad((-0.5, 1.), (-0.5, 0.5), 3.)).unwrap();
    let (mut e, id) = scene(target, vec![driver]);
    e.dispatch(Command::AxisDrill(DrillOptions {
        op: DrillOp::Stencil,
        ..Default::default()
    }))
    .unwrap();
    let m = mesh(&e, id);
    assert!(close(area(m), 16.));
    assert!(
        close(area_with(m, "Stencil"), 2.),
        "{}",
        area_with(m, "Stencil")
    );
    // No edge at the overlap: the inside is one rectangle.
    let inside: Vec<&Face> = m.faces.iter().filter(|f| f.material == "Stencil").collect();
    assert_eq!(inside.len(), 1);
    assert_eq!(inside[0].vertices.len(), 4);
}

#[test]
fn axis_drill_cuts_walls_by_the_prism() {
    // Q7: a wall edge-on to the axis is cut too.
    let wall = grid(1, -2., 2., (0, 1, 2), 0.); // the XY plane, edge-on to Y
    let driver = outline_quad((-0.5, 0.5), (-0.5, 0.5), 5.);
    let (mut e, id) = scene(wall, vec![driver]);
    axis(&mut e, DrillOp::Core, DrillAxis::Y).unwrap();
    let m = mesh(&e, id);
    // The strip x in -0.5..0.5, all of y: 1 x 4 (it lies on the prism's
    // middle, so it is inside).
    assert!(close(area(m), 4.), "{}", area(m));
}

// ── DR-F1..F3: refusals ─────────────────────────────────────────────────

fn refuses_unchanged(e: &mut Editor, cmd: Command, needle: &str) {
    let before = e.scene.clone();
    let err = e.dispatch(cmd).unwrap_err();
    assert!(err.contains(needle), "{err}");
    assert_eq!(e.scene.items, before.items, "DRILL-21: scene unchanged");
    assert!(e.undo.is_empty(), "no undo step");
}

#[test]
fn dr_f1_no_driver() {
    let (mut e, _) = scene(grid_target(), vec![]);
    refuses_unchanged(
        &mut e,
        Command::SolidDrill(opts(DrillOp::Tunnel)),
        "No driver",
    );
    refuses_unchanged(
        &mut e,
        Command::AxisDrill(opts(DrillOp::Tunnel)),
        "No driver",
    );
}

#[test]
fn dr_f2_open_driver_is_refused_with_its_problem_edges() {
    let mut open = unit_cube();
    open.faces.pop();
    let (mut e, _) = scene(grid_target(), vec![open]);
    refuses_unchanged(
        &mut e,
        Command::SolidDrill(opts(DrillOp::Tunnel)),
        "open edges",
    );
    let problem = e
        .drill_problem(DrillKind::Solid, &opts(DrillOp::Tunnel))
        .expect("a problem to show");
    assert_eq!(problem.edges.len(), 4);
}

#[test]
fn dr_f3_driver_edge_on_to_the_axis() {
    let flat_in_xy = grid(1, -0.5, 0.5, (0, 1, 2), 3.);
    let (mut e, _) = scene(grid_target(), vec![flat_in_xy]);
    refuses_unchanged(
        &mut e,
        Command::AxisDrill(opts(DrillOp::Tunnel)),
        "no outline along Y",
    );
}

// ── Other rules ─────────────────────────────────────────────────────────

#[test]
fn undo_and_redo_are_one_step_with_the_tool_label() {
    // DRILL-27.
    let (mut e, id) = scene(grid_target(), vec![unit_cube()]);
    let before = e.scene.clone();
    solid(&mut e, DrillOp::Tunnel).unwrap();
    let after = mesh(&e, id).clone();
    assert_eq!(e.undo.len(), 1);
    assert_eq!(e.undo.last().unwrap().label, "Solid Drill Tunnel");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items, before.items);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(*mesh(&e, id), after);
    assert_eq!(e.solid_drill_preset.op, DrillOp::Tunnel);
}

#[test]
fn driver_out_of_reach() {
    // DRILL-25: Core empties the target; the others change nothing.
    let far = cube(DVec3::new(10., 0., 0.), DVec3::splat(2.));
    let (mut e, id) = scene(grid_target(), vec![far.clone()]);
    solid(&mut e, DrillOp::Core).unwrap();
    assert!(mesh(&e, id).faces.is_empty());
    assert!(e.drill_note.as_deref().unwrap().contains("nothing is left"));
    for op in [DrillOp::Tunnel, DrillOp::Stencil, DrillOp::Slice] {
        let (mut e, _) = scene(grid_target(), vec![far.clone()]);
        refuses_unchanged(&mut e, Command::SolidDrill(opts(op)), "doesn't reach");
    }
}

#[test]
fn slanted_cut_makes_shared_ngons_and_keeps_untouched_quads() {
    // DRILL-16/17: a cube turned 30° about Z cuts the grid off its lines.
    let mut driver = unit_cube();
    let r = glam::DQuat::from_rotation_z(30f64.to_radians());
    for v in &mut driver.vertices {
        *v = r * *v * 1.1;
    }
    let (mut e, id) = scene(grid_target(), vec![driver]);
    let before = grid_target();
    solid(&mut e, DrillOp::Slice).unwrap();
    let m = mesh(&e, id);
    assert!(close(area(m), 16.));
    // Shared cut points: only the grid's own outline is open.
    assert_eq!(boundary_loops(m), 1);
    assert!(edge_uses(m).values().all(|&n| n <= 2));
    // The four corner quads are untouched, in their corner order.
    for f in [0usize, 3, 12, 15] {
        let corners: Vec<DVec3> = before.faces[f]
            .vertices
            .iter()
            .map(|&v| before.vertices[v as usize])
            .collect();
        assert!(
            m.faces.iter().any(|g| g
                .vertices
                .iter()
                .map(|&v| m.vertices[v as usize])
                .eq(corners.iter().copied())),
            "corner quad {f} kept"
        );
    }
    // Core + Tunnel = the whole (DRILL-19).
    let mut parts = 0.;
    for op in [DrillOp::Core, DrillOp::Tunnel] {
        let (mut e, id) = scene(grid_target(), vec![mesh_of(&e_driver(r))]);
        solid(&mut e, op).unwrap();
        parts += area(mesh(&e, id));
    }
    assert!(close(parts, 16.), "{parts}");
}

fn e_driver(r: glam::DQuat) -> Mesh {
    let mut driver = unit_cube();
    for v in &mut driver.vertices {
        *v = r * *v * 1.1;
    }
    driver
}

fn mesh_of(m: &Mesh) -> Mesh {
    m.clone()
}

#[test]
fn moved_items_cut_in_world_space() {
    let (mut e, id) = scene(grid_target(), vec![unit_cube()]);
    e.scene.items[0].transform.translation = DVec3::new(10., 0., 0.);
    e.scene.items[1].transform.translation = DVec3::new(10., 0., 0.);
    solid(&mut e, DrillOp::Core).unwrap();
    let m = mesh(&e, id);
    assert_eq!(m.faces.len(), 4);
    assert!(m.vertices.iter().all(|v| v.x.abs() <= 1. + 1e-9));
}

#[test]
fn last_selected_polygons_drive_and_stay() {
    // DRILL-02 Last Selected: the selected cube cuts the grid in one item.
    let mut m = grid_target();
    append_mesh(&mut m, cube(DVec3::new(0.25, 0.25, 0.), DVec3::splat(1.))).unwrap();
    let (mut e, id) = scene(m, vec![]);
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![16];
    e.dispatch(Command::SolidDrill(DrillOptions {
        op: DrillOp::Tunnel,
        drive: DriveMesh::LastSelected,
        ..Default::default()
    }))
    .unwrap();
    let m = mesh(&e, id);
    // 16 - 1 = 15 square units of grid plus the driver cube's 6.
    assert!(close(area(m), 15. + 6.), "{}", area(m));
    assert_eq!(m.faces.iter().filter(|f| f.material == "D").count(), 6);
}

#[test]
fn uvs_follow_and_new_corners_interpolate() {
    // DRILL-18.
    let mut target = grid_target();
    let rows = target
        .faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&v| {
                    let p = target.vertices[v as usize];
                    Some(DVec2::new(p.x, p.y))
                })
                .collect()
        })
        .collect();
    target.uv_maps.insert("Texture".into(), rows);
    let (mut e, id) = scene(target, vec![e_driver(glam::DQuat::from_rotation_z(0.4))]);
    solid(&mut e, DrillOp::Slice).unwrap();
    let m = mesh(&e, id);
    let rows = &m.uv_maps["Texture"];
    for (f, face) in m.faces.iter().enumerate() {
        for (c, &v) in face.vertices.iter().enumerate() {
            let p = m.vertices[v as usize];
            let uv = rows[f][c].expect("every corner has a UV");
            assert!((uv - DVec2::new(p.x, p.y)).length() < 1e-9);
        }
    }
}

#[test]
fn open_and_curved_targets_are_fine() {
    // DRILL-01: a half-open, bent surface (a cube missing its top) drilled
    // from above.
    let mut open = unit_cube();
    let top = open
        .faces
        .iter()
        .position(|f| {
            f.vertices
                .iter()
                .all(|&v| open.vertices[v as usize].y > 0.5)
        })
        .unwrap();
    open.faces.remove(top);
    let driver = outline_quad((-0.5, 0.5), (-0.5, 0.5), 5.);
    let (mut e, id) = scene(open, vec![driver]);
    axis(&mut e, DrillOp::Tunnel, DrillAxis::Y).unwrap();
    assert!(close(area(mesh(&e, id)), 19.));
}

#[test]
fn command_line_needs_mode_and_axis() {
    let (mut e, id) = scene(grid_target(), vec![unit_cube()]);
    let args =
        |s: &str| serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(s).unwrap();
    assert!(command_exec::run(&mut e, "poly.solidDrill", &args("{}")).is_err());
    command_exec::run(&mut e, "poly.solidDrill", &args(r#"{"mode": "core"}"#)).unwrap();
    assert_eq!(mesh(&e, id).faces.len(), 4);
    let (mut e, _) = scene(grid_target(), vec![unit_cube()]);
    assert!(command_exec::run(&mut e, "poly.drill", &args(r#"{"mode": 1}"#)).is_err());
    command_exec::run(&mut e, "poly.drill", &args(r#"{"mode": 1, "axis": "z"}"#)).unwrap();
}

/// Random turned, scaled and moved boxes through a dense curved sheet:
/// every run either succeeds with Slice = Core + Tunnel by area and no
/// gaps, or refuses with the scene unchanged (DRILL-19..21).
#[test]
fn random_drills_are_exact_or_refused() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut sheet = grid(12, -2., 2., (0, 1, 2), 0.);
    for v in &mut sheet.vertices {
        v.z = 0.3 * (v.x * 1.3).sin() * (v.y * 0.9).cos();
    }
    let (mut ok, mut refused) = (0, 0);
    for _ in 0..40 {
        let r = glam::DQuat::from_euler(glam::EulerRot::XYZ, rnd() * 6.3, rnd() * 6.3, rnd() * 6.3);
        let s = DVec3::new(0.4 + rnd(), 0.4 + rnd(), 0.4 + rnd());
        let t = DVec3::new(rnd() * 2. - 1., rnd() * 2. - 1., rnd() * 0.6 - 0.3);
        let mut driver = unit_cube();
        for v in &mut driver.vertices {
            *v = r * (*v * s) + t;
        }
        let mut areas = Vec::new();
        for op in [DrillOp::Slice, DrillOp::Core, DrillOp::Tunnel] {
            let (mut e, id) = scene(sheet.clone(), vec![driver.clone()]);
            let before = e.scene.clone();
            match solid(&mut e, op) {
                Ok(()) => {
                    let m = mesh(&e, id);
                    m.validate().unwrap();
                    assert!(edge_uses(m).values().all(|&n| n <= 2));
                    areas.push(Some(
                        m.faces.iter().map(|f| polygon_area(m, f)).sum::<f64>(),
                    ));
                }
                Err(_) => {
                    assert_eq!(e.scene.items, before.items);
                    assert!(e.undo.is_empty());
                    areas.push(None);
                }
            }
        }
        if let [Some(all), Some(core), Some(tunnel)] = areas[..] {
            assert!(
                (core + tunnel - all).abs() < 1e-6 * all,
                "{core} + {tunnel} != {all}"
            );
            ok += 1;
        } else {
            refused += 1;
        }
    }
    eprintln!("random drills: {ok} exact, {refused} refused");
    assert!(ok >= 36, "too many refusals: {refused}");
}
