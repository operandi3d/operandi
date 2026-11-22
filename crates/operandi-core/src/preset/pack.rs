//! The Operandi Library pack (F1, Jason 2026-11-11): a downloadable folder
//! of content the preset browser recognises by its `manifest.json`.
//!
//! ```text
//! Operandi Library/
//!   manifest.json       what's inside: name, version, one entry per item
//!   CREDITS.txt         the same, for people
//!   LICENSES/CC0-1.0.txt
//!   Starter/<Category>/<Name>/  CC0 only: <Name>.oppreset, its maps,
//!                       <Name>.png (Starter/<Name>/ without a category
//!                       still reads, from 2026.11.0)
//!   Extras/<Name>/      optional, CC-BY (none ship yet; the reader takes them)
//!   Matcaps/<Category>/<Name>.png      CC0 matcap pictures (2026.11.4)
//!   Environments/<Category>/<Name>.hdr CC0 HDR environments (2026.11.4)
//!   Meshes/<Category>/<Name>.oppreset  CC0 item presets of our own
//!                       (starter_meshes.rs), thumbnail inside (2026.11.5)
//! ```
//!
//! Pictures carry no licence header, so the source folders for Matcaps/
//! and Environments/ each hold a `SOURCES.tsv` (one line per picture:
//! path below the folder, licence, source URL, credit, optionally the
//! date the licence was checked) that
//! [`build_manifest`] reads; a picture without a line, or a line without
//! a picture, refuses the pack like a preset without a header.
//!
//! [`Manifest`] reads and writes `manifest.json`; [`build_manifest`] makes
//! one from the presets' own licence headers (native v37's
//! `PresetHeader::source`) and refuses a pack whose items break the
//! licence rule ([`check_item`]): Starter is CC0 with a source URL, Extras
//! is CC-BY with a source URL and a credit, nothing else ships. The app
//! never downloads anything; it only reads what's on disk.
use super::PresetKind;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// `manifest.json`'s `format`.
pub const FORMAT: &str = "operandi-library";
/// The manifest format this build reads and writes.
pub const FORMAT_VERSION: u32 = 1;
/// The file that marks a folder as a pack.
pub const MANIFEST_FILE: &str = "manifest.json";
/// The CC0-only part of a pack.
pub const STARTER: &str = "Starter";
/// The optional attribution part (CC-BY).
pub const EXTRAS: &str = "Extras";
/// Matcap pictures (CC0 only, like Starter).
pub const MATCAPS: &str = "Matcaps";
/// HDR environment pictures (CC0 only, like Starter).
pub const ENVIRONMENTS: &str = "Environments";
/// Mesh (item) presets, each a single file (CC0 only, like Starter).
pub const MESHES: &str = "Meshes";
/// The licence list a picture folder carries (see the module notes).
pub const SOURCES_FILE: &str = "SOURCES.tsv";
/// The licence every Starter item has.
pub const CC0: &str = "CC0-1.0";

/// The picture folders: top folder, the manifest kind of its items and
/// the extensions it takes.
const PICTURE_FOLDERS: [(&str, &str, &[&str]); 2] = [
    (MATCAPS, "matcap", &["png", "jpg", "jpeg", "exr"]),
    (
        ENVIRONMENTS,
        "environmentImage",
        &super::library::HDR_EXTENSIONS,
    ),
];

/// A pack's `manifest.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub format_version: u32,
    /// Shown in the preset browser's folder tree, e.g. "Operandi Library".
    pub name: String,
    /// Calendar version, e.g. "2026.11.0".
    pub version: String,
    pub items: Vec<ManifestItem>,
}

/// One item of a pack.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestItem {
    /// Relative to the pack folder, with `/` separators.
    pub path: String,
    /// "material", "item", "assembly", "environment" or "renderSettings"
    /// (the preset kind's script name).
    pub kind: String,
    /// SPDX-style, e.g. "CC0-1.0", "CC-BY-4.0".
    pub licence: String,
    pub source_url: String,
    pub credit: String,
    /// When the licence was last checked at the source (YYYY-MM-DD).
    pub checked: String,
    /// The folder the item sits in below Starter/ or Extras/, e.g. "Metal"
    /// for `Starter/Metal/Metal049A/Metal049A.oppreset` (2026.11.1). Empty,
    /// and left out of the JSON, for an item with no category folder, so
    /// 2026.11.0 manifests read unchanged.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
}

