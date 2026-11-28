//! Presets, Stage 0 (PRESET-BROWSER.md PB-40..52): save a reusable piece of
//! a scene to its own file and load it back into any scene. No browser yet.
//!
//! - A preset file IS a cut-down Operandi scene file (PB-Q1, decided): the
//!   same serializer, loader and version numbers, plus a small `preset`
//!   header naming its kind and a reserved thumbnail slot (PB-52; Stage 0
//!   writes none). Files end in `.oppreset` so the Load Preset panel can
//!   offer only presets. Plain File > Open doesn't read them as scenes.
//! - Kinds (PB-02): Material (a material or material group with every
//!   layer inside it), Item (the selected items; a mesh drops its item
//!   transform, PB-44), Assembly (the selected items with everything below
//!   them, hierarchy kept), Environment (every light and environment item;
//!   [chosen] environment items joined lights here when they arrived,
//!   environments-2026-11-09) and Render Settings (samples, denoising and
//!   adaptive sampling, the Render Output layers, and the render camera as
//!   an extra camera). Profile presets (PB-43)
//!   wait for user profile curves, which Operandi doesn't have.
//! - Saving changes nothing in the scene. Each load is one undo step
//!   (PB-35) and loads per kind (PB-31): items are added; a material
//!   replaces what is inside the chosen material group (which keeps its
//!   own mask, so it still covers the same polygons); an environment
//!   replaces every light and environment item; render settings replace the scene's render
//!   settings and Render Outputs and add the cameras.
//! - Provisional readings are marked (PB-Q4..Q7 in
//!   docs/SPECIFICATION_QUESTIONS.md).
pub mod convert;
pub mod texture_set;

use crate::backdrop::SceneImage;
use crate::shader_tree::{ItemMask, NewLayer, Node, NodeId, NodeKind, ShaderTree, TagMask};
use crate::texture_locator::TextureItem;
use crate::{Editor, Item, Result, Scene};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod animation;
pub mod ball;
pub mod basics;
pub mod library;
pub mod lxp;
pub mod matcap;
pub mod modo;
pub mod online;
pub mod pack;
pub mod placement;
pub mod starter_meshes;

/// File extension of preset files (without the dot).
pub const PRESET_EXTENSION: &str = "oppreset";
/// File extension of environment presets (EP: a cut-down scene file like
/// the others, its own extension so the browser and panels tell them
/// apart). Older `.oppreset` environment presets still load.
pub const ENVIRONMENT_PRESET_EXTENSION: &str = "openv";

/// The extension a preset of `kind` is saved with.
pub fn extension_for(kind: PresetKind) -> &'static str {
    match kind {
        PresetKind::Environment => ENVIRONMENT_PRESET_EXTENSION,
        _ => PRESET_EXTENSION,
    }
}

/// Whether `ext` (no dot, any case) names a preset file.
pub fn is_preset_extension(ext: &str) -> bool {
    ext.eq_ignore_ascii_case(PRESET_EXTENSION)
        || ext.eq_ignore_ascii_case(ENVIRONMENT_PRESET_EXTENSION)
}

/// What Save Environment Preset… includes (EP §Save, v0.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnvironmentPresetOptions {
    pub lights: bool,
    /// The selected meshes.
    pub meshes: bool,
    pub cameras: bool,
    /// Copy the pictures next to the preset (named relative).
    pub copy_images: bool,
}

/// How an environment preset is applied (EP §Apply).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyMode {
    /// Every environment goes, and the lights and meshes an earlier preset
    /// brought; hand-made ones stay. Then Add.
    Replace,
    /// The preset's environments go in at the top (so they win), with its
    /// lights and meshes.
    Add,
}

/// What a preset holds (PB-02). Recognised from the file, not its folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PresetKind {
    Material,
    Item,
    Assembly,
    Environment,
    RenderSettings,
}
impl PresetKind {
    pub const ALL: [PresetKind; 5] = [
        PresetKind::Material,
        PresetKind::Item,
        PresetKind::Assembly,
        PresetKind::Environment,
        PresetKind::RenderSettings,
    ];
    pub fn label(self) -> &'static str {
        match self {
            PresetKind::Material => "Material",
            PresetKind::Item => "Item",
            PresetKind::Assembly => "Assembly",
            PresetKind::Environment => "Environment",
            PresetKind::RenderSettings => "Render Settings",
        }
    }
    /// The name scripts use (`preset.save kind:<name>`).
    pub fn script_name(self) -> &'static str {
        match self {
            PresetKind::Material => "material",
            PresetKind::Item => "item",
            PresetKind::Assembly => "assembly",
            PresetKind::Environment => "environment",
            PresetKind::RenderSettings => "renderSettings",
        }
    }
    pub fn from_script_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|k| k.script_name().eq_ignore_ascii_case(name))
    }
}

/// The header that makes a scene file a preset.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetHeader {
    pub kind: PresetKind,
    /// PB-52: a small PNG preview, base64. The preset browser (Stage 1)
    /// shows it, and writes it when a preset is saved into the browser or
    /// the user replaces the thumbnail (`persistence::set_preset_thumbnail`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    /// Where the preset came from and under what licence (converted
    /// content, convert.rs). New in native version 37; older presets have
    /// none and load as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PresetCredit>,
}

/// A preset's licence and credit (the starter pack keeps one per
/// material, so the browser can show and copy it).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetCredit {
    /// SPDX-style licence name, e.g. "CC0-1.0".
    pub licence: String,
    /// The asset's page, e.g. `https://ambientcg.com/a/Bricks113A`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    /// The line to credit, e.g. "ambientCG".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub credit: String,
}

/// A preset as read from or written to a file.
#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    pub header: PresetHeader,
    pub scene: Scene,
}

impl Preset {
    /// NM-17: the preset embeds node graphs (its Node Materials'), for the
    /// preset browser's "Nodes" badge.
    pub fn has_nodes(&self) -> bool {
        !self.scene.shader_tree.node_graphs.is_empty()
    }
}

/// What to save.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetSource {
    /// A Material or Material Group in the shader tree.
    Material(NodeId),
    /// The selected items.
    Items,
    /// The selected items and everything below them.
    Assembly,
    /// Every environment with its layers and Texture Locators (EP §Save);
    /// with `lights` every light too, with `meshes` the selected meshes,
    /// with `cameras` every camera (EP v0.2). Never render settings.
    Environment {
        lights: bool,
        meshes: bool,
        cameras: bool,
    },
    /// Render Outputs and the render camera.
    RenderSettings,
}

/// A built preset and notes for the user (e.g. PB-44's transform warning).
pub struct Built {
    pub preset: Preset,
    pub notes: Vec<String>,
}

