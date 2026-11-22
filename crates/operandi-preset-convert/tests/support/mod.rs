// Each integration test program uses a different subset of these helpers.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "operandi-preset-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_operandi-preset-convert"))
}

pub fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "status: {}; stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

pub fn png(path: &Path, width: u32, height: u32, rgb: [u8; 3]) {
    image::RgbImage::from_pixel(width, height, image::Rgb(rgb))
        .save(path)
        .unwrap();
}

pub fn texture_set(folder: &Path, name: &str) {
    std::fs::create_dir_all(folder).unwrap();
    png(
        &folder.join(format!("{name}_Color.png")),
        4,
        2,
        [210, 40, 20],
    );
    png(&folder.join(format!("{name}_Roughness.png")), 4, 2, [90; 3]);
}

pub fn graph(path: &Path) {
    let material = |name: &str, picture: &str| {
        serde_json::json!({
            "name": name, "output": "Output",
            "nodes": [
                {"name": "Output", "type": "ShaderNodeOutputMaterial"},
                {"name": "Surface", "type": "ShaderNodeBsdfPrincipled"},
                {"name": "Color", "type": "ShaderNodeTexImage", "image": {"path": picture}}
            ],
            "links": [
                {"from": ["Surface", "BSDF"], "to": ["Output", "Surface"]},
                {"from": ["Color", "Color"], "to": ["Surface", "Base Color"]}
            ]
        })
    };
    let graph = serde_json::json!({
        "format": "operandi-node-graph", "format_version": 1,
        "licence": {"name": "CC0-1.0", "url": "https://example.org/source", "credit": "Graph Author"},
        "materials": [material("Other", "Other_Color.png"), material("Chosen Material", "Wall_Color.png")]
    });
    std::fs::write(path, graph.to_string()).unwrap();
}

pub fn preset(path: &Path, thumbnail: Option<String>) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let (_, mut preset) = operandi_core::preset::basics::presets().remove(0);
    preset.header.source = Some(operandi_core::preset::PresetCredit {
        licence: operandi_core::preset::pack::CC0.into(),
        url: "https://example.org/original".into(),
        credit: "Test Author".into(),
    });
    preset.header.thumbnail = thumbnail;
    operandi_core::persistence::save_preset(path, &preset).unwrap();
}
