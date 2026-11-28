//! Render item settings (render item spec v1, RI-xx): new-scene and
//! old-scene values, Denoise on the Final Color output (RI-Q05), the frame
//! size and the camera override, invalid values, undo and save/reopen.
use operandi_core::camera_light_defaults::RENDER_FRAME;
use operandi_core::render_settings::{Caustics, DirectCaustics, RenderSettings};
use operandi_core::*;

/// What `Operandi::initialize_new_document` builds.
fn new_scene() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    e.scene.use_new_scene_render_settings();
    e.next_item_id = e.scene.next_id;
    e
}

fn version(bytes: &[u8]) -> u64 {
    let doc: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    doc["version"].as_u64().unwrap()
}

/// `bytes` as a file of an older build: same contents, `version`.
fn as_version(bytes: &[u8], version: u32) -> Vec<u8> {
    let mut doc: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    doc["version"] = version.into();
    serde_json::to_vec(&doc).unwrap()
}

#[test]
fn a_new_scene_starts_with_the_render_item_defaults() {
    let e = new_scene();
    let r = &e.scene.render;
    assert_eq!(r.samples, render_settings::NEW_SCENE_SAMPLES);
    assert_eq!(r.samples, 64);
    assert_eq!(r.adaptive_threshold, 0.1);
    assert_eq!(r.frame, [1920, 1080]);
    assert_eq!(
        (r.diffuse_depth, r.reflection_depth, r.refraction_depth),
        (8, 4, 8)
    );
    // Preview tab (RI-P01/P02): 97.5 %, 5 minutes.
    assert_eq!(r.preview_convergence, 0.975);
    assert_eq!(r.preview_time_limit, 5.0);
    assert_eq!(r.preview_samples(256), 250);
    assert_eq!(r.preview_seconds(), 300.0);
    assert_eq!(r.max_radiance, 10.0);
    // Render fix F5: new scenes trace only refraction caustics.
    assert_eq!(r.caustics, Caustics::RefractionOnly);
    // RI-Q05: Final Color's Denoising (the one switch) starts on.
    assert!(e.scene.render_denoise());
}

#[test]
fn an_old_scene_keeps_what_it_rendered_with() {
    // A scene saved before the Render item: no render rows at all.
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    let old = as_version(&persistence::encode(&e.scene).unwrap(), 35);
    let scene = persistence::decode(&old).unwrap();
    let r = &scene.render;
    // Samples 16, adaptive sampling off, Cycles' 7 bounces, clamp 10.
    assert_eq!(*r, RenderSettings::default());
    assert_eq!(r.samples, 16);
    assert_eq!(r.adaptive_threshold, 0.0);
    assert_eq!(r.max_bounces(), 7);
    // Its Preview ran to the full target with no time limit.
    assert_eq!(r.preview_samples(256), 256);
    assert_eq!(r.preview_seconds(), 0.0);
    assert!(scene.render_denoise());
}

#[test]
fn every_row_saves_and_reopens() {
    let mut e = new_scene();
    let settings = RenderSettings {
        samples: 1024,
        denoise: true,
        adaptive_threshold: 0.05,
        frame: [1000, 500],
        diffuse_depth: 2,
        reflection_depth: 3,
        refraction_depth: 1,
        max_radiance: 0.0,
        caustics: Caustics::RefractionOnly,
        direct_caustics: Default::default(),
        preview_convergence: 0.5,
        preview_time_limit: 0.25,
        post: Default::default(),
    };
    e.dispatch(Command::SetRenderSettings(settings.clone()))
        .unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(version(&bytes), 36);
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back.render, settings);
    assert_eq!(back, e.scene);
    // Denoising off reopens off.
    let off = RenderSettings {
        denoise: false,
        ..settings
    };
    e.dispatch(Command::SetRenderSettings(off)).unwrap();
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(!back.render_denoise());
}

#[test]
fn only_old_rows_keep_an_older_version_and_new_rows_need_36() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.render.samples = 64;
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(version(&bytes), 27, "older builds can still open it");
    e.scene.render.frame = [800, 600];
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(version(&bytes), 36);
    let err = persistence::decode(&as_version(&bytes, 35)).unwrap_err();
    assert!(err.contains("version 36"), "{err}");
}

#[test]
fn invalid_values_are_refused_and_leave_the_scene() {
    let mut e = new_scene();
    let before = e.scene.clone();
    let steps = e.history.undo.len();
    for bad in [
        RenderSettings {
            frame: [0, 1080],
            ..e.scene.render.clone()
        },
        RenderSettings {
            frame: [1920, 20000],
            ..e.scene.render.clone()
        },
        RenderSettings {
            refraction_depth: 129,
            ..e.scene.render.clone()
        },
        RenderSettings {
            max_radiance: -1.0,
            ..e.scene.render.clone()
        },
        RenderSettings {
            adaptive_threshold: 1.5,
            ..e.scene.render.clone()
        },
        RenderSettings {
            samples: 0,
            ..e.scene.render.clone()
        },
        RenderSettings {
            preview_convergence: 1.2,
            ..e.scene.render.clone()
        },
        RenderSettings {
            preview_time_limit: -1.0,
            ..e.scene.render.clone()
        },
    ] {
        assert!(e.dispatch(Command::SetRenderSettings(bad)).is_err());
    }
    assert_eq!(e.scene, before);
    assert_eq!(e.history.undo.len(), steps);
}

