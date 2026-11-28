//! Action Center rule sheet v6, step 2: what a press off the tool's handle
//! does under each Action Center (GitHub #32, #39). Element picks on every
//! press; Automatic, None and Screen place the centre on every press;
//! every other centre stays put; a handle always wins.
use super::tests::{draw, pointer};
use super::*;

const SIZE: Vec2 = vec2(1440., 900.);

fn quad(v: [u32; 4]) -> operandi_core::Face {
    operandi_core::Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// 3×3 quads in z = 0 facing +z, each 4 m wide; point `r * 4 + c` at
/// (4c, 4r, 0), polygon `r * 3 + c`.
fn grid() -> operandi_core::Mesh {
    let mut m = operandi_core::Mesh {
        vertices: (0..16)
            .map(|i| DVec3::new((i % 4) as f64 * 4., (i / 4) as f64 * 4., 0.))
            .collect(),
        ..Default::default()
    };
    for r in 0..3u32 {
        for c in 0..3u32 {
            let i = r * 4 + c;
            m.faces.push(quad([i, i + 1, i + 5, i + 4]));
        }
    }
    m
}

/// One more quad, 2 m in front of the grid's plane, to its right.
fn other() -> operandi_core::Mesh {
    operandi_core::Mesh {
        vertices: [(16., 0.), (20., 0.), (20., 4.), (16., 4.)]
            .map(|(x, y)| DVec3::new(x, y, 2.))
            .to_vec(),
        faces: vec![quad([0, 1, 2, 3])].into(),
        ..Default::default()
    }
}

struct Rig {
    a: Operandi,
    ctx: Context,
    /// The grid's item (the active mesh) and the other quad's.
    grid: u64,
    other: u64,
}
impl Rig {
    /// The Front view over both meshes, the grid active with its middle
    /// polygon (or that polygon's first edge or point) selected.
    fn new(mode: Mode) -> Self {
        let ctx = Context::default();
        theme::install(&ctx);
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        let other_id = a.editor.scene.active().unwrap().id;
        a.editor.scene.active_mut().unwrap().mesh = other().into();
        a.run(Command::NewMesh);
        let grid_id = a.editor.scene.active().unwrap().id;
        a.editor.scene.active_mut().unwrap().mesh = grid().into();
        assert_ne!(grid_id, other_id);
        a.camera.view = View::Front;
        a.camera.target = DVec3::new(10., 6., 0.);
        a.camera.distance = 40.;
        a.run(Command::Mode(mode));
        match mode {
            Mode::Polygon => a.editor.scene.selection.faces = vec![4],
            Mode::Edge => a.editor.scene.selection.edges = vec![(5, 6)],
            _ => a.editor.scene.selection.vertices = vec![5],
        }
        a.editor.history.undo.clear();
        let mut rig = Self {
            a,
            ctx,
            grid: grid_id,
            other: other_id,
        };
        rig.frame(vec![]);
        rig
    }
    fn frame(&mut self, events: Vec<Event>) {
        draw(&mut self.a, &self.ctx, SIZE, events);
    }
    fn context(&mut self, center: CenterMode, axis: AxisOrientation) {
        assert!(
            self.a
                .run_ok(Command::ActionContext(ActionContext { center, axis })),
            "{}",
            self.a.status
        );
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
    fn screen(&self, p: DVec3) -> Pos2 {
        self.a.camera.project(p, self.a.viewport_rect).unwrap().0
    }
    /// A press and release at world point `p`, dragging by `by` pixels in
    /// between. It must be off the tool's handles.
    fn press(&mut self, p: DVec3, by: Vec2) {
        let at = self.screen(p);
        assert_eq!(
            self.a.handle_hit(at, self.a.viewport_rect),
            None,
            "{p} is on a handle"
        );
        self.press_at(at, by);
    }
    /// A press and release at screen point `at`, off the handles.
    fn press_near(&mut self, at: Pos2) {
        assert_eq!(self.a.handle_hit(at, self.a.viewport_rect), None);
        self.press_at(at, Vec2::ZERO);
    }
    fn press_at(&mut self, at: Pos2, by: Vec2) {
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![pointer(at, true, Modifiers::NONE)]);
        for step in 1..=4 {
            self.frame(vec![Event::PointerMoved(at + by * (step as f32 / 4.))]);
        }
        self.frame(vec![pointer(at + by, false, Modifiers::NONE)]);
    }
    /// Where the live tool pivots, before what it has moved so far.
    fn center(&self) -> DVec3 {
        self.a
            .editor
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center
    }
    fn axes(&self) -> glam::DMat3 {
        self.a
            .editor
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .basis
    }
    fn delta(&self) -> DVec3 {
        self.a.editor.live.as_ref().unwrap().params.delta
    }
}

/// Equal within what a press at a screen position (32-bit) can say.
fn close(a: DVec3, b: DVec3, what: &str) {
    assert!(a.distance(b) < 1e-4, "{what}: {a} != {b}");
}

/// GitHub #32, C-ELEM / A-ELEM: with Element on, every press picks what is
/// under the pointer, a polygon, then an edge, then a point, as often as
/// you like, in every selection mode; the centre and the axes go to it
/// (Z along its normal) and the selection never changes. A press on empty
/// space changes nothing.
#[test]
fn element_picks_on_every_press() {
    for mode in [Mode::Polygon, Mode::Edge, Mode::Vertex] {
        let mut rig = Rig::new(mode);
        rig.a.choose_element_center();
        rig.activate(Tool::Rotate);
        let selection = rig.a.editor.scene.selection.clone();
        close(rig.center(), DVec3::ZERO, "the world origin before a pick");
        assert_eq!(rig.axes(), glam::DMat3::IDENTITY);

        // An unselected polygon: its middle, Z along its normal, X world X.
        rig.press(DVec3::new(10., 10., 0.), Vec2::ZERO);
        close(rig.center(), DVec3::new(10., 10., 0.), "polygon 8");
        close(rig.axes().z_axis, DVec3::Z, "polygon normal");
        close(rig.axes().x_axis, DVec3::X, "world X");

        // An edge (points 1 and 5): its middle, X along it.
        rig.press(DVec3::new(4., 2., 0.), Vec2::ZERO);
        close(rig.center(), DVec3::new(4., 2., 0.), "edge 1-5");
        close(rig.axes().x_axis, DVec3::Y, "along the edge");
        close(rig.axes().z_axis, DVec3::Z, "its polygons' normal");

        // A point (14).
        rig.press(DVec3::new(8., 12., 0.), Vec2::ZERO);
        close(rig.center(), DVec3::new(8., 12., 0.), "point 14");
        close(rig.axes().x_axis, DVec3::X, "world X");

        // Empty space: the mode and the last pick stay.
        let before = rig.a.editor.action_context;
        rig.press(DVec3::new(-6., 14., 0.), Vec2::ZERO);
        assert_eq!(rig.a.editor.action_context, before);
        close(rig.center(), DVec3::new(8., 12., 0.), "still point 14");

        assert_eq!(rig.a.editor.scene.selection, selection, "{mode:?}");
        assert!(matches!(
            rig.a.editor.action_context.center,
            CenterMode::Element(_)
        ));
    }
}

/// C-ELEM: any mesh the view shows can be picked, active or not, and the
/// active mesh and its selection stay as they are. The same press then
/// drags the tool about the new centre.
#[test]
fn element_picks_a_mesh_that_is_not_active_and_drags_in_the_same_press() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.a.choose_element_center();
    rig.activate(Tool::Rotate);
    let selection = rig.a.editor.scene.selection.clone();
    assert_eq!(selection.item, Some(rig.grid));
    assert!(!rig.a.editor.scene.active_meshes().contains(&rig.other));
    let before = rig.a.editor.scene.active().unwrap().mesh.vertices.clone();
    rig.press(DVec3::new(18., 2., 2.), vec2(60., 0.));
    close(
        rig.center(),
        DVec3::new(18., 2., 2.),
        "the other mesh's polygon",
    );
    assert_eq!(rig.a.editor.scene.selection, selection);
    // The drag turned the selected polygon about that centre.
    let after = rig.a.editor.scene.active().unwrap().mesh.vertices.clone();
    let pivot = DVec3::new(18., 2., 2.);
    for v in [5, 6, 9, 10] {
        assert!(after[v].distance(before[v]) > 0.1, "point {v} turned");
        assert!(
            (after[v].distance(pivot) - before[v].distance(pivot)).abs() < 1e-6,
            "point {v} keeps its distance from the pivot"
        );
    }
    // One more pick keeps that turn as its own step and starts afresh.
    // (Polygon 8 shares a corner with the turned polygon, so its middle
    // has moved a little.)
    let middle = rig.a.editor.scene.active().unwrap().mesh.center(8);
    rig.press(middle, Vec2::ZERO);
    close(rig.center(), middle, "polygon 8");
    let now = &rig.a.editor.scene.active().unwrap().mesh.vertices;
    for (v, (kept, was)) in now.iter().zip(&after).enumerate() {
        close(*kept, *was, &format!("point {v}: the first turn is kept"));
    }
    assert_eq!(rig.delta(), DVec3::ZERO);
}

