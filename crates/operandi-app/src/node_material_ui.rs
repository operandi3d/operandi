//! Node Materials in the Shading tab (Nodes N1, NM-05, NM-14, NM-15):
//! Import Node Material…, the layer's and the shared graph's Properties,
//! and the Node View panel. The panel is a registry panel (UIC-157), so it
//! docks, tabs, floats and saves with the layout like any other (UIC-71..73);
//! it follows the Shader Tree's selected material. It shows MaterialX graph
//! layers and, from a Material Group's Show as Nodes, the group's own
//! MaterialX graph, live (node editor slice 1, UIC-244: the one graph
//! renders read, never a copy). The graphs and every
//! edit live in `operandi_core::node_graph` / `node_material`; this file
//! draws them and turns clicks into `Command::ShaderTree` edits.
use super::*;
use operandi_core::node_graph::{Exposed, GraphNode, StoredGraph, ValueKind};
use operandi_core::shader_tree::{Node, NodeId, NodeKind, ShaderTree, ShaderTreeEdit};

/// What the Node View panel shows (NM-15). Not part of the document.
#[derive(Default)]
pub(crate) struct GraphWindow {
    /// The graph shown, and the layer it was opened from (whose overrides
    /// the values show).
    pub graph: Option<u64>,
    pub layer: Option<NodeId>,
    /// A Material Group shown as nodes instead of a stored graph.
    pub group: Option<NodeId>,
    /// The group's view, and the editor revision it was made at.
    pub(crate) group_view: Option<(u64, NodeId, std::result::Result<StoredGraph, String>)>,
    pub pan: Vec2,
    pub zoom: f32,
    /// The selected node (index into the material's nodes).
    pub selected: Option<usize>,
    /// The Shader Tree selection the view last followed: a new selection
    /// moves the view to its material (Node View follows the selection).
    pub(crate) followed: Option<NodeId>,
    /// Bring the Node View panel to the front (or dock it) after this
    /// frame's panels drew: the dock tree is out of `views` while they draw.
    pub(crate) reveal: bool,
    /// Editing a Material Group's node view (node_edit_ui.rs).
    pub(crate) board: crate::node_edit_ui::BoardState,
}

impl Operandi {
    /// Add Layer ▸ Import Node Material…: a node graph file (NM-05).
    pub(super) fn request_import_node_material(&mut self) {
        let selected = self.shading.selected.last().copied();
        // Where the last imported graph's pictures are, else home.
        let folder = self
            .editor
            .scene
            .images
            .last()
            .and_then(|i| {
                std::path::Path::new(&i.path)
                    .parent()
                    .map(|p| p.to_path_buf())
            })
            .or_else(|| std::env::var_os("HOME").map(std::path::PathBuf::from))
            .unwrap_or_default();
        if !self.choose(
            crate::file_chooser::Choose::NodeGraph { folder },
            crate::file_chooser::Purpose::ImportNodeGraph(selected),
        ) {
            self.status = "No file panel is available to choose a node graph".into();
        }
    }

