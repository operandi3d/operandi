//! M3 Vertex panel: headless panel, shortcut, click and command routing.
//! Geometry oracles live in `operandi-core/tests/m3_vertex_tools.rs`.
use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
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

fn app(mesh: operandi_core::Mesh, points: Vec<u32>) -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = View::Front;
    a.category = toolbox::Category::Vertex;
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = points;
    a.editor.history.undo.clear();
    a
}

fn click(a: &mut Operandi, ctx: &Context, size: Vec2, at: Pos2, modifiers: Modifiers) {
    draw(a, ctx, size, vec![Event::PointerMoved(at)]);
    draw(
        a,
        ctx,
        size,
        vec![pointer(at, true, modifiers), pointer(at, false, modifiers)],
    );
}

/// Key press then release: the tests share one Context, and a key left
/// down would make its next press a repeat, which shortcuts ignore.
fn press(a: &mut Operandi, ctx: &Context, size: Vec2, event: Event) {
    let Event::Key { key, modifiers, .. } = event else {
        unreachable!()
    };
    draw(a, ctx, size, vec![event]);
    draw(
        a,
        ctx,
        size,
        vec![Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        }],
    );
}

fn chord(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn vertex_tab_rows_shortcuts_and_verbs_share_one_operation() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 1800.);
    let mut a = app(grid(), vec![5]);
    draw(&mut a, &ctx, size, vec![]);
    for name in [
        "Create",
        "Vertex Bevel",
        "Vertex Extrude",
        "Merge Tool",
        "Weld",
        "Tear",
        "Vertex Slide",
    ] {
        button_by_line(&a, name);
    }
    assert!(a.toolbox_rects.contains_key("Category Vertex"));
    // Each row runs its row's command (the form, toolbox.vertex.json).
    for row in vertex_panel::VertexRow::ALL {
        button_by_line(&a, row.action());
    }
    // Panel row.
    let row = button_by_line(&a, "Vertex Collapse").center();
    click(&mut a, &ctx, size, row, Modifiers::NONE);
    assert_eq!(
        a.editor.history.undo.last().map(|u| u.label.as_str()),
        Some("Collapse"),
        "{}",
        a.status
    );
    let by_row = a.editor.scene.active().unwrap().mesh.clone();
    // CMD-B3-02: the point and its neighbours merge; four polygons go.
    assert_eq!(by_row.faces.len(), 5);
    // Shortcut Ctrl+Backspace (Vertex mode) and the verb give the same result.
    let mut b = app(grid(), vec![5]);
    draw(&mut b, &ctx, size, vec![]);
    let viewport = b.viewport_rect.center();
    draw(&mut b, &ctx, size, vec![Event::PointerMoved(viewport)]);
    press(&mut b, &ctx, size, chord(Key::Backspace, Modifiers::CTRL));
    assert_eq!(
        *b.editor.scene.active().unwrap().mesh,
        *by_row,
        "{}",
        b.status
    );
    let mut c = app(grid(), vec![5]);
    c.execute("vert.collapse");
    assert_eq!(
        *c.editor.scene.active().unwrap().mesh,
        *by_row,
        "{}",
        c.status
    );
    assert_eq!(c.status, "Collapse done");
    // Shift+Backspace removes points (their polygons keep the other
    // corners, CMD-V-04); P makes a polygon from points.
    let mut d = app(grid(), vec![5]);
    let faces_before = d.editor.scene.active().unwrap().mesh.faces.len();
    draw(&mut d, &ctx, size, vec![]);
    draw(&mut d, &ctx, size, vec![Event::PointerMoved(viewport)]);
    press(&mut d, &ctx, size, chord(Key::Backspace, Modifiers::SHIFT));
    assert_eq!(
        d.editor.scene.active().unwrap().mesh.faces.len(),
        faces_before,
        "{}",
        d.status
    );
    let mut e = app(operandi_core::Mesh::default(), vec![]);
    e.execute("vert.new 0 0 0");
    e.execute("vert.new 1 0 0");
    e.execute("vert.new 1 1 0");
    e.editor.scene.selection.vertices = vec![0, 1, 2];
    draw(&mut e, &ctx, size, vec![]);
    draw(&mut e, &ctx, size, vec![Event::PointerMoved(viewport)]);
    press(&mut e, &ctx, size, key(Key::P));
    assert_eq!(
        e.editor.scene.active().unwrap().mesh.faces.len(),
        1,
        "{}",
        e.status
    );
    assert_eq!(e.editor.history.undo.last().unwrap().label, "Make Polygon");
}

