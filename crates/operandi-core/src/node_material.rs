//! The Node Material layer in the shader tree (Nodes N1, NM-10..17).
//!
//! A Node Material points at a graph in the tree's Nodes section
//! (node_graph.rs) and outputs a finished surface. The layer stack below it
//! still folds as before; the renderer mixes the graph's surface over it by
//! the layer's coverage (opacity × masks, NM-11/NM-12). Texture layers above
//! reach inside only through the graph's exposed inputs (NM-13).
use crate::node_graph::{Exposed, ExposedValues, StoredGraph, ValueKind};
use crate::shader_tree::{BlendMode, NodeId, NodeKind, NodeMaterialLayer, ShaderTree};

impl ShaderTree {
    /// The graph a Node Material layer uses.
    pub fn node_material_graph(&self, layer: NodeId) -> Option<&StoredGraph> {
        match &self.get(layer)?.kind {
            NodeKind::NodeMaterial(n) => self.node_graphs.get(n.graph?),
            _ => None,
        }
    }

    /// The value exposed input `e` has on layer `n`: its override, else the
    /// graph's (shared) default (NM-04).
    pub fn exposed_value(n: &NodeMaterialLayer, e: &Exposed) -> Vec<f64> {
        n.overrides
            .get(&e.name)
            .map(|v| e.value_kind().fit(v))
            .unwrap_or_else(|| e.default_value())
    }

    /// The exposed values of layer `n` with drivers from layers above.
    pub fn exposed_values(
        &self,
        n: &NodeMaterialLayer,
        drivers: &std::collections::BTreeMap<String, usize>,
    ) -> ExposedValues {
        let mut out = ExposedValues {
            drivers: drivers.clone(),
            ..Default::default()
        };
        if let Some(g) = n.graph.and_then(|id| self.node_graphs.get(id)) {
            for e in &g.material.exposed {
                out.values.insert(e.name.clone(), Self::exposed_value(n, e));
            }
        }
        out
    }

    /// The exposed inputs a texture layer may drive (NM-13, the Effect
    /// menu's "Node Inputs" heading): those of the Node Materials below it,
    /// among its siblings and below each enclosing layer.
    pub fn node_inputs_for(&self, layer: NodeId) -> Vec<Exposed> {
        let mut out: Vec<Exposed> = Vec::new();
        let mut at = Some(layer);
        for _ in 0..=self.nodes.len() {
            let Some(id) = at else { break };
            let Some(node) = self.get(id) else { break };
            let siblings: Vec<_> = self.children(node.parent).collect();
            let below = siblings
                .iter()
                .position(|n| n.id == id)
                .map_or(&siblings[..0], |k| &siblings[k + 1..]);
            for n in below {
                for d in self
                    .nodes
                    .iter()
                    .filter(|d| self.is_within(d.id, n.id) && d.layer.enable)
                {
                    if let NodeKind::NodeMaterial(m) = &d.kind
                        && let Some(g) = m.graph.and_then(|g| self.node_graphs.get(g))
                    {
                        for e in &g.material.exposed {
                            if !out.iter().any(|o| o.name == e.name) {
                                out.push(e.clone());
                            }
                        }
                    }
                }
            }
            at = node.parent;
        }
        out
    }

    /// Adds imported graphs to the Nodes section, each with a Node Material
    /// layer named after it, placed as Add Layer places a layer. Returns
    /// the new layers.
    pub fn import_node_graphs(
        &mut self,
        graphs: Vec<StoredGraph>,
        selected: Option<NodeId>,
    ) -> crate::Result<Vec<NodeId>> {
        if graphs.is_empty() {
            return Err("Nothing to import".into());
        }
        let mut layers = Vec::new();
        for mut g in graphs {
            g.id = self.node_graphs.next_id();
            let name = g.material.name.clone();
            let graph = g.id;
            self.node_graphs.graphs.push(g);
            let id = self.add(crate::shader_tree::NewLayer::NodeMaterial, selected)?;
            let i = self.index(id).expect("just added");
            let node = &mut self.nodes[i];
            if !name.trim().is_empty() && !name.chars().any(char::is_control) {
                node.name = String::new();
                let unique = self.unique_name(&name);
                self.nodes[i].name = unique;
            }
            if let NodeKind::NodeMaterial(n) = &mut self.nodes[i].kind {
                n.graph = Some(graph);
            }
            layers.push(id);
        }
        Ok(layers)
    }