impl PresetSource {
    pub fn kind(self) -> PresetKind {
        match self {
            PresetSource::Material(_) => PresetKind::Material,
            PresetSource::Items => PresetKind::Item,
            PresetSource::Assembly => PresetKind::Assembly,
            PresetSource::Environment { .. } => PresetKind::Environment,
            PresetSource::RenderSettings => PresetKind::RenderSettings,
        }
    }

    /// Cut the preset out of `scene`. `scene` is not changed.
    pub fn build(self, scene: &Scene) -> Result<Built> {
        let tree = scene.render_shader_tree();
        let mut out = Cut::default();
        let mut notes = Vec::new();
        match self {
            PresetSource::Material(id) => {
                let node = tree.get(id).ok_or("Select a material or material group")?;
                if !matches!(node.kind, NodeKind::Material(_) | NodeKind::Group(_)) {
                    return Err("Select a material or material group to save".into());
                }
                out.add_subtree(scene, &tree, id);
            }
            PresetSource::Items | PresetSource::Assembly => {
                let selected: Vec<u64> = scene
                    .selected_items()
                    .into_iter()
                    .filter(|&id| {
                        scene
                            .items
                            .iter()
                            .any(|i| i.id == id && !i.is_texture_item() && i.in_item_list())
                    })
                    .collect();
                // ENVIRONMENT-PLACEMENT §1: Item and Assembly presets work on
                // Item List items; an environment goes in an Environment
                // preset.
                if selected.is_empty() {
                    return Err("Select one or more items to save as a preset".into());
                }
                if self == PresetSource::Assembly {
                    // Each selected item not already below another selected
                    // one brings its whole branch, placed in world space.
                    for &root in &selected {
                        if selected
                            .iter()
                            .any(|&other| other != root && is_below(scene, root, other))
                        {
                            continue;
                        }
                        for item in scene.branch(root)? {
                            if !item.is_texture_item() {
                                out.items.push(item);
                            }
                        }
                    }
                } else {
                    for &id in &selected {
                        // `branch` gives the item placed in world space.
                        let mut item = scene.branch(id)?.swap_remove(0);
                        item.parent = None;
                        if is_mesh(&item) {
                            // PB-44: a mesh preset keeps no item transform.
                            if item.transform != crate::Transform::default() {
                                notes.push(format!(
                                    "{} will load at the origin: mesh presets don't keep the item transform",
                                    item.name
                                ));
                            }
                            item.transform = crate::Transform::default();
                        }
                        out.items.push(item);
                    }
                }
                out.add_item_materials(scene, &tree);
            }
            PresetSource::Environment {
                lights,
                meshes,
                cameras,
            } => {
                // EP §Save: every environment in order, with the Texture
                // Locators its layers use.
                let envs: Vec<u64> = scene
                    .items
                    .iter()
                    .filter(|i| i.environment.is_some())
                    .map(|i| i.id)
                    .collect();
                let locators = scene.environment_locators(&envs);
                for item in scene.items.iter().filter(|i| {
                    i.environment.is_some()
                        || locators.contains(&i.id)
                        || (lights && i.light.is_some())
                        || (cameras && i.camera.is_some())
                }) {
                    let mut item = scene.branch(item.id)?.swap_remove(0);
                    item.parent = None;
                    out.items.push(item);
                }
                if meshes {
                    let mut any = false;
                    for id in scene.selected_items() {
                        let Some(found) = scene.item(id) else {
                            continue;
                        };
                        if !is_mesh(found) || found.is_texture_item() || !found.in_item_list() {
                            continue;
                        }
                        let mut item = scene.branch(id)?.swap_remove(0);
                        item.parent = None;
                        out.items.push(item);
                        any = true;
                    }
                    if any {
                        out.add_item_materials(scene, &tree);
                    }
                }
                if envs.is_empty() && !out.items.iter().any(|i| i.light.is_some()) {
                    return Err(
                        "The scene has no environments or lights to save as an environment preset"
                            .into(),
                    );
                }
            }
            PresetSource::RenderSettings => {
                for node in tree
                    .nodes
                    .iter()
                    .filter(|n| matches!(n.kind, NodeKind::RenderOutput(_)))
                {
                    let mut node = node.clone();
                    node.parent = None;
                    out.nodes.push(node);
                }
                // PB-02: render settings can carry extra cameras, added on
                // load. Provisional (PB-Q5): the render camera is the one.
                if let Some(camera) = scene.render_camera() {
                    let mut item = scene.branch(camera)?.swap_remove(0);
                    item.parent = None;
                    out.items.push(item);
                }
            }
        }
        let mut cut = out.into_scene(scene)?;
        if self == PresetSource::RenderSettings {
            // Samples, denoising and adaptive sampling (scene.render).
            cut.render = scene.render.clone();
        }
        let preset = Preset {
            header: PresetHeader {
                kind: self.kind(),
                thumbnail: None,
                source: None,
            },
            scene: cut,
        };
        Ok(Built { preset, notes })
    }
}

fn is_mesh(item: &Item) -> bool {
    item.camera.is_none()
        && item.light.is_none()
        && item.backdrop.is_none()
        && item.texture.is_none()
        && item.locator.is_none()
        && item.environment.is_none()
}

/// Whether `id` lies somewhere below `ancestor`.
fn is_below(scene: &Scene, id: u64, ancestor: u64) -> bool {
    let mut at = scene.item(id).and_then(|i| i.parent);
    for _ in 0..=scene.items.len() {
        match at {
            None => return false,
            Some(a) if a == ancestor => return true,
            Some(a) => at = scene.item(a).and_then(|i| i.parent),
        }
    }
    false
}

/// The pieces of a preset scene being gathered.
#[derive(Default)]
struct Cut {
    items: Vec<Item>,
    /// Shader tree layers; top-level ones have no parent.
    nodes: Vec<Node>,
}

impl Cut {
    /// `id` and every layer inside it, with the Texture Locators their
    /// texture layers use. `id` becomes a top-level layer.
    fn add_subtree(&mut self, scene: &Scene, tree: &ShaderTree, id: NodeId) {
        for node in tree.nodes.iter().filter(|n| tree.is_within(n.id, id)) {
            let mut node = node.clone();
            if node.id == id {
                node.parent = None;
            }
            // An instance of a layer outside the preset saves its values
            // as a plain layer.
            if node
                .extras
                .instance
                .as_ref()
                .is_some_and(|l| !tree.is_within(l.source, id))
            {
                node.extras.instance = None;
            }
            if let NodeKind::Texture(t) = &node.kind
                && let Some(locator) = t.locator
                && !self.items.iter().any(|i| i.id == locator)
                && let Some(item) = scene.item(locator)
            {
                let mut item = item.clone();
                item.parent = None;
                self.items.push(item);
            }
            self.nodes.push(node);
        }
    }

