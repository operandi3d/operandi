//! The preset browser, Stage 1 (BEHAVIOR-PRESET-BROWSER-STAGE1 PB-100..122,
//! CONTENT-DIRECTORIES CD-01..27): the panel's state, what loading, saving
//! and the file actions do, and its commands. Drawing is in
//! `preset_browser_ui.rs`, thumbnails in `preset_thumbs.rs`, and what a
//! folder holds in `operandi_core::preset::library`.
//!
//! Every route runs the same operations: double-click, Enter, drag, the
//! right-click menus and the commands (UIC-160). Loads go through
//! `Command::LoadPreset`, `ApplyMaterialPreset` or `SetEnvironmentImage`,
//! one undo step each (PB-35). Roots and view settings live in the data
//! folder's `preset-browser.json` (CD-03), never in scenes. Nothing is
//! written inside a content folder except by Save, Rename, Duplicate or
//! Delete (CD-T5).
//!
//! Command names come from the registry thread (2026-11-11): standard map
//! names (`pbview.*`, `file.*`, `layout.window`, `preset.do`) and the
//! Operandi-only `operandi.pbview.thumbnailSize`. The library pack (F1)
//! adds the Operandi-only `operandi.pbview.libraryGet`; locating a library
//! is `pbview.basePathAdd` (registry ruling, 2026-11-11).
//!
//! The library pack (F1, 2026-11-11): every root whose folder holds a
//! `manifest.json` (core `preset::pack`) shows the pack's name and version,
//! and its entries' licence, credit and source in their tooltips. Packs in
//! the default library folder become roots once by themselves. The
//! built-in Basics (core `preset::basics`) are a read-only root of their
//! own after the user's roots. Nothing here downloads anything.
use super::*;
use crate::preset_thumbs::{Job, Thumbs};
use operandi_core::preset::library::{
    self, Bucket, Entry, EntryKind, ListOptions, Listing, ResolutionChoice, SortBy,
};
use operandi_core::preset::pack::{self, Manifest};
use operandi_core::preset::{ApplyMode, PRESET_EXTENSION, PresetKind, PresetSource};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;

/// The `hash:` of the browser's window in `layout.window`.
pub(crate) const WINDOW_HASH: &str = "presetBrowser";
/// The Input Map action on F6 (PB-101). It once opened a popover; it now
/// toggles the floating window. The name stays: saved Input Maps use it.
pub(crate) const POPOVER_ACTION: &str = "Preset Browser Popover";
/// The Input Map action and Window menu entry that opens its window.
pub(crate) const WINDOW_ACTION: &str = "Preset Browser";
/// Thumbnail size slider range (PB-22), in points.
pub(crate) const THUMB_MIN: f32 = 48.;
pub(crate) const THUMB_MAX: f32 = 192.;
/// How often the shown folder and the roots are looked at for changes
/// (PB-107, CD-01). Provisional (PB1-Q01): polling, not an OS watcher.
const CHECK_SECONDS: f64 = 1.5;
/// Where "Get the free Operandi Library…" sends people (F1).
pub(crate) const LIBRARY_URL: &str = "https://operandi3d.org/library";
/// Why the Basics root's file actions are greyed (UIC-117).
pub(crate) const READ_ONLY_NOTE: &str = "Basics are built into Operandi and read-only";

/// A top-level folder in the tree (PB-12, CD-01).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Root {
    pub path: PathBuf,
    /// CD-02 Rename: a display label only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl Root {
    pub(crate) fn name(&self) -> String {
        self.label
            .clone()
            .unwrap_or_else(|| folder_name(&self.path))
    }
}

/// The save dialog's Centre on Ground row (PG-04).
pub(crate) const CENTER_ON_GROUND: &str = "centerOnGround";

/// What an empty Meshes folder says (PG-01).
pub(crate) const MESHES_HINT: &str = "Drag a mesh here from a 3D view or the Item List to save it.";

/// A folder named "Meshes" (any case): its empty state shows
/// [`MESHES_HINT`] (PG-01).
pub(crate) fn is_meshes_folder(folder: &Path) -> bool {
    folder
        .file_name()
        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("Meshes"))
}

/// Whether dropping `entry` on geometry applies a material (PB-110): a
/// material preset or a texture set.
pub(crate) fn drops_material(entry: &Entry) -> bool {
    matches!(
        entry.kind,
        EntryKind::Preset(PresetKind::Material) | EntryKind::TextureSet | EntryKind::ModoMaterial
    )
}

/// What a Modo material preset's conversion says about it, for its tile
/// (LXP-02 info dot, LXP-05 "partial" badge) and tooltip.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LxpInfo {
    pub partial: bool,
    pub changed_look: bool,
    pub notes: Vec<String>,
}

pub(crate) fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The browser's view modes (PB-104, footer change from Jason 2026-11-11).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ViewMode {
    /// BQ-02 default: tiles with the name above the picture and the kind
    /// below, wrapping across and scrolling down. Files saved before the
    /// Horizontal view said "thumbnails".
    #[default]
    #[serde(alias = "thumbnails")]
    Vertical,
    /// Wider tiles: the picture on the left, name and kind beside it.
    Horizontal,
    List,
}

impl ViewMode {
    /// The footer's order, left to right.
    pub(crate) const ALL: [ViewMode; 3] =
        [ViewMode::Vertical, ViewMode::Horizontal, ViewMode::List];
    pub(crate) fn label(self) -> &'static str {
        match self {
            ViewMode::Vertical => "Vertical",
            ViewMode::Horizontal => "Horizontal",
            ViewMode::List => "List",
        }
    }
    /// `pbview.mode`'s name for the mode (what a `?` query returns).
    pub(crate) fn script_name(self) -> &'static str {
        match self {
            ViewMode::Vertical => "vertical",
            ViewMode::Horizontal => "horizontal",
            ViewMode::List => "list",
        }
    }
    /// Reads `pbview.mode`'s mode:. The integers are FIXED FOREVER:
    /// scripts and saved command lines store them, so never renumber them
    /// and only add new modes at new numbers. 0 is Vertical (it was
    /// "thumbnails" before Horizontal existed), 1 List, 2 Horizontal.
    pub(crate) fn from_script(text: &str) -> Option<Self> {
        Some(match text.to_ascii_lowercase().as_str() {
            "0" | "thumbnails" | "vertical" => ViewMode::Vertical,
            "1" | "list" => ViewMode::List,
            "2" | "horizontal" => ViewMode::Horizontal,
            _ => return None,
        })
    }
    /// Shows pictures (and the size slider).
    pub(crate) fn tiles(self) -> bool {
        self != ViewMode::List
    }
}

/// What `preset-browser.json` holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    version: u32,
    roots: Vec<Root>,
    folder: Option<PathBuf>,
    expanded: Vec<PathBuf>,
    view: ViewMode,
    sort: SortBy,
    thumb_size: f32,
    only_usable: bool,
    resolution: ResolutionChoice,
    /// MX-05: saving a material preset also writes a .mtlx beside it.
    write_materialx: bool,
    window_open: bool,
    window_pos: Option<[f32; 2]>,
    window_size: [f32; 2],
    /// The Presets folder was offered once as the first root (PB-Q2); a
    /// user who removes it doesn't get it back.
    default_root_added: bool,
    /// Packs in the library folder already added as roots once (F1), so a
    /// pack the user removed doesn't come back. Since MC-01, any folder put
    /// in the library folder (a Modo content folder, say) is one too.
    known_packs: Vec<PathBuf>,
    /// MC-20: the kind filter.
    kind_filter: Bucket,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            roots: Vec::new(),
            folder: None,
            expanded: Vec::new(),
            view: ViewMode::default(),
            sort: SortBy::default(),
            thumb_size: 96.,
            only_usable: true,
            resolution: ResolutionChoice::default(),
            write_materialx: false,
            window_open: false,
            window_pos: None,
            // LAY-51: the Preset Browser palette opens at 900 x 600.
            window_size: [900., 600.],
            default_root_added: false,
            known_packs: Vec::new(),
            kind_filter: Bucket::All,
        }
    }
}

/// A root's pack manifest, re-read when the file changes.
#[derive(Clone, Debug)]
pub(crate) struct PackInfo {
    stamp: Option<std::time::SystemTime>,
    pub manifest: Option<Arc<Manifest>>,
}

/// A folder being read in the background (CD-04).
struct Loader {
    folder: PathBuf,
    rx: Receiver<std::io::Result<Listing>>,
    stop: Arc<AtomicBool>,
    /// A search across the folder and everything below it.
    deep: bool,
}

/// A question the browser is asking (PB-112, PB-121, PB-122, PB-13).
pub(crate) enum Ask {
    RemoveRoot(PathBuf),
    Delete(PathBuf),
    /// MC-02: Copy Into Library found a folder of that name there.
    CopyOverwrite(PathBuf),
    /// A name for a new file: what to save, into which folder.
    SaveName {
        folder: PathBuf,
        source: PresetSource,
        form: form::Dialog,
    },
    /// The chosen name is taken (PB-122).
    Clash {
        path: PathBuf,
        source: PresetSource,
    },
    Rename {
        path: PathBuf,
        form: form::Dialog,
    },
    Duplicate {
        path: PathBuf,
        form: form::Dialog,
    },
    RenameRoot {
        path: PathBuf,
        form: form::Dialog,
    },
    NewFolder {
        parent: PathBuf,
        form: form::Dialog,
    },
}

/// A folder dropped on the browser from the Finder or Explorer (MC-02),
/// waiting for Add as Location / Copy Into Library / Cancel.
#[derive(Clone, Debug)]
pub(crate) struct FolderDrop {
    pub folder: PathBuf,
    pub at: Pos2,
    /// In the browser's own OS window rather than the main one.
    pub in_window: bool,
}

/// A folder being copied into the library folder (MC-02, `dircache.copy`).
pub(crate) struct CopyJob {
    pub name: String,
    rx: Receiver<std::result::Result<usize, String>>,
}

/// An entry being dragged out of the browser (PB-110).
#[derive(Clone, Debug)]
pub(crate) struct Drag {
    pub entry: Entry,
}

/// What a material dragged out of the browser would land on (PB-110): the
/// polygons of the mesh under the pointer that share the hit polygon's
/// material tag. Drawn as a highlight only; the scene doesn't change until
/// the drop.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DropHover {
    pub item: u64,
    pub tag: String,
    pub faces: Vec<usize>,
}

/// What a drop of items on a browser folder can save (PB-120).
#[derive(Clone, Debug)]
pub(crate) struct SaveMenu {
    pub folder: PathBuf,
    pub at: Pos2,
    pub choices: Vec<PresetSource>,
    /// In the browser's own OS window rather than the main one.
    pub in_window: bool,
}

pub(crate) struct PresetBrowser {
    pub roots: Vec<Root>,
    /// The folder shown on the right (PB-11).
    pub folder: Option<PathBuf>,
    pub expanded: BTreeSet<PathBuf>,
    pub view: ViewMode,
    pub sort: SortBy,
    pub thumb_size: f32,
    pub only_usable: bool,
    pub resolution: ResolutionChoice,
    /// MX-05 (gear option): saving a material preset also writes the
    /// material as MaterialX beside it.
    pub write_materialx: bool,
    /// The browser's own OS window (PB-100).
    pub window_open: bool,
    pub window_pos: Option<[f32; 2]>,
    pub window_size: [f32; 2],
    default_root_added: bool,
    known_packs: Vec<PathBuf>,
    /// MC-20: which kinds the contents show.
    pub kind_filter: Bucket,
    /// The built-in Basics folder (read-only root after the user's roots).
    pub basics: Option<PathBuf>,
    /// The default library folder, looked in for packs (F1).
    pub library: Option<PathBuf>,
    /// Each root's pack manifest, if it has one.
    pub packs: HashMap<PathBuf, PackInfo>,

