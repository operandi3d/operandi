//! Viewport picking, paint and lasso selection, Frame Selection.
use super::tests::{box_app, draw, face, key, pointer};
use super::*;
use operandi_core::Mesh;

#[test]
fn t11_010_visible_surface_policy_including_wire_and_backface() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let vertices = vec![
        DVec3::new(-1., -1., 1.),
        DVec3::new(1., -1., 1.),
        DVec3::new(1., 1., 1.),
        DVec3::new(-1., 1., 1.),
        DVec3::new(-0.5, -0.5, 0.),
        DVec3::new(1.5, -0.5, 0.),
        DVec3::new(1.5, 1.5, 0.),
        DVec3::new(-0.5, 1.5, 0.),
    ];
    a.editor.scene.active_mut().unwrap().mesh = Mesh {
        attributes: Default::default(),
        uv_maps: Default::default(),
        vertices,
        faces: vec![face(vec![0, 1, 2, 3]), face(vec![4, 5, 6, 7])].into(),
    }
    .into();
    a.camera.view = View::Front;
    a.camera.distance = 5.;
    let r = Rect::from_min_size(Pos2::ZERO, vec2(700., 700.));
    for wire in [false, true] {
        a.display.settings.active.overlay = if wire {
            crate::view_display::Overlay::Uniform
        } else {
            crate::view_display::Overlay::None
        };
        a.editor.scene.selection.mode = Mode::Polygon;
        for (world, expected) in [(DVec3::ZERO, 0), (DVec3::new(1.25, 1.25, 0.), 1)] {
            let p = a.camera.project(world, r).unwrap().0;
            assert!(matches!(a.pick_component(p,r),Some((_,Component::Polygon(f))) if f==expected));
        }
        a.editor.scene.selection.mode = Mode::Vertex;
        let p = a.camera.project(DVec3::new(-0.5, -0.5, 0.), r).unwrap().0;
        assert!(a.pick_component(p, r).is_none());
        let p = a.camera.project(DVec3::new(1.5, 1.5, 0.), r).unwrap().0;
        assert!(matches!(
            a.pick_component(p, r),
            Some((_, Component::Vertex(6)))
        ));
    }
    a.camera.view = View::Orthographic;
    a.camera.yaw = std::f64::consts::PI;
    a.camera.pitch = 0.;
    a.editor.scene.selection.mode = Mode::Polygon;
    assert!(a.pick_component(r.center(), r).is_none());
}
#[test]
fn t11_005_frame_selection_uses_explicit_vertex_edge_and_empty_targets() {
    let mut a = box_app();
    a.editor.scene.selection.mode = Mode::Vertex;
    a.editor.scene.selection.vertices = vec![0];
    a.frame(true);
    assert_eq!(
        a.camera.target,
        a.editor.scene.active().unwrap().mesh.vertices[0]
    );
    assert!(a.camera.distance.is_finite());
    a.editor.scene.selection.vertices.clear();
    let target = a.camera.target;
    let distance = a.camera.distance;
    a.frame(true);
    assert_eq!(a.camera.target, target);
    assert_eq!(a.camera.distance, distance);
    assert!(a.status.contains("Nothing visible"));
    a.editor.scene.selection.mode = Mode::Edge;
    a.editor.scene.selection.edges = vec![(0, 1)];
    a.frame(true);
    assert_eq!(a.camera.target, DVec3::new(0., -1., -1.));
}
#[test]
fn m15_paint_is_continuous_idempotent_and_one_history_checkpoint() {
    let mut a = box_app();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces.clear();
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    let r = a.viewport_rect;
    let p = a.camera.project(DVec3::new(0., 1., 0.), r).unwrap().0;
    let original = a.editor.scene.clone();
    let history = a.editor.history.undo.len();
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(p, true, Modifiers::NONE),
            Event::PointerMoved(p + vec2(20., 0.)),
        ],
    );
    assert_eq!(a.editor.scene.selection.faces.len(), 1);
    assert_eq!(a.editor.history.undo.len(), history);
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![Event::PointerMoved(p), pointer(p, false, Modifiers::NONE)],
    );
    assert_eq!(a.editor.history.undo.len(), history + 1);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, original);
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(p, true, Modifiers::NONE),
            Event::WindowFocused(false),
        ],
    );
    assert_eq!(a.editor.scene, original);
}
#[test]
fn ecs_empty_click_clears_only_active_components_with_exact_history() {
    for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon] {
        for scale in [1., 2.] {
            let mut a = box_app();
            a.editor.scene.selection.mode = mode;
            a.editor.scene.selection.vertices = vec![1, 0];
            a.editor.scene.selection.edges = vec![(0, 1), (0, 3)];
            a.editor.scene.selection.faces = vec![5, 1];
            let original = a.editor.scene.clone();
            let mut cleared = original.clone();
            cleared.clear_components();
            let history = a.editor.history.undo.len();
            let ctx = Context::default();
            ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
            let size = vec2(1440., 900.);
            draw(&mut a, &ctx, size, vec![]);
            draw(&mut a, &ctx, size, vec![]);
            let p = a.viewport_rect.right_top() + vec2(-40., 100.);
            assert!(a.pick_component(p, a.viewport_rect).is_none());
            for mods in [Modifiers::SHIFT, Modifiers::CTRL] {
                draw(
                    &mut a,
                    &ctx,
                    size,
                    vec![pointer(p, true, mods), pointer(p, false, mods)],
                );
                assert_eq!(a.editor.scene, original);
                assert_eq!(a.editor.history.undo.len(), history);
            }
            // A clear belongs to the existing press/release transaction.
            draw(&mut a, &ctx, size, vec![pointer(p, true, Modifiers::NONE)]);
            assert_eq!(a.editor.scene, cleared);
            assert_eq!(a.editor.history.undo.len(), history);
            draw(&mut a, &ctx, size, vec![pointer(p, false, Modifiers::NONE)]);
            assert_eq!(a.editor.history.undo.len(), history + 1);
            assert!(a.editor.selection_start.is_none());
            draw(
                &mut a,
                &ctx,
                size,
                vec![
                    pointer(p, true, Modifiers::NONE),
                    pointer(p, false, Modifiers::NONE),
                ],
            );
            assert_eq!(
                a.editor.history.undo.len(),
                history + 1,
                "no empty checkpoint"
            );
            a.run(Command::Undo);
            assert_eq!(a.editor.scene, original);
            a.run(Command::Redo);
            assert_eq!(a.editor.scene, cleared);
            assert_eq!(
                persistence::decode(&persistence::encode(&a.editor.scene).unwrap()).unwrap(),
                cleared
            );
        }
    }
}
#[test]
fn ecs_empty_press_cancel_and_navigation_preserve_selection() {
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    let original = a.editor.scene.clone();
    let history = a.editor.history.undo.len();
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    draw(&mut a, &ctx, size, vec![]);
    draw(&mut a, &ctx, size, vec![]);
    let p = a.viewport_rect.right_top() + vec2(-40., 100.);
    draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(p, true, Modifiers::ALT),
            Event::PointerMoved(p + vec2(10., 10.)),
            pointer(p + vec2(10., 10.), false, Modifiers::ALT),
        ],
    );
    assert_eq!(a.editor.scene, original);
    assert_eq!(a.editor.history.undo.len(), history);
    draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(p, true, Modifiers::NONE),
            Event::WindowFocused(false),
        ],
    );
    assert_eq!(a.editor.scene, original);
    assert_eq!(a.editor.history.undo.len(), history);
    assert!(a.editor.selection_start.is_none());
}
// Jason decision 2026-11-01: a lasso polygon is eligible unless a surface
// hides it, so an unobstructed back-facing quad is selected in every mode.
#[test]
fn m15_lasso_selects_unobstructed_polygons_whichever_way_they_face() {
    for (wireframe, middle, count) in [
        (false, false, 2),
        (false, true, 2),
        (true, false, 2),
        (true, true, 2),
    ] {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        let mesh = &mut a.editor.scene.active_mut().unwrap().mesh;
        for (x, front) in [(-1.5, true), (1.5, false)] {
            let first = mesh.vertices.len() as u32;
            mesh.vertices.extend([
                DVec3::new(x - 0.5, -0.5, 0.),
                DVec3::new(x + 0.5, -0.5, 0.),
                DVec3::new(x + 0.5, 0.5, 0.),
                DVec3::new(x - 0.5, 0.5, 0.),
            ]);
            let mut vertices = vec![first, first + 1, first + 2, first + 3];
            if !front {
                vertices.reverse();
            }
            mesh.faces.push(Face {
                vertices,
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
        let original = a.editor.scene.active().unwrap().mesh.clone();
        a.camera.view = View::Front;
        a.display.set_style(
            if wireframe {
                crate::view_display::Style::Wireframe
            } else {
                crate::view_display::Style::Shaded
            },
            false,
        );
        a.lasso_style = lasso::Style::Rectangle;
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let r = a.viewport_rect;
        let start = r.left_top() + vec2(10., 10. + crate::viewport_buttons::HEADER_CLEARANCE);
        let end = r.right_bottom() - vec2(10., 10.);
        let button = if middle {
            PointerButton::Middle
        } else {
            PointerButton::Secondary
        };
        let event = |pos, pressed| Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let checkpoints = a.editor.history.undo.len();
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![
                event(start, true),
                Event::PointerMoved(end),
                event(end, false),
            ],
        );
        assert_eq!(
            a.editor.scene.selection.faces.len(),
            count,
            "{wireframe}/{middle}: {}",
            a.status
        );
        assert_eq!(a.editor.scene.active().unwrap().mesh, original);
        assert_eq!(a.editor.history.undo.len(), checkpoints + 1);
        a.run(Command::Undo);
        assert!(a.editor.scene.selection.faces.is_empty());
    }
}
// Jason decision 2026-11-01: the right-drag (culling) lasso skips a polygon
// hidden behind another one; the middle-drag lasso selects through.
#[test]
fn m15_lasso_skips_polygons_hidden_behind_a_surface() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mesh = &mut a.editor.scene.active_mut().unwrap().mesh;
    for z in [0., -0.5] {
        let first = mesh.vertices.len() as u32;
        mesh.vertices.extend([
            DVec3::new(-1., -1., z),
            DVec3::new(1., -1., z),
            DVec3::new(1., 1., z),
            DVec3::new(-1., 1., z),
        ]);
        mesh.faces.push(Face {
            vertices: vec![first, first + 1, first + 2, first + 3],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
    }
    a.camera.view = View::Front;
    a.lasso_style = lasso::Style::Rectangle;
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    let r = a.viewport_rect;
    let start = r.left_top() + vec2(10., 10. + crate::viewport_buttons::HEADER_CLEARANCE);
    let end = r.right_bottom() - vec2(10., 10.);
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
        vec![
            event(start, true),
            Event::PointerMoved(end),
            event(end, false),
        ],
    );
    assert_eq!(a.editor.scene.selection.faces, vec![0]);
    let through = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Middle,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            through(start, true),
            Event::PointerMoved(end),
            through(end, false),
        ],
    );
    assert_eq!(a.editor.scene.selection.faces, vec![0, 1]);
}
/// UIC-306: Shift+Up/Down grow and shrink the selection with Move live,
/// and the tool stays on (Jason, 2026-11-27).
#[test]
fn shift_arrows_grow_and_shrink_with_move_live() {
    let mut a = box_app();
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    a.activate(Tool::Move);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
    let shift = |k| Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::SHIFT,
    };
    draw(&mut a, &ctx, size, vec![shift(Key::ArrowUp)]);
    assert_eq!(a.editor.scene.selection.faces.len(), 5, "{}", a.status);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
    draw(&mut a, &ctx, size, vec![shift(Key::ArrowDown)]);
    assert_eq!(a.editor.scene.selection.faces, vec![0], "{}", a.status);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
    // Other selection keys still wait for Drop.
    draw(&mut a, &ctx, size, vec![key(Key::OpenBracket)]);
    assert_eq!(a.editor.scene.selection.faces, vec![0]);
}

