//! Layouts stage 6: palettes (BEHAVIOR-LAYOUTS-STAGE6.md, palettes.rs).
use super::*;
use crate::floating::Panel;
use crate::layouts_tests::{SIZE, data_root, frames, launch};
use crate::palettes::{PRESET_BROWSER, SHIPPED, UV_EDITOR};
use operandi_layout::{Pane, WindowLayout};
use std::path::Path;

fn app(root: &Path) -> (Operandi, Context) {
    let ctx = Context::default();
    let mut a = launch(root, &ctx);
    a.load_palettes();
    frames(&mut a, &ctx, 2);
    (a, ctx)
}

fn backtick(pressed: bool) -> Event {
    Event::Key {
        key: Key::Backtick,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

/// A frame as the app runs it, keys included.
fn key_frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
            focused: true,
            events,
            ..Default::default()
        },
        |ctx| {
            a.begin_input(ctx);
            a.compose(ctx);
            a.keyboard(ctx);
        },
    );
}

/// Layout > Windows/Palettes.
fn windows_menu(a: &mut Operandi, ctx: &Context) -> app_menu::Menu {
    let menus = a.menu_model(ctx);
    let layout = menus.iter().find(|m| m.title == "Layout").unwrap();
    layout
        .entries
        .iter()
        .find_map(|e| match e {
            app_menu::Entry::Submenu(m) if m.title == "Windows/Palettes" => Some(m.clone()),
            _ => None,
        })
        .expect("Layout > Windows/Palettes")
}

#[test]
fn lay50_every_shipped_palette_file_is_a_layout_file_with_a_palette() {
    for (file, text) in SHIPPED {
        let doc = WindowLayout::from_json(text).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(doc.palette.is_some(), "{file}");
        assert!(crate::palettes::palette_from(&doc).is_some(), "{file}");
    }
    let a = Operandi::empty();
    let hashes: Vec<&str> = a.palettes.list.iter().map(|p| p.hash.as_str()).collect();
    for hash in [
        "toolbox",
        "items",
        "properties",
        "shading",
        "groups",
        "vertexMaps",
        "presetBrowser",
        "uvEditor",
        "uvView",
    ] {
        assert!(hashes.contains(&hash), "{hash}: {hashes:?}");
    }
}

#[test]
fn lay51_palettes_open_at_their_default_sizes() {
    let a = Operandi::empty();
    let size = |h: &str| a.palettes.find(h).unwrap().size;
    assert_eq!(size("uvEditor"), [800., 600.]);
    assert_eq!(size("uvView"), [900., 600.]);
    assert_eq!(size("presetBrowser"), [900., 600.]);
    assert_eq!(size("toolbox"), [260., 650.]);
    for h in ["items", "shading", "groups", "vertexMaps"] {
        assert_eq!(size(h), [400., 600.], "{h}");
    }
    assert_eq!(size("properties"), [350., 700.]);
    assert_eq!(a.preset_browser.window_size, [900., 600.]);
}

#[test]
fn lay50_one_toggle_opens_and_closes_and_reopening_brings_it_forward() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    // The menu item opens it...
    a.menu_action(
        &ctx,
        app_menu::line_action(&format!("layout.window hash:{}", "groups")),
    );
    assert!(a.palette_open("groups"));
    frames(&mut a, &ctx, 2);
    // ...the command opens the open one only once, bringing it forward...
    a.execute("layout.window hash:groups open:true");
    assert!(a.palette_open("groups"));
    assert_eq!(a.palettes.focus.as_deref(), Some("groups"));
    assert_eq!(a.palettes.open.iter().filter(|h| *h == "groups").count(), 1);
    // ...and the same item again closes it.
    a.menu_action(
        &ctx,
        app_menu::line_action(&format!("layout.window hash:{}", "groups")),
    );
    assert!(!a.palette_open("groups"));
    // The bare command toggles too.
    a.execute("layout.window hash:groups");
    assert!(a.palette_open("groups"));
    a.execute("layout.window hash:groups");
    assert!(!a.palette_open("groups"));
    // An unknown palette only reports.
    a.execute("layout.window hash:nothing");
    assert!(a.status.contains("no window nothing"), "{}", a.status);
}

#[test]
fn lay50_the_old_float_toolbox_items_and_properties_are_palettes() {
    let root = data_root();
    crate::layouts_tests::install_legacy_model(&root);
    let (mut a, ctx) = app(&root);
    let docked = a.viewport_rect;
    a.menu_action(
        &ctx,
        app_menu::line_action(&format!("layout.window hash:{}", "toolbox")),
    );
    assert!(a.is_floating(Panel::Toolbox));
    frames(&mut a, &ctx, 2);
    // The side column gives the toolbox up, as Float Toolbox did.
    assert!(a.viewport_rect.left() < docked.left() - 100.);
    // The panel header's Float entry is the same toggle.
    a.set_floating(Panel::Toolbox, false);
    assert!(!a.palette_open("toolbox"));
    frames(&mut a, &ctx, 2);
    assert_eq!(a.viewport_rect, docked);
}

