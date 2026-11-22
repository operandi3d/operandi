//! Navigation keys and gestures, geometry lock and hide in the app
//! (specifications/nav-lock-hide-2026-11-07).
use super::tests::{box_app, draw, key};
use super::*;
use crate::gpu_scene::Resident;
use crate::input::NavigationKind;
use crate::preferences::NavigationPreset;
use eframe::egui::PointerButton;

const SIZE: Vec2 = vec2(1440., 900.);

fn key_with(k: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn polygons(a: &mut Operandi) {
    a.run(Command::Mode(Mode::Polygon));
    a.run(Command::ClearSelection);
}

/// The new default keys parse, don't collide, and Re-centre starts unbound
/// (G stays Edge Slice, Jason 2026-11-07).
#[test]
fn defaults_are_valid_and_recentre_is_unbound() {
    let b = input::default_bindings();
    assert!(input::bindings_valid(&b));
    assert_eq!(b[input::RECENTRE], "");
    assert_eq!(b["Edge Slice"], "G");
    for (action, chord) in [
        ("Lock Selected", "J"),
        ("Lock Unselected", "Shift+J"),
        ("Lock Invert", "Cmd+J"),
        ("Unlock", "I"),
        ("Hide Selected", "H"),
        ("Hide Unselected", "Shift+H"),
        ("Hide Invert", "Ctrl+H"),
        ("Unhide", "U"),
        ("Zoom In", "Period"),
        ("Zoom Out", "Comma"),
        ("Turn", "Slash"),
        ("Turn Upright", "Shift+Slash"),
        ("Align Selected", "Cmd+A"),
        ("Fit and Align Selected", "Shift+Cmd+A"),
    ] {
        assert_eq!(b[action], chord, "{action}");
    }
    // A map saved before these existed keeps its own use of a chord.
    let dir = std::env::temp_dir().join(format!("operandi-nlh-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bindings.json");
    std::fs::write(&path, r#"{"Frame All":"H"}"#).unwrap();
    let loaded = input::load_bindings(&path);
    assert_eq!(loaded["Frame All"], "H");
    assert_eq!(loaded["Hide Selected"], "");
    std::fs::remove_dir_all(&dir).ok();
}

/// HID-10 / LCK-30 through the keyboard: H hides, U shows, J locks, I
/// unlocks.
#[test]
fn keys_hide_and_lock() {
    let mut a = box_app();
    polygons(&mut a);
    let ctx = Context::default();
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![key(Key::H)]);
    assert_eq!(
        a.editor.scene.active().unwrap().elements.hidden.faces.len(),
        6
    );
    draw(&mut a, &ctx, SIZE, vec![key(Key::U)]);
    assert!(a.editor.scene.active().unwrap().elements.hidden.is_empty());
    draw(&mut a, &ctx, SIZE, vec![key(Key::J)]);
    assert_eq!(
        a.editor.scene.active().unwrap().elements.locked.faces.len(),
        6
    );
    draw(&mut a, &ctx, SIZE, vec![key(Key::I)]);
    assert!(a.editor.scene.active().unwrap().elements.locked.is_empty());
}

/// HID-12: Hide Invert is the real Control key; on macOS Cmd+H is left to
/// the system (it hides the app).
#[test]
fn hide_invert_is_physical_ctrl() {
    let mut a = box_app();
    polygons(&mut a);
    a.run(Command::Select {
        item: a.editor.scene.active().unwrap().id,
        hit: operandi_core::Component::Polygon(0),
        replace: true,
        remove: false,
    });
    a.run(Command::EditElements {
        kind: operandi_core::ElementKind::Hide,
        op: operandi_core::ElementOp::Selected,
    });
    let ctx = Context::default();
    draw(&mut a, &ctx, SIZE, vec![]);
    let ctrl = Modifiers {
        ctrl: true,
        command: !cfg!(target_os = "macos"),
        ..Default::default()
    };
    draw(&mut a, &ctx, SIZE, vec![key_with(Key::H, ctrl)]);
    assert_eq!(
        a.editor.scene.active().unwrap().elements.hidden.faces.len(),
        5,
        "{}",
        a.status
    );
    if cfg!(target_os = "macos") {
        let cmd = Modifiers {
            mac_cmd: true,
            command: true,
            ..Default::default()
        };
        assert!(!input::matches_chord("Ctrl+H", Key::H, cmd));
        draw(&mut a, &ctx, SIZE, vec![key_with(Key::H, cmd)]);
        assert_eq!(
            a.editor.scene.active().unwrap().elements.hidden.faces.len(),
            5
        );
    }
}

/// Edit and View menus carry the lock and hide entries with their
/// shortcuts (UIC-04) and no "…" (UIC-03); they run the same commands.
#[test]
fn menus_list_lock_hide_and_navigate() {
    let mut a = box_app();
    polygons(&mut a);
    let ctx = Context::default();
    let menus = a.menu_model(&ctx);
    let titles: Vec<String> = menus
        .iter()
        .flat_map(|m| m.leaves())
        .map(|i| i.title.clone())
        .collect();
    for name in [
        "Lock Selected",
        "Lock Unselected",
        "Lock Invert",
        "Unlock",
        "Hide Selected",
        "Hide Unselected",
        "Hide Invert",
        "Unhide",
        "Re-centre",
        "Zoom In",
        "Turn Upright",
        "Show Locked Mesh Items",
    ] {
        let title = titles
            .iter()
            .find(|t| t.starts_with(name))
            .unwrap_or_else(|| panic!("{name} missing"));
        assert!(!title.contains('…'), "{title}");
    }
    assert!(
        titles
            .iter()
            .any(|t| t.starts_with("Lock Selected") && t.contains("[J]"))
    );
    a.menu_action(&ctx, app_menu::line_action("Hide Selected"));
    assert_eq!(
        a.editor.scene.active().unwrap().elements.hidden.faces.len(),
        6
    );
    a.menu_action(&ctx, app_menu::line_action("Unhide"));
    assert!(a.editor.scene.active().unwrap().elements.hidden.is_empty());
}

/// LCK-23/25 and HID-03 on the graphics card: locked polygons fill in the
/// locked colour while Show Locked Mesh Items is on; hidden ones draw
/// nothing.
#[test]
fn lck_t7_locked_colour_and_hidden_geometry() {
    let mut a = box_app();
    polygons(&mut a);
    let id = a.editor.scene.active().unwrap().id;
    a.run(Command::Select {
        item: id,
        hit: operandi_core::Component::Polygon(0),
        replace: true,
        remove: false,
    });
    a.run(Command::EditElements {
        kind: operandi_core::ElementKind::Lock,
        op: operandi_core::ElementOp::Selected,
    });
    a.run(Command::Select {
        item: id,
        hit: operandi_core::Component::Polygon(1),
        replace: true,
        remove: false,
    });
    a.run(Command::EditElements {
        kind: operandi_core::ElementKind::Hide,
        op: operandi_core::ElementOp::Selected,
    });
    let item = a.editor.scene.active().unwrap().clone();
    let smoothing = Default::default();
    for show in [true, false] {
        crate::gpu_scene::set_show_locked(show);
        let mut resident = Resident::default();
        let (g, _, _) = resident.item(&item, glam::DMat4::IDENTITY, None, None, &smoothing);
        let locked_fill = &g.fill[g.fill_ranges[0].clone()];
        assert!(!locked_fill.is_empty());
        assert_eq!(
            locked_fill
                .iter()
                .all(|v| v.color == theme::viewport::locked()),
            show
        );
        assert!(g.fill_ranges[1].is_empty() && g.edge_ranges[1].is_empty());
    }
    crate::gpu_scene::set_show_locked(true);
    // LCK-T7: still unselectable with the colour off.
    a.show_locked = false;
    a.run(Command::Selection(operandi_core::SelectionOperation::All));
    assert!(!a.editor.scene.selection.faces.contains(&0));
}

/// NAV-10/11 step zoom; NAV-14 upright; NAV-13 turntable toggles.
#[test]
fn nav_zoom_steps_upright_and_turn() {
    let mut a = box_app();
    a.camera.view = View::Perspective;
    let d = a.camera.distance;
    a.execute("Zoom In");
    assert!((a.camera.distance - d / 2.).abs() < 1e-9);
    a.execute("Zoom Out");
    a.execute("Zoom Out");
    assert!((a.camera.distance - d * 2.).abs() < 1e-9);
    a.camera.roll = 0.7;
    a.execute("Turn Upright");
    assert_eq!(a.camera.roll, 0.);
    a.execute("Turn");
    assert!(matches!(a.view_motion, Some(view_nav::Motion::Turn { .. })));
    a.execute("Turn");
    assert!(a.view_motion.is_none());
}

/// NAV-17/18: the view faces the selected polygon head-on.
#[test]
fn nav_align_selected_faces_the_polygon() {
    let mut a = box_app();
    polygons(&mut a);
    a.camera.view = View::Perspective;
    let id = a.editor.scene.active().unwrap().id;
    a.run(Command::Select {
        item: id,
        hit: operandi_core::Component::Polygon(0),
        replace: true,
        remove: false,
    });
    let mesh = a.editor.scene.active().unwrap().mesh.clone();
    let p: Vec<DVec3> = mesh.faces[0]
        .vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .collect();
    let normal = (p[1] - p[0]).cross(p[2] - p[1]).normalize();
    let distance = a.camera.distance;
    a.execute("Align Selected");
    let (_, _, _, forward) = a.camera.basis();
    assert!((forward + normal).length() < 1e-6, "{forward} vs {normal}");
    assert_eq!(a.camera.distance, distance);
    a.execute("Fit and Align Selected");
    let (_, _, _, forward) = a.camera.basis();
    assert!((forward + normal).length() < 1e-6);
}

/// NAV-02/05/07 and NAV-06's Cmd on the Operandi preset.
#[test]
fn nav_mouse_gestures() {
    let p = NavigationPreset::Operandi;
    let alt = Modifiers::ALT;
    assert_eq!(
        p.navigation(PointerButton::Middle, alt),
        Some(NavigationKind::Roll)
    );
    assert_eq!(
        p.navigation(PointerButton::Secondary, alt),
        Some(NavigationKind::Look)
    );
    assert_eq!(
        p.navigation(PointerButton::Secondary, alt | Modifiers::SHIFT),
        Some(NavigationKind::PanVertical)
    );
    assert_eq!(
        p.navigation(PointerButton::Secondary, alt | Modifiers::CTRL),
        Some(NavigationKind::BoxZoom)
    );
    let cmd = Modifiers {
        alt: true,
        mac_cmd: true,
        command: true,
        ..Default::default()
    };
    assert_eq!(
        p.navigation(PointerButton::Primary, cmd),
        Some(NavigationKind::Zoom)
    );
    assert_eq!(
        p.navigation(PointerButton::Secondary, cmd),
        Some(NavigationKind::BoxZoom)
    );
    // Without Alt the buttons keep their own jobs (lasso, tools).
    assert_eq!(
        p.navigation(PointerButton::Secondary, Modifiers::NONE),
        None
    );

    let mut a = box_app();
    a.camera.view = View::Perspective;
    let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
    // Roll: a quarter turn about the centre.
    a.camera.roll = 0.;
    a.navigate_from(NavigationKind::Roll, pos2(600., 300.), pos2(400., 100.), r);
    assert!(a.camera.roll != 0.);
    // Vertical pan ignores sideways motion.
    let (_, right, _, _) = a.camera.basis();
    let target = a.camera.target;
    a.navigate_from(
        NavigationKind::PanVertical,
        pos2(400., 300.),
        pos2(500., 340.),
        r,
    );
    assert!((a.camera.target - target).dot(right).abs() < 1e-9);
    assert!(a.camera.target != target);
    // Box zoom: a box a quarter of the view's height brings the view 4× in.
    let d = a.camera.distance;
    a.box_zoom(Rect::from_center_size(r.center(), vec2(100., 150.)), r);
    assert!((a.camera.distance - d * 0.25).abs() < 1e-9);
}

/// NAV-14 through the keyboard: Shift+/ levels the view whether the key
/// arrives as Slash or as "?".
#[test]
fn shift_slash_key_levels_the_view() {
    for k in [Key::Slash, Key::Questionmark] {
        let mut a = box_app();
        a.camera.view = View::Perspective;
        let ctx = Context::default();
        draw(&mut a, &ctx, SIZE, vec![]);
        a.camera.roll = 0.7;
        draw(&mut a, &ctx, SIZE, vec![key_with(k, Modifiers::SHIFT)]);
        assert_eq!(a.camera.roll, 0., "{k:?}: {}", a.status);
    }
}

/// NAV-14 from the menu after a real Alt+middle roll gesture: the view is
/// tilted, then level; the status says which.
#[test]
fn roll_then_menu_turn_upright_levels() {
    let mut a = box_app();
    a.camera.view = View::Perspective;
    let ctx = Context::default();
    draw(&mut a, &ctx, SIZE, vec![]);
    let r = a.viewport_rect;
    let c = r.center();
    let press = |pressed, pos| Event::PointerButton {
        pos,
        button: PointerButton::Middle,
        pressed,
        modifiers: Modifiers::ALT,
    };
    let (p0, p1) = (c + vec2(150., 0.), c + vec2(0., 150.));
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(p0)]);
    draw(&mut a, &ctx, SIZE, vec![press(true, p0)]);
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(p1)]);
    draw(&mut a, &ctx, SIZE, vec![press(false, p1)]);
    let (_, _, up, _) = a.camera.basis();
    assert!(up.y < 0.9, "the roll gesture tilts the view: up {up}");
    a.menu_action(&ctx, app_menu::line_action("Turn Upright"));
    let (_, right, up, _) = a.camera.basis();
    assert!(right.y.abs() < 1e-9, "level: right {right}");
    assert!(up.y > 0., "world up points up: {up}");
    assert!(a.status.contains("levelled"), "{}", a.status);
    a.menu_action(&ctx, app_menu::line_action("Turn Upright"));
    assert!(a.status.contains("already level"), "{}", a.status);
}

