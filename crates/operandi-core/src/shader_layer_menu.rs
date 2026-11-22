//! The shader layer right-click menu (specifications: shader layer menu
//! v0.1, 2026-11-09): Duplicate, Copy / Paste / Paste Values and Select
//! Polygons (Editor Color is `ShaderTreeEdit::SetEditorColor`). The Shading panel draws the menu; every
//! entry that changes the scene is a `ShaderTreeEdit` so undo, save and
//! scripts see the same operations.
use crate::shader_tree::{NewLayer, Node, NodeId, NodeKind, ShaderTree, TextureLayer};
use crate::{Result, Scene};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What the menu stores on a layer besides its settings (the editor
/// colour lives on the Layer block, editor_color.rs). Saved only when
/// something is set (native version 32).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerExtras {
    /// This layer is an instance of another (shader_instances.rs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<crate::shader_instances::InstanceLink>,
    /// A top-level Material Group kept in the Library (shader_library.rs).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub library: bool,
    /// Lock Preset on a Library item: its settings can't be edited.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    /// A reference: this row shades as that Library item's contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<NodeId>,
    /// The environment preset this material group came in with (EP v0.2):
    /// Replace removes it, even renamed. Native version 42.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_preset: Option<Box<String>>,
}
impl LayerExtras {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Operandi's own shader clipboard (§5): copied layers with everything
/// inside them, their Render Output settings and their Texture Locators.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShaderClipboard {
    /// Panel order. A copied root has `parent: None`; a layer inside one
    /// keeps its parent's id.
    pub nodes: Vec<Node>,
    pub outputs: BTreeMap<NodeId, crate::render_outputs::OutputSettings>,
    /// Locator item id → its settings and transform when copied.
    pub locators: BTreeMap<
        u64,
        (
            Box<crate::texture_locator::TextureLocator>,
            crate::scene::Transform,
        ),
    >,
    /// The node graphs copied Node Materials use, by their id when copied.
    pub graphs: Vec<crate::node_graph::StoredGraph>,
}
impl ShaderClipboard {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    /// The copied roots, for Paste Values.
    pub fn roots(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(|n| n.parent.is_none())
    }
}

/// `name` without a trailing index in the user's Item Index Style
/// ("Layer (2)" → "Layer").
pub fn unnumbered(name: &str) -> &str {
    let style = crate::user_defaults::index_style();
    use crate::user_defaults::IndexStyle::*;
    let digits_end = |s: &str| -> Option<usize> {
        let trimmed = s.trim_end_matches(|c: char| c.is_ascii_digit());
        let n: u64 = s[trimmed.len()..].parse().ok()?;
        (n >= 2).then_some(trimmed.len())
    };
    let stripped = match style {
        SpaceParentheses | Parentheses => name.strip_suffix(')').and_then(|s| {
            let at = digits_end(s)?;
            let open = s[..at].strip_suffix('(')?;
            if style == SpaceParentheses {
                open.strip_suffix(' ')
            } else {
                Some(open)
            }
        }),
        Space => digits_end(name).and_then(|at| name[..at].strip_suffix(' ')),
        Underscore => digits_end(name).and_then(|at| name[..at].strip_suffix('_')),
        Joined => digits_end(name).map(|at| &name[..at]),
    };
    match stripped {
        Some(base) if !base.trim().is_empty() => base,
        _ => name,
    }
}

impl ShaderTree {
    /// The name a copy of `name` takes (§2): its unnumbered base plus the
    /// next free index, always numbered ("Layer" → "Layer (2)", then
    /// "Layer (3)"). `taken` holds names claimed earlier in the same edit.
    pub(crate) fn copy_name(&self, name: &str, taken: &[String]) -> String {
        let base = unnumbered(name);
        let used = |n: &str| self.nodes.iter().any(|x| x.name == n) || taken.iter().any(|t| t == n);
        let style = crate::user_defaults::index_style();
        (2..)
            .map(|n| style.apply(base, n))
            .find(|n| !used(n))
            .unwrap_or_else(|| base.into())
    }

