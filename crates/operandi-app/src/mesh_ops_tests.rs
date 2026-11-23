//! The Mesh Ops tab through the real panel (MeshOps v1, MO-10..28).
use super::tests::{draw, pointer};
use super::*;
use crate::mesh_ops_ui::OpKey;
use operandi_core::meshops_editor::OpRow;

const SIZE: Vec2 = vec2(1440., 900.);

/// An empty mesh with a Cube and a Triangulate op, Mesh Ops tab in front.
fn stacked() -> (Operandi, Context, u64) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::MeshOpCreate("prim.cube.item".into()));
    a.run(Command::MeshOpCreate("pmodel.triangulate.item".into()));
    let id = a.editor.scene.items[0].id;
    let page = a
        .upper_frame
        .tabs
        .iter()
        .position(|k| *k == panel_registry::PanelKind::MeshOps)
        .expect("the Mesh Stack tab is in the default layout");
    a.upper_frame.show_page(page);
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..3 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx, id)
}
fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(
        a,
        ctx,
        SIZE,
        vec![
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    );
    draw(a, ctx, SIZE, vec![]);
}
fn ops(a: &Operandi) -> Vec<String> {
    a.editor.scene.items[0]
        .mesh_ops
        .as_ref()
        .map_or_else(Vec::new, |s| {
            s.ops.iter().map(|o| o.kind.type_id().to_string()).collect()
        })
}

#[test]
fn the_list_shows_ops_top_down_with_the_base_mesh_last() {
    let (a, _, id) = stacked();
    let keys: Vec<OpKey> = a.mesh_ops_tree.rects.iter().map(|(k, _)| *k).collect();
    let stack = a.editor.scene.items[0].mesh_ops.as_ref().unwrap();
    assert_eq!(
        keys,
        vec![
            OpKey::Op(id, stack.ops[1].id),
            OpKey::Op(id, stack.ops[0].id),
            OpKey::Base(id)
        ]
    );
}

#[test]
fn clicking_rows_selects_them_and_the_eye_turns_an_op_off() {
    let (mut a, ctx, id) = stacked();
    let base = a.mesh_ops_tree.rects.last().unwrap().1.center();
    click(&mut a, &ctx, base);
    assert_eq!(a.editor.meshop_row, Some(OpRow { item: id, op: None }));
    let top = a.editor.scene.items[0].mesh_ops.as_ref().unwrap().ops[1].id;
    let eye = a.mesh_ops_tree.cells[&(OpKey::Op(id, top), 0)].center();
    click(&mut a, &ctx, eye);
    assert!(!a.editor.scene.items[0].mesh_ops.as_ref().unwrap().ops[1].enabled);
    let item = a.editor.scene.items[0].clone();
    assert_eq!(
        a.editor
            .meshops
            .current(&a.editor.scene, &item)
            .unwrap()
            .mesh
            .faces
            .len(),
        6
    );
    a.run(Command::Undo);
    assert!(a.editor.scene.items[0].mesh_ops.as_ref().unwrap().ops[1].enabled);
}

#[test]
fn edit_base_mesh_draws_the_base_and_leaving_it_draws_the_result() {
    let (mut a, ctx, id) = stacked();
    a.run(Command::MeshOpEditBase { item: id, on: true });
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.editor.base_edit, Some(id));
    assert!(
        a.editor.evaluated_display().is_none(),
        "the Base Mesh is drawn"
    );
    a.run(Command::MeshOpEditBase {
        item: id,
        on: false,
    });
    let shown = a.editor.evaluated_display().unwrap();
    assert_eq!(shown.items[0].mesh.faces.len(), 12, "the result is drawn");
}

#[test]
fn the_views_count_the_result_s_triangles() {
    let (a, _, _) = stacked();
    // The result: the cube triangulated, 12 triangles (the base is empty).
    assert_eq!(a.drawn_triangles(), 12);
}

#[test]
fn freeze_from_the_row_menu_command_bakes_the_stack() {
    let (mut a, ctx, _) = stacked();
    a.run(Command::MeshOpFreezeAll);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(ops(&a).is_empty());
    assert_eq!(a.editor.scene.items[0].mesh.faces.len(), 12);
}

