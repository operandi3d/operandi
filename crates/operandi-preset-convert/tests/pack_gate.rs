//! The library pack build's licence gate, through the binary
//! `scripts/build-library-pack.sh` runs.
use base64::Engine;
use operandi_core::preset::pack::{CC0, Manifest};
use operandi_core::preset::{PresetCredit, basics};
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("operandi-pack-gate-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn preset(pack: &Path, name: &str, licence: &str, thumbnail: Option<&[u8]>) {
    let dir = pack.join("Starter").join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let (_, mut p) = basics::presets().remove(0);
    p.header.source = Some(PresetCredit {
        licence: licence.into(),
        url: format!("https://example.org/{name}"),
        credit: "Example".into(),
    });
    p.header.thumbnail = thumbnail.map(|b| base64::engine::general_purpose::STANDARD.encode(b));
    operandi_core::persistence::save_preset(&dir.join(format!("{name}.oppreset")), &p).unwrap();
}

fn manifest_step(pack: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_operandi-preset-convert"))
        .arg("pack-manifest")
        .arg(pack)
        .args(["--name", "Operandi Library", "--version", "2026.11.0"])
        .args(["--checked", "2026-11-11"])
        .output()
        .unwrap()
}

#[test]
fn a_cc0_pack_gets_its_manifest_credits_and_thumbnails() {
    let pack = scratch("ok");
    preset(&pack, "Bricks", CC0, Some(b"\x89PNG fake"));
    let out = manifest_step(&pack);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let m = Manifest::read(&pack).unwrap();
    assert_eq!(m.version, "2026.11.0");
    assert_eq!(m.items[0].path, "Starter/Bricks/Bricks.oppreset");
    assert_eq!(m.items[0].source_url, "https://example.org/Bricks");
    assert!(pack.join("CREDITS.txt").is_file());
    assert_eq!(
        std::fs::read(pack.join("Starter/Bricks/Bricks.png")).unwrap(),
        b"\x89PNG fake"
    );
    let _ = std::fs::remove_dir_all(pack);
}

#[test]
fn a_non_cc0_starter_item_fails_the_build() {
    let pack = scratch("refused");
    preset(&pack, "Bricks", CC0, None);
    preset(&pack, "Moss", "CC-BY-4.0", None);
    let out = manifest_step(&pack);
    assert_eq!(out.status.code(), Some(3));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Starter/Moss/Moss.oppreset"), "{err}");
    assert!(!pack.join("manifest.json").exists(), "nothing written");
    let _ = std::fs::remove_dir_all(pack);
}

#[test]
fn pack_sizes_reads_scaled_maps_and_keeps_paths_relative() {
    let dir = scratch("sizes");
    let map = dir.join("Wall_Color.png");
    image::RgbImage::new(4, 2).save(&map).unwrap();
    let (_, mut p) = basics::presets().remove(0);
    p.scene.images.push(operandi_core::backdrop::SceneImage {
        id: 1,
        name: "Wall_Color".into(),
        path: "Wall_Color.png".into(),
        width: 1024,
        height: 1024,
    });
    let file = dir.join("Wall.oppreset");
    operandi_core::persistence::save_preset(&file, &p).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_operandi-preset-convert"))
        .arg("pack-sizes")
        .arg(&file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"path\": \"Wall_Color.png\""), "{text}");
    let loaded = operandi_core::persistence::load_preset(&file).unwrap();
    assert_eq!(
        (loaded.scene.images[0].width, loaded.scene.images[0].height),
        (4, 2)
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn environments_and_matcaps_go_through_the_gate_too() {
    let pack = scratch("pictures");
    preset(&pack, "Bricks", CC0, None);
    let env = pack.join("Environments/Studio");
    std::fs::create_dir_all(&env).unwrap();
    std::fs::write(env.join("Studio Small 08.hdr"), b"#?RADIANCE").unwrap();
    std::fs::write(env.join("Unlisted.exr"), b"exr").unwrap();
    std::fs::write(
        pack.join("Environments/SOURCES.tsv"),
        "Studio/Studio Small 08.hdr\tCC0-1.0\thttps://polyhaven.com/a/studio_small_08\tSergej Majboroda\n",
    )
    .unwrap();
    let out = manifest_step(&pack);
    assert_eq!(out.status.code(), Some(3));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Environments/Studio/Unlisted.exr"), "{err}");

    std::fs::remove_file(env.join("Unlisted.exr")).unwrap();
    let out = manifest_step(&pack);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let m = Manifest::read(&pack).unwrap();
    let item = m
        .items
        .iter()
        .find(|i| i.path == "Environments/Studio/Studio Small 08.hdr")
        .unwrap();
    assert_eq!(item.kind, "environmentImage");
    assert_eq!(item.licence, CC0);
    let credits = std::fs::read_to_string(pack.join("CREDITS.txt")).unwrap();
    assert!(
        credits.contains("https://polyhaven.com/a/studio_small_08"),
        "{credits}"
    );
    let _ = std::fs::remove_dir_all(pack);
}

/// 2026.11.5: mesh presets go through the gate from their headers and keep
/// their thumbnail inside (no picture beside them in the category folder).
#[test]
fn meshes_go_through_the_gate_without_a_picture_beside_them() {
    let pack = scratch("meshes");
    let shape = &operandi_core::preset::starter_meshes::ALL[0];
    let path = pack.join("Meshes").join(shape.relative_path());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut p = shape.preset().unwrap();
    p.header.thumbnail = Some(base64::engine::general_purpose::STANDARD.encode(b"\x89PNG fake"));
    operandi_core::persistence::save_preset(&path, &p).unwrap();
    let out = manifest_step(&pack);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let m = Manifest::read(&pack).unwrap();
    assert_eq!(m.items.len(), 1);
    assert_eq!(m.items[0].kind, "item");
    assert!(!path.with_extension("png").exists());
    let _ = std::fs::remove_dir_all(pack);
}
