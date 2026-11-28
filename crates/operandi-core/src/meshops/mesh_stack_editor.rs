//! Mesh Stack commands on the editor: add, delete, move, enable, set values,
//! select rows, Select by Index, Freeze, Edit Base Mesh (mesh_stack.rs has the
//! data and evaluation). Every stack edit is one undo step of recipe and
//! Base Mesh (SAFE-13); evaluation never records undo.
use crate::mesh_stack::{MeshOp, MeshStack, OpKind, OpStatus, SelectByIndex};
use crate::meshop_selection::{SelKind, SelOp};
use crate::meshop_types::SelElement;
use crate::{Editor, Mode, Result};

/// An op in a stack: its mesh item and its id there. Scripts write it
/// `op:<item>:<op>` (ops aren't scene items in Operandi).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OpRef {
    pub item: u64,
    pub op: u64,
}
impl OpRef {
    pub fn parse(text: &str) -> Option<Self> {
        let rest = text.trim().strip_prefix("op:")?;
        let (item, op) = rest.split_once(':')?;
        Some(Self {
            item: item.trim().parse().ok()?,
            op: op.trim().parse().ok()?,
        })
    }
    pub fn text(self) -> String {
        format!("op:{}:{}", self.item, self.op)
    }
}

/// Select by Index buttons (MO-53).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexEdit {
    Set,
    Clear,
    Add,
    Remove,
}
impl IndexEdit {
    pub fn parse(word: &str) -> Option<Self> {
        Some(match word.trim() {
            "set" | "0" => Self::Set,
            "clear" | "1" => Self::Clear,
            "add" | "2" => Self::Add,
            "remove" | "3" => Self::Remove,
            _ => return None,
        })
    }
}

/// The selected row of the Mesh Operations list: an op, or the item's Base
/// Mesh row (`op: None`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpRow {
    pub item: u64,
    pub op: Option<u64>,
}

/// SAFE-05: what a direct tool says on a mesh with a stack (Q1).
pub const DIRECT_EDIT_REFUSED: &str =
    "This mesh has a Mesh Stack: use Edit Base Mesh to edit its Base Mesh, or Freeze the stack";

/// One change to an op's selection ops.
#[derive(Clone, Debug, PartialEq)]
pub enum SelOpEdit {
    Add(SelOp),
    /// Replace the `n`th (bottom = 0).
    Set(usize, SelOp),
    Remove(usize),
    /// Move the `n`th to position `to` (bottom = 0); the order matters for
    /// Subtract and Multiply (#19).
    Move(usize, usize),
}

impl Editor {
    /// The elements of kind `element` selected on mesh `item` (its Base
    /// Mesh), numbered as the op's Selection input numbers them (edges as
    /// `Mesh::edges`), with that mesh's count of them; `None` unless the
    /// selection mode matches.
    fn picked_on(&self, item: u64, element: SelElement) -> Option<(Vec<u32>, u32)> {
        let sel = &self.scene.selection;
        if sel.item != Some(item) {
            return None;
        }
        let mesh = &self.scene.item(item)?.mesh;
        let mut picked: Vec<u32> = match (element, sel.mode) {
            (SelElement::Polygon, Mode::Polygon) => sel.faces.clone(),
            (SelElement::Point, Mode::Vertex) => sel.vertices.clone(),
            (SelElement::Edge, Mode::Edge) => {
                let edges = mesh.edges();
                sel.edges
                    .iter()
                    .filter_map(|&(a, b)| edges.binary_search(&(a.min(b), a.max(b))).ok())
                    .map(|i| i as u32)
                    .collect()
            }
            _ => return None,
        };
        picked.sort_unstable();
        picked.dedup();
        Some((picked, element.count(mesh) as u32))
    }
    fn stack_item(&self, item: u64) -> Result<&crate::Item> {
        self.scene
            .item(item)
            .filter(|i| i.is_mesh())
            .ok_or_else(|| "That mesh item doesn't exist".to_string())
    }
    fn op_index(&self, r: OpRef) -> Result<usize> {
        self.stack_item(r.item)?
            .mesh_stack
            .as_ref()
            .and_then(|s| s.index_of(r.op))
            .ok_or_else(|| "That mesh operation doesn't exist".to_string())
    }
    fn stack_mut(&mut self, item: u64) -> &mut MeshStack {
        let item = self.scene.item_mut(item).expect("checked by the caller");
        item.mesh_stack.get_or_insert_with(Default::default)
    }
    /// MO-23, SAFE-07: an edit whose op links would make a loop is undone
    /// (the scene goes back to `before`) and refused.
    fn refuse_loop(&mut self, before: &crate::Scene) -> Result<()> {
        let Some((item, _)) = crate::mesh_stack::find_loop(&self.scene) else {
            return Ok(());
        };
        let name = self
            .scene
            .item(item)
            .map_or_else(String::new, |i| i.name.clone());
        self.scene = before.clone();
        Err(format!(
            "That would make a dependency loop: {name} would end up reading its own result"
        ))
    }
    /// An empty stack is no stack: the mesh is a plain mesh again.
    fn drop_empty_stack(&mut self, item: u64) {
        if let Some(i) = self.scene.item_mut(item)
            && i.mesh_stack.as_ref().is_some_and(|s| s.ops.is_empty())
        {
            i.mesh_stack = None;
        }
    }

