//! Group Locators (GL-01..GL-08, provisional design). Acceptance criteria:
//!
//! 1. Add Item > Group Locator makes one empty, transform-only item named
//!    "Group Locator" (" (2)" on a clash) at the origin, alone selected,
//!    placed after the selected item; one undo step, redo brings it back.
//! 2. Item-mode Move, Rotate and Scale on the locator carry a child
//!    mesh's world points with it; the child's own transform is untouched.
//! 3. Parent to Group Locator with one shared parent puts the locator
//!    under that parent with an identity transform, every child keeping
//!    its local transform exactly; the locator takes the first selected
//!    item's list place and ends up the only selection; one undo step.
//! 4. With mixed parents the locator is top level at the origin and every
//!    child keeps its world placement; a child that would need shear
//!    refuses the whole command and leaves the scene alone.
//! 5. No selection is an error with no undo step.
//! 6. Component tools refuse a Group Locator with the no-geometry message;
//!    Move switches to Item mode for it.
//! 7. Delete of an item with children (GL-Q4, Jason 2026-11-08): delete
//!    them too, or move them to the root in place keeping their own
//!    hierarchy; one undo step; multi-selection; shear keeps the local
//!    transform with a note.
//! 8. Save/reopen is native version 30 and round-trips; an older version
//!    claiming a locator is refused.
//! 9. The Locators type tick filters it in the Items list and Select
//!    patterns; old saved filters without the tick keep locators shown.
use glam::DVec3;
use operandi_core::item_list::*;
use operandi_core::*;

fn cube_at(e: &mut Editor, name: &str, at: DVec3) -> u64 {
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.active_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    item.name = name.into();
    item.transform.translation = at;
    item.id
}
fn item(e: &Editor, id: u64) -> &scene::Item {
    e.scene.items.iter().find(|i| i.id == id).unwrap()
}
fn world_points(e: &Editor, id: u64) -> Vec<DVec3> {
    item(e, id)
        .mesh
        .vertices
        .iter()
        .map(|&p| e.scene.world_point(id, p))
        .collect()
}
fn close(a: &[DVec3], b: &[DVec3]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert!((*x - *y).length() < 1e-9, "{x} vs {y}");
    }
}
fn names(e: &Editor) -> Vec<String> {
    e.scene.items.iter().map(|i| i.name.clone()).collect()
}
fn add_locator(e: &mut Editor) -> u64 {
    e.dispatch(Command::AddItem(AddItemKind::GroupLocator))
        .unwrap();
    e.scene.selection.item.unwrap()
}

#[test]
fn add_group_locator_is_named_selected_placed_and_one_undo_step() {
    let mut e = Editor::default();
    let a = cube_at(&mut e, "A", DVec3::ZERO);
    cube_at(&mut e, "B", DVec3::ZERO);
    e.dispatch(Command::SelectItem(Some(a))).unwrap();
    let steps = e.undo.len();
    let g = add_locator(&mut e);
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(e.undo.last().unwrap().label, "New Group Locator");
    assert_eq!(names(&e), ["A", "Group Locator", "B"]);
    let l = item(&e, g);
    assert!(l.is_group_locator() && !l.is_mesh());
    assert!(l.mesh.vertices.is_empty());
    assert_eq!(l.transform, scene::Transform::default());
    assert_eq!(e.scene.selected_items(), vec![g]);
    // A second one is numbered.
    add_locator(&mut e);
    assert!(names(&e).contains(&"Group Locator (2)".to_string()));
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(names(&e), ["A", "B"]);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(names(&e), ["A", "Group Locator", "B"]);
    assert_eq!(AddItemKind::GroupLocator.label(), "Group Locator");
    assert_eq!(
        AddItemKind::from_key("locator.group"),
        Some(AddItemKind::GroupLocator)
    );
}