    /// The Material Groups that shade the saved meshes' polygon tags, so a
    /// mesh preset looks the same in another scene (provisional, PB-Q6).
    fn add_item_materials(&mut self, scene: &Scene, tree: &ShaderTree) {
        let tags: BTreeSet<String> = self
            .items
            .iter()
            .flat_map(|i| i.mesh.faces.iter().map(|f| f.material.clone()))
            .filter(|t| !crate::material::is_default_name(t))
            .collect();
        for tag in tags {
            if let Some(group) = tree.nodes.iter().find(
                |n| matches!(&n.kind, NodeKind::Group(g) if g.tag == TagMask::Name(tag.clone())),
            ) && !self.nodes.iter().any(|n| n.id == group.id)
            {
                self.add_subtree(scene, tree, group.id);
            }
        }
    }

    /// The preset scene. References to anything left behind are cleared;
    /// the pictures used come along.
    fn into_scene(mut self, source: &Scene) -> Result<Scene> {
        let kept: BTreeSet<u64> = self.items.iter().map(|i| i.id).collect();
        let mut image_ids = BTreeSet::new();
        for item in &mut self.items {
            if item.parent.is_some_and(|p| !kept.contains(&p)) {
                item.parent = None;
            }
            detach(item, &kept);
            if let Some(b) = &item.backdrop {
                image_ids.extend(b.image);
            }
            if let Some(c) = &item.camera {
                image_ids.extend(c.background.image);
            }
            if let Some(env) = &item.environment {
                for node in &env.layers {
                    if let NodeKind::Texture(t) = &node.kind
                        && let crate::shader_tree::TextureSource::ImageMap { image, .. } = &t.source
                    {
                        image_ids.extend(*image);
                    }
                }
            }
        }
        for node in &mut self.nodes {
            match &mut node.kind {
                // Item masks name items of the source scene (provisional,
                // PB-Q7: dropped to "(all)").
                NodeKind::Group(g) => g.item = ItemMask::All,
                NodeKind::Texture(t) => {
                    if let crate::shader_tree::TextureSource::ImageMap { image, .. } = &t.source {
                        image_ids.extend(*image);
                    }
                }
                _ => {}
            }
        }
        let tags: BTreeSet<&str> = self
            .items
            .iter()
            .flat_map(|i| i.mesh.faces.iter().map(|f| f.material.as_str()))
            .collect();
        let materials = source
            .materials
            .iter()
            .filter(|(name, _)| tags.contains(name.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let next_node = self.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
        let next_id = self.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        let scene = Scene {
            items: self.items,
            next_id,
            images: source
                .images
                .iter()
                .filter(|i| image_ids.contains(&i.id))
                .cloned()
                .collect(),
            smoothing: source.smoothing.clone(),
            materials,
            shader_tree: ShaderTree {
                // Render Output settings follow their layers (ids kept).
                outputs: source
                    .render_shader_tree()
                    .outputs
                    .iter()
                    .filter(|(id, _)| self.nodes.iter().any(|n| n.id == **id))
                    .map(|(id, o)| (*id, o.clone()))
                    .collect(),
                // NM-17: the graphs its Node Materials use come along.
                node_graphs: {
                    let tree = source.render_shader_tree();
                    let used: Vec<u64> = self
                        .nodes
                        .iter()
                        .filter_map(|n| match &n.kind {
                            crate::shader_tree::NodeKind::NodeMaterial(m) => m.graph,
                            _ => None,
                        })
                        .collect();
                    crate::node_graph::NodeSection {
                        graphs: tree
                            .node_graphs
                            .graphs
                            .iter()
                            .filter(|g| used.contains(&g.id))
                            .cloned()
                            .collect(),
                        ..Default::default()
                    }
                },
                // Free nodes stay with the scene for now (NE-Q03).
                node_views: Default::default(),
                nodes: self.nodes,
                next_id: next_node,
            },
            ..Scene::default()
        };
        scene.validate()?;
        Ok(scene)
    }
}

/// Clear an item's links to items outside `kept` (and to item groups,
/// which presets don't carry).
fn detach(item: &mut Item, kept: &BTreeSet<u64>) {
    if item
        .common
        .target
        .as_ref()
        .is_some_and(|t| !kept.contains(&t.item))
    {
        item.common.target = None;
    }
    if let Some(light) = item.light.as_mut() {
        if light.prototype.is_some_and(|p| !kept.contains(&p)) {
            light.prototype = None;
        }
        if let Some(link) = light.linking.as_mut() {
            link.group = None;
        }
    }
}

/// Add preset items to `scene` with fresh ids (parents, targets and
/// pictures follow). Texture Locators go into the scene's Texture Group.
/// Returns the old-to-new id map.
fn insert_items(
    scene: &mut Scene,
    next: &mut u64,
    items: &[Item],
    images: &[SceneImage],
) -> Result<BTreeMap<u64, u64>> {
    if *next >= u64::MAX - items.len() as u64 - 1
        || scene.items.len() + items.len() > crate::MAX_SCENE_ITEMS
    {
        return Err("Scene item/identity budget exhausted".into());
    }
    let mut image_map = BTreeMap::new();
    for image in images {
        image_map.insert(
            image.id,
            scene.image_for_path(&image.path, image.width, image.height)?,
        );
    }
    let texture_group = if items.iter().any(|i| i.texture_locator().is_some()) {
        Some(scene.ensure_texture_group(next)?)
    } else {
        None
    };
    let start = (*next).max(scene.next_id);
    let ids: BTreeMap<u64, u64> = items
        .iter()
        .enumerate()
        .map(|(k, i)| (i.id, start + k as u64))
        .collect();
    for item in items {
        let mut copy = item.clone();
        copy.id = ids[&item.id];
        copy.parent = item.parent.and_then(|p| ids.get(&p).copied());
        // An instance of an item that didn't come along shows its own copy.
        copy.instance_of = item.instance_of.and_then(|p| ids.get(&p).copied());
        if copy.texture_locator().is_some() {
            copy.parent = texture_group;
        } else if matches!(copy.texture, Some(TextureItem::Group)) {
            // A preset never holds the Texture Group itself.
            continue;
        }
        copy.name = scene.free_name(&item.name);
        copy.source_id = None;
        copy.source_display_name = None;
        if let Some(t) = copy.common.target.as_mut() {
            match ids.get(&t.item) {
                Some(&id) => t.item = id,
                None => copy.common.target = None,
            }
        }
        if let Some(light) = copy.light.as_mut() {
            light.prototype = light.prototype.and_then(|p| ids.get(&p).copied());
        }
        if let Some(b) = copy.backdrop.as_mut() {
            b.image = b.image.and_then(|i| image_map.get(&i).copied());
        }
        if let Some(c) = copy.camera.as_mut() {
            c.background.image = c.background.image.and_then(|i| image_map.get(&i).copied());
        }
        if let Some(env) = copy.environment.as_mut() {
            // Its layers' locators and pictures follow; its layer ids and
            // material name are made free in this scene.
            for node in &mut env.layers {
                if let NodeKind::Texture(t) = &mut node.kind {
                    t.locator = t.locator.and_then(|l| ids.get(&l).copied());
                    if let crate::shader_tree::TextureSource::ImageMap { image, .. } = &mut t.source
                    {
                        *image = image.and_then(|i| image_map.get(&i).copied());
                    }
                }
            }
            scene.renumber_environment(env)?;
        }
        scene.items.push(copy);
    }
    *next = start + ids.len() as u64;
    scene.next_id = scene.next_id.max(*next);
    Ok(ids)
}

/// Adds material preset scene `source` (one root Material Group, as
/// `materialx::material_preset` makes) to `scene` where Add Layer puts a
/// group (below the Base Shader), as the Material Group of polygon tag
/// `tag`, named after it.
/// Importers use it to bring a file's materials in; nothing is recorded.
/// Returns the new group.
pub fn add_material_group(scene: &mut Scene, source: &Scene, tag: &str) -> Result<NodeId> {
    let roots: Vec<NodeId> = source
        .shader_tree
        .nodes
        .iter()
        .filter(|n| n.parent.is_none())
        .map(|n| n.id)
        .collect();
    let [root] = roots[..] else {
        return Err("The material holds no single Material Group".into());
    };
    let nodes = &source.shader_tree.nodes[..];
    let place = nodes
        .iter()
        .filter(|n| within(nodes, n.id, root))
        .position(|n| n.id == root)
        .ok_or("The material holds no single Material Group")?;
    let mut next = scene.next_id;
    let locators = insert_items(scene, &mut next, &source.items, &[])?;
    let images = image_map(scene, &source.images)?;
    let group = scene.shader_tree.next_id + place as u64;
    // Where Add Layer puts a new group: below the Base Shader, which
    // shades everything under it.
    let drop = scene.shader_tree.add_target(NewLayer::Group, None);
    insert_nodes(
        &mut scene.shader_tree,
        &source.shader_tree,
        &[root],
        drop.parent,
        drop.index,
        &locators,
        &images,
    )?;
    let taken: BTreeSet<String> = scene
        .shader_tree
        .nodes
        .iter()
        .filter(|n| n.id != group)
        .map(|n| n.name.clone())
        .collect();
    let node = scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == group)
        .ok_or("The material's group went missing")?;
    let NodeKind::Group(mask) = &mut node.kind else {
        return Err("The material's top layer isn't a Material Group".into());
    };
    mask.tag_type = crate::shader_tree::TagType::Material;
    mask.tag = TagMask::Name(tag.into());
    node.name = crate::user_defaults::free_name(tag, |n| taken.contains(n));
    Ok(group)
}

/// Insert preset layers into `tree` as children of `parent` at position
/// `index`, in the preset's order, with fresh ids. Top-level preset layers
/// (`roots`) take `parent`; layers whose parent was left out are skipped.
fn insert_nodes(
    tree: &mut ShaderTree,
    source: &ShaderTree,
    roots: &[NodeId],
    parent: Option<NodeId>,
    index: usize,
    locators: &BTreeMap<u64, u64>,
    images: &BTreeMap<u64, u64>,
) -> Result<()> {
    let nodes = &source.nodes[..];
    let wanted: Vec<&Node> = nodes
        .iter()
        .filter(|n| roots.iter().any(|&r| within(nodes, n.id, r)))
        .collect();
    crate::shader_tree::check_node_count(tree.nodes.len() + wanted.len())?;
    if tree.next_id >= u64::MAX - wanted.len() as u64 {
        return Err("Shader tree is full".into());
    }
    let ids: BTreeMap<NodeId, NodeId> = wanted
        .iter()
        .enumerate()
        .map(|(k, n)| (n.id, tree.next_id + k as u64))
        .collect();
    let at = tree.slot(crate::shader_tree::Drop { parent, index });
    let mut fresh = Vec::with_capacity(wanted.len());
    for node in wanted {
        let mut copy = node.clone();
        copy.id = ids[&node.id];
        copy.parent = if roots.contains(&node.id) {
            parent
        } else {
            node.parent.and_then(|p| ids.get(&p).copied())
        };
        copy.name = crate::user_defaults::free_name(&node.name, |name| {
            tree.nodes
                .iter()
                .chain(fresh.iter())
                .any(|n: &Node| n.name == name)
        });
        if let NodeKind::Texture(t) = &mut copy.kind {
            t.locator = t.locator.and_then(|l| locators.get(&l).copied());
            if let crate::shader_tree::TextureSource::ImageMap { image, .. } = &mut t.source {
                *image = image.and_then(|i| images.get(&i).copied());
            }
        }
        if let NodeKind::Group(g) = &mut copy.kind {
            g.item = ItemMask::All;
        }
        // NM-17: the preset's graph joins the scene's Nodes section (an
        // identical one already there is shared).
        if let NodeKind::NodeMaterial(m) = &mut copy.kind {
            m.graph = m
                .graph
                .and_then(|g| source.node_graphs.get(g))
                .map(|g| tree.adopt_node_graph(g));
            if m.graph.is_none() {
                m.overrides.clear();
            }
        }
        // An instance whose source came along links to the new copy; one
        // whose source was left out keeps its values as a plain layer.
        let source_id = copy.extras.instance.as_ref().map(|l| l.source);
        match source_id.map(|s| ids.get(&s).copied()) {
            Some(Some(s)) => {
                if let Some(l) = &mut copy.extras.instance {
                    l.source = s;
                }
            }
            Some(None) => copy.extras.instance = None,
            None => {}
        }
        if let Some(settings) = source.outputs.get(&node.id) {
            tree.outputs.insert(copy.id, settings.clone());
        }
        fresh.push(copy);
    }
    tree.next_id += ids.len() as u64;
    tree.nodes.splice(at..at, fresh);
    Ok(())
}

/// `id` is `ancestor` or inside it, among preset layers.
fn within(nodes: &[Node], id: NodeId, ancestor: NodeId) -> bool {
    let mut at = Some(id);
    for _ in 0..=nodes.len() {
        match at {
            None => return false,
            Some(a) if a == ancestor => return true,
            Some(a) => at = nodes.iter().find(|n| n.id == a).and_then(|n| n.parent),
        }
    }
    false
}

/// How picture `path` is written into a preset saved in `folder` (EP §Save):
/// with `copy`, copied there (a different file of the same name gets
/// " (2)") and named by its file name; else relative when it lies in
/// `folder` or below, absolute otherwise. A picture that can't be copied
/// keeps its absolute path, with a note.
fn preset_image_path(folder: &std::path::Path, path: &str, copy: bool) -> (String, Option<String>) {
    use std::path::Path;
    let source = Path::new(path);
    let relative = |p: &Path| {
        p.strip_prefix(folder)
            .ok()
            .map(|r| r.to_string_lossy().replace('\\', "/"))
    };
    if let Some(r) = relative(source) {
        return (r, None);
    }
    if !copy {
        return (path.to_owned(), None);
    }
    let Some(name) = source.file_name() else {
        return (path.to_owned(), None);
    };
    let same = |a: &Path, b: &Path| {
        std::fs::read(a)
            .ok()
            .zip(std::fs::read(b).ok())
            .is_some_and(|(x, y)| x == y)
    };
    let stem = source
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let ext = source.extension().map(|e| e.to_string_lossy().into_owned());
    let mut target = folder.join(name);
    let mut n = 2;
    while target.exists() && !same(source, &target) {
        let file = match &ext {
            Some(e) => format!("{stem} ({n}).{e}"),
            None => format!("{stem} ({n})"),
        };
        target = folder.join(file);
        n += 1;
    }
    if !target.exists()
        && let Err(e) = std::fs::copy(source, &target)
    {
        return (
            path.to_owned(),
            Some(format!("Couldn't copy {}: {e}", source.display())),
        );
    }
    (
        target
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        None,
    )
}

/// The pictures of a loaded preset whose files are missing (EP: they load
/// anyway, keeping their paths, and show missing).
pub fn missing_images(preset: &Preset) -> Vec<String> {
    preset
        .scene
        .images
        .iter()
        .filter(|i| !i.path.is_empty() && !std::path::Path::new(&i.path).exists())
        .map(|i| i.path.clone())
        .collect()
}

/// The scene image ids of a preset, remapped into `scene` (adding entries
/// for new files).
fn image_map(scene: &mut Scene, images: &[SceneImage]) -> Result<BTreeMap<u64, u64>> {
    images
        .iter()
        .map(|i| Ok((i.id, scene.image_for_path(&i.path, i.width, i.height)?)))
        .collect()
}

/// EP §Apply (v0.2) on `candidate`. Replace first removes every
/// environment with its layers and their Texture Locators, and every item
/// group and material group an earlier preset left (with everything in
/// them), keeping hand-made items outside those groups. Then the preset's
/// environments go in at the top, in their order (they win), names,
/// Intensity and visibility kept; its lights, meshes and cameras arrive
/// inside one item group named `name`, and its materials under a Shader
/// Tree group of the same name, both marked as from this preset. Name
/// clashes get " (2)". Returns the new top environment.
fn apply_environment(
    candidate: &mut Scene,
    next: &mut u64,
    source: &Scene,
    mode: ApplyMode,
    name: &str,
) -> Result<Option<u64>> {
    if source.environments().next().is_none()
        && !source
            .items
            .iter()
            .any(|i| i.light.is_some() || i.camera.is_some() || is_mesh(i))
    {
        return Err("This environment preset holds no environments or lights".into());
    }
    if mode == ApplyMode::Replace {
        remove_environments_and_preset_groups(candidate)?;
    }
    let incoming: Vec<Item> = source
        .items
        .iter()
        .filter(|i| !matches!(i.texture, Some(TextureItem::Group)))
        .cloned()
        .collect();
    let ids = insert_items(candidate, next, &incoming, &source.images)?;
    // Lights, meshes and cameras (not environments or their locators) go
    // into one item group named after the preset.
    let grouped: Vec<u64> = source
        .items
        .iter()
        .filter(|i| i.environment.is_none() && !i.is_texture_item() && i.parent.is_none())
        .filter_map(|i| ids.get(&i.id).copied())
        .collect();
    if !grouped.is_empty() {
        candidate.next_id = candidate.next_id.max(*next);
        let group = candidate.add_group_locator()?;
        *next = (*next).max(candidate.next_id);
        let free = crate::user_defaults::free_name(name, |n| {
            candidate.items.iter().any(|i| i.id != group && i.name == n)
        });
        for item in candidate.items.iter_mut() {
            if item.id == group {
                item.name = free.clone();
                item.common.from_preset = Some(name.to_owned());
            } else if grouped.contains(&item.id) {
                // The group sits at the origin with no rotation or scale,
                // so the items keep their world placement.
                item.parent = Some(group);
            }
        }
    }
    // Materials go under a Shader Tree group of the preset's name.
    let roots: Vec<NodeId> = source
        .shader_tree
        .nodes
        .iter()
        .filter(|n| n.parent.is_none() && matches!(n.kind, NodeKind::Group(_)))
        .map(|n| n.id)
        .collect();
    if !roots.is_empty() {
        let group = candidate.shader_tree.add(NewLayer::Group, None)?;
        let mut node = candidate
            .shader_tree
            .get(group)
            .ok_or("The material group wasn't added")?
            .clone();
        node.name = crate::user_defaults::free_name(name, |n| {
            candidate
                .shader_tree
                .nodes
                .iter()
                .any(|other| other.id != group && other.name == n)
        });
        node.extras.from_preset = Some(Box::new(name.to_owned()));
        candidate.shader_tree.replace(node)?;
        let images = image_map(candidate, &source.images)?;
        insert_nodes(
            &mut candidate.shader_tree,
            &source.shader_tree,
            &roots,
            Some(group),
            0,
            &ids,
            &images,
        )?;
    }
    for (tag, material) in &source.materials {
        candidate
            .materials
            .entry(tag.clone())
            .or_insert_with(|| material.clone());
    }
    let top = source
        .items
        .iter()
        .rev()
        .find(|i| i.environment.is_some())
        .and_then(|i| ids.get(&i.id).copied());
    Ok(top)
}

/// EP Replace's removal: every environment with its layers' Texture
/// Locators (none left orphaned), and every item group and material group
/// an earlier preset left, with everything in them. Items outside those
/// groups that were parented into removed ones move to the root in place.
fn remove_environments_and_preset_groups(candidate: &mut Scene) -> Result<()> {
    let envs: Vec<u64> = candidate
        .items
        .iter()
        .filter(|i| i.environment.is_some())
        .map(|i| i.id)
        .collect();
    let mut locators = candidate.environment_locators(&envs);
    let preset_groups: Vec<u64> = candidate
        .items
        .iter()
        .filter(|i| i.common.from_preset.is_some())
        .map(|i| i.id)
        .collect();
    let mut gone: BTreeSet<u64> = envs.iter().copied().collect();
    for &g in &preset_groups {
        gone.insert(g);
        for item in &candidate.items {
            if is_below(candidate, item.id, g) {
                gone.insert(item.id);
            }
        }
    }
    let orphans: Vec<u64> = candidate
        .items
        .iter()
        .filter(|i| !gone.contains(&i.id) && i.parent.is_some_and(|p| gone.contains(&p)))
        .map(|i| i.id)
        .collect();
    for id in orphans {
        if candidate.set_parent(id, None, true).is_err() {
            candidate.set_parent(id, None, false)?;
        }
    }
    candidate.items.retain(|i| !gone.contains(&i.id));
    for g in &mut candidate.groups {
        g.members.retain(|m| !gone.contains(m));
    }
    if candidate.selection.item.is_some_and(|i| gone.contains(&i)) {
        candidate.select_item(None);
    }
    candidate.selection.also.retain(|i| !gone.contains(i));
    let material_groups: Vec<NodeId> = candidate
        .shader_tree
        .nodes
        .iter()
        .filter(|n| n.extras.from_preset.is_some())
        .map(|n| n.id)
        .collect();
    if !material_groups.is_empty() {
        locators.extend(candidate.shader_tree.locators_in(&material_groups));
        candidate.shader_tree.remove(&material_groups)?;
    }
    candidate.drop_environment_locators(&locators);
    Ok(())
}

impl Editor {
    /// Write a preset of `source` to `path` (PB-40..42). The scene doesn't
    /// change. Returns notes for the user.
    pub fn save_preset(&self, path: &std::path::Path, source: PresetSource) -> Result<Vec<String>> {
        self.save_preset_placed(path, source, false)
    }

