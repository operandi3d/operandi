//! Named selection sets (SEL-081/082): saved vertex, edge or polygon
//! memberships of one mesh that can be recalled later, plus named groups of
//! sets.
//!
//! Sets live in the mesh's attributes (`vertex_sets`, `edge_sets`,
//! `polygon_sets`), so they travel with the mesh, are saved in the native
//! scene file and are restored by undo. Groups (`set_groups`) store set names
//! only. Every command works on the active mesh and on the sets of the
//! current selection mode (SEL-081: only same-mode sets are offered).
//!
//! Provisional choices are logged as SET-Q01..Q08 in
//! `docs/SPECIFICATION_QUESTIONS.md`.
use crate::{Editor, Mesh, Mode, Result, Scene};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Polygon sets Operandi itself names that were renamed: (old, new).
/// Adding to the new name first moves an old-named set's members into it,
/// so scenes saved before the rename keep one set.
const RENAMED_POLYGON_SETS: &[(&str, &str)] = &[
    // Retopo Edge Flow's set (TC-53) before 2026-11-23, to match RetopoEdgeLock.
    ("RetopoFlow", "RetopoEdgeFlow"),
];

/// Which components a selection set holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SetKind {
    Vertex,
    Edge,
    Polygon,
}
impl SetKind {
    /// The set kind of a component selection mode; none in Item mode.
    pub fn of_mode(mode: Mode) -> Option<Self> {
        match mode {
            Mode::Vertex => Some(Self::Vertex),
            Mode::Edge => Some(Self::Edge),
            Mode::Polygon => Some(Self::Polygon),
            Mode::Item => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::Edge => "edge",
            Self::Polygon => "polygon",
        }
    }
}

/// One member of a selection set group: a set named by kind and name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetRef {
    pub kind: SetKind,
    pub name: String,
}

/// How Use Selection Set changes the live selection (SEL-082).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UseSetMode {
    /// The selection becomes the set (also Quick Select).
    #[default]
    Replace,
    /// The set's members are added to the selection.
    Add,
    /// The set's members are removed from the selection.
    Remove,
}

/// Longest set or group name accepted from a command.
const MAX_NAME: usize = 256;

/// A trimmed, non-empty name of at most `MAX_NAME` characters.
fn clean_name(name: &str, what: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(format!("Give the {what} a name"));
    }
    if name.chars().count() > MAX_NAME {
        return Err(format!(
            "A {what} name can have at most {MAX_NAME} characters"
        ));
    }
    Ok(name.to_owned())
}

/// Names of the sets of one kind on a mesh, sorted.
pub fn set_names(mesh: &Mesh, kind: SetKind) -> Vec<String> {
    let a = &mesh.attributes;
    match kind {
        SetKind::Vertex => a.vertex_sets.keys().cloned().collect(),
        // Seam maps share the edge sets' storage under their own prefix
        // (uv_seams.rs) and aren't selection sets.
        SetKind::Edge => a
            .edge_sets
            .keys()
            .filter(|n| !crate::uv_seams::is_seam_key(n))
            .cloned()
            .collect(),
        SetKind::Polygon => a.polygon_sets.keys().cloned().collect(),
    }
}

fn has_set(mesh: &Mesh, kind: SetKind, name: &str) -> bool {
    let a = &mesh.attributes;
    match kind {
        SetKind::Vertex => a.vertex_sets.contains_key(name),
        SetKind::Edge => a.edge_sets.contains_key(name),
        SetKind::Polygon => a.polygon_sets.contains_key(name),
    }
}

/// Drop edge-set members that are no longer edges of their mesh. Edits that
/// remove or rebuild edges call this through `Editor::record`, so a saved
/// edge set never names an edge the mesh does not have. Meshes without edge
/// sets are untouched.
pub(crate) fn prune_edge_sets(scene: &mut Scene) {
    for item in &mut scene.items {
        if item.mesh.attributes.edge_sets.is_empty() {
            continue;
        }
        let edges = item.mesh.edges();
        let stale = item
            .mesh
            .attributes
            .edge_sets
            .values()
            .flatten()
            .any(|e| edges.binary_search(e).is_err());
        if stale {
            for set in item.mesh.attributes.edge_sets.values_mut() {
                set.retain(|e| edges.binary_search(e).is_ok());
            }
        }
    }
}