    /// The shown folder's contents, once read.
    pub listing: Option<(PathBuf, Listing)>,
    loader: Option<Loader>,
    /// A search's results: everything below the folder (PB-26).
    pub deep: Option<(PathBuf, Vec<Entry>)>,
    last_check: f64,
    folder_stamp: Option<(std::time::SystemTime, usize)>,
    /// Subfolders by folder, for the tree; re-read on Refresh and checks.
    pub children: HashMap<PathBuf, Vec<PathBuf>>,
    /// Roots whose folder isn't there now (CD-01 "not found").
    pub missing: BTreeSet<PathBuf>,
    /// Modo material presets converted for their tiles, by file (LXP-02,
    /// LXP-05); `None` when the conversion failed.
    pub lxp_info: HashMap<PathBuf, Option<LxpInfo>>,
    /// The highlighted entry (PB-111).
    pub highlight: Option<PathBuf>,
    /// The contents area has the keyboard (PB-111).
    pub contents_focus: bool,
    /// Tiles across in the grid as last drawn, for the arrow keys.
    pub grid_columns: usize,
    pub search_open: bool,
    pub search: String,
    pub search_focus: bool,
    pub tree: tree_list::TreeState<u64>,
    pub list: tree_list::TreeState<u64>,
    pub thumbs: Thumbs,
    /// F6 was pressed; the next frame opens or closes the window under
    /// the pointer (PB-101).
    pub popover_toggle: bool,
    pub ask: Option<Ask>,
    pub drag: Option<Drag>,
    /// The material group a dragged material would land on (PB-110).
    pub drop_hover: Option<DropHover>,
    pub save_menu: Option<SaveMenu>,
    /// MC-02: a folder dropped on the browser, asking what to do.
    pub folder_drop: Option<FolderDrop>,
    /// MC-02: Copy Into Library running in the background.
    pub copy_job: Option<CopyJob>,
    /// Where the browser was drawn last (main window or its own), for
    /// folders dropped from the Finder.
    pub body_rect: Option<(Rect, bool)>,
    /// A drop out of the browser, run in the main window's frame.
    pub deferred_drop: Option<(Entry, Pos2)>,
    /// The keys held when the last drop out of the browser was released
    /// (PG-06 Shift onto a surface, PG-07 Cmd/Ctrl to open a scene).
    pub drop_modifiers: egui::Modifiers,
    /// The drag out of the browser started in its own OS window, so the
    /// main window draws its label once the pointer leaves that window.
    pub drag_in_window: bool,
    /// PG-04: the next item preset save is centred on the ground (the
    /// save dialog's check box, `preset.saveAs centerOnGround:`).
    pub save_centered: bool,
    /// The shader tree group row a dragged material would replace the
    /// contents of (material editing item 7).
    pub tree_drop: Option<u64>,
    /// A material dropped on a shader tree group row, run in the main
    /// window's frame.
    pub deferred_tree_drop: Option<(Entry, u64)>,
    /// The entry whose thumbnail a picked image file replaces.
    pub thumb_target: Option<PathBuf>,
    /// The OS window's place (for drops between windows).
    pub window_native: Option<(Pos2, f32, f32)>,
    /// Folder rows' rects in whichever window drew the tree last, for
    /// drops of items onto a folder (PB-120).
    pub folder_rects: Vec<(PathBuf, Rect)>,
    pub folder_rects_in_window: bool,
    /// The active view's matcap whose file was last reported missing
    /// (MC-08), so the status says it once.
    pub missing_matcap: Option<crate::matcap::Choice>,
    /// The roots the library's matcaps were last found under (MC-05), and
    /// when; a scan running in the background.
    matcap_roots: Vec<PathBuf>,
    matcap_scanned: f64,
    matcap_scan: Option<Receiver<crate::matcap::Library>>,
    /// Matcaps' sidecar notes (MC-11) as read, until the next Refresh.
    matcap_notes: std::cell::RefCell<HashMap<PathBuf, Option<String>>>,
    saved: Saved,
}

impl PresetBrowser {
    pub(crate) fn new(cache: PathBuf) -> Self {
        Self::from_saved(Saved::default(), cache)
    }

    fn from_saved(saved: Saved, cache: PathBuf) -> Self {
        let thumbs = Thumbs::new(cache);
        *thumbs.resolution.lock().expect("resolution") = saved.resolution;
        Self {
            roots: saved.roots.clone(),
            folder: saved.folder.clone(),
            expanded: saved.expanded.iter().cloned().collect(),
            view: saved.view,
            sort: saved.sort,
            thumb_size: saved.thumb_size.clamp(THUMB_MIN, THUMB_MAX),
            only_usable: saved.only_usable,
            resolution: saved.resolution,
            write_materialx: saved.write_materialx,
            window_open: saved.window_open,
            window_pos: saved.window_pos,
            window_size: saved.window_size,
            default_root_added: saved.default_root_added,
            known_packs: saved.known_packs.clone(),
            kind_filter: saved.kind_filter,
            basics: None,
            library: None,
            packs: HashMap::new(),
            listing: None,
            loader: None,
            deep: None,
            last_check: f64::NEG_INFINITY,
            folder_stamp: None,
            children: HashMap::new(),
            missing: BTreeSet::new(),
            lxp_info: HashMap::new(),
            highlight: None,
            contents_focus: false,
            grid_columns: 1,
            search_open: false,
            search: String::new(),
            search_focus: false,
            tree: Default::default(),
            list: Default::default(),
            thumbs,
            popover_toggle: false,
            ask: None,
            drag: None,
            drop_hover: None,
            save_menu: None,
            folder_drop: None,
            copy_job: None,
            body_rect: None,
            deferred_drop: None,
            drop_modifiers: egui::Modifiers::NONE,
            drag_in_window: false,
            save_centered: false,
            tree_drop: None,
            deferred_tree_drop: None,
            thumb_target: None,
            window_native: None,
            folder_rects: Vec::new(),
            folder_rects_in_window: false,
            missing_matcap: None,
            matcap_roots: Vec::new(),
            matcap_scanned: f64::NEG_INFINITY,
            matcap_scan: None,
            matcap_notes: Default::default(),
            saved,
        }
    }

    fn to_saved(&self) -> Saved {
        Saved {
            version: 1,
            roots: self.roots.clone(),
            folder: self.folder.clone(),
            expanded: self.expanded.iter().cloned().collect(),
            view: self.view,
            sort: self.sort,
            thumb_size: self.thumb_size,
            only_usable: self.only_usable,
            resolution: self.resolution,
            write_materialx: self.write_materialx,
            window_open: self.window_open,
            window_pos: self.window_pos,
            window_size: self.window_size,
            default_root_added: self.default_root_added,
            known_packs: self.known_packs.clone(),
            kind_filter: self.kind_filter,
        }
    }

