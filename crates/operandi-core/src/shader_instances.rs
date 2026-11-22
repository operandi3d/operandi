//! Shader layer instances (shader layer menu v0.1 §3): a linked clone of
//! a layer. Linking is per setting and per colour component: every
//! setting the instance hasn't overridden follows its source; editing a
//! setting on the instance makes it a local override for that setting
//! only.
//!
//! How it is kept: an instance is an ordinary layer whose settings always
//! hold its effective values (the source's, with its overrides applied),
//! so rendering, the viewport and presets read it like any layer. Its
//! `extras.instance` names the source and the overridden settings as JSON
//! pointers into the layer's settings ("/layer/opacity",
//! "/kind/Texture/source/Constant/color/1"). After every shader tree edit
//! `sync_instances` turns changes made on an instance into overrides and
//! copies the source's values into every setting that isn't overridden.
//!
//! A setting is a leaf of the settings' JSON: a number, flag, name or
//! empty value, or one component of a list of plain values (a colour's
//! R, G, B). Lists of anything else (gradient keys) are one setting. The
//! eye (Enable) is the row's own and never linked (provisional, SLM-Q06).
use crate::Result;
use crate::shader_tree::{Channel, Layer, Node, NodeId, NodeKind, ShaderTree};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// An instance's link to its source.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceLink {
    pub source: NodeId,
    /// JSON pointers of the overridden settings.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub overrides: BTreeSet<String>,
}

/// The linked part of a layer: everything but its name, place, eye and
/// menu state.
#[derive(Serialize, Deserialize)]
struct Settings {
    layer: Layer,
    kind: NodeKind,
}

/// Layer-block fields that are the row's own and never linked: the eye
/// and the editor colour.
const UNLINKED: [&str; 2] = ["enable", "editor_color"];

fn settings_json(node: &Node) -> Value {
    let mut v = serde_json::to_value(Settings {
        layer: node.layer.clone(),
        kind: node.kind.clone(),
    })
    .unwrap_or(Value::Null);
    if let Some(layer) = v.get_mut("layer").and_then(Value::as_object_mut) {
        for k in UNLINKED {
            layer.remove(k);
        }
    }
    v
}

/// Write `v` back into `node`, keeping its own unlinked fields.
fn set_settings(node: &mut Node, mut v: Value) -> Result<()> {
    let own = serde_json::to_value(&node.layer).unwrap_or(Value::Null);
    if let Some(layer) = v.get_mut("layer").and_then(Value::as_object_mut) {
        for k in UNLINKED {
            if let Some(x) = own.get(k) {
                layer.insert(k.into(), x.clone());
            }
        }
    }
    let s: Settings =
        serde_json::from_value(v).map_err(|e| format!("Instance settings don't fit: {e}"))?;
    node.layer = s.layer;
    node.kind = s.kind;
    Ok(())
}

fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn is_plain(v: &Value) -> bool {
    !matches!(v, Value::Array(_) | Value::Object(_))
}

/// The settings that differ between `a` and `b`, as pointers.
fn diff(a: &Value, b: &Value, path: &str, out: &mut BTreeSet<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let keys: BTreeSet<&String> = x.keys().chain(y.keys()).collect();
            for k in keys {
                let p = format!("{path}/{}", escape(k));
                diff(
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                    &p,
                    out,
                );
            }
        }
        (Value::Array(x), Value::Array(y))
            if x.len() == y.len() && x.iter().chain(y).all(is_plain) =>
        {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                if p != q {
                    out.insert(format!("{path}/{i}"));
                }
            }
        }
        _ if a != b => {
            out.insert(path.to_owned());
        }
        _ => {}
    }
}