#[test]
fn vertex_tool_shortcuts_and_handle_drags() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    // Front view looks along the grid normal, so Extrude's Width handle is
    // the visible one there.
    for (event, tool, label) in [
        (
            chord(Key::X, Modifiers::SHIFT),
            Tool::VertexExtrude,
            "Width",
        ),
        (key(Key::S), Tool::VertexSlide, "Slide"),
    ] {
        let mut a = app(grid(), vec![5]);
        draw(&mut a, &ctx, size, vec![]);
        let viewport = a.viewport_rect.center();
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(viewport)]);
        press(&mut a, &ctx, size, event);
        assert_eq!(
            a.editor.session.live.as_ref().map(|l| l.tool),
            Some(tool),
            "{}",
            a.status
        );
        draw(&mut a, &ctx, size, vec![]);
        let r = a.viewport_rect;
        let (_, start, end, _) = a
            .handles(r)
            .into_iter()
            .find(|h| h.3 == label)
            .unwrap_or_else(|| panic!("{tool:?} {label} handle"));
        let along = (end - start).normalized() * 30.;
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(end, true, Modifiers::NONE)],
        );
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(end + along)]);
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(end + along, false, Modifiers::NONE)],
        );
        let p = &a.editor.session.live.as_ref().unwrap().params;
        let moved = match tool {
            Tool::VertexBevel | Tool::VertexExtrude => p.inset,
            _ => p.shift,
        };
        assert!(moved > 0., "{tool:?}: {moved} {}", a.status);
        a.run(Command::Drop);
        assert_eq!(a.editor.history.undo.len(), 1, "{tool:?} is one step");
        assert_ne!(*a.editor.scene.active().unwrap().mesh, grid(), "{tool:?}");
    }
    // Shift+X still opens polygon Extrude in Polygon mode.
    let mut a = app(grid(), vec![]);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![4];
    draw(&mut a, &ctx, size, vec![]);
    let viewport = a.viewport_rect.center();
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(viewport)]);
    press(&mut a, &ctx, size, chord(Key::X, Modifiers::SHIFT));
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Extrude),
        "{}",
        a.status
    );
}

#[test]
fn create_clicks_add_points_and_weld_clicks_join_them() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = app(operandi_core::Mesh::default(), vec![]);
    draw(&mut a, &ctx, size, vec![]);
    a.execute("tool.set vert.create on");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::VertexCreate),
        "{}",
        a.status
    );
    let center = a.viewport_rect.center();
    for at in [center, center + vec2(80., 0.), center + vec2(80., -80.)] {
        click(&mut a, &ctx, size, at, Modifiers::NONE);
    }
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .sketch_path
            .len(),
        3,
        "{}",
        a.status
    );
    a.execute("tool.set vert.create ?");
    assert_eq!(a.status, "1");
    a.execute("tool.drop");
    let m = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(m.vertices.len(), 3);
    assert_eq!(a.editor.scene.selection.vertices, vec![0, 1, 2]);
    assert_eq!(a.editor.history.undo.len(), 1);
    // Cancel discards a Create session.
    a.execute("tool.set vert.create on");
    click(
        &mut a,
        &ctx,
        size,
        center + vec2(-60., 40.),
        Modifiers::NONE,
    );
    a.run(Command::Cancel);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 3);

    // Weld is a drag now (WD-01): a click on a point without moving
    // changes nothing. (The drag itself: `weld_drag_*` tests below.)
    let mut w = app(grid(), vec![5]);
    draw(&mut w, &ctx, size, vec![]);
    w.execute("tool.set vert.weld on");
    assert!(w.vertex_weld, "{}", w.status);
    let r = w.viewport_rect;
    let (at, _) = w.camera.project(DVec3::new(2., 1., 0.), r).unwrap();
    assert_eq!(w.pick_vertex(at, r), Some(6));
    click(&mut w, &ctx, size, at, Modifiers::NONE);
    assert_eq!(w.editor.scene.active().unwrap().mesh.vertices.len(), 16);
    assert!(w.editor.history.undo.is_empty(), "{}", w.status);
    // Drop (Q) disarms it.
    w.run(Command::Drop);
    assert!(!w.vertex_weld);
}