    /// Imports the graphs in `path` (all or nothing, NM-T4), each as a Node
    /// Material layer placed next to `selected`.
    pub(super) fn import_node_graph_from(
        &mut self,
        path: &std::path::Path,
        selected: Option<NodeId>,
    ) {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                self.status = format!("Couldn't read {}: {e}", path.display());
                return;
            }
        };
        let (mut graphs, refused) =
            match operandi_core::node_graph::import_report(&text, path.parent()) {
                Ok(g) => g,
                Err(e) => {
                    self.status = e;
                    return;
                }
            };
        // The graphs' pictures are copied into the active project
        // (project_files.rs); packed ones live in the scene.
        for g in &mut graphs {
            let groups = g.groups.iter_mut().flat_map(|x| x.nodes.iter_mut());
            for n in g.material.nodes.iter_mut().chain(groups) {
                if let Some(img) = n.image.as_mut().filter(|i| !i.packed && !i.path.is_empty()) {
                    img.path = self
                        .adopt_file(
                            crate::project_files::SOURCE_IMAGES,
                            std::path::Path::new(&img.path),
                        )
                        .to_string_lossy()
                        .into_owned();
                }
            }
        }
        // Clips list the graphs' pictures (NM-21); a missing one keeps its
        // path and renders with the missing-image look.
        let mut images = Vec::new();
        let mut missing = Vec::new();
        for g in &graphs {
            for n in g
                .material
                .nodes
                .iter()
                .chain(g.groups.iter().flat_map(|x| &x.nodes))
            {
                if let Some(img) = &n.image {
                    match crate::backdrop_ui::decode(std::path::Path::new(&img.path)) {
                        Ok((_, (w, h))) => images.push((img.path.clone(), w, h)),
                        Err(_) => missing.push(img.path.clone()),
                    }
                }
            }
        }
        let first = self.editor.scene.shader_tree.next_id;
        let count = graphs.len();
        if self.run_ok(Command::ShaderTree(ShaderTreeEdit::ImportNodeGraphs {
            graphs,
            images,
            selected,
        })) {
            self.select_new_layers(first);
            let refused = if refused.is_empty() {
                String::new()
            } else {
                format!(" {}", refused.join(" "))
            };
            self.status = if missing.is_empty() {
                format!(
                    "Imported {count} Node Material{}",
                    if count == 1 { "" } else { "s" }
                )
            } else {
                format!(
                    "Imported {count} Node Material{}; {} picture{} missing: {}",
                    if count == 1 { "" } else { "s" },
                    missing.len(),
                    if missing.len() == 1 { " is" } else { "s are" },
                    missing.join(", ")
                )
            } + &refused;
        }
    }

    /// The Node Material layer's own rows (NM-14): its graph, then one
    /// control per exposed input. Edits go into `edited` (overrides).
    /// True while a drag continues.
    pub(super) fn node_material_rows(&mut self, ui: &mut Ui, edited: &mut Node) -> bool {
        let NodeKind::NodeMaterial(m) = &mut edited.kind else {
            return false;
        };
        let graphs: Vec<(u64, String)> = self
            .editor
            .scene
            .shader_tree
            .node_graphs
            .graphs
            .iter()
            .map(|g| (g.id, g.material.name.clone()))
            .collect();
        let shown = m.graph.map_or("(none)".to_owned(), |id| {
            graphs
                .iter()
                .find(|(g, _)| *g == id)
                .map_or("(missing)".to_owned(), |(_, n)| n.clone())
        });
        let mut import = false;
        let before = m.graph;
        crate::option_row::field_row(ui, "Graph", |ui| {
            ComboBox::from_id_salt("node-material-graph")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut m.graph, None, "(none)");
                    for (id, name) in &graphs {
                        ui.selectable_value(&mut m.graph, Some(*id), name);
                    }
                    ui.separator();
                    if ui.button("Import Node Material…").clicked() {
                        import = true;
                    }
                });
        });
        if m.graph != before {
            // Another graph's inputs: this layer's values don't carry over.
            m.overrides.clear();
        }
        if import {
            self.request_import_node_material();
        }
        let tree = &self.editor.scene.shader_tree;
        let Some(graph) = m.graph.and_then(|g| tree.node_graphs.get(g)).cloned() else {
            ui.weak("Pick a graph from the Nodes section, or import one");
            return false;
        };
        let unsupported = graph.unsupported();
        if let Some((node, idname)) = unsupported.first() {
            ui.colored_label(
                theme::error(),
                format!("Node “{node}” ({idname}) can't render here; the layers below show"),
            );
        }
        let mut continuing = false;
        if graph.material.exposed.is_empty() && graph.materialx.is_none() {
            ui.weak("This graph has no exposed inputs");
        }
        let mut reset: Option<String> = None;
        for e in &graph.material.exposed {
            let mut value = ShaderTree::exposed_value(m, e);
            let shared = !m.overrides.contains_key(&e.name);
            let (drag, changed) = exposed_field(ui, e, &mut value, shared);
            continuing |= drag;
            if changed {
                m.overrides.insert(e.name.clone(), value);
            }
            if crate::form::rows::take_reset() {
                reset = Some(e.name.clone());
            }
        }
        if let Some(name) = reset {
            m.overrides.remove(&name);
        }
        ui.horizontal(|ui| {
            if graph.materialx.is_some() {
                ui.weak("MaterialX graph: renders in Cycles; the viewport shows its base colour");
                return;
            }
            if ui
                .button("Show Graph")
                .on_hover_text("Open the graph read-only in the Node View panel")
                .clicked()
            {
                self.open_graph_window(graph.id, Some(edited.id));
            }
        });
        continuing
    }

    /// Properties for a graph row in the Nodes section (NM-04): its shared
    /// defaults, what uses it and where it came from.
    pub(super) fn node_graph_properties(&mut self, ui: &mut Ui, id: u64) {
        let tree = &self.editor.scene.shader_tree;
        let Some(graph) = tree.node_graphs.get(id).cloned() else {
            ui.weak("Select a layer in the shader tree");
            return;
        };
        ui.strong("Node Graph");
        ui.label(&graph.material.name);
        let users = tree
            .nodes
            .iter()
            .filter(|n| matches!(&n.kind, NodeKind::NodeMaterial(m) if m.graph == Some(id)))
            .count();
        let count = graph
            .materialx
            .as_ref()
            .map_or(graph.material.nodes.len(), |m| m.node_count());
        ui.weak(format!(
            "{}{count} node{} · used by {users} layer{}",
            if graph.materialx.is_some() {
                "MaterialX graph · "
            } else {
                ""
            },
            if count == 1 { "" } else { "s" },
            if users == 1 { "" } else { "s" }
        ));
        if let Some(text) = source_line(&graph) {
            ui.weak(text);
        }
        if !graph.material.exposed.is_empty() {
            ui.add_space(4.);
            ui.weak("Shared defaults: every layer that doesn't set its own value follows these");
        }
        for e in &graph.material.exposed {
            let mut value = e.default_value();
            let (drag, changed) = exposed_field(ui, e, &mut value, false);
            if changed {
                let continuing = drag && self.shading.graph_drag == Some(id);
                self.shading.graph_drag = drag.then_some(id);
                self.run(Command::ShaderTree(ShaderTreeEdit::SetNodeGraphDefault {
                    graph: id,
                    input: e.name.clone(),
                    value,
                    continuing,
                }));
            }
            let _ = crate::form::rows::take_reset();
        }
        // The read-only view draws Blender-shaped and MaterialX graphs
        // (node editor slice 1).
        if ui
            .button("Show Graph")
            .on_hover_text("Open the graph read-only in its own window")
            .clicked()
        {
            self.open_graph_window(id, None);
        }
    }

    pub(super) fn open_graph_window(&mut self, graph: u64, layer: Option<NodeId>) {
        let w = &mut self.graph_window;
        if w.graph != Some(graph) || w.group.is_some() {
            w.selected = None;
            w.pan = Vec2::ZERO;
            w.zoom = 0.;
        }
        w.graph = Some(graph);
        w.group = None;
        w.layer = layer;
        w.followed = self.shading.selected.last().copied();
        w.reveal = true;
    }

    /// Show as Nodes: Material Group `group`'s node view, in the Node View
    /// panel.
    pub(super) fn open_group_nodes(&mut self, group: NodeId) {
        let w = &mut self.graph_window;
        if w.group != Some(group) {
            w.selected = None;
            w.pan = Vec2::ZERO;
            w.zoom = 0.;
        }
        w.group = Some(group);
        w.graph = None;
        w.layer = None;
        w.group_view = None;
        w.followed = self.shading.selected.last().copied();
        w.reveal = true;
    }

    /// The group's graph as the window shows it, made again only after an
    /// edit (the editor's revision moved).
    pub(crate) fn group_graph(
        &mut self,
        group: NodeId,
    ) -> Option<std::result::Result<StoredGraph, String>> {
        let revision = self.editor.revision;
        if let Some((r, g, view)) = &self.graph_window.group_view
            && *r == revision
            && *g == group
        {
            return Some(view.clone());
        }
        let scene = &self.editor.scene;
        let name = scene.shader_tree.get(group)?.name.clone();
        let view = group_view(scene, group, &name);
        self.graph_window.group_view = Some((revision, group, view.clone()));
        Some(view)
    }

    /// After the panels drew: bring the Node View to the front, docking it
    /// under the views the first time (the dock tree is back in `views`).
    pub(super) fn node_view_frame(&mut self) {
        if std::mem::take(&mut self.graph_window.reveal)
            && let Err(e) = self.reveal_registry_panel(crate::panel_registry::PanelKind::NodeView)
        {
            self.status = e.to_string();
        }
    }

    /// A new Shader Tree selection moves the view to its material: a Node
    /// Material layer's graph, else the Material Group the layer is in.
    /// Anything else (a Render or Environment row, nothing) leaves the view
    /// as it is. Provisional: the spec doesn't say; this keeps the last
    /// material on screen rather than going blank.
    pub(crate) fn node_view_follow_selection(&mut self) {
        let selected = self.shading.selected.last().copied();
        if selected == self.graph_window.followed {
            return;
        }
        self.graph_window.followed = selected;
        let Some(id) = selected else {
            return;
        };
        let tree = &self.editor.scene.shader_tree;
        if let Some(NodeKind::NodeMaterial(m)) = tree.get(id).map(|n| &n.kind)
            && let Some(graph) = m.graph
        {
            if self.graph_window.graph != Some(graph) || self.graph_window.layer != Some(id) {
                let w = &mut self.graph_window;
                if w.graph != Some(graph) || w.group.is_some() {
                    w.selected = None;
                    w.pan = Vec2::ZERO;
                    w.zoom = 0.;
                }
                w.graph = Some(graph);
                w.group = None;
                w.layer = Some(id);
            }
            return;
        }
        let mut at = Some(id);
        while let Some(node) = at.and_then(|i| tree.get(i)) {
            if matches!(node.kind, NodeKind::Group(_)) {
                let group = node.id;
                if self.graph_window.group != Some(group) {
                    let w = &mut self.graph_window;
                    w.selected = None;
                    w.pan = Vec2::ZERO;
                    w.zoom = 0.;
                    w.group = Some(group);
                    w.graph = None;
                    w.layer = None;
                    w.group_view = None;
                    w.board.selected = None;
                }
                return;
            }
            at = node.parent;
        }
    }

    /// The Node View panel's body (NM-15): drag to pan, scroll or pinch to
    /// zoom, click a node to see its values on the right.
    pub(crate) fn node_view_body(&mut self, ui: &mut Ui, _rect: Rect) {
        self.node_view_follow_selection();
        // A Material Group edits as its layers and free nodes (node
        // editing); "As rendered" shows the read-only expanded form.
        if let Some(group) = self.graph_window.group
            && !self.graph_window.board.as_rendered
        {
            self.group_board_body(ui, group);
            return;
        }
        let shown = if let Some(group) = self.graph_window.group {
            match self.group_graph(group) {
                Some(Ok(g)) => Some((g, "Read-only · edit it in the Shader Tree".to_string())),
                Some(Err(e)) => Some((view_graph(empty_material("Nodes")), e)),
                None => {
                    self.graph_window.group = None;
                    None
                }
            }
        } else if let Some(id) = self.graph_window.graph {
            match self.editor.scene.shader_tree.node_graphs.get(id).cloned() {
                Some(graph) => Some(match materialx_view(&graph) {
                    Some(Ok(g)) => (g, "Read-only in this version".to_string()),
                    Some(Err(e)) => (graph, e),
                    None => (graph, "Read-only in this version".to_string()),
                }),
                None => {
                    self.graph_window.graph = None;
                    None
                }
            }
        } else {
            None
        };
        let Some((graph, note)) = shown else {
            empty_node_view(ui);
            return;
        };
        let layer = self
            .graph_window
            .layer
            .and_then(|l| self.editor.scene.shader_tree.get(l))
            .and_then(|n| match &n.kind {
                NodeKind::NodeMaterial(m)
                    if m.graph.is_some() && m.graph == self.graph_window.graph =>
                {
                    Some(m.clone())
                }
                _ => None,
            });
        let w = &mut self.graph_window;
        egui::SidePanel::right("node-graph-values")
            .default_width(220.)
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.strong(&graph.material.name);
                ui.weak(&note);
                if w.group.is_some() {
                    ui.checkbox(&mut w.board.as_rendered, "As rendered")
                        .on_hover_text("The read-only form Cycles renders; untick to edit");
                }
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    node_values(ui, &graph, w.selected, layer.as_ref());
                });
            });
        CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                graph_canvas(ui, &graph, w);
            });
    }
}