#[test]
fn moving_rotating_and_scaling_the_locator_carries_its_child() {
    for (tool, delta) in [
        (Tool::Move, DVec3::new(0., 3., 0.)),
        (Tool::Rotate, DVec3::new(0., 0., 90.)),
        (Tool::Scale, DVec3::splat(2.)),
    ] {
        let mut e = Editor::default();
        let child = cube_at(&mut e, "Child", DVec3::new(2., 0., 0.));
        let g = add_locator(&mut e);
        e.dispatch(Command::SetParent {
            item: child,
            parent: Some(g),
            keep_position: true,
        })
        .unwrap();
        e.dispatch(Command::SelectItem(Some(g))).unwrap();
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        let before = world_points(&e, child);
        let local = item(&e, child).transform.clone();
        // Move/Rotate/Scale switch to Item mode for a non-mesh item.
        e.dispatch(Command::Activate(tool)).unwrap();
        assert_eq!(e.scene.selection.mode, Mode::Item);
        e.dispatch(Command::Preview(Parameters {
            delta,
            ..Default::default()
        }))
        .unwrap();
        e.dispatch(Command::Drop).unwrap();
        let m = e.scene.world_matrix(g).unwrap();
        let expected: Vec<DVec3> = before.iter().map(|&p| m.transform_point3(p)).collect();
        assert_ne!(m, glam::DMat4::IDENTITY, "{tool:?} moved the locator");
        close(&world_points(&e, child), &expected);
        assert_eq!(item(&e, child).transform, local, "{tool:?}");
    }
}

#[test]
fn shared_parent_keeps_local_transforms_exactly() {
    let mut e = Editor::default();
    let p = cube_at(&mut e, "P", DVec3::new(1., 2., 3.));
    item_rotate(&mut e, p, DVec3::new(0.2, 0.4, 0.6));
    let a = cube_at(&mut e, "A", DVec3::new(1., 0., 0.));
    let b = cube_at(&mut e, "B", DVec3::new(0., 1., 0.));
    for id in [a, b] {
        e.dispatch(Command::SetParent {
            item: id,
            parent: Some(p),
            keep_position: false,
        })
        .unwrap();
    }
    let locals = [item(&e, a).transform.clone(), item(&e, b).transform.clone()];
    let worlds = [world_points(&e, a), world_points(&e, b)];
    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    e.scene.toggle_item(a);
    let steps = e.undo.len();
    e.dispatch(Command::ParentToGroupLocator).unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(e.undo.last().unwrap().label, "Parent to Group Locator");
    let g = e.scene.selection.item.unwrap();
    assert_eq!(e.scene.selected_items(), vec![g]);
    let l = item(&e, g);
    assert!(l.is_group_locator());
    assert_eq!(l.parent, Some(p));
    assert_eq!(l.transform, scene::Transform::default());
    assert_eq!(item(&e, a).parent, Some(g));
    assert_eq!(item(&e, b).parent, Some(g));
    assert_eq!(item(&e, a).transform, locals[0]);
    assert_eq!(item(&e, b).transform, locals[1]);
    close(&world_points(&e, a), &worlds[0]);
    close(&world_points(&e, b), &worlds[1]);
    // The locator took the first selected item's (B's) list place.
    assert_eq!(names(&e), ["P", "A", "Group Locator", "B"]);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(names(&e), ["P", "A", "B"]);
    assert_eq!(item(&e, a).parent, Some(p));
}

fn item_rotate(e: &mut Editor, id: u64, r: DVec3) {
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .unwrap()
        .transform
        .rotation = r;
}

#[test]
fn mixed_parents_keep_world_positions() {
    let mut e = Editor::default();
    let p = cube_at(&mut e, "P", DVec3::new(1., 2., 3.));
    item_rotate(&mut e, p, DVec3::new(0., 0., 0.5));
    let a = cube_at(&mut e, "A", DVec3::new(1., 0., 0.));
    e.dispatch(Command::SetParent {
        item: a,
        parent: Some(p),
        keep_position: false,
    })
    .unwrap();
    let b = cube_at(&mut e, "B", DVec3::new(-4., 0., 1.));
    let b_local = item(&e, b).transform.clone();
    let worlds = [world_points(&e, a), world_points(&e, b)];
    e.dispatch(Command::SelectItem(Some(a))).unwrap();
    e.scene.toggle_item(b);
    e.dispatch(Command::ParentToGroupLocator).unwrap();
    let g = e.scene.selection.item.unwrap();
    assert_eq!(item(&e, g).parent, None);
    assert_eq!(item(&e, g).transform, scene::Transform::default());
    assert_eq!(item(&e, a).parent, Some(g));
    assert_eq!(item(&e, b).parent, Some(g));
    close(&world_points(&e, a), &worlds[0]);
    close(&world_points(&e, b), &worlds[1]);
    // A top-level child keeps its transform as it was.
    assert_eq!(item(&e, b).transform, b_local);
}