/// Arm Weld on the grid (Front view) and return the app, the context, the
/// window size and the viewport rectangle.
fn weld_app(points: Vec<u32>) -> (Operandi, Context, Vec2, Rect) {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = app(grid(), points);
    draw(&mut a, &ctx, size, vec![]);
    a.toggle_weld();
    assert!(a.vertex_weld, "{}", a.status);
    let r = a.viewport_rect;
    (a, ctx, size, r)
}

fn screen(a: &Operandi, p: DVec3, r: Rect) -> Pos2 {
    a.camera.project(p, r).unwrap().0
}

/// Press at `from`, move through `path`, release at its last point.
fn weld_drag(a: &mut Operandi, ctx: &Context, size: Vec2, from: Pos2, path: &[Pos2]) {
    draw(a, ctx, size, vec![Event::PointerMoved(from)]);
    draw(a, ctx, size, vec![pointer(from, true, Modifiers::NONE)]);
    for &p in path {
        draw(a, ctx, size, vec![Event::PointerMoved(p)]);
    }
    let last = *path.last().unwrap_or(&from);
    draw(a, ctx, size, vec![pointer(last, false, Modifiers::NONE)]);
}

#[test]
fn weld_drag_snaps_and_merges_at_the_destination() {
    // WD-02/03/04: drag point 5 to near point 6 (inside the snap radius but
    // not on it): it snaps, and the release merges at 6's position.
    let (mut a, ctx, size, r) = weld_app(vec![0, 15]);
    let from = screen(&a, DVec3::new(1., 1., 0.), r);
    let six = screen(&a, DVec3::new(2., 1., 0.), r);
    let near_six = six + vec2(weld_drag::WELD_SNAP_RADIUS - 4., 0.);
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(from, true, Modifiers::NONE)],
    );
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(near_six)]);
    assert_eq!(a.weld_drag_snap(), Some(6), "highlighted snap target");
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.vertices.len(),
        16,
        "nothing changes until the release"
    );
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(near_six, false, Modifiers::NONE)],
    );
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.vertices.len(), 15, "{}", a.status);
    assert!(mesh.vertices.iter().any(|p| *p == DVec3::new(2., 1., 0.)));
    assert!(!mesh.vertices.iter().any(|p| *p == DVec3::new(1., 1., 0.)));
    assert_eq!(a.editor.history.undo.len(), 1);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Weld");
    // The selection (points 0 and 15) is kept, renumbered past the removed 5.
    assert_eq!(a.editor.scene.selection.vertices, vec![0, 14]);
    // The tool stays armed for another drag, which is its own undo step.
    assert!(a.vertex_weld);
    let from = screen(&a, DVec3::new(2., 2., 0.), r);
    let to = screen(&a, DVec3::new(2., 1., 0.), r);
    weld_drag(&mut a, &ctx, size, from, &[to]);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 14);
    assert_eq!(a.editor.history.undo.len(), 2, "{}", a.status);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 15);
}

#[test]
fn weld_drag_released_in_empty_space_moves_the_point() {
    // WD-05: beyond the snap radius of every point, the point moves to the
    // pointer at its starting depth (z = 0 in the Front view).
    let (mut a, ctx, size, r) = weld_app(vec![]);
    let from = screen(&a, DVec3::new(1., 1., 0.), r);
    let to = screen(&a, DVec3::new(1.5, 1.5, 0.), r);
    assert!(to.distance(screen(&a, DVec3::new(1., 1., 0.), r)) > weld_drag::WELD_SNAP_RADIUS);
    weld_drag(&mut a, &ctx, size, from, &[to]);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.vertices.len(), 16, "{}", a.status);
    assert!(mesh.vertices[5].distance(DVec3::new(1.5, 1.5, 0.)) < 1e-3);
    assert_eq!(mesh.faces, grid().faces);
    assert_eq!(a.editor.history.undo.len(), 1);
    assert!(
        a.editor.scene.selection.vertices.is_empty(),
        "selection unchanged"
    );
    assert!(a.vertex_weld);
}