#[test]
fn m15_up_arrow_selects_more_and_shift_cmd_click_fills() {
    let mut a = box_app();
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    a.editor.scene.selection.mode = Mode::Polygon;
    let mesh = a.editor.scene.active().unwrap().mesh.clone();
    let by_normal = |n: DVec3| {
        (0..mesh.faces.len())
            .find(|&f| mesh.normal(f).dot(n) > 0.9)
            .unwrap() as u32
    };
    let (front, top, back) = (
        by_normal(DVec3::Z),
        by_normal(DVec3::Y),
        by_normal(-DVec3::Z),
    );
    a.editor.scene.selection.faces = vec![front, top];
    draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::ArrowUp)]);
    assert_eq!(a.editor.scene.selection.faces, vec![front, top, back]);
    draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::ArrowDown)]);
    assert_eq!(a.editor.scene.selection.faces, vec![front, top]);

    // Side band encloses the top cap; the top view sees only the top face.
    let band: Vec<u32> = (0..mesh.faces.len() as u32)
        .filter(|&f| mesh.normal(f as usize).y.abs() < 0.5)
        .collect();
    a.editor.scene.selection.faces = band.clone();
    let history = a.editor.history.undo.len();
    let p = a.viewport_rect.center();
    // Jason decision 2026-11-01 (SEL-Q-M3): one Shift+Cmd click fills.
    let mods = Modifiers::MAC_CMD | Modifiers::SHIFT;
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![pointer(p, true, mods), pointer(p, false, mods)],
    );
    assert_eq!(a.editor.scene.selection.faces.len(), 5, "{}", a.status);
    assert!(a.editor.scene.selection.faces.contains(&top));
    assert_eq!(&a.editor.scene.selection.faces[..4], &band[..]);
    assert!(a.editor.history.undo.len() > history);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.selection.faces, band);
}