#[test]
fn lay51_palettes_reopen_where_they_were_after_closing_and_after_a_restart() {
    let root = data_root();
    {
        let (mut a, ctx) = app(&root);
        a.execute("layout.window hash:shading open:true");
        a.execute("layout.window hash:uvEditor open:true");
        frames(&mut a, &ctx, 2);
        // The user moves and sizes them.
        a.palettes.places.insert(
            "shading".into(),
            crate::palettes::Place {
                size: [420., 640.],
                position: Some([70., 80.]),
            },
        );
        let key = a.palettes.views[UV_EDITOR];
        let window = a.views.window_of(Pane::Uv(key)).unwrap();
        let w = a.views.windows.iter_mut().find(|w| w.id == window).unwrap();
        w.size = [810., 610.];
        w.position = Some([30., 40.]);
        frames(&mut a, &ctx, 2);
        // Closed and opened: the same place.
        a.execute("layout.window hash:uvEditor");
        assert!(!a.palette_open(UV_EDITOR));
        a.execute("layout.window hash:uvEditor");
        let key = a.palettes.views[UV_EDITOR];
        let window = a.views.window_of(Pane::Uv(key)).unwrap();
        let w = a.views.windows.iter().find(|w| w.id == window).unwrap();
        assert_eq!((w.size, w.position), ([810., 610.], Some([30., 40.])));
        frames(&mut a, &ctx, 2);
    }
    // Quit and launch: open again, where they were.
    let (a, _ctx) = app(&root);
    assert!(a.palette_open("shading"));
    assert!(a.palette_open(UV_EDITOR));
    assert_eq!(
        a.palettes.place("shading"),
        crate::palettes::Place {
            size: [420., 640.],
            position: Some([70., 80.])
        }
    );
    let key = a.palettes.views[UV_EDITOR];
    assert!(a.is_uv_editor(key));
    let window = a.views.window_of(Pane::Uv(key)).unwrap();
    let w = a.views.windows.iter().find(|w| w.id == window).unwrap();
    assert_eq!((w.size, w.position), ([810., 610.], Some([30., 40.])));
}

#[test]
fn lay52_palettes_stay_open_where_they_are_across_a_layout_switch() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("layout.window hash:uvEditor open:true");
    a.execute("layout.window hash:items open:true");
    a.execute("layout.window hash:groups open:true");
    frames(&mut a, &ctx, 2);
    let key = a.palettes.views[UV_EDITOR];
    let window = a.views.window_of(Pane::Uv(key)).unwrap();
    a.views
        .windows
        .iter_mut()
        .find(|w| w.id == window)
        .unwrap()
        .position = Some([55., 66.]);
    let place = |a: &Operandi| {
        let key = a.palettes.views[UV_EDITOR];
        let window = a.views.window_of(Pane::Uv(key)).unwrap();
        let w = a.views.windows.iter().find(|w| w.id == window).unwrap();
        (w.size, w.position)
    };
    let before = place(&a);
    for layout in ["UV", "Model", "UV"] {
        a.execute(&format!("layout.restore {layout}"));
        assert_eq!(a.layouts.current, layout);
        frames(&mut a, &ctx, 2);
        assert!(a.palette_open(UV_EDITOR), "{layout}");
        assert!(a.palette_open("items") && a.palette_open("groups"));
        assert_eq!(place(&a), before, "{layout}");
        let key = a.palettes.views[UV_EDITOR];
        assert!(a.is_uv_editor(key) && a.uv_views.contains_key(&key));
        assert_eq!(
            a.views.windows.len(),
            1,
            "one UV Editor, not one per layout"
        );
    }
    // The layout files never list a palette's window.
    for name in ["Model", "UV"] {
        let doc = a.layouts.find(name).unwrap().doc();
        assert!(doc.windows.is_empty(), "{name}");
    }
}

