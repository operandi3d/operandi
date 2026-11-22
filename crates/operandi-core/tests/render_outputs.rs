//! Shader tree remaining, stage 6: Render Outputs and their settings.
use operandi_core::render_outputs::{OutputSettings, ToneMap};
use operandi_core::shader_tree::{NewLayer, NodeKind, OutputEffect, ShaderTreeEdit};
use operandi_core::{Command, Editor};

fn edit(e: &mut Editor, edit: ShaderTreeEdit) {
    e.dispatch(Command::ShaderTree(edit)).unwrap();
}

#[test]
fn a_new_scene_renders_final_color_and_alpha() {
    let e = Editor::default();
    let outs = e.scene.shader_tree.render_outputs();
    let effects: Vec<_> = outs.iter().map(|o| o.effect).collect();
    assert_eq!(effects, vec![OutputEffect::Alpha, OutputEffect::FinalColor]);
    assert!(outs.iter().all(|o| o.settings == OutputSettings::default()));
}

#[test]
fn outputs_land_on_top_and_disabled_ones_are_skipped() {
    let mut e = Editor::default();
    edit(
        &mut e,
        ShaderTreeEdit::Add {
            layer: NewLayer::RenderOutput(OutputEffect::Depth),
            selected: Some(4),
        },
    );
    let depth = e.scene.shader_tree.next_id - 1;
    let node = e.scene.shader_tree.get(depth).unwrap();
    assert_eq!(node.name, "Depth Output");
    assert_eq!(node.parent, None);
    assert_eq!(e.scene.shader_tree.nodes[0].id, depth, "at the top");
    edit(
        &mut e,
        ShaderTreeEdit::SetEnable {
            ids: vec![1, 2],
            enable: false,
        },
    );
    let outs = e.scene.shader_tree.render_outputs();
    assert_eq!(outs.len(), 1);
    assert_eq!(outs[0].effect, OutputEffect::Depth);
}

#[test]
fn output_settings_edit_undo_save_and_delete() {
    let mut e = Editor::default();
    let settings = OutputSettings {
        gamma: 2.2,
        tone_map: ToneMap::ReinhardLuminance,
        tone_amount: 0.5,
        ..Default::default()
    };
    edit(
        &mut e,
        ShaderTreeEdit::SetOutput {
            node: 2,
            settings: Box::new(settings.clone()),
            continuing: false,
        },
    );
    assert_eq!(e.scene.shader_tree.output_settings(2), settings);
    // Saved with native v22 and read back.
    // A pre-OpenPBR material, so the older version is what is tested.
    e.scene.shader_tree.legacy_openpbr();
    let bytes = operandi_core::persistence::encode(&e.scene).unwrap();
    assert!(
        String::from_utf8(bytes.clone())
            .unwrap()
            .contains("\"version\": 26")
    );
    let back = operandi_core::persistence::decode(&bytes).unwrap();
    assert_eq!(back.shader_tree.output_settings(2), settings);
    // One undo step.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        e.scene.shader_tree.output_settings(2),
        OutputSettings::default()
    );
    e.dispatch(Command::Redo).unwrap();
    // Back to the starting values: nothing stored.
    edit(
        &mut e,
        ShaderTreeEdit::SetOutput {
            node: 2,
            settings: Box::default(),
            continuing: false,
        },
    );
    assert!(e.scene.shader_tree.outputs.is_empty());
    // Settings on a layer that isn't an output are refused; deleting the
    // output drops its settings.
    assert!(
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetOutput {
            node: 4,
            settings: Box::new(settings.clone()),
            continuing: false,
        }))
        .is_err()
    );
    edit(
        &mut e,
        ShaderTreeEdit::SetOutput {
            node: 2,
            settings: Box::new(settings),
            continuing: false,
        },
    );
    edit(&mut e, ShaderTreeEdit::Remove(vec![2]));
    assert!(e.scene.shader_tree.outputs.is_empty());
    assert!(e.scene.shader_tree.validate().is_ok());
    assert!(
        !e.scene
            .shader_tree
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::RenderOutput(OutputEffect::FinalColor)))
    );
}

#[test]
fn out_of_range_settings_are_refused() {
    let mut e = Editor::default();
    let bad = OutputSettings {
        gamma: -1.,
        ..Default::default()
    };
    assert!(
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetOutput {
            node: 2,
            settings: Box::new(bad),
            continuing: false,
        }))
        .is_err()
    );
    assert!(e.scene.shader_tree.outputs.is_empty());
}

/// Cycles built-in outputs (2026-11-20): the new effects and settings save
/// as native v53 and read back; a scene without them keeps its number.
#[test]
fn built_in_outputs_and_their_settings_round_trip_as_v53() {
    use operandi_core::render_outputs::{CryptomatteType, OutputColorspace};
    let mut e = Editor::default();
    let plain = operandi_core::persistence::encode(&e.scene).unwrap();
    assert!(
        !String::from_utf8(plain)
            .unwrap()
            .contains("\"version\": 53")
    );
    let mut ids = Vec::new();
    for effect in [
        OutputEffect::IlluminationDirect,
        OutputEffect::IlluminationIndirect,
        OutputEffect::IlluminationTotal,
        OutputEffect::UvCoordinates,
        OutputEffect::SegmentId,
        OutputEffect::Cryptomatte,
    ] {
        edit(
            &mut e,
            ShaderTreeEdit::Add {
                layer: NewLayer::RenderOutput(effect),
                selected: None,
            },
        );
        let id = e.scene.shader_tree.next_id - 1;
        assert_eq!(
            e.scene.shader_tree.get(id).unwrap().name,
            format!("{} Output", effect.label())
        );
        ids.push(id);
    }
    let crypto = *ids.last().unwrap();
    let settings = OutputSettings {
        colorspace: OutputColorspace::Linear,
        cryptomatte_type: CryptomatteType::Material,
        cryptomatte_levels: 10,
        ..Default::default()
    };
    edit(
        &mut e,
        ShaderTreeEdit::SetOutput {
            node: crypto,
            settings: Box::new(settings.clone()),
            continuing: false,
        },
    );
    // item.channel reaches the settings files leave out.
    let line = format!("item.channel colorspace Srgb layer:{}", ids[0]);
    let command = operandi_core::command_line::command_for_line(&mut e, &line)
        .unwrap()
        .unwrap()
        .unwrap();
    e.dispatch(command).unwrap();
    assert_eq!(
        e.scene.shader_tree.output_settings(ids[0]).colorspace,
        OutputColorspace::Srgb
    );
    let bytes = operandi_core::persistence::encode(&e.scene).unwrap();
    assert!(
        String::from_utf8(bytes.clone())
            .unwrap()
            .contains("\"version\": 53")
    );
    let back = operandi_core::persistence::decode(&bytes).unwrap();
    assert_eq!(back.shader_tree.output_settings(crypto), settings);
    let effects: Vec<OutputEffect> = back
        .shader_tree
        .render_outputs()
        .iter()
        .map(|o| o.effect)
        .collect();
    assert!(effects.contains(&OutputEffect::Cryptomatte));
    assert!(effects.contains(&OutputEffect::IlluminationTotal));
    // Odd levels are refused.
    let bad = OutputSettings {
        cryptomatte_levels: 5,
        ..Default::default()
    };
    assert!(
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetOutput {
            node: crypto,
            settings: Box::new(bad),
            continuing: false,
        }))
        .is_err()
    );
}
