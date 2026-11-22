//! Preset browser Stage 1, the part without a window: what a folder of
//! content holds (BEHAVIOR-PRESET-BROWSER-STAGE1 PB-104..107, CONTENT-
//! DIRECTORIES CD-01..27).
//!
//! - [`list_folder`] reads one folder: presets by their kind header
//!   (CD-10), scenes and importable models (CD-11), images (CD-12), HDR
//!   images as environments (CD-13), and texture sets folded into one
//!   material entry (CD-14, CD-20..24). Hidden files and OS junk are never
//!   listed (CD-04). Nothing here writes to disk (CD-T5).
//! - [`TextureSet::material_preset`] turns a texture set into an in-memory
//!   Material preset, so loading one goes through the same `LoadPreset`
//!   command as a `.oppreset` file (one undo step, PB-35; CD-25).
//!
//! Provisional readings are marked and listed as PB1-Q.. in
//! docs/SPECIFICATION_QUESTIONS.md.
use super::PresetKind;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Scene files the browser opens as their own scene (CD-11).
pub const SCENE_EXTENSIONS: [&str; 2] = ["opscene", "lxo"];
/// Pictures (CD-12). HDR ones are environments unless they belong to a
/// texture set (CD-13).
pub const IMAGE_EXTENSIONS: [&str; 9] = [
    "png", "jpg", "jpeg", "tif", "tiff", "bmp", "tga", "exr", "hdr",
];
/// High dynamic range pictures (CD-13).
pub const HDR_EXTENSIONS: [&str; 2] = ["hdr", "exr"];

/// What an entry in the contents area is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntryKind {
    /// An `.oppreset` file of this kind (CD-10).
    Preset(PresetKind),
    /// An `.oppreset` file whose header can't be read: listed honestly,
    /// loading reports why.
    BrokenPreset,
    /// An Operandi scene or another scene file Open reads (CD-11).
    Scene,
    /// A model File > Import reads (CD-11).
    Model,
    /// A picture (CD-12).
    Image,
    /// A picture in or below a folder named "Matcaps": a lit ball the
    /// Matcap display style uses (MC-01).
    Matcap,
    /// An HDR picture, shown as an environment (CD-13).
    Environment,
    /// Several pictures that make one material (CD-14, CD-20).
    TextureSet,
    /// A MaterialX file, loaded as a material (MX-07). When it reads a
    /// texture set's pictures it stands for that set.
    MaterialX,
    /// A Modo mesh preset (`.lxl`, MC-11), added as a new mesh item
    /// through the LXO reader.
    ModoMeshPreset,
    /// A Modo material preset (`.lxp` whose `DESC` names a material and
    /// that holds a material item, LXP-01): converted to an Operandi
    /// Material preset when loaded (`preset::lxp`).
    ModoMaterial,
    /// A Modo file Operandi can't use yet (MC-12): a generic preset
    /// (`.lxp`), an environment preset (`.lxe`) or a web link (`.lrl`).
    /// Listed greyed with its own thumbnail and kind (`Entry::modo_kind`)
    /// when "Show only usable files" is off.
    ModoPreset,
    /// Anything else (CD-15), shown in list view only.
    Other,
}

impl EntryKind {
    /// The kind's word for the list view and tooltips.
    pub fn label(self) -> &'static str {
        match self {
            EntryKind::Preset(k) => match k {
                PresetKind::Material => "Material Preset",
                PresetKind::Item => "Item Preset",
                PresetKind::Assembly => "Assembly Preset",
                PresetKind::Environment => "Environment Preset",
                PresetKind::RenderSettings => "Render Settings Preset",
            },
            EntryKind::BrokenPreset => "Preset (unreadable)",
            EntryKind::Scene => "Scene",
            EntryKind::Model => "Model",
            EntryKind::Image => "Image",
            EntryKind::Matcap => "Matcap",
            EntryKind::Environment => "Environment Image",
            EntryKind::TextureSet => "Texture Set",
            EntryKind::MaterialX => "MaterialX Material",
            EntryKind::ModoMeshPreset => "Modo Mesh Preset",
            EntryKind::ModoMaterial => "Modo Material Preset",
            EntryKind::ModoPreset => "Modo Preset",
            EntryKind::Other => "File",
        }
    }
    /// Whether the browser can do something with it (CD-15's "Show only
    /// usable files").
    pub fn usable(self) -> bool {
        !matches!(
            self,
            EntryKind::Other | EntryKind::BrokenPreset | EntryKind::ModoPreset
        )
    }
    /// Whether it gets a picture in thumbnail view, rather than only its
    /// kind's placeholder. Scenes and models get a rendered one (PG-05).
    pub fn has_picture(self) -> bool {
        matches!(
            self,
            EntryKind::Preset(_)
                | EntryKind::Scene
                | EntryKind::Model
                | EntryKind::Image
                | EntryKind::Matcap
                | EntryKind::Environment
                | EntryKind::TextureSet
                | EntryKind::MaterialX
                | EntryKind::ModoMeshPreset
                | EntryKind::ModoMaterial
                | EntryKind::ModoPreset
        )
    }
}