    /// Read `path`; anything unreadable starts afresh. The Presets folder
    /// becomes the first root once (PB-Q2), even before it exists.
    pub(crate) fn load(path: &Path, presets: PathBuf, cache: PathBuf) -> Self {
        let saved = std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<Saved>(&t).ok())
            .filter(|s| s.version == 1)
            .unwrap_or_default();
        let mut browser = Self::from_saved(saved, cache);
        if !browser.default_root_added {
            browser.default_root_added = true;
            if !browser.roots.iter().any(|r| r.path == presets) {
                browser.roots.insert(
                    0,
                    Root {
                        path: presets,
                        label: None,
                    },
                );
            }
        }
        browser
    }

    /// Write the settings when they changed.
    pub(crate) fn save_if_changed(&mut self, path: &Path) {
        let now = self.to_saved();
        if now != self.saved {
            if let Ok(text) = serde_json::to_string_pretty(&now) {
                let _ = std::fs::write(path, text);
            }
            self.saved = now;
        }
    }

    /// Set the built-in Basics folder and the library folder, add packs
    /// found in it, and read the roots' manifests (start-up).
    pub(crate) fn set_places(&mut self, basics: Option<PathBuf>, library: PathBuf) {
        self.basics = basics;
        self.library = Some(library);
        self.scan_library();
        self.read_manifests();
        if self.folder.is_none() {
            self.folder = self.basics.clone();
        }
    }

    /// F1: packs in the library folder become roots, once each. MC-01:
    /// so does any other folder put there (a Modo content folder dropped
    /// in the Finder, or a link to one), without a restart.
    pub(crate) fn scan_library(&mut self) -> bool {
        let Some(library) = &self.library else {
            return false;
        };
        let mut found_all = pack::packs_in(library);
        if !found_all.iter().any(|p| p == library) {
            for folder in library::subfolders(library).unwrap_or_default() {
                if !found_all.contains(&folder) {
                    found_all.push(folder);
                }
            }
        }
        let mut added = false;
        for found in found_all {
            if self.known_packs.contains(&found) {
                continue;
            }
            self.known_packs.push(found.clone());
            if !self.roots.iter().any(|r| r.path == found) {
                self.roots.push(Root {
                    path: found,
                    label: None,
                });
                added = true;
            }
        }
        added
    }

    /// Read (again) the manifest of every root whose `manifest.json` is
    /// new or changed. Returns whether any changed.
    pub(crate) fn read_manifests(&mut self) -> bool {
        let mut changed = false;
        let paths: Vec<PathBuf> = self.roots.iter().map(|r| r.path.clone()).collect();
        self.packs.retain(|p, _| paths.contains(p));
        for path in paths {
            let file = path.join(pack::MANIFEST_FILE);
            let stamp = std::fs::metadata(&file)
                .ok()
                .and_then(|m| m.modified().ok());
            if self.packs.get(&path).is_some_and(|p| p.stamp == stamp) {
                continue;
            }
            let manifest = stamp.and_then(|_| Manifest::read(&path).ok()).map(Arc::new);
            self.packs.insert(path, PackInfo { stamp, manifest });
            changed = true;
        }
        changed
    }

    /// The manifest of root `path`, if it's a pack.
    pub(crate) fn manifest(&self, root: &Path) -> Option<&Manifest> {
        self.packs.get(root)?.manifest.as_deref()
    }

    /// Whether any root (that's there now) is a pack (F1's empty state).
    pub(crate) fn has_pack(&self) -> bool {
        self.roots
            .iter()
            .any(|r| !self.missing.contains(&r.path) && self.manifest(&r.path).is_some())
    }

    /// The pack holding `path` and the manifest's item for it.
    pub(crate) fn pack_item(&self, path: &Path) -> Option<(&Manifest, &pack::ManifestItem)> {
        self.roots
            .iter()
            .filter(|r| path.starts_with(&r.path))
            .filter_map(|r| {
                let m = self.manifest(&r.path)?;
                Some((m, m.item_for(&r.path, path)?))
            })
            .next()
    }

    /// The newest version of root `root`'s pack installed elsewhere, when
    /// it's newer than this one (two versions installed side by side).
    pub(crate) fn newer_version(&self, root: &Path) -> Option<(String, PathBuf)> {
        let mine = self.manifest(root)?;
        self.roots
            .iter()
            .filter(|r| r.path != root)
            .filter_map(|r| Some((self.manifest(&r.path)?, &r.path)))
            .filter(|(m, _)| m.name == mine.name)
            .filter(|(m, _)| pack::compare_versions(&m.version, &mine.version).is_gt())
            .max_by(|a, b| pack::compare_versions(&a.0.version, &b.0.version))
            .map(|(m, p)| (m.version.clone(), p.clone()))
    }

    /// The name a root shows: its label, else its pack's name and
    /// version, else its folder's name.
    pub(crate) fn root_title(&self, root: &Root) -> String {
        if let Some(label) = &root.label {
            return label.clone();
        }
        match self.manifest(&root.path) {
            Some(m) => m.title(),
            None => folder_name(&root.path),
        }
    }

    /// Inside the built-in Basics: no file may be written there.
    pub(crate) fn read_only(&self, path: &Path) -> bool {
        self.basics.as_deref().is_some_and(|b| path.starts_with(b))
    }

    /// Add `folder` as a root (PB-12): the one route for every way a root
    /// is added (`pbview.basePathAdd`, Add Folder…, Locate Library
    /// Folder…), so a pack behaves the same however it's added. When the
    /// folder is a pack (F1), or holds packs in its subfolders, the packs
    /// are added and noted as known, so the library folder's scan won't
    /// re-add one removed later; otherwise the folder itself is. `label`
    /// names the root only when the folder itself is added. An existing
    /// root is just shown. Returns what was added, for the status line.
    pub(crate) fn add_root(&mut self, folder: PathBuf, label: Option<String>) -> String {
        let packs = pack::packs_in(&folder);
        let has_packs = !packs.is_empty();
        let added = if has_packs {
            packs
        } else {
            vec![folder.clone()]
        };
        for p in &added {
            if has_packs && !self.known_packs.contains(p) {
                self.known_packs.push(p.clone());
            }
            if !self.roots.iter().any(|r| &r.path == p) {
                self.roots.push(Root {
                    path: p.clone(),
                    label: if *p == folder { label.clone() } else { None },
                });
            }
        }
        self.read_manifests();
        self.show_folder(added[0].clone());
        let titles: Vec<String> = added
            .iter()
            .map(|p| {
                self.manifest(p)
                    .map_or_else(|| p.display().to_string(), Manifest::title)
            })
            .collect();
        format!("Added {} to the Preset Browser", titles.join(", "))
    }

    /// Take `path` out of the tree; files are untouched (PB-13).
    pub(crate) fn remove_root(&mut self, path: &Path) -> bool {
        let before = self.roots.len();
        self.roots.retain(|r| r.path != path);
        if self.folder.as_deref().is_some_and(|f| f.starts_with(path)) {
            self.folder = self
                .roots
                .first()
                .map(|r| r.path.clone())
                .or_else(|| self.basics.clone());
            self.listing = None;
            self.deep = None;
        }
        self.roots.len() != before
    }

    /// BQ-06: move roots `moved` before (or after, `true`) the root
    /// `target`, or to the end. A drop below a root's subfolders lands
    /// after that root; only roots move.
    pub(crate) fn reorder_roots(&mut self, moved: &[PathBuf], target: Option<(PathBuf, bool)>) {
        if moved.is_empty() {
            return;
        }
        let taken: Vec<Root> = self
            .roots
            .iter()
            .filter(|r| moved.contains(&r.path))
            .cloned()
            .collect();
        self.roots.retain(|r| !moved.contains(&r.path));
        let at = match target {
            Some((path, after)) => self
                .roots
                .iter()
                .position(|r| path.starts_with(&r.path))
                .map_or(self.roots.len(), |i| i + usize::from(after)),
            None => self.roots.len(),
        };
        let at = at.min(self.roots.len());
        self.roots.splice(at..at, taken);
    }

    /// Show `folder` on the right (PB-11).
    pub(crate) fn show_folder(&mut self, folder: PathBuf) {
        if self.folder.as_ref() != Some(&folder) {
            self.folder = Some(folder);
            self.listing = None;
            self.deep = None;
            self.highlight = None;
            self.folder_stamp = None;
            self.thumbs.clear_queue();
            self.start_listing(false);
        }
    }

    /// Read the shown folder again in the background (PB-107, CD-04).
    pub(crate) fn start_listing(&mut self, deep: bool) {
        let Some(folder) = self.folder.clone() else {
            return;
        };
        if let Some(old) = self.loader.take() {
            old.stop.store(true, Ordering::Relaxed);
        }
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let options = ListOptions {
            only_usable: self.only_usable,
        };
        let (f, s) = (folder.clone(), stop.clone());
        let _ = std::thread::Builder::new()
            .name("preset-browser-list".into())
            .spawn(move || {
                let kind = |p: &Path| {
                    operandi_core::persistence::read_preset_header(p)
                        .ok()
                        .map(|h| h.kind)
                };
                let stopped = || s.load(Ordering::Relaxed);
                let result = if deep {
                    deep_list(&f, options, &kind, &stopped)
                } else {
                    library::list_folder(&f, options, &kind, &stopped)
                };
                let _ = tx.send(result);
            });
        self.loader = Some(Loader {
            folder,
            rx,
            stop,
            deep,
        });
    }

    /// Keep the library's matcaps (MC-05) current: find them again in the
    /// background when the roots change, after Refresh (`force`), and
    /// every half minute. Returns whether a new list arrived.
    pub(crate) fn scan_matcaps(&mut self, now: f64, force: bool) -> bool {
        const EVERY: f64 = 30.;
        let mut changed = false;
        if let Some(rx) = &self.matcap_scan
            && let Ok(list) = rx.try_recv()
        {
            self.matcap_scan = None;
            crate::matcap::set_library(list);
            changed = true;
        }
        let roots: Vec<PathBuf> = self
            .roots
            .iter()
            .map(|r| r.path.clone())
            .chain(self.basics.clone())
            .collect();
        let due = force || roots != self.matcap_roots || now - self.matcap_scanned >= EVERY;
        if due && self.matcap_scan.is_none() {
            self.matcap_roots = roots.clone();
            self.matcap_scanned = now;
            let (tx, rx) = std::sync::mpsc::channel();
            let spawned = std::thread::Builder::new()
                .name("preset-browser-matcaps".into())
                .spawn(move || {
                    let mut seen = std::collections::HashSet::new();
                    let list: crate::matcap::Library = roots
                        .iter()
                        .flat_map(|r| library::find_matcaps(r, &|| false))
                        .filter(|m| seen.insert(m.path.clone()))
                        .collect();
                    let _ = tx.send(list);
                });
            if spawned.is_ok() {
                self.matcap_scan = Some(rx);
            }
        }
        changed
    }

    /// A matcap's sidecar note (MC-11), read once.
    pub(crate) fn matcap_note(&self, path: &Path) -> Option<String> {
        self.matcap_notes
            .borrow_mut()
            .entry(path.to_path_buf())
            .or_insert_with(|| library::matcap_note(path))
            .clone()
    }

    /// The listing is being read (the panel shows a busy mark, CD-04).
    pub(crate) fn listing_busy(&self) -> bool {
        self.loader.is_some()
    }

    /// Per frame: take finished listings, look for changes on disk now and
    /// then. Returns whether anything changed.
    pub(crate) fn tick(&mut self, now: f64) -> bool {
        let mut changed = false;
        if let Some(loader) = &self.loader
            && let Ok(result) = loader.rx.try_recv()
        {
            let loader = self.loader.take().expect("checked");
            if Some(&loader.folder) == self.folder.as_ref() {
                let listing = result.unwrap_or_default();
                if loader.deep {
                    self.deep = Some((loader.folder.clone(), listing.entries));
                } else {
                    self.children
                        .insert(loader.folder.clone(), listing.folders.clone());
                    self.listing = Some((loader.folder, listing));
                }
                changed = true;
            }
        }
        if now - self.last_check >= CHECK_SECONDS {
            self.last_check = now;
            // CD-01: roots on drives that went away grey out and come back.
            let missing: BTreeSet<PathBuf> = self
                .roots
                .iter()
                .filter(|r| !r.path.is_dir())
                .map(|r| r.path.clone())
                .collect();
            if missing != self.missing {
                for back in self.missing.difference(&missing) {
                    self.children.remove(back);
                    if self.folder.as_ref().is_some_and(|f| f.starts_with(back)) {
                        self.folder_stamp = None;
                    }
                }
                self.missing = missing;
                changed = true;
            }
            // F1: a pack unzipped into the library folder, or a manifest
            // that changed, shows without a restart.
            changed |= self.scan_library();
            changed |= self.read_manifests();
            // PB-107: the shown folder changed on disk.
            if let Some(folder) = self.folder.clone() {
                let stamp = folder_stamp(&folder);
                if self.folder_stamp.is_some() && stamp != self.folder_stamp
                    || self.listing.is_none() && self.loader.is_none() && stamp.is_some()
                {
                    self.start_listing(false);
                    self.children.remove(&folder);
                }
                self.folder_stamp = stamp;
            }
        }
        changed
    }

    /// Refresh (PB-107, CD-02): read `folder` again, and its subfolders.
    pub(crate) fn refresh(&mut self, folder: Option<&Path>) {
        self.matcap_scanned = f64::NEG_INFINITY;
        self.matcap_notes.borrow_mut().clear();
        match folder {
            Some(f) => {
                self.children.retain(|k, _| !k.starts_with(f));
            }
            None => self.children.clear(),
        }
        self.folder_stamp = None;
        if folder.is_none() || folder == self.folder.as_deref() {
            self.start_listing(false);
        }
    }

    /// The subfolders of `folder`, read when first needed.
    pub(crate) fn subfolders(&mut self, folder: &Path) -> Vec<PathBuf> {
        if let Some(c) = self.children.get(folder) {
            return c.clone();
        }
        let c = library::subfolders(folder).unwrap_or_default();
        self.children.insert(folder.to_path_buf(), c.clone());
        c
    }

    /// The entries shown on the right: the folder's, or with a search,
    /// matches from the folder and everything below it (PB-26), sorted.
    pub(crate) fn shown(&self) -> Vec<Entry> {
        let text = self.search.trim().to_lowercase();
        let mut entries: Vec<Entry> = if self.search_open && !text.is_empty() {
            match &self.deep {
                Some((f, entries)) if Some(f) == self.folder.as_ref() => entries.clone(),
                _ => self
                    .listing
                    .as_ref()
                    .map(|(_, l)| l.entries.clone())
                    .unwrap_or_default(),
            }
            .into_iter()
            .filter(|e| e.name.to_lowercase().contains(&text))
            .collect()
        } else {
            self.listing
                .as_ref()
                .filter(|(f, _)| Some(f) == self.folder.as_ref())
                .map(|(_, l)| l.entries.clone())
                .unwrap_or_default()
        };
        entries.retain(|e| e.in_bucket(self.kind_filter));
        library::sort(&mut entries, self.sort);
        entries
    }

    /// MC-20: show only one kind. Not undoable (a view setting).
    pub(crate) fn set_kind_filter(&mut self, bucket: Bucket) {
        if self.kind_filter != bucket {
            self.kind_filter = bucket;
            self.highlight = None;
        }
    }

    /// CD-15: "Show only usable files"; the folder is read again.
    pub(crate) fn set_only_usable(&mut self, on: bool) {
        if self.only_usable != on {
            self.only_usable = on;
            self.refresh(None);
        }
    }

    /// MC-02, `dircache.copy`: copy `source` into `dest` (the library
    /// folder) in the background. A folder of that name already there is
    /// refused unless `overwrite`. Files in `source` are only read.
    pub(crate) fn start_copy(
        &mut self,
        source: &Path,
        dest: &Path,
        overwrite: bool,
    ) -> std::result::Result<String, String> {
        if self.copy_job.is_some() {
            return Err("A copy into the library is already running".into());
        }
        if !source.is_dir() {
            return Err(format!("No folder at {}", source.display()));
        }
        let name = folder_name(source);
        let target = dest.join(&name);
        let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        if canonical(dest).starts_with(canonical(source)) {
            return Err(format!("Can't copy {name} into itself"));
        }
        if target.exists() && !overwrite {
            return Err(format!(
                "{name} is already in the library; overwrite:true replaces its files"
            ));
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let from = source.to_path_buf();
        std::thread::Builder::new()
            .name("preset-browser-copy".into())
            .spawn(move || {
                let _ = tx.send(copy_tree(&from, &target).map_err(|e| e.to_string()));
            })
            .map_err(|e| e.to_string())?;
        self.copy_job = Some(CopyJob {
            name: name.clone(),
            rx,
        });
        Ok(format!("Copying {name} into the library…"))
    }

    /// A finished copy's message, once.
    pub(crate) fn poll_copy(&mut self) -> Option<String> {
        let job = self.copy_job.as_ref()?;
        let result = match job.rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(_) => Err("the copy stopped".into()),
        };
        let name = self.copy_job.take().expect("checked").name;
        self.refresh(None);
        self.scan_library();
        Some(match result {
            Ok(n) => format!("Copied {name} into the library ({n} files)"),
            Err(e) => format!("Copy of {name} into the library failed: {e}"),
        })
    }

    /// Search typed: look below the folder too (PB-26).
    pub(crate) fn search_changed(&mut self) {
        if self.search_open && !self.search.trim().is_empty() {
            let have = self
                .deep
                .as_ref()
                .is_some_and(|(f, _)| Some(f) == self.folder.as_ref());
            if !have && !self.loader.as_ref().is_some_and(|l| l.deep) {
                self.start_listing(true);
            }
        }
    }

    /// The entry at `path` in what's shown or listed.
    pub(crate) fn entry(&self, path: &Path) -> Option<Entry> {
        self.listing
            .iter()
            .flat_map(|(_, l)| l.entries.iter())
            .chain(self.deep.iter().flat_map(|(_, e)| e.iter()))
            .find(|e| e.path == path)
            .cloned()
    }

    /// Whether the browser's state asks a modal question right now.
    pub(crate) fn asking(&self) -> bool {
        self.ask.is_some()
    }

    pub(crate) fn thumb_job(entry: &Entry) -> Job {
        Job {
            key: entry.path.clone(),
            kind: entry.kind,
            set: entry.texture_set.clone(),
            embed: false,
        }
    }
}

