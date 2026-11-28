//! Subdivision in the app: Tab / Shift+Tab over the viewport, the command
//! line and menu, and drawing a smooth mesh.
use super::tests::{draw, key};
use super::*;
use operandi_core::PolygonRepresentation;

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 1)
            .unwrap()
            .into();
    a.editor.history.undo.clear();
    a
}

fn kinds(a: &Operandi) -> Vec<PolygonRepresentation> {
    a.editor
        .scene
        .active()
        .unwrap()
        .mesh
        .attributes
        .representations
        .clone()
}

#[test]
fn tab_and_shift_tab_over_the_viewport_toggle_and_draw() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = app();
    draw(&mut a, &ctx, size, vec![]);
    let viewport = a.viewport_rect.center();
    let mut shift_tab = key(Key::Tab);
    if let Event::Key { modifiers, .. } = &mut shift_tab {
        *modifiers = Modifiers::SHIFT;
    }
    draw(
        &mut a,
        &ctx,
        size,
        vec![Event::PointerMoved(viewport), shift_tab],
    );
    assert_eq!(kinds(&a), vec![PolygonRepresentation::CatmullClark; 6]);
    // The smooth surface draws (and its evaluation is cached).
    draw(&mut a, &ctx, size, vec![]);
    let item = a.editor.scene.active().unwrap().clone();
    let surface = a.subdivision_cache.get(&item).unwrap();
    assert_eq!(surface.mesh.faces.len(), 96);
    // Tab turns smooth polygons back to Face.
    draw(
        &mut a,
        &ctx,
        size,
        vec![Event::PointerMoved(viewport), key(Key::Tab)],
    );
    assert!(kinds(&a).is_empty(), "{}", a.status);
}

#[test]
fn command_line_and_menu_reach_the_same_commands() {
    let mut a = app();
    a.execute("toggle subdivision");
    assert_eq!(kinds(&a), vec![PolygonRepresentation::Subdivs; 6]);
    a.execute("freeze");
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 96);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Freeze");
    let ctx = Context::default();
    let titles: Vec<String> = a
        .menu_model(&ctx)
        .iter()
        .flat_map(app_menu::Menu::leaves)
        .map(|i| i.title.clone())
        .collect();
    assert!(titles.iter().any(|t| t.starts_with("Toggle Catmull-Clark")));
    assert!(titles.iter().any(|t| t.starts_with("Freeze")));
}

#[test]
fn resident_drawing_caches_geometry_and_reuses_it() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = app();
    a.force_resident = true;
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    draw(&mut a, &ctx, size, vec![]);
    let viewport = a.viewport_rect.center();
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(viewport)]);
    let builds = a.resident_cache.builds();
    assert!(builds >= 1);
    // Orbiting and moving the pointer off the mesh don't rebuild it.
    a.camera.yaw += 0.5;
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(pos2(5., 5.))]);
    a.camera.pitch += 0.2;
    draw(&mut a, &ctx, size, vec![]);
    assert_eq!(a.resident_cache.builds(), builds);
    // Its geometry: level 2 Catmull-Clark, 96 quads (192 triangles) and
    // 48 traced cage edges.
    let item = a.editor.scene.active().unwrap().clone();
    let matrix = a.editor.scene.world_matrix(item.id).unwrap();
    let smooth = a.subdivision_cache.get(&item);
    let mut fresh = crate::gpu_scene::Resident::default();
    let (geometry, _, _) = fresh.item(
        &item,
        matrix,
        None,
        smooth.as_deref(),
        &a.editor.scene.smoothing,
    );
    assert_eq!(geometry.fill.len(), 192 * 3);
    // 48 traced edges, each stroked by the pieces on both sides.
    assert_eq!(geometry.edges.len(), 2 * 48 * 6);
}

