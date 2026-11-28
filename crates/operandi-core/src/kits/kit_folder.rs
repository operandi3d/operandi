//! The Kits folder: finding installed kits, which ones are on, and
//! installing a kit from a zip or a folder (kits plan K1; docs/KITS.md).
//!
//! Every kit is one folder inside the Kits folder, named after its id,
//! with `kit.toml` at its top. Installing a kit never turns it on: a kit
//! can run programs, so the user turns it on themselves
//! (`operandi.kit.enable`), and that choice is kept in `kits.toml` beside
//! the Kits folder.
use crate::Result;
use crate::kit_manifest::{self, KitManifest, MANIFEST_FILE};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A folder found in the Kits folder.
#[derive(Clone, Debug)]
pub struct FoundKit {
    pub dir: PathBuf,
    /// The checked manifest, or why the kit can't load.
    pub manifest: std::result::Result<KitManifest, String>,
}

impl FoundKit {
    /// The kit's id, else its folder name.
    pub fn id(&self) -> String {
        match &self.manifest {
            Ok(m) => m.kit.id.clone(),
            Err(_) => self
                .dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }
}

/// Every kit folder in `kits_dir`, sorted by folder name. A folder whose
/// manifest is missing or wrong is listed with the reason; a second kit
/// with an id already found, or one in a folder not named after its id,
/// is listed as a problem too. A missing Kits folder means no kits.
pub fn discover(kits_dir: &Path) -> Vec<FoundKit> {
    let Ok(entries) = std::fs::read_dir(kits_dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        // Hidden folders (.DS_Store's siblings, half-done installs) aren't kits.
        .filter(|p| {
            p.is_dir()
                && !p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .collect();
    dirs.sort();
    let mut ids = BTreeSet::new();
    dirs.into_iter()
        .map(|dir| {
            let manifest = KitManifest::read(&dir).and_then(|m| {
                let folder = dir.file_name().map(|n| n.to_string_lossy().into_owned());
                if folder.as_deref() != Some(m.kit.id.as_str()) {
                    return Err(format!(
                        "Kit \"{}\" must be in a folder named {}",
                        m.kit.id, m.kit.id
                    ));
                }
                if !ids.insert(m.kit.id.clone()) {
                    return Err(format!("A second kit with id {}", m.kit.id));
                }
                Ok(m)
            });
            FoundKit { dir, manifest }
        })
        .collect()
}

/// Which kits are on: `kits.toml` in the app's data folder.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KitState {
    #[serde(default)]
    pub enabled: BTreeSet<String>,
}

impl KitState {
    /// Read `path`; a missing or unreadable file means no kit is on (the
    /// safe answer for a file that says which programs may run).
    pub fn read(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }
    pub fn write(&self, path: &Path) -> Result<()> {
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        let header = "# Kits that are on (operandi.kit.enable / operandi.kit.disable).\n";
        operandi_files::write_atomic(path, format!("{header}{text}").as_bytes())
            .map_err(|e| format!("Can't save {}: {e}", path.display()))
    }
}

/// Largest kit zip read: helpers with big libraries (OpenCascade) are a
/// few hundred MB.
const MAX_KIT_ZIP: u64 = 2 * 1024 * 1024 * 1024;

/// Install the kit in `source` (a `.zip` or a kit folder) into
/// `kits_dir/<id>`. The manifest is checked first; a kit already there is
/// replaced only with `replace`. The kit is left off. Returns its id.
pub fn install(source: &Path, kits_dir: &Path, replace: bool) -> Result<String> {
    let files = if source.is_dir() {
        read_folder(source)?
    } else {
        let bytes = operandi_files::read_whole(source, MAX_KIT_ZIP)?;
        if !crate::interchange::zip::is_zip(&bytes) {
            return Err(format!(
                "{} is neither a kit folder nor a .zip",
                source.display()
            ));
        }
        unwrap_top_folder(crate::interchange::zip::read(&bytes)?)
    };
    let manifest_text = files
        .iter()
        .find(|(name, _)| name == MANIFEST_FILE)
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
        .ok_or_else(|| format!("No {MANIFEST_FILE} at the top of {}", source.display()))?;
    let manifest = KitManifest::parse(&manifest_text)?;
    for (name, _) in &files {
        kit_manifest::check_inside(name)?;
    }
    let id = manifest.kit.id.clone();
    let dest = kits_dir.join(&id);
    if dest.exists() && !replace {
        return Err(format!(
            "Kit {id} is already installed; use replace:true to install over it"
        ));
    }
    // Write beside the destination, then swap it in, so a failed install
    // leaves the old kit as it was.
    std::fs::create_dir_all(kits_dir).map_err(|e| e.to_string())?;
    let staging = kits_dir.join(format!(".installing-{id}"));
    let _ = std::fs::remove_dir_all(&staging);
    let written = (|| -> Result<()> {
        for (name, bytes) in &files {
            let path = staging.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        // ZIP entries carry no Unix permissions here: mark the declared
        // helpers runnable.
        for imp in &manifest.importers {
            for helper in imp.helper.values() {
                make_runnable(&staging.join(helper))?;
            }
        }
        Ok(())
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }
    if dest.exists() {
        std::fs::remove_dir_all(&dest).map_err(|e| format!("Can't replace {id}: {e}"))?;
    }
    std::fs::rename(&staging, &dest).map_err(|e| format!("Can't install {id}: {e}"))?;
    Ok(id)
}

/// A kit zip may hold the kit at its top or inside one folder (as
/// zipping a folder does). Drops that folder from every name, and the
/// junk macOS adds to zips.
fn unwrap_top_folder(files: Vec<(String, Vec<u8>)>) -> Vec<(String, Vec<u8>)> {
    let files: Vec<_> = files
        .into_iter()
        .filter(|(n, _)| {
            !n.starts_with("__MACOSX/") && !n.ends_with(".DS_Store") && !n.ends_with('/')
        })
        .collect();
    if files.iter().any(|(n, _)| n == MANIFEST_FILE) {
        return files;
    }
    let top = files
        .first()
        .and_then(|(n, _)| n.split_once('/'))
        .map(|(t, _)| format!("{t}/"));
    match top {
        Some(top) if files.iter().all(|(n, _)| n.starts_with(&top)) => files
            .into_iter()
            .map(|(n, b)| (n[top.len()..].to_owned(), b))
            .collect(),
        _ => files,
    }
}

/// Every file under `dir` as (relative path with `/`, contents).
fn read_folder(dir: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).map_err(|e| format!("{}: {e}", d.display()))? {
            let path = entry.map_err(|e| e.to_string())?.path();
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            if name.as_deref().is_some_and(|n| n.starts_with('.')) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path
                    .strip_prefix(dir)
                    .map_err(|e| e.to_string())?
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                out.push((rel, bytes));
            }
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(unix)]
fn make_runnable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if !path.is_file() {
        // A helper for another platform needn't be in this zip.
        return Ok(());
    }
    let mut perms = std::fs::metadata(path)
        .map_err(|e| e.to_string())?
        .permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(not(unix))]
fn make_runnable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "[kit]\nid = \"zipped\"\nname = \"Zipped\"\nversion = \"1\"\napi_version = 1\nlicence = \"MIT\"\n";

    fn temp(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("operandi-kitzip-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_zip_installs_with_or_without_a_top_folder() {
        let root = temp("both");
        for (n, prefix) in ["", "zipped-1.0-macos/"].iter().enumerate() {
            let names = [
                format!("{prefix}kit.toml"),
                format!("{prefix}data/readme.txt"),
            ];
            let zip = crate::interchange::zip::write(&[
                (names[0].as_str(), MANIFEST.as_bytes()),
                (names[1].as_str(), b"hello".as_slice()),
                ("__MACOSX/._kit.toml", b"junk".as_slice()),
            ])
            .unwrap();
            let path = root.join(format!("kit{n}.zip"));
            std::fs::write(&path, zip).unwrap();
            let kits = root.join(format!("Kits{n}"));
            assert_eq!(install(&path, &kits, false).unwrap(), "zipped");
            assert_eq!(
                std::fs::read(kits.join("zipped/data/readme.txt")).unwrap(),
                b"hello"
            );
            assert!(!kits.join("zipped/__MACOSX").exists());
        }
    }

    #[test]
    fn a_zip_entry_outside_the_kit_is_refused() {
        let root = temp("escape");
        let zip = crate::interchange::zip::write(&[
            ("kit.toml", MANIFEST.as_bytes()),
            ("../evil.txt", b"x".as_slice()),
        ])
        .unwrap();
        let path = root.join("kit.zip");
        std::fs::write(&path, zip).unwrap();
        let e = install(&path, &root.join("Kits"), false).unwrap_err();
        assert!(e.contains("inside the kit folder"), "{e}");
        assert!(!root.join("evil.txt").exists());
        assert!(!root.join("Kits/zipped").exists());
    }
}
