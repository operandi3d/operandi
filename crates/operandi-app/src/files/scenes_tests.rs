//! Several open scenes (scenes.rs): switching, per-scene state, New, Open,
//! Close Scene, Close All, quit and the recovery journals.
use super::*;

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a
}

/// An edit that makes the scene dirty and leaves one undo step.
fn edit(a: &mut Operandi) {
    a.run(Command::NewMesh);
    assert!(a.dirty(), "{}", a.status);
}

fn scratch(a: &Operandi, name: &str) -> PathBuf {
    let dir = a.environment.paths().data_root().join("scenes-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn each_scene_keeps_its_own_history_selection_path_and_dirty_state() {
    let mut a = app();
    edit(&mut a);
    let file = scratch(&a, "first.opscene");
    assert!(a.save(file.clone()));
    edit(&mut a);
    let first = a.editor.scene.clone();
    let first_undo = a.editor.history.undo.len();

    a.request_file(FileAction::New);
    assert_eq!(a.scenes.tabs.len(), 2);
    assert_eq!(a.scenes.active, 1);
    assert!(a.path.is_none() && !a.dirty());
    assert!(
        a.editor.history.undo.is_empty(),
        "a new scene starts its own history"
    );
    // The other scene was saved, so no Untitled scene is open.
    assert_eq!(a.scene_label(), "Untitled");
    edit(&mut a);
    a.editor.scene.selection.mode = Mode::Edge;
    let second = a.editor.scene.clone();

    // Undo here leaves the first scene alone.
    a.run(Command::Undo);
    assert!(a.switch_scene(0));
    assert_eq!(a.editor.scene, first);
    assert_eq!(a.editor.history.undo.len(), first_undo);
    assert_eq!(a.path.as_deref(), Some(file.as_path()));
    assert!(a.dirty());
    assert_eq!(a.scene_label(), "first.opscene");

    assert!(a.switch_scene(1));
    a.run(Command::Redo);
    assert_eq!(a.editor.scene.items, second.items);
    assert!(a.path.is_none());
}

#[test]
fn a_live_tool_stays_with_its_scene() {
    let mut a = app();
    a.execute("Cone");
    assert!(a.editor.session.cone.is_some(), "{}", a.status);
    a.fields.insert("cone_sides".into(), "7".into());
    a.request_file(FileAction::New);
    assert!(a.editor.session.cone.is_none(), "the new scene has no tool");
    assert!(a.fields.is_empty(), "nor the old scene's field drafts");
    assert!(a.switch_scene(0));
    assert!(a.editor.session.cone.is_some(), "Cone is still armed");
    assert_eq!(a.fields.get("cone_sides").map(String::as_str), Some("7"));
}

/// Astra P2-001: a scene's open tool keeps its own tweak history and step
/// counts while another scene is edited; Undo and Redo after switching back
/// act as if the switch never happened.
#[test]
fn a_live_tools_tweak_history_stays_with_its_scene() {
    use operandi_core::{HistoryRequest, Parameters};
    fn drag(a: &mut Operandi, delta: DVec3) {
        let base = a.editor.session.live.as_ref().unwrap().params.clone();
        a.editor.session.tweak_held = true;
        for t in [0.5, 1.] {
            a.editor
                .dispatch(Command::Preview(Parameters {
                    delta: delta * t,
                    ..base.clone()
                }))
                .unwrap();
        }
        a.editor.end_tweak();
        a.editor.dispatch(Command::CommitTransformStep).unwrap();
    }
    let mut a = app();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
            .unwrap()
            .into();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    a.editor.synchronize_identities().unwrap();
    let original = a.editor.scene.clone();
    a.editor.dispatch(Command::Activate(Tool::Move)).unwrap();
    drag(&mut a, DVec3::Y * 0.25);
    let first = a.editor.scene.clone();
    drag(&mut a, DVec3::Y * 0.5);
    let second = a.editor.scene.clone();

    // Another scene, with its own tool and tweaks.
    a.request_file(FileAction::New);
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
            .unwrap()
            .into();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![1];
    a.editor.synchronize_identities().unwrap();
    a.editor.dispatch(Command::Activate(Tool::Move)).unwrap();
    drag(&mut a, DVec3::X);
    assert!(a.switch_scene(0));

    // Each Undo takes back one of this scene's drags, Move still open.
    let undo = |a: &mut Operandi| {
        a.editor
            .dispatch_history_reported(HistoryRequest::Undo)
            .unwrap()
    };
    undo(&mut a);
    assert_eq!(a.editor.scene, first);
    assert!(
        a.editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool == Tool::Move)
    );
    undo(&mut a);
    assert_eq!(a.editor.scene, original);
    a.editor
        .dispatch_history_reported(HistoryRequest::Redo)
        .unwrap();
    a.editor
        .dispatch_history_reported(HistoryRequest::Redo)
        .unwrap();
    assert_eq!(a.editor.scene, second);
    // The other scene's drag is still its own to undo.
    assert!(a.switch_scene(1));
    let moved = a.editor.scene.clone();
    undo(&mut a);
    assert_ne!(a.editor.scene, moved, "the other scene's Undo still works");
}

