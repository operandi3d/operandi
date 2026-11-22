//! The UV map list (E1; UVE-1 section 1, UVT-01): named UV maps per mesh,
//! one of them selected (current). New, select, rename, delete several,
//! copy a whole map and paste it over another. Every change but Copy is one
//! undo step and is saved with the scene (`MeshAttributes::current_uv_map`,
//! native version 47).
//!
//! Provisional readings (docs/SPECIFICATION_QUESTIONS.md):
//! - UVE-1 Q1: New Map's default name is "Texture"; when the mesh already
//!   has it the next free "Texture 2", "Texture 3" ... is offered.
//! - UVE-1 Q2: deleting a map leaves Image Map layers and Texture Locators
//!   naming it as they are; they show what a missing map shows today (the
//!   renderer's fallback). Renaming doesn't follow into them either, since
//!   locators name maps across meshes.
//! - UVE-1 Q3: Paste works on any mesh whose polygons have the same corner
//!   counts as the copied one (same topology), the active mesh included.
use crate::uv_tools::{DEFAULT_UV_MAP, target_uv_map};
use crate::{Editor, Mesh, Result};
use glam::DVec2;

/// One map's values, polygon by polygon, corner by corner.
pub type UvRows = Vec<Vec<Option<DVec2>>>;

/// The map list's edits (`Command::UvMap`).
#[derive(Clone, Debug, PartialEq)]
pub enum UvMapEdit {
    /// New Map: an empty map, which becomes the selected one. `None` takes
    /// the default name (Q1).
    New(Option<String>),
    /// Click a map: it becomes the selected map.
    Select(String),
    Rename {
        from: String,
        to: String,
    },
    /// Delete one or several maps at once.
    Delete(Vec<String>),
    /// Copy the whole map's values to the UV map clipboard (no undo step).
    Copy(String),
    /// Replace the map's values with the clipboard's.
    Paste(String),
}

/// The UV map clipboard: one whole map's values and the corner count of
/// each polygon they came from.
#[derive(Clone, Debug, PartialEq)]
pub struct UvMapClipboard {
    pub name: String,
    pub rows: UvRows,
}

/// The mesh's selected UV map: the stored one while the mesh still has it,
/// otherwise `target_uv_map`'s choice ("Texture", else the first). `None`
/// when the mesh has no UV map.
pub fn current_map(mesh: &Mesh) -> Option<String> {
    (!mesh.uv_maps.is_empty()).then(|| target_uv_map(mesh, None))
}

/// The name New Map offers: `base` (default "Texture"), or the first free
/// "base 2", "base 3" ... when the mesh has it (UVE-1 Q1, provisional).
pub fn free_name(mesh: &Mesh, base: &str) -> String {
    let base = base.trim();
    let base = if base.is_empty() {
        DEFAULT_UV_MAP
    } else {
        base
    };
    if !mesh.uv_maps.contains_key(base) {
        return base.to_owned();
    }
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|n| !mesh.uv_maps.contains_key(n))
        .expect("a free name")
}

/// An empty map for `mesh`: every corner unmapped.
pub fn empty_rows(mesh: &Mesh) -> UvRows {
    mesh.faces
        .iter()
        .map(|f| vec![None; f.vertices.len()])
        .collect()
}

/// Make sure `mesh` has a UV map to write into and return its name: the
/// selected map, or a new empty "Texture" that becomes selected when the
/// mesh has none (UVT-01, UVE-1 section 1).
pub fn ensure_map(mesh: &mut Mesh) -> String {
    if let Some(name) = current_map(mesh) {
        return name;
    }
    let rows = empty_rows(mesh);
    mesh.uv_maps.insert(DEFAULT_UV_MAP.into(), rows);
    mesh.attributes.current_uv_map = Some(DEFAULT_UV_MAP.into());
    DEFAULT_UV_MAP.into()
}

fn check_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A UV map needs a name".into());
    }
    Ok(name.to_owned())
}

/// Rename a map's companion data (point UVs, corner records) with it.
fn rename_companions(mesh: &mut Mesh, from: &str, to: &str) {
    if let Some(v) = mesh.attributes.point_uvs.remove(from) {
        mesh.attributes.point_uvs.insert(to.to_owned(), v);
    }
    if let Some(v) = mesh.attributes.uv_records.remove(from) {
        mesh.attributes.uv_records.insert(to.to_owned(), v);
    }
}

fn remove_companions(mesh: &mut Mesh, name: &str) {
    mesh.attributes.point_uvs.remove(name);
    mesh.attributes.uv_records.remove(name);
}