/// Carry edge sets through a point renumbering: each endpoint follows
/// `point`, and an edge whose endpoint went away leaves the set.
pub(crate) fn remap_edge_sets(
    sets: &BTreeMap<String, BTreeSet<(u32, u32)>>,
    point: impl Fn(u32) -> Option<u32>,
) -> BTreeMap<String, BTreeSet<(u32, u32)>> {
    sets.iter()
        .map(|(name, set)| {
            let members = set
                .iter()
                .filter_map(|&(a, b)| {
                    let (a, b) = (point(a)?, point(b)?);
                    (a != b).then_some((a.min(b), a.max(b)))
                })
                .collect();
            (name.clone(), members)
        })
        .collect()
}

impl Editor {
    /// The active mesh and the set kind of the current selection mode, for
    /// commands that need both.
    fn set_context(&self, what: &str) -> Result<SetKind> {
        if self.i2_active() {
            return Err(format!("Finish or Cancel the Extrude stage before {what}"));
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err(format!("Finish the selection gesture before {what}"));
        }
        let kind = SetKind::of_mode(self.scene.selection.mode)
            .ok_or_else(|| format!("{what} works in Vertex, Edge or Polygon selection mode"))?;
        let item = self
            .scene
            .active()
            .ok_or_else(|| format!("Select a mesh before {what}"))?;
        if !self.scene.shows(item) {
            return Err(format!("Show the mesh before {what}"));
        }
        Ok(kind)
    }

    /// Why a Selection Set command named `what` can't run right now (a
    /// tool or gesture in progress, wrong mode, no visible mesh), or
    /// `None`: the check every Selection Set command makes first, which
    /// their buttons grey with (UIC-223).
    pub fn selection_set_refusal(&self, what: &str) -> Option<String> {
        self.set_context(what).err()
    }

    /// Whether the current mode has anything selected (what Assign
    /// Selection Set saves).
    pub fn has_set_selection(&self) -> bool {
        let s = &self.scene.selection;
        match SetKind::of_mode(s.mode) {
            Some(SetKind::Vertex) => !s.vertices.is_empty(),
            Some(SetKind::Edge) => !s.edges.is_empty(),
            Some(SetKind::Polygon) => !s.faces.is_empty(),
            None => false,
        }
    }

    /// Names of the active mesh's selection sets for the current selection
    /// mode, sorted (empty in Item mode or with no mesh). For set lists.
    pub fn selection_set_names(&self) -> Vec<String> {
        match (
            SetKind::of_mode(self.scene.selection.mode),
            self.scene.active(),
        ) {
            (Some(kind), Some(item)) => set_names(&item.mesh, kind),
            _ => Vec::new(),
        }
    }

    /// The active mesh's selection set groups, by name, with the sets each
    /// holds that still exist.
    pub fn selection_set_groups(&self) -> Vec<(String, Vec<SetRef>)> {
        let Some(item) = self.scene.active() else {
            return Vec::new();
        };
        item.mesh
            .attributes
            .set_groups
            .iter()
            .map(|(name, sets)| {
                let live = sets
                    .iter()
                    .filter(|s| has_set(&item.mesh, s.kind, &s.name))
                    .cloned()
                    .collect();
                (name.clone(), live)
            })
            .collect()
    }

    /// Commit an attribute-only change of the active mesh as one undo step.
    /// Topology is untouched, so component identities are kept.
    fn commit_set_edit(&mut self, label: &str, before: Scene) -> Result<()> {
        if self.scene == before {
            return Ok(());
        }
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.observe(&self.scene))
        {
            self.scene = before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }

    /// Assign Selection Set: save the current selection under `name` as a
    /// set of the current mode. An existing set of that name gains the
    /// selection (SET-Q01, provisional); nothing is removed from it.
    pub(crate) fn assign_selection_set(&mut self, name: String) -> Result<()> {
        const WHAT: &str = "Assign Selection Set";
        let name = clean_name(&name, "selection set")?;
        let kind = self.set_context(WHAT)?;
        let selection = self.scene.selection.clone();
        let before = self.scene.clone();
        let item = self.scene.active_mut().ok_or("Select a mesh")?;
        let mesh: &mut Mesh = &mut item.mesh;
        let (points, faces) = (mesh.vertices.len(), mesh.faces.len());
        match kind {
            SetKind::Vertex => {
                if selection.vertices.is_empty() {
                    return Err(format!("Select vertices before {WHAT}"));
                }
                if selection.vertices.iter().any(|&v| v as usize >= points) {
                    return Err("A selected vertex no longer exists".into());
                }
                mesh.attributes
                    .vertex_sets
                    .entry(name)
                    .or_default()
                    .extend(selection.vertices);
            }
            SetKind::Edge => {
                if selection.edges.is_empty() {
                    return Err(format!("Select edges before {WHAT}"));
                }
                let edges = mesh.edges();
                let members: Vec<(u32, u32)> = selection
                    .edges
                    .iter()
                    .map(|&(a, b)| (a.min(b), a.max(b)))
                    .collect();
                if members.iter().any(|e| edges.binary_search(e).is_err()) {
                    return Err("A selected edge no longer exists".into());
                }
                mesh.attributes
                    .edge_sets
                    .entry(name)
                    .or_default()
                    .extend(members);
            }
            SetKind::Polygon => {
                if selection.faces.is_empty() {
                    return Err(format!("Select polygons before {WHAT}"));
                }
                if selection.faces.iter().any(|&f| f as usize >= faces) {
                    return Err("A selected polygon no longer exists".into());
                }
                let sets = &mut mesh.attributes.polygon_sets;
                // A set under its old name joins the new one, in this step.
                if let Some((old, _)) = RENAMED_POLYGON_SETS.iter().find(|(_, new)| *new == name)
                    && let Some(members) = sets.remove(*old)
                {
                    sets.entry(name.clone()).or_default().extend(members);
                }
                sets.entry(name).or_default().extend(selection.faces);
            }
        }
        self.commit_set_edit(WHAT, before)
    }

    /// Use Selection Set: select, deselect or replace with the members of
    /// the current mode's set `name`. A selection change, so it is undone
    /// like any other selection step.
    pub(crate) fn use_selection_set(&mut self, name: String, mode: UseSetMode) -> Result<()> {
        const WHAT: &str = "Use Selection Set";
        let kind = self.set_context(WHAT)?;
        let name = name.trim();
        let item = self.scene.active().ok_or("Select a mesh")?;
        let id = item.id;
        let a = &item.mesh.attributes;
        let missing = || format!("No {} selection set named \"{name}\"", kind.label());
        let before = self.scene.clone();
        fn apply<T: Copy + Ord>(live: &mut Vec<T>, set: &BTreeSet<T>, mode: UseSetMode) {
            match mode {
                UseSetMode::Replace => *live = set.iter().copied().collect(),
                UseSetMode::Add => {
                    let have: BTreeSet<T> = live.iter().copied().collect();
                    live.extend(set.iter().copied().filter(|m| !have.contains(m)));
                }
                UseSetMode::Remove => live.retain(|m| !set.contains(m)),
            }
        }
        match kind {
            SetKind::Vertex => {
                let set = a.vertex_sets.get(name).ok_or_else(missing)?.clone();
                apply(&mut self.scene.selection.vertices, &set, mode);
            }
            SetKind::Edge => {
                let set = a.edge_sets.get(name).ok_or_else(missing)?.clone();
                apply(&mut self.scene.selection.edges, &set, mode);
            }
            SetKind::Polygon => {
                let set = a.polygon_sets.get(name).ok_or_else(missing)?.clone();
                apply(&mut self.scene.selection.faces, &set, mode);
            }
        }
        debug_assert_eq!(self.scene.selection.item, Some(id));
        if self.scene != before {
            self.record_selection("Selection".into(), before);
        }
        Ok(())
    }

