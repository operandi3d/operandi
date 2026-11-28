//! Original PM acceptance fixtures. These exercise egui event routing and the
//! real preview/history/persistence services; they are not native-input tests.
use super::*;

fn frame(
    a: &mut Operandi,
    ctx: &Context,
    mods: Modifiers,
    focused: bool,
    events: Vec<Event>,
) -> FullOutput {
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 900.))),
            modifiers: mods,
            focused,
            events,
            ..Default::default()
        },
        |ctx| {
            a.begin_input(ctx);
            a.compose(ctx);
            a.keyboard(ctx);
        },
    )
}
fn pointer(p: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: p,
        pressed,
        modifiers,
        button: PointerButton::Primary,
    }
}
fn drag(a: &mut Operandi, ctx: &Context, start: Pos2, end: Pos2, mods: Modifiers) {
    frame(
        a,
        ctx,
        mods,
        true,
        vec![
            pointer(start, true, mods),
            Event::PointerMoved(end),
            pointer(end, false, mods),
        ],
    );
}
fn fixture() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Box);
    a.run(Command::Preview(Parameters {
        position: DVec3::new(-4., 0., 0.),
        size: DVec3::new(1., 2., 3.),
        ..Default::default()
    }));
    a.run(Command::Drop);
    let mesh = &mut a.editor.scene.active_mut().unwrap().mesh;
    mesh.faces[0].uv = Some(vec![glam::DVec2::new(0.25, 0.75); 4]);
    mesh.faces[0].material = "original material".into();
    mesh.uv_maps.insert(
        "seam".into(),
        vec![vec![Some(glam::DVec2::new(0.125, 0.875)); 4]; 6],
    );
    a
}
fn assert_unchanged(a: &Operandi, before: &Editor) {
    assert_eq!(a.editor.scene, before.scene);
    assert!(a.editor.session.live == before.session.live);
    assert_eq!(a.editor.history.undo.len(), before.history.undo.len());
    assert_eq!(a.editor.history.redo.len(), before.history.redo.len());
    assert_eq!(
        *a.editor.presets.get::<operandi_core::CubePreset>(),
        *before.presets.get::<operandi_core::CubePreset>()
    );
}
/// Create-panel cells on the primitive-quick template (MB-Q8): Cube,
/// Sphere, Ellipsoid, Cone, Cylinder, Torus, Capsule. Each shows a "Unit"
/// tag while Ctrl is held and an "Item" tag while Shift is held.
const UNIT_CELLS: usize = 7;
fn text_count(output: &FullOutput, needle: &str) -> usize {
    fn count(s: &egui::Shape, needle: &str) -> usize {
        match s {
            egui::Shape::Text(t) => usize::from(t.galley.text() == needle),
            egui::Shape::Vec(shapes) => shapes.iter().map(|s| count(s, needle)).sum(),
            _ => 0,
        }
    }
    output.shapes.iter().map(|s| count(&s.shape, needle)).sum()
}

#[test]
fn pm_t07_t09_t12_preview_before_entry_release_focus_and_scale_are_scene_neutral() {
    for scale in [1., 2.] {
        for active in [false, true] {
            let mut a = fixture();
            if active {
                a.activate(Tool::Sphere);
            }
            let ctx = Context::default();
            theme::install(&ctx);
            ctx.set_pixels_per_point(scale);
            for _ in 0..2 {
                frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
            }
            let rects = a.toolbox_rects.clone();
            let before = a.editor.clone();
            let viewport_center = a.viewport_rect.center();
            let output = frame(
                &mut a,
                &ctx,
                Modifiers::CTRL,
                true,
                vec![Event::PointerMoved(viewport_center)],
            );
            assert_eq!(text_count(&output, "Unit"), UNIT_CELLS);
            for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
                let point = a.toolbox_rects[tool.label()].center();
                let output = frame(
                    &mut a,
                    &ctx,
                    Modifiers::CTRL,
                    true,
                    vec![Event::PointerMoved(point)],
                );
                assert_eq!(text_count(&output, "Unit"), UNIT_CELLS);
                assert_eq!(a.toolbox_rects[tool.label()], rects[tool.label()]);
            }
            assert_unchanged(&a, &before);
            let output = frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
            assert_eq!(text_count(&output, "Unit"), 0);
            let output = frame(
                &mut a,
                &ctx,
                Modifiers::CTRL,
                false,
                vec![Event::WindowFocused(false)],
            );
            assert_eq!(text_count(&output, "Unit"), 0);
            let output = frame(
                &mut a,
                &ctx,
                Modifiers::NONE,
                true,
                vec![Event::WindowFocused(true)],
            );
            assert_eq!(text_count(&output, "Unit"), 0);
            assert_unchanged(&a, &before);
        }
    }
}