#[test]
fn weld_drag_just_outside_the_snap_radius_does_not_merge() {
    let (mut a, ctx, size, r) = weld_app(vec![]);
    let from = screen(&a, DVec3::new(1., 1., 0.), r);
    let six = screen(&a, DVec3::new(2., 1., 0.), r);
    weld_drag(
        &mut a,
        &ctx,
        size,
        from,
        &[six + vec2(0., weld_drag::WELD_SNAP_RADIUS + 1.)],
    );
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 16);
    assert_eq!(a.editor.history.undo.len(), 1, "a move, not a merge");
}

#[test]
fn weld_drag_ignores_the_selection_and_escape_cancels_it() {
    // The press picks the point under the pointer even when others are
    // selected; Escape mid-drag cancels only that drag.
    let (mut a, ctx, size, r) = weld_app(vec![10]);
    let from = screen(&a, DVec3::new(1., 1., 0.), r);
    let six = screen(&a, DVec3::new(2., 1., 0.), r);
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(from, true, Modifiers::NONE)],
    );
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(six)]);
    assert_eq!(a.weld_drag_source(), Some(5));
    press(&mut a, &ctx, size, key(Key::Escape));
    assert!(a.weld_drag_source().is_none());
    assert!(a.vertex_weld, "Weld stays armed: {}", a.status);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(six, false, Modifiers::NONE)],
    );
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 16);
    assert!(a.editor.history.undo.is_empty());
    assert_eq!(a.editor.scene.selection.vertices, vec![10]);
}

#[test]
fn weld_drag_from_empty_space_does_nothing() {
    let (mut a, ctx, size, r) = weld_app(vec![]);
    let from = screen(&a, DVec3::new(1.5, 1.5, 0.), r);
    let to = screen(&a, DVec3::new(2., 1., 0.), r);
    weld_drag(&mut a, &ctx, size, from, &[to]);
    assert!(a.editor.history.undo.is_empty(), "{}", a.status);
    assert!(a.editor.scene.selection.vertices.is_empty());
}

#[test]
fn vertex_bevel_drags_anywhere_in_the_viewport() {
    // Jason's GUI test (2026-10-31): away from the handle, right widens the
    // Inset and left narrows it.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = app(grid(), vec![5]);
    draw(&mut a, &ctx, size, vec![]);
    let corner = a.viewport_rect.left_top() + vec2(60., 60.);
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(corner)]);
    press(&mut a, &ctx, size, key(Key::B));
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::VertexBevel)
    );
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    assert_eq!(a.handle_hit(corner, r), None, "start away from handles");
    let drag = |a: &mut Operandi, dx: f32| {
        draw(a, &ctx, size, vec![pointer(corner, true, Modifiers::NONE)]);
        let to = corner + vec2(dx, 0.);
        draw(a, &ctx, size, vec![Event::PointerMoved(to)]);
        draw(a, &ctx, size, vec![pointer(to, false, Modifiers::NONE)]);
        draw(a, &ctx, size, vec![Event::PointerMoved(corner)]);
        a.editor.session.live.as_ref().unwrap().params.inset
    };
    let wide = drag(&mut a, 60.);
    assert!(wide > 0., "{}", a.status);
    let narrow = drag(&mut a, -30.);
    assert!(narrow < wide && narrow > 0., "{wide} {narrow}");
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.len(), 1);
}

#[test]
fn armed_weld_drops_with_q_and_escape() {
    // Jason's GUI test (2026-10-31): Weld could not be dropped.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    // CV-03: Space opens Mini Properties now, it no longer drops.
    for event in [key(Key::Q), key(Key::Escape)] {
        let mut a = app(grid(), vec![]);
        draw(&mut a, &ctx, size, vec![]);
        let viewport = a.viewport_rect.center();
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(viewport)]);
        a.toggle_weld();
        assert!(a.vertex_weld, "{}", a.status);
        press(&mut a, &ctx, size, event.clone());
        assert!(!a.vertex_weld, "{event:?}: {}", a.status);
        assert_eq!(a.status, "Weld off");
        assert_eq!(a.editor.scene.selection.mode, Mode::Vertex, "{event:?}");
    }
}

