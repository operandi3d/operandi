//! Project-relative file references (docs/PROJECT_SYSTEM.md, phase 2).
//!
//! In memory every external file a scene uses is an absolute path, exactly
//! as before projects existed, so no tool has to know about projects. Only
//! at the file boundary (`persistence::save` / `persistence::load`) are the
//! paths of files inside the scene file's project written as
//! `project:<relative/path>` (forward slashes on every platform) and turned
//! back into full paths on load. A copied or moved project therefore keeps
//! finding its pictures, node-graph images and IES files.
//!
//! Files outside the project, and scenes saved outside any project (the
//! crash-recovery journal in the app data folder, a loose scene), keep
//! absolute paths. Import provenance (`Scene::import_source`) is history,
//! not a reference, and is never rewritten.
use crate::Scene;
use crate::project::Project;
use std::path::{Component, Path, PathBuf};

/// The prefix of a project-relative reference in a saved scene.
pub const PREFIX: &str = "project:";

/// Every saved field that names an external file the scene reads (or, for
/// a Render Output, writes). Add new path fields here so they move with
/// the project.
pub fn for_each_path_mut(scene: &mut Scene, mut f: impl FnMut(&mut String)) {
    for image in &mut scene.images {
        f(&mut image.path);
    }
    for item in &mut scene.items {
        if let Some(light) = &mut item.light {
            f(&mut light.photometry_file);
        }
    }
    for output in scene.shader_tree.outputs.values_mut() {
        f(&mut output.filename);
    }
    for graph in &mut scene.shader_tree.node_graphs.graphs {
        let groups = graph.groups.iter_mut().flat_map(|g| g.nodes.iter_mut());
        for node in graph.material.nodes.iter_mut().chain(groups) {
            if let Some(image) = node.image.as_mut().filter(|i| !i.packed) {
                f(&mut image.path);
            }
        }
    }
}

/// Read-only twin of [`for_each_path_mut`]; keep the two in step.
fn for_each_path(scene: &Scene, mut f: impl FnMut(&str)) {
    for image in &scene.images {
        f(&image.path);
    }
    for item in &scene.items {
        if let Some(light) = &item.light {
            f(&light.photometry_file);
        }
    }
    for output in scene.shader_tree.outputs.values() {
        f(&output.filename);
    }
    for graph in &scene.shader_tree.node_graphs.graphs {
        let groups = graph.groups.iter().flat_map(|g| g.nodes.iter());
        for node in graph.material.nodes.iter().chain(groups) {
            if let Some(image) = node.image.as_ref().filter(|i| !i.packed) {
                f(&image.path);
            }
        }
    }
}

/// Whether any saved path is project-relative (native version 48).
pub fn has_project_paths(scene: &Scene) -> bool {
    let mut any = false;
    for_each_path(scene, |p| any |= p.starts_with(PREFIX));
    any
}

/// The project a scene file at `scene_file` belongs to, if any.
pub fn project_root_for(scene_file: &Path) -> Option<PathBuf> {
    Project::enclosing(scene_file.parent()?)
}

