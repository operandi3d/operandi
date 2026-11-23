//! The shared tree list through the real panels (UI bible section 10 and
//! the list rules of section 4): selection clicks, eye sweeps and solo,
//! double-click on a parent, the search row, column lines, and the panel
//! groups' tabs saved with the layout.
use super::tests::{box_app, draw, pointer};
use super::*;
use crate::items_tree::{EYE, ItemKey};

const SIZE: Vec2 = vec2(1440., 900.);

/// A box plus three empty meshes, drawn until the layout settles.
fn four_items() -> (Operandi, Context, Vec<u64>) {
    let mut a = box_app();
    for _ in 0..3 {
        a.run(Command::NewMesh);
    }
    let ids = a.editor.scene.items.iter().map(|i| i.id).collect();
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..3 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx, ids)
}

/// `draw` with modifier keys held in the frame's input.
fn frame_with(a: &mut Operandi, ctx: &Context, events: Vec<Event>, modifiers: Modifiers) {
    let mut raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
        events,
        modifiers,
        focused: true,
        ..Default::default()
    };
    raw.viewports
        .entry(ViewportId::ROOT)
        .or_default()
        .native_pixels_per_point = Some(1.);
    let _ = ctx.run(raw, |ctx| {
        a.begin_input(ctx);
        a.compose(ctx);
        a.keyboard(ctx);
    });
}

fn click_with(a: &mut Operandi, ctx: &Context, at: Pos2, m: Modifiers) {
    frame_with(a, ctx, vec![Event::PointerMoved(at)], m);
    frame_with(a, ctx, vec![pointer(at, true, m), pointer(at, false, m)], m);
    draw(a, ctx, SIZE, vec![]);
}

fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![pointer(from, true, Modifiers::NONE)]);
    for k in 1..=6 {
        let p = from + (to - from) * (k as f32 / 6.);
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(a, ctx, SIZE, vec![pointer(to, false, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![]);
}

fn eye(a: &Operandi, id: u64) -> Rect {
    let scene = a.scenes.active_tab().id;
    a.items_tree.cells[&(ItemKey::Item(scene, id), EYE)]
}

#[test]
fn click_ctrl_click_and_shift_click_select_rows() {
    // UIC-30..32; Ctrl is the real Ctrl key on every platform (F1).
    let (mut a, ctx, ids) = four_items();
    let row = |a: &Operandi, i: usize| a.scene_rows[&ids[i]].center();
    let p0 = row(&a, 1);
    click_with(&mut a, &ctx, p0, Modifiers::NONE);
    assert_eq!(a.editor.scene.picked_items(), [ids[1]]);
    let p0 = row(&a, 3);
    click_with(&mut a, &ctx, p0, Modifiers::CTRL);
    assert_eq!(a.editor.scene.picked_items(), [ids[1], ids[3]]);
    let p0 = row(&a, 3);
    click_with(&mut a, &ctx, p0, Modifiers::CTRL);
    assert_eq!(
        a.editor.scene.picked_items(),
        [ids[1]],
        "Ctrl again removes"
    );
    // Shift: the range from the last clicked row (row 3) to this one.
    let p0 = row(&a, 0);
    click_with(&mut a, &ctx, p0, Modifiers::SHIFT);
    assert_eq!(
        a.editor.scene.picked_items(),
        [ids[3], ids[2], ids[1], ids[0]]
    );
    assert_eq!(
        a.editor.scene.selection.item,
        Some(ids[0]),
        "the clicked end is primary (pick order anchor..clicked)"
    );
    // Cmd toggles like Ctrl (Jason, 2026-11-08): row 2 is selected, so
    // Cmd-click takes it out and a second Cmd-click puts it back.
    let p0 = row(&a, 2);
    click_with(&mut a, &ctx, p0, Modifiers::MAC_CMD | Modifiers::COMMAND);
    assert_eq!(a.editor.scene.picked_items(), [ids[3], ids[1], ids[0]]);
    click_with(&mut a, &ctx, p0, Modifiers::MAC_CMD | Modifiers::COMMAND);
    assert_eq!(
        a.editor.scene.picked_items(),
        [ids[3], ids[1], ids[0], ids[2]]
    );
    // Opt (Alt) is a plain click.
    click_with(&mut a, &ctx, p0, Modifiers::ALT);
    assert_eq!(a.editor.scene.picked_items(), [ids[2]]);
}

#[test]
fn dragging_down_the_eye_column_hides_every_row_passed_in_one_step() {
    // UIC-138.
    let (mut a, ctx, ids) = four_items();
    let steps = a.editor.undo.len();
    let p0 = eye(&a, ids[0]).center();
    let p1 = eye(&a, ids[2]).center();
    drag(&mut a, &ctx, p0, p1);
    let visible: Vec<bool> = a.editor.scene.items.iter().map(|i| i.visible).collect();
    assert_eq!(visible, [false, false, false, true]);
    assert_eq!(a.editor.undo.len(), steps + 1);
    a.run(Command::Undo);
    assert!(a.editor.scene.items.iter().all(|i| i.visible));
}

#[test]
fn alt_click_on_an_eye_shows_only_that_row_and_again_brings_the_rest_back() {
    let (mut a, ctx, ids) = four_items();
    let p0 = eye(&a, ids[1]).center();
    click_with(&mut a, &ctx, p0, Modifiers::ALT);
    let visible: Vec<bool> = a.editor.scene.items.iter().map(|i| i.visible).collect();
    assert_eq!(visible, [false, true, false, false]);
    let p0 = eye(&a, ids[1]).center();
    click_with(&mut a, &ctx, p0, Modifiers::ALT);
    assert!(a.editor.scene.items.iter().all(|i| i.visible));
}

#[test]
fn double_click_on_a_parent_selects_it_and_its_children() {
    // UIC-38 (the parent stays selected: bible default for FLAGS Q4).
    let (mut a, ctx, ids) = four_items();
    for &child in &ids[1..3] {
        a.run(Command::SetParent {
            item: child,
            parent: Some(ids[0]),
            keep_position: true,
        });
    }
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    let at = a.scene_rows[&ids[0]].center();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    for _ in 0..2 {
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![
                pointer(at, true, Modifiers::NONE),
                pointer(at, false, Modifiers::NONE),
            ],
        );
    }
    draw(&mut a, &ctx, SIZE, vec![]);
    let mut selected = a.editor.scene.selected_items();
    selected.sort();
    assert_eq!(selected, [ids[0], ids[1], ids[2]]);
    assert_eq!(a.editor.scene.selection.item, Some(ids[0]));
    assert!(
        a.items_tree.rename.is_none(),
        "a double-click never renames"
    );
}

#[test]
fn the_funnel_opens_a_search_row_and_off_brings_the_full_list_back() {
    // UIC-40 / UIC-132: the funnel is the tool row's last control.
    let (mut a, ctx, ids) = four_items();
    let funnel = a.items_tree.tool_rects["funnel"];
    let notes = a.toolbox_rects["Item list Notes"];
    assert!(funnel.left() > notes.right(), "funnel at the far right");
    click_with(&mut a, &ctx, funnel.center(), Modifiers::NONE);
    assert!(a.items_tree.search.open);
    let name = a.editor.scene.items[2].name.clone();
    a.item_list.filter.text = Some(name.clone());
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    assert_eq!(a.scene_rows.keys().copied().collect::<Vec<_>>(), [ids[2]]);
    // Pattern mode (whole name) and Match Case reach the core filter.
    a.item_list.filter.text_pattern = true;
    a.item_list.filter.text = Some(a.editor.scene.items[0].name.to_lowercase());
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    assert_eq!(a.scene_rows.keys().copied().collect::<Vec<_>>(), [ids[0]]);
    a.item_list.filter.text_case = true;
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    let first = &a.editor.scene.items[0].name;
    if *first != first.to_lowercase() {
        assert!(a.scene_rows.is_empty(), "Match Case");
    }
    let funnel = a.items_tree.tool_rects["funnel"];
    click_with(&mut a, &ctx, funnel.center(), Modifiers::NONE);
    assert_eq!(a.item_list.filter.text, None);
    assert_eq!(a.scene_rows.len(), ids.len());
}

#[test]
fn column_lines_run_the_whole_height_even_below_the_last_row() {
    // UIC-136 / UIC-145: an empty list keeps its header and lines, with no
    // "nothing here" message.
    let (mut a, ctx, _) = four_items();
    a.upper_frame.show_page(crate::groups_ui::GROUPS_PAGE);
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    let body = a.groups_tree.body;
    assert!(a.groups_tree.rects.is_empty(), "no groups yet");
    assert!(body.height() > 3. * tree_list::ROW, "{body:?}");
    assert_eq!(a.groups_tree.dividers.len(), 1, "eye | Name");
}

#[test]
fn shader_tree_rows_rename_on_a_slow_second_click() {
    // UIC-39: click, pause, click.
    let (mut a, ctx, _) = four_items();
    a.upper_frame.show_page(1);
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    let name = a.shading.row_rects[&3][1].center();
    click_with(&mut a, &ctx, name, Modifiers::NONE);
    assert_eq!(a.shading.selected, [3]);
    // Wait out the double-click time (each frame moves the clock on by
    // about 1/60 s), then click again.
    for _ in 0..40 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![
            pointer(name, true, Modifiers::NONE),
            pointer(name, false, Modifiers::NONE),
        ],
    );
    assert!(
        matches!(
            a.shading.tree.rename,
            Some((shading_panel::ShaderKey::Layer(3), _))
        ),
        "{:?}",
        a.shading.tree.rename
    );
}