#[test]
fn lay53_backtick_hides_every_palette_and_shows_them_again() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    assert_eq!(a.bindings["Toggle Palettes"], "`");
    // Nothing open: nothing to hide.
    key_frame(&mut a, &ctx, vec![backtick(true), backtick(false)]);
    assert!(!a.palettes.hidden);
    a.execute("layout.window hash:uvEditor open:true");
    a.execute("layout.window hash:shading open:true");
    frames(&mut a, &ctx, 2);
    key_frame(&mut a, &ctx, vec![backtick(true), backtick(false)]);
    assert!(a.palettes.hidden, "{}", a.status);
    // Still open, not shown: the UV Editor's window isn't drawn.
    assert!(a.palette_open(UV_EDITOR) && a.palette_open("shading"));
    let key = a.palettes.views[UV_EDITOR];
    a.view_rects.clear();
    frames(&mut a, &ctx, 2);
    assert!(!a.view_rects.contains_key(&key));
    key_frame(&mut a, &ctx, vec![backtick(true), backtick(false)]);
    assert!(!a.palettes.hidden);
    frames(&mut a, &ctx, 2);
    assert!(a.view_rects.contains_key(&key));
    // The command is the same switch; opening a palette shows them all.
    a.execute("operandi.layout.togglePalettes");
    assert!(a.palettes.hidden);
    a.execute("layout.window hash:groups open:true");
    assert!(!a.palettes.hidden);
}

#[test]
fn lay54_texture_open_uv_editor_toggles_the_uv_editor_palette() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.menu_action(&ctx, app_menu::line_action("Open UV Editor"));
    assert!(a.palette_open(UV_EDITOR));
    let key = a.palettes.views[UV_EDITOR];
    assert!(a.is_uv_editor(key));
    frames(&mut a, &ctx, 2);
    // The UV tools down its right side.
    assert!(a.uv_editor_strip_width(key) > 0.);
    // Texture > Open UV Editor shows a check while it's open.
    let menus = a.menu_model(&ctx);
    let texture = menus.iter().find(|m| m.title == "Texture").unwrap();
    assert!(matches!(
        &texture.entries[0],
        app_menu::Entry::Item(i) if i.checked
    ));
    a.menu_action(&ctx, app_menu::line_action("Open UV Editor"));
    assert!(!a.palette_open(UV_EDITOR));
    assert!(a.views.windows.is_empty());
    // The plain UV View palette has no tool strip.
    a.execute("layout.window hash:uvView");
    let key = a.palettes.views["uvView"];
    assert!(!a.is_uv_editor(key));
    assert_eq!(a.uv_editor_strip_width(key), 0.);
}

#[test]
fn lay55_layout_windows_palettes_lists_every_palette_and_toggle_palettes() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("layout.window hash:items open:true");
    let menu = windows_menu(&mut a, &ctx);
    let items: Vec<(String, bool)> = menu
        .entries
        .iter()
        .filter_map(|e| match e {
            app_menu::Entry::Item(i) => Some((i.title.clone(), i.checked)),
            _ => None,
        })
        .collect();
    for p in &a.palettes.list {
        assert!(
            items.iter().any(|(l, _)| l.starts_with(&p.name)),
            "{}: {items:?}",
            p.name
        );
    }
    assert!(items.iter().any(|(l, on)| l == "Items" && *on), "{items:?}");
    assert!(
        items
            .iter()
            .any(|(l, _)| l.starts_with("Toggle Palettes") && l.contains('`')),
        "{items:?}"
    );
    // The Window menu no longer floats panels: they're palettes now.
    let menus = a.menu_model(&ctx);
    let window = menus.iter().find(|m| m.title == "Window").unwrap();
    assert!(!window.entries.iter().any(|e| matches!(
        e,
        app_menu::Entry::Item(i) if i.title.starts_with("Float") || i.title.starts_with("Preset Browser")
    )));
    // Toggle Palettes from the menu.
    a.menu_action(
        &ctx,
        app_menu::line_action("operandi.layout.togglePalettes"),
    );
    assert!(a.palettes.hidden);
    // The Preset Browser entry is its window.
    a.menu_action(
        &ctx,
        app_menu::line_action(&format!("layout.window hash:{}", PRESET_BROWSER)),
    );
    assert!(a.preset_browser.window_open && !a.palettes.hidden);
}

#[test]
fn the_old_floating_panels_file_becomes_open_palettes_once() {
    let root = data_root();
    std::fs::write(
        root.join("panel-layout.json"),
        r#"{ "version": 1, "floating": [["Items", [333.0, 444.0]]] }"#,
    )
    .unwrap();
    let (a, _ctx) = app(&root);
    assert!(a.is_floating(Panel::Items));
    assert_eq!(a.palettes.place("items").size, [333., 444.]);
    assert!(root.join("palettes.json").is_file());
}

