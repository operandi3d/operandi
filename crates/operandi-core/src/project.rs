//! Projects (docs/PROJECT_SYSTEM.md, Jason's design): a root folder holding
//! `operandi.project.toml` and standard folders for scenes, images, renders
//! and so on. Everything that reads or writes project files asks the project
//! where a kind of file goes. Paths in the manifest are relative to the root,
//! so a project can be moved or copied and still work.
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

/// Manifest file name in a project's root folder.
pub const MANIFEST: &str = "operandi.project.toml";
/// Manifest format this build reads and writes.
pub const FORMAT: u32 = 1;
/// Standard folder kinds, each created under its own name by default.
/// `materials` (material presets, MaterialX) and `scripts` were added
/// 2026-11-16; an older project's manifest doesn't list them, so they take
/// their default folder and are made on the next Set Project.
pub const FOLDERS: [&str; 10] = [
    "scenes",
    "sourceimages",
    "renders",
    "cache",
    "references",
    "assets",
    "exports",
    "workspaces",
    "materials",
    "scripts",
];

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    project: Header,
    #[serde(default)]
    paths: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    name: String,
    format: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    root: PathBuf,
    name: String,
    /// Folder kind → folder relative to the root.
    paths: BTreeMap<String, PathBuf>,
}

impl Project {
    /// Make `root` a project: create the folder if needed, add any missing
    /// standard folders and write the manifest. Existing files are never
    /// moved or deleted, so this also turns a loose folder into a project.
    /// Refused inside another project (no nesting) or if `root` is already one.
    pub fn create(root: &Path, name: &str) -> Result<Project> {
        let name = name.trim();
        if !root.is_absolute() {
            return Err("Choose an absolute folder for the project".into());
        }
        if name.is_empty() {
            return Err("Give the project a name".into());
        }
        if root.join(MANIFEST).exists() {
            return Err(format!(
                "{} is already a project; use Set Project to open it",
                root.display()
            ));
        }
        if let Some(outer) = root.parent().and_then(Self::enclosing) {
            return Err(format!(
                "Projects can't be nested; {} is already a project",
                outer.display()
            ));
        }
        let project = Project {
            root: root.to_path_buf(),
            name: name.to_owned(),
            paths: FOLDERS
                .iter()
                .map(|&k| (k.to_owned(), PathBuf::from(k)))
                .collect(),
        };
        let text = project.manifest_text()?;
        let fail = |e: std::io::Error| format!("Couldn't create the project: {e}");
        std::fs::create_dir_all(root).map_err(fail)?;
        for folder in project.paths.values() {
            std::fs::create_dir_all(root.join(folder)).map_err(fail)?;
        }
        // create_new: never overwrite a manifest that appeared meanwhile.
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(MANIFEST))
            .and_then(|mut f| f.write_all(text.as_bytes()))
            .map_err(fail)?;
        Ok(project)
    }

    /// Read the project whose manifest is in `root`.
    pub fn open(root: &Path) -> Result<Project> {
        let file = root.join(MANIFEST);
        let text = std::fs::read_to_string(&file)
            .map_err(|e| format!("{} isn't a project ({e})", root.display()))?;
        Self::parse(root, &text)
    }

    fn parse(root: &Path, text: &str) -> Result<Project> {
        let manifest: Manifest =
            toml::from_str(text).map_err(|e| format!("{MANIFEST} can't be read: {e}"))?;
        if manifest.project.format != FORMAT {
            return Err(format!(
                "{MANIFEST} uses format {}; this Operandi reads format {FORMAT}",
                manifest.project.format
            ));
        }
        if manifest.project.name.trim().is_empty() {
            return Err(format!("{MANIFEST} has no project name"));
        }
        let mut paths: BTreeMap<String, PathBuf> = FOLDERS
            .iter()
            .map(|&k| (k.to_owned(), PathBuf::from(k)))
            .collect();
        for (kind, folder) in manifest.paths {
            let folder = PathBuf::from(folder);
            // Only plain relative folders, so the project stays movable and
            // can't point outside itself.
            if folder.as_os_str().is_empty()
                || !folder
                    .components()
                    .all(|c| matches!(c, Component::Normal(_)))
            {
                return Err(format!(
                    "{MANIFEST}: the {kind} folder must be a plain folder inside the project"
                ));
            }
            paths.insert(kind, folder);
        }
        Ok(Project {
            root: root.to_path_buf(),
            name: manifest.project.name,
            paths,
        })
    }

    fn manifest_text(&self) -> Result<String> {
        let manifest = Manifest {
            project: Header {
                name: self.name.clone(),
                format: FORMAT,
            },
            paths: self
                .paths
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string_lossy().into_owned()))
                .collect(),
        };
        toml::to_string(&manifest).map_err(|e| format!("Couldn't write {MANIFEST}: {e}"))
    }

    /// The nearest folder at or above `path` that is a project.
    pub fn enclosing(path: &Path) -> Option<PathBuf> {
        path.ancestors()
            .find(|dir| dir.join(MANIFEST).is_file())
            .map(Path::to_path_buf)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where files of `kind` go, if the project defines that kind.
    pub fn folder(&self, kind: &str) -> Option<PathBuf> {
        self.paths.get(kind).map(|f| self.root.join(f))
    }

    /// Make any of the project's folders that are missing (an older project
    /// gaining a new standard folder, or one deleted by hand). Never moves
    /// or deletes anything.
    pub fn add_missing_folders(&self) -> Result<()> {
        for folder in self.paths.values() {
            std::fs::create_dir_all(self.root.join(folder))
                .map_err(|e| format!("Couldn't make {}: {e}", folder.display()))?;
        }
        Ok(())
    }

    /// The scenes folder (always defined).
    pub fn scenes(&self) -> PathBuf {
        self.folder("scenes")
            .unwrap_or_else(|| self.root.join("scenes"))
    }
}
