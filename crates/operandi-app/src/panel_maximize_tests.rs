//! MN-08: the Maximise keys over any docked panel.
use super::tests::draw;
use super::*;
use operandi_layout::Pane;

const SIZE: Vec2 = vec2(1440., 900.);

fn press_maximise(a: &mut Operandi, ctx: &Context, over: Pos2) {
    // Numpad 0, as the window reports it.
    a.native_numbers = vec![native::NumberKey {
        digit: 0,
        keypad: true,
        pressed: true,
    }];
    let key = |pressed| Event::Key {
        key: Key::Num0,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    draw(
        a,
        ctx,
        SIZE,
        vec![Event::PointerMoved(over), key(true), key(false)],
    );
    a.native_numbers.clear();
    draw(a, ctx, SIZE, vec![]);
    draw(a, ctx, SIZE, vec![]);
}

fn rect(a: &Operandi, pane: Pane) -> Option<Rect> {
    if !a.layouts.mixed_dock {
        return a.side_panel_rects.get(&pane).copied();
    }
    let tree = &a.views.main;
    let id = tree.tiles.find_pane(&pane)?;
    tree.tiles.rect(id)
}

fn setup() -> (Operandi, Context) {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    for _ in 0..3 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx)
}

#[test]
fn the_items_list_fills_its_group_and_comes_back() {
    let (mut a, ctx) = setup();
    let items = rect(&a, Pane::Items).expect("Items is docked");
    press_maximise(&mut a, &ctx, items.center());
    assert_eq!(a.panel_maximized, Some(Pane::Items), "{}", a.status);
    let big = rect(&a, Pane::Items).unwrap();
    assert!(big.area() > items.area() * 1.2, "{items:?} -> {big:?}");
    // Again, over the same panel: the group as it was.
    press_maximise(&mut a, &ctx, big.center());
    assert_eq!(a.panel_maximized, None, "{}", a.status);
    let back = rect(&a, Pane::Items).unwrap();
    assert!(
        (back.height() - items.height()).abs() < 2.,
        "{items:?} {back:?}"
    );
    // No undo step: layout, not scene.
    assert!(a.status.contains("restored"), "{}", a.status);
}

#[test]
fn the_pointer_picks_the_panel_and_views_keep_their_own() {
    let (mut a, ctx) = setup();
    let properties = rect(&a, Pane::Properties).expect("Properties is docked");
    press_maximise(&mut a, &ctx, properties.center());
    assert_eq!(a.panel_maximized, Some(Pane::Properties), "{}", a.status);
    let at = rect(&a, Pane::Properties).unwrap().center();
    press_maximise(&mut a, &ctx, at);
    assert_eq!(a.panel_maximized, None);
    // Over a 3D view the key is the view's Maximise, as before.
    let view = a.viewport_rect.center();
    press_maximise(&mut a, &ctx, view);
    assert_eq!(a.panel_maximized, None);
}

#[test]
fn a_focused_text_field_takes_the_key() {
    let (mut a, ctx) = setup();
    let items = rect(&a, Pane::Items).unwrap();
    // Focus some text field: the key goes to typing, not Maximise.
    let id = egui::Id::new("mn08-text");
    a.text_editors.insert(id);
    ctx.memory_mut(|m| m.request_focus(id));
    press_maximise(&mut a, &ctx, items.center());
    assert_eq!(a.panel_maximized, None);
}

#[test]
fn tool_properties_fills_the_left_column() {
    let (mut a, ctx) = setup();
    let props = rect(&a, Pane::ToolProperties).expect("Tool Properties shows");
    press_maximise(&mut a, &ctx, props.center());
    assert_eq!(
        a.panel_maximized,
        Some(Pane::ToolProperties),
        "{}",
        a.status
    );
    assert!(rect(&a, Pane::ToolProperties).unwrap().height() > props.height() * 1.2);
    assert_eq!(rect(&a, Pane::Toolbox), None, "the drawer gave way");
}

#[test]
fn in_the_dock_tree_a_panels_group_is_its_split() {
    // A right column: Items over a tab group (Shader Tree, Groups) over
    // Properties.
    let mut tiles = egui_tiles::Tiles::default();
    let items = tiles.insert_pane(Pane::Items);
    let shader = tiles.insert_pane(Pane::DockTab(1));
    let groups = tiles.insert_pane(Pane::DockTab(2));
    let tabs = tiles.insert_tab_tile(vec![shader, groups]);
    let props = tiles.insert_pane(Pane::Properties);
    let column = tiles.insert_vertical_tile(vec![items, tabs, props]);
    let view = tiles.insert_pane(Pane::Viewport(0));
    let root = tiles.insert_horizontal_tile(vec![view, column]);
    let tree = egui_tiles::Tree::new("mn08", root, tiles);
    assert_eq!(
        panel_maximize::siblings(&tree, Pane::Items),
        vec![tabs, props]
    );
    // A tab maximises its whole tab group within the column.
    assert_eq!(
        panel_maximize::siblings(&tree, Pane::DockTab(2)),
        vec![items, props]
    );
}
