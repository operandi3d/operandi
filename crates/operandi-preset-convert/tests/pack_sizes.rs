mod support;

use operandi_core::persistence::{load_preset, save_preset};
use operandi_core::preset::pack::CC0;
use support::{Scratch, command, png, preset, success};

#[test]
fn sizes_refresh_multiple_presets_and_keep_unreadable_maps_unchanged() {
    let scratch = Scratch::new();
    let mut cmd = command();
    cmd.arg("pack-sizes");
    for name in ["One", "Two"] {
        let path = scratch.path().join(format!("{name}.oppreset"));
        preset(&path, None);
        png(&scratch.path().join(format!("{name}.png")), 3, 2, [60; 3]);
        let mut p = load_preset(&path).unwrap();
        for (id, file) in [(1, format!("{name}.png")), (2, "Missing.png".into())] {
            p.scene.images.push(operandi_core::backdrop::SceneImage {
                id,
                name: file.clone(),
                path: file,
                width: 1024,
                height: 512,
            });
        }
        save_preset(&path, &p).unwrap();
        cmd.arg(path);
    }
    success(cmd.output().unwrap());
    for name in ["One", "Two"] {
        let path = scratch.path().join(format!("{name}.oppreset"));
        let p = load_preset(&path).unwrap();
        assert_eq!((p.scene.images[0].width, p.scene.images[0].height), (3, 2));
        assert_eq!(
            (p.scene.images[1].width, p.scene.images[1].height),
            (1024, 512)
        );
        assert_eq!(p.header.source.unwrap().licence, CC0);
        let document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(
            document["scene"]["images"][0]["path"],
            format!("{name}.png")
        );
        assert_eq!(document["scene"]["images"][1]["path"], "Missing.png");
    }
}

#[test]
fn sizes_reports_an_invalid_preset_without_overwriting_it() {
    let scratch = Scratch::new();
    let path = scratch.path().join("bad.oppreset");
    let bytes = b"not a preset";
    std::fs::write(&path, bytes).unwrap();
    let output = command().arg("pack-sizes").arg(&path).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!output.stderr.is_empty());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