/// An Alt+left orbit that runs off the edge of the window keeps turning
/// until the button is released (Windows tester, 2026-11-21): leaving the
/// window used to end the orbit, so the view froze until a new click.
/// Losing focus still ends it.
#[test]
fn orbit_keeps_going_when_the_pointer_leaves_the_window() {
    let mut a = box_app();
    a.camera.view = View::Perspective;
    let ctx = Context::default();
    draw(&mut a, &ctx, SIZE, vec![]);
    let c = a.viewport_rect.center();
    let press = |pressed, pos| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::ALT,
    };
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(c)]);
    draw(&mut a, &ctx, SIZE, vec![press(true, c)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(c + vec2(40., 0.))],
    );
    assert!(a.navigation.is_some(), "Alt+left drag orbits");
    let outside = pos2(SIZE.x + 200., c.y);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerGone, Event::PointerMoved(outside)],
    );
    assert!(
        a.navigation.is_some(),
        "the orbit survives leaving the window"
    );
    let yaw = a.camera.yaw;
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(outside + vec2(100., 0.))],
    );
    assert_ne!(
        a.camera.yaw, yaw,
        "moves outside the window still turn the view"
    );
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![press(false, outside + vec2(100., 0.))],
    );
    assert!(a.navigation.is_none(), "the release outside ends it");
    // Focus loss mid-orbit still ends it.
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(c)]);
    draw(&mut a, &ctx, SIZE, vec![press(true, c)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(c + vec2(40., 0.))],
    );
    assert!(a.navigation.is_some());
    draw(&mut a, &ctx, SIZE, vec![Event::WindowFocused(false)]);
    assert!(a.navigation.is_none(), "losing focus ends the orbit");
}