impl Manifest {
    pub fn new(name: &str, version: &str, items: Vec<ManifestItem>) -> Self {
        Self {
            format: FORMAT.into(),
            format_version: FORMAT_VERSION,
            name: name.into(),
            version: version.into(),
            items,
        }
    }

    /// Read `text`; anything that isn't an Operandi Library manifest of a
    /// version this build knows is an error.
    pub fn parse(text: &str) -> Result<Self, String> {
        let m: Manifest = serde_json::from_str(text).map_err(|e| format!("manifest.json: {e}"))?;
        if m.format != FORMAT {
            return Err(format!(
                "manifest.json: format is {:?}, not {FORMAT:?}",
                m.format
            ));
        }
        if m.format_version != FORMAT_VERSION {
            return Err(format!(
                "manifest.json: format_version {} (this Operandi reads {FORMAT_VERSION})",
                m.format_version
            ));
        }
        Ok(m)
    }

    /// The manifest of the pack at `folder`, if it has one.
    pub fn read(folder: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(folder.join(MANIFEST_FILE))
            .map_err(|e| format!("{}: {e}", folder.join(MANIFEST_FILE).display()))?;
        Self::parse(&text)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a manifest always serialises") + "\n"
    }

    pub fn write(&self, folder: &Path) -> Result<(), String> {
        let path = folder.join(MANIFEST_FILE);
        std::fs::write(&path, self.to_json()).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The item at `path` (a file in the pack), or failing that the item
    /// whose file sits in the same folder as `path` (a material's maps
    /// share its credit). `pack` is the pack's folder.
    pub fn item_for(&self, pack: &Path, path: &Path) -> Option<&ManifestItem> {
        let rel = relative(pack, path)?;
        if let Some(i) = self.items.iter().find(|i| i.path == rel) {
            return Some(i);
        }
        let folder = parent_of(&rel)?;
        self.items
            .iter()
            .find(|i| parent_of(&i.path).as_deref() == Some(folder.as_str()))
    }

    /// "Operandi Library 2026.11.0".
    pub fn title(&self) -> String {
        format!("{} {}", self.name, self.version)
    }

    /// How many items carry each licence, most first: "32 CC0-1.0".
    pub fn licence_summary(&self) -> String {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for i in &self.items {
            match counts.iter_mut().find(|(l, _)| *l == i.licence) {
                Some((_, n)) => *n += 1,
                None => counts.push((i.licence.clone(), 1)),
            }
        }
        counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        counts
            .iter()
            .map(|(l, n)| format!("{n} {l}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl ManifestItem {
    /// The lines the browser's tooltip shows.
    pub fn info(&self) -> String {
        let mut out = format!("Licence: {}", self.licence);
        if !self.credit.is_empty() {
            out += &format!("\nCredit: {}", self.credit);
        }
        if !self.source_url.is_empty() {
            out += &format!("\nSource: {}", self.source_url);
        }
        if !self.checked.is_empty() {
            out += &format!("\nLicence checked {}", self.checked);
        }
        out
    }
}

/// `path` below `base`, with `/` separators.
fn relative(base: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(base).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The category of the item at `rel` (relative to the pack): the folder
/// between Starter/ or Extras/ and the item's own folder, when there is
/// one. `Starter/Metal/Metal049A/Metal049A.oppreset` is "Metal";
/// `Starter/Metal049A/Metal049A.oppreset` has none.
pub fn category_of(rel: &str) -> String {
    let parts: Vec<&str> = rel.split('/').collect();
    match parts.as_slice() {
        [top, category, _, _] if *top == STARTER || *top == EXTRAS => (*category).to_owned(),
        // Pictures and mesh presets sit straight in their category folder.
        [top, category, _] if *top == MATCAPS || *top == ENVIRONMENTS || *top == MESHES => {
            (*category).to_owned()
        }
        _ => String::new(),
    }
}

fn parent_of(rel: &str) -> Option<String> {
    rel.rsplit_once('/').map(|(p, _)| p.to_owned())
}

/// Whether `item` may ship in a pack, by where it sits: Starter takes only
/// CC0 with a source URL; Extras takes CC-BY (any version) with a source
/// URL and a credit. Nothing else is allowed (the licence rule: only CC0 or
/// our own work in Starter, nothing that isn't cleanly GPL v3 compatible).
pub fn check_item(item: &ManifestItem) -> Result<(), String> {
    let top = item.path.split('/').next().unwrap_or("");
    if item.source_url.trim().is_empty() {
        return Err(format!("{}: no source URL", item.path));
    }
    match top {
        STARTER | MATCAPS | ENVIRONMENTS | MESHES => {
            if item.licence != CC0 {
                return Err(format!(
                    "{}: {top} takes only {CC0}, not {:?}",
                    item.path, item.licence
                ));
            }
        }
        EXTRAS => {
            let by = item.licence.starts_with("CC-BY-")
                && item.licence["CC-BY-".len()..]
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.');
            if !(by || item.licence == CC0) {
                return Err(format!(
                    "{}: Extras takes CC-BY or {CC0}, not {:?}",
                    item.path, item.licence
                ));
            }
            if by && item.credit.trim().is_empty() {
                return Err(format!("{}: CC-BY needs a credit line", item.path));
            }
        }
        _ => {
            return Err(format!(
                "{}: items live in {STARTER}/, {EXTRAS}/, {MESHES}/, {MATCAPS}/ or {ENVIRONMENTS}/",
                item.path
            ));
        }
    }
    Ok(())
}

/// Every licence problem in `manifest` (empty when it may ship).
pub fn check(manifest: &Manifest) -> Vec<String> {
    manifest
        .items
        .iter()
        .filter_map(|i| check_item(i).err())
        .collect()
}

/// Make the manifest of the pack at `folder` from the `.oppreset` files in
/// its Starter/, Extras/ and Meshes/ folders, each one's licence and credit read
/// from its own header. Fails, listing every problem, when a preset has no
/// licence header or breaks [`check_item`]. Nothing is written.
pub fn build_manifest(
    folder: &Path,
    name: &str,
    version: &str,
    checked: &str,
) -> Result<Manifest, Vec<String>> {
    let mut items = Vec::new();
    let mut problems = Vec::new();
    for part in [STARTER, EXTRAS, MESHES] {
        let mut presets = Vec::new();
        collect_presets(&folder.join(part), &mut presets);
        presets.sort();
        for path in presets {
            let rel = relative(folder, &path).unwrap_or_default();
            let header = match crate::persistence::read_preset_header(&path) {
                Ok(h) => h,
                Err(e) => {
                    problems.push(format!("{rel}: {e}"));
                    continue;
                }
            };
            let Some(source) = header.source else {
                problems.push(format!("{rel}: no licence in its header"));
                continue;
            };
            let item = ManifestItem {
                category: category_of(&rel),
                path: rel,
                kind: kind_name(header.kind).into(),
                licence: source.licence,
                source_url: source.url,
                credit: source.credit,
                checked: checked.into(),
            };
            match check_item(&item) {
                Ok(()) => items.push(item),
                Err(e) => problems.push(e),
            }
        }
    }
    for (top, kind, extensions) in PICTURE_FOLDERS {
        pictures_from_sources(
            folder,
            top,
            kind,
            extensions,
            checked,
            &mut items,
            &mut problems,
        );
    }
    if items.is_empty() && problems.is_empty() {
        problems.push(format!("{} holds no presets", folder.display()));
    }
    if problems.is_empty() {
        Ok(Manifest::new(name, version, items))
    } else {
        Err(problems)
    }
}

/// The items of the picture folder `top` (Matcaps/ or Environments/) in
/// the pack at `folder`, from its SOURCES.tsv. Every picture needs a line
/// and every line a picture; problems are collected, not fatal here.
fn pictures_from_sources(
    folder: &Path,
    top: &str,
    kind: &str,
    extensions: &[&str],
    checked: &str,
    items: &mut Vec<ManifestItem>,
    problems: &mut Vec<String>,
) {
    let dir = folder.join(top);
    let mut pictures = Vec::new();
    collect_files(&dir, &mut pictures, &|p: &Path| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| extensions.iter().any(|x| x.eq_ignore_ascii_case(e)))
    });
    if pictures.is_empty() {
        return;
    }
    let sources = match std::fs::read_to_string(dir.join(SOURCES_FILE)) {
        Ok(text) => text,
        Err(e) => {
            problems.push(format!("{top}/{SOURCES_FILE}: {e}"));
            return;
        }
    };
    let mut rows: Vec<(String, ManifestItem)> = Vec::new();
    for (n, line) in sources.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').map(str::trim).collect();
        // An optional fifth column is the date the licence was checked at
        // the source, when it differs from the pack's.
        let (path, licence, url, credit, when) = match cols.as_slice() {
            [p, l, u, c] => (*p, *l, *u, *c, checked),
            [p, l, u, c, w] if !w.is_empty() => (*p, *l, *u, *c, *w),
            _ => {
                problems.push(format!(
                    "{top}/{SOURCES_FILE} line {}: needs path, licence, source URL, credit \
                     (and optionally the date checked) separated by tabs",
                    n + 1
                ));
                continue;
            }
        };
        let rel = format!("{top}/{path}");
        rows.push((
            rel.clone(),
            ManifestItem {
                category: category_of(&rel),
                path: rel,
                kind: kind.into(),
                licence: licence.into(),
                source_url: url.into(),
                credit: credit.into(),
                checked: when.into(),
            },
        ));
    }
    pictures.sort();
    for picture in &pictures {
        let rel = relative(folder, picture).unwrap_or_default();
        match rows.iter().position(|(p, _)| *p == rel) {
            Some(i) => {
                let (_, item) = rows.remove(i);
                match check_item(&item) {
                    Ok(()) => items.push(item),
                    Err(e) => problems.push(e),
                }
            }
            None => problems.push(format!("{rel}: no line in {top}/{SOURCES_FILE}")),
        }
    }
    for (rel, _) in rows {
        problems.push(format!(
            "{rel}: listed in {top}/{SOURCES_FILE} but not in the pack"
        ));
    }
}

fn kind_name(kind: PresetKind) -> &'static str {
    kind.script_name()
}

fn collect_presets(folder: &Path, out: &mut Vec<PathBuf>) {
    collect_files(folder, out, &|p: &Path| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(super::is_preset_extension)
    });
}