#[test]
fn open_fills_an_untouched_scene_and_switches_to_an_open_file() {
    let mut a = app();
    edit(&mut a);
    let file = scratch(&a, "open-me.opscene");
    assert!(a.save(file.clone()));
    a.request_file(FileAction::CloseScene);
    assert_eq!(a.scenes.tabs.len(), 1);
    assert!(a.path.is_none(), "the last scene closes into a blank one");

    // The blank start scene is untouched: Open uses it.
    a.open(file.clone());
    assert_eq!(a.scenes.tabs.len(), 1);
    assert_eq!(a.path.as_deref(), Some(file.as_path()));

    // A second scene, then Open of the same file switches back to it.
    a.request_file(FileAction::New);
    edit(&mut a);
    a.open(file.clone());
    assert_eq!(a.scenes.tabs.len(), 2, "no second copy");
    assert_eq!(a.scenes.active, 0);
    assert_eq!(a.path.as_deref(), Some(file.as_path()));

    // Open from a touched scene adds a tab.
    edit(&mut a);
    let other = scratch(&a, "other.opscene");
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    persistence::save(&other, &scene).unwrap();
    a.open(other.clone());
    assert_eq!(a.scenes.tabs.len(), 3);
    assert_eq!(a.path.as_deref(), Some(other.as_path()));
}

#[test]
fn close_scene_asks_for_unsaved_work_and_leaves_one_blank_scene() {
    let mut a = app();
    a.request_file(FileAction::New);
    edit(&mut a);
    a.request_file(FileAction::CloseScene);
    assert!(matches!(a.pending, Some(FileAction::CloseScene)), "asks");
    a.pending = None; // Cancel
    assert_eq!(a.scenes.tabs.len(), 2);
    a.request_file(FileAction::CloseScene);
    a.pending = None;
    a.do_file(FileAction::CloseScene); // Discard
    assert_eq!(a.scenes.tabs.len(), 1);
    assert_eq!(a.scene_label(), "Untitled");

    // The last scene: a new blank Untitled one replaces it.
    edit(&mut a);
    a.request_file(FileAction::CloseScene);
    a.pending = None;
    a.do_file(FileAction::CloseScene);
    assert_eq!(a.scenes.tabs.len(), 1);
    assert!(!a.dirty() && a.editor.history.undo.is_empty());
    assert_eq!(
        a.editor.scene.items.len(),
        4,
        "a new scene's empty mesh, camera, light and environment"
    );
}

#[test]
fn untitled_numbers_follow_the_open_scenes() {
    let mut a = app();
    a.request_file(FileAction::New);
    a.request_file(FileAction::New);
    assert_eq!(a.scene_label(), "Untitled (3)");
    assert!(a.switch_scene(1));
    a.request_file(FileAction::CloseScene);
    a.request_file(FileAction::New);
    assert_eq!(a.scene_label(), "Untitled (4)", "gaps are never reused");
    // Closing every Untitled scene starts the count again.
    a.request_close_all();
    a.scenes.confirm_close_all = false;
    a.scenes.closing_all = true;
    a.close_all_step();
    assert_eq!(a.scene_label(), "Untitled");
    a.request_file(FileAction::New);
    assert_eq!(a.scene_label(), "Untitled (2)");
}