/// One entry in a folder's contents.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// What the browser shows under the thumbnail: the file name without
    /// its extension, or a texture set's base name (CD-14).
    pub name: String,
    /// The file. For a texture set: its base colour map, or its first map.
    pub path: PathBuf,
    pub kind: EntryKind,
    pub modified: Option<SystemTime>,
    /// Bytes on disk (a texture set: every map together).
    pub size: u64,
    /// Set for [`EntryKind::TextureSet`], and for a MaterialX file that
    /// reads a texture set's pictures (its thumbnail).
    pub texture_set: Option<TextureSet>,
    /// For Modo files: the kind their header names ("Material preset",
    /// MC-12), for tooltips and the list view.
    pub modo_kind: Option<String>,
}

impl Entry {
    /// A Modo scene (`.lxo`, MC-10).
    pub fn is_modo_scene(&self) -> bool {
        self.kind == EntryKind::Scene && extension(&self.path) == "lxo"
    }
    /// The kind words shown for the entry: a Modo file's own kind
    /// ("Modo material preset", MC-40), else its kind's label.
    pub fn kind_label(&self) -> String {
        match (&self.modo_kind, self.kind) {
            (Some(k), _) => format!("Modo {}", lower_first(k)),
            (None, EntryKind::Scene) if self.is_modo_scene() => "Modo Scene".into(),
            _ => self.kind.label().into(),
        }
    }
    /// MC-12: why a greyed Modo entry does nothing. LXP-04: a material
    /// preset that holds no material Operandi can read says so.
    pub fn unsupported_note(&self) -> Option<String> {
        if self.kind != EntryKind::ModoPreset {
            return None;
        }
        if extension(&self.path) == super::lxp::EXTENSION
            && self
                .modo_kind
                .as_deref()
                .is_some_and(|k| k.to_ascii_lowercase().starts_with("material"))
        {
            return Some(super::lxp::NO_MATERIAL.into());
        }
        Some(format!("{}. Not supported yet.", self.kind_label()))
    }
    /// Which of the kind filter's buckets the entry is in (MC-20).
    pub fn in_bucket(&self, bucket: Bucket) -> bool {
        let modo = self.modo_kind.as_deref().unwrap_or("").to_ascii_lowercase();
        match bucket {
            Bucket::All => true,
            Bucket::Meshes => {
                matches!(
                    self.kind,
                    EntryKind::Model
                        | EntryKind::ModoMeshPreset
                        | EntryKind::Preset(PresetKind::Item)
                ) || self.is_modo_scene()
            }
            Bucket::Materials => {
                matches!(
                    self.kind,
                    EntryKind::Preset(PresetKind::Material)
                        | EntryKind::TextureSet
                        | EntryKind::MaterialX
                        | EntryKind::ModoMaterial
                ) || self.kind == EntryKind::ModoPreset && modo.starts_with("material")
            }
            Bucket::Environments => {
                matches!(
                    self.kind,
                    EntryKind::Environment | EntryKind::Preset(PresetKind::Environment)
                ) || self.kind == EntryKind::ModoPreset && modo.starts_with("environment")
            }
            Bucket::Images => self.kind == EntryKind::Image,
            Bucket::Scenes => self.kind == EntryKind::Scene,
        }
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

/// The browser's kind filter (MC-20): which entries the contents area
/// shows. A Modo content folder feeds these like any other root.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Bucket {
    #[default]
    All,
    Meshes,
    Materials,
    Environments,
    Images,
    Scenes,
}

impl Bucket {
    /// The menu's order. The integers `operandi.pbview.kindFilter` takes
    /// are these positions: FIXED, only add new ones at the end.
    pub const ALL: [Bucket; 6] = [
        Bucket::All,
        Bucket::Meshes,
        Bucket::Materials,
        Bucket::Environments,
        Bucket::Images,
        Bucket::Scenes,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Bucket::All => "All",
            Bucket::Meshes => "Meshes",
            Bucket::Materials => "Materials",
            Bucket::Environments => "Environments",
            Bucket::Images => "Images",
            Bucket::Scenes => "Scenes",
        }
    }
    pub fn script_name(self) -> &'static str {
        match self {
            Bucket::All => "all",
            Bucket::Meshes => "meshes",
            Bucket::Materials => "materials",
            Bucket::Environments => "environments",
            Bucket::Images => "images",
            Bucket::Scenes => "scenes",
        }
    }
    /// A name or its position in [`Bucket::ALL`].
    pub fn from_script(text: &str) -> Option<Self> {
        if let Ok(n) = text.parse::<usize>() {
            return Self::ALL.get(n).copied();
        }
        Self::ALL
            .into_iter()
            .find(|b| b.script_name().eq_ignore_ascii_case(text))
    }
    /// What an empty bucket says (MC-21); `None` keeps the usual empty
    /// state.
    pub fn empty_hint(self) -> Option<&'static str> {
        (self == Bucket::Meshes).then_some("Add a folder of models to see them here.")
    }
}

