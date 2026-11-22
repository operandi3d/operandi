//! Falloff batch 5 (falloff spec 2026-11-15): Noise and Image in Tool
//! Properties, the Image picture stamping weights (FO-50, FO-54).
use super::tests::box_app;
use super::*;
use crate::falloff_panel::{FalloffFields, FalloffP};
use crate::tool_form::{Item, ToolFields};
use operandi_core::{ImageFalloff, NoiseFalloff};

fn rows(a: &Operandi) -> Vec<String> {
    let p: FalloffP = FalloffFields::params(a).unwrap();
    FalloffFields::form(a, &p)
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Section { items, .. } => Some(items),
            _ => None,
        })
        .flatten()
        .filter_map(|i| match i {
            Item::Field(f) => Some(f.row.label.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn noise_and_image_sections() {
    let mut a = box_app();
    a.run(Command::SpatialFalloff(Some(Falloff::Noise(
        NoiseFalloff::default(),
    ))));
    assert_eq!(rows(&a), ["Scale", "Use World Transforms", "Mix Mode"]);
    a.run(Command::SpatialFalloff(Some(Falloff::Image(
        ImageFalloff::default(),
    ))));
    assert_eq!(
        rows(&a),
        [
            "Image",
            "Repeat",
            "Soft Border",
            "Use Selected UV Map",
            "Scale",
            "Pos X",
            "Pos Y",
            "Rotation",
            "Use World Transforms",
            "Mix Mode"
        ]
    );
}

/// FO-54: a white picture laid over the view gives every point in view
/// full weight once chosen.
#[test]
fn a_chosen_picture_stamps_weights() {
    let mut a = box_app();
    a.viewport_rect = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
    a.run(Command::SpatialFalloff(Some(Falloff::Image(
        ImageFalloff {
            soft_border: 0.,
            ..Default::default()
        },
    ))));
    let dir = std::env::temp_dir().join(format!("operandi-falloff-image-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("white.png");
    image::GrayImage::from_pixel(4, 4, image::Luma([255u8]))
        .save(&path)
        .unwrap();
    a.falloff_image_chosen(path);
    let id = a.editor.scene.active().unwrap().id;
    let w = a.editor.falloff_stamps.get(&(0, id)).expect("stamped");
    assert!(w.iter().all(|&w| (w - 1.).abs() < 1e-6), "{w:?}");
    let _ = std::fs::remove_dir_all(dir);
}
