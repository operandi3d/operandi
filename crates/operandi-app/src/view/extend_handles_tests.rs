//! Edge Extend's handles, Plane circles, Haul and Shift+click in the
//! viewport (SPEC-EDGE-EXTEND-ROTATE-SCALE v1).
use super::tests::{draw, pointer};
use super::*;
use crate::viewport::plane_handles;
use crate::viewport::{EXTEND_INSET_HANDLE, EXTEND_SHIFT_HANDLE};
use operandi_core::{ExtendHaul, ExtendPlane};

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
    /// The top boundary 12-13-14-15 selected, Edge Extend live, Front view.
    fn new() -> Self {
        let ctx = Context::default();
        theme::install(&ctx);
        let size = vec2(1440., 900.);
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid().into();
        a.camera.view = View::Front;
        // The selected top row (and its handles) mid-view.
        a.camera.target = DVec3::new(1.5, 3., 0.);
        a.camera.distance = 8.;
        a.run(Command::Mode(Mode::Edge));
        a.editor.scene.selection.edges = vec![(12, 13), (13, 14), (14, 15)];
        a.editor.history.undo.clear();
        let mut rig = Self { a, ctx, size };
        rig.frame(vec![]);
        rig.a.activate(Tool::EdgeExtend);
        assert_eq!(rig.tool(), Some(Tool::EdgeExtend), "{}", rig.a.status);
        rig.frame(vec![]);
        rig
    }
    fn frame(&mut self, events: Vec<Event>) {
        draw(&mut self.a, &self.ctx, self.size, events);
    }
    fn tool(&self) -> Option<Tool> {
        self.a.editor.session.live.as_ref().map(|l| l.tool)
    }
    fn params(&self) -> Parameters {
        self.a.editor.session.live.as_ref().unwrap().params.clone()
    }
    fn set(&mut self, f: impl FnOnce(&mut Parameters)) {
        let mut p = self.params();
        f(&mut p);
        self.a.run(Command::Preview(p));
        self.frame(vec![]);
    }
    fn ids(&self) -> Vec<usize> {
        let mut ids: Vec<usize> = self
            .a
            .handles(self.a.viewport_rect)
            .iter()
            .map(|h| h.0)
            .collect();
        ids.sort();
        ids
    }
    fn handle_end(&self, id: usize) -> Pos2 {
        self.a
            .handles(self.a.viewport_rect)
            .into_iter()
            .find(|h| h.0 == id)
            .unwrap_or_else(|| panic!("handle {id}"))
            .2
    }
    fn corner(&self) -> Pos2 {
        self.a.viewport_rect.left_top() + vec2(80., 80.)
    }
    fn drag_with(&mut self, from: Pos2, by: Vec2, modifiers: Modifiers) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![pointer(from, true, modifiers)]);
        for step in 1..=4 {
            self.frame(vec![Event::PointerMoved(from + by * (step as f32 / 4.))]);
        }
        self.frame(vec![pointer(from + by, false, modifiers)]);
    }
    fn drag(&mut self, from: Pos2, by: Vec2) {
        self.drag_with(from, by, Modifiers::NONE);
    }
    fn points(&self) -> usize {
        self.a.editor.scene.active().unwrap().mesh.vertices.len()
    }
}

#[test]
fn move_handles_show_by_default_and_each_set_toggles() {
    let mut rig = Rig::new();
    // Spec v1.1: Move on, the rest off. Front view: the Z arrow points at
    // the camera and is left out.
    assert_eq!(rig.ids(), vec![0, 1]);
    rig.set(|p| p.edge_extend.handles.scale = true);
    assert_eq!(rig.ids(), vec![0, 1, 6, 7]);
    rig.set(|p| {
        p.edge_extend.handles.moves = false;
        p.edge_extend.handles.scale = false;
        p.edge_extend.handles.rotate = true;
        p.edge_extend.handles.local = true;
    });
    let ids = rig.ids();
    assert!(ids.contains(&5), "the Z ring faces the view: {ids:?}");
    // Shift runs along +Z, at the camera, so only Inset is drawn here.
    assert!(ids.contains(&EXTEND_INSET_HANDLE) && !ids.contains(&EXTEND_SHIFT_HANDLE));
    assert!(!ids.iter().any(|&i| i < 3 || (6..9).contains(&i)));
}