#[test]
fn pm_t05_t06_t08_t10_unit_clicks_route_by_event_and_preserve_live_task_guards() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        for live in [false, true] {
            for mods in [
                Modifiers::CTRL,
                Modifiers::SHIFT,
                Modifiers {
                    ctrl: true,
                    shift: true,
                    ..Modifiers::NONE
                },
            ] {
                let mut a = fixture();
                if live {
                    a.activate(tool);
                }
                let ctx = Context::default();
                theme::install(&ctx);
                frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
                let p = a.toolbox_rects[tool.label()].center();
                let before = a.editor.clone();
                // Real per-event Control, even when final raw state is neutral.
                frame(
                    &mut a,
                    &ctx,
                    Modifiers::NONE,
                    true,
                    vec![pointer(p, true, mods), pointer(p, false, mods)],
                );
                if mods.ctrl && mods.shift {
                    // MB-13: an unmatched pair runs the base (draw / drop).
                    // Clicking the open tool's button drops it.
                    assert_eq!(
                        a.editor.session.live.as_ref().map(|l| l.tool),
                        (!live).then_some(tool),
                        "{}",
                        a.status
                    );
                    assert_eq!(a.editor.scene.items.len(), before.scene.items.len());
                } else {
                    // A live tool (nothing drawn yet) is dropped first.
                    assert!(a.editor.session.live.is_none());
                    assert_eq!(a.editor.history.undo.len(), before.history.undo.len() + 1);
                    assert_eq!(
                        a.editor.scene.items.len(),
                        before.scene.items.len() + usize::from(mods.shift)
                    );
                    let result = a.editor.scene.clone();
                    assert!(!result.active().unwrap().mesh.faces.is_empty());
                    a.run(Command::Undo);
                    assert_eq!(a.editor.scene, before.scene);
                    a.run(Command::Redo);
                    assert_eq!(a.editor.scene, result);
                }
            }
        }
    }
    // An empty scene is not an implied destination or a new mesh request.
    let mut a = Operandi::empty();
    let ctx = Context::default();
    frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
    let p = a.toolbox_rects["Sphere"].center();
    let before = a.editor.clone();
    drag(&mut a, &ctx, p, p, Modifiers::CTRL);
    assert_unchanged(&a, &before);
    a.run(Command::NewMesh);
    // Release before click routes ordinary activation, never a stale unit action.
    frame(&mut a, &ctx, Modifiers::CTRL, true, vec![]);
    drag(&mut a, &ctx, p, p, Modifiers::NONE);
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Sphere);
    assert!(a.editor.scene.active().unwrap().mesh.vertices.is_empty());
}

#[test]
fn pm_t08_t09_field_menu_modal_and_navigation_own_modifiers() {
    for owner in ["text", "menu", "modal", "navigation"] {
        let mut a = fixture();
        a.activate(Tool::Sphere);
        let ctx = Context::default();
        theme::install(&ctx);
        for _ in 0..2 {
            frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
        }
        match owner {
            "text" => {
                let id = Id::new("pm-focused-field");
                a.text_editors.insert(id);
                ctx.memory_mut(|m| m.request_focus(id));
            }
            "menu" => {
                egui::Popup::open_id(&ctx, Id::new("pm-popup"));
            }
            "modal" => {
                a.show_bindings = true;
            }
            _ => {
                a.navigation = Some(input::Navigation {
                    kind: input::GestureKind::Navigate(input::NavigationKind::Orbit),
                    start: Pos2::ZERO,
                    last: Pos2::ZERO,
                    modifiers: Modifiers::ALT,
                    button: PointerButton::Primary,
                });
            }
        }
        let before = a.editor.clone();
        let output = frame(&mut a, &ctx, Modifiers::CTRL, true, vec![]);
        // MB-15: labels swap app-wide, also while a text field has focus.
        if owner == "text" {
            assert_eq!(text_count(&output, "Unit"), UNIT_CELLS);
        }
        assert_unchanged(&a, &before);
        // Menus, modals and navigation own a Ctrl-click: nothing changes.
        // A focused text field doesn't (MB-15: modifiers act app-wide), so
        // the click inserts a unit Cube, dropping the open Sphere first.
        {
            if owner == "text" {
                // The test field is never drawn, so keep it focused for the
                // click.
                ctx.memory_mut(|m| m.request_focus(Id::new("pm-focused-field")));
            }
            let p = a.toolbox_rects["Cube"].center();
            frame(
                &mut a,
                &ctx,
                Modifiers::NONE,
                true,
                vec![
                    pointer(p, true, Modifiers::CTRL),
                    pointer(p, false, Modifiers::CTRL),
                ],
            );
            if owner == "text" {
                assert!(a.editor.session.live.is_none(), "{}", a.status);
                assert_eq!(a.editor.history.undo.len(), before.history.undo.len() + 1);
                assert_ne!(a.editor.scene, before.scene);
            } else {
                assert_unchanged(&a, &before);
            }
        }
    }
}