#[test]
fn every_m3_vertex_row_is_callable_from_the_command_line() {
    let mut a = app(grid(), vec![5]);
    for id in [
        "vert.create",
        "vert.bevel",
        "vert.extrude",
        "vert.mergeTool",
        "vert.weld",
        "vert.tear",
        "vert.slide",
    ] {
        a.execute(&format!("tool.set {id} ?"));
        assert_eq!(a.status, "0", "{id}: {}", a.status);
    }
    for topic in [
        "vert.new",
        "vert.split",
        "vert.splitEdge",
        "vert.align",
        "vert.merge",
        "vert.join",
        "vert.collapse",
        "vert.remove",
        "vert.removeEdge",
        "vert.reorder",
        "vert.syncOrder",
        "vert.weld",
        "view.showVertices",
        "vert.set",
        "poly.make",
        "poly.split",
        "tool.set",
    ] {
        a.execute(&format!("help {topic}"));
        assert!(a.status.starts_with("Help: "), "{topic}: {}", a.status);
    }
    a.execute("help");
    assert!(a.status.contains("vert.syncOrder"), "{}", a.status);
    // tool.set on/off for a live vertex tool, through the shared catalog.
    a.execute("tool.set vert.bevel on");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::VertexBevel),
        "{}",
        a.status
    );
    a.execute("tool.set vert.bevel ?");
    assert_eq!(a.status, "1");
    a.execute("tool.set vert.bevel off");
    assert!(a.editor.session.live.is_none());
    // Malformed input refuses before mutation.
    let before = a.editor.scene.clone();
    for bad in [
        "vert.align axis:q",
        "vert.align uniform:maybe",
        "vert.merge range:fixed",
        "vert.merge keep:true",
        "vert.join foo",
        "vert.join average:true average:false",
        "vert.reorder mode:up",
        "vert.new 1 2",
        "vert.split now",
        "vert.weld target:x",
        "view.showVertices maybe",
        "tool.set vert.weld sideways",
    ] {
        a.execute(bad);
        assert_ne!(a.status, "Ready", "{bad}");
        assert!(!a.status.ends_with(" done"), "{bad}: {}", a.status);
        assert_eq!(a.editor.scene, before, "{bad}");
        assert!(a.editor.history.undo.is_empty(), "{bad}");
    }
    // Each verb runs its operation as one undo step.
    let mut bent = grid();
    bent.vertices[5].y = 1.3;
    for (verb, label) in [
        ("vert.split", "Split"),
        ("vert.join average:true", "Join Averaged"),
        ("vert.align axis:y uniform:true", "Align"),
        ("vert.merge range:fixed dist:2", "Merge"),
        ("vert.remove", "Remove"),
        ("vert.reorder mode:top", "Reorder Index"),
    ] {
        let mut b = app(bent.clone(), vec![4, 5, 6, 7]);
        b.execute(verb);
        assert_eq!(b.editor.history.undo.len(), 1, "{verb}: {}", b.status);
        assert_eq!(b.editor.history.undo.last().unwrap().label, label, "{verb}");
    }
    let mut b = app(grid(), vec![5, 6]);
    b.execute("vert.removeEdge");
    assert_eq!(
        b.editor.scene.active().unwrap().mesh.faces.len(),
        8,
        "{}",
        b.status
    );
    let mut b = app(grid(), vec![5, 6]);
    b.execute("vert.splitEdge");
    assert_eq!(
        b.editor.scene.active().unwrap().mesh.vertices.len(),
        17,
        "{}",
        b.status
    );
    let mut b = app(grid(), vec![5]);
    b.execute("vert.weld target:6");
    assert_eq!(
        b.editor.history.undo.last().unwrap().label,
        "Weld",
        "{}",
        b.status
    );
    // Rows with options open their dialog without arguments.
    let mut b = app(grid(), vec![5]);
    b.execute("vert.new");
    assert!(b.vertex_dialog.is_some());
}