    /// The top-most of `ids` (a row inside another listed row goes with
    /// it), in panel order.
    pub(crate) fn top_most(&self, ids: &[NodeId]) -> Vec<NodeId> {
        self.rows(&|_| false)
            .into_iter()
            .map(|(n, _)| n.id)
            .filter(|&id| ids.contains(&id))
            .filter(|&id| {
                !ids.iter()
                    .any(|&other| other != id && self.is_within(id, other))
            })
            .collect()
    }

    /// The layers with ids from `first` on whose parent is older: what
    /// one Duplicate or Paste added, in panel order.
    pub fn top_level_since(&self, first: NodeId) -> Vec<NodeId> {
        self.rows(&|_| false)
            .into_iter()
            .map(|(n, _)| n)
            .filter(|n| n.id >= first && n.parent.is_none_or(|p| p < first))
            .map(|n| n.id)
            .collect()
    }

    /// `root` and everything inside it, in panel order.
    pub(crate) fn branch(&self, root: NodeId) -> Vec<&Node> {
        self.rows(&|_| false)
            .into_iter()
            .map(|(n, _)| n)
            .filter(|n| self.is_within(n.id, root))
            .collect()
    }

    /// Insert copies of `nodes` (a clipboard's: roots have `parent: None`)
    /// so each root becomes child `index` of `parent`, with fresh ids and
    /// copy names. Returns old id → new id.
    pub(crate) fn insert_copies(
        &mut self,
        nodes: &[Node],
        parent: Option<NodeId>,
        index: usize,
    ) -> Result<BTreeMap<NodeId, NodeId>> {
        if self.nodes.len() + nodes.len() > 65_536 || self.next_id >= u64::MAX - nodes.len() as u64
        {
            return Err("Shader tree is full".into());
        }
        if let Some(p) = parent {
            let holder = self.get(p).ok_or("Paste target no longer exists")?;
            if let Some(root) = nodes
                .iter()
                .find(|n| n.parent.is_none() && !holder.can_hold(n))
            {
                return Err(format!("{} can't hold {}", holder.name, root.name));
            }
        }
        let mut ids = BTreeMap::new();
        let mut taken = Vec::new();
        for n in nodes {
            ids.insert(n.id, self.next_id);
            self.next_id += 1;
        }
        let mut roots = Vec::new();
        let mut inside = Vec::new();
        for n in nodes {
            let mut copy = n.clone();
            copy.id = ids[&n.id];
            copy.name = self.copy_name(&n.name, &taken);
            taken.push(copy.name.clone());
            match n.parent.and_then(|p| ids.get(&p)) {
                Some(&p) => {
                    copy.parent = Some(p);
                    inside.push(copy);
                }
                None => {
                    copy.parent = parent;
                    roots.push(copy);
                }
            }
        }
        // Roots go in at the spot, in order; their contents follow them.
        let at = self.slot(crate::shader_tree::Drop { parent, index });
        let count = roots.len();
        self.nodes.splice(at..at, roots);
        let after = at + count;
        self.nodes.splice(after..after, inside);
        Ok(ids)
    }

    /// Copy (§5): the layers in `ids` with everything inside them.
    pub fn copy_layers(&self, ids: &[NodeId]) -> ShaderClipboard {
        let mut clip = ShaderClipboard::default();
        for root in self.top_most(ids) {
            for n in self.branch(root) {
                // An instance copies as a plain layer with its source's
                // values (§2).
                let mut copy = crate::shader_instances::as_plain_copy(self, n);
                // A copy of a Library item is an ordinary group.
                copy.extras.library = false;
                copy.extras.locked = false;
                if n.id == root {
                    copy.parent = None;
                }
                if let Some(o) = self.outputs.get(&n.id) {
                    clip.outputs.insert(n.id, o.clone());
                }
                if let NodeKind::NodeMaterial(m) = &copy.kind
                    && let Some(g) = m.graph.and_then(|g| self.node_graphs.get(g))
                    && !clip.graphs.iter().any(|x| x.id == g.id)
                {
                    clip.graphs.push(g.clone());
                }
                clip.nodes.push(copy);
            }
        }
        clip
    }
}