#[test]
fn scale_handle_drag_scales_the_new_edges() {
    let mut rig = Rig::new();
    // A straight row scaled along itself in place would make flat quads.
    rig.set(|p| {
        p.delta.y = 0.5;
        p.edge_extend.handles.scale = true;
    });
    let at = rig.handle_end(6);
    assert_eq!(rig.a.handle_hit(at, rig.a.viewport_rect), Some(6));
    rig.drag(at, vec2(-40., 0.));
    let p = rig.params();
    assert!(p.scale_delta.x < 1., "{}", rig.a.status);
    assert_eq!((p.scale_delta.y, p.scale_delta.z), (1., 1.));
    assert_eq!(rig.points(), 20, "4 new points");
    // The move arrow still moves the Offset, and the handles follow it.
    let before = rig.handle_end(1);
    rig.drag(before, vec2(0., -40.));
    assert!(rig.params().delta.y > 0.);
    assert!(
        rig.handle_end(1).y < before.y - 20.,
        "{before:?} {:?}",
        rig.handle_end(1)
    );
}

#[test]
fn rotate_ring_turns_the_new_edges() {
    let mut rig = Rig::new();
    rig.set(|p| {
        p.delta.y = 0.5;
        p.edge_extend.handles.rotate = true;
    });
    let center = rig
        .a
        .camera
        .project(rig.a.transform_handle_center(), rig.a.viewport_rect)
        .unwrap()
        .0;
    let at = rig.handle_end(5);
    assert_eq!(rig.a.handle_hit(at, rig.a.viewport_rect), Some(5));
    // Around the centre: a quarter turn's worth of arc.
    let by = (center + (at - center).rot90()) - at;
    rig.drag(at, by * 0.3);
    assert!(rig.params().rotation_delta.z.abs() > 1., "{}", rig.a.status);
}

#[test]
fn plane_scale_circle_scales_both_axes_evenly() {
    let mut rig = Rig::new();
    assert!(!rig.ids().iter().any(|&i| plane_handles::is_plane_handle(i)));
    rig.set(|p| {
        p.delta.y = 0.5;
        p.edge_extend.plane = ExtendPlane::Scale;
    });
    // Front view looks down Z: the XY circle (normal Z).
    let xy = plane_handles::PLANE_HANDLE_BASE + 2;
    let at = rig.handle_end(xy);
    assert_eq!(rig.a.handle_hit(at, rig.a.viewport_rect), Some(xy));
    rig.drag(at, vec2(-30., 0.));
    let s = rig.params().scale_delta;
    assert!(
        s.x < 1. && (s.x - s.y).abs() < 1e-12 && s.z == 1.,
        "{s:?} {}",
        rig.a.status
    );
    // Plane Move moves the Offset in that plane instead.
    rig.set(|p| p.edge_extend.plane = ExtendPlane::Move);
    let at = rig.handle_end(xy);
    rig.drag(at, vec2(20., -20.));
    let d = rig.params().delta;
    assert!(d.x > 0. && d.y > 0.5 && d.z == 0., "{d:?}");
}

#[test]
fn haul_local_drags_inset_across_and_shift_up() {
    let mut rig = Rig::new();
    let corner = rig.corner();
    assert_eq!(rig.a.handle_hit(corner, rig.a.viewport_rect), None);
    rig.drag(corner, vec2(0., -40.));
    assert!(rig.params().delta.y > 0., "Global Offsets by default");
    rig.set(|p| {
        p.delta = DVec3::ZERO;
        p.edge_extend.haul = ExtendHaul::Local;
    });
    rig.drag(corner, vec2(40., 0.));
    let p = rig.params();
    assert!(p.inset > 0. && p.shift == 0. && p.delta == DVec3::ZERO);
    rig.drag(corner, vec2(0., -40.));
    assert!(rig.params().shift > 0.);
}

