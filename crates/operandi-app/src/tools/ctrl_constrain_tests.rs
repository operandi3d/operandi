//! Ctrl constrains tool drags (HM rules, Jason 2026-11-07): the press
//! decides the modifiers for the whole drag (HM-01); a Ctrl haul with Move
//! or Transform locks to one work-plane axis (HM-10/11/12, HM-40); Rotate
//! snaps to Rotation Snap Angle (HM-20/21); Ctrl on a Scale axis handle
//! scales uniformly (HM-30); Ctrl drags out a uniform primitive (HM-50);
//! Alt combinations navigate and never reach the tool (HM-08).
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
    held: Option<Parameters>,
}
impl Rig {
    /// Front view: the work plane follows the view, so it is XY.
    fn new() -> Self {
        let ctx = Context::default();
        theme::install(&ctx);
        let size = vec2(1440., 900.);
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid().into();
        a.camera.view = View::Front;
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![4];
        a.editor.history.undo.clear();
        let mut rig = Self {
            a,
            ctx,
            size,
            held: None,
        };
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
            self.a.editor.session.live.as_ref().map(|l| l.tool),
            Some(tool),
            "{}",
            self.a.status
        );
        self.frame(vec![]);
    }
    /// Press at `from`, move through each offset in turn, release at the
    /// last. `self.held` keeps the tool's values just before the release
    /// (transform tools start each drag fresh, so they reset after it).
    fn drag_path(&mut self, from: Pos2, path: &[Vec2], modifiers: Modifiers) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![pointer(from, true, modifiers)]);
        for &by in path {
            self.frame(vec![Event::PointerMoved(from + by)]);
        }
        self.held = self
            .a
            .editor
            .session
            .live
            .as_ref()
            .map(|l| l.params.clone());
        let end = from + *path.last().unwrap();
        self.frame(vec![pointer(end, false, modifiers)]);
    }
    fn drag(&mut self, from: Pos2, by: Vec2, modifiers: Modifiers) {
        let path: Vec<Vec2> = (1..=4).map(|s| by * (s as f32 / 4.)).collect();
        self.drag_path(from, &path, modifiers);
    }
    fn params(&self) -> Parameters {
        self.a.editor.session.live.as_ref().unwrap().params.clone()
    }
    /// The values the last drag reached before its release.
    fn held(&self) -> Parameters {
        self.held.clone().expect("a drag ran")
    }
    fn handle_tip(&self, id: usize) -> Pos2 {
        self.a
            .handles(self.a.viewport_rect)
            .into_iter()
            .find(|h| h.0 == id)
            .unwrap_or_else(|| panic!("no handle {id}"))
            .2
    }
}

#[test]
fn hm10_ctrl_haul_move_rightward_on_xy_changes_only_x() {
    let mut rig = Rig::new();
    rig.activate(Tool::Move);
    let corner = rig.corner();
    rig.drag(corner, vec2(80., 6.), Modifiers::CTRL);
    let d = rig.held().delta;
    assert!(d.x > 0., "moved right: {d} ({})", rig.a.status);
    assert_eq!((d.y, d.z), (0., 0.), "locked to X: {d}");
}

#[test]
fn hm11_hm12_the_axis_stays_locked_when_the_pointer_turns() {
    let mut rig = Rig::new();
    rig.activate(Tool::Move);
    let corner = rig.corner();
    // Picks Y on the first upward travel, then swings far to the right.
    rig.drag_path(
        corner,
        &[vec2(1., -8.), vec2(40., -12.), vec2(160., -14.)],
        Modifiers::CTRL,
    );
    let d = rig.held().delta;
    assert!(d.y > 0., "moved up: {d}");
    assert_eq!((d.x, d.z), (0., 0.), "still locked to Y: {d}");
}

#[test]
fn hm01_a_plain_drag_after_a_ctrl_drag_is_free() {
    let mut rig = Rig::new();
    rig.activate(Tool::Move);
    let corner = rig.corner();
    rig.drag(corner, vec2(60., 0.), Modifiers::CTRL);
    let locked = rig.held().delta;
    rig.drag(corner, vec2(40., -40.), Modifiers::NONE);
    let d = rig.held().delta;
    assert!(locked.y == 0. && d.x > 0. && d.y > 0., "free again: {d}");
}