/// Copy the folder `from` to `to`, file by file (links followed); returns
/// how many files were copied. Hidden OS junk is left behind (CD-04).
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(to)?;
    let mut count = 0;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if library::is_hidden(&name.to_string_lossy()) {
            continue;
        }
        let (src, dst) = (entry.path(), to.join(&name));
        if src.is_dir() {
            count += copy_tree(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst)?;
            count += 1;
        }
    }
    Ok(count)
}

/// A cheap "did this folder change" mark: its modified time and how many
/// entries it has.
fn folder_stamp(folder: &Path) -> Option<(std::time::SystemTime, usize)> {
    let modified = std::fs::metadata(folder).ok()?.modified().ok()?;
    let count = std::fs::read_dir(folder).ok()?.count();
    Some((modified, count))
}

/// Everything in `folder` and below it, for a search (at most 20,000
/// entries, so a huge drive can't stall it).
fn deep_list(
    folder: &Path,
    options: ListOptions,
    kind: &dyn Fn(&Path) -> Option<PresetKind>,
    stop: &dyn Fn() -> bool,
) -> std::io::Result<Listing> {
    let mut out = Listing::default();
    let mut todo = vec![folder.to_path_buf()];
    while let Some(dir) = todo.pop() {
        if stop() || out.entries.len() > 20_000 {
            break;
        }
        let Ok(listing) = library::list_folder(&dir, options, kind, stop) else {
            continue;
        };
        todo.extend(listing.folders);
        out.entries.extend(listing.entries);
    }
    Ok(out)
}

/// Where a load is aimed (PB-110).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Target {
    /// Double-click, Enter, Load, a script: the usual place per kind.
    Default,
    /// Dropped at this main-window point.
    Drop(Pos2),
}

impl Operandi {
    /// The browser's state file.
    fn browser_file(&self) -> PathBuf {
        self.environment
            .paths()
            .data_root()
            .join("preset-browser.json")
    }

    /// Where thumbnails are cached (PB-106).
    pub(super) fn browser_cache(&self) -> PathBuf {
        self.environment
            .paths()
            .data_root()
            .join("Cache")
            .join("Thumbnails")
    }

    /// The browser is the front tab of a side panel group.
    pub(super) fn panel_shown(&self, kind: crate::panel_registry::PanelKind) -> bool {
        self.upper_frame.front() == Some(kind) || self.lower_frame.front() == Some(kind)
    }

    /// A preview render is running.
    pub(super) fn preview_busy(&self) -> bool {
        self.preview.busy()
    }

    /// Select the shader tree's material group named `name` (preset.do's
    /// `target:`), so a material loads into it.
    pub(super) fn select_group_named(&mut self, name: &str) -> bool {
        let tree = &self.editor.scene.shader_tree;
        match tree.nodes.iter().find(|n| {
            n.name.eq_ignore_ascii_case(name)
                && matches!(n.kind, operandi_core::shader_tree::NodeKind::Group(_))
        }) {
            Some(n) => {
                self.shading.selected = vec![n.id];
                true
            }
            None => {
                self.status = format!("No material group named {name}");
                false
            }
        }
    }

    /// Read the browser's saved roots and settings (at start-up).
    pub(super) fn load_preset_browser(&mut self) {
        let presets = self.environment.paths().presets();
        // PG-01, MC-09: a fresh Presets folder gets its empty starter
        // folders (one left alone once it holds anything).
        if let Err(e) = library::make_starter_folders(&presets) {
            self.status = format!("Presets folder: {e}");
        }
        let cache = self.browser_cache();
        *self.preset_browser = PresetBrowser::load(&self.browser_file(), presets, cache);
        // F1: the Basics are written from the app's own definitions each
        // start (unchanged files stay as they are, so thumbnails stay
        // cached); a data folder that can't be written just has none.
        let basics_dir = self.environment.paths().basics();
        let basics = match operandi_core::preset::basics::write_to(&basics_dir) {
            Ok(_) => Some(basics_dir),
            Err(e) => {
                self.status = format!("Basics presets: {e}");
                None
            }
        };
        let library = self.environment.paths().library().to_path_buf();
        // UIC-73: an open window reopens where it was; a fresh one shows
        // the Basics, so it isn't empty.
        self.preset_browser.set_places(basics, library);
    }

    /// Refuse a write inside the read-only Basics (UIC-117's note).
    fn refuse_read_only(&mut self, path: &Path) -> bool {
        if self.preset_browser.read_only(path) {
            self.status = READ_ONLY_NOTE.into();
            true
        } else {
            false
        }
    }

    /// "Get the free Operandi Library…" (F1): `url` (default
    /// [`LIBRARY_URL`]) in the default web browser. Nothing is downloaded
    /// and nothing is undoable; a failed open is an error the user can read.
    pub(super) fn browser_get_library(&mut self, url: &str) -> std::result::Result<(), String> {
        self.open_with_default_app(std::ffi::OsStr::new(url), url)
            .map_err(|e| {
                format!(
                    "Couldn't open {url} in your web browser ({e}). Visit it by hand to get the Operandi Library."
                )
            })?;
        if !cfg!(test) {
            self.status = format!("Opened {url} in your web browser");
        }
        Ok(())
    }

    /// "Locate Library Folder…" (F1): a folder chooser starting in the
    /// library folder; the folder picked takes `pbview.basePathAdd`'s path.
    pub(super) fn browser_locate_library(&mut self) {
        let folder = self
            .preset_browser
            .library
            .clone()
            .filter(|l| l.is_dir())
            .unwrap_or_else(|| self.environment.paths().data_root().to_path_buf());
        if !self.choose(
            file_chooser::Choose::Folder {
                folder,
                title: "Locate Library Folder",
            },
            file_chooser::Purpose::PresetBrowserRoot,
        ) {
            self.status = "No file panel is available; use pbview.basePathAdd path:<folder>".into();
        }
    }

    pub(super) fn save_preset_browser(&mut self) {
        let path = self.browser_file();
        self.preset_browser.save_if_changed(&path);
    }

    /// Window > Preset Browser (PB-101): open or close its window.
    pub(super) fn toggle_preset_browser(&mut self) {
        self.set_preset_browser_window(!self.preset_browser.window_open);
    }

    pub(super) fn set_preset_browser_window(&mut self, open: bool) {
        self.preset_browser.window_open = open;
    }

    /// F6 (PB-101): opens or closes the browser's floating window. It used
    /// to be a popover fixed inside the main window; Jason (2026-11-11)
    /// wanted it movable like the other pop-up panels, so F6 now drives the
    /// same window. The first time (no remembered place) it opens under the
    /// pointer.
    pub(super) fn toggle_preset_browser_under_pointer(&mut self, ctx: &Context) {
        let open = !self.preset_browser.window_open;
        if open && self.preset_browser.window_pos.is_none() {
            let pointer = self
                .main_window_pointer(ctx)
                .unwrap_or_else(|| ctx.content_rect().center());
            if let Some(inner) = ctx.input(|i| i.viewport().inner_rect) {
                let width = self.preset_browser.window_size[0];
                self.preset_browser.window_pos = Some([
                    inner.left() + pointer.x - width / 2.,
                    inner.top() + pointer.y - 12.,
                ]);
            }
        }
        self.set_preset_browser_window(open);
    }