// Jason decision 2026-11-01 (SEL-U01): Shift+click adds in every mode, and
// in Polygon mode a plain Shift+click (no Cmd) adds rather than fills.
#[test]
fn shift_click_adds_in_every_component_mode() {
    let mut a = box_app();
    // Perspective shows several faces of the box; the top view shows one.
    a.camera.view = View::Perspective;
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    let r = a.viewport_rect;
    for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon] {
        a.editor.scene.selection.mode = mode;
        a.run(Command::ClearSelection);
        // Candidate screen points: projected points, edge midpoints and
        // polygon centers; keep two that pick different components.
        let mesh = a.editor.scene.active().unwrap().mesh.clone();
        let mut world: Vec<DVec3> = mesh.vertices.clone();
        for f in &mesh.faces {
            let n = f.vertices.len();
            let corners: Vec<DVec3> = f
                .vertices
                .iter()
                .map(|&v| mesh.vertices[v as usize])
                .collect();
            world.push(corners.iter().copied().sum::<DVec3>() / n as f64);
            for i in 0..n {
                world.push((corners[i] + corners[(i + 1) % n]) * 0.5);
            }
        }
        let hits: Vec<Pos2> = world
            .into_iter()
            .filter_map(|w| a.camera.project(w, r).map(|(at, _)| at))
            .collect();
        let mut picked = Vec::new();
        for at in hits {
            if let Some(hit) = a.pick_component(at, r)
                && !picked.iter().any(|(_, h)| *h == hit)
            {
                picked.push((at, hit));
            }
            if picked.len() == 2 {
                break;
            }
        }
        assert_eq!(picked.len(), 2, "{mode:?}: two pickable components");
        let count = |a: &Operandi| {
            let s = &a.editor.scene.selection;
            match mode {
                Mode::Vertex => s.vertices.len(),
                Mode::Edge => s.edges.len(),
                _ => s.faces.len(),
            }
        };
        for (i, (at, _)) in picked.iter().enumerate() {
            let mods = if i == 0 {
                Modifiers::NONE
            } else {
                Modifiers::SHIFT
            };
            draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![pointer(*at, true, mods), pointer(*at, false, mods)],
            );
        }
        assert_eq!(count(&a), 2, "{mode:?}: Shift+click added ({})", a.status);
    }
}

