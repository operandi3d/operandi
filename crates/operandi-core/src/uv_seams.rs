//! UV seam maps (E13, BEHAVIOR-UV-TOOLS §13, UVS-1 §1.6): named sets of
//! edges where Unwrap cuts. An edge is in a seam map or not (UVT-101).
//!
//! UVT-101 (build guess, provisional UVS-P01): a seam map is stored the
//! way edge selection sets are, in `MeshAttributes::edge_sets` under the
//! key [`SEAM_PREFIX`] + its name, so every edit that renumbers or removes
//! edges carries seam maps exactly as it carries edge sets, and files need
//! no new version. The selection set lists leave these keys out
//! (`selection_sets::set_names`), so seam maps show only as seam maps.
use crate::topology::edge_key;
use crate::{Mesh, Mode, Result};
use std::collections::BTreeSet;

/// The edge-set key prefix of a seam map.
pub const SEAM_PREFIX: &str = "#uvseam:";
/// UVS-P02 (build guess): the seam map a mesh's first Add Seam makes.
pub const DEFAULT_SEAM_MAP: &str = "UV Seam";

pub fn is_seam_key(key: &str) -> bool {
    key.starts_with(SEAM_PREFIX)
}

fn key(name: &str) -> String {
    format!("{SEAM_PREFIX}{name}")
}

/// The mesh's seam map names, sorted.
pub fn seam_maps(mesh: &Mesh) -> Vec<String> {
    mesh.attributes
        .edge_sets
        .keys()
        .filter_map(|k| k.strip_prefix(SEAM_PREFIX).map(str::to_owned))
        .collect()
}

/// The edges of seam map `name` (empty when it doesn't exist).
pub fn seam_edges(mesh: &Mesh, name: &str) -> BTreeSet<(u32, u32)> {
    mesh.attributes
        .edge_sets
        .get(&key(name))
        .cloned()
        .unwrap_or_default()
}

/// The Cut Map popover's buttons (UVS-1 §1.6, UVT §13.1).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SeamEdit {
    /// Add Seam: the selected edges (every edge with none selected) join
    /// the map, made when missing.
    Add(String),
    /// Clear Seam: the selected edges leave the map (all with none
    /// selected).
    Clear(String),
    /// Select Seam: the map's edges become the selection (Edge mode).
    Select(String),
    /// Delete Seam Map.
    Delete(String),
    /// Mark these edges.
    Mark(String, Vec<(u32, u32)>),
    /// Seam Marker (UVT-100): the Conditional Loop grown from the selected
    /// edges (its starting stop rules) goes into the map instead of the
    /// selection.
    Marker(String),
    /// The UV Seam field: which map the buttons use (no undo step).
    Use(String),
}

impl SeamEdit {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Add(_) | Self::Mark(..) => "Add Seam",
            Self::Marker(_) => "Seam Marker",
            Self::Use(_) => "UV Seam",
            Self::Clear(_) => "Clear Seam",
            Self::Select(_) => "Select Seam",
            Self::Delete(_) => "Delete Seam Map",
        }
    }
}

fn check_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A seam map needs a name".into());
    }
    Ok(name)
}

/// The selection's edges (UVS-1 §1.6: Add Seam with no edges selected adds
/// every edge): selected edges, edges between selected points, the
/// selected polygons' edges.
fn chosen_edges(mesh: &Mesh, s: &crate::Selection) -> Vec<(u32, u32)> {
    let all = mesh.edges();
    let picked: BTreeSet<(u32, u32)> = match s.mode {
        Mode::Edge => s.edges.iter().map(|&(a, b)| edge_key(a, b)).collect(),
        Mode::Vertex => {
            let on: BTreeSet<u32> = s.vertices.iter().copied().collect();
            all.iter()
                .copied()
                .filter(|(a, b)| on.contains(a) && on.contains(b))
                .collect()
        }
        Mode::Polygon => s
            .faces
            .iter()
            .filter_map(|&f| mesh.faces.get(f as usize))
            .flat_map(|f| {
                let v = &f.vertices;
                (0..v.len()).map(move |i| edge_key(v[i], v[(i + 1) % v.len()]))
            })
            .collect(),
        Mode::Item => BTreeSet::new(),
    };
    if picked.is_empty() {
        all.to_vec()
    } else {
        picked.into_iter().collect()
    }
}

