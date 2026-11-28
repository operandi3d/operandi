//! Bloom and Glare settings (`RenderSettings::post`): off in new and old
//! scenes, one undo step, save/reopen at native version 68, refused by a
//! version 67 reader, and invalid values refused.
use operandi_core::post_effects::PostEffects;
use operandi_core::*;

fn version(bytes: &[u8]) -> u64 {
    let doc: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    doc["version"].as_u64().unwrap()
}

fn with_bloom_and_glare(e: &Editor) -> render_settings::RenderSettings {
    let mut s = e.scene.render.clone();
    s.post.bloom.enabled = true;
    s.post.bloom.radius = 0.1;
    s.post.bloom.tint = [1.0, 0.5, 0.25];
    s.post.glare.enabled = true;
    s.post.glare.streaks = 6;
    s.post.glare.angle = 15.0;
    s
}

#[test]
fn new_and_old_scenes_start_with_both_off_and_files_leave_them_out() {
    let mut e = Editor::default();
    e.scene.use_new_scene_render_settings();
    assert!(e.scene.render.post.is_default());
    assert!(!e.scene.render.post.is_active());
    assert!(render_settings::RenderSettings::default().post.is_default());
    let bytes = persistence::encode(&e.scene).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("\"post\""));
    assert!(version(&bytes) < 68);
}

#[test]
fn a_change_is_one_undo_step_and_reopens() {
    let mut e = Editor::default();
    let before = e.scene.render.clone();
    let s = with_bloom_and_glare(&e);
    e.dispatch(Command::SetRenderSettings(s.clone())).unwrap();
    assert_eq!(e.scene.render, s);
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(version(&bytes), 68);
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back.render.post, s.post);
    assert_eq!(e.history.undo.last().unwrap().label, "Render Settings");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.render, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.render, s);
}

#[test]
fn a_version_67_file_with_them_is_refused() {
    let mut e = Editor::default();
    e.dispatch(Command::SetRenderSettings(with_bloom_and_glare(&e)))
        .unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    let mut doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    doc["version"] = 67.into();
    let old = serde_json::to_vec(&doc).unwrap();
    let err = persistence::decode(&old).unwrap_err();
    assert!(err.contains("68"), "{err}");
}

/// BG-Q2: the Final Color output's own Bloom is gone. An old file's
/// becomes the scene's Bloom, and is never written back to the output.
#[test]
fn an_old_files_output_bloom_becomes_the_scenes_bloom() {
    let mut e = Editor::default();
    e.scene.use_new_scene_render_settings();
    let id = e
        .scene
        .shader_tree
        .render_outputs()
        .into_iter()
        .find(|o| o.effect == shader_tree::OutputEffect::FinalColor)
        .expect("a new scene has a Final Color output")
        .id;
    let bytes = persistence::encode(&e.scene).unwrap();
    let mut doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    doc["version"] = 55.into();
    // A scene with the starting tree leaves it out of the file.
    if doc["scene"]["shader_tree"].is_null() {
        doc["scene"]["shader_tree"] = serde_json::to_value(&e.scene.shader_tree).unwrap();
    }
    let outputs = &mut doc["scene"]["shader_tree"]["outputs"];
    if outputs.is_null() {
        *outputs = serde_json::json!({});
    }
    let key = serde_json::to_value(id).unwrap().to_string();
    outputs[key.trim_matches('"')] =
        serde_json::json!({"bloom": true, "bloom_threshold": 2.0, "bloom_radius": 0.03});
    let old = serde_json::to_vec(&doc).unwrap();
    let scene = persistence::decode(&old).unwrap();
    let b = &scene.render.post.bloom;
    assert!(b.enabled);
    assert_eq!(b.threshold, 2.0);
    assert_eq!(b.intensity, 1.0);
    // 3 % of a 1920 × 1080 frame's diagonal, as a share of its width.
    assert!((b.radius - 0.0229).abs() < 1e-3, "{}", b.radius);
    assert!(!scene.shader_tree.output_settings(id).bloom);
    let again = persistence::encode(&scene).unwrap();
    let text = String::from_utf8_lossy(&again);
    assert!(!text.contains("bloom_threshold"));
    assert_eq!(version(&again), 68);
}

#[test]
fn invalid_values_are_refused_and_change_nothing() {
    let mut e = Editor::default();
    let before = e.scene.render.clone();
    let mut s = before.clone();
    s.post = PostEffects::default();
    s.post.glare.streaks = 5;
    assert!(e.dispatch(Command::SetRenderSettings(s)).is_err());
    let mut s = before.clone();
    s.post.bloom.intensity = -1.0;
    assert!(e.dispatch(Command::SetRenderSettings(s)).is_err());
    assert_eq!(e.scene.render, before);
}
