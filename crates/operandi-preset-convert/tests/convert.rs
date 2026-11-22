mod support;

use operandi_core::persistence::{load_preset, preset_thumbnail};
use operandi_core::preset::PresetKind;
use std::path::Path;
use support::{Scratch, command, graph, success, texture_set};

#[test]
fn default_conversion_writes_material_with_base_colour_thumbnail() {
    let scratch = Scratch::new();
    let maps = scratch.path().join("maps");
    texture_set(&maps, "Wall");
    let out = scratch.path().join("output/Wall.oppreset");
    success(
        command()
            .arg(&maps)
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap(),
    );
    let preset = load_preset(&out).unwrap();
    assert_eq!(preset.header.kind, PresetKind::Material);
    assert_eq!(preset.header.source, None);
    assert_eq!(preset.scene.images.len(), 2);
    for image in &preset.scene.images {
        assert_eq!((image.width, image.height), (4, 2));
        assert_eq!(Path::new(&image.path).parent(), Some(maps.as_path()));
    }
    let thumbnail = image::load_from_memory(&preset_thumbnail(&preset.header).unwrap())
        .unwrap()
        .to_rgb8();
    assert_eq!(thumbnail.dimensions(), (128, 64));
    assert_eq!(thumbnail.get_pixel(0, 0).0, [210, 40, 20]);
}

#[test]
fn copy_maps_and_no_thumbnail_make_a_portable_preset() {
    let scratch = Scratch::new();
    let maps = scratch.path().join("maps");
    texture_set(&maps, "Wall");
    let folder = scratch.path().join("output");
    let out = folder.join("Wall.oppreset");
    success(
        command()
            .arg(&maps)
            .arg("--out")
            .arg(&out)
            .args(["--copy-maps", "--no-thumbnail"])
            .output()
            .unwrap(),
    );
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let images = document["scene"]["images"].as_array().unwrap();
    assert_eq!(images.len(), 2);
    for image in images {
        let relative = Path::new(image["path"].as_str().unwrap());
        assert!(relative.is_relative());
        assert_eq!(relative.components().count(), 1);
        assert_eq!(
            std::fs::read(folder.join(relative)).unwrap(),
            std::fs::read(maps.join(relative)).unwrap()
        );
    }
    assert_eq!(load_preset(&out).unwrap().header.thumbnail, None);
    let moved = scratch.path().join("moved");
    std::fs::rename(folder, &moved).unwrap();
    let preset = load_preset(&moved.join("Wall.oppreset")).unwrap();
    for image in preset.scene.images {
        assert_eq!(Path::new(&image.path).parent(), Some(moved.as_path()));
        assert!(Path::new(&image.path).is_file());
    }
}

#[test]
fn no_mtlx_disables_the_matching_materialx_file() {
    let scratch = Scratch::new();
    texture_set(scratch.path(), "Wall");
    std::fs::write(scratch.path().join("Wall.mtlx"), r#"<materialx version="1.39">
      <image name="Color" type="color3"><input name="file" type="filename" value="Wall_Color.png" /></image>
      <open_pbr_surface name="Surface" type="surfaceshader"><input name="base_color" type="color3" nodename="Color" /></open_pbr_surface>
      <surfacematerial name="Wall" type="material"><input name="surfaceshader" type="surfaceshader" nodename="Surface" /></surfacematerial>
    </materialx>"#).unwrap();
    for (disable, count) in [(false, 1), (true, 2)] {
        let out = scratch.path().join(format!("{disable}.oppreset"));
        let mut cmd = command();
        cmd.arg(scratch.path()).arg("--out").arg(&out);
        if disable {
            cmd.arg("--no-mtlx");
        }
        let stdout = success(cmd.output().unwrap());
        assert_eq!(stdout.contains("MaterialX Wall.mtlx"), !disable);
        assert_eq!(load_preset(&out).unwrap().scene.images.len(), count);
    }
}

#[test]
fn empty_input_and_unknown_set_or_material_write_nothing() {
    let scratch = Scratch::new();
    let out = scratch.path().join("output/Wall.oppreset");
    let check = |options: &[&str], message: &str| {
        let output = command()
            .arg(scratch.path())
            .arg("--out")
            .arg(&out)
            .args(options)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(message), "{stderr}");
        assert!(!out.parent().unwrap().exists());
    };
    check(&[], "holds no texture set");
    texture_set(scratch.path(), "Wall");
    check(&["--set", "Missing"], "no texture set named Missing");
    let graph_path = scratch.path().join("graph.json");
    graph(&graph_path);
    check(
        &[
            "--graph",
            graph_path.to_str().unwrap(),
            "--material",
            "Missing",
        ],
        "the graph has no material Missing",
    );
}
