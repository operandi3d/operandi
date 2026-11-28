//! Interchange formats: OBJ, STL, glTF/GLB, PLY, Alembic, 3MF, FBX and
//! VideoScape GEO import and export, plus SVG, STEP and IGES export. These are general-purpose formats with public
//! descriptions, not Modo behavior; nothing here claims Modo parity.
//!
//! Import always builds a complete, validated scene before anything is
//! published, so a refused file leaves the current document untouched.
//! Export bakes each visible item's world transform into its points (none of
//! the flat formats carry item transforms) and writes through a temporary in
//! the destination folder, so a failed export never leaves a partial file.
use crate::{Face, Item, Mesh, Result, Scene};
use glam::{DMat4, DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

mod abc;
mod cad;
#[cfg(test)]
mod cad_tests;
mod fbx;
mod fbx_write;
mod geo;
mod gltf_io;
mod iges;
mod obj;
mod ply;
mod step;
mod stl;
mod svg;
#[cfg(test)]
mod tests;
mod threemf;
pub(crate) mod zip;

/// Largest interchange file read. Text OBJ files of real models can be large;
/// the mesh validator's point/face budget still applies after parsing.
/// A text OBJ of 5 million polygons is about 500 MB, so this was raised
/// from 512 MiB; it only stops a file far bigger than any real model from
/// being read into memory whole.
/// Raised from 4 GiB with scene Open (hidden-limits B7): a 50M polygon OBJ
/// is 3-5 GB. What fits in memory is read.
const MAX_IMPORT_BYTES: u64 = 256 << 30;
/// UV map that imported texture coordinates land in; the same name the
/// primitives use, so tools that look for "Texture" find imported UVs.
const UV_MAP: &str = crate::PRIMITIVE_UV_MAP;

/// A file format the user picks for export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Obj,
    StlBinary,
    StlAscii,
    Gltf,
    Glb,
    Geo,
    Svg,
    /// Modo 17.1v1 scene (specifications/lxo-export-2026-11-03).
    Lxo,
    PlyBinary,
    PlyAscii,
    Alembic,
    /// Universal Scene Description, binary (.usd). Written by the
    /// operandi-usd crate through the exporter the app registers
    /// ([`set_usd_exporter`]); core has no OpenUSD of its own.
    Usd,
    /// USD as text (.usda).
    Usda,
    /// USD binary with the crate extension (.usdc).
    Usdc,
    /// USD packed with its pictures into one zip archive (.usdz).
    Usdz,
    /// Binary FBX 7.4 (`fbx_write.rs`).
    Fbx,
    /// 3D Manufacturing Format (`threemf.rs`).
    ThreeMf,
    /// STEP AP214: exact Tab patches and flat faces as solids or
    /// surfaces (`cad.rs`, `step.rs`).
    Step,
    /// IGES 5.3: the same faces as untrimmed B-spline surfaces (`iges.rs`).
    Iges,
}
impl ExportFormat {
    pub const ALL: [ExportFormat; 19] = [
        Self::Obj,
        Self::StlBinary,
        Self::StlAscii,
        Self::Gltf,
        Self::Glb,
        Self::Geo,
        Self::Svg,
        Self::Lxo,
        Self::PlyBinary,
        Self::PlyAscii,
        Self::Alembic,
        Self::Usd,
        Self::Usda,
        Self::Usdc,
        Self::Usdz,
        Self::Fbx,
        Self::ThreeMf,
        Self::Step,
        Self::Iges,
    ];
    /// Menu and panel title.
    pub fn label(self) -> &'static str {
        match self {
            Self::Obj => "Wavefront OBJ",
            Self::StlBinary => "STL (Binary)",
            Self::StlAscii => "STL (ASCII)",
            Self::Gltf => "glTF (.gltf)",
            Self::Glb => "glTF Binary (.glb)",
            Self::Geo => "VideoScape GEO",
            Self::Svg => "SVG Drawing (Front View)",
            Self::Lxo => "LXO Scene",
            Self::PlyBinary => "PLY (Binary)",
            Self::PlyAscii => "PLY (ASCII)",
            Self::Alembic => "Alembic (.abc)",
            Self::Usd => "USD (.usd)",
            Self::Usda => "USD ASCII (.usda)",
            Self::Usdc => "USD Binary (.usdc)",
            Self::Usdz => "USDZ (.usdz)",
            Self::Fbx => "FBX (.fbx)",
            Self::ThreeMf => "3D Manufacturing (.3mf)",
            Self::Step => "STEP (.step)",
            Self::Iges => "IGES (.iges)",
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Self::Obj => "obj",
            Self::StlBinary | Self::StlAscii => "stl",
            Self::Gltf => "gltf",
            Self::Glb => "glb",
            Self::Geo => "geo",
            Self::Svg => "svg",
            Self::Lxo => "lxo",
            Self::PlyBinary | Self::PlyAscii => "ply",
            Self::Alembic => "abc",
            Self::Usd => "usd",
            Self::Usda => "usda",
            Self::Usdc => "usdc",
            Self::Usdz => "usdz",
            Self::Fbx => "fbx",
            Self::ThreeMf => "3mf",
            Self::Step => "step",
            Self::Iges => "iges",
        }
    }
    /// One of the USD formats, which keep the scene's structure.
    pub fn is_usd(self) -> bool {
        matches!(self, Self::Usd | Self::Usda | Self::Usdc | Self::Usdz)
    }
}

