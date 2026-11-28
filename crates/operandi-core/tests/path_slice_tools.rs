//! Edit tab waves 2 and 3 (WAVE2-PATH-SLICE, WAVE3-AXIS-SLICE). Acceptance
//! criteria:
//!
//! 1. Starting values match the spec (PS section 2, PS-21/22/30/40/44,
//!    AX-11).
//! 2. With no path (no points, no placed shape) the tools change nothing;
//!    Cancel leaves the scene as it was.
//! 3. Pen Slice cuts its path along the Axis through every surface it
//!    meets (top and bottom of a box), or only facing ones with Front Face
//!    Only; nothing moves.
//! 4. Primitive Slice cuts its shape into the clicked face along the face's
//!    normal (Action Axis); Lazy Apply waits for the release; Stencil tags
//!    the inside, Core keeps only the inside.
//! 5. Curve Slice cuts the curve sampled into Path Steps.
//! 6. Polygon Axis Slice cuts the selected polygons (all when none) along X,
//!    Y and Z planes over their bounding box.
//! 7. Each change is a tweak inside the tool; Drop is one step named after
//!    the tool; Undo/Redo; the result saves and reopens unchanged.
//! 8. Unbuilt operations are refused and change nothing; options carry to
//!    the next session, paths don't.
use glam::DVec3;
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

/// `n` x `n` unit quads in XZ (y = 0), facing +Y.
fn grid(n: u32) -> Mesh {
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(face(vec![a, a + n + 1, a + n + 2, a + 1]));
        }
    }
    mesh
}

fn cube() -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap()
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
fn params(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}
fn gesture(e: &mut Editor, p: Parameters) {
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
}
fn square(y: f64, h: f64) -> Vec<DVec3> {
    vec![
        DVec3::new(-h, y, -h),
        DVec3::new(h, y, -h),
        DVec3::new(h, y, h),
        DVec3::new(-h, y, h),
    ]
}

#[test]
fn starting_values_match_the_spec() {
    let c = PathCutter::default();
    assert_eq!(c.operation, SliceOperation::Slice);
    assert_eq!(c.axis, SliceAxis::Y);
    assert_eq!(c.vector, DVec3::Y);
    assert!(!c.front_face_only);
    assert_eq!(c.material, "Default");
    assert_eq!((c.negative_depth, c.positive_depth), (0., 1.));
    assert!(c.preview_geometry);
    assert!(!c.select_sliced_edges);
    assert_eq!(c.edge_set, "Sliced Edges");
    let pen = PenSlice::default();
    assert!(pen.closed);
    let curve = CurveSlice::default();
    assert_eq!(curve.steps, 24);
    assert!(!curve.auto_steps && !curve.closed);
    assert_eq!(curve.smooth_angle, 5.);
    let prim = PrimitiveSlice::default();
    assert_eq!(prim.shape, PrimitiveShape::Rectangle);
    assert_eq!((prim.half_x, prim.half_y, prim.radius), (0.5, 0.5, 0.5));
    assert_eq!((prim.sides, prim.corner_segments), (24, 8));
    assert_eq!((prim.clone_x, prim.clone_y), (1, 1));
    assert_eq!((prim.gap_x, prim.gap_y), (0.01, 0.01));
    assert!(prim.lazy_apply && !prim.edit_from_center && !prim.keep_aspect);
    assert_eq!(prim.cutter.axis, SliceAxis::ActionAxis);
    let axis = AxisSlice::default();
    assert_eq!(axis.mode, AxisSliceMode::Number);
    // Spec v1 (AX-11): Numbers start at 0 (no cuts until raised), Size 1 m.
    assert_eq!(axis.number, [0, 0, 0]);
    assert_eq!(axis.size, DVec3::ONE);
    assert_eq!(axis.offset, DVec3::ZERO);
    assert!(!axis.hold);
}

