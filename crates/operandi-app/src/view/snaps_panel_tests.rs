//! Snaps and Precision panel and its presets (snaps-precision-2026-11-10
//! panel spec §7), driven with synthetic egui events.
use super::tests::{draw, pointer};
use super::viewport::precision_handles::{
    ABS_BASE, ABS_SCALE, AXIS_END, AXIS_START, ROT_REST, ROT_TARGET,
};
use super::*;
use crate::handle_preferences::HandleDetail;
use operandi_core::{TweakConnect, TweakMode};

/// A 4 × 4 point grid (3 × 3 quads) on z = 0, points one metre apart.
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
    fn new() -> Self {
        let ctx = Context::default();
        theme::install(&ctx);
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid().into();
        a.editor.synchronize_identities().unwrap();
        a.camera.view = View::Front;
        a.camera.target = DVec3::new(1.5, 1.5, 0.);
        a.camera.distance = 8.;
        a.run(Command::Mode(Mode::Polygon));
        // The middle quad: points 5, 6, 10, 9.
        a.editor.scene.selection.faces = vec![4];
        a.editor.history.undo.clear();
        let mut rig = Self {
            a,
            ctx,
            // Tall enough that the Create tab's last button stays above
            // Tool Properties under the in-window menu row (2026-11-22).
            size: vec2(1440., 960.),
        };
        rig.frame(vec![]);
        rig
    }
    fn frame(&mut self, events: Vec<Event>) {
        draw(&mut self.a, &self.ctx, self.size, events);
    }
    fn screen(&self, p: DVec3) -> Pos2 {
        self.a.camera.project(p, self.a.viewport_rect).unwrap().0
    }
    fn point(i: u32) -> DVec3 {
        DVec3::new((i % 4) as f64, (i / 4) as f64, 0.)
    }
    fn click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![pointer(at, true, Modifiers::NONE)]);
        self.frame(vec![pointer(at, false, Modifiers::NONE)]);
        self.frame(vec![]);
    }
    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![pointer(from, true, Modifiers::NONE)]);
        for step in 1..=4 {
            self.frame(vec![Event::PointerMoved(
                from + (to - from) * (step as f32 / 4.),
            )]);
        }
        self.frame(vec![pointer(to, false, Modifiers::NONE)]);
        self.frame(vec![]);
    }
    fn handle(&self, id: usize) -> Pos2 {
        let r = self.a.viewport_rect;
        let (_, at, _) = self
            .a
            .precision_points(r)
            .into_iter()
            .find(|h| h.0 == id)
            .unwrap_or_else(|| panic!("no precision handle {id}"));
        self.screen(at)
    }
    fn params(&self) -> Parameters {
        self.a.editor.session.live.as_ref().unwrap().params.clone()
    }
    /// Press a preset and click point `i` for its Element centre.
    fn start(&mut self, preset: &str, i: u32) {
        self.a.execute(preset);
        assert!(self.a.snap_preset.is_some(), "{}", self.a.status);
        let at = self.screen(Self::point(i));
        self.click(at);
        assert!(
            matches!(self.a.editor.action_context.center, CenterMode::Element(_)),
            "{}",
            self.a.status
        );
    }
}

#[test]
fn panel_opens_from_measure_and_the_edit_menu() {
    let mut rig = Rig::new();
    rig.a.category = toolbox::Category::Create;
    rig.a.create_folded.clear();
    rig.frame(vec![]);
    let button = rig.a.toolbox_rects["Snaps and Precision"].center();
    rig.click(button);
    assert!(rig.a.snaps_open);
    rig.click(button);
    assert!(!rig.a.snaps_open);
    rig.a.execute("Snaps and Precision");
    assert!(rig.a.snaps_open);
    // It draws without trouble.
    rig.frame(vec![]);
}

#[test]
fn drag_snap_presets_start_element_move_with_snapping() {
    let mut rig = Rig::new();
    assert!(!rig.a.snapping.enabled);
    rig.a.execute("Drag Snap");
    assert!(rig.a.snapping.enabled, "a preset turns snapping on (Q2)");
    let o = rig.params().tweak_options;
    assert_eq!(
        rig.a.editor.session.live.as_ref().unwrap().tool,
        Tool::Tweak
    );
    assert_eq!(
        (o.mode, o.range, o.connect),
        (TweakMode::Auto, 0., TweakConnect::Connectivity)
    );
    rig.a.execute("Drag Snap Rigid");
    assert_eq!(rig.params().tweak_options.connect, TweakConnect::Rigid);
    // Pressing the running preset again drops it; snapping stays on.
    rig.a.execute("Drag Snap Rigid");
    assert!(rig.a.editor.session.live.is_none());
    assert!(rig.a.snap_preset.is_none());
    assert!(rig.a.snapping.enabled);
    // The script name works too, `snap:true` and all.
    rig.a.execute("tool.set SnapDrag 1 snap:true");
    assert_eq!(
        rig.a.editor.session.live.as_ref().unwrap().tool,
        Tool::Tweak,
        "{}",
        rig.a.status
    );
}

#[test]
fn drag_snap_lands_a_vertex_on_another() {
    let mut rig = Rig::new();
    rig.a.execute("Drag Snap");
    // Drag point 5 (1,1) to near point 15 (3,3): it lands exactly there.
    let from = rig.screen(Rig::point(5));
    let to = rig.screen(Rig::point(15)) + vec2(4., -3.);
    rig.drag(from, to);
    let mesh = &rig.a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.vertices[5], Rig::point(15), "{}", rig.a.status);
    // Nothing else moved.
    assert_eq!(mesh.vertices[6], Rig::point(6));
}