#[test]
fn quit_asks_about_each_unsaved_scene_and_cancel_keeps_everything() {
    let mut a = app();
    edit(&mut a);
    let first = a.editor.scene.clone();
    a.request_file(FileAction::New);
    edit(&mut a);
    let second = a.editor.scene.clone();
    a.request_file(FileAction::New); // clean third scene

    a.request_file(FileAction::Close);
    assert!(matches!(a.pending, Some(FileAction::Close)));
    assert_eq!(a.scenes.active, 0, "the first unsaved scene asks first");
    a.pending = None;
    a.do_file(FileAction::Close); // Discard
    assert!(matches!(a.pending, Some(FileAction::Close)));
    assert_eq!(a.scenes.active, 1, "then the next");
    a.pending = None; // Cancel: nothing is lost
    assert!(!a.allow_close);
    assert_eq!(a.editor.scene, second);
    assert!(a.switch_scene(0));
    assert_eq!(a.editor.scene, first, "a discard is only final on quit");

    // Again, discarding both: the app quits and leaves no journal behind.
    a.journal();
    assert!(a.journal_path().exists());
    a.request_file(FileAction::Close);
    a.pending = None;
    a.do_file(FileAction::Close);
    a.pending = None;
    a.do_file(FileAction::Close);
    assert!(a.allow_close);
    assert!(scenes::journal_files(&a.environment).is_empty());
}

#[test]
fn close_all_confirms_then_closes_every_scene() {
    let mut a = app();
    a.request_file(FileAction::New);
    edit(&mut a);
    a.request_file(FileAction::New);
    a.request_close_all();
    assert!(a.scenes.confirm_close_all && a.has_modal());
    a.scenes.confirm_close_all = false;
    a.scenes.closing_all = true;
    a.close_all_step();
    // The clean third scene closed; the unsaved second one asks.
    assert!(matches!(a.pending, Some(FileAction::CloseScene)));
    assert_eq!(a.scenes.tabs.len(), 2);
    a.pending = None;
    a.do_file(FileAction::CloseScene); // Discard
    assert!(!a.scenes.closing_all);
    assert_eq!(a.scenes.tabs.len(), 1);
    assert!(!a.dirty() && a.path.is_none());
}

#[test]
fn every_open_scene_has_its_own_journal_and_all_are_recovered() {
    let mut a = app();
    edit(&mut a);
    a.journal();
    let first = a.editor.scene.clone();
    a.request_file(FileAction::New);
    edit(&mut a);
    edit(&mut a);
    a.journal();
    let second = a.editor.scene.clone();
    assert_eq!(scenes::journal_files(&a.environment).len(), 2);

    // A new run finds both and recovers each into its own tab.
    let mut b = Operandi::with_environment(a.environment.clone());
    b.initialize_new_document();
    b.read_recovery();
    assert_eq!(b.recovery.as_ref(), Some(&first));
    assert_eq!(b.recovery_more, vec![second.clone()]);
    let scene = b.recovery.take().unwrap();
    b.editor.scene = scene;
    let more = std::mem::take(&mut b.recovery_more);
    b.recover_extra_scenes(more);
    assert_eq!(b.scenes.tabs.len(), 2);
    assert_eq!(b.editor.scene, second);
    assert!(b.dirty());
    assert!(b.switch_scene(0));
    assert_eq!(b.editor.scene, first);
}

/// A new run on `a`'s data folder that has read the journals, as at launch.
fn relaunch(a: &Operandi) -> Operandi {
    let mut b = Operandi::with_environment(a.environment.clone());
    b.initialize_new_document();
    b.read_recovery();
    b
}

/// Astra P7-001: Cancel on Recover Scene keeps the journals for the next
/// launch, and closing or quitting this session must not delete them.
#[test]
fn cancelled_recovery_survives_close_scene_and_quit() {
    let mut a = app();
    edit(&mut a);
    a.journal();
    let first = a.editor.scene.clone();
    a.request_file(FileAction::New);
    edit(&mut a);
    a.journal();
    let second = a.editor.scene.clone();
    assert_eq!(scenes::journal_files(&a.environment).len(), 2);

    // Cancel, then Close Scene on the (clean) new scene.
    let mut b = relaunch(&a);
    b.keep_journals();
    b.close_active_scene();
    assert_eq!(scenes::journal_files(&b.environment).len(), 2);
    // Cancel, then edit, discard and quit.
    let mut c = relaunch(&a);
    c.keep_journals();
    edit(&mut c);
    c.continue_quit();
    assert!(c.allow_close);
    assert_eq!(scenes::journal_files(&c.environment).len(), 2);
    // Both journals are still offered.
    let d = relaunch(&a);
    assert_eq!(d.recovery.as_ref(), Some(&first));
    assert_eq!(d.recovery_more, vec![second]);
}