/// Writes a scene to a USD file: `operandi_usd::export`, registered by the
/// app so every entry point (menus, scene.saveAs, MCP) reaches it.
pub type UsdExporter = fn(&Path, &Scene, ExportFormat) -> Result<ExportReport>;
static USD_EXPORTER: std::sync::OnceLock<UsdExporter> = std::sync::OnceLock::new();
/// Register the USD writer (once; later calls are ignored).
pub fn set_usd_exporter(exporter: UsdExporter) {
    let _ = USD_EXPORTER.set(exporter);
}
/// Whether a USD writer is registered (the app registers one; the headless
/// MCP sidecar has none).
pub fn has_usd_exporter() -> bool {
    USD_EXPORTER.get().is_some()
}

/// scene.saveAs `format:` values, in `command_registry::EXPORT_FORMATS`
/// order.
pub const SCRIPT_EXPORT_FORMATS: [ExportFormat; 19] = [
    ExportFormat::Obj,
    ExportFormat::StlBinary,
    ExportFormat::StlAscii,
    ExportFormat::Gltf,
    ExportFormat::Glb,
    ExportFormat::Geo,
    ExportFormat::Svg,
    ExportFormat::Lxo,
    ExportFormat::PlyBinary,
    ExportFormat::PlyAscii,
    ExportFormat::Alembic,
    ExportFormat::Usd,
    ExportFormat::Usda,
    ExportFormat::Usdc,
    ExportFormat::Usdz,
    ExportFormat::Fbx,
    ExportFormat::ThreeMf,
    ExportFormat::Step,
    ExportFormat::Iges,
];

/// The export format a file name's extension names: the first in
/// [`SCRIPT_EXPORT_FORMATS`] order (so .stl and .ply mean their binary
/// forms).
pub fn export_format_of_path(path: &str) -> Option<ExportFormat> {
    let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    // The short CAD extensions name the same formats.
    let ext = match ext.as_str() {
        "stp" => "step".to_owned(),
        "igs" => "iges".to_owned(),
        _ => ext,
    };
    SCRIPT_EXPORT_FORMATS
        .into_iter()
        .find(|f| f.extension() == ext)
}

/// A scene.saveAs `format:` value (case ignored), by
/// `command_registry::EXPORT_FORMATS`.
pub fn script_export_format(name: &str) -> Option<ExportFormat> {
    let crate::command_registry::ArgType::Choice(names) = crate::command_registry::EXPORT_FORMATS
    else {
        return None;
    };
    names
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name))
        .map(|i| SCRIPT_EXPORT_FORMATS[i])
}

/// Extensions accepted by `import`, lower case. The USD ones need the
/// app's USD reader (`set_usd_importer`).
pub const IMPORT_EXTENSIONS: [&str; 13] = [
    "obj", "stl", "gltf", "glb", "geo", "ply", "fbx", "abc", "3mf", "usd", "usda", "usdc", "usdz",
];

/// The USD file extensions, lower case.
pub const USD_EXTENSIONS: [&str; 4] = ["usd", "usda", "usdc", "usdz"];

/// True when `path` names a USD file (by extension).
pub fn is_usd(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| USD_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Which shading network a USD import reads for each material
/// (docs/BEHAVIOR-USD-IMPORT.md, U2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UsdMaterials {
    /// The MaterialX network when there is one, else UsdPreviewSurface.
    #[default]
    MaterialX,
    /// UsdPreviewSurface when there is one, else MaterialX.
    PreviewSurface,
    /// No Material Groups; polygons still get their material tags.
    None,
}

/// The USD import options dialog (docs/BEHAVIOR-USD-IMPORT.md). Default:
/// everything on, units and up axis from the file.
#[derive(Clone, Debug, PartialEq)]
pub struct UsdImportOptions {
    /// 'Y' or 'Z' to override the file's upAxis.
    pub up_axis: Option<char>,
    /// Overrides the file's metersPerUnit.
    pub meters_per_unit: Option<f64>,
    pub materials: UsdMaterials,
    /// Cameras and lights arrive in stage U3; until then they are
    /// reported as left out whatever these say.
    pub cameras: bool,
    pub lights: bool,
    /// Read a node graph sidecar (stage U4): `graphs_file`, else
    /// `<file name>_graphs.json` beside the USD file when there is one.
    pub graphs_sidecar: bool,
    pub graphs_file: Option<std::path::PathBuf>,
    /// Instances as instance items. Operandi has none yet, so instances
    /// always arrive as copies (sharing their mesh until edited).
    pub instances: bool,
    /// Where pictures packed in a .usdz are unpacked: the scene's project
    /// folder when it has one, else `<file>_textures` beside the file.
    pub texture_folder: Option<std::path::PathBuf>,
}
impl Default for UsdImportOptions {
    fn default() -> Self {
        Self {
            up_axis: None,
            meters_per_unit: None,
            materials: UsdMaterials::MaterialX,
            cameras: true,
            lights: true,
            graphs_sidecar: true,
            graphs_file: None,
            instances: true,
            texture_folder: None,
        }
    }
}

