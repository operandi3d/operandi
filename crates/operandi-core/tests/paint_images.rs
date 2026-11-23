//! 3D painting stage 1 in the scene (PNT-01..39): New Image, Add Color /
//! Mask / Displacement Texture, the paint target's refusals, the image
//! commands, saving and changed images. Only with `--features painting`.
#![cfg(feature = "painting")]
use operandi_core::paint::*;
use operandi_core::shader_tree::{BlendMode, NodeKind, TextureEffect, TextureSource};
use operandi_core::*;
use std::sync::Arc;

/// A cube with the primitive's UVs, selected.
fn cube() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::InsertUnit {
        tool: Tool::Box,
        destination: UnitDestination::ExistingMesh,
    })
    .unwrap();
    assert!(!e.scene.active().unwrap().mesh.uv_maps.is_empty());
    use operandi_core::shader_tree::{NewLayer, ShaderTreeEdit};
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: None,
    }))
    .unwrap();
    let group = base_group(&e);
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Constant,
        selected: Some(group),
    }))
    .unwrap();
    e
}

fn small(color: Option<[f32; 3]>) -> NewImage {
    NewImage {
        resolution: 64,
        color,
        ..NewImage::default()
    }
}

fn add(e: &mut Editor, kind: TextureKind, selected: Option<u64>) -> u64 {
    e.dispatch(Command::Paint(PaintEdit::AddTexture {
        kind,
        image: small(Some([1., 0., 0.])),
        selected,
        item: None,
    }))
    .unwrap();
    e.paint.last_layer.unwrap()
}

/// A Material Group holding a Constant (the default tree has none).
fn base_group(e: &Editor) -> u64 {
    e.scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Group(_)))
        .map(|n| n.id)
        .expect("made by cube()")
}

#[test]
fn new_image_starts_blank_rgba_2048_and_is_not_an_undo_step() {
    let mut e = Editor::default();
    let steps = e.undo.len();
    e.dispatch(Command::Paint(PaintEdit::NewImage(NewImage::default())))
        .unwrap();
    let id = e.paint.last_image.unwrap();
    let image = e.scene.images.iter().find(|i| i.id == id).unwrap();
    assert_eq!((image.width, image.height), (2048, 2048));
    assert!(image.path.is_empty(), "the file name comes on first save");
    let px = &e.scene.paint.buffers[&id];
    assert_eq!(px.format, pixels::Format::Rgba);
    assert_eq!(px.depth, pixels::Depth::U8);
    assert_eq!(px.get(12345), [0.; 4]);
    assert_eq!(e.undo.len(), steps);
    assert!(e.image_changed(id), "a new image has no file yet");
    // Out of range sizes are refused and change nothing.
    let before = e.scene.clone();
    for side in [63, 8193] {
        assert!(
            e.dispatch(Command::Paint(PaintEdit::NewImage(NewImage {
                resolution: side,
                ..NewImage::default()
            })))
            .is_err()
        );
    }
    assert_eq!(e.scene, before);
}

#[test]
fn float_and_set_color_images() {
    let mut e = Editor::default();
    e.dispatch(Command::Paint(PaintEdit::NewImage(NewImage {
        resolution: 64,
        float: true,
        format: pixels::Format::Rgb,
        color: Some([0.2, 0.4, 2.]),
        ..NewImage::default()
    })))
    .unwrap();
    let px = e.scene.paint.buffers[&e.paint.last_image.unwrap()].clone();
    assert_eq!(px.depth, pixels::Depth::F32);
    assert_eq!(px.get(0), [0.2, 0.4, 2., 1.]);
}

