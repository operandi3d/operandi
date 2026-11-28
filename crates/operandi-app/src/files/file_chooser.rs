//! Native file and folder panels (macOS Open/Save panels via `rfd`; also
//! native on Windows and Linux), shown as non-blocking sheets. Tests swap in
//! a plain function as a stand-in: they have no window to show a panel, and
//! without either the typed-path dialogs remain as a fallback.
use std::path::{Path, PathBuf};

/// What to ask the user for.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Choose {
    /// An existing scene or LXO file, starting in `folder`.
    Open { folder: PathBuf },
    /// An interchange file (OBJ, STL, glTF/GLB, GEO) to import.
    Import { folder: PathBuf },
    /// Where to save a scene, starting in `folder` with `name` suggested.
    Save { folder: PathBuf, name: String },
    /// Where to export in `format`, starting in `folder` with `name` suggested.
    Export {
        folder: PathBuf,
        name: String,
        format: operandi_core::interchange::ExportFormat,
    },
    /// A PNG or JPEG picture for a Backdrop, starting in `folder`.
    Image { folder: PathBuf },
    /// An IES photometry file for a photometric light (LP-PH-01).
    Photometry { folder: PathBuf },
    /// A latitude-longitude picture for an environment (HDR, EXR, PNG,
    /// JPEG or TIFF).
    EnvironmentImage { folder: PathBuf },
    /// A node graph file (JSON) to import as Node Materials (NM-05).
    NodeGraph { folder: PathBuf },
    /// A Python script to run (Run Script…).
    Script { folder: PathBuf },
    /// A JSON colour scheme to import.
    LoadColorScheme { folder: PathBuf },
    /// Where to export the current colours as JSON.
    SaveColorScheme { folder: PathBuf, name: String },
    /// Where to save a rendered picture (PNG or JPEG).
    RenderImage { folder: PathBuf, name: String },
    /// Render window Save Image... (IV-36): where to save in `format`.
    ViewerSave {
        folder: PathBuf,
        name: String,
        format: crate::image_viewer::SaveFormat,
    },
    /// Render window Load Image... (IV-38): a picture to add to the strip.
    ViewerLoad { folder: PathBuf },
    /// Where to save a preset (PB-40..42), starting in `folder` with
    /// `name` suggested; `title` names the kind.
    SavePreset {
        folder: PathBuf,
        name: String,
        title: String,
    },
    /// Where to save an environment preset (`.openv`), `name` suggested.
    SaveEnvironmentPreset { folder: PathBuf, name: String },
    /// A preset file to load (PB-50): only presets are offered.
    LoadPreset { folder: PathBuf },
    /// A MaterialX file to import (Stage B).
    MaterialX { folder: PathBuf },
    /// Where to export a material as MaterialX, `name` suggested.
    SaveMaterialX { folder: PathBuf, name: String },
    /// Where to save a baked map (PNG), `name` suggested; `title` is the
    /// command's (bake_dialogs.rs).
    SaveImage {
        folder: PathBuf,
        name: String,
        title: String,
    },
    /// A PNG texture to pad around the active mesh's UVs.
    LoadTexture { folder: PathBuf },
    /// Where to save a painted image, `name` suggested (its extension
    /// first among the types).
    #[cfg(feature = "painting")]
    SavePaintImage { folder: PathBuf, name: String },
    /// Where to export the UV layout (UVT-120): SVG, or EPS with `eps`.
    SaveUvLayout {
        folder: PathBuf,
        name: String,
        eps: bool,
    },
    /// A pie file (Export Pie / Import Pie, UIC-312): where to save one,
    /// `name` suggested, or with no name one to open.
    PieFile {
        folder: PathBuf,
        name: Option<String>,
    },
    /// A picture for a control's custom icon (UIC-314): PNG, JPEG or SVG.
    IconFile { folder: PathBuf },
    /// A folder (new ones can be made in the panel), starting in `folder`.
    Folder {
        folder: PathBuf,
        title: &'static str,
    },
}

/// Shows a panel; `None` when cancelled.
pub(super) type Chooser = fn(Choose) -> Option<PathBuf>;