    /// Delete Selection Set: remove the current mode's set `name`, or every
    /// set of the current mode with `None`. Geometry is never deleted, and
    /// the set leaves any group holding it.
    pub(crate) fn delete_selection_set(&mut self, name: Option<String>) -> Result<()> {
        const WHAT: &str = "Delete Selection Set";
        let kind = self.set_context(WHAT)?;
        let before = self.scene.clone();
        let item = self.scene.active_mut().ok_or("Select a mesh")?;
        let doomed: Vec<String> = match &name {
            Some(name) => {
                let name = name.trim().to_owned();
                if !has_set(&item.mesh, kind, &name) {
                    return Err(format!(
                        "No {} selection set named \"{name}\"",
                        kind.label()
                    ));
                }
                vec![name]
            }
            None => {
                let all = set_names(&item.mesh, kind);
                if all.is_empty() {
                    return Err(format!("This mesh has no {} selection sets", kind.label()));
                }
                all
            }
        };
        let a2 = &mut item.mesh.attributes;
        for name in &doomed {
            match kind {
                SetKind::Vertex => drop(a2.vertex_sets.remove(name)),
                SetKind::Edge => drop(a2.edge_sets.remove(name)),
                SetKind::Polygon => drop(a2.polygon_sets.remove(name)),
            }
        }
        // Groups keep their name when their last set goes (SET-Q06).
        for sets in a2.set_groups.values_mut() {
            sets.retain(|s| !(s.kind == kind && doomed.contains(&s.name)));
        }
        self.commit_set_edit(WHAT, before)
    }

    /// Create Selection Set Group: a group `name` holding the current mode's
    /// sets `sets`, or all of them when `sets` is empty (SET-Q05). An
    /// existing group gains the sets it does not hold yet.
    pub(crate) fn create_selection_set_group(
        &mut self,
        name: String,
        sets: Vec<String>,
    ) -> Result<()> {
        const WHAT: &str = "Create Selection Set Group";
        let name = clean_name(&name, "selection set group")?;
        let kind = self.set_context(WHAT)?;
        let before = self.scene.clone();
        let item = self.scene.active_mut().ok_or("Select a mesh")?;
        let members: Vec<String> = if sets.is_empty() {
            set_names(&item.mesh, kind)
        } else {
            sets.iter().map(|s| s.trim().to_owned()).collect()
        };
        if members.is_empty() {
            return Err(format!(
                "Assign a {} selection set before {WHAT}",
                kind.label()
            ));
        }
        if let Some(gone) = members.iter().find(|s| !has_set(&item.mesh, kind, s)) {
            return Err(format!(
                "No {} selection set named \"{gone}\"",
                kind.label()
            ));
        }
        let group = item.mesh.attributes.set_groups.entry(name).or_default();
        for set in members {
            let set = SetRef { kind, name: set };
            if !group.contains(&set) {
                group.push(set);
            }
        }
        self.commit_set_edit(WHAT, before)
    }

    /// Delete Selection Set Group: remove group `name`; its sets stay.
    pub(crate) fn delete_selection_set_group(&mut self, name: String) -> Result<()> {
        const WHAT: &str = "Delete Selection Set Group";
        self.set_context(WHAT)?;
        let name = name.trim();
        let before = self.scene.clone();
        let item = self.scene.active_mut().ok_or("Select a mesh")?;
        if item.mesh.attributes.set_groups.remove(name).is_none() {
            return Err(format!("No selection set group named \"{name}\""));
        }
        self.commit_set_edit(WHAT, before)
    }
}