#[test]
fn color_texture_goes_on_top_of_the_selected_group_with_the_first_uv_map() {
    let mut e = cube();
    let group = base_group(&e);
    let layer = add(&mut e, TextureKind::Color, Some(group));
    let tree = &e.scene.shader_tree;
    let node = tree.get(layer).unwrap();
    assert_eq!(node.parent, Some(group));
    assert_eq!(tree.children(Some(group)).next().unwrap().id, layer);
    let NodeKind::Texture(t) = &node.kind else {
        panic!("a texture layer")
    };
    assert_eq!(t.effect, TextureEffect::DiffuseColor);
    let TextureSource::ImageMap { image, .. } = &t.source else {
        panic!("an Image Map")
    };
    assert_eq!(*image, e.paint.last_image);
    let locator = e.scene.texture_locator(t.locator.unwrap()).unwrap();
    let first = e
        .scene
        .active()
        .unwrap()
        .mesh
        .uv_maps
        .keys()
        .next()
        .cloned();
    assert_eq!(locator.uv_map, first);
    assert_eq!(paint_target_problem(&e.scene, Some(layer)), None);
}

#[test]
fn without_a_group_color_goes_above_the_base_material() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Paint(PaintEdit::AddTexture {
        kind: TextureKind::Color,
        image: small(None),
        selected: None,
        item: None,
    }))
    .unwrap();
    let layer = e.paint.last_layer.unwrap();
    let rows: Vec<String> = e
        .scene
        .shader_tree
        .rows(&|_| false)
        .into_iter()
        .map(|(n, _)| n.name.clone())
        .collect();
    let name = &e.scene.shader_tree.get(layer).unwrap().name;
    let at = rows.iter().position(|n| n == name).unwrap();
    assert_eq!(rows[at + 1], "Base Material");
}

#[test]
fn mask_goes_directly_above_the_selected_layer() {
    let mut e = cube();
    let group = base_group(&e);
    let color = add(&mut e, TextureKind::Color, Some(group));
    // The Constant, now below the colour layer.
    let below = e
        .scene
        .shader_tree
        .children(Some(group))
        .nth(1)
        .map(|n| n.id)
        .unwrap();
    assert!(matches!(
        e.scene.shader_tree.get(below).unwrap().kind,
        NodeKind::Texture(_)
    ));
    let mask = add(&mut e, TextureKind::Mask, Some(below));
    let order: Vec<u64> = e
        .scene
        .shader_tree
        .children(Some(group))
        .map(|n| n.id)
        .collect();
    let at = order.iter().position(|&id| id == mask).unwrap();
    assert_eq!(order[at + 1], below);
    assert_eq!(order[0], color);
    let NodeKind::Texture(t) = &e.scene.shader_tree.get(mask).unwrap().kind else {
        panic!()
    };
    assert_eq!(t.effect, TextureEffect::LayerMask);
    // A mask needs a layer to mask.
    assert!(
        e.dispatch(Command::Paint(PaintEdit::AddTexture {
            kind: TextureKind::Mask,
            image: small(None),
            selected: None,
            item: None,
        }))
        .is_err()
    );
}

#[test]
fn displacement_is_float_vector_displacement() {
    let mut e = cube();
    let layer = {
        let g = base_group(&e);
        add(&mut e, TextureKind::Displacement, Some(g))
    };
    let NodeKind::Texture(t) = &e.scene.shader_tree.get(layer).unwrap().kind else {
        panic!()
    };
    assert_eq!(t.effect, TextureEffect::VectorDisplacement);
    let image = e.paint.last_image.unwrap();
    assert_eq!(e.scene.paint.buffers[&image].depth, pixels::Depth::F32);
    assert_eq!(
        pixels::FileType::default_for(e.scene.paint.buffers[&image].depth),
        pixels::FileType::Exr
    );
}

#[test]
fn undoing_add_texture_removes_the_layer_but_keeps_the_image() {
    let mut e = cube();
    let steps = e.undo.len();
    let layer = {
        let g = base_group(&e);
        add(&mut e, TextureKind::Color, Some(g))
    };
    let image = e.paint.last_image.unwrap();
    assert_eq!(e.undo.len(), steps + 1, "one undo step");
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.shader_tree.get(layer).is_none());
    assert!(e.scene.images.iter().any(|i| i.id == image));
    assert!(e.scene.paint.buffers.contains_key(&image));
    e.dispatch(Command::Redo).unwrap();
    assert!(e.scene.shader_tree.get(layer).is_some());
    assert_eq!(e.scene.images.iter().filter(|i| i.id == image).count(), 1);
}

