//! Instance versions of the Duplicate tools and the Item menu's Instance
//! (BEHAVIOR-DUPLICATE-PANEL DX-30..38). Acceptance criteria:
//!
//! 1. Instance Clone / Array / Radial Array / Scatter make instances of the
//!    source at the same places the Item-mode copies go; the source is
//!    untouched; a component mode works on the active mesh item (DX-11).
//! 2. Instance Mirror: a negative-scale instance whose points land mirrored
//!    (DX-34).
//! 3. Parent: Hierarchy chains, Root puts all under the source, Primary
//!    under a new Group Locator under the source; world places don't move.
//!    Hierarchy (the option) copies the source's children under each copy.
//! 4. Editing the source's mesh changes every instance.
//! 5. Drop is one undo step; Undo removes every instance; the instances
//!    save and reopen as instances.
//! 6. Curve / Pen Instance places instances along the path.
//! 7. Instance (DX-38): one instance per selected mesh item, same place,
//!    selected, one undo step; nothing selected is refused.
use glam::DVec3;
use operandi_core::duplicate_items::ParentMode;
use operandi_core::path_tools::{PathKind, PathTool};
use operandi_core::*;

fn editor(center: DVec3, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(center, DVec3::ONE, 0).unwrap().into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}
fn params(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}
fn instance_on(d: &mut DuplicateParameters) {
    d.copier = Some(operandi_core::duplicate_copier::Copier::Instance);
}
fn run(e: &mut Editor, tool: Tool, set: impl FnOnce(&mut DuplicateParameters)) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = params(e);
    instance_on(&mut p.duplicate);
    set(&mut p.duplicate);
    p.duplicate.placed = true;
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
}
fn world_centre(e: &Editor, id: u64) -> DVec3 {
    let w = e.scene.world_matrix(id).unwrap();
    let m = &e.scene.item(id).unwrap().mesh;
    m.vertices
        .iter()
        .map(|&v| w.transform_point3(v))
        .sum::<DVec3>()
        / m.vertices.len() as f64
}
fn source(e: &Editor) -> u64 {
    e.scene.items[0].id
}

#[test]
fn instance_clone_makes_instances_in_a_row() {
    for mode in [Mode::Item, Mode::Polygon] {
        let mut e = editor(DVec3::ZERO, mode);
        let before = e.scene.items[0].clone();
        run(&mut e, Tool::Clone, |d| {
            d.clone.count = 3;
            d.clone.offset = DVec3::new(0., 0., 2.);
        });
        assert_eq!(e.scene.items.len(), 4, "{mode:?}");
        assert_eq!(
            e.scene.items[0].mesh, before.mesh,
            "the source's mesh is untouched"
        );
        for (i, item) in e.scene.items.iter().enumerate().skip(1) {
            assert_eq!(item.instance_of, Some(before.id));
            assert_eq!(item.mesh, before.mesh);
            assert_eq!(item.name, format!("{} ({})", before.name, i + 1));
            assert!(world_centre(&e, item.id).abs_diff_eq(DVec3::new(0., 0., 2. * i as f64), 1e-9));
        }
    }
}

#[test]
fn instance_array_radial_and_scatter() {
    let mut e = editor(DVec3::ZERO, Mode::Item);
    run(&mut e, Tool::Array, |d| d.array.count = [3, 1, 1]);
    assert_eq!(e.scene.items.len(), 3);
    assert!(
        e.scene.items[1..]
            .iter()
            .all(|i| i.instance_of == Some(source(&e)))
    );

    let mut e = editor(DVec3::new(2., 0., 0.), Mode::Item);
    run(&mut e, Tool::RadialArray, |d| d.radial.circle.count = 4);
    assert_eq!(e.scene.items.len(), 4);
    let c = world_centre(&e, e.scene.items[2].id);
    assert!(c.abs_diff_eq(DVec3::new(-2., 0., 0.), 1e-9), "{c}");

    let mut e = editor(DVec3::ZERO, Mode::Item);
    run(&mut e, Tool::ScatterClone, |d| {
        d.scatter.generator.count = 5
    });
    assert!(e.scene.items.len() > 1);
    assert!(
        e.scene.items[1..]
            .iter()
            .all(|i| i.instance_of == Some(source(&e)))
    );
}

#[test]
fn instance_mirror_has_negative_scale() {
    let mut e = editor(DVec3::new(2., 0., 0.), Mode::Item);
    run(&mut e, Tool::Mirror, |_| {});
    assert_eq!(e.scene.items.len(), 2);
    let copy = &e.scene.items[1];
    assert_eq!(copy.instance_of, Some(source(&e)));
    assert!(copy.transform.scale.x < 0.);
    let w = e.scene.world_matrix(copy.id).unwrap();
    assert!(glam::DMat3::from_mat4(w).determinant() < 0.);
    assert!(world_centre(&e, copy.id).abs_diff_eq(DVec3::new(-2., 0., 0.), 1e-9));
}