impl Scene {
    /// Copy (§5), with the copied texture layers' locators.
    pub fn copy_shader_layers(&self, ids: &[NodeId]) -> ShaderClipboard {
        let mut clip = self.shader_tree.copy_layers(ids);
        for n in &clip.nodes {
            if let NodeKind::Texture(TextureLayer {
                locator: Some(l), ..
            }) = &n.kind
                && let Some(item) = self.item(*l)
                && let Some(settings) = item.texture_locator()
            {
                clip.locators
                    .insert(*l, (Box::new(settings.clone()), item.transform.clone()));
            }
        }
        clip
    }

    /// Put copies of `clip` into the tree at `parent`/`index`: fresh ids
    /// and names, each copied texture layer its own new locator (layers
    /// that shared one share the new one), and the Render Output settings.
    /// Returns the new roots' ids.
    pub(crate) fn paste_shader_copies(
        &mut self,
        next: &mut u64,
        clip: &ShaderClipboard,
        parent: Option<NodeId>,
        index: usize,
    ) -> Result<Vec<NodeId>> {
        let ids = self.shader_tree.insert_copies(&clip.nodes, parent, index)?;
        // Copied Node Materials keep their graphs, even pasted into another
        // scene (an identical graph already here is shared).
        for new in ids.values() {
            let Some(i) = self.shader_tree.index(*new) else {
                continue;
            };
            let NodeKind::NodeMaterial(m) = &self.shader_tree.nodes[i].kind else {
                continue;
            };
            let graph = m
                .graph
                .and_then(|g| clip.graphs.iter().find(|x| x.id == g))
                .cloned();
            let id = graph.map(|g| self.shader_tree.adopt_node_graph(&g));
            if let NodeKind::NodeMaterial(m) = &mut self.shader_tree.nodes[i].kind
                && id.is_some()
            {
                m.graph = id;
            }
        }
        for (old, new) in &ids {
            if let Some(o) = clip.outputs.get(old) {
                self.shader_tree.outputs.insert(*new, o.clone());
            }
        }
        let mut new_locators: BTreeMap<u64, u64> = BTreeMap::new();
        for n in &clip.nodes {
            let NodeKind::Texture(TextureLayer {
                locator: Some(old), ..
            }) = &n.kind
            else {
                continue;
            };
            let new_id = ids[&n.id];
            let locator = match new_locators.get(old) {
                Some(&l) => Some(l),
                None => match clip.locators.get(old) {
                    Some((settings, transform)) => {
                        let name = self
                            .shader_tree
                            .get(new_id)
                            .map_or_else(String::new, |x| x.name.clone());
                        let l = self.add_texture_locator(next, &name, (**settings).clone())?;
                        if let Some(item) = self.item_mut(l) {
                            item.transform = transform.clone();
                        }
                        new_locators.insert(*old, l);
                        Some(l)
                    }
                    // Copied without its locator (it was missing): renders
                    // as UV Map until it gets one, as before.
                    None => None,
                },
            };
            if let Some(i) = self.shader_tree.index(new_id)
                && let NodeKind::Texture(t) = &mut self.shader_tree.nodes[i].kind
            {
                t.locator = locator;
            }
        }
        Ok(clip
            .nodes
            .iter()
            .filter(|n| n.parent.is_none())
            .map(|n| ids[&n.id])
            .collect())
    }

    /// Duplicate (§2): each top-most layer of `ids` gets an independent
    /// copy directly above it, in the same parent. Returns the copies.
    pub(crate) fn duplicate_shader_layers(
        &mut self,
        next: &mut u64,
        ids: &[NodeId],
    ) -> Result<Vec<NodeId>> {
        let roots = self.shader_tree.top_most(ids);
        if roots.is_empty() {
            return Err("Select shader tree layers to duplicate".into());
        }
        let mut out = Vec::new();
        for root in roots {
            let clip = self.copy_shader_layers(&[root]);
            let node = self.shader_tree.get(root).expect("listed");
            let parent = node.parent;
            let index = self
                .shader_tree
                .children(parent)
                .position(|n| n.id == root)
                .expect("a child of its parent");
            let library = node.extras.library;
            let copies = self.paste_shader_copies(next, &clip, parent, index)?;
            // Duplicating a Library item makes another Library item.
            if library {
                for &c in &copies {
                    let i = self.shader_tree.index(c).expect("pasted");
                    self.shader_tree.nodes[i].extras.library = true;
                }
            }
            out.extend(copies);
        }
        Ok(out)
    }

