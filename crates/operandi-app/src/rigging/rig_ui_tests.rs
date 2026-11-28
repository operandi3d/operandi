//! Rig phase 4 in the app: the Animate > Constrain / Hierarchy menus and
//! the Channel Links window run `operandi.rig.*` lines; driven channels
//! show the driven light; Compensation is kept in the preferences.
use super::*;
use crate::tests::{box_app, draw};

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

fn frames(a: &mut Operandi, ctx: &Context, n: usize) {
    for _ in 0..n {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
}

#[test]
fn the_animate_menu_has_the_rig_entries() {
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let a = box_app();
    let menus = a.menu_model(&ctx);
    let animate = menus
        .iter()
        .find(|m| m.title == "Animate")
        .expect("Animate menu");
    let titles: Vec<String> = animate
        .entries
        .iter()
        .filter_map(|e| match e {
            crate::app_menu::Entry::Submenu(m) => Some(m.title.clone()),
            crate::app_menu::Entry::Item(i) => Some(i.title.clone()),
            _ => None,
        })
        .collect();
    for want in ["Constrain", "Hierarchy", "Channel Links"] {
        assert!(titles.iter().any(|t| t == want), "{titles:?}");
    }
}

#[test]
fn channel_links_window_links_the_two_selected_items() {
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.new_mesh().unwrap();
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    let (driven, driver) = (ids[0], ids[1]);
    a.editor.scene.selection.also = vec![driven];
    a.editor.scene.selection.item = Some(driver);
    a.execute("operandi.rig.links");
    assert!(a.links.open);
    frames(&mut a, &ctx, 2);
    assert!(a.palettes.places.contains_key("channelLinks"));
    // The window's Add Link runs this line.
    a.execute(&format!(
        "operandi.rig.linkAdd driven:{driven}:scl.Z driver:{driver}:pos.X mode:direct"
    ));
    assert!(operandi_core::rig::driven_by(&a.editor.scene, driven, "scl.Z").is_some());
    frames(&mut a, &ctx, 1);
}

#[test]
fn compensation_is_kept_in_the_preferences_and_constraints_drive_fields() {
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    a.execute("operandi.rig.compensation on:true");
    frames(&mut a, &ctx, 1);
    assert!(a.undo_autosave.rig_compensation);
    a.editor.scene.new_mesh().unwrap();
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    a.execute(&format!(
        "operandi.rig.constrain kind:position items:{} targets:{}",
        ids[0], ids[1]
    ));
    assert!(operandi_core::rig::driven_by(&a.editor.scene, ids[0], "pos.X").is_some());
    frames(&mut a, &ctx, 2);
}

#[test]
fn the_schematic_draws_and_its_drops_become_rig_lines() {
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.new_mesh().unwrap();
    a.editor.scene.new_mesh().unwrap();
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    a.execute(&format!(
        "operandi.rig.modifierAdd kind:math item:{}",
        ids[1]
    ));
    a.execute("operandi.rig.schematicView");
    assert!(a.schematic.open);
    frames(&mut a, &ctx, 2);
    assert!(a.palettes.places.contains_key("schematic"));
    let model = operandi_core::rig_schematic::build(&a.editor.scene);
    let m = a.editor.scene.animation.rig.pieces[0].id;
    let (item0, item2, piece) = (
        format!("item:{}", ids[0]),
        format!("item:{}", ids[2]),
        format!("piece:{m}"),
    );
    let mut model = model;
    // Item nodes the user added.
    for id in [ids[0], ids[2]] {
        model.nodes.push(operandi_core::rig_schematic::Node {
            key: format!("item:{id}"),
            label: String::new(),
            id,
            piece: false,
            kind: None,
            detail: None,
            inputs: vec![],
            outputs: vec![],
            broken: false,
            column: 0,
        });
    }
    use crate::schematic_ui::connect_line;
    assert_eq!(
        connect_line(&model, &item0, "pos.X", &piece, "A").unwrap(),
        format!(
            "operandi.rig.modifierSet piece:{m} inputs:A={}:pos.X",
            ids[0]
        )
    );
    assert_eq!(
        connect_line(&model, &piece, "out", &item2, "rot.Y").unwrap(),
        format!("operandi.rig.modifierSet piece:{m} output:{}:rot.Y", ids[2])
    );
    assert_eq!(
        connect_line(&model, &item0, "pos.X", &item2, "scl.Z").unwrap(),
        format!(
            "operandi.rig.linkAdd driven:{}:scl.Z driver:{}:pos.X",
            ids[2], ids[0]
        )
    );
    assert!(connect_line(&model, &item0, "world", &piece, "A").is_err());
    let line = connect_line(&model, &item0, "pos.X", &piece, "A").unwrap();
    a.execute(&line);
    assert_eq!(
        operandi_core::rig_schematic::build(&a.editor.scene)
            .wires
            .len(),
        1
    );
    frames(&mut a, &ctx, 1);
}

#[test]
fn the_schematic_shows_one_page_and_saves_it_when_asked() {
    use operandi_core::schematic_pages::PageId;
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.new_mesh().unwrap();
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    a.execute(&format!(
        "operandi.rig.modifierAdd kind:expression item:{}",
        ids[1]
    ));
    a.execute("operandi.rig.schematicView");
    frames(&mut a, &ctx, 2);
    // No workspace yet: the whole rig on one unsaved page (SC-74).
    assert_eq!(a.schematic.page, Some(PageId::All));
    a.execute("operandi.rig.workspaceAdd name:Arms");
    frames(&mut a, &ctx, 1);
    let ws = a.editor.scene.animation.rig.workspaces[0].id;
    assert_eq!(
        a.schematic.page,
        Some(PageId::Saved(ws)),
        "the saved page replaces it"
    );
    // Fit runs with or without its argument (the button passes one).
    for line in [
        "operandi.rig.schematicFit",
        "operandi.rig.schematicFit selected:false",
    ] {
        a.execute(line);
        assert_eq!(a.schematic.fit, Some(false), "{line}");
        frames(&mut a, &ctx, 1);
        assert_eq!(a.schematic.fit, None, "consumed by the next frame");
    }
    a.execute("operandi.rig.schematicFit selected:true");
    assert_eq!(a.schematic.fit, Some(true));
}

#[test]
fn schematic_add_selected_lands_mid_view_and_says_what_it_did() {
    use operandi_core::schematic_pages::PageId;
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.new_mesh().unwrap();
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    a.execute(&format!(
        "operandi.rig.workspaceAdd name:Arms place:{{item:{}@0/0}}",
        ids[0]
    ));
    let ws = a.editor.scene.animation.rig.workspaces[0].id;
    a.execute("operandi.rig.schematicView");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.schematic.page, Some(PageId::Saved(ws)));
    let view = a.schematic_page(PageId::Saved(ws));
    let keys = [format!("item:{}", ids[0]), format!("item:{}", ids[1])];
    let mut lines = Vec::new();
    a.schematic_add(Some(PageId::Saved(ws)), &view, &keys, &mut lines);
    a.schematic_run(lines);
    let nodes = &a.editor.scene.animation.rig.workspaces[0].nodes;
    assert_eq!(nodes.len(), 2, "only the one not there yet");
    assert_eq!(
        a.schematic.note.as_deref(),
        Some("Added 1 node to this page")
    );
    let view = a.schematic_page(PageId::Saved(ws));
    let mut lines = Vec::new();
    a.schematic_add(Some(PageId::Saved(ws)), &view, &keys, &mut lines);
    assert!(lines.is_empty());
    assert_eq!(a.schematic.note.as_deref(), Some("Already on this page"));
}

