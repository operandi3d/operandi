mod support;

use operandi_core::persistence::load_preset;
use operandi_core::preset::PresetCredit;
use serde_json::json;
use support::{Scratch, command, graph, success, texture_set};

#[test]
fn graph_material_set_and_credit_flags_select_and_override_values() {
    let scratch = Scratch::new();
    texture_set(scratch.path(), "Wall");
    texture_set(scratch.path(), "Other");
    let graph_path = scratch.path().join("graph.json");
    graph(&graph_path);
    // The default set is Wall; Other would auto-pick the Other material.
    // Each selector must therefore override a different automatic choice.
    for licence_flag in ["--licence", "--license"] {
        let out = scratch.path().join(format!("{licence_flag}.oppreset"));
        let stdout = success(
            command()
                .arg(scratch.path())
                .arg("--out")
                .arg(&out)
                .arg("--graph")
                .arg(&graph_path)
                .args([
                    "--material",
                    "Chosen Material",
                    "--set",
                    "oThEr",
                    licence_flag,
                    "CC-BY-4.0",
                    "--url",
                    "https://example.org/override",
                    "--credit",
                    "CLI Author",
                ])
                .output()
                .unwrap(),
        );
        assert!(stdout.contains("converted Other ->"), "{stdout}");
        assert!(
            stdout.contains("graph material Chosen Material"),
            "{stdout}"
        );
        let preset = load_preset(&out).unwrap();
        assert_eq!(preset.scene.images.len(), 1);
        assert!(preset.scene.images[0].path.ends_with("Wall_Color.png"));
        assert_eq!(
            preset.header.source,
            Some(PresetCredit {
                licence: "CC-BY-4.0".into(),
                url: "https://example.org/override".into(),
                credit: "CLI Author".into(),
            })
        );
    }
}

#[test]
fn graph_auto_selection_inherits_credit_and_refuses_unsupported_materials() {
    let scratch = Scratch::new();
    texture_set(scratch.path(), "Wall");
    let graph_path = scratch.path().join("graph.json");
    graph(&graph_path);
    let out = scratch.path().join("Wall.oppreset");
    let stdout = success(
        command()
            .arg(scratch.path())
            .arg("--out")
            .arg(&out)
            .arg("--graph")
            .arg(&graph_path)
            .output()
            .unwrap(),
    );
    assert!(
        stdout.contains("graph material Chosen Material"),
        "{stdout}"
    );
    assert_eq!(
        load_preset(&out).unwrap().header.source,
        Some(PresetCredit {
            licence: "CC0-1.0".into(),
            url: "https://example.org/source".into(),
            credit: "Graph Author".into(),
        })
    );
    let mut document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&graph_path).unwrap()).unwrap();
    document["materials"][1]["nodes"][1]["type"] = json!("ShaderNodeEmission");
    std::fs::write(&graph_path, document.to_string()).unwrap();
    let refused = scratch.path().join("refused/Wall.oppreset");
    let output = command()
        .arg(scratch.path())
        .arg("--out")
        .arg(&refused)
        .arg("--graph")
        .arg(&graph_path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Principled BSDF")
    );
    assert!(!refused.parent().unwrap().exists());
}