/// Astra P7-002: a damaged first journal doesn't hide the healthy ones,
/// and Recover or Discard never deletes a journal that couldn't be read.
#[test]
fn a_damaged_journal_neither_hides_healthy_ones_nor_gets_deleted() {
    let mut a = app();
    edit(&mut a);
    a.journal();
    a.request_file(FileAction::New);
    edit(&mut a);
    a.journal();
    let second = a.editor.scene.clone();
    let files = scenes::journal_files(&a.environment);
    assert_eq!(files.len(), 2);
    std::fs::write(&files[0], b"{ not a scene").unwrap();

    let mut b = relaunch(&a);
    let warning = b
        .recovery_warning
        .clone()
        .expect("the damaged one is reported");
    assert!(warning.contains("Diagnostic copy"), "{warning}");
    assert_eq!(
        b.recovery.as_ref(),
        Some(&second),
        "the healthy one is offered"
    );
    assert!(b.recovery_more.is_empty());
    b.recovery_warning = None; // OK on the notice
    b.recover_journals();
    assert_eq!(b.editor.scene, second);
    // The recovered scene journals under the open tab's slot; the second
    // slot's old journal went with the recovery.
    assert!(b.journal_path().exists());
    assert!(!files[1].exists(), "the recovered journal was replaced");
    let copies = std::fs::read_dir(files[0].parent().unwrap())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains("-corrupt-"))
        .count();
    assert_eq!(copies, 1, "the damaged journal was copied aside");

    // Discard deletes only what it read: a damaged second journal stays.
    let mut c = app();
    edit(&mut c);
    c.journal();
    c.request_file(FileAction::New);
    edit(&mut c);
    c.journal();
    let files = scenes::journal_files(&c.environment);
    std::fs::write(&files[1], b"garbage").unwrap();
    let mut d = relaunch(&c);
    assert!(d.recovery.is_some() && d.recovery_warning.is_some());
    d.discard_journals();
    assert!(!files[0].exists());
    assert!(files[1].exists(), "an unreadable journal is never deleted");
}

#[test]
fn command_w_closes_the_scene_not_the_app() {
    let events = [Event::Key {
        key: Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: crate::tests::PLATFORM_CMD,
    }];
    assert!(matches!(
        input::file_shortcut(&events),
        Some(input::FileShortcut::Close)
    ));
    let shifted = [Event::Key {
        key: Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: crate::tests::PLATFORM_CMD | Modifiers::SHIFT,
    }];
    assert!(matches!(
        input::file_shortcut(&shifted),
        Some(input::FileShortcut::CloseAll)
    ));
}

#[test]
fn items_list_rows_switch_create_and_close_scenes() {
    use scenes::SceneRowAction;
    let mut a = app();
    edit(&mut a);
    let first = a.editor.scene.clone();
    a.scene_row_action(SceneRowAction::New);
    assert_eq!(a.scenes.tabs.len(), 2);
    assert_eq!(a.scene_row_name(0), "Untitled");
    assert_eq!(a.scene_row_name(1), "Untitled (2)");

    // The list draws every scene's row without a tab strip.
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        crate::tests::draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }

    a.scene_row_action(SceneRowAction::Switch(0));
    assert_eq!(a.scenes.active, 0);
    assert_eq!(a.editor.scene, first);

    // Clicking an item listed under another scene makes that scene current
    // and selects the item.
    let item = a.editor.scene.items[0].id;
    a.scene_row_action(SceneRowAction::Switch(1));
    a.scene_row_action(SceneRowAction::SelectItem(0, item));
    assert_eq!(a.scenes.active, 0);
    assert_eq!(a.editor.scene.selection.item, Some(item));
    for _ in 0..2 {
        crate::tests::draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }

    // Right-click entries on another scene's item act in that scene.
    a.scene_row_action(SceneRowAction::Switch(1));
    a.scene_row_action(SceneRowAction::Item(0, item, scenes::ItemMenu::Delete));
    assert_eq!(a.scenes.active, 0);
    assert!(a.editor.scene.items.iter().all(|i| i.id != item));
    a.run(Command::Undo);

    // A row's Close Scene makes that scene current, then closes it.
    a.scene_row_action(SceneRowAction::Close(1));
    assert_eq!(a.scenes.tabs.len(), 1);
    assert_eq!(a.editor.scene.items, first.items);
}