/// Every file below `folder` (hidden ones left out) that `keep` takes.
fn collect_files(folder: &Path, out: &mut Vec<PathBuf>, keep: &dyn Fn(&Path) -> bool) {
    let Ok(read) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in read.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if super::library::is_hidden(&name) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out, keep);
        } else if keep(&path) {
            out.push(path);
        }
    }
}

/// CREDITS.txt for `manifest`.
pub fn credits_text(manifest: &Manifest) -> String {
    let mut out = format!(
        "CREDITS - {}\n\nStarter/ is CC0 1.0 Universal (public domain): no attribution is required, \
         but the sources are listed with thanks. See LICENSES/CC0-1.0.txt.\n",
        manifest.title()
    );
    if manifest
        .items
        .iter()
        .any(|i| i.path.starts_with(MATCAPS) || i.path.starts_with(ENVIRONMENTS))
    {
        out += &format!(
            "{MATCAPS}/ and {ENVIRONMENTS}/ are CC0 1.0 Universal too, with their sources listed below.\n"
        );
    }
    if manifest.items.iter().any(|i| i.path.starts_with(MESHES)) {
        out += &format!(
            "{MESHES}/ is CC0 1.0 Universal too: shapes made entirely by Operandi's own code.\n"
        );
    }
    if manifest.items.iter().any(|i| i.path.starts_with(EXTRAS)) {
        out += "Extras/ items are CC-BY: credit them as listed when you use them.\n";
    }
    out += "\npath | category | licence | credit | source URL | licence checked\n";
    for i in &manifest.items {
        out += &format!(
            "{} | {} | {} | {} | {} | {}\n",
            i.path, i.category, i.licence, i.credit, i.source_url, i.checked
        );
    }
    out
}

/// Compare two pack versions ("2026.11.0" < "2026.12.0" < "2026.12.1"),
/// number by number; a part that isn't a number compares as text.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let mut x = a.split('.');
    let mut y = b.split('.');
    loop {
        match (x.next(), y.next()) {
            (None, None) => return Ordering::Equal,
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (Some(p), Some(q)) => {
                let o = match (p.parse::<u64>(), q.parse::<u64>()) {
                    (Ok(m), Ok(n)) => m.cmp(&n),
                    _ => p.cmp(q),
                };
                if o != Ordering::Equal {
                    return o;
                }
            }
        }
    }
}

/// The packs in a library folder: the folder itself when it holds a
/// manifest, else each subfolder that does. Sorted by path.
pub fn packs_in(library: &Path) -> Vec<PathBuf> {
    if library.join(MANIFEST_FILE).is_file() {
        return vec![library.to_path_buf()];
    }
    let mut out: Vec<PathBuf> = std::fs::read_dir(library)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| !super::library::is_hidden(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .filter(|p| p.join(MANIFEST_FILE).is_file())
        .collect();
    out.sort();
    out
}

#[cfg(test)]
#[path = "pack/tests.rs"]
mod tests;
