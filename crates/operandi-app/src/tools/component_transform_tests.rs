//! Component transform frames, the Scale center handle and the Extrude gizmo.
use super::tests::{draw, pointer};
use super::*;

// Local diagnostics only: user scenes stay outside repository fixtures and
// distributions. Compare the same debug build, geometry and camera before/after.
#[test]
#[ignore = "requires OPERANDI_PERF_SCENE pointing to a local diagnostic copy"]
fn component_frame_diagnostic() {
    let path = std::env::var_os("OPERANDI_PERF_SCENE").expect("local scene path");
    let mut a = Operandi::empty();
    a.editor.scene = persistence::load(std::path::Path::new(&path)).unwrap();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    for mode in [Mode::Polygon, Mode::Vertex, Mode::Edge] {
        a.editor.scene.selection.mode = mode;
        let now = std::time::Instant::now();
        let _ = draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(pos2(700., 450.))],
        );
        println!(
            "COMPONENT_FRAME {mode:?} {:.3} ms",
            now.elapsed().as_secs_f64() * 1000.
        );
    }
}
#[test]
fn beveled_scaled_sphere_component_frames_preserve_scene_history_and_attributes() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Sphere);
    a.run(Command::Preview(
        a.editor.session.live.as_ref().unwrap().params.clone(),
    ));
    a.run(Command::Drop);
    for (i, face) in a
        .editor
        .scene
        .active_mut()
        .unwrap()
        .mesh
        .faces
        .iter_mut()
        .enumerate()
    {
        face.uv = Some(vec![glam::DVec2::splat(i as f64); face.vertices.len()]);
        face.material = format!("surface-{i}");
        face.part = Some("original fixture".into());
    }
    a.editor.scene.selection.faces = vec![50, 51];
    a.activate(Tool::Bevel);
    for stage in 0..3 {
        a.run(Command::Preview(Parameters {
            inset: 0.002,
            shift: 0.01,
            ..Default::default()
        }));
        assert_eq!(a.status, "Ready");
        if stage < 2 {
            a.run(Command::Continue);
        }
    }
    a.activate(Tool::Scale);
    a.run(Command::Preview(Parameters {
        delta: DVec3::splat(0.8),
        ..Default::default()
    }));
    a.run(Command::Drop);
    assert_eq!(a.status, "Ready");
    let ctx = Context::default();
    theme::install(&ctx);
    a.camera.view = View::Front;
    for mode in [
        Mode::Vertex,
        Mode::Edge,
        Mode::Polygon,
        Mode::Vertex,
        Mode::Edge,
    ] {
        a.run(Command::Mode(mode));
        a.run(Command::Selection(SelectionOperation::All));
        let before = a.editor.clone();
        let _ = draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![Event::PointerMoved(pos2(700., 450.))],
        );
        assert!(
            a.editor == before,
            "render/hover must preserve {mode:?} state and history"
        );
        let bytes = persistence::encode(&a.editor.scene).unwrap();
        assert_eq!(persistence::decode(&bytes).unwrap(), a.editor.scene);
    }
    a.run(Command::Undo);
    let undone = a.editor.scene.clone();
    a.run(Command::Redo);
    let redone = a.editor.scene.clone();
    assert_ne!(undone.selection, redone.selection);
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    assert_eq!(a.editor.scene, redone);
}
#[test]
fn scale_center_handle_scales_uniformly() {
    // The handle where the Scale tool's three handles meet scales
    // the selection uniformly.
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![1];
    a.camera.view = View::Front;
    a.activate(Tool::Scale);
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    let (_, center, _, label) = a
        .handles(r)
        .into_iter()
        .find(|h| h.0 == 3)
        .expect("center handle");
    assert_eq!(label, "Uniform");
    assert_eq!(
        a.handle_hit(center, r),
        Some(3),
        "center wins over the axis handles"
    );
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(center)]);
    let start_scene = a.editor.scene.clone();
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(center, true, Modifiers::NONE)],
    );
    let end = center + vec2(30., -20.);
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(end, false, Modifiers::NONE)],
    );
    // Measure the drag's change.
    assert!(a.editor.session.tweaks.can_undo());
    let size = |scene: &operandi_core::Scene| {
        let mesh = &scene.active().unwrap().mesh;
        let face = &mesh.faces[1];
        let points: Vec<_> = face
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect();
        let min = points.iter().fold(DVec3::INFINITY, |m, p| m.min(*p));
        let max = points.iter().fold(DVec3::NEG_INFINITY, |m, p| m.max(*p));
        max - min
    };
    let (before, after) = (size(&start_scene), size(&a.editor.scene));
    for axis in 0..3 {
        if before[axis] > 1e-9 {
            let ratio = after[axis] / before[axis];
            assert!(ratio > 1.4, "{ratio} {}", a.status);
        }
    }
    assert_eq!(
        a.editor.session.live.as_ref().unwrap().params.delta,
        DVec3::ONE
    );
}
#[test]
fn extrude_click_starts_then_gizmo_and_free_drag() {
    // User correction 2026-10-30: no gizmo until the first click, which
    // creates the extrusion in place; then the arrows constrain and a drag
    // off the gizmo moves freely.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
            .unwrap()
            .into();
    a.run(Command::Mode(Mode::Polygon));
    let top = {
        let m = &a.editor.scene.active().unwrap().mesh;
        (0..m.faces.len()).find(|&f| m.normal(f).y > 0.9).unwrap() as u32
    };
    a.editor.scene.selection.faces = vec![top];
    a.editor.history.undo.clear();
    a.camera.view = View::Perspective;
    a.run(Command::Activate(Tool::Extrude));
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    assert!(a.handles(r).is_empty(), "no gizmo before the first click");
    let faces = a.editor.scene.active().unwrap().mesh.faces.len();
    let click = r.left_top() + vec2(40., 80.);
    draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(click, true, Modifiers::NONE),
            pointer(click, false, Modifiers::NONE),
        ],
    );
    assert!(!a.handles(r).is_empty(), "{}", a.status);
    // The click made the walls without moving the cap.
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces.len(),
        faces + 4,
        "{}",
        a.status
    );
    assert_eq!(
        a.editor.session.live.as_ref().unwrap().params.extent,
        DVec3::ZERO
    );
    // Dropping right after the click keeps the in-place extrusion (one Undo).
    let mut dropped = a.editor.clone();
    dropped.dispatch(Command::Drop).unwrap();
    assert_eq!(dropped.scene.active().unwrap().mesh.faces.len(), faces + 4);
    dropped.dispatch(Command::Undo).unwrap();
    assert_eq!(dropped.scene.active().unwrap().mesh.faces.len(), faces);
    // Free drag off the gizmo: moves along the view's right axis.
    let start = r.left_top() + vec2(60., 120.);
    assert!(a.handle_hit(start, r).is_none());
    let end = start + vec2(30., 0.);
    draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(start, true, Modifiers::NONE),
            Event::PointerMoved(end),
            pointer(end, false, Modifiers::NONE),
        ],
    );
    let (_, right, _, _) = a.camera.basis();
    let extent = a.editor.session.live.as_ref().unwrap().params.extent;
    assert!(extent.length() > 0., "{}", a.status);
    assert!(
        extent.normalize().dot(right) > 0.999,
        "{extent} {}",
        a.status
    );
    // An arrow drag afterwards still works (constrained to its axis).
    let handle = a.handles(r).into_iter().find(|h| h.0 == 1).unwrap();
    let to = handle.2 + (handle.2 - handle.1).normalized() * 20.;
    draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(handle.2, true, Modifiers::NONE),
            Event::PointerMoved(to),
            pointer(to, false, Modifiers::NONE),
        ],
    );
    let after = a.editor.session.live.as_ref().unwrap().params.extent;
    assert!(after.y > extent.y + 1e-6, "{after} {}", a.status);
    assert!((after.x - extent.x).abs() < 1e-9 && (after.z - extent.z).abs() < 1e-9);
    a.editor.scene.validate().unwrap();
}

