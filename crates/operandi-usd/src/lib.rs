//! USD export through OpenUSD: meshes, item transforms and hierarchy, UV
//! maps, normals, subdivision surfaces with creases (stage 1), and
//! shader-tree surfaces as UsdPreviewSurface materials with image textures,
//! plus .usdz packages (stage 2). USD import (docs/BEHAVIOR-USD-IMPORT.md,
//! stages U1 and U2): [`import`], built by `import.rs` from what the
//! bridge's reader describes (`stage.rs`).
//!
//! [`plan`] works out the prims in plain Rust; the `usd` feature links the
//! C++ bridge (bridge/, built by scripts/build-usd.sh) that writes them.
//! Without the feature, [`export`] says USD isn't included. The app
//! registers [`export`] with `interchange::set_usd_exporter`, so menus,
//! scene.saveAs and the MCP bridge all reach it.
use operandi_core::interchange::{self, ExportFormat, ExportReport};
use operandi_core::{Result, Scene};
use std::path::{Path, PathBuf};

pub mod anim_usd;
#[cfg(all(test, feature = "usd"))]
mod anim_usd_tests;
pub mod curves;
#[cfg(all(test, feature = "usd"))]
mod curves_tests;
pub mod cycles_mtlx;
#[cfg(all(test, feature = "usd"))]
mod export_gaps_tests;
#[cfg(feature = "usd")]
pub mod ffi;
#[cfg(all(test, feature = "usd"))]
mod foreign_tests;
pub mod import;
pub mod import_extras;
pub mod import_graphs;
pub mod import_lights;
pub mod import_materials;
pub mod import_mtlx_graph;
#[cfg(test)]
mod import_tests;
pub mod import_variants;
pub mod lights;
pub mod materials;
pub mod materialx_graph_out;
#[cfg(all(test, feature = "usd"))]
mod materialx_graph_tests;
pub mod materialx_out;
pub mod mesh_lights;
#[cfg(all(test, feature = "usd"))]
mod mesh_lights_tests;
#[cfg(all(test, feature = "usd"))]
mod node_graph_usd_tests;
#[cfg(all(test, feature = "usd"))]
mod patch_export_tests;
pub mod plan;
pub mod plan_extras;
#[cfg(test)]
mod plan_tests;
pub mod point_instancer;
#[cfg(all(test, feature = "usd"))]
mod point_instancer_tests;
pub mod render_usd;
#[cfg(all(test, feature = "usd"))]
mod render_usd_tests;
#[cfg(all(test, feature = "usd"))]
mod replica_usd_tests;
#[cfg(all(test, feature = "usd"))]
mod rig_usd_tests;
pub mod skel_usd;
#[cfg(all(test, feature = "usd"))]
mod skel_usd_tests;
pub mod stage;
#[cfg(all(test, feature = "usd"))]
mod tests;
#[cfg(test)]
mod tree_network_tests;
#[cfg(all(test, feature = "usd"))]
mod untested_rows_tests;

pub use plan::{Plan, plan};

/// True when this build can write USD files.
pub fn available() -> bool {
    #[cfg(feature = "usd")]
    {
        ffi::ready().is_ok()
    }
    #[cfg(not(feature = "usd"))]
    {
        false
    }
}

/// The OpenUSD version linked in, if any.
pub fn usd_version() -> Option<String> {
    #[cfg(feature = "usd")]
    {
        Some(ffi::usd_version())
    }
    #[cfg(not(feature = "usd"))]
    {
        None
    }
}

/// Read the USD file at `path` (.usd, .usda, .usdc or .usdz) into a new
/// scene, with the import report as notes. The app registers this with
/// `interchange::set_usd_importer`, so File > Open/Import, drops,
/// scene.importReference and the MCP bridge all reach it.
pub fn import(
    path: &Path,
    options: &interchange::UsdImportOptions,
) -> Result<interchange::Imported> {
    #[cfg(feature = "usd")]
    {
        let stage = ffi::read(path)?;
        import::build(&stage, path, options, &FileAssets)
    }
    #[cfg(not(feature = "usd"))]
    {
        let _ = (path, options);
        Err("This build of Operandi doesn't include USD support".into())
    }
}

/// Pictures on disk and inside .usdz packages, through OpenUSD.
#[cfg(feature = "usd")]
struct FileAssets;
#[cfg(feature = "usd")]
impl import::Assets for FileAssets {
    fn image_size(&self, path: &Path) -> Option<(u32, u32)> {
        image::image_dimensions(path).ok()
    }
    fn copy_packaged(&self, resolved: &str, destination: &Path) -> Result<()> {
        ffi::copy_asset(resolved, destination)
    }
}

