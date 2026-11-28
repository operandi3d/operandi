//! App-lifetime launch inputs and paths. Resolution is pure; storage policy stays
//! with each existing loader/writer. Document destinations are never config paths.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub(super) struct AppEnvironment {
    paths: AppPaths,
    title: String,
    // Keep native paths as OS strings, including non-Unicode file names.
    arguments: Vec<OsString>,
    /// Tests: the throwaway data folder `Operandi::empty` made, removed when
    /// the last app (or copied environment) using it goes away. They used
    /// to pile up by the hundred thousand (test speed, 2026-11-16).
    #[cfg(test)]
    pub(super) scratch: Option<std::sync::Arc<ScratchRoot>>,
}

/// A test data folder, removed on drop.
#[cfg(test)]
#[derive(Debug)]
pub(super) struct ScratchRoot(pub PathBuf);
#[cfg(test)]
impl Drop for ScratchRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
impl AppEnvironment {
    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn capture() -> Self {
        let data = std::env::var_os("OPERANDI_DEV_DATA");
        let title = std::env::var_os("OPERANDI_DEV_TITLE");
        let arguments = std::env::args_os().collect();
        // Normal development builds retain their established workspace paths.
        // Packaged builds must not refer to the machine that compiled them.
        // A development build run on another machine (a shared QA app: the
        // compile machine's source folder isn't there) uses the packaged
        // paths too, and so does one whose source folder can't be written
        // (contributor report 2026-11-19: "Permission denied" on a path
        // from the build machine).
        let source_here = data.is_some() || dev_source_usable(env!("CARGO_MANIFEST_DIR"));
        if option_env!("OPERANDI_MILESTONE1_PACKAGE") == Some("1") || !source_here {
            let windows = cfg!(target_os = "windows");
            let home = std::env::var_os(if windows { "USERPROFILE" } else { "HOME" })
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let local_data = std::env::var_os("LOCALAPPDATA")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from);
            let has_documents = home.join("Documents").is_dir();
            let mut env = Self::from_packaged_inputs(
                data,
                title,
                arguments,
                home,
                local_data,
                windows,
                has_documents,
            );
            env.paths.library = default_library();
            return env;
        }
        let mut env = Self::from_inputs(data, title, arguments, env!("CARGO_MANIFEST_DIR"));
        // A real launch reads the shared library folder (QA apps too, so an
        // installed pack shows up); tests keep theirs in their data folder.
        env.paths.library = default_library();
        env
    }
    fn from_packaged_inputs(
        data_override: Option<OsString>,
        title: Option<OsString>,
        arguments: Vec<OsString>,
        home: PathBuf,
        local_data: Option<PathBuf>,
        windows: bool,
        has_documents: bool,
    ) -> Self {
        Self {
            paths: AppPaths::packaged(data_override, home, local_data, windows, has_documents),
            title: title
                .and_then(|s| s.into_string().ok())
                .unwrap_or_else(|| "Operandi-milestone1".into()),
            arguments,
            #[cfg(test)]
            scratch: None,
        }
    }
    pub fn from_inputs(
        data_override: Option<OsString>,
        title: Option<OsString>,
        arguments: Vec<OsString>,
        manifest_dir: &str,
    ) -> Self {
        Self {
            paths: AppPaths::resolve(data_override, manifest_dir),
            title: title
                .and_then(|s| s.into_string().ok())
                .unwrap_or_else(|| "Operandi Rust Dev".into()),
            arguments,
            #[cfg(test)]
            scratch: None,
        }
    }
    pub fn document_arguments(&self) -> Vec<PathBuf> {
        crate::open_documents::from_args(self.arguments.iter().skip(1).cloned())
    }

    /// The QA fixture named after the program, kept as the OS gave it: a name
    /// that isn't valid Unicode opens like any other (Astra P8-007).
    #[cfg(any(feature = "qa", test))]
    pub fn document_argument(&self) -> Option<PathBuf> {
        self.arguments.get(1).map(PathBuf::from)
    }
}