/// Which way the contents are sorted (PB-105; manual and star sort are
/// Stage 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortBy {
    #[default]
    Name,
    Type,
    Modified,
    Size,
}

impl SortBy {
    pub const ALL: [SortBy; 4] = [SortBy::Name, SortBy::Type, SortBy::Modified, SortBy::Size];
    pub fn label(self) -> &'static str {
        match self {
            SortBy::Name => "Name",
            SortBy::Type => "File Type",
            SortBy::Modified => "Last Modified",
            SortBy::Size => "File Size",
        }
    }
    /// The script spelling (`sort:name`).
    pub fn script_name(self) -> &'static str {
        match self {
            SortBy::Name => "name",
            SortBy::Type => "type",
            SortBy::Modified => "modified",
            SortBy::Size => "size",
        }
    }
    pub fn from_script_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|s| s.script_name().eq_ignore_ascii_case(name))
    }
}

/// Sort `entries` (PB-105). Name is case-insensitive and natural
/// ("Brick 2" before "Brick 10"); Last Modified is newest first (PB-24);
/// File Size is largest first [chosen]. Ties fall back to the name.
pub fn sort(entries: &mut [Entry], by: SortBy) {
    let name = |a: &Entry, b: &Entry| natural_cmp(&a.name, &b.name);
    match by {
        SortBy::Name => entries.sort_by(name),
        SortBy::Type => {
            entries.sort_by(|a, b| a.kind.label().cmp(b.kind.label()).then_with(|| name(a, b)))
        }
        SortBy::Modified => {
            entries.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| name(a, b)))
        }
        SortBy::Size => entries.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| name(a, b))),
    }
}

/// CD-04: hidden files and OS junk are never shown.
pub fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
        || name.eq_ignore_ascii_case("Thumbs.db")
        || name.eq_ignore_ascii_case("desktop.ini")
        || name.eq_ignore_ascii_case("$RECYCLE.BIN")
        || name.starts_with("~$")
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// What a single file is, from its extension and (for presets) its header.
/// `read_preset_kind` is given the path of an `.oppreset` file.
pub fn classify(path: &Path, read_preset_kind: &dyn Fn(&Path) -> Option<PresetKind>) -> EntryKind {
    let ext = extension(path);
    if super::is_preset_extension(&ext) {
        return read_preset_kind(path).map_or(EntryKind::BrokenPreset, EntryKind::Preset);
    }
    if SCENE_EXTENSIONS.contains(&ext.as_str()) {
        return EntryKind::Scene;
    }
    if ext == crate::materialx::EXTENSION {
        return EntryKind::MaterialX;
    }
    if crate::interchange::IMPORT_EXTENSIONS.contains(&ext.as_str()) {
        return EntryKind::Model;
    }
    if ext == super::modo::MESH_PRESET_EXTENSION {
        return EntryKind::ModoMeshPreset;
    }
    if ext == super::lxp::EXTENSION && is_modo_material(path) {
        return EntryKind::ModoMaterial;
    }
    if super::modo::UNSUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return EntryKind::ModoPreset;
    }
    if HDR_EXTENSIONS.contains(&ext.as_str()) {
        return EntryKind::Environment;
    }
    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return EntryKind::Image;
    }
    EntryKind::Other
}