/// A press on a handle always wins: with Element on, a press on the
/// tool's arrow drags the arrow and picks nothing, though a polygon is
/// under it.
#[test]
fn a_press_on_a_handle_wins_over_picking() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.a.choose_element_center();
    rig.activate(Tool::Move);
    rig.press(DVec3::new(10., 10., 0.), Vec2::ZERO);
    let picked = rig.a.editor.action_context;
    let r = rig.a.viewport_rect;
    let before = rig.a.editor.scene.active().unwrap().mesh.vertices.clone();
    let (_, origin, end, label) = rig
        .a
        .handles(r)
        .into_iter()
        .find(|h| h.0 == 0)
        .expect("the X arrow");
    let on_arrow = origin + (end - origin) * 0.7;
    assert_eq!(rig.a.handle_hit(on_arrow, r), Some(0), "{label}");
    rig.press_at(on_arrow, vec2(50., 0.));
    assert_eq!(rig.a.editor.action_context, picked, "nothing was picked");
    // The arrow moved the selected polygon along X only.
    let after = &rig.a.editor.scene.active().unwrap().mesh.vertices;
    for v in [5, 6, 9, 10] {
        let moved = after[v] - before[v];
        assert!(moved.x > 0.1 && moved.y == 0. && moved.z == 0., "{moved}");
    }
}