#[test]
fn show_vertices_changes_no_scene_selection_or_history() {
    let mut a = app(grid(), vec![5]);
    let before = a.editor.scene.clone();
    // Starts off (Jason, 2026-11-03; it started checked before).
    assert!(!a.display.settings.drawing.show_verts);
    a.execute("view.showVertices off");
    assert!(!a.display.settings.drawing.show_verts);
    a.execute("view.showVertices on");
    assert!(a.display.settings.drawing.show_verts);
    a.execute("view.showVertices ?");
    assert_eq!(a.status, "1");
    a.run_vertex_row(vertex_panel::VertexRow::ShowVertices);
    assert!(!a.display.settings.drawing.show_verts);
    assert_eq!(a.editor.scene, before);
    assert!(a.editor.history.undo.is_empty());
}

#[test]
fn sync_index_order_captures_then_applies() {
    let mut a = app(
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap(),
        vec![],
    );
    let source = a.editor.scene.selection.item.unwrap();
    let (target, _) = operandi_core::vertex_command_geometry(
        &operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap(),
        &[6, 2, 4, 7],
        operandi_core::VertexCommand::Reorder(operandi_core::ReorderMode::Top),
    )
    .unwrap();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0, 1];
    a.execute("vert.syncOrder");
    assert!(a.status.contains("source captured"), "{}", a.status);
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = target.into();
    a.editor.synchronize_identities().unwrap();
    assert_ne!(a.editor.scene.selection.item, Some(source));
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0, 1];
    a.execute("vert.syncOrder");
    assert_eq!(a.status, "Sync Index Order done");
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.vertices,
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
            .unwrap()
            .vertices
    );
}

#[test]
fn create_xyz_fields_move_the_last_point_and_escape_cancels() {
    // Codex GUI pass F1/F2 (2026-10-31): X/Y/Z text was ignored and Escape
    // over the viewport left Create running.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = app(operandi_core::Mesh::default(), vec![]);
    draw(&mut a, &ctx, size, vec![]);
    a.execute("tool.set vert.create on");
    let center = a.viewport_rect.center();
    click(&mut a, &ctx, size, center, Modifiers::NONE);
    click(&mut a, &ctx, size, center + vec2(80., 0.), Modifiers::NONE);
    draw(&mut a, &ctx, size, vec![]);
    let first = a.editor.session.live.as_ref().unwrap().params.sketch_path[0];
    a.fields.insert("create_point0".into(), "1 m".into());
    a.fields.insert("create_point2".into(), "0.5 m".into());
    assert!(a.confirm_fields(), "{}", a.status);
    let path = a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .sketch_path
        .clone();
    assert_eq!(path[0], first, "earlier points stay put");
    assert_eq!((path[1].x, path[1].z), (1., 0.5));
    a.run(Command::Drop);
    let m = &a.editor.scene.active().unwrap().mesh;
    assert_eq!((m.vertices[1].x, m.vertices[1].z), (1., 0.5));
    assert_eq!(a.editor.history.undo.len(), 1);

    // Escape over the viewport cancels a new Create session.
    let before = a.editor.scene.clone();
    a.execute("tool.set vert.create on");
    click(
        &mut a,
        &ctx,
        size,
        center + vec2(-60., 40.),
        Modifiers::NONE,
    );
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .sketch_path
            .len(),
        1
    );
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(center)]);
    press(&mut a, &ctx, size, key(Key::Escape));
    assert!(a.editor.session.live.is_none(), "{}", a.status);
    assert_eq!(
        a.editor.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    assert_eq!(a.editor.history.undo.len(), 1);
}

