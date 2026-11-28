//! Path tools (CURVE-EXTRUDE-PEN-CLONE v1, CX-xx; SPEC-CURVE-CLONE v1):
//! acceptance criteria from PATH_TOOLS_ACCEPTANCE.md.
//!
//! CE1 smooth path, sections on the middle and last points; CE2 17
//! polygons with the start open; CE3 Align to Path; CE4 Start at Source;
//! CE5 end cap selected; CE6 Cap Start on / Cap End off; CE7 several
//! polygons; CE10 Automatic. PC1–PC4 Pen Clone; CC1–CC3 Curve Clone.
//! Shared: empty selection refused, Cancel, one undo step, save/reopen,
//! Symmetry, presets.
use glam::DVec3;
use operandi_core::path_tools::{self, PathKind, PathTool};
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "M".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// CX test polygon: a 0.2 × 0.2 quad in the XZ plane at the origin,
/// normal +Y.
fn quad_at(c: DVec3) -> Mesh {
    let h = 0.1;
    let mut m = Mesh {
        vertices: vec![
            c + DVec3::new(-h, 0., -h),
            c + DVec3::new(-h, 0., h),
            c + DVec3::new(h, 0., h),
            c + DVec3::new(h, 0., -h),
        ],
        ..Default::default()
    };
    m.faces.push(face(vec![0, 1, 2, 3]));
    m.validate().unwrap();
    m
}

fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}
fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn path3() -> Vec<DVec3> {
    vec![DVec3::ZERO, DVec3::new(0., 1., 0.), DVec3::new(1., 2., 0.)]
}

/// The Curve Extrude preset's values (CX-01): Start at Source on, Cap
/// Start off, Invert on.
fn extrude_preset(points: Vec<DVec3>, steps: u32) -> PathTool {
    let mut t = PathTool::default();
    t.curve.kind = PathKind::Curve;
    t.curve.points = points;
    t.steps.number = steps;
    t.steps.start_at_source = true;
    t.sweep.cap_start = false;
    t.sweep.invert = true;
    t
}

fn run(e: &mut Editor, tool: Tool, set: impl FnOnce(&mut DuplicateParameters)) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    set(&mut p.duplicate);
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
}

/// Points of the ring of four at section `k` (each section adds 4 points).
fn ring(m: &Mesh, k: usize) -> Vec<DVec3> {
    m.vertices[4 * k..4 * k + 4].to_vec()
}
fn centre(ps: &[DVec3]) -> DVec3 {
    ps.iter().sum::<DVec3>() / ps.len() as f64
}

/// Signed volume of the closed part plus open ends: positive when walls
/// face out.
fn wall_faces_out(m: &Mesh) -> bool {
    // Each wall's normal points away from the path's centre line locally:
    // checked against the polygon centre minus the ring centre it sits on.
    m.faces.iter().enumerate().all(|(f, _)| {
        let n = m.normal(f);
        let c = m.center(f);
        let nearest = m
            .vertices
            .chunks(4)
            .map(centre)
            .min_by(|a, b| a.distance(c).total_cmp(&b.distance(c)))
            .unwrap();
        // Caps sit on their ring centre; walls are off it.
        (c - nearest).length() < 1e-6 || n.dot(c - nearest) > 0.
    })
}

#[test]
fn ce1_path_is_smooth_and_sections_land_on_the_points() {
    let t = extrude_preset(path3(), 4);
    let s = path_tools::sections(&t, true).unwrap();
    assert_eq!(s.len(), 5);
    assert!(s[2].position.distance(DVec3::new(0., 1., 0.)) < 1e-9);
    assert!(s[4].position.distance(DVec3::new(1., 2., 0.)) < 1e-9);
    // CX-10: between the first two points the curve bulges to x ≈ −0.098
    // at a quarter of the way.
    let quarter = curves::point_at(&path3(), t.curve.curve_kind(), false, 0.25).unwrap();
    assert!(quarter.x < -0.05, "{quarter}");
}