impl Operandi {
    /// The node view of Material Group `group`, editable (node editing).
    fn group_board_body(&mut self, ui: &mut Ui, group: NodeId) {
        let tree = self.editor.scene.shader_tree.clone();
        let Some(board) = tree.board(group) else {
            self.graph_window.group = None;
            empty_node_view(ui);
            return;
        };
        let name = tree.get(group).map_or(String::new(), |n| n.name.clone());
        let mut actions = Vec::new();
        let w = &mut self.graph_window;
        egui::SidePanel::right("node-graph-values")
            .default_width(240.)
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.strong(&name);
                ui.weak("Right-click to add nodes · drag outputs to inputs");
                ui.checkbox(&mut w.board.as_rendered, "As rendered")
                    .on_hover_text("The read-only form Cycles renders");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    crate::node_edit_ui::board_values(ui, &board, &tree, w, &mut actions);
                });
            });
        CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                crate::node_edit_ui::board_canvas(ui, &board, &tree, group, w, &mut actions);
            });
        self.apply_board_actions(group, actions);
    }
}

/// Nothing to show yet: say how to fill the panel.
fn empty_node_view(ui: &mut Ui) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0., theme::workspace::list());
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "Select a material in the Shader Tree\nto see its nodes",
        crate::typography::standard_font(),
        theme::disabled_text(),
    );
}