/// GitHub #39, C-AUTO: with None or Automatic every press off the handle
/// puts the centre where the press lands on the work plane (here the
/// Front view's XY plane), keeping the centre's own depth, again and
/// again, on the model or beside it.
#[test]
fn none_and_automatic_place_the_centre_on_every_press() {
    for center in [CenterMode::None, CenterMode::Automatic] {
        let mut rig = Rig::new(Mode::Polygon);
        rig.context(center, AxisOrientation::World);
        rig.activate(Tool::Rotate);
        close(
            rig.center(),
            DVec3::new(6., 6., 0.),
            "the selection's box middle",
        );
        // Beside the model.
        rig.press(DVec3::new(-5., 13., 7.), Vec2::ZERO);
        close(rig.center(), DVec3::new(-5., 13., 0.), "first press");
        // On the model (an unselected polygon), and on the selection.
        rig.press(DVec3::new(1.5, 2.5, 0.), Vec2::ZERO);
        close(rig.center(), DVec3::new(1.5, 2.5, 0.), "second press");
        rig.press(DVec3::new(6.5, 5.5, 0.), Vec2::ZERO);
        close(rig.center(), DVec3::new(6.5, 5.5, 0.), "third press");
        assert_eq!(rig.a.editor.action_context.center, center);
    }
}

/// GitHub #39: with Snapping on the placed centre snaps to what is under
/// the pointer, a corner of the selection included (nothing is moving
/// yet, so a pivot can sit on what it will turn), in all three
/// coordinates. With Snapping off it lands where the pointer is.
#[test]
fn a_placed_centre_snaps_when_snapping_is_on() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.activate(Tool::Rotate);
    let nudge = vec2(5., -4.);
    // Off: beside the point.
    let at = rig.screen(DVec3::new(12., 0., 0.)) + nudge;
    rig.press_near(at);
    let free = rig.center();
    assert!(free.distance(DVec3::new(12., 0., 0.)) > 1e-3, "{free}");
    assert!(free.distance(DVec3::new(12., 0., 0.)) < 0.5, "{free}");
    // On: a point of an unselected polygon.
    rig.a.snapping.enabled = true;
    let at = rig.screen(DVec3::new(0., 12., 0.)) + nudge;
    rig.press_near(at);
    assert_eq!(rig.center(), DVec3::new(0., 12., 0.), "snapped to point 12");
    // A corner of the selected polygon.
    let at = rig.screen(DVec3::new(8., 8., 0.)) + nudge;
    rig.press_near(at);
    assert_eq!(rig.center(), DVec3::new(8., 8., 0.), "snapped to point 10");
    // A point of the other mesh, 2 m nearer the eye: its depth too.
    let at = rig.screen(DVec3::new(20., 0., 2.)) + nudge;
    rig.press_near(at);
    assert_eq!(
        rig.center(),
        DVec3::new(20., 0., 2.),
        "snapped in depth too"
    );
}

/// C-AUTO in a Perspective view: the press lands on the ground work plane
/// and the centre keeps its own height.
#[test]
fn a_placed_centre_takes_the_ground_hit_and_keeps_its_height() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.a.camera.view = View::Perspective;
    rig.a.camera.yaw = 0.5;
    rig.a.camera.pitch = 1.1;
    rig.frame(vec![]);
    rig.activate(Tool::Rotate);
    let height = rig.center().y;
    assert_eq!(rig.a.actual_plane(), 0, "the ground plane");
    let r = rig.a.viewport_rect;
    let at = r.center() + vec2(-200., 150.);
    assert_eq!(rig.a.handle_hit(at, r), None);
    let (origin, direction) = rig.a.camera.ray(at, r);
    let ground = origin + direction * (-origin.y / direction.y);
    rig.press_at(at, Vec2::ZERO);
    close(
        rig.center(),
        DVec3::new(ground.x, height, ground.z),
        "ground X and Z, the centre's height",
    );
}