/// Vertex mode, Move live, Front view; returns a point on the model (next to
/// an unselected vertex) that is clear of every handle.
fn vertex_move_app(ctx: &Context) -> (Operandi, Pos2) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    a.editor.scene.selection.mode = Mode::Vertex;
    a.editor.scene.selection.vertices = vec![0];
    a.camera.view = View::Front;
    a.activate(Tool::Move);
    theme::install(ctx);
    draw(&mut a, ctx, vec2(1440., 900.), vec![]);
    let r = a.viewport_rect;
    let handles: Vec<Pos2> = a.handles(r).into_iter().map(|h| h.2).collect();
    // Near another vertex: Vertex mode picks within 12 px of a point, which
    // on a dense mesh is almost anywhere on it.
    let mesh = a.editor.scene.active().unwrap().mesh.clone();
    let on_model = mesh
        .vertices
        .iter()
        .filter_map(|&v| a.camera.project(v, r).map(|p| p.0 + vec2(3., 3.)))
        .filter(|&p| {
            a.handle_hit(p, r).is_none()
                && a.pick_component(p, r).is_some()
                && handles.iter().all(|h| h.distance(p) > 40.)
        })
        .last()
        .expect("a point on the model away from the handles");
    (a, on_model)
}

#[test]
fn move_drags_freely_from_the_model_off_the_handles() {
    // Jason, 2026-11-03: a drag anywhere off the handles moves freely in
    // the view plane, also when pressing on the model in Vertex mode.
    let ctx = Context::default();
    let (mut a, start) = vertex_move_app(&ctx);
    let size = vec2(1440., 900.);
    let before = a.editor.scene.active().unwrap().mesh.vertices[0];
    let center = a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .action_frame
        .unwrap()
        .center;
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(start)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(start, true, Modifiers::NONE)],
    );
    let end = start + vec2(40., -30.);
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(end, false, Modifiers::NONE)],
    );
    let after = a.editor.scene.active().unwrap().mesh.vertices[0];
    let moved = after - before;
    // Front view: right is +X, up is +Y, no depth change.
    assert!(moved.x > 0.05 && moved.y > 0.05, "{moved} {}", a.status);
    assert!(moved.z.abs() < 1e-9, "{moved}");
    // C-AUTO (AC-RULES v6; it was "the press on the model keeps the
    // center"): with no Action Center every press puts the center under
    // the pointer, at its own depth; it then follows the moved point.
    let live = a.editor.session.live.as_ref().expect("Move stays active");
    assert_eq!(live.tool, Tool::Move);
    let (origin, _) = a.camera.ray(start, a.viewport_rect);
    let placed = DVec3::new(origin.x, origin.y, center.z);
    let now = live.action_frame.unwrap().center;
    assert!((now - (placed + moved)).length() < 1e-4, "{now}");
}

