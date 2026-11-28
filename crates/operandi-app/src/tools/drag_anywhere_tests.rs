//! A click-drag anywhere in the viewport drives the live tool, not only a
//! drag on its handle (Jason, 2026-11-04; DRAG-Q01): up/down acts like the
//! move arrow, left/right like the scale widget. Each press-to-release is
//! one undo step and the tool stays active.
use super::tests::{draw, pointer};
use super::*;

/// 3×3 quads in z = 0 facing +z; point `r * 4 + c` at (c, r, 0).
fn grid() -> operandi_core::Mesh {
    let mut m = operandi_core::Mesh {
        vertices: (0..16)
            .map(|i| DVec3::new((i % 4) as f64, (i / 4) as f64, 0.))
            .collect(),
        ..Default::default()
    };
    for r in 0..3u32 {
        for c in 0..3u32 {
            let i = r * 4 + c;
            m.faces.push(operandi_core::Face {
                vertices: vec![i, i + 1, i + 5, i + 4],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    m
}

struct Rig {
    a: Operandi,
    ctx: Context,
    size: Vec2,
}
impl Rig {
    fn new(mode: Mode) -> Self {
        let ctx = Context::default();
        theme::install(&ctx);
        let size = vec2(1440., 900.);
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid().into();
        a.camera.view = View::Front;
        a.run(Command::Mode(mode));
        match mode {
            Mode::Polygon => a.editor.scene.selection.faces = vec![4],
            Mode::Edge => a.editor.scene.selection.edges = vec![(5, 6)],
            _ => a.editor.scene.selection.vertices = vec![5],
        }
        a.editor.history.undo.clear();
        let mut rig = Self { a, ctx, size };
        rig.frame(vec![]);
        rig
    }
    fn frame(&mut self, events: Vec<Event>) {
        draw(&mut self.a, &self.ctx, self.size, events);
    }
    /// A corner of the viewport, away from the mesh and every handle.
    fn corner(&self) -> Pos2 {
        self.a.viewport_rect.left_top() + vec2(80., 80.)
    }
    fn activate(&mut self, tool: Tool) {
        self.a.activate(tool);
        assert_eq!(
            self.a.editor.live.as_ref().map(|l| l.tool),
            Some(tool),
            "{}",
            self.a.status
        );
        self.frame(vec![]);
    }
    fn drag(&mut self, from: Pos2, by: Vec2) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![pointer(from, true, Modifiers::NONE)]);
        for step in 1..=4 {
            self.frame(vec![Event::PointerMoved(from + by * (step as f32 / 4.))]);
        }
        self.frame(vec![pointer(from + by, false, Modifiers::NONE)]);
    }
    fn params(&self) -> Parameters {
        self.a.editor.live.as_ref().unwrap().params.clone()
    }
}

#[test]
fn bevel_up_shifts_and_right_grows_the_cap() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.activate(Tool::Bevel);
    let corner = rig.corner();
    assert_eq!(rig.a.handle_hit(corner, rig.a.viewport_rect), None);
    let base = rig.params();
    rig.drag(corner, vec2(0., -60.));
    let up = rig.params();
    assert!(up.shift > base.shift, "{}", rig.a.status);
    assert_eq!(up.inset, base.inset);
    rig.drag(corner, vec2(40., 0.));
    let right = rig.params();
    assert_eq!(right.shift, up.shift);
    assert!(right.inset < up.inset, "out enlarges the cap");
    // Each drag is one step; the tool stays.
    rig.a.run(Command::Undo);
    assert_eq!(rig.params().inset, up.inset);
    assert_eq!(
        rig.a.editor.live.as_ref().map(|l| l.tool),
        Some(Tool::Bevel)
    );
}

#[test]
fn modified_drags_off_the_handles_stay_blocked() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.activate(Tool::Bevel);
    let corner = rig.corner();
    let base = rig.params();
    rig.frame(vec![pointer(corner, true, Modifiers::CTRL)]);
    rig.frame(vec![Event::PointerMoved(corner + vec2(0., -60.))]);
    rig.frame(vec![pointer(
        corner + vec2(0., -60.),
        false,
        Modifiers::CTRL,
    )]);
    assert_eq!(rig.params().shift, base.shift);
}

#[test]
fn smooth_shift_offsets_up_and_scales_right() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.activate(Tool::SmoothShift);
    let corner = rig.corner();
    // The first click starts it, as before.
    rig.drag(corner, vec2(0., 0.));
    assert!(
        rig.a.editor.live.as_ref().unwrap().applied,
        "{}",
        rig.a.status
    );
    let base = rig.params();
    rig.drag(corner, vec2(0., -50.));
    let up = rig.params();
    assert!(up.smooth_shift.offset > base.smooth_shift.offset);
    rig.drag(corner, vec2(30., 0.));
    assert!(rig.params().smooth_shift.scale > up.smooth_shift.scale);
}