    /// Load `entry` (PB-30/31, PB-110, CD-11..14), one undo step. Returns
    /// whether it did something.
    pub(super) fn browser_load(&mut self, entry: &Entry, target: Target) -> bool {
        if self.has_modal() {
            return false;
        }
        let name = entry.name.clone();
        match entry.kind {
            // EP §Apply: double-click or Load is Replace; dropped on the
            // Preview it is Add; dropped on a 3D view a small menu asks.
            EntryKind::Preset(PresetKind::Environment) => match target {
                Target::Default => {
                    self.apply_environment_preset_from(&entry.path, ApplyMode::Replace, true)
                }
                Target::Drop(at) if self.preview.picture_rect.contains(at) => {
                    self.apply_environment_preset_from(&entry.path, ApplyMode::Add, true)
                }
                Target::Drop(at) => {
                    self.env_drop_menu = Some((entry.path.clone(), at));
                    true
                }
            },
            EntryKind::Preset(kind) => {
                if let Target::Drop(at) = target
                    && kind == PresetKind::Material
                    && let Some((item, faces)) = self.drop_geometry(at)
                {
                    let mut preset = match operandi_core::persistence::load_preset(&entry.path) {
                        Ok(p) => p,
                        Err(e) => {
                            self.status = format!("Load preset {name}: {e}");
                            return false;
                        }
                    };
                    self.adopt_scene_files(&mut preset.scene);
                    return self.apply_dropped_material(preset, item, faces, &name);
                }
                // PG-06: an item preset Shift-dropped rests on the surface
                // or work plane under the pointer.
                if let Target::Drop(at) = target
                    && matches!(kind, PresetKind::Item | PresetKind::Assembly)
                    && let Some(point) = self.shift_drop_point(at)
                {
                    let mut preset = match operandi_core::persistence::load_preset(&entry.path) {
                        Ok(p) => p,
                        Err(e) => {
                            self.status = format!("Load preset {name}: {e}");
                            return false;
                        }
                    };
                    operandi_core::preset::placement::rest_on(&mut preset.scene, point);
                    self.adopt_scene_files(&mut preset.scene);
                    if !self.confirm_fields() {
                        return false;
                    }
                    let ok = self.run_ok(Command::LoadPreset {
                        preset: Box::new(preset),
                        group: None,
                    });
                    if ok {
                        self.status = format!("Placed {name}");
                    }
                    return ok;
                }
                let group = self.preset_target_group();
                self.load_preset_from(entry.path.clone(), group, true);
                true
            }
            EntryKind::BrokenPreset => {
                self.status = match operandi_core::persistence::read_preset_header(&entry.path) {
                    Err(e) => format!("Can't read {name}: {e}"),
                    Ok(_) => format!("Can't read {name}"),
                };
                false
            }
            // PG-07: dragged into a view, a scene joins the current one
            // (Cmd/Ctrl held: opens as its own scene, UIC-74); double-click
            // opens it.
            EntryKind::Scene => match target {
                Target::Drop(at) if !self.preset_browser.drop_modifiers.command => {
                    let rest = self.shift_drop_point(at);
                    self.import_into_current(
                        &entry.path,
                        crate::import_into::ImportKinds::default(),
                        rest,
                    )
                }
                _ => {
                    self.request_open_path(entry.path.clone());
                    true
                }
            },
            // MC-11: a new mesh item at the origin; Shift-dropped, under the
            // pointer (PG-06, as for model files).
            EntryKind::ModoMeshPreset => {
                let at = match target {
                    Target::Drop(at) => Some(at),
                    Target::Default => None,
                };
                self.browser_add_modo(entry, at)
            }
            // LXP-03: like any library material, in one undo step.
            EntryKind::ModoMaterial => {
                let converted = match self.modo_material_preset(entry) {
                    Ok(c) => c,
                    Err(e) => {
                        self.status = e;
                        return false;
                    }
                };
                let note = operandi_core::preset::lxp::note_line(&converted);
                let preset = converted.preset;
                let done = if let Target::Drop(at) = target
                    && let Some((item, faces)) = self.drop_geometry(at)
                {
                    self.apply_dropped_material(preset, item, faces, &name)
                } else {
                    let group = self.preset_target_group();
                    if !self.confirm_fields() {
                        return false;
                    }
                    let ok = self.run_ok(Command::LoadPreset {
                        preset: Box::new(preset),
                        group,
                    });
                    if ok {
                        self.status = format!("Loaded Modo material {name}");
                    }
                    ok
                };
                // §2 step 5: the one import note.
                if done && !note.is_empty() {
                    self.status = format!("{} · {note}", self.status);
                }
                done
            }
            // MC-12: greyed; it only says why.
            EntryKind::ModoPreset => {
                self.status = entry.unsupported_note().unwrap_or_default();
                false
            }
            // CD-11, PG-06: dragged in or double-clicked, a model joins the
            // scene at the origin (Shift-dropped: on the surface under the
            // pointer). Right-click Open opens it as its own scene.
            EntryKind::Model => {
                let rest = match target {
                    Target::Drop(at) => self.shift_drop_point(at),
                    Target::Default => None,
                };
                self.import_into_current(
                    &entry.path,
                    crate::import_into::ImportKinds::default(),
                    rest,
                )
            }
            EntryKind::Image => match target {
                // UIC-74 by where it lands: a Backdrop over a view.
                Target::Drop(at) => {
                    let path = self.adopt_dropped(entry.path.clone());
                    self.file_dropped(path, Some(at));
                    true
                }
                // CD-12: the image viewer.
                Target::Default => {
                    self.viewer_load(&entry.path);
                    self.open_render_window();
                    true
                }
            },
            // MC-02/03: the active view, or the view it's dropped on (over a
            // mesh too: it never touches the scene's materials).
            EntryKind::Matcap => {
                let view = match target {
                    Target::Drop(at) => self
                        .corner
                        .areas
                        .iter()
                        .find(|(_, a)| a.contains(at))
                        .map(|(&v, _)| v),
                    Target::Default => None,
                };
                self.use_matcap(&entry.path, view, false)
            }
            // CD-13: an HDR loads as the top environment's image, dragged
            // onto a view's empty space or double-clicked. Dragged onto a
            // mesh it goes there as any picture does (UIC-74).
            EntryKind::Environment => match target {
                Target::Drop(at) if self.drop_geometry(at).is_some() => {
                    let path = self.adopt_dropped(entry.path.clone());
                    self.file_dropped(path, Some(at));
                    true
                }
                _ => {
                    let path = self.adopt_file(crate::project_files::SOURCE_IMAGES, &entry.path);
                    self.environment_image_chosen(None, &path)
                }
            },
            EntryKind::TextureSet => {
                let Some(set) = &entry.texture_set else {
                    return false;
                };
                let (mut preset, notes) = match set
                    .material_preset(self.preset_browser.resolution, &|p| {
                        image::image_dimensions(p).ok()
                    }) {
                    Ok(p) => p,
                    Err(e) => {
                        self.status = e;
                        return false;
                    }
                };
                self.adopt_scene_files(&mut preset.scene);
                if let Target::Drop(at) = target
                    && let Some((item, faces)) = self.drop_geometry(at)
                {
                    return self.apply_dropped_material(preset, item, faces, &name);
                }
                let group = self.preset_target_group();
                if !self.confirm_fields() {
                    return false;
                }
                if self.run_ok(Command::LoadPreset {
                    preset: Box::new(preset),
                    group,
                }) {
                    self.status = std::iter::once(format!("Loaded texture set {name}"))
                        .chain(notes)
                        .collect::<Vec<_>>()
                        .join(" · ");
                    true
                } else {
                    false
                }
            }
            EntryKind::MaterialX => {
                let (preset, notes) = match self.materialx_preset(entry) {
                    Ok(p) => p,
                    Err(e) => {
                        self.status = e;
                        return false;
                    }
                };
                if let Target::Drop(at) = target
                    && let Some((item, faces)) = self.drop_geometry(at)
                {
                    return self.apply_dropped_material(preset, item, faces, &name);
                }
                let group = self.preset_target_group();
                if !self.confirm_fields() {
                    return false;
                }
                if self.run_ok(Command::LoadPreset {
                    preset: Box::new(preset),
                    group,
                }) {
                    self.status = std::iter::once(format!("Loaded MaterialX {name}"))
                        .chain(notes)
                        .collect::<Vec<_>>()
                        .join(" · ");
                    true
                } else {
                    false
                }
            }
            EntryKind::Other => {
                self.browser_open_default(&entry.path);
                true
            }
        }
    }

    /// The MaterialX file `entry` as a material preset, or a refusal
    /// (MX-03/04) before any change; a texture set beside it adds its AO
    /// (MX-Q03). Shared by the browser and `operandi.materialx.import`.
    pub(super) fn materialx_preset(
        &mut self,
        entry: &Entry,
    ) -> std::result::Result<(operandi_core::preset::Preset, Vec<String>), String> {
        let name = &entry.name;
        let size = |p: &Path| image::image_dimensions(p).ok();
        let (mut preset, mut notes) =
            operandi_core::materialx::material_preset(&entry.path, None, &size)
                .map_err(|e| format!("MaterialX {name}: {e}"))?;
        if let Some(set) = &entry.texture_set {
            let more = set
                .add_ambient_occlusion(&mut preset, self.preset_browser.resolution, &size)
                .map_err(|e| format!("MaterialX {name}: {e}"))?;
            notes.extend(more);
        }
        self.adopt_scene_files(&mut preset.scene);
        Ok((preset, notes))
    }

    fn apply_dropped_material(
        &mut self,
        preset: operandi_core::preset::Preset,
        item: u64,
        faces: Vec<usize>,
        name: &str,
    ) -> bool {
        if !self.confirm_fields() {
            return false;
        }
        if self.run_ok(Command::ApplyMaterialPreset {
            preset: Box::new(preset),
            item,
            faces,
        }) {
            self.status = format!("Applied {name}");
            true
        } else {
            false
        }
    }

    /// Show the matcap at `path` (MC-02/03/06): in `view` (made the active
    /// one), the active view, or with `all` every 3D view, each switched
    /// to the Matcap style. It runs the same commands as Viewport
    /// Properties (UIC-160): a view setting, never an undo step (MC-04).
    pub(super) fn use_matcap(&mut self, path: &Path, view: Option<u32>, all: bool) -> bool {
        if let Some(view) = view {
            self.activate_view(view);
        }
        let image = path.display().to_string();
        let image = if image.contains(' ') {
            format!("{{{image}}}")
        } else {
            image
        };
        let command = crate::viewport_properties::MATCAP_COMMAND;
        if all {
            self.execute(&format!("{command} name:custom image:{image} all:1"));
        } else {
            self.execute("view3d.shadingStyle Matcap");
            self.execute(&format!("{command} name:custom image:{image}"));
        }
        let ok = self.display.settings.drawing.matcap.path().as_deref() == Some(path);
        if ok && !self.display.settings.drawing.matcap.missing() {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.status = if all {
                format!("Matcap {name} in all 3D views")
            } else {
                format!("Matcap: {name}")
            };
        }
        ok
    }

    /// MC-10/11: read a Modo scene or mesh preset (`entry`) with the LXO
    /// reader and add its items to this scene, one undo step, selected.
    /// With `at` (a drop) they move to the work plane under the pointer;
    /// otherwise they stay where the file has them (a mesh preset at the
    /// origin). Groups the reader can't take are left out with a note; a
    /// file it refuses changes nothing (Q-MC1).
    fn browser_add_modo(&mut self, entry: &Entry, at: Option<Pos2>) -> bool {
        let name = entry.name.clone();
        let mut keep_going = |_: &str| true;
        let mut control = operandi_core::lxo::Control::new(&mut keep_going);
        let read = if entry.kind == EntryKind::ModoMeshPreset {
            operandi_core::lxo::load_mesh_preset(&entry.path, &mut control)
        } else {
            operandi_core::lxo::load(&entry.path, &mut control)
        };
        let candidate = match read {
            Ok(c) => c,
            Err(e) => {
                self.status = format!("Can't add {name}: {e}");
                return false;
            }
        };
        let left_out = candidate.report.rejected_groups.len();
        let mut scene = candidate.scene;
        // MC-10/T3: Modo materials arrive as named material groups with a
        // plain material, so a library material can be dropped on them.
        let tags: BTreeSet<String> = scene
            .items
            .iter()
            .flat_map(|i| i.mesh.faces.iter().map(|f| f.material.clone()))
            .filter(|t| !t.is_empty() && !operandi_core::material::is_default_name(t))
            .collect();
        for tag in tags {
            let material = scene.materials.get(&tag).cloned().unwrap_or_default();
            if let Err(e) = scene.shader_tree.ensure_material(&tag, &material) {
                self.status = format!("Can't add {name}: {e}");
                return false;
            }
        }
        if let Some(point) = at.and_then(|p| self.shift_drop_point(p)) {
            for item in scene.items.iter_mut().filter(|i| i.parent.is_none()) {
                item.transform.translation += point;
            }
        }
        self.adopt_scene_files(&mut scene);
        if !self.confirm_fields() {
            return false;
        }
        let ok = self.run_ok(Command::LoadPreset {
            preset: Box::new(operandi_core::preset::Preset {
                header: operandi_core::preset::PresetHeader {
                    kind: PresetKind::Assembly,
                    thumbnail: None,
                    source: None,
                },
                scene,
            }),
            group: None,
        });
        if ok {
            self.status = if left_out > 0 {
                format!(
                    "Added {name} (geometry only; {left_out} groups Operandi can't read left out)"
                )
            } else {
                format!("Added {name} (geometry only)")
            };
        }
        ok
    }

    /// MC-08: tell once when the active view's matcap file is missing (it
    /// draws Clay Studio).
    pub(super) fn note_missing_matcap(&mut self) {
        let choice = self.display.settings.drawing.matcap;
        let missing = choice.missing().then_some(choice);
        if missing != self.preset_browser.missing_matcap {
            self.preset_browser.missing_matcap = missing;
            if let Some(c) = missing {
                self.status = format!("Matcap not found: {}", c.label());
            }
        }
    }

