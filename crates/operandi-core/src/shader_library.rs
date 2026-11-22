//! The shader tree's Library and references (shader layer menu v0.1 §4).
//!
//! The Library holds material definitions: Material Groups with their
//! contents, kept in the same flat `ShaderTree::nodes` list as top-level
//! groups flagged `extras.library`. They don't render by themselves. A
//! reference is a single row in the tree (a Folder flagged
//! `extras.reference`) that shades as its Library group's contents: at
//! render time [`ShaderTree::resolved`] puts a copy of the group in the
//! reference's place and leaves the Library out, so editing the Library
//! group updates every reference.
use crate::shader_tree::{Drop, Node, NodeId, NodeKind, ShaderTree};
use crate::{Result, Scene};
use std::borrow::Cow;

impl ShaderTree {
    /// The Library item (top-level Library group) `id` is in or is.
    pub fn library_root(&self, id: NodeId) -> Option<NodeId> {
        let mut at = Some(id);
        for _ in 0..=self.nodes.len() {
            let node = at.and_then(|a| self.get(a))?;
            if node.parent.is_none() {
                return node.extras.library.then_some(node.id);
            }
            at = node.parent;
        }
        None
    }
    pub fn in_library(&self, id: NodeId) -> bool {
        self.library_root(id).is_some()
    }
    /// The Library items, in order.
    pub fn library(&self) -> impl Iterator<Item = &Node> {
        self.children(None).filter(|n| n.extras.library)
    }
    /// The Library item reference `id` shades as.
    pub fn reference_target(&self, id: NodeId) -> Option<NodeId> {
        self.get(id)?.extras.reference
    }
    /// Every reference to Library item `library`, in panel order.
    pub fn references_to(&self, library: NodeId) -> Vec<NodeId> {
        self.rows(&|_| false)
            .into_iter()
            .filter(|(n, _)| n.extras.reference == Some(library))
            .map(|(n, _)| n.id)
            .collect()
    }
    /// Whether settings of `id` are locked: it is (in) a locked Library
    /// item.
    pub fn is_locked(&self, id: NodeId) -> bool {
        self.library_root(id)
            .and_then(|r| self.get(r))
            .is_some_and(|r| r.extras.locked)
    }

    /// Checks for the Library and references (part of `validate`).
    pub(crate) fn validate_library(&self) -> Result<()> {
        for n in &self.nodes {
            if n.extras.library && (n.parent.is_some() || !matches!(n.kind, NodeKind::Group(_))) {
                return Err(format!(
                    "{}: only a top-level Material Group can be a Library item",
                    n.name
                ));
            }
            if n.extras.locked && !n.extras.library {
                return Err(format!("{}: only a Library item can be locked", n.name));
            }
            if let Some(target) = n.extras.reference {
                if !matches!(n.kind, NodeKind::Folder) || self.has_children(n.id) {
                    return Err(format!("{}: a reference holds no layers", n.name));
                }
                if !self.get(target).is_some_and(|t| t.extras.library) {
                    return Err(format!("{}: its Library item is gone", n.name));
                }
                if self.in_library(n.id) {
                    return Err(format!(
                        "{}: a reference can't sit inside the Library",
                        n.name
                    ));
                }
                if n.extras.instance.is_some() {
                    return Err(format!("{}: a reference can't be an instance", n.name));
                }
            }
        }
        Ok(())
    }