    /// The id `g` has in this tree's Nodes section: an identical graph's,
    /// else a new entry's.
    pub fn adopt_node_graph(&mut self, g: &StoredGraph) -> u64 {
        if let Some(same) = self.node_graphs.graphs.iter().find(|x| {
            x.material == g.material && x.groups == g.groups && x.materialx == g.materialx
        }) {
            return same.id;
        }
        let mut copy = g.clone();
        copy.id = self.node_graphs.next_id();
        let id = copy.id;
        self.node_graphs.graphs.push(copy);
        id
    }

    /// NM-40, for a graph the viewport can't draw from its nodes
    /// (`graph_view`): the colour a Node Material shows there, the base
    /// colour of the principled surface
    /// feeding its output when that is a plain value (an Emission's or a
    /// glass's colour likewise), else mid grey.
    pub fn node_viewport_colour(&self, n: &crate::shader_eval::NodeUse) -> [f64; 3] {
        use crate::node_graph::FlatValue;
        const GREY: [f64; 3] = [0.5; 3];
        let Some(g) = self.node_graphs.get(n.graph) else {
            return GREY;
        };
        let values = ExposedValues {
            values: n.values.clone(),
            ..Default::default()
        };
        let Ok(flat) = g.flatten(&values) else {
            return GREY;
        };
        let Some((node, _)) = flat.feeding("Surface") else {
            return GREY;
        };
        let socket = match flat.nodes[node].cycles {
            "principled_bsdf" => "Base Color",
            "emission" | "glass_bsdf" | "transparent_bsdf" => "Color",
            _ => return GREY,
        };
        if flat.is_linked(node, socket) {
            return GREY;
        }
        match flat.value(node, socket) {
            Some(FlatValue::Vec3(c)) => *c,
            // Cycles' own default when the file gave none.
            None if socket == "Base Color" => [0.8; 3],
            _ => GREY,
        }
    }

    /// Sets a graph's exposed default (NM-04).
    pub fn set_node_graph_default(
        &mut self,
        graph: u64,
        input: &str,
        value: Vec<f64>,
    ) -> crate::Result<()> {
        let g = self
            .node_graphs
            .get_mut(graph)
            .ok_or("That node graph isn't in the scene")?;
        let e = g
            .material
            .exposed
            .iter_mut()
            .find(|e| e.name == input)
            .ok_or_else(|| format!("The graph has no input “{input}”"))?;
        let v = check_value(e, &value)?;
        e.default = match e.value_kind() {
            ValueKind::Float => serde_json::json!(v[0]),
            // RGBA keeps an alpha of 1, as the file writes it.
            ValueKind::Color => serde_json::json!([v[0], v[1], v[2], 1.0]),
            ValueKind::Vector => serde_json::json!([v[0], v[1], v[2]]),
        };
        Ok(())
    }