#[test]
fn dragging_items_between_scenes_copies_or_moves_them() {
    use scenes::SceneRowAction;
    let mut a = app();
    let first_id = a.scenes.tabs[0].id;
    let item = a.editor.scene.items[0].id;
    a.request_file(FileAction::New);
    let second_id = a.scenes.tabs[1].id;
    let host = a.editor.scene.items[0].id;
    assert!(a.switch_scene(0));

    // Copy (the default) onto an item of the other scene.
    a.scene_row_action(SceneRowAction::Transfer {
        from: first_id,
        item,
        to: second_id,
        parent: Some(host),
        moving: false,
    });
    assert_eq!(a.scenes.active, 1, "the receiving scene is current");
    // New scenes hold a mesh, a camera, a light (CL-1) and an
    // environment (ENV §1).
    assert_eq!(a.editor.scene.items.len(), 5, "{}", a.status);
    let arrived = a.editor.scene.items[4].clone();
    assert_eq!(arrived.parent, Some(host));
    assert_eq!(a.editor.scene.selection.item, Some(arrived.id));
    assert!(a.status.starts_with("Copied 1 item"), "{}", a.status);
    assert!(a.switch_scene(0));
    assert_eq!(a.editor.scene.items.len(), 4, "a copy leaves the source");

    // Move (Option) onto the other scene's row: the source loses it.
    a.scene_row_action(SceneRowAction::Transfer {
        from: first_id,
        item,
        to: second_id,
        parent: None,
        moving: true,
    });
    assert_eq!(a.scenes.active, 1);
    assert_eq!(a.editor.scene.items.len(), 6);
    assert!(a.switch_scene(0));
    assert!(
        a.editor.scene.items.iter().all(|i| i.id != item),
        "moved out"
    );
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.items.len(), 4, "undone in its own scene");

    // The list draws drop targets for the other scene's rows.
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        crate::tests::draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    assert!(
        a.scenes
            .drop_rects
            .iter()
            .any(|&(_, s, i)| s == second_id && i.is_none())
    );
    assert!(
        a.scenes
            .drop_rects
            .iter()
            .any(|&(_, s, i)| s == second_id && i == Some(host))
    );
}