/// C-SCREEN: Screen is in force when chosen, at Automatic's centre with
/// the view's axes; every press places it again, never snapped, and takes
/// the view's axes as they are at that press.
#[test]
fn screen_places_unsnapped_with_the_views_axes() {
    let mut rig = Rig::new(Mode::Polygon);
    rig.a.snapping.enabled = true;
    rig.a.choose_screen_center();
    assert!(matches!(
        rig.a.editor.action_context.center,
        CenterMode::Screen(_)
    ));
    rig.activate(Tool::Rotate);
    close(rig.center(), DVec3::new(6., 6., 0.), "Automatic's centre");
    // Front view: right is X, up is Y, toward the viewer is Z.
    assert!(rig.axes().abs_diff_eq(glam::DMat3::IDENTITY, 1e-9));
    let at = rig.screen(DVec3::new(12., 0., 0.)) + vec2(5., -4.);
    rig.press_at(at, Vec2::ZERO);
    let placed = rig.center();
    assert!(placed.distance(DVec3::new(12., 0., 0.)) > 1e-3, "unsnapped");
    assert!(placed.distance(DVec3::new(12., 0., 0.)) < 0.5, "{placed}");
    // From another direction the next press brings that view's axes.
    rig.a.camera.view = View::Perspective;
    rig.a.camera.yaw = 0.5;
    rig.a.camera.pitch = 0.6;
    rig.frame(vec![]);
    let r = rig.a.viewport_rect;
    let at = r.center() + vec2(-200., 150.);
    assert_eq!(rig.a.handle_hit(at, r), None);
    rig.press_at(at, Vec2::ZERO);
    let (_, right, up, forward) = rig.a.camera.basis();
    close(rig.axes().x_axis, right, "screen right");
    close(rig.axes().y_axis, up, "screen up");
    close(rig.axes().z_axis, -forward, "toward the viewer");
    assert!(rig.center().distance(placed) > 0.1, "placed again");
}

/// The fixed and selection centres: a press off the handle never moves
/// them, beside the model, on it or on the selection. It only drags the
/// tool.
#[test]
fn other_centres_stay_put_under_a_press() {
    for (center, axis) in [
        (CenterMode::Origin, AxisOrientation::World),
        (CenterMode::Selection, AxisOrientation::Selection),
        (CenterMode::SelectionBorder, AxisOrientation::Selection),
        (CenterMode::SelectionAutoAxis, AxisOrientation::World),
        (CenterMode::Local, AxisOrientation::World),
        (CenterMode::Pivot, AxisOrientation::Item),
        (CenterMode::Parent, AxisOrientation::Parent),
    ] {
        let mut rig = Rig::new(Mode::Polygon);
        rig.context(center, axis);
        rig.activate(Tool::Rotate);
        let frame = rig.a.editor.live.as_ref().unwrap().action_frame;
        let before = rig.a.editor.scene.active().unwrap().mesh.vertices.clone();
        for (p, by) in [
            (DVec3::new(-5., 13., 0.), Vec2::ZERO),
            (DVec3::new(1.5, 2.5, 0.), Vec2::ZERO),
            (DVec3::new(6.5, 5.5, 0.), Vec2::ZERO),
            (DVec3::new(-5., 13., 0.), vec2(40., 0.)),
        ] {
            rig.press(p, by);
            assert_eq!(
                rig.a.editor.live.as_ref().unwrap().action_frame,
                frame,
                "{center:?} after a press at {p}"
            );
            assert_eq!(rig.a.editor.action_context, ActionContext { center, axis });
        }
        // The last press dragged the tool.
        assert_ne!(
            rig.a.editor.scene.active().unwrap().mesh.vertices,
            before,
            "{center:?}"
        );
    }
}

/// The stale pending click: Element, then Origin, then a press. The press
/// must not pick anything; Origin stays. The same for a Snaps and
/// Precision preset's waiting click once another Action Center is chosen
/// from the menu.
#[test]
fn choosing_another_action_center_ends_element_picking() {
    let origin = ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    };
    let mut rig = Rig::new(Mode::Polygon);
    rig.a.choose_element_center();
    rig.context(origin.center, origin.axis);
    rig.activate(Tool::Move);
    rig.press(DVec3::new(10., 10., 0.), Vec2::ZERO);
    assert_eq!(rig.a.editor.action_context, origin);
    close(rig.center(), DVec3::ZERO, "Origin");

    let mut rig = Rig::new(Mode::Polygon);
    rig.activate(Tool::Move);
    rig.a.preset_center_click = true;
    rig.a.choose_action_context(origin);
    assert!(!rig.a.preset_center_click);
    rig.press(DVec3::new(10., 10., 0.), Vec2::ZERO);
    assert_eq!(rig.a.editor.action_context, origin);
}