// Jason decision 2026-11-01: edges follow the same rule. A right-drag lasso
// around a whole cube in perspective skips the edges hidden at the back; a
// middle-drag selects all twelve.
#[test]
fn lasso_skips_hidden_cube_edges_and_middle_drag_selects_through() {
    let mut a = box_app();
    a.camera.view = View::Perspective;
    a.lasso_style = lasso::Style::Rectangle;
    a.editor.scene.selection.mode = Mode::Edge;
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    let r = a.viewport_rect;
    let (start, end) = (
        r.left_top() + vec2(10., 10. + crate::viewport_buttons::HEADER_CLEARANCE),
        r.right_bottom() - vec2(10., 10.),
    );
    let lasso = |a: &mut Operandi, button| {
        let event = |pos, pressed| Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        };
        draw(
            a,
            &ctx,
            vec2(1440., 900.),
            vec![
                event(start, true),
                Event::PointerMoved(end),
                event(end, false),
            ],
        );
        a.editor.scene.selection.edges.len()
    };
    let culled = lasso(&mut a, PointerButton::Secondary);
    assert!(
        (1..12).contains(&culled),
        "{culled} visible edges: {}",
        a.status
    );
    assert_eq!(lasso(&mut a, PointerButton::Middle), 12);
}

/// SEL-PHY-03: on the Grid4 with polygon 0 picked, Shift+Cmd on polygon 3
/// (outside any selected perimeter) selects the row [0,1,2,3].
#[test]
fn shift_cmd_second_polygon_selects_the_row_between() {
    let mut a = box_app();
    let mut points = Vec::new();
    for y in 0..5 {
        for x in 0..5 {
            points.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    let mut faces = Vec::new();
    for y in 0..4u32 {
        for x in 0..4u32 {
            let v = y * 5 + x;
            faces.push(vec![v, v + 1, v + 6, v + 5]);
        }
    }
    let mut mesh = a.editor.scene.active().unwrap().mesh.to_owned_value();
    mesh.vertices = points;
    mesh.faces = faces
        .into_iter()
        .map(|vertices| Face {
            vertices,
            ..mesh.faces[0].clone()
        })
        .collect();
    mesh.uv_maps.clear();
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = View::Front;
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    a.execute("Frame All");
    draw(&mut a, &ctx, size, vec![]);
    let (p, _) = a
        .camera
        .project(DVec3::new(3.5, 0.5, 0.), a.viewport_rect)
        .unwrap();
    let mods = Modifiers::MAC_CMD | Modifiers::SHIFT;
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(p, true, mods), pointer(p, false, mods)],
    );
    assert_eq!(
        a.editor.scene.selection.faces,
        vec![0, 1, 2, 3],
        "{}",
        a.status
    );
}
// A hand-drawn lasso records a point per pixel of pointer travel, so a real
// loop has hundreds of points. It must still select polygons on an ordinary
// mesh (a 24x12 globe here) instead of refusing on the work budget.
#[test]
fn freeform_lasso_with_a_long_path_selects_polygons_on_a_real_mesh() {
    for partial in [false, true] {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = operandi_core::radial::globe_geometry(
            DVec3::ZERO,
            operandi_core::RadialParameters {
                radius: DVec3::ONE,
                sides: 24,
                segments: 12,
            },
        )
        .unwrap()
        .into();
        a.editor.scene.selection.mode = Mode::Polygon;
        a.camera.view = View::Front;
        a.lasso_style = lasso::Style::Freeform;
        a.lasso_partial = partial;
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let r = a.viewport_rect;
        // A big loop around the whole globe, one point per pixel or so.
        let center = r.center();
        let radius = r.height() * 0.45;
        let steps = 2000;
        let at = |i: usize| {
            let t = i as f32 / steps as f32 * std::f32::consts::TAU;
            center + vec2(t.cos(), t.sin()) * radius
        };
        let mut events = vec![Event::PointerButton {
            pos: at(0),
            button: PointerButton::Secondary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }];
        events.extend((1..=steps).map(|i| Event::PointerMoved(at(i))));
        events.push(Event::PointerButton {
            pos: at(steps),
            button: PointerButton::Secondary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        let started = std::time::Instant::now();
        draw(&mut a, &ctx, vec2(1440., 900.), events);
        let elapsed = started.elapsed();
        let faces = a.editor.scene.active().unwrap().mesh.faces.len();
        let selected = a.editor.scene.selection.faces.len();
        // The culling lasso takes the visible (front) half of the globe.
        assert!(
            selected >= faces / 3 && selected < faces,
            "partial {partial}: {selected} of {faces} selected; status: {}",
            a.status
        );
        assert!(elapsed.as_millis() < 2000, "lasso took {elapsed:?}");
    }
}

/// A 4×4 quad grid facing +Y (the Top view), point = row × 5 + column.
fn grid_app() -> Operandi {
    let mut a = box_app();
    let mesh = &mut a.editor.scene.active_mut().unwrap().mesh;
    **mesh = Mesh::default();
    for row in 0..5 {
        for column in 0..5 {
            mesh.vertices
                .push(DVec3::new(column as f64 - 2., 0., row as f64 - 2.));
        }
    }
    for row in 0..4u32 {
        for column in 0..4u32 {
            let a = row * 5 + column;
            mesh.faces.push(face(vec![a, a + 5, a + 6, a + 1]));
        }
    }
    a
}

/// The screen point of edge `a`-`b`'s midpoint, checked to pick that edge.
fn edge_point(a: &Operandi, edge: (u32, u32)) -> Pos2 {
    let mesh = &a.editor.scene.active().unwrap().mesh;
    let mid = (mesh.vertices[edge.0 as usize] + mesh.vertices[edge.1 as usize]) * 0.5;
    let r = a.viewport_rect;
    let at = a.camera.project(mid, r).unwrap().0;
    assert_eq!(
        a.pick_component(at, r).map(|(_, c)| c),
        Some(Component::Edge(edge.0, edge.1))
    );
    at
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2, mods: Modifiers) {
    draw(
        a,
        ctx,
        vec2(1440., 900.),
        vec![pointer(at, true, mods), pointer(at, false, mods)],
    );
}

fn sorted_edges(a: &Operandi) -> Vec<(u32, u32)> {
    let mut edges = a.editor.scene.selection.edges.clone();
    edges.sort();
    edges
}

// Edge mode: double-click selects the edge's loop, Shift+double-click adds
// it; the first click is its own Selection step, the loop one more.
#[test]
fn edge_double_click_selects_and_shift_adds_the_loop() {
    let mut a = grid_app();
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    a.editor.scene.selection.mode = Mode::Edge;
    let row = edge_point(&a, (6, 7));
    let history = a.editor.history.undo.len();
    click(&mut a, &ctx, row, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.edges, vec![(6, 7)], "{}", a.status);
    click(&mut a, &ctx, row, Modifiers::NONE);
    assert_eq!(
        sorted_edges(&a),
        vec![(5, 6), (6, 7), (7, 8), (8, 9)],
        "{}",
        a.status
    );
    assert_eq!(a.status, "Select Loop: 4 selected Edge");
    assert_eq!(a.editor.history.undo.len(), history + 2);

    // A third press is an ordinary click again.
    click(&mut a, &ctx, row, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.edges, vec![(6, 7)]);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.selection.edges.len(), 4);

    // Shift+double-click on column 1 adds that loop after the row (column
    // 0 is the open border, whose loop runs all round it: PF-03).
    let column = edge_point(&a, (6, 11));
    click(&mut a, &ctx, column, Modifiers::SHIFT);
    click(&mut a, &ctx, column, Modifiers::SHIFT);
    let edges = &a.editor.scene.selection.edges;
    assert_eq!(edges.len(), 8, "{}", a.status);
    assert_eq!(
        {
            let mut row: Vec<_> = edges[..4].to_vec();
            row.sort();
            row
        },
        vec![(5, 6), (6, 7), (7, 8), (8, 9)]
    );
    assert!(edges.contains(&(1, 6)) && edges.contains(&(16, 21)));
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.selection.edges.len(), 5, "loop undone alone");
}