    /// Paste (§5): directly above `selected`, in its parent; with nothing
    /// selected, where Add Layer puts a layer (below the Base Shader).
    pub(crate) fn paste_shader_layers(
        &mut self,
        next: &mut u64,
        clip: &ShaderClipboard,
        selected: Option<NodeId>,
    ) -> Result<Vec<NodeId>> {
        if clip.is_empty() {
            return Err("Nothing to paste: copy shader layers first".into());
        }
        let (parent, index) = match selected.and_then(|id| self.shader_tree.get(id)) {
            Some(n) => {
                let index = self
                    .shader_tree
                    .children(n.parent)
                    .position(|x| x.id == n.id)
                    .expect("a child of its parent");
                (n.parent, index)
            }
            None => {
                let d = self
                    .shader_tree
                    .add_target(crate::shader_tree::NewLayer::Material, None);
                (d.parent, d.index)
            }
        };
        self.paste_shader_copies(next, clip, parent, index)
    }
}

impl NewLayer {
    /// The texture types Change Type offers (§6): Add Layer's texture
    /// types.
    pub fn texture_types() -> Vec<NewLayer> {
        let mut out = vec![NewLayer::Constant, NewLayer::ImageMap];
        out.extend(
            crate::procedural::ProceduralKind::ALL
                .iter()
                .map(|&k| NewLayer::Procedural(k)),
        );
        out.extend([
            NewLayer::Gradient,
            NewLayer::VertexMap,
            NewLayer::Variation,
            NewLayer::Process,
            NewLayer::Occlusion,
        ]);
        out
    }
}

impl ShaderTree {
    /// Change Type (§6): texture layer `id` becomes type `to` in place,
    /// keeping its id, parent, position, Layer block (Enable, Opacity,
    /// Blend Mode, Invert), Effect, the masks inside it, Ignore Scale
    /// Group, editor colour and Texture Locator. Every type-specific
    /// setting starts at the new type's Add Layer values. The name gets
    /// the next index ("Layer" → "Layer (2)"); a layer still named after
    /// its type takes the new type's name instead (provisional, SLM-Q05).
    pub(crate) fn change_type(&mut self, id: NodeId, to: NewLayer) -> Result<()> {
        let node = self.get(id).ok_or("Shader tree layer no longer exists")?;
        let NodeKind::Texture(old) = &node.kind else {
            return Err("Only texture layers can change type".into());
        };
        if node.extras.instance.is_some() {
            return Err("An instance can't change type; change its source".into());
        }
        let NodeKind::Texture(fresh) = to.kind() else {
            return Err(format!("{} isn't a texture type", to.label()));
        };
        if old.source.label() == fresh.source.label() {
            return Err(format!("The layer is already {}", to.label()));
        }
        let named_after_type = unnumbered(&node.name) == old.source.label();
        let mut changed = node.clone();
        changed.kind = NodeKind::Texture(TextureLayer {
            source: fresh.source,
            ..old.clone()
        });
        changed.name = if named_after_type {
            let others = |n: &str| self.nodes.iter().any(|x| x.id != id && x.name == n);
            crate::user_defaults::free_name(to.label(), others)
        } else {
            self.copy_name(&node.name, &[])
        };
        let i = self.index(id).expect("found above");
        changed.validate()?;
        self.nodes[i] = changed;
        Ok(())
    }
}

/// The type Paste Values matches on (§5): Material → Material, Noise →
/// Noise.
fn value_type(node: &Node) -> &'static str {
    node.type_label()
}

