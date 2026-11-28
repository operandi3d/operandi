//! VP-04 (Jason, 2026-11-09): when no mesh is selected, the last selected
//! mesh stays active. It keeps the active look and stays the mesh being
//! edited, as if it were still selected. Only a scene in which no mesh has
//! been selected yet (a file just opened) has every mesh inactive
//! (provisional, VPQ-10). Deleting or hiding the remembered mesh stops it
//! being active.
//!
//! The scene remembers the mesh (`Scene::last_mesh`); the views draw it
//! active and pick on it (`shown_active_meshes`, `is_active_mesh`); and the
//! editor selects it again when a command would edit it
//! (`Editor::adopt_last_mesh`).
use crate::{Command, Scene};

/// The remembered mesh. Not document state: two scenes that differ only
/// here are the same document (undo, save and the "changed" checks compare
/// scenes), so it always compares equal.
#[derive(Clone, Copy, Debug, Default)]
pub struct LastMesh(pub Option<u64>);
impl PartialEq for LastMesh {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Scene {
    /// Remember the primary selected mesh, if a mesh is selected.
    pub fn note_last_mesh(&mut self) {
        if let Some(&id) = self.selected_meshes().first() {
            self.last_mesh = LastMesh(Some(id));
        }
    }
    /// The mesh that stays active because no mesh is selected (VP-04):
    /// the remembered one, while it still exists and is shown.
    pub fn remembered_mesh(&self) -> Option<u64> {
        let id = self.last_mesh.0?;
        if !self.selected_meshes().is_empty() {
            return None;
        }
        let item = self.item(id)?;
        (item.is_mesh() && item.visible && !self.is_display_hidden(id)).then_some(id)
    }
    /// The meshes the views treat as active: the active meshes, or the
    /// remembered mesh while no mesh is selected.
    pub fn shown_active_meshes(&self) -> Vec<u64> {
        let active = self.active_meshes();
        if active.is_empty() {
            self.remembered_mesh().into_iter().collect()
        } else {
            active
        }
    }
    /// Every item `is_active_mesh` holds for, as a set: what a loop over
    /// all items should ask instead of `is_active_mesh` per item, which
    /// searches the selection each time.
    pub fn active_item_set(&self) -> crate::fxhash::FxHashSet<u64> {
        let mut set: crate::fxhash::FxHashSet<u64> = self
            .selection
            .item
            .into_iter()
            .chain(self.selection.also.iter().copied())
            .collect();
        set.extend(self.remembered_mesh());
        set
    }
    /// Whether item `id` draws with the active look (VP-04/05): it is
    /// selected, or it is the remembered mesh while no mesh is selected.
    pub fn is_active_mesh(&self, id: u64) -> bool {
        self.selection.has_item(id) || self.remembered_mesh() == Some(id)
    }
    /// With nothing selected at all, select the remembered mesh again (in
    /// the same selection mode). True when it did. A selected camera,
    /// light or other item is left alone.
    pub fn adopt_last_mesh(&mut self) -> bool {
        if self.selection.item.is_some() || !self.selection.also.is_empty() {
            return false;
        }
        let Some(id) = self.remembered_mesh() else {
            return false;
        };
        self.select_item(Some(id));
        true
    }
}

impl crate::Editor {
    /// Before `cmd` runs: when nothing is selected, select the remembered
    /// mesh so the command edits it (VP-04). Returns the selection to put
    /// back when the command turns out not to touch it (`settle_last_mesh`).
    /// Only in Vertex, Edge and Polygon modes, where the mesh is what is
    /// edited: in Item mode commands act on the item selection, and
    /// nothing selected means nothing to act on (provisional, VPQ note in
    /// SPECIFICATION_QUESTIONS.md). Selection, history and view commands
    /// never adopt: they pick, clear or restore a selection of their own.
    pub(crate) fn adopt_last_mesh_for(&mut self, cmd: &Command) -> Option<crate::Selection> {
        if matches!(
            cmd,
            Command::Undo
                | Command::Redo
                | Command::Cancel
                | Command::Mode(_)
                | Command::Workplane(_)
                | Command::SelectItem(_)
                | Command::SelectItems(_)
                | Command::Select { .. }
                | Command::ClearSelection
                | Command::EndSelection { .. }
        ) || item_level(cmd)
            || self.scene.selection.item.is_some()
            || self.scene.selection.mode == crate::Mode::Item
        {
            return None;
        }
        let before = self.scene.selection.clone();
        self.scene.adopt_last_mesh().then_some(before)
    }
    /// After the command: keep the adopted selection only when the command
    /// worked and edited the mesh (its geometry or its component
    /// selection) or opened a tool. Otherwise (it failed, or only hid an
    /// item, renamed one ...) nothing was edited, and the selection goes
    /// back to nothing selected.
    pub(crate) fn settle_last_mesh(
        &mut self,
        unadopted: crate::Selection,
        adopted: &Scene,
        task_before: Option<crate::ActiveTask>,
        ok: bool,
    ) {
        let opened = self.session.active_task() != task_before;
        if ok && (opened || mesh_edited(adopted, &self.scene)) {
            return;
        }
        if self.scene.selection.item == adopted.selection.item
            && self.scene.selection.also.is_empty()
        {
            self.scene.selection = crate::Selection {
                mode: self.scene.selection.mode,
                ..unadopted
            };
        }
    }
}

/// Whether `after` has different mesh geometry or a different component
/// selection than `before`.
fn mesh_edited(before: &Scene, after: &Scene) -> bool {
    let (b, a) = (&before.selection, &after.selection);
    if a.vertices != b.vertices || a.edges != b.edges || a.faces != b.faces || a.tag != b.tag {
        return true;
    }
    before.items.len() != after.items.len()
        || before.items.iter().zip(&after.items).any(|(x, y)| {
            x.id != y.id
                || !crate::Shared::same(x.mesh.shared(), y.mesh.shared()) && x.mesh != y.mesh
        })
}

/// Commands about items, the scene or settings rather than a mesh's
/// geometry: with nothing selected they keep meaning "nothing selected"
/// (add at the end of the list, nothing to parent, and so on).
fn item_level(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::SetRenderSettings(..)
            | Command::SetSceneTime(..)
            | Command::NewMesh
            | Command::NewMeshBeside
            | Command::NewNamedMesh(_)
            | Command::NewBackdrop
            | Command::BackdropLoadImage { .. }
            | Command::EditBackdrop { .. }
            | Command::BackdropAutoFit { .. }
            | Command::BackdropSize { .. }
            | Command::AddBackdrop { .. }
            | Command::LoadImage { .. }
            | Command::AddItem(..)
            | Command::ItemPattern { .. }
            | Command::NewCamera
            | Command::NewLight { .. }
            | Command::EditCamera { .. }
            | Command::CameraAutofocus { .. }
            | Command::CameraBackgroundLoad { .. }
            | Command::SyncCameraToView { .. }
            | Command::ResetTransform { .. }
            | Command::FreezeTransform { .. }
            | Command::EditLight { .. }
            | Command::EditItemCommon { .. }
            | Command::NewGroup { .. }
            | Command::RenameGroup { .. }
            | Command::DeleteGroup { .. }
            | Command::AddToGroup { .. }
            | Command::RemoveFromGroup { .. }
            | Command::SelectGroup { .. }
            | Command::LoadPhotometry { .. }
            | Command::SetRenderCamera { .. }
            | Command::CameraView { .. }
            | Command::DeleteItem
            | Command::DeleteItems { .. }
            | Command::DuplicateHierarchy
            | Command::DuplicateItem
            | Command::InstanceItems
            | Command::InsertItems { .. }
            | Command::RemoveBranch { .. }
            | Command::ItemTransform { .. }
            | Command::Rename { .. }
            | Command::Visibility { .. }
            | Command::SetItemsVisible { .. }
            | Command::VisibilityWithChildren { .. }
            | Command::SetParent { .. }
            | Command::ParentToGroupLocator
            | Command::MoveItems { .. }
            | Command::MergeItems
            | Command::UnmergeItem
            | Command::ToggleItemSelection { .. }
            | Command::ShaderTree { .. }
            | Command::LoadPreset { .. }
            | Command::InsertMaterialPreset { .. }
            | Command::SetEnvironmentImage { .. }
            | Command::ApplyEnvironmentPreset { .. }
            | Command::ApplyMaterialPreset { .. }
            | Command::DropAction { .. }
            | Command::ActionContext { .. }
            | Command::SetActionCenter { .. }
            | Command::ToolClick { .. }
            | Command::ResetActionCenter
            | Command::Symmetry { .. }
            | Command::SpatialFalloff { .. }
            | Command::BackgroundConstraint { .. }
            | Command::AddedFalloffs { .. }
            | Command::SpatialFalloffMix { .. }
            | Command::FalloffElementPick { .. }
            | Command::FalloffStamps { .. }
            | Command::FalloffPath { .. }
            | Command::SelectWeightMap { .. }
            | Command::EnterTagMode { .. }
            | Command::SetPolygonTagKind { .. }
    )
}
