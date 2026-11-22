//! Items list header (spec item-list-header v1). Acceptance criteria:
//!
//! 1. Add Item makes one item of the picked type at the origin with
//!    factory defaults, named after the type with " (2)", " (3)" … on a
//!    clash, alone selected (IL-03 1–3).
//! 2. It lands directly after the selected item as its sibling, also for a
//!    child selection; with nothing selected, at the end of the root list
//!    (IL-03 4). Adding a camera never changes the render camera.
//! 3. One undo step removes it, redo brings it back, and save/reopen keeps
//!    the list order (IL-03 5).
//! 4. Recent is newest first, moves a repeat to the top and keeps 8
//!    (IL-04; persistence is tested in the app).
//! 5. Patterns: `*` any run, `?` one character, case-insensitive whole-name
//!    match; type ticks and Limit to selection children narrow the matches;
//!    Select / Unselect / Hide are one undo step each; no match is an
//!    error that leaves the scene alone (IL-05).
//! 6. The list filter is view-only: type ticks, show-only-selected, text
//!    search, Flat, Flatten (dots), Sort by Name / Type; a filtered-out
//!    parent of a shown child stays as a greyed row (IL-06..08).
//!
//! Disabled menu entries and the header controls are tested in the app
//! (item_list_ui.rs).
use operandi_core::camera_light::LightKind;
use operandi_core::item_list::*;
use operandi_core::*;
use std::collections::BTreeSet;

fn editor_with(names: &[&str]) -> Editor {
    let mut e = Editor::default();
    for _ in names {
        e.scene.new_mesh().unwrap();
    }
    for (item, name) in e.scene.items.iter_mut().zip(names) {
        item.name = (*name).into();
    }
    e.next_item_id = e.scene.next_id;
    e.scene.select_item(None);
    e
}
fn names(e: &Editor) -> Vec<String> {
    e.scene.items.iter().map(|i| i.name.clone()).collect()
}
fn id(e: &Editor, name: &str) -> u64 {
    e.scene.items.iter().find(|i| i.name == name).unwrap().id
}

#[test]
fn add_item_makes_one_defaulted_item_named_uniquely_and_alone_selected() {
    let mut e = editor_with(&["A"]);
    for (kind, first, second) in [
        (AddItemKind::Mesh, "Mesh", "Mesh (2)"),
        (AddItemKind::Camera, "Camera", "Camera (2)"),
        (AddItemKind::Backdrop, "Backdrop Item", "Backdrop Item (2)"),
        (
            AddItemKind::Light(LightKind::Spot),
            "Spot Light",
            "Spot Light (2)",
        ),
        (
            AddItemKind::Light(LightKind::Portal),
            "Portal",
            "Portal (2)",
        ),
    ] {
        for name in [first, second] {
            e.dispatch(Command::AddItem(kind)).unwrap();
            let item = e.scene.active().unwrap();
            assert_eq!(item.name, name);
            assert_eq!(item.transform, scene::Transform::default());
            assert!(e.scene.selection.also.is_empty(), "alone selected");
        }
    }
    let item = e.scene.active().unwrap();
    assert_eq!(item.light.as_ref().unwrap().kind, LightKind::Portal);
}

#[test]
fn add_item_goes_after_the_selected_item_as_a_sibling() {
    let mut e = editor_with(&["A", "B", "C"]);
    e.dispatch(Command::SelectItem(Some(id(&e, "A")))).unwrap();
    e.dispatch(Command::AddItem(AddItemKind::Mesh)).unwrap();
    assert_eq!(names(&e), ["A", "Mesh", "B", "C"]);
    assert_eq!(e.scene.active().unwrap().parent, None);

    // A child selection: the new item shares its parent.
    let (b, c) = (id(&e, "B"), id(&e, "C"));
    e.scene.items.iter_mut().find(|i| i.id == c).unwrap().parent = Some(b);
    e.dispatch(Command::SelectItem(Some(c))).unwrap();
    e.dispatch(Command::AddItem(AddItemKind::Camera)).unwrap();
    assert_eq!(names(&e), ["A", "Mesh", "B", "C", "Camera"]);
    assert_eq!(e.scene.active().unwrap().parent, Some(b));

    // Nothing selected: end of the root list.
    e.dispatch(Command::SelectItem(None)).unwrap();
    e.dispatch(Command::AddItem(AddItemKind::Backdrop)).unwrap();
    assert_eq!(names(&e).last().unwrap(), "Backdrop Item");
    assert_eq!(e.scene.active().unwrap().parent, None);
}

#[test]
fn adding_a_camera_ahead_of_the_render_camera_keeps_it() {
    let mut e = editor_with(&["A"]);
    e.dispatch(Command::AddItem(AddItemKind::Camera)).unwrap();
    let first = e.scene.render_camera().unwrap();
    e.dispatch(Command::SelectItem(Some(id(&e, "A")))).unwrap();
    e.dispatch(Command::AddItem(AddItemKind::Camera)).unwrap();
    assert_eq!(names(&e), ["A", "Camera (2)", "Camera"]);
    assert_eq!(e.scene.render_camera(), Some(first));
}