/// One exposed input's control (NM-14: colour swatch, float field, vector
/// fields). `shared` marks a value that follows the graph's default.
/// Returns (dragging, changed).
fn exposed_field(ui: &mut Ui, e: &Exposed, value: &mut Vec<f64>, shared: bool) -> (bool, bool) {
    let before = value.clone();
    let mut drag = false;
    let label = if shared {
        e.name.clone()
    } else {
        format!("{} •", e.name)
    };
    crate::option_row::field_row(ui, &label, |ui| match e.value_kind() {
        ValueKind::Color => {
            let mut c = [value[0], value[1], value[2]];
            crate::colour_field::show(ui, &mut c, crate::colour_field::Options::scene());
            *value = c.to_vec();
        }
        ValueKind::Float => {
            let lo = e.min.unwrap_or(-1e6);
            let hi = e.max.unwrap_or(1e6);
            let r = ui.add(
                crate::numeric_tab::DragValue::new(&mut value[0])
                    .range(lo..=hi)
                    .speed(if hi - lo <= 10. { 0.005 } else { 0.05 })
                    .max_decimals(4),
            );
            drag = r.dragged() && !r.drag_started();
        }
        ValueKind::Vector => {
            ui.horizontal(|ui| {
                for v in value.iter_mut().take(3) {
                    let r = ui.add(
                        crate::numeric_tab::DragValue::new(v)
                            .speed(0.01)
                            .max_decimals(4),
                    );
                    drag |= r.dragged() && !r.drag_started();
                }
            });
        }
    })
    .response
    .on_hover_text(if shared {
        "Follows the graph's shared default"
    } else {
        "This layer's own value (•) · Reset to Default to follow the graph again"
    });
    (drag, *value != before)
}

