//! Camera Properties in the app (spec camera-properties-2026-11-06): the
//! camera's tabs and rows, Sync to View, Resolution Override reaching
//! renders, the Background Image in Camera view and renders, and saved
//! Film Back presets.
use super::tests::{box_app, draw};
use super::*;
use operandi_core::camera_light::CameraEdit;

const SIZE: Vec2 = vec2(1440., 2600.);

/// Painted text `t` shows `label`: whole, or cut with "…" because the
/// panel is narrow (UIC-111/115 put the full name in the tooltip).
fn shows(t: &str, label: &str) -> bool {
    t == label
        || t.strip_suffix('…')
            .is_some_and(|start| !start.is_empty() && label.starts_with(start))
}

fn texts(out: &egui::FullOutput) -> Vec<(String, f32)> {
    let mut v = Vec::new();
    for s in &out.shapes {
        if let egui::Shape::Text(t) = &s.shape {
            v.push((t.galley.job.text.clone(), t.pos.y));
        }
    }
    v
}

fn properties_texts(a: &mut Operandi) -> Vec<(String, f32)> {
    let ctx = Context::default();
    a.upper_frame.show_page(0);
    draw(a, &ctx, SIZE, vec![]);
    texts(&draw(a, &ctx, SIZE, vec![]))
}

fn y_of(texts: &[(String, f32)], label: &str) -> f32 {
    texts
        .iter()
        .find(|(t, _)| shows(t, label))
        .unwrap_or_else(|| panic!("no {label:?} in {texts:?}"))
        .1
}

fn has(texts: &[(String, f32)], label: &str) -> bool {
    texts.iter().any(|(t, _)| shows(t, label))
}

fn new_camera(a: &mut Operandi) -> u64 {
    a.add_item(operandi_core::item_list::AddItemKind::Camera);
    a.editor.scene.selection.item.unwrap()
}

fn camera(a: &Operandi, id: u64) -> operandi_core::camera_light::CameraSettings {
    a.editor.scene.item(id).unwrap().camera.clone().unwrap()
}

fn write_png(name: &str, w: u32, h: u32) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("operandi-{name}-{}.png", std::process::id()));
    let mut img = image::RgbaImage::new(w, h);
    for p in img.pixels_mut() {
        *p = image::Rgba([255, 0, 0, 255]);
    }
    img.save(&path).unwrap();
    path
}

#[test]
fn camera_tabs_and_the_overlay_tab() {
    let mut a = box_app();
    let id = new_camera(&mut a);
    assert_eq!(
        a.property_tabs(),
        [
            "Camera View",
            "Camera Effects",
            "Display",
            "Assembly",
            "User Channels",
            "Tags"
        ]
    );
    a.run(Command::EditCamera {
        item: id,
        edit: CameraEdit::ShowOverlay(true),
    });
    assert_eq!(a.property_tabs()[2], "Camera Overlay");
    a.properties_tab = "Camera Overlay".into();
    let t = properties_texts(&mut a);
    for label in [
        "Overlay Opacity",
        "Action Safe Visible",
        "Title Safe Visible",
        "Grid",
        "Horizon",
        "Line Color",
    ] {
        y_of(&t, label);
    }
}

