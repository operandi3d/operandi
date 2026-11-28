//! VAC native input adapter tests use synthetic egui events, not OS gestures.
use super::*;
use operandi_core::Mesh;
fn fixture() -> Operandi {
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.editor.scene.active_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::new(3., 1., 0.), DVec3::splat(2.), 0)
            .unwrap()
            .into();
    app.camera.target = DVec3::new(3., 1., 0.);
    app.camera.distance = 6.;
    app.camera.view = View::Top;
    app
}
#[test]
fn vac_t08_t13_t24_face_pick_transform_handle_origins_and_hits() {
    let rect = Rect::from_min_size(pos2(244., 76.), vec2(1000., 800.));
    for tool in [Tool::Move, Tool::Rotate, Tool::Scale, Tool::Transform] {
        for center in [CenterMode::None, CenterMode::Selection, CenterMode::Origin] {
            let mut app = fixture();
            let (pick, _) = app.camera.project(DVec3::new(3., 2., 0.), rect).unwrap();
            app.select_at(pick, rect, Modifiers::NONE);
            assert_eq!(app.editor.scene.selection.faces, vec![5]);
            app.run(Command::ActionContext(ActionContext {
                center,
                axis: AxisOrientation::Workplane(1),
            }));
            app.activate(tool);
            let expected = app
                .camera
                .project(
                    if center == CenterMode::Origin {
                        DVec3::ZERO
                    } else {
                        DVec3::new(3., 2., 0.)
                    },
                    rect,
                )
                .unwrap()
                .0;
            let handles = app.handles(rect);
            assert!(!handles.is_empty());
            for h in handles.iter().filter(|h| h.0 < 3) {
                assert!(h.1.distance(expected) < 0.001);
            }
            let p = handles
                .iter()
                .find(|h| app.handle_hit(h.2, rect) == Some(h.0))
                .expect("at least one isolated handle");
            assert_eq!(app.handle_hit(p.2, rect), Some(p.0));
        }
    }
}
#[test]
fn vac_t19_t20_bevel_handle_drags_edit_correct_parameters_and_cap() {
    for axis in 0..2 {
        let mut app = fixture();
        app.editor.scene.selection.faces = vec![5];
        app.camera.view = View::Perspective;
        app.activate(Tool::Bevel);
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(1000., 800.));
        let h = app.handles(rect).into_iter().find(|h| h.0 == axis).unwrap();
        assert_eq!(app.handle_hit(h.2, rect), Some(axis));
        assert_eq!(h.3, if axis == 0 { "Shift" } else { "Inset" });
        let base = app.editor.session.live.as_ref().unwrap().params.clone();
        app.update_handle(
            axis,
            base,
            h.2,
            // Shift: out along its arrow. Inset: in toward the center.
            h.2 + (h.2 - h.1).normalized() * if axis == 0 { 8. } else { -8. },
            Tool::Bevel,
            rect,
        );
        let p = &app.editor.session.live.as_ref().unwrap().params;
        assert!(if axis == 0 {
            p.shift > 0. && p.inset == 0.
        } else {
            p.inset > 0. && p.shift == 0.
        });
        let mesh = &app.editor.scene.active().unwrap().mesh;
        let cap = app.editor.scene.selection.faces[0] as usize;
        assert!((mesh.center(cap).y - (2. + p.shift)).abs() < 1e-6);
    }
}