    /// Node Material layers point at graphs that exist, override only their
    /// exposed inputs with finite values, and blend Normal (NM-12: the
    /// other modes come later).
    pub(crate) fn validate_node_materials(&self) -> crate::Result<()> {
        self.node_graphs.validate()?;
        for n in &self.nodes {
            if let NodeKind::NodeMaterial(m) = &n.kind {
                if n.layer.blend != BlendMode::Normal {
                    return Err(format!(
                        "{}: Node Materials blend Normal only for now",
                        n.name
                    ));
                }
                let Some(id) = m.graph else {
                    if !m.overrides.is_empty() {
                        return Err(format!("{}: values for a missing graph", n.name));
                    }
                    continue;
                };
                let g = self
                    .node_graphs
                    .get(id)
                    .ok_or_else(|| format!("{}: its node graph is missing", n.name))?;
                for (name, v) in &m.overrides {
                    let e = g
                        .exposed(name)
                        .ok_or_else(|| format!("{}: the graph has no input “{name}”", n.name))?;
                    check_value(e, v).map_err(|err| format!("{}: {err}", n.name))?;
                }
            }
            if let NodeKind::Texture(t) = &n.kind
                && let Some(name) = &t.node_input
                && (name.trim().is_empty() || name.len() > 1024)
            {
                return Err(format!("{}: invalid node input", n.name));
            }
        }
        Ok(())
    }
}

/// `value` checked against exposed input `e`: the right count of finite
/// numbers within its min and max.
pub fn check_value(e: &Exposed, value: &[f64]) -> crate::Result<Vec<f64>> {
    let kind = e.value_kind();
    if value.len() != kind.count() || value.iter().any(|v| !v.is_finite()) {
        return Err(format!("{}: needs {} number(s)", e.name, kind.count()));
    }
    if kind == ValueKind::Float {
        if let Some(min) = e.min
            && value[0] < min
        {
            return Err(format!("{} must be at least {min}", e.name));
        }
        if let Some(max) = e.max
            && value[0] > max
        {
            return Err(format!("{} must be at most {max}", e.name));
        }
    }
    Ok(value.to_vec())
}

#[cfg(test)]
#[path = "node_material_tests.rs"]
mod tests;

impl crate::Scene {
    /// Puts node graph `graph` in the Material Group of polygon tag `tag`
    /// as a Node Material layer, replacing what the group held (and the
    /// Texture Locators only that used); a group is made, where Add Layer
    /// puts one, when the scene has none for the tag. Importers use it
    /// (USD's graphs sidecar); nothing is recorded. Returns the layer.
    pub fn put_node_graph_for_tag(
        &mut self,
        tag: &str,
        graph: crate::node_graph::StoredGraph,
    ) -> crate::Result<NodeId> {
        use crate::shader_tree::{NewLayer, NodeKind, TagMask, TagType};
        let mask = TagMask::Name(tag.to_owned());
        let found = self
            .shader_tree
            .nodes
            .iter()
            .find(|n| {
                n.parent.is_none()
                    && matches!(&n.kind, NodeKind::Group(g)
                        if g.tag_type == TagType::Material && g.tag == mask)
            })
            .map(|n| n.id);
        let group = match found {
            Some(g) => g,
            None => {
                let g = self.shader_tree.add(NewLayer::Group, None)?;
                let node = self
                    .shader_tree
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == g)
                    .ok_or("The new group went missing")?;
                if let NodeKind::Group(m) = &mut node.kind {
                    m.tag_type = TagType::Material;
                    m.tag = mask;
                }
                node.name = tag.to_owned();
                g
            }
        };
        let old: Vec<NodeId> = self
            .shader_tree
            .children(Some(group))
            .map(|n| n.id)
            .collect();
        if !old.is_empty() {
            let locators = self.shader_tree.locators_in(&old);
            self.shader_tree.remove(&old)?;
            self.drop_unused_locators(&locators);
        }
        let layers = self
            .shader_tree
            .import_node_graphs(vec![graph], Some(group))?;
        let layer = *layers.first().ok_or("The node graph wasn't added")?;
        // "<tag> Node Material", as the MaterialX reader's "<tag> Material".
        let taken: std::collections::BTreeSet<String> = self
            .shader_tree
            .nodes
            .iter()
            .filter(|n| n.id != layer)
            .map(|n| n.name.clone())
            .collect();
        let name =
            crate::user_defaults::free_name(&format!("{tag} Node Material"), |n| taken.contains(n));
        if let Some(node) = self.shader_tree.nodes.iter_mut().find(|n| n.id == layer) {
            node.name = name;
        }
        Ok(layer)
    }
}