#[test]
fn pm_t01_t11_uniform_sphere_geometry_seams_history_and_persistence() {
    for (view, plane, end) in [
        (View::Top, 0, DVec3::new(4., 0., -1.)),
        (View::Front, 2, DVec3::new(-1., 4., 0.)),
        (View::Right, 1, DVec3::new(0., -1., 4.)),
        (View::Perspective, 0, DVec3::new(-4., 0., 1.)),
    ] {
        let mut a = fixture();
        a.camera.view = view;
        a.editor.scene.workplane.fixed = true;
        a.editor.scene.workplane.plane = plane;
        a.activate(Tool::Sphere);
        let ctx = Context::default();
        theme::install(&ctx);
        for _ in 0..2 {
            frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
        }
        let before = a.editor.scene.clone();
        let history = a.editor.history.undo.len();
        let r = a.viewport_rect;
        let start = a.camera.project(DVec3::ZERO, r).unwrap().0;
        let end = a.camera.project(end, r).unwrap().0;
        let p0 = a.camera.plane_point(start, r, plane, DVec3::ZERO).unwrap();
        let p1 = a.camera.plane_point(end, r, plane, DVec3::ZERO).unwrap();
        drag(&mut a, &ctx, start, end, Modifiers::CTRL);
        let params = a.editor.session.live.as_ref().unwrap().params.clone();
        let radius = params.radial.unwrap().radius;
        assert!(
            a.editor.session.live.as_ref().unwrap().applied,
            "{}",
            a.status
        );
        assert_eq!(radius.x, radius.y);
        assert_eq!(radius.y, radius.z);
        assert!((radius.x - (p1 - p0).abs().max_element()).abs() < 1e-8);
        assert!(params.position.abs_diff_eq(p0, 1e-8));
        let mesh = &a.editor.scene.active().unwrap().mesh;
        let original = &before.active().unwrap().mesh;
        assert_eq!(
            &mesh.vertices[..original.vertices.len()],
            &original.vertices
        );
        assert_eq!(&mesh.faces[..original.faces.len()], &original.faces[..]);
        assert_eq!(&mesh.uv_maps["seam"][..6], &original.uv_maps["seam"]);
        for v in &mesh.vertices[original.vertices.len()..] {
            assert!(((v - params.position).length() - radius.x).abs() < 1e-8);
        }
        mesh.validate().unwrap();
        assert_eq!(a.editor.history.undo.len(), history);
        a.run(Command::Drop);
        let result = a.editor.scene.clone();
        assert_eq!(a.editor.history.undo.len(), history + 1);
        a.run(Command::Undo);
        assert_eq!(a.editor.scene, before);
        a.run(Command::Redo);
        assert_eq!(a.editor.scene, result);
        assert!(a.editor.session.live.is_none());
        let path = std::env::temp_dir().join(format!(
            "operandi-pm-sphere-{}-{plane}.opscene",
            std::process::id()
        ));
        operandi_core::persistence::save(&path, &result).unwrap();
        assert_eq!(operandi_core::persistence::load(&path).unwrap(), result);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn pm_t04_ctrl_side_moves_preserve_all_dimensions_modes_and_unrelated_geometry() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        for mode in [BoxHandleMode::Resize, BoxHandleMode::FromCenter] {
            for axis in 0..3 {
                for maximum in [false, true] {
                    let mut a = fixture();
                    a.camera.view = View::Perspective;
                    a.camera.distance = 12.;
                    a.activate(tool);
                    let mut params = a.editor.session.live.as_ref().unwrap().params.clone();
                    params.position = DVec3::new(1., 1., 1.);
                    params.size = DVec3::new(2., 3., 4.);
                    params.box_handle_mode = mode;
                    if let Some(radial) = params.radial.as_mut() {
                        radial.radius = DVec3::new(1., 1.5, 2.);
                        radial.sides = 8;
                        radial.segments = 4;
                    }
                    a.run(Command::Preview(params.clone()));
                    let ctx = Context::default();
                    theme::install(&ctx);
                    for _ in 0..2 {
                        frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
                    }
                    let before = a.editor.scene.clone();
                    let r = a.viewport_rect;
                    let id = 3 + axis * 2 + usize::from(maximum);
                    let start = a.handles(r).into_iter().find(|h| h.0 == id).unwrap().2;
                    assert_eq!(a.handle_hit(start, r), Some(id));
                    let direction = [DVec3::X, DVec3::Y, DVec3::Z][axis];
                    let unit = a.camera.project(params.position + direction, r).unwrap().0
                        - a.camera.project(params.position, r).unwrap().0;
                    drag(&mut a, &ctx, start, start + unit * 0.4, Modifiers::CTRL);
                    let actual = &a.editor.session.live.as_ref().unwrap().params;
                    assert_eq!(actual.size, params.size);
                    assert_eq!(actual.radial, params.radial);
                    assert_eq!(actual.box_handle_mode, mode);
                    let delta = actual.position - params.position;
                    assert!(
                        delta.abs_diff_eq(direction * 0.4, 1e-5),
                        "{tool:?}/{axis}/{maximum}: {delta:?}: {}",
                        a.status
                    );
                    let base_count = a
                        .editor
                        .session
                        .live
                        .as_ref()
                        .unwrap()
                        .base
                        .active()
                        .unwrap()
                        .mesh
                        .vertices
                        .len();
                    let mesh = &a.editor.scene.active().unwrap().mesh;
                    let prior = &before.active().unwrap().mesh;
                    assert_eq!(&mesh.vertices[..base_count], &prior.vertices[..base_count]);
                    assert_eq!(mesh.faces, prior.faces);
                    assert_eq!(mesh.uv_maps, prior.uv_maps);
                    for (v, p) in mesh.vertices[base_count..]
                        .iter()
                        .zip(&prior.vertices[base_count..])
                    {
                        assert!(v.abs_diff_eq(*p + delta, 1e-8));
                    }
                    // The next ordinary drag resizes again; Ctrl never changes
                    // the stored mode or drops the current creation task.
                    let moved = actual.clone();
                    let start = a.handles(r).into_iter().find(|h| h.0 == id).unwrap().2;
                    let sign = if maximum { 1. } else { -1. };
                    drag(
                        &mut a,
                        &ctx,
                        start,
                        start + unit * (0.2 * sign),
                        Modifiers::NONE,
                    );
                    let resized = &a.editor.session.live.as_ref().unwrap().params;
                    if tool == Tool::Box {
                        assert_ne!(resized.size, moved.size);
                    } else {
                        assert_ne!(resized.radial, moved.radial);
                    }
                    assert_eq!(resized.box_handle_mode, mode);
                    a.run(Command::Cancel);
                    assert_eq!(
                        a.editor.scene.active().unwrap().mesh.vertices.len(),
                        base_count
                    );
                }
            }
        }
    }
}

