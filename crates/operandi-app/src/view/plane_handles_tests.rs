//! Plane handles (plane-handle spec v1, PH-xx): circles lying flat in the
//! plane of two arrows, at their outer corner, on Move, Scale and Transform's
//! move part, plus Scale's center circle. Hover tooltip (UIC-05) and
//! hauling (UIC-76) follow the arrows' rules.
use super::tests::{draw, pointer};
use super::viewport::plane_handles::{PLANE_HANDLE_BASE, PLANE_HANDLE_DIAMETER};
use super::*;
use crate::theme::viewport as palette;

const YZ: usize = PLANE_HANDLE_BASE;
const XZ: usize = PLANE_HANDLE_BASE + 1;
const XY: usize = PLANE_HANDLE_BASE + 2;

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
    /// Front view looks along Z: the XY circle faces the view and the YZ
    /// and XZ planes are edge-on. `oblique` turns the view so all three show.
    fn new(oblique: bool) -> Self {
        let ctx = Context::default();
        theme::install(&ctx);
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid().into();
        a.camera.view = View::Front;
        if oblique {
            a.camera.view = View::Perspective;
            a.camera.yaw = 0.6;
            a.camera.pitch = 0.5;
        }
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![4];
        a.editor.history.undo.clear();
        let mut rig = Self {
            a,
            ctx,
            size: vec2(1440., 900.),
            held: None,
        };
        rig.frame(vec![]);
        rig
    }
    fn frame(&mut self, events: Vec<Event>) {
        draw(&mut self.a, &self.ctx, self.size, events);
    }
    fn activate(&mut self, tool: Tool) {
        self.a.activate(tool);
        assert_eq!(
            self.a.editor.session.live.as_ref().map(|l| l.tool),
            Some(tool)
        );
        self.frame(vec![]);
    }
    fn circles(&self) -> Vec<(usize, Pos2)> {
        let tool = self.a.editor.session.live.as_ref().unwrap().tool;
        self.a
            .plane_handles(tool, self.a.viewport_rect)
            .into_iter()
            .map(|h| (h.0, h.2))
            .collect()
    }
    fn circle(&self, id: usize) -> Pos2 {
        self.circles()
            .into_iter()
            .find(|c| c.0 == id)
            .unwrap_or_else(|| panic!("no circle {id}: {:?}", self.circles()))
            .1
    }
    fn drag(&mut self, from: Pos2, by: Vec2, modifiers: Modifiers) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![pointer(from, true, modifiers)]);
        for step in 1..=4 {
            self.frame(vec![Event::PointerMoved(from + by * (step as f32 / 4.))]);
        }
        self.held = self
            .a
            .editor
            .session
            .live
            .as_ref()
            .map(|l| l.params.clone());
        self.frame(vec![pointer(from + by, false, modifiers)]);
    }
    fn held(&self) -> Parameters {
        self.held.clone().unwrap()
    }
}

#[test]
fn ph11_edge_on_planes_are_hidden_and_unpickable() {
    let mut rig = Rig::new(false);
    rig.activate(Tool::Move);
    let ids: Vec<usize> = rig.circles().iter().map(|c| c.0).collect();
    assert_eq!(ids, vec![XY], "only the plane facing the view");
    let mut oblique = Rig::new(true);
    oblique.activate(Tool::Move);
    let ids: Vec<usize> = oblique.circles().iter().map(|c| c.0).collect();
    assert_eq!(ids, vec![YZ, XZ, XY]);
}

#[test]
fn ph20_move_circle_drag_changes_only_its_two_axes() {
    let mut rig = Rig::new(true);
    rig.activate(Tool::Move);
    let at = rig.circle(YZ);
    assert_eq!(rig.a.handle_hit(at, rig.a.viewport_rect), Some(YZ));
    rig.drag(at, vec2(30., -40.), Modifiers::NONE);
    let d = rig.held().delta;
    assert_eq!(d.x, 0., "YZ circle never moves X: {d}");
    assert!(d.y != 0. || d.z != 0., "moved: {d} ({})", rig.a.status);
    // Front view: the XY circle moves in X and Y.
    let mut rig = Rig::new(false);
    rig.activate(Tool::Move);
    let at = rig.circle(XY);
    rig.drag(at, vec2(40., -30.), Modifiers::NONE);
    let d = rig.held().delta;
    assert!(d.x > 0. && d.y > 0. && d.z == 0., "{d}");
}

#[test]
fn ph22_ctrl_changes_nothing_on_a_circle() {
    let plain = {
        let mut rig = Rig::new(false);
        rig.activate(Tool::Move);
        let at = rig.circle(XY);
        rig.drag(at, vec2(50., -8.), Modifiers::NONE);
        rig.held().delta
    };
    let mut rig = Rig::new(false);
    rig.activate(Tool::Move);
    let at = rig.circle(XY);
    rig.drag(at, vec2(50., -8.), Modifiers::CTRL);
    let d = rig.held().delta;
    assert!(d.x > 0. && d.y > 0., "both axes: {d}");
    assert!(
        d.abs_diff_eq(plain, 1e-12),
        "same as without Ctrl: {d} vs {plain}"
    );
}