#[test]
fn slow_second_click_and_other_modes_do_not_select_a_loop() {
    let mut a = grid_app();
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    a.editor.scene.selection.mode = Mode::Edge;
    let at = edge_point(&a, (6, 7));
    click(&mut a, &ctx, at, Modifiers::NONE);
    // Synthetic frames advance 1/60 s: 30 of them is half a second.
    for _ in 0..30 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    click(&mut a, &ctx, at, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.edges, vec![(6, 7)]);

    // Ctrl (remove) presses never make a loop.
    click(&mut a, &ctx, at, Modifiers::CTRL);
    click(&mut a, &ctx, at, Modifiers::CTRL);
    assert!(a.editor.scene.selection.edges.is_empty());

    // Point 6 (a Polygon double-click selects the connected piece; see
    // polygon_double_click_selects_the_connected_piece).
    let r = a.viewport_rect;
    a.editor.scene.selection.mode = Mode::Vertex;
    a.run(Command::ClearSelection);
    let at = a.camera.project(DVec3::new(-1., 0., -1.), r).unwrap().0;
    click(&mut a, &ctx, at, Modifiers::NONE);
    click(&mut a, &ctx, at, Modifiers::NONE);
    let s = &a.editor.scene.selection;
    assert_eq!(s.vertices.len(), 1, "Vertex: one pick");
    assert!(s.edges.is_empty());
}