// Jason decision 2026-11-01 (PM-Q02): only the modifiers held at press count;
// pressing or releasing one mid-drag is ignored and the drag carries on.
#[test]
fn pm_t03_modifier_change_mid_drag_is_ignored_and_focus_loss_releases_capture() {
    for (tool, initial) in [Tool::Box, Tool::Sphere, Tool::Cylinder]
        .into_iter()
        .flat_map(|tool| [Modifiers::CTRL, Modifiers::NONE].map(|initial| (tool, initial)))
    {
        let changed = if initial.ctrl {
            Modifiers::NONE
        } else {
            Modifiers::CTRL
        };
        // The same drag twice: once holding `initial` throughout, once
        // switching to `changed` partway. Both must end identically.
        let run = |late: Modifiers| {
            let mut a = fixture();
            a.camera.view = View::Top;
            a.activate(tool);
            let ctx = Context::default();
            theme::install(&ctx);
            frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
            let start = a.viewport_rect.center();
            let end = start + vec2(90., 35.);
            frame(
                &mut a,
                &ctx,
                initial,
                true,
                vec![pointer(start, true, initial), Event::PointerMoved(end)],
            );
            let last = end + vec2(60., 45.);
            frame(
                &mut a,
                &ctx,
                late,
                true,
                vec![Event::PointerMoved(last), pointer(last, false, late)],
            );
            (a, ctx, start)
        };
        let (steady, _, _) = run(initial);
        let (mut a, ctx, start) = run(changed);
        assert!(
            !a.status.contains("modifier keys"),
            "{tool:?}: {}",
            a.status
        );
        assert!(a.navigation.is_none());
        assert_eq!(a.editor.scene, steady.editor.scene, "{tool:?} {initial:?}");
        // Compare everything but identity_base, whose epoch is a process-wide
        // counter and so always differs between two separate apps.
        let (x, y) = (
            a.editor.session.live.as_ref().expect("live"),
            steady.editor.session.live.as_ref().expect("live"),
        );
        assert!(
            x.base == y.base && x.params == y.params,
            "{tool:?} {initial:?}"
        );
        assert!(x.applied == y.applied && x.action_frame == y.action_frame);
        assert!(x.identity_preview == y.identity_preview && x.preview_error == y.preview_error);
        assert_eq!(
            a.editor.history.undo.len(),
            steady.editor.history.undo.len()
        );
        // Focus loss ends capture; return with no modifier cannot continue it.
        frame(
            &mut a,
            &ctx,
            initial,
            true,
            vec![pointer(start, true, initial)],
        );
        frame(
            &mut a,
            &ctx,
            initial,
            false,
            vec![Event::WindowFocused(false)],
        );
        assert!(a.navigation.is_none());
        let scene = a.editor.scene.clone();
        let output = frame(
            &mut a,
            &ctx,
            Modifiers::NONE,
            true,
            vec![Event::WindowFocused(true), Event::PointerMoved(start)],
        );
        assert_eq!(text_count(&output, "Unit"), 0);
        assert_eq!(a.editor.scene, scene);
    }
}