#[test]
fn a_users_palette_file_sits_over_the_shipped_one() {
    let root = data_root();
    std::fs::create_dir_all(root.join("Palettes")).unwrap();
    std::fs::write(
        root.join("Palettes").join("groups.json"),
        r#"{ "version": 1, "name": "My Groups", "order": 4,
             "palette": { "hash": "groups", "size": [500, 500] },
             "frame": { "type": "panel", "kind": "Groups" } }"#,
    )
    .unwrap();
    let (a, _ctx) = app(&root);
    let p = a.palettes.find("groups").unwrap();
    assert_eq!((p.name.as_str(), p.size), ("My Groups", [500., 500.]));
    assert_eq!(
        a.palettes
            .list
            .iter()
            .filter(|p| p.hash == "groups")
            .count(),
        1
    );
}

/// LAY-Q2 (answered): a torn-off view belongs to the app. Switching never
/// touches it; switching back to the layout it came from opens that
/// layout's saved version, so its slot gets a fresh view with the camera
/// it had when torn off; closing the floating window is permanent.
#[test]
fn lay_q2_a_torn_off_view_stays_and_its_layout_gets_a_fresh_view_back() {
    let root = data_root();
    crate::layouts_tests::install_legacy_model(&root);
    let (mut a, ctx) = app(&root);
    a.split_view();
    frames(&mut a, &ctx, 2);
    let views_before = a.views.group_views(None).len();
    assert_eq!(views_before, 2);
    let torn = a.active_view;
    a.camera.distance = 7.25;
    frames(&mut a, &ctx, 2);
    a.tear_off_view(torn);
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.windows.len(), 1);
    assert_eq!(a.views.group_views(None).len(), 1);
    let window = a.views.windows[0].id;
    a.views.windows[0].position = Some([90., 91.]);
    let size = a.views.windows[0].size;
    let key = a.views.windows[0]
        .tree
        .tiles
        .iter()
        .find_map(|(_, t)| match t {
            egui_tiles::Tile::Pane(p) => p.view_key(),
            _ => None,
        })
        .unwrap();
    assert_eq!(key, torn);
    let display = a.displays.get(&key).cloned().map(|d| d.settings);
    // Another layout: the floating view is untouched.
    a.execute("layout.restore UV");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.windows.len(), 1);
    let w = &a.views.windows[0];
    assert_eq!((w.size, w.position), (size, Some([90., 91.])));
    let key = w
        .tree
        .tiles
        .iter()
        .find_map(|(_, t)| match t {
            egui_tiles::Tile::Pane(p) => p.view_key(),
            _ => None,
        })
        .unwrap();
    assert_eq!(a.cameras[&key].distance, 7.25, "its camera");
    assert_eq!(
        a.displays.get(&key).map(|d| d.settings),
        display,
        "its shading"
    );
    let _ = window;
    // Back to Model: its saved version, so two views in the main window
    // again, and the floating one too.
    a.execute("layout.restore Model");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.group_views(None).len(), views_before);
    assert_eq!(a.views.windows.len(), 1, "not docked back");
    let fresh: Vec<u32> = a.views.group_views(None);
    assert!(
        fresh
            .iter()
            .any(|k| a.cameras.get(k).is_some_and(|c| c.distance == 7.25)
                || (*k == a.active_view && a.camera.distance == 7.25)),
        "the slot's fresh view has the torn view's camera"
    );
    // Closing it is permanent: no switch brings it back.
    let id = a.views.windows[0].id;
    a.discard_view_window(id);
    for layout in ["UV", "Model"] {
        a.execute(&format!("layout.restore {layout}"));
        frames(&mut a, &ctx, 2);
        assert!(a.views.windows.is_empty(), "{layout}");
    }
    assert_eq!(a.views.group_views(None).len(), views_before);
}

#[test]
fn floating_views_reopen_after_a_restart_and_leave_the_layout_file() {
    let root = data_root();
    crate::layouts_tests::install_legacy_model(&root);
    let camera = Camera {
        view: camera::View::Front,
        target: glam::DVec3::new(1., 2., 3.),
        distance: 4.,
        ..Camera::default()
    };
    {
        let (mut a, ctx) = app(&root);
        a.split_view();
        frames(&mut a, &ctx, 2);
        let view = a.active_view;
        a.tear_off_view(view);
        a.activate_view(view);
        a.camera = camera.clone();
        frames(&mut a, &ctx, 2);
        assert_eq!(a.views.windows.len(), 1);
        assert!(a.layouts.find("Model").unwrap().doc().windows.is_empty());
    }
    let (a, _ctx) = app(&root);
    let [window] = a.views.windows.as_slice() else {
        panic!("expected one reopened floating window");
    };
    let [view]: [u32; 1] = a.views.group_views(Some(window.id)).try_into().unwrap();
    assert_eq!(a.cameras[&view], camera);
}