#[test]
fn paint_target_refusals() {
    let mut e = cube();
    assert_eq!(paint_target_problem(&e.scene, None), Some(NO_TEXTURE));
    let group = base_group(&e);
    assert_eq!(
        paint_target_problem(&e.scene, Some(group)),
        Some(NO_TEXTURE)
    );
    let layer = add(&mut e, TextureKind::Color, Some(group));
    // Hidden itself, or its group hidden.
    for hide in [layer, group] {
        let mut s = e.scene.clone();
        let mut n = s.shader_tree.get(hide).unwrap().clone();
        n.layer.enable = false;
        s.shader_tree.replace(n).unwrap();
        assert_eq!(
            paint_target_problem(&s, Some(layer)),
            Some(NOTHING_TO_PAINT)
        );
    }
    // A mesh without UVs.
    let mut s = e.scene.clone();
    let mut mesh = (*s.active().unwrap().mesh).clone();
    mesh.uv_maps = Default::default();
    s.active_mut().unwrap().mesh = mesh.into();
    assert_eq!(paint_target_problem(&s, Some(layer)), Some(NO_UV));
}

#[test]
fn image_commands_are_one_undo_step_each_and_mark_the_image_changed() {
    let mut e = cube();
    {
        let g = base_group(&e);
        add(&mut e, TextureKind::Color, Some(g))
    };
    let image = e.paint.last_image.unwrap();
    let dir = scratch();
    let path = dir.join("red.png");
    e.save_image(image, &path).unwrap();
    assert!(!e.image_changed(image));
    assert_eq!(
        e.scene.images.iter().find(|i| i.id == image).unwrap().path,
        path.display().to_string()
    );

    let steps = e.undo.len();
    e.dispatch(Command::Paint(PaintEdit::Invert { image, via: None }))
        .unwrap();
    assert_eq!(e.scene.paint.buffers[&image].get(0), [0., 1., 1., 1.]);
    assert!(e.image_changed(image));
    assert_eq!(e.undo.len(), steps + 1);
    e.dispatch(Command::Undo).unwrap();
    assert!(!e.image_changed(image), "undone back to the saved pixels");
    e.dispatch(Command::Redo).unwrap();

    e.dispatch(Command::Paint(PaintEdit::Clear { image, via: None }))
        .unwrap();
    assert_eq!(e.scene.paint.buffers[&image].get(0), [0., 0., 0., 0.]);
    e.dispatch(Command::Paint(PaintEdit::SetResolution {
        image,
        width: 128,
        height: 128,
    }))
    .unwrap();
    let record = e.scene.images.iter().find(|i| i.id == image).unwrap();
    assert_eq!((record.width, record.height), (128, 128));
    assert_eq!(e.undo.len(), steps + 3);
    assert!(
        e.dispatch(Command::Paint(PaintEdit::SetResolution {
            image,
            width: 32,
            height: 32
        }))
        .is_err()
    );
    assert_eq!(e.changed_images(), vec![image]);
}

#[test]
fn image_loads_from_its_file_and_saves_back() {
    let dir = scratch();
    let path = dir.join("loaded.png");
    let mut px = pixels::PaintImage::new(pixels::Layout {
        width: 64,
        height: 64,
        format: pixels::Format::Rgb,
        depth: pixels::Depth::U16,
    })
    .unwrap();
    px.fill([0.25, 0.5, 0.75, 1.]);
    pixels::io::save(&px, &path).unwrap();
    let mut e = Editor::default();
    let id = e
        .scene
        .image_for_path(&path.display().to_string(), 64, 64)
        .unwrap();
    assert!(!e.image_changed(id));
    let loaded = e.image_pixels(id).unwrap();
    assert_eq!(*loaded, px, "loaded at its own depth");
    assert!(!e.image_changed(id));
    e.dispatch(Command::Paint(PaintEdit::Invert {
        image: id,
        via: None,
    }))
    .unwrap();
    assert!(e.image_changed(id));
    e.save_image(id, &path).unwrap();
    assert!(!e.image_changed(id));
    assert_eq!(e.reload_image(id), Ok(()));
    assert!((e.scene.paint.buffers[&id].get(0)[0] - 0.75).abs() < 1e-4);
}