/// #54: channels dragged from a Properties label and let go over the
/// Schematic come onto the page as ports of their item; a label that was
/// one of the selected channels brings them all. With no page yet, the drop
/// starts a workspace.
#[test]
fn channels_dropped_on_the_schematic_become_ports() {
    use crate::form::rows::ChannelDrag;
    use operandi_core::schematic_layout::Metrics;
    use operandi_core::schematic_pages::{self, PageId};
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let mut a = box_app();
    let id = a.editor.scene.items[0].id;
    a.execute("operandi.rig.schematicView");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.schematic.page, None, "no rig, no page yet");
    // Dropped on the empty Schematic: a workspace with the item on it.
    let drag = ChannelDrag {
        channels: vec![format!("{id}:pos.X")],
        selected: false,
    };
    let line = a
        .schematic_channels_line(&drag, egui::pos2(0., 0.))
        .unwrap();
    a.schematic.after_save.push(line);
    a.schematic_run(vec!["operandi.rig.workspaceAdd name:Workspace".into()]);
    let ws = a.editor.scene.animation.rig.workspaces[0].id;
    assert_eq!(a.schematic.page, Some(PageId::Saved(ws)));
    // The selected channels, from a selected label, onto that page.
    a.execute(&format!("operandi.channel.select channel:{id}:rot.Y"));
    a.execute(&format!(
        "operandi.channel.select channel:{id}:scl.Z add:true"
    ));
    let drag = ChannelDrag {
        channels: vec![format!("{id}:rot.Y")],
        selected: true,
    };
    let line = a
        .schematic_channels_line(&drag, egui::pos2(50., 50.))
        .unwrap();
    a.schematic_run(vec![line.replace("{ws}", &ws.to_string())]);
    let view = schematic_pages::page(&a.editor.scene, PageId::Saved(ws), &Metrics::default());
    let node = view
        .schematic
        .nodes
        .iter()
        .find(|n| n.key == format!("item:{id}"))
        .expect("the item is on the page");
    for port in ["pos.X", "rot.Y", "scl.Z"] {
        assert!(
            node.outputs.iter().any(|p| p == port),
            "{port}: {:?}",
            node.outputs
        );
    }
}