fn click(pos: Pos2, button: PointerButton) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]
}

/// LCK-01/03 through the real Items list: right-click the row, click Lock,
/// the item is locked; right-click again, Unlock.
#[test]
fn items_list_right_click_lock_and_unlock() {
    let mut a = box_app();
    let id = a.editor.scene.active().unwrap().id;
    let ctx = Context::default();
    ctx.enable_accesskit();
    draw(&mut a, &ctx, SIZE, vec![]);
    for (label, locked) in [("Lock", true), ("Unlock", false)] {
        let row = a.scene_rows[&id];
        draw(
            &mut a,
            &ctx,
            SIZE,
            click(row.center(), PointerButton::Secondary),
        );
        let out = draw(&mut a, &ctx, SIZE, vec![]);
        let nodes = out.platform_output.accesskit_update.unwrap().nodes;
        let button = nodes
            .iter()
            .map(|(_, n)| n)
            .find(|n| n.role() == accesskit::Role::Button && n.label() == Some(label))
            .unwrap_or_else(|| panic!("no {label} entry in the right-click menu"));
        let b = button.bounds().unwrap();
        let at = pos2(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32);
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
        draw(&mut a, &ctx, SIZE, click(at, PointerButton::Primary));
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(
            a.editor.scene.active().unwrap().is_locked(),
            locked,
            "{label}: {}",
            a.status
        );
    }
}