/// Write `scene` to `path` as USD. `format` picks .usd (binary), .usda
/// (text) or .usdc; the file name's own extension wins when it names a USD
/// format. Writes go to a temporary in the same folder that replaces
/// `path` only when complete, so a failed export leaves no file.
///
/// `scene` is what gets written, mesh by mesh: callers pass the evaluated
/// scene (`Editor::evaluated(Purpose::Final)`), so MeshOps results are
/// exported rather than their Base Meshes, as every entry point does.
pub fn export(path: &Path, scene: &Scene, format: ExportFormat) -> Result<ExportReport> {
    if !format.is_usd() {
        return Err(format!("{} isn't a USD format", format.label()));
    }
    interchange::refuse_scene_file(path)?;
    let plan = plan(scene, &interchange::io_options())?;
    if plan.polygons == 0 && plan.curves == 0 {
        return Err("Nothing to export: no item has polygons or curves".into());
    }
    let mut notes = plan.notes.clone();
    write_file(path, &plan, format, scene, &mut notes)?;
    Ok(ExportReport {
        items: plan.items,
        polygons: plan.polygons,
        notes,
    })
}

/// The temporary `export` writes first: hidden, same folder, and keeping
/// the extension, which tells OpenUSD the format.
fn temporary_for(path: &Path, format: ExportFormat) -> Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or("Choose a file to export to")?
        .to_string_lossy();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .filter(|e| matches!(e.as_str(), "usd" | "usda" | "usdc" | "usdz"))
        .unwrap_or_else(|| format.extension().to_string());
    Ok(path.with_file_name(format!(".{name}.{}.tmp.{ext}", std::process::id())))
}

/// The images the plan's materials read, by `Scene::images` id.
fn used_images(plan: &Plan) -> std::collections::BTreeSet<u64> {
    let mtlx = plan
        .materials
        .iter()
        .filter_map(|m| m.mtlx.as_ref())
        .flat_map(|n| n.inputs())
        .filter_map(|i| match i.value {
            crate::materialx_out::NetValue::Image(id) => Some(id),
            _ => None,
        });
    plan.materials
        .iter()
        .flat_map(|m| m.inputs.iter())
        .filter_map(|i| i.texture.as_ref().map(|t| t.image))
        .chain(mtlx)
        .collect()
}

/// The pictures MaterialX graph layers read (not in `Scene::images`), by
/// their key.
fn used_files(plan: &Plan) -> std::collections::BTreeMap<u64, String> {
    plan.materials
        .iter()
        .filter_map(|m| m.mtlx.as_ref())
        .flat_map(|n| n.inputs())
        .filter_map(|i| match &i.value {
            crate::materialx_out::NetValue::File { key, path } => Some((*key, path.clone())),
            _ => None,
        })
        .collect()
}

/// Picture types a .usdz may hold (the USDZ spec: PNG and JPEG, plus
/// OpenEXR since USD 21.11).
const USDZ_IMAGE_TYPES: [&str; 4] = ["png", "jpg", "jpeg", "exr"];

/// Copies each used image into `folder` (made when needed) and returns its
/// asset path: `prefix` + file name, numbered when two images share a name.
/// Images whose file is missing get no path and a note.
fn copy_images(
    plan: &Plan,
    scene: &Scene,
    folder: &Path,
    prefix: &str,
    usdz: bool,
    notes: &mut Vec<String>,
) -> Result<std::collections::BTreeMap<u64, String>> {
    let mut files = std::collections::BTreeMap::new();
    let mut taken = std::collections::BTreeSet::new();
    let mut missing = Vec::new();
    let mut unpackable = Vec::new();
    // Scene pictures, then MaterialX graph layers' pictures by path.
    let mut sources: Vec<(u64, String, String)> = used_images(plan)
        .into_iter()
        .filter_map(|id| {
            let image = scene.images.iter().find(|i| i.id == id)?;
            Some((id, image.path.clone(), image.name.clone()))
        })
        .collect();
    sources.extend(used_files(plan).into_iter().map(|(key, path)| {
        let name = Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        (key, path, name)
    }));
    for (id, path, image_name) in sources {
        let source = Path::new(&path);
        if !source.is_file() {
            missing.push(image_name);
            continue;
        }
        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "image".into());
        let ext = source
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default();
        if usdz && !USDZ_IMAGE_TYPES.contains(&ext.to_ascii_lowercase().as_str()) {
            unpackable.push(image_name.clone());
        }
        let mut name = format!("{stem}.{ext}");
        let mut n = 2;
        while !taken.insert(name.to_ascii_lowercase()) {
            name = format!("{stem}_{n}.{ext}");
            n += 1;
        }
        std::fs::create_dir_all(folder)
            .map_err(|e| format!("Cannot make {}: {e}", folder.display()))?;
        let target = folder.join(&name);
        std::fs::copy(source, &target).map_err(|e| {
            format!(
                "Cannot copy {} to {}: {e}",
                source.display(),
                target.display()
            )
        })?;
        files.insert(id, format!("{prefix}{name}"));
    }
    if !missing.is_empty() {
        notes.push(format!(
            "Missing image file(s) left out: {}",
            missing.join(", ")
        ));
    }
    if !unpackable.is_empty() {
        notes.push(format!(
            "USDZ viewers may not read {} (USDZ takes PNG, JPEG and EXR)",
            unpackable.join(", ")
        ));
    }
    Ok(files)
}