/// What to do with the chosen path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Purpose {
    SaveAs,
    Open,
    /// File > Import: into the current scene (`scene.importReference`).
    Import,
    Export(operandi_core::interchange::ExportFormat),
    SetProject,
    ProjectFolder,
    BackupDirectory,
    /// The picture for Backdrop item `0`.
    BackdropImage(u64),
    /// The Background Image for camera item `0` (CP-V26).
    CameraBackground(u64),
    /// The picture for shader tree Image Map layer `0`.
    ShaderImage(u64),
    /// Load Image… on environment item `0`: its image layer's picture.
    EnvironmentImage(u64),
    /// Preview > Options > Save Image.
    PreviewImage,
    /// Render window Save Image... (IV-36).
    ViewerSave(crate::image_viewer_ui::SaveDialog),
    /// Render window Load Image... (IV-38).
    ViewerLoad,
    /// Preferences > Rendering > Recent Frames Folder (IV-12).
    RecentFramesFolder,
    ImportColorScheme,
    ExportColorScheme,
    /// The photometry file for light item `0`.
    Photometry(u64),
    /// A new light-material image layer for light item `0`.
    LightLayerImage(u64),
    /// The active view's Reflection matcap image (matcap.rs).
    MatcapImage,
    /// The Image falloff's picture (FO-54).
    FalloffImage,
    /// Preferences > Defaults > Application > Default Scene.
    DefaultScene,
    /// Save a preset of this source (preset_ui.rs); `1` also writes a
    /// .mtlx beside a material preset (`preset.saveAs mtlx:`).
    SavePreset(operandi_core::preset::PresetSource, bool),
    /// Save an environment preset with these options (EP §Save).
    SaveEnvironmentPreset(operandi_core::preset::EnvironmentPresetOptions),
    /// Load a preset; a material preset goes into this material group.
    LoadPreset(Option<operandi_core::shader_tree::NodeId>),
    /// Import Node Material…: placed as Add Layer places a layer next to
    /// this selected shader tree layer.
    ImportNodeGraph(Option<operandi_core::shader_tree::NodeId>),
    /// Run Script…: run the chosen .py file.
    RunScript,
    /// File > Import MaterialX…: new layers go directly above layer `0`,
    /// or at the top of the shader tree.
    ImportMaterialX(Option<operandi_core::shader_tree::NodeId>),
    /// File > Export Material as MaterialX…: the material or group `0`.
    ExportMaterialX(operandi_core::shader_tree::NodeId),
    /// A folder to add as a Preset Browser root (PB-12).
    PresetBrowserRoot,
    /// A picture for the Preset Browser entry in `thumb_target` (PB-112).
    PresetBrowserThumbnail,
    /// Export UVs to SVG / EPS (`true`).
    UvLayout(bool),
    /// A custom icon for the workbench control in
    /// `workbench.icon_target` (LAY-82).
    WorkbenchIcon,
    /// Bake Curvature/Thickness...: the file the pending line writes.
    BakeTo,
    /// Pad Texture to UVs...: the picture the pending line pads.
    PadTexture,
    /// Save Image / Save Image As for image `1` of scene tab `0`
    /// (painting.rs). The tab travels with the answer (Astra P8-001).
    #[cfg(feature = "painting")]
    PaintImage(u64, u64),
    /// Export Pie: write the pie in `pie_edit.export` there.
    ExportPie,
    /// Import Pie: read the chosen pie file.
    ImportPie,
    /// Choose File… for the control in `pie_edit.icon_target` (UIC-314).
    FormIcon,
}

/// A native panel shown as a sheet on the window. It doesn't block: a modal
/// panel run from inside the frame update let winit receive events (a window
/// resize) while it was still handling the frame, which aborted the app.
/// The app polls this each frame and acts when the user answers.
pub(super) struct Pending {
    pub purpose: Purpose,
    future: std::pin::Pin<Box<dyn std::future::Future<Output = Option<PathBuf>>>>,
}
/// Tests: a stand-in native panel that stays open until the test puts an
/// answer here (`Some(None)` is Cancel).
#[cfg(test)]
pub(super) type HeldAnswer = std::rc::Rc<std::cell::RefCell<Option<Option<PathBuf>>>>;