#[test]
fn mixed_parents_needing_shear_refuse_the_whole_command() {
    let mut e = Editor::default();
    let p = cube_at(&mut e, "P", DVec3::ZERO);
    {
        let t = &mut e
            .scene
            .items
            .iter_mut()
            .find(|i| i.id == p)
            .unwrap()
            .transform;
        t.scale = DVec3::new(3., 1., 1.);
    }
    let a = cube_at(&mut e, "A", DVec3::ZERO);
    item_rotate(&mut e, a, DVec3::new(0., 0., 0.7));
    e.dispatch(Command::SetParent {
        item: a,
        parent: Some(p),
        keep_position: false,
    })
    .unwrap();
    let b = cube_at(&mut e, "B", DVec3::X);
    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    e.scene.toggle_item(a);
    let before = e.scene.clone();
    let steps = e.undo.len();
    let err = e.dispatch(Command::ParentToGroupLocator).unwrap_err();
    assert!(err.contains("shear"), "{err}");
    assert_eq!(e.scene, before);
    assert_eq!(e.undo.len(), steps);
}

#[test]
fn parent_to_group_locator_needs_a_selection() {
    let mut e = Editor::default();
    cube_at(&mut e, "A", DVec3::ZERO);
    e.dispatch(Command::SelectItem(None)).unwrap();
    let before = e.scene.clone();
    let steps = e.undo.len();
    assert!(e.dispatch(Command::ParentToGroupLocator).is_err());
    assert_eq!(e.scene, before);
    assert_eq!(e.undo.len(), steps);
}

#[test]
fn component_tools_refuse_a_group_locator() {
    let mut e = Editor::default();
    cube_at(&mut e, "A", DVec3::ZERO);
    add_locator(&mut e);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    let err = e.dispatch(Command::Activate(Tool::Bevel)).unwrap_err();
    assert_eq!(err, editor::LOCATOR_NO_GEOMETRY);
    assert!(err.contains("locators"));
}

#[test]
fn save_and_reopen_is_native_version_30() {
    let mut e = Editor::default();
    let child = cube_at(&mut e, "Child", DVec3::X);
    let g = add_locator(&mut e);
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == g)
        .unwrap()
        .transform
        .translation = DVec3::Y;
    e.dispatch(Command::SetParent {
        item: child,
        parent: Some(g),
        keep_position: false,
    })
    .unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 30"), "{}", &text[..200]);
    assert!(text.contains("\"locator\": \"Group\""));
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    assert!(
        back.items
            .iter()
            .find(|i| i.id == g)
            .unwrap()
            .is_group_locator()
    );
    let older = text.replacen("\"version\": 30", "\"version\": 29", 1);
    assert!(persistence::decode(older.as_bytes()).is_err());
    // A scene without locators keeps its older version.
    let mut plain = e.scene.clone();
    plain.items.retain(|i| i.id != g);
    plain.items.iter_mut().for_each(|i| i.parent = None);
    plain.selection = Default::default();
    let text = String::from_utf8(persistence::encode(&plain).unwrap()).unwrap();
    assert!(!text.contains("\"version\": 30"));
}