/// LXP-01/04: an `.lxp` whose `DESC` names a material type and whose
/// items include a material the converter reads. Reads the file (presets
/// are small); a broken one stays a greyed Modo preset.
pub fn is_modo_material(path: &Path) -> bool {
    let named = super::modo::read_info(path)
        .and_then(|i| i.item_type)
        .is_some_and(|t| super::lxp::is_material_type(&t));
    named
        && std::fs::read(path)
            .ok()
            .and_then(|b| crate::lxo::preset_items::read(&b).ok())
            .is_some_and(|p| super::lxp::has_material(&p))
}

/// MC-13: scripts, macros and config files never show, with "Show only
/// usable files" on or off.
pub fn never_shown(path: &Path) -> bool {
    super::modo::NEVER_SHOWN.contains(&extension(path).as_str())
}

/// The kind a Modo preset's header names (MC-12), read from its `FORM`
/// header and `DESC` only (MC-03). Web links aren't `FORM` files.
pub fn modo_kind(path: &Path, kind: EntryKind) -> Option<String> {
    match kind {
        EntryKind::ModoMeshPreset => return Some("Mesh preset".into()),
        EntryKind::ModoMaterial => return Some("Material preset".into()),
        EntryKind::ModoPreset => {}
        _ => return None,
    }
    if extension(path) == "lrl" {
        return Some("Web link".into());
    }
    match super::modo::read_info(path).map(|i| i.kind_label()) {
        Some(l) if l != "Preset" && l != "Scene" => Some(l),
        _ if extension(path) == "lxe" => Some("Environment preset".into()),
        _ => Some("Preset".into()),
    }
}

/// The folder name that makes its pictures matcaps (MC-01), any case.
pub const MATCAP_FOLDER: &str = "Matcaps";

/// Whether `folder` is a folder named "Matcaps" (any case) or below one
/// (MC-01).
pub fn in_matcap_folder(folder: &Path) -> bool {
    folder.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(MATCAP_FOLDER)
    })
}

/// Whether `path` is a picture the Matcap style can use (MC-01, MC-07):
/// any picture, HDR ones too, in or below a "Matcaps" folder.
pub fn is_matcap(path: &Path) -> bool {
    IMAGE_EXTENSIONS.contains(&extension(path).as_str())
        && path.parent().is_some_and(in_matcap_folder)
}

/// A matcap's sidecar note (MC-11): `name.txt` beside `name.png`, holding
/// its licence, author and source. Read as plain text, trimmed, at most
/// 2 KB; `None` when there's none or it's empty.
pub fn matcap_note(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path.with_extension("txt")).ok()?;
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(2048)]);
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// One matcap found by [`find_matcaps`]: its group (the folders between
/// the "Matcaps" folder and it, or "Matcaps" when it sits right in one),
/// its name and its file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundMatcap {
    pub group: String,
    pub name: String,
    pub path: PathBuf,
}