#[test]
fn vertex_only_dialog_rows_are_greyed_out_in_other_modes() {
    // Codex GUI pass F3–F5 (2026-10-31): these dialogs opened in Polygon mode.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 1800.);
    // Set Position, Align, Merge and Join run from any component mode now
    // (selection inference, spec §4); New Vertex and Reorder stay Vertex-only.
    let rows = [
        vertex_panel::VertexRow::NewVertex,
        vertex_panel::VertexRow::ReorderIndex,
    ];
    for row in [
        vertex_panel::VertexRow::SetPosition,
        vertex_panel::VertexRow::Align,
        vertex_panel::VertexRow::Merge,
        vertex_panel::VertexRow::Join,
    ] {
        assert!(!row.needs_vertex_mode(), "{}", row.label());
    }
    let mut a = app(grid(), vec![5]);
    a.editor.scene.selection.faces = vec![4];
    a.run(Command::Mode(Mode::Polygon));
    draw(&mut a, &ctx, size, vec![]);
    for row in rows {
        assert!(row.needs_vertex_mode());
        let name = row.action();
        let at = button_by_line(&a, name).center();
        click(&mut a, &ctx, size, at, Modifiers::NONE);
        assert!(!a.has_modal(), "{name} opened a dialog in Polygon mode");
        assert!(a.set_position.is_none(), "{name}");
        // Shortcut and command routes refuse too.
        a.run_vertex_row(row);
        assert!(!a.has_modal() && a.set_position.is_none(), "{name}");
        assert!(
            a.status.contains("works in Vertex mode"),
            "{name}: {}",
            a.status
        );
    }
    // Other rows are unaffected, and Vertex mode re-enables these.
    assert!(!vertex_panel::VertexRow::Split.needs_vertex_mode());
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![5];
    draw(&mut a, &ctx, size, vec![]);
    let at = button_by_line(&a, "Vertex Align").center();
    click(&mut a, &ctx, size, at, Modifiers::NONE);
    assert!(
        a.has_modal(),
        "Align should open in Vertex mode: {}",
        a.status
    );
}

#[test]
fn right_drag_lasso_selects_loose_points() {
    // Jason's GUI test (2026-10-31): fresh Create points had no polygon, so
    // the facing rule rejected them and the lasso selected nothing.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mesh = operandi_core::Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(0., 1., 0.),
        ],
        ..Default::default()
    };
    for view in [View::Front, View::Back] {
        let mut a = app(mesh.clone(), vec![]);
        a.camera.view = view;
        draw(&mut a, &ctx, size, vec![]);
        let r = a.viewport_rect;
        let ev = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let ring: Vec<Pos2> = (0..=16)
            .map(|i| {
                let t = i as f32 / 16. * std::f32::consts::TAU;
                r.center() + vec2(t.cos() * r.width(), t.sin() * r.height()) * 0.45
            })
            .collect();
        draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(ring[0]), ev(ring[0], true)],
        );
        for &p in &ring[1..] {
            draw(&mut a, &ctx, size, vec![Event::PointerMoved(p)]);
        }
        draw(&mut a, &ctx, size, vec![ev(*ring.last().unwrap(), false)]);
        let mut got = a.editor.scene.selection.vertices.clone();
        got.sort_unstable();
        assert_eq!(got, vec![0, 1, 2, 3], "{view:?}: {}", a.status);
    }
}

#[test]
fn right_drag_lasso_takes_points_nothing_hides() {
    // Jason's GUI test (2026-10-31): a corner whose polygons all face away
    // was visible and clickable but the facing-only lasso skipped it.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let lasso_all = |mesh: operandi_core::Mesh| {
        let mut a = app(mesh, vec![]);
        a.camera.view = View::Perspective;
        draw(&mut a, &ctx, size, vec![]);
        let r = a.viewport_rect;
        let ev = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let path = [
            r.left_top() + vec2(5., 5. + crate::viewport_buttons::HEADER_CLEARANCE),
            r.right_top() + vec2(-5., 5.),
            r.right_bottom() - vec2(5., 5.),
            r.left_bottom() + vec2(5., -5.),
            r.left_top() + vec2(5., 5. + crate::viewport_buttons::HEADER_CLEARANCE),
        ];
        draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(path[0]), ev(path[0], true)],
        );
        for &p in &path[1..] {
            draw(&mut a, &ctx, size, vec![Event::PointerMoved(p)]);
        }
        draw(&mut a, &ctx, size, vec![ev(path[4], false)]);
        let mut got = a.editor.scene.selection.vertices.clone();
        got.sort_unstable();
        (a, got)
    };
    let cube = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    let (a, closed) = lasso_all(cube.clone());
    let (eye, ..) = a.camera.basis();
    let nearest = (0..8)
        .min_by(|&i, &j| {
            let d = |k: usize| cube.vertices[k].distance(eye);
            d(i).total_cmp(&d(j))
        })
        .unwrap() as u32;
    let farthest = (0..8)
        .max_by(|&i, &j| {
            let d = |k: usize| cube.vertices[k].distance(eye);
            d(i).total_cmp(&d(j))
        })
        .unwrap() as u32;
    assert!(closed.contains(&nearest), "{closed:?}");
    assert!(
        !closed.contains(&farthest),
        "closed box hides its far corner"
    );
    // Open the box toward the camera: every polygon at the far corner faces
    // away from the viewer, yet nothing hides it any more.
    let mut open = cube.clone();
    open.faces.retain(|f| !f.vertices.contains(&nearest));
    let (a, opened) = lasso_all(open);
    assert!(opened.contains(&farthest), "{opened:?}: {}", a.status);
}