#[test]
fn a_change_is_one_undo_step() {
    let mut e = new_scene();
    let before = e.scene.render.clone();
    let steps = e.history.undo.len();
    let after = RenderSettings {
        reflection_depth: 1,
        ..before.clone()
    };
    e.dispatch(Command::SetRenderSettings(after.clone()))
        .unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Render Settings");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.render, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.render, after);
    // Setting what is already there records nothing.
    e.dispatch(Command::SetRenderSettings(after)).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
}

#[test]
fn the_frame_size_is_the_render_size_unless_the_camera_overrides_it() {
    let mut e = new_scene();
    let id = e.scene.render_camera().unwrap();
    let camera = |e: &Editor| {
        e.scene
            .items
            .iter()
            .find(|i| i.id == id)
            .unwrap()
            .camera
            .clone()
            .unwrap()
    };
    assert_eq!(camera(&e).render_size(e.scene.render.frame), RENDER_FRAME);
    let settings = RenderSettings {
        frame: [1000, 400],
        ..e.scene.render.clone()
    };
    e.dispatch(Command::SetRenderSettings(settings)).unwrap();
    assert_eq!(camera(&e).render_size(e.scene.render.frame), [1000, 400]);
    // CP-V17: Override Render Resolution wins.
    let mut c = camera(&e);
    c.resolution.enable = true;
    c.resolution.width = 640;
    c.resolution.height = 480;
    assert_eq!(c.render_size(e.scene.render.frame), [640, 480]);
}

#[test]
fn max_bounces_is_the_largest_depth() {
    let r = RenderSettings::new_scene();
    assert_eq!(r.max_bounces(), 8);
    assert_eq!(RenderSettings::default().max_bounces(), 7);
    for (c, reflect, refract) in [
        (Caustics::None, false, false),
        (Caustics::ReflectionOnly, true, false),
        (Caustics::RefractionOnly, false, true),
        (Caustics::Both, true, true),
    ] {
        assert_eq!((c.reflective(), c.refractive()), (reflect, refract));
    }
}

/// Direct Caustics spec v1: the Caustics section saves and reopens at
/// version 46; at its defaults it is left out, so the file keeps the
/// older version; bad photon counts are refused.
#[test]
fn direct_caustics_saves_and_reopens() {
    let mut e = new_scene();
    let plain = persistence::encode(&e.scene).unwrap();
    assert!(version(&plain) < 46);
    assert!(!String::from_utf8_lossy(&plain).contains("direct_caustics"));
    let on = RenderSettings {
        direct_caustics: DirectCaustics {
            enabled: true,
            total_photons: 2_000_000,
            local_photons: 64,
            walkthrough: true,
        },
        ..e.scene.render.clone()
    };
    e.dispatch(Command::SetRenderSettings(on.clone())).unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(version(&bytes), 46);
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back.render, on);
    let err = persistence::decode(&as_version(&bytes, 45)).unwrap_err();
    assert!(err.contains("version 46"), "{err}");
    e.dispatch(Command::Undo).unwrap();
    assert!(!e.scene.render.direct_caustics.enabled);
    for bad in [0, 100_000_001] {
        let mut s = on.clone();
        s.direct_caustics.total_photons = bad;
        assert!(e.dispatch(Command::SetRenderSettings(s)).is_err());
    }
    let mut s = on;
    s.direct_caustics.local_photons = 0;
    assert!(e.dispatch(Command::SetRenderSettings(s)).is_err());
}

/// A light's Affect Caustics is one undo step, kept 0..1, and saved.
#[test]
fn affect_caustics_edits_undo_and_save() {
    use operandi_core::camera_light::{LightEdit, LightKind, LightSettings};
    let mut e = new_scene();
    let id = e
        .scene
        .add_light(LightSettings::new(LightKind::Spot), Default::default())
        .unwrap();
    let edit = |v| Command::EditLight {
        item: id,
        edit: LightEdit::AffectCaustics(v),
    };
    let caustics = |e: &Editor| {
        let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
        item.light.as_ref().unwrap().stored.affect_caustics
    };
    assert_eq!(caustics(&e), 1.0);
    e.dispatch(edit(0.0)).unwrap();
    assert_eq!(caustics(&e), 0.0);
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(back, e.scene);
    assert!(e.dispatch(edit(1.5)).is_err());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(caustics(&e), 1.0);
}