impl crate::Editor {
    /// The seam map the Cut Map buttons use.
    pub fn uv_seam_name(&self) -> String {
        if self.uv_seam_map.trim().is_empty() {
            DEFAULT_SEAM_MAP.to_owned()
        } else {
            self.uv_seam_map.clone()
        }
    }

    /// `Command::UvSeam`: one Cut Map button, one undo step (Select Seam
    /// changes the selection, also one step).
    pub(crate) fn uv_seam(&mut self, edit: SeamEdit) -> Result<()> {
        if let SeamEdit::Use(name) = &edit {
            self.uv_seam_map = check_name(name)?.to_owned();
            return Ok(());
        }
        self.require_idle()?;
        let label = edit.label();
        let before = self.scene.clone();
        let selection = self.scene.selection.clone();
        let item = self
            .scene
            .active()
            .ok_or_else(|| format!("Select a mesh for {label}"))?;
        let id = item.id;
        let mut mesh: Mesh = (*item.mesh).clone();
        match &edit {
            SeamEdit::Select(name) => {
                let edges = seam_edges(&mesh, check_name(name)?);
                self.scene.selection = crate::Selection {
                    mode: Mode::Edge,
                    item: selection.item,
                    edges: edges.into_iter().collect(),
                    ..Default::default()
                };
                if self.scene.selection != before.selection {
                    self.record(label.into(), before);
                }
                return Ok(());
            }
            SeamEdit::Add(name) => {
                let edges = chosen_edges(&mesh, &selection);
                mesh.attributes
                    .edge_sets
                    .entry(key(check_name(name)?))
                    .or_default()
                    .extend(edges);
            }
            SeamEdit::Mark(name, edges) => {
                mesh.attributes
                    .edge_sets
                    .entry(key(check_name(name)?))
                    .or_default()
                    .extend(edges.iter().map(|&(a, b)| edge_key(a, b)));
            }
            SeamEdit::Marker(name) => {
                if selection.mode != Mode::Edge || selection.edges.is_empty() {
                    return Err("Seam Marker: select an edge in Edge mode to start the loop".into());
                }
                let seeds: Vec<(u32, u32)> = selection.edges.clone();
                let grown = crate::select_tools::conditional_loop(
                    &mesh,
                    &seeds,
                    &BTreeSet::new(),
                    &crate::select_tools::ConditionalLoopOptions::default(),
                )?;
                mesh.attributes
                    .edge_sets
                    .entry(key(check_name(name)?))
                    .or_default()
                    .extend(seeds.iter().chain(&grown).map(|&(a, b)| edge_key(a, b)));
            }
            SeamEdit::Use(_) => unreachable!("handled above"),
            SeamEdit::Clear(name) => {
                let edges: BTreeSet<(u32, u32)> =
                    chosen_edges(&mesh, &selection).into_iter().collect();
                if let Some(set) = mesh.attributes.edge_sets.get_mut(&key(check_name(name)?)) {
                    set.retain(|e| !edges.contains(e));
                }
            }
            SeamEdit::Delete(name) => {
                if mesh
                    .attributes
                    .edge_sets
                    .remove(&key(check_name(name)?))
                    .is_none()
                {
                    return Err(format!("The mesh has no seam map called {name}"));
                }
            }
        }
        if mesh == *self.scene.active().unwrap().mesh {
            return Err(format!("{label}: nothing to change"));
        }
        self.scene.item_mut(id).ok_or("Missing mesh")?.mesh = mesh.into();
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }
}