#[test]
fn advanced_handles_sets_the_style_one_way() {
    let mut rig = Rig::new();
    assert_eq!(rig.a.handle_preferences.selected, HandleDetail::Basic);
    rig.a.execute("Advanced Handles");
    assert_eq!(rig.a.handle_preferences.selected, HandleDetail::Advanced);
    rig.a.execute("Advanced Handles");
    assert_eq!(rig.a.handle_preferences.selected, HandleDetail::Advanced);
    rig.a.execute("tool.handleStyle default");
    assert_eq!(rig.a.handle_preferences.selected, HandleDetail::Basic);
    rig.a.execute("tool.handleStyle invisible");
    assert_eq!(rig.a.handle_preferences.selected, HandleDetail::Invisible);
}

#[test]
fn scale_absolute_snaps_base_then_scale_handle() {
    let mut rig = Rig::new();
    rig.start("Scale Absolute", 0);
    let abs = rig.params().absolute_handles.unwrap();
    assert_eq!(abs.base, DVec3::splat(1.2), "spec D: base 1.2");
    // Base X onto point 1 (one metre along X), then scale X onto point 3.
    let from = rig.handle(ABS_BASE);
    rig.drag(from, rig.screen(Rig::point(1)) + vec2(3., 2.));
    let base = rig.params().absolute_handles.unwrap().base.x;
    assert!(
        (base.abs() - 1.).abs() < 1e-9,
        "base {base}; {}",
        rig.a.status
    );
    let from = rig.handle(ABS_SCALE);
    rig.drag(from, rig.screen(Rig::point(3)) + vec2(-2., 3.));
    // One undo step was recorded with the scale ×3 on X about x = 0.
    let mesh = &rig.a.editor.scene.active().unwrap().mesh;
    assert!(
        (mesh.vertices[5].x - 3.).abs() < 1e-9,
        "{}",
        mesh.vertices[5]
    );
    assert!(
        (mesh.vertices[6].x - 6.).abs() < 1e-9,
        "{}",
        mesh.vertices[6]
    );
    assert_eq!(mesh.vertices[5].y, 1., "Y unchanged");
}

#[test]
fn size_absolute_scales_by_relative_over_absolute() {
    // SZ-size_04 from the spec: Absolute 2, 1, 0.5 and Relative 4, 3, 1
    // scale by ×2, ×3, ×2 about the centre.
    let mut rig = Rig::new();
    rig.start("Size Absolute", 0);
    let mut p = rig.params();
    let mut h = p.absolute_handles.unwrap();
    assert!(h.size);
    h.base = DVec3::new(2., 1., 0.5);
    p.absolute_handles = Some(h);
    p.delta = DVec3::new(4., 3., 1.) / h.base;
    rig.a.run(Command::Preview(p));
    let mesh = &rig.a.editor.scene.active().unwrap().mesh;
    // Point 5 (1, 1, 0) about the origin: (2, 3, 0) (the frame's X may
    // point either way along world X; the scale is the same).
    assert!(
        (mesh.vertices[5] - DVec3::new(2., 3., 0.)).length() < 1e-9,
        "{}",
        mesh.vertices[5]
    );
}

#[test]
fn planar_rotate_snap_turns_from_rest_to_target() {
    let mut rig = Rig::new();
    rig.start("Planar Rotate Snap", 0);
    // The axis is the grid's normal; rest onto point 1 (+X), target onto
    // point 4 (+Y): a quarter turn.
    let from = rig.handle(ROT_REST);
    rig.drag(from, rig.screen(Rig::point(2)));
    let from = rig.handle(ROT_TARGET);
    rig.drag(from, rig.screen(Rig::point(8)));
    let mesh = &rig.a.editor.scene.active().unwrap().mesh;
    // Point 5 (1, 1) turned a quarter about the origin: (−1, 1).
    assert!(
        (mesh.vertices[5] - DVec3::new(-1., 1., 0.)).length() < 1e-6,
        "{}; {}",
        mesh.vertices[5],
        rig.a.status
    );
}

#[test]
fn planar_rotate_snap_axis_sets_the_axis_from_two_points() {
    let mut rig = Rig::new();
    rig.start("Planar Rotate Snap Axis", 0);
    let from = rig.handle(AXIS_START);
    rig.drag(from, rig.screen(Rig::point(1)));
    let from = rig.handle(AXIS_END);
    rig.drag(from, rig.screen(Rig::point(13)));
    let AxisOrientation::Element(frame) = rig.a.editor.action_context.axis else {
        panic!("axis not set: {}", rig.a.status);
    };
    let z = frame.basis().z_axis;
    assert!((z.abs() - DVec3::Y).length() < 1e-6, "{z}");
}

#[test]
fn leaving_a_preset_puts_the_action_centre_back() {
    let mut rig = Rig::new();
    let before = rig.a.editor.action_context;
    rig.start("Scale Absolute", 0);
    assert_ne!(rig.a.editor.action_context, before);
    rig.a.run(Command::Drop);
    rig.frame(vec![]);
    assert!(rig.a.snap_preset.is_none());
    assert_eq!(rig.a.editor.action_context, before);
}