#[cfg(test)]
impl Pending {
    pub fn held(purpose: Purpose, answer: HeldAnswer) -> Self {
        Self {
            purpose,
            future: Box::pin(std::future::poll_fn(move |_| {
                match answer.borrow_mut().take() {
                    Some(a) => std::task::Poll::Ready(a),
                    None => std::task::Poll::Pending,
                }
            })),
        }
    }
}

impl Pending {
    /// `Some(answer)` once the panel closed; `None` while it's open.
    pub fn poll(&mut self) -> Option<Option<PathBuf>> {
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        match self.future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(answer) => Some(answer),
            std::task::Poll::Pending => None,
        }
    }
}

/// Extensions a panel enables. One filter holding every type: macOS shows
/// only the first filter's types as selectable, which greyed out .lxo (or
/// the interchange formats) although choosing them worked. Upper-case
/// spellings are listed for platforms that match case-sensitively.
fn readable(scene: bool) -> Vec<String> {
    let mut list: Vec<String> = vec!["lxo".into()];
    if scene {
        list.push("opscene".into());
    }
    // Built-in types, then those of kits that are on (KIT-1).
    list.extend(
        operandi_core::interchange::import_extensions()
            .into_iter()
            .map(|(ext, _)| ext),
    );
    list.iter()
        .flat_map(|e| [e.to_string(), e.to_ascii_uppercase()])
        .collect()
}

/// What Load Image… on an environment offers.
pub(super) const ENVIRONMENT_IMAGE_EXTENSIONS: &[&str] =
    &["hdr", "exr", "png", "jpg", "jpeg", "tif", "tiff"];

