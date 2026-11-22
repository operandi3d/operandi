//! Copying an item and its children into another scene (MULTI-SCENE.md
//! v0.3, MS-Q10 provisional): `Scene::branch`, `Command::InsertItems` and
//! `Command::RemoveBranch`.
use glam::DVec3;
use operandi_core::scene::Transform;
use operandi_core::*;

/// A scene with Mesh 1 at x=1 and its child Mesh 2 at local y=2.
fn source() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::NewMesh).unwrap();
    let (parent, child) = (e.scene.items[0].id, e.scene.items[1].id);
    for (id, at) in [(parent, DVec3::X), (child, DVec3::new(0., 2., 0.))] {
        e.dispatch(Command::ItemTransform {
            item: id,
            transform: Transform {
                translation: at,
                ..Default::default()
            },
        })
        .unwrap();
    }
    e.dispatch(Command::SetParent {
        item: child,
        parent: Some(parent),
        keep_position: false,
    })
    .unwrap();
    e
}

#[test]
fn a_branch_arrives_with_new_ids_free_names_and_its_world_position() {
    let src = source();
    let parent = src.scene.items[0].id;
    let child = src.scene.items[1].id;
    // Copying only the child: it leaves its parent but stays at (1, 2, 0).
    let branch = src.scene.branch(child).unwrap();
    assert_eq!(branch.len(), 1);
    assert_eq!(branch[0].parent, None);
    assert!((branch[0].transform.translation - DVec3::new(1., 2., 0.)).length() < 1e-12);

    let mut dst = Editor::default();
    dst.dispatch(Command::NewMesh).unwrap();
    let whole = src.scene.branch(parent).unwrap();
    assert_eq!(whole.len(), 2, "children come along");
    dst.dispatch(Command::InsertItems {
        items: whole,
        parent: None,
        images: vec![],
    })
    .unwrap();
    assert_eq!(dst.scene.items.len(), 3);
    let names: Vec<_> = dst.scene.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["Mesh", "Mesh (2)", "Mesh (2) (2)"]);
    let (root, kid) = (&dst.scene.items[1], &dst.scene.items[2]);
    assert_ne!(root.id, dst.scene.items[0].id, "fresh ids");
    assert_eq!(kid.parent, Some(root.id), "hierarchy kept");
    assert_eq!(dst.scene.selection.item, Some(root.id));
    assert!((dst.scene.world_point(kid.id, DVec3::ZERO) - DVec3::new(1., 2., 0.)).length() < 1e-12);
    assert_eq!(
        *kid.mesh, *src.scene.items[1].mesh,
        "geometry and materials"
    );

    // One undo step removes the whole arrival.
    dst.dispatch(Command::Undo).unwrap();
    assert_eq!(dst.scene.items.len(), 1);
}

#[test]
fn dropping_onto_an_item_parents_the_arrival_there_keeping_its_place() {
    let src = source();
    let child = src.scene.items[1].id;
    let mut dst = Editor::default();
    dst.dispatch(Command::NewMesh).unwrap();
    let host = dst.scene.items[0].id;
    dst.dispatch(Command::ItemTransform {
        item: host,
        transform: Transform {
            translation: DVec3::new(0., 0., 5.),
            ..Default::default()
        },
    })
    .unwrap();
    dst.dispatch(Command::InsertItems {
        items: src.scene.branch(child).unwrap(),
        parent: Some(host),
        images: vec![],
    })
    .unwrap();
    let arrived = dst.scene.items.last().unwrap();
    assert_eq!(arrived.parent, Some(host));
    assert!(
        (dst.scene.world_point(arrived.id, DVec3::ZERO) - DVec3::new(1., 2., 0.)).length() < 1e-12
    );
    assert!(
        dst.dispatch(Command::InsertItems {
            items: Vec::new(),
            parent: None,
            images: vec![],
        })
        .is_err()
    );
}

#[test]
fn removing_a_branch_takes_its_children_and_undoes_in_one_step() {
    let mut src = source();
    let parent = src.scene.items[0].id;
    let before = src.scene.clone();
    src.dispatch(Command::RemoveBranch(parent)).unwrap();
    assert!(src.scene.items.is_empty());
    src.dispatch(Command::Undo).unwrap();
    assert_eq!(src.scene, before);
}