#[test]
fn pm_drag_ctrl_mouseup_precedes_the_frames_final_modifier_release() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        for scale in [1., 2.] {
            let mut a = fixture();
            a.camera.view = View::Top;
            a.editor.scene.workplane.fixed = true;
            a.editor.scene.workplane.plane = 0;
            a.activate(tool);
            let ctx = Context::default();
            ctx.set_pixels_per_point(scale);
            for _ in 0..2 {
                frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
            }
            let start = a.viewport_rect.center();
            let middle = start + vec2(60., 25.);
            let end = start + vec2(150., 40.);
            let overall = |a: &Operandi| {
                let p = &a.editor.session.live.as_ref().unwrap().params;
                p.radial.map_or(p.size, |r| r.radius * 2.)
            };
            frame(
                &mut a,
                &ctx,
                Modifiers::CTRL,
                true,
                vec![
                    pointer(start, true, Modifiers::CTRL),
                    Event::PointerMoved(middle),
                ],
            );
            let previous = overall(&a).x;
            // Mouse-up still carries Ctrl. Its subsequent key release may already
            // be in the final frame snapshot; it is not a mid-drag change.
            frame(
                &mut a,
                &ctx,
                Modifiers::NONE,
                true,
                vec![
                    Event::PointerMoved(end),
                    pointer(end, false, Modifiers::CTRL),
                ],
            );
            let actual = overall(&a);
            assert!(
                actual.x > previous * 2.,
                "{tool:?}/{scale}: {previous} → {actual}: {}",
                a.status
            );
            assert_eq!(actual.x, actual.y);
            assert_eq!(actual.y, actual.z);
            assert!(!a.status.contains("PM-Q02"));
            assert!(a.navigation.is_none());
            let scene = a.editor.scene.clone();
            frame(
                &mut a,
                &ctx,
                Modifiers::NONE,
                true,
                vec![Event::PointerMoved(start)],
            );
            assert_eq!(a.editor.scene, scene);
        }
    }
}