/// Whether a development build's source folder (its compile-time
/// manifest folder) exists on this machine and its `development` folder
/// can be written, so the workspace data paths work here.
fn dev_source_usable(manifest_dir: &str) -> bool {
    let manifest = std::path::Path::new(manifest_dir);
    let Some(root) = manifest.parent().and_then(|p| p.parent()) else {
        return false;
    };
    if !manifest.is_dir() {
        return false;
    }
    let development = root.join("development");
    std::fs::create_dir_all(&development).is_ok()
        && std::fs::metadata(&development).is_ok_and(|m| !m.permissions().readonly())
}

/// The default library folder: `OPERANDI_LIBRARY_FOLDER` when set, else
/// `Operandi/Library` in the platform's data folder (macOS
/// ~/Library/Application Support, Windows %APPDATA%, Linux
/// $XDG_DATA_HOME or ~/.local/share).
fn default_library() -> PathBuf {
    if let Some(p) = std::env::var_os("OPERANDI_LIBRARY_FOLDER").filter(|v| !v.is_empty()) {
        return PathBuf::from(p);
    }
    let var = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let home = var(if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    })
    .unwrap_or_else(|| PathBuf::from("."));
    let base = if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support")
    } else if cfg!(target_os = "windows") {
        var("APPDATA").unwrap_or_else(|| home.join("AppData").join("Roaming"))
    } else {
        var("XDG_DATA_HOME").unwrap_or_else(|| home.join(".local").join("share"))
    };
    base.join("Operandi").join("Library")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AppPaths {
    data_root: PathBuf,
    /// Where the Operandi Library pack is unzipped (F1).
    library: PathBuf,
    open_dialog: String,
    save_dialog: String,
}
impl AppPaths {
    fn packaged(
        data_override: Option<OsString>,
        home: PathBuf,
        local_data: Option<PathBuf>,
        windows: bool,
        has_documents: bool,
    ) -> Self {
        let data_base = if windows {
            local_data.unwrap_or_else(|| home.join("AppData").join("Local"))
        } else {
            home.join("Library").join("Application Support")
        };
        let document_dir = if has_documents {
            home.join("Documents")
        } else {
            home
        };
        let mut open_dialog = document_dir.to_string_lossy().into_owned();
        if !open_dialog.ends_with(std::path::MAIN_SEPARATOR) {
            open_dialog.push(std::path::MAIN_SEPARATOR);
        }
        let data_root = data_override
            .map(PathBuf::from)
            .unwrap_or_else(|| data_base.join("Operandi-milestone1"));
        Self {
            library: data_root.join("Library"),
            data_root,
            open_dialog,
            save_dialog: document_dir
                .join("Untitled.opscene")
                .to_string_lossy()
                .into_owned(),
        }
    }
    pub fn data_root(&self) -> &Path {
        &self.data_root
    }
    /// Where preset files go by default (PB-Q2): "Presets" in the data
    /// folder. Made on the first preset save.
    pub fn presets(&self) -> PathBuf {
        self.data_root.join("Presets")
    }
    /// The default library folder (F1): where the Operandi Library pack
    /// is unzipped. The preset browser shows every pack in it.
    pub fn library(&self) -> &Path {
        &self.library
    }
    /// The built-in Basics presets, written by the app (read-only root).
    pub fn basics(&self) -> PathBuf {
        self.data_root.join("Cache").join("Basics")
    }
    pub fn open_dialog(&self) -> &str {
        &self.open_dialog
    }
    pub fn save_dialog(&self) -> &str {
        &self.save_dialog
    }
    fn resolve(data_override: Option<OsString>, manifest_dir: &str) -> Self {
        let document_root = manifest_dir.replace("/crates/operandi-app", "");
        let data_root = data_override.map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(format!("{manifest_dir}/../../development/milestone-1.1"))
        });
        Self {
            library: data_root.join("Library"),
            data_root,
            open_dialog: format!("{document_root}/development/"),
            save_dialog: format!("{document_root}/development/Untitled.opscene"),
        }
    }
    /// The Kits folder: one folder per installed kit (docs/KITS.md).
    pub fn kits(&self) -> PathBuf {
        self.data_root.join("Kits")
    }
    /// Which kits are on.
    pub fn kit_state(&self) -> PathBuf {
        self.data_root.join("kits.toml")
    }
    pub fn recovery(&self) -> PathBuf {
        self.data_root.join("recovery.opscene")
    }
    pub fn input_map(&self) -> PathBuf {
        self.data_root.join("input-map.json")
    }
    /// Mouse navigation preset and other Preferences-window input options.
    pub fn input_preferences(&self) -> PathBuf {
        self.data_root.join("input-preferences.json")
    }
    /// Which side panels float in their own windows (floating.rs).
    pub fn panel_layout(&self) -> PathBuf {
        self.data_root.join("panel-layout.json")
    }
    /// The view area's arrangement before layouts stage 1: read once by
    /// the migration into the Model layout (layouts.rs).
    pub fn view_layout(&self) -> PathBuf {
        self.data_root.join("view-layout.json")
    }
    /// Each view's display settings before layouts stage 1: read once by
    /// the migration (layouts.rs).
    pub fn view_displays(&self) -> PathBuf {
        self.data_root.join("view-displays.json")
    }
    /// Viewport Properties' One Tab / All Tabs presets and the new-view
    /// default (view_tab_presets.rs).
    pub fn viewport_tab_presets(&self) -> PathBuf {
        self.data_root.join("viewport-tab-presets.json")
    }
    /// Viewport Properties: which sections the user folded.
    pub fn viewport_properties(&self) -> PathBuf {
        self.data_root.join("viewport-properties.json")
    }
    /// The colours the user changed in Preferences > Display > Colors.
    pub fn color_theme(&self) -> PathBuf {
        self.data_root.join("color-theme.json")
    }
    /// Saved colour schemes, one JSON file each.
    pub fn color_schemes(&self) -> PathBuf {
        self.data_root.join("color-schemes")
    }
    /// Tool defaults, overlays and file info (defaults_preferences.rs).
    pub fn defaults_preferences(&self) -> PathBuf {
        self.data_root.join("defaults-preferences.json")
    }
    /// Units and grid preferences (units_preferences.rs).
    pub fn units_preferences(&self) -> PathBuf {
        self.data_root.join("units-preferences.json")
    }
    /// Tool handle preferences (handle_preferences.rs).
    /// File I/O (file_io_preferences.rs).
    pub fn file_io_preferences(&self) -> PathBuf {
        self.data_root.join("file-io-preferences.json")
    }
    /// Saved camera Film Back presets (camera_props_ui.rs).
    pub fn film_back_presets(&self) -> PathBuf {
        self.data_root.join("film-back-presets.json")
    }
    /// Saved values behind numeric fields' preset arrows (value_presets.rs).
    pub fn value_presets(&self) -> PathBuf {
        self.data_root.join("value-presets.json")
    }
    /// Rendering (render_preferences.rs).
    pub fn render_preferences(&self) -> PathBuf {
        self.data_root.join("render-preferences.json")
    }
    /// Rendering > Render Window (image_viewer.rs).
    pub fn image_viewer_preferences(&self) -> PathBuf {
        self.data_root.join("image-viewer-preferences.json")
    }
    /// The Render window's place, panels and display choices (UIC-73).
    pub fn render_window(&self) -> PathBuf {
        self.data_root.join("render-window.json")
    }
    /// The default Recent Frames Folder: the render history cache (IV-12).
    pub fn recent_frames(&self) -> PathBuf {
        self.data_root.join("Recent Frames")
    }
    /// Input > 3D Mouse (space_mouse/prefs.rs).
    pub fn space_mouse_preferences(&self) -> PathBuf {
        self.data_root.join("space-mouse-preferences.json")
    }
    /// Defaults > UI (ui_preferences.rs).
    pub fn ui_preferences(&self) -> PathBuf {
        self.data_root.join("ui-preferences.json")
    }
    /// Defaults > Mesh Items (mesh_items_preferences.rs).
    pub fn mesh_items_preferences(&self) -> PathBuf {
        self.data_root.join("mesh-items-preferences.json")
    }
    pub fn handle_preferences(&self) -> PathBuf {
        self.data_root.join("handle-preferences.json")
    }
    /// Physical Sun places stored with Store Location (light_tier3_ui.rs).
    pub fn sun_locations(&self) -> PathBuf {
        self.data_root.join("sun-locations.json")
    }
    /// Subdivide and UnSubdivide's last-used options (subdivide_ui.rs).
    pub fn subdivide_settings(&self) -> PathBuf {
        self.data_root.join("subdivide.json")
    }
    /// Every sculpt brush's settings and Lock Borders/Corners (sculpt_ui.rs,
    /// SC-95). User prefs, not scene data (SC-94).
    pub fn sculpt_settings(&self) -> PathBuf {
        self.data_root.join("sculpt.json")
    }
    /// The top-bar Drop Action choice (drop_action_ui.rs).
    pub fn drop_action(&self) -> PathBuf {
        self.data_root.join("drop-action.json")
    }
    /// Items list Add Item Recent (item_list_ui.rs).
    pub fn item_list(&self) -> PathBuf {
        self.data_root.join("item-list.json")
    }
    /// Snapping switch, targets and ranges (snapping.rs).
    pub fn snapping_preferences(&self) -> PathBuf {
        self.data_root.join("snapping-preferences.json")
    }
    /// Mesh Constraint settings and pinned popover (background_ui.rs).
    pub fn mesh_constraint_preferences(&self) -> PathBuf {
        self.data_root.join("mesh-constraint-preferences.json")
    }
    /// Vertex Normal Tools options (vertex_normal_ui.rs, G9 VN-6).
    pub fn vertex_normal_preferences(&self) -> PathBuf {
        self.data_root.join("vertex-normal-preferences.json")
    }
    /// The Statistics panel's open groups (statistics_ui.rs).
    pub fn statistics_panel(&self) -> PathBuf {
        self.data_root.join("statistics-panel.json")
    }
    /// Preview window place and Preview Settings (preview.rs).
    pub fn preview_window(&self) -> PathBuf {
        self.data_root.join("preview-window.json")
    }
    /// Selection preferences (selection_preferences.rs).
    pub fn selection_preferences(&self) -> PathBuf {
        self.data_root.join("selection-preferences.json")
    }
    /// Undo limits and auto-save settings (undo_autosave.rs).
    pub fn undo_autosave(&self) -> PathBuf {
        self.data_root.join("undo-autosave.json")
    }
    /// Online libraries: which sources are on, resolution, refresh
    /// (online_library.rs).
    pub fn online_preferences(&self) -> PathBuf {
        self.data_root.join("online-library-preferences.json")
    }
    /// Every online library's cache and browser root (A34).
    pub fn online_libraries(&self) -> PathBuf {
        self.data_root.join("Online Libraries")
    }
    /// Update channel, launch check and skipped build (update_check.rs).
    pub fn update_preferences(&self) -> PathBuf {
        self.data_root.join("update-preferences.json")
    }
    /// View > Layout's Lock Thumb / Divider Dragging (panel_dot.rs).
    pub fn layout_locks(&self) -> PathBuf {
        self.data_root.join("layout-locks.json")
    }
    pub fn viewport_display(&self) -> PathBuf {
        self.data_root.join("viewport-display.json")
    }
    /// Saved viewport display presets (view_display.rs).
    pub fn viewport_presets(&self) -> PathBuf {
        self.data_root.join("viewport-presets.json")
    }
    pub fn bevel_recipes(&self) -> PathBuf {
        self.data_root.join("bevel-recipes.json")
    }
    pub fn profile_browser(&self) -> PathBuf {
        self.data_root.join("profile-browser.json")
    }
    /// Last and recent projects (per user, not per project).
    pub fn projects(&self) -> PathBuf {
        self.data_root.join("projects.json")
    }
    pub fn layout(&self) -> PathBuf {
        self.data_root.join("layout.json")
    }
    /// Crash files and the crash prompt's answers (bug_report.rs).
    pub fn crashes(&self) -> PathBuf {
        self.data_root.join(crate::bug_report::CRASH_FOLDER)
    }
    /// Report Bug's window screenshots (bug_report_capture.rs).
    pub fn reports(&self) -> PathBuf {
        self.data_root
            .join(crate::bug_report_capture::REPORTS_FOLDER)
    }
    /// One-time tips already shown (report_bug_dialog.rs, BUG-24).
    pub fn tips(&self) -> PathBuf {
        self.data_root.join("tips.json")
    }
    /// Main window size, place and maximised/fullscreen state (window_state.rs).
    pub fn main_window(&self) -> PathBuf {
        self.data_root.join("main-window.json")
    }
    /// Each dialog's last OK'd values (form/, UIC-95).
    pub fn dialog_values(&self) -> PathBuf {
        self.data_root.join(crate::form::MEMORY_FILE)
    }
    pub fn runtime_viewport(&self) -> PathBuf {
        self.data_root.join("runtime-viewport.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const MANIFEST: &str = "/original/worktree/crates/operandi-app";

    /// Contributor report, 2026-11-19: a development build run on another
    /// machine tried the compile machine's source folder ("Permission
    /// denied"). Its workspace paths are used only where that folder is.
    #[test]
    fn dev_paths_only_where_the_build_machines_source_folder_is() {
        assert!(!dev_source_usable(
            "/Users/someone-else/Operandi/crates/operandi-app"
        ));
        assert!(!dev_source_usable(MANIFEST));
        let root = std::env::temp_dir().join(format!("operandi-dev-src-{}", std::process::id()));
        let manifest = root.join("crates/operandi-app");
        std::fs::create_dir_all(&manifest).unwrap();
        assert!(dev_source_usable(manifest.to_str().unwrap()));
        assert!(root.join("development").is_dir());
        std::fs::remove_dir_all(&root).unwrap();
    }
    fn context(data: Option<OsString>) -> AppEnvironment {
        AppEnvironment::from_inputs(data, None, vec!["app".into()], MANIFEST)
    }
    #[test]
    fn packaged_paths_follow_user_and_preserve_overrides() {
        let home = PathBuf::from("/user-home");
        let mac = AppEnvironment::from_packaged_inputs(
            None,
            None,
            vec![],
            home.clone(),
            None,
            false,
            true,
        );
        assert_eq!(
            mac.paths.data_root,
            home.join("Library/Application Support/Operandi-milestone1")
        );
        assert_eq!(
            mac.paths.save_dialog,
            home.join("Documents")
                .join("Untitled.opscene")
                .to_string_lossy()
        );
        assert_eq!(mac.title, "Operandi-milestone1");
        let local = PathBuf::from("/local-data");
        let win = AppEnvironment::from_packaged_inputs(
            None,
            None,
            vec![],
            home.clone(),
            Some(local.clone()),
            true,
            false,
        );
        assert_eq!(win.paths.data_root, local.join("Operandi-milestone1"));
        assert_eq!(
            win.paths.save_dialog,
            home.join("Untitled.opscene").to_string_lossy()
        );
        let override_app = AppEnvironment::from_packaged_inputs(
            Some("custom".into()),
            Some("Custom".into()),
            vec![],
            home,
            None,
            true,
            true,
        );
        assert_eq!(override_app.paths.data_root, PathBuf::from("custom"));
        assert_eq!(override_app.title, "Custom");
    }
    #[test]
    fn packaged_capture_uses_runtime_overrides_and_user_documents() {
        if option_env!("OPERANDI_MILESTONE1_PACKAGE") != Some("1") {
            return;
        }
        let profile = std::env::var_os("OPERANDI_DEV_DATA").expect("isolated test profile");
        let title = std::env::var_os("OPERANDI_DEV_TITLE").expect("isolated test title");
        let app = AppEnvironment::capture();
        assert_eq!(app.paths.data_root, PathBuf::from(profile));
        assert_eq!(app.title, title.to_string_lossy());
        assert!(!app.paths.open_dialog.contains(env!("CARGO_MANIFEST_DIR")));
        assert!(!app.paths.save_dialog.contains(env!("CARGO_MANIFEST_DIR")));
    }
    #[test]
    fn e07_01_resolution_preserves_fallback_and_override_spelling() {
        assert_eq!(
            context(None).paths.data_root,
            PathBuf::from("/original/worktree/crates/operandi-app/../../development/milestone-1.1")
        );
        for value in ["", "relative/../settings", "/absolute/settings"] {
            let paths = context(Some(value.into())).paths;
            assert_eq!(paths.data_root, PathBuf::from(value));
            assert_eq!(paths.open_dialog, "/original/worktree/development/");
            assert_eq!(
                paths.save_dialog,
                "/original/worktree/development/Untitled.opscene"
            );
        }
    }
    #[test]
    fn e07_01_contexts_keep_separate_resource_and_document_paths() {
        let a = context(Some("first".into()));
        let b = context(Some("second".into()));
        let snapshot = a.clone();
        assert_eq!(a.paths, snapshot.paths);
        assert_eq!(a.paths.open_dialog, b.paths.open_dialog);
        assert_eq!(a.paths.save_dialog, b.paths.save_dialog);
        let resources = |p: &AppPaths| {
            [
                p.recovery(),
                p.input_map(),
                p.viewport_display(),
                p.bevel_recipes(),
                p.profile_browser(),
                p.layout(),
                p.runtime_viewport(),
            ]
        };
        for (a, b) in resources(&a.paths).iter().zip(resources(&b.paths)) {
            assert!(a.starts_with("first"));
            assert!(b.starts_with("second"));
            assert_eq!(a.file_name(), b.file_name());
        }
    }
    #[test]
    fn e07_01_launch_title_and_argument_are_explicit() {
        let app = AppEnvironment::from_inputs(
            None,
            Some("Title".into()),
            vec![
                "app".into(),
                "relative/native.opscene".into(),
                "ignored".into(),
            ],
            MANIFEST,
        );
        assert_eq!(app.title, "Title");
        assert_eq!(
            app.document_argument(),
            Some("relative/native.opscene".into())
        );
        assert_eq!(context(None).title, "Operandi Rust Dev");
        assert_eq!(context(None).document_argument(), None);
        assert_eq!(
            AppEnvironment::from_inputs(None, Some("".into()), vec![], MANIFEST).title,
            ""
        );
    }
    #[cfg(unix)]
    #[test]
    fn e07_01_non_unicode_data_survives_and_title_falls_back() {
        use std::os::unix::ffi::OsStringExt;
        let raw = OsString::from_vec(vec![b'r', 0xff]);
        let app =
            AppEnvironment::from_inputs(Some(raw.clone()), Some(raw.clone()), vec![], MANIFEST);
        assert_eq!(app.paths.data_root, PathBuf::from(raw));
        assert_eq!(app.title, "Operandi Rust Dev");
    }
    /// Astra P8-007: a file name that isn't valid Unicode used to crash
    /// the launch; it now names the file to open, byte for byte.
    #[cfg(unix)]
    #[test]
    fn e07_01_non_unicode_argument_opens_as_given() {
        use std::os::unix::ffi::OsStringExt;
        let name = OsString::from_vec(vec![b's', 0xff, b'.', b'o', b'b', b'j']);
        let app =
            AppEnvironment::from_inputs(None, None, vec!["app".into(), name.clone()], MANIFEST);
        assert_eq!(app.title, "Operandi Rust Dev");
        assert_eq!(app.document_argument(), Some(PathBuf::from(name.clone())));
        let words = crate::command_line_words(vec![name, "--render-matcap".into()].into_iter());
        assert_eq!(words[1], "--render-matcap");
        assert!(words[0].starts_with('s'));
    }
    #[test]
    fn e07_02_app_context_survives_new_and_failed_open() {
        let mut app = crate::Operandi::empty();
        let paths = app.environment.paths().clone();
        app.do_file(crate::FileAction::New);
        assert_eq!(app.environment.paths(), &paths);
        app.open(paths.data_root().join("absent.opscene"));
        assert_eq!(app.environment.paths(), &paths);
        assert!(app.path.is_none());
        std::fs::remove_dir_all(paths.data_root()).unwrap();
    }
}
