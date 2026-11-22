//! An item's eye that hides its children (GV-01..06, 2026-11-10).
//! Acceptance criteria:
//!
//! 1. Hiding a parent with `hides_children` hides its whole subtree
//!    through it: the children keep their own eyes but aren't shown, leave
//!    the viewport (`display_hidden`) and refuse component tools (GV-01).
//! 2. Showing the parent brings the children back as they were: a child
//!    hidden on its own stays hidden (GV-02).
//! 3. A plain hide (`hides_children` false) hides only the item (GV-03).
//! 4. Any parent works, not only Group Locators (GV-06).
//! 5. One undo step each way; undo restores the eye exactly.
//! 6. Save/reopen is native version 35 and round-trips; an older version
//!    claiming the flag is refused; a scene without it keeps its version
//!    and old files load with every eye as before.
//! 7. Render/export readers see the inherited hiding (`resolved_eyes`).
use glam::DVec3;
use operandi_core::*;

/// A parent mesh with a child mesh under it: (editor, parent, child).
fn family() -> (Editor, u64, u64) {
    let mut e = Editor::default();
    let add = |e: &mut Editor, at: DVec3| {
        e.dispatch(Command::NewMesh).unwrap();
        let item = e.scene.active_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        item.transform.translation = at;
        item.id
    };
    let parent = add(&mut e, DVec3::ZERO);
    let child = add(&mut e, DVec3::X * 3.);
    e.dispatch(Command::SetParent {
        item: child,
        parent: Some(parent),
        keep_position: false,
    })
    .unwrap();
    (e, parent, child)
}

fn item(e: &Editor, id: u64) -> &scene::Item {
    e.scene.items.iter().find(|i| i.id == id).unwrap()
}

fn hide(e: &mut Editor, id: u64, hides_children: bool) {
    e.dispatch(Command::SetItemsVisible {
        items: vec![id],
        visible: false,
        hides_children,
    })
    .unwrap();
}

fn show(e: &mut Editor, id: u64) {
    e.dispatch(Command::SetItemsVisible {
        items: vec![id],
        visible: true,
        hides_children: true,
    })
    .unwrap();
}

#[test]
fn gv01_hiding_a_parent_hides_its_subtree_and_keeps_the_childrens_eyes() {
    let (mut e, parent, child) = family();
    let steps = e.undo.len();
    hide(&mut e, parent, true);
    assert_eq!(e.undo.len(), steps + 1);
    assert!(item(&e, child).visible, "the child's own eye is untouched");
    assert!(!e.scene.shows(item(&e, child)));
    assert!(e.scene.hidden_by_parent_eye(item(&e, child)));
    assert!(e.scene.display_hidden().contains(&child));
    assert!(!e.scene.drawn_in_viewport(child, true));
    // A component tool on the hidden child is refused like a hidden mesh.
    e.dispatch(Command::SelectItem(Some(child))).unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    assert!(e.dispatch(Command::FlipPolygons).is_err());
    // Undo puts the eye back exactly.
    e.dispatch(Command::Undo).unwrap();
    assert!(item(&e, parent).visible && !item(&e, parent).hides_children);
    assert!(e.scene.shows(item(&e, child)));
}

#[test]
fn gv02_showing_the_parent_brings_children_back_as_they_were() {
    let (mut e, parent, child) = family();
    // A grandchild hidden on its own stays hidden.
    e.dispatch(Command::NewMesh).unwrap();
    let own = e.scene.active().unwrap().id;
    e.dispatch(Command::SetParent {
        item: own,
        parent: Some(child),
        keep_position: false,
    })
    .unwrap();
    hide(&mut e, own, false);
    hide(&mut e, parent, true);
    assert!(!e.scene.shows(item(&e, child)));
    show(&mut e, parent);
    assert!(!item(&e, parent).hides_children, "cleared when shown");
    assert!(e.scene.shows(item(&e, child)));
    assert!(!e.scene.shows(item(&e, own)), "hidden on its own");
    assert!(e.scene.display_hidden().is_empty());
}

