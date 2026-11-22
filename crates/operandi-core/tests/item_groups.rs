//! Item groups (no spec yet; GRP-Q*) and Light Linking (LP-30..32).
use operandi_core::camera_light::*;
use operandi_core::*;

fn new_scene() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    e.next_item_id = e.scene.next_id;
    e
}

fn ids(e: &Editor) -> Vec<u64> {
    e.scene.items.iter().map(|i| i.id).collect()
}

fn last_label(e: &Editor) -> &str {
    &e.undo.last().unwrap().label
}

fn new_group(e: &mut Editor, name: Option<&str>, members: Vec<u64>) -> u64 {
    let before: Vec<u64> = e.scene.groups.iter().map(|g| g.id).collect();
    e.dispatch(Command::NewGroup {
        name: name.map(Into::into),
        members,
    })
    .unwrap();
    e.scene
        .groups
        .iter()
        .find(|g| !before.contains(&g.id))
        .unwrap()
        .id
}

#[test]
fn groups_are_named_ordered_and_one_step_each() {
    let mut e = new_scene();
    let [mesh, camera, light, ..] = ids(&e)[..] else {
        panic!()
    };
    let steps = e.undo.len();
    let a = new_group(&mut e, None, vec![light, mesh, light, 999]);
    assert_eq!(last_label(&e), "New Group");
    let g = e.scene.group(a).unwrap();
    assert_eq!(g.name, "Group");
    assert_eq!(
        g.members,
        [light, mesh],
        "order kept, duplicates and unknowns dropped"
    );
    // Unique names.
    let b = new_group(&mut e, None, vec![]);
    assert_eq!(e.scene.group(b).unwrap().name, "Group (2)");
    e.dispatch(Command::RenameGroup {
        group: b,
        name: "Group".into(),
    })
    .unwrap();
    assert_eq!(e.scene.group(b).unwrap().name, "Group (2)", "taken");
    e.dispatch(Command::RenameGroup {
        group: b,
        name: " Lights ".into(),
    })
    .unwrap();
    assert_eq!(last_label(&e), "Rename Group");
    assert_eq!(e.scene.group(b).unwrap().name, "Lights");
    assert!(
        e.dispatch(Command::RenameGroup {
            group: b,
            name: "  ".into(),
        })
        .is_err()
    );
    // An item may be in several groups.
    e.dispatch(Command::AddToGroup {
        group: b,
        items: vec![light, camera],
    })
    .unwrap();
    assert_eq!(last_label(&e), "Add to Group");
    assert_eq!(e.scene.groups_of(light), [a, b]);
    assert!(
        e.dispatch(Command::AddToGroup {
            group: b,
            items: vec![light],
        })
        .is_err(),
        "already in"
    );
    e.dispatch(Command::RemoveFromGroup {
        group: b,
        items: vec![light],
    })
    .unwrap();
    assert_eq!(last_label(&e), "Remove from Group");
    assert_eq!(e.scene.group_members(b), [camera]);
    // The rename to a taken name changed nothing, so it added no step.
    assert_eq!(e.undo.len(), steps + 5);
    for _ in 0..5 {
        e.dispatch(Command::Undo).unwrap();
    }
    assert!(e.scene.groups.is_empty());
}

#[test]
fn deleting_items_and_groups() {
    let mut e = new_scene();
    let [mesh, camera, _, ..] = ids(&e)[..] else {
        panic!()
    };
    let g = new_group(&mut e, Some("Set"), vec![mesh, camera]);
    // Deleting an item takes it out of the group.
    e.dispatch(Command::SelectItem(Some(camera))).unwrap();
    e.dispatch(Command::DeleteItem).unwrap();
    assert_eq!(e.scene.group(g).unwrap().members, [mesh]);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.group(g).unwrap().members, [mesh, camera]);
    // Deleting a group keeps its items.
    let items = e.scene.items.len();
    e.dispatch(Command::DeleteGroup(g)).unwrap();
    assert_eq!(last_label(&e), "Delete Group");
    assert!(e.scene.groups.is_empty());
    assert_eq!(e.scene.items.len(), items);
    assert!(e.dispatch(Command::DeleteGroup(g)).is_err());
}

#[test]
fn select_group_selects_its_members() {
    let mut e = new_scene();
    let [mesh, _, light, ..] = ids(&e)[..] else {
        panic!()
    };
    let g = new_group(&mut e, None, vec![light, mesh]);
    let steps = e.undo.len();
    e.dispatch(Command::SelectGroup(g)).unwrap();
    assert_eq!(e.scene.selection.item, Some(light));
    assert_eq!(e.scene.selection.also, [mesh]);
    assert_eq!(e.undo.len(), steps, "selecting isn't an undo step");
    assert!(e.dispatch(Command::SelectGroup(12345)).is_err());
}

