//! The Render window's Image Processing tab (post_effects_ui.rs): Bloom
//! and Glare change the shown picture without rendering again, leave the
//! rendered pixels and other outputs alone, and go into Save Image.
use crate::Operandi;
use crate::image_viewer::{self, SaveFormat};
use crate::image_viewer_ui::SaveDialog;
use crate::render_ui::Picture;
use crate::tests::box_app;
use eframe::egui;
use operandi_core::Command;

const W: u32 = 48;
const H: u32 = 32;

/// A grey picture with one bright pixel, as if just rendered, and a Depth
/// output beside it.
fn app() -> Operandi {
    let mut a = box_app();
    let mut rgba = vec![0.2f32; (W * H * 4) as usize];
    let i = ((16 * W + 24) * 4) as usize;
    rgba[i..i + 3].copy_from_slice(&[40.0; 3]);
    a.render.picture = Some(Picture {
        width: W,
        height: H,
        rgba: rgba.clone(),
        slot: None,
    });
    a.render.outputs = vec![("Depth".into(), rgba)];
    a.render.viewer.show_live();
    a
}

fn set_bloom(a: &mut Operandi, on: bool) {
    let mut s = a.editor.scene.render.clone();
    s.post.bloom.enabled = on;
    s.post.glare.enabled = on;
    a.run(Command::SetRenderSettings(s));
}

fn raw(a: &Operandi) -> Vec<f32> {
    a.render.picture.as_ref().unwrap().rgba.clone()
}

#[test]
fn turning_bloom_on_and_off_changes_only_what_is_shown() {
    let mut a = app();
    let rendered = raw(&a);
    a.refresh_post();
    assert_eq!(a.shown_pixels().unwrap().2, rendered.as_slice());
    set_bloom(&mut a, true);
    a.render.texture_stale = false;
    a.refresh_post();
    // The window shows the glow at once, and repaints for it.
    assert!(a.render.texture_stale);
    let shown = a.shown_pixels().unwrap().2.to_vec();
    assert_ne!(shown, rendered);
    let near = ((16 * W + 26) * 4) as usize;
    assert!(shown[near] > 0.2);
    // The rendered picture is kept as it was, for the next change.
    assert_eq!(raw(&a), rendered);
    assert_eq!(a.shown_raw_pixels().unwrap().2, rendered.as_slice());
    // Nothing changed: nothing is worked out again.
    a.render.texture_stale = false;
    a.refresh_post();
    assert!(!a.render.texture_stale);
    // Other outputs keep their numbers.
    a.render.shown = Some(0);
    a.refresh_post();
    assert_eq!(a.shown_pixels().unwrap().2, rendered.as_slice());
    a.render.shown = None;
    // Undo turns it off again: the picture as rendered.
    a.run(Command::Undo);
    a.refresh_post();
    assert!(!a.editor.scene.render.post.is_active());
    assert_eq!(a.shown_pixels().unwrap().2, rendered.as_slice());
}

#[test]
fn picture_formats_save_with_bloom_float_files_and_other_outputs_without() {
    let mut a = app();
    set_bloom(&mut a, true);
    a.refresh_post();
    // As the window shows it, clipped to what a picture file holds.
    let shown: Vec<f32> = a
        .shown_pixels()
        .unwrap()
        .2
        .iter()
        .map(|v| v.min(1.0))
        .collect();
    let folder = std::env::temp_dir().join(format!("operandi-post-save-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let save = |a: &mut Operandi, name: &str, output: Option<usize>, format: SaveFormat| {
        a.viewer_save_to(
            &folder.join(name),
            SaveDialog {
                slot: None,
                output,
                format,
            },
        );
        let file = folder.join(format!("{name}.{}", format.extension()));
        image_viewer::load_image(&file)
            .unwrap_or_else(|e| panic!("{e}: {}", a.status))
            .2
    };
    let close = |x: &[f32], y: &[f32]| x.iter().zip(y).all(|(p, q)| (p - q).abs() < 2e-3);
    // PNG (and every picture format): the glow is in the file.
    let png = save(&mut a, "picture", None, SaveFormat::Png16);
    assert!(close(&png, &shown));
    // EXR and Multilayer EXR keep the numbers as rendered (BG-Q4).
    let exr = save(&mut a, "float", None, SaveFormat::Exr);
    assert!(close(&exr, &raw(&a)));
    let layers = save(&mut a, "layers", None, SaveFormat::ExrMultilayer);
    assert!(close(&layers, &raw(&a)));
    // Other outputs as rendered in any format.
    let depth = save(&mut a, "depth", Some(0), SaveFormat::Exr);
    assert!(close(&depth, &raw(&a)));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_tab_draws_off_and_on_and_changes_nothing_by_itself() {
    let mut a = app();
    let ctx = egui::Context::default();
    let frame = |a: &mut Operandi| {
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| a.image_processing_tab(ui));
        });
    };
    frame(&mut a);
    set_bloom(&mut a, true);
    frame(&mut a);
    // Drawing the tab changes nothing by itself.
    assert!(a.editor.scene.render.post.bloom.enabled);
    assert_eq!(
        a.editor.history.undo.last().map(|r| r.label.as_str()),
        Some("Render Settings")
    );
}