/// Whether `inner` is `outer` or lies inside it.
fn within(inner: &str, outer: &str) -> bool {
    inner == outer
        || inner
            .strip_prefix(outer)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// The instance's effective settings: `source`'s with `instance`'s own
/// values at `overrides`. Overrides the source no longer has (its type
/// changed) are dropped.
fn effective(source: &Node, instance: &Node, overrides: &mut BTreeSet<String>) -> Value {
    let mut out = settings_json(source);
    let own = settings_json(instance);
    overrides.retain(|p| {
        let Some(v) = own.pointer(p) else {
            return false;
        };
        match out.pointer_mut(p) {
            Some(slot) => {
                *slot = v.clone();
                true
            }
            None => false,
        }
    });
    out
}

impl ShaderTree {
    /// The source of instance `id`, if it is one.
    pub fn instance_source(&self, id: NodeId) -> Option<NodeId> {
        self.get(id)?.extras.instance.as_ref().map(|l| l.source)
    }
    pub fn is_instance(&self, id: NodeId) -> bool {
        self.instance_source(id).is_some()
    }
    /// Every instance of `source`, in panel order.
    pub fn instances_of(&self, source: NodeId) -> Vec<NodeId> {
        self.rows(&|_| false)
            .into_iter()
            .filter(|(n, _)| {
                n.extras
                    .instance
                    .as_ref()
                    .is_some_and(|l| l.source == source)
            })
            .map(|(n, _)| n.id)
            .collect()
    }

    /// Make the instances agree with their sources after an edit:
    /// settings changed on an instance (against `before`) become
    /// overrides, then every setting that isn't overridden takes the
    /// source's value.
    pub(crate) fn sync_instances(&mut self, before: Option<&ShaderTree>) -> Result<()> {
        if !self.nodes.iter().any(|n| n.extras.instance.is_some()) {
            return Ok(());
        }
        let ids: Vec<NodeId> = self
            .nodes
            .iter()
            .filter(|n| n.extras.instance.is_some())
            .map(|n| n.id)
            .collect();
        let mut updates: BTreeMap<NodeId, Node> = BTreeMap::new();
        for id in ids {
            let node = self.get(id).expect("listed").clone();
            let mut link = node.extras.instance.clone().expect("listed");
            // An edit that set the link itself (Create Instance, Revert to
            // Source) has no changes of the user's to keep.
            if let Some(old) = before.and_then(|t| t.get(id))
                && old.extras.instance.as_ref() == Some(&link)
            {
                diff(
                    &settings_json(old),
                    &settings_json(&node),
                    "",
                    &mut link.overrides,
                );
            }
            let source = self
                .get(link.source)
                .ok_or_else(|| format!("{}: its source layer is gone", node.name))?;
            if source.extras.instance.is_some() {
                return Err(format!(
                    "{}: an instance's source can't be an instance",
                    node.name
                ));
            }
            let value = effective(source, &node, &mut link.overrides);
            let mut synced = node.clone();
            set_settings(&mut synced, value)?;
            synced.extras.instance = Some(link);
            updates.insert(id, synced);
        }
        for node in &mut self.nodes {
            if let Some(u) = updates.remove(&node.id) {
                *node = u;
            }
        }
        Ok(())
    }

    /// Create Instance (§3): a linked clone of each top-most layer of
    /// `ids` directly above it, named by the Duplicate rule. An instance of
    /// an instance links to its source. Groups and Folders can't be
    /// instanced (provisional, SLM-Q07); nor can Render Outputs.
    pub(crate) fn create_instances(&mut self, ids: &[NodeId]) -> Result<Vec<NodeId>> {
        let roots = self.top_most(ids);
        if roots.is_empty() {
            return Err("Select shader tree layers to instance".into());
        }
        let mut out = Vec::new();
        let mut taken = Vec::new();
        for root in roots {
            let node = self.get(root).expect("listed").clone();
            if !can_instance(&node) {
                return Err(format!("{} can't be instanced", node.type_label()));
            }
            let source_id = node.extras.instance.as_ref().map_or(root, |l| l.source);
            let source = self
                .get(source_id)
                .ok_or("Missing instance source")?
                .clone();
            if self.nodes.len() >= 65_536 || self.next_id >= u64::MAX - 1 {
                return Err("Shader tree is full".into());
            }
            let id = self.next_id;
            self.next_id += 1;
            let name = self.copy_name(&source.name, &taken);
            taken.push(name.clone());
            let instance = Node {
                id,
                parent: node.parent,
                name,
                layer: crate::shader_tree::Layer {
                    enable: true,
                    editor_color: None,
                    ..source.layer.clone()
                },
                kind: source.kind.clone(),
                extras: crate::shader_layer_menu::LayerExtras {
                    instance: Some(InstanceLink {
                        source: source_id,
                        overrides: Default::default(),
                    }),
                    ..Default::default()
                },
            };
            let at = self.index(root).expect("listed");
            self.nodes.insert(at, instance);
            out.push(id);
        }
        Ok(out)
    }

    /// Revert to Source: drop instance `id`'s overrides at or inside
    /// `paths` (all of them for `None`).
    pub(crate) fn revert_overrides(&mut self, id: NodeId, paths: Option<&[String]>) -> Result<()> {
        let i = self.index(id).ok_or("Shader tree layer no longer exists")?;
        let link = self.nodes[i]
            .extras
            .instance
            .as_mut()
            .ok_or("Only an instance has settings to revert")?;
        match paths {
            None => link.overrides.clear(),
            Some(paths) => link
                .overrides
                .retain(|o| !paths.iter().any(|p| within(o, p) || within(p, o))),
        }
        self.sync_instances(None)
    }

    /// Layers removed with `ids`: an instance's source going takes its
    /// instances with it (§3).
    pub(crate) fn with_instances(&self, ids: &[NodeId]) -> Vec<NodeId> {
        let mut out = ids.to_vec();
        for n in &self.nodes {
            if let Some(l) = &n.extras.instance
                && ids.iter().any(|&id| self.is_within(l.source, id))
                && !out.contains(&n.id)
            {
                out.push(n.id);
            }
        }
        out
    }

    /// How many instances deleting `ids` also deletes (not counting ones
    /// inside `ids` already), for the delete confirmation.
    pub fn instances_deleted_with(&self, ids: &[NodeId]) -> usize {
        self.with_instances(ids)
            .into_iter()
            .filter(|&n| !ids.iter().any(|&id| self.is_within(n, id)))
            .count()
    }
}

/// Whether Create Instance applies to `node`.
pub fn can_instance(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::Material(_) | NodeKind::Shader(_) | NodeKind::Texture(_)
    )
}

