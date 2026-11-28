//! Drawing and editing primitives (Cube, Sphere, Cylinder) in the viewport and panel.
use super::tests::{box_app, draw, key, pointer};
use super::*;

#[test]
fn t11_019_plane_drag_world_sizes_and_parallel_refusal() {
    for (plane, view, end, center, size) in [
        (
            0,
            View::Top,
            DVec3::new(2., 0., 3.),
            DVec3::new(1., 0., 1.5),
            DVec3::new(2., 0., 3.),
        ),
        (
            2,
            View::Front,
            DVec3::new(2., 3., 0.),
            DVec3::new(1., 1.5, 0.),
            DVec3::new(2., 3., 0.),
        ),
        (
            1,
            View::Right,
            DVec3::new(0., 2., 3.),
            DVec3::new(0., 1., 1.5),
            DVec3::new(0., 2., 3.),
        ),
    ] {
        let ctx = Context::default();
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.workplane.fixed = true;
        a.editor.scene.workplane.plane = plane;
        a.camera.view = view;
        a.camera.distance = 12.;
        a.activate(Tool::Box);
        let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        let r = a.viewport_rect;
        let from = a.camera.project(DVec3::ZERO, r).unwrap().0;
        let to = a.camera.project(end, r).unwrap().0;
        let _ = draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![
                pointer(from, true, Modifiers::NONE),
                Event::PointerMoved(to),
                pointer(to, false, Modifiers::NONE),
            ],
        );
        assert_eq!(a.viewport_rect, r, "viewport frame changed during draw");
        let p = &a.editor.session.live.as_ref().unwrap().params;
        assert!(
            (p.position - center).length() < 1e-5,
            "plane {plane}: {p:?}; rect {r:?}"
        );
        assert!((p.size - size).length() < 1e-5, "{plane}: {p:?}");
        let mesh = &a.editor.scene.active().unwrap().mesh;
        let lo = mesh
            .vertices
            .iter()
            .copied()
            .fold(DVec3::splat(f64::INFINITY), DVec3::min);
        let hi = mesh
            .vertices
            .iter()
            .copied()
            .fold(DVec3::splat(f64::NEG_INFINITY), DVec3::max);
        assert!((hi - lo - size).length() < 1e-5);
    }
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Box);
    a.camera.view = View::Front;
    a.editor.scene.workplane.fixed = true; // explicitly test edge-on fixed XZ
    let ctx = Context::default();
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    let p = a.viewport_rect.center();
    let scene = a.editor.scene.clone();
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(p, true, Modifiers::NONE),
            Event::PointerMoved(p + vec2(50., 50.)),
            pointer(p + vec2(50., 50.), false, Modifiers::NONE),
        ],
    );
    assert_eq!(a.editor.scene, scene);
    assert!(a.status.contains("parallel"));
}
#[test]
fn m15_flat_draw_is_live_before_release_and_click_does_not_repeat() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Box);
    a.camera.view = View::Top;
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    let r = a.viewport_rect;
    let start = a.camera.project(DVec3::ZERO, r).unwrap().0;
    let end = a.camera.project(DVec3::new(2., 0., 3.), r).unwrap().0;
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(start, true, Modifiers::NONE),
            Event::PointerMoved(end),
        ],
    );
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 1);
    assert_eq!(a.editor.session.live.as_ref().unwrap().params.size.y, 0.);
    let live = a.editor.scene.clone();
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![pointer(end, false, Modifiers::NONE)],
    );
    assert_eq!(a.editor.scene, live);
    let outside = r.center() + vec2(200., 150.);
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(outside, true, Modifiers::NONE),
            pointer(outside, false, Modifiers::NONE),
        ],
    );
    assert_eq!(a.editor.scene, live);
}
#[test]
fn m15_cube_owns_right_drag_and_detents_replace_current_preview() {
    let mut a = box_app();
    a.activate(Tool::Box);
    a.run(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..Default::default()
    }));
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    let r = a.viewport_rect;
    let p = r.center() + vec2(200., 100.);
    let event = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![event(p, true), Event::PointerMoved(p + vec2(24., -12.))],
    );
    assert_eq!(
        a.editor.session.live.as_ref().unwrap().params.segments,
        [3, 1, 2]
    );
    assert!(a.editor.selection_start.is_none());
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![Event::PointerMoved(p), event(p, false)],
    );
    assert_eq!(
        a.editor.session.live.as_ref().unwrap().params.segments,
        [1; 3]
    );
    // Existing six faces plus one current six-face preview, never another copy.
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 12);
}
#[test]
fn m15_cube_side_picking_tracks_logical_scale_and_resizing() {
    for (size, scale) in [
        (vec2(1100., 720.), 1.),
        (vec2(1440., 900.), 1.),
        (vec2(1440., 900.), 2.),
        (vec2(2560., 1440.), 2.),
    ] {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.activate(Tool::Box);
        a.run(Command::Preview(Parameters {
            size: DVec3::splat(2.),
            ..Default::default()
        }));
        let ctx = Context::default();
        ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
        for _ in 0..2 {
            draw(&mut a, &ctx, size, vec![]);
        }
        let r = a.viewport_rect;
        let start = a.handles(r).into_iter().find(|h| h.0 == 4).unwrap().2;
        let end = a.camera.project(DVec3::new(1.5, 0., 0.), r).unwrap().0;
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
        let params = &a.editor.session.live.as_ref().unwrap().params;
        assert!(
            (params.position.x - params.size.x * 0.5 + 1.).abs() < 1e-8,
            "{size:?}/{scale}"
        );
        assert!(params.size.x > 2.);
        assert_eq!(params.size.y, 2.);
        assert_eq!(params.size.z, 2.);
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 6);
    }
}
#[test]
fn m16_shared_numeric_radius_bounds_resolution_and_invalid_input() {
    for tool in [Tool::Sphere, Tool::Cylinder] {
        let mut a = box_app();
        let before = a.editor.scene.active().unwrap().mesh.clone();
        a.activate(tool);
        for (key, text) in [
            ("position0", "1 m"),
            ("position1", "2 m"),
            ("position2", "-3 m"),
            ("radius0", "100 cm"),
            ("radius1", "3 m/2"),
            ("radius2", "2 m"),
            ("radial_sides", "4*2"),
            (
                "radial_segments",
                if tool == Tool::Sphere { "4" } else { "3" },
            ),
        ] {
            a.fields.insert(key.into(), text.into());
            assert!(a.confirm_fields(), "{}: {}", tool.label(), a.status);
        }
        let p = &a.editor.session.live.as_ref().unwrap().params;
        assert_eq!(p.position, DVec3::new(1., 2., -3.));
        assert_eq!(p.radial.unwrap().radius, DVec3::new(1., 1.5, 2.));
        assert_eq!(
            &a.editor.scene.active().unwrap().mesh.faces[..6],
            &before.faces[..]
        );
        a.fields.insert("min0".into(), "-2 m".into());
        assert!(a.confirm_fields());
        let p = &a.editor.session.live.as_ref().unwrap().params;
        assert_eq!(p.position.x, 0.);
        assert_eq!(p.radial.unwrap().radius.x, 2.);
        assert_eq!(p.position.x + p.radial.unwrap().radius.x, 2.);
        let scene = a.editor.scene.clone();
        for (key, text) in [
            ("radius1", "-2 m"),
            ("radius0", "nan"),
            ("radial_sides", "8.5"),
            ("radial_segments", "0"),
            ("min0", "100 m"),
        ] {
            a.clear_fields();
            a.fields.insert(key.into(), text.into());
            assert!(!a.confirm_fields(), "{key}");
            assert_eq!(a.editor.scene, scene);
        }
        a.clear_fields();
        let ctx = Context::default();
        theme::install(&ctx);
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let point = a.field_rects["radius0"].center();
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![
                pointer(point, true, Modifiers::NONE),
                pointer(point, false, Modifiers::NONE),
            ],
        );
        // Q in a number field drops the tool (drop_keys.rs, Jason
        // 2026-11-21); nothing was typed, so the scene is as it was.
        draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Q)]);
        assert!(
            a.editor.session.live.is_none(),
            "Q in a number field drops the tool"
        );
        let _ = &scene;
    }
}
#[test]
fn m16_draft_plane_capture_numeric_solid_and_off_handle_refusal() {
    for tool in [Tool::Sphere, Tool::Cylinder] {
        for (plane, view, end, normal) in [
            (0, View::Top, DVec3::new(2., 0., 3.), 1),
            (1, View::Right, DVec3::new(0., 2., 3.), 0),
            (2, View::Front, DVec3::new(2., 3., 0.), 2),
        ] {
            let mut a = box_app();
            a.editor.scene.workplane.fixed = true;
            a.editor.scene.workplane.plane = plane;
            a.camera.view = view;
            a.camera.distance = 12.;
            let base = a.editor.scene.clone();
            a.activate(tool);
            let ctx = Context::default();
            theme::install(&ctx);
            for _ in 0..2 {
                draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
            }
            let r = a.viewport_rect;
            let start = a.camera.project(DVec3::ZERO, r).unwrap().0;
            let end = a.camera.project(end, r).unwrap().0;
            draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![
                    pointer(start, true, Modifiers::NONE),
                    Event::PointerMoved(end),
                ],
            );
            let draft = a.editor.session.live.as_ref().unwrap().params.clone();
            assert_eq!(draft.radial.unwrap().radius[normal], 0.);
            // drag-creation-2026-11-06: the flat base is real geometry.
            assert_ne!(a.editor.scene, base);
            draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![pointer(end, false, Modifiers::NONE)],
            );
            assert_eq!(
                a.editor.session.live.as_ref().unwrap().params,
                draft,
                "stable release"
            );
            a.fields.insert(format!("radius{normal}"), "1 m".into());
            assert!(a.confirm_fields());
            assert!(a.editor.session.live.as_ref().unwrap().applied);
            let solid = a.editor.scene.clone();
            let remote = r.center() + vec2(170., 120.);
            draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![
                    pointer(remote, true, Modifiers::NONE),
                    Event::PointerMoved(remote + vec2(40., 10.)),
                    pointer(remote + vec2(40., 10.), false, Modifiers::NONE),
                ],
            );
            // A drag away from the handles now drags the nearest side, as
            // the Cube does (Jason, 2026-11-04; DRAG-Q01).
            assert_ne!(a.editor.scene, solid, "{}", a.status);
            let dragged = a.editor.scene.clone();
            assert!(!a.handles(r).is_empty(), "approved explicit side handles");
            a.run(Command::Drop);
            assert_eq!(a.editor.history.undo.last().unwrap().label, tool.label());
            a.run(Command::Undo);
            assert_eq!(a.editor.scene, base);
            a.run(Command::Redo);
            assert_eq!(a.editor.scene, dragged);
        }
    }
}
#[test]
fn m16_mid_panel_enter_refreshes_dependent_bounds_in_same_frame() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.activate(tool);
        let ctx = Context::default();
        theme::install(&ctx);
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let field = if tool == Tool::Box {
            "size1"
        } else {
            "radius1"
        };
        let point = a.field_rects[field].center();
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![
                pointer(point, true, Modifiers::NONE),
                pointer(point, false, Modifiers::NONE),
            ],
        );
        a.fields.insert(field.into(), "3 m/2".into());
        draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Enter)]);
        let expected = if tool == Tool::Box { 0.75 } else { 1.5 };
        assert_eq!(numeric::parse(&a.fields["min1"]).unwrap(), -expected);
        assert_eq!(numeric::parse(&a.fields["max1"]).unwrap(), expected);
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        assert_eq!(numeric::parse(&a.fields["min1"]).unwrap(), -expected);
    }
}
#[test]
fn par_t01_t02_pointer_only_depth_resize_history_persistence_and_prior_mesh() {
    for tool in [Tool::Sphere, Tool::Cylinder] {
        for plane in [0, 2] {
            let mut a = box_app();
            a.camera.view = View::Perspective;
            a.camera.distance = 12.;
            a.editor.scene.workplane.fixed = true;
            a.editor.scene.workplane.plane = plane;
            let before = a.editor.scene.clone();
            let history = a.editor.history.undo.len();
            a.activate(tool);
            let ctx = Context::default();
            theme::install(&ctx);
            for _ in 0..2 {
                draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
            }
            let r = a.viewport_rect;
            let start = a.camera.project(DVec3::new(-1., 0., 0.), r).unwrap().0;
            let endpoint = if plane == 0 {
                DVec3::new(2., 0., 3.)
            } else {
                DVec3::new(2., 3., 0.)
            };
            let end = a.camera.project(endpoint, r).unwrap().0;
            draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![
                    pointer(start, true, Modifiers::NONE),
                    Event::PointerMoved(end),
                    pointer(end, false, Modifiers::NONE),
                ],
            );
            let normal = if plane == 0 { 1 } else { 2 };
            let base = a.editor.session.live.as_ref().unwrap().params.clone();
            assert_eq!(base.radial.unwrap().radius[normal], 0.);
            let handle = a
                .handles(r)
                .into_iter()
                .find(|h| h.0 == 4 + normal * 2)
                .unwrap();
            assert_eq!(a.handle_hit(handle.2, r), Some(handle.0));
            let unit = [DVec3::X, DVec3::Y, DVec3::Z][normal];
            let c = a.camera.project(base.position, r).unwrap().0;
            let u = a.camera.project(base.position + unit, r).unwrap().0;
            let end = handle.2 + (u - c) * 1.2;
            draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![
                    pointer(handle.2, true, Modifiers::NONE),
                    Event::PointerMoved(end),
                    pointer(end, false, Modifiers::NONE),
                ],
            );
            let solid = a.editor.session.live.as_ref().unwrap().params.clone();
            assert!(
                solid.radial.unwrap().radius.min_element() > 0.,
                "{} {:?}: {}",
                tool.label(),
                plane,
                a.status
            );
            if tool == Tool::Sphere {
                assert_eq!(solid.position, base.position);
            } else {
                assert!(
                    (solid.position[normal]
                        - solid.radial.unwrap().radius[normal]
                        - (base.position[normal]))
                        .abs()
                        < 1e-6
                );
            }
            assert_eq!(
                &a.editor.scene.active().unwrap().mesh.vertices[..8],
                &before.active().unwrap().mesh.vertices
            );
            let handle = a.handles(r).into_iter().find(|h| h.0 == 4).unwrap();
            let end = handle.2 + vec2(20., 0.);
            draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![
                    pointer(handle.2, true, Modifiers::NONE),
                    Event::PointerMoved(end),
                    pointer(end, false, Modifiers::NONE),
                ],
            );
            let committed = a.editor.scene.clone();
            a.run(Command::Drop);
            assert_eq!(a.editor.history.undo.len(), history + 1);
            a.run(Command::Undo);
            assert_eq!(a.editor.scene, before);
            a.run(Command::Redo);
            assert_eq!(a.editor.scene, committed);
            let path =
                std::env::temp_dir().join(format!("operandi-par-mouse-{tool:?}-{plane}.opscene"));
            assert!(a.save(path.clone()));
            assert_eq!(persistence::load(&path).unwrap(), committed);
            let _ = std::fs::remove_file(path);
        }
    }
}
// VP2: user-reported automatic-plane, primitive label and collapsed depth-handle
// corrections. Synthetic events drive the actual viewport arbitration/preview.
#[test]
fn vp2_automatic_new_scene_draws_on_the_view_plane_and_retains_explicit_fixed_state() {
    for (view, yaw, pitch, plane) in [
        (View::Front, 0., 0., 2),
        (View::Back, 0., 0., 2),
        (View::Top, 0., 0., 0),
        (View::Bottom, 0., 0., 0),
        (View::Right, 0., 0., 1),
        (View::Left, 0., 0., 1),
        (View::Perspective, 0., 0.9, 0),
        (View::Perspective, 1.2, 0.2, 1),
        (View::Perspective, 0., 0.2, 2),
    ] {
        let mut a = Operandi::empty();
        assert!(!a.editor.scene.workplane.fixed);
        assert!(
            a.display.settings.drawing.grid && !a.viewport_preferences.workplane_display,
            "ground grid on and work plane display off by default"
        );
        a.camera.view = view;
        a.camera.yaw = yaw;
        a.camera.pitch = pitch;
        a.run(Command::NewMesh);
        a.activate(Tool::Box);
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        assert_eq!(a.actual_plane(), plane);
        let normal = match plane {
            1 => 0,
            2 => 2,
            _ => 1,
        };
        let mut corner = DVec3::new(2., 2., 2.);
        corner[normal] = 0.;
        let start = a.camera.project(DVec3::ZERO, a.viewport_rect).unwrap().0;
        let end = a.camera.project(corner, a.viewport_rect).unwrap().0;
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![
                pointer(start, true, Modifiers::NONE),
                Event::PointerMoved(end),
                pointer(end, false, Modifiers::NONE),
            ],
        );
        let params = &a.editor.session.live.as_ref().unwrap().params;
        assert_eq!(params.size[normal], 0.);
        for axis in 0..3 {
            if axis != normal {
                assert!((params.size[axis] - 2.).abs() < 1e-5);
            }
        }
        assert!(
            a.editor
                .scene
                .active()
                .unwrap()
                .mesh
                .vertices
                .iter()
                .all(|p| p[normal].abs() < 1e-6)
        );
        a.run(Command::Cancel);
        a.run(Command::Workplane(Workplane {
            fixed: true,
            plane,
            center: DVec3::ZERO,
        }));
        a.camera.view = View::Perspective;
        a.camera.yaw += 1.5;
        a.camera.pitch = -0.2;
        assert_eq!(a.actual_plane(), plane, "explicit Fixed must remain fixed");
        let reopened = persistence::decode(&persistence::encode(&a.editor.scene).unwrap()).unwrap();
        assert!(reopened.workplane.fixed);
        assert_eq!(reopened.workplane.plane, plane);
    }
}
#[test]
fn vp2_flat_cube_center_square_grows_depth_instead_of_crossing_the_minimum_bound() {
    for normal in 0..3 {
        for (mode, ctrl) in [
            (BoxHandleMode::Resize, false),
            (BoxHandleMode::FromCenter, false),
            (BoxHandleMode::MoveAll, false),
            (BoxHandleMode::Resize, true),
        ] {
            let mut a = box_app();
            let before = a.editor.scene.clone();
            let history = a.editor.history.undo.len();
            a.camera.view = View::Perspective;
            a.camera.target = DVec3::new(3., 2., 1.);
            a.camera.distance = 8.;
            a.activate(Tool::Box);
            let mut flat = Parameters {
                position: a.camera.target,
                size: DVec3::splat(2.),
                box_handle_mode: mode,
                ..Default::default()
            };
            flat.size[normal] = 0.;
            a.run(Command::Preview(flat.clone()));
            let ctx = Context::default();
            theme::install(&ctx);
            for _ in 0..2 {
                draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
            }
            let r = a.viewport_rect;
            let handles = a.handles(r);
            let depth: Vec<_> = handles
                .iter()
                .filter(|h| h.0 >= 3 && (h.0 - 3) / 2 == normal)
                .collect();
            assert_eq!(depth.len(), 1, "no coincident +/- depth hits");
            let h = depth[0];
            assert_eq!(h.0, 4 + normal * 2);
            assert_eq!(a.handle_hit(h.2, r), Some(h.0));
            let mut unit = flat.position;
            unit[normal] += 1.;
            let projected_unit = a.camera.project(unit, r).unwrap().0
                - a.camera.project(flat.position, r).unwrap().0;
            let end = h.2 + projected_unit * 0.75;
            let mods = if ctrl {
                Modifiers::CTRL
            } else {
                Modifiers::NONE
            };
            draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![
                    pointer(h.2, true, mods),
                    Event::PointerMoved(h.2 + projected_unit * 0.25),
                    Event::PointerMoved(end),
                    pointer(end, false, mods),
                ],
            );
            let p = &a.editor.session.live.as_ref().unwrap().params;
            let moving = ctrl || mode == BoxHandleMode::MoveAll;
            let expected_size = if moving {
                0.
            } else if mode == BoxHandleMode::FromCenter {
                1.5
            } else {
                0.75
            };
            assert!(
                (p.size[normal] - expected_size).abs() < 1e-5,
                "{normal}/{mode:?}/{ctrl}: {}",
                a.status
            );
            let expected_center = flat.position[normal]
                + if moving {
                    0.75
                } else if mode == BoxHandleMode::Resize {
                    0.375
                } else {
                    0.
                };
            assert!(
                (p.position[normal] - expected_center).abs() < 1e-5,
                "{normal}/{mode:?}/{ctrl}: {p:?}; expected center {expected_center}; {}",
                a.status
            );
            for axis in 0..3 {
                if axis != normal {
                    assert_eq!(p.size[axis], flat.size[axis]);
                    assert_eq!(p.position[axis], flat.position[axis]);
                }
            }
            let item = a.editor.scene.active().unwrap();
            assert_eq!(
                item.mesh.faces.len(),
                if moving { 7 } else { 12 },
                "preview replaces the flat stage once"
            );
            assert_eq!(
                &item.mesh.vertices[..8],
                &before.active().unwrap().mesh.vertices
            );
            let changed = a.editor.scene.clone();
            a.run(Command::Drop);
            assert_eq!(a.editor.history.undo.len(), history + 1);
            a.run(Command::Undo);
            assert_eq!(a.editor.scene, before);
            a.run(Command::Redo);
            assert_eq!(a.editor.scene, changed);
            assert_eq!(
                persistence::decode(&persistence::encode(&changed).unwrap()).unwrap(),
                changed
            );
            a.activate(Tool::Box);
            a.run(Command::Preview(flat));
            a.run(Command::Cancel);
            assert_eq!(a.editor.scene, changed);
        }
    }
}
#[test]
fn vp2_primitive_handle_labels_have_tooltips_without_viewport_ink() {
    // Text drawn inside the viewport (above its bottom strip); shortcut
    // badges in the toolbox, such as Transform's Y, don't count.
    fn ink(shape: &egui::Shape, text: &str, viewport: egui::Rect) -> usize {
        match shape {
            egui::Shape::Text(s) => usize::from(
                s.galley.text() == text
                    && viewport.contains(s.pos)
                    && s.pos.y < viewport.bottom() - 100.,
            ),
            egui::Shape::Vec(shapes) => shapes.iter().map(|s| ink(s, text, viewport)).sum(),
            _ => 0,
        }
    }
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut a = box_app();
        a.camera.view = View::Perspective;
        a.activate(tool);
        let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
        if let Some(radial) = p.radial.as_mut() {
            radial.radius = DVec3::ONE;
        } else {
            p.size = DVec3::splat(2.);
        }
        a.run(Command::Preview(p));
        let ctx = Context::default();
        theme::install(&ctx);
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let output = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        let handles = a.handles(a.viewport_rect);
        assert!(!handles.is_empty());
        for h in handles {
            assert!(!h.3.is_empty(), "accessible role retained");
            assert_eq!(
                output
                    .shapes
                    .iter()
                    .map(|s| ink(&s.shape, &h.3, a.viewport_rect))
                    .sum::<usize>(),
                0,
                "{} label {:?}",
                tool.label(),
                h.3
            );
        }
    }
}