fn source_line(g: &StoredGraph) -> Option<String> {
    let app = g.source.get("app").and_then(|v| v.as_str());
    let file = g.source.get("file").and_then(|v| v.as_str());
    let licence = g.licence.get("name").and_then(|v| v.as_str());
    let mut parts = Vec::new();
    if let Some(file) = file {
        parts.push(format!("From {file}"));
    }
    if let Some(app) = app {
        parts.push(format!("({app})"));
    }
    if let Some(l) = licence {
        parts.push(format!("· {l}"));
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// The selected node's values (read-only).
fn node_values(
    ui: &mut Ui,
    g: &StoredGraph,
    selected: Option<usize>,
    layer: Option<&operandi_core::shader_tree::NodeMaterialLayer>,
) {
    let Some(n) = selected.and_then(|i| g.material.nodes.get(i)) else {
        ui.weak("Click a node to see its values");
        if !g.material.exposed.is_empty() {
            ui.add_space(6.);
            ui.strong("Exposed inputs");
            for e in &g.material.exposed {
                let v = match layer {
                    Some(m) => ShaderTree::exposed_value(m, e),
                    None => e.default_value(),
                };
                ui.label(format!(
                    "{} → {} · {}: {}",
                    e.name,
                    e.to.node(),
                    e.to.name(),
                    numbers_text(&v)
                ));
            }
        }
        return;
    };
    ui.strong(&n.name);
    if let Some(category) = operandi_core::materialx::view::category(&n.idname) {
        let ty = n.props.get("type").and_then(|v| v.as_str()).unwrap_or("");
        ui.weak(format!("MaterialX {category} ({ty})"));
        ui.separator();
        for (k, v) in &n.inputs {
            let text = match &v.value {
                serde_json::Value::String(s) => s.clone(),
                other => numbers_text(&operandi_core::node_graph::numbers(other)),
            };
            ui.label(format!("{k}: {text}"));
        }
        for l in g.material.links.iter().filter(|l| l.to.node() == n.name) {
            ui.label(format!("{} ← {}", l.to.name(), l.from.node()));
        }
        return;
    }
    ui.weak(operandi_core::node_graph::type_label(&n.idname));
    if !operandi_core::node_graph::is_supported(&n.idname) {
        ui.colored_label(theme::error(), format!("{} doesn't render here", n.idname));
    }
    ui.separator();
    for (k, v) in &n.props {
        if k == "ramp" {
            continue;
        }
        ui.label(format!("{k}: {}", prop_text(v)));
    }
    if n.idname == "ShaderNodeValToRGB" {
        let (stops, interpolation) = operandi_core::node_graph::ramp_stops(n);
        ui.label(format!("ramp ({}):", interpolation.to_lowercase()));
        for (at, c) in stops {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(14., 14.), Sense::hover());
                ui.painter()
                    .rect_filled(r, 2., linear_colour([c[0], c[1], c[2]]));
                ui.label(format!("{at:.3}: {}", numbers_text(&c)));
            });
        }
    }
    if let Some(img) = &n.image {
        ui.label(format!("image: {}", img.path));
        ui.label(format!("colour space: {}", img.colorspace));
    }
    for (k, v) in &n.inputs {
        let text = match &v.value {
            serde_json::Value::String(s) => s.clone(),
            other => numbers_text(&operandi_core::node_graph::numbers(other)),
        };
        ui.label(format!("{k}: {text}"));
    }
    for e in g.material.exposed.iter().filter(|e| e.to.node() == n.name) {
        let v = match layer {
            Some(m) => ShaderTree::exposed_value(m, e),
            None => e.default_value(),
        };
        ui.label(format!(
            "{} (exposed “{}”): {}",
            e.to.name(),
            e.name,
            numbers_text(&v)
        ));
    }
}