#[test]
fn a_selected_op_shows_its_values_in_the_properties_panel() {
    let (mut a, ctx, _) = stacked();
    let top = a.mesh_ops_tree.rects[0].1.center();
    click(&mut a, &ctx, top);
    assert_eq!(a.property_tabs(), [crate::mesh_ops_ui::MESH_OP_TAB]);
    // Select the Bevel instead and check its rows are drawn there.
    a.run(Command::MeshOpCreate("poly.bevel.item".into()));
    crate::ui_inventory::take_rects();
    draw(&mut a, &ctx, SIZE, vec![]);
    let rects = crate::ui_inventory::take_rects();
    assert!(rects.contains_key("Shift"), "{:?}", rects.keys());
    assert!(rects.contains_key("Group Polygons"));
    // The Base Mesh row (or another front tab) is the item's own panel.
    let base = a.mesh_ops_tree.rects.last().unwrap().1.center();
    click(&mut a, &ctx, base);
    assert_ne!(a.property_tabs(), [crate::mesh_ops_ui::MESH_OP_TAB]);
}

#[test]
fn frame_all_frames_a_mesh_ops_result() {
    // Jason 2026-11-17: Frame All did nothing with only a MeshOps item.
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::MeshOpCreate("prim.cube.item".into()));
    let item = a.editor.scene.items[0].id;
    let op = a.editor.scene.items[0].mesh_ops.as_ref().unwrap().ops[0].id;
    let mut v = serde_json::Map::new();
    v.insert("cenX".into(), 10.0.into());
    a.run(Command::MeshOpSet {
        op: operandi_core::meshops_editor::OpRef { item, op },
        values: v,
    });
    a.frame(false);
    assert!(
        (a.camera.target.x - 10.).abs() < 1e-9,
        "{}",
        a.camera.target
    );
    // Frame Selection with the item selected (Items mode) frames it too.
    a.run(Command::Mode(Mode::Item));
    a.camera.target = glam::DVec3::ZERO;
    a.frame(true);
    assert!(
        (a.camera.target.x - 10.).abs() < 1e-9,
        "{}",
        a.camera.target
    );
}

#[test]
fn ab_5_add_operator_files_ops_in_the_mesh_operations_tree() {
    let c = crate::mesh_ops_ui::operator_catalog();
    let folder_of = |label: &str| {
        c.entries
            .iter()
            .find(|e| e.label == label)
            .unwrap_or_else(|| panic!("{label} missing"))
            .folder
            .join("/")
    };
    for (label, folder) in [
        ("Cube", "Mesh Operations/Create/Primitives"),
        ("Create Vertex", "Mesh Operations/Create"),
        ("Polygon Unify", "Mesh Operations/Deform"),
        ("Jitter", "Mesh Operations/Deform"),
        ("Radial Array", "Mesh Operations/Duplicate"),
        ("Edge Subdivide", "Mesh Operations/Edge"),
        ("Boolean", "Mesh Operations/Edit"),
        ("Merge Meshes", "Mesh Operations/Edit"),
        ("Polygon Reduce", "Mesh Operations/Polygon"),
        ("Flip Polygons", "Mesh Operations/Polygon"),
        ("UV Unwrap", "Mesh Operations/UV"),
        ("Vertex Bevel", "Mesh Operations/Vertex"),
    ] {
        assert_eq!(folder_of(label), folder, "{label}");
    }
    // Empty roots (Deformers, Assemblies) don't show (AB-41).
    assert!(c.folders().iter().all(|f| f[0] == "Mesh Operations"));
}

#[test]
fn ab_50_add_item_lists_what_operandi_makes() {
    let c = crate::item_list_ui::item_catalog();
    let mesh = c.entries.iter().find(|e| e.label == "Mesh").unwrap();
    assert_eq!(mesh.folder, ["Items"]);
    let spot = c.entries.iter().find(|e| e.label == "Spot Light").unwrap();
    assert_eq!(spot.folder, ["Items", "Lights"]);
    assert!(
        c.entries.iter().all(|e| e.label != "Bend Effector"),
        "not built: not listed"
    );
}

