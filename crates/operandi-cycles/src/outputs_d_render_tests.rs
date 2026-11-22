//! Render outputs group D through real Cycles renders (2026-11-20):
//! Illumination (Unshadowed) from the lamps-only helper render, Light Path
//! Expressions adding up to the whole picture, Shaded AA Samples' heat map.
#![cfg(feature = "cycles")]
use super::*;
use operandi_core::camera_light::{LightKind, LightSettings};
use operandi_core::mesh::Mesh;
use operandi_core::render_outputs::{OutputSettings, RenderOutput, heat_value};
use operandi_core::scene::Transform;
use operandi_core::shader_tree::OutputEffect;

const SIZE: u32 = 24;
/// Half the picture's height at the floor, metres (camera 6 m above).
const HALF: f64 = 2.4;

fn output(effect: OutputEffect, lpe: &str) -> RenderOutput {
    RenderOutput {
        id: 1,
        name: format!("{} {lpe}", effect.label()),
        effect,
        settings: OutputSettings {
            lpe: lpe.into(),
            ..Default::default()
        },
        in_group: false,
    }
}

/// A 4 m floor and a 0.8 m cube on it.
fn floor_and_cube() -> Scene {
    let mut scene = Scene::default();
    for (centre, size) in [
        (DVec3::new(0.0, 0.0, -0.05), DVec3::new(4.0, 4.0, 0.1)),
        (DVec3::new(0.0, 0.0, 0.4), DVec3::splat(0.8)),
    ] {
        scene.new_mesh().unwrap();
        scene.items.last_mut().unwrap().mesh = Mesh::box_mesh(centre, size, 1).unwrap().into();
    }
    scene
}

fn add_light(scene: &mut Scene, at: DVec3, label: &str) {
    let mut light = LightSettings::new(LightKind::Point);
    light.lpe_label = label.into();
    scene
        .add_light(
            light,
            Transform {
                translation: at,
                ..Default::default()
            },
        )
        .unwrap();
}

fn settings(scene: &Scene, samples: u32) -> Settings {
    let view = View::perspective(
        DVec3::new(0.0, 0.0, 6.0),
        DVec3::X,
        DVec3::Y,
        -DVec3::Z,
        HALF / 6.0,
    );
    let mut s = Settings::studio(SIZE, SIZE, view).with_only_scene_lights(scene);
    s.samples = samples;
    s.gpu = false;
    s.denoise = false;
    s
}

/// Renders the outputs the way a final render does: the main render, the
/// helper when needed, then the pictures.
fn render_outputs(
    scene: &Scene,
    mut s: Settings,
    outputs: &[RenderOutput],
) -> (Vec<(String, Vec<f32>)>, Vec<String>, Settings) {
    let mut meshes = scene_meshes(scene).unwrap();
    s.passes = output_passes(outputs);
    let context = OutputContext::of(scene, &s);
    let notes = prepare_outputs(&mut s, &mut meshes, outputs, &context);
    let mut image = render(&meshes, &s).unwrap();
    if let Some(helper) = unshadowed_settings(&s, outputs) {
        attach_unshadowed(&mut image, render(&meshes, &helper).unwrap());
    }
    (output_pictures_in(outputs, &image, &context), notes, s)
}

/// The mean green of the 3 × 3 pixels round floor point (x, y).
fn at(picture: &[f32], x: f64, y: f64) -> f32 {
    let col = ((x / HALF + 1.0) / 2.0 * f64::from(SIZE)) as i64;
    let row = ((1.0 - y / HALF) / 2.0 * f64::from(SIZE)) as i64;
    let mut sum = 0.0;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (c, r) = ((col + dx) as usize, (row + dy) as usize);
            sum += picture[(r * SIZE as usize + c) * 4 + 1];
        }
    }
    sum / 9.0
}

fn mean(picture: &[f32]) -> f32 {
    picture.chunks(4).map(|p| p[0] + p[1] + p[2]).sum::<f32>() / picture.len() as f32 * 4.0
}

/// D-Q1 (Jason, 2026-11-20): the lamp's light with no shadow; no world,
/// no bounce light. The floor in the cube's shadow is lit; away from the
/// cube Unshadowed is Illumination (Direct).
#[test]
fn unshadowed_has_no_shadow_patch() {
    let mut scene = floor_and_cube();
    add_light(&mut scene, DVec3::new(1.5, 0.0, 1.2), "");
    let mut s = settings(&scene, 32);
    s.world = Vec::new();
    let outputs = [
        output(OutputEffect::IlluminationDirect, ""),
        output(OutputEffect::IlluminationUnshadowed, ""),
        output(OutputEffect::DiffuseShadingUnshadowed, ""),
        output(OutputEffect::DiffuseColor, ""),
    ];
    let (pics, ..) = render_outputs(&scene, s, &outputs);
    let (direct, unshadowed, shading, colour) = (&pics[0].1, &pics[1].1, &pics[2].1, &pics[3].1);
    // In the cube's shadow (the lamp is at +x).
    let shadow = (-1.0, 0.0);
    assert!(
        at(direct, shadow.0, shadow.1) < 0.2 * at(unshadowed, shadow.0, shadow.1),
        "the shadow is gone: direct {} unshadowed {}",
        at(direct, shadow.0, shadow.1),
        at(unshadowed, shadow.0, shadow.1)
    );
    // Away from the cube they agree.
    let (d, u) = (at(direct, 1.0, 1.2), at(unshadowed, 1.0, 1.2));
    assert!(
        d > 0.0 && (u - d).abs() < 0.15 * d,
        "direct {d} unshadowed {u}"
    );
    // Diffuse Shading (Unshadowed) = it × the surface colour (D-Q4).
    let c = at(colour, 1.0, 1.2);
    let sh = at(shading, 1.0, 1.2);
    assert!((sh - u * c).abs() < 0.1 * u * c + 1e-4, "{sh} vs {}", u * c);
}

