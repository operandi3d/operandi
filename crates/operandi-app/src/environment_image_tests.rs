//! HDRI environment stage 1: Load Image…, Clear Image and Rotation on an
//! environment, through the commands their controls run.
use super::*;

/// A tiny picture on disk (PNG: the app reads its size with `image`).
fn picture(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-env-image-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    image::RgbImage::from_pixel(8, 4, image::Rgb([200, 120, 40]))
        .save(&path)
        .unwrap();
    path
}

fn top_env(a: &Operandi) -> u64 {
    a.editor.scene.environments().next().unwrap().id
}

#[test]
fn load_image_command_fills_the_top_environment_in_one_step() {
    let mut a = Operandi::empty();
    let path = picture("load-sky.png");
    let undo = a.editor.undo.len();
    a.run_command_line(&format!(
        "operandi.environment.loadImage filename:{{{}}}",
        path.display()
    ));
    let env = top_env(&a);
    let image = a.editor.scene.environment_image(env).expect("an image");
    assert_eq!((image.width, image.height), (8, 4));
    // An empty scene's default environment is made first, then one step.
    assert!(a.editor.undo.len() <= undo + 2);
    let steps = a.editor.undo.len();
    // Clear Image runs texture.delete on the image layer: one step.
    let layer = a.editor.scene.environment_image_layer(env).unwrap().id;
    a.run_command_line(&format!("texture.delete item:{layer}"));
    assert!(a.editor.scene.environment_image_layer(env).is_none());
    assert_eq!(a.editor.undo.len(), steps + 1);
    a.run(Command::Undo);
    assert!(a.editor.scene.environment_image(env).is_some());
}

#[test]
fn load_image_without_a_file_asks_for_one() {
    fn stand_in(request: crate::file_chooser::Choose) -> Option<std::path::PathBuf> {
        assert!(matches!(
            request,
            crate::file_chooser::Choose::EnvironmentImage { .. }
        ));
        Some(picture("asked-sky.png"))
    }
    let mut a = Operandi::empty();
    a.run(Command::AddItem(
        operandi_core::item_list::AddItemKind::Environment,
    ));
    let env = top_env(&a);
    a.chooser = Some(stand_in);
    a.run_command_line(&format!("operandi.environment.loadImage item:{env}"));
    assert!(
        a.editor
            .scene
            .environment_image(env)
            .is_some_and(|i| i.path.ends_with("asked-sky.png"))
    );
}

#[test]
fn rotation_is_the_locator_heading_through_transform_channel() {
    let mut a = Operandi::empty();
    let path = picture("turn-sky.png");
    assert!(a.environment_image_chosen(None, &path));
    let env = top_env(&a);
    let (locator, heading) = a.editor.scene.environment_image_heading(env).unwrap();
    assert_eq!(heading, 0.0);
    let steps = a.editor.undo.len();
    a.run_command_line(&format!("transform.channel rot.Y 30deg item:{locator}"));
    let (_, heading) = a.editor.scene.environment_image_heading(env).unwrap();
    assert!((heading - 30f64.to_radians()).abs() < 1e-9, "{heading}");
    assert_eq!(a.editor.undo.len(), steps + 1);
}

#[test]
fn environment_tab_draws_with_and_without_an_image() {
    let mut a = Operandi::empty();
    a.run(Command::AddItem(
        operandi_core::item_list::AddItemKind::Environment,
    ));
    let env = top_env(&a);
    let ctx = Context::default();
    theme::install(&ctx);
    let draw = |a: &mut Operandi| {
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| a.environment_item_rows(ui, env));
        });
    };
    draw(&mut a);
    assert!(a.environment_image_chosen(Some(env), &picture("drawn-sky.png")));
    draw(&mut a);
}

/// A tiny Radiance HDR file on disk.
fn hdr_file(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-env-image-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let img = image::Rgb32FImage::from_pixel(8, 4, image::Rgb([2.0, 1.0, 0.5]));
    image::DynamicImage::ImageRgb32F(img).save(&path).unwrap();
    path
}

/// HDRI stage 4 (UIC-74 deviation, Jason OK 2026-11-12): an .hdr dropped
/// on a view's empty space is the top environment's image; a PNG there is
/// still a Backdrop, and so is an .hdr dropped on a mesh.
#[test]
fn hdr_dropped_on_empty_view_space_becomes_the_environment() {
    let mut a = Operandi::empty();
    let view = Rect::from_min_size(Pos2::ZERO, vec2(400., 300.));
    a.corner.areas.insert(a.active_view, view);
    a.viewport_rect = view;
    a.camera.view = View::Top;
    let hdr = hdr_file("drop-sky.hdr");
    let steps = a.editor.undo.len();
    a.file_dropped(hdr.clone(), Some(pos2(10., 10.)));
    let env = top_env(&a);
    assert!(
        a.editor
            .scene
            .environment_image(env)
            .is_some_and(|i| i.path.ends_with("drop-sky.hdr")),
        "{}",
        a.status
    );
    assert!(a.editor.scene.items.iter().all(|i| i.backdrop.is_none()));
    assert!(a.editor.undo.len() <= steps + 2);
    // A PNG on empty space: a Backdrop, as before.
    a.file_dropped(picture("drop-plain.png"), Some(pos2(10., 10.)));
    assert!(a.editor.scene.items.iter().any(|i| i.backdrop.is_some()));
    // Off the views: an image entry only.
    let before = a.editor.scene.items.len();
    a.file_dropped(hdr_file("drop-off.hdr"), Some(pos2(900., 100.)));
    assert_eq!(a.editor.scene.items.len(), before);
    assert!(
        a.editor
            .scene
            .environment_image(env)
            .is_some_and(|i| i.path.ends_with("drop-sky.hdr"))
    );
}

/// MN-07.1: on a mesh an .hdr is a texture of that surface, like any
/// picture (it was a Backdrop before).
#[test]
fn hdr_dropped_on_a_mesh_is_a_texture() {
    let mut a = super::tests::box_app();
    let view = Rect::from_min_size(Pos2::ZERO, vec2(400., 300.));
    a.corner.areas.insert(a.active_view, view);
    a.viewport_rect = view;
    a.camera.view = View::Top;
    let centre = a.camera.project(glam::DVec3::ZERO, view).unwrap().0;
    assert!(
        a.drop_geometry(centre).is_some(),
        "the box is under the pointer"
    );
    a.file_dropped(hdr_file("on-mesh.hdr"), Some(centre));
    assert!(a.editor.scene.items.iter().all(|i| i.backdrop.is_none()));
    assert!(a.status.starts_with("Image Map added"), "{}", a.status);
}