/// Astra P9-002: a reload of a file that doesn't decode (another app still
/// writing it) keeps the pixels the image had, and says why.
#[test]
fn a_reload_that_fails_keeps_the_pixels() {
    let dir = scratch();
    let path = dir.join("half-written.png");
    let mut px = pixels::PaintImage::new(pixels::Layout {
        width: 16,
        height: 16,
        format: pixels::Format::Rgb,
        depth: pixels::Depth::U8,
    })
    .unwrap();
    px.fill([1., 0., 0., 1.]);
    pixels::io::save(&px, &path).unwrap();
    let mut e = Editor::default();
    let id = e
        .scene
        .image_for_path(&path.display().to_string(), 16, 16)
        .unwrap();
    let before = e.image_pixels(id).unwrap();
    std::fs::write(&path, b"\x89PNG half").unwrap();
    assert!(e.reload_image(id).is_err());
    assert!(
        Arc::ptr_eq(&e.scene.paint.buffers[&id], &before),
        "the old pixels stay"
    );
    assert!(!e.image_changed(id), "and still count as saved");
}

#[test]
fn expand_uv_seams_grows_colour_past_the_islands() {
    let mut e = cube();
    {
        let g = base_group(&e);
        add(&mut e, TextureKind::Color, Some(g))
    };
    let image = e.paint.last_image.unwrap();
    // Each face its own small island, with gaps between them.
    let mut mesh = (*e.scene.active().unwrap().mesh).clone();
    let uvs: Vec<Vec<Option<glam::DVec2>>> = (0..mesh.faces.len())
        .map(|f| {
            let (cx, cy) = ((f % 3) as f64 / 3., (f / 3) as f64 / 2.);
            let sq = [(0.05, 0.05), (0.25, 0.05), (0.25, 0.4), (0.05, 0.4)];
            (0..mesh.faces[f].vertices.len())
                .map(|c| Some(glam::DVec2::new(cx + sq[c % 4].0, cy + sq[c % 4].1)))
                .collect()
        })
        .collect();
    let name = mesh.uv_maps.keys().next().unwrap().clone();
    mesh.uv_maps.insert(name, uvs);
    e.scene.active_mut().unwrap().mesh = mesh.into();
    // Every pixel different, so any copy shows.
    let mut px = (*e.scene.paint.buffers[&image]).clone();
    for i in 0..px.pixel_count() {
        px.set(i, [(i % 64) as f32 / 63., (i / 64) as f32 / 63., 0.5, 1.]);
    }
    let before = Arc::new(px);
    e.scene.paint.buffers.insert(image, before.clone());
    let steps = e.undo.len();
    let expand = |e: &mut Editor, border| {
        e.dispatch(Command::Paint(PaintEdit::ExpandSeams {
            image,
            via: None,
            item: None,
            uv_map: None,
            border,
        }))
    };
    // Border 0 changes nothing and makes no step.
    expand(&mut e, 0).unwrap();
    assert_eq!(e.undo.len(), steps);
    expand(&mut e, 4).unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    let after = &e.scene.paint.buffers[&image];
    let changed = (0..after.pixel_count())
        .filter(|&i| after.get(i) != before.get(i))
        .count();
    assert!(changed > 0, "pixels next to the islands take their colour");
    assert!(changed < after.pixel_count() / 2, "only within the border");
    // Without a mesh it says so.
    let mut empty = Editor::default();
    empty
        .dispatch(Command::Paint(PaintEdit::NewImage(small(None))))
        .unwrap();
    let id = empty.paint.last_image.unwrap();
    assert!(
        empty
            .dispatch(Command::Paint(PaintEdit::ExpandSeams {
                image: id,
                via: None,
                item: None,
                uv_map: None,
                border: 4
            }))
            .is_err()
    );
}