// Jason, 2026-11-10: in Polygons mode a double-click selects every polygon
// connected to the clicked one; the first click is its own step, the piece
// one more "Select Connected" step. Slow second clicks and Ctrl don't.
#[test]
fn polygon_double_click_selects_the_connected_piece() {
    let mut a = grid_app();
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    a.editor.scene.selection.mode = Mode::Polygon;
    let r = a.viewport_rect;
    // The center of the polygon on points 6, 7, 11, 12.
    let at = a.camera.project(DVec3::new(-0.5, 0., -0.5), r).unwrap().0;
    let all = a.editor.scene.active().unwrap().mesh.faces.len();
    let history = a.editor.history.undo.len();
    click(&mut a, &ctx, at, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.faces.len(), 1, "{}", a.status);
    let clicked = a.editor.scene.selection.faces[0];
    click(&mut a, &ctx, at, Modifiers::NONE);
    let faces = &a.editor.scene.selection.faces;
    assert_eq!(faces.len(), all, "{}", a.status);
    assert_eq!(faces[0], clicked, "the clicked polygon stays first");
    assert_eq!(
        a.status,
        format!("Select Connected: {all} selected Polygons")
    );
    assert_eq!(a.editor.history.undo.len(), history + 2);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.selection.faces, vec![clicked]);

    // A third press is an ordinary click again.
    a.run(Command::Redo);
    click(&mut a, &ctx, at, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.faces, vec![clicked]);

    // A slow second click is two ordinary clicks.
    a.run(Command::ClearSelection);
    click(&mut a, &ctx, at, Modifiers::NONE);
    for _ in 0..30 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    click(&mut a, &ctx, at, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.faces, vec![clicked]);

    // Ctrl (remove) presses never select the piece.
    click(&mut a, &ctx, at, Modifiers::CTRL);
    click(&mut a, &ctx, at, Modifiers::CTRL);
    assert!(a.editor.scene.selection.faces.is_empty());

    // Shift+double-click adds the piece.
    click(&mut a, &ctx, at, Modifiers::SHIFT);
    click(&mut a, &ctx, at, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.selection.faces.len(), all, "{}", a.status);
}

