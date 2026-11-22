//! Painting stage 6: Preferences > Images & Painting and Default Tools
//! (PNT-214), the Images list's details and the Image Viewer (PNT-216).
use super::*;
use crate::tests::draw;
use std::sync::Arc;

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

fn app() -> Operandi {
    crate::layouts::ship_paint_layout_in_tests();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a
}

#[test]
fn the_pages_draw_and_their_rows_are_pref_value_commands() {
    let mut a = app();
    let ctx = Context::default();
    a.open_preferences();
    for page in [
        crate::preferences::Page::ImagesPainting,
        crate::preferences::Page::DefaultTools,
    ] {
        a.preferences_ui.page = page;
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    let p = &a.painting.prefs;
    assert!(p.antialiasing && p.auto_create && !p.images_as_icons);
    assert_eq!(p.new_resolution, 2048);
    assert_eq!(p.max_brush, 500.);
    assert_eq!(p.default_fg, crate::painting_prefs::DefaultFg::White);
    for label in [
        "Enable Image Antialiasing by Default",
        "Default New Image Resolution",
        "Always Display Images as Icons",
        "Auto Create Textures",
        "Default Foreground Color",
        "Maximum Brush Size",
    ] {
        assert!(
            crate::prefs_commands::pref_for_label(crate::preferences::Page::ImagesPainting, label)
                .is_some(),
            "{label}"
        );
    }
    assert!(
        crate::prefs_commands::pref_for_label(
            crate::preferences::Page::DefaultTools,
            "Default Brush"
        )
        .is_some()
    );
}

#[test]
fn the_preferences_change_what_painting_does() {
    let mut a = app();
    a.execute("pref.value painting.newImageResolution 512");
    a.execute("pref.value painting.antialiasing false");
    a.execute("operandi.paint.addTexture kind:color color:0,0,0");
    let image = a.current_image().unwrap();
    let info = a
        .editor
        .scene
        .images
        .iter()
        .find(|i| i.id == image)
        .unwrap();
    assert_eq!((info.width, info.height), (512, 512), "{}", a.status);
    let layer = a.paint_target().unwrap();
    let aa = match &a.editor.scene.shader_tree.get(layer).unwrap().kind {
        operandi_core::shader_tree::NodeKind::Texture(t) => match &t.source {
            operandi_core::shader_tree::TextureSource::ImageMap { settings, .. } => {
                settings.antialiasing
            }
            _ => panic!(),
        },
        _ => panic!(),
    };
    assert!(!aa, "Add Texture's layer follows the preference");
    // Default Foreground Color: every tool's FG.
    a.execute("pref.value painting.defaultForeground Black");
    a.execute("operandi.paint.airbrush");
    assert_eq!(
        a.painting.brush.settings(PaintToolAlias::Airbrush).fg.color,
        [0.; 3]
    );
    // Maximum Brush Size caps the tip.
    a.execute("pref.value painting.maxBrushSize 40");
    a.execute("operandi.paint.set size:300");
    assert_eq!(a.painting.brush.tip().size, 40.);
    // Default Brush: a tool turned on starts with it.
    a.execute("operandi.paint.airbrush");
    a.execute("pref.value painting.defaultBrush Hard");
    a.execute("operandi.paint.tip kind:procedural");
    a.execute("operandi.paint.airbrush");
    a.execute("operandi.paint.paintbrush");
    assert_eq!(
        a.painting.brush.tips.choice,
        operandi_core::paint_tips::TipChoice::Hard
    );
    a.execute("pref.value painting.defaultBrush \"Last Selected\"");
    a.execute("operandi.paint.tip kind:sphere");
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.paintbrush");
    assert_eq!(
        a.painting.brush.tips.choice,
        operandi_core::paint_tips::TipChoice::Sphere
    );
}

use operandi_core::paint_stroke::PaintTool as PaintToolAlias;

#[test]
fn the_image_viewer_shows_the_image_live_while_painting() {
    let mut a = app();
    let ctx = Context::default();
    a.execute("operandi.paint.addTexture kind:color resolution:256 color:0,0,0");
    let image = a.current_image().unwrap();
    a.execute("operandi.paint.images");
    a.execute(&format!("operandi.image.view image:{image}"));
    let _ = draw(&mut a, &ctx, SIZE, vec![]);
    let _ = draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.line_state("operandi.image.view").unwrap().on);
    let shown = a.paint_viewer_picture().unwrap();
    assert!(Arc::ptr_eq(&shown, &a.editor.scene.paint.buffers[&image]));
    // A stroke in the 3D view: the viewer's picture is the live one.
    a.execute("operandi.paint.airbrush");
    let r = a.viewport_rect;
    a.paint_press(r.center(), r, Modifiers::NONE);
    std::thread::sleep(std::time::Duration::from_millis(40));
    for k in 1..10 {
        a.paint_drag(r.center() + vec2(k as f32 * 4., 0.), r);
    }
    let during = a.paint_viewer_picture().unwrap();
    assert!(
        !Arc::ptr_eq(&during, &shown),
        "the viewer follows the stroke"
    );
    a.finish_paint_stroke();
    let _ = draw(&mut a, &ctx, SIZE, vec![]);
    // The list shows the format and the unsaved mark.
    let px = &a.editor.scene.paint.buffers[&image];
    assert_eq!(crate::painting_viewer::format_text(px), "RGBA 8-bit");
    assert!(a.editor.image_changed(image));
    // A non-Photoshop image has no layers to pick.
    a.execute(&format!(
        "operandi.image.view image:{image} layer:Background"
    ));
    assert_eq!(a.status, "Only a Photoshop image has layers");
}