    /// PG-06: with Shift held at the drop, the world point under `at`: the
    /// first mesh surface the pointer's ray meets, else the work plane.
    /// `None` without Shift (the drop lands at the origin).
    pub(super) fn shift_drop_point(&self, at: Pos2) -> Option<glam::DVec3> {
        if !self.preset_browser.drop_modifiers.shift {
            return None;
        }
        self.drop_point(at)
    }

    /// The world point under a main-window point: the mesh surface hit,
    /// else the work plane.
    pub(super) fn drop_point(&self, at: Pos2) -> Option<glam::DVec3> {
        let (&view, &area) = self.corner.areas.iter().find(|(_, a)| a.contains(at))?;
        let rect = self.view_rects.get(&view).copied().unwrap_or(area);
        let camera = if view == self.active_view {
            &self.camera
        } else {
            self.cameras.get(&view)?
        };
        let (origin, direction) = camera.ray(at, rect);
        if let Some((item, _)) = self.drop_hit(at)
            && let Some((point, _)) = operandi_core::drop_action::surface_hit(
                &self.editor.scene,
                item,
                operandi_core::drop_action::Ray { origin, direction },
            )
        {
            return Some(point);
        }
        camera.plane_point(at, rect, self.actual_plane(), self.workplane_center())
    }

    /// The mesh and polygon under a main-window point, if it's a mesh with
    /// polygons.
    fn drop_hit(&self, at: Pos2) -> Option<(u64, usize)> {
        let (&view, &area) = self.corner.areas.iter().find(|(_, a)| a.contains(at))?;
        // Pick through the view's picture, not its whole pane (the pane
        // includes the status line, which shifted the hit).
        let rect = self.view_rects.get(&view).copied().unwrap_or(area);
        let camera = if view == self.active_view {
            &self.camera
        } else {
            self.cameras.get(&view)?
        };
        let surface = self.pick_surface();
        let (item, face, _) = surface.pick(camera, at, rect)?;
        let it = self.editor.scene.item(item)?;
        ((face as usize) < it.mesh.faces.len()).then_some((item, face as usize))
    }

    /// The mesh and polygons a material dropped at a main-window point
    /// lands on (PB-110, PB1-Q02 as changed by Jason 2026-11-11): every
    /// polygon of the hit mesh with the hit polygon's material tag, and
    /// nothing else.
    pub(super) fn drop_geometry(&self, at: Pos2) -> Option<(u64, Vec<usize>)> {
        let hover = self.drop_hover_at(at, None)?;
        Some((hover.item, hover.faces))
    }

    /// [`Self::drop_geometry`] as a [`DropHover`]; `known` is reused when
    /// the pointer is still on the same mesh and tag, so moving over one
    /// material group doesn't rescan the mesh every frame.
    fn drop_hover_at(&self, at: Pos2, known: Option<&DropHover>) -> Option<DropHover> {
        let (item, face) = self.drop_hit(at)?;
        let mesh = &self.editor.scene.item(item)?.mesh;
        let tag = &mesh.faces[face].material;
        if let Some(k) = known
            && k.item == item
            && &k.tag == tag
        {
            return Some(k.clone());
        }
        let faces = mesh
            .faces
            .iter()
            .enumerate()
            .filter(|(i, f)| &f.material == tag && !mesh.is_polyline(*i))
            .map(|(i, _)| i)
            .collect();
        Some(DropHover {
            item,
            tag: tag.clone(),
            faces,
        })
    }

    /// The shader tree group row under `at` (a main-window point) a
    /// dragged material would load into: a Group layer's row. Other rows
    /// and empty space are no target.
    pub(super) fn tree_drop_target(&self, at: Pos2) -> Option<u64> {
        if !self.shading_tab() {
            return None;
        }
        let tree = &self.editor.scene.shader_tree;
        self.shading
            .tree
            .rects
            .iter()
            .find(|(_, r)| r.contains(at))
            .and_then(|(key, _)| match *key {
                crate::shading_panel::ShaderKey::Layer(id) => Some(id),
                _ => None,
            })
            .filter(|&id| {
                matches!(
                    tree.get(id).map(|n| &n.kind),
                    Some(operandi_core::shader_tree::NodeKind::Group(_))
                )
            })
    }

    /// Material editing item 7: a material dropped on group row `group`
    /// replaces the group's contents, as a drop on its polygons does
    /// (load into a group, `preset_candidate`). One undo step.
    /// A Modo material preset converted (BEHAVIOR-LXP-MATERIALS), images
    /// looked for beside it and under the browser's folders (§4.2).
    /// Its pictures are copied into the active project (project_files.rs).
    pub(super) fn modo_material_preset(
        &mut self,
        entry: &Entry,
    ) -> std::result::Result<operandi_core::preset::lxp::Converted, String> {
        let roots: Vec<PathBuf> = self
            .preset_browser
            .roots
            .iter()
            .map(|r| r.path.clone())
            .collect();
        let mut converted =
            operandi_core::preset::lxp::material_preset(&entry.path, &roots, &|p| {
                image::image_dimensions(p).ok()
            })
            .map_err(|e| format!("Modo material {}: {e}", entry.name))?;
        self.adopt_scene_files(&mut converted.preset.scene);
        Ok(converted)
    }
    /// The tile's conversion summary, converted once per file (images
    /// only checked for, not opened).
    pub(super) fn lxp_info(&mut self, entry: &Entry) -> Option<LxpInfo> {
        if entry.kind != EntryKind::ModoMaterial {
            return None;
        }
        if let Some(found) = self.preset_browser.lxp_info.get(&entry.path) {
            return found.clone();
        }
        let roots: Vec<PathBuf> = self
            .preset_browser
            .roots
            .iter()
            .map(|r| r.path.clone())
            .collect();
        let info = operandi_core::preset::lxp::material_preset(&entry.path, &roots, &|p| {
            p.is_file().then_some((1, 1))
        })
        .ok()
        .map(|c| LxpInfo {
            partial: c.partial,
            changed_look: c.changed_look,
            notes: c.notes,
        });
        self.preset_browser
            .lxp_info
            .insert(entry.path.clone(), info.clone());
        info
    }
    pub(super) fn browser_load_into_group(&mut self, entry: &Entry, group: u64) -> bool {
        if self.has_modal() || !drops_material(entry) {
            return false;
        }
        let name = entry.name.clone();
        let preset = match entry.kind {
            EntryKind::Preset(PresetKind::Material) => {
                match operandi_core::persistence::load_preset(&entry.path) {
                    Ok(mut p) => {
                        self.adopt_scene_files(&mut p.scene);
                        p
                    }
                    Err(e) => {
                        self.status = format!("Load preset {name}: {e}");
                        return false;
                    }
                }
            }
            EntryKind::TextureSet => {
                let Some(set) = &entry.texture_set else {
                    return false;
                };
                match set.material_preset(self.preset_browser.resolution, &|p| {
                    image::image_dimensions(p).ok()
                }) {
                    Ok((mut p, _)) => {
                        self.adopt_scene_files(&mut p.scene);
                        p
                    }
                    Err(e) => {
                        self.status = e;
                        return false;
                    }
                }
            }
            EntryKind::ModoMaterial => match self.modo_material_preset(entry) {
                Ok(c) => c.preset,
                Err(e) => {
                    self.status = e;
                    return false;
                }
            },
            _ => return false,
        };
        if !self.confirm_fields() {
            return false;
        }
        let group_name = self
            .editor
            .scene
            .shader_tree
            .get(group)
            .map_or_else(String::new, |n| n.name.clone());
        if self.run_ok(Command::LoadPreset {
            preset: Box::new(preset),
            group: Some(group),
        }) {
            self.status = format!("Loaded {name} into {group_name}");
            true
        } else {
            false
        }
    }

    /// While a material is dragged out of the browser: the material group
    /// under `at` (a main-window point; `None` = not over the main window)
    /// replaces the highlight (PB-110). Nothing in the scene changes.
    pub(super) fn update_drop_hover(&mut self, at: Option<Pos2>) {
        let drops_material = self
            .preset_browser
            .drag
            .as_ref()
            .is_some_and(|d| drops_material(&d.entry));
        let hover = at
            .filter(|_| drops_material)
            .and_then(|at| self.drop_hover_at(at, self.preset_browser.drop_hover.as_ref()));
        self.preset_browser.drop_hover = hover;
    }

    /// Show in Finder / Explorer (PB-112, CD-02).
    pub(super) fn browser_reveal(&mut self, path: &Path) {
        let result = if cfg!(target_os = "macos") {
            std::process::Command::new("open")
                .arg("-R")
                .arg(path)
                .spawn()
        } else if cfg!(target_os = "windows") {
            std::process::Command::new("explorer")
                .arg(format!("/select,{}", path.display()))
                .spawn()
        } else {
            std::process::Command::new("xdg-open")
                .arg(path.parent().unwrap_or(path))
                .spawn()
        };
        if let Err(e) = result {
            self.status = format!("Can't show {}: {e}", path.display());
        }
    }

    /// Open with default app (PB-112).
    pub(super) fn browser_open_default(&mut self, path: &Path) {
        let shown = path.display().to_string();
        if let Err(e) = self.open_with_default_app(path.as_os_str(), &shown) {
            self.status = format!("Can't open {shown}: {e}");
        }
    }