#[test]
fn dragging_a_point_patches_the_surface_to_the_same_result() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    // A 24 x 24 grid, so a moved point's neighbourhood is a small part.
    let n = 24u32;
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(
                i as f64 / 6.,
                j as f64 / 6.,
                ((i * j) % 4) as f64 * 0.05,
            ));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let q = j * (n + 1) + i;
            mesh.faces.push(operandi_core::Face {
                vertices: vec![q, q + 1, q + n + 2, q + n + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.force_resident = true;
    draw(&mut a, &ctx, size, vec![]);
    let builds = a.resident_cache.builds();
    for step in 1..4 {
        let mut moved = (*a.editor.scene.active().unwrap().mesh).clone();
        moved.vertices[300].z += 0.05 * step as f64;
        a.editor.scene.active_mut().unwrap().mesh = moved.into();
        draw(&mut a, &ctx, size, vec![]);
    }
    // Patched each time, not rebuilt from scratch...
    assert_eq!(a.resident_cache.builds(), builds);
    let item = a.editor.scene.active().unwrap().clone();
    let matrix = a.editor.scene.world_matrix(item.id).unwrap();
    let smooth = a.subdivision_cache.get(&item).unwrap();
    // ...to the same surface and geometry as a fresh evaluation.
    let (fresh_mesh, _, fresh_traced) = operandi_core::subdivision::display_surface_traced(
        &item.mesh,
        &item.subdivision.clone().unwrap_or_default(),
    )
    .unwrap();
    assert_eq!(smooth.traced, fresh_traced);
    for (p, q) in smooth.mesh.vertices.iter().zip(&fresh_mesh.vertices) {
        assert!((*p - *q).abs().max_element() < 1e-9);
    }
    let picked = None;
    let (cached, _, _) = a.resident_cache.item(
        &item,
        matrix,
        picked.clone(),
        Some(&*smooth),
        &Default::default(),
    );
    let mut fresh = crate::gpu_scene::Resident::default();
    let (built, _, _) = fresh.item(&item, matrix, picked, Some(&*smooth), &Default::default());
    assert_eq!(cached.fill, built.fill);
    assert_eq!(cached.edges, built.edges);
}

#[test]
fn selected_and_weighted_edges_follow_the_smooth_surface() {
    // Jason's QA 2026-11-05: in subdivision, a cage edge is drawn along the
    // surface, not on the floating cage.
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    let item = a.editor.scene.active().unwrap().clone();
    let edge = item.mesh.edges()[0];
    let wanted = [edge].into();
    let paths = crate::subdivision_view::edge_segments(
        &a.subdivision_cache,
        &item,
        glam::DMat4::IDENTITY,
        &wanted,
    );
    assert_eq!(paths.len(), 1);
    let (got, segments) = &paths[0];
    assert_eq!(*got, edge);
    // Level 2: the cage edge becomes 4 surface segments, joined end to end.
    assert_eq!(segments.len(), 4);
    let surface = a.subdivision_cache.get(&item).unwrap();
    for &(p, q) in segments {
        for x in [p, q] {
            assert!(
                surface
                    .mesh
                    .vertices
                    .iter()
                    .any(|v| (*v - x).length() < 1e-12)
            );
        }
    }
    // Off the cage line: the surface is inside the cube.
    let (ca, cb) = (
        item.mesh.vertices[edge.0 as usize],
        item.mesh.vertices[edge.1 as usize],
    );
    let mid = (ca + cb) / 2.;
    let nearest = segments
        .iter()
        .flat_map(|&(p, q)| [p, q])
        .map(|p| p.distance(mid))
        .fold(f64::INFINITY, f64::min);
    assert!(nearest > 0.05, "{nearest}");
    // A plain (Face) mesh keeps the cage line.
    let mut flat = item.clone();
    flat.mesh = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
        .unwrap()
        .into();
    let paths = crate::subdivision_view::edge_segments(
        &a.subdivision_cache,
        &flat,
        glam::DMat4::IDENTITY,
        &wanted,
    );
    assert_eq!(paths[0].1, vec![(ca, cb)]);
}

#[test]
fn edges_pick_on_the_smooth_surface_not_the_cage() {
    // Jason's QA 2026-11-05: in subdivision, hovering the drawn (smooth)
    // edge picks it; the invisible cage line doesn't.
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = app();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.run(Command::Mode(Mode::Edge));
    draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    a.camera.distance = 4.;
    let r = a.viewport_rect;
    let item = a.editor.scene.active().unwrap().clone();
    let radius = a.pick_radius();
    let surface = a.surface_cache.get(&a.editor.scene, &a.subdivision_cache);
    let mut tried = 0;
    for edge in item.mesh.edges() {
        let paths = crate::subdivision_view::edge_segments(
            &a.subdivision_cache,
            &item,
            glam::DMat4::IDENTITY,
            &[edge].into(),
        );
        let middle = paths[0].1[1].1;
        let (Some((on, _)), Some((ca, _)), Some((cb, _))) = (
            a.camera.project(middle, r),
            a.camera.project(item.mesh.vertices[edge.0 as usize], r),
            a.camera.project(item.mesh.vertices[edge.1 as usize], r),
        ) else {
            continue;
        };
        if !surface.visible(&a.camera, middle, r)
            || crate::input::segment_distance(on, ca, cb) < 2. * radius
        {
            continue;
        }
        tried += 1;
        assert_eq!(
            a.pick_component(on, r),
            Some((item.id, Component::Edge(edge.0, edge.1))),
            "{edge:?}"
        );
        // The cage line's middle is off the drawn surface: nothing there.
        assert_ne!(
            a.pick_component(ca.lerp(cb, 0.5), r),
            Some((item.id, Component::Edge(edge.0, edge.1)))
        );
    }
    assert!(tried > 0);
}

/// Tab in any text or number field the app draws never toggles Sub-D
/// (Jason, 2026-11-10: it still did). Sweeps every field of the main window
/// in Item and Polygon mode, with the pointer over the viewport.
#[test]
fn tab_in_any_field_never_toggles_subdivision() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    for mode in [Mode::Item, Mode::Polygon] {
        let mut a = app();
        a.editor.scene.selection.mode = mode;
        draw(&mut a, &ctx, size, vec![]);
        draw(&mut a, &ctx, size, vec![]);
        let viewport = a.viewport_rect.center();
        let mut ids = crate::numeric_tab::drawn_fields(&ctx);
        ids.extend(a.text_editors.iter().copied());
        ids.sort_by_key(|id| id.value());
        ids.dedup();
        assert!(ids.len() > 3, "{mode:?}: {} fields", ids.len());
        let mut leaks = Vec::new();
        let mut tried = 0;
        for id in ids {
            if ctx.read_response(id).is_none() {
                continue;
            }
            ctx.memory_mut(|m| m.request_focus(id));
            draw(&mut a, &ctx, size, vec![Event::PointerMoved(viewport)]);
            if ctx.memory(|m| m.focused()) != Some(id) {
                continue;
            }
            tried += 1;
            draw(&mut a, &ctx, size, vec![key(Key::Tab)]);
            if !kinds(&a).is_empty() {
                leaks.push(format!(
                    "{id:?} {:?}",
                    ctx.read_response(id).map(|r| r.rect)
                ));
                a.execute("toggle subdivision");
            }
            ctx.memory_mut(|m| m.surrender_focus(m.focused().unwrap_or(Id::NULL)));
            draw(&mut a, &ctx, size, vec![]);
        }
        assert!(
            leaks.is_empty(),
            "{mode:?}: Tab toggled Sub-D from {leaks:?}"
        );
        eprintln!("{mode:?}: {tried} fields tried");
        assert!(tried > 3, "{mode:?}: only {tried} fields took focus");
        // With no field focused, Tab over the viewport is the shortcut.
        draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(viewport), key(Key::Tab)],
        );
        assert!(!kinds(&a).is_empty(), "{mode:?}: Tab is bound to Sub-D");
    }
}