#[test]
fn add_operator_opens_the_browser_and_double_click_adds() {
    let (mut a, ctx, id) = stacked();
    let before = a
        .editor
        .scene
        .item(id)
        .unwrap()
        .mesh_ops
        .as_ref()
        .unwrap()
        .ops
        .len();
    a.add_browsers
        .entry(crate::mesh_ops_ui::ADD_OPERATOR)
        .or_insert_with(|| {
            (
                Default::default(),
                egui::Rect::from_min_size(pos2(1000., 200.), vec2(80., 20.)),
            )
        })
        .0
        .open = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    // Search for Cube and double-click its row.
    a.add_browsers
        .get_mut(crate::mesh_ops_ui::ADD_OPERATOR)
        .unwrap()
        .0
        .search = "Cube".into();
    draw(&mut a, &ctx, SIZE, vec![]);
    // Enter on a selected row adds it (AB-22).
    a.add_browsers
        .get_mut(crate::mesh_ops_ui::ADD_OPERATOR)
        .unwrap()
        .0
        .selected = Some("prim.cube.item".into());
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let after = a
        .editor
        .scene
        .item(id)
        .unwrap()
        .mesh_ops
        .as_ref()
        .unwrap()
        .ops
        .len();
    assert_eq!(after, before + 1);
    assert!(
        !a.add_browsers[crate::mesh_ops_ui::ADD_OPERATOR].0.open,
        "unpinned: it closes"
    );
}

#[test]
fn ab_10_11_the_browser_is_a_450_by_600_pop_over_with_two_vertical_columns() {
    // Jason 2026-11-17: it stretched across the window with everything on
    // one row.
    let (mut a, ctx, _) = stacked();
    a.add_browsers
        .entry(crate::mesh_ops_ui::ADD_OPERATOR)
        .or_insert_with(|| {
            (
                Default::default(),
                egui::Rect::from_min_size(pos2(900., 120.), vec2(90., 20.)),
            )
        })
        .0
        .open = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    crate::add_browser::RECTS.with(|r| r.borrow_mut().clear());
    draw(&mut a, &ctx, SIZE, vec![]);
    let rects = crate::add_browser::RECTS.with(|r| r.borrow().clone());
    let get = |k: &str| {
        rects
            .iter()
            .rev()
            .find(|(t, _)| t == k)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| {
                panic!(
                    "{k} not drawn: {:?}",
                    rects.iter().map(|(t, _)| t).collect::<Vec<_>>()
                )
            })
    };
    let window = get("window");
    assert!(
        (window.width() - 450.).abs() < 2. && (window.height() - 600.).abs() < 2.,
        "{window:?}"
    );
    // Directories: one column, rows stacked downwards, inside the left 36 %.
    let root = get("dir:Mesh Operations");
    let polygon = get("dir:Mesh Operations/Polygon");
    assert!(polygon.top() > root.bottom() - 1., "{root:?} {polygon:?}");
    assert!(polygon.right() < window.left() + window.width() * 0.45);
    // Names: right of the divider, headers and entries stacked downwards.
    let header = get("group:Mesh Operations/Create/Primitives");
    let cube = get("entry:Cube");
    let cylinder = get("entry:Cylinder");
    assert!(header.left() > root.right(), "{header:?} right of {root:?}");
    assert!(cube.top() > header.bottom() - 1. && cylinder.top() > cube.bottom() - 1.);
    assert!((cube.left() - cylinder.left()).abs() < 1., "one column");
    assert!(cube.right() <= window.right() + 1.);
}

#[test]
fn every_op_icon_name_is_a_generator_icon() {
    // Ops with no direct-tool icon use the default cube (Jason).
    let generic = [
        "pmodel.invertPolys.item",
        "pmodel.unifyPolygons.item",
        "delete.meshop.item",
        "meshcleanup.tool.item",
        // Relax Mesh is a small text button with no icon (DEFORM §1).
        "sculpt.relaxMesh.item",
        // The skin (SK-36) until its studio icon is rendered.
        "operandi.skin.item",
    ];
    for d in operandi_core::meshops::op_defs() {
        if d.id().starts_with("test.") || generic.contains(&d.id()) {
            continue;
        }
        assert!(
            crate::mesh_ops_ui::op_icon(d.id()).is_some(),
            "{} has no icon",
            d.id()
        );
    }
}