#[test]
fn edge_tools_drag_anywhere() {
    // Extrude: up = Extrude, right = Width.
    let mut rig = Rig::new(Mode::Edge);
    // On a flat grid it needs both at once to make valid polygons.
    rig.activate(Tool::EdgeExtrude);
    rig.drag(rig.corner(), vec2(40., -50.));
    assert!(rig.params().inset > 0., "{}", rig.a.status);
    assert!(rig.params().shift > 0., "{}", rig.a.status);

    // One size-like value: left/right.
    for tool in [Tool::EdgeBevel, Tool::EdgeChamfer] {
        let mut rig = Rig::new(Mode::Edge);
        rig.activate(tool);
        rig.drag(rig.corner(), vec2(40., 0.));
        assert!(
            rig.params().chamfer.offset > 0.,
            "{tool:?}: {}",
            rig.a.status
        );
    }
    let mut rig = Rig::new(Mode::Edge);
    rig.activate(Tool::EdgeCrease);
    rig.drag(rig.corner(), vec2(40., 0.));
    assert!(rig.params().shift > 0., "{}", rig.a.status);

    // Slide follows its rail; Extend moves freely.
    let mut rig = Rig::new(Mode::Edge);
    rig.activate(Tool::EdgeSlide);
    let corner = rig.corner();
    rig.drag(corner, vec2(0., -40.));
    rig.drag(corner, vec2(40., 0.));
    assert_ne!(rig.params().shift, 0., "{}", rig.a.status);
    let mut rig = Rig::new(Mode::Edge);
    rig.a.editor.scene.selection.edges = vec![(0, 1)];
    rig.activate(Tool::EdgeExtend);
    rig.drag(rig.corner(), vec2(30., -30.));
    assert_ne!(rig.params().delta, DVec3::ZERO, "{}", rig.a.status);
}

#[test]
fn vertex_tools_drag_anywhere() {
    let mut rig = Rig::new(Mode::Vertex);
    rig.activate(Tool::VertexExtrude);
    let corner = rig.corner();
    rig.drag(corner, vec2(40., 0.));
    assert!(rig.params().inset > 0., "{}", rig.a.status);
    rig.drag(corner, vec2(0., -50.));
    assert!(rig.params().shift > 0., "{}", rig.a.status);

    let mut rig = Rig::new(Mode::Vertex);
    rig.activate(Tool::VertexMerge);
    rig.drag(rig.corner(), vec2(40., 0.));
    assert!(rig.params().shift > 0., "{}", rig.a.status);

    let mut rig = Rig::new(Mode::Vertex);
    rig.a.editor.scene.selection.faces = vec![4];
    rig.activate(Tool::VertexTear);
    rig.drag(rig.corner(), vec2(30., -30.));
    assert_ne!(rig.params().delta, DVec3::ZERO, "{}", rig.a.status);
}

#[test]
fn poly_haul_hauls_from_anywhere() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.activate(Tool::PolyHaul);
    let corner = rig.corner();
    assert_eq!(rig.a.pick_face(corner, rig.a.viewport_rect), None);
    rig.drag(corner, vec2(0., -40.));
    assert!(rig.params().poly_haul_offset > 0., "{}", rig.a.status);
}

#[test]
fn a_drawn_sphere_resizes_from_anywhere() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.a.run(Command::Drop);
    rig.a.activate(Tool::Sphere);
    rig.a.camera.view = View::Top;
    rig.frame(vec![]);
    let center = rig.a.viewport_rect.center();
    rig.drag(center, vec2(80., 0.));
    let drawn = rig.params().radial.unwrap().radius;
    assert!(drawn.min_element() > 0., "{}", rig.a.status);
    rig.drag(rig.corner(), vec2(-30., -30.));
    assert_ne!(
        rig.params().radial.unwrap().radius,
        drawn,
        "{}",
        rig.a.status
    );
}

