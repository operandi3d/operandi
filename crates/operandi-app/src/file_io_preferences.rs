//! Preferences > File I/O (spec preferences-2026-11-06, Tier 2): Wavefront,
//! STL and glTF options, and how rendered pictures are saved. Saved per
//! user in `file-io-preferences.json`. Defaults are the spec's where it
//! gives them, otherwise *provisional* and as before.
use crate::prefs_file::PrefsFile;
use eframe::egui;
use operandi_core::interchange::{FileUnit, GltfExportType, IoOptions};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

/// Default Image Format for rendered pictures.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    /// Provisional default: what renders were saved as so far.
    #[default]
    Png,
    Jpeg,
}
impl ImageFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileIoPreferences {
    version: u8,
    pub obj_import_units: FileUnit,
    pub obj_groups_as_meshes: bool,
    pub obj_export_units: FileUnit,
    pub obj_save_groups: bool,
    pub obj_save_mtl: bool,
    pub stl_import_units: FileUnit,
    pub stl_export_units: FileUnit,
    pub gltf_export: GltfExportType,
    pub gltf_geometry: bool,
    pub gltf_materials: bool,
    pub gltf_tangents: bool,
    /// Indices data type: Unsigned Short when on (Unsigned Integer is the
    /// spec's default).
    pub gltf_short_indices: bool,
    pub image_format: ImageFormat,
    /// PNG Compression Level, 1 (fast) to 9 (small). Provisional: 6.
    pub png_compression: u8,
    /// JPEG Quality, percent. Provisional: 90.
    pub jpeg_quality: u8,
    /// Copy Added Files into the Project (docs/PROJECT_SYSTEM.md phase 2):
    /// a picture or IES file chosen from outside the active project is
    /// copied into it. On by default; files saved before it had no entry.
    pub copy_into_project: bool,
    /// TL-83: what USD exports write of the animation.
    pub usd_animation: operandi_core::interchange::UsdAnimation,
}
impl Default for FileIoPreferences {
    fn default() -> Self {
        let o = IoOptions::default();
        Self {
            version: 1,
            obj_import_units: o.obj_import_units,
            obj_groups_as_meshes: o.obj_groups_as_meshes,
            obj_export_units: o.obj_export_units,
            obj_save_groups: o.obj_save_groups,
            obj_save_mtl: o.obj_save_mtl,
            stl_import_units: o.stl_import_units,
            stl_export_units: o.stl_export_units,
            gltf_export: o.gltf_export,
            gltf_geometry: o.gltf_geometry,
            gltf_materials: o.gltf_materials,
            gltf_tangents: o.gltf_tangents,
            gltf_short_indices: o.gltf_short_indices,
            image_format: ImageFormat::Png,
            png_compression: 6,
            jpeg_quality: 90,
            copy_into_project: true,
            usd_animation: o.usd_animation,
        }
    }
}
impl FileIoPreferences {
    /// Hand the format options to core's readers and writers.
    pub fn apply(&self) {
        operandi_core::interchange::set_io_options(IoOptions {
            obj_import_units: self.obj_import_units,
            obj_groups_as_meshes: self.obj_groups_as_meshes,
            obj_export_units: self.obj_export_units,
            obj_save_groups: self.obj_save_groups,
            obj_save_mtl: self.obj_save_mtl,
            stl_import_units: self.stl_import_units,
            stl_export_units: self.stl_export_units,
            gltf_export: self.gltf_export,
            gltf_geometry: self.gltf_geometry,
            gltf_materials: self.gltf_materials,
            gltf_tangents: self.gltf_tangents,
            gltf_short_indices: self.gltf_short_indices,
            up_axis: crate::camera::up_axis(),
            // TL-83; scene.saveAs animation: overrides it per export.
            usd_animation: self.usd_animation,
        });
    }
    pub fn image(&self) -> ImageOptions {
        ImageOptions {
            png_compression: self.png_compression,
            jpeg_quality: self.jpeg_quality,
        }
    }
}
impl crate::prefs_file::PrefsFile for FileIoPreferences {
    const INVALID: &'static str = "Invalid file I/O preference";
    fn valid(&self) -> bool {
        self.version == 1
            && (1..=9).contains(&self.png_compression)
            && (1..=100).contains(&self.jpeg_quality)
    }
}