#[test]
fn add_item_is_one_undo_step_and_survives_save_and_reopen() {
    let mut e = editor_with(&["A", "B"]);
    e.dispatch(Command::SelectItem(Some(id(&e, "A")))).unwrap();
    let undo = e.undo.len();
    e.dispatch(Command::AddItem(AddItemKind::Light(LightKind::Point)))
        .unwrap();
    assert_eq!(e.undo.len(), undo + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(names(&e), ["A", "B"]);
    assert_eq!(e.scene.selection.item, Some(id(&e, "A")));
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(names(&e), ["A", "Point Light", "B"]);

    let dir = std::env::temp_dir().join(format!("operandi-il-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("order.opscene");
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    let back: Vec<_> = back.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(back, ["A", "Point Light", "B"]);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn add_item_keys_round_trip() {
    for kind in AddItemKind::ALL {
        assert_eq!(AddItemKind::from_key(kind.key()), Some(kind));
    }
    assert_eq!(AddItemKind::from_key("locator"), None);
}

#[test]
fn recent_is_newest_first_moves_repeats_up_and_keeps_eight() {
    let mut recent = Vec::new();
    for kind in [
        AddItemKind::Light(LightKind::Spot),
        AddItemKind::Backdrop,
        AddItemKind::Mesh,
    ] {
        push_recent(&mut recent, kind);
    }
    push_recent(&mut recent, AddItemKind::Light(LightKind::Spot));
    assert_eq!(
        recent,
        [
            AddItemKind::Light(LightKind::Spot),
            AddItemKind::Mesh,
            AddItemKind::Backdrop
        ]
    );
    for kind in AddItemKind::ALL {
        push_recent(&mut recent, kind);
    }
    assert_eq!(recent.len(), RECENT_CAP);
    assert_eq!(recent[0], *AddItemKind::ALL.last().unwrap());
}

#[test]
fn patterns_use_star_and_question_mark_case_insensitively() {
    assert!(pattern_matches("*", "anything"));
    assert!(pattern_matches("*", ""));
    assert!(pattern_matches("cam*", "Camera (2)"));
    assert!(pattern_matches("CAM*", "camera"));
    assert!(!pattern_matches("cam", "Camera"), "whole name");
    assert!(pattern_matches("*light", "Spot Light"));
    assert!(pattern_matches("m?sh", "Mesh"));
    assert!(!pattern_matches("m?sh", "Meesh"));
    assert!(pattern_matches("*a*b*", "xxaYYb"));
    assert!(!pattern_matches("*a*b", "xxaYYbc"));
    assert!(pattern_matches("Mesh (?)", "Mesh (2)"));
}

#[test]
fn pattern_types_and_limit_to_children_narrow_matches() {
    let mut e = editor_with(&["Arm", "Hand", "Finger", "Leg"]);
    let (arm, hand, finger) = (id(&e, "Arm"), id(&e, "Hand"), id(&e, "Finger"));
    e.scene.items[1].parent = Some(arm);
    e.scene.items[2].parent = Some(hand);
    e.dispatch(Command::AddItem(AddItemKind::Camera)).unwrap();
    let mut query = PatternQuery::default();
    assert_eq!(e.scene.pattern_matches(&query).len(), 5);
    query.types.set(ItemType::Camera, false);
    assert_eq!(e.scene.pattern_matches(&query).len(), 4);
    query.types = TypeTicks::none();
    assert!(e.scene.pattern_matches(&query).is_empty());
    query.types = TypeTicks::none().invert();

    // Children at any depth of the selected items, not the items themselves.
    e.dispatch(Command::SelectItem(Some(arm))).unwrap();
    query.limit_to_children = true;
    assert_eq!(e.scene.pattern_matches(&query), [hand, finger]);
    query.pattern = "f*".into();
    assert_eq!(e.scene.pattern_matches(&query), [finger]);
    e.dispatch(Command::SelectItem(None)).unwrap();
    assert!(e.scene.pattern_matches(&query).is_empty());
}

#[test]
fn pattern_actions_are_one_undo_step_each() {
    let mut e = editor_with(&["Box", "Box (2)", "Ball"]);
    let query = |p: &str| PatternQuery {
        pattern: p.into(),
        ..Default::default()
    };
    let undo = e.undo.len();
    e.dispatch(Command::ItemPattern {
        query: query("b*"),
        action: PatternAction::Select,
    })
    .unwrap();
    assert_eq!(e.scene.selected_items().len(), 3);
    e.dispatch(Command::ItemPattern {
        query: query("box*"),
        action: PatternAction::Unselect,
    })
    .unwrap();
    assert_eq!(e.scene.selected_items(), [id(&e, "Ball")]);
    e.dispatch(Command::ItemPattern {
        query: query("box?(2)"),
        action: PatternAction::Hide,
    })
    .unwrap();
    assert!(!e.scene.items[1].visible && e.scene.items[0].visible);
    assert_eq!(e.undo.len(), undo + 3);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.items[1].visible);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selected_items().len(), 3);

    // No match: an error, nothing changes, no undo step.
    let before = e.scene.clone();
    let undo = e.undo.len();
    assert!(
        e.dispatch(Command::ItemPattern {
            query: query("zzz"),
            action: PatternAction::Select,
        })
        .is_err()
    );
    assert_eq!(e.scene, before);
    assert_eq!(e.undo.len(), undo);
}

/// Arm > Hand > Finger, Leg, Camera.
fn rig() -> Editor {
    let mut e = editor_with(&["Arm", "Hand", "Finger", "Leg"]);
    let (arm, hand) = (id(&e, "Arm"), id(&e, "Hand"));
    e.scene.items[1].parent = Some(arm);
    e.scene.items[2].parent = Some(hand);
    e.dispatch(Command::AddItem(AddItemKind::Camera)).unwrap();
    e.dispatch(Command::SelectItem(None)).unwrap();
    e
}
fn shown(e: &Editor, filter: &ListFilter) -> Vec<(String, usize, usize, bool)> {
    list_rows(&e.scene, filter, &BTreeSet::new())
        .into_iter()
        .map(|r| {
            let name = e.scene.items.iter().find(|i| i.id == r.id).unwrap();
            (name.name.clone(), r.depth, r.dots, r.greyed)
        })
        .collect()
}
fn row(name: &str, depth: usize, dots: usize, greyed: bool) -> (String, usize, usize, bool) {
    (name.into(), depth, dots, greyed)
}

#[test]
fn plain_filter_draws_the_tree_and_respects_collapse() {
    let e = rig();
    let filter = ListFilter::default();
    assert_eq!(
        shown(&e, &filter),
        [
            row("Arm", 1, 0, false),
            row("Hand", 2, 0, false),
            row("Finger", 3, 0, false),
            row("Leg", 1, 0, false),
            row("Camera", 1, 0, false),
        ]
    );
    let collapsed = BTreeSet::from([id(&e, "Arm")]);
    assert_eq!(list_rows(&e.scene, &filter, &collapsed).len(), 3);
}

#[test]
fn filters_hide_types_and_keep_greyed_parents() {
    let mut e = rig();
    let mut filter = ListFilter::default();
    filter.types.set(ItemType::Camera, false);
    assert_eq!(shown(&e, &filter).len(), 4);

    // Show-only-selected: the selected finger, its parents greyed.
    e.dispatch(Command::SelectItem(Some(id(&e, "Finger"))))
        .unwrap();
    let filter = ListFilter {
        only_selected: true,
        ..Default::default()
    };
    assert_eq!(
        shown(&e, &filter),
        [
            row("Arm", 1, 0, true),
            row("Hand", 2, 0, true),
            row("Finger", 3, 0, false),
        ]
    );

    // Text filter: matches with greyed parents; Flat drops the parents.
    let mut filter = ListFilter {
        text: Some("FING".into()),
        ..Default::default()
    };
    assert_eq!(shown(&e, &filter)[2], row("Finger", 3, 0, false));
    assert_eq!(shown(&e, &filter).len(), 3);
    filter.text_flat = true;
    assert_eq!(shown(&e, &filter), [row("Finger", 1, 0, false)]);
    // An empty search filters nothing.
    filter.text = Some("  ".into());
    assert_eq!(shown(&e, &filter).len(), 5);
}

#[test]
fn flatten_puts_dots_per_level_and_sorts_apply() {
    let e = rig();
    let filter = ListFilter {
        flatten: true,
        ..Default::default()
    };
    assert_eq!(
        shown(&e, &filter),
        [
            row("Arm", 1, 0, false),
            row("Hand", 1, 1, false),
            row("Finger", 1, 2, false),
            row("Leg", 1, 0, false),
            row("Camera", 1, 0, false),
        ]
    );
    let by_name = ListFilter {
        sort_by_name: true,
        ..Default::default()
    };
    let roots: Vec<_> = shown(&e, &by_name)
        .into_iter()
        .filter(|r| r.1 == 1)
        .map(|r| r.0)
        .collect();
    assert_eq!(roots, ["Arm", "Camera", "Leg"]);
    let by_type = ListFilter {
        sort_by_type: true,
        flatten: true,
        sort_by_name: true,
        ..Default::default()
    };
    let all: Vec<_> = shown(&e, &by_type).into_iter().map(|r| r.0).collect();
    assert_eq!(all, ["Camera", "Arm", "Finger", "Hand", "Leg"]);
}

#[test]
fn filtering_never_changes_the_scene() {
    let mut e = rig();
    e.dispatch(Command::SelectItem(Some(id(&e, "Leg"))))
        .unwrap();
    let before = e.scene.clone();
    for filter in [
        ListFilter {
            flatten: true,
            sort_by_name: true,
            sort_by_type: true,
            only_selected: true,
            text: Some("l".into()),
            text_flat: true,
            text_pattern: true,
            text_case: true,
            types: TypeTicks::none(),
            by_color: true,
            color: Some(operandi_core::EditorColor::Red),
        },
        ListFilter::default(),
    ] {
        let _ = list_rows(&e.scene, &filter, &BTreeSet::new());
    }
    assert_eq!(e.scene, before);
}