#[test]
fn the_add_browser_drags_by_its_title_bar_and_the_circle_pins_it() {
    // Jason 2026-11-17: our popup rules, movable, nothing spilling out.
    let (mut a, ctx, _) = stacked();
    a.add_browsers
        .entry(crate::mesh_ops_ui::ADD_OPERATOR)
        .or_insert_with(|| {
            (
                Default::default(),
                egui::Rect::from_min_size(pos2(900., 120.), vec2(90., 20.)),
            )
        })
        .0
        .open = true;
    let window = || {
        crate::add_browser::RECTS.with(|r| {
            r.borrow()
                .iter()
                .rev()
                .find(|(t, _)| t == "window")
                .map(|(_, r)| *r)
                .unwrap()
        })
    };
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![]);
    let before = window();
    // Drag the title bar left of centre (clear of the circle).
    let grab = pos2(before.left() + 60., before.top() + 14.);
    let to = grab + vec2(-200., 40.);
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(grab)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(grab, true, Modifiers::NONE)],
    );
    for k in 1..=4 {
        let p = grab + (to - grab) * (k as f32 / 4.);
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(to, false, Modifiers::NONE)],
    );
    draw(&mut a, &ctx, SIZE, vec![]);
    let after = window();
    assert!(
        (after.left() - (before.left() - 200.)).abs() < 3.
            && (after.top() - (before.top() + 40.)).abs() < 3.,
        "{before:?} -> {after:?}"
    );
    let state = &a.add_browsers[crate::mesh_ops_ui::ADD_OPERATOR].0;
    assert!(state.open && state.place.is_some());
    // Every row stays inside the window.
    crate::add_browser::RECTS.with(|r| r.borrow_mut().clear());
    draw(&mut a, &ctx, SIZE, vec![]);
    let rects = crate::add_browser::RECTS.with(|r| r.borrow().clone());
    let w = window();
    for (t, r) in rects
        .iter()
        .filter(|(t, _)| t.starts_with("group:") || t.starts_with("dir:"))
    {
        assert!(r.right() <= w.right() + 1., "{t} {r:?} outside {w:?}");
    }
}

/// Bug report #5: in Item mode the Add Mesh Operation browser vanished on
/// the first click inside it. On the shipped Model layout the 3D view is
/// drawn before the Mesh Stack panel, so the view read the press before the
/// panel's browser claimed the pointer that frame (`menu_owned_input`): the
/// click in the search field deselected the mesh under the browser, and the
/// Mesh Stack (and its browser) went away. A press on any window above a
/// view belongs to that window, whatever order things are drawn in.
#[test]
fn a_click_in_the_add_operator_browser_over_a_view_stays_in_the_browser() {
    let root = crate::layouts_tests::data_root();
    let ctx = Context::default();
    let mut a = crate::layouts_tests::launch(&root, &ctx);
    a.execute("layout.restore Model");
    assert!(a.layouts.mixed_dock, "the view draws before the dock panel");
    let id = a.editor.scene.items[0].id;
    a.run(Command::Mode(Mode::Item));
    let page = a
        .upper_frame
        .tabs
        .iter()
        .position(|k| *k == panel_registry::PanelKind::MeshOps)
        .expect("the Mesh Stack tab is in the Model layout");
    a.upper_frame.show_page(page);
    for _ in 0..3 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    assert!(a.editor.scene.active().is_some_and(|i| i.id == id));
    // Opened from a button on the right, so it opens to its left, over the view.
    let anchor = egui::Rect::from_min_size(pos2(1300., 200.), vec2(90., 20.));
    a.add_browsers
        .entry(crate::mesh_ops_ui::ADD_OPERATOR)
        .or_insert_with(|| (Default::default(), anchor))
        .0
        .open = true;
    crate::add_browser::RECTS.with(|r| r.borrow_mut().clear());
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![]);
    let drawn = |name: &str| {
        crate::add_browser::RECTS.with(|r| {
            r.borrow()
                .iter()
                .rev()
                .find(|(t, _)| t.starts_with(name))
                .map(|(_, r)| *r)
                .unwrap_or_else(|| panic!("the browser drew {name}"))
        })
    };
    let (window, field, row) = (drawn("window"), drawn("search"), drawn("dir:"));
    // The reporter clicked in the search field, over the 3D view; then
    // empty space low in the browser, and a folder row.
    for (what, at) in [
        (
            "the search field",
            pos2(field.left() + 30., field.center().y),
        ),
        (
            "empty space",
            pos2(window.left() + 20., window.bottom() - 12.),
        ),
        ("a folder row", pos2(row.left() + 20., row.center().y)),
    ] {
        assert!(
            a.viewport_rect.contains(at),
            "{what}: {at:?} {:?}",
            a.viewport_rect
        );
        click(&mut a, &ctx, at);
        assert!(
            a.editor.scene.active().is_some_and(|i| i.id == id),
            "{what}: the mesh under the browser stays selected"
        );
        assert!(
            a.add_browsers[crate::mesh_ops_ui::ADD_OPERATOR].0.open,
            "{what}: the browser stays open"
        );
    }
}