thread_local! {
    /// Keys for the test's palette OS window, and a field to focus there.
    static CHILD_EVENTS: std::cell::RefCell<Vec<Event>> = const { std::cell::RefCell::new(Vec::new()) };
    static CHILD_FOCUS: std::cell::Cell<Option<Id>> = const { std::cell::Cell::new(None) };
    static CHILD_FOCUSED: std::cell::Cell<Option<Id>> = const { std::cell::Cell::new(None) };
}

/// Run palettes as real OS windows (egui immediate viewports), as the app
/// does, rather than drawing them inside the main window as tests
/// otherwise do. Each has the keyboard and gets `CHILD_EVENTS`.
fn real_os_windows(ctx: &Context) {
    ctx.set_embed_viewports(false);
    Context::set_immediate_viewport_renderer(|ctx, mut viewport| {
        let id = viewport.ids.this;
        let mut raw = RawInput {
            viewport_id: id,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(300., 500.))),
            events: CHILD_EVENTS.with(|e| std::mem::take(&mut *e.borrow_mut())),
            focused: true,
            ..Default::default()
        };
        let info = egui::ViewportInfo {
            focused: Some(true),
            ..Default::default()
        };
        raw.viewports.insert(id, info);
        raw.viewports.insert(ViewportId::ROOT, Default::default());
        let _ = ctx.run(raw, |ctx| {
            if let Some(field) = CHILD_FOCUS.take() {
                ctx.memory_mut(|m| m.request_focus(field));
            }
            (viewport.viewport_ui_cb)(ctx);
            CHILD_FOCUSED.set(ctx.memory(|m| m.focused()));
        });
    });
}

