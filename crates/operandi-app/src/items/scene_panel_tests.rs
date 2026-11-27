//! Scenes list and item Properties: delete, transforms and history.
use super::tests::{box_app, draw, key, pointer};
use super::*;

#[test]
fn par_t13_scene_keyboard_owns_object_not_components_and_history_roundtrip() {
    for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon] {
        let mut a = box_app();
        a.editor.scene.selection.mode = mode;
        a.editor.scene.selection.faces = vec![0];
        let original = a.editor.scene.items[0].clone();
        a.run(Command::NewMesh);
        let target = a.editor.scene.selection.item.unwrap();
        let ctx = Context::default();
        theme::install(&ctx);
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let row = a.scene_rows[&target].center();
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![
                pointer(row, true, Modifiers::NONE),
                pointer(row, false, Modifiers::NONE),
            ],
        );
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        assert!(ctx.memory(|m| m.has_focus(Id::new("scene-list-focus"))));
        let before = a.editor.scene.clone();
        draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Backspace)]);
        assert_eq!(a.editor.scene.items, vec![original.clone()]);
        assert_eq!(a.editor.scene.selection.mode, mode);
        a.run(Command::Undo);
        assert_eq!(a.editor.scene, before);
        a.run(Command::Redo);
        assert_eq!(a.editor.scene.items, vec![original.clone()]);
        a.run(Command::Undo);
        let p = std::env::temp_dir().join(format!("operandi-par-delete-{mode:?}.opscene"));
        assert!(a.save(p.clone()));
        assert_eq!(persistence::load(&p).unwrap(), before);
        let _ = std::fs::remove_file(p);
        // Moving focus to the viewport cannot route another object delete.
        let point = a.viewport_rect.left_top() + vec2(40., 40.);
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![
                pointer(point, true, Modifiers::NONE),
                pointer(point, false, Modifiers::NONE),
            ],
        );
        draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Delete)]);
        assert_eq!(a.editor.scene.items.len(), 2);
    }
}
#[test]
fn par_t13_text_focus_and_live_tool_protect_scene() {
    let mut a = box_app();
    let id = a.editor.scene.selection.item.unwrap();
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    a.rename_item = Some(id);
    a.name = "Mesh 123".into();
    draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Backspace)]);
    assert_eq!(a.editor.scene.items.len(), 1);
    a.rename_item = None;
    a.activate(Tool::Sphere);
    ctx.memory_mut(|m| m.request_focus(Id::new("scene-list-focus")));
    // UIC-307: deleting the item settles the tool first; the delete runs.
    let before = a.editor.scene.items.len();
    a.run(Command::DeleteItem);
    assert_eq!(a.editor.scene.items.len(), before - 1, "{}", a.status);
}
#[test]
fn par_t05_item_property_command_preserves_component_geometry_and_rotation_order() {
    let mut a = box_app();
    let original = a.editor.scene.clone();
    let item = a.editor.scene.active().unwrap();
    let id = item.id;
    let mut t = item.transform.clone();
    t.translation = DVec3::new(1., 2., 3.);
    t.rotation = DVec3::new(0.1, 0.2, 0.3);
    t.scale = DVec3::new(2., 3., 4.);
    a.run(Command::ItemTransform {
        item: id,
        transform: t.clone(),
    });
    assert_eq!(
        a.editor.scene.active().unwrap().mesh,
        original.active().unwrap().mesh
    );
    assert_eq!(a.editor.scene.active().unwrap().transform, t);
    let committed = a.editor.scene.clone();
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, original);
    a.run(Command::Redo);
    assert_eq!(a.editor.scene, committed);
    t.scale.x = 0.;
    a.run(Command::ItemTransform {
        item: id,
        transform: t,
    });
    assert_eq!(a.editor.scene, committed);
}
#[test]
fn par_t14_combined_families_are_distinct_steps_and_shared_grid() {
    let mut a = box_app();
    let before = a.editor.scene.clone();
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    // The shared transform row spans the full panel above the tabs.
    let first = a.toolbox_rects["Transform"];
    for name in ["Transform", "Move", "Rotate", "Scale"] {
        assert_eq!(a.toolbox_rects[name].size(), first.size());
    }
    assert_eq!(first.height(), a.toolbox_rects["Cube"].height());
    assert!(
        (a.toolbox_rects["Scale"].right() - a.toolbox_rects["Category Create"].right()).abs() <= 1.
    );
    a.activate(Tool::Transform);
    let n = a.editor.history.undo.len();
    let r = a.viewport_rect;
    let mut families = vec![];
    let mut drags = vec![];
    for id in [0, 4, 8] {
        let start = a.editor.scene.clone();
        let handle = a.handles(r).into_iter().find(|h| h.0 == id).unwrap();
        assert_eq!(a.handle_hit(handle.2, r), Some(id));
        let end = handle.2 + vec2(20., -15.);
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
        drags.push((start, a.editor.scene.clone()));
    }
    // Each family's drag is its own tweak of the session (Jason,
    // 2026-11-04), and each changed the item differently.
    assert_eq!(a.editor.history.undo.len(), n, "{}", a.status);
    // Move shifts the centroid; Rotate keeps it and the spread; Scale
    // keeps the centroid but changes the spread.
    let shape = |scene: &operandi_core::Scene| {
        let v = &scene.active().unwrap().mesh.vertices;
        let centroid = v.iter().copied().sum::<DVec3>() / v.len() as f64;
        let spread: f64 = v.iter().map(|p| p.distance(centroid)).sum();
        (centroid, spread)
    };
    for (before, after) in &drags {
        assert_ne!(before, after);
        let ((c0, s0), (c1, s1)) = (shape(before), shape(after));
        families.push((c0.distance(c1) > 1e-6, (s0 - s1).abs() > 1e-6));
    }
    assert_eq!(families, [(true, false), (false, false), (false, true)]);
    let changed = a.editor.scene.clone();
    // Drop records the session as one step.
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.len(), n + 1);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
    a.run(Command::Redo);
    assert_eq!(a.editor.scene, changed);
}
#[test]
fn par_t13_remapped_delete_is_scene_scoped_and_forward_delete_restores_nonempty_mesh() {
    let mut a = box_app();
    let before = a.editor.scene.clone();
    let ctx = Context::default();
    theme::install(&ctx);
    a.bindings.insert("Delete Item".into(), "K".into());
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::K)]);
    assert_eq!(a.editor.scene, before);
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![Event::Key {
            key: Key::K,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let id = before.selection.item.unwrap();
    let row = a.scene_rows[&id].center();
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(row, true, Modifiers::NONE),
            pointer(row, false, Modifiers::NONE),
        ],
    );
    // The box is the scene's only mesh, so Delete Item asks first
    // (Jason, 2026-11-21); Enter answers Yes.
    let answer_yes = |a: &mut Operandi| {
        assert_eq!(a.editor.scene, before, "asks before deleting");
        assert!(a.scenes.confirm_last_mesh.is_some());
        draw(a, &ctx, vec2(1440., 900.), vec![]);
        draw(a, &ctx, vec2(1440., 900.), vec![key(Key::Enter)]);
        draw(a, &ctx, vec2(1440., 900.), vec![]);
        assert!(a.scenes.confirm_last_mesh.is_none());
    };
    draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::K)]);
    answer_yes(&mut a);
    assert!(a.editor.scene.items.is_empty());
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
    ctx.memory_mut(|m| m.request_focus(Id::new("scene-list-focus")));
    draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Delete)]);
    answer_yes(&mut a);
    assert!(a.editor.scene.items.is_empty());
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
}
#[test]
fn par_t05_property_text_focus_expressions_invalid_escape_and_history() {
    let mut a = box_app();
    let before = a.editor.scene.clone();
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    let field = a.field_rects["item-translation0"].center();
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(field, true, Modifiers::NONE),
            pointer(field, false, Modifiers::NONE),
        ],
    );
    a.item_fields.insert("translation0".into(), "1 m/4".into());
    draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Enter)]);
    assert_eq!(
        a.editor.scene.active().unwrap().transform.translation.x,
        0.25
    );
    assert_eq!(
        a.editor.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
    draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    let field = a.field_rects["item-scale0"].center();
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(field, true, Modifiers::NONE),
            pointer(field, false, Modifiers::NONE),
        ],
    );
    a.item_fields.insert("scale0".into(), "1/0".into());
    draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Enter)]);
    assert_eq!(a.editor.scene, before);
    draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Q)]);
    assert!(a.editor.live.is_none());
    draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Escape)]);
    assert_eq!(a.editor.scene, before);
}
#[test]
fn item_transform_arrows_drag_and_one_drag_is_one_undo_step() {
    // UIC-53: Position X's mini-slider arrows sit right of its text; a
    // drag to the right moves the item, and Undo takes the whole drag back.
    let mut a = box_app();
    let before = a.editor.scene.clone();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    let at = a.field_rects["item-translation0-arrows"].center();
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(at)]);
    draw(&mut a, &ctx, size, vec![pointer(at, true, Modifiers::NONE)]);
    for dx in [10., 20., 30., 40.] {
        draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(at + vec2(dx, 0.))],
        );
    }
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(at + vec2(40., 0.), false, Modifiers::NONE)],
    );
    let x = a.editor.scene.active().unwrap().transform.translation.x;
    assert!(x > 0.1, "dragged to {x}");
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
}
/// The scene is the Items list's root row (Jason, 2026-11-03): items sit
/// indented under it, dragging a row onto another parents it, dragging it
/// onto the scene row unparents it, and the scene row folds every item away.
#[test]
fn scene_root_row_parents_unparents_and_folds() {
    let mut a = box_app();
    a.run(Command::NewMesh);
    let first = a.editor.scene.items[0].id;
    let second = a.editor.scene.selection.item.unwrap();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    let root = a.scene_root_rect.unwrap();
    assert!(root.bottom() <= a.scene_rows[&first].top() + 0.5);
    let drag = |a: &mut Operandi, from: Pos2, to: Pos2| {
        let mid = from + (to - from) / 2.;
        // Press on its own frame: egui hit-tests where the pointer ends up.
        draw(a, &ctx, size, vec![pointer(from, true, Modifiers::NONE)]);
        draw(a, &ctx, size, vec![Event::PointerMoved(mid)]);
        draw(a, &ctx, size, vec![Event::PointerMoved(to)]);
        draw(a, &ctx, size, vec![pointer(to, false, Modifiers::NONE)]);
        draw(a, &ctx, size, vec![]);
    };
    let (from, to) = (
        a.scene_rows[&second].center(),
        a.scene_rows[&first].center(),
    );
    drag(&mut a, from, to);
    let parent = |a: &Operandi| a.editor.scene.item(second).unwrap().parent;
    assert_eq!(parent(&a), Some(first));
    let from = a.scene_rows[&second].center();
    drag(&mut a, from, root.center());
    assert_eq!(parent(&a), None);
    let disclosure = a.scene_root_rect.unwrap().left_center() + vec2(7., 0.);
    draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(disclosure, true, Modifiers::NONE),
            pointer(disclosure, false, Modifiers::NONE),
        ],
    );
    draw(&mut a, &ctx, size, vec![]);
    assert!(a.scene_rows.is_empty());
}
/// IT-PHY-04: in Items mode with Move, Ctrl+Shift dragging the X handle
/// leaves the original where it was and moves a selected copy.
#[test]
fn ctrl_shift_handle_drag_moves_a_copy() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Item));
    let original = a.editor.scene.items[0].clone();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    a.activate(Tool::Move);
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    let r = a.viewport_rect;
    let handle = a.handles(r).into_iter().find(|h| h.0 == 0).unwrap();
    let start = handle.2;
    let end = start + vec2(40., 0.);
    let mods = Modifiers::CTRL | Modifiers::SHIFT;
    draw(&mut a, &ctx, size, vec![pointer(start, true, mods)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![Event::PointerMoved(start + vec2(20., 0.))],
    );
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
    draw(&mut a, &ctx, size, vec![pointer(end, false, mods)]);
    a.run(Command::Drop);
    let items = &a.editor.scene.items;
    assert_eq!(items.len(), 2, "{}", a.status);
    assert_eq!(items[0], original);
    assert_eq!(a.editor.scene.selection.item, Some(items[1].id));
    assert!(items[1].transform.translation.x.abs() > 1e-6);
    assert_eq!(items[1].mesh, original.mesh);
}
/// Jason, 2026-11-03 (provisional): the center handle moves freely in the
/// view plane, and a press away from the model puts the center there and
/// drags from it; Rotate then turns about that point.
#[test]
fn free_center_handle_and_click_away_center() {
    let size = vec2(1440., 900.);
    let drag = |a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2| {
        draw(a, ctx, size, vec![pointer(from, true, Modifiers::NONE)]);
        draw(
            a,
            ctx,
            size,
            vec![Event::PointerMoved(from + (to - from) / 2.)],
        );
        draw(a, ctx, size, vec![Event::PointerMoved(to)]);
        draw(a, ctx, size, vec![pointer(to, false, Modifiers::NONE)]);
    };
    // Free move: the selection follows the pointer, not one axis.
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let ctx = Context::default();
    theme::install(&ctx);
    a.activate(Tool::Move);
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    let r = a.viewport_rect;
    let free = a
        .handles(r)
        .into_iter()
        .find(|h| h.0 == viewport::FREE_HANDLE)
        .unwrap();
    assert_eq!(a.handle_hit(free.2, r), Some(viewport::FREE_HANDLE));
    drag(&mut a, &ctx, free.2, free.2 + vec2(30., -20.));
    // The release recorded the drag as its own step (Jason, 2026-11-03).
    let after = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let delta = before
        .iter()
        .zip(&after)
        .map(|(b, a)| *a - *b)
        .find(|d| d.length() > 1e-9)
        .unwrap_or_default();
    let (_, right, up, _) = a.camera.basis();
    assert!(delta.length() > 1e-6, "{}", a.status);
    // Along the view's right and up, nothing into the screen.
    let (_, _, _, forward) = a.camera.basis();
    assert!(delta.dot(forward).abs() < 1e-9 && delta.dot(right) > 0. && delta.dot(up) > 0.);
    a.run(Command::Drop);
    assert_ne!(a.editor.scene.active().unwrap().mesh.vertices, before);

    // Rotate after a press away from the model turns about that point.
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.activate(Tool::Rotate);
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    let r = a.viewport_rect;
    let away = r.left_top() + vec2(60., r.height() / 2.);
    assert!(a.pick_component(away, r).is_none());
    drag(&mut a, &ctx, away, away + vec2(40., 0.));
    let live = a.editor.live.as_ref().unwrap();
    let center = live.action_frame.unwrap().center;
    let (projected, _) = a.camera.project(center, r).unwrap();
    assert!(projected.distance(away) < 1., "{projected:?} vs {away:?}");
    assert!(a.editor.tweaks.can_undo(), "{}", a.status);
    assert_ne!(a.editor.history.undo.last().unwrap().before, a.editor.scene);
}

