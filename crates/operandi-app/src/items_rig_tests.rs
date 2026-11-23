//! RG-01 / IK-02 / SK-36 in the Items list: rig pieces, skins and joint
//! influences show under their items; select, rename, enable, delete,
//! and their menus.
use super::*;

/// A cube constrained to a second one, a two-joint chain bound to a mesh.
fn rigged() -> (Operandi, u64, u64, u64, u64) {
    crate::anim_switch::set_for_test(true);
    let mut a = crate::tests::box_app();
    let cube = a.editor.scene.items[0].id;
    a.run(Command::NewMesh);
    let target = a.editor.scene.items.last().unwrap().id;
    a.execute(&format!(
        "operandi.rig.constrain kind:position items:{cube} targets:{target}"
    ));
    let piece = a.editor.scene.animation.rig.pieces[0].id;
    a.execute("operandi.skel.add position:-0.5,0,0");
    let root = a.editor.scene.items.last().unwrap().id;
    a.execute(&format!("operandi.skel.add position:0.5,0,0 parent:{root}"));
    a.execute("operandi.anim.setup on:true");
    a.execute(&format!(
        "operandi.skin.bind items:{cube} root:{root} type:distance"
    ));
    a.execute("operandi.anim.setup on:false");
    (a, cube, target, piece, root)
}

fn here(a: &Operandi) -> u64 {
    a.scenes.active_tab().id
}

fn keys(a: &Operandi) -> Vec<ItemKey> {
    a.item_rows().into_iter().map(|r| r.key).collect()
}

fn event(a: &mut Operandi, ev: TreeEvent<ItemKey>) {
    let ctx = Context::default();
    let mut ev = Some(ev);
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ev) = ev.take() {
                a.item_event(ui, ev, &[]);
            }
        });
    });
}

#[test]
fn rg01_pieces_and_the_skin_show_under_their_items() {
    let (mut a, cube, _, piece, root) = rigged();
    let s = here(&a);
    let k = keys(&a);
    let at = |key: ItemKey| {
        k.iter()
            .position(|x| *x == key)
            .unwrap_or_else(|| panic!("{key:?} in {k:?}"))
    };
    let item = at(ItemKey::Item(s, cube));
    assert_eq!(at(ItemKey::Rig(s, piece)), item + 1, "right under its item");
    let op = a
        .editor
        .scene
        .item(cube)
        .unwrap()
        .mesh_stack
        .as_ref()
        .unwrap()
        .ops[0]
        .id;
    assert!(at(ItemKey::Skin(s, cube, op)) > item);
    let rows = a.item_rows();
    let rig = rows
        .iter()
        .find(|r| r.key == ItemKey::Rig(s, piece))
        .unwrap();
    assert_eq!(rig.name, "Position Constraint");
    assert_eq!(rig.depth, rows[item].depth + 1);
    // SK-36: the skin opens to its joints.
    assert!(
        !k.contains(&ItemKey::Influence(s, cube, root)),
        "closed at first"
    );
    a.item_disclosure(ItemKey::Skin(s, cube, op), false);
    assert!(keys(&a).contains(&ItemKey::Influence(s, cube, root)));
    // Folding the item hides its rig rows too.
    a.item_disclosure(ItemKey::Item(s, cube), false);
    assert!(!keys(&a).contains(&ItemKey::Rig(s, piece)));
}

#[test]
fn rig_rows_select_rename_enable_and_delete() {
    let (mut a, cube, target, piece, root) = rigged();
    let s = here(&a);
    a.run(Command::SelectItem(Some(target)));
    event(
        &mut a,
        TreeEvent::Click {
            key: ItemKey::Rig(s, piece),
            how: ClickHow::Only,
        },
    );
    assert_eq!(
        a.editor.scene.selection.item,
        Some(cube),
        "selects its item"
    );
    assert_eq!(a.rig_row, Some(ItemKey::Rig(s, piece)));
    let row = a
        .item_rows()
        .into_iter()
        .find(|r| r.key == ItemKey::Rig(s, piece))
        .unwrap();
    assert_eq!(row.selected, Selected::Primary);
    // Rename: one undo step; the Schematic and query show it too.
    let steps = a.editor.undo.len();
    event(
        &mut a,
        TreeEvent::Rename {
            key: ItemKey::Rig(s, piece),
            name: "Follow the box".into(),
        },
    );
    assert_eq!(a.editor.undo.len(), steps + 1);
    let row = a
        .item_rows()
        .into_iter()
        .find(|r| r.key == ItemKey::Rig(s, piece))
        .unwrap();
    assert_eq!(row.name, "Follow the box");
    // Enable toggle.
    event(
        &mut a,
        TreeEvent::Toggle {
            key: ItemKey::Rig(s, piece),
            column: EYE,
            solo: false,
        },
    );
    assert!(
        !a.editor
            .scene
            .animation
            .rig
            .piece(piece)
            .unwrap()
            .flag("enable")
    );
    event(
        &mut a,
        TreeEvent::Toggle {
            key: ItemKey::Rig(s, piece),
            column: EYE,
            solo: false,
        },
    );
    assert!(
        a.editor
            .scene
            .animation
            .rig
            .piece(piece)
            .unwrap()
            .flag("enable")
    );
    // Skin and influence toggles.
    let op = a
        .editor
        .scene
        .item(cube)
        .unwrap()
        .mesh_stack
        .as_ref()
        .unwrap()
        .ops[0]
        .id;
    event(
        &mut a,
        TreeEvent::Toggle {
            key: ItemKey::Skin(s, cube, op),
            column: EYE,
            solo: false,
        },
    );
    assert!(
        !a.editor
            .scene
            .item(cube)
            .unwrap()
            .mesh_stack
            .as_ref()
            .unwrap()
            .ops[0]
            .enabled,
        "{}",
        a.status
    );
    event(
        &mut a,
        TreeEvent::Toggle {
            key: ItemKey::Influence(s, cube, root),
            column: EYE,
            solo: false,
        },
    );
    let (_, v) = operandi_core::skin_commands::skin_op(&a.editor.scene, cube).unwrap();
    assert!(!v.enabled[v.joints.iter().position(|j| *j == root).unwrap()]);
    // The influence row selects its joint.
    event(
        &mut a,
        TreeEvent::Click {
            key: ItemKey::Influence(s, cube, root),
            how: ClickHow::Only,
        },
    );
    assert_eq!(a.editor.scene.selection.item, Some(root));
}

#[test]
fn uic60_rig_menus_are_never_empty_and_delete_works() {
    let (mut a, cube, _, piece, root) = rigged();
    let op = a
        .editor
        .scene
        .item(cube)
        .unwrap()
        .mesh_stack
        .as_ref()
        .unwrap()
        .ops[0]
        .id;
    let ctx = Context::default();
    let mut drawn = 0;
    for which in 0..3 {
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let before = ui.cursor().top();
                match which {
                    0 => a.rig_piece_menu(ui, piece, &mut || {}),
                    1 => a.skin_row_menu(ui, cube, op),
                    _ => a.influence_menu(ui, cube, root),
                }
                if ui.cursor().top() > before {
                    drawn += 1;
                }
            });
        });
    }
    assert_eq!(drawn, 3, "every rig menu shows entries");
    // Delete through the command the menu runs: one undo step.
    let steps = a.editor.undo.len();
    a.execute(&format!("operandi.rig.remove piece:{piece}"));
    assert_eq!(a.editor.undo.len(), steps + 1);
    assert!(!keys(&a).contains(&ItemKey::Rig(here(&a), piece)));
}