    /// `meshop.create type:<id>` (MO-15, MO-18, MO-20): adds an op to the
    /// selected mesh, directly above the selected row (on top when the
    /// mesh or nothing in its stack is selected), and selects it.
    pub(crate) fn meshop_create(&mut self, type_id: &str) -> Result<()> {
        self.require_idle()?;
        let kind = OpKind::new(type_id)?;
        let item = self
            .scene
            .active()
            .filter(|i| i.is_mesh())
            .ok_or("Select a mesh item to add a mesh operation to")?;
        let id = item.id;
        let len = item.mesh_stack.as_ref().map_or(0, |s| s.ops.len());
        let at = match self.meshop_row.filter(|r| r.item == id) {
            Some(OpRow { op: None, .. }) => 0,
            Some(OpRow { op: Some(op), .. }) => item
                .mesh_stack
                .as_ref()
                .and_then(|s| s.index_of(op))
                .map_or(len, |i| i + 1),
            None => len,
        };
        // MO-20 auto-capture: the components selected on the mesh become a
        // Select by Index. They number the Base Mesh, so they're captured
        // only when the op's input is the Base Mesh (provisional, MOQ-01).
        // #19: in the kind the op picks (polygons, edges or points), from
        // the matching selection mode.
        let element = kind
            .def()
            .map_or(SelElement::Polygon, |d| d.selection_element());
        let selection = (kind.takes_selection() && at == 0)
            .then(|| self.picked_on(id, element))
            .flatten()
            .filter(|(p, _)| !p.is_empty())
            .map(|(polygons, made_on)| SelectByIndex {
                polygons,
                made_on: Some(made_on),
            });
        // The other items selected with it (BOP-10: Boolean's driver).
        let kind = match (&kind, kind.def()) {
            (OpKind::Op { type_id, values }, Some(def)) => {
                let picked: Vec<u64> = self
                    .scene
                    .selected_items()
                    .into_iter()
                    .filter(|&i| i != id)
                    .collect();
                let mut values = values.clone();
                def.created(&mut values, &picked);
                // A Falloff step takes a copy of the active falloff
                // (meshop_tool_pipe.rs, `falloff`).
                if values.get("falloff").is_some_and(|v| v.is_null())
                    && let Some(w) = self.primitive_weights()
                {
                    values["falloff"] = serde_json::to_value(w).map_err(|e| e.to_string())?;
                }
                OpKind::Op {
                    type_id: type_id.clone(),
                    values,
                }
            }
            _ => kind,
        };
        let label = format!("Add {}", kind.label());
        let before = self.scene.clone();
        let stack = self.stack_mut(id);
        let op = stack.next_id;
        stack.next_id += 1;
        stack.ops.insert(
            at,
            MeshOp {
                id: op,
                enabled: true,
                kind,
                selection,
                selops: Vec::new(),
            },
        );
        // A kit op's defaults could already name items (SAFE-07).
        self.refuse_loop(&before)?;
        self.meshop_row = Some(OpRow {
            item: id,
            op: Some(op),
        });
        self.record(label, before);
        Ok(())
    }

    /// `item.delete item:<op>` (MO-21).
    pub(crate) fn meshop_delete(&mut self, ops: &[OpRef]) -> Result<()> {
        self.require_idle()?;
        for &r in ops {
            self.op_index(r)?;
        }
        let before = self.scene.clone();
        for &r in ops {
            let stack = self.stack_mut(r.item);
            stack.ops.retain(|o| o.id != r.op);
            self.drop_empty_stack(r.item);
            if self.meshop_row
                == Some(OpRow {
                    item: r.item,
                    op: Some(r.op),
                })
            {
                self.meshop_row = Some(OpRow {
                    item: r.item,
                    op: None,
                });
            }
        }
        let label = if ops.len() == 1 {
            "Delete Mesh Operation"
        } else {
            "Delete Mesh Operations"
        };
        self.record(label.into(), before);
        Ok(())
    }

    /// `item.parent item:<op> position:<n>` (MO-21): moves an op to
    /// position `n` in evaluation order (0 runs first, right above the Base
    /// Mesh).
    pub(crate) fn meshop_move(&mut self, r: OpRef, position: usize) -> Result<()> {
        self.require_idle()?;
        let from = self.op_index(r)?;
        let before = self.scene.clone();
        let stack = self.stack_mut(r.item);
        let to = position.min(stack.ops.len() - 1);
        if to == from {
            return Ok(());
        }
        let op = stack.ops.remove(from);
        stack.ops.insert(to, op);
        self.record("Move Mesh Operation".into(), before);
        Ok(())
    }

    /// `deformer.enable deformer:<op> enable:<bool>` (MO-12 eye).
    pub(crate) fn meshop_enable(&mut self, r: OpRef, on: bool) -> Result<()> {
        self.require_idle()?;
        let i = self.op_index(r)?;
        let before = self.scene.clone();
        let stack = self.stack_mut(r.item);
        if stack.ops[i].enabled == on {
            return Ok(());
        }
        stack.ops[i].enabled = on;
        self.refuse_loop(&before)?;
        self.record(
            if on {
                "Enable Mesh Operation"
            } else {
                "Disable Mesh Operation"
            }
            .into(),
            before,
        );
        Ok(())
    }