#[test]
fn command_z_undoes_each_move_drag_and_keeps_the_tool() {
    let ctx = Context::default();
    let (mut a, start) = vertex_move_app(&ctx);
    let size = vec2(1440., 900.);
    let original = a.editor.scene.active().unwrap().mesh.vertices[0];
    let mut positions = vec![original];
    let mut from = start;
    for step in [vec2(40., 0.), vec2(0., -40.)] {
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(from, true, Modifiers::NONE)],
        );
        // Several frames of movement: press to release is still one undo
        // step (Jason, 2026-11-04).
        let to = from + step;
        for t in [0.25, 0.5, 0.75, 1.] {
            draw(
                &mut a,
                &ctx,
                size,
                vec![Event::PointerMoved(from + step * t)],
            );
        }
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(to, false, Modifiers::NONE)],
        );
        positions.push(a.editor.scene.active().unwrap().mesh.vertices[0]);
        from = to;
    }
    assert_ne!(positions[1], positions[0]);
    assert_ne!(positions[2], positions[1]);
    let command_z = |shift: bool| {
        let modifiers = Modifiers {
            shift,
            ..crate::tests::PLATFORM_CMD
        };
        [true, false].map(|pressed| Event::Key {
            key: Key::Z,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        })
    };
    for expected in [positions[1], positions[0]] {
        draw(&mut a, &ctx, size, command_z(false).to_vec());
        assert_eq!(
            a.editor.scene.active().unwrap().mesh.vertices[0],
            expected,
            "{}",
            a.status
        );
        assert_eq!(
            a.editor.session.live.as_ref().map(|l| l.tool),
            Some(Tool::Move)
        );
        // The fields show the restarted tool, not the undone offset.
        assert!(
            a.fields.get("delta0").is_none_or(|v| v == "0 m"),
            "{:?}",
            a.fields
        );
    }
    draw(&mut a, &ctx, size, command_z(true).to_vec());
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.vertices[0],
        positions[1]
    );
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
}