/// Every matcap in or below `root` (MC-01, MC-05), sorted by group then
/// name. Hidden folders are skipped, and the walk stops `MAX_DEPTH`
/// folders down and when `stop` says so.
pub fn find_matcaps(root: &Path, stop: &dyn Fn() -> bool) -> Vec<FoundMatcap> {
    const MAX_DEPTH: usize = 8;
    let mut found = Vec::new();
    // (folder, depth, the folder's path below its Matcaps folder if in one)
    let mut queue = vec![(
        root.to_path_buf(),
        0,
        in_matcap_folder(root).then(Vec::<String>::new),
    )];
    while let Some((folder, depth, below)) = queue.pop() {
        if stop() {
            break;
        }
        let Ok(read) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in read.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_hidden(&name) {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                if depth < MAX_DEPTH {
                    let below = match &below {
                        Some(b) => Some([b.as_slice(), std::slice::from_ref(&name)].concat()),
                        None => name.eq_ignore_ascii_case(MATCAP_FOLDER).then(Vec::new),
                    };
                    queue.push((path, depth + 1, below));
                }
            } else if let Some(b) = &below
                && IMAGE_EXTENSIONS.contains(&extension(&path).as_str())
            {
                found.push(FoundMatcap {
                    group: if b.is_empty() {
                        MATCAP_FOLDER.to_owned()
                    } else {
                        b.join(" / ")
                    },
                    name: stem(&path),
                    path,
                });
            }
        }
    }
    found.sort_by(|a, b| {
        natural_cmp(&a.group, &b.group).then_with(|| natural_cmp(&a.name, &b.name))
    });
    found
}

/// The fresh-install folders of the user's Presets root (PG-01, MC-09),
/// created empty so people can drop their own content in.
pub const STARTER_FOLDERS: [&str; 5] = ["Meshes", "Materials", "Matcaps", "Colours", "Scenes"];

/// Create [`STARTER_FOLDERS`] in `root` when `root` doesn't exist yet or
/// holds nothing (a fresh install). A root that already holds anything is
/// left as it is, so folders the user deleted aren't put back. Returns
/// whether it made them.
pub fn make_starter_folders(root: &Path) -> std::io::Result<bool> {
    let empty = match std::fs::read_dir(root) {
        Ok(mut entries) => {
            entries.all(|e| e.is_ok_and(|e| is_hidden(&e.file_name().to_string_lossy())))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => return Err(e),
    };
    if !empty {
        return Ok(false);
    }
    for name in STARTER_FOLDERS {
        std::fs::create_dir_all(root.join(name))?;
    }
    Ok(true)
}

/// A folder's contents as read from disk.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Listing {
    /// Subfolders (for the folder tree), sorted by name, hidden ones left out.
    pub folders: Vec<PathBuf>,
    pub entries: Vec<Entry>,
}

/// Options that change what a listing holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListOptions {
    /// CD-15: leave out files the browser can't use (default on).
    pub only_usable: bool,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self { only_usable: true }
    }
}

/// The subfolders of `folder` (hidden ones left out), sorted by name.
pub fn subfolders(folder: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(folder)?
        .filter_map(|e| e.ok())
        .filter(|e| !is_hidden(&e.file_name().to_string_lossy()))
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()) || e.path().is_dir())
        .map(|e| e.path())
        .collect();
    out.sort_by(|a, b| natural_cmp(&file_name(a), &file_name(b)));
    Ok(out)
}