/// Reads a USD file: `operandi_usd::import`, registered by the app like
/// the exporter.
pub type UsdImporter = fn(&Path, &UsdImportOptions) -> Result<Imported>;
static USD_IMPORTER: std::sync::OnceLock<UsdImporter> = std::sync::OnceLock::new();
/// Register the USD reader (once; later calls are ignored).
pub fn set_usd_importer(importer: UsdImporter) {
    let _ = USD_IMPORTER.set(importer);
}

/// True when `path` has an extension `import` reads: a built-in one or
/// one a kit that's on adds (`kit_import`, KIT-1).
pub fn is_importable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|e| {
            IMPORT_EXTENSIONS.contains(&e.as_str())
                || crate::kit_import::kit_importer_for(&e).is_some()
        })
}

/// Every extension `import` reads now: the built-in ones, then those of
/// kits that are on, each with the Import dialog's label for it.
pub fn import_extensions() -> Vec<(String, String)> {
    let mut list: Vec<(String, String)> = IMPORT_EXTENSIONS
        .iter()
        .map(|e| (e.to_string(), e.to_ascii_uppercase()))
        .collect();
    for importer in crate::kit_import::kit_importers() {
        for ext in &importer.extensions {
            list.push((ext.clone(), importer.label.clone()));
        }
    }
    list
}

/// A scene read from an interchange file, with what did not come across.
#[derive(Debug)]
pub struct Imported {
    pub scene: Scene,
    pub notes: Vec<String>,
}

/// Read an interchange file into a new scene. Nothing is published on error.
/// USD files use the default import options.
pub fn import(path: &Path) -> Result<Imported> {
    import_with(path, &UsdImportOptions::default())
}

/// [`import`], with `usd` as a USD file's import options.
pub fn import_with(path: &Path, usd: &UsdImportOptions) -> Result<Imported> {
    if is_usd(path) {
        let importer = USD_IMPORTER
            .get()
            .ok_or("This build of Operandi doesn't include USD import")?;
        return importer(path, usd);
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    // A file type a kit adds goes to the kit's helper program. Kits can't
    // take over a built-in type (kit_manifest), so this never shadows one.
    if !IMPORT_EXTENSIONS.contains(&ext.as_str())
        && let Some(importer) = crate::kit_import::kit_importer_for(&ext)
    {
        return crate::kit_import::run(&importer, path);
    }
    let bytes = operandi_files::read_whole(path, MAX_IMPORT_BYTES)?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Mesh".into());
    let mut notes = Vec::new();
    let options = io_options();
    let turn = options.file_to_scene();
    let scaled = |mut meshes: Vec<RawMesh>, unit: FileUnit| {
        if unit != FileUnit::Meters || options.up_axis != 1 {
            for m in &mut meshes {
                for p in &mut m.points {
                    *p = turn * (*p * unit.metres());
                }
            }
        }
        meshes
    };
    let meshes = match ext.as_str() {
        "obj" => scaled(
            obj::read(&bytes, &stem, options.obj_groups_as_meshes, &mut notes)?,
            options.obj_import_units,
        ),
        "stl" => scaled(
            stl::read(&bytes, &stem, &mut notes)?,
            options.stl_import_units,
        ),
        "gltf" | "glb" => scaled(
            gltf_io::read(&bytes, path.parent(), &stem, &mut notes)?,
            FileUnit::Meters,
        ),
        "geo" => geo::read(&bytes, &stem, &mut notes)?,
        // PLY carries no unit; read as metres like glTF.
        "ply" => scaled(ply::read(&bytes, &stem, &mut notes)?, FileUnit::Meters),
        // ufbx has already converted the file to Y-up metres.
        "fbx" => scaled(fbx::read(&bytes, &stem, &mut notes)?, FileUnit::Meters),
        // The Alembic library reads the file itself; no unit, metres.
        "abc" => scaled(abc::read(path, &stem)?, FileUnit::Meters),
        // The reader has applied the file's unit and turned it Y-up.
        "3mf" => {
            let mut materials = BTreeMap::new();
            let mut meshes = threemf::read(&bytes, &stem, &mut notes, &mut materials)?;
            if let Some(first) = meshes.first_mut() {
                first.materials = materials;
            }
            scaled(meshes, FileUnit::Meters)
        }
        _ => return Err(format!("Operandi can't import .{ext} files")),
    };
    let scene = scene_from(meshes, &mut notes)?;
    Ok(Imported { scene, notes })
}

/// A length unit a file's numbers are read or written in (OBJ and STL
/// carry none of their own).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FileUnit {
    #[default]
    Meters,
    Centimeters,
    Millimeters,
    Inches,
}
impl FileUnit {
    pub const ALL: [Self; 4] = [
        Self::Meters,
        Self::Centimeters,
        Self::Millimeters,
        Self::Inches,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Meters => "Meters",
            Self::Centimeters => "Centimeters",
            Self::Millimeters => "Millimeters",
            Self::Inches => "Inches",
        }
    }
    /// Metres per file unit.
    pub fn metres(self) -> f64 {
        match self {
            Self::Meters => 1.,
            Self::Centimeters => 0.01,
            Self::Millimeters => 0.001,
            Self::Inches => 0.0254,
        }
    }
}