#[test]
fn bevel_handle_projects_from_the_picked_side_only_with_symmetry_and_no_action_center() {
    let rect = Rect::from_min_size(Pos2::ZERO, vec2(1000., 800.));
    for (symmetry, center, expected) in [
        (Symmetry::x(), CenterMode::None, DVec3::X),
        // ACS-01 (rulebook 2026-11-19): Selection takes the + half's centre
        // under Symmetry too (it used to stay between the halves).
        (Symmetry::x(), CenterMode::Selection, DVec3::X),
        (Symmetry::x(), CenterMode::Origin, DVec3::ZERO),
        (Symmetry::OFF, CenterMode::None, DVec3::ZERO),
    ] {
        let mut app = fixture();
        app.camera.view = View::Perspective;
        app.camera.target = DVec3::ZERO;
        let mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0).unwrap();
        let face_at = |at| {
            (0..mesh.faces.len())
                .find(|&face| mesh.center(face) == at)
                .unwrap() as u32
        };
        let picked = face_at(DVec3::X);
        let mirror = face_at(-DVec3::X);
        app.editor.scene.active_mut().unwrap().mesh = mesh.into();
        app.run(Command::Symmetry(symmetry));
        app.run(Command::ActionContext(ActionContext {
            center,
            axis: AxisOrientation::World,
        }));
        let item = app.editor.scene.active().unwrap().id;
        app.run(Command::Select {
            item,
            hit: Component::Polygon(picked),
            replace: true,
            remove: false,
        });
        if symmetry == Symmetry::OFF {
            app.run(Command::Select {
                item,
                hit: Component::Polygon(mirror),
                replace: false,
                remove: false,
            });
        }
        assert_eq!(app.editor.scene.selection.faces, [picked, mirror]);
        app.activate(Tool::Bevel);
        let expected = app.camera.project(expected, rect).unwrap().0;
        let handles = app.handles(rect);
        assert!(!handles.is_empty());
        for handle in handles {
            assert!(
                handle.1.distance(expected) < 0.001,
                "{symmetry:?} {center:?}"
            );
        }
    }
}
#[test]
fn vac_t02_t04_t07_planes_display_and_context_are_independent() {
    let mut app = fixture();
    app.editor.scene.workplane.fixed = false;
    for (view, plane) in [
        (View::Front, 2),
        (View::Back, 2),
        (View::Top, 0),
        (View::Bottom, 0),
        (View::Left, 1),
        (View::Right, 1),
    ] {
        app.camera.view = view;
        assert_eq!(app.actual_plane(), plane);
    }
    app.editor.scene.workplane.fixed = true;
    for plane in 0..3 {
        app.editor.scene.workplane.plane = plane;
        app.camera.view = View::Perspective;
        for pitch in [-1.49, -0.6, 0., 0.6, 1.49] {
            for yaw in 0..36 {
                app.camera.yaw = yaw as f64 * std::f64::consts::TAU / 36.;
                app.camera.pitch = pitch;
                assert_eq!(app.actual_plane(), plane);
            }
        }
    }
    let before = app.editor.clone();
    app.display.settings.drawing.grid = false;
    app.viewport_preferences.workplane_display = false;
    app.axes = true;
    app.plane_widget = false;
    assert!(app.editor == before);
    app.activate(Tool::Move);
    app.fields.insert("delta0".into(), "abc".into());
    let before = app.editor.clone();
    app.run(Command::ActionContext(ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    }));
    assert!(app.editor == before);
    assert_eq!(app.fields["delta0"], "abc");
    assert!(app.field_errors.contains_key("delta0"));
}
#[test]
fn vac_t05_grid_uniform_rays_and_distant_coverage() {
    let rect = Rect::from_min_size(Pos2::ZERO, vec2(1000., 800.));
    for target in [DVec3::ZERO, DVec3::new(10003., 1., 10000.)] {
        for view in [View::Perspective, View::Front, View::Top, View::Right] {
            let camera = Camera {
                target,
                view,
                ..Default::default()
            };
            for plane in 0..3 {
                let grid = grid::PlaneGrid::new(
                    &camera,
                    rect,
                    grid::Display {
                        plane,
                        center: DVec3::ZERO,
                        axes: false,
                        workplane: true,
                        visible: true,
                        opacity: 1.,
                        zero_axes: false,
                        fade: true,
                    },
                    2.,
                );
                assert_eq!(grid.data.len(), 44);
                assert!(grid.data.iter().all(|x| x.is_finite()));
                assert_eq!([grid.data[19], grid.data[23]], [2000., 1600.]);
                assert!(grid::spacing(&camera, rect, plane, DVec3::ZERO) > 0.);
                let right = DVec3::new(
                    grid.data[4] as f64,
                    grid.data[5] as f64,
                    grid.data[6] as f64,
                );
                assert!((right - camera.basis().1).length() < 1e-6);
            }
        }
    }
}

