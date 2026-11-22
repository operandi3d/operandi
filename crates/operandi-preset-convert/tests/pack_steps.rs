mod support;

use base64::Engine;
use operandi_core::persistence::{load_preset, save_preset};
use operandi_core::preset::pack::Manifest;
use std::path::Path;
use std::process::Command;
use support::{Scratch, command, png, preset, success};

fn manifest_command(folder: &Path) -> Command {
    let mut cmd = command();
    cmd.arg("pack-manifest").arg(folder).args([
        "--name",
        "Test Library",
        "--version",
        "1.2.3",
        "--checked",
        "2026-11-18",
    ]);
    cmd
}

#[test]
fn manifest_preserves_existing_thumbnail_and_skips_absent_thumbnail() {
    let scratch = Scratch::new();
    let existing = scratch.path().join("Starter/Existing/Existing.oppreset");
    // An existing sidecar must be kept even if the embedded data is invalid.
    preset(&existing, Some("invalid base64!".into()));
    png(&existing.with_extension("png"), 2, 1, [10, 20, 30]);
    let bytes = std::fs::read(existing.with_extension("png")).unwrap();
    let absent = scratch.path().join("Starter/Absent/Absent.oppreset");
    preset(&absent, None);
    success(manifest_command(scratch.path()).output().unwrap());
    assert_eq!(
        std::fs::read(existing.with_extension("png")).unwrap(),
        bytes
    );
    assert!(!absent.with_extension("png").exists());
    let manifest = Manifest::read(scratch.path()).unwrap();
    assert_eq!(
        (manifest.name.as_str(), manifest.version.as_str()),
        ("Test Library", "1.2.3")
    );
    assert_eq!(manifest.items.len(), 2);
    assert!(
        manifest
            .items
            .iter()
            .all(|item| item.checked == "2026-11-18")
    );
    let credits = std::fs::read_to_string(scratch.path().join("CREDITS.txt")).unwrap();
    assert!(credits.contains("Test Author"));
    assert!(credits.contains("https://example.org/original"));
}

#[test]
fn manifest_extracts_a_generated_png_and_reports_invalid_base64() {
    let scratch = Scratch::new();
    let image = scratch.path().join("thumbnail.png");
    png(&image, 2, 1, [30, 60, 90]);
    let bytes = std::fs::read(image).unwrap();
    let path = scratch.path().join("Starter/Wall/Wall.oppreset");
    preset(
        &path,
        Some(base64::engine::general_purpose::STANDARD.encode(&bytes)),
    );
    success(manifest_command(scratch.path()).output().unwrap());
    assert_eq!(std::fs::read(path.with_extension("png")).unwrap(), bytes);

    let bad = scratch.path().join("Starter/Bad/Bad.oppreset");
    preset(&bad, Some("invalid base64!".into()));
    let output = manifest_command(scratch.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("Starter/Bad/Bad.oppreset: thumbnail:"),
        "{stderr}"
    );
    assert!(!bad.with_extension("png").exists());
}

#[test]
fn licence_refusal_lists_every_problem_before_writing_files() {
    let scratch = Scratch::new();
    for name in ["One", "Two"] {
        let path = scratch
            .path()
            .join(format!("Starter/{name}/{name}.oppreset"));
        preset(&path, None);
        let mut p = load_preset(&path).unwrap();
        p.header.source.as_mut().unwrap().licence = "CC-BY-4.0".into();
        save_preset(&path, &p).unwrap();
    }
    let output = manifest_command(scratch.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    for name in ["One", "Two"] {
        assert!(
            stderr.contains(&format!("Starter/{name}/{name}.oppreset")),
            "{stderr}"
        );
    }
    assert!(!scratch.path().join("manifest.json").exists());
    assert!(!scratch.path().join("CREDITS.txt").exists());
}