    /// The tree as it renders (§4): the Library left out and each
    /// reference replaced by a copy of its Library group (fresh ids, the
    /// reference's eye). Borrowed when there is neither.
    pub fn resolved(&self) -> Cow<'_, ShaderTree> {
        if !self
            .nodes
            .iter()
            .any(|n| n.extras.library || n.extras.reference.is_some())
        {
            return Cow::Borrowed(self);
        }
        let mut out = ShaderTree {
            nodes: Vec::with_capacity(self.nodes.len()),
            next_id: self.next_id,
            outputs: self.outputs.clone(),
            node_graphs: self.node_graphs.clone(),
            node_views: self.node_views.clone(),
        };
        let mut next = self.next_id;
        for n in &self.nodes {
            if self.in_library(n.id) {
                continue;
            }
            let Some(target) = n.extras.reference else {
                out.nodes.push(n.clone());
                continue;
            };
            // A copy of the Library group's branch, its root in the
            // reference's place.
            let branch = self.branch(target);
            let ids: std::collections::BTreeMap<NodeId, NodeId> = branch
                .iter()
                .map(|b| {
                    let id = next;
                    next += 1;
                    (b.id, id)
                })
                .collect();
            for b in branch {
                let mut copy = b.clone();
                copy.id = ids[&b.id];
                copy.extras = Default::default();
                if b.id == target {
                    copy.parent = n.parent;
                    copy.layer.enable &= n.layer.enable;
                    copy.name = n.name.clone();
                } else {
                    copy.parent = b.parent.and_then(|p| ids.get(&p).copied());
                }
                if let Some(o) = self.outputs.get(&b.id) {
                    out.outputs.insert(copy.id, o.clone());
                }
                out.nodes.push(copy);
            }
        }
        out.outputs
            .retain(|id, _| out.nodes.iter().any(|n| n.id == *id));
        out.next_id = next;
        Cow::Owned(out)
    }

    /// Drag onto the Library heading (§4): each Material Group of `ids`
    /// (with its contents) becomes a Library item. Its mask stays with it.
    pub(crate) fn move_to_library(&mut self, ids: &[NodeId]) -> Result<()> {
        let roots = self.top_most(ids);
        if roots.is_empty() {
            return Err("Drag a Material Group onto the Library".into());
        }
        for &id in &roots {
            let n = self.get(id).ok_or("Shader tree layer no longer exists")?;
            if !matches!(n.kind, NodeKind::Group(_)) {
                return Err("Only Material Groups go into the Library".into());
            }
            if self.in_library(id) {
                return Err(format!("{} is already in the Library", n.name));
            }
            if n.extras.reference.is_some() || n.extras.instance.is_some() {
                return Err(format!("{} can't go into the Library", n.name));
            }
            if self
                .nodes
                .iter()
                .any(|x| self.is_within(x.id, id) && x.id != id && x.extras.reference.is_some())
            {
                return Err(format!(
                    "{} holds a reference: De-Reference it first",
                    n.name
                ));
            }
        }
        let end = self.children(None).count();
        self.move_to(
            &roots,
            Drop {
                parent: None,
                index: end,
            },
        )?;
        for id in roots {
            let i = self.index(id).expect("moved");
            self.nodes[i].extras.library = true;
        }
        Ok(())
    }

    /// Drag a Library item into the tree (§4): a reference to `library`
    /// at `drop`, named after it.
    pub(crate) fn add_reference(&mut self, library: NodeId, drop: Drop) -> Result<NodeId> {
        let lib = self
            .get(library)
            .ok_or("That Library item is gone")?
            .clone();
        if !lib.extras.library {
            return Err("Only a Library item can be referenced".into());
        }
        if drop.parent.is_some_and(|p| self.in_library(p)) {
            return Err("A reference can't go inside the Library".into());
        }
        if let Some(p) = drop.parent
            && !self.get(p).is_some_and(|n| n.is_container())
        {
            return Err("Drop a reference into a group or between layers".into());
        }
        if self.nodes.len() >= 65_536 || self.next_id >= u64::MAX - 1 {
            return Err("Shader tree is full".into());
        }
        let id = self.next_id;
        self.next_id += 1;
        let node = Node {
            id,
            parent: drop.parent,
            name: self.unique_name(&lib.name),
            layer: Default::default(),
            kind: NodeKind::Folder,
            extras: crate::shader_layer_menu::LayerExtras {
                reference: Some(library),
                ..Default::default()
            },
        };
        let at = self.slot(drop);
        self.nodes.insert(at, node);
        Ok(id)
    }

    /// Lock Preset / Unlock Preset (§4) on Library item `id`.
    pub(crate) fn set_locked(&mut self, id: NodeId, locked: bool) -> Result<()> {
        let i = self.index(id).ok_or("Shader tree layer no longer exists")?;
        if !self.nodes[i].extras.library {
            return Err("Only a Library item can be locked".into());
        }
        self.nodes[i].extras.locked = locked;
        Ok(())
    }

    /// Refuse an edit that changed a locked Library item (§4): its
    /// layers, their settings, names or order. Deleting the whole item and
    /// changing row colours are allowed.
    pub(crate) fn check_locks(&self, before: &ShaderTree) -> Result<()> {
        for root in before.library().filter(|r| r.extras.locked) {
            if self.get(root.id).is_none() {
                continue;
            }
            let strip = |t: &ShaderTree| -> Vec<Node> {
                t.branch(root.id)
                    .into_iter()
                    .map(|n| {
                        let mut n = n.clone();
                        n.layer.editor_color = None;
                        n
                    })
                    .collect()
            };
            if strip(self) != strip(before) {
                return Err(format!("{} is locked: Unlock Preset to edit it", root.name));
            }
        }
        Ok(())
    }

    /// References removed with `ids`: a Library item going takes its
    /// references with it.
    pub(crate) fn with_references(&self, ids: &[NodeId]) -> Vec<NodeId> {
        let mut out = ids.to_vec();
        for n in &self.nodes {
            if let Some(t) = n.extras.reference
                && ids.iter().any(|&id| self.is_within(t, id))
                && !out.contains(&n.id)
            {
                out.push(n.id);
            }
        }
        out
    }

    /// The references deleting `ids` would also delete, for the question.
    pub fn references_deleted_with(&self, ids: &[NodeId]) -> Vec<NodeId> {
        self.with_references(ids)
            .into_iter()
            .filter(|&n| !ids.iter().any(|&id| self.is_within(n, id)))
            .collect()
    }
}