    /// `item.channel <name> <value> item:<op>`: sets op values. A drag
    /// folds into one step through `undo_fold` (SAFE-13).
    pub(crate) fn meshop_set(
        &mut self,
        r: OpRef,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()> {
        self.require_idle()?;
        let i = self.op_index(r)?;
        let next = self
            .stack_item(r.item)?
            .mesh_stack
            .as_ref()
            .expect("checked")
            .ops[i]
            .kind
            .with_values(values)?;
        let before = self.scene.clone();
        let stack = self.stack_mut(r.item);
        if stack.ops[i].kind == next {
            return Ok(());
        }
        let label = format!("{} Values", next.label());
        stack.ops[i].kind = next;
        self.refuse_loop(&before)?;
        self.record(label, before);
        Ok(())
    }

    /// `select.item <op>`: the selected row (list state, not undone).
    pub(crate) fn meshop_select(&mut self, row: Option<OpRow>) -> Result<()> {
        if let Some(row) = row {
            let item = self.stack_item(row.item)?;
            if let Some(op) = row.op
                && item
                    .mesh_stack
                    .as_ref()
                    .and_then(|s| s.index_of(op))
                    .is_none()
            {
                return Err("That mesh operation doesn't exist".into());
            }
        }
        self.meshop_row = row;
        Ok(())
    }

    /// `selop.selection mode:<set|clear|add|remove> item:<op>` (MO-53):
    /// edits the op's Select by Index from the polygons selected on its
    /// mesh. Clear removes the Select by Index (the op then works on the
    /// whole mesh).
    pub(crate) fn meshop_index(&mut self, r: OpRef, edit: IndexEdit) -> Result<()> {
        self.require_idle()?;
        let i = self.op_index(r)?;
        let op = &self
            .stack_item(r.item)?
            .mesh_stack
            .as_ref()
            .expect("checked")
            .ops[i];
        if !op.kind.takes_selection() {
            return Err(format!("{} has no Selection input", op.kind.label()));
        }
        let element = op
            .kind
            .def()
            .map_or(SelElement::Polygon, |d| d.selection_element());
        let (picked, made_on) = self.picked_on(r.item, element).unwrap_or_default();
        if edit != IndexEdit::Clear && picked.is_empty() {
            return Err(format!(
                "Select {} on the Base Mesh first (Edit Base Mesh)",
                element.plural()
            ));
        }
        let mut polygons = op.selection.clone().unwrap_or_default().polygons;
        match edit {
            IndexEdit::Set => polygons = picked,
            IndexEdit::Clear => polygons.clear(),
            IndexEdit::Add => polygons.extend(picked),
            IndexEdit::Remove => polygons.retain(|f| !picked.contains(f)),
        }
        polygons.sort_unstable();
        polygons.dedup();
        let next = (!polygons.is_empty() || edit != IndexEdit::Clear).then_some(SelectByIndex {
            polygons,
            made_on: Some(made_on),
        });
        let before = self.scene.clone();
        let stack = self.stack_mut(r.item);
        if stack.ops[i].selection == next {
            return Ok(());
        }
        stack.ops[i].selection = next;
        self.record("Select by Index".into(), before);
        Ok(())
    }

    /// `selop.create` / `selop.edit` (MO-50..53): one undo step. A Select
    /// by Previous Operation must name an op lower in the same stack
    /// (PG-02).
    pub(crate) fn meshop_selop(&mut self, r: OpRef, edit: SelOpEdit) -> Result<()> {
        self.require_idle()?;
        let i = self.op_index(r)?;
        let stack = self
            .stack_item(r.item)?
            .mesh_stack
            .as_ref()
            .expect("checked");
        let op = &stack.ops[i];
        if !op.kind.takes_selection() {
            return Err(format!("{} has no Selection input", op.kind.label()));
        }
        let check = |s: &SelOp| -> Result<()> {
            if let SelKind::Previous { source, .. } = s.kind
                && source != 0
                && !stack.ops[..i].iter().any(|o| o.id == source)
            {
                return Err(
                    "Select by Previous Operation needs an operation lower in the same stack"
                        .into(),
                );
            }
            Ok(())
        };
        let mut selops = op.selops.clone();
        let label = match edit {
            SelOpEdit::Add(mut s) => {
                check(&s)?;
                // Select by Falloff takes a copy of the active falloff
                // when added, as the Falloff step does (TB-01).
                if let SelKind::Falloff { falloff, .. } = &mut s.kind
                    && falloff.is_none()
                {
                    *falloff = self.primitive_weights();
                }
                let label = format!("Add {}", s.kind.label());
                selops.push(s);
                label
            }
            SelOpEdit::Set(n, s) => {
                check(&s)?;
                *selops
                    .get_mut(n)
                    .ok_or("That selection operation doesn't exist")? = s;
                "Selection Operation Values".into()
            }
            SelOpEdit::Remove(n) => {
                if n >= selops.len() {
                    return Err("That selection operation doesn't exist".into());
                }
                selops.remove(n);
                "Remove Selection Operation".into()
            }
            SelOpEdit::Move(n, to) => {
                if n >= selops.len() || to >= selops.len() {
                    return Err("That selection operation doesn't exist".into());
                }
                let s = selops.remove(n);
                selops.insert(to, s);
                "Reorder Selection Operations".into()
            }
        };
        let before = self.scene.clone();
        let stack = self.stack_mut(r.item);
        if stack.ops[i].selops == selops {
            return Ok(());
        }
        stack.ops[i].selops = selops;
        self.record(label, before);
        Ok(())
    }

    /// `deformer.freeze` (MO-74..77, SAFE-14): bakes `r` and every op below
    /// it into a new Base Mesh; the ops above stay live. With `duplicate`,
    /// the frozen copy is a new mesh item and the original keeps its whole
    /// stack. One undo step. Refused unless every op frozen ran cleanly.
    pub(crate) fn meshop_freeze(&mut self, r: OpRef, duplicate: bool) -> Result<()> {
        self.require_idle()?;
        let i = self.op_index(r)?;
        let item = self.stack_item(r.item)?.clone();
        let stack = item.mesh_stack.as_ref().expect("checked");
        let below = MeshStack {
            ops: stack.ops[..=i].to_vec(),
            next_id: stack.next_id,
        };
        let probe = crate::Item {
            mesh_stack: Some(below),
            ..item.clone()
        };
        let evaluated = self
            .mesh_stack
            .current(&self.scene, &probe)
            .ok_or("Freeze couldn't evaluate the stack")?;
        if let Some((n, reason)) =
            evaluated
                .statuses
                .iter()
                .enumerate()
                .find_map(|(n, s)| match s {
                    OpStatus::Failed(reason) => Some((n, reason)),
                    _ => None,
                })
        {
            return Err(format!(
                "Can't freeze: {} failed ({reason}). Fix it, turn it off or delete it first",
                stack.ops[n].kind.label()
            ));
        }
        let above: Vec<MeshOp> = stack.ops[i + 1..].to_vec();
        let next_id = stack.next_id;
        let rest = (!above.is_empty()).then_some(MeshStack {
            ops: above,
            next_id,
        });
        let before = self.scene.clone();
        if duplicate {
            if self.scene.next_id >= u64::MAX - 1
                || self.scene.items.len() >= crate::MAX_SCENE_ITEMS
            {
                return Err("Scene item/identity budget exhausted".into());
            }
            let id = self.scene.next_id;
            self.scene.next_id += 1;
            let taken = |n: &str| self.scene.items.iter().any(|i| i.name == n);
            let name = crate::user_defaults::free_name(&item.name, taken);
            let copy = crate::Item {
                id,
                name,
                mesh: evaluated.mesh.clone().into(),
                mesh_stack: rest,
                elements: Default::default(),
                source_id: None,
                ..item.clone()
            };
            let at = self
                .scene
                .item_index(r.item)
                .map_or(self.scene.items.len(), |p| p + 1);
            self.scene.items.insert(at, copy);
            self.scene.selection = crate::Selection {
                item: Some(id),
                mode: self.scene.selection.mode,
                ..Default::default()
            };
            self.meshop_row = None;
            self.record("Duplicate and Freeze".into(), before);
        } else {
            let target = self.scene.item_mut(r.item).expect("checked");
            target.mesh = evaluated.mesh.clone().into();
            target.mesh_stack = rest;
            // The frozen mesh is new topology; lock and hide sets numbered
            // the old Base Mesh.
            target.elements = Default::default();
            if self.scene.selection.item == Some(r.item) {
                self.scene.selection.vertices.clear();
                self.scene.selection.edges.clear();
                self.scene.selection.faces.clear();
            }
            self.meshop_row = None;
            if self.base_edit == Some(r.item) && !self.scene_has_stack(r.item) {
                self.base_edit = None;
            }
            self.record("Freeze".into(), before);
        }
        Ok(())
    }

    /// `mesh.freeze`: freezes the selected mesh's whole stack.
    pub(crate) fn meshop_freeze_all(&mut self) -> Result<()> {
        let item = self
            .scene
            .active()
            .ok_or("Select a mesh with a Mesh Stack to freeze")?;
        let top = item
            .mesh_stack
            .as_ref()
            .and_then(|s| s.ops.last())
            .ok_or("This mesh has no Mesh Stack to freeze")?;
        let r = OpRef {
            item: item.id,
            op: top.id,
        };
        self.meshop_freeze(r, false)
    }

    fn scene_has_stack(&self, item: u64) -> bool {
        self.scene
            .items
            .iter()
            .any(|i| i.id == item && i.mesh_stack.is_some())
    }

    /// `deformer.selectBaseMesh item:<id> select:<bool>` (MO-27): direct
    /// tools edit this mesh's Base Mesh, drawn with its result ghosted.
    pub(crate) fn meshop_edit_base(&mut self, item: u64, on: bool) -> Result<()> {
        if on {
            if !self.scene_has_stack(item) {
                return Err("Only a mesh with a Mesh Stack has a Base Mesh to edit".into());
            }
            self.base_edit = Some(item);
            self.meshop_row = Some(OpRow { item, op: None });
            if self.scene.selection.item != Some(item) {
                self.scene.selection = crate::Selection {
                    item: Some(item),
                    mode: self.scene.selection.mode,
                    ..Default::default()
                };
            }
        } else if self.base_edit == Some(item) {
            self.base_edit = None;
        }
        Ok(())
    }

    /// A failed command on a mesh shown by its stack's result: the tool
    /// saw the Base Mesh, so say how to reach it (SAFE-05).
    pub(crate) fn explain_mesh_stack_error(&self, error: String, stack_command: bool) -> String {
        let procedural = self
            .scene
            .active()
            .is_some_and(|i| i.mesh_stack.is_some() && self.base_edit != Some(i.id));
        if stack_command || !procedural || error.contains(DIRECT_EDIT_REFUSED) {
            error
        } else {
            format!("{error}. {DIRECT_EDIT_REFUSED}")
        }
    }

    /// Run after every command but Undo/Redo (identity_editor.rs). SAFE-05:
    /// a command other than a stack edit may not change the mesh of an item
    /// with a stack, unless that item is in Edit Base Mesh. SAFE-06: items
    /// shown by their result keep no component selection. Stale list and
    /// Edit Base Mesh state is cleared.
    pub(crate) fn guard_mesh_stack(
        &mut self,
        initial: &crate::Scene,
        stack_command: bool,
    ) -> Result<()> {
        if !stack_command {
            for item in &self.scene.items {
                let Some(old) = initial.item(item.id) else {
                    continue;
                };
                let procedural = item.mesh_stack.is_some() || old.mesh_stack.is_some();
                if procedural
                    && self.base_edit != Some(item.id)
                    && !crate::Shared::same(old.mesh.shared(), item.mesh.shared())
                    && *old.mesh != *item.mesh
                    // Weight maps are the skin's input, not its geometry:
                    // painting them is allowed (SK-41, skin.rs).
                    && !crate::skin::weights_only(&old.mesh, &item.mesh)
                {
                    return Err(DIRECT_EDIT_REFUSED.into());
                }
            }
        }
        self.settle_mesh_stack();
        Ok(())
    }

    /// Clears list and Edit Base Mesh state that no longer fits the scene,
    /// and component selection of items shown by their result.
    pub(crate) fn settle_mesh_stack(&mut self) {
        if let Some(id) = self.base_edit
            && !self.scene_has_stack(id)
        {
            self.base_edit = None;
        }
        if let Some(row) = self.meshop_row {
            let fits = self.scene.items.iter().any(|i| {
                i.id == row.item
                    && i.is_mesh()
                    && row.op.is_none_or(|op| {
                        i.mesh_stack
                            .as_ref()
                            .is_some_and(|s| s.index_of(op).is_some())
                    })
            });
            if !fits {
                self.meshop_row = None;
            }
        }
        let shown_by_result: Vec<u64> = self
            .scene
            .items
            .iter()
            .filter(|i| i.mesh_stack.is_some() && Some(i.id) != self.base_edit)
            // A skin keeps every point: its points stay selectable for the
            // weight tools (SK-42).
            .filter(|i| !i.mesh_stack.as_ref().is_some_and(crate::skin::only_skin))
            .map(|i| i.id)
            .collect();
        for id in shown_by_result {
            let s = &mut self.scene.selection;
            if s.item == Some(id) {
                s.vertices.clear();
                s.edges.clear();
                s.faces.clear();
            }
            s.others.remove(&id);
        }
    }
}

/// Scripts (command_exec.rs): the Mesh Stack commands, and the item commands
/// the registry reuses for ops (`item.delete`, `item.parent`,
/// `item.channel`, `select.item` with `item:op:<item>:<op>`). `None` lets
/// the ordinary item command run.
pub(crate) fn script_command(
    e: &Editor,
    name: &str,
    a: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<crate::Command>> {
    use crate::Command;
    use crate::command_exec::{opt_bool, opt_str, opt_u64, req_str};
    let op_arg = |key: &str| -> Option<OpRef> { a.get(key)?.as_str().and_then(OpRef::parse) };
    let selected_op = || -> Result<OpRef> {
        match e.meshop_row {
            Some(OpRow { item, op: Some(op) }) => Ok(OpRef { item, op }),
            _ => Err("Select a mesh operation first".into()),
        }
    };
    let op_or_selected = |key: &str| -> Result<OpRef> {
        match a.get(key) {
            None | Some(serde_json::Value::Null) => selected_op(),
            Some(v) => v
                .as_str()
                .and_then(OpRef::parse)
                .ok_or_else(|| format!("{key} must be an op, written op:<item>:<op>")),
        }
    };
    Ok(Some(match name {
        "meshop.create" => {
            // mode, category and applyDefaultPreset are accepted, not
            // used yet (the preset browser isn't merged).
            Command::MeshOpCreate(req_str(a, "type")?.into())
        }
        "item.delete" => match op_arg("item") {
            Some(r) => Command::MeshOpDelete(vec![r]),
            None => return Ok(None),
        },
        "item.parent" => match op_arg("item") {
            Some(op) => Command::MeshOpMove {
                op,
                position: opt_u64(a, "position")?
                    .ok_or("position is required to move a mesh operation")?
                    as usize,
            },
            None => return Ok(None),
        },
        "select.item" => match a.get("item").and_then(|v| v.as_str()) {
            Some(text) => {
                // op:<item>:base selects the Base Mesh row.
                if let Some(item) = text
                    .strip_prefix("op:")
                    .and_then(|r| r.strip_suffix(":base"))
                    .and_then(|i| i.parse().ok())
                {
                    Command::MeshOpSelect(Some(OpRow { item, op: None }))
                } else {
                    let r =
                        OpRef::parse(text).ok_or("item must be an item id or op:<item>:<op>")?;
                    Command::MeshOpSelect(Some(OpRow {
                        item: r.item,
                        op: Some(r.op),
                    }))
                }
            }
            None => return Ok(None),
        },
        "item.channel" => match op_arg("item") {
            Some(op) => {
                let key = req_str(a, "name")?;
                let value = a
                    .get("value")
                    .filter(|v| !v.is_null())
                    .ok_or("value is required")?;
                // The command line gives numbers and booleans as text.
                let value = match value {
                    serde_json::Value::String(t) => serde_json::from_str(t)
                        .unwrap_or_else(|_| serde_json::Value::String(t.clone())),
                    v => v.clone(),
                };
                let mut values = serde_json::Map::new();
                values.insert(key.into(), value);
                Command::MeshOpSet { op, values }
            }
            None => return Ok(None),
        },
        "deformer.enable" => Command::MeshOpEnable {
            op: op_or_selected("deformer")?,
            on: opt_bool(a, "enable")?.unwrap_or(true),
        },
        "deformer.freeze" => Command::MeshOpFreeze {
            op: op_or_selected("deformer")?,
            duplicate: opt_bool(a, "duplicate")?.unwrap_or(false),
        },
        "deformer.rebind" => {
            let op = op_or_selected("deformer")?;
            let stack = e
                .scene
                .item(op.item)
                .and_then(|it| it.mesh_stack.as_ref())
                .ok_or("The item has no Mesh Stack")?;
            let index = stack.index_of(op.op).ok_or("The mesh operation is gone")?;
            if stack.ops[index].kind.type_id() != crate::meshop_wrap::WRAP_ID {
                return Err("Only a Wrap mesh operation can be rebound".into());
            }
            let mut values = serde_json::Map::new();
            values.insert(
                "binding".into(),
                crate::meshop_wrap::bind(&e.scene, op.item, op.op)?,
            );
            Command::MeshOpSet { op, values }
        }
        // Registry rev 3: the menu's Mirror and Array (a Clone Effector
        // with its generator).
        "mirror.clone.create" => Command::MeshOpCreate("mirror.clone.item".into()),
        "array.clone.create" => Command::MeshOpCreate("array.clone.item".into()),
        "mesh.freeze" => Command::MeshOpFreezeAll,
        "deformer.selectBaseMesh" => Command::MeshOpEditBase {
            item: match opt_u64(a, "item")? {
                Some(id) => id,
                None => e.scene.selection.item.ok_or("No active item")?,
            },
            on: opt_bool(a, "select")?.unwrap_or(true),
        },
        "selop.indexcreate" => Command::MeshOpIndex {
            op: selected_op()?,
            edit: IndexEdit::Set,
        },
        "selop.selection" => {
            let mode = match a.get("mode") {
                None | Some(serde_json::Value::Null) => "set".to_string(),
                Some(v) => v.as_str().map_or_else(|| v.to_string(), str::to_owned),
            };
            Command::MeshOpIndex {
                op: op_or_selected("item")?,
                edit: IndexEdit::parse(&mode)
                    .ok_or("mode must be set, clear, add or remove (0 to 3)")?,
            }
        }
        "selop.create" => {
            let type_id = req_str(a, "type")?;
            let kind = SelKind::menu()
                .into_iter()
                .map(|(_, k)| k)
                .find(|k| {
                    serde_json::to_value(k)
                        .ok()
                        .and_then(|v| v["type"].as_str().map(str::to_owned))
                        .as_deref()
                        == Some(type_id)
                })
                .ok_or_else(|| format!("“{type_id}” isn't a selection operation"))?;
            Command::MeshOpSelOp {
                op: op_or_selected("item")?,
                edit: SelOpEdit::Add(SelOp::new(kind)),
            }
        }
        "selop.edit" => {
            let op = op_or_selected("item")?;
            let n = opt_u64(a, "index")?.ok_or("index is required")? as usize;
            if opt_bool(a, "remove")?.unwrap_or(false) {
                Command::MeshOpSelOp {
                    op,
                    edit: SelOpEdit::Remove(n),
                }
            } else if let Some(to) = opt_u64(a, "position")? {
                // #19: `selop.edit index:<n> position:<m>` reorders.
                Command::MeshOpSelOp {
                    op,
                    edit: SelOpEdit::Move(n, to as usize),
                }
            } else {
                let item = e
                    .scene
                    .item(op.item)
                    .ok_or("That mesh item doesn't exist")?;
                let current = item
                    .mesh_stack
                    .as_ref()
                    .and_then(|s| s.index_of(op.op).map(|i| &s.ops[i]))
                    .and_then(|o| o.selops.get(n))
                    .ok_or("That selection operation doesn't exist")?;
                let mut v = serde_json::to_value(current).map_err(|e| e.to_string())?;
                let changes = a
                    .get("values")
                    .and_then(|v| v.as_object())
                    .ok_or("values must be an object of selection-op values")?;
                for (k, x) in changes {
                    if k == "type" || v.get(k).is_none() {
                        return Err(format!("The selection operation has no value “{k}”"));
                    }
                    v[k] = x.clone();
                }
                let next: SelOp = serde_json::from_value(v).map_err(|e| e.to_string())?;
                Command::MeshOpSelOp {
                    op,
                    edit: SelOpEdit::Set(n, next),
                }
            }
        }
        _ => {
            let _ = opt_str;
            return Ok(None);
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;

    fn editor_with_mesh() -> (Editor, u64) {
        let mut e = Editor::default();
        e.scene.new_mesh().unwrap();
        let id = e.scene.items[0].id;
        (e, id)
    }
    fn ops(e: &Editor, id: u64) -> Vec<String> {
        e.scene
            .item(id)
            .and_then(|i| i.mesh_stack.as_ref())
            .map_or_else(Vec::new, |s| {
                s.ops.iter().map(|o| o.kind.type_id().to_string()).collect()
            })
    }
    fn result_faces(e: &Editor, id: u64) -> usize {
        let item = e.scene.item(id).unwrap();
        e.mesh_stack
            .current(&e.scene, item)
            .unwrap()
            .mesh
            .faces
            .len()
    }
    fn create(e: &mut Editor, t: &str) {
        e.dispatch(Command::MeshOpCreate(t.into())).unwrap();
    }
    fn top(e: &Editor, id: u64) -> OpRef {
        let item = e.scene.item(id).unwrap();
        OpRef {
            item: id,
            op: item.mesh_stack.as_ref().unwrap().ops.last().unwrap().id,
        }
    }

    #[test]
    fn add_lands_above_the_selected_row_and_undo_restores_the_stack() {
        let (mut e, id) = editor_with_mesh();
        create(&mut e, "prim.cube.item");
        create(&mut e, "pmodel.triangulate.item");
        assert_eq!(ops(&e, id), ["prim.cube.item", "pmodel.triangulate.item"]);
        // Base Mesh row selected: the new op goes right above it (MO-18).
        e.dispatch(Command::MeshOpSelect(Some(OpRow { item: id, op: None })))
            .unwrap();
        create(&mut e, "prim.sphere.item");
        assert_eq!(
            ops(&e, id),
            [
                "prim.sphere.item",
                "prim.cube.item",
                "pmodel.triangulate.item"
            ]
        );
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(ops(&e, id), ["prim.cube.item", "pmodel.triangulate.item"]);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(
            ops(&e, id),
            [
                "prim.sphere.item",
                "prim.cube.item",
                "pmodel.triangulate.item"
            ]
        );
        // The Base Mesh is untouched by the ops (MO-01).
        assert!(e.scene.items[0].mesh.faces.is_empty());
    }

    #[test]
    fn no_mesh_selected_is_refused() {
        let mut e = Editor::default();
        assert!(
            e.dispatch(Command::MeshOpCreate("prim.cube.item".into()))
                .is_err()
        );
        let (mut e, _) = editor_with_mesh();
        assert!(e.dispatch(Command::MeshOpCreate("no.such".into())).is_err());
        assert!(e.scene.items[0].mesh_stack.is_none());
    }

    #[test]
    fn direct_tools_refuse_on_a_stack_unless_editing_the_base_mesh() {
        let (mut e, id) = editor_with_mesh();
        create(&mut e, "prim.cube.item");
        let before = e.scene.clone();
        let err = e
            .dispatch(Command::SubdividePolygons {
                options: Default::default(),
                times: 1,
            })
            .unwrap_err();
        assert!(err.contains(DIRECT_EDIT_REFUSED), "{err}");
        assert_eq!(e.scene, before, "nothing changed");
        // Edit Base Mesh: tools edit the Base Mesh (MO-27).
        e.dispatch(Command::MeshOpEditBase { item: id, on: true })
            .unwrap();
        e.scene.items[0].mesh = crate::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
            .unwrap()
            .into();
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        e.dispatch(Command::SubdividePolygons {
            options: Default::default(),
            times: 1,
        })
        .unwrap();
        assert_eq!(e.scene.items[0].mesh.faces.len(), 24);
        // The stack runs on the edited Base Mesh: 24 + the Cube's 6.
        assert_eq!(result_faces(&e, id), 30);
    }

    #[test]
    fn freeze_bakes_below_keeps_above_and_undoes_in_one_step() {
        let (mut e, id) = editor_with_mesh();
        create(&mut e, "prim.cube.item");
        let cube = top(&e, id);
        create(&mut e, "pmodel.triangulate.item");
        let full = result_faces(&e, id);
        let before = e.scene.clone();
        e.dispatch(Command::MeshOpFreeze {
            op: cube,
            duplicate: false,
        })
        .unwrap();
        assert_eq!(
            e.scene.items[0].mesh.faces.len(),
            6,
            "the cube is the Base Mesh"
        );
        assert_eq!(ops(&e, id), ["pmodel.triangulate.item"]);
        assert_eq!(result_faces(&e, id), full, "same result");
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before);
        // Freezing the top op leaves a plain mesh.
        e.dispatch(Command::MeshOpFreeze {
            op: top(&e, id),
            duplicate: false,
        })
        .unwrap();
        assert!(e.scene.items[0].mesh_stack.is_none());
        assert_eq!(e.scene.items[0].mesh.faces.len(), full);
    }

    #[test]
    fn duplicate_and_freeze_keeps_the_original_stack() {
        let (mut e, id) = editor_with_mesh();
        create(&mut e, "prim.cube.item");
        let r = top(&e, id);
        e.dispatch(Command::MeshOpFreeze {
            op: r,
            duplicate: true,
        })
        .unwrap();
        assert_eq!(e.scene.items.len(), 2);
        assert_eq!(ops(&e, id), ["prim.cube.item"]);
        let copy = &e.scene.items[1];
        assert!(copy.mesh_stack.is_none());
        assert_eq!(copy.mesh.faces.len(), 6);
        assert_eq!(e.scene.selection.item, Some(copy.id));
    }

    #[test]
    fn freeze_refuses_a_failed_op() {
        let (mut e, id) = editor_with_mesh();
        create(&mut e, "prim.cube.item");
        let r = top(&e, id);
        let mut bad = serde_json::Map::new();
        bad.insert("sizeX".into(), serde_json::json!(-0.0));
        bad.insert("sizeY".into(), serde_json::json!(0.0));
        bad.insert("sizeZ".into(), serde_json::json!(0.0));
        e.dispatch(Command::MeshOpSet { op: r, values: bad })
            .unwrap();
        // Three zero axes: the Cube kernel makes nothing, which is fine.
        assert!(
            e.dispatch(Command::MeshOpFreeze {
                op: r,
                duplicate: false
            })
            .is_ok()
        );
        let (mut e, id) = editor_with_mesh();
        create(&mut e, "prim.sphere.item");
        let r = top(&e, id);
        let mut bad = serde_json::Map::new();
        bad.insert("sides".into(), serde_json::json!(1));
        e.dispatch(Command::MeshOpSet { op: r, values: bad })
            .unwrap();
        let before = e.scene.clone();
        assert!(
            e.dispatch(Command::MeshOpFreeze {
                op: r,
                duplicate: false
            })
            .is_err()
        );
        assert_eq!(e.scene, before);
    }

    #[test]
    fn set_refuses_unknown_names_and_wrong_types() {
        let (mut e, id) = editor_with_mesh();
        create(&mut e, "poly.bevel.item");
        let r = top(&e, id);
        let mut v = serde_json::Map::new();
        v.insert("nope".into(), serde_json::json!(1));
        assert!(e.dispatch(Command::MeshOpSet { op: r, values: v }).is_err());
        let mut v = serde_json::Map::new();
        v.insert("shift".into(), serde_json::json!("big"));
        assert!(e.dispatch(Command::MeshOpSet { op: r, values: v }).is_err());
    }

    #[test]
    fn auto_capture_takes_the_base_selection() {
        let (mut e, id) = editor_with_mesh();
        e.scene.items[0].mesh = crate::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
            .unwrap()
            .into();
        e.scene.selection.mode = Mode::Polygon;
        e.scene.selection.faces = vec![3, 1];
        create(&mut e, "pmodel.triangulate.item");
        let item = &e.scene.items[0];
        let op = &item.mesh_stack.as_ref().unwrap().ops[0];
        assert_eq!(op.selection.as_ref().unwrap().polygons, vec![1, 3]);
        assert_eq!(result_faces(&e, id), 8);
        // The mesh now shows its result: no component selection (SAFE-06).
        assert!(e.scene.selection.faces.is_empty());
    }

    #[test]
    fn clone_create_commands_and_channels_reach_effector_and_generator() {
        let (mut e, id) = editor_with_mesh();
        let none = serde_json::Map::new();
        create(&mut e, "prim.cube.item");
        crate::command_exec::run(&mut e, "mirror.clone.create", &none).unwrap();
        crate::command_exec::run(&mut e, "array.clone.create", &none).unwrap();
        assert_eq!(
            ops(&e, id),
            ["prim.cube.item", "mirror.clone.item", "array.clone.item"]
        );
        let mirror = OpRef {
            item: id,
            op: e.scene.items[0].mesh_stack.as_ref().unwrap().ops[1].id,
        };
        for (name, value) in [("axis", "1"), ("dist", "0.01")] {
            let mut a = serde_json::Map::new();
            a.insert("name".into(), name.into());
            a.insert("value".into(), value.into());
            a.insert("item".into(), mirror.text().into());
            crate::command_exec::run(&mut e, "item.channel", &a).unwrap();
        }
        let op = &e.scene.items[0].mesh_stack.as_ref().unwrap().ops[1];
        assert_eq!(op.kind.type_id(), "mirror.clone.item");
        assert_eq!(op.kind.values()["axis"], 1);
        assert_eq!(op.kind.values()["dist"], 0.01);
    }

    #[test]
    fn delete_move_enable() {
        let (mut e, id) = editor_with_mesh();
        create(&mut e, "prim.cube.item");
        let cube = top(&e, id);
        create(&mut e, "pmodel.triangulate.item");
        let tri = top(&e, id);
        e.dispatch(Command::MeshOpEnable { op: tri, on: false })
            .unwrap();
        assert_eq!(result_faces(&e, id), 6);
        e.dispatch(Command::MeshOpMove {
            op: tri,
            position: 0,
        })
        .unwrap();
        assert_eq!(ops(&e, id), ["pmodel.triangulate.item", "prim.cube.item"]);
        e.dispatch(Command::MeshOpDelete(vec![tri, cube])).unwrap();
        assert!(e.scene.items[0].mesh_stack.is_none());
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(ops(&e, id), ["pmodel.triangulate.item", "prim.cube.item"]);
    }

    #[test]
    fn save_and_reopen_gives_the_same_result() {
        let (mut e, id) = editor_with_mesh();
        create(&mut e, "prim.cylinder.item");
        create(&mut e, "poly.bevel.item");
        create(&mut e, "mirror.clone.item");
        let bytes = crate::persistence::encode(&e.scene).unwrap();
        let back = crate::persistence::decode(&bytes).unwrap();
        assert_eq!(back, e.scene);
        let a = e.mesh_stack.current(&e.scene, &e.scene.items[0]).unwrap();
        let fresh = crate::mesh_stack::Runtime::default();
        let b = fresh.current(&back, back.item(id).unwrap()).unwrap();
        assert_eq!(*a.mesh, *b.mesh);
        // Mirror is a Clone Effector with a generator: native version 40.
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(doc["version"], 40);
        // The result is not in the file (SAFE-03): only the empty Base Mesh.
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"mesh_ops\""));
        assert!(text.contains("\"prim.cylinder.item\""));
    }
}