#[test]
fn ph31_scale_circle_scales_its_two_axes_equally() {
    let mut rig = Rig::new(false);
    rig.activate(Tool::Scale);
    let at = rig.circle(XY);
    assert_eq!(rig.a.handle_hit(at, rig.a.viewport_rect), Some(XY));
    rig.drag(at, vec2(40., 0.), Modifiers::NONE);
    let d = rig.held().delta;
    assert!(d.x > 1. && (d.x - d.y).abs() < 1e-12 && d.z == 1., "{d}");
}

#[test]
fn ph32_transform_shows_its_move_circles() {
    let mut rig = Rig::new(true);
    rig.activate(Tool::Transform);
    assert_eq!(rig.circles().len(), 3);
    let at = rig.circle(XY);
    rig.drag(at, vec2(40., 0.), Modifiers::NONE);
    let p = rig.held();
    assert_eq!(p.delta.z, 0.);
    assert!(p.delta.length() > 0.);
    assert_eq!(p.scale_delta, DVec3::ONE, "moves, doesn't scale");
}

#[test]
fn ph20_one_undo_per_circle_drag() {
    let mut rig = Rig::new(false);
    rig.activate(Tool::Move);
    let at = rig.circle(XY);
    rig.drag(at, vec2(40., -30.), Modifiers::NONE);
    let moved = rig.a.editor.scene.active().unwrap().mesh.vertices[5];
    assert_ne!(moved, DVec3::new(1., 1., 0.));
    rig.a.run(Command::Undo);
    assert_eq!(
        rig.a.editor.scene.active().unwrap().mesh.vertices[5],
        DVec3::new(1., 1., 0.)
    );
}

#[test]
fn ph11_center_and_arrows_win_over_circles() {
    let mut rig = Rig::new(false);
    rig.activate(Tool::Move);
    let r = rig.a.viewport_rect;
    let handles = rig.a.handles(r);
    let center = handles
        .iter()
        .find(|h| h.0 == viewport::FREE_HANDLE)
        .unwrap()
        .2;
    assert_eq!(rig.a.handle_hit(center, r), Some(viewport::FREE_HANDLE));
    let x_tip = handles.iter().find(|h| h.0 == 0).unwrap().2;
    assert_eq!(rig.a.handle_hit(x_tip, r), Some(0));
}

#[test]
fn ph08_ph40_size_follows_handle_prefs_and_invisible_hides() {
    let mut rig = Rig::new(false);
    rig.activate(Tool::Move);
    let size = rig.a.plane_handle_size();
    assert!(
        (size - 72. * PLANE_HANDLE_DIAMETER).abs() < 1e-4,
        "1/6 of the arrow"
    );
    rig.a.handle_preferences.scale = 2.;
    assert!((rig.a.plane_handle_size() - size * 2.).abs() < 1e-4);
    rig.a.handle_preferences.selected = crate::handle_preferences::HandleDetail::Advanced;
    assert!(rig.a.handles(rig.a.viewport_rect).iter().any(|h| h.0 == XY));
    rig.a.handle_preferences.selected = crate::handle_preferences::HandleDetail::Invisible;
    assert!(!rig.a.handles(rig.a.viewport_rect).iter().any(|h| h.0 == XY));
}

#[test]
fn ph07_circle_sits_at_the_outer_corner_of_its_arrows() {
    let mut rig = Rig::new(true);
    rig.activate(Tool::Move);
    let r = rig.a.viewport_rect;
    let handles = rig.a.handles(r);
    let arrow = |id: usize| handles.iter().find(|h| h.0 == id).unwrap();
    let origin = arrow(0).1;
    let corner = origin + (arrow(0).2 - origin) + (arrow(1).2 - origin);
    assert!(
        rig.circle(XY).distance(corner) < 0.5,
        "{:?} vs {corner:?}",
        rig.circle(XY)
    );
}

/// The grid tilted and the action axis set to the selection, so the
/// arrows turn away from the world axes (the forum report's gizmo).
fn turned(tool: Tool) -> Rig {
    let mut rig = Rig::new(true);
    {
        let mesh = &mut rig.a.editor.scene.active_mut().unwrap().mesh;
        let turn = glam::DQuat::from_rotation_x(0.7) * glam::DQuat::from_rotation_y(0.4);
        let mut m: operandi_core::Mesh = (**mesh).clone();
        for v in &mut m.vertices {
            *v = turn * *v;
        }
        *mesh = m.into();
    }
    rig.a.editor.action_context.axis = operandi_core::AxisOrientation::Selection;
    rig.frame(vec![]);
    rig.activate(tool);
    assert!(
        rig.a.handle_axis(0).distance(DVec3::X) > 0.1,
        "the arrows really are turned"
    );
    rig
}

