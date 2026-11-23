//! Layouts stage 1 (specifications/layouts-2026-11-16, layouts.rs).
use super::*;
use crate::layouts::{LEGACY_MODEL, Layouts, SHIPPED};
use crate::panel_registry::PanelKind;
use crate::tests::{box_app, draw};
use operandi_layout::{Pane, WindowLayout};
use std::path::Path;

pub(super) const SIZE: Vec2 = vec2(1440., 900.);

pub(super) fn frames(a: &mut Operandi, ctx: &Context, n: usize) {
    for _ in 0..n {
        draw(a, ctx, SIZE, vec![]);
        a.persist_layout(ctx);
    }
}

/// Layout > Layouts (stage 2 adds New, Save and the rest beside it).
fn layouts_submenu(layout: &app_menu::Menu) -> &app_menu::Menu {
    layout
        .entries
        .iter()
        .find_map(|e| match e {
            app_menu::Entry::Submenu(m) if m.title == "Layouts" => Some(m),
            _ => None,
        })
        .expect("Layout > Layouts")
}

fn width(ctx: &Context, id: &str) -> f32 {
    egui::containers::panel::PanelState::load(ctx, Id::new(id))
        .expect("the panel drew")
        .rect
        .width()
}

/// A fresh data folder for an app that reopens it later.
pub(super) fn data_root() -> PathBuf {
    let mut app = Operandi::empty();
    // This folder outlives the app that made it (apps reopen it later), so
    // it isn't removed with it.
    std::mem::forget(app.environment.scratch.take());
    let root = app.environment.paths().data_root().to_path_buf();
    // `empty` made it; start clean.
    for entry in std::fs::read_dir(&root).unwrap().flatten() {
        let _ = std::fs::remove_dir_all(entry.path());
        let _ = std::fs::remove_file(entry.path());
    }
    root
}

/// An app on `root` launched the way main() launches it (layout part).
pub(super) fn launch(root: &Path, ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty_at(root.to_path_buf());
    a.run(Command::NewMesh);
    a.start_layouts();
    a.finish_layout_migration(ctx);
    a
}

/// Keep tests of the old column controls on the layout that used them.
pub(super) fn install_legacy_model(root: &Path) {
    let folder = root.join("Layouts");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("Model.json"), LEGACY_MODEL).unwrap();
}

/// The old files of a user who split the view, made one view wireframe,
/// widened both side panels and changed the tabs.
fn write_old_files(root: &Path) -> operandi_layout::Layout {
    let mut views = operandi_layout::Layout::single_viewport();
    views.add_viewport();
    std::fs::write(root.join("view-layout.json"), views.to_json().unwrap()).unwrap();
    let mut wire = view_display::Settings::default();
    wire.active.style = view_display::Style::Wireframe;
    let displays: BTreeMap<u32, view_display::Settings> = [(1, wire)].into();
    std::fs::write(
        root.join("view-displays.json"),
        serde_json::to_string(&displays).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("layout.json"),
        serde_json::json!({
            "version": 1,
            "split": 0.4,
            "tool_split": 0.3,
            "left": 300.,
            "right": 350.,
            "panel_groups": {
                "upper": {"tabs": ["items", "groups"], "active": 1},
                "lower": {"tabs": ["properties", "shader_tree"], "active": 0}
            }
        })
        .to_string(),
    )
    .unwrap();
    views
}

#[test]
fn migration_turns_the_old_files_into_model_unchanged() {
    let root = data_root();
    let old_views = write_old_files(&root);
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 3);
    // What the old loaders made of the same files (the build before).
    let old_ctx = Context::default();
    theme::install(&old_ctx);
    let mut b = Operandi::empty_at(root.clone());
    b.run(Command::NewMesh);
    b.load_views();
    b.load_layout(&old_ctx);
    for _ in 0..3 {
        draw(&mut b, &old_ctx, SIZE, vec![]);
    }
    assert_eq!(a.layouts.current, "Model");
    assert_eq!(a.views.panes(), old_views.panes());
    assert_eq!(a.views.panes(), b.views.panes());
    assert_eq!(a.view_rects, b.view_rects, "same view sizes");
    assert_eq!(width(&ctx, "tools"), width(&old_ctx, "tools"));
    assert_eq!(width(&ctx, "tools"), 300.);
    assert_eq!(width(&ctx, "scene"), 350.);
    assert_eq!(a.scene_fraction, b.scene_fraction);
    assert_eq!(a.tool_split, 0.3);
    assert_eq!(a.upper_frame.tabs, [PanelKind::Items, PanelKind::Groups]);
    assert_eq!(a.upper_frame.active, 1);
    assert_eq!(
        a.lower_frame.tabs,
        [PanelKind::Properties, PanelKind::ShaderTree]
    );
    assert_eq!(
        a.displays[&1].settings.active.style,
        view_display::Style::Wireframe
    );
    assert_eq!(a.category, toolbox::Category::Create);

    // The migrated Model differs from the shipped one, so it is kept...
    let file = root.join("Layouts/Model.json");
    let saved = WindowLayout::from_json(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(saved.name, "Model");
    // ...and the next launch opens it from there, the same.
    let ctx2 = Context::default();
    let mut c = launch(&root, &ctx2);
    frames(&mut c, &ctx2, 3);
    assert_eq!(c.views.panes(), old_views.panes());
    assert_eq!(c.view_rects, a.view_rects);
    assert!((width(&ctx2, "tools") - 300.).abs() < 0.01);
    assert!((width(&ctx2, "scene") - 350.).abs() < 0.01);
    assert_eq!(c.upper_frame.tabs, a.upper_frame.tabs);
    assert_eq!(c.upper_frame.active, 1);
    assert_eq!(c.lower_frame.tabs, a.lower_frame.tabs);
    assert!((c.scene_fraction - a.scene_fraction).abs() < 1e-4);
    assert!((c.tool_split - 0.3).abs() < 1e-4);
    assert_eq!(
        c.displays[&1].settings.active.style,
        view_display::Style::Wireframe
    );
    // The old files are read once: changing them no longer matters.
    std::fs::write(root.join("layout.json"), r#"{"version":1,"left":420}"#).unwrap();
    let ctx3 = Context::default();
    let mut d = launch(&root, &ctx3);
    frames(&mut d, &ctx3, 2);
    assert!((width(&ctx3, "tools") - 300.).abs() < 0.01);
}

#[test]
fn a_fresh_install_opens_model_and_writes_no_copy_until_it_changes() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 3);
    assert_eq!(a.layouts.current, "Model");
    assert_eq!(a.views.viewports(), [0]);
    assert_eq!(a.layouts.tool_tabs_horizontal, Some(true));
    assert_eq!(
        a.upper_frame.tabs,
        [
            PanelKind::Items,
            PanelKind::ShaderTree,
            PanelKind::Groups,
            PanelKind::MeshOps
        ]
    );
    assert_eq!(a.upper_frame.front(), Some(PanelKind::Items));
    assert_eq!(a.lower_frame.tabs, [PanelKind::Properties]);
    let tabs = a.dots.rects[&panel_dot::Dot::ToolTabs];
    let toolbox = a.dots.rects[&panel_dot::Dot::Side(floating::Panel::Toolbox)];
    let tool_properties = a.dots.rects[&panel_dot::Dot::ToolProperties];
    let items = a.dots.rects[&panel_dot::Dot::Side(floating::Panel::Items)];
    let properties = a.dots.rects[&panel_dot::Dot::Side(floating::Panel::Properties)];
    let viewport = a.view_rects[&0];
    assert!(tabs.bottom() < toolbox.top().min(viewport.top()).min(items.top()));
    assert!(toolbox.bottom() < tool_properties.top());
    assert!(toolbox.right() < viewport.left());
    assert!(viewport.right() < items.left());
    assert!(items.bottom() < properties.top());
    let create = a.toolbox_rects["Category Create"];
    let select = a.toolbox_rects["Category Select"];
    assert_eq!(create.top(), select.top());
    assert!(create.right() < select.left());
    let default = a.capture_layout().unwrap();
    assert_eq!(
        default.frame.slots().len(),
        9,
        "all dock leaves, no timeline"
    );
    // LAY-41: no user copy while it matches the shipped one.
    assert!(!root.join("Layouts/Model.json").exists());
    // LAY-40: a tab change is kept without saving.
    a.upper_frame.show_page(2);
    frames(&mut a, &ctx, 1);
    assert!(root.join("Layouts/Model.json").exists());
    a.upper_frame.show_page(0);
    frames(&mut a, &ctx, 1);
    assert!(!root.join("Layouts/Model.json").exists());
}

