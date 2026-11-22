//! Master plan phase 0 doors in the app: a kit item without its kit shows
//! in the Items list with its badge and draws (KIT-1), and viewport clicks
//! select through the one click path (AR-23).
use super::tests::draw;
use super::*;
use crate::items_tree::ItemKey;
use crate::tree_list::Badge;
use operandi_core::kit_item::KitItem;

const SIZE: Vec2 = vec2(1440., 900.);

fn app_with_kit_item() -> (Operandi, u64) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let item = a.editor.scene.active_mut().unwrap();
    item.name = "Splat".into();
    item.kit = Some(Box::new(KitItem {
        kit: "splat".into(),
        kind: "splat.cloud".into(),
        version: 1,
        values: serde_json::json!({"count": 10}),
        bounds: Some([DVec3::splat(-1.), DVec3::splat(1.)]),
    }));
    let id = item.id;
    (a, id)
}

#[test]
fn a_kit_item_without_its_kit_shows_its_badge_and_draws() {
    let (mut a, id) = app_with_kit_item();
    let row = a
        .item_rows()
        .into_iter()
        .find(|r| matches!(r.key, ItemKey::Item(_, i) if i == id))
        .expect("the item has a row");
    assert!(row.badges.contains(&Badge::MissingKit));
    assert!(row.tip.as_deref().unwrap().contains("Needs kit \"splat\""));
    // No active-mesh toggle: it isn't a mesh.
    assert!(!a.editor.scene.item(id).unwrap().is_mesh());
    // Drawn (as its box) with the rest of the frame, selected or not.
    let ctx = Context::default();
    draw(&mut a, &ctx, SIZE, vec![]);
    a.run(Command::SelectItem(None));
    draw(&mut a, &ctx, SIZE, vec![]);
}

#[test]
fn a_click_in_empty_space_clears_and_a_shift_click_does_not() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1)
            .unwrap()
            .into();
    a.run(Command::Mode(Mode::Polygon));
    let id = a.editor.scene.items[0].id;
    a.run(Command::Select {
        item: id,
        hit: operandi_core::Component::Polygon(0),
        replace: true,
        remove: false,
    });
    let ctx = Context::default();
    draw(&mut a, &ctx, SIZE, vec![]);
    let r = a.viewport_rect;
    let corner = r.min + vec2(5., 5.);
    let shift = Modifiers {
        shift: true,
        ..Modifiers::NONE
    };
    a.select_at(corner, r, shift);
    assert_eq!(a.editor.scene.selection.faces, [0]);
    a.select_at(corner, r, Modifiers::NONE);
    assert!(a.editor.scene.selection.faces.is_empty());
}