#[test]
fn every_transform_tool_drags_freely_on_the_model_in_every_mode() {
    // Jason, 2026-11-03: off the gizmo, including on the geometry.
    let size = vec2(1440., 900.);
    for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item] {
        for tool in [Tool::Move, Tool::Rotate, Tool::Scale, Tool::Transform] {
            let ctx = Context::default();
            theme::install(&ctx);
            let mut a = Operandi::empty();
            a.run(Command::NewMesh);
            a.editor.scene.active_mut().unwrap().mesh =
                operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
                    .unwrap()
                    .into();
            let s = &mut a.editor.scene.selection;
            s.mode = mode;
            match mode {
                Mode::Vertex => s.vertices = vec![0, 1],
                Mode::Edge => s.edges = vec![(0, 1)],
                Mode::Polygon => s.faces = vec![1],
                _ => {}
            }
            a.camera.view = View::Front;
            a.activate(tool);
            draw(&mut a, &ctx, size, vec![]);
            let r = a.viewport_rect;
            let handles: Vec<Pos2> = a.handles(r).into_iter().map(|h| h.2).collect();
            let mesh = a.editor.scene.active().unwrap().mesh.clone();
            let mut candidates: Vec<DVec3> = mesh.vertices.clone();
            for i in 0..=10 {
                for j in 0..=10 {
                    candidates.push(DVec3::new(
                        -0.9 + 0.18 * i as f64,
                        -0.9 + 0.18 * j as f64,
                        1.,
                    ));
                }
            }
            let start = candidates
                .iter()
                .filter_map(|&v| a.camera.project(v, r).map(|p| p.0 + vec2(3., 3.)))
                .find(|&p| {
                    a.handle_hit(p, r).is_none()
                        && a.pick_component(p, r).is_some()
                        && handles.iter().all(|h| h.distance(p) > 40.)
                })
                .unwrap_or_else(|| panic!("{mode:?} {tool:?}: no point on the model"));
            let before = a.editor.scene.clone();
            draw(&mut a, &ctx, size, vec![Event::PointerMoved(start)]);
            draw(
                &mut a,
                &ctx,
                size,
                vec![pointer(start, true, Modifiers::NONE)],
            );
            let end = start + vec2(40., -30.);
            draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
            draw(
                &mut a,
                &ctx,
                size,
                vec![pointer(end, false, Modifiers::NONE)],
            );
            assert_ne!(a.editor.scene, before, "{mode:?} {tool:?}: {}", a.status);
            // The drag is a tweak of the tool session (Jason, 2026-11-04).
            assert!(a.editor.session.tweaks.can_undo(), "{mode:?}");
            assert_eq!(a.editor.session.live.as_ref().map(|l| l.tool), Some(tool));
        }
    }
}
#[test]
fn local_scale_y_handle_drag_lengthens_two_leaning_edges() {
    // The "Model the Mug" case (Jason, 2026-11-21), through the real
    // handle: two separate leaning side edges of a tapered tube, Action
    // Center Local, drag the Scale tool's Y handle up. Each edge stretches
    // along itself about its own middle, keeping its lean.
    let sides = 8u32;
    let mut vertices = Vec::new();
    for ring in 0..4 {
        for side in 0..sides {
            let t = side as f64 / sides as f64 * std::f64::consts::TAU;
            let radius = 1. - 0.13 * ring as f64;
            vertices.push(DVec3::new(radius * t.cos(), ring as f64, radius * t.sin()));
        }
    }
    let mut faces = Vec::new();
    for ring in 0..3 {
        for side in 0..sides {
            let (p, q) = (ring * sides + side, ring * sides + (side + 1) % sides);
            faces.push(operandi_core::Face {
                vertices: vec![p, q, q + sides, p + sides],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = operandi_core::Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
    .into();
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(8, 16), (12, 20)];
    a.run(Command::ActionContext(operandi_core::ActionContext::LOCAL));
    a.camera.view = View::Front;
    a.activate(Tool::Scale);
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    let (_, _, handle, _) = a
        .handles(r)
        .into_iter()
        .find(|h| h.0 == 1)
        .expect("Y handle");
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let check = |a: &Operandi, factor: f64, what: &str| {
        let m = &a.editor.scene.active().unwrap().mesh;
        for (lo, hi) in [(8, 16), (12, 20)] {
            let c = (before[lo] + before[hi]) / 2.;
            for v in [lo, hi] {
                let want = c + (before[v] - c) * factor;
                assert!(
                    (m.vertices[v] - want).length() < 1e-9,
                    "{what} point {v}: {:?} != {want:?}",
                    m.vertices[v]
                );
            }
        }
    };
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(handle)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(handle, true, Modifiers::NONE)],
    );
    let end = handle + vec2(0., -45.);
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
    let factor = a.editor.session.live.as_ref().unwrap().params.delta.y;
    assert!(factor > 1.3, "Y factor {factor}");
    check(&a, factor, "dragging");
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(end, false, Modifiers::NONE)],
    );
    check(&a, factor, "released");
}
/// A tube of radius 1, `sides` around and three rows of quads (face
/// `row * sides + side`), its points at y = 0..3.
fn tube(sides: u32) -> operandi_core::Mesh {
    let mut vertices = Vec::new();
    for ring in 0..4 {
        for side in 0..sides {
            let t = side as f64 / sides as f64 * std::f64::consts::TAU;
            vertices.push(DVec3::new(t.cos(), ring as f64, t.sin()));
        }
    }
    let mut faces = Vec::new();
    for ring in 0..3 {
        for side in 0..sides {
            let (p, q) = (ring * sides + side, ring * sides + (side + 1) % sides);
            faces.push(operandi_core::Face {
                vertices: vec![p, p + sides, q + sides, q],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    operandi_core::Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
}

#[test]
fn local_handle_sits_on_the_last_selected_cluster() {
    // Jason, 2026-11-21: two polygon clusters on a mug wall (one low, one
    // high), Local. The handle is drawn on the last selected one, at its
    // centre, not floating between them; its Z (blue) points out of the
    // wall, Y (green) up the wall and X (red) around it.
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = tube(8).into();
    a.run(Command::Mode(Mode::Polygon));
    a.run(Command::ActionContext(operandi_core::ActionContext::LOCAL));
    let (low, high) = (1u32, 2 * 8 + 1);
    for order in [vec![low, high], vec![high, low]] {
        a.editor.scene.selection.faces = order.clone();
        a.activate(Tool::Scale);
        let mesh = &a.editor.scene.active().unwrap().mesh;
        let last = *order.last().unwrap() as usize;
        let face = &mesh.faces[last].vertices;
        let corners: Vec<DVec3> = face.iter().map(|&v| mesh.vertices[v as usize]).collect();
        let lo = corners.iter().fold(DVec3::INFINITY, |m, p| m.min(*p));
        let hi = corners.iter().fold(DVec3::NEG_INFINITY, |m, p| m.max(*p));
        let want = (lo + hi) / 2.;
        let normal = mesh.normal(last).normalize();
        let at = a.transform_handle_center();
        assert!((at - want).length() < 1e-9, "{order:?}: {at:?} != {want:?}");
        let basis = a.editor.handle_frame().expect("Local handle frame").basis;
        assert!(
            basis.z_axis.dot(normal) > 1. - 1e-9,
            "Z out of the wall: {basis:?}"
        );
        assert!(basis.y_axis.y > 0.99, "Y up the wall: {basis:?}");
        assert!(basis.x_axis.y.abs() < 1e-9, "X around the wall: {basis:?}");
        a.run(Command::Cancel);
    }
}

#[cfg(test)]
#[test]
fn live_cancel_footer_describes_move_session_and_continued_bevel() {
    use crate::tool_form as tf;

    let cancel = |app: &Operandi| {
        let mut form = tf::ToolForm::<()>::new();
        crate::live_fields::live_footer(app, &mut form);
        form.items
            .into_iter()
            .find_map(|item| match item {
                tf::Item::Buttons(buttons) => {
                    buttons.into_iter().find(|button| button.label == "Cancel")
                }
                _ => None,
            })
            .unwrap()
    };
    let ctx = Context::default();
    let (mut a, mut from) = vertex_move_app(&ctx);
    let size = vec2(1440., 900.);
    let activation = a.editor.scene.clone();
    for delta in [vec2(40., 0.), vec2(0., -40.)] {
        let before = a.editor.scene.clone();
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(from, true, Modifiers::NONE)],
        );
        let to = from + delta;
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(to)]);
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(to, false, Modifiers::NONE)],
        );
        assert_ne!(a.editor.scene, before);
        from = to;
    }
    let button = cancel(&a);
    assert_eq!(button.tip, "Discard the current tool's changes");
    let tf::ButtonAction::Run(run) = button.on else {
        panic!("Cancel must run a command")
    };
    run(&mut a);
    assert_eq!(a.editor.scene, activation);
    assert!(a.editor.session.live.is_none());

    let mut a = crate::tests::box_app();
    a.editor.scene.selection.faces = vec![5];
    let activation = a.editor.scene.clone();
    a.activate(Tool::Bevel);
    assert!(a.run_ok(Command::Preview(Parameters {
        inset: 0.1,
        ..Default::default()
    })));
    assert!(a.run_ok(Command::Continue));
    let continued = a.editor.scene.clone();
    assert_ne!(continued, activation);
    assert!(a.run_ok(Command::Preview(Parameters {
        inset: 0.05,
        ..Default::default()
    })));
    let button = cancel(&a);
    assert_eq!(
        button.tip,
        "Restore this stage; keep previously continued stages"
    );
    let tf::ButtonAction::Run(run) = button.on else {
        panic!("Cancel must run a command")
    };
    run(&mut a);
    assert_eq!(a.editor.scene, continued);
}