#[test]
fn hm20_ctrl_haul_rotate_steps_by_the_snap_angle() {
    let mut rig = Rig::new();
    assert_eq!(rig.a.handle_preferences.rotation_snap, 15., "HM-21 default");
    rig.activate(Tool::Rotate);
    let corner = rig.corner();
    rig.drag(corner, vec2(47., 0.), Modifiers::CTRL);
    let d = rig.held().delta;
    assert!(d.length() > 0., "turned: {d} ({})", rig.a.status);
    for a in d.to_array() {
        assert!((a / 15. - (a / 15.).round()).abs() < 1e-9, "15° steps: {d}");
    }
    // Without Ctrl the same haul isn't snapped.
    let mut rig = Rig::new();
    rig.activate(Tool::Rotate);
    rig.drag(corner, vec2(47., 0.), Modifiers::NONE);
    let free = rig.held().delta;
    assert!(
        free.to_array()
            .iter()
            .any(|a| (a / 15.).fract().abs() > 1e-6)
    );
}

#[test]
fn hm40_transform_ctrl_haul_locks_to_one_axis() {
    let mut rig = Rig::new();
    rig.activate(Tool::Transform);
    let corner = rig.corner();
    rig.drag(corner, vec2(80., 6.), Modifiers::CTRL);
    let d = rig.held().delta;
    assert!(d.x > 0., "{d} ({})", rig.a.status);
    assert_eq!((d.y, d.z), (0., 0.));
}

#[test]
fn hm30_ctrl_on_a_scale_axis_handle_scales_all_three_equally() {
    let mut rig = Rig::new();
    rig.activate(Tool::Scale);
    let tip = rig.handle_tip(0);
    rig.drag(tip, vec2(50., 0.), Modifiers::CTRL);
    let d = rig.held().delta;
    assert!(d.x > 1., "grew: {d} ({})", rig.a.status);
    assert!(
        (d.x - d.y).abs() < 1e-12 && (d.x - d.z).abs() < 1e-12,
        "{d}"
    );
    // A plain axis drag still scales just that axis.
    let mut rig = Rig::new();
    rig.activate(Tool::Scale);
    let tip = rig.handle_tip(0);
    rig.drag(tip, vec2(50., 0.), Modifiers::NONE);
    let d = rig.held().delta;
    assert!(d.x > 1. && d.y == 1. && d.z == 1., "{d}");
}

#[test]
fn hm40_transform_scale_handle_follows_hm30() {
    let mut rig = Rig::new();
    rig.activate(Tool::Transform);
    // Transform's scale family is handles 6..8.
    let tip = rig.handle_tip(6);
    rig.drag(tip, vec2(50., 0.), Modifiers::CTRL);
    let s = rig.held().scale_delta;
    assert!(s.x > 1., "{s} ({})", rig.a.status);
    assert!(
        (s.x - s.y).abs() < 1e-12 && (s.x - s.z).abs() < 1e-12,
        "{s}"
    );
}

#[test]
fn hm50_ctrl_drag_out_cube_is_equal_sided() {
    let mut rig = Rig::new();
    rig.activate(Tool::Box);
    let corner = rig.corner();
    rig.drag(corner, vec2(90., -30.), Modifiers::CTRL);
    let s = rig.held().size;
    assert!(s.x > 0., "{s} ({})", rig.a.status);
    assert!(
        (s.x - s.y).abs() < 1e-12 && (s.x - s.z).abs() < 1e-12,
        "{s}"
    );
}

#[test]
fn hm08_alt_combinations_navigate_and_never_edit() {
    for modifiers in [
        Modifiers::ALT,
        Modifiers::ALT | Modifiers::SHIFT,
        Modifiers::ALT | Modifiers::CTRL,
    ] {
        let mut rig = Rig::new();
        rig.activate(Tool::Move);
        let base = rig.params();
        let scene = rig.a.editor.scene.clone();
        let corner = rig.corner();
        rig.drag(corner, vec2(60., -40.), modifiers);
        assert_eq!(rig.params(), base, "{modifiers:?}");
        assert_eq!(rig.a.editor.scene, scene, "{modifiers:?}");
        assert!(rig.a.editor.history.undo.is_empty(), "{modifiers:?}");
    }
}

#[test]
fn hm32_ctrl_haul_scale_is_uniform() {
    let mut rig = Rig::new();
    rig.activate(Tool::Scale);
    let corner = rig.corner();
    rig.drag(corner, vec2(60., -10.), Modifiers::CTRL);
    let d = rig.held().delta;
    assert!(d.x > 1., "{d} ({})", rig.a.status);
    assert!(
        (d.x - d.y).abs() < 1e-12 && (d.x - d.z).abs() < 1e-12,
        "{d}"
    );
}