#[test]
fn no_path_changes_nothing_and_cancel_restores() {
    for tool in [Tool::PenSlice, Tool::CurveSlice, Tool::PrimitiveSlice] {
        let mut e = editor(cube(), Mode::Polygon);
        let before = mesh(&e).clone();
        e.dispatch(Command::Activate(tool)).unwrap();
        let p = params(&e);
        gesture(&mut e, p);
        assert_eq!(*mesh(&e), before, "{tool:?}");
        let mut p = params(&e);
        p.slice.pen_slice.points = square(1., 0.25);
        p.slice.curve_slice.points = square(1., 0.25);
        p.slice.curve_slice.closed = true;
        p.slice.primitive_slice.place = Some(ShapePlace::on(DVec3::new(0., 0.5, 0.), DVec3::Y));
        p.slice.primitive_slice.half_x = 0.25;
        p.slice.primitive_slice.half_y = 0.25;
        gesture(&mut e, p);
        assert_ne!(*mesh(&e), before, "{tool:?} cuts");
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(*mesh(&e), before, "{tool:?} cancel");
        assert!(e.session.live.is_none());
    }
}

#[test]
fn pen_slice_cuts_top_and_bottom_or_front_only_and_drop_is_one_step() {
    let mut e = editor(cube(), Mode::Polygon);
    let before = mesh(&e).clone();
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::PenSlice)).unwrap();
    let mut p = params(&e);
    p.slice.pen_slice.points = square(2., 0.25);
    gesture(&mut e, p);
    // A closed outline inside the top and inside the bottom: each becomes an
    // inner polygon and a ring in two pieces.
    assert_eq!(mesh(&e).faces.len(), 6 + 2 * 2);
    assert_eq!(&mesh(&e).vertices[..8], &before.vertices[..]);
    mesh(&e).validate().unwrap();
    let mut p = params(&e);
    p.slice.pen_slice.cutter.front_face_only = true;
    gesture(&mut e, p);
    assert_eq!(mesh(&e).faces.len(), 6 + 2);
    // Mid-tool undo takes back the option and keeps the tool.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e).faces.len(), 10);
    assert_eq!(
        e.session.live.as_ref().map(|l| l.tool),
        Some(Tool::PenSlice)
    );
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Pen Slice");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(*mesh(&e), before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(mesh(&e).faces.len(), 10);
}

#[test]
fn open_pen_path_across_a_grid_and_custom_axis() {
    let mut e = editor(grid(3), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::PenSlice)).unwrap();
    let mut p = params(&e);
    let o = &mut p.slice.pen_slice;
    o.closed = false;
    o.points = vec![DVec3::new(-1., 1., 0.5), DVec3::new(4., 1., 0.5)];
    gesture(&mut e, p);
    assert_eq!(mesh(&e).faces.len(), 9 + 3);
    // Looking along X the path is edge-on to nothing it can cut: the same
    // line seen along X lies across the grid's edge-on side.
    let mut p = params(&e);
    p.slice.pen_slice.cutter.axis = SliceAxis::Custom;
    p.slice.pen_slice.cutter.vector = DVec3::new(0., 1., 0.);
    gesture(&mut e, p);
    assert_eq!(mesh(&e).faces.len(), 12);
    let mut p = params(&e);
    p.slice.pen_slice.cutter.vector = DVec3::ZERO;
    assert!(e.dispatch(Command::Preview(p)).is_err());
}

#[test]
fn primitive_slice_cuts_the_face_along_its_normal_and_lazy_apply_waits() {
    let mut e = editor(cube(), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::PrimitiveSlice)).unwrap();
    let mut p = params(&e);
    let o = &mut p.slice.primitive_slice;
    // On the +X side face: the cut runs along X through both X sides.
    o.place = Some(ShapePlace::on(DVec3::new(0.5, 0., 0.), DVec3::X));
    o.half_x = 0.2;
    o.half_y = 0.2;
    o.held = true;
    gesture(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 6, "Lazy Apply: no cut while held");
    p.slice.primitive_slice.held = false;
    gesture(&mut e, p.clone());
    assert_eq!(mesh(&e).faces.len(), 10);
    assert!(
        mesh(&e)
            .vertices
            .iter()
            .any(|v| (v.x + 0.5).abs() < 1e-9 && v.y.abs() > 0.1 && v.y.abs() < 0.3)
    );
    // Circle with 12 sides.
    p.slice.primitive_slice.shape = PrimitiveShape::Circle;
    p.slice.primitive_slice.radius = 0.3;
    p.slice.primitive_slice.sides = 12;
    gesture(&mut e, p);
    assert_eq!(mesh(&e).vertices.len(), 8 + 2 * 12);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.last().unwrap().label, "Primitive Slice");
}