#[test]
fn ce2_ce5_seventeen_polygons_start_open_end_cap_selected() {
    let mut e = editor(quad_at(DVec3::ZERO), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    run(&mut e, Tool::CurveExtrude, |d| {
        d.curve_extrude = extrude_preset(path3(), 4)
    });
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 17, "16 sides + end cap");
    assert_eq!(m.vertices.len(), 20);
    // The start is open: no polygon on the first ring alone.
    let first: Vec<u32> = (0..4).collect();
    assert!(
        !m.faces
            .iter()
            .any(|f| f.vertices.iter().all(|v| first.contains(v)))
    );
    // CX-15: the end cap is selected, and it sits on the last point.
    assert_eq!(e.scene.selection.faces.len(), 1);
    let cap = e.scene.selection.faces[0] as usize;
    assert!(m.center(cap).distance(DVec3::new(1., 2., 0.)) < 1e-9);
    assert!(wall_faces_out(m));
    // CX-Q8: the cap faces the source's normal turned by the path's
    // accumulated tangent turn, (0.9502, 0.3115, 0), not the end tangent.
    let n = m.normal(cap);
    assert!(n.distance(DVec3::new(0.9502, 0.3115, 0.)) < 1e-4, "{n}");
}

#[test]
fn ce3_align_to_path_turns_sections_off_only_moves() {
    let t = extrude_preset(path3(), 4);
    let r = path_tools::sweep_geometry(&quad_at(DVec3::ZERO), &[0], &t).unwrap();
    let last = ring(&r.mesh, 4);
    let n = (last[1] - last[0]).cross(last[3] - last[0]).normalize();
    // CX-Q8: each section is the previous one turned by the smallest
    // rotation between their tangents, carried from the source quad (normal
    // +Y), so the last faces +Y turned by the whole tangent turn.
    let path = path_tools::sections(&t, true).unwrap();
    let mut carried = glam::DQuat::IDENTITY;
    for k in 1..path.len() {
        carried = glam::DQuat::from_rotation_arc(path[k - 1].tangent, path[k].tangent) * carried;
    }
    let want = carried * DVec3::Y;
    assert!(n.dot(want).abs() > 0.999999, "{n} vs {want}");
    assert!(
        want.distance(DVec3::new(0.9502, 0.3115, 0.)) < 1e-4,
        "{want}"
    );
    let mut off = t.clone();
    off.steps.align_to_path = false;
    let r = path_tools::sweep_geometry(&quad_at(DVec3::ZERO), &[0], &off).unwrap();
    let last = ring(&r.mesh, 4);
    let n = (last[1] - last[0]).cross(last[3] - last[0]).normalize();
    assert!(n.dot(DVec3::Y).abs() > 0.999999, "{n}");
    assert!(centre(&last).distance(DVec3::new(1., 2., 0.)) < 1e-9);
}

#[test]
fn ce4_start_at_source_moves_the_path_onto_the_selection() {
    let at = DVec3::new(3., 0., 0.);
    let t = extrude_preset(path3(), 4);
    let r = path_tools::sweep_geometry(&quad_at(at), &[0], &t).unwrap();
    assert!(centre(&ring(&r.mesh, 4)).distance(at + DVec3::new(1., 2., 0.)) < 1e-9);
    let mut off = t.clone();
    off.steps.start_at_source = false;
    let r = path_tools::sweep_geometry(&quad_at(at), &[0], &off).unwrap();
    assert!(centre(&ring(&r.mesh, 4)).distance(DVec3::new(1., 2., 0.)) < 1e-9);
}

#[test]
fn ce6_cap_start_keeps_the_original_cap_end_off_opens_the_far_end() {
    let mut t = extrude_preset(path3(), 4);
    t.sweep.cap_start = true;
    t.sweep.cap_end = false;
    let r = path_tools::sweep_geometry(&quad_at(DVec3::ZERO), &[0], &t).unwrap();
    assert_eq!(r.mesh.faces.len(), 17);
    // The original polygon is still face 0, now facing back down the path.
    assert_eq!(r.mesh.faces[0].vertices.len(), 4);
    assert!(r.mesh.normal(0).dot(DVec3::Y) < -0.99);
    // The start cap is selected afterwards (2026-11-20).
    assert_eq!(r.end_caps, [0]);
}

#[test]
fn ce7_several_polygons_each_sweep_from_where_they_sit() {
    let mut m = quad_at(DVec3::ZERO);
    let other = quad_at(DVec3::new(2., 0., 0.));
    append_mesh(&mut m, other).unwrap();
    let t = extrude_preset(path3(), 4);
    let r = path_tools::sweep_geometry(&m, &[0, 1], &t).unwrap();
    assert_eq!(r.mesh.faces.len(), 34);
    assert_eq!(r.end_caps.len(), 2);
    let mut ends: Vec<DVec3> = r
        .end_caps
        .iter()
        .map(|&f| r.mesh.center(f as usize))
        .collect();
    ends.sort_by(|a, b| a.x.total_cmp(&b.x));
    assert!(ends[0].distance(DVec3::new(1., 2., 0.)) < 1e-9);
    assert!(ends[1].distance(DVec3::new(3., 2., 0.)) < 1e-9);
}