#[test]
fn flatten_composites_in_tree_order_into_a_new_layer() {
    let mut e = cube();
    let group = base_group(&e);
    let bottom = add(&mut e, TextureKind::Color, Some(group)); // red
    e.dispatch(Command::Paint(PaintEdit::AddTexture {
        kind: TextureKind::Color,
        image: small(Some([0., 0., 1.])),
        selected: Some(group),
        item: None,
    }))
    .unwrap();
    let top = e.paint.last_layer.unwrap(); // blue, above
    let mut n = e.scene.shader_tree.get(top).unwrap().clone();
    n.layer.opacity = 0.5;
    n.layer.blend = BlendMode::Normal;
    e.scene.shader_tree.replace(n).unwrap();
    let images = e.scene.images.len();
    let steps = e.undo.len();
    e.dispatch(Command::Paint(PaintEdit::Flatten {
        layers: vec![bottom, top],
    }))
    .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(e.scene.images.len(), images + 1);
    let flat = e.paint.last_layer.unwrap();
    let order: Vec<u64> = e
        .scene
        .shader_tree
        .children(Some(group))
        .map(|n| n.id)
        .collect();
    assert_eq!(&order[..3], &[flat, top, bottom]);
    let image = layer_image(&e.scene, flat).unwrap();
    let p = e.scene.paint.buffers[&image].get(0);
    assert!((p[0] - 0.5).abs() < 0.01 && (p[2] - 0.5).abs() < 0.01 && p[1] < 0.01);
    assert!(!e.scene.shader_tree.get(top).unwrap().layer.enable);
    // Undo keeps the flattened image (images aren't undone).
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.shader_tree.get(flat).is_none());
    assert!(e.scene.images.iter().any(|i| i.id == image));
    // One layer, or layers of different effects, are refused.
    assert!(
        e.dispatch(Command::Paint(PaintEdit::Flatten { layers: vec![top] }))
            .is_err()
    );
}

#[test]
fn photoshop_images_paint_into_a_layer_and_save_whole() {
    use operandi_core::paint::pixels::psd::PsdDoc;
    let mut e = cube();
    let group = base_group(&e);
    let layer = add(&mut e, TextureKind::Color, Some(group));
    let image = e.paint.last_image.unwrap();
    // A new image saved as PSD: one pixel layer, "Layer 1" (PNT-308).
    let path = scratch().join("skin.psd");
    e.save_image(image, &path).unwrap();
    assert!(!e.image_changed(image));
    let rows = e.psd_rows(image).unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["Composite", "Layer 1"]);
    // The Composite asks which layer (PNT-304).
    let err = e
        .dispatch(Command::Paint(PaintEdit::Invert {
            image,
            via: Some(layer),
        }))
        .unwrap_err();
    assert_eq!(err, PSD_WHICH_LAYER);
    // New Layer: "Operandi Paint 1", and the map shows it.
    let steps = e.undo.len();
    e.dispatch(Command::Paint(PaintEdit::NewPsdLayer { via: layer }))
        .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(
        psd_layer_of(&e.scene, layer).as_deref(),
        Some("Operandi Paint 1")
    );
    e.dispatch(Command::Paint(PaintEdit::Invert {
        image,
        via: Some(layer),
    }))
    .unwrap();
    // The new layer was transparent black: inverted it is white, alpha 0;
    // the composite still shows Layer 1's red.
    assert_eq!(e.layer_picture(layer).unwrap().get(0), [1., 1., 1., 0.]);
    assert!(e.image_changed(image));
    // Set Resolution would change the document's structure.
    assert!(
        e.dispatch(Command::Paint(PaintEdit::SetResolution {
            image,
            width: 128,
            height: 128
        }))
        .is_err()
    );
    e.save_image(image, &path).unwrap();
    let back = PsdDoc::read(&path).unwrap();
    let names: Vec<String> = back.rows().into_iter().map(|r| r.name).collect();
    assert_eq!(names, ["Composite", "Operandi Paint 1", "Layer 1"]);
    // Flat formats are refused for a Photoshop image.
    assert!(e.save_image(image, &scratch().join("skin.png")).is_err());
    // Reopened, the layer pick survives in the scene file.
    let text = serde_json::to_string(&e.scene.shader_tree).unwrap();
    assert!(text.contains("Operandi Paint 1"));
}

fn scratch() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "operandi-paint-core-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