/// What the instance's settings are, as a source would give them: its
/// source's values (for Duplicate and Copy of an instance, §2).
pub(crate) fn as_plain_copy(tree: &ShaderTree, node: &Node) -> Node {
    let mut copy = node.clone();
    if let Some(link) = &node.extras.instance {
        if let Some(source) = tree.get(link.source) {
            copy.layer = Layer {
                enable: node.layer.enable,
                editor_color: node.layer.editor_color,
                ..source.layer.clone()
            };
            copy.kind = source.kind.clone();
        }
        copy.extras.instance = None;
    }
    copy
}

/// The Properties labels of `node`'s overridden settings, and the paths
/// each stands for, for the panel's marker and Revert to Source. Paths no
/// label covers are listed under "".
pub fn overridden_labels(node: &Node) -> Vec<(&'static str, Vec<String>)> {
    let Some(link) = &node.extras.instance else {
        return Vec::new();
    };
    let mut groups: Vec<(&'static str, Vec<String>)> = label_paths(node);
    let mut out: Vec<(&'static str, Vec<String>)> = Vec::new();
    let mut covered = BTreeSet::new();
    for (label, paths) in groups.drain(..) {
        let hit: Vec<String> = link
            .overrides
            .iter()
            .filter(|o| paths.iter().any(|p| within(o, p) || within(p, o)))
            .cloned()
            .collect();
        if !hit.is_empty() {
            covered.extend(hit.iter().cloned());
            out.push((label, paths));
        }
    }
    let rest: Vec<String> = link
        .overrides
        .iter()
        .filter(|o| !covered.contains(*o))
        .cloned()
        .collect();
    if !rest.is_empty() {
        out.push(("", rest));
    }
    out
}

/// Every Properties label of `node` with the settings it edits.
fn label_paths(node: &Node) -> Vec<(&'static str, Vec<String>)> {
    let mut out: Vec<(&'static str, Vec<String>)> = vec![
        ("Invert", vec!["/layer/invert".into()]),
        ("Blend Mode", vec!["/layer/blend".into()]),
        ("Opacity", vec!["/layer/opacity".into()]),
    ];
    let base = settings_json(node);
    // Effect, wherever the type keeps it.
    if let Some(kind) = base.get("kind").and_then(Value::as_object) {
        for (variant, v) in kind {
            if v.get("effect").is_some() {
                out.push(("Effect", vec![format!("/kind/{}/effect", escape(variant))]));
            }
        }
    }
    let count = node.clone().channels().len();
    for i in 0..count {
        let mut copy = node.clone();
        let name = {
            let mut channels = copy.channels();
            let (name, channel) = &mut channels[i];
            perturb(channel);
            *name
        };
        let mut paths = BTreeSet::new();
        diff(&base, &settings_json(&copy), "", &mut paths);
        if !paths.is_empty() {
            out.push((name, paths.into_iter().collect()));
        }
    }
    out
}

/// Change a channel's value to something it can't already be.
fn perturb(channel: &mut Channel<'_>) {
    const OFF: f64 = 12_345.678;
    match channel {
        Channel::Percent(v)
        | Channel::PercentIn(v, ..)
        | Channel::Distance(v)
        | Channel::Angle(v)
        | Channel::Number(v, ..) => **v += OFF,
        Channel::Color(c) => {
            for v in c.iter_mut() {
                *v += OFF;
            }
        }
        Channel::Bool(b) => **b = !**b,
    }
}