/// Read `folder` (CD-04, CD-10..24). `stop` is checked between files so a
/// big folder's listing can be dropped when the user moves on.
pub fn list_folder(
    folder: &Path,
    options: ListOptions,
    read_preset_kind: &dyn Fn(&Path) -> Option<PresetKind>,
    stop: &dyn Fn() -> bool,
) -> std::io::Result<Listing> {
    let mut folders = Vec::new();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        if stop() {
            break;
        }
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_hidden(&name) {
            continue;
        }
        let path = entry.path();
        if never_shown(&path) {
            continue;
        }
        let meta = std::fs::metadata(&path).ok();
        if meta.as_ref().is_some_and(|m| m.is_dir()) {
            folders.push(path);
            continue;
        }
        let (modified, size) = meta
            .map(|m| (m.modified().ok(), m.len()))
            .unwrap_or((None, 0));
        files.push((path, modified, size));
    }
    folders.sort_by(|a, b| natural_cmp(&file_name(a), &file_name(b)));
    // MC-01: in a Matcaps folder every picture is a matcap; none of them
    // are read as texture-set maps.
    if in_matcap_folder(folder) {
        let mut entries = Vec::new();
        for (path, modified, size) in files {
            let picture = IMAGE_EXTENSIONS.contains(&extension(&path).as_str());
            let kind = if picture {
                EntryKind::Matcap
            } else {
                classify(&path, read_preset_kind)
            };
            // A matcap's sidecar note (MC-11) belongs to it; not listed.
            let sidecar = extension(&path) == "txt"
                && IMAGE_EXTENSIONS
                    .iter()
                    .any(|e| path.with_extension(e).is_file());
            if sidecar || (options.only_usable && !kind.usable()) {
                continue;
            }
            let modo_kind = modo_kind(&path, kind);
            entries.push(Entry {
                name: stem(&path),
                path,
                kind,
                modified,
                size,
                texture_set: None,
                modo_kind,
            });
        }
        return Ok(Listing { folders, entries });
    }
    // Texture sets first: their maps leave the loose pictures (CD-14).
    let pictures: Vec<&Path> = files
        .iter()
        .map(|(p, _, _)| p.as_path())
        .filter(|p| IMAGE_EXTENSIONS.contains(&extension(p).as_str()))
        .collect();
    let mut sets = texture_sets(&pictures);
    let mut in_set = std::collections::HashSet::new();
    let mut entries = Vec::new();
    // MX-07: a .mtlx that reads a set's pictures beats guessing from their
    // names; it stands for the set (its name and thumbnail) and the set
    // isn't listed again.
    for (path, modified, size) in files
        .iter()
        .filter(|(p, _, _)| extension(p) == crate::materialx::EXTENSION)
    {
        let Ok(doc) = crate::materialx::read(path) else {
            continue;
        };
        let reads = crate::materialx::pictures(&doc, folder);
        let Some(i) = sets
            .iter()
            .position(|s| s.files().any(|f| reads.iter().any(|r| same_file(r, f))))
        else {
            continue;
        };
        let set = sets.remove(i);
        for f in set.files() {
            in_set.insert(f.to_path_buf());
        }
        in_set.insert(path.clone());
        entries.push(Entry {
            name: set.name.clone(),
            path: path.clone(),
            kind: EntryKind::MaterialX,
            modified: *modified,
            size: *size,
            texture_set: Some(set),
            modo_kind: None,
        });
    }
    for set in sets {
        let mut size = 0;
        let mut modified = None;
        for path in set.files() {
            in_set.insert(path.to_path_buf());
            if let Some((_, m, s)) = files.iter().find(|(p, _, _)| p == path) {
                size += s;
                modified = modified.max(*m);
            }
        }
        entries.push(Entry {
            name: set.name.clone(),
            path: set.representative().to_path_buf(),
            kind: EntryKind::TextureSet,
            modified,
            size,
            texture_set: Some(set),
            modo_kind: None,
        });
    }
    for (path, modified, size) in files {
        if in_set.contains(&path) {
            continue;
        }
        let kind = classify(&path, read_preset_kind);
        if options.only_usable && !kind.usable() {
            continue;
        }
        let modo_kind = modo_kind(&path, kind);
        entries.push(Entry {
            name: stem(&path),
            path,
            kind,
            modified,
            size,
            texture_set: None,
            modo_kind,
        });
    }
    Ok(Listing { folders, entries })
}

// ------------------------------------------------------------ texture sets

// Texture sets (CD-20..27) are read and turned into Material presets by the
// shared loader in texture_set.rs, which the material converter uses too.
pub use super::texture_set::{
    MapFile, MapKind, MapName, ResolutionChoice, TextureSet, natural_cmp, parse_map_name,
    texture_sets,
};
use super::texture_set::{file_name, stem};

/// Two paths to one file ("./a.png" and "a.png").
fn same_file(a: &Path, b: &Path) -> bool {
    let clean = |p: &Path| -> PathBuf {
        p.components()
            .filter(|c| !matches!(c, std::path::Component::CurDir))
            .collect()
    };
    clean(a) == clean(b)
}

/// A free file name in `folder` for `name` (PB-122's "Keep both"): `name`
/// itself when free, else "name 2", "name 3"… before the extension.
pub fn free_path(folder: &Path, name: &str, extension: &str) -> PathBuf {
    let first = folder.join(format!("{name}.{extension}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| folder.join(format!("{name} {n}.{extension}")))
        .find(|p| !p.exists())
        .expect("some number is free")
}

#[cfg(test)]
mod tests;