/// glTF Export Type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GltfExportType {
    #[default]
    All,
    /// The selected items only.
    Selected,
    /// The selected items and everything parented under them.
    SelectedWithHierarchy,
}
impl GltfExportType {
    pub const ALL: [Self; 3] = [Self::All, Self::Selected, Self::SelectedWithHierarchy];
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "Export All",
            Self::Selected => "Export Selected",
            Self::SelectedWithHierarchy => "Export Selected with Hierarchy",
        }
    }
}

/// File I/O preferences the readers and writers follow (Preferences >
/// File I/O). Defaults are the spec's where it gives them.
#[derive(Clone, Debug, PartialEq)]
pub struct IoOptions {
    pub obj_import_units: FileUnit,
    /// Split `g` groups into meshes even when the file has `o` objects.
    pub obj_groups_as_meshes: bool,
    pub obj_export_units: FileUnit,
    /// Write `g` instead of `o` per mesh.
    pub obj_save_groups: bool,
    /// Write a .mtl beside the .obj.
    pub obj_save_mtl: bool,
    pub stl_import_units: FileUnit,
    pub stl_export_units: FileUnit,
    pub gltf_export: GltfExportType,
    pub gltf_geometry: bool,
    pub gltf_materials: bool,
    pub gltf_tangents: bool,
    /// Unsigned Short indices where a primitive has under 65,536 vertices.
    pub gltf_short_indices: bool,
    /// The scene's Up Axis (0 X, 1 Y, 2 Z). Interchange files are read and
    /// written Y-up (glTF's rule, and the common one for OBJ and STL), so
    /// another up axis turns geometry on the way in and out.
    pub up_axis: u8,
    /// TL-83: USD export of animation.
    pub usd_animation: UsdAnimation,
}

/// TL-83: what a USD export writes of the scene's animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UsdAnimation {
    /// Time samples once a frame across the scene range (TL-80..82).
    #[default]
    On,
    /// No time samples: the scene as it stands now.
    Off,
    /// No time samples, and the stage's range is the current frame.
    CurrentFrame,
}

impl UsdAnimation {
    pub fn parse(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "on" | "true" => Some(Self::On),
            "off" | "false" => Some(Self::Off),
            "current" | "currentframe" => Some(Self::CurrentFrame),
            _ => None,
        }
    }
}
impl IoOptions {
    /// The turn from a Y-up file to the scene.
    pub fn file_to_scene(&self) -> glam::DMat3 {
        match self.up_axis {
            2 => glam::DMat3::from_cols(DVec3::X, DVec3::Z, -DVec3::Y),
            0 => glam::DMat3::from_cols(-DVec3::Y, DVec3::X, DVec3::Z),
            _ => glam::DMat3::IDENTITY,
        }
    }
}
impl Default for IoOptions {
    fn default() -> Self {
        Self {
            obj_import_units: FileUnit::Meters,
            obj_groups_as_meshes: false,
            obj_export_units: FileUnit::Meters,
            obj_save_groups: false,
            obj_save_mtl: true,
            stl_import_units: FileUnit::Meters,
            stl_export_units: FileUnit::Meters,
            gltf_export: GltfExportType::All,
            gltf_geometry: true,
            gltf_materials: true,
            gltf_tangents: false,
            gltf_short_indices: false,
            up_axis: 1,
            usd_animation: UsdAnimation::On,
        }
    }
}
thread_local! {
    static IO_OPTIONS: std::cell::RefCell<IoOptions> = std::cell::RefCell::default();
}
/// Set the File I/O options this thread's imports and exports follow.
pub fn set_io_options(options: IoOptions) {
    IO_OPTIONS.with(|o| *o.borrow_mut() = options);
}
/// The File I/O options this thread's imports and exports follow.
pub fn io_options() -> IoOptions {
    IO_OPTIONS.with(|o| o.borrow().clone())
}

/// What an export wrote, for the status line.
/// Author and copyright written into exports that carry them (glTF).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileInfo {
    pub author: String,
    pub copyright: String,
}
thread_local! {
    static FILE_INFO: std::cell::RefCell<FileInfo> = std::cell::RefCell::default();
}
/// Set the author and copyright this thread's exports write.
pub fn set_file_info(info: FileInfo) {
    FILE_INFO.with(|f| *f.borrow_mut() = info);
}
/// The author and copyright this thread's exports write.
pub fn file_info() -> FileInfo {
    FILE_INFO.with(|f| f.borrow().clone())
}

#[derive(Debug, Default, PartialEq)]
pub struct ExportReport {
    pub items: usize,
    pub polygons: usize,
    pub notes: Vec<String>,
}