#[test]
fn locators_type_tick_filters_rows_and_patterns() {
    let mut e = Editor::default();
    cube_at(&mut e, "A", DVec3::ZERO);
    let g = add_locator(&mut e);
    assert_eq!(ItemType::of(item(&e, g)), ItemType::Locator);
    assert_eq!(ItemType::Locator.label(), "Locators");
    assert!(ItemType::ALL.contains(&ItemType::Locator));
    let mut filter = ListFilter::default();
    let rows = |f: &ListFilter| -> Vec<u64> {
        list_rows(&e.scene, f, &Default::default())
            .iter()
            .map(|r| r.id)
            .collect()
    };
    assert!(rows(&filter).contains(&g));
    filter.types.set(ItemType::Locator, false);
    assert!(!rows(&filter).contains(&g));
    let query = PatternQuery {
        types: TypeTicks {
            locators: false,
            ..TypeTicks::all()
        },
        ..Default::default()
    };
    assert!(!e.scene.pattern_matches(&query).contains(&g));
    assert!(
        e.scene
            .pattern_matches(&PatternQuery::default())
            .contains(&g)
    );
    // Old saved filters have no `locators` key: still ticked.
    let old: TypeTicks =
        serde_json::from_str(r#"{"backdrops":true,"cameras":false,"lights":true,"meshes":true}"#)
            .unwrap();
    assert!(old.locators && !old.cameras);
}

/// GL-Q4 (Jason, 2026-11-08): a scene of Root > G > (A > A1, B), all moved.
fn delete_rig() -> (Editor, [u64; 5]) {
    let mut e = Editor::default();
    let root = cube_at(&mut e, "Root", DVec3::new(0., 0., 5.));
    let a = cube_at(&mut e, "A", DVec3::new(1., 0., 0.));
    let a1 = cube_at(&mut e, "A1", DVec3::new(0., 1., 0.));
    let b = cube_at(&mut e, "B", DVec3::new(-2., 0., 0.));
    e.dispatch(Command::SelectItem(Some(a))).unwrap();
    e.scene.toggle_item(b);
    e.dispatch(Command::ParentToGroupLocator).unwrap();
    let g = e.scene.selection.item.unwrap();
    for (child, parent) in [(g, root), (a1, a)] {
        e.dispatch(Command::SetParent {
            item: child,
            parent: Some(parent),
            keep_position: false,
        })
        .unwrap();
    }
    let t = &mut e
        .scene
        .items
        .iter_mut()
        .find(|i| i.id == g)
        .unwrap()
        .transform;
    t.translation = DVec3::new(3., 0., 0.);
    t.rotation = DVec3::new(0., 0., 0.4);
    (e, [root, g, a, a1, b])
}

#[test]
fn delete_with_children_yes_deletes_the_whole_branch() {
    let (mut e, [root, g, a, a1, b]) = delete_rig();
    assert_eq!(
        item_delete::children_question(&e.scene, &[g]).unwrap(),
        "The item Group Locator has child items. Do you want to delete the child items too?"
    );
    e.dispatch(Command::SelectItem(Some(g))).unwrap();
    let before = e.scene.clone();
    let steps = e.undo.len();
    e.dispatch(Command::DeleteItems(item_delete::ChildrenOnDelete::Delete))
        .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    let left: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    assert_eq!(left, vec![root]);
    assert!([g, a, a1, b].iter().all(|id| !left.contains(id)));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items, before.items);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.items.len(), 1);
}

#[test]
fn delete_with_children_no_moves_them_to_the_root_in_place() {
    let (mut e, [root, g, a, a1, b]) = delete_rig();
    let worlds: Vec<Vec<DVec3>> = [a, a1, b].iter().map(|&id| world_points(&e, id)).collect();
    let a1_local = item(&e, a1).transform.clone();
    e.dispatch(Command::SelectItem(Some(g))).unwrap();
    e.dispatch(Command::DeleteItems(
        item_delete::ChildrenOnDelete::MoveToRoot,
    ))
    .unwrap();
    assert!(e.scene.items.iter().all(|i| i.id != g));
    assert_eq!(item(&e, a).parent, None);
    assert_eq!(item(&e, b).parent, None);
    // A's own child stays under it, untouched.
    assert_eq!(item(&e, a1).parent, Some(a));
    assert_eq!(item(&e, a1).transform, a1_local);
    assert_eq!(item(&e, root).parent, None);
    for (id, world) in [a, a1, b].iter().zip(&worlds) {
        close(&world_points(&e, *id), world);
    }
    assert_eq!(e.scene.selected_items(), Vec::<u64>::new());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(item(&e, a).parent, Some(g));
}

#[test]
fn plain_delete_moves_children_to_the_root_and_childless_items_delete_as_before() {
    // Scripts' and MCP's default (GL-Q6).
    let (mut e, [_, g, a, _, b]) = delete_rig();
    e.dispatch(Command::SelectItem(Some(g))).unwrap();
    e.dispatch(Command::DeleteItem).unwrap();
    assert_eq!((item(&e, a).parent, item(&e, b).parent), (None, None));
    let steps = e.undo.len();
    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    assert!(item_delete::children_question(&e.scene, &[b]).is_none());
    e.dispatch(Command::DeleteItem).unwrap();
    assert!(e.scene.items.iter().all(|i| i.id != b));
    assert_eq!(e.undo.len(), steps + 1);
}