/// Show the native panel for `request` without blocking.
pub(super) fn start(request: Choose, purpose: Purpose) -> Pending {
    use rfd::AsyncFileDialog as D;
    let path = |h: Option<rfd::FileHandle>| h.map(|h| h.path().to_path_buf());
    let future: std::pin::Pin<Box<dyn std::future::Future<Output = Option<PathBuf>>>> =
        match request {
            Choose::Open { folder } => {
                let f = D::new()
                    .set_title("Open Scene")
                    .set_directory(folder)
                    .add_filter("Scenes and models", &readable(true))
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::Import { folder } => {
                let f = D::new()
                    .set_title("Import")
                    .set_directory(folder)
                    .add_filter(
                        "Modo LXO, OBJ, STL, glTF, GLB or VideoScape GEO",
                        &readable(false),
                    )
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::Save { folder, name } => {
                let f = D::new()
                    .set_title("Save Scene As")
                    .set_directory(folder)
                    .set_file_name(name)
                    .add_filter("Operandi scene", &["opscene"])
                    .save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::Export {
                folder,
                name,
                format,
            } => {
                let f = D::new()
                    .set_title(format!("Export {}", format.label()))
                    .set_directory(folder)
                    .set_file_name(name)
                    .add_filter(format.label(), &[format.extension()])
                    .save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::Image { folder } => {
                let types: Vec<String> = ["png", "jpg", "jpeg"]
                    .iter()
                    .flat_map(|e| [e.to_string(), e.to_ascii_uppercase()])
                    .collect();
                let f = D::new()
                    .set_title("Load Backdrop Image")
                    .set_directory(folder)
                    .add_filter("PNG or JPEG picture", &types)
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::EnvironmentImage { folder } => {
                let types: Vec<String> = ENVIRONMENT_IMAGE_EXTENSIONS
                    .iter()
                    .flat_map(|e| [e.to_string(), e.to_ascii_uppercase()])
                    .collect();
                let f = D::new()
                    .set_title("Load Environment Image")
                    .set_directory(folder)
                    .add_filter("HDR, OpenEXR, PNG, JPEG or TIFF picture", &types)
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::Photometry { folder } => {
                let f = D::new()
                    .set_title("Load Photometry File")
                    .set_directory(folder)
                    .add_filter("IES photometry", &["ies", "IES"])
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::PieFile { folder, name } => {
                let types = [crate::pie_forms::EXTENSION, "json"];
                match name {
                    Some(name) => {
                        let f = D::new()
                            .set_title("Export Pie")
                            .set_directory(folder)
                            .set_file_name(name)
                            .add_filter("Operandi pie", &types)
                            .save_file();
                        Box::pin(async move { path(f.await) })
                    }
                    None => {
                        let f = D::new()
                            .set_title("Import Pie")
                            .set_directory(folder)
                            .add_filter("Operandi pie", &types)
                            .pick_file();
                        Box::pin(async move { path(f.await) })
                    }
                }
            }
            Choose::IconFile { folder } => {
                let types: Vec<String> = ["png", "jpg", "jpeg", "svg"]
                    .iter()
                    .flat_map(|e| [e.to_string(), e.to_ascii_uppercase()])
                    .collect();
                let f = D::new()
                    .set_title("Choose Icon")
                    .set_directory(folder)
                    .add_filter("PNG, JPEG or SVG picture", &types)
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::Script { folder } => {
                let f = D::new()
                    .set_title("Run Script")
                    .set_directory(folder)
                    .add_filter("Python scripts", &["py", "PY"])
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::NodeGraph { folder } => {
                let f = D::new()
                    .set_title("Import Node Material")
                    .set_directory(folder)
                    .add_filter("Node graphs", &["json", "JSON"])
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::LoadColorScheme { folder } => {
                let f = D::new()
                    .set_title("Import Color Scheme")
                    .set_directory(folder)
                    .add_filter("Color schemes", &["json", "JSON"])
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::SaveColorScheme { folder, name } => {
                let f = D::new()
                    .set_title("Export Color Scheme")
                    .set_directory(folder)
                    .set_file_name(name)
                    .add_filter("Color schemes", &["json"])
                    .save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::RenderImage { folder, name } => {
                let f = D::new()
                    .set_title("Save Image")
                    .set_directory(folder)
                    .set_file_name(name)
                    .add_filter("PNG", &["png"])
                    .add_filter("JPEG", &["jpg", "jpeg"])
                    .save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::ViewerSave {
                folder,
                name,
                format,
            } => {
                let f = D::new()
                    .set_title("Save Image")
                    .set_directory(folder)
                    .set_file_name(name)
                    .add_filter(format.label(), &[format.extension()])
                    .save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::ViewerLoad { folder } => {
                let types: Vec<String> = crate::image_viewer::LOAD_EXTENSIONS
                    .iter()
                    .flat_map(|e| [e.to_string(), e.to_ascii_uppercase()])
                    .collect();
                let f = D::new()
                    .set_title("Load Image")
                    .set_directory(folder)
                    .add_filter("Pictures (PNG, JPEG, TIFF, OpenEXR, HDR, TGA, BMP)", &types)
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::SavePreset {
                folder,
                name,
                title,
            } => {
                let f = D::new()
                    .set_title(title)
                    .set_directory(folder)
                    .set_file_name(name)
                    .add_filter(
                        "Operandi preset",
                        &[operandi_core::preset::PRESET_EXTENSION],
                    )
                    .save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::SaveEnvironmentPreset { folder, name } => {
                let f = D::new()
                    .set_title("Save Environment Preset")
                    .set_directory(folder)
                    .set_file_name(name)
                    .add_filter(
                        "Operandi environment preset",
                        &[operandi_core::preset::ENVIRONMENT_PRESET_EXTENSION],
                    )
                    .save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::LoadPreset { folder } => {
                let ext = operandi_core::preset::PRESET_EXTENSION;
                let env = operandi_core::preset::ENVIRONMENT_PRESET_EXTENSION;
                let f = D::new()
                    .set_title("Load Preset")
                    .set_directory(folder)
                    .add_filter(
                        "Operandi preset",
                        &[
                            ext,
                            &ext.to_ascii_uppercase(),
                            env,
                            &env.to_ascii_uppercase(),
                        ],
                    )
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::MaterialX { folder } => {
                let f = D::new()
                    .set_title("Import MaterialX")
                    .set_directory(folder)
                    .add_filter("MaterialX", &["mtlx", "MTLX"])
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::SaveMaterialX { folder, name } => {
                let f = D::new()
                    .set_title("Export Material as MaterialX")
                    .set_directory(folder)
                    .set_file_name(name)
                    .add_filter("MaterialX", &["mtlx"])
                    .save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::SaveImage {
                folder,
                name,
                title,
            } => {
                let f = D::new()
                    .set_title(title)
                    .set_directory(folder)
                    .set_file_name(name)
                    .add_filter("PNG", &["png"])
                    .save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::SaveUvLayout { folder, name, eps } => {
                let (label, ext) = if eps { ("EPS", "eps") } else { ("SVG", "svg") };
                let f = D::new()
                    .set_title(format!("Export UVs to {label}"))
                    .set_directory(folder)
                    .set_file_name(name)
                    .add_filter(label, &[ext])
                    .save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::LoadTexture { folder } => {
                let f = D::new()
                    .set_title("Pad Texture to UVs")
                    .set_directory(folder)
                    .add_filter("PNG picture", &["png", "PNG"])
                    .pick_file();
                Box::pin(async move { path(f.await) })
            }
            #[cfg(feature = "painting")]
            Choose::SavePaintImage { folder, name } => {
                use operandi_core::paint::pixels::FileType;
                // The suggested name's type first: macOS offers the first
                // filter's types.
                let first = FileType::of(Path::new(&name)).unwrap_or(FileType::Png);
                let mut d = D::new()
                    .set_title("Save Image")
                    .set_directory(folder)
                    .set_file_name(&name)
                    .add_filter(first.label(), &[first.extension()]);
                for t in FileType::ALL.into_iter().filter(|t| *t != first) {
                    d = d.add_filter(t.label(), &[t.extension()]);
                }
                let f = d.save_file();
                Box::pin(async move { path(f.await) })
            }
            Choose::Folder { folder, title } => {
                let f = D::new()
                    .set_title(title)
                    .set_directory(folder)
                    .set_can_create_directories(true)
                    .pick_folder();
                Box::pin(async move { path(f.await) })
            }
        };
    Pending { purpose, future }
}

/// Split a suggested path into the folder to start in and the file name.
/// A path ending in a separator (or naming an existing folder) is a folder.
pub(super) fn split(suggested: &str) -> (PathBuf, String) {
    let path = Path::new(suggested);
    if suggested.ends_with('/') || path.is_dir() {
        return (path.to_path_buf(), String::new());
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    (folder, name)
}

impl super::Operandi {
    /// Ask for a path. Returns false when there is no panel (the caller shows
    /// its typed-path dialog instead). The answer arrives through `chosen`,
    /// now for a stand-in, or from `poll_choice` for the native sheet.
    pub(super) fn choose(&mut self, request: Choose, purpose: Purpose) -> bool {
        #[cfg(test)]
        if let Some(answer) = &self.held_panel {
            if self.choosing.is_none() {
                self.choosing = Some(Pending::held(purpose, answer.clone()));
            }
            return true;
        }
        if self.native_panels {
            if self.choosing.is_none() {
                self.choosing = Some(start(request, purpose));
            }
            return true;
        }
        let Some(choose) = self.chooser else {
            return false;
        };
        match choose(request) {
            Some(path) => self.chosen(purpose, path),
            None => self.choice_cancelled(purpose),
        }
        true
    }

    /// Called every frame: finish a native panel once it closes.
    pub(super) fn poll_choice(&mut self, ctx: &eframe::egui::Context) {
        let Some(pending) = &mut self.choosing else {
            return;
        };
        match pending.poll() {
            None => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
            Some(answer) => {
                let purpose = pending.purpose;
                self.choosing = None;
                match answer {
                    Some(path) => self.chosen(purpose, path),
                    None => self.choice_cancelled(purpose),
                }
                ctx.request_repaint();
            }
        }
    }

    /// A panel closed with Cancel. Most purposes just drop the request.
    fn choice_cancelled(&mut self, purpose: Purpose) {
        #[cfg(feature = "painting")]
        if let Purpose::PaintImage(..) = purpose {
            self.cancel_image_saves();
        }
        let _ = purpose;
    }

    fn chosen(&mut self, purpose: Purpose, path: PathBuf) {
        use crate::project_files::{REFERENCES, SOURCE_IMAGES};
        match purpose {
            Purpose::SaveAs => {
                if self.save(path)
                    && let Some(action) = self.pending.take()
                {
                    self.do_file(action);
                }
            }
            Purpose::Open => self.open(path),
            Purpose::Import => {
                self.import_into_current(&path, crate::import_into::ImportKinds::default(), None);
            }
            Purpose::Export(format) => {
                self.export_to(path, format);
            }
            Purpose::SetProject => match operandi_core::Project::open(&path) {
                Ok(project) => self.set_project(project),
                Err(error) => self.status = error,
            },
            Purpose::ProjectFolder => self.project_dialog_folder(&path),
            Purpose::BackupDirectory => self.set_backup_dir(path),
            // Files the scene will refer to are first copied into the
            // active project (project_files.rs).
            Purpose::FalloffImage => self.falloff_image_chosen(path),
            Purpose::BackdropImage(item) => {
                let path = self.adopt_file(REFERENCES, &path);
                self.backdrop_image_chosen(item, path)
            }
            Purpose::CameraBackground(item) => {
                let path = self.adopt_file(REFERENCES, &path);
                self.camera_background_chosen(item, path)
            }
            Purpose::ShaderImage(node) => {
                let path = self.adopt_file(SOURCE_IMAGES, &path);
                self.shader_image_chosen(node, path)
            }
            Purpose::EnvironmentImage(item) => {
                let path = self.adopt_file(SOURCE_IMAGES, &path);
                self.environment_image_chosen(Some(item), &path);
            }
            Purpose::PreviewImage => self.preview_save_to(path),
            Purpose::ViewerSave(d) => self.viewer_save_to(&path, d),
            Purpose::ViewerLoad => self.viewer_load(&path),
            Purpose::RecentFramesFolder => self.set_recent_frames_folder(Some(path)),
            Purpose::ImportColorScheme => self.import_color_scheme(&path),
            Purpose::ExportColorScheme => self.export_color_scheme(path),
            Purpose::Photometry(item) => {
                let path = self.adopt_file(SOURCE_IMAGES, &path);
                self.photometry_chosen(item, path)
            }
            Purpose::LightLayerImage(item) => {
                let path = self.adopt_file(SOURCE_IMAGES, &path);
                self.light_layer_chosen(item, path)
            }
            Purpose::MatcapImage => {
                self.display.settings.drawing.matcap = crate::matcap::Choice::custom(&path)
            }
            Purpose::DefaultScene => self.set_default_scene(path),
            Purpose::SavePreset(source, mtlx) => self.save_preset_to(path, source, mtlx),
            Purpose::SaveEnvironmentPreset(options) => {
                self.save_environment_preset_to(path, options);
            }
            Purpose::LoadPreset(group) => self.load_preset_from(path, group, true),
            Purpose::ImportNodeGraph(selected) => self.import_node_graph_from(&path, selected),
            Purpose::RunScript => self.python_run_file(&path),
            Purpose::ExportPie => self.export_pie_chosen(path),
            Purpose::ImportPie => self.import_pie_chosen(path),
            Purpose::FormIcon => self.form_icon_chosen(path),
            Purpose::ImportMaterialX(beside) => self.import_materialx_from(&path, beside),
            Purpose::ExportMaterialX(node) => self.export_materialx_to(path, node),
            Purpose::PresetBrowserRoot => {
                self.status = self.preset_browser.add_root(path, None);
            }
            Purpose::WorkbenchIcon => self.workbench_icon_chosen(&path),
            Purpose::BakeTo => self.bake_to(&path),
            Purpose::PadTexture => self.pad_texture(&path),
            #[cfg(feature = "painting")]
            Purpose::PaintImage(tab, image) => self.paint_image_chosen(tab, image, path),
            Purpose::UvLayout(eps) => self.export_uv_layout(&path, eps),
            Purpose::PresetBrowserThumbnail => {
                if let Some(target) = self.preset_browser.thumb_target.take() {
                    match crate::image_decode::open(&path) {
                        Ok(picture) => self.set_entry_thumbnail(&target, picture),
                        Err(e) => self.status = format!("Can't read {}: {e}", path.display()),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backdrop_chosen_outside_the_project_uses_a_copy_and_undo_keeps_it() {
        use operandi_core::Command;
        let base =
            std::env::temp_dir().join(format!("operandi-chooser-adopt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let picture = base.join("Desktop/ref.png");
        std::fs::create_dir_all(picture.parent().unwrap()).unwrap();
        image::RgbaImage::new(4, 2).save(&picture).unwrap();
        let root = base.join("Proj");
        let mut a = crate::Operandi::empty();
        a.project = Some(operandi_core::Project::create(&root, "Proj").unwrap());
        a.create_backdrop();
        let item = a.editor.scene.selection.item.unwrap();
        let before = a.editor.scene.images.len();
        a.chosen(Purpose::BackdropImage(item), picture.clone());
        let copy = root.join("references").join("ref.png");
        assert!(copy.is_file() && picture.is_file());
        assert!(
            a.editor
                .scene
                .images
                .iter()
                .any(|i| i.path == copy.to_string_lossy())
        );
        // Undo takes back the scene change only; the copy stays.
        a.run(Command::Undo);
        assert_eq!(a.editor.scene.images.len(), before);
        assert!(copy.is_file());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn split_separates_folder_and_name() {
        assert_eq!(
            split("/p/scenes/Untitled.opscene"),
            (PathBuf::from("/p/scenes"), "Untitled.opscene".into())
        );
        assert_eq!(
            split("/p/scenes/"),
            (PathBuf::from("/p/scenes/"), String::new())
        );
    }

    #[test]
    fn split_handles_relative_empty_and_hidden_names() {
        assert_eq!(split(""), (PathBuf::new(), String::new()));
        assert_eq!(
            split("scene.opscene"),
            (PathBuf::new(), "scene.opscene".into())
        );
        assert_eq!(
            split("scenes/.draft"),
            (PathBuf::from("scenes"), ".draft".into())
        );
        assert_eq!(
            split("scenes/new/"),
            (PathBuf::from("scenes/new/"), String::new())
        );
    }

    #[test]
    fn split_distinguishes_existing_folders_from_files() {
        let base = std::env::temp_dir().join(format!(
            "operandi-file-chooser-split-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&base).unwrap();
        let file = base.join("scene.opscene");
        std::fs::write(&file, b"").unwrap();
        assert_eq!(split(base.to_str().unwrap()), (base.clone(), String::new()));
        assert_eq!(
            split(file.to_str().unwrap()),
            (base.clone(), "scene.opscene".into())
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn readable_filters_include_both_cases_and_scene_only_for_open() {
        let imports = readable(false);
        let open = readable(true);
        assert_eq!(&imports[..2], ["lxo", "LXO"]);
        assert_eq!(&open[..4], ["lxo", "LXO", "opscene", "OPSCENE"]);
        assert_eq!(&open[4..], &imports[2..]);
        // Built-in types first; a kit's (kit_commands_tests) may follow.
        assert!(imports.len() >= 2 + 2 * operandi_core::interchange::IMPORT_EXTENSIONS.len());
        for (pair, extension) in imports[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .zip(operandi_core::interchange::IMPORT_EXTENSIONS)
        {
            assert_eq!(
                pair.as_slice(),
                [extension, &extension.to_ascii_uppercase()]
            );
        }
        assert!(
            !imports
                .iter()
                .any(|extension| extension.eq_ignore_ascii_case("opscene"))
        );
    }

    #[test]
    fn pending_poll_distinguishes_waiting_cancel_and_selected_path() {
        let mut waiting = Pending {
            purpose: Purpose::Open,
            future: Box::pin(std::future::pending()),
        };
        assert_eq!(waiting.poll(), None);
        let mut cancelled = Pending {
            purpose: Purpose::Open,
            future: Box::pin(std::future::ready(None)),
        };
        assert_eq!(cancelled.poll(), Some(None));
        let path = PathBuf::from("scene.opscene");
        let mut selected = Pending {
            purpose: Purpose::Open,
            future: Box::pin(std::future::ready(Some(path.clone()))),
        };
        assert_eq!(selected.poll(), Some(Some(path)));
    }
}