/// Write the scene's visible items to `path` in `format`.
pub fn export(path: &Path, scene: &Scene, format: ExportFormat) -> Result<ExportReport> {
    if format.is_usd() {
        // USD keeps the hierarchy and each item's own eye, so it gets the
        // scene as it is.
        let exporter = USD_EXPORTER
            .get()
            .ok_or("This build of Operandi doesn't include USD support")?;
        return exporter(path, scene, format);
    }
    // A child hidden by its parent's eye (GV-01) exports as hidden.
    let resolved = scene.resolved_eyes();
    let scene = &resolved;
    if format == ExportFormat::Lxo {
        return export_lxo(path, scene);
    }
    if matches!(format, ExportFormat::Step | ExportFormat::Iges) {
        return cad::export(path, scene, format);
    }
    let options = io_options();
    let mut meshes = if matches!(format, ExportFormat::Gltf | ExportFormat::Glb) {
        world_meshes_of(scene, options.gltf_export)?
    } else {
        world_meshes(scene)?
    };
    // OBJ and STL numbers in the chosen export unit.
    let unit = match format {
        ExportFormat::Obj => options.obj_export_units,
        ExportFormat::StlBinary | ExportFormat::StlAscii => options.stl_export_units,
        _ => FileUnit::Meters,
    };
    // Up Axis: files are written Y-up (SVG and GEO too, as before only
    // for a Y-up scene).
    let turn = options.file_to_scene().transpose();
    let turns = options.up_axis != 1 && format != ExportFormat::Svg;
    if unit != FileUnit::Meters || turns {
        for m in &mut meshes {
            for p in &mut m.mesh.vertices {
                if turns {
                    *p = turn * *p;
                }
                *p /= unit.metres();
            }
        }
    }
    if meshes.iter().all(|m| m.mesh.faces.is_empty()) {
        return Err("Nothing to export: no visible item has polygons".into());
    }
    let mut report = ExportReport {
        items: meshes.iter().filter(|m| !m.mesh.faces.is_empty()).count(),
        polygons: meshes.iter().map(|m| m.mesh.faces.len()).sum(),
        notes: Vec::new(),
    };
    let hidden = scene
        .items
        .iter()
        .filter(|i| !i.visible && i.is_mesh())
        .count();
    if hidden > 0 {
        report
            .notes
            .push(format!("{hidden} hidden item(s) not exported"));
    }
    let data = match format {
        ExportFormat::Obj => {
            let mtl = path.with_extension("mtl");
            let mtl_name = mtl
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let (obj, library) = obj::write(
                &meshes,
                options.obj_save_groups,
                options.obj_save_mtl.then_some(mtl_name.as_str()),
                &mut report.notes,
            );
            if let Some(library) = library {
                write_atomic(&mtl, &library)?;
            }
            obj
        }
        ExportFormat::StlBinary => stl::write_binary(&meshes, &mut report.notes)?,
        ExportFormat::StlAscii => stl::write_ascii(&meshes, &mut report.notes),
        ExportFormat::Gltf => gltf_io::write(&meshes, false, &mut report.notes)?,
        ExportFormat::Glb => gltf_io::write(&meshes, true, &mut report.notes)?,
        ExportFormat::Geo => geo::write(&meshes, &mut report.notes)?,
        ExportFormat::Svg => svg::write(&meshes, &mut report.notes),
        ExportFormat::PlyBinary => ply::write_binary(&meshes, &mut report.notes)?,
        ExportFormat::PlyAscii => ply::write_ascii(&meshes, &mut report.notes)?,
        ExportFormat::Fbx => fbx_write::write(&meshes, &mut report.notes)?,
        ExportFormat::ThreeMf => threemf::write(&meshes, &mut report.notes)?,
        ExportFormat::Alembic => {
            write_atomic_with(path, |temp| abc::write(temp, &meshes, &mut report.notes))?;
            return Ok(report);
        }
        ExportFormat::Lxo => unreachable!("LXO returns through export_lxo above"),
        ExportFormat::Step | ExportFormat::Iges => {
            unreachable!("STEP and IGES return through cad::export above")
        }
        ExportFormat::Usd | ExportFormat::Usda | ExportFormat::Usdc | ExportFormat::Usdz => {
            unreachable!("USD returns through the registered exporter above")
        }
    };
    write_atomic(path, &data)?;
    Ok(report)
}