#[test]
fn delete_with_children_multi_selection_is_one_step() {
    let (mut e, [root, g, a, a1, b]) = delete_rig();
    e.dispatch(Command::SelectItem(Some(a))).unwrap();
    e.scene.toggle_item(root);
    assert_eq!(
        item_delete::children_question(&e.scene, &e.scene.selected_items()).unwrap(),
        "Some of the selected items have child items. Do you want to delete the child items too?"
    );
    let a1_world = world_points(&e, a1);
    let steps = e.undo.len();
    e.dispatch(Command::DeleteItems(
        item_delete::ChildrenOnDelete::MoveToRoot,
    ))
    .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    let left: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    assert!(!left.contains(&a) && !left.contains(&root));
    // G (Root's child) and A1 (A's child) go to the root; B stays in G.
    assert_eq!(item(&e, g).parent, None);
    assert_eq!(item(&e, a1).parent, None);
    assert_eq!(item(&e, b).parent, Some(g));
    close(&world_points(&e, a1), &a1_world);
}

#[test]
fn delete_with_children_needing_shear_keeps_the_local_transform() {
    let mut e = Editor::default();
    let p = cube_at(&mut e, "P", DVec3::ZERO);
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == p)
        .unwrap()
        .transform
        .scale = DVec3::new(3., 1., 1.);
    let c = cube_at(&mut e, "C", DVec3::X);
    item_rotate(&mut e, c, DVec3::new(0., 0., 0.7));
    e.dispatch(Command::SetParent {
        item: c,
        parent: Some(p),
        keep_position: false,
    })
    .unwrap();
    let local = item(&e, c).transform.clone();
    let mut scene = e.scene.clone();
    let sheared = scene
        .delete_items(&[p], item_delete::ChildrenOnDelete::MoveToRoot)
        .unwrap();
    assert_eq!(sheared, vec!["C".to_string()]);
    assert!(
        item_delete::sheared_note(&sheared)
            .unwrap()
            .contains("shear")
    );
    assert_eq!(
        scene.items.iter().find(|i| i.id == c).unwrap().transform,
        local
    );
}

#[test]
fn exports_bake_the_locator_into_children_and_draw_nothing_for_it() {
    // GL-08: the locator has no geometry; a child exports where it is.
    let mut e = Editor::default();
    let child = cube_at(&mut e, "Child", DVec3::ZERO);
    e.dispatch(Command::SelectItem(Some(child))).unwrap();
    e.dispatch(Command::ParentToGroupLocator).unwrap();
    let g = e.scene.selection.item.unwrap();
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == g)
        .unwrap()
        .transform
        .translation = DVec3::new(10., 0., 0.);
    let dir = std::env::temp_dir().join(format!("operandi-gl-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let obj = dir.join("scene.obj");
    let report = interchange::export(&obj, &e.scene, interchange::ExportFormat::Obj).unwrap();
    assert_eq!(report.items, 1);
    let text = std::fs::read_to_string(&obj).unwrap();
    let xs: Vec<f64> = text
        .lines()
        .filter_map(|l| l.strip_prefix("v "))
        .map(|l| l.split_whitespace().next().unwrap().parse().unwrap())
        .collect();
    assert!(!xs.is_empty() && xs.iter().all(|x| *x > 9.0), "{xs:?}");
    // LXO leaves the locator out; the child keeps its world place.
    let lxo_path = dir.join("scene.lxo");
    let report = interchange::export(&lxo_path, &e.scene, interchange::ExportFormat::Lxo).unwrap();
    assert!(
        report.notes.iter().any(|n| n.contains("Group Locator")),
        "{:?}",
        report.notes
    );
    let bytes = std::fs::read(&lxo_path).unwrap();
    let mut progress = |_: &str| true;
    let imported = lxo::parse(&bytes, &mut lxo::Control::new(&mut progress)).unwrap();
    assert_eq!(imported.scene.items.len(), 1);
    let back = &imported.scene.items[0];
    assert!((back.transform.translation - DVec3::new(10., 0., 0.)).length() < 1e-6);
    std::fs::remove_dir_all(&dir).ok();
}