#[test]
fn stencil_and_core() {
    let mut e = editor(grid(4), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::PrimitiveSlice)).unwrap();
    let mut p = params(&e);
    let o = &mut p.slice.primitive_slice;
    o.place = Some(ShapePlace::on(DVec3::new(2., 0., 2.), DVec3::Y));
    o.half_x = 1.5;
    o.half_y = 1.5;
    o.cutter.operation = SliceOperation::Stencil;
    o.cutter.material = "Red".into();
    gesture(&mut e, p.clone());
    let red = mesh(&e)
        .faces
        .iter()
        .filter(|f| f.material == "Red")
        .count();
    assert_eq!(red, 4 + 12);
    p.slice.primitive_slice.cutter.operation = SliceOperation::Core;
    gesture(&mut e, p);
    assert_eq!(mesh(&e).faces.len(), 16);
    assert!(mesh(&e).faces.iter().all(|f| f.material == "M"));
}

#[test]
fn unbuilt_operation_is_refused_and_changes_nothing() {
    let mut e = editor(cube(), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::PenSlice)).unwrap();
    let mut p = params(&e);
    p.slice.pen_slice.points = square(2., 0.25);
    gesture(&mut e, p.clone());
    let cut = mesh(&e).clone();
    p.slice.pen_slice.cutter.operation = SliceOperation::Union;
    assert!(e.dispatch(Command::Preview(p)).is_err());
    assert_eq!(*mesh(&e), cut);
}

#[test]
fn curve_slice_cuts_the_sampled_curve_and_saves_and_reopens() {
    let mut e = editor(grid(4), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::CurveSlice)).unwrap();
    let mut p = params(&e);
    let o = &mut p.slice.curve_slice;
    o.points = vec![
        DVec3::new(-0.5, 0., 1.5),
        DVec3::new(2., 0., 2.5),
        DVec3::new(4.5, 0., 1.5),
    ];
    o.steps = 8;
    gesture(&mut e, p.clone());
    let eight = mesh(&e).clone();
    eight.validate().unwrap();
    assert!(eight.faces.len() > 16);
    // More steps, more points on the cut.
    p.slice.curve_slice.steps = 32;
    gesture(&mut e, p);
    assert!(mesh(&e).vertices.len() > eight.vertices.len());
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.last().unwrap().label, "Curve Slice");
    let dir = std::env::temp_dir().join(format!("operandi-path-slice-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("curve.opscene");
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    assert_eq!(back.active().unwrap().mesh, e.scene.active().unwrap().mesh);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn axis_slice_cuts_the_selection_box_and_drop_is_one_step() {
    let mut e = editor(grid(4), Mode::Polygon);
    // The middle 2 x 2 polygons: x and z 1..3.
    e.scene.selection.faces = vec![5, 6, 9, 10];
    let before = mesh(&e).clone();
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::AxisSlice)).unwrap();
    // Number X 1 at activation: the cut at x = 2 lies on existing points,
    // so nothing is split yet.
    let mut p = params(&e);
    p.slice.axis_slice.number = [3, 0, 1];
    gesture(&mut e, p.clone());
    // X cuts at 1.5, 2 (on points), 2.5; Z at 2 (on points).
    assert_eq!(mesh(&e).faces.len(), 16 + 4);
    // The selected polygons' pieces stay selected.
    assert_eq!(e.scene.selection.faces.len(), 8);
    p.slice.axis_slice.offset = DVec3::new(0.25, 0., 0.);
    gesture(&mut e, p.clone());
    // 1.75, 2.25, 2.75 across both columns' rows.
    assert_eq!(mesh(&e).faces.len(), 16 + 6);
    p.slice.axis_slice.hold = true;
    gesture(&mut e, p);
    assert_eq!(*mesh(&e), before);
    let mut p = params(&e);
    p.slice.axis_slice.hold = false;
    gesture(&mut e, p);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Polygon Axis Slice");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(*mesh(&e), before);
}

#[test]
fn options_carry_to_the_next_session_but_paths_do_not() {
    let mut e = editor(cube(), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::PenSlice)).unwrap();
    let mut p = params(&e);
    p.slice.pen_slice.points = square(2., 0.25);
    p.slice.pen_slice.closed = true;
    p.slice.pen_slice.cutter.front_face_only = true;
    gesture(&mut e, p);
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Activate(Tool::PenSlice)).unwrap();
    let p = params(&e);
    assert!(p.slice.pen_slice.points.is_empty());
    assert!(p.slice.pen_slice.cutter.front_face_only);
}