#[test]
fn shipped_layouts_are_files_model_and_uv() {
    let names: Vec<&str> = SHIPPED.iter().map(|(file, _)| *file).collect();
    assert!(
        names.contains(&"model.json") && names.contains(&"uv.json"),
        "{names:?}"
    );
    let layouts = Layouts::read(SHIPPED, None);
    assert_eq!(layouts.names(), ["Model", "UV", "Animate"]);
    for entry in &layouts.list {
        let doc = entry.doc();
        assert!(doc.starred, "LAY-21: {} is starred", doc.name);
        assert!(doc.open_command.is_some());
        assert!(doc.key_context.is_none(), "LAY-20 is reserved");
    }
}

#[test]
fn uvt201_the_uv_file_gives_a_uv_and_a_perspective_view_and_the_uv_tab() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.camera.view = camera::View::Top;
    frames(&mut a, &ctx, 2);
    a.execute("layout.restore UV");
    assert_eq!(a.layouts.current, "UV", "{}", a.status);
    frames(&mut a, &ctx, 2);
    let panes = a.views.panes();
    let [Pane::Uv(uv), Pane::Viewport(persp)] = panes.as_slice() else {
        panic!("{panes:?}");
    };
    let (ru, rp) = (a.view_rects[uv], a.view_rects[persp]);
    assert!(ru.center().x < rp.center().x, "UV on the left");
    assert!((ru.width() - rp.width()).abs() < 4., "50/50: {ru:?} {rp:?}");
    assert_eq!(a.cameras[persp].view, camera::View::Perspective);
    assert_eq!(a.category, toolbox::Category::Uv);
    assert!(a.layouts.uv_tab);
    assert_eq!(a.upper_frame.front(), Some(PanelKind::UvMaps));
    assert!(a.toolbox_rects.contains_key("Category UV surface"));
}

#[test]
fn lay33_switching_back_restores_each_layout_as_left() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    frames(&mut a, &ctx, 2);
    // Model: split, turn the camera, change a tab and the dividers.
    a.split_view();
    a.camera.yaw = 1.25;
    a.category = toolbox::Category::Polygon;
    a.upper_frame.show_page(2);
    a.scene_fraction = 0.3;
    frames(&mut a, &ctx, 2);
    let model_views = a.views.panes();
    let model_active = a.active_view;
    let model_left = width(&ctx, "tools");
    let scene = a.editor.scene.clone();
    let undo = a
        .editor
        .history_eligibility(HistoryAction::Undo)
        .label
        .map(|l| l.to_string());

    a.execute("layout.restore UV");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.layouts.current, "UV");
    // UV: turn its 3D view, open another toolbox tab.
    let uv_views = a.views.panes();
    let Some(Pane::Viewport(persp)) = uv_views.get(1).copied() else {
        panic!("{uv_views:?}");
    };
    a.activate_view(persp);
    a.camera.pitch = 0.9;
    a.category = toolbox::Category::Edge;
    frames(&mut a, &ctx, 2);
    // LAY-02: no scene, undo or selection change from switching.
    assert_eq!(a.editor.scene, scene);

    a.execute("layout.restore Model");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.panes(), model_views);
    assert_eq!(a.active_view, model_active);
    assert!(
        (a.camera.yaw - 1.25).abs() < 1e-9,
        "the camera angle comes back"
    );
    assert_eq!(a.category, toolbox::Category::Polygon);
    assert_eq!(a.upper_frame.active, 2);
    assert_eq!(a.scene_fraction, 0.3);
    assert!((width(&ctx, "tools") - model_left).abs() < 0.01);
    assert!(!a.layouts.uv_tab);
    assert_eq!(a.editor.scene, scene);
    assert_eq!(
        a.editor
            .history_eligibility(HistoryAction::Undo)
            .label
            .map(|l| l.to_string()),
        undo
    );

    a.execute("layout.restore UV");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.panes(), uv_views);
    assert_eq!(a.active_view, persp);
    assert!((a.camera.pitch - 0.9).abs() < 1e-9);
    assert_eq!(
        a.category,
        toolbox::Category::Edge,
        "the open command doesn't rerun"
    );
    assert!(a.layouts.uv_tab);
    // The two layouts' views never share a key.
    assert!(uv_views.iter().all(|p| !model_views.contains(p)));
}