#[test]
fn ce8_bezier_and_b_spline_paths_sweep_too() {
    for kind in [PathKind::Bezier, PathKind::BSpline] {
        let mut t = extrude_preset(
            vec![
                DVec3::ZERO,
                DVec3::new(0., 0.5, 0.),
                DVec3::new(0.5, 1.5, 0.),
                DVec3::new(1., 2., 0.),
            ],
            6,
        );
        t.curve.kind = kind;
        let r = path_tools::sweep_geometry(&quad_at(DVec3::ZERO), &[0], &t).unwrap();
        assert_eq!(r.mesh.faces.len(), 6 * 4 + 1, "{kind:?}");
        let end = r.mesh.center(r.end_caps[0] as usize);
        assert!(
            end.distance(DVec3::new(1., 2., 0.)) < 1e-9,
            "{kind:?} {end}"
        );
    }
}

#[test]
fn ce10_automatic_puts_more_sections_where_the_path_bends() {
    let mut t = extrude_preset(
        vec![DVec3::ZERO, DVec3::new(0., 1., 0.), DVec3::new(0., 2., 0.)],
        4,
    );
    t.steps.automatic = true;
    // A straight path: one section per span.
    assert_eq!(path_tools::sections(&t, true).unwrap().len(), 3);
    t.curve.points = path3();
    let bent = path_tools::sections(&t, true).unwrap().len();
    assert!(bent > 3, "{bent}");
}

/// A 0.2 cube centred at the origin.
fn cube(size: f64) -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::splat(size), 0).unwrap()
}

fn pen_clone(points: Vec<DVec3>) -> PathTool {
    let mut t = PathTool::default();
    t.curve.kind = PathKind::Pen;
    t.curve.points = points;
    t
}

/// The centres of each 8-point cube copy.
fn cube_centres(m: &Mesh) -> Vec<DVec3> {
    m.vertices.chunks(8).map(centre).collect()
}

#[test]
fn pc1_pc2_pen_clone_places_bottoms_on_the_points() {
    let mut e = editor(cube(0.2), Mode::Polygon);
    run(&mut e, Tool::CurveClone, |d| {
        d.curve_clone = pen_clone(path3());
        d.curve_clone.pen_align = false;
    });
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 24);
    assert_eq!(m.faces.len(), 18);
    let c = cube_centres(m);
    assert!(c[0].distance(DVec3::ZERO) < 1e-9, "the original stays");
    assert!(c[1].distance(DVec3::new(0., 1.1, 0.)) < 1e-9, "{}", c[1]);
    assert!(c[2].distance(DVec3::new(1., 2.1, 0.)) < 1e-9, "{}", c[2]);
}

/// The direction a copy's local +Y now points (from its bottom to top
/// face centres).
fn up_of(m: &Mesh, copy: usize) -> DVec3 {
    let ps = &m.vertices[copy * 8..copy * 8 + 8];
    // An edge that ran along +Y in the source.
    let src = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(0.2), 0).unwrap();
    let (i, j) = (0..8)
        .flat_map(|i| (0..8).map(move |j| (i, j)))
        .find(|&(i, j)| {
            (src.vertices[j] - src.vertices[i]).distance(DVec3::new(0., 0.2, 0.)) < 1e-9
        })
        .unwrap();
    (ps[j] - ps[i]).normalize()
}

#[test]
fn pc3_align_turns_inner_copies_to_the_average_direction() {
    let mut e = editor(cube(0.2), Mode::Polygon);
    run(&mut e, Tool::CurveClone, |d| {
        d.curve_clone = pen_clone(path3())
    });
    let m = mesh(&e);
    let angle = |v: DVec3| v.angle_between(DVec3::Y).to_degrees();
    assert!(angle(up_of(m, 0)) < 1e-9, "original unturned");
    assert!(
        (angle(up_of(m, 1)) - 22.5).abs() < 1e-6,
        "{}",
        angle(up_of(m, 1))
    );
    assert!(
        (angle(up_of(m, 2)) - 45.).abs() < 1e-6,
        "{}",
        angle(up_of(m, 2))
    );
}

