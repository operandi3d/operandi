//! Render outputs group D (2026-11-20): Illumination / Diffuse Shading
//! (Unshadowed), Light Path Expressions, Shaded AA Samples, and Render >
//! Add Per-Light Render Outputs.
use operandi_core::camera_light::{LightEdit, LightKind};
use operandi_core::render_outputs::{OutputSettings, writes_srgb};
use operandi_core::shader_tree::{NewLayer, NodeKind, OutputEffect, ShaderTreeEdit};
use operandi_core::{Command, Editor, persistence};

fn add(e: &mut Editor, effect: OutputEffect) -> u64 {
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::RenderOutput(effect),
        selected: None,
    }))
    .unwrap();
    e.scene.shader_tree.next_id - 1
}

fn version(e: &Editor) -> u64 {
    let bytes = persistence::encode(&e.scene).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["version"].as_u64().unwrap()
}

#[test]
fn the_new_outputs_save_reopen_and_take_version_55_only_when_used() {
    let mut e = Editor::default();
    let plain = version(&e);
    assert!(plain < 55, "a scene without them keeps its number");
    let mut ids = Vec::new();
    for effect in [
        OutputEffect::IlluminationUnshadowed,
        OutputEffect::DiffuseShadingUnshadowed,
        OutputEffect::LightPathExpressions,
        OutputEffect::ShadedAaSamples,
    ] {
        let id = add(&mut e, effect);
        assert_eq!(
            e.scene.shader_tree.get(id).unwrap().name,
            format!("{} Output", effect.label())
        );
        ids.push(id);
    }
    assert_eq!(version(&e), 55);
    // The expression through item.channel, as the panel edits it.
    let line = format!("item.channel lpe \"C<RD>L\" layer:{}", ids[2]);
    let command = operandi_core::command_line::command_for_line(&mut e, &line)
        .unwrap()
        .unwrap()
        .unwrap();
    e.dispatch(command).unwrap();
    assert_eq!(e.scene.shader_tree.output_settings(ids[2]).lpe, "C<RD>L");
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(back.shader_tree.output_settings(ids[2]).lpe, "C<RD>L");
    let effects: Vec<OutputEffect> = back
        .shader_tree
        .render_outputs()
        .iter()
        .map(|o| o.effect)
        .collect();
    assert!(effects.contains(&OutputEffect::ShadedAaSamples));
    // An expression alone needs v55 too; older files load unchanged.
    assert!(!OutputSettings::default().needs_v55());
    let old: OutputSettings = serde_json::from_str("{\"gamma\": 2.0}").unwrap();
    assert_eq!(old.lpe, "");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.shader_tree.output_settings(ids[2]).lpe, "");
}

#[test]
fn heat_map_is_a_fixed_data_output_and_unshadowed_is_beauty() {
    use OutputEffect as O;
    let s = OutputSettings::default();
    assert!(O::ShadedAaSamples.is_data() && O::ShadedAaSamples.fixed_colorspace());
    assert!(!O::ShadedAaSamples.processes());
    let srgb = operandi_core::render_outputs::OutputColorspace::Srgb;
    assert!(
        !writes_srgb(O::ShadedAaSamples, srgb, false),
        "colours as they are"
    );
    for o in [
        O::IlluminationUnshadowed,
        O::DiffuseShadingUnshadowed,
        O::LightPathExpressions,
    ] {
        assert!(writes_srgb(o, s.colorspace, false) && !writes_srgb(o, s.colorspace, true));
        assert!(o.processes());
    }
    assert!(O::IlluminationUnshadowed.needs_unshadowed_render());
    assert!(O::DiffuseShadingUnshadowed.needs_unshadowed_render());
    assert!(!O::IlluminationDirect.needs_unshadowed_render());
}

fn lpe_outputs(e: &Editor) -> Vec<(String, String)> {
    let tree = &e.scene.shader_tree;
    tree.nodes
        .iter()
        .filter(|n| {
            matches!(
                n.kind,
                NodeKind::RenderOutput(OutputEffect::LightPathExpressions)
            )
        })
        .map(|n| (n.name.clone(), tree.output_settings(n.id).lpe))
        .collect()
}

#[test]
fn add_per_light_outputs_is_one_step_and_skips_lights_that_have_one() {
    let mut e = Editor::default();
    // No light: an error, nothing changes.
    let lights: Vec<u64> = e
        .scene
        .items
        .iter()
        .filter(|i| i.light.is_some())
        .map(|i| i.id)
        .collect();
    for id in lights {
        e.dispatch(Command::SelectItems(vec![id])).unwrap();
        e.dispatch(Command::DeleteItem).unwrap();
    }
    let run = |e: &mut Editor| {
        operandi_core::command_line::command_for_line(e, "operandi.render.addPerLightOutputs")
            .expect("a known command")
    };
    assert!(run(&mut e).is_err());
    e.dispatch(Command::NewLight(LightKind::Point)).unwrap();
    let key = e.scene.selection.item.unwrap();
    e.dispatch(Command::EditLight {
        item: key,
        edit: LightEdit::LpeLabel("Key".into()),
    })
    .unwrap();
    e.dispatch(Command::NewLight(LightKind::Spot)).unwrap();
    let rim = e.scene.selection.item.unwrap();
    let rim_name = e.scene.item(rim).unwrap().name.clone();
    let steps = e.undo.len();
    assert!(matches!(run(&mut e), Ok(None)));
    assert_eq!(e.undo.len(), steps + 1, "one undo step");
    assert_eq!(
        e.undo.last().unwrap().label,
        operandi_core::per_light_outputs::LABEL
    );
    let outs = lpe_outputs(&e);
    assert_eq!(outs.len(), 2, "{outs:?}");
    for want in [
        ("LPE: C.*L'Key'".to_owned(), "C.*L'Key'".to_owned()),
        (
            format!("LPE: C.*L'{rim_name}'"),
            format!("C.*L'{rim_name}'"),
        ),
    ] {
        assert!(outs.contains(&want), "{outs:?}");
    }
    // The empty label took the light's name.
    assert_eq!(
        e.scene.item(rim).unwrap().light.as_ref().unwrap().lpe_label,
        rim_name
    );
    // Again: nothing new.
    assert!(run(&mut e).is_err());
    assert_eq!(lpe_outputs(&e).len(), 2);
    // Undo takes it all back, labels included; redo brings it again.
    e.dispatch(Command::Undo).unwrap();
    assert!(lpe_outputs(&e).is_empty());
    assert_eq!(
        e.scene.item(rim).unwrap().light.as_ref().unwrap().lpe_label,
        ""
    );
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(lpe_outputs(&e).len(), 2);
}