#[test]
fn pm_drag_uniform_primitives_geometry_history_and_roundtrip() {
    for (tool, scale) in [Tool::Box, Tool::Sphere, Tool::Cylinder]
        .into_iter()
        .flat_map(|tool| [1., 2.].map(|scale| (tool, scale)))
    {
        for (view, plane, extent) in [
            (View::Top, 0, DVec3::new(-3., 0., 1.)),
            (View::Front, 2, DVec3::new(1., -3., 0.)),
            (View::Right, 1, DVec3::new(0., 1., -3.)),
            (View::Perspective, 0, DVec3::new(3., 0., -1.)),
            (View::Top, 0, DVec3::new(3., 0., 0.)),
            (View::Front, 2, DVec3::new(3., 0., 0.)),
            (View::Right, 1, DVec3::new(0., 3., 0.)),
        ] {
            let mut a = fixture();
            a.camera.view = view;
            a.editor.scene.workplane.fixed = true;
            a.editor.scene.workplane.plane = plane;
            let center = DVec3::new(0.2, -0.3, 0.4);
            a.editor.scene.workplane.center = center;
            a.activate(tool);
            let ctx = Context::default();
            ctx.set_pixels_per_point(scale);
            theme::install(&ctx);
            for _ in 0..2 {
                frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
            }
            let before = a.editor.scene.clone();
            let history = a.editor.history.undo.len();
            let r = a.viewport_rect;
            let start = a.camera.project(center, r).unwrap().0;
            let end = a.camera.project(center + extent, r).unwrap().0;
            let p0 = a.camera.plane_point(start, r, plane, center).unwrap();
            let p1 = a.camera.plane_point(end, r, plane, center).unwrap();
            drag(&mut a, &ctx, start, end, Modifiers::CTRL);
            let live = a.editor.session.live.as_ref().unwrap();
            assert!(live.applied, "{tool:?}/{view:?}: {}", a.status);
            let position = p0;
            let size = 2. * (p1 - p0).abs().max_element();
            assert!(live.params.position.abs_diff_eq(position, 1e-8));
            if tool == Tool::Box {
                assert!(live.params.size.abs_diff_eq(DVec3::splat(size), 1e-8));
            } else {
                assert!(
                    live.params
                        .radial
                        .unwrap()
                        .radius
                        .abs_diff_eq(DVec3::splat(size * 0.5), 1e-8)
                );
            }
            let mesh = &a.editor.scene.active().unwrap().mesh;
            let original = &before.active().unwrap().mesh;
            assert_eq!(
                &mesh.vertices[..original.vertices.len()],
                &original.vertices
            );
            assert_eq!(&mesh.faces[..original.faces.len()], &original.faces[..]);
            assert_eq!(&mesh.uv_maps["seam"][..6], &original.uv_maps["seam"]);
            for v in &mesh.vertices[original.vertices.len()..] {
                let relative = *v - position;
                assert!(relative.abs().max_element() <= size * 0.5 + 1e-8);
                let surface = if tool == Tool::Box {
                    relative.abs().max_element()
                } else if tool == Tool::Sphere {
                    relative.length()
                } else {
                    relative.x.hypot(relative.z)
                };
                assert!((surface - size * 0.5).abs() < 1e-8);
            }
            mesh.validate().unwrap();
            assert_eq!(a.editor.history.undo.len(), history);
            a.run(Command::Drop);
            let result = a.editor.scene.clone();
            assert_eq!(a.editor.history.undo.len(), history + 1);
            a.run(Command::Undo);
            assert_eq!(a.editor.scene, before);
            a.run(Command::Redo);
            assert_eq!(a.editor.scene, result);
            assert!(a.editor.session.live.is_none());
            let path = std::env::temp_dir().join(format!(
                "operandi-ctrl-drag-{}-{tool:?}-{plane}.opscene",
                std::process::id()
            ));
            operandi_core::persistence::save(&path, &result).unwrap();
            assert_eq!(operandi_core::persistence::load(&path).unwrap(), result);
            std::fs::remove_file(path).unwrap();
        }
    }
}

#[test]
fn pm_t02_alt_and_command_are_not_unit_aliases() {
    // Alt/Command alone remain ordinary toolbar activation, never unit aliases.
    for mods in [
        Modifiers::ALT,
        Modifiers {
            mac_cmd: true,
            command: true,
            ..Modifiers::NONE
        },
    ] {
        let mut a = fixture();
        let ctx = Context::default();
        frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
        let p = a.toolbox_rects["Sphere"].center();
        drag(&mut a, &ctx, p, p, mods);
        assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Sphere);
        assert!(!a.editor.session.live.as_ref().unwrap().applied);
    }
}

#[test]
fn pm_drag_click_without_motion_cancel_and_combinations_preserve_base() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut a = fixture();
        a.camera.view = View::Top;
        let base = a.editor.clone();
        a.activate(tool);
        let ctx = Context::default();
        frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
        let start = a.viewport_rect.center();
        let initial = a.editor.clone();
        drag(&mut a, &ctx, start, start, Modifiers::CTRL);
        assert_unchanged(&a, &initial);
        for mods in [
            Modifiers {
                ctrl: true,
                shift: true,
                ..Modifiers::NONE
            },
            Modifiers {
                ctrl: true,
                mac_cmd: true,
                command: true,
                ..Modifiers::NONE
            },
        ] {
            drag(&mut a, &ctx, start, start + vec2(80., 30.), mods);
            assert_unchanged(&a, &initial);
        }
        drag(&mut a, &ctx, start, start + vec2(100., 0.), Modifiers::CTRL);
        assert!(
            a.editor.session.live.as_ref().unwrap().applied,
            "{tool:?}: {}",
            a.status
        );
        a.run(Command::Cancel);
        assert_unchanged(&a, &base);
    }
}