/// LXO keeps the scene's structure (items, transforms, parenting, hidden
/// state), so unlike the flat formats nothing is baked and hidden items are
/// written hidden. An imported LXO source stays read-only.
fn export_lxo(path: &Path, scene: &Scene) -> Result<ExportReport> {
    if let Some(source) = &scene.import_source {
        let original = Path::new(&source.path);
        if path == original
            || (path.canonicalize().is_ok()
                && path.canonicalize().ok() == original.canonicalize().ok())
        {
            return Err("The imported LXO source is read-only; choose another name".into());
        }
    }
    let mut notes = Vec::new();
    // Backdrops aren't written (BD-14); neither are cameras and lights yet
    // (CL-3 adds them). One that parents other items stays as an empty item
    // so the hierarchy survives.
    let mut model = scene.clone();
    // Group Locators aren't written (LXO export has no empty items yet):
    // their children move up to the locator's own parent in place, so
    // they keep their world placement (GL-08, provisional).
    let locators: Vec<u64> = model
        .items
        .iter()
        .filter(|i| i.is_group_locator())
        .map(|i| i.id)
        .collect();
    if !locators.is_empty() {
        let ids: Vec<u64> = model.items.iter().map(|i| i.id).collect();
        for id in ids {
            let up = |scene: &Scene, mut p: Option<u64>| {
                while let Some(at) = p.filter(|at| locators.contains(at)) {
                    p = scene.item(at).and_then(|i| i.parent);
                }
                p
            };
            let parent = model.item(id).and_then(|i| i.parent);
            if parent.is_some_and(|p| locators.contains(&p)) {
                let target = up(&model, parent);
                model.set_parent(id, target, true).map_err(|e| {
                    format!("LXO export leaves Group Locators out, and an item under one can't keep its place: {e}")
                })?;
            }
        }
        model.items.retain(|i| !i.is_group_locator());
        notes.push(format!(
            "{} Group Locator(s) not exported; their children keep their places",
            locators.len()
        ));
    }
    let parents: std::collections::BTreeSet<u64> =
        model.items.iter().filter_map(|i| i.parent).collect();
    model
        .items
        .retain(|i| i.is_mesh() || parents.contains(&i.id));
    for item in &mut model.items {
        item.backdrop = None;
        item.camera = None;
        item.light = None;
    }
    model.render_camera = None;
    model.images.clear();
    if model
        .selection
        .item
        .is_some_and(|id| model.items.iter().all(|i| i.id != id))
    {
        model.select_item(None);
    }
    let skipped = scene.items.len() - locators.len() - model.items.len();
    if skipped > 0 {
        notes.push(format!("{skipped} Backdrop(s) not exported"));
    }
    let scene = &model;
    let data = crate::lxo::export::write(scene, &mut notes)?;
    write_atomic(path, &data)?;
    Ok(ExportReport {
        items: scene
            .items
            .iter()
            .filter(|i| !i.mesh.faces.is_empty())
            .count(),
        polygons: scene.items.iter().map(|i| i.mesh.faces.len()).sum(),
        notes,
    })
}

/// Same-folder create-new temporary, sync, rename: the destination is either
/// the old file or the complete new one.
fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    write_atomic_with(path, |temp| {
        let mut file = OpenOptions::new()
            .write(true)
            .open(temp)
            .map_err(|e| format!("Cannot create export temporary: {e}"))?;
        file.write_all(data)
            .map_err(|e| format!("Cannot write export: {e}"))
    })
}

/// `write_atomic` for a writer that fills the temporary file itself (a C
/// library that opens the path, like Alembic's). The temporary is created
/// empty first so an existing file is never taken over.
fn write_atomic_with(path: &Path, fill: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
    refuse_scene_file(path)?;
    path.file_name().ok_or("Choose a file to export to")?;
    operandi_files::write_atomic_with(path, fill)
}

/// Export never replaces an Operandi scene.
pub fn refuse_scene_file(path: &Path) -> Result<()> {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("opscene"))
    {
        return Err("Export can't replace a scene file; choose another name".into());
    }
    Ok(())
}

/// A polygon as read from a file, before validation.
#[derive(Clone, Debug, Default)]
pub(crate) struct RawFace {
    pub vertices: Vec<u32>,
    /// One per corner when the file had texture coordinates for this face.
    pub uv: Option<Vec<DVec2>>,
    pub material: String,
    pub smooth: bool,
}

/// One item's worth of geometry read from a file.
#[derive(Clone, Debug, Default)]
pub(crate) struct RawMesh {
    pub name: String,
    pub points: Vec<DVec3>,
    pub faces: Vec<RawFace>,
    /// Material settings the file carried, by tag name (glTF).
    pub materials: BTreeMap<String, crate::Material>,
}

/// Build the new scene. Each non-empty raw mesh becomes one item.
fn scene_from(meshes: Vec<RawMesh>, notes: &mut Vec<String>) -> Result<Scene> {
    let mut scene = Scene::default();
    let mut first = None;
    for mut raw in meshes {
        let name = raw.name.clone();
        scene.materials.append(&mut raw.materials);
        let Some(mesh) = build_mesh(raw, notes)? else {
            continue;
        };
        scene.new_mesh()?;
        let item = scene.items.last_mut().expect("new_mesh added an item");
        if !name.trim().is_empty() {
            item.name = name;
        }
        item.mesh = mesh.into();
        first.get_or_insert(item.id);
    }
    if scene.items.is_empty() {
        return Err("The file has no polygons to import; current document preserved".into());
    }
    scene.select_item(first);
    scene.validate()?;
    Ok(scene)
}