/// Saves the baked environment as `environment.hdr` in `folder` and
/// returns its asset path.
#[cfg(feature = "usd")]
fn write_dome(plan: &Plan, folder: &Path, prefix: &str) -> Result<Option<String>> {
    let Some(picture) = plan.dome.as_ref().and_then(|(d, _)| d.dome.as_ref()) else {
        return Ok(None);
    };
    std::fs::create_dir_all(folder)
        .map_err(|e| format!("Cannot make {}: {e}", folder.display()))?;
    let file = folder.join("environment.hdr");
    std::fs::write(&file, lights::hdr_bytes(picture))
        .map_err(|e| format!("Cannot write {}: {e}", file.display()))?;
    Ok(Some(format!("{prefix}environment.hdr")))
}

#[cfg(feature = "usd")]
fn write_file(
    path: &Path,
    plan: &Plan,
    format: ExportFormat,
    scene: &Scene,
    notes: &mut Vec<String>,
) -> Result<()> {
    let temp = temporary_for(path, format)?;
    let _ = std::fs::remove_file(&temp);
    let usdz = format == ExportFormat::Usdz
        || path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("usdz"));
    if usdz {
        // The layer and its pictures go in a hidden folder beside the file,
        // which OpenUSD then packs; the folder never outlives the export.
        let build = path.with_file_name(format!(
            ".{}.{}.usdz-build",
            path.file_name().unwrap_or_default().to_string_lossy(),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&build);
        let result = (|| {
            std::fs::create_dir_all(&build)
                .map_err(|e| format!("Cannot make {}: {e}", build.display()))?;
            let files = copy_images(
                plan,
                scene,
                &build.join("textures"),
                "textures/",
                true,
                notes,
            )?;
            let dome = write_dome(plan, &build.join("textures"), "textures/")?;
            if dome.is_some() {
                notes.push(
                    "The environment picture is .hdr, which some USDZ viewers don't read".into(),
                );
            }
            let layer = build.join("scene.usdc");
            ffi::write(&layer, plan, false, &files, dome.as_deref())?;
            ffi::package_usdz(&layer, &temp)?;
            std::fs::rename(&temp, path)
                .map_err(|e| format!("Cannot replace {}: {e}", path.display()))
        })();
        let _ = std::fs::remove_dir_all(&build);
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        return result;
    }
    // Pictures go in "<name>_textures" beside the file, named relative to it.
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "scene".into());
    let folder = path.with_file_name(format!("{stem}_textures"));
    let files = copy_images(
        plan,
        scene,
        &folder,
        &format!("./{stem}_textures/"),
        false,
        notes,
    )?;
    let dome = write_dome(plan, &folder, &format!("./{stem}_textures/"))?;
    let text = format == ExportFormat::Usda;
    let result = ffi::write(&temp, plan, text, &files, dome.as_deref()).and_then(|()| {
        std::fs::rename(&temp, path).map_err(|e| format!("Cannot replace {}: {e}", path.display()))
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[cfg(not(feature = "usd"))]
fn write_file(
    path: &Path,
    _plan: &Plan,
    format: ExportFormat,
    _scene: &Scene,
    _notes: &mut Vec<String>,
) -> Result<()> {
    let _ = temporary_for(path, format)?;
    let _ = copy_images;
    Err("This build of Operandi doesn't include USD support".into())
}