#[test]
fn items_rows_alternate_shading_under_selection() {
    // Scene root (row 0) plain, first item striped, and so on.
    assert_eq!(crate::tree_list::row_fill(0), theme::workspace::list());
    assert_eq!(
        crate::tree_list::row_fill(1),
        theme::workspace::list_stripe()
    );
    assert_eq!(crate::tree_list::row_fill(2), theme::workspace::list());
    assert_ne!(theme::workspace::list(), theme::workspace::list_stripe());
    // Selection fills must stay distinct from both stripes.
    for fill in [theme::workspace::selected(), theme::workspace::primary()] {
        assert_ne!(fill, theme::workspace::list());
        assert_ne!(fill, theme::workspace::list_stripe());
    }
}

#[test]
fn item_transform_edits_every_selected_item_and_shows_mixed() {
    // UIC-56/57/118: two meshes at different X: Position X reads "(mixed)";
    // typing 2 m moves both, one Undo puts both back; the arrows move each
    // from its own value.
    let mut a = box_app();
    a.run(Command::NewMesh);
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    let (first, second) = (ids[0], ids[1]);
    let mut moved = a.editor.scene.item(second).unwrap().transform.clone();
    moved.translation.x = 1.;
    a.run(Command::ItemTransform {
        item: second,
        transform: moved,
    });
    a.editor.scene.selection.item = Some(first);
    a.editor.scene.selection.also = vec![second];
    let before = a.editor.scene.clone();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    assert_eq!(a.item_fields["translation0"], crate::form::rows::MIXED);
    assert_eq!(a.item_fields["translation1"], "0 m");
    let field = a.field_rects["item-translation0"].center();
    draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(field, true, Modifiers::NONE),
            pointer(field, false, Modifiers::NONE),
        ],
    );
    a.item_fields.insert("translation0".into(), "2".into());
    draw(&mut a, &ctx, size, vec![key(Key::Enter)]);
    let x = |a: &Operandi, id: u64| a.editor.scene.item(id).unwrap().transform.translation.x;
    assert_eq!((x(&a, first), x(&a, second)), (2., 2.));
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.items, before.items);
    // A drag on the arrows keeps the 1 m gap between them.
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    let at = a.field_rects["item-translation0-arrows"].center();
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(at)]);
    draw(&mut a, &ctx, size, vec![pointer(at, true, Modifiers::NONE)]);
    for dx in [10., 20., 30.] {
        draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(at + vec2(dx, 0.))],
        );
    }
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(at + vec2(30., 0.), false, Modifiers::NONE)],
    );
    assert!(x(&a, first) > 0.05, "moved {}", x(&a, first));
    assert!((x(&a, second) - x(&a, first) - 1.).abs() < 1e-9);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.items, before.items);
}