/// D-Q1: a sky-only scene renders black, though the sky lights it.
#[test]
fn unshadowed_is_black_without_lamps() {
    let scene = floor_and_cube();
    let s = settings(&scene, 16);
    assert!(!s.world.is_empty(), "the studio sky lights it");
    let outputs = [
        output(OutputEffect::IlluminationTotal, ""),
        output(OutputEffect::IlluminationUnshadowed, ""),
    ];
    let (pics, ..) = render_outputs(&scene, s, &outputs);
    assert!(mean(&pics[0].1) > 0.05, "the sky lights the floor");
    assert!(
        mean(&pics[1].1) < 1e-4,
        "no lamp: black {}",
        mean(&pics[1].1)
    );
}

/// The parts add up to the whole: diffuse + glossy + transmission +
/// glowing surfaces + background = C.*, and the lights' groups add up to
/// C.*L.
#[test]
fn light_path_expressions_add_up() {
    let mut scene = floor_and_cube();
    add_light(&mut scene, DVec3::new(1.5, 0.0, 1.2), "Key");
    add_light(&mut scene, DVec3::new(-1.5, 1.0, 1.5), "Rim");
    add_light(&mut scene, DVec3::new(0.0, -1.5, 2.0), "");
    let s = settings(&scene, 32);
    let parts = ["C<RD>.*", "C<RS>.*", "C<TS>.*", "CO", "CB", "CV"];
    let mut outputs: Vec<RenderOutput> = ["C.*", "C.*L", "C.*L'Key'", "C.*L'Rim'"]
        .iter()
        .chain(&parts)
        .map(|e| output(OutputEffect::LightPathExpressions, e))
        .collect();
    outputs.push(output(OutputEffect::FinalColor, ""));
    outputs.push(output(OutputEffect::LightPathExpressions, "CDS.*"));
    let (pics, notes, s) = render_outputs(&scene, s, &outputs);
    // The hidden group for the unlabelled lamp.
    assert!(
        s.light_groups.iter().any(|g| g == LPE_LIGHTS),
        "{:?}",
        s.light_groups
    );
    let all = mean(&pics[0].1);
    assert!(all > 0.05);
    // C.* is Final Color before image processing.
    assert!((mean(&pics[10].1) - all).abs() < 1e-5);
    let sum: f32 = (4..10).map(|k| mean(&pics[k].1)).sum();
    assert!((sum - all).abs() < 0.03 * all, "parts {sum} whole {all}");
    // The labelled lamps are part of all the lamps.
    let (lamps, key, rim) = (mean(&pics[1].1), mean(&pics[2].1), mean(&pics[3].1));
    assert!(
        key > 0.0 && rim > 0.0 && key + rim < lamps,
        "{key} {rim} {lamps}"
    );
    // The world lights the scene too, so the lamps are less than the whole.
    assert!(lamps < all);
    // Unsupported: black, with a note for the status line.
    assert_eq!(mean(&pics[11].1), 0.0);
    assert!(
        notes
            .iter()
            .any(|n| n.contains("Not supported by the renderer")),
        "{notes:?}"
    );
}

/// Shaded AA Samples: with adaptive sampling some pixels stop early
/// (bluer); without it every pixel is red and no pass is asked for.
#[test]
fn shaded_aa_samples_heat_map() {
    let mut scene = floor_and_cube();
    add_light(&mut scene, DVec3::new(1.5, 0.0, 1.2), "");
    let outputs = [output(OutputEffect::ShadedAaSamples, "")];
    let mut s = settings(&scene, 128);
    s.adaptive_threshold = 0.1;
    let (pics, notes, s2) = render_outputs(&scene, s, &outputs);
    assert!(s2.passes.contains(&Pass::SampleCount));
    assert!(notes.is_empty(), "{notes:?}");
    let shares: Vec<f32> = pics[0]
        .1
        .chunks(4)
        .map(|p| heat_value([p[0], p[1], p[2]]))
        .collect();
    let min = shares.iter().copied().fold(1.0f32, f32::min);
    assert!(min < 0.5, "some pixels stop early: least {min}");
    assert!(shares.iter().all(|t| (0.0..=1.0).contains(t)));
    // Adaptive sampling off: solid red, no pass, the notice.
    let (pics, notes, s3) = render_outputs(&scene, settings(&scene, 8), &outputs);
    assert!(!s3.passes.contains(&Pass::SampleCount));
    assert_eq!(notes, [AA_OFF_NOTE]);
    assert!(
        pics[0]
            .1
            .chunks(4)
            .all(|p| p[0] == 1.0 && p[1] == 0.0 && p[2] == 0.0)
    );
}