    /// Hand a file or web address to the OS's default app.
    fn open_with_default_app(
        &mut self,
        what: &std::ffi::OsStr,
        shown: &str,
    ) -> std::io::Result<()> {
        // Tests never open anything on the machine running them.
        if cfg!(test) {
            self.status = format!("Would open {shown}");
            return Ok(());
        }
        let result = if cfg!(target_os = "macos") {
            std::process::Command::new("open").arg(what).spawn()
        } else if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                .args(["/C", "start", ""])
                .arg(what)
                .spawn()
        } else {
            std::process::Command::new("xdg-open").arg(what).spawn()
        };
        result.map(drop)
    }

    /// Rename a file (PB-112). A texture set's maps are all renamed, keeping
    /// their suffixes. The extension stays.
    pub(super) fn browser_rename(&mut self, path: &Path, new_name: &str) -> bool {
        if self.refuse_read_only(path) {
            return false;
        }
        let new_name = new_name.trim();
        if new_name.is_empty() || new_name.contains(['/', '\\', ':']) {
            self.status = "Type a name without / \\ or :".into();
            return false;
        }
        let entry = self.preset_browser.entry(path);
        let moves: Vec<(PathBuf, PathBuf)> =
            match entry.as_ref().and_then(|e| e.texture_set.clone()) {
                Some(set) => set
                    .files()
                    .map(|f| {
                        let name = folder_name(f);
                        let rest = name.get(set.name.len()..).unwrap_or("");
                        (
                            f.to_path_buf(),
                            f.with_file_name(format!("{new_name}{rest}")),
                        )
                    })
                    .collect(),
                None => {
                    let ext = path
                        .extension()
                        .map(|e| format!(".{}", e.to_string_lossy()))
                        .unwrap_or_default();
                    vec![(
                        path.to_path_buf(),
                        path.with_file_name(format!("{new_name}{ext}")),
                    )]
                }
            };
        if let Some((_, taken)) = moves.iter().find(|(from, to)| from != to && to.exists()) {
            self.status = format!("{} already exists", folder_name(taken));
            return false;
        }
        for (from, to) in &moves {
            if let Err(e) = std::fs::rename(from, to) {
                self.status = format!("Can't rename {}: {e}", folder_name(from));
                return false;
            }
            self.preset_browser.thumbs.forget(from, false);
        }
        self.preset_browser.highlight = moves.first().map(|(_, to)| to.clone());
        self.preset_browser.refresh(path.parent());
        self.status = format!("Renamed to {new_name}");
        true
    }

    /// Duplicate a file under a new name (PB-112).
    pub(super) fn browser_duplicate(&mut self, path: &Path, new_name: &str) -> bool {
        if self.refuse_read_only(path) {
            return false;
        }
        let new_name = new_name.trim();
        if new_name.is_empty() || new_name.contains(['/', '\\', ':']) {
            self.status = "Type a name without / \\ or :".into();
            return false;
        }
        let entry = self.preset_browser.entry(path);
        let copies: Vec<(PathBuf, PathBuf)> =
            match entry.as_ref().and_then(|e| e.texture_set.clone()) {
                Some(set) => set
                    .files()
                    .map(|f| {
                        let name = folder_name(f);
                        let rest = name.get(set.name.len()..).unwrap_or("");
                        (
                            f.to_path_buf(),
                            f.with_file_name(format!("{new_name}{rest}")),
                        )
                    })
                    .collect(),
                None => {
                    let ext = path
                        .extension()
                        .map(|e| format!(".{}", e.to_string_lossy()))
                        .unwrap_or_default();
                    vec![(
                        path.to_path_buf(),
                        path.with_file_name(format!("{new_name}{ext}")),
                    )]
                }
            };
        if let Some((_, taken)) = copies.iter().find(|(_, to)| to.exists()) {
            self.status = format!("{} already exists", folder_name(taken));
            return false;
        }
        for (from, to) in &copies {
            if let Err(e) = std::fs::copy(from, to) {
                self.status = format!("Can't duplicate {}: {e}", folder_name(from));
                return false;
            }
        }
        self.preset_browser.highlight = copies.first().map(|(_, to)| to.clone());
        self.preset_browser.refresh(path.parent());
        self.status = format!("Duplicated as {new_name}");
        true
    }

    /// Delete from disk (PB-112; the menu asks first). A texture set's maps
    /// all go.
    pub(super) fn browser_delete(&mut self, path: &Path) -> bool {
        if self.refuse_read_only(path) {
            return false;
        }
        let entry = self.preset_browser.entry(path);
        let files: Vec<PathBuf> = match entry.as_ref().and_then(|e| e.texture_set.clone()) {
            Some(set) => set.files().map(Path::to_path_buf).collect(),
            None => vec![path.to_path_buf()],
        };
        for f in &files {
            if let Err(e) = std::fs::remove_file(f) {
                self.status = format!("Can't delete {}: {e}", folder_name(f));
                return false;
            }
            self.preset_browser.thumbs.forget(f, true);
        }
        if self.preset_browser.highlight.as_deref() == Some(path) {
            self.preset_browser.highlight = None;
        }
        self.preset_browser.refresh(path.parent());
        self.status = format!(
            "Deleted {}",
            entry.map_or_else(|| folder_name(path), |e| e.name)
        );
        true
    }

    /// Save `source` into the browser folder as `path` (PB-120..122). A name
    /// that's taken asks first unless `replace`. The new entry is shown at
    /// once (PB-46) and its thumbnail renders into the file.
    pub(super) fn browser_save(&mut self, path: PathBuf, source: PresetSource, replace: bool) {
        if self.refuse_read_only(&path) {
            return;
        }
        if path.exists() && !replace {
            self.preset_browser.ask = Some(Ask::Clash { path, source });
            return;
        }
        if !self.confirm_fields() {
            return;
        }
        let file = folder_name(&path);
        let center = std::mem::take(&mut self.preset_browser.save_centered);
        match self.editor.save_preset_placed(&path, source, center) {
            Ok(notes) => {
                self.status =
                    std::iter::once(format!("Saved {} preset {file}", source.kind().label()))
                        .chain(notes)
                        .collect::<Vec<_>>()
                        .join(" · ");
                self.preset_browser.thumbs.forget(&path, true);
                if let Some(parent) = path.parent() {
                    self.preset_browser.show_folder(parent.to_path_buf());
                    self.preset_browser.refresh(Some(parent));
                }
                self.preset_browser.highlight = Some(path.clone());
                // PB-T15: the thumbnail follows shortly, into the file.
                self.preset_browser
                    .thumbs
                    .render_into_file(&path, source.kind());
            }
            Err(e) => self.status = format!("Save preset {file}: {e}"),
        }
    }

    /// The preset kinds a drop of the current selection onto a folder can
    /// save (PB-45/PB-120): the selected items (Item and Assembly), the
    /// shader tree's material, Cancel always last in the menu.
    pub(super) fn save_choices(&self, from_shader_tree: bool) -> Vec<PresetSource> {
        let mut out = Vec::new();
        if from_shader_tree {
            if let Some(id) = self.selected_material_row() {
                out.push(PresetSource::Material(id));
            }
            return out;
        }
        for s in [PresetSource::Items, PresetSource::Assembly] {
            if s.build(&self.editor.scene).is_ok() {
                out.push(s);
            }
        }
        out
    }

    /// The suggested file stem for `source` (no extension).
    pub(super) fn preset_stem(&self, source: PresetSource) -> String {
        let name = self.preset_file_name(source);
        name.strip_suffix(&format!(".{PRESET_EXTENSION}"))
            .or_else(|| {
                name.strip_suffix(&format!(
                    ".{}",
                    operandi_core::preset::ENVIRONMENT_PRESET_EXTENSION
                ))
            })
            .unwrap_or(&name)
            .to_owned()
    }

    /// Ask a name for saving `source` into `folder` (PB-121).
    pub(super) fn ask_save_here(&mut self, folder: PathBuf, source: PresetSource) {
        if self.refuse_read_only(&folder) {
            return;
        }
        if let Err(e) = source.build(&self.editor.scene) {
            self.status = e;
            return;
        }
        let stem = self.preset_stem(source);
        let mut rows = vec![form::Row::text("name", "Name", &stem)];
        // PG-04: a mesh can be saved centred on the ground.
        if matches!(source, PresetSource::Items | PresetSource::Assembly) {
            rows.push(
                form::Row::check(CENTER_ON_GROUND, "Centre on Ground", false).tip(
                    "Saves the copy centred on X and Z, sitting on the ground (Y = 0). \
                     The mesh keeps no item position, rotation or scale either way.",
                ),
            );
        }
        let title = if self.selection_is_meshes() && source == PresetSource::Items {
            "Save Mesh Preset".to_owned()
        } else {
            format!("Save {} Preset", source.kind().label())
        };
        let form = form::Dialog::with_defaults(&title, rows).focus("name");
        self.preset_browser.ask = Some(Ask::SaveName {
            folder,
            source,
            form,
        });
    }

    /// Whether every selected item is a mesh (PG-02's "Save Mesh Preset…").
    pub(super) fn selection_is_meshes(&self) -> bool {
        let scene = &self.editor.scene;
        let ids: Vec<u64> = scene
            .selection
            .item
            .into_iter()
            .chain(scene.selection.also.iter().copied())
            .collect();
        !ids.is_empty()
            && ids
                .iter()
                .all(|id| scene.items.iter().any(|i| i.id == *id && i.is_mesh()))
    }

    /// Keep the browser's own state up to date each frame.
    pub(super) fn tick_preset_browser(&mut self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        let busy = self.render.busy() || self.preview_busy();
        self.preset_browser
            .thumbs
            .render_allowed
            .store(!busy, Ordering::Relaxed);
        *self
            .preset_browser
            .thumbs
            .resolution
            .lock()
            .expect("resolution") = self.preset_browser.resolution;
        // MC-02: a copy into the library says when it's done, shown or not.
        if let Some(message) = self.preset_browser.poll_copy() {
            self.status = message;
            ctx.request_repaint();
        } else if self.preset_browser.copy_job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        // MC-05, MC-08: the view's Matcap menu needs the library's
        // matcaps whether or not the browser is open.
        if self.preset_browser.scan_matcaps(now, false) {
            ctx.request_repaint();
        }
        self.note_missing_matcap();
        let shown = self.preset_browser.window_open
            || self.panel_shown(crate::panel_registry::PanelKind::PresetBrowser);
        if !shown {
            // Other panels' thumbnails (the Image Map's Image list) still
            // arrive while the browser is hidden.
            if self.preset_browser.thumbs.poll(ctx) {
                ctx.request_repaint();
            }
            if self.preset_browser.thumbs.busy() {
                ctx.request_repaint_after(std::time::Duration::from_millis(60));
            }
            return;
        }
        let mut changed = self.preset_browser.tick(now);
        changed |= self.preset_browser.thumbs.poll(ctx);
        if changed {
            ctx.request_repaint();
        }
        if self.preset_browser.listing_busy() || self.preset_browser.thumbs.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(CHECK_SECONDS));
        }
    }

    /// The `pbview.*`, `file.*`, `layout.window` and
    /// `operandi.pbview.thumbnailSize` commands (UIC-160), and `preset.do`
    /// on anything that isn't an `.oppreset` file. Returns whether `text`
    /// was one of them.
    pub(super) fn preset_browser_canonical(&mut self, text: &str) -> bool {
        let Ok(args) = command::tokens(text) else {
            return false;
        };
        let Some(first) = args.first().map(|a| a.to_ascii_lowercase()) else {
            return false;
        };
        if first == "help"
            && args.len() == 2
            && let Some((_, help)) = BROWSER_COMMANDS
                .iter()
                .find(|(c, _)| c.eq_ignore_ascii_case(&args[1]))
        {
            self.status = (*help).into();
            return true;
        }
        let rest = &args[1..];
        let bind = |names: &[&str]| operandi_core::command_args::bind(&args[0], rest, names);
        let flag = |v: Option<&str>| -> Option<bool> {
            match v?.to_ascii_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => Some(true),
                "false" | "0" | "off" | "no" => Some(false),
                _ => None,
            }
        };
        let result: std::result::Result<(), String> = (|| {
            match first.as_str() {
                "layout.window" => {
                    // Every palette (LAY-50, palettes.rs); the Preset
                    // Browser is hash:presetBrowser.
                    let b = bind(&["hash", "open"])?;
                    let open = match b[1] {
                        None => None,
                        Some(v) => Some(flag(Some(v)).ok_or("open: is true or false")?),
                    };
                    self.palette_window_line(b[0], open)?;
                }
                "pbview.basepathadd" => {
                    let b = bind(&["path", "basePathKey", "set"])?;
                    let path = PathBuf::from(b[0].ok_or("pbview.basePathAdd needs path:<folder>")?);
                    if !path.is_dir() {
                        return Err(format!("No folder at {}", path.display()));
                    }
                    self.status = self.preset_browser.add_root(path, b[1].map(str::to_owned));
                }
                "pbview.basepathremove" => {
                    let b = bind(&["path", "basePathKey"])?;
                    let path = match (b[0], b[1]) {
                        (Some(p), _) => PathBuf::from(p),
                        (None, Some(label)) => self
                            .preset_browser
                            .roots
                            .iter()
                            .find(|r| r.label.as_deref() == Some(label))
                            .map(|r| r.path.clone())
                            .ok_or(format!("No root labelled {label}"))?,
                        _ => return Err("pbview.basePathRemove needs path:<folder>".into()),
                    };
                    if self.preset_browser.read_only(&path) {
                        return Err(format!("{READ_ONLY_NOTE}; they can't be removed"));
                    }
                    if !self.preset_browser.remove_root(&path) {
                        return Err(format!("{} isn't a Preset Browser root", path.display()));
                    }
                    self.status = format!(
                        "Removed {} from the Preset Browser; its files are untouched",
                        path.display()
                    );
                }
                "pbview.refresh" => {
                    bind(&[])?;
                    self.preset_browser.refresh(None);
                    self.status = "Preset Browser refreshed".into();
                }
                "pbview.mode" => {
                    let b = bind(&["mode"])?;
                    let v = b[0].ok_or("pbview.mode needs mode:<vertical|list|horizontal>")?;
                    if v == "?" {
                        self.status = self.preset_browser.view.script_name().into();
                    } else {
                        self.preset_browser.view = ViewMode::from_script(v).ok_or(
                            "pbview.mode: mode is vertical (0), list (1) or horizontal (2)",
                        )?;
                    }
                }
                "pbview.sortmode" => {
                    let b = bind(&["mode"])?;
                    let v = b[0].ok_or("pbview.sortMode needs mode:<name|type|modified|size>")?;
                    self.preset_browser.sort = match v.parse::<usize>() {
                        Ok(n) => *SortBy::ALL.get(n).ok_or("pbview.sortMode: 0 to 3")?,
                        Err(_) => SortBy::from_script_name(v)
                            .ok_or("pbview.sortMode: name, type, modified or size")?,
                    };
                }
                "pbview.filterstring" => {
                    let b = bind(&["filter"])?;
                    self.preset_browser.search = b[0].unwrap_or("").to_owned();
                    self.preset_browser.search_open = true;
                    self.preset_browser.search_changed();
                }
                "pbview.filterstringenable" => {
                    let b = bind(&["state"])?;
                    self.preset_browser.search_open = match b[0] {
                        None => !self.preset_browser.search_open,
                        Some(v) => flag(Some(v)).ok_or("state: is true or false")?,
                    };
                    self.preset_browser.search_changed();
                }
                "operandi.pbview.kindfilter" => {
                    let b = bind(&["kind"])?;
                    let v = b[0].ok_or(
                        "operandi.pbview.kindFilter needs kind:<all|meshes|materials|environments|images|scenes>",
                    )?;
                    if v == "?" {
                        self.status = self.preset_browser.kind_filter.script_name().into();
                    } else {
                        let bucket = Bucket::from_script(v).ok_or(
                            "operandi.pbview.kindFilter: kind is all (0), meshes (1), materials (2), environments (3), images (4) or scenes (5)",
                        )?;
                        self.preset_browser.set_kind_filter(bucket);
                    }
                }
                "operandi.pbview.onlyusable" => {
                    let b = bind(&["state"])?;
                    if b[0] == Some("?") {
                        self.status = self.preset_browser.only_usable.to_string();
                    } else {
                        let on = match b[0] {
                            None => !self.preset_browser.only_usable,
                            Some(v) => flag(Some(v)).ok_or("state: is true or false")?,
                        };
                        self.preset_browser.set_only_usable(on);
                    }
                }
                "dircache.copy" => {
                    let b = bind(&["source", "dest", "overwrite"])?;
                    let source = PathBuf::from(b[0].ok_or("dircache.copy needs source:<folder>")?);
                    let dest = match b[1] {
                        Some(d) => PathBuf::from(d),
                        None => self
                            .preset_browser
                            .library
                            .clone()
                            .ok_or("dircache.copy needs dest:<folder>")?,
                    };
                    let overwrite = match b[2] {
                        None => false,
                        Some(v) => flag(Some(v)).ok_or("overwrite: is true or false")?,
                    };
                    std::fs::create_dir_all(&dest)
                        .map_err(|e| format!("Can't make {}: {e}", dest.display()))?;
                    self.status = self.preset_browser.start_copy(&source, &dest, overwrite)?;
                }
                "operandi.pbview.libraryget" => {
                    let b = bind(&["url"])?;
                    self.browser_get_library(b[0].unwrap_or(LIBRARY_URL))?;
                }
                "operandi.pbview.thumbnailsize" => {
                    let b = bind(&["size"])?;
                    let size: f32 = b[0]
                        .ok_or("operandi.pbview.thumbnailSize needs size:<points>")?
                        .parse()
                        .map_err(|_| "size: is a number of points")?;
                    self.preset_browser.thumb_size = size.clamp(THUMB_MIN, THUMB_MAX);
                }
                "file.rename" | "file.duplicate" => {
                    let b = bind(&["newName", "path"])?;
                    let name = b[0].ok_or(format!("{} needs newName:<name>", args[0]))?;
                    let path = self.browser_command_path(b[1])?;
                    let ok = if first == "file.rename" {
                        self.browser_rename(&path, name)
                    } else {
                        self.browser_duplicate(&path, name)
                    };
                    if !ok {
                        return Err(std::mem::take(&mut self.status));
                    }
                }
                "file.delete" => {
                    let b = bind(&["path"])?;
                    let path = self.browser_command_path(b[0])?;
                    if !self.browser_delete(&path) {
                        return Err(std::mem::take(&mut self.status));
                    }
                }
                _ => return Err(String::new()),
            }
            Ok(())
        })();
        match result {
            Ok(()) => true,
            Err(e) if e.is_empty() => false,
            Err(e) => {
                self.status = e;
                true
            }
        }
    }

    /// `path:` given, else the highlighted entry.
    fn browser_command_path(&self, given: Option<&str>) -> std::result::Result<PathBuf, String> {
        match given {
            Some(p) => {
                let p = PathBuf::from(p);
                if p.exists() {
                    Ok(p)
                } else {
                    Err(format!("No file at {}", p.display()))
                }
            }
            None => {
                self.preset_browser.highlight.clone().ok_or_else(|| {
                    "Select an entry in the Preset Browser, or give path:<file>".into()
                })
            }
        }
    }

    /// `preset.do preset:<file> [target:<group>]` for files that aren't
    /// presets (texture-set maps, HDRs, pictures, scenes): loads them as
    /// the browser would. `target:` (an Operandi extra) names the material
    /// group a material goes into.
    pub(super) fn preset_do_any(&mut self, path: &Path, target: Option<&str>) -> bool {
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(operandi_core::preset::is_preset_extension)
        {
            return false;
        }
        if !path.exists() {
            self.status = format!("No file at {}", path.display());
            return true;
        }
        let entry = self.entry_for_path(path);
        if let Some(group) = target
            && !self.select_group_named(group)
        {
            return true;
        }
        self.browser_load(&entry, Target::Default);
        true
    }

    /// The browser's entry for `path`: from the listing, or read from its
    /// folder (a texture-set map stands for its set).
    pub(super) fn entry_for_path(&self, path: &Path) -> Entry {
        if let Some(e) = self.preset_browser.entry(path) {
            return e;
        }
        let kind = |p: &Path| {
            operandi_core::persistence::read_preset_header(p)
                .ok()
                .map(|h| h.kind)
        };
        if let Some(parent) = path.parent()
            && let Ok(listing) =
                library::list_folder(parent, ListOptions { only_usable: false }, &kind, &|| false)
            && let Some(e) = listing.entries.into_iter().find(|e| {
                e.path == path
                    || e.texture_set
                        .as_ref()
                        .is_some_and(|s| s.files().any(|f| f == path))
            })
        {
            return e;
        }
        Entry {
            name: path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.to_path_buf(),
            kind: library::classify(path, &kind),
            modified: None,
            size: 0,
            texture_set: None,
            modo_kind: library::modo_kind(path, library::classify(path, &kind)),
        }
    }

    /// MC-02: a folder dropped from the Finder or Explorer at `at` (in the
    /// window `in_window` says). Over the browser it asks Add as Location /
    /// Copy Into Library / Cancel; returns false when it isn't over the
    /// browser.
    pub(super) fn browser_folder_dropped(
        &mut self,
        folder: PathBuf,
        at: Option<Pos2>,
        in_window: bool,
    ) -> bool {
        let Some((rect, window)) = self.preset_browser.body_rect else {
            return false;
        };
        let Some(at) = at.filter(|p| window == in_window && rect.contains(*p)) else {
            return false;
        };
        self.preset_browser.folder_drop = Some(FolderDrop {
            folder,
            at,
            in_window,
        });
        true
    }

    /// MC-02 Copy Into Library from the drop menu: asks before
    /// overwriting a folder of that name.
    pub(super) fn browser_copy_to_library(&mut self, folder: &Path, overwrite: bool) {
        let Some(library) = self.preset_browser.library.clone() else {
            self.status = "No library folder".into();
            return;
        };
        if !overwrite && library.join(folder_name(folder)).exists() {
            self.preset_browser.ask = Some(Ask::CopyOverwrite(folder.to_path_buf()));
            return;
        }
        if let Err(e) = std::fs::create_dir_all(&library) {
            self.status = format!("Can't make {}: {e}", library.display());
            return;
        }
        self.status = match self.preset_browser.start_copy(folder, &library, overwrite) {
            Ok(m) | Err(m) => m,
        };
    }
}