// Jason, 2026-11-06: with a loop selected, Shift+click an edge across it,
// then Shift+double-click that edge (or press Loop): both loops end up
// selected. From an empty selection Shift+double-click selects one loop.
#[test]
fn shift_click_then_shift_double_click_or_loop_keeps_both_loops() {
    let mut a = grid_app();
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    a.editor.scene.selection.mode = Mode::Edge;
    let row_loop = vec![(5, 6), (6, 7), (7, 8), (8, 9)];
    // An inner column: the border's loop runs all round it (PF-03).
    let column_loop = [(1, 6), (6, 11), (11, 16), (16, 21)];
    let both: Vec<(u32, u32)> = {
        let mut v: Vec<_> = row_loop.iter().chain(&column_loop).copied().collect();
        v.sort();
        v
    };
    let row = edge_point(&a, (6, 7));
    let column = edge_point(&a, (6, 11));
    let pause = |a: &mut Operandi| {
        for _ in 0..30 {
            draw(a, &ctx, vec2(1440., 900.), vec![]);
        }
    };

    // Shift+double-click from nothing.
    click(&mut a, &ctx, row, Modifiers::SHIFT);
    click(&mut a, &ctx, row, Modifiers::SHIFT);
    assert_eq!(sorted_edges(&a), row_loop, "{}", a.status);
    pause(&mut a);

    // Shift+click across it, a pause, then Shift+double-click there.
    click(&mut a, &ctx, column, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.selection.edges.len(), 5);
    pause(&mut a);
    click(&mut a, &ctx, column, Modifiers::SHIFT);
    click(&mut a, &ctx, column, Modifiers::SHIFT);
    assert_eq!(sorted_edges(&a), both, "{}", a.status);

    // The same Shift+click, then the Loop command.
    a.run(Command::ClearSelection);
    pause(&mut a);
    click(&mut a, &ctx, row, Modifiers::NONE);
    click(&mut a, &ctx, row, Modifiers::NONE);
    pause(&mut a);
    click(&mut a, &ctx, column, Modifiers::SHIFT);
    a.run(Command::Selection(SelectionOperation::Loop));
    assert_eq!(sorted_edges(&a), both, "{}", a.status);
}