#[test]
fn pc4_replace_source_moves_the_original_to_the_first_point() {
    let mut e = editor(cube(0.2), Mode::Polygon);
    e.scene.active_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::new(5., 0., 0.), DVec3::splat(0.2), 0)
            .unwrap()
            .into();
    run(&mut e, Tool::CurveClone, |d| {
        d.curve_clone = pen_clone(path3());
        d.curve_clone.pen_align = false;
        d.curve_clone.clone.copies.replace_source = true;
    });
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 24);
    let mut c = cube_centres(m);
    c.sort_by(|a, b| a.y.total_cmp(&b.y));
    assert!(c[0].distance(DVec3::new(0., 0.1, 0.)) < 1e-9, "{}", c[0]);
}

#[test]
fn cc1_curve_clone_by_length_from_the_source() {
    let mut t = PathTool::default();
    t.curve.points = vec![DVec3::ZERO, DVec3::X];
    t.steps.number = 3;
    t.curve.by_length = true;
    t.steps.start_at_source = true;
    t.steps.align_to_path = false;
    let mut e = editor(cube(0.1), Mode::Polygon);
    run(&mut e, Tool::CurveClone, |d| d.curve_clone = t);
    let mut xs: Vec<f64> = cube_centres(mesh(&e)).iter().map(|c| c.x).collect();
    xs.sort_by(f64::total_cmp);
    assert_eq!(xs.len(), 3);
    for (x, want) in xs.iter().zip([0., 0.5, 1.]) {
        assert!((x - want).abs() < 1e-6, "{xs:?}");
    }
}

#[test]
fn cc2_arc_of_seven_copies_turned_to_the_tangent() {
    let mut t = PathTool::default();
    t.curve.points = vec![DVec3::ZERO, DVec3::new(1., 0., 1.), DVec3::new(2., 0., 0.)];
    t.steps.number = 7;
    t.steps.start_at_source = true;
    let mut e = editor(cube(0.1), Mode::Polygon);
    run(&mut e, Tool::CurveClone, |d| d.curve_clone = t.clone());
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 7 * 8);
    // Copies stay upright (they turn about Y) and the middle one faces +X.
    let s = path_tools::sections(&t, false).unwrap();
    assert!(s[3].tangent.distance(DVec3::X) < 1e-4, "{}", s[3].tangent);
}

#[test]
fn cc3_closed_bank_and_curve_polygon() {
    let mut t = PathTool::default();
    t.curve.points = vec![
        DVec3::ZERO,
        DVec3::new(1., 0., 0.),
        DVec3::new(1., 0., 1.),
        DVec3::new(0., 0., 1.),
    ];
    t.curve.closed = true;
    t.steps.number = 8;
    t.curve.create_curve_polygon = true;
    let mut e = editor(cube(0.1), Mode::Polygon);
    run(&mut e, Tool::CurveClone, |d| d.curve_clone = t.clone());
    let m = mesh(&e);
    // 8 copies round the loop (the source is the first) + the curve.
    assert_eq!(m.faces.len(), 8 * 6 + 1);
    assert!(m.is_polyline(m.faces.len() - 1));
    // Bank rolls every copy about the path.
    let s = path_tools::sections(&t, false).unwrap();
    assert_eq!(s.len(), 8);
}