impl ShaderTree {
    /// Paste Values (§5): every setting of `from` onto each of `targets`,
    /// which must be of the same type. Name, position, locator and editor
    /// colour stay (provisional, SLM-Q04).
    pub(crate) fn paste_values(&mut self, from: &Node, targets: &[NodeId]) -> Result<()> {
        if targets.is_empty() {
            return Err("Select the layers to paste values onto".into());
        }
        for &id in targets {
            let target = self.get(id).ok_or("Shader tree layer no longer exists")?;
            if value_type(target) != value_type(from) {
                return Err("Paste Values needs a layer of the same type".into());
            }
        }
        for &id in targets {
            let mut node = self.get(id).expect("checked").clone();
            let locator = match &node.kind {
                NodeKind::Texture(t) => t.locator,
                _ => None,
            };
            node.layer = crate::shader_tree::Layer {
                editor_color: node.layer.editor_color,
                ..from.layer.clone()
            };
            node.kind = from.kind.clone();
            if let NodeKind::Texture(t) = &mut node.kind {
                t.locator = locator;
            }
            self.replace(node)?;
        }
        Ok(())
    }

    /// The Material Group Select Polygons uses for `id` (§7): the layer
    /// itself when it is one, else the nearest one around it.
    pub fn polygon_group(&self, id: NodeId) -> Option<NodeId> {
        let mut at = Some(id);
        for _ in 0..=self.nodes.len() {
            let node = at.and_then(|a| self.get(a))?;
            if matches!(node.kind, NodeKind::Group(_)) {
                return Some(node.id);
            }
            at = node.parent;
        }
        None
    }
}

impl Scene {
    /// The polygons of every visible mesh that layer `layer` of `tree`
    /// reaches (`also_self`: its own mask counts too, for a group).
    pub(crate) fn polygons_reached(
        &self,
        tree: &ShaderTree,
        layer: NodeId,
        also_self: bool,
    ) -> Vec<(u64, Vec<u32>)> {
        let mut out = Vec::new();
        let own = tree.get(layer).and_then(|n| match &n.kind {
            NodeKind::Group(g) if also_self => Some(g.clone()),
            _ => None,
        });
        for item in self.items.iter().filter(|i| i.is_mesh() && self.shows(i)) {
            let ancestors = self.item_ancestors(item.id);
            let mut sets: Vec<Vec<&str>> = vec![Vec::new(); item.mesh.faces.len()];
            for (name, faces) in &item.mesh.attributes.polygon_sets {
                for &f in faces {
                    if let Some(list) = sets.get_mut(f as usize) {
                        list.push(name.as_str());
                    }
                }
            }
            let mut faces = Vec::new();
            for (f, face) in item.mesh.faces.iter().enumerate() {
                let ctx = crate::shader_eval::Context {
                    item: item.id,
                    ancestors: &ancestors,
                    material: &face.material,
                    part: face.part.as_deref(),
                    sets: &sets[f],
                    locators: &[],
                };
                if own
                    .as_ref()
                    .is_none_or(|g| crate::shader_eval::mask_matches(g, &ctx))
                    && tree.layer_applies(layer, &ctx)
                {
                    faces.push(f as u32);
                }
            }
            if !faces.is_empty() {
                out.push((item.id, faces));
            }
        }
        out
    }

    /// Select Polygons (§7): the polygons the Material Group around `id`
    /// matches, on every visible mesh, in Polygon mode. The meshes that
    /// have some become the selected items.
    pub(crate) fn select_shader_polygons(&mut self, id: NodeId) -> Result<usize> {
        let tree = self.render_shader_tree().into_owned();
        let group = tree
            .polygon_group(id)
            .ok_or("Select Polygons needs a Material Group")?;
        let reached = self.polygons_reached(&tree, group, true);
        let Some((first, _)) = reached.first() else {
            return Err("No polygons match this group's mask".into());
        };
        let count = reached.iter().map(|(_, f)| f.len()).sum();
        self.select_item(Some(*first));
        self.selection.also = reached.iter().skip(1).map(|(i, _)| *i).collect();
        self.selection.mode = crate::Mode::Polygon;
        self.selection.tag = None;
        self.selection.vertices.clear();
        self.selection.edges.clear();
        self.selection.others.clear();
        for (item, faces) in reached {
            self.selection.put_components(
                item,
                crate::multi_active::ComponentSelection {
                    faces,
                    ..Default::default()
                },
            );
        }
        Ok(count)
    }
}