/// `path` relative to `root` with forward slashes, if it is inside it.
/// Compared as written first, then with symbolic links resolved (a
/// project reached through `/Volumes/...` and its real path).
pub fn relative_to(root: &Path, path: &Path) -> Option<String> {
    if !path.is_absolute() {
        return None;
    }
    let lexical = path.strip_prefix(root).ok().map(Path::to_path_buf);
    let rel = lexical.or_else(|| {
        let root = root.canonicalize().ok()?;
        let path = path.canonicalize().ok()?;
        path.strip_prefix(&root).ok().map(Path::to_path_buf)
    })?;
    let mut parts = Vec::new();
    for c in rel.components() {
        match c {
            Component::Normal(s) => parts.push(s.to_str()?.to_owned()),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// `scene` with every file inside `root` written as `project:<rel>`, or
/// `None` when nothing changes (no clone for the common case).
pub fn to_project_relative(scene: &Scene, root: &Path) -> Option<Scene> {
    let rel = |p: &str| {
        (!p.is_empty() && !p.starts_with(PREFIX))
            .then(|| relative_to(root, Path::new(p)))
            .flatten()
    };
    let mut any = false;
    for_each_path(scene, |p| any |= rel(p).is_some());
    if !any {
        return None;
    }
    let mut out = scene.clone();
    for_each_path_mut(&mut out, |p| {
        if let Some(r) = rel(p) {
            *p = format!("{PREFIX}{r}");
        }
    });
    Some(out)
}

/// The relative part of a `project:` reference as a path, refusing
/// anything that could step outside the project (`..`, roots, drives).
fn relative_part(text: &str) -> Option<PathBuf> {
    let rel = text.strip_prefix(PREFIX)?;
    let mut out = PathBuf::new();
    for part in rel.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            p if Path::new(p).components().count() != 1
                || !matches!(Path::new(p).components().next(), Some(Component::Normal(_))) =>
            {
                return None;
            }
            p => out.push(p),
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

/// Turn `project:` references back into full paths for a scene read from
/// `scene_file`. The enclosing project wins (even for a file that is
/// missing, so the missing-file message names where it belongs). With no
/// project, the scene file's folder and its ancestors are tried for a file
/// that exists. Otherwise the text is kept as it was saved.
pub fn resolve(scene: &mut Scene, scene_file: &Path) {
    if !has_project_paths(scene) {
        return;
    }
    let root = project_root_for(scene_file);
    let folder = scene_file.parent();
    for_each_path_mut(scene, |p| {
        let Some(rel) = relative_part(p) else {
            return;
        };
        let found = match &root {
            Some(root) => Some(root.join(&rel)),
            None => folder.and_then(|f| {
                f.ancestors()
                    .map(|a| a.join(&rel))
                    .find(|candidate| candidate.exists())
            }),
        };
        if let Some(found) = found {
            *p = found.to_string_lossy().into_owned();
        }
    });
}

/// What [`copy_into`] did.
#[derive(Clone, Debug, PartialEq)]
pub enum Adopted {
    /// Copied to this new file.
    Copied(PathBuf),
    /// A file with the same name and bytes was already there.
    Reused(PathBuf),
}
impl Adopted {
    pub fn path(&self) -> &Path {
        match self {
            Adopted::Copied(p) | Adopted::Reused(p) => p,
        }
    }
}

/// Copy `file` into `folder` (created if needed), keeping its name. A file
/// of that name with the same bytes is reused; a different one keeps its
/// name and the copy becomes `name (2).ext`, `name (3).ext`... The original
/// is never moved, renamed or deleted. The copy goes through a temporary
/// file, so a failed copy never leaves a half-written file under the name.
pub fn copy_into(folder: &Path, file: &Path) -> std::io::Result<Adopted> {
    use std::io::{Error, ErrorKind};
    let name = file
        .file_name()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "no file name"))?;
    std::fs::create_dir_all(folder)?;
    let stem = Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = Path::new(name)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for n in 1u32..10_000 {
        let candidate = if n == 1 {
            folder.join(name)
        } else {
            folder.join(format!("{stem} ({n}){ext}"))
        };
        if candidate.exists() {
            if same_bytes(&candidate, file)? {
                return Ok(Adopted::Reused(candidate));
            }
            continue;
        }
        let temp = folder.join(format!(
            ".{}.{}.copying",
            name.to_string_lossy(),
            std::process::id()
        ));
        let copied = std::fs::copy(file, &temp).and_then(|_| {
            // Never replace a file that appeared meanwhile.
            if candidate.exists() {
                return Err(Error::new(ErrorKind::AlreadyExists, "name taken"));
            }
            std::fs::rename(&temp, &candidate)
        });
        if let Err(e) = copied {
            let _ = std::fs::remove_file(&temp);
            return Err(e);
        }
        return Ok(Adopted::Copied(candidate));
    }
    Err(Error::other("too many files with that name"))
}

/// Whether two files hold the same bytes.
fn same_bytes(a: &Path, b: &Path) -> std::io::Result<bool> {
    use std::io::Read;
    if std::fs::metadata(a)?.len() != std::fs::metadata(b)?.len() {
        return Ok(false);
    }
    let (mut fa, mut fb) = (std::fs::File::open(a)?, std::fs::File::open(b)?);
    let (mut ba, mut bb) = (vec![0u8; 64 * 1024], vec![0u8; 64 * 1024]);
    loop {
        let n = fa.read(&mut ba)?;
        if n == 0 {
            return Ok(true);
        }
        fb.read_exact(&mut bb[..n])?;
        if ba[..n] != bb[..n] {
            return Ok(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_part_refuses_escapes() {
        assert_eq!(
            relative_part("project:sourceimages/a b.png"),
            Some(PathBuf::from("sourceimages").join("a b.png"))
        );
        assert_eq!(relative_part("project:../x.png"), None);
        assert_eq!(relative_part("project:a/../../x.png"), None);
        assert_eq!(relative_part("project:"), None);
        assert_eq!(relative_part("/abs/x.png"), None);
    }

    #[test]
    fn relative_to_uses_forward_slashes_and_needs_inside() {
        let root = Path::new("/p/Proj");
        assert_eq!(
            relative_to(root, Path::new("/p/Proj/sourceimages/w/oak.png")).as_deref(),
            Some("sourceimages/w/oak.png")
        );
        assert_eq!(relative_to(root, Path::new("/p/Other/oak.png")), None);
        assert_eq!(relative_to(root, Path::new("/p/Proj")), None);
        assert_eq!(relative_to(root, Path::new("rel/oak.png")), None);
    }
}
