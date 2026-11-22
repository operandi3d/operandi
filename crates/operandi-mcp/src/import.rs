//! `scene_import`: add the meshes of an interchange file (OBJ, STL, glTF/GLB,
//! VideoScape GEO, PLY, FBX, Alembic, 3MF, USD) to the open scene as one
//! undo step.
//!
//! The file is read by the same reader as File > Import
//! (`operandi_core::interchange::import`, with the app's File I/O
//! preferences), but the result joins the open scene instead of replacing
//! it: the items go in through the Item/Assembly preset load
//! (`Command::LoadPreset`), which adds items, their material groups and
//! material tags in one step. Nothing here opens a dialog; every problem
//! is an error result and the scene is left as it was.
use operandi_core::command_exec::req_str;
use operandi_core::interchange;
use operandi_core::preset::{Preset, PresetHeader, PresetKind};
use operandi_core::{Command, Editor};
use serde_json::{Map, Value, json};
use std::path::Path;

type Result<T> = std::result::Result<T, String>;

/// The `tools/list` entry.
pub fn definition() -> Value {
    json!({
        "name": "scene_import",
        "description": format!(
            "Add the meshes of an interchange file ({}) to the open scene, as one undo step (\"Import <file>\"). Same reader and File I/O preferences as File > Import, but the scene is kept: the file's meshes are added as separate items with no hierarchy, each item's transform baked into its points (its materials join the shader tree when the scene has none of the same tag), and become the item selection. USD files (.usd, .usda, .usdc, .usdz) are the exception: their Xform hierarchy comes in as parented items with local transforms, quads and n-gons, UV sets, colour maps, creases and bound materials as Material Groups, with the USD import dialog's defaults (units and up axis from the file; instances arrive as copies; cameras and lights are not imported yet). Never opens a dialog: a missing, unreadable or unsupported file is an error and nothing changes. .opscene files open with scene_load; .lxo needs File > Open in the app. To write an interchange file, command_run scene.saveAs {{filename, format?}}.",
            interchange::IMPORT_EXTENSIONS.join(", ")
        ),
        "inputSchema": {"type": "object", "properties": {
            "path": {"type": "string", "description": "The file to import."}
        }, "required": ["path"], "additionalProperties": false}
    })
}

/// Run `scene_import` on the (staged) editor. Returns the added items
/// (id, name) and the reader's notes on what didn't come across.
pub fn scene_import(e: &mut Editor, args: &Map<String, Value>) -> Result<Value> {
    crate::commands::check_keys("scene_import", args, &["path"])?;
    let given = req_str(args, "path")?;
    let path = Path::new(given);
    let ext = path
        .extension()
        .and_then(|x| x.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "opscene" => return Err("scene_import adds interchange files; open .opscene files with scene_load".into()),
        "lxo" => return Err("scene_import can't read .lxo files (their import asks which groups to bring in); use File > Open in the app".into()),
        _ if !interchange::is_importable(path) => {
            return Err(format!(
                "scene_import reads {} files, not {given}",
                interchange::IMPORT_EXTENSIONS.join(", ")
            ));
        }
        _ => {}
    }
    if !path.is_file() {
        return Err(format!("No file at {given}"));
    }
    let imported = interchange::import(path)?;
    if imported.scene.items.iter().all(|i| i.is_texture_item()) {
        return Err(format!("{given} holds no meshes to import"));
    }
    let before: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    e.dispatch(Command::LoadPreset {
        preset: Box::new(Preset {
            header: PresetHeader {
                // Assembly keeps the file's hierarchy and item transforms.
                kind: PresetKind::Assembly,
                thumbnail: None,
                source: None,
            },
            scene: imported.scene,
        }),
        group: None,
    })?;
    let file = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if let Some(step) = e.undo.last_mut().map(std::sync::Arc::make_mut) {
        step.label = format!("Import {file}");
    }
    let added: Vec<Value> = e
        .scene
        .items
        .iter()
        .filter(|i| !before.contains(&i.id))
        .map(|i| json!({"id": i.id, "name": i.name, "parent": i.parent}))
        .collect();
    Ok(json!({"imported": added, "notes": imported.notes}))
}