#[test]
fn numeric_sessions_drag_anywhere() {
    // Spike: up lengthens.
    let mut rig = Rig::new(Mode::Polygon);
    rig.a.run(Command::ArmSpike);
    rig.frame(vec![]);
    let before = rig.a.editor.spike.as_ref().unwrap().requested;
    rig.drag(rig.corner(), vec2(0., -50.));
    assert!(
        rig.a.editor.spike.as_ref().unwrap().requested > before,
        "{}",
        rig.a.status
    );

    // Reduction: the press initializes, left lowers Number. Nothing selected
    // reduces the whole mesh (a partial selection counts only itself).
    let mut rig = Rig::new(Mode::Polygon);
    rig.a.editor.scene.selection.faces.clear();
    rig.a.run(Command::ArmReduction);
    rig.frame(vec![]);
    rig.drag(rig.corner(), vec2(-60., 0.));
    let session = rig.a.editor.reduction.as_ref().unwrap();
    let count = session
        .source
        .as_ref()
        .map(|s| s.count)
        .unwrap_or_else(|| panic!("{}", rig.a.status));
    assert!(session.parameters.number < count, "{}", rig.a.status);

    // Torus: drawn from the press like the Cone and Ellipsoid.
    let mut rig = Rig::new(Mode::Polygon);
    rig.a
        .run(Command::ArmShape(operandi_core::ShapeKind::Torus));
    rig.frame(vec![]);
    let center = rig.a.viewport_rect.center();
    rig.drag(center, vec2(120., 0.));
    let ShapeParameters::Torus(t) = &rig.a.editor.shape.as_ref().unwrap().requested else {
        panic!("torus")
    };
    assert!(t.ring_radius > 0.5, "{}", rig.a.status);
}

/// Tube: each click adds the next control point; it doesn't restart the
/// tube at the new spot (Jason, 2026-11-04). A drag moves the point it
/// added, and one Undo takes back that whole click.
#[test]
fn tube_clicks_add_control_points() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.a.run(Command::ArmShape(operandi_core::ShapeKind::Tube));
    rig.frame(vec![]);
    let center = rig.a.viewport_rect.center();
    let points = |rig: &Rig| match &rig.a.editor.shape.as_ref().unwrap().requested {
        ShapeParameters::Tube(t) => t.points.clone(),
        _ => panic!("tube"),
    };
    let click = |rig: &mut Rig, at: Pos2| {
        rig.frame(vec![Event::PointerMoved(at)]);
        rig.frame(vec![pointer(at, true, Modifiers::NONE)]);
        rig.frame(vec![pointer(at, false, Modifiers::NONE)]);
    };
    click(&mut rig, center);
    assert_eq!(points(&rig).len(), 1, "{}", rig.a.status);
    assert!(rig.a.editor.shape.as_ref().unwrap().error.is_none());
    click(&mut rig, center + vec2(0., -100.));
    click(&mut rig, center + vec2(100., -150.));
    let p = points(&rig);
    assert_eq!(p.len(), 3, "{}", rig.a.status);
    assert!(p[0].distance(p[1]) > 0.1 && p[1].distance(p[2]) > 0.1);
    assert!(rig.a.editor.shape.as_ref().unwrap().preview.is_some());

    // A drag adds a point where it starts and moves it to where it ends.
    rig.drag(center + vec2(200., 0.), vec2(0., 100.));
    let p = points(&rig);
    assert_eq!(p.len(), 4, "{}", rig.a.status);
    rig.a.run(Command::Undo);
    assert_eq!(points(&rig).len(), 3, "{}", rig.a.status);
    rig.a.run(Command::Undo);
    assert_eq!(points(&rig).len(), 2, "{}", rig.a.status);
}

#[test]
fn tube_clicks_add_points_and_a_press_on_one_moves_it() {
    // Jason, 2026-11-05: each click adds a point to the path; earlier
    // points stay where they were.
    let mut rig = Rig::new(Mode::Polygon);
    rig.a.run(Command::ArmShape(operandi_core::ShapeKind::Tube));
    rig.frame(vec![]);
    let c = rig.a.viewport_rect.center();
    let points = |rig: &Rig| match &rig.a.editor.shape.as_ref().unwrap().requested {
        ShapeParameters::Tube(t) => t.points.clone(),
        _ => panic!("tube"),
    };
    for (i, at) in [c, c + vec2(100., 0.), c + vec2(100., -100.)]
        .into_iter()
        .enumerate()
    {
        rig.drag(at, vec2(0., 0.));
        assert_eq!(points(&rig).len(), i + 1, "{}", rig.a.status);
    }
    let before = points(&rig);
    // A press-drag in empty space adds one point and drags it.
    rig.drag(c + vec2(-150., 0.), vec2(-20., 0.));
    let after = points(&rig);
    assert_eq!(after.len(), 4);
    assert_eq!(&after[..3], &before[..]);
    assert!(after[3].x < before[0].x, "{after:?}");
    // A press on the second point moves it and adds nothing.
    rig.drag(c + vec2(100., 0.), vec2(0., 40.));
    let moved = points(&rig);
    assert_eq!(moved.len(), 4);
    assert_ne!(moved[1], before[1]);
    assert_eq!(moved[0], before[0]);
    assert_eq!(moved[2], before[2]);
}