/// Turn file polygons into a mesh the validator accepts, without dropping
/// usable data: repeated corners collapse, polygons with fewer than three
/// distinct corners are skipped (and counted), zero-area polygons are kept
/// as recorded degenerate faces, and edges shared by more than two polygons
/// or with mismatched winding are recorded as exceptions that tools refuse to
/// edit, rather than failing the import.
fn build_mesh(raw: RawMesh, notes: &mut Vec<String>) -> Result<Option<Mesh>> {
    // A mesh over the size limit is left out with a note; the rest of the
    // file still comes in (hidden-limits B1: it refused the whole file).
    if let Some(e) = crate::mesh_too_big(raw.points.len(), raw.faces.len()) {
        notes.push(format!("{}: left out. {e}", raw.name));
        return Ok(None);
    }
    if raw.points.iter().any(|p| !p.is_finite()) {
        return Err(format!("{}: non-finite point coordinates", raw.name));
    }
    let mut faces = Vec::new();
    let mut uvs: Vec<Vec<Option<DVec2>>> = Vec::new();
    let mut skipped = 0;
    for f in raw.faces {
        if f.vertices.iter().any(|&v| v as usize >= raw.points.len()) {
            return Err(format!("{}: polygon refers to a missing point", raw.name));
        }
        // Collapse consecutive repeats (including last-to-first).
        let mut corners: Vec<(u32, Option<DVec2>)> = Vec::new();
        for (i, &v) in f.vertices.iter().enumerate() {
            let uv = f.uv.as_ref().and_then(|u| u.get(i).copied());
            if corners.last().is_none_or(|c| c.0 != v) {
                corners.push((v, uv));
            }
        }
        while corners.len() > 1 && corners.first().map(|c| c.0) == corners.last().map(|c| c.0) {
            corners.pop();
        }
        if corners.iter().map(|c| c.0).collect::<BTreeSet<_>>().len() < 3 || corners.len() > 4096 {
            skipped += 1;
            continue;
        }
        let has_uv = f.uv.is_some() && corners.iter().all(|c| c.1.is_some_and(|u| u.is_finite()));
        uvs.push(corners.iter().map(|c| c.1.filter(|_| has_uv)).collect());
        faces.push(Face {
            vertices: corners.iter().map(|c| c.0).collect(),
            uv: None,
            material: f.material,
            smooth: f.smooth,
            part: None,
            material_label: None,
        });
    }
    if skipped > 0 {
        notes.push(format!(
            "{}: skipped {skipped} point/line element(s) with fewer than 3 corners",
            raw.name
        ));
    }
    if faces.is_empty() {
        return Ok(None);
    }
    // Keep only points some polygon uses, in file order.
    let mut remap = vec![u32::MAX; raw.points.len()];
    for f in &faces {
        for &v in &f.vertices {
            remap[v as usize] = 0;
        }
    }
    let mut points = Vec::new();
    for (i, slot) in remap.iter_mut().enumerate() {
        if *slot == 0 {
            *slot = points.len() as u32;
            points.push(raw.points[i]);
        }
    }
    for f in &mut faces {
        for v in &mut f.vertices {
            *v = remap[*v as usize];
        }
    }
    let mut mesh = Mesh {
        vertices: points,
        faces: faces.into(),
        ..Default::default()
    };
    if uvs.iter().flatten().any(Option::is_some) {
        mesh.uv_maps.insert(UV_MAP.into(), uvs);
    }
    finish_mesh(&mut mesh, &raw.name, notes)?;
    Ok(Some(mesh))
}

/// The checks every imported mesh gets: zero-area polygons are recorded as
/// degenerate and non-manifold or mismatched edges as exceptions (both
/// noted), then the mesh is validated.
pub fn finish_mesh(mesh: &mut Mesh, name: &str, notes: &mut Vec<String>) -> Result<()> {
    let degenerate: BTreeSet<u32> = (0..mesh.faces.len())
        .filter(|&f| {
            let face = &mesh.faces[f];
            face.vertices.iter().collect::<BTreeSet<_>>().len() != face.vertices.len()
                || mesh.normal(f) == DVec3::ZERO
        })
        .map(|f| f as u32)
        .collect();
    if !degenerate.is_empty() {
        notes.push(format!(
            "{name}: {} zero-area polygon(s) kept as degenerate",
            degenerate.len()
        ));
    }
    mesh.attributes.degenerate_faces = degenerate;
    let exceptions = edge_exceptions(mesh);
    if !exceptions.is_empty() {
        notes.push(format!(
            "{name}: {} edge(s) are non-manifold or have mismatched winding; tools won't edit them",
            exceptions.len()
        ));
    }
    mesh.attributes.extrude_edge_uses = exceptions;
    mesh.validate().map_err(|e| format!("{name}: {e}"))
}

/// Edges the validator treats as exceptional: more than two uses, or two
/// uses running the same direction. The validator's own result, so always
/// what `Mesh::validate` expects (polylines aren't counted), and
/// remembered per topology so the validation that follows doesn't
/// recompute it. It used to build a map entry per edge here, which took
/// most of the time opening a 5-million-polygon scene.
pub fn edge_exceptions(mesh: &Mesh) -> BTreeMap<String, Vec<[u32; 3]>> {
    (*mesh.exceptional_edge_uses()).clone()
}