/// A MaterialX graph layer's graph as the window draws it; `None` for a
/// Cycles-node graph.
fn materialx_view(g: &StoredGraph) -> Option<std::result::Result<StoredGraph, String>> {
    let mx = g.materialx.as_ref()?;
    Some(
        mx.document()
            .and_then(|d| operandi_core::materialx::view::graph(&d, Some(&mx.material)))
            .map(|material| StoredGraph {
                material,
                ..g.clone()
            }),
    )
}

#[cfg(test)]
pub(crate) fn materialx_view_for_tests(
    g: &StoredGraph,
) -> Option<std::result::Result<StoredGraph, String>> {
    materialx_view(g)
}

/// Material Group `group`'s MaterialX graph (its surface's tree form, the
/// graph Cycles renders) as the window draws it.
pub(crate) fn group_view(
    scene: &operandi_core::Scene,
    group: NodeId,
    name: &str,
) -> std::result::Result<StoredGraph, String> {
    use operandi_core::materialx::{tree, tree_standard, view};
    let surface =
        tree_standard::group_surface(scene, group).ok_or("Only a Material Group shows as nodes")?;
    let text = tree::write(&surface, scene, name);
    let doc = operandi_core::materialx::doc::parse(&text)?;
    let mut material = view::graph(&doc, None)?;
    material.name = name.to_owned();
    Ok(view_graph(material))
}

/// A graph only the window shows (not in the scene).
fn view_graph(material: operandi_core::node_graph::GraphMaterial) -> StoredGraph {
    StoredGraph {
        id: 0,
        material,
        groups: Vec::new(),
        source: serde_json::Value::Null,
        licence: serde_json::Value::Null,
        materialx: None,
    }
}

fn empty_material(name: &str) -> operandi_core::node_graph::GraphMaterial {
    operandi_core::node_graph::GraphMaterial {
        name: name.into(),
        output: String::new(),
        nodes: Vec::new(),
        links: Vec::new(),
        exposed: Vec::new(),
        groups_used: Vec::new(),
        extra: Default::default(),
    }
}