/// How a picture is encoded (Image I/O).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageOptions {
    pub png_compression: u8,
    pub jpeg_quality: u8,
}
impl Default for ImageOptions {
    fn default() -> Self {
        FileIoPreferences::default().image()
    }
}

/// Write 8-bit RGBA `pixels` to `path`, PNG or JPEG by its extension.
pub fn save_rgba(
    path: &Path,
    pixels: &[u8],
    width: u32,
    height: u32,
    options: ImageOptions,
) -> Result<(), String> {
    use image::ImageEncoder;
    let jpeg = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg"));
    let file = fs::File::create(path).map_err(|e| e.to_string())?;
    let writer = io::BufWriter::new(file);
    if jpeg {
        let rgb: Vec<u8> = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|c| [c[0], c[1], c[2]])
            .collect();
        image::codecs::jpeg::JpegEncoder::new_with_quality(
            writer,
            options.jpeg_quality.clamp(1, 100),
        )
        .write_image(&rgb, width, height, image::ExtendedColorType::Rgb8)
        .map_err(|e| e.to_string())
    } else {
        use image::codecs::png::{CompressionType, FilterType, PngEncoder};
        let level = match options.png_compression {
            0..=3 => CompressionType::Fast,
            4..=7 => CompressionType::Default,
            _ => CompressionType::Best,
        };
        PngEncoder::new_with_quality(writer, level, FilterType::Adaptive)
            .write_image(pixels, width, height, image::ExtendedColorType::Rgba8)
            .map_err(|e| e.to_string())
    }
}