/// `draw` with modifier keys held in the frame's input (as when Ctrl goes
/// down partway through a drag).
fn frame_with(rig: &mut Rig, events: Vec<Event>, modifiers: Modifiers) {
    let mut raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, rig.size)),
        events,
        modifiers,
        focused: true,
        ..Default::default()
    };
    raw.viewports
        .entry(ViewportId::ROOT)
        .or_default()
        .native_pixels_per_point = Some(1.);
    let a = &mut rig.a;
    a.hold_bound_tab(&rig.ctx, &mut raw);
    let _ = rig.ctx.run(raw, |ctx| {
        a.begin_input(ctx);
        a.compose(ctx);
        a.keyboard(ctx);
    });
}

#[test]
fn hm12a_ctrl_pressed_mid_drag_does_nothing() {
    let mut rig = Rig::new();
    rig.activate(Tool::Move);
    let corner = rig.corner();
    rig.frame(vec![Event::PointerMoved(corner)]);
    rig.frame(vec![pointer(corner, true, Modifiers::NONE)]);
    for step in 1..=4 {
        let at = corner + vec2(15., -15.) * step as f32;
        frame_with(&mut rig, vec![Event::PointerMoved(at)], Modifiers::CTRL);
    }
    let d = rig.params().delta;
    assert!(d.x > 0. && d.y > 0., "still a free drag: {d}");
    let end = corner + vec2(60., -60.);
    frame_with(
        &mut rig,
        vec![pointer(end, false, Modifiers::CTRL)],
        Modifiers::CTRL,
    );
}

#[test]
fn hm21_rotate_steps_follow_the_preference_live() {
    let mut rig = Rig::new();
    rig.a.handle_preferences.rotation_snap = 30.;
    rig.activate(Tool::Rotate);
    let corner = rig.corner();
    rig.drag(corner, vec2(50., 0.), Modifiers::CTRL);
    let d = rig.held().delta;
    assert!(d.length() > 0., "{d}");
    for a in d.to_array() {
        assert!((a / 30. - (a / 30.).round()).abs() < 1e-9, "30° steps: {d}");
    }
}

#[test]
fn hm71_ctrl_tweak_keeps_to_the_first_axis() {
    let mut rig = Rig::new();
    rig.activate(Tool::Tweak);
    let r = rig.a.viewport_rect;
    let item = rig.a.editor.scene.active().unwrap().id;
    let center = rig
        .a
        .editor
        .scene
        .world_point(item, DVec3::new(1.5, 1.5, 0.));
    let press = rig.a.camera.project(center, r).unwrap().0;
    // Heads right first, then goes diagonal.
    rig.drag_path(
        press,
        &[vec2(8., -1.), vec2(30., -25.), vec2(50., -45.)],
        Modifiers::CTRL,
    );
    let d = rig.held().delta;
    assert!(d.x > 0., "{d} ({})", rig.a.status);
    assert_eq!((d.y, d.z), (0., 0.), "not diagonal: {d}");
}

#[test]
fn hm52_ctrl_drag_away_from_a_drawn_cube_moves_it_whole() {
    let mut rig = Rig::new();
    rig.activate(Tool::Box);
    let corner = rig.corner();
    rig.drag(corner, vec2(90., -60.), Modifiers::NONE);
    let drawn = rig.params();
    rig.drag(corner + vec2(300., 200.), vec2(40., 0.), Modifiers::CTRL);
    let moved = rig.params();
    assert!(
        moved.size.abs_diff_eq(drawn.size, 1e-12),
        "not resized: {} vs {} ({})",
        moved.size,
        drawn.size,
        rig.a.status
    );
    assert_ne!(moved.position, drawn.position, "moved ({})", rig.a.status);
}

// ---- ctrl-extras: Shift continues (HM-02), middle repeats (HM-04),
// ---- Ctrl+Shift copies selected polygons (HM-06).

fn middle(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Middle,
    }
}

/// Point 5 is a corner of the selected polygon 4.
fn point5(rig: &Rig) -> DVec3 {
    rig.a.editor.scene.active().unwrap().mesh.vertices[5]
}