#[test]
fn parent_modes_keep_world_places() {
    let places = |e: &Editor| -> Vec<DVec3> {
        e.scene
            .items
            .iter()
            .filter(|i| i.is_mesh())
            .map(|i| world_centre(e, i.id))
            .collect()
    };
    let clone = |d: &mut DuplicateParameters| {
        d.clone.count = 3;
        d.clone.offset = DVec3::new(1., 0., 0.);
    };
    let mut plain = editor(DVec3::ZERO, Mode::Item);
    run(&mut plain, Tool::Clone, clone);
    let want = places(&plain);
    for parent in ParentMode::ALL {
        let mut e = editor(DVec3::ZERO, Mode::Item);
        // A moved source, so parent transforms matter.
        e.scene.items[0].transform.translation = DVec3::new(0., 1., 0.);
        e.scene.items[0].transform.rotation = DVec3::new(0., 30., 0.);
        let shifted: Vec<DVec3> = {
            let mut p = editor(DVec3::ZERO, Mode::Item);
            p.scene.items[0].transform = e.scene.items[0].transform.clone();
            run(&mut p, Tool::Clone, clone);
            places(&p)
        };
        let _ = &want;
        run(&mut e, Tool::Clone, |d| {
            clone(d);
            d.clone.copies.item.parent = parent;
        });
        let got = places(&e);
        assert_eq!(got.len(), 4, "{parent:?}");
        for (a, b) in got.iter().zip(&shifted) {
            assert!(a.abs_diff_eq(*b, 1e-9), "{parent:?}: {a} vs {b}");
        }
        let src = source(&e);
        let copies: Vec<&Item> = e
            .scene
            .items
            .iter()
            .filter(|i| i.instance_of.is_some())
            .collect();
        match parent {
            ParentMode::Off => assert!(copies.iter().all(|c| c.parent.is_none())),
            ParentMode::Root => assert!(copies.iter().all(|c| c.parent == Some(src))),
            ParentMode::Hierarchy => {
                assert_eq!(copies[0].parent, Some(src));
                assert_eq!(copies[1].parent, Some(copies[0].id));
                assert_eq!(copies[2].parent, Some(copies[1].id));
            }
            ParentMode::Primary => {
                let group = e.scene.items.iter().find(|i| i.locator.is_some()).unwrap();
                assert_eq!(group.parent, Some(src));
                assert!(copies.iter().all(|c| c.parent == Some(group.id)));
            }
        }
    }
}

#[test]
fn hierarchy_copies_children_under_each_copy() {
    let mut e = editor(DVec3::ZERO, Mode::Item);
    let parent = source(&e);
    e.dispatch(Command::NewMesh).unwrap();
    let child = e.scene.items[1].id;
    e.scene.items[1].mesh = Mesh::box_mesh(DVec3::Y, DVec3::splat(0.5), 0)
        .unwrap()
        .into();
    e.scene.items[1].parent = Some(parent);
    e.scene.select_item(Some(parent));
    run(&mut e, Tool::Clone, |d| {
        d.clone.count = 2;
        d.clone.offset = DVec3::X * 3.;
        d.clone.copies.item.hierarchy = true;
    });
    // Source, its child, and two copies each with a child.
    assert_eq!(e.scene.items.len(), 6);
    let copies: Vec<u64> = e
        .scene
        .items
        .iter()
        .filter(|i| i.instance_of == Some(parent))
        .map(|i| i.id)
        .collect();
    assert_eq!(copies.len(), 2);
    for (k, c) in copies.iter().enumerate() {
        let kids: Vec<&Item> = e
            .scene
            .items
            .iter()
            .filter(|i| i.parent == Some(*c))
            .collect();
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].instance_of, Some(child));
        let at = world_centre(&e, kids[0].id);
        assert!(
            at.abs_diff_eq(DVec3::new(3. * (k + 1) as f64, 1., 0.), 1e-9),
            "{at}"
        );
    }
}

#[test]
fn editing_the_source_changes_the_instances() {
    let mut e = editor(DVec3::ZERO, Mode::Item);
    run(&mut e, Tool::Clone, |d| d.clone.offset = DVec3::X * 2.);
    let copy = e.scene.items[1].id;
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.select_item(Some(source(&e)));
    e.dispatch(Command::DeleteComponents).ok();
    // Delete with nothing selected may refuse; edit directly instead.
    let before = e.scene.clone();
    e.scene.items[0].mesh.vertices[0] += DVec3::Y;
    e.scene.sync_instances(Some(&before));
    assert_eq!(e.scene.item(copy).unwrap().mesh, e.scene.items[0].mesh);
}