#[test]
fn pm_t13_t16_unit_click_size_ignores_view_grid_plane_and_refusal_keeps_pending_text() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut expected = None;
        for (distance, grid, plane) in [(2., true, 0), (100., false, 2)] {
            let mut a = fixture();
            a.camera.distance = distance;
            a.display.settings.drawing.grid = grid;
            a.editor.scene.workplane.fixed = true;
            a.editor.scene.workplane.plane = plane;
            a.editor.scene.workplane.center = DVec3::new(12., -13., 7.);
            let ctx = Context::default();
            theme::install(&ctx);
            frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
            let p = a.toolbox_rects[tool.label()].center();
            drag(&mut a, &ctx, p, p, Modifiers::SHIFT);
            let mesh = &a.editor.scene.active().unwrap().mesh;
            assert!(a.editor.session.live.is_none());
            // The unit Cylinder is 1.5 m tall (unit-primitive-2026-11-05).
            let half_height = if tool == Tool::Cylinder { 0.75 } else { 0.5 };
            assert!(mesh.vertices.iter().all(
                |v| v.x.abs().max(v.z.abs()) <= 0.5 + 1e-10 && v.y.abs() <= half_height + 1e-10
            ));
            if let Some(expected) = &expected {
                assert_eq!(mesh, expected);
            } else {
                expected = Some(mesh.clone());
            }
        }
        let mut a = fixture();
        a.activate(tool);
        let key = if tool == Tool::Box {
            "size0"
        } else {
            "radius0"
        };
        // The live tool is dropped first (its pending text confirmed into
        // it), then the unit goes into a new mesh.
        a.fields.insert(key.into(), "123 m".into());
        let before = a.editor.clone();
        // The Shift variant's line (layouts stage 3 batch 7).
        a.execute(match tool {
            Tool::Box => "Cube New Unit",
            Tool::Sphere => "Sphere New Unit",
            _ => "Cylinder New Unit",
        });
        assert!(a.editor.session.live.is_none(), "{}", a.status);
        assert_eq!(a.editor.scene.items.len(), before.scene.items.len() + 1);
        assert!(a.editor.history.undo.len() > before.history.undo.len());
    }
}

#[test]
fn ic_t07_08_09_centered_oracles_quadrants_crossing_and_captured_anchor() {
    let center = DVec3::new(3., 4., -2.);
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        for (plane, view, u, v) in [
            (0, View::Top, DVec3::X, DVec3::Z),
            (2, View::Front, DVec3::X, DVec3::Y),
            (1, View::Right, DVec3::Y, DVec3::Z),
            (0, View::Perspective, DVec3::X, DVec3::Z),
        ] {
            for (sx, sy) in [
                (1., 1.),
                (-1., 1.),
                (-1., -1.),
                (1., -1.),
                (1., 0.),
                (0., -1.),
            ] {
                let mut a = fixture();
                a.camera.view = view;
                a.camera.target = center;
                a.camera.distance = 10.;
                a.editor.scene.workplane.fixed = true;
                a.editor.scene.workplane.plane = plane;
                a.editor.scene.workplane.center = center;
                a.activate(tool);
                let ctx = Context::default();
                theme::install(&ctx);
                frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
                let rect = a.viewport_rect;
                let start = a.camera.project(center, rect).unwrap().0;
                let end = a
                    .camera
                    .project(center + u * sx + v * sy * 0.5, rect)
                    .unwrap()
                    .0;
                let anchor = a.camera.plane_point(start, rect, plane, center).unwrap();
                frame(
                    &mut a,
                    &ctx,
                    Modifiers::CTRL,
                    true,
                    vec![
                        pointer(start, true, Modifiers::CTRL),
                        Event::PointerMoved(end),
                    ],
                );
                let check = |a: &Operandi, end: Pos2| {
                    let q = a.camera.plane_point(end, rect, plane, center).unwrap();
                    let half = (q - anchor).abs().max_element();
                    let p = &a.editor.session.live.as_ref().unwrap().params;
                    assert!(
                        p.position.abs_diff_eq(anchor, 1e-9),
                        "{tool:?}/{view:?}: {:?} != {anchor:?}",
                        p.position
                    );
                    let size = p.radial.map_or(p.size, |r| r.radius * 2.);
                    assert!(size.abs_diff_eq(DVec3::splat(2. * half), 1e-8));
                };
                check(&a, end);
                // Crossing shrinks/regrows around the same captured point.
                let across = a
                    .camera
                    .project(center - u * sx * 0.25 - v * sy * 0.125, rect)
                    .unwrap()
                    .0;
                frame(
                    &mut a,
                    &ctx,
                    Modifiers::CTRL,
                    true,
                    vec![Event::PointerMoved(across)],
                );
                check(&a, across);
                // The safe zero-motion policy retains the last valid preview.
                let prior = a.editor.scene.clone();
                frame(
                    &mut a,
                    &ctx,
                    Modifiers::CTRL,
                    true,
                    vec![Event::PointerMoved(start)],
                );
                assert_eq!(a.editor.scene, prior);
                // Camera changes cannot recapture the model-space anchor.
                a.camera.distance = 11.;
                let final_pos = a
                    .camera
                    .project(center + u * sx + v * sy * 0.5, rect)
                    .unwrap()
                    .0;
                frame(
                    &mut a,
                    &ctx,
                    Modifiers::NONE,
                    true,
                    vec![
                        Event::PointerMoved(final_pos),
                        pointer(final_pos, false, Modifiers::CTRL),
                    ],
                );
                check(&a, final_pos);
                assert!(a.navigation.is_none());
                let scene = a.editor.scene.clone();
                frame(
                    &mut a,
                    &ctx,
                    Modifiers::NONE,
                    true,
                    vec![Event::PointerMoved(start)],
                );
                assert_eq!(a.editor.scene, scene);
                a.run(Command::Cancel);
            }
        }
    }
}

