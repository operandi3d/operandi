//! Multiple active meshes, stage 1 (Jason, 2026-11-08): every selected mesh
//! item is active. In Vertex, Edge and Polygon modes each one shows and
//! picks its components, and keeps its own component selection.
//!
//! The primary item (`Selection::item`) keeps its components in the
//! existing `vertices`/`edges`/`faces` fields, so every tool keeps working
//! on it unchanged. The other active meshes keep theirs in
//! `Selection::others`. Mesh-editing tools still touch only the primary.
use crate::{Component, Mode, Result, Scene, Selection, SelectionOperation};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One mesh's selected (and remembered) vertices, edges and polygons, in
/// selection order. Edges are sorted endpoint pairs, as in `Selection`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentSelection {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vertices: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<(u32, u32)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub faces: Vec<u32>,
}
impl ComponentSelection {
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.edges.is_empty() && self.faces.is_empty()
    }
    /// How many of `mode`'s components are selected (0 in Item mode).
    pub fn count(&self, mode: Mode) -> usize {
        match mode {
            Mode::Vertex => self.vertices.len(),
            Mode::Edge => self.edges.len(),
            Mode::Polygon => self.faces.len(),
            Mode::Item => 0,
        }
    }
    fn clear(&mut self, mode: Mode) {
        match mode {
            Mode::Vertex => self.vertices.clear(),
            Mode::Edge => self.edges.clear(),
            Mode::Polygon => self.faces.clear(),
            Mode::Item => {}
        }
    }
}

impl Selection {
    /// Mesh `id`'s selected vertices: the primary fields for the primary
    /// item, else its `others` entry (empty when it has none).
    pub fn vertices_of(&self, id: u64) -> &[u32] {
        if self.item == Some(id) {
            &self.vertices
        } else {
            self.others.get(&id).map_or(&[], |c| &c.vertices)
        }
    }
    pub fn edges_of(&self, id: u64) -> &[(u32, u32)] {
        if self.item == Some(id) {
            &self.edges
        } else {
            self.others.get(&id).map_or(&[], |c| &c.edges)
        }
    }
    pub fn faces_of(&self, id: u64) -> &[u32] {
        if self.item == Some(id) {
            &self.faces
        } else {
            self.others.get(&id).map_or(&[], |c| &c.faces)
        }
    }
    /// How many of the current mode's components are selected on every
    /// mesh together (the primary's alone in Item mode is not counted).
    pub fn component_count(&self) -> usize {
        let primary = ComponentSelection {
            vertices: self.vertices.clone(),
            edges: self.edges.clone(),
            faces: self.faces.clone(),
        };
        primary.count(self.mode)
            + self
                .others
                .values()
                .map(|c| c.count(self.mode))
                .sum::<usize>()
    }
    /// Take mesh `id`'s components out of the selection (leaving it with
    /// none).
    pub(crate) fn take_components(&mut self, id: u64) -> ComponentSelection {
        if self.item == Some(id) {
            ComponentSelection {
                vertices: std::mem::take(&mut self.vertices),
                edges: std::mem::take(&mut self.edges),
                faces: std::mem::take(&mut self.faces),
            }
        } else {
            self.others.remove(&id).unwrap_or_default()
        }
    }
    /// The selection as mesh `id` sees it alone: `id` the only selected
    /// item, its components in the primary fields, the same mode (and tag
    /// mode when it is the primary).
    pub(crate) fn sole(&self, id: u64) -> Selection {
        let mut sole = Selection {
            item: Some(id),
            mode: self.mode,
            tag: if self.item == Some(id) {
                self.tag.clone()
            } else {
                None
            },
            ..Default::default()
        };
        sole.put_components(id, self.clone().take_components(id));
        sole
    }
    /// Give mesh `id` the components a sole-mesh run (`sole`) left in
    /// `after`, and take its mode (and tag mode for the primary).
    pub(crate) fn take_sole_result(&mut self, id: u64, mut after: Selection) {
        let components = after.take_components(id);
        self.put_components(id, components);
        self.mode = after.mode;
        if self.item == Some(id) {
            self.tag = after.tag;
        }
    }
    /// Give mesh `id` these components (replacing its own).
    pub(crate) fn put_components(&mut self, id: u64, c: ComponentSelection) {
        if self.item == Some(id) {
            self.vertices = c.vertices;
            self.edges = c.edges;
            self.faces = c.faces;
        } else if c.is_empty() {
            self.others.remove(&id);
        } else {
            self.others.insert(id, c);
        }
    }
}