#[test]
fn groups_save_as_version_18() {
    // Groups were first numbered 17 on their branch; the Material Shading
    // Model took 17 on main, so groups moved to 18.
    let mut e = new_scene();
    let [mesh, ..] = ids(&e)[..] else { panic!() };
    new_group(&mut e, Some("Hero"), vec![mesh]);
    let bytes = persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 33"));
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
    for older in [16, 17] {
        let old = text.replacen("\"version\": 33", &format!("\"version\": {older}"), 1);
        assert!(persistence::decode(old.as_bytes()).is_err(), "{older}");
    }
    // No groups: the older version still.
    let plain = new_scene();
    let text = String::from_utf8(persistence::encode(&plain.scene).unwrap()).unwrap();
    assert!(!text.contains("\"groups\""));
    assert!(!text.contains("\"version\": 18"));
}

#[test]
fn files_of_versions_16_and_17_still_load() {
    // Migration: a scene without groups written as 16 (light properties)
    // or 17 (Material Shading Model) reads unchanged in an 18 build.
    // Without its Environment (version 33): a file from before 33 gets the
    // default environment on load, so compare everything else.
    let mut plain = new_scene();
    plain.scene.items.retain(|i| i.environment.is_none());
    let text = String::from_utf8(persistence::encode(&plain.scene).unwrap()).unwrap();
    let without_environments = |mut s: Scene| {
        s.items.retain(|i| i.environment.is_none());
        s.next_id = 0;
        s
    };
    let written = text
        .split("\"version\": ")
        .nth(1)
        .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next())
        .unwrap()
        .to_owned();
    for version in [16, 17, 18, 19, 20] {
        let doc = text.replacen(
            &format!("\"version\": {written}"),
            &format!("\"version\": {version}"),
            1,
        );
        assert_eq!(
            without_environments(persistence::decode(doc.as_bytes()).unwrap()),
            without_environments(persistence::decode(text.as_bytes()).unwrap()),
            "version {version}"
        );
    }
    let future = text.replacen(&format!("\"version\": {written}"), "\"version\": 99", 1);
    assert!(persistence::decode(future.as_bytes()).is_err());
}

#[test]
fn light_linking_rule() {
    let mut l = LightLinking::default();
    assert!(!l.control && l.group.is_none() && l.mode == LinkMode::Exclude);
    // Off: lights everything.
    assert!(l.lights(1, Some(&[1])));
    l.control = true;
    // Exclude: everything but the group; none excludes nothing.
    assert!(!l.lights(1, Some(&[1])) && l.lights(2, Some(&[1])));
    assert!(l.lights(1, None));
    // Include: only the group; none or empty lights nothing.
    l.mode = LinkMode::Include;
    assert!(l.lights(1, Some(&[1])) && !l.lights(2, Some(&[1])));
    assert!(!l.lights(1, None) && !l.lights(1, Some(&[])));
}

#[test]
fn light_linking_rows_are_named_steps() {
    let mut e = new_scene();
    let [mesh, _, light, ..] = ids(&e)[..] else {
        panic!()
    };
    let g = new_group(&mut e, None, vec![mesh]);
    let edit = |e: &mut Editor, edit| e.dispatch(Command::EditLight { item: light, edit });
    edit(&mut e, LightEdit::LinkControl(true)).unwrap();
    assert_eq!(last_label(&e), "Control Light Linking");
    edit(&mut e, LightEdit::LinkGroup(Some(g))).unwrap();
    assert_eq!(last_label(&e), "Item Group");
    edit(&mut e, LightEdit::LinkMode(LinkMode::Include)).unwrap();
    assert_eq!(last_label(&e), "Light Linking Mode");
    assert!(edit(&mut e, LightEdit::LinkGroup(Some(98765))).is_err());
    let l = e
        .scene
        .items
        .iter()
        .find(|i| i.id == light)
        .unwrap()
        .light
        .clone()
        .unwrap();
    assert_eq!(
        l.linking,
        Some(LightLinking {
            control: true,
            group: Some(g),
            mode: LinkMode::Include
        })
    );
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
    // Deleting the group leaves the light pointing at nothing (Include:
    // lights nothing).
    e.dispatch(Command::DeleteGroup(g)).unwrap();
    assert!(e.scene.group(g).is_none());
}