/// Tab in a field of a palette in its own OS window (Absolute) moves to
/// the next field and never toggles Sub-D (Jason, 2026-11-11).
#[test]
fn tab_in_an_os_window_palette_field_moves_on_and_never_toggles_subdivision() {
    let ctx = Context::default();
    theme::install(&ctx);
    real_os_windows(&ctx);
    let size = vec2(1440., 900.);
    let mut a = app();
    draw(&mut a, &ctx, size, vec![]);
    draw(&mut a, &ctx, size, vec![]);
    let main_fields = crate::numeric_tab::drawn_fields(&ctx);
    a.absolute_open = true;
    draw(&mut a, &ctx, size, vec![]);
    draw(&mut a, &ctx, size, vec![]);
    let palette: Vec<Id> = crate::numeric_tab::drawn_fields(&ctx)
        .into_iter()
        .filter(|id| !main_fields.contains(id))
        .collect();
    assert!(palette.len() >= 2, "{palette:?}");
    CHILD_FOCUS.set(Some(palette[0]));
    draw(&mut a, &ctx, size, vec![]);
    draw(&mut a, &ctx, size, vec![]);
    assert_eq!(CHILD_FOCUSED.get(), Some(palette[0]));
    CHILD_EVENTS.with(|e| e.borrow_mut().push(key(Key::Tab)));
    draw(&mut a, &ctx, size, vec![]);
    draw(&mut a, &ctx, size, vec![]);
    assert!(kinds(&a).is_empty(), "Tab toggled Sub-D: {}", a.status);
    assert_eq!(
        CHILD_FOCUSED.get(),
        Some(palette[1]),
        "Tab moved to the next field"
    );
}

/// A smooth cube in the viewport, camera close enough to tell the surface
/// from the cage, after one draw.
fn smooth_cube_in_view(mode: Mode) -> (Operandi, Rect, operandi_core::Item) {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = app();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.run(Command::Mode(mode));
    draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    a.camera.distance = 4.;
    let r = a.viewport_rect;
    let item = a.editor.scene.active().unwrap().clone();
    (a, r, item)
}