impl Scene {
    /// Every selected mesh item, primary first, then the others in
    /// selection order. Stale ids and non-mesh items (cameras, lights,
    /// backdrops, texture locators) are skipped. Hidden meshes count here:
    /// their component selections are kept while they're hidden.
    pub fn selected_meshes(&self) -> Vec<u64> {
        let mut out = Vec::new();
        let mut seen = crate::fxhash::FxHashSet::default();
        for id in self
            .selection
            .item
            .into_iter()
            .chain(self.selection.also.iter().copied())
        {
            if self.item(id).is_some_and(|i| i.is_mesh()) && seen.insert(id) {
                out.push(id);
            }
        }
        out
    }
    /// The active meshes: every selected mesh that is shown (visible, not
    /// display-hidden), primary first. They draw, pick and select
    /// components, and component-mode picking is limited to them (Pick
    /// Other Meshes in Component Modes off); empty there means no limit.
    pub fn active_meshes(&self) -> Vec<u64> {
        self.selected_meshes()
            .into_iter()
            .filter(|&id| !self.is_display_hidden(id) && self.item(id).is_some_and(|i| i.visible))
            .collect()
    }
    /// An active mesh other than the primary item.
    pub fn is_other_active_mesh(&self, id: u64) -> bool {
        self.selection.item != Some(id) && self.active_meshes().contains(&id)
    }
    /// Clear `mode`'s components on every active mesh.
    pub(crate) fn clear_active_components(&mut self, mode: Mode) {
        for id in self.active_meshes() {
            let mut components = self.selection.take_components(id);
            components.clear(mode);
            self.selection.put_components(id, components);
        }
    }
    /// Run `f` with mesh `id` standing in as the only selected item, its
    /// components in the primary fields, then give it the components `f`
    /// left there. The item selection, mode and tag mode come back as
    /// they were (the mode as `f` left it when `f` succeeds). When `f`
    /// fails nothing changes.
    pub(crate) fn as_sole_mesh<R>(
        &mut self,
        id: u64,
        f: impl FnOnce(&mut Scene) -> Result<R>,
    ) -> Result<R> {
        if self.selection.item == Some(id) && self.selection.others.is_empty() {
            return f(self);
        }
        let saved = self.selection.clone();
        self.selection = saved.sole(id);
        let result = f(self);
        let after = std::mem::replace(&mut self.selection, saved);
        if result.is_ok() {
            self.selection.take_sole_result(id, after);
        }
        result
    }
    /// Run `f` on each active mesh in turn (see `as_sole_mesh`), primary
    /// first. Meshes where it fails keep their selection; the call fails
    /// only when it fails on every mesh, with the first error, and then
    /// nothing changes. Provisional (MAM-Q01): one failing mesh doesn't
    /// stop the others.
    pub(crate) fn on_active_meshes(
        &mut self,
        mut f: impl FnMut(&mut Scene) -> Result<()>,
    ) -> Result<()> {
        let ids = self.active_meshes();
        if ids.is_empty() || (ids.len() == 1 && self.selection.item == Some(ids[0])) {
            return f(self);
        }
        let mode = self.selection.mode;
        let mut first_error = None;
        let mut mode_after = None;
        for id in ids {
            self.selection.mode = mode;
            match self.as_sole_mesh(id, &mut f) {
                Ok(()) => {
                    mode_after.get_or_insert(self.selection.mode);
                }
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        match mode_after {
            Some(m) => {
                self.selection.mode = m;
                Ok(())
            }
            None => {
                self.selection.mode = mode;
                Err(first_error.unwrap_or_else(|| "Nothing to select".into()))
            }
        }
    }
    /// A selection command (Select All, Grow, Loop, Convert ...) on every
    /// active mesh.
    pub fn selection_operation_all(&mut self, operation: SelectionOperation) -> Result<()> {
        self.on_active_meshes(|s| s.selection_operation(operation))
    }
    /// A component click on an active mesh: the item selection stays.
    /// `replace` clears that kind of component on every active mesh first.
    pub(crate) fn select_active_component(
        &mut self,
        item: u64,
        component: Component,
        replace: bool,
        remove: bool,
    ) {
        let mode = match component {
            Component::Vertex(_) => Mode::Vertex,
            Component::Edge(..) => Mode::Edge,
            Component::Polygon(_) => Mode::Polygon,
            Component::Item => return,
        };
        if replace {
            self.clear_active_components(mode);
        }
        let mut c = self.selection.take_components(item);
        fn edit<T: PartialEq>(list: &mut Vec<T>, value: T, remove: bool) {
            if remove {
                list.retain(|v| v != &value);
            } else if !list.contains(&value) {
                list.push(value);
            }
        }
        match component {
            Component::Vertex(v) => edit(&mut c.vertices, v, remove),
            Component::Edge(a, b) => edit(&mut c.edges, (a.min(b), a.max(b)), remove),
            Component::Polygon(f) => edit(&mut c.faces, f, remove),
            Component::Item => {}
        }
        self.selection.put_components(item, c);
    }
    /// Drop `others` entries that no longer belong: meshes no longer
    /// selected (or now the primary), gone or no longer meshes, and meshes
    /// whose topology changed since `before` (their remembered components
    /// named the old geometry, e.g. Subdivide in Item mode). Moved points
    /// keep their selection (multi-mesh transforms move them). Indices a mesh no
    /// longer has are dropped too.
    pub fn prune_component_selections(&mut self, before: Option<&Scene>) {
        self.prune_component_selections_keeping(before, &Default::default());
    }
    /// As `prune_component_selections`, but meshes in `fresh` keep their
    /// components through a topology change: a tool that ran on them
    /// (multi_tools.rs) wrote those components for the new geometry.
    pub(crate) fn prune_component_selections_keeping(
        &mut self,
        before: Option<&Scene>,
        fresh: &std::collections::BTreeSet<u64>,
    ) {
        if self.selection.others.is_empty() {
            return;
        }
        let active = self.selected_meshes();
        let primary = self.selection.item;
        let items = &self.items;
        self.selection.others.retain(|id, c| {
            if Some(*id) == primary || !active.contains(id) {
                return false;
            }
            let Some(item) = items.iter().find(|i| i.id == *id) else {
                return false;
            };
            if let Some(before) = before
                && !fresh.contains(id)
                && let Some(old) = before.item(*id)
                && !crate::Shared::same(old.mesh.shared(), item.mesh.shared())
                && (old.mesh.vertices.len() != item.mesh.vertices.len()
                    || old.mesh.faces != item.mesh.faces)
            {
                return false;
            }
            let mesh = &item.mesh;
            c.vertices.retain(|&v| (v as usize) < mesh.vertices.len());
            c.faces.retain(|&f| (f as usize) < mesh.faces.len());
            if !c.edges.is_empty() {
                let edges = mesh.edges();
                c.edges.retain(|e| edges.binary_search(e).is_ok());
            }
            !c.is_empty()
        });
    }
    /// The same checks as the primary's components, per other mesh. Entries
    /// for meshes no longer selected are not checked here: the editor
    /// drops them after every command (`prune_component_selections`).
    pub(crate) fn validate_other_components(&self) -> Result<()> {
        if self.selection.others.is_empty() {
            return Ok(());
        }
        let active = self.selected_meshes();
        for (id, c) in self.selection.others.iter() {
            if self.selection.item == Some(*id) || !active.contains(id) {
                continue;
            }
            let item = self.item(*id).ok_or("Invalid selected item")?;
            check_components(&item.mesh, &c.vertices, &c.edges, &c.faces)?;
        }
        Ok(())
    }
}

/// Duplicate-free, in range, edges present: one mesh's component selection.
pub(crate) fn check_components(
    mesh: &crate::Mesh,
    vertices: &[u32],
    edges: &[(u32, u32)],
    faces: &[u32],
) -> Result<()> {
    use std::collections::BTreeSet;
    if vertices.iter().collect::<BTreeSet<_>>().len() != vertices.len()
        || edges.iter().collect::<BTreeSet<_>>().len() != edges.len()
        || faces.iter().collect::<BTreeSet<_>>().len() != faces.len()
    {
        return Err("Duplicate component selection".into());
    }
    if vertices.iter().any(|&v| v as usize >= mesh.vertices.len())
        || faces.iter().any(|&f| f as usize >= mesh.faces.len())
        || (!edges.is_empty() && {
            // Edge identities are sorted canonical endpoint pairs. Built
            // once per validation; never cached across mesh edits.
            let all = mesh.edges();
            edges.iter().any(|e| all.binary_search(e).is_err())
        })
    {
        return Err("Invalid component selection".into());
    }
    Ok(())
}

impl crate::Editor {
    /// MAM-01 tool safety: tools edit only the primary mesh for now. With
    /// nothing of the current mode selected on the primary but components
    /// selected on another active mesh, a tool would treat "nothing
    /// selected" as the whole primary mesh; refuse instead. Shape tools
    /// (which add to the primary whatever is selected) are not affected.
    pub(crate) fn guard_primary_only_edit(&self, before: &Scene, creating: bool) -> Result<()> {
        let s = &before.selection;
        if creating
            || s.others.is_empty()
            || !matches!(s.mode, Mode::Vertex | Mode::Edge | Mode::Polygon)
        {
            return Ok(());
        }
        let primary_count = match s.mode {
            Mode::Vertex => s.vertices.len(),
            Mode::Edge => s.edges.len(),
            Mode::Polygon => s.faces.len(),
            Mode::Item => 0,
        };
        if primary_count > 0 || s.others.values().all(|c| c.count(s.mode) == 0) {
            return Ok(());
        }
        let Some(id) = s.item else {
            return Ok(());
        };
        let (Some(old), Some(new)) = (before.item(id), self.scene.item(id)) else {
            return Ok(());
        };
        if crate::Shared::same(old.mesh.shared(), new.mesh.shared()) || *old.mesh == *new.mesh {
            return Ok(());
        }
        Err(format!(
            "Tools edit only the primary mesh ({}) for now; select components on it, or make another mesh primary in the Items list",
            old.name
        ))
    }
    /// Whether a shape (primitive) tool or numeric constructor is open:
    /// those add geometry to the primary mesh whatever is selected.
    pub(crate) fn creating_shape(&self) -> bool {
        self.session
            .live
            .as_ref()
            .is_some_and(|l| l.tool.is_primitive())
            || self.session.ellipsoid.is_some()
            || self.session.shape.is_some()
            || self.session.cone.is_some()
            || self.session.pen_draft.is_some()
    }
}

/// Keep `others` keyed by u64 in a stable order (serde map of u64 keys).
/// Boxed: `Selection` sits in every scene copy the editor and app keep,
/// and the app state is close to the stack limit.
pub type OtherComponents = Box<BTreeMap<u64, ComponentSelection>>;
pub(crate) fn no_others(others: &OtherComponents) -> bool {
    others.is_empty()
}