impl Scene {
    /// The shader tree as renders and the viewport use it: older files'
    /// materials mapped in, the Library left out and references resolved.
    pub fn shading_tree(&self) -> Cow<'_, ShaderTree> {
        match self.render_shader_tree() {
            Cow::Borrowed(t) => t.resolved(),
            Cow::Owned(t) => Cow::Owned(t.resolved().into_owned()),
        }
    }

    /// References ▸ De-Reference (§4): each reference of `ids` becomes a
    /// real copy of its Library group at that spot, no longer linked; the
    /// copied texture layers get their own locators.
    pub(crate) fn dereference(&mut self, next: &mut u64, ids: &[NodeId]) -> Result<Vec<NodeId>> {
        let refs: Vec<NodeId> = ids
            .iter()
            .copied()
            .filter(|&id| self.shader_tree.reference_target(id).is_some())
            .collect();
        if refs.is_empty() {
            return Err("Select a reference to De-Reference".into());
        }
        let mut out = Vec::new();
        for r in refs {
            let node = self.shader_tree.get(r).expect("listed").clone();
            let target = node.extras.reference.expect("listed");
            let mut clip = self.copy_shader_layers(&[target]);
            for n in &mut clip.nodes {
                n.extras.library = false;
                n.extras.locked = false;
            }
            let index = self
                .shader_tree
                .children(node.parent)
                .position(|n| n.id == r)
                .expect("a child of its parent");
            let roots = self.paste_shader_copies(next, &clip, node.parent, index)?;
            self.shader_tree.remove(&[r])?;
            // The copy keeps the reference's name and eye.
            for &root in &roots {
                let i = self.shader_tree.index(root).expect("pasted");
                self.shader_tree.nodes[i].name = node.name.clone();
                self.shader_tree.nodes[i].layer.enable &= node.layer.enable;
            }
            out.extend(roots);
        }
        Ok(out)
    }
}