    /// [`save_preset`](Self::save_preset), and with `center_on_ground`
    /// (PG-04, `preset.saveAs centerOnGround:true`) the saved copy of an
    /// item preset is moved so its meshes' box is centred on X and Z and
    /// sits on Y = 0. The scene doesn't change.
    pub fn save_preset_placed(
        &self,
        path: &std::path::Path,
        source: PresetSource,
        center_on_ground: bool,
    ) -> Result<Vec<String>> {
        self.require_idle()?;
        if self.session.pen_draft.is_some() {
            return Err("Finish or Cancel Pen before saving a preset".into());
        }
        let mut built = source.build(&self.scene)?;
        if center_on_ground && matches!(source, PresetSource::Items | PresetSource::Assembly) {
            placement::rest_on(&mut built.preset.scene, glam::DVec3::ZERO);
        }
        crate::persistence::save_preset(path, &built.preset)?;
        Ok(built.notes)
    }

    /// EP §Save: write an environment preset of every environment (with
    /// the lights, selected meshes and cameras `options` asks for) to
    /// `path`.
    /// Pictures are named relative to the preset when they sit beside or
    /// under it, else absolutely; with `copy_images` each is copied next
    /// to the preset first and named relative. The scene doesn't change.
    /// Returns notes for the user.
    pub fn save_environment_preset(
        &self,
        path: &std::path::Path,
        options: EnvironmentPresetOptions,
    ) -> Result<Vec<String>> {
        self.require_idle()?;
        let built = PresetSource::Environment {
            lights: options.lights,
            meshes: options.meshes,
            cameras: options.cameras,
        }
        .build(&self.scene)?;
        let copy_images = options.copy_images;
        let mut preset = built.preset;
        let mut notes = built.notes;
        let folder = path.parent().unwrap_or(std::path::Path::new("."));
        for image in &mut preset.scene.images {
            let (stored, note) = preset_image_path(folder, &image.path, copy_images);
            image.path = stored;
            notes.extend(note);
        }
        crate::persistence::save_preset(path, &preset)?;
        Ok(notes)
    }