#[test]
fn every_way_in_runs_layout_restore() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    frames(&mut a, &ctx, 2);
    // Layout > Layouts lists every layout, the current one ticked.
    let menus = a.menu_model(&ctx);
    let layout = menus
        .iter()
        .find(|m| m.title == "Layout")
        .expect("a Layout menu");
    let leaves = layouts_submenu(layout).leaves();
    assert_eq!(leaves.len(), 3);
    a.menu_action(
        &ctx,
        app_menu::line_action(&Operandi::restore_line(&a.layouts.names()[1])),
    );
    assert_eq!(a.layouts.current, "UV");
    // Window > Layouts uses the same live list and command actions.
    let menus = a.menu_model(&ctx);
    let window = menus.iter().find(|m| m.title == "Window").unwrap();
    let window_layouts = layouts_submenu(window).leaves();
    let layout = menus.iter().find(|m| m.title == "Layout").unwrap();
    assert!(app_menu::same_shape(
        std::slice::from_ref(layouts_submenu(layout)),
        std::slice::from_ref(layouts_submenu(window)),
    ));
    assert!(window_layouts[1].checked);
    a.menu_action(&ctx, window_layouts[0].action);
    assert_eq!(a.layouts.current, "Model", "{}", a.status);
    // A name that isn't a layout only reports it.
    a.execute("layout.restore Nowhere");
    assert_eq!(a.layouts.current, "Model");
    assert!(a.status.contains("No layout named"), "{}", a.status);
    // A tool open: switching waits.
    a.activate(Tool::Move);
    a.execute("layout.restore UV");
    assert_eq!(a.layouts.current, "Model");
}

/// LAY-70 / UIC-190: a third layout is a file and nothing else.
const THIRD: &str = r#"{
  "version": 1,
  "name": "Third",
  "order": 5,
  "open_command": "operandi.toolbox.tab sculpt",
  "frame": {
    "type": "split", "dir": "row",
    "children": [
      {"fraction": 0.2, "frame": {"type": "split", "dir": "column", "children": [
        {"fraction": 0.7, "frame": {"type": "panel", "kind": "Toolbox"}},
        {"fraction": 0.3, "frame": {"type": "panel", "kind": "Tool Properties"}}]}},
      {"fraction": 0.6, "frame": {"type": "split", "dir": "column", "children": [
        {"fraction": 0.5, "frame": {"type": "panel", "kind": "3D View"}},
        {"fraction": 0.5, "frame": {"type": "panel", "kind": "3D View"}}]}},
      {"fraction": 0.2, "frame": {"type": "split", "dir": "column", "children": [
        {"fraction": 0.5, "frame": {"type": "tabs", "tabs": [{"kind": "Items"}, {"kind": "Groups"}]}},
        {"fraction": 0.5, "frame": {"type": "tabs", "tabs": [{"kind": "Properties"}]}}]}}
    ]
  }
}"#;

#[test]
fn lay70_a_third_layout_is_added_by_adding_a_file() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    // The shipped list plus one more file, read by the same code.
    let mut files: Vec<(&str, &str)> = SHIPPED.to_vec();
    files.push(("third.json", THIRD));
    a.layouts = Layouts::read(&files, None);
    frames(&mut a, &ctx, 2);
    assert_eq!(a.layouts.names(), ["Model", "UV", "Animate", "Third"]);
    assert!(
        a.layouts_pie()
            .entries
            .iter()
            .flatten()
            .any(|entry| entry.command == "layout.restore Third"),
        "a pie entry for it"
    );
    let menus = a.menu_model(&ctx);
    let layout = menus.iter().find(|m| m.title == "Layout").unwrap();
    assert_eq!(
        layouts_submenu(layout).leaves().len(),
        4,
        "and a menu entry"
    );
    a.execute("layout.restore Third");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.layouts.current, "Third", "{}", a.status);
    assert_eq!(a.views.viewports().len(), 2);
    let r: Vec<Rect> = a
        .views
        .viewports()
        .iter()
        .map(|v| a.view_rects[v])
        .collect();
    assert!(r[0].center().y < r[1].center().y, "stacked");
    assert_eq!(a.category, toolbox::Category::Sculpt);
    assert_eq!(a.upper_frame.tabs, [PanelKind::Items, PanelKind::Groups]);
    assert!((width(&ctx, "tools") - 288.).abs() < 0.01);
    assert!((a.tool_split - 0.3).abs() < 1e-6);
    // A user's file in the Layouts folder is offered too (copied from
    // another computer, LAY-75).
    let root = data_root();
    std::fs::create_dir_all(root.join("Layouts")).unwrap();
    std::fs::write(root.join("Layouts/Third.json"), THIRD).unwrap();
    let ctx2 = Context::default();
    let b = launch(&root, &ctx2);
    assert_eq!(b.layouts.names(), ["Model", "UV", "Animate", "Third"]);
}

#[test]
fn lay49_a_panel_kind_this_build_lacks_is_an_empty_named_panel() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    let text = THIRD
        .replace(r#"{"kind": "Groups"}"#, r#"{"kind": "Schematic"}"#)
        .replace(
            r#"{"fraction": 0.5, "frame": {"type": "panel", "kind": "3D View"}}]"#,
            r#"{"fraction": 0.5, "frame": {"type": "panel", "kind": "Timeline"}}]"#,
        );
    let mut files: Vec<(&str, &str)> = SHIPPED.to_vec();
    files.push(("third.json", &text));
    a.layouts = Layouts::read(&files, None);
    frames(&mut a, &ctx, 1);
    a.execute("layout.restore Third");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.layouts.current, "Third", "{}", a.status);
    // The rest of the layout is there.
    assert_eq!(a.views.viewports().len(), 1);
    assert_eq!(a.upper_frame.tabs.len(), 2);
    assert_eq!(a.upper_frame.tabs[0], PanelKind::Items);
    assert_eq!(a.upper_frame.name(a.upper_frame.tabs[1]), "Schematic");
    let missing = a
        .views
        .panes()
        .into_iter()
        .find(|p| matches!(p, Pane::Missing(_)))
        .expect("an empty view-area panel");
    assert_eq!(a.views.title_of(missing), "Timeline");
    // Showing the empty tab draws nothing else and breaks nothing.
    a.upper_frame.show_page(1);
    frames(&mut a, &ctx, 2);
    // Saved back with its names, for a build that has them.
    let doc = a.capture_layout().unwrap();
    let kinds: Vec<&str> = doc.frame.slots().iter().map(|s| s.kind.as_str()).collect();
    assert!(
        kinds.contains(&"Schematic") && kinds.contains(&"Timeline"),
        "{kinds:?}"
    );
    a.execute("layout.restore Model");
    assert_eq!(a.layouts.current, "Model");
}

#[test]
fn lay49_an_unknown_file_version_falls_back_to_the_shipped_layout() {
    let root = data_root();
    std::fs::create_dir_all(root.join("Layouts")).unwrap();
    let uv = SHIPPED.iter().find(|(f, _)| *f == "uv.json").unwrap().1;
    let newer = uv
        .replace("\"version\": 1", "\"version\": 7")
        .replace("\"active\": 4", "\"active\": 0");
    let uv_file = root.join("Layouts/UV.json");
    std::fs::write(&uv_file, &newer).unwrap();
    // One that isn't the three-column shape this build draws.
    let other = uv.replace("\"Toolbox\"", "\"Timeline\"");
    std::fs::write(
        root.join("Layouts/Model.json"),
        other.replace("\"UV\"", "\"Model\""),
    )
    .unwrap();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    assert!(a.layouts.find("UV").unwrap().user.is_none(), "ignored");
    a.execute("layout.restore UV");
    assert_eq!(
        a.upper_frame.front(),
        Some(PanelKind::UvMaps),
        "the shipped UV"
    );
    frames(&mut a, &ctx, 2);
    assert_eq!(std::fs::read_to_string(uv_file).unwrap(), newer);
    // The misshapen Model copy is refused; the shipped Model opens.
    a.execute("layout.restore Model");
    assert_eq!(a.layouts.current, "Model");
    assert_eq!(a.views.viewports().len(), 1);
}