fn prop_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn numbers_text(v: &[f64]) -> String {
    v.iter()
        .map(|x| format!("{x:.3}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A linear colour shown on screen (sRGB transfer).
fn linear_colour(c: [f64; 3]) -> Color32 {
    let s = |v: f64| {
        let v = v.clamp(0., 1.);
        let s = if v <= 0.003_130_8 {
            v * 12.92
        } else {
            1.055 * v.powf(1. / 2.4) - 0.055
        };
        (s * 255.).round() as u8
    };
    Color32::from_rgb(s(c[0]), s(c[1]), s(c[2]))
}

/// Node boxes laid out where the file put them, else in columns by depth.
pub(crate) fn layout(g: &StoredGraph) -> Vec<Pos2> {
    let nodes = &g.material.nodes;
    if nodes.iter().all(|n| n.location().is_some()) {
        // Blender's locations grow upward; the screen grows down.
        return nodes
            .iter()
            .map(|n| {
                let [x, y] = n.location().unwrap_or_default();
                pos2(x as f32, -y as f32)
            })
            .collect();
    }
    // Depth from the output: how many links lie between a node and it.
    let mut depth = vec![0usize; nodes.len()];
    for _ in 0..nodes.len() {
        let mut moved = false;
        for l in &g.material.links {
            let (Some(a), Some(b)) = (
                nodes.iter().position(|n| n.name == l.from.node()),
                nodes.iter().position(|n| n.name == l.to.node()),
            ) else {
                continue;
            };
            if depth[a] < depth[b] + 1 {
                depth[a] = depth[b] + 1;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    let deepest = depth.iter().copied().max().unwrap_or(0);
    let mut rows = vec![0usize; deepest + 1];
    depth
        .iter()
        .map(|&d| {
            let row = rows[d];
            rows[d] += 1;
            pos2((deepest - d) as f32 * 220., row as f32 * 120.)
        })
        .collect()
}

const NODE_W: f32 = 160.;

fn node_height(n: &GraphNode, g: &StoredGraph) -> f32 {
    let inputs = sockets_in(n, g).len();
    let outputs = sockets_out(n, g).len();
    28. + 16. * inputs.max(outputs).max(1) as f32
}

/// The input sockets a node shows: linked ones and those with values.
fn sockets_in(n: &GraphNode, g: &StoredGraph) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in g.material.links.iter().filter(|l| l.to.node() == n.name) {
        if !out.iter().any(|s| s == l.to.key()) {
            out.push(l.to.key().to_string());
        }
    }
    for k in n.inputs.keys() {
        if !out.contains(k) {
            out.push(k.clone());
        }
    }
    out
}

fn sockets_out(n: &GraphNode, g: &StoredGraph) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in g.material.links.iter().filter(|l| l.from.node() == n.name) {
        if !out.iter().any(|s| s == l.from.key()) {
            out.push(l.from.key().to_string());
        }
    }
    out
}

/// The canvas: boxes, socket rows and links; pan, zoom and select.
fn graph_canvas(ui: &mut Ui, g: &StoredGraph, w: &mut GraphWindow) {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0., theme::workspace::list());
    let places = layout(g);
    // Fit the graph on first show.
    if w.zoom <= 0. {
        let (lo, hi) = places.iter().zip(&g.material.nodes).fold(
            (pos2(f32::MAX, f32::MAX), pos2(f32::MIN, f32::MIN)),
            |(lo, hi), (p, n)| (lo.min(*p), hi.max(*p + vec2(NODE_W, node_height(n, g)))),
        );
        let span = (hi - lo).max(vec2(1., 1.));
        w.zoom = (rect.width() / (span.x + 80.))
            .min(rect.height() / (span.y + 80.))
            .clamp(0.2, 1.5);
        w.pan = rect.center().to_vec2() - ((lo.to_vec2() + hi.to_vec2()) * 0.5) * w.zoom;
    }
    if response.dragged() {
        w.pan += response.drag_delta();
    }
    if response.hovered() {
        let (scroll, pinch, at) = ui.input(|i| {
            (
                i.smooth_scroll_delta.y,
                i.zoom_delta(),
                i.pointer.hover_pos(),
            )
        });
        let factor = pinch * (scroll * 0.002).exp();
        if factor != 1.
            && let Some(at) = at
        {
            let z = (w.zoom * factor).clamp(0.1, 3.);
            let k = z / w.zoom;
            w.pan = at.to_vec2() + (w.pan - at.to_vec2()) * k;
            w.zoom = z;
        }
    }
    let to_screen = |p: Pos2| pos2(p.x * w.zoom + w.pan.x, p.y * w.zoom + w.pan.y);
    let boxes: Vec<Rect> = places
        .iter()
        .zip(&g.material.nodes)
        .map(|(p, n)| Rect::from_min_size(to_screen(*p), vec2(NODE_W, node_height(n, g)) * w.zoom))
        .collect();
    let socket_at = |i: usize, key: &str, output: bool| -> Option<Pos2> {
        let n = &g.material.nodes[i];
        let list = if output {
            sockets_out(n, g)
        } else {
            sockets_in(n, g)
        };
        let k = list.iter().position(|s| s == key)?;
        let b = boxes[i];
        let y = b.top() + (28. + 16. * k as f32 + 8.) * w.zoom;
        Some(pos2(if output { b.right() } else { b.left() }, y))
    };
    // Links under the boxes.
    for l in &g.material.links {
        let (Some(a), Some(b)) = (
            g.material
                .nodes
                .iter()
                .position(|n| n.name == l.from.node()),
            g.material.nodes.iter().position(|n| n.name == l.to.node()),
        ) else {
            continue;
        };
        let (Some(p), Some(q)) = (
            socket_at(a, l.from.key(), true),
            socket_at(b, l.to.key(), false),
        ) else {
            continue;
        };
        let dx = ((q.x - p.x).abs() * 0.5).max(30. * w.zoom);
        let curve = egui::epaint::CubicBezierShape::from_points_stroke(
            [p, p + vec2(dx, 0.), q - vec2(dx, 0.), q],
            false,
            Color32::TRANSPARENT,
            Stroke::new(1.5_f32, theme::text().gamma_multiply(0.6)),
        );
        painter.add(curve);
    }
    let font = FontId::proportional((11. * w.zoom).clamp(6., 16.));
    let small = FontId::proportional((10. * w.zoom).clamp(5., 14.));
    let mut clicked = None;
    for (i, (n, b)) in g.material.nodes.iter().zip(&boxes).enumerate() {
        // A MaterialX node renders through the MaterialX translation.
        let supported = operandi_core::node_graph::is_supported(&n.idname)
            || operandi_core::materialx::view::category(&n.idname).is_some();
        let selected = w.selected == Some(i);
        painter.rect_filled(*b, 4. * w.zoom, theme::panel());
        let header = Rect::from_min_size(b.min, vec2(b.width(), 22. * w.zoom));
        painter.rect_filled(
            header,
            4. * w.zoom,
            if supported {
                theme::orange().gamma_multiply(0.35)
            } else {
                theme::error().gamma_multiply(0.5)
            },
        );
        painter.rect_stroke(
            *b,
            4. * w.zoom,
            Stroke::new(
                if selected { 2_f32 } else { 1_f32 },
                if selected {
                    theme::orange()
                } else {
                    theme::text().gamma_multiply(0.3)
                },
            ),
            egui::StrokeKind::Inside,
        );
        painter.text(
            header.left_center() + vec2(6. * w.zoom, 0.),
            Align2::LEFT_CENTER,
            &n.name,
            font.clone(),
            theme::text(),
        );
        for (k, s) in sockets_in(n, g).iter().enumerate() {
            let y = b.top() + (28. + 16. * k as f32 + 8.) * w.zoom;
            painter.circle_filled(pos2(b.left(), y), 3. * w.zoom, theme::text());
            painter.text(
                pos2(b.left() + 6. * w.zoom, y),
                Align2::LEFT_CENTER,
                s,
                small.clone(),
                theme::text().gamma_multiply(0.8),
            );
        }
        for (k, s) in sockets_out(n, g).iter().enumerate() {
            let y = b.top() + (28. + 16. * k as f32 + 8.) * w.zoom;
            painter.circle_filled(pos2(b.right(), y), 3. * w.zoom, theme::text());
            painter.text(
                pos2(b.right() - 6. * w.zoom, y),
                Align2::RIGHT_CENTER,
                s,
                small.clone(),
                theme::text().gamma_multiply(0.8),
            );
        }
        if response.clicked()
            && response
                .interact_pointer_pos()
                .is_some_and(|p| b.contains(p))
        {
            clicked = Some(i);
        }
    }
    if response.clicked() {
        w.selected = clicked;
    }
}
