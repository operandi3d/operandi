//! Cycles built-in Render Outputs in the app (2026-11-20): colour spaces
//! when saving and showing, the Cryptomatte cache and OpenEXR Multilayer
//! saves, through a stand-in for Cycles.
use crate::Operandi;
use crate::image_viewer::{SaveFormat, ViewTransform};
use crate::image_viewer_ui::SaveDialog;
use crate::tests::{box_app, draw};
use eframe::egui::{self, Vec2};
use operandi_core::render_outputs::{
    CryptomatteType, OutputColorspace, OutputSettings, cryptomatte_metadata_prefix, hash_to_float,
    murmur3_32,
};
use operandi_core::shader_tree::{NewLayer, OutputEffect};
use operandi_cycles::{Image, Pass, RenderControl, RenderMesh, Settings};
use std::path::PathBuf;
use std::time::Duration;

/// Stands in for Cycles: grey 0.5 everywhere, every pass 0.5 except the
/// first Object cryptomatte pass, which ranks "Box" with full coverage.
fn fake(
    _meshes: &[RenderMesh],
    s: &Settings,
    _base: Option<Vec<f32>>,
    _control: &RenderControl,
) -> operandi_core::Result<Image> {
    let n = s.width as usize * s.height as usize;
    let id = hash_to_float(murmur3_32(b"Box"));
    let passes = s
        .passes
        .iter()
        .map(|&p| {
            let px = match p {
                Pass::CryptoObject0 => [id, 1.0, 0.0, 0.0],
                _ if p.crypto_kind().is_some() => [0.0; 4],
                _ => [0.5; 4],
            };
            (p, px.repeat(n))
        })
        .collect();
    Ok(Image {
        width: s.width,
        height: s.height,
        rgba: vec![0.5; n * 4],
        device: "Test".into(),
        used_gpu: false,
        denoised: false,
        samples: s.samples,
        light_groups: Vec::new(),
        passes,
        values: Vec::new(),
        cryptomatte_manifests: vec![(
            CryptomatteType::Object,
            format!(r#"{{"Box":"{:08x}","":"00000000"}}"#, murmur3_32(b"Box")),
        )],
    })
}

fn app(name: &str) -> (Operandi, egui::Context, PathBuf) {
    let folder =
        std::env::temp_dir().join(format!("operandi-ro-save-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let mut a = box_app();
    a.camera.view = crate::camera::View::Perspective;
    a.render.set_renderer(fake, folder.clone());
    let ctx = egui::Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, Vec2::new(1000., 700.), vec![]);
    }
    (a, ctx, folder)
}

fn wait(a: &mut Operandi, ctx: &egui::Context) {
    for _ in 0..12_000 {
        if !a.render.busy() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
        a.poll_render(ctx);
    }
    panic!("render never finished");
}

fn add(a: &mut Operandi, effect: OutputEffect, settings: OutputSettings) -> u64 {
    let tree = &mut a.editor.scene.shader_tree;
    let id = tree.add(NewLayer::RenderOutput(effect), None).unwrap();
    if settings != OutputSettings::default() {
        tree.outputs.insert(id, settings);
    }
    id
}

/// Data outputs save their linear numbers (0.5 → 128, not sRGB's 188);
/// a beauty output forced Linear does too; Default beauty stays sRGB.
#[test]
fn saved_outputs_follow_their_colour_space() {
    let (mut a, ctx, folder) = app("colour");
    add(&mut a, OutputEffect::Depth, OutputSettings::default());
    add(
        &mut a,
        OutputEffect::IlluminationDirect,
        OutputSettings {
            colorspace: OutputColorspace::Linear,
            ..Default::default()
        },
    );
    add(
        &mut a,
        OutputEffect::IlluminationIndirect,
        OutputSettings::default(),
    );
    let render = folder.join("Render.png");
    a.render.overrides = Some(crate::render_ui::RenderOverrides {
        size: Some([8, 4]),
        path: Some(render.clone()),
        ..Default::default()
    });
    a.start_render(&ctx);
    wait(&mut a, &ctx);
    assert!(a.status.starts_with("Rendered"), "{}", a.status);
    let red = |name: &str| {
        let path = crate::render_ui::light_group_path(&folder.join("Render.png"), name);
        image::open(&path)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
            .into_rgba8()
            .get_pixel(0, 0)[0]
    };
    assert_eq!(red("Depth Output"), 128, "data: linear");
    assert_eq!(red("Illumination (Direct) Output"), 128, "forced linear");
    assert_eq!(red("Illumination (Indirect) Output"), 188, "beauty: sRGB");
    // The window shows the data output raw, the beauty one by its switch.
    let slot = a.render.history.slots[0].clone();
    let k = |name: &str| slot.outputs.iter().position(|n| n == name).unwrap();
    let depth = slot.meta(k("Depth Output"));
    assert_eq!(depth.effect, Some(OutputEffect::Depth));
    assert_eq!(depth.view(ViewTransform::Srgb), ViewTransform::Linear);
    let forced = slot.meta(k("Illumination (Direct) Output"));
    assert_eq!(forced.view(ViewTransform::Srgb), ViewTransform::Linear);
    let beauty = slot.meta(k("Illumination (Indirect) Output"));
    assert_eq!(beauty.view(ViewTransform::Srgb), ViewTransform::Srgb);
    assert_eq!(beauty.view(ViewTransform::Linear), ViewTransform::Linear);
    let _ = std::fs::remove_dir_all(&folder);
}

/// A Cryptomatte render keeps its raw layers in the cache; Save Image as
/// OpenEXR Multilayer writes every output and the Cryptomatte layers and
/// metadata into one file, from the live picture and from a history slot.
#[test]
fn multilayer_save_has_every_output_and_cryptomatte() {
    let (mut a, ctx, folder) = app("multilayer");
    add(&mut a, OutputEffect::Depth, OutputSettings::default());
    add(&mut a, OutputEffect::Cryptomatte, OutputSettings::default());
    a.render.overrides = Some(crate::render_ui::RenderOverrides {
        size: Some([8, 4]),
        ..Default::default()
    });
    a.start_render(&ctx);
    wait(&mut a, &ctx);
    let slot = a.render.history.slots[0].clone();
    assert!(slot.crypto_file(&a.render.history.folder).is_file());
    for (which, slot_id) in [("live", None), ("slot", Some(slot.id))] {
        let path = folder.join(format!("{which}.exr"));
        if slot_id.is_some() {
            // Not the picture on screen: read from the cache.
            a.render.picture.as_mut().unwrap().slot = None;
        }
        a.viewer_save_to(
            &path,
            SaveDialog {
                slot: slot_id,
                output: None,
                format: SaveFormat::ExrMultilayer,
            },
        );
        assert!(a.status.starts_with("Saved"), "{which}: {}", a.status);
        let image = exr::prelude::read_all_flat_layers_from_file(&path).unwrap();
        let layer = &image.layer_data[0];
        let names: Vec<String> = layer
            .channel_data
            .list
            .iter()
            .map(|c| String::from_utf8_lossy(c.name.as_slice()).into_owned())
            .collect();
        for want in [
            "R",
            "A",
            "Depth Output.R",
            "Cryptomatte Output.G",
            "CryptoObject00.R",
            "CryptoObject02.A",
        ] {
            assert!(
                names.iter().any(|n| n == want),
                "{which}: no {want} in {names:?}"
            );
        }
        let key = cryptomatte_metadata_prefix("CryptoObject");
        let manifest = layer
            .attributes
            .other
            .iter()
            .find(|(k, _)| k.as_slice() == format!("{key}manifest").as_bytes())
            .map(|(_, v)| format!("{v:?}"))
            .unwrap_or_else(|| panic!("{which}: no manifest"));
        assert!(manifest.contains("Box"), "{manifest}");
    }
    let _ = std::fs::remove_dir_all(&folder);
}