#[test]
fn lay22_restart_opens_the_layout_open_at_quit() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    a.execute("layout.restore UV");
    frames(&mut a, &ctx, 2);
    // The UV layout untouched: still no copy of it, only the state file.
    assert!(!root.join("Layouts/UV.json").exists());
    let ctx2 = Context::default();
    theme::install(&ctx2);
    let mut b = Operandi::empty_at(root.clone());
    b.run(Command::NewMesh);
    let status = b.status.clone();
    b.start_layouts();
    assert_eq!(b.status, status, "a quiet start");
    b.finish_layout_migration(&ctx2);
    frames(&mut b, &ctx2, 2);
    assert_eq!(b.layouts.current, "UV");
    assert!(matches!(
        b.views.panes().as_slice(),
        [Pane::Uv(_), Pane::Viewport(_)]
    ));
    assert_eq!(b.category, toolbox::Category::Uv);
}

#[test]
fn lay40_view_cameras_survive_restart_and_layout_switching() {
    let root = data_root();
    let ctx = Context::default();
    let mut app = launch(&root, &ctx);
    let perspective = Camera {
        target: glam::DVec3::new(1.25, -2.5, 0.5),
        distance: 0.02,
        yaw: 1.25,
        pitch: -0.375,
        roll: 0.125,
        // An Alt+Shift+wheel field of view is the view's own and stays.
        lens: 1.125,
        near_depth: 0.004,
        near_distance: 0.02,
        ..Camera::default()
    };
    let top = Camera {
        view: camera::View::Top,
        target: glam::DVec3::new(-3., 0., 2.),
        distance: 25.,
        ..Camera::default()
    };
    let front = Camera {
        view: camera::View::Front,
        distance: 14.,
        ..Camera::default()
    };
    app.split_view();
    let [left, right]: [u32; 2] = app.views.viewports().try_into().unwrap();
    app.activate_view(left);
    app.camera = perspective.clone();
    app.display.settings.active.style = view_display::Style::Wireframe;
    app.activate_view(right);
    app.camera = top.clone();

    app.execute("layout.restore UV");
    let [Pane::Uv(uv), Pane::Viewport(view)]: [Pane; 2] = app.views.panes().try_into().unwrap()
    else {
        panic!("expected a UV and a 3D view");
    };
    assert_eq!(app.cameras[&view], Camera::default());
    let center = glam::DVec2::new(-0.25, 1.5);
    let zoom = 512.;
    let uv_view = app.uv_views.get_mut(&uv).unwrap();
    uv_view.center = center;
    uv_view.zoom = zoom;
    app.activate_view(view);
    app.camera = front.clone();

    app.execute("layout.restore Model");
    assert_eq!(app.active_view, right);
    assert_eq!(app.cameras[&left], perspective);
    assert_eq!(app.camera, top);
    app.execute("layout.restore UV");
    assert_eq!(app.camera, front);
    assert_eq!(
        (app.uv_views[&uv].center, app.uv_views[&uv].zoom),
        (center, zoom)
    );
    // This last change is saved by the normal frame without a switch.
    let front = Camera {
        target: glam::DVec3::new(2., 3., 4.),
        ..front
    };
    app.camera = front.clone();
    frames(&mut app, &ctx, 1);
    drop(app);

    let ctx = Context::default();
    let mut reopened = launch(&root, &ctx);
    assert_eq!(reopened.layouts.current, "UV");
    let [Pane::Uv(uv), Pane::Viewport(view)]: [Pane; 2] =
        reopened.views.panes().try_into().unwrap()
    else {
        panic!("expected a UV and a 3D view after restart");
    };
    assert_eq!(
        (reopened.uv_views[&uv].center, reopened.uv_views[&uv].zoom),
        (center, zoom)
    );
    reopened.activate_view(view);
    assert_eq!(reopened.camera, front);

    reopened.execute("layout.restore Model");
    let [left, right]: [u32; 2] = reopened.views.viewports().try_into().unwrap();
    reopened.activate_view(left);
    assert_eq!(reopened.camera, perspective);
    assert_eq!(
        reopened.display.settings.active.style,
        view_display::Style::Wireframe
    );
    reopened.activate_view(right);
    assert_eq!(reopened.camera, top);
}

/// A layout saved by an older build whose Maximise had scaled a
/// Perspective view's lens (Sylvain's stretched view) opens at the normal
/// lens, set back so the orbit centre keeps its size on screen.
#[test]
fn a_saved_wide_lens_opens_at_the_normal_lens() {
    let slot: operandi_layout::Slot = serde_json::from_value(serde_json::json!({
        "kind": "view3d",
        "settings": {"camera": {
            "view": "perspective",
            "target": [1., 2., 3.],
            "distance": 10.,
            "yaw": 0.5,
            "pitch": 0.25,
            "roll": 0.,
            "lens": camera::DEFAULT_LENS / 2.,
        }},
    }))
    .unwrap();
    let c = crate::layouts::camera_from_slot(&slot);
    assert_eq!(c.lens, camera::DEFAULT_LENS);
    assert!((c.distance - 20.).abs() < 1e-9, "{}", c.distance);
    assert_eq!(c.target, glam::DVec3::new(1., 2., 3.));
}

#[test]
fn legacy_layout_without_camera_settings_uses_default_cameras() {
    let root = data_root();
    install_legacy_model(&root);
    let ctx = Context::default();
    let mut app = launch(&root, &ctx);
    assert_eq!(app.camera, Camera::default());

    // An older layout also opens at its own defaults when the current
    // layout has already been panned, orbited and zoomed.
    app.camera.target = glam::DVec3::ONE;
    app.camera.yaw = 1.25;
    app.camera.distance = 14.;
    app.execute("layout.restore UV");
    let [Pane::Uv(uv), Pane::Viewport(view)]: [Pane; 2] = app.views.panes().try_into().unwrap()
    else {
        panic!("expected a UV and a 3D view");
    };
    assert_eq!(app.cameras[&view], Camera::default());
    let default = uv_view::UvView::default();
    assert_eq!(
        (app.uv_views[&uv].center, app.uv_views[&uv].zoom),
        (default.center, default.zoom)
    );
}