#[test]
fn points_show_on_the_smooth_surface_not_the_cage() {
    // Jason 2026-11-15: on a SubD mesh, elements are met where they are
    // drawn, on the smooth surface, not the cage.
    let (a, _, item) = smooth_cube_in_view(Mode::Vertex);
    let shown = a.shown_points(&item);
    let surface = a.subdivision_cache.get(&item).unwrap();
    for (v, &p) in shown.iter().enumerate() {
        // A surface point, pulled in from the cube's corner, and where the
        // point's traced edges meet.
        assert!(surface.mesh.vertices.contains(&p), "{v}");
        assert!(p.length() < item.mesh.vertices[v].length() - 0.1, "{v}");
        let edge = item
            .mesh
            .edges()
            .into_iter()
            .find(|&(x, y)| x == v as u32 || y == v as u32)
            .unwrap();
        let path = crate::subdivision_view::edge_path(
            &a.subdivision_cache,
            &item,
            glam::DMat4::IDENTITY,
            edge,
        );
        assert_eq!(path.len(), 5, "level 2: four pieces");
        let end = if edge.0 == v as u32 { path[0] } else { path[4] };
        assert_eq!(end, p);
    }
    // A mesh with no smooth polygons keeps its cage points.
    let mut flat = item.clone();
    flat.id = 999_999;
    flat.mesh = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 1)
        .unwrap()
        .into();
    assert_eq!(&*a.shown_points(&flat), &flat.mesh.vertices[..]);
}

#[test]
fn points_shared_with_a_face_polygon_stay_on_the_cage() {
    let mut a = app();
    // Only the top polygon smooth: no point has only smooth polygons.
    a.editor.scene.selection.mode = Mode::Polygon;
    a.run(Command::Select {
        item: a.editor.scene.active().unwrap().id,
        hit: Component::Polygon(0),
        replace: true,
        remove: false,
    });
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    let item = a.editor.scene.active().unwrap().clone();
    assert!(operandi_core::subdivision::has_smooth(&item.mesh));
    assert_eq!(&*a.shown_points(&item), &item.mesh.vertices[..]);
}

#[test]
fn points_pick_on_the_smooth_surface_not_the_cage() {
    let (a, r, item) = smooth_cube_in_view(Mode::Vertex);
    let shown = a.shown_points(&item).into_owned();
    let surface = a.surface_cache.get(&a.editor.scene, &a.subdivision_cache);
    let radius = a.pick_radius();
    let mut tried = 0;
    for (v, &p) in shown.iter().enumerate() {
        let cage = item.mesh.vertices[v];
        let (Some((on, _)), Some((off, _))) = (a.camera.project(p, r), a.camera.project(cage, r))
        else {
            continue;
        };
        if !surface.visible(&a.camera, p, r) || on.distance(off) < 2. * radius {
            continue;
        }
        tried += 1;
        assert_eq!(
            a.pick_component(on, r),
            Some((item.id, Component::Vertex(v as u32)))
        );
        assert_ne!(
            a.pick_component(off, r),
            Some((item.id, Component::Vertex(v as u32)))
        );
    }
    assert!(tried > 0);
}