/// The Bevel handle at `axis` (0 Shift, 1 Inset): where to press and which
/// way it points on screen. The tests use Inset: in the Front view the
/// Shift handle points at the camera.
fn bevel_handle(rig: &Rig, axis: usize) -> (Pos2, Vec2) {
    let (_, start, end, _) = rig
        .a
        .handles(rig.a.viewport_rect)
        .into_iter()
        .find(|h| h.0 == axis)
        .expect("Bevel handles show as soon as the tool is on (TA-01)");
    (start + (end - start) * 0.8, (end - start).normalized())
}

fn counts(rig: &Rig) -> (usize, usize) {
    let m = &rig.a.editor.scene.active().unwrap().mesh;
    (m.vertices.len(), m.faces.len())
}

impl Rig {
    fn drag_with(&mut self, from: Pos2, by: Vec2, modifiers: Modifiers) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![pointer(from, true, modifiers)]);
        for step in 1..=4 {
            self.frame(vec![Event::PointerMoved(from + by * (step as f32 / 4.))]);
        }
        self.frame(vec![pointer(from + by, false, modifiers)]);
    }
    /// Polygon 0's centre on screen: a face away from the handles.
    fn face_0(&self) -> Pos2 {
        self.a
            .camera
            .project(DVec3::new(0.5, 0.5, 0.), self.a.viewport_rect)
            .unwrap()
            .0
    }
    fn tool(&self) -> Option<Tool> {
        self.a.editor.live.as_ref().map(|l| l.tool)
    }
}

/// One bevel on the 3×3 grid's middle quad: 16 + 4 points, 9 + 4 polygons.
const ONE: (usize, usize) = (20, 13);
/// A second bevel stacked on its cap.
const TWO: (usize, usize) = (24, 17);

/// Jason, 2026-11-09: a second handle drag keeps adjusting the same bevel.
/// With `bevel_redrag_stacks` on (the old bevel-drag G2) it stacks instead.
#[test]
fn bevel_handle_drag_release_drag_again_is_one_bevel() {
    for stacks in [false, true] {
        let mut rig = Rig::new(Mode::Polygon);
        rig.a.bevel_redrag_stacks = stacks;
        rig.activate(Tool::Bevel);
        let (at, dir) = bevel_handle(&rig, 1);
        rig.drag(at, dir * -20.);
        assert_eq!(counts(&rig), ONE, "{}", rig.a.status);
        let first = rig.params().inset;
        assert_ne!(first, 0.);
        let (at, dir) = bevel_handle(&rig, 1);
        rig.drag(at, dir * -20.);
        if stacks {
            assert_eq!(counts(&rig), TWO, "G2: the re-drag stacks");
        } else {
            assert_eq!(counts(&rig), ONE, "same bevel");
            assert_ne!(rig.params().inset, first, "the second drag adds to it");
        }
    }
}

/// A panel edit after a handle drag adjusts that bevel (TA-02).
#[test]
fn bevel_drag_then_panel_edit_is_one_bevel() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.activate(Tool::Bevel);
    let (at, dir) = bevel_handle(&rig, 1);
    rig.drag(at, dir * -20.);
    assert_eq!(counts(&rig), ONE);
    let mut p = rig.params();
    p.shift = 0.1;
    rig.a.run(Command::ConfigureBevel(p));
    rig.frame(vec![]);
    assert_eq!(counts(&rig), ONE, "{}", rig.a.status);
    assert_eq!(rig.params().shift, 0.1);
}

/// TA-01 through the app: a panel edit right after switching Bevel on
/// changes the mesh with no viewport click.
#[test]
fn bevel_panel_edit_right_after_activation_is_live() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.activate(Tool::Bevel);
    assert_eq!(counts(&rig), (16, 9), "switching on changes nothing");
    let mut p = rig.params();
    p.inset = 0.1;
    rig.a.run(Command::ConfigureBevel(p));
    rig.frame(vec![]);
    assert_eq!(counts(&rig), ONE, "{}", rig.a.status);
}