#[test]
fn hm02_shift_haul_continues_the_last_drag_and_plain_starts_fresh() {
    let mut rig = Rig::new();
    rig.activate(Tool::Move);
    let corner = rig.corner();
    rig.drag(corner, vec2(60., 0.), Modifiers::NONE);
    let first = rig.held().delta.x;
    assert!(first > 0.);
    rig.drag(corner, vec2(60., 0.), Modifiers::SHIFT);
    let continued = rig.held().delta.x;
    assert!(
        (continued - 2. * first).abs() < 1e-9,
        "values keep growing: {continued} vs {first} ({})",
        rig.a.status
    );
    assert!((point5(&rig).x - (1. + 2. * first)).abs() < 1e-9);
    rig.drag(corner, vec2(60., 0.), Modifiers::NONE);
    assert!((rig.held().delta.x - first).abs() < 1e-9, "fresh again");
    assert!((point5(&rig).x - (1. + 3. * first)).abs() < 1e-9);
}

#[test]
fn hm02_shift_on_a_handle_continues_too() {
    let mut rig = Rig::new();
    rig.activate(Tool::Move);
    let tip = rig.handle_tip(0);
    rig.drag(tip, vec2(50., 0.), Modifiers::NONE);
    let first = rig.held().delta.x;
    let tip = rig.handle_tip(0);
    rig.drag(tip, vec2(50., 0.), Modifiers::SHIFT);
    assert!(
        (rig.held().delta.x - 2. * first).abs() < 1e-9,
        "{}",
        rig.a.status
    );
}

#[test]
fn hm02_shift_with_nothing_to_continue_starts_fresh() {
    let mut rig = Rig::new();
    rig.activate(Tool::Rotate);
    let corner = rig.corner();
    rig.drag(corner, vec2(40., 0.), Modifiers::SHIFT);
    assert!(rig.held().delta.length() > 0., "{}", rig.a.status);
}

#[test]
fn hm04_middle_click_repeats_the_previous_drag_once() {
    let mut rig = Rig::new();
    rig.activate(Tool::Move);
    let corner = rig.corner();
    rig.drag(corner, vec2(60., 0.), Modifiers::NONE);
    let step = point5(&rig).x - 1.;
    assert!(step > 0.);
    rig.frame(vec![Event::PointerMoved(corner)]);
    rig.frame(vec![middle(corner, true)]);
    rig.frame(vec![middle(corner, false)]);
    assert!(
        (point5(&rig).x - (1. + 2. * step)).abs() < 1e-9,
        "{} ({})",
        point5(&rig),
        rig.a.status
    );
    // The repeat is its own step: one Undo takes back just it.
    rig.a.run(Command::Undo);
    assert!(
        (point5(&rig).x - (1. + step)).abs() < 1e-9,
        "{} step {step} ({}) tweaks {}",
        point5(&rig),
        rig.a.status,
        rig.a.editor.session.tweaks.undo_steps()
    );
}

#[test]
fn hm04_middle_drag_starts_from_the_previous_values() {
    let mut rig = Rig::new();
    rig.activate(Tool::Move);
    let corner = rig.corner();
    rig.drag(corner, vec2(60., 0.), Modifiers::NONE);
    let first = rig.held().delta.x;
    rig.frame(vec![Event::PointerMoved(corner)]);
    rig.frame(vec![middle(corner, true)]);
    rig.frame(vec![Event::PointerMoved(corner + vec2(30., 0.))]);
    let d = rig.params().delta;
    assert!(d.x > first * 1.4, "repeat plus the drag: {d} vs {first}");
    rig.frame(vec![middle(corner + vec2(30., 0.), false)]);
}

#[test]
fn hm04_middle_with_nothing_to_repeat_changes_nothing() {
    let mut rig = Rig::new();
    rig.activate(Tool::Scale);
    let scene = rig.a.editor.scene.clone();
    let corner = rig.corner();
    rig.frame(vec![middle(corner, true)]);
    rig.frame(vec![middle(corner, false)]);
    assert_eq!(rig.a.editor.scene, scene);
    assert!(
        rig.a.status.contains("Nothing to repeat"),
        "{}",
        rig.a.status
    );
}

// ---- SPEC-REPEAT-TRANSFORM: middle-click re-applies the last amount.