// Jason decision 2026-11-01 (BOX-Q05): Shift-drag away from a live
// primitive's handles keeps it and draws the next one with the same settings.
#[test]
fn shift_drag_off_handles_keeps_the_primitive_and_draws_the_next() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut a = fixture();
        a.camera.view = View::Top;
        a.activate(tool);
        let ctx = Context::default();
        theme::install(&ctx);
        frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
        let c = a.viewport_rect.center();
        // An ordinary radial draw is a flat draft; Ctrl-draw makes it solid.
        let first_mods = if tool == Tool::Box {
            Modifiers::NONE
        } else {
            Modifiers::CTRL
        };
        drag(&mut a, &ctx, c, c + vec2(60., 40.), first_mods);
        // Change a setting so "same settings" is visible on the next one.
        let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
        p.segments = [2, 2, 2];
        if let Some(r) = p.radial.as_mut() {
            r.sides = 9;
        }
        a.run(Command::Preview(p));
        let first = a.editor.scene.active().unwrap().mesh.faces.len();
        let history = a.editor.history.undo.len();
        let start = c + vec2(250., 150.);
        drag(
            &mut a,
            &ctx,
            start,
            start + vec2(60., 40.),
            first_mods | Modifiers::SHIFT,
        );
        let live = a
            .editor
            .session
            .live
            .as_ref()
            .expect("next primitive is live");
        assert_eq!(live.tool, tool);
        assert_eq!(
            a.editor.history.undo.len(),
            history + 1,
            "{tool:?}: first one kept"
        );
        let faces = a.editor.scene.active().unwrap().mesh.faces.len();
        assert!(
            faces > first,
            "{tool:?}: next one drawn ({faces} > {first})"
        );
        if tool == Tool::Box {
            assert_eq!(live.params.segments, [2, 2, 2]);
        } else {
            assert_eq!(live.params.radial.unwrap().sides, 9, "{tool:?}");
        }
        // Cancel discards only the second; the first stays.
        a.run(Command::Cancel);
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), first);
    }
}

#[test]
fn shift_drag_before_drawing_starts_nothing_new() {
    let mut a = fixture();
    a.camera.view = View::Top;
    a.activate(Tool::Box);
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
    let before = a.editor.clone();
    let c = a.viewport_rect.center();
    drag(&mut a, &ctx, c, c + vec2(60., 40.), Modifiers::SHIFT);
    assert_eq!(a.editor.history.undo.len(), before.history.undo.len());
    assert_eq!(a.editor.scene, before.scene);
}

#[test]
fn ellipsoid_cone_torus_capsule_cells_make_units() {
    // Jason, 2026-11-05: these four had no unit primitive.
    for label in ["Ellipsoid", "Cone", "Torus", "Capsule"] {
        for mods in [Modifiers::CTRL, Modifiers::SHIFT] {
            let mut a = Operandi::empty();
            a.run(Command::NewMesh);
            let ctx = Context::default();
            theme::install(&ctx);
            frame(&mut a, &ctx, Modifiers::NONE, true, vec![]);
            let p = a.toolbox_rects[label].center();
            let before = a.editor.clone();
            frame(
                &mut a,
                &ctx,
                Modifiers::NONE,
                true,
                vec![pointer(p, true, mods), pointer(p, false, mods)],
            );
            assert!(!a.editor.session.has_active_task(), "{label}: {}", a.status);
            assert_eq!(
                a.editor.history.undo.len(),
                before.history.undo.len() + 1,
                "{label}: {}",
                a.status
            );
            assert_eq!(
                a.editor.scene.items.len(),
                before.scene.items.len() + usize::from(mods.shift),
                "{label}"
            );
            assert!(!a.editor.scene.active().unwrap().mesh.faces.is_empty());
            a.run(Command::Undo);
            assert_eq!(a.editor.scene, before.scene, "{label}");
        }
    }
}