    /// Load `preset` into the scene as one undo step (PB-31, PB-35).
    /// `group` is the Material Group a material preset goes into.
    pub(crate) fn load_preset(&mut self, preset: Preset, group: Option<NodeId>) -> Result<()> {
        if self.session.pen_draft.is_some() {
            return Err("Finish or Cancel Pen before loading a preset".into());
        }
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before loading a preset".into());
        }
        self.require_idle()?;
        let kind = preset.header.kind;
        let candidate = self.preset_candidate(self.scene.clone(), preset, group)?;
        if candidate == self.scene {
            return Ok(());
        }
        self.next_item_id = self.next_item_id.max(candidate.next_id);
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record(format!("Load {} Preset", kind.label()), before);
        Ok(())
    }

    /// EP §Apply: environment preset `preset` applied as `mode`, one undo
    /// step named after the mode; its lights and meshes remember `name`.
    /// Returns the new top environment.
    pub(crate) fn apply_environment_preset(
        &mut self,
        preset: Preset,
        mode: ApplyMode,
        name: &str,
    ) -> Result<Option<u64>> {
        if self.session.pen_draft.is_some() {
            return Err("Finish or Cancel Pen before loading a preset".into());
        }
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before loading a preset".into());
        }
        self.require_idle()?;
        if preset.header.kind != PresetKind::Environment {
            return Err("That isn't an environment preset".into());
        }
        let mut candidate = self.scene.clone();
        candidate.shader_tree = candidate.render_shader_tree().into_owned();
        let mut next = candidate.next_id.max(self.next_item_id);
        let top = apply_environment(&mut candidate, &mut next, &preset.scene, mode, name)?;
        candidate.shader_tree.validate()?;
        candidate.validate()?;
        self.next_item_id = self.next_item_id.max(candidate.next_id);
        let before = std::mem::replace(&mut self.scene, candidate);
        let label = match mode {
            ApplyMode::Replace => "Replace Environment",
            ApplyMode::Add => "Add Environment",
        };
        self.record(label.into(), before);
        Ok(top)
    }

    /// Add material preset `preset`'s layers to the tree as one undo step,
    /// replacing nothing: directly above layer `beside` (in its group), or
    /// where Add Layer puts a group (below the Base Shader) when `beside`
    /// is `None`.
    pub(crate) fn insert_material_preset(
        &mut self,
        preset: Preset,
        beside: Option<NodeId>,
    ) -> Result<()> {
        if self.session.pen_draft.is_some() {
            return Err("Finish or Cancel Pen before loading a preset".into());
        }
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before loading a preset".into());
        }
        self.require_idle()?;
        if preset.header.kind != PresetKind::Material {
            return Err("Only a material preset can be added as layers".into());
        }
        let mut candidate = self.scene.clone();
        candidate.shader_tree = candidate.render_shader_tree().into_owned();
        let drop = match beside {
            // Where Add Layer puts a new group: below the Base Shader. At
            // the very top it would sit above the winning Shader, where
            // its layers don't surface the polygon (G-19) and the import
            // would show nothing.
            None => candidate
                .shader_tree
                .add_target(crate::shader_tree::NewLayer::Group, None),
            Some(id) => {
                let tree = &candidate.shader_tree;
                let node = tree
                    .get(id)
                    .ok_or_else(|| format!("No shader tree layer {id}"))?;
                let index = tree
                    .children(node.parent)
                    .position(|n| n.id == id)
                    .unwrap_or(0);
                crate::shader_tree::Drop {
                    parent: node.parent,
                    index,
                }
            }
        };
        let mut next = candidate.next_id.max(self.next_item_id);
        let source = preset.scene;
        let roots: Vec<NodeId> = source
            .shader_tree
            .nodes
            .iter()
            .filter(|n| n.parent.is_none())
            .map(|n| n.id)
            .collect();
        if roots.is_empty() {
            return Err("This material preset holds no material".into());
        }
        let locators = insert_items(&mut candidate, &mut next, &source.items, &[])?;
        let images = image_map(&mut candidate, &source.images)?;
        insert_nodes(
            &mut candidate.shader_tree,
            &source.shader_tree,
            &roots,
            drop.parent,
            drop.index,
            &locators,
            &images,
        )?;
        candidate.shader_tree.validate()?;
        candidate.validate()?;
        self.next_item_id = self.next_item_id.max(candidate.next_id);
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record("Load Material Preset".to_string(), before);
        Ok(())
    }

    /// `base` with `preset` loaded into it per PB-31 (`group` for a
    /// material), validated. Nothing is recorded.
    fn preset_candidate(
        &self,
        base: Scene,
        preset: Preset,
        group: Option<NodeId>,
    ) -> Result<Scene> {
        let mut candidate = base;
        // Like a shader tree edit: an older file's materials join the tree
        // first, so what renders now is what the preset is added to.
        candidate.shader_tree = candidate.render_shader_tree().into_owned();
        let mut next = candidate.next_id.max(self.next_item_id);
        let kind = preset.header.kind;
        let source = preset.scene;
        let roots: Vec<NodeId> = source
            .shader_tree
            .nodes
            .iter()
            .filter(|n| n.parent.is_none())
            .map(|n| n.id)
            .collect();
        match kind {
            PresetKind::Material => {
                let target = group
                    .filter(|&g| {
                        candidate
                            .shader_tree
                            .get(g)
                            .is_some_and(|n| matches!(n.kind, NodeKind::Group(_)))
                    })
                    // PB-Q3 (decided): no material group, no change.
                    .ok_or("Select a material group in the Shading panel to load a material preset into")?;
                let [root] = roots[..] else {
                    return Err("This material preset holds no single material or group".into());
                };
                // The group's contents are replaced; the group keeps its
                // name, mask, blend and opacity.
                let old: Vec<NodeId> = candidate
                    .shader_tree
                    .children(Some(target))
                    .map(|n| n.id)
                    .collect();
                if !old.is_empty() {
                    let locators = candidate.shader_tree.locators_in(&old);
                    candidate.shader_tree.remove(&old)?;
                    candidate.drop_unused_locators(&locators);
                }
                let locators = insert_items(&mut candidate, &mut next, &source.items, &[])?;
                let images = image_map(&mut candidate, &source.images)?;
                let inner: Vec<NodeId> = match source.shader_tree.get(root).map(|n| &n.kind) {
                    Some(NodeKind::Group(_)) => source
                        .shader_tree
                        .children(Some(root))
                        .map(|n| n.id)
                        .collect(),
                    _ => vec![root],
                };
                insert_nodes(
                    &mut candidate.shader_tree,
                    &source.shader_tree,
                    &inner,
                    Some(target),
                    0,
                    &locators,
                    &images,
                )?;
            }
            PresetKind::Item | PresetKind::Assembly => {
                if source.items.iter().all(|i| i.is_texture_item()) {
                    return Err("This preset holds no items".into());
                }
                let ids = insert_items(&mut candidate, &mut next, &source.items, &source.images)?;
                animation::carry(&mut candidate, &mut next, &source, &ids)?;
                // The meshes' material groups come along when this scene
                // has none for their tags (provisional, PB-Q6).
                let missing: Vec<NodeId> = roots
                    .iter()
                    .copied()
                    .filter(|&r| match source.shader_tree.get(r).map(|n| &n.kind) {
                        Some(NodeKind::Group(g)) => match &g.tag {
                            TagMask::Name(tag) => {
                                candidate.shader_tree.material_for_tag(tag).is_none()
                            }
                            _ => false,
                        },
                        _ => false,
                    })
                    .collect();
                if !missing.is_empty() {
                    let drop = candidate.shader_tree.add_target(NewLayer::Group, None);
                    let images = image_map(&mut candidate, &source.images)?;
                    insert_nodes(
                        &mut candidate.shader_tree,
                        &source.shader_tree,
                        &missing,
                        drop.parent,
                        drop.index,
                        &ids,
                        &images,
                    )?;
                }
                for (tag, material) in &source.materials {
                    candidate
                        .materials
                        .entry(tag.clone())
                        .or_insert_with(|| material.clone());
                }
                // Select what arrived: the top items, first one primary.
                let top: Vec<u64> = source
                    .items
                    .iter()
                    .filter(|i| !i.is_texture_item() && i.parent.is_none())
                    .filter_map(|i| ids.get(&i.id).copied())
                    .collect();
                candidate.select_item(top.first().copied());
                candidate.selection.also = top.iter().skip(1).copied().collect();
            }
            PresetKind::Environment => {
                // A plain load is Replace (EP §Apply: double-click).
                apply_environment(
                    &mut candidate,
                    &mut next,
                    &source,
                    ApplyMode::Replace,
                    "Environment Preset",
                )?;
            }
            PresetKind::RenderSettings => {
                candidate.render = source.render.clone();
                let outputs: Vec<NodeId> = candidate
                    .shader_tree
                    .nodes
                    .iter()
                    .filter(|n| matches!(n.kind, NodeKind::RenderOutput(_)))
                    .map(|n| n.id)
                    .collect();
                if !outputs.is_empty() {
                    candidate.shader_tree.remove(&outputs)?;
                }
                let images = image_map(&mut candidate, &source.images)?;
                let new_outputs: Vec<NodeId> = roots
                    .iter()
                    .copied()
                    .filter(|&r| {
                        matches!(
                            source.shader_tree.get(r).map(|n| &n.kind),
                            Some(NodeKind::RenderOutput(_))
                        )
                    })
                    .collect();
                insert_nodes(
                    &mut candidate.shader_tree,
                    &source.shader_tree,
                    &new_outputs,
                    None,
                    0,
                    &BTreeMap::new(),
                    &images,
                )?;
                let cameras: Vec<Item> = source
                    .items
                    .iter()
                    .filter(|i| i.camera.is_some())
                    .cloned()
                    .collect();
                if !cameras.is_empty() {
                    insert_items(&mut candidate, &mut next, &cameras, &source.images)?;
                }
            }
        }
        candidate.shader_tree.validate()?;
        candidate.validate()?;
        Ok(candidate)
    }

    /// PB-110: a material preset dragged onto geometry. Polygons `faces` of
    /// mesh `item` (every polygon when empty) take a new material tag named
    /// after the preset, a Material Group for that tag goes on top of the
    /// shader tree, and the preset loads into it: one undo step.
    pub(crate) fn apply_material_preset(
        &mut self,
        preset: Preset,
        item: u64,
        faces: &[usize],
    ) -> Result<()> {
        if preset.header.kind != PresetKind::Material {
            return Err("Only a material preset can be dropped onto geometry".into());
        }
        if self.session.pen_draft.is_some() || self.i2_active() {
            return Err("Finish or Cancel the current tool before applying a material".into());
        }
        self.require_idle()?;
        let name = preset
            .scene
            .shader_tree
            .nodes
            .iter()
            .find(|n| n.parent.is_none())
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "Material".into());
        let mut base = self.scene.clone();
        base.shader_tree = base.render_shader_tree().into_owned();
        // PB-110 / PB1-Q02 (Jason, 2026-11-11): dropped on polygons that
        // share one material tag, the material goes into that tag's
        // material group, replacing the group's contents as loading into a
        // group does; the polygons keep their tag. With no group for the
        // tag yet, one is made for it (named after the tag). A second drop
        // replaces the first instead of stacking. Everything in the group
        // goes: a hand-made layer can't be told from a dropped one
        // (provisional, PB1-Q15).
        if let Some(tag) = shared_tag(&base, item, faces) {
            let group = match tag_group(&base, &tag) {
                Some(g) => g,
                None => {
                    let g = base.shader_tree.add(NewLayer::Group, None)?;
                    let mut node = base.shader_tree.get(g).ok_or("No group")?.clone();
                    node.name = tag.clone();
                    if let NodeKind::Group(m) = &mut node.kind {
                        m.tag_type = crate::shader_tree::TagType::Material;
                        m.tag = TagMask::Name(tag.clone());
                    }
                    base.shader_tree.replace(node)?;
                    g
                }
            };
            let candidate = self.preset_candidate(base, preset, Some(group))?;
            self.next_item_id = self.next_item_id.max(candidate.next_id);
            let before = std::mem::replace(&mut self.scene, candidate);
            self.record("Apply Material Preset".into(), before);
            return Ok(());
        }
        // Polygons with several tags (or none given: the whole mesh) get a
        // new tag named after the material, and its own group.
        let tag = crate::user_defaults::free_name(&name, |t| {
            base.shader_tree.nodes.iter().any(
                |n| matches!(&n.kind, NodeKind::Group(g) if g.tag == TagMask::Name(t.to_owned())),
            )
        });
        let target = base.item_mut(item).ok_or("Drop onto a mesh")?;
        if target.mesh.faces.is_empty() {
            return Err(format!("{} has no polygons", target.name));
        }
        let mut mesh = (*target.mesh).clone();
        for (k, face) in mesh.faces.iter_mut().enumerate() {
            if faces.is_empty() || faces.contains(&k) {
                face.material = tag.clone();
            }
        }
        target.mesh = mesh.into();
        let group = base.shader_tree.add(NewLayer::Group, None)?;
        let mut node = base.shader_tree.get(group).ok_or("No group")?.clone();
        node.name = tag.clone();
        if let NodeKind::Group(g) = &mut node.kind {
            g.tag_type = crate::shader_tree::TagType::Material;
            g.tag = TagMask::Name(tag.clone());
        }
        base.shader_tree.replace(node)?;
        let candidate = self.preset_candidate(base, preset, Some(group))?;
        self.next_item_id = self.next_item_id.max(candidate.next_id);
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record("Apply Material Preset".into(), before);
        Ok(())
    }

    /// CD-13 / EP §1: the picture at `path` becomes an environment's
    /// image, as one undo step. `item` names the environment; `None` is
    /// the top (newest) one, made when the scene has none (not the one the
    /// camera sees). An environment with an Image Map layer gets the new
    /// picture in its top one, keeping that layer's rotation, opacity and
    /// blend; otherwise a new Image Map layer (Spherical about Y, Opacity
    /// 100 %, Environment Color) goes directly above its Environment
    /// Material. The layer is named after the file. Other layers stay as
    /// they are.
    pub(crate) fn set_environment_image(
        &mut self,
        path: &str,
        width: u32,
        height: u32,
        item: Option<u64>,
    ) -> Result<()> {
        self.require_idle()?;
        let mut candidate = self.scene.clone();
        candidate.next_id = candidate.next_id.max(self.next_item_id);
        candidate.put_environment_image(path, width, height, item)?;
        candidate.validate()?;
        if candidate == self.scene {
            return Ok(());
        }
        self.next_item_id = self.next_item_id.max(candidate.next_id);
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record("Set Environment Image".into(), before);
        Ok(())
    }
}