#[test]
fn weld_and_edge_clicks_refuse_while_a_numeric_task_is_open() {
    // Gap closed 2026-10-31: these only refused for live tools and Pen.
    let mut a = app(grid(), vec![5]);
    a.run(Command::ArmCone);
    assert!(a.editor.session.has_active_task(), "{}", a.status);
    a.toggle_weld();
    assert!(!a.vertex_weld);
    assert!(a.status.contains("Drop or Cancel"), "{}", a.status);
    a.toggle_edge_click(edge_panel::EdgeClick::AddLoop);
    assert!(a.edge_click.is_none());
    assert!(a.status.contains("Drop or Cancel"), "{}", a.status);
}

#[test]
fn right_drag_lasso_takes_every_shown_point_on_a_large_sphere() {
    // Jason's report (2026-11-09): the right-drag lasso missed points. The
    // hidden-point test aimed its ray through the point's f32 screen
    // position; the rounding put a neighbouring polygon in front, so on a
    // 100 m sphere about a third of the front points were skipped.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    for (scale, view) in [
        (1., View::Perspective),
        (10., View::Perspective),
        (100., View::Perspective),
        (100., View::Front),
    ] {
        let mesh: operandi_core::Mesh = operandi_core::radial::globe_geometry(
            DVec3::ZERO,
            operandi_core::RadialParameters {
                radius: DVec3::splat(scale),
                sides: 64,
                segments: 32,
            },
        )
        .unwrap();
        let mut a = app(mesh.clone(), vec![]);
        a.camera.view = view;
        a.camera.distance *= scale;
        draw(&mut a, &ctx, size, vec![]);
        let r = a.viewport_rect;
        let ev = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let top = 5. + crate::viewport_buttons::HEADER_CLEARANCE;
        let path = [
            r.left_top() + vec2(5., top),
            r.right_top() + vec2(-5., top),
            r.right_bottom() - vec2(5., 5.),
            r.left_bottom() + vec2(5., -5.),
            r.left_top() + vec2(5., top),
        ];
        draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(path[0]), ev(path[0], true)],
        );
        for &p in &path[1..] {
            draw(&mut a, &ctx, size, vec![Event::PointerMoved(p)]);
        }
        draw(&mut a, &ctx, size, vec![ev(path[4], false)]);
        let got: std::collections::BTreeSet<u32> =
            a.editor.scene.selection.vertices.iter().copied().collect();
        let (eye, _, _, forward) = a.camera.basis();
        for (i, &v) in mesh.vertices.iter().enumerate() {
            let toward = if view == View::Perspective {
                (eye - v).normalize()
            } else {
                -forward
            };
            let facing = v.normalize().dot(toward);
            if facing > 0.05 {
                assert!(
                    got.contains(&(i as u32)),
                    "{scale} m {view:?}: front point {i} missed"
                );
            } else if facing < -0.2 {
                assert!(
                    !got.contains(&(i as u32)),
                    "{scale} m {view:?}: back point {i} taken"
                );
            }
        }
    }
}