/// Jason, 2026-11-09: Shift+click on a face keeps the bevel and starts a
/// fresh one (values reset); the next drag shapes it, stacked on the first.
/// Undo then steps back one edit at a time with the tool on.
#[test]
fn bevel_shift_click_face_then_drag_stacks_a_second_bevel() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.activate(Tool::Bevel);
    let (at, dir) = bevel_handle(&rig, 1);
    rig.drag(at, dir * -20.);
    assert_eq!(counts(&rig), ONE);
    let first = rig.params().inset;
    let face = rig.face_0();
    rig.drag_with(face, vec2(0., 0.), Modifiers::SHIFT);
    assert_eq!(counts(&rig), ONE, "the click alone adds nothing");
    assert_eq!((rig.params().inset, rig.params().shift), (0., 0.));
    assert_eq!(rig.a.editor.scene.selection.faces.len(), 1);
    let (at, dir) = bevel_handle(&rig, 1);
    rig.drag(at, dir * -10.);
    assert_eq!(counts(&rig), TWO, "{}", rig.a.status);

    rig.a.run(Command::Undo); // the second drag
    assert_eq!(counts(&rig), ONE, "{}", rig.a.status);
    assert_eq!(rig.params().inset, 0.);
    rig.a.run(Command::Undo); // the Shift+click: the first bevel's values again
    assert_eq!(counts(&rig), ONE, "{}", rig.a.status);
    assert_eq!(rig.params().inset, first);
    rig.a.run(Command::Undo); // the first drag
    assert_eq!(counts(&rig), (16, 9), "{}", rig.a.status);
    assert_eq!(rig.tool(), Some(Tool::Bevel));
}

/// Jason, 2026-11-09: drag, release, Shift+drag = two stacked bevels; a
/// Shift+drag on a handle does the same.
#[test]
fn bevel_shift_drag_stacks_a_second_bevel() {
    for on_handle in [false, true] {
        let mut rig = Rig::new(Mode::Polygon);
        rig.activate(Tool::Bevel);
        let (at, dir) = bevel_handle(&rig, 1);
        rig.drag(at, dir * -20.);
        assert_eq!(counts(&rig), ONE);
        if on_handle {
            let (at, dir) = bevel_handle(&rig, 1);
            rig.drag_with(at, dir * -10., Modifiers::SHIFT);
        } else {
            let corner = rig.corner();
            rig.drag_with(corner, vec2(-30., 0.), Modifiers::SHIFT);
        }
        assert_eq!(counts(&rig), TWO, "on handle {on_handle}: {}", rig.a.status);
        assert_ne!(rig.params().inset, 0.);
    }
}

/// The centre of face 4 (points 5, 6, 10, 9) as the scene shows it now.
fn face_4_center(rig: &Rig) -> DVec3 {
    let mesh = &rig.a.editor.scene.active().unwrap().mesh;
    [5, 6, 10, 9]
        .iter()
        .map(|&v| mesh.vertices[v])
        .sum::<DVec3>()
        / 4.
}

/// C-AUTO (AC-RULES v6, GitHub #39): with no Action Center every press
/// off the handle moves the Move widget to the press, beside the model, on
/// an unselected polygon and on the selected one alike, and the drag that
/// follows moves the selection from there. (It replaces VAC-Q02, where a
/// press on the model left the widget and one on the selection brought it
/// back.) The other Action Centers are in `ac_press_tests.rs`.
#[test]
fn every_press_moves_the_move_widget_to_the_press() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.activate(Tool::Move);
    let r = rig.a.viewport_rect;
    let at_selection = |rig: &Rig| (rig.a.transform_handle_center() - face_4_center(rig)).length();
    assert!(at_selection(&rig) < 1e-9);
    // Beside the model.
    rig.drag(rig.corner(), vec2(30., 0.));
    assert!(
        at_selection(&rig) > 0.5,
        "the widget stayed on the selection"
    );
    // On an unselected polygon (face 0), then on the selected one.
    for target in [DVec3::new(0.5, 0.5, 0.), face_4_center(&rig)] {
        let at = rig.a.camera.project(target, r).unwrap().0;
        assert_eq!(rig.a.handle_hit(at, r), None);
        rig.drag(at, vec2(0., 0.));
        let widget = rig.a.transform_handle_center();
        assert!((widget - target).length() < 1e-4, "{widget} at {target}");
    }
    // The drag in the same press still moves the selection.
    let moved = face_4_center(&rig);
    let on_face = rig
        .a
        .camera
        .project(moved + DVec3::new(0.3, 0.3, 0.), r)
        .unwrap()
        .0;
    assert_eq!(rig.a.handle_hit(on_face, r), None);
    rig.drag(on_face, vec2(0., -40.));
    assert!(face_4_center(&rig).y > moved.y + 0.01);
}