#[test]
fn uic221_an_unknown_open_command_only_reports_it() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    let text = THIRD.replace("operandi.toolbox.tab sculpt", "no.such.command now");
    let mut files: Vec<(&str, &str)> = SHIPPED.to_vec();
    files.push(("third.json", &text));
    a.layouts = Layouts::read(&files, None);
    frames(&mut a, &ctx, 1);
    let scene = a.editor.scene.clone();
    a.execute("layout.restore Third");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.layouts.current, "Third");
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.category, toolbox::Category::Create);
}

#[test]
fn uic15_reset_puts_a_changed_layout_back_and_drops_its_copy() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    a.execute("layout.restore UV");
    frames(&mut a, &ctx, 1);
    a.split_view();
    frames(&mut a, &ctx, 1);
    assert_eq!(a.views.panes().len(), 3);
    assert!(root.join("Layouts/UV.json").exists());
    a.reset_layout("UV");
    frames(&mut a, &ctx, 1);
    assert_eq!(a.views.panes().len(), 2);
    assert!(!root.join("Layouts/UV.json").exists());
}

/// The "looks identical" check in-process: the whole window drawn from
/// the old files by the old loaders (the build before) and by the
/// migration paints the same shapes, at the same places, in the same
/// order.
#[test]
fn migrated_model_paints_exactly_what_the_old_loaders_paint() {
    let root = data_root();
    write_old_files(&root);
    let shapes = |a: &mut Operandi, ctx: &Context| {
        let mut out = None;
        for _ in 0..4 {
            out = Some(draw(a, ctx, SIZE, vec![]));
        }
        let out = out.unwrap();
        out.shapes
            .iter()
            .map(|s| format!("{:?} {:?}", s.clip_rect, s.shape))
            .collect::<Vec<_>>()
    };
    let ctx = Context::default();
    let mut new = launch(&root, &ctx);
    let new_shapes = shapes(&mut new, &ctx);
    let old_ctx = Context::default();
    theme::install(&old_ctx);
    let mut old = Operandi::empty_at(root.clone());
    old.run(Command::NewMesh);
    old.load_views();
    old.load_layout(&old_ctx);
    let old_shapes = shapes(&mut old, &old_ctx);
    let differ: Vec<(usize, &String, &String)> = new_shapes
        .iter()
        .zip(&old_shapes)
        .enumerate()
        .filter(|(_, (n, o))| n != o)
        .map(|(i, (n, o))| (i, n, o))
        .take(5)
        .collect();
    assert_eq!(new_shapes.len(), old_shapes.len());
    assert!(differ.is_empty(), "{differ:#?}");
}

/// The "looks identical" check against an earlier build (opt-in): write
/// every shape Model paints at rest, one per line, to the file
/// `OPERANDI_PAINT_DUMP` names. Run it on both builds and compare.
#[test]
#[ignore]
fn paint_dump_model_at_rest() {
    let Ok(path) = std::env::var("OPERANDI_PAINT_DUMP") else {
        return;
    };
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    let mut out = None;
    for _ in 0..4 {
        out = Some(draw(&mut a, &ctx, SIZE, vec![]));
    }
    let text: String = out
        .unwrap()
        .shapes
        .iter()
        .map(|s| format!("{:?} {:?}\n", s.clip_rect, s.shape))
        .collect();
    std::fs::write(path, text).unwrap();
}

#[test]
fn dock_tree_migrates_each_legacy_shipped_layout_losslessly() {
    use operandi_layout::Frame;

    let files = [
        ("model.json", LEGACY_MODEL),
        ("uv.json", include_str!("../assets/layouts/uv.json")),
        (
            "paint.json",
            include_str!("../assets/layouts-painting/paint.json"),
        ),
    ];
    for (file, source) in files {
        let legacy = WindowLayout::from_json(source).unwrap();
        let layouts = Layouts::read(&[(file, source)], None);
        let tree = layouts.list[0].doc();
        assert_eq!(*tree, WindowLayout::from_json(&tree.to_json()).unwrap());

        let mut back = tree.clone();
        let Frame::Split { children, .. } = &mut back.frame else {
            panic!("{file}: expected a dock row");
        };
        let Frame::ViewArea { frame } = &children[1].frame else {
            panic!("{file}: expected a view-area leaf");
        };
        children[1].frame = (**frame).clone();
        assert_eq!(back, legacy, "{file}");
    }
}

#[test]
fn dock_tree_round_trips_a_non_column_arrangement() {
    use crate::layouts::dock::views_mut;
    use operandi_layout::{Dir, FloatWindow, Frame, Part, Slot};

    let mut doc = WindowLayout::from_json(THIRD).unwrap();
    let Frame::Split { children, .. } = &mut doc.frame else {
        panic!("expected the legacy dock row");
    };
    let view_frame = children[1].frame.clone();
    children[1].frame = Frame::Split {
        dir: Dir::Column,
        locked: true,
        children: vec![
            Part {
                fraction: 0.8,
                frame: Frame::ViewArea {
                    frame: Box::new(view_frame.clone()),
                },
            },
            Part {
                fraction: 0.2,
                frame: Frame::Tabs {
                    tabs: vec![Slot::new("Shader Tree")],
                    active: 0,
                    minimized: false,
                },
            },
        ],
    };
    doc.windows.push(FloatWindow {
        frame: Frame::Panel(Slot::new(operandi_layout::UV_VIEW)),
        size: [360., 240.],
        position: Some([48., 96.]),
    });
    let json = doc.to_json();
    let layouts = Layouts::read(&[("nested.json", &json)], None);
    let mut loaded = layouts.list[0].doc().clone();
    assert_eq!(loaded, doc);
    let views = views_mut(&mut loaded.frame).unwrap();
    assert_eq!(*views, view_frame);
    *views = Frame::Panel(Slot::new(operandi_layout::EMPTY));
    assert_eq!(WindowLayout::from_json(&loaded.to_json()).unwrap(), loaded);
}