#[test]
fn shared_empty_selection_cancel_undo_and_reopen() {
    // Empty polygon selection: Curve Extrude says so and changes nothing.
    let mut e = editor(quad_at(DVec3::ZERO), Mode::Polygon);
    let before = mesh(&e).clone();
    e.dispatch(Command::Activate(Tool::CurveExtrude)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.duplicate.curve_extrude = extrude_preset(path3(), 4);
    assert!(e.dispatch(Command::Preview(p)).is_err() || mesh(&e) == &before);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(mesh(&e), &before);
    // One undo step after Drop; save and reopen keep the result.
    e.scene.selection.faces = vec![0];
    run(&mut e, Tool::CurveExtrude, |d| {
        d.curve_extrude = extrude_preset(path3(), 4)
    });
    let after = mesh(&e).clone();
    assert_eq!(after.faces.len(), 17);
    let dir = std::env::temp_dir().join(format!("op-path-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("extrude.opscene");
    e.save_committed(&file).unwrap();
    let back = persistence::load(&file).unwrap();
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(back.items[0].mesh, after);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), &before);
    // The path isn't kept for the next session; the options are.
    e.dispatch(Command::Activate(Tool::CurveExtrude)).unwrap();
    let p = &e
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .duplicate
        .curve_extrude;
    assert!(p.curve.points.is_empty());
    assert_eq!(p.steps.number, 4);
}

#[test]
fn shared_symmetry_mirrors_the_sweep() {
    let mut m = quad_at(DVec3::new(1., 0., 0.));
    append_mesh(&mut m, quad_at(DVec3::new(-1., 0., 0.))).unwrap();
    let mut e = editor(m, Mode::Polygon);
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.scene.selection.faces = vec![0, 1];
    let path = vec![
        DVec3::new(1., 0., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(2., 2., 0.),
    ];
    run(&mut e, Tool::CurveExtrude, |d| {
        d.curve_extrude = extrude_preset(path, 4)
    });
    let m = mesh(&e);
    let caps: Vec<DVec3> = e
        .scene
        .selection
        .faces
        .iter()
        .map(|&f| m.center(f as usize))
        .collect();
    assert_eq!(caps.len(), 2);
    assert!(
        caps.iter()
            .any(|c| c.distance(DVec3::new(2., 2., 0.)) < 1e-9),
        "{caps:?}"
    );
    assert!(
        caps.iter()
            .any(|c| c.distance(DVec3::new(-2., 2., 0.)) < 1e-9),
        "{caps:?}"
    );
}

#[test]
fn ce9_pen_extrude_makes_one_section_per_point_turned_along_the_path() {
    // The button: Pen kind, Cap Start off, Cap End on (hands-on 2026-11-17).
    let mut e = editor(quad_at(DVec3::ZERO), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    e.dispatch(Command::ActivatePreset("PenExtrude".into()))
        .unwrap();
    assert_eq!(e.live_tool_label(), Some("Pen Extrude"));
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    let t = &p.duplicate.curve_extrude;
    assert_eq!(t.curve.kind, PathKind::Pen);
    assert!(!t.sweep.cap_start && t.sweep.cap_end);
    p.duplicate.curve_extrude.curve.points = path3();
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    // Two sections after the polygon: 2 rings of 4 walls + the end cap,
    // with no in-between steps.
    assert_eq!(m.faces.len(), 9);
    assert_eq!(m.vertices.len(), 12);
    // The middle section sits on the middle point, turned halfway between
    // the two segments (22.5°); the last faces along the last segment.
    let mid = ring(m, 1);
    assert!(centre(&mid).distance(DVec3::new(0., 1., 0.)) < 1e-9);
    let n = |r: &[DVec3]| (r[1] - r[0]).cross(r[3] - r[0]).normalize();
    assert!((n(&mid).angle_between(DVec3::Y).to_degrees() - 22.5).abs() < 1e-6);
    let last = ring(m, 2);
    assert!((n(&last).angle_between(DVec3::Y).to_degrees() - 45.).abs() < 1e-6);
    assert!(centre(&last).distance(DVec3::new(1., 2., 0.)) < 1e-9);
}

/// Every cap Curve Extrude makes is selected afterwards, with Align to
/// Path on or off (2026-11-20): start only, end only, both, or none. The
/// start cap is the source polygon, left in place with its winding
/// reversed; the end cap sits on the last point.
#[test]
fn the_cap_selected_after_follows_the_caps_made() {
    for align in [true, false] {
        for (cap_start, cap_end) in [(true, false), (true, true), (false, true), (false, false)] {
            let mut e = editor(quad_at(DVec3::ZERO), Mode::Polygon);
            e.scene.selection.faces = vec![0];
            let source = mesh(&e).faces[0].vertices.clone();
            run(&mut e, Tool::CurveExtrude, |d| {
                let mut t = extrude_preset(path3(), 4);
                t.sweep.cap_start = cap_start;
                t.sweep.cap_end = cap_end;
                t.steps.align_to_path = align;
                d.curve_extrude = t;
            });
            let m = mesh(&e);
            let case = format!("align {align} start {cap_start} end {cap_end}");
            let selected = &e.scene.selection.faces;
            assert_eq!(
                selected.len(),
                usize::from(cap_start) + usize::from(cap_end),
                "{case}"
            );
            let mut back = source.clone();
            back.reverse();
            let is_start = |f: u32| {
                let got = &m.faces[f as usize].vertices;
                (0..4).any(|k| [got[k..].to_vec(), got[..k].to_vec()].concat() == back)
            };
            let is_end = |f: u32| m.center(f as usize).distance(DVec3::new(1., 2., 0.)) < 1e-9;
            assert_eq!(selected.iter().any(|&f| is_start(f)), cap_start, "{case}");
            assert_eq!(selected.iter().any(|&f| is_end(f)), cap_end, "{case}");
        }
    }
}