#[test]
fn gv03_a_plain_hide_hides_only_the_item() {
    let (mut e, parent, child) = family();
    hide(&mut e, parent, false);
    assert!(!e.scene.shows(item(&e, parent)));
    assert!(e.scene.shows(item(&e, child)));
    assert!(!e.scene.display_hidden().contains(&child));
    // The plain toggle (script item.visibility) is a plain off too.
    show(&mut e, parent);
    e.dispatch(Command::Visibility(parent)).unwrap();
    assert!(!item(&e, parent).hides_children);
    assert!(e.scene.shows(item(&e, child)));
}

#[test]
fn gv06_a_group_locator_parent_works_the_same() {
    let (mut e, _, child) = family();
    e.dispatch(Command::SelectItems(vec![child])).unwrap();
    e.dispatch(Command::ParentToGroupLocator).unwrap();
    let group = e
        .scene
        .items
        .iter()
        .find(|i| i.is_group_locator())
        .unwrap()
        .id;
    hide(&mut e, group, true);
    assert!(item(&e, child).visible);
    assert!(!e.scene.shows(item(&e, child)));
}

#[test]
fn visibility_with_children_still_rewrites_the_children() {
    // Scripts keep the old command; it never leaves an "all off" eye.
    let (mut e, parent, child) = family();
    hide(&mut e, parent, true);
    e.dispatch(Command::VisibilityWithChildren(parent)).unwrap();
    assert!(item(&e, parent).visible && item(&e, child).visible);
    e.dispatch(Command::VisibilityWithChildren(parent)).unwrap();
    assert!(!item(&e, parent).visible && !item(&e, child).visible);
    assert!(!item(&e, parent).hides_children);
}

#[test]
fn hides_children_saves_at_version_35_and_old_files_load() {
    let (mut e, parent, child) = family();
    hide(&mut e, parent, true);
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(text.contains("\"version\": 35"), "{}", &text[..200]);
    assert!(text.contains("\"hides_children\": true"));
    let back = persistence::decode(text.as_bytes()).unwrap();
    assert_eq!(back, e.scene);
    assert!(!back.shows(back.items.iter().find(|i| i.id == child).unwrap()));
    // An older version claiming the flag is refused.
    let older = text.replacen("\"version\": 35", "\"version\": 34", 1);
    assert!(persistence::decode(older.as_bytes()).is_err());
    // Without the flag the version stays older and nothing is written.
    show(&mut e, parent);
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(!text.contains("\"version\": 35"));
    assert!(!text.contains("hides_children"));
    // An old file (no flag, hidden parent) loads as before: a plain off,
    // the child shown.
    hide(&mut e, parent, false);
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    let old = persistence::decode(text.as_bytes()).unwrap();
    assert!(!old.items.iter().any(|i| i.hides_children));
    assert!(old.shows(old.items.iter().find(|i| i.id == child).unwrap()));
}

#[test]
fn resolved_eyes_bake_the_inherited_hiding_for_render_and_export() {
    let (mut e, parent, child) = family();
    assert_eq!(e.scene.resolved_eyes(), e.scene, "nothing to bake");
    hide(&mut e, parent, true);
    let r = e.scene.resolved_eyes();
    let flag = |id| r.items.iter().find(|i| i.id == id).unwrap().visible;
    assert!(!flag(parent) && !flag(child));
    assert!(!r.items.iter().any(|i| i.hides_children));
    // OBJ export leaves the hidden child out like any hidden mesh.
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let dir = std::env::temp_dir().join(format!("operandi-eye-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("eye.obj");
    let report = interchange::export(&path, &e.scene, interchange::ExportFormat::Obj).unwrap();
    assert_eq!(report.items, 1, "only the shown mesh");
    assert!(
        report.notes.iter().any(|n| n.contains("2 hidden item(s)")),
        "{:?}",
        report.notes
    );
    std::fs::remove_dir_all(&dir).ok();
}