#[test]
fn tweak_grabs_and_outlines_on_the_smooth_surface() {
    // Jason 2026-11-15: Tweak highlighted and grabbed the cage.
    let (mut a, r, item) = smooth_cube_in_view(Mode::Vertex);
    a.activate(Tool::Tweak);
    let shown = a.shown_points(&item).into_owned();
    let surface = a.surface_cache.get(&a.editor.scene, &a.subdivision_cache);
    let radius = a.pick_radius();
    let mut tried = 0;
    for (v, &p) in shown.iter().enumerate() {
        let cage = item.mesh.vertices[v];
        let (Some((on, _)), Some((off, _))) = (a.camera.project(p, r), a.camera.project(cage, r))
        else {
            continue;
        };
        if !surface.visible(&a.camera, p, r) || on.distance(off) < 2. * radius {
            continue;
        }
        tried += 1;
        let pick = a.tweak_pick(on, r).unwrap();
        assert_eq!(
            pick.grab.element,
            operandi_core::TweakElement::Vertex(v as u32)
        );
        assert!(pick.at.distance(p) < 1e-9);
    }
    assert!(tried > 0);
    // An edge: the middle of its second drawn piece grabs it there.
    let mut edges = 0;
    for edge in item.mesh.edges() {
        let path = crate::subdivision_view::edge_path(
            &a.subdivision_cache,
            &item,
            glam::DMat4::IDENTITY,
            edge,
        );
        let middle = path[1].lerp(path[2], 0.5);
        let (Some((on, _)), Some((ca, _)), Some((cb, _))) = (
            a.camera.project(middle, r),
            a.camera.project(item.mesh.vertices[edge.0 as usize], r),
            a.camera.project(item.mesh.vertices[edge.1 as usize], r),
        ) else {
            continue;
        };
        if !surface.visible(&a.camera, middle, r)
            || crate::input::segment_distance(on, ca, cb) < 2. * radius
            || shown
                .iter()
                .filter_map(|&s| a.camera.project(s, r))
                .any(|(q, _)| q.distance(on) < 2. * radius)
        {
            continue;
        }
        edges += 1;
        let pick = a.tweak_pick(on, r).unwrap();
        assert_eq!(
            pick.grab.element,
            operandi_core::TweakElement::Edge(edge.0, edge.1)
        );
        assert!(
            pick.at.distance(middle) < 0.02,
            "{}",
            pick.at.distance(middle)
        );
        // Add Loop and the other edge clicks meet the same place: the
        // second of four pieces, halfway, is 3/8 along the cage edge.
        let (pa, pb, t) = a.pick_edge_parameter(on, r).unwrap();
        assert_eq!((pa.min(pb), pa.max(pb)), edge);
        let from_low = if pa <= pb { t } else { 1. - t };
        assert!((from_low - 0.375).abs() < 0.02, "{from_low}");
    }
    assert!(edges > 0);
}

#[test]
fn edge_slice_picks_and_shows_its_points_on_the_smooth_surface() {
    let (mut a, r, item) = smooth_cube_in_view(Mode::Edge);
    a.activate(Tool::EdgeSlice);
    let surface = a.surface_cache.get(&a.editor.scene, &a.subdivision_cache);
    let radius = a.pick_radius();
    let mut tried = 0;
    for edge in item.mesh.edges() {
        let path = crate::subdivision_view::edge_path(
            &a.subdivision_cache,
            &item,
            glam::DMat4::IDENTITY,
            edge,
        );
        let middle = path[1].lerp(path[2], 0.5);
        let (Some((on, _)), Some((ca, _)), Some((cb, _))) = (
            a.camera.project(middle, r),
            a.camera.project(item.mesh.vertices[edge.0 as usize], r),
            a.camera.project(item.mesh.vertices[edge.1 as usize], r),
        ) else {
            continue;
        };
        if !surface.visible(&a.camera, middle, r)
            || crate::input::segment_distance(on, ca, cb) < 2. * radius
        {
            continue;
        }
        tried += 1;
        let (pa, pb, t) = a.slice_pick_edge(on, r, true).unwrap();
        assert_eq!((pa.min(pb), pa.max(pb)), edge);
        let from_low = if pa <= pb { t } else { 1. - t };
        assert!((from_low - 0.375).abs() < 0.02, "{from_low}");
        // The placed point shows on the surface where it was clicked.
        let shown = a
            .slice_point_world(&operandi_core::SlicePoint::Edge { a: pa, b: pb, t })
            .unwrap();
        assert!(shown.distance(middle) < 0.02, "{}", shown.distance(middle));
    }
    assert!(tried > 0);
}

/// #57: Show Meshes off hides mesh items on the graphics-card path too
/// (only the slower projected path read it, so on a real GPU it did
/// nothing).
#[test]
fn show_meshes_off_hides_meshes_drawn_from_the_graphics_card() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = app();
    a.force_resident = true;
    draw(&mut a, &ctx, size, vec![]);
    let id = a.editor.scene.active().unwrap().id;
    assert!(a.resident_cache.cached(id).is_some(), "drawn from the card");
    a.display.settings.drawing.meshes = false;
    draw(&mut a, &ctx, size, vec![]);
    assert!(a.resident_cache.cached(id).is_none(), "no longer drawn");
    a.display.settings.drawing.meshes = true;
    draw(&mut a, &ctx, size, vec![]);
    assert!(a.resident_cache.cached(id).is_some(), "back again");
}