/// An item ready to write: world-space points, polygons wound for the world
/// transform, and per-corner UVs from the item's texture map.
pub(crate) struct WorldMesh {
    pub name: String,
    pub mesh: Mesh,
    /// Polygon index in `mesh` → whether it is an open polyline.
    pub polyline: Vec<bool>,
    pub uvs: Option<Vec<Vec<Option<DVec2>>>>,
    /// The scene's default-material smoothing (SS-01).
    pub smoothing: crate::shading::SmoothingSettings,
    /// The scene's stored materials, by tag name.
    pub materials: BTreeMap<String, crate::Material>,
}
impl WorldMesh {
    /// Triangles of a polygon as corner indices. Uses the same ear clipping as
    /// drawing; a fan is the fallback for shapes it refuses (they are drawn
    /// the same way, so the exported surface matches what was on screen as
    /// closely as the shape allows).
    pub fn triangles(&self, face: usize) -> Vec<[usize; 3]> {
        self.mesh.surface_triangles(face).unwrap_or_else(|_| {
            (1..self.mesh.faces[face].vertices.len().saturating_sub(1))
                .map(|i| [0, i, i + 1])
                .collect()
        })
    }
    /// Unit normal of a polygon, or zero for degenerate ones.
    pub fn normal(&self, face: usize) -> DVec3 {
        self.mesh.normal(face).normalize_or_zero()
    }
    pub fn surface_faces(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.mesh.faces.len()).filter(|&f| !self.polyline[f])
    }
}

/// The visible mesh items an export of `kind` writes.
fn world_meshes_of(scene: &Scene, kind: GltfExportType) -> Result<Vec<WorldMesh>> {
    if kind == GltfExportType::All {
        return world_meshes(scene);
    }
    let mut chosen: BTreeSet<u64> = scene.selected_items().into_iter().collect();
    if kind == GltfExportType::SelectedWithHierarchy {
        loop {
            let more: Vec<u64> = scene
                .items
                .iter()
                .filter(|i| {
                    i.parent.is_some_and(|p| chosen.contains(&p)) && !chosen.contains(&i.id)
                })
                .map(|i| i.id)
                .collect();
            if more.is_empty() {
                break;
            }
            chosen.extend(more);
        }
    }
    let mut out = Vec::new();
    for item in scene
        .items
        .iter()
        .filter(|i| i.visible && i.is_mesh() && chosen.contains(&i.id))
    {
        out.push(world_mesh(scene, item)?);
    }
    if out.is_empty() {
        return Err("Nothing to export: no selected item has polygons".into());
    }
    Ok(out)
}

fn world_meshes(scene: &Scene) -> Result<Vec<WorldMesh>> {
    let mut out = Vec::new();
    // Backdrops are viewport references, never part of the model (BD-14);
    // cameras and lights have no geometry.
    for item in scene.items.iter().filter(|i| i.visible && i.is_mesh()) {
        out.push(world_mesh(scene, item)?);
    }
    Ok(out)
}

fn world_mesh(scene: &Scene, item: &Item) -> Result<WorldMesh> {
    let matrix: DMat4 = scene.world_matrix(item.id)?;
    let mirrored = matrix.determinant() < 0.0;
    let mut mesh: Mesh = (*item.mesh).clone();
    for p in &mut mesh.vertices {
        *p = matrix.transform_point3(*p);
    }
    let polyline: Vec<bool> = (0..mesh.faces.len()).map(|f| mesh.is_polyline(f)).collect();
    let map = mesh
        .uv_maps
        .get(UV_MAP)
        .or_else(|| mesh.uv_maps.values().next())
        .cloned();
    let mut uvs: Option<Vec<Vec<Option<DVec2>>>> = map.or_else(|| {
        mesh.faces.iter().any(|f| f.uv.is_some()).then(|| {
            mesh.faces
                .iter()
                .map(|f| match &f.uv {
                    Some(uv) => uv.iter().map(|&u| Some(u)).collect(),
                    None => vec![None; f.vertices.len()],
                })
                .collect()
        })
    });
    // A mirroring transform turns the polygons inside out; reverse each
    // surface polygon (and its UVs) so the exported normals face outward.
    if mirrored {
        for (f, face) in mesh.faces.iter_mut().enumerate() {
            if polyline[f] {
                continue;
            }
            face.vertices[1..].reverse();
            if let Some(uv) = uvs.as_mut().and_then(|u| u.get_mut(f)) {
                uv[1..].reverse();
            }
        }
    }
    if uvs
        .as_ref()
        .is_some_and(|u| u.iter().flatten().all(Option::is_none))
    {
        uvs = None;
    }
    Ok(WorldMesh {
        smoothing: scene.smoothing.clone(),
        materials: scene.materials.clone(),
        name: item.name.clone(),
        mesh,
        polyline,
        uvs,
    })
}

/// Per-corner normals, by the scene's smoothing (SS-02) so an export
/// shades like the viewport.
pub(crate) fn corner_normals(m: &WorldMesh) -> Vec<Vec<DVec3>> {
    crate::shading::mesh_corner_normals(&m.mesh, &m.smoothing)
}

/// Shortest decimal that reads back as the same f32. Interchange formats
/// store single precision; writing f32 avoids noise digits from f64.
pub(crate) fn num(v: f64) -> String {
    let f = v as f32;
    if f == 0.0 { "0".into() } else { format!("{f}") }
}