impl Rig {
    fn middle_click(&mut self) {
        let corner = self.corner();
        self.frame(vec![Event::PointerMoved(corner)]);
        self.frame(vec![middle(corner, true)]);
        self.frame(vec![middle(corner, false)]);
    }
    /// A typed tool value: the live tool previews it, nothing recorded yet.
    fn type_amount(&mut self, delta: DVec3) {
        let mut p = self.params();
        p.delta = delta;
        assert!(self.a.run_ok(Command::Preview(p)), "{}", self.a.status);
        self.frame(vec![]);
    }
    fn points(&self) -> Vec<DVec3> {
        self.a.editor.scene.active().unwrap().mesh.vertices.clone()
    }
    /// Centre of the selected polygon 4 (points 5, 6, 9, 10).
    fn face4_centre(&self) -> DVec3 {
        let v = self.points();
        (v[5] + v[6] + v[9] + v[10]) / 4.
    }
}

fn same_points(a: &[DVec3], b: &[DVec3], eps: f64) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| p.abs_diff_eq(*q, eps))
}

#[test]
fn rpt_t01_typed_angle_then_middle_click_doubles_it_in_two_steps() {
    let mut rig = Rig::new();
    let start = rig.points();
    rig.activate(Tool::Rotate);
    rig.type_amount(DVec3::new(0., 0., 15.));
    let once = rig.points();
    assert!(!same_points(&once, &start, 1e-9), "typed 15° turned it");
    rig.middle_click();
    let twice = rig.points();
    // Compare with a single typed 30° turn from the start.
    let mut reference = Rig::new();
    reference.activate(Tool::Rotate);
    reference.type_amount(DVec3::new(0., 0., 30.));
    assert!(
        same_points(&twice, &reference.points(), 1e-9),
        "15° + repeat = 30° ({})",
        rig.a.status
    );
    // Two undo steps: the repeat, then the typed apply.
    rig.a.run(Command::Undo);
    assert!(same_points(&rig.points(), &once, 1e-9), "{}", rig.a.status);
    rig.a.run(Command::Undo);
    assert!(same_points(&rig.points(), &start, 1e-9), "{}", rig.a.status);
    assert!(rig.a.editor.session.live.is_some(), "the tool stays on");
}

#[test]
fn rpt_t02_twenty_four_repeats_of_15_degrees_come_full_circle() {
    let mut rig = Rig::new();
    // The whole grid turns, so no neighbouring polygon folds over.
    rig.a.editor.scene.selection.faces = (0..9).collect();
    rig.frame(vec![]);
    let start = rig.points();
    rig.activate(Tool::Rotate);
    rig.type_amount(DVec3::new(0., 0., 15.));
    for _ in 0..23 {
        rig.middle_click();
    }
    assert!(
        same_points(&rig.points(), &start, 1e-5),
        "360° in all ({})",
        rig.a.status
    );
}

#[test]
fn rpt_t04_dropping_the_tool_forgets_the_amount() {
    let mut rig = Rig::new();
    rig.activate(Tool::Rotate);
    rig.type_amount(DVec3::new(0., 0., 15.));
    rig.a.run(Command::Drop);
    rig.frame(vec![]);
    rig.activate(Tool::Rotate);
    let scene = rig.a.editor.scene.clone();
    let steps = rig.a.editor.history.undo.len();
    rig.middle_click();
    assert_eq!(rig.a.editor.scene, scene, "{}", rig.a.status);
    assert_eq!(rig.a.editor.history.undo.len(), steps, "no undo step");
}

#[test]
fn rpt_t05_move_repeats_add_up() {
    let mut rig = Rig::new();
    let start = rig.points();
    rig.activate(Tool::Move);
    rig.type_amount(DVec3::new(0.1, 0., 0.));
    for _ in 0..3 {
        rig.middle_click();
    }
    assert!(
        (rig.points()[5].x - (start[5].x + 0.4)).abs() < 1e-9,
        "{} ({})",
        rig.points()[5],
        rig.a.status
    );
}

#[test]
fn rpt_t06_scale_200_percent_repeated_is_400() {
    let mut rig = Rig::new();
    let start = rig.points();
    rig.activate(Tool::Scale);
    rig.type_amount(DVec3::splat(2.));
    rig.middle_click();
    let v = rig.points();
    let was = start[6].x - start[5].x;
    assert!(
        ((v[6].x - v[5].x) - 4. * was).abs() < 1e-9,
        "{} ({})",
        v[6].x - v[5].x,
        rig.a.status
    );
}