#[test]
fn dock_tree_reopens_a_legacy_user_file_and_saves_tab_edits() {
    use operandi_layout::Frame;

    let root = data_root();
    let ctx = Context::default();
    let file = root.join("Layouts/Third.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, THIRD).unwrap();
    let mut app = launch(&root, &ctx);
    app.execute("layout.restore Third");
    frames(&mut app, &ctx, 2);
    assert_eq!(app.views.panes().len(), 2);
    assert_eq!(width(&ctx, "tools"), 288.);
    assert_eq!(app.upper_frame.tabs, [PanelKind::Items, PanelKind::Groups]);
    app.upper_frame.show_page(1);
    frames(&mut app, &ctx, 1);
    let saved = WindowLayout::from_json(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let Frame::Split { children, .. } = saved.frame else {
        panic!("expected a dock row");
    };
    assert!(matches!(children[1].frame, Frame::ViewArea { .. }));

    let ctx = Context::default();
    let mut reopened = launch(&root, &ctx);
    frames(&mut reopened, &ctx, 2);
    assert_eq!(reopened.layouts.current, "Third");
    assert_eq!(reopened.upper_frame.front(), Some(PanelKind::Groups));
    let rects = |a: &Operandi| {
        a.views
            .views()
            .iter()
            .map(|key| a.view_rects[key])
            .collect::<Vec<_>>()
    };
    assert_eq!(rects(&reopened), rects(&app));
}

/// The Mesh Stack panel was called "Mesh Ops" until 2026-11-19: saved layouts
/// with the old kind still open it, and layouts are written with the new name.
#[test]
fn old_mesh_ops_kind_opens_the_mesh_stack_panel() {
    use crate::layouts::kind_named;
    assert_eq!(kind_named("Mesh Ops"), Some(PanelKind::MeshOps));
    assert_eq!(kind_named("Mesh Stack"), Some(PanelKind::MeshOps));
    assert_eq!(
        crate::panel_registry::panel_type(PanelKind::MeshOps).name,
        "Mesh Stack"
    );
}

#[test]
fn docking_layout_side_dividers_survive_reopen() {
    let root = data_root();
    install_legacy_model(&root);
    let ctx = Context::default();
    let mut app = launch(&root, &ctx);
    frames(&mut app, &ctx, 3);
    let left = width(&ctx, "tools");
    let site = egui::containers::panel::PanelState::load(&ctx, Id::new("tools"))
        .unwrap()
        .rect;
    let upper = app.scene_divider_rect.center();
    let tools = app.tool_divider_rect.center();
    let right = width(&ctx, "scene");
    let right_edge = egui::containers::panel::PanelState::load(&ctx, Id::new("scene"))
        .unwrap()
        .rect
        .left_center();
    for (from, delta) in [
        (site.right_center(), vec2(48., 0.)),
        (right_edge, vec2(-35., 0.)),
        (upper, vec2(0., 54.)),
        (tools, vec2(0., -45.)),
    ] {
        let to = from + delta;
        draw(
            &mut app,
            &ctx,
            SIZE,
            vec![
                tests::pointer(from, true, Modifiers::NONE),
                Event::PointerMoved(to),
                tests::pointer(to, false, Modifiers::NONE),
            ],
        );
        frames(&mut app, &ctx, 1);
    }
    assert!(width(&ctx, "tools") > left);
    assert!(width(&ctx, "scene") > right);
    assert!(app.scene_fraction > 0.5);
    assert!(app.tool_split > 0.4);
    let saved = app.capture_layout().unwrap();
    let new_ctx = Context::default();
    let mut reopened = launch(&root, &new_ctx);
    frames(&mut reopened, &new_ctx, 3);
    assert!(saved.same_as(&reopened.capture_layout().unwrap()));
    assert_eq!(width(&ctx, "tools"), width(&new_ctx, "tools"));
    assert_eq!(width(&ctx, "scene"), width(&new_ctx, "scene"));
    assert_eq!(app.tool_divider_rect, reopened.tool_divider_rect);
    assert_eq!(app.scene_divider_rect, reopened.scene_divider_rect);
}

#[test]
fn docking_layout_has_no_right_divider_hit_after_a_group_floats() {
    let root = data_root();
    install_legacy_model(&root);
    let ctx = Context::default();
    let mut app = launch(&root, &ctx);
    frames(&mut app, &ctx, 2);
    let old_hit = app.scene_divider_rect.center();
    assert!(app.scene_divider_rect.is_positive());
    app.set_floating(floating::Panel::Items, true);
    frames(&mut app, &ctx, 2);
    assert_eq!(app.scene_divider_rect, Rect::NOTHING);
    draw(
        &mut app,
        &ctx,
        SIZE,
        vec![tests::pointer(old_hit, true, Modifiers::NONE)],
    );
    assert!(!app.layout_owned_input);
    assert!(app.layout_capture.is_none());
}

#[test]
fn docking_layout_save_reopens_the_full_arrangement() {
    use operandi_layout::Frame;

    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    // Change a divider, the order and active tab, and the toolbox set.
    let tool = a.views.main.tiles.find_pane(&Pane::Toolbox).unwrap();
    let left_column = a.views.main.tiles.parent_of(tool).unwrap();
    let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(rows))) =
        a.views.main.tiles.get_mut(left_column)
    else {
        panic!("expected the Toolbox and Tool Properties rows");
    };
    rows.shares.set_share(rows.children[0], 0.7);
    rows.shares.set_share(rows.children[1], 0.3);
    a.upper_frame.reorder(2, 0);
    let groups = a
        .upper_frame
        .tabs
        .iter()
        .position(|k| *k == PanelKind::Groups)
        .unwrap();
    a.upper_frame.show_page(groups);
    a.execute("operandi.toolbox.set uv");
    // Float Tool Tabs and a view in separate windows. Open a named
    // palette as well, as it can coexist with the two dock windows.
    a.split_view();
    frames(&mut a, &ctx, 2);
    a.tear_off_view(a.active_view);
    a.dot_menu_action(panel_dot::Dot::ToolTabs, panel_dot::MenuAction::Detach);
    assert_eq!(a.views.windows.len(), 2);
    for (index, w) in a.views.windows.iter_mut().enumerate() {
        let offset = 20. * index as f32;
        w.position = Some([90. + offset, 110. + offset]);
        w.size = [640. + offset, 390. + offset];
    }
    a.execute("layout.window hash:uvEditor open:true");
    a.execute("operandi.layout.saveAs {Docked and Floating}");
    assert_eq!(a.layouts.current, "Docked and Floating", "{}", a.status);
    a.execute("operandi.layout.save");
    let file = root.join("Layouts/Docked and Floating.json");
    let saved = WindowLayout::from_json(&std::fs::read_to_string(file).unwrap()).unwrap();
    assert!(matches!(saved.frame, Frame::Split { .. }));
    assert_eq!(saved.windows.len(), 2);
    assert_eq!(saved.toolbox.as_ref().unwrap().set, "UV");
    assert_eq!(saved.windows[0].size, [640., 390.]);
    assert_eq!(saved.windows[1].position, Some([110., 130.]));
    assert!(
        saved
            .windows
            .iter()
            .any(|w| w.frame.slots().iter().any(|s| s.kind == "Tool Tabs"))
    );
    assert!(
        saved
            .windows
            .iter()
            .any(|w| w.frame.slots().iter().any(|s| s.kind == "3D View"))
    );

    a.execute("layout.restore UV");
    assert_eq!(a.views.windows.len(), 1, "only the named palette follows");
    assert!(a.palette_open(crate::palettes::UV_EDITOR));
    a.execute("layout.restore {Docked and Floating}");
    assert!(saved.same_as(&a.capture_layout().unwrap()));

    let new_ctx = Context::default();
    let mut reopened = launch(&root, &new_ctx);
    reopened.load_palettes();
    frames(&mut reopened, &new_ctx, 3);
    assert_eq!(reopened.layouts.current, "Docked and Floating");
    assert_eq!(reopened.category, toolbox::Category::Uv);
    assert_eq!(reopened.layouts.tool_tabs_horizontal, Some(true));
    assert_eq!(reopened.upper_frame.tabs[0], PanelKind::Groups);
    assert_eq!(reopened.upper_frame.front(), Some(PanelKind::Groups));
    assert!(saved.same_as(&reopened.capture_layout().unwrap()));
}