/// N21 (provisional, Q-DC-04): a drawn Sphere or Cylinder shows three move
/// arrows at its centre; dragging the X arrow moves the shape along X and
/// keeps its radii.
#[test]
fn n21_drawn_round_primitives_have_move_arrows_at_their_centre() {
    for tool in [Tool::Sphere, Tool::Cylinder] {
        let mut a = box_app();
        a.activate(tool);
        for (key, text) in [("radius0", "1 m"), ("radius1", "1 m"), ("radius2", "1 m")] {
            a.fields.insert(key.into(), text.into());
            assert!(a.confirm_fields(), "{}: {}", tool.label(), a.status);
        }
        let ctx = Context::default();
        theme::install(&ctx);
        let size = vec2(1440., 900.);
        for _ in 0..2 {
            draw(&mut a, &ctx, size, vec![]);
        }
        let r = a.viewport_rect;
        let handles = a.handles(r);
        // Seen from the top: X and Z (Y points at the camera).
        for axis in [0, 2] {
            assert!(
                handles.iter().any(|h| h.0 == axis),
                "{}: arrow {axis} in {handles:?}",
                tool.label()
            );
        }
        let before = a.editor.session.live.as_ref().unwrap().params.clone();
        let tip = handles.iter().find(|h| h.0 == 0).unwrap().2;
        let to = tip + vec2(60., 0.);
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(tip)]);
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(tip, true, Modifiers::NONE)],
        );
        for k in 1..=6 {
            let at = tip + (to - tip) * (k as f32 / 6.);
            draw(&mut a, &ctx, size, vec![Event::PointerMoved(at)]);
        }
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(to, false, Modifiers::NONE)],
        );
        let after = &a.editor.session.live.as_ref().unwrap().params;
        assert!(
            after.position.x > before.position.x + 0.01,
            "{}: {:?}",
            tool.label(),
            after.position
        );
        assert!((after.position.z - before.position.z).abs() < 1e-9);
        assert_eq!(after.radial.unwrap().radius, before.radial.unwrap().radius);
    }
}