#[cfg(test)]
mod tests;

/// The material tag every polygon in `faces` of `item` shares, if they
/// share one (and `faces` isn't empty).
fn shared_tag(scene: &Scene, item: u64, faces: &[usize]) -> Option<String> {
    let mesh = &scene.item(item)?.mesh;
    let first = &mesh.faces.get(*faces.first()?)?.material;
    faces
        .iter()
        .all(|&f| mesh.faces.get(f).is_some_and(|p| &p.material == first))
        .then(|| first.clone())
}

/// The top-level material group masking `tag`.
fn tag_group(scene: &Scene, tag: &str) -> Option<NodeId> {
    let mask = TagMask::Name(tag.to_owned());
    scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| {
            n.parent.is_none()
                && matches!(&n.kind, NodeKind::Group(g)
                    if g.tag_type == crate::shader_tree::TagType::Material && g.tag == mask)
        })
        .map(|n| n.id)
}

impl Scene {
    /// Set Environment Image without an undo step: environment `item` (or
    /// the first, made when the scene has none) shows picture `path` in
    /// its image layer, added above its material when it has none.
    /// Returns the environment and the layer.
    pub fn put_environment_image(
        &mut self,
        path: &str,
        width: u32,
        height: u32,
        item: Option<u64>,
    ) -> Result<(u64, NodeId)> {
        use crate::environment::{EnvironmentLayerEdit, LayerPlace};
        use crate::shader_tree::TextureSource;
        let env_id = match item {
            Some(id) => {
                self.items
                    .iter()
                    .find(|i| i.id == id && i.environment.is_some())
                    .ok_or("Select an environment")?;
                id
            }
            None => {
                self.ensure_environment()?;
                self.environments()
                    .next()
                    .map(|i| i.id)
                    .ok_or("The scene has no environment")?
            }
        };
        let image = self.image_for_path(path, width, height)?;
        let mut layer = match self.environment_image_layer(env_id) {
            Some(layer) => layer.clone(),
            None => {
                self.edit_environment_layers(
                    env_id,
                    EnvironmentLayerEdit::Add {
                        layer: NewLayer::ImageMap,
                        at: LayerPlace::AboveMaterial,
                    },
                )?;
                self.environment_image_layer(env_id)
                    .cloned()
                    .ok_or("The image layer wasn't added")?
            }
        };
        if let NodeKind::Texture(t) = &mut layer.kind
            && let TextureSource::ImageMap { image: slot, .. } = &mut t.source
        {
            *slot = Some(image);
        }
        layer.name = std::path::Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or(layer.name);
        let id = layer.id;
        self.edit_environment_layers(env_id, EnvironmentLayerEdit::Replace(Box::new(layer)))?;
        Ok((env_id, id))
    }
}