#[test]
fn drop_is_one_step_and_instances_save() {
    let mut e = editor(DVec3::ZERO, Mode::Item);
    let before = e.scene.clone();
    let steps = e.history.undo.len();
    run(&mut e, Tool::Clone, |d| {
        d.clone.count = 2;
        d.clone.offset = DVec3::X * 2.;
    });
    assert_eq!(e.history.undo.len(), steps + 1);
    let dir = std::env::temp_dir().join(format!("op-inst-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("instances.opscene");
    e.save_committed(&path).unwrap();
    let back = operandi_core::persistence::load(&path).unwrap();
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(back.items.len(), 3);
    assert!(
        back.items[1..]
            .iter()
            .all(|i| i.instance_of == Some(before.items[0].id))
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}

#[test]
fn pen_instance_places_along_the_path() {
    let mut e = editor(DVec3::ZERO, Mode::Polygon);
    run(&mut e, Tool::CurveClone, |d| {
        let mut t = PathTool::default();
        t.curve.kind = PathKind::Pen;
        t.curve.points = vec![DVec3::ZERO, DVec3::new(0., 2., 0.), DVec3::new(0., 4., 0.)];
        t.pen_align = false;
        d.curve_clone = t;
    });
    let copies: Vec<u64> = e
        .scene
        .items
        .iter()
        .filter(|i| i.instance_of.is_some())
        .map(|i| i.id)
        .collect();
    assert_eq!(
        copies.len(),
        2,
        "{:?}",
        e.scene.items.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
    let mut ys: Vec<f64> = copies.iter().map(|&c| world_centre(&e, c).y).collect();
    ys.sort_by(f64::total_cmp);
    assert!(ys[1] > ys[0] + 1.);
    assert_eq!(
        e.scene.items[0].mesh.faces.len(),
        6,
        "the source isn't copied into"
    );
}

#[test]
fn the_instance_command() {
    let mut e = editor(DVec3::new(1., 2., 3.), Mode::Item);
    let src = source(&e);
    let steps = e.history.undo.len();
    e.dispatch(Command::InstanceItems).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Instance");
    assert_eq!(e.scene.items.len(), 2);
    let made = e.scene.items[1].clone();
    assert_eq!(made.instance_of, Some(src));
    assert_eq!(e.scene.selection.item, Some(made.id));
    assert!(world_centre(&e, made.id).abs_diff_eq(DVec3::new(1., 2., 3.), 1e-9));
    // An instance of an instance points at the first source.
    e.dispatch(Command::InstanceItems).unwrap();
    assert_eq!(e.scene.items.last().unwrap().instance_of, Some(src));
    e.scene.select_item(None);
    let scene = e.scene.clone();
    assert!(e.dispatch(Command::InstanceItems).is_err());
    assert_eq!(e.scene, scene);
}

/// Forum report (2026-11-24): instancing a big mesh took seconds a copy,
/// because each new item's point, polygon and edge IDs were built and
/// checked from the polygons again. A new item sharing another's polygons
/// now lays out its own fresh block of IDs from that item's. The instance
/// shares the mesh (no copy), and both items' IDs stay valid and distinct;
/// a Duplicate gets the same treatment and still edits on its own.
#[test]
fn instance_and_duplicate_share_the_mesh_and_get_their_own_ids() {
    use std::collections::BTreeSet;
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    // A few thousand polygons.
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    for _ in 0..3 {
        e.scene.selection.faces.clear();
        command_line::run_line(&mut e, "poly.subdivide")
            .unwrap()
            .unwrap();
    }
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    let src = source(&e);
    let ids_of = |e: &mut Editor, id: u64| {
        let view = e.read_mesh(id).unwrap();
        let i = view.identities;
        let mut all: BTreeSet<u64> = i.vertices.iter().map(|v| v.0).collect();
        all.extend(i.faces.iter().map(|f| f.0));
        all.extend(i.corners.iter().flatten().map(|c| c.0));
        all.extend(i.edges.values().map(|x| x.0));
        (all, i.edges.len())
    };
    let (source_ids, source_edges) = ids_of(&mut e, src);
    let mut seen = source_ids.clone();
    for (label, command) in [
        ("Instance", Command::InstanceItems),
        ("Duplicate", Command::DuplicateItem),
    ] {
        e.scene.select_item(Some(src));
        e.dispatch(command).unwrap();
        let copy = e.scene.selection.item.unwrap();
        assert_ne!(copy, src);
        if label == "Instance" {
            assert!(
                Shared::same(
                    e.scene.item(copy).unwrap().mesh.shared(),
                    e.scene.item(src).unwrap().mesh.shared()
                ),
                "an instance shares its source's mesh"
            );
        }
        let (ids, edges) = ids_of(&mut e, copy);
        assert_eq!(ids.len(), source_ids.len(), "{label}: one ID per component");
        assert_eq!(edges, source_edges);
        assert!(ids.is_disjoint(&seen), "{label}: fresh IDs");
        seen.extend(ids);
    }
    // The duplicate is a mesh of its own: editing it leaves the source.
    let dup = e.scene.selection.item.unwrap();
    let before = e.scene.item(src).unwrap().mesh.vertices[0];
    let mut moved = e.scene.item(dup).unwrap().mesh.open().clone();
    moved.vertices[0] += DVec3::X;
    e.scene.item_mut(dup).unwrap().mesh = moved.into();
    assert_eq!(e.scene.item(src).unwrap().mesh.vertices[0], before);
    e.read_mesh(dup).unwrap();
}