/// GL-Q4 (Jason, 2026-11-08): Delete asks about child items only when
/// there are some; Shift+Escape (Cancel) changes nothing, Return (Yes)
/// deletes them too, Escape (No) moves them to the root in place (GL-Q7).
/// Each answer is one undo step.
#[test]
fn delete_asks_about_child_items_only_when_there_are_some() {
    use egui::accesskit::Role;
    let size = vec2(1440., 900.);
    let mut a = app();
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    let frame = |a: &mut Operandi, events: Vec<Event>| {
        crate::tests::draw(a, &ctx, size, events)
            .platform_output
            .accesskit_update
            .map(|u| u.nodes.into_iter().map(|(_, n)| n).collect::<Vec<_>>())
            .unwrap_or_default()
    };
    a.run(Command::NewMesh);
    let child = a.editor.scene.selection.item.unwrap();
    a.run(Command::ParentToGroupLocator);
    let group = a.editor.scene.selection.item.unwrap();
    // A childless item: no question, deleted at once.
    a.run(Command::AddItem(
        operandi_core::item_list::AddItemKind::GroupLocator,
    ));
    let lone = a.editor.scene.selection.item.unwrap();
    a.request_delete();
    assert!(!a.scenes.confirm_delete_children);
    assert!(a.editor.scene.items.iter().all(|i| i.id != lone));

    a.run(Command::SelectItem(Some(group)));
    let before = a.editor.scene.clone();
    let steps = a.editor.history.undo.len();
    a.request_delete();
    assert!(a.scenes.confirm_delete_children);
    let nodes = frame(&mut a, vec![]);
    let text = |n: &egui::accesskit::Node| {
        format!(
            "{}{}",
            n.label().unwrap_or_default(),
            n.value().unwrap_or_default()
        )
    };
    let asked = nodes
        .iter()
        .any(|n| text(n).contains("The item Group Locator has child items"));
    assert!(
        asked,
        "{:?}",
        nodes
            .iter()
            .map(text)
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
    );
    // Shift+Escape: Cancel, nothing changes (UIC-96).
    frame(
        &mut a,
        vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::SHIFT,
        }],
    );
    frame(&mut a, vec![]);
    assert!(!a.scenes.confirm_delete_children);
    assert_eq!(a.editor.scene, before);
    assert_eq!(a.editor.history.undo.len(), steps);

    // No (Escape, GL-Q7): the child goes to the root.
    a.request_delete();
    let nodes = frame(&mut a, vec![]);
    for label in ["Yes", "No", "Cancel"] {
        assert!(
            nodes
                .iter()
                .any(|n| n.role() == Role::Button && n.label() == Some(label)),
            "{label} button"
        );
    }
    frame(&mut a, vec![crate::tests::key(Key::Escape)]);
    frame(&mut a, vec![]);
    assert!(!a.scenes.confirm_delete_children, "{}", a.status);
    assert!(a.editor.scene.items.iter().all(|i| i.id != group));
    let kept = a.editor.scene.item(child).unwrap();
    assert_eq!(kept.parent, None);
    assert_eq!(a.editor.history.undo.len(), steps + 1);

    // Return (Yes): the children go too.
    a.run(Command::Undo);
    a.run(Command::SelectItem(Some(group)));
    a.request_delete();
    frame(&mut a, vec![]);
    frame(&mut a, vec![]);
    frame(&mut a, vec![crate::tests::key(Key::Enter)]);
    frame(&mut a, vec![]);
    assert!(!a.scenes.confirm_delete_children);
    assert!(
        a.editor
            .scene
            .items
            .iter()
            .all(|i| i.id != group && i.id != child)
    );
}

/// Astra P8-002: a parked scene whose Ellipsoid preview waits to be applied
/// needs saving by the same rule as the open scene, so Quit asks about it.
#[test]
fn a_parked_numeric_preview_counts_as_unsaved() {
    let mut a = super::tests::box_app();
    let file = scratch(&a, "preview.opscene");
    assert!(a.save(file));
    assert!(!a.dirty());
    let r = eframe::egui::Rect::from_min_size(eframe::egui::Pos2::ZERO, vec2(800., 600.));
    a.run(Command::ArmEllipsoid);
    let plane = a.actual_plane();
    let center = a.camera.project(DVec3::new(2., 0., 0.), r).unwrap().0;
    let edge = a.camera.project(DVec3::new(3., 0., 0.), r).unwrap().0;
    let input::GestureKind::SolidDraw { anchor, .. } = a.begin_solid_draw(false, center, r) else {
        panic!("no draw gesture: {}", a.status);
    };
    a.solid_draw(false, plane, anchor, edge, r, true);
    assert!(
        a.editor
            .session
            .ellipsoid
            .as_ref()
            .unwrap()
            .preview
            .is_some()
    );
    assert!(a.dirty(), "the open scene counts its preview");
    a.request_file(FileAction::New);
    assert!(a.tab_dirty(0), "and so does the same scene parked");
}

/// Astra P8-003: the shader Delete question counts as an open dialog, so a
/// queued Open or the MCP bridge waits for its answer.
#[test]
fn the_shader_delete_question_is_modal() {
    let mut a = app();
    assert!(!a.has_modal());
    a.shading.confirm_delete = Some(Vec::new());
    assert!(a.has_modal());
    assert_eq!(a.open_dialog(), Some("shader Delete question"));
}

/// Astra P2-002: geometry generations never restart, so after a tab
/// switch rebuilds the viewport caches, the same item id in the other
/// scene never gets a generation the GPU already holds for this one.
#[test]
fn geometry_generations_differ_across_rebuilt_caches() {
    let a = super::tests::box_app();
    let item = a.editor.scene.active().unwrap().clone();
    let matrix = a.editor.scene.world_matrix(item.id).unwrap();
    let build = || {
        crate::gpu_scene::Resident::default()
            .item(&item, matrix, None, None, &a.editor.scene.smoothing)
            .1
    };
    let first = build();
    let second = build();
    assert_ne!(first, second, "a fresh cache must not reuse a generation");
}