#[test]
fn refused_polygon_double_click_keeps_the_dispatch_message_and_selection() {
    for polyline in [true, false] {
        let ctx = Context::default();
        let mut a = grid_app();
        let mesh = &mut a.editor.scene.active_mut().unwrap().mesh;
        let total = mesh.faces.len();
        if polyline {
            use operandi_core::PolygonRepresentation;
            mesh.attributes.representations = vec![PolygonRepresentation::Face; total];
            mesh.attributes.representations[total - 1] = PolygonRepresentation::Polyline {
                closed: false,
                curve: Default::default(),
            };
        }
        a.editor.scene.selection.mode = Mode::Polygon;
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let at = a
            .camera
            .project(DVec3::new(-0.5, 0., -0.5), a.viewport_rect)
            .unwrap()
            .0;
        click(&mut a, &ctx, at, Modifiers::NONE);
        let before = a.editor.scene.selection.clone();
        assert_eq!(before.faces.len(), 1);
        click(&mut a, &ctx, at, Modifiers::NONE);
        if polyline {
            assert_eq!(
                a.status,
                "This selection command doesn't work on polylines yet; click components or use Select All or Invert"
            );
            assert_eq!(a.editor.scene.selection, before);
        } else {
            assert_eq!(a.editor.scene.selection.faces.len(), total);
            assert_eq!(
                a.status,
                format!("Select Connected: {total} selected Polygons")
            );
        }
    }
}