/// Help topics (`help <command>`).
pub(crate) const BROWSER_COMMANDS: [(&str, &str); 16] = [
    (
        "layout.window",
        "Help: layout.window hash:<palette> [open:<true|false>] opens or closes a palette (presetBrowser, uvEditor, toolbox, items, properties, shading, groups, vertexMaps, commandHistory, uvView); bare open: toggles, open:true brings an open one forward.",
    ),
    (
        "pbview.basePathAdd",
        "Help: pbview.basePathAdd path:<folder> [basePathKey:<label>] adds a folder as a Preset Browser root; an Operandi Library, or a folder of them, adds its packs.",
    ),
    (
        "pbview.basePathRemove",
        "Help: pbview.basePathRemove path:<folder> takes a root out of the Preset Browser; its files are untouched.",
    ),
    (
        "pbview.refresh",
        "Help: pbview.refresh reads the Preset Browser's folders again.",
    ),
    (
        "pbview.mode",
        "Help: pbview.mode mode:<thumbnails|list> picks the Preset Browser's view (0 thumbnails, 1 list).",
    ),
    (
        "pbview.sortMode",
        "Help: pbview.sortMode mode:<name|type|modified|size> sorts the Preset Browser's contents.",
    ),
    (
        "pbview.filterString",
        "Help: pbview.filterString filter:<text> searches the shown folder and everything below it.",
    ),
    (
        "pbview.filterStringEnable",
        "Help: pbview.filterStringEnable state:<true|false> shows or hides the Preset Browser's search.",
    ),
    (
        "operandi.pbview.libraryGet",
        "Help: operandi.pbview.libraryGet [url:<address>] opens the free Operandi Library's web page (or url:) in your web browser; Operandi downloads nothing, and it isn't undoable.",
    ),
    (
        "operandi.pbview.thumbnailSize",
        "Help: operandi.pbview.thumbnailSize size:<points> sets the Preset Browser's thumbnail size (48 to 192).",
    ),
    (
        "operandi.pbview.kindFilter",
        "Help: operandi.pbview.kindFilter kind:<all|meshes|materials|environments|images|scenes> shows only that kind in the Preset Browser (0 to 5 in that order; ? returns it).",
    ),
    (
        "operandi.pbview.onlyUsable",
        "Help: operandi.pbview.onlyUsable [state:<true|false>] hides (true) or lists (false) files the Preset Browser can't use; bare toggles, ? returns it.",
    ),
    (
        "dircache.copy",
        "Help: dircache.copy source:<folder> [dest:<folder>] [overwrite:<true|false>] copies a folder into the library folder (or dest:) in the background; an existing folder of that name needs overwrite:true.",
    ),
    (
        "file.rename",
        "Help: file.rename newName:<name> [path:<file>] renames the selected Preset Browser entry (a texture set's maps together).",
    ),
    (
        "file.duplicate",
        "Help: file.duplicate newName:<name> [path:<file>] copies the selected Preset Browser entry under a new name.",
    ),
    (
        "file.delete",
        "Help: file.delete [path:<file>] deletes the selected Preset Browser entry from disk.",
    ),
];

#[cfg(test)]
#[path = "preset_browser_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "preset_browser_modo_tests.rs"]
mod modo_tests;