#[test]
fn docking_layout_keeps_saved_model_and_custom_layout_bytes() {
    let root = data_root();
    install_legacy_model(&root);
    // Simulate a user who had both named layouts and leftover standalone
    // preferences. The saved Model takes precedence over those preferences.
    write_old_files(&root);
    let model = root.join("Layouts/Model.json");
    let custom = root.join("Layouts/Third.json");
    std::fs::write(&custom, THIRD).unwrap();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 3);
    assert_eq!(a.views.viewports().len(), 1);
    assert_eq!(a.layouts.tool_tabs_horizontal, None);
    assert_eq!(std::fs::read_to_string(&model).unwrap(), LEGACY_MODEL);
    a.execute("layout.restore Third");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.viewports().len(), 2);
    a.execute("layout.restore Model");
    frames(&mut a, &ctx, 2);
    assert_eq!(std::fs::read_to_string(&model).unwrap(), LEGACY_MODEL);
    assert_eq!(std::fs::read_to_string(&custom).unwrap(), THIRD);
}

#[test]
fn docking_layout_keeps_a_view_window_embedded_in_a_legacy_file() {
    let root = data_root();
    install_legacy_model(&root);
    let mut legacy = WindowLayout::from_json(LEGACY_MODEL).unwrap();
    let window = operandi_layout::FloatWindow {
        frame: operandi_layout::Frame::Panel(operandi_layout::Slot::new(operandi_layout::VIEW_3D)),
        size: [420., 320.],
        position: Some([30., 40.]),
    };
    legacy.windows.push(window.clone());
    std::fs::write(root.join("Layouts/Model.json"), legacy.to_json()).unwrap();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    assert_eq!(
        a.capture_layout().unwrap().windows,
        std::slice::from_ref(&window)
    );
    a.execute("layout.restore UV");
    assert!(a.views.windows.is_empty());
    a.execute("layout.restore Model");
    assert_eq!(a.capture_layout().unwrap().windows, [window]);
}

#[test]
fn duplicate_panels_keep_their_first_slots_and_window_menu_recovers_missing_panels() {
    use crate::panel_registry::PanelKind as K;
    use operandi_layout::{DockPanel, FloatWindow, Frame, Slot};

    let mut a = box_app();
    let ctx = Context::default();
    let mut doc = a.capture_layout().unwrap();
    let items = || Frame::DockTab(Slot::new("Items"));
    doc.frame = Frame::DockTabs {
        children: vec![
            Frame::DockPanel {
                pane: DockPanel::Items,
                frame: Box::new(Frame::Tabs {
                    tabs: ["Items", "Shader Tree", "items"].map(Slot::new).into(),
                    active: 1,
                    minimized: false,
                }),
            },
            items(),
            Frame::Panel(Slot::new(operandi_layout::VIEW_3D)),
        ],
        active: 2,
    };
    doc.windows = [
        items(),
        Frame::DockTabs {
            children: vec![items(), Frame::DockTab(Slot::new("UV Maps"))],
            active: 1,
        },
    ]
    .into_iter()
    .map(|frame| FloatWindow {
        frame,
        size: [250., 300.],
        position: None,
    })
    .collect();
    a.apply_layout_doc(&doc).unwrap();
    assert_eq!(a.upper_frame.tabs, [K::Items, K::ShaderTree]);
    assert_eq!(a.upper_frame.front(), Some(K::ShaderTree));
    assert!(a.lower_frame.tabs.is_empty());
    assert_eq!(a.views.windows.len(), 1);
    assert_eq!(a.views.dock_tabs.len(), 1);
    assert!(a.registry_panel_open(K::UvMaps));
    assert!(!a.registry_panel_open(K::Groups));

    let menus = a.menu_model(&ctx);
    let window = menus.iter().find(|menu| menu.title == "Window").unwrap();
    let panels = window
        .entries
        .iter()
        .find_map(|entry| match entry {
            app_menu::Entry::Submenu(menu) if menu.title == "Panels" => Some(menu),
            _ => None,
        })
        .unwrap();
    let entries = panels.leaves();
    assert_eq!(entries.len(), crate::panel_registry::REGISTRY.len());
    let entry = |name| entries.iter().find(|entry| entry.title == name).unwrap();
    assert!(entry("Items").checked);
    assert!(entry("UV Maps").checked);
    assert!(!entry("Groups").checked);

    a.menu_action(&ctx, entry("Groups").action);
    assert_eq!(
        a.upper_frame.tabs,
        [K::Items, K::ShaderTree, K::Groups],
        "{}",
        a.status
    );
    assert_eq!(a.upper_frame.front(), Some(K::Groups));
    a.menu_action(&ctx, entry("Items").action);
    assert_eq!(a.upper_frame.tabs, [K::Items, K::ShaderTree, K::Groups]);
    assert_eq!(a.upper_frame.front(), Some(K::Items));
    a.menu_action(&ctx, entry("UV Maps").action);
    assert_eq!(a.views.windows.len(), 1);
    assert_eq!(a.views.dock_tabs.len(), 1);
    assert!(!a.registry_panel_open(K::Properties));
    let repaired = a.capture_layout().unwrap();
    a.apply_layout_doc(&repaired).unwrap();
    assert!(repaired.same_as(&a.capture_layout().unwrap()));
}

#[test]
fn window_panels_recovers_a_row_when_the_saved_layout_has_no_sidebar() {
    use crate::panel_registry::PanelKind as K;
    use operandi_layout::{Frame, Slot};

    let mut a = box_app();
    let mut doc = a.capture_layout().unwrap();
    doc.frame = Frame::Panel(Slot::new(operandi_layout::VIEW_3D));
    a.apply_layout_doc(&doc).unwrap();
    let before = a.views.panes();
    assert!(before.iter().all(|pane| pane.view_key().is_some()));
    assert!(!a.registry_panel_open(K::ShaderTree));
    a.execute("operandi.panel.show kind:{Shader Tree}");
    assert_eq!(a.upper_frame.tabs, [K::ShaderTree], "{}", a.status);
    assert!(a.views.dock_slot(Pane::Items).is_some());
    assert_eq!(a.views.panes().len(), before.len() + 1);
    assert!(!a.registry_panel_open(K::Items));
}