/// Whether ellipse half-axis `u` runs along `axis` from the gizmo on screen.
fn along_on_screen(rig: &Rig, u: Vec2, axis: DVec3) -> bool {
    let r = rig.a.viewport_rect;
    let center = rig.a.transform_handle_center();
    let (o, _) = rig.a.camera.project(center, r).unwrap();
    let (x, _) = rig.a.camera.project(center + axis, r).unwrap();
    let cross = u.x * (x - o).y - u.y * (x - o).x;
    cross.abs() / (u.length() * (x - o).length()) < 1e-3
}

/// PH-21 (FG-Q01, Jason 2026-11-22; forum report "funky gizmo"): under a
/// turned action axis the circles lie in their two arrows' plane, not in
/// a world plane, on Move and Scale alike; a Move circle drag moves along
/// those two arrows only.
#[test]
fn ph07a_ph21_circles_turn_with_their_arrows() {
    for tool in [Tool::Move, Tool::Scale] {
        let rig = turned(tool);
        let r = rig.a.viewport_rect;
        let e = rig.a.plane_ellipse(XY, r).unwrap();
        assert!(
            along_on_screen(&rig, e.u, rig.a.handle_axis(0)),
            "{tool:?} u"
        );
        assert!(
            along_on_screen(&rig, e.v, rig.a.handle_axis(1)),
            "{tool:?} v"
        );
        assert!(
            !along_on_screen(&rig, e.u, DVec3::X),
            "{tool:?} not world X"
        );
    }
    let mut rig = turned(Tool::Move);
    let normal = rig.a.handle_axis(2);
    let height = |rig: &Rig| -> Vec<f64> {
        let m = &rig.a.editor.scene.active().unwrap().mesh;
        [5, 6, 9, 10]
            .iter()
            .map(|&i| m.vertices[i].dot(normal))
            .collect()
    };
    let before = height(&rig);
    let at = rig.circle(XY);
    rig.drag(at, vec2(40., -25.), Modifiers::NONE);
    let after = height(&rig);
    for (b, a) in before.iter().zip(&after) {
        assert!((a - b).abs() < 1e-9, "kept along the turned Z: {b} -> {a}");
    }
    let d = rig.held().delta;
    assert!(d.z == 0. && (d.x != 0. || d.y != 0.), "{d}");
}

#[test]
fn ph10_hover_lights_only_the_rim_and_tooltip_says_what_it_does() {
    assert_eq!(
        Operandi::plane_handle_rim(XY, true),
        palette::handle_active()
    );
    assert_eq!(Operandi::plane_handle_rim(XY, false), palette::handles()[2]);
    assert_eq!(Operandi::plane_handle_rim(YZ, false), palette::handles()[0]);
    assert_eq!(
        Operandi::plane_handle_tooltip(Tool::Move, XY),
        "Plane handle: drag to move in the XY plane"
    );
    assert_eq!(
        Operandi::plane_handle_tooltip(Tool::Scale, XZ),
        "Plane handle: drag to scale X and Z equally"
    );
}

#[test]
fn ph31_scale_center_circle_scales_uniformly() {
    let mut rig = Rig::new(false);
    rig.activate(Tool::Scale);
    let r = rig.a.viewport_rect;
    let center = rig
        .a
        .handles(r)
        .into_iter()
        .find(|h| h.0 == 3)
        .expect("Scale's center handle")
        .2;
    // A press just inside the larger circle's edge picks it.
    let inside = center + vec2(rig.a.scale_center_size() / 2. - 1., 0.);
    assert_eq!(rig.a.handle_hit(inside, r), Some(3));
    rig.drag(center, vec2(40., 0.), Modifiers::NONE);
    let d = rig.held().delta;
    assert!(
        d.x > 1. && (d.x - d.y).abs() < 1e-12 && (d.x - d.z).abs() < 1e-12,
        "{d}"
    );
}

#[test]
fn ph32a_rotate_has_no_circles() {
    let mut rig = Rig::new(true);
    rig.activate(Tool::Rotate);
    assert!(
        !rig.a
            .handles(rig.a.viewport_rect)
            .iter()
            .any(|h| (PLANE_HANDLE_BASE..PLANE_HANDLE_BASE + 3).contains(&h.0))
    );
}

#[test]
fn ph34_item_mode_move_has_the_same_circles() {
    let mut rig = Rig::new(true);
    rig.a.run(Command::Mode(Mode::Item));
    rig.frame(vec![]);
    rig.activate(Tool::Move);
    let ids: Vec<usize> = rig.circles().iter().map(|c| c.0).collect();
    assert_eq!(ids, vec![YZ, XZ, XY]);
    let at = rig.circle(XY);
    assert_eq!(rig.a.handle_hit(at, rig.a.viewport_rect), Some(XY));
}