/// CL-21 / CL-G12: right-click a camera's row, Set as Render Camera makes
/// it the render camera (one undo step); on the render camera's own row
/// the entry is greyed.
#[test]
fn items_list_right_click_sets_the_render_camera() {
    let mut a = box_app();
    a.run(Command::NewCamera);
    let first = a.editor.scene.selection.item.unwrap();
    a.run(Command::NewCamera);
    let second = a.editor.scene.selection.item.unwrap();
    a.run(Command::SetRenderCamera(first));
    let ctx = Context::default();
    ctx.enable_accesskit();
    draw(&mut a, &ctx, SIZE, vec![]);
    // The entry on a row: (disabled, centre).
    let entry = |a: &mut Operandi, id: u64| {
        let row = a.scene_rows[&id];
        draw(a, &ctx, SIZE, click(row.center(), PointerButton::Secondary));
        let out = draw(a, &ctx, SIZE, vec![]);
        let nodes = out.platform_output.accesskit_update.unwrap().nodes;
        let button = nodes
            .iter()
            .map(|(_, n)| n)
            .find(|n| {
                n.role() == accesskit::Role::Button && n.label() == Some("Set as Render Camera")
            })
            .expect("no Set as Render Camera entry in the camera's right-click menu");
        let b = button.bounds().unwrap();
        (
            button.is_disabled(),
            pos2(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32),
        )
    };
    let (disabled, at) = entry(&mut a, second);
    assert!(!disabled);
    let undo = a.editor.undo.len();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(&mut a, &ctx, SIZE, click(at, PointerButton::Primary));
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.editor.scene.render_camera(), Some(second), "{}", a.status);
    assert_eq!(a.editor.undo.len(), undo + 1);
    draw(&mut a, &ctx, SIZE, vec![key(Key::Escape)]);
    let (disabled, _) = entry(&mut a, second);
    assert!(disabled, "greyed on the render camera's row");
}