#[test]
fn the_plus_menu_adds_a_panel_and_tabs_are_saved_with_the_layout() {
    // UIC-156/157.
    let (mut a, ctx, _) = four_items();
    // Groups moves to the lower group, in front.
    a.lower_frame.add(panel_registry::PanelKind::Groups);
    a.upper_frame.remove(panel_registry::PanelKind::Groups);
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    assert!(a.front_panel(panel_registry::PanelKind::Groups));
    assert!(a.lower_frame.rects.len() == 2);
    let text = a.layout_json(&ctx);
    let mut b = box_app();
    let dir = std::env::temp_dir().join(format!("operandi-tree-layout-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("layout.json");
    std::fs::write(&path, text).unwrap();
    b.load_layout_from(&ctx, &path);
    assert_eq!(b.upper_frame.tabs, a.upper_frame.tabs);
    assert_eq!(b.lower_frame.tabs, a.lower_frame.tabs);
    assert_eq!(
        b.lower_frame.front(),
        Some(panel_registry::PanelKind::Groups)
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A point `t` of the way down item `id`'s row.
fn row_at(a: &Operandi, id: u64, t: f32) -> Pos2 {
    let r = a.scene_rows[&id];
    pos2(r.center().x, r.top() + r.height() * t)
}

fn tree_order(a: &Operandi) -> Vec<u64> {
    a.editor.scene.tree_order()
}

#[test]
fn dragging_a_row_between_rows_reorders_it_without_parenting() {
    // UIC-143: the top band of a row inserts before it, at its level.
    let (mut a, ctx, ids) = four_items();
    let steps = a.editor.undo.len();
    let (from, to) = (row_at(&a, ids[3], 0.5), row_at(&a, ids[1], 0.1));
    drag(&mut a, &ctx, from, to);
    assert_eq!(tree_order(&a), [ids[0], ids[3], ids[1], ids[2]]);
    assert!(a.editor.scene.items.iter().all(|i| i.parent.is_none()));
    assert_eq!(a.editor.undo.len(), steps + 1, "one drop, one step");
    assert_eq!(a.editor.undo.last().unwrap().label, "Reorder");
    // The bottom band inserts after.
    let (from, to) = (row_at(&a, ids[0], 0.5), row_at(&a, ids[2], 0.9));
    drag(&mut a, &ctx, from, to);
    assert_eq!(tree_order(&a), [ids[3], ids[1], ids[2], ids[0]]);
    a.run(Command::Undo);
    assert_eq!(tree_order(&a), [ids[0], ids[3], ids[1], ids[2]]);
}

#[test]
fn dropping_on_the_middle_of_a_row_parents_and_into_an_open_parent_goes_first() {
    let (mut a, ctx, ids) = four_items();
    let (from, to) = (row_at(&a, ids[2], 0.5), row_at(&a, ids[0], 0.5));
    drag(&mut a, &ctx, from, to);
    let parent = |a: &Operandi, id: u64| a.editor.scene.item(id).unwrap().parent;
    assert_eq!(parent(&a, ids[2]), Some(ids[0]));
    assert_eq!(a.editor.undo.last().unwrap().label, "Parent");
    // Row 0 is open with child 2 under it: its bottom band puts 3 first.
    let (from, to) = (row_at(&a, ids[3], 0.5), row_at(&a, ids[0], 0.9));
    drag(&mut a, &ctx, from, to);
    assert_eq!(parent(&a, ids[3]), Some(ids[0]));
    assert_eq!(tree_order(&a), [ids[0], ids[3], ids[2], ids[1]]);
}

#[test]
fn a_multi_selection_drags_together_in_list_order_to_the_end() {
    let (mut a, ctx, ids) = four_items();
    // Selected 3 first, then 0.
    a.run(Command::SelectItems(vec![ids[3], ids[0]]));
    draw(&mut a, &ctx, SIZE, vec![]);
    let from = row_at(&a, ids[0], 0.5);
    let to = pos2(from.x, a.items_tree.body.bottom() - 4.);
    drag(&mut a, &ctx, from, to);
    assert_eq!(tree_order(&a), [ids[1], ids[2], ids[0], ids[3]]);
}

#[test]
fn dropping_a_parent_into_its_own_child_does_nothing() {
    let (mut a, ctx, ids) = four_items();
    a.run(Command::SetParent {
        item: ids[1],
        parent: Some(ids[0]),
        keep_position: true,
    });
    draw(&mut a, &ctx, SIZE, vec![]);
    let scene = a.editor.scene.clone();
    let steps = a.editor.undo.len();
    for t in [0.1, 0.5, 0.9] {
        let (from, to) = (row_at(&a, ids[0], 0.5), row_at(&a, ids[1], t));
        drag(&mut a, &ctx, from, to);
    }
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.editor.undo.len(), steps);
}