/// The grid shader's ground hit for a view-local pixel, written out on the
/// CPU from the uniform `PlaneGrid` uploads (grid.wgsl `fs`), in world space.
fn grid_shader_hit(camera: &Camera, data: &[f32], local: Pos2, scale: f32) -> Option<DVec3> {
    let get = |i: usize| DVec3::new(data[i] as f64, data[i + 1] as f64, data[i + 2] as f64);
    let origin_shift = camera.basis().0 - get(0);
    let (px, py) = ((local.x * scale) as f64, (local.y * scale) as f64);
    let lens = data[40] as f64;
    let x = (px / data[19] as f64 - 0.5) * data[7] as f64 / lens;
    let y = (0.5 - py / data[23] as f64) / lens;
    let normal = get(16).cross(get(20));
    let mut ray = get(12);
    let mut origin = get(0);
    if data[15] > 0.5 {
        ray = (ray + get(4) * x + get(8) * y).normalize();
    } else {
        origin += (get(4) * x + get(8) * y) * data[11] as f64;
    }
    let denom = ray.dot(normal);
    let t = -origin.dot(normal) / denom;
    (denom.abs() > 1e-6 && t > 0.).then(|| origin + ray * t + origin_shift)
}

#[test]
fn grid_follows_the_view_lens_after_maximise() {
    // A maximised view keeps its world scale by changing its lens (v1.1).
    // The ground grid must use that lens too, or an object standing on the
    // ground appears to float above the grid.
    let rect = Rect::from_min_size(pos2(40., 30.), vec2(1600., 900.));
    let scale = 2.;
    for lens in [crate::camera::DEFAULT_LENS, 0.9 * 450. / 900., 1.7] {
        for view in [View::Perspective, View::Front, View::Top] {
            let camera = Camera {
                view,
                lens,
                ..Default::default()
            };
            let plane = if view == View::Front { 2 } else { 0 };
            let grid = grid::PlaneGrid::new(
                &camera,
                rect,
                grid::Display {
                    plane,
                    center: DVec3::ZERO,
                    axes: false,
                    workplane: false,
                    visible: true,
                    opacity: 1.,
                    zero_axes: false,
                    fade: true,
                },
                scale,
            );
            let mut checked = 0;
            for local in [pos2(200., 700.), pos2(1400., 850.), pos2(800., 600.)] {
                let Some(expected) =
                    camera.plane_point(rect.min + local.to_vec2(), rect, plane, DVec3::ZERO)
                else {
                    continue;
                };
                let hit = grid_shader_hit(&camera, &grid.data, local, scale)
                    .expect("the grid hits the plane where the camera ray does");
                assert!(
                    (hit - expected).length() < 1e-3 * (1. + expected.length()),
                    "lens {lens} {view:?} at {local:?}: grid {hit:?} vs mesh {expected:?}"
                );
                checked += 1;
            }
            assert!(checked > 0);
        }
    }
}

#[test]
fn vac_t24_real_layout_projection_and_hit_regions_at_simulated_scales() {
    for (width, height, scale) in [
        (1920., 1080., 1.),
        (2560., 1440., 1.5),
        (3840., 2160., 2.),
        (5120., 2880., 2.),
    ] {
        let ctx = Context::default();
        theme::install(&ctx);
        let mut app = fixture();
        app.editor.scene.selection.faces = vec![5];
        app.camera.view = View::Perspective;
        app.activate(Tool::Transform);
        for _ in 0..3 {
            let mut input = RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    vec2(width / scale, height / scale),
                )),
                focused: true,
                ..Default::default()
            };
            input
                .viewports
                .entry(ViewportId::ROOT)
                .or_default()
                .native_pixels_per_point = Some(scale);
            let _ = ctx.run(input, |ctx| {
                app.begin_input(ctx);
                app.compose(ctx);
            });
        }
        let r = app.viewport_rect;
        let expected = app.camera.project(DVec3::new(3., 2., 0.), r).unwrap().0;
        let handles = app.handles(r);
        for h in handles.iter().filter(|h| h.0 < 3) {
            assert!(h.1.distance(expected) < 0.001);
        }
        let hittable = handles
            .iter()
            .filter(|h| app.handle_hit(h.2, r) == Some(h.0))
            .count();
        assert!(hittable >= 3, "{width}x{height} @ {scale}: {hittable}");
    }
}
