//! MN-07: drag and drop gaps. A picture dropped on a mesh becomes a
//! texture of that surface's material, several files drop at once, and
//! items dragged from a 3D view join a group in the Groups list.
use super::tests::{box_app, draw, pointer};
use super::*;
use operandi_core::shader_tree::{NodeKind, TextureSource};

const SIZE: Vec2 = vec2(1440., 900.);

fn picture(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-mn07-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    image::RgbImage::from_pixel(8, 4, image::Rgb([200, 120, 40]))
        .save(&path)
        .unwrap();
    path
}

fn top_view(a: &mut Operandi) -> Pos2 {
    let view = Rect::from_min_size(Pos2::ZERO, vec2(400., 300.));
    a.corner.areas.insert(a.active_view, view);
    a.viewport_rect = view;
    a.camera.view = View::Top;
    a.camera.project(glam::DVec3::ZERO, view).unwrap().0
}

/// The Image Map layers showing a picture named `name`, with their parent.
fn image_layers(a: &Operandi, name: &str) -> Vec<Option<u64>> {
    let scene = &a.editor.scene;
    scene
        .shader_tree
        .nodes
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::Texture(t) => match &t.source {
                TextureSource::ImageMap {
                    image: Some(id), ..
                } if scene
                    .images
                    .iter()
                    .any(|i| i.id == *id && i.path.ends_with(name)) =>
                {
                    Some(n.parent)
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn a_picture_on_a_mesh_textures_that_material_in_one_step() {
    let mut a = box_app();
    let centre = top_view(&mut a);
    let tag = {
        let (item, faces) = a.drop_geometry(centre).expect("over the box");
        a.editor.scene.item(item).unwrap().mesh.faces[faces[0]]
            .material
            .clone()
    };
    let undo = a.editor.undo.len();
    a.file_dropped(picture("brick.png"), Some(centre));
    assert!(
        a.editor.scene.items.iter().all(|i| i.backdrop.is_none()),
        "no Backdrop"
    );
    let layers = image_layers(&a, "brick.png");
    assert_eq!(layers.len(), 1, "{}", a.status);
    // In the material group masking the polygon's tag, when there is one.
    if let Some(group) = layers[0] {
        let g = a.editor.scene.shader_tree.get(group).unwrap();
        let NodeKind::Group(g) = &g.kind else {
            panic!("parent is a group")
        };
        assert_eq!(
            g.tag,
            operandi_core::shader_tree::TagMask::Name(tag.clone())
        );
    }
    assert_eq!(a.editor.undo.len(), undo + 1, "one undo step");
    a.run(Command::Undo);
    assert!(image_layers(&a, "brick.png").is_empty());
}

#[test]
fn empty_view_space_still_makes_a_backdrop() {
    let mut a = Operandi::empty();
    top_view(&mut a);
    a.file_dropped(picture("plain.png"), Some(pos2(10., 10.)));
    assert!(a.editor.scene.items.iter().any(|i| i.backdrop.is_some()));
}

#[test]
fn several_files_are_all_handled() {
    let mut a = box_app();
    let centre = top_view(&mut a);
    let one = picture("one.png");
    let two = picture("two.png");
    // Two scene files: they queue to open one by one.
    let scene_a = std::env::temp_dir().join("mn07-a.opscene");
    let scene_b = std::env::temp_dir().join("mn07-b.opscene");
    a.files_dropped(
        vec![one, two, scene_a.clone(), scene_b.clone()],
        Some(centre),
    );
    assert_eq!(image_layers(&a, "one.png").len(), 1, "{}", a.status);
    assert_eq!(image_layers(&a, "two.png").len(), 1, "{}", a.status);
    assert_eq!(a.document_opens.next(false), Some(scene_a));
    assert_eq!(a.document_opens.next(false), Some(scene_b));
}

#[test]
fn items_dragged_from_the_view_join_a_group() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.run(Command::Mode(Mode::Item));
    let mesh = a.editor.scene.items[0].id;
    a.run(Command::SelectItem(Some(mesh)));
    // An empty group to drop on.
    a.run(Command::SelectItems(vec![]));
    a.new_group_from_selection(&ctx);
    let group = a.picked_group(&ctx).expect("a group");
    assert!(a.editor.scene.group_members(group).is_empty());
    a.run(Command::SelectItem(Some(mesh)));
    a.upper_frame.show_page(crate::groups_ui::GROUPS_PAGE);
    a.camera.view = View::Top;
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    let row = a
        .groups_tree
        .rects
        .iter()
        .find(|(k, _)| *k == crate::groups_ui::GroupKey::Group(group))
        .unwrap()
        .1
        .center();
    let on_box = a
        .camera
        .project(glam::DVec3::new(0., 1., 0.), a.viewport_rect)
        .unwrap()
        .0;
    let undo = a.editor.undo.len();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(on_box)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(on_box, true, Modifiers::NONE)],
    );
    for t in [0.25, 0.5, 0.75, 1.] {
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(on_box.lerp(row, t))],
        );
    }
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(row, false, Modifiers::NONE)],
    );
    assert_eq!(a.editor.scene.group_members(group), [mesh], "{}", a.status);
    assert_eq!(a.editor.undo.len(), undo + 1);
}