#[test]
fn local_handles_drag_inset_and_shift() {
    let mut rig = Rig::new();
    rig.set(|p| p.edge_extend.handles.local = true);
    let (inset, shift) = rig.a.extend_local_axes().unwrap();
    // The top edge's polygon: Inset points up (out of it), Shift along +Z.
    assert!((inset - DVec3::Y).length() < 1e-9 && (shift - DVec3::Z).length() < 1e-9);
    let at = rig.handle_end(EXTEND_INSET_HANDLE);
    assert_eq!(
        rig.a.handle_hit(at, rig.a.viewport_rect),
        Some(EXTEND_INSET_HANDLE)
    );
    rig.drag(at, vec2(0., -30.));
    assert!(rig.params().inset > 0., "{}", rig.a.status);
}

#[test]
fn shift_click_extends_again_with_the_same_values() {
    let mut rig = Rig::new();
    let corner = rig.corner();
    rig.drag(corner, vec2(0., -40.));
    assert_eq!(rig.points(), 20);
    let first = rig.params().delta;
    rig.drag_with(corner, vec2(0., 0.), Modifiers::SHIFT);
    assert_eq!(rig.points(), 24, "{}", rig.a.status);
    assert_eq!(rig.params().delta, first);
    assert_eq!(rig.tool(), Some(Tool::EdgeExtend));
    // Undo with the tool on goes back to the single extension.
    rig.a.run(Command::Undo);
    assert_eq!(rig.points(), 20, "{}", rig.a.status);
    assert_eq!(rig.tool(), Some(Tool::EdgeExtend));
}

/// The registry's reference names drive the same fields (tool.attr).
#[test]
fn tool_attr_takes_the_reference_names() {
    let mut rig = Rig::new();
    rig.a.execute("tool.attr edge.extend offY 0.5");
    rig.a.execute("tool.attr edge.extend sclX 0.5");
    rig.a.execute("tool.attr edge.extend rotZ 0.1");
    rig.a.execute("tool.attr edge.extend segs 2");
    rig.a.execute("tool.attr edge.extend planeHandle scale");
    rig.a.execute("tool.attr edge.extend hauling local");
    rig.a.execute("tool.attr edge.extend uvs v");
    assert_eq!(
        rig.params().edge_extend.make_uvs,
        operandi_core::ExtendUvs::V
    );
    rig.a.execute("tool.attr edge.extend uvs 0");
    assert_eq!(
        rig.params().edge_extend.make_uvs,
        operandi_core::ExtendUvs::Off
    );
    rig.a.execute("tool.attr edge.extend planeHandle 1");
    assert_eq!(rig.params().edge_extend.plane, ExtendPlane::Move);
    rig.a.execute("tool.attr edge.extend uvs v");
    rig.a.execute("tool.attr edge.extend rotateHandle true");
    let p = rig.params();
    assert_eq!(p.delta.y, 0.5, "{}", rig.a.status);
    assert!((p.scale_delta.x - 0.5).abs() < 1e-12);
    assert!((p.rotation_delta.z - 0.1_f64.to_degrees()).abs() < 1e-9);
    assert_eq!(p.segments[0], 2);
    assert_eq!(p.edge_extend.plane, ExtendPlane::Move);
    assert_eq!(p.edge_extend.haul, ExtendHaul::Local);
    assert_eq!(p.edge_extend.make_uvs, operandi_core::ExtendUvs::V);
    assert!(p.edge_extend.handles.rotate);
    rig.a.execute("tool.attr edge.extend sclX ?");
    assert_eq!(rig.a.status, "0.5");
}