/// Apply one list edit to `mesh`. Returns the history label, or `None`
/// when the edit changes nothing in the mesh (Copy).
pub fn apply(
    mesh: &mut Mesh,
    edit: &UvMapEdit,
    clipboard: &mut Option<UvMapClipboard>,
) -> Result<Option<String>> {
    match edit {
        UvMapEdit::New(name) => {
            let name = match name {
                Some(n) => {
                    let n = check_name(n)?;
                    if mesh.uv_maps.contains_key(&n) {
                        return Err(format!("The mesh already has a UV map called {n}"));
                    }
                    n
                }
                None => free_name(mesh, DEFAULT_UV_MAP),
            };
            let rows = empty_rows(mesh);
            mesh.uv_maps.insert(name.clone(), rows);
            mesh.attributes.current_uv_map = Some(name.clone());
            Ok(Some(format!("New UV Map {name}")))
        }
        UvMapEdit::Select(name) => {
            if !mesh.uv_maps.contains_key(name) {
                return Err(format!("The mesh has no UV map called {name}"));
            }
            if current_map(mesh).as_deref() == Some(name.as_str())
                && mesh.attributes.current_uv_map.is_some()
            {
                return Ok(None);
            }
            mesh.attributes.current_uv_map = Some(name.clone());
            Ok(Some(format!("Select UV Map {name}")))
        }
        UvMapEdit::Rename { from, to } => {
            let to = check_name(to)?;
            let rows = mesh
                .uv_maps
                .get(from)
                .cloned()
                .ok_or_else(|| format!("The mesh has no UV map called {from}"))?;
            if to == *from {
                return Ok(None);
            }
            if mesh.uv_maps.contains_key(&to) {
                return Err(format!("The mesh already has a UV map called {to}"));
            }
            let was_current = current_map(mesh).as_deref() == Some(from.as_str());
            mesh.uv_maps.remove(from);
            mesh.uv_maps.insert(to.clone(), rows);
            rename_companions(mesh, from, &to);
            if was_current {
                mesh.attributes.current_uv_map = Some(to.clone());
            }
            Ok(Some(format!("Rename UV Map {from} to {to}")))
        }
        UvMapEdit::Delete(names) => {
            if names.is_empty() {
                return Err("Select the UV maps to delete".into());
            }
            for n in names {
                if !mesh.uv_maps.contains_key(n) {
                    return Err(format!("The mesh has no UV map called {n}"));
                }
            }
            let current = current_map(mesh);
            for n in names {
                mesh.uv_maps.remove(n);
                remove_companions(mesh, n);
            }
            // The selection moves to the first map left, if any.
            mesh.attributes.current_uv_map = match current {
                Some(c) if mesh.uv_maps.contains_key(&c) => Some(c),
                _ => mesh.uv_maps.keys().next().cloned(),
            };
            Ok(Some(if names.len() == 1 {
                format!("Delete UV Map {}", names[0])
            } else {
                format!("Delete {} UV Maps", names.len())
            }))
        }
        UvMapEdit::Copy(name) => {
            let rows = mesh
                .uv_maps
                .get(name)
                .cloned()
                .ok_or_else(|| format!("The mesh has no UV map called {name}"))?;
            *clipboard = Some(UvMapClipboard {
                name: name.clone(),
                rows,
            });
            Ok(None)
        }
        UvMapEdit::Paste(name) => {
            let copied = clipboard
                .as_ref()
                .ok_or("Copy a UV map first (right-click a map, Copy)")?;
            if !mesh.uv_maps.contains_key(name) {
                return Err(format!("The mesh has no UV map called {name}"));
            }
            let fits = copied.rows.len() == mesh.faces.len()
                && copied
                    .rows
                    .iter()
                    .zip(mesh.faces.iter())
                    .all(|(r, f)| r.len() == f.vertices.len());
            if !fits {
                return Err(format!(
                    "{} was copied from a mesh with different polygons; paste it on the same mesh",
                    copied.name
                ));
            }
            let rows = copied.rows.clone();
            mesh.uv_maps.insert(name.clone(), rows);
            mesh.attributes.point_uvs.remove(name);
            if mesh.attributes.uv_records.contains_key(name) {
                let records =
                    crate::corner_uv::records_from_uvs(&mesh.faces, &mesh.uv_maps[name.as_str()]);
                mesh.attributes.uv_records.insert(name.clone(), records);
            }
            Ok(Some(format!("Paste UV Map {} into {name}", copied.name)))
        }
    }
}

impl Editor {
    /// `Command::UvMap`: one list edit on the active mesh, one undo step
    /// (Copy takes none).
    pub(crate) fn uv_map_edit(&mut self, edit: UvMapEdit) -> Result<()> {
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish selection before changing UV maps".into());
        }
        let before = self.scene.clone();
        let mut clipboard = self.uv_map_clipboard.clone();
        let item = self
            .scene
            .active_mut()
            .ok_or("Select a mesh to edit its UV maps")?;
        let mut mesh: Mesh = (*item.mesh).clone();
        let label = apply(&mut mesh, &edit, &mut clipboard)?;
        self.uv_map_clipboard = clipboard;
        let Some(label) = label else {
            return Ok(());
        };
        item.mesh = mesh.into();
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record(label, before);
        Ok(())
    }
}