#[test]
fn rpt_t07_selection_centre_stays_put_while_spinning() {
    let mut rig = Rig::new();
    let centre = rig.face4_centre();
    rig.activate(Tool::Rotate);
    rig.type_amount(DVec3::new(0., 0., 15.));
    rig.middle_click();
    rig.middle_click();
    assert!(
        rig.face4_centre().abs_diff_eq(centre, 1e-9),
        "{} vs {centre}",
        rig.face4_centre()
    );
}

#[test]
fn hm06_ctrl_shift_in_polygon_mode_drags_a_copy() {
    let mut rig = Rig::new();
    rig.activate(Tool::Move);
    let before = rig.a.editor.scene.active().unwrap().mesh.clone();
    let tip = rig.handle_tip(0);
    rig.drag(tip, vec2(50., 0.), Modifiers::CTRL | Modifiers::SHIFT);
    let mesh = rig.a.editor.scene.active().unwrap().mesh.clone();
    assert_eq!(mesh.faces.len(), before.faces.len() + 1, "{}", rig.a.status);
    assert_eq!(mesh.vertices[..16], before.vertices[..16], "original stays");
    let copy = &mesh.faces[9].vertices;
    let moved = mesh.vertices[copy[0] as usize];
    assert!(moved.x > 1., "the copy moved: {moved}");
    assert_eq!(rig.a.editor.scene.selection.faces, vec![9]);
}

#[test]
fn hm06_ctrl_shift_with_no_whole_polygon_is_refused() {
    let mut rig = Rig::new();
    rig.a.run(Command::Mode(Mode::Vertex));
    rig.a.editor.scene.selection.vertices = vec![5];
    rig.a.editor.history.undo.clear();
    rig.frame(vec![]);
    rig.activate(Tool::Move);
    let faces = rig.a.editor.scene.active().unwrap().mesh.faces.len();
    let tip = rig.handle_tip(0);
    rig.drag(tip, vec2(50., 0.), Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(rig.a.editor.scene.active().unwrap().mesh.faces.len(), faces);
    assert!(rig.a.status.contains("Nothing whole"), "{}", rig.a.status);
    assert_eq!(point5(&rig), DVec3::new(1., 1., 0.), "nothing moved");
}

#[test]
fn hm06_ctrl_shift_in_vertex_mode_copies_fully_selected_polygons() {
    let mut rig = Rig::new();
    rig.a.run(Command::Mode(Mode::Vertex));
    // Points of polygon 4 (5, 6, 10, 9) and one stray point.
    rig.a.editor.scene.selection.vertices = vec![0, 5, 6, 9, 10];
    rig.frame(vec![]);
    rig.activate(Tool::Move);
    let tip = rig.handle_tip(0);
    rig.drag(tip, vec2(50., 0.), Modifiers::CTRL | Modifiers::SHIFT);
    let mesh = &rig.a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 10, "{}", rig.a.status);
    assert_eq!(mesh.vertices.len(), 20);
    assert_eq!(rig.a.editor.scene.selection.vertices, vec![16, 17, 18, 19]);
}

#[test]
fn hm02_one_undo_takes_back_the_shift_drag() {
    let mut rig = Rig::new();
    rig.activate(Tool::Move);
    let corner = rig.corner();
    rig.drag(corner, vec2(60., 0.), Modifiers::NONE);
    let after_first = point5(&rig);
    rig.drag(corner, vec2(60., 0.), Modifiers::SHIFT);
    rig.a.run(Command::Undo);
    assert!(
        point5(&rig).abs_diff_eq(after_first, 1e-9),
        "{}",
        point5(&rig)
    );
    rig.a.run(Command::Undo);
    assert_eq!(point5(&rig), DVec3::new(1., 1., 0.));
}

#[test]
fn hm04_a_preset_that_orbits_with_middle_wins() {
    let mut rig = Rig::new();
    rig.a.input_preferences.navigation = crate::preferences::NavigationPreset::Blender;
    rig.activate(Tool::Move);
    let corner = rig.corner();
    rig.drag(corner, vec2(60., 0.), Modifiers::NONE);
    let moved = point5(&rig);
    rig.frame(vec![middle(corner, true)]);
    rig.frame(vec![Event::PointerMoved(corner + vec2(30., 10.))]);
    rig.frame(vec![middle(corner + vec2(30., 10.), false)]);
    assert_eq!(point5(&rig), moved, "orbit, no repeat");
}