/// Jason, 2026-11-19: the mode bar docks top or bottom only; the shipped
/// Model layout has it at the bottom, older layout files keep it at the
/// top, and the choice is saved with the layout.
#[test]
fn mode_bar_docks_top_or_bottom_and_is_saved_with_the_layout() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    assert!(
        a.layouts.mode_bar_bottom,
        "the Model layout ships it at the bottom"
    );
    let screen = ctx.content_rect();
    let bar = a.layouts.mode_bar_rect.expect("the mode bar drew");
    assert!(bar.center().y > screen.center().y, "bottom: {bar:?}");

    a.dock_mode_bar(false);
    frames(&mut a, &ctx, 2);
    let bar = a.layouts.mode_bar_rect.unwrap();
    assert!(bar.center().y < screen.center().y, "top: {bar:?}");
    let saved = a.capture_layout().unwrap();
    assert!(!saved.mode_bar_bottom);
    let text = saved.to_json();
    assert!(
        !text.contains("mode_bar_bottom"),
        "top is the default, not written"
    );

    a.dock_mode_bar(true);
    let saved = WindowLayout::from_json(&a.capture_layout().unwrap().to_json()).unwrap();
    assert!(saved.mode_bar_bottom, "bottom is written and read back");

    // A layout file from before the mode bar could move keeps it on top.
    let old = WindowLayout::from_json(THIRD).unwrap();
    assert!(!old.mode_bar_bottom);
}

/// Jason, 2026-11-19: the message line is a thin strip at the bottom of
/// the Tool Properties column, not a row under every view; with Tool
/// Properties floating it goes back under the views.
#[test]
fn message_line_sits_under_tool_properties_and_falls_back() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    a.status = "Mode bar test message".into();
    frames(&mut a, &ctx, 2);
    assert!(a.message_line_in_tool_properties());
    let line = a.layouts.message_line_rect.unwrap_or_else(|| {
        panic!(
            "the message line drew (mixed {}, panes {:?})",
            a.layouts.mixed_dock,
            a.views.panes()
        )
    });
    let column = a
        .toolbox_rects
        .values()
        .fold(Rect::NOTHING, |r, t| r.union(*t));
    assert!(
        line.left() < column.right() + 1. && line.right() <= ctx.content_rect().center().x,
        "under the left column: {line:?}"
    );
    assert!(line.height() <= crate::toolbox::MESSAGE_LINE_HEIGHT);

    a.set_floating(crate::floating::Panel::Toolbox, true);
    if !a.layouts.mixed_dock {
        assert!(
            !a.message_line_in_tool_properties(),
            "floating: back under the views"
        );
    }
}

/// Jason, 2026-11-19: the UV tool set opens a UV view beside the view and
/// closes it on leaving, only when it opened it; a layout entry is lit only
/// while the screen matches that saved layout.
#[test]
fn companion_uv_view_and_matching_layout_highlight() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    frames(&mut a, &ctx, 2);
    let start = a.views.viewports();
    assert!(a.layout_matches_screen("Model"), "fresh Model matches");
    assert!(!a.layout_matches_screen("UV"), "not the open layout");

    a.execute("operandi.toolbox.set UV");
    frames(&mut a, &ctx, 2);
    let now = a.views.viewports();
    assert_eq!(now.len(), start.len() + 1, "{}", a.status);
    let added = *now.iter().find(|v| !start.contains(v)).unwrap();
    assert!(a.is_uv_view(added));
    assert!(
        !a.layout_matches_screen("Model"),
        "UV tools in Model isn't Model"
    );

    a.execute("operandi.toolbox.set Model");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.viewports(), start, "the companion view closed");
    assert!(a.layout_matches_screen("Model"));

    // A UV view the user opened is theirs: the set uses it and leaves it.
    a.toggle_uv_view();
    a.execute("operandi.toolbox.set UV");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.views.viewports(), start, "no second UV view");
    a.execute("operandi.toolbox.set Model");
    frames(&mut a, &ctx, 2);
    assert!(a.is_uv_view(a.active_view), "the user's UV view stays");

    a.execute("layout.restore UV");
    frames(&mut a, &ctx, 2);
    assert!(a.layout_matches_screen("UV"));
    assert!(!a.layout_matches_screen("Model"));
}

/// An orbit drag changes the layout (it holds the camera) on every frame;
/// the file is written once the drag ends, not on every frame of it.
/// Writing it every frame froze Windows for 100-300 ms now and then.
#[test]
fn camera_drag_writes_the_layout_once_it_ends() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    a.camera.view = crate::camera::View::Perspective;
    frames(&mut a, &ctx, 2);
    let name = a.layouts.current.clone();
    let path = a.layout_file(&name).expect("a layout file path");
    let read = || std::fs::read_to_string(&path).unwrap_or_default();
    let before = read();
    let c = a.viewport_rect.center();
    let press = |pressed, pos| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::ALT,
    };
    let step = |a: &mut Operandi, events| {
        draw(a, &ctx, SIZE, events);
        a.persist_layout(&ctx);
    };
    step(&mut a, vec![Event::PointerMoved(c)]);
    step(&mut a, vec![press(true, c)]);
    let yaw = a.camera.yaw;
    for k in 1..=5 {
        step(
            &mut a,
            vec![Event::PointerMoved(c + vec2(20. * k as f32, 0.))],
        );
    }
    assert!(a.navigation.is_some(), "Alt+left drag orbits");
    assert_ne!(a.camera.yaw, yaw, "the view turned");
    assert_eq!(read(), before, "no layout writes during the drag");
    step(&mut a, vec![press(false, c + vec2(100., 0.))]);
    step(&mut a, vec![]);
    assert!(a.navigation.is_none());
    assert_ne!(read(), before, "the turned view is saved after the drag");
}

/// A wheel zoom is smoothed over several frames, each changing the camera;
/// the layout is written once the wheel goes quiet, not on each of them.
#[test]
fn wheel_zoom_writes_the_layout_once_it_settles() {
    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    a.camera.view = crate::camera::View::Perspective;
    frames(&mut a, &ctx, 2);
    let name = a.layouts.current.clone();
    let path = a.layout_file(&name).expect("a layout file path");
    let read = || std::fs::read_to_string(&path).unwrap_or_default();
    let before = read();
    let distance = a.camera.distance;
    let c = a.viewport_rect.center();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(c)]);
    a.persist_layout(&ctx);
    for _ in 0..3 {
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., 40.),
                modifiers: Modifiers::NONE,
            }],
        );
        a.persist_layout(&ctx);
    }
    assert_ne!(a.camera.distance, distance, "the wheel zoomed");
    assert_eq!(read(), before, "no layout writes while the wheel turns");
    frames(&mut a, &ctx, 60);
    assert_ne!(read(), before, "the zoomed view is saved once it settles");
}