#[test]
fn camera_view_rows_follow_the_spec_order() {
    let mut a = box_app();
    new_camera(&mut a);
    a.properties_tab = "Camera View".into();
    let t = properties_texts(&mut a);
    let order = [
        "Name",
        "Reset",
        "Targeting",
        "Target Distance",
        "Sync to View",
        "Projection Type",
        "Focal Length",
        "Lens Preset",
        "Angle of View",
        "Lens Distortion",
        "Override Render Resolution",
        "Width",
        "Height",
        "Images",
        "Film Back Preset",
        "Film Width",
        "Film Fit",
    ];
    for pair in order.windows(2) {
        assert!(
            y_of(&t, pair[0]) < y_of(&t, pair[1]),
            "{} before {}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn camera_effects_and_display_rows() {
    let mut a = box_app();
    new_camera(&mut a);
    a.properties_tab = "Camera Effects".into();
    let t = properties_texts(&mut a);
    let order = [
        "Depth of Field",
        "Focus Distance",
        "Autofocus",
        "F-Stop",
        "Iris Blades",
        "Iris Rotation",
        "Edge Weighting",
        "Clipping Plane",
        "Clipping Distance",
    ];
    for pair in order.windows(2) {
        assert!(y_of(&t, pair[0]) < y_of(&t, pair[1]), "{pair:?}");
    }
    a.properties_tab = "Display".into();
    let t = properties_texts(&mut a);
    for label in ["Size", "Show Camera Overlay", "Show Stereo Volume"] {
        y_of(&t, label);
    }
    // CP-D08: no Dimensions on a camera.
    assert!(!has(&t, "Add Dimensions"));
}

#[test]
fn sync_to_view_moves_the_camera_to_the_view_in_one_step() {
    let mut a = box_app();
    let id = new_camera(&mut a);
    a.camera.view = crate::camera::View::Perspective;
    a.camera.yaw = 0.7;
    a.camera.pitch = -0.3;
    a.camera.distance = 6.0;
    let undo = a.editor.undo.len();
    a.sync_camera_to_view(id);
    assert_eq!(a.editor.undo.len(), undo + 1);
    let frame = operandi_core::camera_light::item_frame(&a.editor.scene, id).unwrap();
    let (eye, _, _, forward) = a.camera.basis();
    assert!((frame.position - eye).length() < 1e-9);
    assert!((frame.forward - forward).length() < 1e-9);
    assert!((camera(&a, id).target_distance - 6.0).abs() < 1e-12);
}

#[test]
fn resolution_override_sets_the_render_size() {
    let mut a = box_app();
    let id = new_camera(&mut a);
    a.run(Command::SetRenderCamera(id));
    let (w, h, _) = a.render_frame(1.0).unwrap();
    assert_eq!((w, h), (1920, 1080));
    for edit in [
        CameraEdit::ResolutionEnable(true),
        CameraEdit::ResolutionWidth(800),
        CameraEdit::ResolutionHeight(800),
    ] {
        a.run(Command::EditCamera { item: id, edit });
    }
    let (w, h, view) = a.render_frame(1.0).unwrap();
    assert_eq!((w, h), (800, 800));
    // Square picture: horizontal and vertical fields match the film width.
    let c = camera(&a, id);
    assert!((view.half_height_tan - c.half_tangents(1.0).1).abs() < 1e-12);
}

#[test]
fn background_image_shows_in_camera_view_and_composites_renders() {
    let mut a = box_app();
    let id = new_camera(&mut a);
    a.run(Command::SetRenderCamera(id));
    let path = write_png("camera-bg", 8, 4);
    let undo = a.editor.undo.len();
    a.camera_background_chosen(id, path.clone());
    assert_eq!(a.editor.undo.len(), undo + 1);
    assert!(camera(&a, id).background.image.is_some());
    let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
    // Only a view looking through the camera draws it, under Backdrops.
    assert!(a.camera_background_draw(r).is_none());
    a.enter_camera_view();
    a.sync_camera_view(r);
    let draw = a.camera_background_draw(r).unwrap();
    assert_eq!(draw.pixels.width, 8);
    assert_eq!(a.backdrop_frame(r).0.items.len(), 1);
    // Off keeps the image but draws nothing.
    a.run(Command::EditCamera {
        item: id,
        edit: CameraEdit::BackgroundEnable(false),
    });
    assert!(a.camera_background_draw(r).is_none());
    a.run(Command::EditCamera {
        item: id,
        edit: CameraEdit::BackgroundEnable(true),
    });
    let (pixels, overscan) = a.render_camera_background().unwrap();
    assert_eq!(overscan, 1.0);
    // An empty 4 × 2 render (transparent film) takes the red picture;
    // a covered pixel keeps its colour.
    let mut rgba = vec![0.0f32; 4 * 2 * 4];
    rgba[0..4].copy_from_slice(&[0.0, 0.5, 0.0, 1.0]);
    crate::camera_props_ui::composite_background(&mut rgba, 4, 2, &pixels, overscan);
    assert_eq!(&rgba[0..4], &[0.0, 0.5, 0.0, 1.0]);
    assert!((rgba[4] - 1.0).abs() < 1e-6 && rgba[5] == 0.0 && rgba[7] == 1.0);
    let _ = std::fs::remove_file(path);
}

#[test]
fn composite_keeps_the_image_shape_and_overscan() {
    let bg = crate::backdrop_gpu::Pixels {
        key: 0,
        width: 1,
        height: 1,
        rgba: vec![255, 255, 255, 255],
    };
    // A square image in a 4 × 2 picture covers the middle 2 columns.
    let mut rgba = vec![0.0f32; 4 * 2 * 4];
    crate::camera_props_ui::composite_background(&mut rgba, 4, 2, &bg, 1.0);
    let alpha: Vec<f32> = rgba.chunks(4).map(|p| p[3]).collect();
    assert_eq!(alpha, [0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0]);
    // Overscan 200 % fills it.
    let mut rgba = vec![0.0f32; 4 * 2 * 4];
    crate::camera_props_ui::composite_background(&mut rgba, 4, 2, &bg, 2.0);
    assert!(rgba.chunks(4).all(|p| p[3] == 1.0));
}

#[test]
fn film_back_presets_save_and_load() {
    let path =
        std::env::temp_dir().join(format!("operandi-film-backs-{}.json", std::process::id()));
    let presets = crate::camera_props_ui::FilmBackPresets {
        presets: vec![operandi_core::camera_props::FilmBackPreset {
            name: "Pocket".into(),
            width: 0.0128,
            height: 0.0096,
        }],
    };
    presets.save(&path).unwrap();
    assert_eq!(
        crate::camera_props_ui::FilmBackPresets::load(&path),
        presets
    );
    let _ = std::fs::remove_file(&path);
    assert!(
        crate::camera_props_ui::FilmBackPresets::load(&path)
            .presets
            .is_empty()
    );
}

#[test]
fn bd_12_a_camera_backdrop_fills_the_camera_frame_only_in_a_camera_view() {
    use operandi_core::backdrop::{BackdropEdit, Projection};
    let mut a = box_app();
    let id = new_camera(&mut a);
    a.run(Command::SetRenderCamera(id));
    let path = write_png("camera-backdrop", 8, 4);
    a.create_backdrop();
    let backdrop = a.editor.scene.selection.item.unwrap();
    a.backdrop_image_chosen(backdrop, path.clone());
    a.run(Command::EditBackdrop {
        item: backdrop,
        edit: BackdropEdit::Projection(Projection::Camera),
    });
    let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
    // Not drawn in the ordinary views (BD-12).
    for view in [View::Front, View::Perspective] {
        a.camera.view = view;
        assert!(a.backdrop_frame(r).0.items.is_empty(), "{view:?}");
    }
    a.enter_camera_view();
    a.sync_camera_view(r);
    let items = a.backdrop_frame(r).0.items;
    assert_eq!(items.len(), 1);
    // Its corners are exactly the render frame's, whatever its own place
    // and scale.
    let frame = a.camera_frame_corners(r, 1., 1.).unwrap();
    assert_eq!(items[0].corners, frame);
    a.editor.scene.item_mut(backdrop).unwrap().transform.scale = DVec3::splat(5.);
    assert_eq!(a.backdrop_frame(r).0.items[0].corners, frame);
    // On screen it fills the frame: stretched to the frame's shape.
    let on_screen: Vec<Pos2> = frame
        .iter()
        .map(|&p| a.camera.project(p, r).unwrap().0)
        .collect();
    let width = on_screen[1].x - on_screen[0].x;
    let height = on_screen[3].y - on_screen[0].y;
    let [w, h] = camera(&a, id).render_size(a.editor.scene.render.frame);
    assert!(
        ((width / height) - w as f32 / h as f32).abs() < 1e-3,
        "{width} × {height}"
    );
    let _ = std::fs::remove_file(path);
}