impl super::Operandi {
    /// Preferences > File I/O.
    pub(super) fn file_io_page(&mut self, ui: &mut egui::Ui) {
        let before = self.file_io_preferences.clone();
        let f = &mut self.file_io_preferences;
        egui::ScrollArea::vertical()
            .id_salt("preferences-file-io")
            .max_height(ui.available_height())
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let d = FileIoPreferences::default();
                use crate::form::{Unit, rows};
                let units: Vec<_> = FileUnit::ALL.iter().map(|&u| (u, u.label())).collect();
                rows::section(ui, "Wavefront OBJ", false, |ui| {
                    rows::check_d(
                        ui,
                        "Import Groups as Separate Meshes",
                        &mut f.obj_groups_as_meshes,
                        d.obj_groups_as_meshes,
                        "Each OBJ group becomes its own mesh item",
                    );
                    rows::choice_d(
                        ui,
                        "Import Units",
                        &mut f.obj_import_units,
                        &d.obj_import_units,
                        &units,
                        "The unit an OBJ file's numbers are read in",
                    );
                    rows::check_d(
                        ui,
                        "Save Meshes as Groups",
                        &mut f.obj_save_groups,
                        d.obj_save_groups,
                        "Each mesh item is written as an OBJ group",
                    );
                    rows::check_d(
                        ui,
                        "Save Material File",
                        &mut f.obj_save_mtl,
                        d.obj_save_mtl,
                        "Write a .mtl file beside the OBJ",
                    );
                    rows::choice_d(
                        ui,
                        "Export Units",
                        &mut f.obj_export_units,
                        &d.obj_export_units,
                        &units,
                        "The unit an OBJ file's numbers are written in",
                    );
                });
                rows::section(ui, "STL", false, |ui| {
                    rows::choice_d(
                        ui,
                        "Interpret Import Units As",
                        &mut f.stl_import_units,
                        &d.stl_import_units,
                        &units,
                        "The unit an STL file's numbers are read in",
                    );
                    rows::choice_d(
                        ui,
                        "Interpret Export Units As",
                        &mut f.stl_export_units,
                        &d.stl_export_units,
                        &units,
                        "The unit an STL file's numbers are written in",
                    );
                });
                rows::section(ui, "glTF", false, |ui| {
                    let types: Vec<_> = GltfExportType::ALL
                        .iter()
                        .map(|&t| (t, t.label()))
                        .collect();
                    rows::choice_d(
                        ui,
                        "Export Type",
                        &mut f.gltf_export,
                        &d.gltf_export,
                        &types,
                        "One .glb file, or .gltf with its data beside it",
                    );
                    rows::check_d(
                        ui,
                        "Save Geometry",
                        &mut f.gltf_geometry,
                        d.gltf_geometry,
                        "Write the meshes",
                    );
                    rows::check_d(
                        ui,
                        "Save Materials",
                        &mut f.gltf_materials,
                        d.gltf_materials,
                        "Write the materials",
                    );
                    rows::check_d(
                        ui,
                        "Save Tangent Basis",
                        &mut f.gltf_tangents,
                        d.gltf_tangents,
                        "Write tangents for normal maps",
                    );
                    rows::choice_d(
                        ui,
                        "Indices Data Type",
                        &mut f.gltf_short_indices,
                        &d.gltf_short_indices,
                        &[(true, "Unsigned Short"), (false, "Unsigned Integer")],
                        "The number type polygon indices are written as",
                    );
                });
                rows::section(ui, "USD", false, |ui| {
                    use operandi_core::interchange::UsdAnimation as A;
                    rows::choice_d(
                        ui,
                        "Animation",
                        &mut f.usd_animation,
                        &d.usd_animation,
                        &[
                            (A::On, "On"),
                            (A::Off, "Off"),
                            (A::CurrentFrame, "Current Frame Only"),
                        ],
                        "What USD exports write of the animation: time samples for every frame of the scene range, none, or the current frame only (TL-83)",
                    );
                });
                rows::section(ui, "Images", false, |ui| {
                    rows::choice_d(
                        ui,
                        "Default Image Format",
                        &mut f.image_format,
                        &d.image_format,
                        &[(ImageFormat::Png, "PNG"), (ImageFormat::Jpeg, "JPEG")],
                        "What renders and Preview pictures are saved as",
                    );
                    rows::number_d(
                        ui,
                        "PNG Compression Level",
                        &mut f.png_compression,
                        d.png_compression,
                        Unit::Count,
                        1..=9,
                        "Higher is smaller and slower",
                    );
                    rows::number_d(
                        ui,
                        "JPEG Quality",
                        &mut f.jpeg_quality,
                        d.jpeg_quality,
                        Unit::Percent,
                        1..=100,
                        "Higher looks better and is larger",
                    );
                });
                rows::section(ui, "Projects", false, |ui| {
                    rows::check_d(
                        ui,
                        "Copy Added Files into the Project",
                        &mut f.copy_into_project,
                        d.copy_into_project,
                        "With a project set, pictures, IES files and imported materials' \
                         images from outside it are copied into its folders",
                    );
                });
                rows::note(
                    ui,
                    "STL's ASCII or binary is the choice in File > Export. Not built: \
                     JPEG Sub-Sampling (the encoder has one setting), OBJ static meshes \
                     and deformed geometry, glTF animation, and the EXR and image cache \
                     options (those features come later).",
                );
            });
        if self.file_io_preferences == before {
            return;
        }
        self.file_io_preferences.apply();
        if let Err(error) = self
            .file_io_preferences
            .save(&self.environment.paths().file_io_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_save_with_the_chosen_encoding() {
        let dir = std::env::temp_dir().join(format!("operandi-image-io-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        // Noisy pixels so compression and quality show in the size.
        let (w, h) = (64u32, 64u32);
        let pixels: Vec<u8> = (0..w * h * 4)
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
            .collect();
        let size = |name: &str, o: ImageOptions| {
            let path = dir.join(name);
            save_rgba(&path, &pixels, w, h, o).unwrap();
            fs::metadata(&path).unwrap().len()
        };
        let low = size(
            "low.jpg",
            ImageOptions {
                jpeg_quality: 10,
                png_compression: 6,
            },
        );
        let high = size(
            "high.jpg",
            ImageOptions {
                jpeg_quality: 100,
                png_compression: 6,
            },
        );
        assert!(low < high, "{low} < {high}");
        let png = dir.join("p.png");
        save_rgba(&png, &pixels, w, h, ImageOptions::default()).unwrap();
        assert_eq!(image::open(&png).unwrap().width(), w);
        fs::remove_dir_all(dir).unwrap();
        // Preferences reach core and survive a save.
        let p = FileIoPreferences {
            obj_export_units: FileUnit::Centimeters,
            gltf_short_indices: true,
            image_format: ImageFormat::Jpeg,
            ..Default::default()
        };
        assert_eq!(p.image_format.extension(), "jpg");
        let path =
            std::env::temp_dir().join(format!("operandi-fileio-{}.json", std::process::id()));
        p.save(&path).unwrap();
        assert_eq!(FileIoPreferences::load(&path), p);
        let _ = fs::remove_file(path);
    }
}