/// #55 (Leo): in Item mode, pressing on an item in the 3D view and
/// releasing over an Items list row parents it to that row's item; one
/// undo step takes it back. Released over the view it's a plain select.
#[test]
fn an_item_dragged_from_the_view_onto_a_list_row_is_parented() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Item));
    a.run(Command::NewMesh);
    let cube = a.editor.scene.items[0].id;
    let other = a.editor.scene.selection.item.unwrap();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    a.camera.view = View::Perspective;
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    // The cube's middle in the view; the other mesh's row in the list.
    let r = a.viewport_rect;
    let on_cube = a.camera.project(DVec3::ZERO, r).unwrap().0;
    let row = a.scene_rows[&other].center();
    let undo = a.editor.history.undo.len();
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(on_cube)]);
    draw(&mut a, &ctx, size, vec![pointer(on_cube, true, Modifiers::NONE)]);
    for k in 1..=4 {
        let at = on_cube + (row - on_cube) * (k as f32 / 4.);
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(at)]);
    }
    draw(&mut a, &ctx, size, vec![pointer(row, false, Modifiers::NONE)]);
    draw(&mut a, &ctx, size, vec![]);
    assert_eq!(
        a.editor.scene.item(cube).unwrap().parent,
        Some(other),
        "{}",
        a.status
    );
    assert!(a.editor.history.undo.len() <= undo + 2, "selection + parent");
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.item(cube).unwrap().parent, None);
}
