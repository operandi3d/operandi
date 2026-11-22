//! PB-110 drags out of the preset browser onto a mesh (Jason, 2026-11-11):
//! the hover highlights the hit polygon's material group only, nothing in
//! the scene changes until the drop, the views don't paint a selection
//! under the drag, and the drop fills that tag's group in one undo step.
use super::tests::{draw, pointer};
use super::*;
use crate::preset_browser::{Drag, Target};
use operandi_core::preset::library::{Entry, EntryKind};
use operandi_core::preset::{PresetKind, PresetSource};
use operandi_core::shader_tree::{NewLayer, NodeKind, TagMask};

/// One mesh of two 2 m boxes side by side, seen from the top, in Polygon
/// mode: the left box's polygons tagged "TableTop", the right's
/// "Table_Edge".
fn rig() -> (Operandi, Context, Vec2, u64) {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let id = a.editor.scene.items.last().unwrap().id;
    let mut mesh = operandi_core::Mesh::default();
    for (x, tag) in [(-3., "TableTop"), (3., "Table_Edge")] {
        let part =
            operandi_core::Mesh::box_mesh(DVec3::new(x, 0., 0.), DVec3::splat(2.), 0).unwrap();
        let offset = mesh.vertices.len() as u32;
        mesh.vertices.extend(part.vertices);
        for mut f in part.faces {
            f.vertices.iter_mut().for_each(|v| *v += offset);
            f.material = tag.into();
            mesh.faces.push(f);
        }
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = View::Top;
    a.run(Command::Mode(Mode::Polygon));
    a.editor.undo.clear();
    draw(&mut a, &ctx, size, vec![]);
    a.frame(false);
    draw(&mut a, &ctx, size, vec![]);
    (a, ctx, size, id)
}

/// A viewport point over a polygon tagged `tag`.
fn over(a: &Operandi, tag: &str) -> Pos2 {
    let r = a.viewport_rect;
    let mesh = &a.editor.scene.items[0].mesh;
    for y in (0..r.height() as i32).step_by(10) {
        for x in (0..r.width() as i32).step_by(10) {
            let p = r.left_top() + vec2(x as f32, y as f32);
            if let Some((_, Component::Polygon(f))) = a.pick_component(p, r)
                && mesh.faces[f as usize].material == tag
            {
                return p;
            }
        }
    }
    panic!("no {tag} polygon on screen");
}

/// A material preset file and its browser entry.
fn material_entry(a: &mut Operandi) -> Entry {
    let folder = a.environment.paths().data_root().join("drop-presets");
    std::fs::create_dir_all(&folder).unwrap();
    let mut source = operandi_core::Editor::default();
    let group = source.scene.shader_tree.add(NewLayer::Group, None).unwrap();
    source
        .scene
        .shader_tree
        .add(NewLayer::Material, Some(group))
        .unwrap();
    source
        .scene
        .shader_tree
        .add(NewLayer::Constant, Some(group))
        .unwrap();
    let path = folder.join("Walnut.oppreset");
    source
        .save_preset(&path, PresetSource::Material(group))
        .unwrap();
    Entry {
        name: "Walnut".into(),
        path,
        kind: EntryKind::Preset(PresetKind::Material),
        modified: None,
        size: 0,
        texture_set: None,
        modo_kind: None,
    }
}

fn faces_tagged(a: &Operandi, tag: &str) -> Vec<usize> {
    a.editor.scene.items[0]
        .mesh
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.material == tag)
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn hovering_highlights_one_material_group_and_changes_nothing() {
    let (mut a, ctx, size, id) = rig();
    let entry = material_entry(&mut a);
    let (top, edge) = (over(&a, "TableTop"), over(&a, "Table_Edge"));
    let before = a.editor.scene.clone();
    a.preset_browser.drag = Some(Drag {
        entry: entry.clone(),
    });
    // The mouse button is held across the view, as a drag from a docked
    // browser or the old popover delivers it: the view paints nothing.
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(top)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(top, true, Modifiers::NONE)],
    );
    for step in 1..=4 {
        let p = top + (edge - top) * (step as f32 / 4.);
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(p)]);
    }
    assert!(
        a.editor.scene.selection.faces_of(id).is_empty(),
        "no paint-select"
    );
    a.update_drop_hover(Some(top));
    let hover = a.preset_browser.drop_hover.clone().unwrap();
    assert_eq!((hover.item, hover.tag.as_str()), (id, "TableTop"));
    assert_eq!(hover.faces, faces_tagged(&a, "TableTop"));
    // Moving onto the other tag replaces the highlight; it doesn't add.
    a.update_drop_hover(Some(edge));
    let hover = a.preset_browser.drop_hover.clone().unwrap();
    assert_eq!(hover.tag, "Table_Edge");
    assert_eq!(hover.faces, faces_tagged(&a, "Table_Edge"));
    // Off the views: no highlight.
    a.update_drop_hover(None);
    assert!(a.preset_browser.drop_hover.is_none());
    assert_eq!(
        a.editor.scene, before,
        "the scene doesn't change while dragging"
    );
    assert_eq!(a.editor.undo.len(), 0);
    // Cancelling (Esc, or a drop off the views) leaves everything as it was.
    a.update_drop_hover(Some(edge));
    a.cancel_browser_drag();
    assert!(a.preset_browser.drag.is_none() && a.preset_browser.drop_hover.is_none());
    assert_eq!(a.editor.scene, before);
}

#[test]
fn the_drop_fills_the_hovered_tags_group_in_one_step() {
    let (mut a, _ctx, _size, _id) = rig();
    let entry = material_entry(&mut a);
    let edge = over(&a, "Table_Edge");
    let before = a.editor.scene.clone();
    assert!(a.browser_load(&entry, Target::Drop(edge)));
    assert_eq!(a.editor.undo.len(), 1);
    // Tags unchanged; Table_Edge now has a group of its own holding the
    // preset; TableTop has none.
    assert_eq!(
        a.editor.scene.items[0].mesh.faces,
        before.items[0].mesh.faces
    );
    let tree = &a.editor.scene.shader_tree;
    let group = |tag: &str| {
        tree.nodes
            .iter()
            .find(|n| matches!(&n.kind, NodeKind::Group(g) if g.tag == TagMask::Name(tag.into())))
    };
    let g = group("Table_Edge").expect("a group for the tag").clone();
    assert_eq!(g.name, "Table_Edge");
    assert_eq!(tree.children(Some(g.id)).count(), 2);
    // TableTop's material is untouched (nothing of the preset in it).
    if let Some(top) = group("TableTop") {
        assert!(
            !tree
                .children(Some(top.id))
                .any(|n| matches!(n.kind, NodeKind::Texture(_)))
        );
    }
    // A second drop on the same group replaces, it doesn't stack.
    let groups = |a: &Operandi| {
        a.editor
            .scene
            .shader_tree
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Group(_)))
            .count()
    };
    let after_first = groups(&a);
    assert!(a.browser_load(&entry, Target::Drop(edge)));
    assert_eq!(groups(&a), after_first);
    let tree = &a.editor.scene.shader_tree;
    assert_eq!(tree.children(Some(g.id)).count(), 2);
    a.run(Command::Undo);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
}
